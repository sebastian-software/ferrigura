# Wardley (P3): the parser and the DB

Status: the parser, the DB and the model are ported. The differential run `pnpm port:diff wardley` shows no difference on its 222 inputs (167 accepted by the reference, 55 rejected by both), and `pnpm port:diff config` passes with the `wardley-beta` section compared. The renderer is ported (section 9): `pnpm port:geometry wardley` is within tolerance on its 10 fixtures and 12 probes. The research is in [NOTES.md](./NOTES.md), sections 6 to 7, and is the source of the rules here, with its probes; [P3-PLAN.md](../P3-PLAN.md) and [P2-PARSERS.md](../P2-PARSERS.md) give the rules of the task.

## 1. Scope

| Part           | Where                                                                             | Status                                                         |
| -------------- | --------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| Model          | `crates/ferrigura-model/src/wardley.rs`                                           | ported (section 3)                                             |
| Lexer          | `crates/ferrigura-syntax/src/wardley/scanner.rs`                                  | ported, in the 43-token order of NOTES section 2.2             |
| Grammar        | `crates/ferrigura-syntax/src/wardley/grammar.rs`                                  | ported, recursive descent over the rules of NOTES section 2.3  |
| DB and builder | `crates/ferrigura-syntax/src/wardley/db.rs`, `mod.rs` (`populate`)                | ported, in the calls of NOTES section 3.1                      |
| Exporter       | `tools/port/export-wardley.ts`                                                    | written; runs the reference's parser and DB (`getWardleyData`) |
| Specs          | `crates/ferrigura-syntax/tests/wardley_{spec,grammar,db_spec}.rs`, `db.rs` (unit) | 20 + 29 + 28 integration tests, 4 unit tests                   |
| Renderer       | `crates/ferrigura-render/src/wardley/`                                            | ported (section 9)                                             |

## 2. The parse

The steps are those of `Diagram.fromText`, as in the architecture port: `prepare` (front matter, CR and CRLF, the quoting of HTML attributes), `remove_directives` and `cleanup_comments`, then `encode_entities` and a line break. The grammar gives the statements with their offsets, and `populate` makes the DB calls in the order of `wardleyParser.ts`: the header texts, the size, the evolution, anchors, components, notes, pipelines, links, evolves, the annotations box, the annotations, the accelerators and the deaccelerators. The first failing call is the error.

Two decisions of the port:

- **Detection is case-insensitive and the parse is not.** `canonical.rs` detects the header with `/^\s*wardley-beta/i` first, as `detectType` does, so `Wardley-Beta` is detected and then rejected by the grammar (quirk L4). The scanner keeps `KW_WARDLEY` case-sensitive.
- **Errors are ferrigura's.** Lexer and parser errors name the offending position; the DB's errors keep the reference's wording, since the messages are the same text a user sees. The differential compares acceptance only.

## 3. The model

`WardleyDiagram`, camelCase, as `canonical-model.md` ("Wardley") describes it:

- `version`, `title`, `accTitle`, `accDescr`: the DB's header texts, sanitised as the setters sanitise them, with the entity placeholders kept.
- `size`, `axes`, `annotationsBox`: `null` when the text gives none. `axes.stages` and `axes.stageBoundaries` are `null` without an `evolution`, and `stageBoundaries` is `[]` for an evolution with no boundary.
- `nodes` (the DB's node map, in the first-insertion order), `links`, `trends`, `pipelines`, `annotations`, `notes`, `accelerators`, `deaccelerators`: in the DB's order.
- Node fields: `className` (`anchor`, `component`, `pipeline-component`), `labelOffsetX/Y`, `inertia` (`false` for a component without the word, `null` for anchors and pipeline components), `sourceStrategy`, `inPipeline`, `isPipelineParent`.
- Link fields: `source`, `target` (the resolved ids), `dashed`, `label`, `flow`.
- The DB's `x` (evolution) and `y` (visibility) are kept as the DB holds them, not the statement's order (`Q8`).
- A non-finite number is its sentinel string (`Number` in `wardley.rs`). Only an integer or a decimal of more than 308 digits gives one: the label offsets, the annotation number, the size, and a stage boundary.

Left out: the layout (there is none), the renderer's stylesheet and theme fallbacks, the configuration (compared by `port:diff config`), and the axis labels, which no statement sets (`xLabel` and `yLabel` are always `Evolution` and `Visibility` in the renderer).

## 4. Reproduced quirks

Each quirk has its probe as a test. The first two groups are the lexer and the grammar (`wardley_grammar.rs`), the next two the DB (`wardley_db_spec.rs`, and `db.rs` for the builder), and the last the reference's specs.

| Quirk | Input (probe)                                            | Result                                                                                         | Test                                               |
| ----- | -------------------------------------------------------- | ---------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| L1    | `component marketing`, `buyer`, `builder`                | rejected: the strategy word is a prefix                                                        | `l1_*`                                             |
| L2    | `component notes`, `evolutionary`, `componentA`, `title` | rejected: a keyword is a prefix                                                                | `l2_*`                                             |
| L3    | `component 2fast`                                        | rejected: `INT` comes before `ID`                                                              | `l3_*`                                             |
| L4    | `Wardley-Beta`                                           | detected, then rejected                                                                        | `l4_*`, and the `Wardley-Beta` case of `detect.rs` |
| L5    | `wardley-betax`                                          | rejected: no word end on the header                                                            | `l5_*`                                             |
| L6    | `component titlebar`                                     | accepted as a name                                                                             | `l6_*`                                             |
| L7    | `component Size`                                         | accepted: keywords are case-sensitive                                                          | `l7_*`                                             |
| L8    | `component foo-`                                         | accepted: a trailing `-` is in the name                                                        | `l8_*`                                             |
| L9    | `A .-. B`, `A -- B`                                      | rejected: no such arrow                                                                        | `l9_*`                                             |
| L10   | `A +> B ;`                                               | lexer error: `LINK_LABEL` needs a character                                                    | `l10_*`                                            |
| L11   | `component A [0, 0.5]`, `[1, 0.5]`                       | rejected: coordinates are decimals                                                             | `l11_*`                                            |
| L12   | `size [1200.5, 900]`                                     | rejected: integers                                                                             | `l12_*`                                            |
| L13   | `evolution A@1 -> B`                                     | rejected; `@1.0` is 1                                                                          | `l13_*`                                            |
| L14   | `evolution A -> B`                                       | accepted; a single stage is rejected                                                           | `l14_*`                                            |
| L15   | `(inertia) (build)`, `(build) label [..]`                | rejected: the order is fixed                                                                   | `l15_*`                                            |
| L16   | `annotations [1, 0]`                                     | accepted: integers are coordinates                                                             | `l16_*`                                            |
| L17   | `title One`, `title Two`                                 | `Two`: the last value                                                                          | `l17_*`                                            |
| L18   | `title ` (blank)                                         | empty: the last line gives the empty title; the front matter title then applies                | `l18_*`, `the_front_matter_title_*`                |
| L19   | `"A\"B"`, `'Single'`                                     | `A"B`, `Single`                                                                                | `l19_*`                                            |
| L20   | `title 50%% done`                                        | `50`                                                                                           | `l20_*`                                            |
| L21   | `wardley-beta component A [..]`                          | accepted: a statement on the header line                                                       | `l21_*`                                            |
| L22   | CRLF, tabs, no final newline                             | accepted                                                                                       | `l22_*`                                            |
| L23   | `accTitle: …`, `accDescr {…}`                            | the common converter                                                                           | `l23_*`                                            |
| D1    | a repeated component                                     | one node at the first position, the last values; a repeat keeps a label offset it is not given | `d1_*`                                             |
| D2    | two pipelines of one parent                              | the second replaces the components, both sets of nodes exist                                   | `d2_*`                                             |
| D3    | `X -> Y`, neither declared                               | kept, no error                                                                                 | `d3_*`                                             |
| D4    | `evolve Nope 0.8`                                        | no trend                                                                                       | `d4_*`                                             |
| D5    | `evolve A` before `component A`                          | works                                                                                          | `d5_*`                                             |
| D6    | `Queue -> K`, a pipeline component                       | resolved by its label to `K_Queue`                                                             | `d6_*`                                             |
| D7    | `pipeline Nope { … }`                                    | error                                                                                          | `d7_*`                                             |
| D8    | two `annotations` blocks                                 | the first positions the box                                                                    | `d8_*`                                             |
| D9    | `component A [1.5, 1.0]`                                 | x 100, y 1.5: a value above 1 is not scaled                                                    | `d9_*`                                             |
| D10   | `A +'a<b'> B`                                            | backward: the `<` of the label is tested first                                                 | `d10_*`                                            |

Further quirks, read from the reference and probed in the differential rather than in a dedicated test: a `---` block in the body is hidden (the `-` punctuation's longer alternative), `A +'-.->'> B` is dashed because the reference tests `includes('-.->')` on the arrow's text, a `<>` inside a flow label gives a bidirectional flow, `+''>` gives an empty label (not none), a second-name of `""` is no second name, and a pipeline on an anchor is accepted. All of them are in `tools/port/cases.ts`.

## 5. The scanner

The lexer takes the first terminal that matches (Chevrotain, not longest match), in the order of NOTES section 2.2: the punctuation, `WARDLEY_NUMBER`, the link tokens, `LINK_LABEL`, `STRATEGY`, the fourteen keywords, `NAME_WITH_SPACES`, the blanks, the header terminals (`ACC_DESCR`, `ACC_TITLE`, `TITLE`), `INT`, `STRING`, `ID`, `NEWLINE`, and the hidden comments and directives. The `-` punctuation is the one token with longer alternatives (`->`, `-->`, `-.->` and a `---` block), so `dash_step` checks them first. The header matchers are the common rules of `common.langium`, and `colon_form`, `directive_len` and `yaml_len` are the shared lexer's (`lexer.rs`). `NAME_WITH_SPACES` is implemented as its greedy match, since nothing follows the pattern.

## 6. The grammar

Every statement ends with `EOL` (one or more line breaks, or the end). The header lines are `((accDescr | accTitle | title) EOL)`, and the entry accepts them between the statements. Each rule is a function of `grammar.rs`. A component's suffix is read with two tokens of lookahead: `(` followed by a strategy is the decorator, and `(` followed by `inertia` is the parenthesised inertia.

## 7. Shared files

None changed except `crates/ferrigura-syntax/examples/canonical.rs` (the `wardley` arm now detects first, as the architecture arm does) and the registries: `tools/port/status.ts` (the three suites), `tools/port/diff-config.ts` (`wardley-beta` is no longer in `NOT_PORTED_SECTIONS`), and `tools/port/cases.ts` (the probes, and the `Wardley` undetected case). `docs-port/canonical-model.md` has the Wardley paragraph.

## 8. Open questions

- **The header matchers are copies.** `scanner.rs` has its own `title`, `accTitle` and `accDescr` matchers, the same as `architecture/scanner.rs`. Lifting them into `common.rs` would remove the copy; it touches a shared file, so it waits for the coordinator's choice.
- **The `Number` sentinel** is in the model (`wardley.rs`), not in `value.rs`. The other models that need one (kanban, journey) keep their own, and a shared helper would serve all of them.

## 9. The renderer

The renderer is the reference's `draw` (NOTES section 6), a projection with no layout engine. It writes the root, `g.wardley-map` and the three markers in the reference's order (NOTES 6.3), and no stylesheet (Q2). The answers to NOTES 8.3:

| Question           | Decision                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Q1, the font       | The reference's text takes the page's default face, and so does its annotation box's measure, taken while the reference draws. Both sides are measured in the bundled face: ferrigura embeds the regular and bold faces (`@font-face`, the only style it writes) and names the family on `g.wardley-map`; the harness pins the reference's default to the same face for wardley only (`pageHtml` for the rule on the rendered SVG, and `renderReference`'s `pinFamily` while the reference draws, since the core sets the role after the draw). Users of the output see the bundled face where the reference shows the page's default.                                                                                                                                           |
| Q2, the stylesheet | Not written. `styles.ts` is not ported: the attributes are the appearance, as in the reference's output (NOTES 6.9).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| Q3, the fallbacks  | The `wardley` group of the theme variables is a token group (`WardleyTokens`, the last block of `tokens.rs`). The redux themes give none, so `wardley/theme.rs` uses the fallbacks of `getTheme`. `annotationStroke`, `annotationTextColor` and `annotationFill` are read by the reference but nothing it draws takes them, so they are not tokens.                                                                                                                                                                                                                                                                                                                                                                                                                              |
| Q4, the quirks     | All reproduced, each with a test in `crates/ferrigura-render/tests/wardley_render.rs`: the pipeline parent's move before the trends (the trend's end stays at the node's visibility), the `NaN` of a link between coincident ends, the finite trend into its own position, the label turned over at ±90 degrees, the `6 6` dash and the overlays' own fills (no stylesheet), the literal `#000` dividers, the +10 label offset with a strategy, the inertia offset, the annotation box (widest line plus 105 px, clamped to the margins, and an empty box group when no line has text), the `Infinity` number, the placeholders decoded after the escape, the markers named with the diagram's id, the links left out (a pipeline component to its parent, a name with no node). |
| Q5, the probes     | Twelve probes under `tools/port/geometry/probes/wardley`: the accelerators, the decorators and inertia, the flows, the stage names and boundaries, the canvas and grid, the annotations' edge cases, the edges and trends, the pipeline's trend, a pipeline on an anchor, the entities and `%%` in titles, a repeated component, and the defaults.                                                                                                                                                                                                                                                                                                                                                                                                                               |

The reference has no renderer spec (section 5), so the tests are the quirk tests. Each fixture and probe is bit-identical to the reference: `pnpm port:geometry wardley` reports no numeric difference on all 22.

Decisions the coordinator should know:

- The text of a map is drawn in the bundled face, not in the page's default face (Q1). That changes what a user sees for the wardley text, and it is the decision of `P3-PLAN.md`.
- The annotation box measures each line as the DOM holds it, with the placeholders of `encodeEntities`, as the reference measures it; the characters the font subset takes are the decoded ones. No other text is measured: the map's labels are placed without their widths, as in the reference.
- Colour schemes: the map has no CSS, so `colorScheme: auto` draws the light tokens only, as the reference's output does.
