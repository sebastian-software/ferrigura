# Notice: sources and licence of ferrigura-swimlane

The crate is MIT-licensed, as its sources are. It ports the swimlane layout engine of Mermaid 12.2.0,
`packages/mermaid/src/rendering-util/layout-algorithms/swimlanes/` (default path only), which is MIT
(Copyright (c) 2014 - 2022 Knut Sveidqvist, and the Mermaid contributors). The text of the licence is in
`LICENSE`, copied from the repository's root.

The port reads no third-party package. The floating-point helpers (`Math.hypot`, `Math.sin` and the rest)
come from `ferrigura-jsmath`, whose notice is in its own `NOTICE.md`. The `localeCompare` of the layout is
ICU's root collation for ASCII, written as a table in `src/js.rs` and checked against V8.
