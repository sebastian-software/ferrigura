# ferrigura

ferrigura renders Mermaid diagram text to SVG in Rust, without a browser. It is a port of Mermaid 12.2.0: the same diagram text gives the same diagram, and the geometry of the output is checked against Mermaid run in Chromium.

The crates are not published yet. The repository is [sebastian-software/ferrigura](https://github.com/sebastian-software/ferrigura).

## Installation

Once the crates are published, add the library with Cargo:

```sh
cargo add ferrigura ferrigura-model serde_json
```

`ferrigura` renders. `ferrigura-model` provides `Value`, the type of the configuration. `serde_json` is needed only for the examples that build a configuration from JSON. The minimum Rust version is 1.95.

## Quick start

```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use ferrigura_model::value::Value;

let text = "pie title Pets\n  \"Dogs\" : 386\n  \"Cats\" : 85\n";
let svg = ferrigura::render(text, &Value::empty_object())?;
assert!(svg.starts_with("<svg "));
# Ok(())
# }
```

`render` takes the diagram text and the configuration, and returns the SVG document as a `String`. An empty object as the configuration gives the defaults.

## Reusing a renderer

`render` keeps a small cache of renderers, keyed by the configuration, so a configuration that is rendered again is resolved once. To render many diagrams with one configuration, make a `Renderer`, which resolves the configuration when it is made:

```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use ferrigura::Renderer;
use ferrigura_model::value::Value;

let renderer = Renderer::new(&Value::empty_object());
let pie = renderer.render("pie\n  \"Dogs\" : 386\n  \"Cats\" : 85\n")?;
let flowchart = renderer.render("flowchart LR\n  A[Start] --> B[End]\n")?;
assert!(pie.starts_with("<svg "));
assert!(flowchart.starts_with("<svg "));
# Ok(())
# }
```

A `Renderer` is `Send` and `Sync`, so one renderer can serve renders on several threads at once. For the same configuration, its output is the output of `render`, byte for byte.

## Options

`render_with` and `Renderer::render_with` take a `RenderOptions` as well. It holds what is not Mermaid configuration:

| Field          | Type                  | Meaning                                                                                                                                                                                                                                        |
| -------------- | --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `id`           | `Option<String>`      | The id of the SVG element, which scopes its styles. It must start with an ASCII letter and contain only ASCII letters, digits, `-` and `_`. By default it is derived from the text and the configuration, so the same input gives the same id. |
| `color_scheme` | `Option<ColorScheme>` | `Light`, `Dark` or `Auto`. `Auto` writes both colour sets, and the dark one applies under `prefers-color-scheme: dark`. `None` keeps the colours of the configured theme.                                                                      |
| `now`          | `Option<i64>`         | The current time in milliseconds since the epoch, for the dates that depend on it: the today marker of a gantt chart and the dates without a day. `None` reads the system clock.                                                               |

```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use ferrigura::{ColorScheme, RenderOptions, Renderer};
use ferrigura_model::value::Value;

let renderer = Renderer::new(&Value::empty_object());
let svg = renderer.render_with(
    "pie title Pets\n  \"Dogs\" : 386\n  \"Cats\" : 85\n",
    &RenderOptions {
        id: Some("pets".to_owned()),
        color_scheme: Some(ColorScheme::Auto),
        ..RenderOptions::default()
    },
)?;
assert!(svg.contains("id=\"pets\""));
# Ok(())
# }
```

## Errors

The functions return `ferrigura::Error`, which implements `std::error::Error`. An error is returned for text that is not a diagram or does not parse, for a configuration value of the wrong type, for an option that is not valid (such as an `id` that is not a CSS identifier), and for a bundled font that cannot be read.

## Configuration

The configuration is Mermaid's `initialize` object, as a `Value`. A diagram can add to it with front matter or with directives, as in Mermaid:

```text
---
config:
  themeVariables:
    pie1: "#00ff00"
---
pie
  "Dogs" : 386
```

```text
%%{init: {"themeVariables": {"pie1": "#00ff00"}}}%%
pie
  "Dogs" : 386
```

Building the configuration in Rust:

```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use ferrigura_model::value::Value;

let config = Value::from(serde_json::json!({
    "themeVariables": { "pie1": "#00ff00" }
}));
let svg = ferrigura::render("pie\n  \"Dogs\" : 386\n", &config)?;
assert!(svg.contains("#00ff00"));
# Ok(())
# }
```

Two themes ship: `redux-color` (light) and `redux-dark-color` (dark), which are Mermaid's current defaults. The `theme` setting takes either name. Any other name gives one of the two: a name that ends in `dark` gives the dark theme, and any other name gives the light one. `themeVariables` override the colour values of the theme.

## Diagram types

ferrigura renders 33 diagram types. The header keyword of each is the one Mermaid uses:

| Diagram        | Header                                                                                               |
| -------------- | ---------------------------------------------------------------------------------------------------- |
| Flowchart      | `flowchart`, `graph`                                                                                 |
| Sequence       | `sequenceDiagram`                                                                                    |
| Class          | `classDiagram`, `classDiagram-v2`                                                                    |
| State          | `stateDiagram`, `stateDiagram-v2`                                                                    |
| ER             | `erDiagram`                                                                                          |
| Gantt          | `gantt`                                                                                              |
| Pie            | `pie`                                                                                                |
| Timeline       | `timeline`                                                                                           |
| gitGraph       | `gitGraph`                                                                                           |
| User journey   | `journey`                                                                                            |
| Info           | `info`                                                                                               |
| Mindmap        | `mindmap`                                                                                            |
| Block          | `block`                                                                                              |
| Packet         | `packet`                                                                                             |
| XY chart       | `xychart`                                                                                            |
| Sankey         | `sankey`                                                                                             |
| Quadrant chart | `quadrantChart`                                                                                      |
| Requirement    | `requirementDiagram`                                                                                 |
| C4             | `C4Context`, `C4Container`, `C4Component`, `C4Dynamic`, `C4Deployment`                               |
| Kanban         | `kanban`                                                                                             |
| Architecture   | `architecture`, `architecture-beta`                                                                  |
| Treemap        | `treemap`, `treemap-beta`                                                                            |
| Ishikawa       | `ishikawa`, `ishikawa-beta`, in any letter case                                                      |
| Event modeling | `eventmodeling`                                                                                      |
| Agentflow      | `agentflow-beta`                                                                                     |
| Cynefin        | `cynefin-beta`                                                                                       |
| Radar          | `radar-beta`                                                                                         |
| Railroad       | `railroad-beta`, `railroad-ebnf-beta`, `railroad-abnf-beta`, `railroad-peg-beta`, in any letter case |
| Swimlane       | `swimlane-beta`                                                                                      |
| Tree view      | `treeView-beta`                                                                                      |
| Use case       | `usecase-beta`                                                                                       |
| Venn           | `venn-beta`                                                                                          |
| Wardley map    | `wardley-beta`, in any letter case                                                                   |

## How the output differs from Mermaid

- Labels are SVG `text` and `tspan` elements, not HTML in `foreignObject`. Markdown in labels (bold, italic, code and line breaks) is written as styled text runs. `htmlLabels` is accepted, and labels are drawn as SVG text either way.
- Text is measured in bundled fonts: Lexica Ultralegible (regular, italic, bold and bold italic) and Atkinson Hyperlegible Mono for code. The SVG embeds the glyphs that it draws as subsets of these faces, in `@font-face` rules with data URIs, so a viewer draws the text that was measured.
- Characters that the text faces lack are set in bundled symbol faces: the arrows (U+2190 to U+21FF), the text-default symbols of U+2600 to U+27BF that the symbol face covers, and the emoji with emoji presentation.
- Colours are CSS custom properties, declared on the diagram's own id. Each element also carries its concrete colour as a presentation attribute, for viewers that do not read custom properties.
- Each diagram's styles are scoped to its id, so several SVG diagrams can share one page.
- There is no browser and no JavaScript at run time. Interactive parts of Mermaid, such as click callbacks and the `loose` and `sandbox` security levels, have no meaning here.
- The output is SVG only.
- The library does not print warnings. A fallback, such as one of the layout fallbacks below, is silent.

Layouts: flowcharts, class, state, ER, requirement, agentflow and use case diagrams use ELK Layered. Mindmaps use cose-bilkent by default, and Mermaid's tidy tree with `layout: tidy-tree`. Architecture diagrams use fcose. Venn diagrams use the layout of venn.js 2.0.0, and swimlanes have their own layout.

The geometry of the output (node and label boxes, edge paths and the viewBox) is compared with Mermaid 12.2.0 run in Chromium, on macOS. It agrees within 0.05 px on every fixture that the comparison covers. The comparison leaves out a fixture only for a stated reason, such as `look: handDrawn`, a layout algorithm other than ELK Layered, or a character that no bundled face covers.

## Not supported yet

- `look: handDrawn`. Shapes take the `classic` geometry as plain SVG elements, without the hand-drawn strokes.
- Math in labels (`$$...$$`, KaTeX). It is not typeset.
- Mermaid's classic themes (`default`, `dark`, `forest` and `neutral`). A theme name that is not one of the two shipped themes gives one of them, as described above.
- Text in CJK, kana and Arabic, and emoji that the bundled faces do not cover. There is no fallback font yet.
- Images and icon packs. In flowcharts an image is not read, so an image shape needs `assetWidth` and `assetHeight`. Icon packs are not supported.
- ELK layout algorithms other than `elk.layered`. A diagram that names another one is laid out with `elk.layered`.
- Swimlane diagrams with `ignoreCrossLaneEdges: false` or `automaticLaneOrdering: true`. The render fails with an error that names the option.

## Licences

The crates are MIT-licensed, as Mermaid is: ferrigura ports Mermaid 12.2.0, and the licence text is the `LICENSE` file of the repository. `ferrigura-elk`, which ports ELK, is licensed under EPL-2.0. Some crates port code or bundle fonts under the licences of their sources, which their `NOTICE.md` files list: `ferrigura-cose`, `ferrigura-elk`, `ferrigura-jsmath`, `ferrigura-path`, `ferrigura-swimlane`, `ferrigura-text` and `ferrigura-venn`.

The bundled fonts are under the SIL Open Font License 1.1: Lexica Ultralegible, Atkinson Hyperlegible Mono, Noto Sans Math (the arrows) and Noto Sans Symbols 2. The emoji font is a cut of Twemoji Mozilla 0.7.0, whose code is under Apache-2.0. Its art is under the Creative Commons Attribution 4.0 licence. The emoji art comes from Twemoji (<https://twitter.github.io/twemoji>), used and redistributed under that licence. The font holds only the 1,185 emoji with emoji presentation that the source covers, and its hinting is dropped. The file `crates/ferrigura-text/NOTICE.md` has the details.
