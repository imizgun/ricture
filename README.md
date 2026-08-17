# ricture

A small screenshot tool for Wayland, written in Rust. Inspired by [Spectacle](https://apps.kde.org/spectacle/) and [MarkShot](https://github.com/jswysnemc/mark-shot).

No GTK/Qt — the overlay is a raw `wlr-layer-shell` surface rendered in software with [`tiny-skia`](https://github.com/RazrFalcon/tiny-skia).

## Building

This project depends on `libxkbcommon` at build time (via `smithay-client-toolkit`), which needs `pkg-config` to be found. A [Nix flake](./flake.nix) is included with a dev shell that provides it:

```sh
nix develop --command cargo build --release
```

Without Nix, just make sure `pkg-config` and `libxkbcommon`'s dev headers are installed and `cargo build --release` should work directly.

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

## Layout

A Cargo workspace, one crate per concern:

- `crates/capture` (`ricture-capture`) — pure Wayland screen capture. No UI, no rendering; just talks to `wlr-screencopy` and hands back raw RGBA pixels.
- `crates/overlay` (`ricture-overlay`) — the interactive part: the layer-shell surface, input handling, and `tiny-skia` rendering. This is also where annotation tools will eventually live.
- `crates/ricture` (`ricture`, the binary) — thin entry point that wires the other two together and does the final crop + save.

## License

GPL-3.0 — see [LICENSE](./LICENSE).
