# ferrigura-swimlane

`ferrigura-swimlane` is the swimlane layout of Mermaid 12.2.0, ported from its layout engine: the Sugiyama phases, the orthogonal router and the geometry passes of the default path. The options `ignoreCrossLaneEdges: false` and `automaticLaneOrdering: true` are not ported, and a layout with them is refused with an error.

This [Ferrigura](https://github.com/sebastian-software/ferrigura) implementation crate exposes an experimental API. The [`ferrigura` facade](https://crates.io/crates/ferrigura) provides the [supported Rust API](https://github.com/sebastian-software/ferrigura/blob/main/docs/decisions/0022-stable-public-api.md).

## License

MIT, as the workspace and as Mermaid is (the `LICENSE` file, copied from the repository root). The `localeCompare` of the layout is ICU's root collation for ASCII, written as a table. `NOTICE.md` has the details.
