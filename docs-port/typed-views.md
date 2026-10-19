# Typed views for the renderers (issue #4)

Step 3 of [decision 0009](./decisions/0009-configuration-resolved-once.md), issue [#4](https://github.com/sebastian-software/ferrigura/issues/4). This page is the contract between the facade and the renderers. Decided by Sebastian on 2026-10-19, with the three choices below.

## Where we are

- The facade resolves the configuration once per renderer and per directive (steps 1 and 2) and hands each renderer the whole typed `MermaidConfig`, or for pie its own section (`PieInput::config: &PieDiagramConfig`), and `Themes`: one or two `Value` maps of the theme's variables.
- Renderers read theme variables by name through `variables::{text_of, texts_of, truthy, px}`: 168 reads of at least 49 names in about 60 files. A misspelt name is a silent `None`, which the reference's `undefined` makes look plausible.
- A few renderers read the untyped configuration besides (`packet::bits_per_row`, `TreemapSettings::of(&prepared.effective)`), because the schema-derived types do not cover what they need.
- Each renderer that writes colours as custom properties names its own fixed list of tokens (`COLOUR_TOKENS` in `flowchart/style.rs`, `TOKENS` in `common/style.rs`); no renderer iterates the whole map.

## The contract

A renderer receives exactly what it reads, typed and read-only:

```rust
/// What every renderer gets: its own model, its own configuration section, the few global values
/// it reads, the theme as typed tokens, and the id.
pub struct Context<'a, Model, Section> {
    pub diagram: &'a Model,
    pub section: &'a Section,
    pub globals: &'a Globals,
    pub theme: ThemeView<'a>,
    pub id: &'a str,
}

/// The global options renderers read, resolved once per view. The list is closed: a renderer that
/// needs another global value adds it here.
pub struct Globals {
    pub font_family: String,
    pub font_size: Option<f64>,
    pub look: Look,
    pub use_max_width: bool,     // per diagram, filled from the section where the reference does
    pub security_level: SecurityLevel,
    pub layout: LayoutName,
    pub hand_drawn_seed: u32,
    // … as the inventory of global reads finds them
}

/// The theme, as typed tokens. `light` is the scheme the presentation attributes and the sizes come
/// from; `dark` is set when the SVG carries both schemes (`colorScheme: auto`).
pub struct ThemeView<'a> {
    pub light: &'a ThemeTokens,
    pub dark: Option<&'a ThemeTokens>,
}
```

`ThemeTokens` is one struct with a field per variable the redux themes compute (about 150, in the order of `theme-redux-color.js`), generated from a single list of `(field, "jsName", kind)` by a macro, so that the custom-property name of each field is known at compile time (`ThemeTokens::NAMES`). A token holds the value as the reference holds it: a `Text` for colours and sizes (the output writes it as given, so `hsl(240, 90%, NaN%)` stays what it is), a `bool`, or a number. Missing values are `None`, as the reference's `undefined`. Diagram groups of `themeVariables` (`themeVariables.treeView`, `.packet`, `.radar`, `.xyChart`, …) are `Option<…Tokens>` fields with their own typed structs, filled from the defaults and the overrides as the reference merges them.

Each diagram's `Section` is the schema-derived section (`MermaidConfigPie` and so on). Where the schema does not name what the reference reads (the treemap section, packet's `bitsPerRow` as a number from any source), the section gets a handwritten type next to the generated ones, and the untyped reads go away.

Nothing below the facade can write to any of these; a renderer's derived values are its own locals.

## What it checks

- A token or option a renderer reads exists, by type, at compile time; a misspelling does not compile.
- The output stays byte for byte the same at every step: every `pnpm port:diff` run, the geometry of every type, and the byte comparison of all `.mmd` files under three configurations and the colour schemes that steps 1 and 2 used.
- Once a renderer is migrated, it no longer imports `variables::text_of` and friends; a test fails if a migrated module does.

## How we get there

1. Generate `ThemeTokens` and the diagram token groups, and build them in the facade from the variables `Value` (one conversion per theme and override set, cached with the theme). Keep `Themes` beside it until the last renderer moves.
2. Inventory the global reads (`grep` over `input.config.` outside the type's own section) and fill `Globals`.
3. Migrate renderer by renderer, in parallel tasks: the custom renderers first, then the unified renderer's shapes and pipeline, which several types share.
4. Remove `Themes`, `variables::{text_of, …}` and the whole-config fields from the inputs.
5. The P3 renderers are written on `Context` from the start.

## Decided

1. **One generic `Context<'a, Model, Section>`** for every renderer, in place of a struct per renderer: one shape for 33 types, which the P3 renderers learn once.
2. **`ThemeTokens` replaces the `Value` map entirely** (step 4). The renderers name every token they write, so nothing needs the map's iteration order; the map stays only at the boundary where `themeVariables` arrive.
3. **The order of the work** as above: the tokens and the globals first (model and facade), then the renderers in parallel tasks that each keep the output byte for byte the same, then the removal of `Themes` and the lookups by name.
