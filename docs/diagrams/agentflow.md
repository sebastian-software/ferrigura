# Agentflow

An agentflow diagram draws an agent's workflow: tasks, tools, decisions and inputs as nodes of their own shapes, arrows between them with a meaning (sequence, failure, reference), `flow` containers that group nodes and can be collapsed, and connectors that join branches. Its keyword is `agentflow-beta`, followed by an optional direction. The parser is the Jison grammar of `agentflow.jison` and the diagram DB of `agentflowDb.ts`, ported to `ferrigura-syntax` (`src/agentflow.rs` and `src/agentflow/`); the lexer is the flowchart's scanner with agentflow's rules. The renderer in `ferrigura-render` (`src/agentflow/`) is the [unified renderer](./flowchart.md) with agentflow's own layout data, color palette and style sheet, and it lays out with ELK Layered like the flowchart.

## Model

The canonical model is the diagram DB's state after the parse ([canonical-model.md](../canonical-model.md#agentflow), `ferrigura-model/src/agentflow.rs`). Optional keys are left out when the reference's value is `undefined`.

- **Diagram:** `version`, `title` (the front matter title, sanitized; the grammar has no `title` statement), `accTitle`, `accDescr`, `direction` (the header's, as the DB keeps it), `vertices`, `edges`, `subGraphs`, `connectors`, `classes` and `tooltips` (a map from id to text).
- **Vertex:** `id`, `text` (the label, sanitized; the id when there is none), `labelType` (`text`, `string` or `markdown`), `type` (the shape after alias resolution, absent for a bare id), `styles`, `classes`, `props` (the fields of the `[| field: value | text]` form; `{}` for a vertex created without it, absent for a connector), `metadata`, `link`, `linkTarget`, `isConnector`.
- **Edge:** `id`, `isUserDefinedId`, `start`, `end`, `type` (`arrow_point`, `arrow_cross` or `arrow_open`), `stroke` (`normal` or `dotted`), `length`, `text`, `labelType`, `classes`, `style`, `interpolate`, `animate`, `animation`, `edgeSemantic` (`sequence`, `failure` or `reference`), `metadata`.
- **Container** (`subGraphs`): `id`, `nodes` (the members, as the body gave them), `title`, `labelType`, `dir`, `type` (`flow`), `classes`, `metadata`.
- **Class definition:** `id`, `styles`, `textStyles` (the styles that set a color, with `fill` renamed `bgFill`).

Order: vertices in creation order, connectors in creation order and also in `vertices`, edges in creation order, containers in completion order. Metadata is the YAML as js-yaml reads it, with the prototype keys removed at every depth and non-finite numbers written as sentinel strings.

Left out: the diagnostics (the reference raises them when the layout data is read, so after the parse there are none; `agentflow::semantic::diagnostics` reads them from the model), the element mappings and source positions, the DOM ids of the vertices (`agentflow-<id>-<n>`, from the DB's vertex counter), and `linkStyle default`'s style and interpolation, which the renderer reads but the model does not serialize. The renderer's layout data is built from the model and is not part of it.

## Parsing

`agentflow::parse` runs the reference's steps before the grammar: front matter, directives, `encodeEntities` and a final line break. The `%%` comments stay in the text (the reference keeps them for source positions), unless the text is longer than `maxTextSize` (50,000 UTF-16 units), which parses the text without comments. The wrapper's rule runs too: the blanks between a `}` and the line break after it are removed, for every brace in the text.

The lexer is the flowchart's scanner (`flowchart/scanner.rs`) with a `Dialect` for agentflow: the header word, the keyword table (`style`, `default`, `linkStyle`, `interpolate`, `classDef`, `class`, `flow`, `connector`, `global`, `end` and the link targets), the link operators, `%%` comments as tokens and the blank rule. It follows Jison's rules: the first rule that matches wins, and a keyword literal matches only as a whole word ([0007](../decisions/0007-hand-written-parsers.md)). The grammar (`grammar.rs`) is recursive descent with one function per production, calling the DB (`db.rs`) in the reduction order of the LALR parser. `metadata.rs` reads the `@{ … }` block, `shapes.rs` holds the shape vocabulary, and `semantic.rs` the diagnostics and the semantic projection.

Quirks kept on purpose, because the reference's DB holds them:

- **Silent drops.** A line that contains `direction TB`, `LR`, `BT`, `RL` or `TD` anywhere, in a label or a comment too, is one direction token and its text is dropped. A `click` followed by a blank is one token that takes the rest of the line. A `direction` statement at the top level does nothing; only the header's and a container's own set a direction.
- **Header direction.** `<` is RL, `^` is BT, `>` is LR, `v` is TB, `TD` is TB; any other word stays as written, so `BR` reaches the model.
- **Whole-word keywords.** `flowers`, `classroom`, `styled`, `endpoint` and `globalX` are identifiers; `end --> b` and `call --> b` are syntax errors. `connectors` is never a vertex, though an edge that names it is kept.
- **Shapes.** A shape in the metadata is resolved as an alias (`task`, `tool`, `input`, `decision`, `refdoc`, `action`, `round`, `lean_right`), then its authored name must be lowercase without an underscore (`No such shape: Task. Shape names should be lowercase.`), and the resolved name must be in the shape registry. So `connector`, `collapsedGroup`, `squareRect` and `roundedRect` cannot be authored (the last two have capitals). The layout data then normalizes the shape: a removed shape (`cylinder`, `doc`, `circle`, `stadium`, …) raises `SHAPE_REMOVED` and an unsupported one `SHAPE_UNSUPPORTED`, and the node is drawn as `roundedRect` either way. `rect` and `squareRect` are `roundedRect` without a diagnostic.
- **Edges.** `-->` is a point arrow with the semantic `sequence`, `--x` a cross arrow (`failure`), `-.-` a dotted link without an arrow (`reference`); `==>` and `~~~` are lexical errors. The length is the dash count less one, or the dot count, at most 10. An edge id is the author's (`e1@-->`), else `L_<start>_<end>_<n>` with `n` 0 for the first edge of a pair and the pair's count plus one after it, so the second edge is `_2`. `maxEdges` (500) is checked before each edge is added.
- **`linkStyle`.** The position is the text of the number token and is an index only when it is a canonical decimal of an existing edge; `00` or an index out of range is an error. A style with no `fill` gets `fill:none` once, for all the edges of the statement. `linkStyle default` applies to every edge, the earlier ones too, while `linkStyle default interpolate` reaches only the edges made after it.
- **Containers.** A repeated `flow` id merges its members into the first record and takes the last title and direction. A node that is in two containers stays in the first. A `global` block takes its ids out of every container, those declared earlier included. Metadata written for a container id, a connector or an edge id goes to that record and makes no vertex; a container's `shape` and `label` metadata have no effect. The member `hasOwnProperty` makes the next member a TypeError in the reference's dedupe, so a container with both is a syntax error.
- **Connectors.** A connector is a vertex with `isConnector`, titled by its text or its id; declaring it again keeps the first record and takes a non-empty title; declaring it over a vertex replaces the record.
- **Metadata.** A single-line block is wrapped in braces and a multi-line one is read as it is; a block that fails to read is read again without the trailing commas of its lines. A value that is not an object spreads as JavaScript's spread does, and `0`, `""` and `NaN` are not truthy.
- **Lexer at the end of the text.** Jison reads the end with every rule of its state, so an open `accTitle:`, `accDescr {`, callback or click gives one empty token there.
- **Labels and links** are sanitized with the strict `sanitizeText` and links with `formatUrl`. The security level is the default, so a click sets the class `clickable` and runs no callback.

Errors are ferrigura's own texts; the differential run compares acceptance and the models of accepted inputs.

## Layout and rendering

`agentflow::render` follows the reference's `renderer.ts`: `getData`, the measure step, the ELK graph and ELK Layered, the paint of the unified renderer, then the root element. The shared parts (shapes, edges, markers, the ELK glue) are described in [flowchart.md](./flowchart.md); what is agentflow's own:

- **Layout data** (`data.rs`, `getData` of `agentflowDb.ts`). Collapsed containers (`metadata.view: collapsed`) hide their members and their members' members; an edge into a hidden node is redirected to the outermost collapsed container, and an edge whose two ends collapse into one container is dropped unless it was a self-loop. Each node takes its parent from the containers walked last to first, and a parent that would close a containment cycle is refused. A container is a `flowGroup` (padding 8), or a `collapsedGroup` with its type in its metadata; a vertex is a node of the shape `render_shape` gives it, with `flowchart.padding` (an unset or zero value is 8), `agentflow.minNodeWidth` and `agentflow.wrappingWidth`. An edge takes its `curve` from its own interpolation, else the `linkStyle default interpolate` value, else `flowchart.curve`.
- **The `flowGroup` cluster** (`unified/flow_group.rs`, `createContainerGroup` in `clusters.js`): a transparent frame with a 0.75 px stroke, corner radius 10 and a title that takes no `cluster-label` class. The collapsed form is the flowchart's `collapsedGroup` with the `flow` container type (`unified/shapes/collapsed_group.rs`).
- **Color slots** (`colours.rs`, `colorSlots.ts`). A node's kind is `tool`, `task`, `decision`, `input`, `refdoc`, `connector` or `action`, which owns palette slot 0 to 6 in that order and gets the class `af-kind-<kind>`. A container takes a slot from a counter over the declaration order (a parent before its children, a collapsed container included), starting at slot 7 and wrapping to the palette's length less 7 (at least one).
- **Style sheet** (`style.rs`, `styles.ts`). The palette rules of `genColor` come first, then the static rules, the icon rules and the `classDef` rules. The colors are custom properties on the diagram's id ([0011](../decisions/0011-themes-as-css-variables.md)). Only `redux-color` and `redux-dark-color` have a palette; for other themes the kind classes stay and no palette rule is written. A theme with no background colors strokes the slots without filling them.
- **Root element.** The markers are the flowchart's with the diagram's own name in their ids, plus the two `hierarchy` markers; the two drop-shadow filters come from the look definitions. The title is `text.agentflowTitleText`, centered over everything drawn and `titleTopMargin` above the origin; it is outside the layout. The view box adds `diagramPadding` on each side, and `useMaxWidth` selects `width="100%"` with a `max-width` over a fixed size.

Configuration read: the `agentflow` section's `titleTopMargin` (25), `diagramPadding` (8), `useMaxWidth`, `wrappingWidth` (120) and `minNodeWidth` (120); `flowchart.padding`, `flowchart.curve` and `flowchart.inheritDir` (a container with no direction takes the diagram's); and the theme and look, which default to `redux-color` and `neo` for this type. `nodeSpacing` and `rankSpacing` are accepted and do nothing, as in the reference.

## Differences from Mermaid

- Labels are SVG text, and the output is checked with `htmlLabels: false` ([0009](../decisions/0009-svg-text-labels.md)).
- Click callbacks do not exist and `securityLevel` is not read by the parser ([0006](../decisions/0006-rust-only-no-browser.md)); a click sets `clickable` and the link, and `call` binds nothing.
- Only ELK Layered runs. `layout:` values and container `algorithm` metadata that name another ELK algorithm fall back to Layered, and the geometry run leaves such fixtures out ([0012](../decisions/0012-layout-engines-ported-exactly.md)). `look: handDrawn` is out of scope; the geometry run leaves those fixtures out too.
- Error messages are ferrigura's own ([0007](../decisions/0007-hand-written-parsers.md)).

## Known gaps

- Element mappings and source positions (`getElementAtPosition`, the `add*Mapping` hooks, the front matter line offset, positions in errors) are not ported. They are the reference's editor API; their ported specs are ignored with that reason.
- The DB's diagnostic store (`emitWarning`, `emitError`, `getDiagnostics`) is not ported; the diagnostics are a pure reading of the model.
- `maxEdges` is read from the configuration by the reference; `agentflow::parse` uses 500, and the facade does not pass the configuration's value on (`parse_with_max_edges` takes one).
- The reference's conformance runner is not ported; its eight fixture pairs are ported as specs.
- The layout data is not compared with the reference's by a run of its own; the geometry run checks the result.

## Checking it

- `pnpm -C tools/port port:diff agentflow` compares the models on the inputs of the fixtures, the spec literals, the documentation and the explicit cases of `cases.ts`; `pnpm -C tools/port port:diff config` includes agentflow's configuration. `exceptions.ts` has no agentflow entry.
- `pnpm -C tools/port port:geometry agentflow` compares node, edge and label geometry for the fixtures in `e2e/diagrams/agentflow/` and the probes in `tools/port/geometry/probes/agentflow/` (link styles and default interpolation, class definitions, wrapping and minimum width, inherited direction, padding and title margin, node and rank spacing, nested and mutually containing collapsed containers, edge kinds, removed and unsupported shapes, connectors, Markdown and HTML labels).
- The Rust specs are the ports of the reference's spec files: `crates/ferrigura-syntax/tests/agentflow_*_spec.rs` for the parser and DB, with the conformance pairs in `tests/fixtures/agentflow/`, and `crates/ferrigura-render/tests/agentflow_*_spec.rs` for the layout data, the color slots and the render. Specs that test what is not ported are ignored with their reason.

## Learnings

- **The DB holds what the lexer swallows.** The silent drops (`direction` anywhere on a line, `click `) are the grammar's rules `.*direction\s+XX[^\n]*` and the click state; they produce an empty model without an error, so a differential run only sees them with inputs that contain them (`cases.ts` has them).
- **The diagnostics are not in the parse.** `SHAPE_REMOVED`, `SHAPE_UNSUPPORTED` and `CONTAINMENT_VIOLATION` come from `getData`; a model compared after the parse never has them.
- **`linkStyle` bounds never fire.** The reference tests `typeof pos === 'number'` on a string, so its bounds message is unreachable and a bad index surfaces as a TypeError. ferrigura rejects the same inputs.
- **The palette wins over the attributes.** In the reference the palette rule of a container's slot beats the frame's own `fill="none"` and its stroke attribute, so a container with a slot is filled. The style sheet writes the palette rules first and the same selectors, so the same cascade results.
- **A kind's slot is fixed, a container's is counted.** A container never folds back onto a kind's slot, also on a palette shorter than the seven kinds.
- **A container's explicit `elk.layered`** gives other ELK options than the inherited algorithm (a minimum size and the title strip's constraints), and in the reference another size for the same container; the two are not the same layout.
- **Padding `|| 8`.** `flowchart.padding` of zero is 8 in `getData`, not 0.
- **The wrapper before Jison.** The brace-blank rule keeps line numbers right after a blank line because its pattern never touches a line break; the port keeps the same pattern.
- **The exporter needs a window.** The DB sanitizes through DOMPurify, so `export-agentflow.ts` runs the reference with `provideDomForSanitizing`.
