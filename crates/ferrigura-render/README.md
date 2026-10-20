# ferrigura-render

`ferrigura-render` lays out the diagrams of ferrigura and writes them as SVG: the renderers of each diagram type, the shapes, the edges and the style sheets. It uses the layout engines of the other crates, `ferrigura-elk`, `ferrigura-cose`, `ferrigura-swimlane` and `ferrigura-venn`, and the text measurement of `ferrigura-text`.

This crate is a part of [ferrigura](https://github.com/sebastian-software/ferrigura). Its API may change between versions. To render diagrams, use the `ferrigura` crate, which calls it.

The crate is not published yet.

## Licence

MIT, as the workspace (the `license` field of `Cargo.toml`), like Mermaid.
