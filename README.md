# ricture

A small screenshot tool for Wayland, written in Rust. Inspired by [Spectacle](https://apps.kde.org/spectacle/) and [MarkShot](https://github.com/jswysnemc/mark-shot).

No GTK/Qt — the overlay is a raw `wlr-layer-shell` surface rendered in software with [`tiny-skia`](https://github.com/RazrFalcon/tiny-skia), UI renders with [`Slint`](https://github.com/slint-ui/slint)

## Features
- [x] Area and fullscreen screenshots
- [x] Notification via [`rustbus`](https://github.com/KillingSpark/rustbus)
- [ ] Animation on startup
- [ ] Annotation tools:
- - [x] Brush
- - [x] Arrow
- - [x] Line
- - [ ] Primitive shapes
- - [ ] Text
- - [ ] Numbered circles
- - [ ] Blur
- - [ ] Scaler


## Building

This project depends on `libxkbcommon` (via `smithay-client-toolkit`) and `fontconfig` (via Slint's system font loading) at build time, both found through `pkg-config`. A [Nix flake](./flake.nix) is included with a dev shell that provides them:

```sh
nix develop
cargo build --release
```

Without Nix, just make sure `pkg-config` plus the dev headers for `libxkbcommon` and `fontconfig` are installed and `cargo build --release` should work directly.

Use `--release` — the overlay redraws itself continuously while open, and an unoptimized debug build is easily 30x slower per frame, which is very noticeable as input lag while dragging.

### Runtime dependencies

None beyond the libraries above. The compositor must support `wlr-layer-shell`, `wlr-screencopy` and, for copying to the clipboard, `wlr-data-control` (ricture serves the clipboard itself, no `wl-clipboard` needed) — wlroots-based compositors, niri, Hyprland and KDE do; GNOME does not.

## Profiling

A profiling build prints a startup timeline (time and memory per stage) and per-frame stage statistics to stderr on exit:

```sh
cargo prof                          # interactive run, report when the overlay closes
RICTURE_PROFILE_EXIT=1 cargo prof   # close right after the first frame, for repeatable startup numbers
```

`cargo prof` is an alias (see [`.cargo/config.toml`](./.cargo/config.toml)) for `cargo run --profile profiling --features profiling`. The `profiling` profile is `release` plus line tables, so it also works with `perf`. Without the feature, all profiling calls compile to nothing.

## Running

```sh
cargo run --release
```
or (for instant fullscreen shot):
```sh
cargo run --release -- --fullscreen 
```

Drag a rectangle, hit `Enter` to save it to `screenshot.png` in the current directory, or `Esc` to cancel.

Annotations aren't confined to the selected area: a stroke may run past its edge and stays there, dimmed like everything outside it. Only the selected area is exported, so moving the selection afterwards decides what of the drawing ends up in the image.

## Keybinds
| Key | Action |
|-----|--------|
| `Ctrl + C` / `Enter` / `Space` |   Copy selected area to clipboard     |
| `Ctrl + S` |   Save selected to `.png` in ~/Pictures/Screenshots directory     |
| `Ctrl + Z` |   Undo last annotation     |
| `Esc` |   Cancel     |
| `1`, `2`, `3`, ... |   Pick the tool in that place in the toolbar (its tooltip shows the key)     |
| `0` |   Back to selecting the area     |

Every one of them is remappable, see [`[keybinds]`](#config). Keys are matched
by their position on the keyboard rather than by the character the active
layout produces, so `Ctrl + C` keeps working on a Cyrillic layout; the names in
the config are the labels of a US layout. Keys bound to nothing do nothing —
the overlay closes only on `Esc` or on a confirmation.

## Config
`ricture` uses a TOML config file located at `~/.config/ricture/config.toml` (expample [`here`](https://github.com/imizgun/ricture/blob/main/examples/config.toml)):
```toml
[general]
save_path = "/home/user/Pictures/Screenshots"
enable_notifications = true

[appearance]
rect_color = "#ffffff" # '#rrggbbaa' is also valid

[tools.brush]
# color = "#ff3b30" # '#rrggbbaa' is also valid
# width = 4 # 1..32 logical px, larger values are limited to 32

[keybinds]
# 'Mod+Key' with any of ctrl/shift/alt/super; a list binds several keys to the
# same action, an empty list ('copy = []') leaves it unbound.
cancel = "Escape"
copy = ["Return", "Space", "Ctrl+C"]
save = "Ctrl+S"
undo = "Ctrl+Z"
select = "0"

[keybinds.tools]
# By tool name. Tools left out keep the digit of their place in the toolbar.
# brush = "1"
# arrow = ["2", "a"]
# line = "3"
```

The brush starts with the color and width of the last confirmed screenshot, remembered in `~/.local/state/ricture/state.toml`. Set them in `[tools.brush]` to always start with fixed values instead.



## Layout

A Cargo workspace, one crate per concern:

- `crates/capture` (`ricture-capture`) — pure Wayland screen capture. No UI, no rendering; just talks to `wlr-screencopy` and hands back raw RGBA pixels.
- `crates/overlay` (`ricture-overlay`) — the interactive part: the layer-shell surface, input handling, `tiny-skia` rendering and the Slint UI. Knows tools only through the `ricture-tools` traits.
- `crates/tools` (`ricture-tools`) — annotation tools: the `Tool` / `Gesture` / `Annotation` traits, the `Document` holding annotations, and the tool implementations. Adding a tool = implementing `Tool` and listing it in `default_tools()`; its place in that list gives it its toolbar slot and its default keybind, and its `id` is the name the config binds keys to.
- `crates/config` (`ricture-config`) — TOML config parsing and validation.
- `crates/export` (`ricture-export`) — PNG saving and clipboard copying.
- `crates/notify` (`ricture-notify`) — desktop notifications.
- `crates/ricture` (`ricture`, the binary) — thin entry point that wires the other two together and does the final crop + save.

## License

GPL-3.0 — see [LICENSE](./LICENSE).
