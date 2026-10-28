# Use case diagram

A use case diagram draws actors, use cases, system boundaries, notes and JSON tables, and the relationships between them (associations with arrow ends, `include`, `extend` and generalization). It starts with `usecase-beta`; the detector accepts the keyword when whitespace or the end of the text follows it. ferrigura parses it with a hand-written scanner and recursive-descent parser in `ferrigura-syntax` (`src/use_case`), and draws it on the unified renderer in `ferrigura-render` (`src/use_case`, with its own shapes in `src/unified/shapes/usecase_*.rs`), laid out by ELK Layered ([0012](../decisions/0012-layout-engines-ported-exactly.md)). The unified renderer's shared machinery (the shapes registry, labels, clusters, edges, markers, the ELK glue) is described in [flowchart.md](./flowchart.md); this page covers what is the use case diagram's own.

## Model

The canonical model is the DB's state after the parse (`usecaseDb.ts`), as `UseCaseDiagram` in `ferrigura-model` (`src/use_case.rs`). The conventions of [canonical-model.md](../canonical-model.md) apply; these are the fields. Every list is in the DB's order, and an optional key that the reference leaves `undefined` is absent from the JSON.

| Field              | Holds                                                                                                                                                                              |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `title`            | Always `""`                                                                                                                                                                        |
| `direction`        | `TB`, `BT`, `RL` or `LR` (the default); `TD` is stored as `TB`, and the last `direction` statement wins                                                                            |
| `accTitle`         | The accessibility title, sanitized and trimmed                                                                                                                                     |
| `accDescription`   | The accessibility description, sanitized                                                                                                                                           |
| `actors`           | `{ id, label, labelType, type, icon?, business, stereotype?, parentId?, classes, styles }`; `type` is `normal`, `hollow`, `awesome` or `icon` (an `icon` metadata makes it `icon`) |
| `useCases`         | `{ id, label, labelType, shape, business, stereotype?, parentId?, classes, styles }`; `shape` is `ellipse`, or `rect` for the bracket form `Id[Label]`                             |
| `systemBoundaries` | `{ id, label, labelType, type, members, classes, styles }`; `type` is `rect` or `package`, and `members` are the ids of its elements in declaration order                          |
| `relationships`    | `{ id, explicitId, source, target, type, arrowType, label?, labelType?, minlen, classes, styles, animate, animation? }`                                                            |
| `notes`            | `{ id, target, label, labelType }`, with ids `note-0`, `note-1` and so on                                                                                                          |
| `jsonNodes`        | `{ id, value, propertyOrder, classes, styles, rows }`                                                                                                                              |
| `classDefs`        | `{ id, styles }`, the last `classDef` of an id winning                                                                                                                             |
| `ast`              | The graph AST of the parse (`usecaseAst.ts`): header, nodes, edges, groups, class definitions and one statement per line, with spans                                               |

Details of the relationship and the table fields:

- `type` of a relationship is `association`, `include`, `extend` or `generalization`. `arrowType` is the DB's number: 0 solid arrow, 1 back arrow, 2 plain line, 3 circle, 4 cross, 5 reversed circle, 6 reversed cross. `minlen` is the number of dashes minus one. An anonymous edge is `edge-0`, `edge-1` and so on in source order, and `explicitId` tells an `id@` from those. `animation` is `fast` or `slow`; `animate: true` alone sets `animate` and leaves `animation` absent.
- `value` of a JSON table is the object as `JSON.parse` reads it, a non-finite number written as its sentinel string. `propertyOrder` lists the keys of each object in the order of the text, keyed by JSON pointer (`""` is the root), because a JavaScript object does not keep that order. `rows` are the display rows of `getData`: `{ key, accessibleKey, value }`, the only place that shows how JavaScript writes the numbers (`String(number)`) and in which order the keys appear.
- `ast` spans are `[start, end]` in UTF-16 code units, as JavaScript counts them. The AST drops the key `__proto__` from `nodes`, `groups` and `classDefs`, and leaves out keys that the reference adds only for a truthy text (an empty `icon`, `stereotype` or `label`); the `label` of an actor, use case or boundary is left out when it equals the id.

What the model keeps from the DB's behavior:

- **Labels are raw.** The DB sanitizes labels in `getData` only, so the model holds them as the grammar gave them, with the entity placeholders of `encodeEntities` (`ﬂ°…¶ß`). The header texts are sanitized, twice, as the reference does (once in the visitor's setter, once at `commit`): `accTitle` loses its leading blanks and `accDescr` its blanks after each line break.
- **The title is `""`.** `Diagram.fromText` sets the front matter title, then the parser calls `clear`, which resets it. The grammar has no `title` statement.
- **An id from a quoted name** is `label.replace(/\W/g, '_')`, so `"Café"` is `Caf_` and `"Hello World!"` is `Hello_World_`; two labels can collide, and the builder reports it.
- **A reference to an undeclared id** creates a use case with the `ellipse` shape.
- **Order of elements.** Actors, use cases, boundaries and JSON tables are sorted by the offset of the first mention of their id (`reorderElements`), a relationship endpoint included, not by the offset of their declaration.

The model leaves out the DB's symbol table and counters, the configuration, and the render input (`getData`). The renderer builds the latter from the model.

## Parsing

`prepare`, `remove_directives`, `cleanup_comments` (whole-line `%%` lines and blank lines before them) and `encode_entities` run as `Diagram.fromText` runs them, and the text gets a final line feed. The syntax is line oriented: the header ends its line, and every statement ends with a line break or the end of the text.

The reference's parser is Chevrotain's ([0007](../decisions/0007-hand-written-parsers.md)). The pipeline in `ferrigura-syntax/src/use_case/` follows its stages:

- **`scanner.rs`** is the lexer, in three modes: the default mode, the JSON body after `json <id> @`, and the stereotype after `<<`. Chevrotain tries the patterns of a mode in the written order and takes the first that matches, not the longest, except that a keyword's `longer_alt` (`Identifier`) wins when it matches more. The scanner keeps that table and its custom matchers (comments, accessibility lines and blocks, Markdown strings, JSON objects, stereotype text) by hand.
- **`grammar.rs`** is recursive descent with the reference's choices. Where Chevrotain decides by its lookahead or a GATE, the same test runs in the same order: the order of the `statement` alternatives, the OPTION guards of `relationTail` and `entityName`, the `hasLabeledRight` look-ahead of the arrows, and `LA(0)`, the exact image of the last token consumed.
- **`visitor.rs`** turns the syntax tree into the builder's drafts (`draft.rs`) and the AST statements, with UTF-16 spans.
- **`builder.rs`** ports `usecaseModelBuilder.ts`: `finalize` runs the reference's steps in their order and checks the whole diagram, and the first failing check decides the error. A diagram is accepted or rejected as a whole.
- **`json.rs`** reads a JSON table's object as `JSON.parse` reads it, together with the order of its keys, in one pass; it also holds `js_number_to_string` (ECMA-262's Number-to-String) and `json_rows`.
- **`ast.rs`** builds the graph AST.

Grammar quirks of the reference that ferrigura keeps on purpose, each probed against it:

- **Keywords are not names.** `actor`, `end`, `note`, `for`, `class`, `classDef`, `style`, `json`, `direction`, `systemBoundary`, `include`, `extend`, `true`, `false`, `TD`, `TB`, `BT`, `LR`, `RL` and `usecase-beta` cannot be ids. They are label text and style values; quoted names are the way out. `include` and `extend` alone are case-insensitive. The direction keywords are upper case.
- **`x--` and `o--` are operators** wherever they start a token, so `x-->y` fails, and `x --> y` parses. `--o` and `--x` take the start of any id that begins with `o` or `x`, so `A --oz B` fails.
- **A label needs the exact image.** `A <-- label -- B` and `A -- label --> B` are labeled; `<---` and `---` cannot take a label. The closing operator must match the family: `<--`, `o--` and `x--` close with `--`; `--` closes with `-->`, `--`, `--o` or `--x`. The closer must stand on the same line.
- **`metadataAssignmentStatement` shadows the entity statement** when `@{` follows a target, so `A @{ type: "hollow" } --> B` is an error; the assignment stands on its own line. An edge's metadata is the statement `e1@{ animate: true }`.
- **Metadata values** are identifiers, quoted strings, `true` or `false`. A number (`1.5`) is an error, a keyword (`type: actor`) is an error, and `1mg` is an identifier.
- **Boundaries hold entity names and `actor` lines only.** There are no relations, notes, styles or nested boundaries inside one. `allowmixing`, `newpage`, `package`, `rectangle` and `skinparam` are forbidden as the first word of a statement (PlantUML words).
- **Label text is the source slice**, with its inner spacing. The scanner's last-resort token takes any run of non-printable-ASCII characters, so a non-ASCII letter is label text but a syntax error in a name; an id is ASCII `\w`.
- **A comment** is `%%` as the first token of its line; `A %% c` is an error, and so is the header followed by one. The accessibility lines need a line start, indented or not.
- **A stereotype** is non-blank text between `<<` and `>>` on one line.
- **The dead alternative of `CssIdentifier`** (`--…`) is dropped: `--` is matched first. `foo-bar` is a CSS identifier.
- **Only the first lexer error and the first parser error are reported**, by position.

## Layout and rendering

`ferrigura-render/src/use_case/` ports `usecaseDb.ts`'s `getData`, `usecaseRenderer.ts`'s `draw` and `styles.ts`; `ferrigura-render/src/unified/shapes/usecase_*.rs` ports the shapes.

- **`data.rs`** builds the unified layout data from the model, in the reference's order: actors, use cases, notes, JSON tables and boundaries, then the relationships, then one dotted edge per note. A node's color index counts actors and use cases together; boundaries count on their own, and notes and JSON tables do not count. It also holds `prepareUsecaseLayoutData`'s steps: plain labels are escaped (`&`, `<`, `>`), JSON cells are escaped, and the stereotype of an ellipse or a rectangle is folded into the label. It makes the accessible names (`use case Sign in`, `Note for X: …`, `association label from A to B`, `package system boundary …`) and the DOM ids (`usecase-<id>`, `usecase-<diagram>-<edge>`).
- **`mod.rs`** runs the steps of `draw`: layout data, the measure step, the data attributes of `annotateUsecaseElements` (`data-usecase-id`, `data-usecase-kind`, `role`, `aria-label`), the layout, the paint, the font subsets, the root with its `viewBox` (the drawn box plus `diagramPadding` on each side) and the fonts as custom properties on the root.
- **`style.rs`** ports the stylesheet (`styles.ts`): the role colors (each of the eight `usecase*` theme variables with its fallbacks), the palette rules of `genColor`, and the rules of actors, use cases, boundaries, notes, JSON tables, relationships and markers. Colors are written as `var(--name, value)`; under `colorScheme: auto` the rules of the dark set come after the light ones in a media query ([0011](../decisions/0011-themes-as-css-variables.md)).
- **Shapes.** `usecaseActor` and its three variants (figure of 56 by 72, label under it, stereotype above the label, the business marker as a chord at 60 degrees, an invisible outline that sizes the node), `usecaseEllipse`, `usecaseBusiness` (the ellipse around label and stereotype, with the marker), `usecaseJsonTable` and the boundary `usecaseSystemBoundary` (`clusters.js`: a rectangle with its title at the top, a tab for `type: package`). A `[Rect]` use case is the shared `rect` shape, and a note the shared `note` shape.
- **Layout.** The use case diagram uses ELK Layered through the shared glue. Ellipses set `spreadPorts`, which gives them `elk.portAlignment.default: CENTER`. Boundaries are ELK subgraphs.
- **Markers.** The point, circle and cross markers are the flowchart's under the type's own name; the extension marker of a generalization is the type's own, `<svgId>_usecase-extension…`, with a `-margin` variant for the neo look.

The sections of the `usecase` configuration that the renderer reads are `wrappingWidth`, `minNodeWidth`, the six font settings, `diagramPadding`, `useMaxWidth` and `colorScheme`. The defaults of the section are `look: neo` and `theme: redux-color`, which the configuration layers resolve ([0008](../decisions/0008-canonical-model-and-typed-views.md)).

## Differences from Mermaid

- **Labels are SVG text**, not HTML in `foreignObject` ([0009](../decisions/0009-svg-text-labels.md)). The folded stereotype's first line gets its `usecase-stereotype` class directly, where the reference adds it with a DOM query.
- **The layout is always ELK Layered** ([0012](../decisions/0012-layout-engines-ported-exactly.md)). The reference accepts `layout:` with any registered algorithm and falls back to ELK for an unknown one; ferrigura lays out any other value with Layered as well.
- **Only the two redux themes ship** ([0011](../decisions/0011-themes-as-css-variables.md)). A fixture that names another theme is drawn with the one it falls back to.
- **`look: handDrawn` draws the plain neo or classic elements.** roughjs is not ported, and the `isHandDrawn` splits of the stylesheet are not ported either.
- **Error messages are ferrigura's own** ([0007](../decisions/0007-hand-written-parsers.md)): a rejected text is an error at a position, and the differential run compares acceptance, not the text. The specs assert that a text fails and where.
- **No icon packs.** An actor with an `icon` metadata draws the reference's fallback symbol, which carries the `usecase-actor-icon-fallback` class.
- **JSON tables.** A lone surrogate written as a `\u` escape becomes U+FFFD, since a Rust `String` holds scalar values only; objects and arrays nested deeper than 512 levels are an error.

## Known gaps

- `look: handDrawn` and icon packs (above).
- `usecase.nodeSpacing` and `usecase.rankSpacing` are not read by the renderer: ELK takes the unified renderer's spacing. The geometry run is within tolerance on the fixture that sets both (`10-configuration-and-accessibility`), but the options have no effect.
- Layout data has no differential run of its own: `getData`'s output is checked by Rust specs against the reference's spec expectations, and by the geometry run through the SVG, not field by field against the reference (there is no `usecase-data` run).
- The type has fixtures but no probes under `tools/port/geometry/probes`.

## Checking it

- `pnpm -C tools/port port:diff usecase` compares the canonical model, the AST and the JSON rows on every input: the fixtures, the string literals of Mermaid's specs, the documentation examples and the cases of `tools/port/cases.ts` (the grammar's probes, one input each). There are no exceptions.
- `pnpm -C tools/port port:diff config` includes the `usecase` section's defaults.
- `pnpm -C tools/port port:geometry usecase` renders the fixtures in `e2e/diagrams/usecase` with both and compares node positions and sizes, edge points and label boxes within 0.05 px. The `look: handDrawn` fixture is left out by the harness's rule. Colors are not compared, and the fixtures that name a theme ferrigura does not ship (`themed-elements-base-theme`, `-dark-theme`, `-default-theme`, `-forest-theme`, `-neutral-theme`) are rendered with the theme they fall back to. Four fixtures write their text with HTML entities (`&lt;&lt;Employee&gt;&gt;`); the harness decodes them (`decodeFixtureEntities`) before both sides parse.
- The specs of `ferrigura-syntax`, ported from Mermaid's: `use_case_lexer_spec`, `use_case_parser_spec`, `use_case_publication_spec`, `use_case_spec`, `use_case_db_spec`, `use_case_json_spec` and `use_case_docs_spec`, which parses the examples of the syntax page (`tests/data/usecase.md`). Those the public `parse` cannot isolate are ignored with their reason: the grammar-level tests whose text the model builder rejects, and the one that reads the raw Chevrotain statements.
- The specs of `ferrigura-render`: `use_case_data_spec` (the layout data and the color slots), `use_case_render_spec` (the stylesheet and the role colors) and `use_case_quirks_spec` (one test per quirk of the reference's renderer, below).
- The snapshots (`crates/ferrigura/tests/snapshots/svg.tsv`) pin the output of every fixture.

## Learnings

The grammar's rules are in [Parsing](#parsing). These are the ones that the renderer and the harness add.

- **A label is measured at the theme's font, and drawn at the diagram's.** `labelHelper` measures the label at the theme's font size (14 px for `redux-color`) before the shape adds its class, so the ellipse and the figure are sized by that box, and the actor's and business use case's labels are drawn afterwards at `actorFontSize` and `usecaseFontSize`. The font families of the section are written into custom properties only; they never change the measured width (which is why a fixture with `usecaseFontFamily: Georgia` still compares). `NodeFields::drawn_font_size` carries the drawn size.
- **An edge label is 10 px in a 14 px box.** The stylesheet sets `.edgeLabel` to 10 px, but its group is the box measured at 14 px plus 2 px on each side. The text starts at minus half of its drawn width, which is the measured width scaled to 10 px; the measured box is float32, so the drawn width agrees with the reference to about a hundredth of a pixel.
- **The stereotype fold differs by shape.** An ellipse or a rectangle gets `«stereotype»<br/>label` as a Markdown label, the Markdown markers (`* [ ] \ _` and the backquote) backslash-escaped in both parts. Only the ellipse's first line takes `usecase-stereotype` (the reference's captures give a folded rectangle none). An actor and a business ellipse keep the stereotype as a label of their own, above the figure or the label.
- **Escaping runs after sanitizing, and the names run before escaping.** `getData` sanitizes a label, then `draw` escapes a plain label (`&`, `<`, `>`) while the accessible name is made from the sanitized label. DOMPurify returns a text without `<` unchanged, so `&` stays, and a text with `<` is serialized, so `<` becomes `&lt;` before the escape makes it `&amp;lt;`.
- **A Markdown label is not centered by half its width.** Labels are start-anchored at the origin, so `createText` does not shift the group left; the shape centers the box with `translate(-w/2 - x, …)`.
- **An empty diagram has the padding around nothing:** `viewBox="-20 -20 40 40"` and `max-width: 40px`.
- **A property of a class keeps the place of its first write and the value of its last.** `getCompiledStyles` merges the `default` class and the node's classes by property, and a JSON table's duplicate key keeps its first position and its last value, as `JSON.parse` and the reference's key walk do.
- **JavaScript's number text.** A JSON cell shows `String(value)`: `Infinity`, `0` for `-0`, the shortest digits that read back (an exact tie takes the even digit), exponent form outside 1e-6 to 1e21, and an integer above 2^53 rounded (`12345678901234567890` is `12345678901234567000`). `serde_json`'s default text is not that, so `json.rs` formats the number itself.
- **The business marker's chord** is computed with V8's `cos` and `sin` of `π/3` (`ferrigura-jsmath`) and a plain `sqrt`, which is exact.
- **Entities.** `#35;`-style entities reach the model as placeholders and are decoded at render time, in the label path. The reference's e2e viewer decodes entities before parsing, which is why the geometry harness does so for the fixtures that write them.
