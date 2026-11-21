# ferrigura-syntax

`ferrigura-syntax` holds the parsers of the diagram types of ferrigura. They are written by hand on one shared lexer base, and each one returns the model of `ferrigura-model`. The crate also reads the YAML front matter, with a port of the js-yaml loader, and the configuration directives of a diagram.

This [Ferrigura](https://github.com/sebastian-software/ferrigura) implementation crate exposes an experimental API. The [`ferrigura` facade](https://crates.io/crates/ferrigura) provides the [supported Rust API](https://github.com/sebastian-software/ferrigura/blob/main/docs/decisions/0022-stable-public-api.md).

## License

MIT, as the workspace (the `license` field of `Cargo.toml`), like Mermaid.
