# ferrigura-model

`ferrigura-model` holds the diagram models of ferrigura, the configuration and the themes. The configuration types and their defaults are generated at build time from the configuration schema of the repository, and the themes are typed color tokens. The models also have a canonical JSON form, which the differential tests compare with Mermaid's own data.

This [Ferrigura](https://github.com/sebastian-software/ferrigura) implementation crate exposes an experimental API. The [`ferrigura` facade](https://crates.io/crates/ferrigura) provides the [supported Rust API](https://github.com/sebastian-software/ferrigura/blob/main/docs/decisions/0022-stable-public-api.md).

## License

MIT, as the workspace (the `license` field of `Cargo.toml`), like Mermaid.
