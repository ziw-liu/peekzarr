# peekzarr

Visualize OME-Zarr images in the terminal.
Supports local files and HTTP.

## Example

```
peekzarr https://public.czbiohub.org/comp.micro/viscy/VS_datasets/VSCyto2D/test/a549_hoechst_cellmask_test.zarr/0/0/0 -s 0,0
peekzarr https://public.czbiohub.org/royerlab/zebrahub/imaging/single-objective/ZSNS001.ome.zarr -a /2
```

See full help message with `peekzarr -h`.

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
- tmux: To view high resolution images, version 3.4+ is required and the feature is behind a compile flag. The true-color feature also needs to be enabled.

## License

Licensed under either of
[Apache 2.0](./LICENSES/Apache-2.0.txt) or [MIT](./LICENSES/MIT.txt) terms,
at your option.
