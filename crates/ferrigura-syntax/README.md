# ferrigura-syntax

`ferrigura-syntax` holds the parsers of the diagram types of ferrigura. They are written by hand on one shared lexer base, and each one returns the model of `ferrigura-model`. The crate also reads the YAML front matter, with a port of the js-yaml loader, and the configuration directives of a diagram.

This crate is a part of [ferrigura](https://github.com/sebastian-software/ferrigura). Its API may change between versions. To render diagrams, use the `ferrigura` crate.

The crate is not published yet.

## Licence

MIT, as the workspace (the `license` field of `Cargo.toml`), like Mermaid.
