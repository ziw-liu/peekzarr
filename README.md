# peekzarr

Visualize OME-Zarr images in the terminal.

## System requirements

libsixel

## Build

```sh
cargo build --release
```

## Terminal emulator compatibility

Known to work with recent versions of:

- Konsole (Linux)
- iTerm2 (macOS)
- Ghostty (macOS)

Known issues:

- VS Code: [`terminal.integrated.enableImages`](vscode://settings/terminal.integrated.enableImages) needs to be enabled.
- tmux: To view high resolution images, version 3.4+ is required and the feature is behind a compile flag.
