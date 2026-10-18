# Resolve the configuration once, and hand renderers read-only typed views

The configuration is resolved in three layers, each computed once, at the level where it can change:

1. **Per process:** the defaults (the schema's and the reference's TypeScript-only ones) and the variables of each shipped theme, computed once and frozen.
2. **Per renderer:** the site configuration, the counterpart of Mermaid's `initialize`, merged onto the defaults, typed, and the light and dark theme variables with its `themeVariables` overrides. The API gains a `Renderer` that holds them (`Renderer::new(config)` in Rust, `new Renderer(config)` in the Node addon); `render(text, config)` stays as a shorthand over a small cache keyed by the configuration.
3. **Per diagram:** only the diagram's own front matter and directives, merged on top when there are any.

The layering keeps the reference's semantics (`assignWithDepth` with its depth, the directive sanitising, the secure keys), which `pnpm port:diff config` keeps checking.

Below the facade, the configuration is read-only. A renderer receives a typed view of what it reads: its own section of the typed configuration, the few global values it uses (font, look, security level and the like), and the theme as a typed set of the semantic tokens of [0007](./0007-themes-as-css-variables.md), not a map read by name. Nothing a renderer derives is written back; it is the renderer's own data.

Measured on 2026-10-17 ([comparisons](../comparisons/README.md)): 84 percent of a small render goes into rebuilding the defaults, merging the layers, reading the typed view and computing both colour schemes, although between renders usually only the diagram's own text changes. Mermaid shows the cost of the other extreme: `getConfig()` hands each of its 250 call sites a deep copy of the whole configuration, theme variables included, to protect a global mutable configuration from the 21 sites that write to what they get; that copy is 57 percent of its render time. Most call sites read one or two values.

Considered and not chosen:

- **A cache inside the present `render(text, config)` alone.** It gets most of the speed and keeps the API, but renderers keep reading the whole configuration and the theme by string key (42 places), and the P3 renderers would copy that.
- **Mermaid's structure** (a global configuration with `getConfig` and `setConfig`). The port needs the reference's semantics, not its structure, and a global rules out concurrent renders with different configurations.
- **The whole typed configuration for every renderer, as now.** Simple, but each renderer depends on everything, and the tokens of 0007 never become the interface.

## Consequences

- The order of the work: the cache first, with no change to the API or the output; then the `Renderer`; then the typed sections and theme tokens, renderer by renderer, before the P3 renderers are written, so that they start on the new contract.
- The theme tokens become a Rust type checked against the redux themes' variables, so a token a renderer reads exists at compile time rather than as a missing key at run time. `themeVariables` stay strings at the boundary and are parsed once per renderer.
- The output does not change. Every differential and geometry check guards each step; a step that moves a single byte of an SVG is a bug.
- Decided by Sebastian on 2026-10-18.
