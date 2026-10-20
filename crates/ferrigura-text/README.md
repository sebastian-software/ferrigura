# ferrigura-text

`ferrigura-text` measures text with the bundled fonts, wraps lines the way Mermaid wraps its labels, and embeds subsets of the fonts in the SVG, so that a viewer draws the text that was measured. It bundles the faces that ferrigura uses: Lexica Ultralegible, Atkinson Hyperlegible Mono, and the symbol and emoji faces.

This crate is a part of [ferrigura](https://github.com/sebastian-software/ferrigura). Its API may change between versions. To render diagrams, use the `ferrigura` crate.

The crate is not published yet.

## Licence

The crate is MIT-licensed, as the workspace (the `license` field of `Cargo.toml`). The bundled fonts keep their own licences: the SIL Open Font License 1.1 for the text and symbol faces, and Creative Commons Attribution 4.0 for the Twemoji art, with its attribution. `NOTICE.md` lists each file, its source and its licence.
