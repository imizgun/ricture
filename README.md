# ricture

A small screenshot tool for Wayland, written in Rust. Inspired by [Spectacle](https://apps.kde.org/spectacle/) and [MarkShot](https://github.com/jswysnemc/mark-shot).

No GTK/Qt — the overlay is a raw `wlr-layer-shell` surface rendered in software with [`tiny-skia`](https://github.com/RazrFalcon/tiny-skia), UI renders with [`Slint`](https://github.com/slint-ui/slint)

## Features
- [x] Area and fullscreen screenshots
- [x] Notification via [`rustbus`](https://github.com/KillingSpark/rustbus)
- [ ] Animation on startup
- [ ] Annotation tools:
- - [x] Brush
- - [ ] Arrow
- - [ ] Line
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

Copying to clipboard shells out to the `wl-copy` binary from [`wl-clipboard`](https://github.com/bugaevc/wl-clipboard) — it must be installed and on `$PATH` at run time, separately from the build-time dependencies above. `nix build .#default` wraps the binary with it automatically; outside Nix, install `wl-clipboard` through your distro's package manager.

## Running

```sh
cargo run --release
```
or (for instant fullscreen shot):
```sh
cargo run --release -- --fullscreen 
```

Drag a rectangle, hit `Enter` to save it to `screenshot.png` in the current directory, or `Esc` to cancel.

## Keybinds
| Key | Action |
|-----|--------|
| `Ctrl + C` / `Enter` / `Space` |   Copy selected area to clipboard     |
| `Ctrl + S` |   Save selected to `.png` in ~/Pictures/Screenshots directory     |
| `Ctrl + Z` |   Undo last annotation     |

## Config
`ricture` uses a TOML config file located at `~/.config/ricture/config.toml` (expample [`here`](https://github.com/imizgun/ricture/blob/main/examples/config.toml)):
```toml
[general]
save_path = "/home/user/Pictures/Screenshots"
enable_notifications = true

[appearance]
rect_color = "#ffffff" # '#rrggbbaa' is also valid

[tools.brush]
color = "#ff3b30" # '#rrggbbaa' is also valid
width = 4 # 1..32 logical px, larger values are limited to 32
```



## Layout

A Cargo workspace, one crate per concern:

- `crates/capture` (`ricture-capture`) — pure Wayland screen capture. No UI, no rendering; just talks to `wlr-screencopy` and hands back raw RGBA pixels.
- `crates/overlay` (`ricture-overlay`) — the interactive part: the layer-shell surface, input handling, `tiny-skia` rendering and the Slint UI. Knows tools only through the `ricture-tools` traits.
- `crates/tools` (`ricture-tools`) — annotation tools: the `Tool` / `Gesture` / `Annotation` traits, the `Document` holding annotations, and the tool implementations. Adding a tool = implementing `Tool` and listing it in `default_tools()`.
- `crates/config` (`ricture-config`) — TOML config parsing and validation.
- `crates/export` (`ricture-export`) — PNG saving and clipboard copying.
- `crates/notify` (`ricture-notify`) — desktop notifications.
- `crates/ricture` (`ricture`, the binary) — thin entry point that wires the other two together and does the final crop + save.

## License

GPL-3.0 — see [LICENSE](./LICENSE).
