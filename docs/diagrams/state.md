# State diagram

A state diagram draws states, the transitions between them, composite states with concurrent regions, forks, joins, choices and notes. It starts with `stateDiagram` or `stateDiagram-v2`; both headers are one diagram type with one configuration section, `state`. ferrigura parses it with a hand-written scanner and recursive-descent parser in `ferrigura-syntax` (`src/state/`), draws it with the unified renderer in `ferrigura-render` (`src/state/`, with the state shapes and cluster kinds under `src/unified/`) and lays it out with ELK Layered. The shared machinery is in [flowchart.md](./flowchart.md); this page covers what is specific to states.

## Model

`ferrigura_model::state::StateDiagram` is the Diagram DB after parsing (`stateDb.ts`), read before `getData()` runs, because `getData()` translates the document once more.

- **Header:** `title` (the front matter's, sanitized), `accTitle`, `accDescr`.
- **`direction`** is the DB's `getDirection()`: the first `dir` statement of the root document, `TB` when there is none. **`layoutDirection`** is the direction the layout uses (`getData().direction`): the last `dir` statement of the root document. They differ when a diagram gives several.
- **`document`** is the root statement tree after `docTranslator`. A statement carries only the keys its JavaScript object has, tagged by `stmt`: `state` (`id`, `type`, `description`, `classes`, `doc`, `note`, `start`), `relation` (`state1`, `state2`, `description`), `dir`, `classDef`, `style`, `applyClass` and `click`. A statement the DB ignores but the document keeps (`scale`, `hide empty description`, an accessibility text, a composite's bare name, a floating note) is a JSON string, because it counts in the test for a trailing region.
- **`states`**, the root document's states in the order of their first mention: `id`, `type` (the first type the state was given), `descriptions`, `doc`, `note`, `classes`, `styles` and `textStyles` (always empty here).
- **`relations`**, **`classes`** (`id`, `styles`, `textStyles`) and **`links`** (one per `click` statement, in source order).

The model leaves out `dividerCnt`, `startEndCount`, the link map's keys beyond the state id, the callback functions and tooltips' bindings, `getData()` and the DB's `stmt` field on entries. A trailing concurrency region gets a random id in the reference; the model writes `<generated:k>` for the k-th one, in the order the reference creates them, and the exporter numbers the reference's ids the same way (`tools/port/state-generated.ts`). The conventions common to all models are in [canonical-model.md](../canonical-model.md).

## Parsing

The text goes through the shared preparation (front matter, directives), comment-line removal and `encodeEntities`, and gets one line break at the end, as `Diagram.fromText` does. There is no closing-brace rewrite. `parse_grammar` runs the grammar and the DB alone, as the ported specs call the reference's parser.

**Scanner** (`state/scanner.rs`). The rules of `stateDiagram.jison` in file order on a state stack ([0007](../decisions/0007-hand-written-parsers.md)): first match wins; a rule with no condition applies in INITIAL only; the whole grammar is case-insensitive (`%options case-insensitive`); keyword literals ending in a word character get the implicit word boundary, while literals followed by `\s+` or `\s*` (`classDef`, `stateDiagram`, `as`) have none. Zero-length matches are tokens (class definition ids, style lists, accessibility values, state ids, note text), and the scanner makes Jison's last attempt at the end of the input. The table lists every rule with its line in the grammar.

**Grammar** (`state/grammar.rs`). Recursive descent over the tokens. A document is a list of statements with no separator needed, so `A B` is two states; inside `{ … }` newlines are not tokens. Lookahead tells `idStatement`, `idStatement DESCR` and `idStatement --> idStatement [DESCR]` apart (a relation keeps its description, a state keeps none), a composite with or without a body, and the `state "text" as ID` forms.

**DB** (`state/db.rs`). `docTranslator` turns `[*]` into `<parent>_start` or `<parent>_end` (the root parent is `root`), splits a document at its dividers into regions, and gives each trailing region a generated id; `extract` then fills the root-level maps. The one parse-time error that `extract` can raise is kept: a composite with two or more descriptions fails with the reference's "Group nodes can only have label" error, which the port finds by simulating `dataFetcher`'s accumulation of descriptions.

Quirks of the reference that are kept on purpose:

- A state keeps the first `type` it was given: `A --> B` and then `state A <<fork>>` leaves `A` a plain state.
- `class`, `:::` and `style` on a state, and relations, inside a composite do not reach the DB maps (only root statements are extracted). The drawing still shows the relations, from the composite's `doc`.
- `classDef default …` is a parse error (the lexer gives `DEFAULT_CLASSDEF_ID`, the grammar expects `DEFAULT`). A standalone `A:::cls` outside a relation adds no class.
- `state foo` and `state foo : d` alone create nothing. `title Foo` in the body creates the states `title` and `Foo`; only the front matter sets a title. `note "text" as N` (a floating note) is the string `note `.
- A line that holds `direction TB` (or `BT`, `RL`, `LR`) from the current position is one `direction` token, so a description such as `A : foo direction TB` loses its state.
- `;` is not a separator. It belongs to an id and ends a description or note text.
- `A - B`, `A -> B` and `--> A` are parse errors. `A --> B : ` has no label; `A --> B : x:y` keeps its colon.
- Whitespace before a line break swallows it, so `click A "x" "y"  ` followed by a line is an error.
- A block note keeps the indentation of its later lines; `note right of A  : text` (two blanks before the colon) keeps the colon in the text.
- `state "x" as S1:alt` has the descriptions `["x", "alt"]` and the id `S1`; with a body the description and id are kept raw. `[*] : desc` gives the start state a description.
- A trailing `--` in a composite is a leaf divider with no `doc`. The reference cannot draw it (`No such shape: divider`), and neither does the renderer.
- A composite id with a trailing blank (`state "x" as [*] {`) is keyed `[*]` in the DB and drawn as `<parent>_start`, because the second `docTranslator` pass trims it.
- `classDef … color:x` also puts `fill:x` into `textStyles`, which only the DB holds. `style A …` twice replaces the first; `class A,B foo` gives both `foo`.
- `click A "url" "tip"` keeps the quotes of both values; the renderer strips them.

## Layout and rendering

`state/mod.rs` is `stateRenderer-v3-unified.ts`: it builds the layout data with `state/data.rs`, sets the diagram type `stateDiagram` for both headers (it names the markers), runs the unified pipeline, wraps clickable states, and writes the title and the root. The root is `<svg class="statediagram" aria-roledescription="stateDiagram">`, the title is `statediagramTitleText`, and the viewBox is padded by 8.

**Layout data** (`state/data.rs`, `dataFetcher.ts` and `stateDb.getData`). Each state becomes a node: `rect` with a corner radius of 10 and padding 8 (`rectWithTitle` when it has two or more descriptions, the first being the label), `stateStart` and `stateEnd` for `[*]`, `fork`/`join`, `choice`, and for a composite `roundedWithTitle` as a group (a concurrency region is a `divider` group). Every node gets `wrappingWidth` and `minWidth` from `state.wrappingWidth` and `state.minNodeWidth` (120). A note adds a `<id>----note-<n>` node inside a `<id>----parent` group of shape `noteGroup` (padding 16) and a dashed `note-edge`. Relations become `edge<n>` edges with the barb marker of the look. `graphItemCount` is shared by DOM ids and edge ids: it grows for each relation and each note group, not for each node, and a node's DOM id `state-<id>-<n>` takes the counter at its last visit. Containers take color slots in pre-order; a divider region inherits its parent's slot. The classes are `<classes> statediagram-state`, and groups add `statediagram-cluster` (and `-alt` on alternate depths).

**Shapes and clusters.** `unified/shapes/rect_with_title.rs` (a title above a divider, then the rows; the rows are measured by their first line), `unified/shapes/state.rs` (`forkJoin`, `stateStart`, `stateEnd`), `unified/shapes/symbols.rs` (`choice`) and, for the clusters, `unified/clusters.rs` (`roundedWithTitle`, `noteGroup` and `divider`, which paint no title).

**Markers.** The barb (`unified/markers.rs`): `barb` in the classic look and `barbNeo` with its margin variant in the neo look; ids are `<svgId>_stateDiagram-barbEnd[-margin]`. The edge offset of `arrow_barb_neo` is 5.5.

**Style sheet** (`state/style.rs`, `styles.js` and the shared rules). Colors are custom properties on the diagram id ([0011](../decisions/0011-themes-as-css-variables.md)); the palette rules of `genColor` come first, for the two color themes. The selectors that match nothing in the reference (`g.stateGroup`) are not written, and the reference's `//` comments and nested `text {}` inside `.state-note`, which are not CSS, are not written either.

**Links.** After the render, a clicked state's node is wrapped as `<a xlink:href title>` around its `<g>`, with no transform on the anchor and the surrounding quotes stripped; only leaves are wrapped.

## Differences from Mermaid

- **Labels are SVG text**, also with `htmlLabels: true` ([0009](../decisions/0009-svg-text-labels.md)).
- **`layout: dagre`** is laid out with ELK ([0012](../decisions/0012-layout-engines-ported-exactly.md)); `look: handDrawn` takes the classic geometry.
- **One `extract`.** The reference runs `extract` at the parse and again when it draws, and `docTranslator` twice during the parse; the second runs give the same states, relations, classes and links, so the port runs it once. The exception is the trimmed-id case above, which the parse reproduces.
- **Callbacks** are not kept ([0006](../decisions/0006-rust-only-no-browser.md)).

## Known gaps

- The unified edge drawing takes the `flowchart` section's `curve` for state diagrams; the reference reads a `curve` at the top level of its configuration, which no configuration sets. The two agree unless a user sets `flowchart.curve` for a state diagram.
- Fixtures with `look: handDrawn` are not compared in the geometry run.

## Checking it

- `pnpm -C tools/port port:diff state` compares the canonical models on both fixture folders (`state-diagram`, `state-diagram-v2`), the state inputs of `e2e/platform/dev-diagrams`, the specs and the docs; the reference's detector runs first.
- `pnpm -C tools/port port:diff state-data` compares `getData` (`export-state-data.ts`, the `state_data` example), with the random region ids numbered in creation order; `pnpm -C tools/port port:diff config` compares the `state` section.
- `pnpm -C tools/port port:geometry state` renders every fixture with both in Chromium and compares geometry.
- Rust: the `state_*_spec.rs` files in `ferrigura-syntax/tests` (ported from the reference's state specs; the renderer and DB-direct tests are ignored with their reasons), `state_data_*_spec.rs` in `ferrigura-render/tests` (color slots, notes, styles), `crates/ferrigura/tests/state.rs` for the root, markers and title, and the state graphs among the captured inputs of `ferrigura-elk` (`tests/corpus/state-diagram*`).

## Learnings

- **The DB and the layout disagree about the direction.** `getDirection()` returns the first root `dir`, `getData().direction` the last; the layout uses the second. A `dir` inside a composite is that composite's own direction.
- **Only the root document is extracted.** Anything the DB maps hold for a state in a composite, such as classes, styles or relations, is missing; `dataFetcher` reads classes and styles from those maps, so a class on an inner state is not drawn.
- **`graphItemCount` is visible in the SVG.** The ids `state-<id>-<n>` and `edge<n>` depend on the order in which the fetcher visits states and on which visits bump the counter, so the walk must follow the reference's `setupDoc` order exactly.
- **State labels start at the label's origin.** The state stylesheet has no `.node .label text { text-anchor: middle }`, which is the flowchart's rule. The label group sits at `translate(-W/2, -H/2)`, with W the label box widened to `state.minNodeWidth`, and `rectWithTitle` titles and cluster titles start at their origin too. The unified renderer takes the anchor from the diagram type (`pipeline::node_anchor`).
- **The note's size** is the larger of its label box plus twice the padding and the node's width, which comes from the measure step's minimum of 120. The note padding is `flowchart.padding` (15 by default), not a state key.
- **rough.js at roughness 0.** The solid fill of a polygon is an open path (`M p0 L p1 … L pn`), while the choice's `rc.path` with `Z` is closed. Every stroke side is drawn twice; the passes lie on the side, so length and sampling match, and only the random divergence point of each stroke moves control points. Fork bars, notes and choices use `shapes/rough.rs` for this.
- **Spacing keys do nothing with ELK.** The reference sets `nodeSpacing` and `rankSpacing` on the layout data (50 each), but only the swimlane and other layouts read them; ELK reads its own options, so the renderer reads neither.
- **Both headers are one type.** The detector id is `stateDiagram`, so `stateDiagram-v2` shares the section, the marker ids and the root class; the header is case-sensitive in the detector and case-insensitive in the lexer.
- **Directive text stops at U+2028, U+2029 and a bare CR** in the reference's regex (`loose_directive_len` in `preprocess.rs`); a fuzzer found this through state inputs, and it applies to every type.
