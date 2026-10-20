# Typed views for the renderers (issue #4)

Step 3 of [decision 0009](./decisions/0009-configuration-resolved-once.md), issue [#4](https://github.com/sebastian-software/ferrigura/issues/4). This page is the contract between the facade and the renderers. Decided by Sebastian on 2026-10-19, with the three choices below, and built the same day: every renderer takes a `Context` (see [What is built](#what-is-built)).

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

## What is built

Steps 1 to 4, on 2026-10-19: the tokens and the globals with pie and info first, then the other 31 renderers in three parallel tasks (the unified renderer with its eight types; sequence, gantt, timeline, journey and gitGraph; the nine other custom renderers), then the removal of `Themes` and of the readings by name. Step 5 is the P3 renderers'.

**Tokens** (`ferrigura_model::theme::tokens`). `Token` holds the value the map held under a name, or nothing: `text()`, `texts()`, `truthy()` and `px()` are the four readings the renderers made by name before, and `value()` gives the value itself. `Family<N>` holds the indexed names (`get(index)`, `name(index)`, `entries()`). `ThemeTokens::from_value(&Value)` builds the struct, `ThemeTokens::NAMES` lists the names and `entries()` gives every token with its name. `XyChartTokens` and `PacketTokens` are the groups. One macro, `token_struct!`, writes each struct from one list of `field = "jsName": token | family(N, first) | group(Type)`.

- The list has 190 fields in `ThemeTokens`: 175 variables, 13 families holding 112 members, and the groups `xyChart` (13 names) and `packet` (10 names). That is 287 top-level tokens and 310 with the groups.
- Source of the names: the 271 keys that `redux-color` and `redux-dark-color` produce, with the `xyChart` object, and the names the renderers read, found by recording every `Value::get` key over the workspace tests and the e2e corpus (`darkMode`, `rowEven`, `rowOdd`, and the fourteen `em*` event-modeling variables, which no theme produces). Renderer source names (the `COLOUR_TOKENS` lists) are all among them. Not included: `radar` and `treeView`, which no renderer reads, and the `elk.*` and `nodeSize.*` options, which are not theme variables.
- Families: `pie` 1 to 12, `venn` 1 to 8, `border` 1 to 2; `cScale`, `cScaleInv`, `cScaleLabel`, `cScalePeer` 0 to 11; `fillType`, `git`, `gitBranchLabel`, `gitInv` 0 to 7; `surface`, `surfacePeer` 0 to 4. A family keeps the members that the user's overrides set outside its slots, so `get(index)` reads every index the map holds.
- The equivalence test (`ferrigura-render`, `variables::tests`) compares the four readings of every token, of every family index from 0 to 29 and of the two groups, with the readings by name on the same map. It covers both redux themes with no overrides, and 400 generated `themeVariables` maps with values of every kind (colours, numbers, flags, lists, nested objects, wrong types) and names inside and outside the families, as well as the overrides themselves. The readings by name (`text_of`, `texts_of`, `truthy`, `px`) are compiled for this test only, so no renderer can call them.

**Contexts** (`ferrigura_render::context`). `Context<'a, Model, Section>` carries the diagram, the section, the globals, a `ThemeView` (`light`, and `dark` for `colorScheme: auto`) and the id. `Globals::of(&MermaidConfig)` has seven fields: `look`, `theme`, `font_family`, `font_size`, `security_level`, `layout` (as the view resolves it) and `html_labels`. They are the top-level options that renderers read. `use_max_width` is read from each diagram's own section, so it is not a global.

**Sections.** Each diagram type's section is what its renderer reads:

| Diagram types                                                                                                           | Section                                                                                                                                                                                                           |
| ----------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| pie, timeline, journey, gitGraph, sequence, xychart, quadrant chart, sankey, C4, architecture, ishikawa, event modeling | The schema's section (`PieDiagramConfig` and so on)                                                                                                                                                               |
| info                                                                                                                    | `()`                                                                                                                                                                                                              |
| flowchart, class, state, ER, requirement                                                                                | `unified::Section`: the `flowchart`, `elk`, `class`, `er` and `state` sections, which the unified renderer reads across the types (class and requirement take the state section's title margin and `useMaxWidth`) |
| mindmap, block                                                                                                          | `MindmapSection`, `BlockSection`: the type's section and `unified::Section`                                                                                                                                       |
| kanban                                                                                                                  | `KanbanSections`: the `kanban` and the `mindmap` section, whose `padding` and `useMaxWidth` the viewBox takes                                                                                                     |
| gantt                                                                                                                   | `GanttSection`: the `gantt` section and the clock (`RenderOptions::now`)                                                                                                                                          |
| packet                                                                                                                  | `PacketSection`: the `packet` section and `bitsPerRow` as the reference's arithmetic reads it                                                                                                                     |
| treemap                                                                                                                 | `TreemapSettings`, the handwritten settings                                                                                                                                                                       |

Inside the unified renderer, the pipeline, the shapes, the clusters, the palette and the ELK glue take `unified::Config`: the section, the globals and the light tokens.

**Facade** (`crates/ferrigura/src/lib.rs`). `Renderer::new` builds the tokens of both schemes, and `View::resolve` builds the configured tokens and the globals with the view; the maps of variables are not kept. A directive that changes the overrides builds its own tokens. `Prepared::theme()` gives the `ThemeView` of the colour scheme.

**How a renderer reads.** Its input is `&Context<'_, Model, Section>`. It reads `input.theme.light.<field>` with `text()`, `px()`, `truthy()`, `texts()` or `value()`, a family member with `get(index)`, `input.theme.dark` for the second scheme, and the globals from `input.globals`. `common::style::base_style_sheet` takes the `ThemeView`. The test `context::tests::no_renderer_takes_the_whole_configuration` fails when a file below `src` names `MermaidConfig` outside its tests, except for the two constructors that the facade calls (`Globals::of`, `unified::Section::of`).

**Against the contract.**

- The contract said "about 150" tokens. The two redux themes produce 271 names (with the `xyChart` object), and the renderers read 17 more names that no theme produces, so the list has 287 top-level tokens.
- `Globals` has no `use_max_width` and no `hand_drawn_seed`: no renderer reads them from the top level (`hand_drawn_seed` is not read at all). The `look`, `theme`, `security_level` and `layout` fields are the configuration's strings, not enums, so that the reads keep their comparisons.
- The untyped reads moved, rather than going away: no renderer makes them, but the facade does, from the effective configuration as before, when it builds the packet and treemap sections and the sequence's parse option (`wrap`). The values are the reference's for a value of the wrong type too (a `bitsPerRow` that is not a number gives `NaN`).
- A few reads take the value itself (`Token::value()`), where the reference reads it unlike the four readings: the xychart's colours and the class notes' colours take strings only, the C4 radius takes numbers only, and the shapes' `theme_number` and `theme_flag` read as `parseFloat` and without the truthiness of lists and objects.
- `unified::edges::label` computed its font size from the user's `themeVariables`; no render called it, since the pipeline passes the tokens' `fontSize`. It takes the size now, and no `Globals` field holds the overrides.

**Byte comparison** (2026-10-19). The corpus is the 2336 `.mmd` files under `e2e/diagrams` and `tools/port/geometry/probes`, each rendered by the release `render` example under three configurations (none; a `themeVariables` override with family members, tokens, both groups and wrong types; `theme: redux-dark-color`) and four colour schemes (none, `light`, `dark`, `auto`): 28 032 renders, with the clock fixed by `--now`, since the gantt today marker depends on it. The base and the change give the same bytes for all of them at every step: the foundation, each commit of the three migration branches, the merged branch, and the removal.

Before the pin, the foundation differed from the base in 96 renders: the last digits of some points of the braces shapes (`braces`, `curlyBraces`), once `generate_circle_points` in `unified/shapes/util.rs` was no longer inlined. The helper is marked `#[inline]`, which restores the base bytes; nothing else in the shape code changed. The reason the optimiser's inlining decision moved was not found. The corpus comparison is the check that catches a repeat.

The pin is gone since 2026-10-19: the renderer takes its transcendental functions from `ferrigura_jsmath` (STATUS, browser arithmetic), so no call to the platform's libm is left for the optimiser to fold or combine. The debug and the release build of that change give the same bytes for all 28 032 corpus renders; the two builds of the base differ in 12 (the pie with a long title, in the last digit of a slice label's `y`).

## Decided

1. **One generic `Context<'a, Model, Section>`** for every renderer, in place of a struct per renderer: one shape for 33 types, which the P3 renderers learn once.
2. **`ThemeTokens` replaces the `Value` map entirely** (step 4). The renderers name every token they write, so nothing needs the map's iteration order; the map stays only at the boundary where `themeVariables` arrive.
3. **The order of the work** as above: the tokens and the globals first (model and facade), then the renderers in parallel tasks that each keep the output byte for byte the same, then the removal of `Themes` and the lookups by name.
