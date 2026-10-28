# Entity relationship diagram

An entity relationship diagram draws entities with their attributes and the relationships between them, optionally grouped in subgraphs. It starts with `erDiagram`. ferrigura parses it with a hand-written lexer and recursive-descent parser in `ferrigura-syntax` (`src/er/`), draws it with the unified renderer in `ferrigura-render` (`src/er/`, with the entity box in `src/unified/shapes/er_box.rs`) and lays it out with ELK Layered. The shared machinery is in [flowchart.md](./flowchart.md); this page covers what is specific to ER diagrams.

## Model

`ferrigura_model::er::ErDiagram` is the Diagram DB after parsing (`erDb.ts`), with the DB's names in camelCase.

- **Header:** `title` (from the front matter; the grammar has no title statement), `accTitle`, `accDescr` and `direction` (`TB`, or the last top-level `direction` statement).
- **`entities`**, in creation order: `id` (`entity-<name>-<n>`, where `n` is the number of entities before it), `name` (without quotes), `alias` (`""` when none), `attributes`, `classes` (given by `:::` and `class`, without the DB's implicit `default`) and `styles`.
- **An attribute** has `type` (with a trailing `?` when the source had one), `name` (backticks removed), `keys` (as written: `PK`, `pk`, `FK`, `UK`) and `comment` (`""` when none).
- **`relationships`**, in source order: `entityA`, `roleA`, `entityB` and `relSpec` (`cardA`, `relType`, `cardB`). An end is an entity id, or the id of a subgraph of that name when one exists at the point of the statement. The cardinalities are the DB's strings (`ZERO_OR_ONE`, `ZERO_OR_MORE`, `ONE_OR_MORE`, `ONLY_ONE`, `MD_PARENT`); `cardA` is the end on the right of the line, `cardB` the end on the left; `relType` is `IDENTIFYING` or `NON_IDENTIFYING`.
- **`classes`** from `classDef`: `id`, `styles` (the concatenated tokens of each comma-separated part) and `textStyles` (the styles that name a color, with the first `fill` written `bgFill`).
- **`subgraphs`**, in closing order (a repeated id is its own entry): `id` (trimmed), `title` (sanitized and trimmed), `nodes`, `classes`, `styles` and `dir` (`null` when none).

A subgraph's `nodes` are the DB's members and not what is drawn: besides entity names and nested subgraph ids they hold the strings of the `classDef`, `class`, `style`, `accTitle` and `accDescr` statements inside it, because the grammar's `document` rule concatenates the value of every line. Empty and repeated strings are dropped, direction objects set `dir` instead (the last wins), and a member already held by an earlier subgraph is dropped. An entity named like such a string is drawn inside the subgraph, while the model keeps the string in `nodes`.

The model leaves out `shape`, `labelType` (always `markdown`), `look`, `cssCompiledStyles` and `colorIndex` (set by `getData`), and the parser's `subCount` and `subgraphDepth`. Header texts, subgraph titles and the front matter title are sanitized as the DB sanitizes them; entity names, aliases, attributes, roles, styles and class names are raw. The conventions common to all models are in [canonical-model.md](../canonical-model.md).

## Parsing

The text goes through the shared preparation (front matter, directives), comment-line removal and `encodeEntities`, and gets one line break at the end, as `Diagram.fromText` does. The detector is case-sensitive (`^\s*erDiagram`), although the lexer is case-insensitive. `parse_grammar` runs the lexer, parser and DB on the text it is given, as the ported specs do.

**Lexer** (`er/scanner.rs`). The rules of `erDiagram.jison` on a state stack ([0007](../decisions/0007-hand-written-parsers.md)): first match wins, keywords are case-insensitive for ASCII letters with Jison's implicit word boundary, and the states are INITIAL, the three accessibility states, `block` (after `{`), `block_bq` (backticks inside a block) and `style` (after `style` and `classDef`). At the end of the input the rules run once more on the empty rest; a value state still open then fails, and INITIAL gives `EOF` followed by Jison's raw end marker.

**Grammar** (`er/grammar.rs`) is recursive descent; each DB call is made in the order of Jison's reductions, and the value of a statement is the list of members that the enclosing subgraph keeps. `er/db.rs` is the DB.

Quirks of the reference that are kept on purpose:

- `.*direction\s+WORD[^\n]*` takes the rest of the line from the current token start, so `A --> B : x direction TB` keeps only the direction. The rules are tried in the order TB, BT, RL, LR, not by position. Inside a subgraph a `direction` goes to the member list and not to the diagram.
- A keyword needs a word boundary after it: `endpoint` and `classes` are names. `end` also eats the blanks and line breaks that follow. `to` and `optionally to` are keywords, so a role spelled `to` fails unquoted.
- `1` is `ONLY_ONE` only before a blank and a letter, quote or `*`, before a blank and a digit, or before `--`, `..`, `.-` and `-.`; otherwise it is a name. `u` is `MD_PARENT` only before `.`, `-` or `|`.
- A quoted entity name is `"[^"%\r\n\v\b\\]+"`; any other quoted text is a `WORD`, valid only as a role. Unquoted names are `([^\x00-\x7F]|\w|-|\*|\.)+`: every non-ASCII character is a name character, a no-break space included, while `#`, `%`, `"`, `;` and `'` are not.
- Inside a block `\s+` skips line breaks, so attributes need no line break between them (`string name int id` is two attributes); `A {}` and `A {\n}` are valid.
- A generic type `([^\s]*)[~].*[~]([^\s]*)` is greedy to the last `~` on the line, blanks included, so `map~K,V~ data "x~y"` fails.
- An attribute key matches `PK`, `FK` or `UK` in any case and is kept as written; `PK` as an attribute name fails. A comment is `"…"` with no escapes and must follow the keys. `string?` is a type; `string name?` fails.
- In a `style` or `classDef` line there is no catch-all rule, so `.`, `(` and `"` fail, and `;` does not end the state. `classDef` and `style` join their tokens without blanks (`fill:#f9f, stroke: red` gives `fill:#f9f` and `stroke:red`), and the ids of a `classDef` are style-state words, so `classDef A,fill:red` has the ids `A` and `fill`. A blank run that crosses a line break joins the next line to the style.
- `class` ids are INITIAL names: `class A B C` is `class A B` followed by the entity `C`.
- `accTitle:` and `accDescr:` skip line breaks after the colon; `accDescr {` ends at the first `}`.
- A line may hold several statements; there is no separator, and a role ends a statement.
- The alias is set by `addEntity(name, alias)` only when the entity has none yet. A subgraph id and an entity name share one namespace: `G ||--o{ B` with a subgraph `G` first adds an entity `G`, and the relationship uses the subgraph id.
- `title` is not a token, and the grammar's title productions are dead.

## Layout and rendering

`er/mod.rs` is `erRenderer-unified.ts`: it builds the layout data with `er/data.rs`, sets the diagram type `er` (the marker prefix), runs the unified pipeline and writes the title and the root. The root is `<svg class="erDiagram">`, the title `erDiagramTitleText`, and the viewBox is the drawing's box plus 8 (the viewBox does not use `diagramPadding`).

**Layout data** (`er/data.rs`, `getData`). Subgraph nodes come first, in reverse closing order (shape `rect`, group, padding 8), then entities in insertion order (shape `erBox`), then relationships. An entity whose name is also a subgraph id is skipped. An edge has the id `id_<A>_<B>_<k>` (`k` counts relationships, not pairs), the label `roleA`, the markers `cardB` at its start and `cardA` at its end (lower-cased), and the pattern `solid` for an identifying relationship and `dashed` otherwise. Entities take color slots in entity order.

**The entity box** (`unified/shapes/er_box.rs`, `erBox.ts`). Without attributes an entity is a rectangle around its name, at least `er.minEntityWidth` (100) wide. With attributes it is a table: the name in a header row, then a row per attribute with the columns type, name, keys and comment. Column widths are the widest cell plus the padding, the keys and comment columns disappear when empty, and a name wider than the columns spreads the extra width over the present columns. Dividers are polygons 0.0001 wide; odd rows take the theme's `rowOdd` fill and even rows `rowEven`. The texts are Markdown, laid out by `createText` as SVG text, never wrapped inside a cell; `~K,V~` is shown as `<K,V>`.

**Edges and markers** (`unified/markers.rs`). The four cardinality markers have a classic and a neo form (the neo markers use `userSpaceOnUse` and the theme's `strokeWidth`; the circles take `mainBkg`); ids are `<svgId>_er-<type>Start|End`. There are no margin variants and no marker path offsets, so an edge meets the box exactly. ELK-routed edges are drawn with rounded corners (radius 5), and the label sits at the edge's center.

**Style sheet** (`er/style.rs`, `styles.ts`). Colors are custom properties on the diagram id ([0011](../decisions/0011-themes-as-css-variables.md)). In the two color themes each entity is painted from the palette by `data-color-id`, in the order of the entities.

## Differences from Mermaid

- **Labels are SVG text**, also with `htmlLabels: true` ([0009](../decisions/0009-svg-text-labels.md)). The entity box follows the setting only for the padding scale described under Learnings.
- **`layout: dagre`** is laid out with ELK ([0012](../decisions/0012-layout-engines-ported-exactly.md)); `look: handDrawn` takes the classic geometry.
- **`md_parent`** has no marker, as in the reference, which logs a warning that ferrigura does not.
- **`er.nodeSpacing` and `rankSpacing`** are not read: the reference writes them to the flowchart section, where no ELK code reads them.

## Known gaps

- Fixtures with `look: handDrawn` and those with glyphs the bundled faces do not cover (see [0010](../decisions/0010-text-measurement-and-fonts.md)) are not compared in the geometry run.
- The ported specs leave out the reference's `it.skip` (the lexer cannot handle non-English letters) and two tests with empty bodies.

## Checking it

- `pnpm -C tools/port port:diff er` compares the canonical models on the fixtures, the string literals of the JavaScript specs, the docs and the generated character cases of `cases.ts`; the reference's real header setters run with the DOM that DOMPurify needs.
- `pnpm -C tools/port port:diff er-data` compares `getData` (`export-er-data.ts`, the `er_data` example); `pnpm -C tools/port port:diff config` compares the `er` section.
- `pnpm -C tools/port port:geometry er` renders every fixture with both in Chromium and compares geometry.
- Rust: `er_spec.rs`, `er_characters_spec.rs` (the reference's loop over characters, one generated test per case), `er_db_spec.rs` and `er_subgraph_spec.rs` in `ferrigura-syntax/tests`, `er_get_data_spec.rs` in `ferrigura-render/tests`, and the ER graphs among the captured inputs of `ferrigura-elk` (`tests/corpus/er-diagram__*`).

## Learnings

- **The default look is neo**, although the top-level default is classic: the `er` section defaults it, and the scope resolution picks it up.
- **The padding scale depends on `htmlLabels`.** `erBox` multiplies `diagramPadding` and `entityPadding` by 1.25 when `htmlLabels` is false, and only on the attribute path; an entity without attributes uses the unscaled `diagramPadding`. ferrigura follows the setting (1 by default, 1.25 for `false`), while drawing SVG text in both; the geometry run forces `false`.
- **`rowOdd` and `rowEven` are not set by the redux themes.** Only the base, default and dark themes define them, so with `redux-color` the reference draws the row rectangles without a fill path (the first row group has one path). ferrigura reads the tokens when a theme has them and draws no fill otherwise.
- **A box without attributes wraps at 120**, the `flowchart.wrappingWidth`, in the declared font size; an attribute cell never wraps because its wrap width is its own width plus 100.
- **The minimum-width test uses an integer.** `calculateTextWidth` rounds, so `calc(label) + 2·padding < 100` compares a rounded value, while the cells' boxes come from the SVG text's `getBBox`.
- **The column-adjustment quirk.** The surplus width is computed with the header's `nameBBox`, because the loop's `nameBBox` is block-scoped; `keys.join()` joins with a comma and no blank.
- **Edge ids count relationships,** not pairs, so parallel relationships between two entities get `id_A_B_1` and `id_A_B_2`.
- **`nodeSpacing` and `rankSpacing` are dead for ELK.** The renderer writes 140 and 80 into the flowchart section; the ELK path reads its own options.
- **Subgraph members are phantoms.** The strings of `classDef`, `class`, `style` and the accessibility statements inside a subgraph are in `nodes` and take part in the removal of members that an earlier subgraph holds. They are not drawn; the cases of `cases.ts` pin them.
- **`accDescr` loses blank lines.** The setter's `\n\s+` to `\n` replaces the converter of pie and info, so `line one`, a blank line and `  line two` becomes `line one\nline two`.
