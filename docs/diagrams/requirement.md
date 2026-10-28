# Requirement diagram

A requirement diagram draws requirements and the elements that satisfy or verify them, joined by typed relationships. It starts with `requirementDiagram`. ferrigura parses it with a hand-written scanner and recursive-descent parser in `ferrigura-syntax` (`src/requirement/`), draws it with the unified renderer in `ferrigura-render` (`src/requirement/`, with the box in `src/unified/shapes/requirement_box.rs`) and lays it out with ELK Layered. The shared machinery is in [flowchart.md](./flowchart.md); this page covers what is specific to requirement diagrams.

## Model

`ferrigura_model::requirement::RequirementDiagram` is the Diagram DB after parsing (`requirementDb.ts`), with camelCase keys.

- **Header:** `title` (the front matter's, sanitized), `accTitle`, `accDescr` and `direction` (`TB`, `BT`, `RL` or `LR`: the last `direction` statement, `TB` when none).
- **`requirements`**, in the order they are first given: `name`, `type` (the display string: `Requirement`, `Functional Requirement`, `Interface Requirement`, `Performance Requirement`, `Physical Requirement` or `Design Constraint`), `requirementId`, `text`, `risk` (`Low`, `Medium` or `High`), `verifyMethod` (`Analysis`, `Demonstration`, `Inspection` or `Test`), `classes` (`default` first, then the classes given to it) and `cssStyles`. A property that is not set is `""`.
- **`elements`:** `name`, `type`, `docRef`, `classes` and `cssStyles`.
- **`relationships`**, in the order of their statements, duplicates kept: `type` (`contains`, `copies`, `derives`, `satisfies`, `verifies`, `refines` or `traces`), `src` and `dst`.
- **`classes`** from `classDef`: `id`, `styles` and `textStyles`.

Texts keep the DB's entity placeholders (`ﬂ°name¶ß`, `ﬂ°°35¶ß`), because the DB does not sanitize requirement, element, relationship or class texts; the renderer decodes them when it draws. The header texts go through the sanitizer of the flowchart with the DB's whitespace rules (`setAccTitle`, `setAccDescription`) and keep their inner blanks. The model leaves out `getData` and everything the renderer derives. The conventions common to all models are in [canonical-model.md](../canonical-model.md).

## Parsing

The text goes through the shared preparation (front matter, directives), comment-line removal and `encodeEntities`, and gets one line break at the end, as `Diagram.fromText` does. The front matter title is set before the parse and is not encoded.

**Scanner** (`requirement/scanner.rs`). The rules of `requirementDiagram.jison` in file order on a state stack ([0007](../decisions/0007-hand-written-parsers.md)): first match wins; rules without a state apply in INITIAL only; keywords match case-insensitively with Jison's implicit word boundary, while names keep their case. The states are INITIAL, `style` (entered by `style`, `classDef` and `class`, left at a line break), `string` and the accessibility states. The scanner reproduces Jison's `next()` with its `done` flag: when the input is used up the rules run once more, and every call after that returns Jison's raw end symbol, which the grammar does not accept.

**Grammar and DB** (`requirement/mod.rs`, `requirement/db.rs`). Recursive descent; each production makes its DB calls where Jison's actions run. A block's properties are collected first, because the actions of a body run before the action of its block. A property takes its first value, and a block whose name exists keeps its first block. `X <- T - Y` stores the relationship `Y T X`.

Quirks of the reference that are kept on purpose:

- A `title` line is a parse error in the body (the `title` token has no production), and so is a value that starts with `title ` (`text: title x`). The title comes from the front matter only.
- A `{` must be followed by a line break, and `\s+` takes line breaks, so `requirement a {` followed by blanks and a line break fails, and `requirement a {}` fails. A header followed by blanks and a line break fails too.
- A single `%` at a token start begins a comment, because the reference reads `\%%` as one `%`. A `#` at a token start is a comment as well, which works inside a block but not before the header (`cleanupComments` does not remove `#` lines).
- An unquoted name is `[\w][^:,\r\n\{\<\>\-\=]*`, trimmed, ASCII `\w` only: it may hold spaces, `#`, `"`, `}` and `;`, but not `-`, `:` or `,`. So `id: REQ-001`, `text: a, b` and `requirement my-req {` fail, and `text: note: x` fails while `text: "note: x"` does not. An empty quoted string fails; an unclosed `"` takes the text to the end and fails.
- `direction` anywhere on the line of a token start makes the token a direction, with the rest of the line: `text: go direction TB here` fails.
- A style value is its tokens joined with no blanks (`fill: red` is `fill:red`); `%`, a tab or `(` fail in a `style` line. `classDef c` with no styles fails.
- `setCssStyle` stops at the first id that is no requirement or element, so the ids after it are not styled; a requirement is looked up before an element. `defineClass` also appends the styles to nodes that already carry the class, so a class applied before its `classDef` still gets its styles.
- `accDescr {}` after the header is accepted with no value; `accTitle:` or `accDescr {` at the end of the text is rejected, because the empty value is followed by the raw end.
- A relationship's ends are not checked against the declared names, and a requirement and an element may share a name.
- `direction` is the last statement; `clear()` does not reset it, and a new DB starts at `TB`.

## Layout and rendering

`requirement/mod.rs` is `requirementRenderer.ts`: it builds the layout data with `requirement/data.rs`, sets the diagram type `requirement` (the marker prefix), runs the unified pipeline and writes the title and the root. The root is `<svg class="requirementDiagram" aria-roledescription="requirement">`, the title is `requirementDiagramTitleText` at `-titleTopMargin`, and the viewBox is the drawing's box plus 8.

**Layout data** (`requirement/data.rs`, `getData`). Nodes are the requirements in order, then the elements, each of shape `requirementBox` and carrying its fields in `Node.requirement`. Edges are the relationships, with the id `<src>-<dst>-<n>` (`n` is the relationship's index) and a Markdown label `<<kind>>`. A `contains` edge has `requirement_contains` at its start and no dash; every other edge is dashed (`stroke-dasharray: 10,7`) and ends with `requirement_arrow`. Boxes take color slots in node order.

**The box** (`unified/shapes/requirement_box.rs`, `requirementBox.ts`). The box draws its own texts, as `erBox` does: the type `<<Requirement>>` (or `<<Element>>`), the name in bold, then the body (`ID:`, `Text:`, `Risk:` and `Verification:` for a requirement, `Type:` and `Doc Ref:` for an element), with empty values left out. Each text is measured with `getBBox` plus 6 in height; the outline is the box of the texts plus 20; each label is then moved by its index among the labels (the type and the name stay centered, the body starts 10 in). A divider under the name is a polygon in the neo look and a line in the classic look, drawn when the body has an `ID`, `Text` or `Risk` for a requirement, or a `Type` for an element.

**Markers** (`unified/markers.rs`). `requirement_arrow` and `requirement_contains` in a classic and a neo form (the neo form uses `userSpaceOnUse` and the theme's `strokeWidth`); there are no marker offsets and no color variants.

**Style sheet** (`requirement/style.rs`, `styles.js`). The reference's rules in its order, with colors as custom properties ([0011](../decisions/0011-themes-as-css-variables.md)); the rules that match nothing in the reference's output (`.reqBox` and the like) are written too. In the two color themes the boxes are painted from the palette by `data-color-id`.

## Differences from Mermaid

- **Labels are SVG text**, also with `htmlLabels: true` ([0009](../decisions/0009-svg-text-labels.md)).
- **`layout: dagre`** is laid out with ELK ([0012](../decisions/0012-layout-engines-ported-exactly.md)); `look: handDrawn` takes the classic geometry.
- **Two inputs fail to render** (`check_names` in `requirement/mod.rs`): a relationship that names no requirement or element, which makes the reference fail inside ELK, and a requirement and an element with one name, which makes it write two nodes with one id and a `translate(undefined…)` error. Both parse without error, and no fixture has either. Both are exceptions of the renderer by name, not rules of the comparison.

## Known gaps

- Fixtures with `look: handDrawn` are not compared in the geometry run, and two malformed fixtures that the reference itself rejects are reported as reference failures.
- Front matter YAML that the reference's reader rejects and the shared reader accepts (a value after a key-less `:`) is accepted here; it is a gap of `yaml.rs`, shared by every type.

## Checking it

- `pnpm -C tools/port port:diff requirement` compares the canonical models on the fixtures, the specs, the docs and the explicit cases of `cases.ts`, one case per quirk above; `export-requirement.ts` writes the texts as the DB holds them.
- `pnpm -C tools/port port:diff requirement-data` compares `getData` (`export-requirement-data.ts`, the `requirement_data` example); `pnpm -C tools/port port:diff config` compares the `requirement` section.
- `pnpm -C tools/port port:geometry requirement` renders every fixture with both in Chromium and compares geometry.
- Rust: `requirement_spec.rs` and `requirement_db_spec.rs` in `ferrigura-syntax/tests` (the DB spec drives `parse` with the text that makes each DB call), `requirement_get_data_spec.rs` in `ferrigura-render/tests`, and the requirement graphs among the captured inputs of `ferrigura-elk` (`tests/corpus/requirement__*`).

## Learnings

- **The renderer reads the `state` section.** `state.titleTopMargin` (25) places the title and `state.useMaxWidth` decides the size attributes, copied from the state renderer in the reference. `state.nodeSpacing` and `rankSpacing` have no effect, and the `requirement.rect_*`, `text_color`, `fontSize` and `line_height` keys are read nowhere.
- **The default look is neo,** resolved by the requirement scope although the top-level default is classic.
- **Weights are measured per word.** The reference measures every word of a label in the weight its group's style declares: the name is bold because its group's style says so (51.086 px against 46.130 px for `sys_req`), and a `font-weight:bold` on a node or in a class is measured the same way. The port applies the group's `font-weight` to every word (`style_weight`).
- **The body starts at the origin for every layout.** The reference's `center` branch for a `layout` other than `elk` writes no `text-anchor`, and `createText` never centers a node's text, so the body is left-aligned either way.
- **The wrap width is measured on the raw text,** which holds entity placeholders, plus 50; the text drawn is `sanitize_text(decode_entities(…))`. An empty text has no label and no height.
- **The divider rule counts the body, not the verification.** It is drawn when `accumulated > typeHeight + nameHeight + 20`, and `Verification:` (and `Doc Ref:`) do not add to `accumulated`, so a requirement with only a verification method has no divider.
- **Re-translation counts `.label` groups only.** The type and the name are labels 0 and 1; an empty name would shift the index, which the grammar does not allow.
- **The edge path style is `fill:none;;;;fill:none;` in the classic look,** because the empty style entry is kept; the neo look prefixes a dash array computed from the path length, with no marker offsets.
- **The unused rules of the sheet stay.** `.reqBox`, `.reqTitle`, `.reqLabel`, `.reqLabelBox`, `.req-title-line` and `.relationshipLabel` match no element, but the reference writes them.
