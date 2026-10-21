# ferrigura-swimlane

`ferrigura-swimlane` is the swimlane layout of Mermaid 12.2.0, ported from its layout engine: the Sugiyama phases, the orthogonal router and the geometry passes of the default path. The options `ignoreCrossLaneEdges: false` and `automaticLaneOrdering: true` are not ported, and a layout with them is refused with an error.

This crate is a part of [ferrigura](https://github.com/sebastian-software/ferrigura). Its API may change between versions. To render diagrams, use the `ferrigura` crate.

## Licence

MIT, as the workspace and as Mermaid is (the `LICENSE` file, copied from the repository root). The `localeCompare` of the layout is ICU's root collation for ASCII, written as a table. `NOTICE.md` has the details.
