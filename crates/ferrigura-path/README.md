# ferrigura-path

`ferrigura-path` computes the bounding box of an SVG path as Chromium's `getBBox()` does, bit for bit. It reads the path data as Blink reads it, decomposes the arcs as Blink does, and takes the tight bounds from Skia, in float. The mindmap's cloud and bang shapes take their size from here.

This [Ferrigura](https://github.com/sebastian-software/ferrigura) implementation crate exposes an experimental API. The [`ferrigura` facade](https://crates.io/crates/ferrigura) provides the [supported Rust API](https://github.com/sebastian-software/ferrigura/blob/main/docs/decisions/0022-stable-public-api.md).

## License

The crate is MIT-licensed. Its ported parts keep the licenses of their sources: Skia's BSD-3-Clause license (`LICENSE-SKIA`) and Chromium's license (`LICENSE-CHROMIUM`), which covers Blink and `ui/gfx`. `NOTICE.md` lists each module and its source.
