# ferrigura-model

`ferrigura-model` holds the diagram models of ferrigura, the configuration and the themes. The configuration types and their defaults are generated at build time from the configuration schema of the repository, and the themes are typed colour tokens. The models also have a canonical JSON form, which the differential tests compare with Mermaid's own data.

This crate is a part of [ferrigura](https://github.com/sebastian-software/ferrigura). Its API may change between versions. To render diagrams, use the `ferrigura` crate.

## Licence

MIT, as the workspace (the `license` field of `Cargo.toml`), like Mermaid.
