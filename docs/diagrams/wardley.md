# Wardley map

A Wardley map plots components of a value chain on two axes, visibility (up) and evolution (right, in stages from genesis to commodity), with links between them, pipelines, evolution trends, annotations, notes and accelerators. The keyword is `wardley-beta`. ferrigura reads it with a hand-written scanner and parser of the Langium grammar (`ferrigura-syntax`, `src/wardley`) and draws it with a renderer of its own (`ferrigura-render`, `src/wardley`); there is no layout engine, a position is the projection of the statement's coordinates onto the plot. The configuration section is `wardley-beta`, named after the keyword.

## Model

`ferrigura_model::wardley::WardleyDiagram` is the DB state after the parse (`getWardleyData`), version 1:

| Field                                     | Holds                                                                                                                                                                                                        |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `title`, `accTitle`, `accDescr`           | The header texts, sanitized as the common setters do, with the entity placeholders kept; `""` when absent                                                                                                    |
| `size`                                    | `{ width, height }` from the `size` statement, `null` when the text gives none                                                                                                                               |
| `axes`                                    | `{ stages, stageBoundaries }`: the stage names (`Name` or `Name / Second`) and the `@` boundaries, both `null` without an `evolution` statement; `stageBoundaries` is `[]` for an evolution with no boundary |
| `nodes`                                   | Anchors, components and pipeline components, in the DB's order: the first insertion of each id, with the last values                                                                                         |
| `links`                                   | `{ source, target, dashed, label, flow }`: the ids the DB resolves the ends to, in text order                                                                                                                |
| `trends`                                  | `{ nodeId, targetX, targetY }`, one per node, in the order the node first got one                                                                                                                            |
| `pipelines`                               | `{ nodeId, componentIds }`, one per parent, with the components of its last block                                                                                                                            |
| `annotations`                             | `{ number, coordinates, text }`                                                                                                                                                                              |
| `notes`, `accelerators`, `deaccelerators` | Text or name with `x` and `y`, in text order                                                                                                                                                                 |
| `annotationsBox`                          | The position of the box, from the first `annotations` statement; `null` when there is none                                                                                                                   |

A node is `{ id, label, x, y, className, labelOffsetX, labelOffsetY, inertia, sourceStrategy, inPipeline, isPipelineParent }`. The model keeps the DB's quirks:

- **Coordinates are the DB's.** `x` is the evolution and `y` the visibility, both percentages, though the statements write the visibility first (`component A [visibility, evolution]`). An annotation, an accelerator and the annotations box are `[visibility, evolution]` in the text and `x`, `y` in the model. A value above 1 is not scaled: `[1.5, 1.0]` is `y` 1.5 and `x` 100.
- **A pipeline component's id is `Parent_Name`,** its label is its name, and its `y` is the parent's.
- **`null` is the DB's `undefined`.** `inertia` is `false` for a component without the word and `null` for an anchor or a pipeline component; the label offsets, `sourceStrategy`, `inPipeline` and `isPipelineParent` are `null` when unset.
- **A repeated component** is one node at the first position with the last values, except that a repeat keeps a label offset it is not given and a class. `inertia` and `sourceStrategy` are overwritten even by nothing.
- **A second pipeline of one parent** replaces the first's components; both sets of component nodes exist.
- **A link may name nodes that do not exist** (kept, no error), and an `evolve` of an unknown node makes no trend. A link end is resolved by id first, then by the first node whose label equals the name, which is how a pipeline component is found.
- **Non-finite numbers** are the sentinel strings `"NaN"`, `"Infinity"` and `"-Infinity"`; only an integer or a decimal of more than 308 digits gives one (a label offset, the annotation number, the size, a stage boundary).

Left out are the layout, the renderer's theme fallbacks, the configuration, and the axis labels, which no statement sets (they are always `Evolution` and `Visibility`). The conventions of the whole family are in [canonical-model.md](../canonical-model.md#wardley).

## Parsing

`wardley::parse` runs the steps of `Diagram.fromText`: it prepares the text (front matter and its title), removes directives and comment lines, encodes entities and adds a line break. The grammar gives the statements, and `populate` makes the DB calls in the order of `wardleyParser.ts`: the header texts, the size, the evolution, anchors, components, notes, pipelines, links, evolves, the annotations box, the annotations, the accelerators and the deaccelerators. The order matters: components are all added before pipelines and evolves, so an `evolve` may name a component declared later, and a link may name a pipeline component by its label. The first failing call is the error, with the reference's wording ([0007](../decisions/0007-hand-written-parsers.md)).

The scanner runs the terminals of the Chevrotain lexer in the order Langium builds them, and the first that matches wins, not the longest: the punctuation, decimals (`WARDLEY_NUMBER`), the link tokens, the strategies, the fourteen keywords, `NAME_WITH_SPACES`, blanks, the header terminals, `INT`, `STRING`, `ID`, line breaks, and the hidden comments and directives. The `-` punctuation is the one token with longer alternatives (`->`, `-->`, `-.->` and a `---` block), which are tried first.

The quirks that ferrigura keeps because the reference has them, each pinned by a test in `wardley_grammar.rs` and `wardley_db_spec.rs`:

- **Keywords and strategies are prefixes.** They come before names in the lexer, so `marketing`, `buyer`, `notes` and `evolutionary` are rejected as names; `titlebar` and `accTitlex` are names, because the name's lookahead needs a blank after `title`.
- **`INT` comes before `ID`,** so a name that starts with a digit is rejected. Non-ASCII letters are a lexical error. A `-` in a name stays unless it is followed by `>`.
- **Detection is case-insensitive, the parse is not.** `Wardley-Beta` is detected (`^\s*wardley-beta` with the `i` flag) and then rejected by the grammar. `wardley-betax` has no word end and is rejected.
- **Numbers.** Coordinates and boundaries are decimals (`[0, 0.5]` and `@1` are rejected, `@1.0` is 1); the `size` and the label offsets are integers; the coordinates of `annotations` and `annotation` also take integers. An evolution needs at least two stages, and a boundary comes before the second name.
- **The order of a component's suffix is fixed:** label, decorator, then `inertia` or `(inertia)`.
- **Links.** `->`, `-->`, `-.->`, `>`, `+<`, `+>`, `+<>`, `+'label'>`, `+'label'<` and `+'label'<>` are arrows; `--` and `.-.` are not (the `.-.` branch of the DB's dashed test is dead). `A +> B ;` is a lexical error because `LINK_LABEL` needs a character. A link is dashed because the reference tests `includes('-.->')` on the arrow's text, and its flow is backward when the arrow's text contains `<`, so `A +'a<b'> B` gets a start marker.
- **Titles.** The last `title` wins, and an empty last title lets the front matter title stand. The header may share its line with a statement.

## Layout and rendering

The renderer ports `wardleyRenderer.ts` (`draw`). `mod.rs` writes the root, the three markers and the groups in the reference's order, `plot.rs` holds the projection and the node placements, `axes.rs` the background, title, axes, stages and grid, `pipelines.rs` the boxes and the move of the parents, `links.rs` the links and the trends, `nodes.rs` the node groups, `annotations.rs` the annotations and their box, `marks.rs` the notes and the accelerators, `text.rs` the labels and the measure of the annotations box, and `theme.rs` the colors.

- **Projection.** The plot spans `padding` to `width - padding` and `height - padding`; `projectX(v) = padding + v/100 * chartWidth` and `projectY(v) = height - padding - v/100 * chartHeight`. The canvas is the `size` statement's, else the configuration's (`width` 900, `height` 600, `padding` 48).
- **Pipeline parents move** before the links, the trends and the nodes are drawn: the parent's `x` becomes the middle of its components and its `y` the top of the pipeline box minus a sixth of the square, so everything after uses the moved position. A trend still ends at the DB's `y`.
- **Links** are shortened by the radius at each end, or the square's half diagonal for a pipeline parent. A link between coincident ends divides by zero and writes `NaN`, as the reference does; the label is rotated by `atan2` and turned over outside plus or minus 90 degrees. Links from a pipeline component to its parent, and links to a name with no node, are left out.
- **Text.** Only the annotations box measures text: its width is the widest line plus 20 px of padding and a fixed 105 px, its height is `16` per line plus 20 plus half the tallest line, and it is clamped to the plot's margins. Every other label is placed without its width, so overlaps are not avoided. The box measures each line as the DOM holds it, with the entity placeholders; the font subset takes the decoded characters.
- **No stylesheet.** The reference removes every child of the root before it draws, which removes its stylesheet, so the attributes are the appearance (`6 6` dashes, the overlay fills, the anchor labels in `#000`). ferrigura writes no rules either, only the `@font-face` of the embedded faces, and names the family on `g.wardley-map`.
- **Colors.** The `wardley` group of the theme variables has the keys the reference reads (`WardleyTokens`), with the fallbacks of `getTheme`: another theme token, then a constant. The redux themes give no group, so a light map takes white nodes with black strokes and the dark one a `#333` background with the same white nodes. `colorScheme: auto` draws the light tokens only, as the map has no CSS.
- **Markers** carry the diagram's id (`arrow-<id>`, `link-arrow-end-<id>`, `link-arrow-start-<id>`), which keeps several maps on one page apart.
- **Math.** `atan2`, `cos` and `sin` come from `ferrigura-jsmath`, which gives V8's results to the last bit ([0012](../decisions/0012-layout-engines-ported-exactly.md)).

## Differences from Mermaid

- The text is drawn in the bundled face ([0010](../decisions/0010-text-measurement-and-fonts.md)). The reference sets no font for the map, so its text takes the page's default face; a viewer of ferrigura's output sees the bundled face where Mermaid would show the page's default. The harness pins the reference's default to the bundled face for this type, so both sides measure the same text.
- Labels are SVG text ([0009](../decisions/0009-svg-text-labels.md)); the reference's wardley code writes SVG text too.
- The `wardley-beta` section applies to the diagram. The pinned Mermaid ignored a theme, look or layout set under it and stripped the section from front matter and directives; patch 03 of the reference connects the type to its section, and ferrigura reads it as the patched reference does ([0014](../decisions/0014-upstream-reference-at-a-pin.md)).

## Known gaps

None known: the parser, the DB and the renderer cover every statement of the grammar.

## Checking it

- `pnpm -C tools/port port:diff wardley` compares the canonical models of the fixtures, the string literals of Mermaid's specs, the documentation and the cases of `tools/port/cases.ts`. `pnpm -C tools/port port:diff config` compares the `wardley-beta` section.
- `pnpm -C tools/port port:geometry wardley` compares the geometry of the fixtures and of the probes under `tools/port/geometry/probes/wardley/` (accelerators, decorators and inertia, link flows, stage names and boundaries, canvas and grid, annotation edge cases, edges and trends, pipelines with trends and on anchors, titles and entities, a repeated component, defaults). The comparison reports no numeric difference. The harness forces every fixture's theme to a shipped one, so the `wardley` tokens of Mermaid's other themes are never exercised.
- Rust specs: `wardley_spec` (the port of Mermaid's spec), `wardley_grammar` (one test per lexer and grammar quirk), `wardley_db_spec` (the DB quirks) in `ferrigura-syntax`; `wardley_render` in `ferrigura-render` (one test per renderer rule: the pipeline parent's move, the `NaN` link, the label turned over, the box width and clamping, the placeholders, the markers).

## Learnings

- **The reference's stylesheet never reaches the output.** `draw` selects the root and removes all its children, the `<style>` mermaid inserted first among them. Had it survived, `.wardley-link--dashed` would override the attribute `6 6`, `.wardley-node circle` would paint the sourcing overlays with the component fill, and `.wardley-node-label` would recolor the anchor labels. Emitting CSS for this type would change the picture.
- **Fonts decide the annotations box.** The reference measures in the page's default face (Times in the harness): `1. Critical component` is 95.625 px there and 101.75 px in Lexica Ultralegible, so the box width differs by the face and the pinned page matters for the comparison.
- **The `y` of a trend is the DB's, not the moved parent's.** The pipeline step moves a parent before the trends are drawn, but the trend's end keeps the node's visibility; a trend into its own position falls back to the target at zero length and stays finite.
- **`-0` and shortest round-trip numbers.** d3 writes `String(value)`: `12.600000000000001` for the title size, `-0` as `0`, and `NaN` as `NaN`. A negative zero printed as `-0` would differ from the reference.
- **The accelerator's path has line breaks and indentation** in the reference (a template literal). An SVG parser ignores them, and ferrigura keeps them so that the output is the same text.
- **The DOM order is the order of the `append` calls,** and `insert('rect', 'text')` puts the annotations box's rectangle before its first text. An annotations box with no annotation text leaves an empty `g.wardley-annotations-box`.
- **Entities.** `encodeEntities` runs before the parse and the whole SVG is decoded afterwards: `Cost #35; here` shows `Cost # here`, `R&amp;D` shows literally, and `a <b>x</b>` in a note is shown as typed because d3's `text` does not parse markup.
