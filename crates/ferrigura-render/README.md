# ferrigura-render

`ferrigura-render` lays out the diagrams of ferrigura and writes them as SVG: the renderers of each diagram type, the shapes, the edges and the style sheets. It uses the layout engines of the other crates, `ferrigura-elk` and `ferrigura-cose`, and the text measurement of `ferrigura-text`.

This [Ferrigura](https://github.com/sebastian-software/ferrigura) implementation crate exposes an experimental API. The [`ferrigura` facade](https://crates.io/crates/ferrigura) provides the [supported Rust API](https://github.com/sebastian-software/ferrigura/blob/main/docs/decisions/0022-stable-public-api.md).

## License

MIT, as the workspace (the `license` field of `Cargo.toml`), like Mermaid.
