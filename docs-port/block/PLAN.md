# Block diagram parser: plan

Status: parser and renderer ported. Reference: the TypeScript tree of this worktree (`block.jison`, `blockDB.ts`, and the renderer's `blockRenderer.ts`, `renderHelpers.ts`, `layout.ts`, `styles.ts`). The research and the probes behind it are in [NOTES.md](./NOTES.md); this page says what the port does, which quirks it keeps, and what the differential checks. Quirk ids `B1`-`B22` are the rows of section 8; the code comments name the reference's functions and lines instead.

## 1. Sources

| File                                                                                           | Used for                                                                               |
| ---------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| `diagrams/block/parser/block.jison` (lines 11-162 lexer, 164-292 grammar)                      | Rule order, start conditions, and the grammar's actions                                |
| `diagrams/block/blockDB.ts` (105-192 populate, 211-293 helpers, 295-313 hierarchy and columns) | `populateBlockDatabase`, `addStyleClass`, `addStyle2Node`, `setCssClass`, `generateId` |
| `diagrams/common/common.ts` (`sanitizeText`, 56-83)                                            | Labels: `sanitizeMore`, then DOMPurify with `FORBID_TAGS: ['style']`                   |
| `Diagram.ts` (`fromText`)                                                                      | Order of `clear`, `yy` assignment, `encodeEntities`, `+ "\n"`                          |
| `preprocess.ts`, `diagram-api/comments.ts`                                                     | Line breaks, attribute quotes, front matter, directives, comment lines                 |
| `diagrams/block/blockTypes.ts`, `layout.ts` (the layout is in the renderer, section 9)         | The field names; the layout is the renderer's                                          |

The Rust code: `crates/ferrigura-syntax/src/block/` (`mod.rs` entry point and preprocessing, `scanner.rs` lexer, `grammar.rs` parser and the statements, `db.rs` DB and model), the model in `crates/ferrigura-model/src/block.rs`, and the exporter `tools/port/export-block.ts`.

## 2. Text before the scanner

The steps of `Diagram.fromText`, shared with the other ports (`prepare`, `remove_directives`, `cleanup_comments`, `encode_entities`, `+ "\n"`). The front matter title is read and dropped: the block DB has no title setter. The text the scanner reads is the encoded text with a line break at the end.

## 3. Scanner

Jison's `next()` takes the first rule that matches at the current position, not the longest. The rules of the initial state are checked in this order (the reference's line numbers):

| #   | Rule                                                                                                                                                       | Token                                          |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| 1   | `block:`                                                                                                                                                   | `id-block`                                     |
| 2   | `block` and a word boundary (39-40)                                                                                                                        | `BLOCK_DIAGRAM_KEY`                            |
| 3   | one or more JavaScript white space, line breaks included (43; the `NL` rules after it are dead)                                                            | skipped                                        |
| 4   | `columns`, white space, `auto` and a word boundary (47)                                                                                                    | `COLUMNS` = -1                                 |
| 5   | `columns`, white space, digits (48)                                                                                                                        | `COLUMNS` = the number                         |
| 6   | `"` and a backtick (49), `"` (52)                                                                                                                          | enters `md_string`, enters `string` (no token) |
| 7   | `space:` and digits (55), `space` and a word boundary (56)                                                                                                 | `SPACE_BLOCK` = the number, or 1               |
| 8   | `default`, `linkStyle`, `interpolate`, each with a word boundary (57-59)                                                                                   | keywords the grammar does not use              |
| 9   | `classDef`, white space (61); `class`, white space (66); `style`, white space (70)                                                                         | enters the statement's state                   |
| 10  | `accTitle`, `accDescr` with `:` and white space (74, 76); `accDescr {` (78)                                                                                | the accessibility states                       |
| 11  | `end` and a word boundary, then white space (81)                                                                                                           | `end`                                          |
| 12  | the shape openers, in the order of lines 103-121: `-)`, `(-`, `))`, `)`, `(((`, `((`, `{{`, `{`, `>`, `([`, `(`, `[[`, `[\|`, `[(`, `)))`, `[\`, `[/`, `[` | `NODE_DSTART`, enters `NODE`                   |
| 13  | `<[` (123)                                                                                                                                                 | `BLOCK_ARROW_START`, enters `BLOCK_ARROW`      |
| 14  | `[^\(\[\n\-\)\{\}\s\<\>:=]+` (125)                                                                                                                         | `NODE_ID`                                      |
| 15  | the end of the input (126)                                                                                                                                 | `EOF`                                          |
| 16  | the links `\s*[xo<]?` and a run of `-`, `=`, or `.-`, or `~~~`, with an optional mark and blanks (148-151)                                                 | `LINK`                                         |
| 17  | the same prefixes with `--`, `==` or `-.` and no closing mark (152-154)                                                                                    | `START_LINK`, enters `LINK_LABEL`              |
| 18  | `:` and digits (160)                                                                                                                                       | `SIZE`                                         |

The other states (`string`, `md_string`, `NODE`, `BLOCK_ARROW`, `ARROW_DIR`, `LINK_LABEL`, `classDef`, `class`, `style` and the accessibility states) have the rules the grammar's `%x` states give (lines 49-160). Two things hold across them:

- A keyword whose rule ends in a word character gets Jison's implicit word boundary: `blockA`, `spaceship`, `defaults`, `columnsX`, `endpoint`, `classy` and `stylish` are node ids (B3). A keyword followed by a `\s+` needs its blank, so there is no boundary to add.
- The rules that match an empty text (`[^\n]*`, `[^"]*`, `[^\}]*`) return a token even when empty. At the end of the input the scanner gives Jison's `done` behaviour: the first call that starts with the input empty reads the rules of the current state, and every later call returns Jison's raw end symbol, which the grammar does not accept.

## 4. Grammar and DB calls

```
start      : BLOCK_DIAGRAM_KEY document EOF
document   : statement+                      (composite: until end)
statement  : nodeStatement                   (node, [SIZE], then (link node)*)
           | COLUMNS                         column-setting
           | SPACE_BLOCK                     space, generateId()
           | id-block nodeStatement document end   composite (generateId() dropped)
           | BLOCK_DIAGRAM_KEY document end  composite, generateId()
           | classDef CLASSDEF_ID STYLEOPTS  classDef
           | class CLASSENTITY_IDS STYLECLASS applyClass
           | style STYLE_ENTITY_IDS STYLE_DEFINITION_DATA applyStyles
node       : NODE_ID [ NODE_DSTART STR NODE_DEND | BLOCK_ARROW_START STR DIR+ BLOCK_ARROW_END ]
link       : LINK | START_LINK LINK_LABEL STR LINK
```

Each action gives the same objects as the reference's. A plain node is `{id, label, type, directions, widthInColumns}` with the span `1` or the `SIZE`. A link gives three items, `[left, edge, right]`, where the left block keeps its id, label, type and directions but not its span, and the right block has no span; a longer chain keeps only the last link's three items, and in a longer chain the last link's left block has no fields, as the reference's array has none (B2, B5). The shape text is the opener and the closer (`[` and `]` give `[]`, `((` and `))` give `(())`, and so on), mapped by `typeStr2Type`; `<[]>` is a block arrow.

Parse errors are the first token the grammar rejects, and the first text no rule takes; both carry their position (ADR 0002).

## 5. DB

The reference keeps one object per block, and the DB map and the tree hold the same objects. The port keeps the objects in an arena, and the map and the tree hold their indices. That keeps the identities the reference relies on: a later mention can change an object the tree does not show (B7, B11, B12). The populate step follows `populateBlockDatabase` item by item, in list order:

1. Labels are sanitised when non-empty (edge labels too). A label that sanitises to empty takes the id, as it is defaulted after sanitising (B17).
2. `classDef` → `addStyleClass`; `applyClass` → `setCssClass`; `applyStyles` → `addStyle2Node`, which is an error when no block has the id, and not called when the definition is empty.
3. `column-setting` → the parent's `columns`.
4. An edge joins the edge list, with its id prefixed by the count of its key (B13).
5. Anything else: the default label; an id seen before keeps its first position, takes the new type unless the new one is `na`, and takes the new label when it differs from the id (B12); a new composite takes the next palette index before its children are populated, so outer composites come first (`colorIndex`). The object is registered, its children are populated, and it joins the parent's children unless it was already known. A space with span `w` becomes `w` cells.

`clear` resets the DB, with the root registered under `root` (B11). Warnings are the reference's `log.warn` of a block wider than the first `columns` of its list; `parse_with_warnings` returns them.

## 6. Model

`BlockDiagram` with `version` 1 and camelCase keys: `columns` (the columns of the entry registered as `root`, absent when that entry has none), `blocks` (the root's children in order), `edges`, `classDefs`. A block has `id`, `type`, `label`, `width` (on space cells), `widthInColumns`, `columns`, `directions`, `classes`, `styles`, `colorIndex` and `children`, each left out when the reference leaves it `undefined`. An edge has `id`, `start`, `end`, `label`, `thickness`, `pattern`, `directions`, `arrowTypeEnd` and `arrowTypeStart`; its `type` is always `edge` and is left out. A class has `id`, `styles` and `textStyles`. The one `undefined` that is written is the object of a chained link's first block: an empty object (B2).

Scalars the reference computes as `parseInt` are numbers (`f64`, JavaScript's number). Blanks are kept where the reference keeps them: in directions and styles (B18).

## 7. Generated ids

The reference draws the ids of space cells and of anonymous composites with `Math.random`. The port generates `id-<twelve digits>-<n>`, with the counter `n` of the parse in both places: the same format, deterministic (`generate_id`). Because a user's node id cannot contain `-` (NODE_ID excludes it), every block id with a `-` is a generated one, and a space cell's suffix is `-<j>`.

The differential replaces the generated ids on both sides by placeholders, in tree order: the base `id-<random>-<n>` becomes `$genK` for the k-th base first seen, the suffix `-<j>` stays, and a space cell's label (its base) takes the same placeholder. The JavaScript side (`normaliseGenerated` in `export-block.ts`) and the Rust side (`normalise_block_ids` in the `canonical` example) do the same. The counter itself is not compared.

## 8. Quirks reproduced

| Id  | Quirk                                                                                                                                                                                                                                                             | Reference                          | Port                                            |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------- | ----------------------------------------------- |
| B1  | A diagram with no statement is rejected (`block` alone, or `block` followed by blanks). A header `block:x` at the start is rejected, though the detector accepts it                                                                                               | `start`, `document`                | `grammar::start`, `grammar::document`           |
| B2  | `A --> B --> C` gives a first block with no id, label or type, and the edge `1-undefined-C` with no start. A second chain's first block is dropped, because the map already holds the `undefined` key                                                             | `nodeStatement`, populate step 5   | `grammar::chain`, `db::add_block`               |
| B3  | The first rule wins, so ids may contain `"`, `]`, `.`, `~`, `o` and `x`: `A o--o B` has the node `o`; `A ~~~ B` has the node `~~~`; `x-y` is rejected; keywords are prefixes (`blockA`, `spaceship`, `columnsX`)                                                  | `%lex`                             | `scanner::initial`, `scanner::node_id_len`      |
| B4  | Edges: `---` has no end arrow; `====` and `==>` are thick; `-.`, `-.-` and `-.->` are dotted; `<-->` has a point at each end; a link with no leading mark starts with `arrow_open`                                                                                | `edgeStrToEdgeData` and the others | `db::edge_str_to_*`                             |
| B5  | `A:2 --> B` drops the span of `A`, since an edge's nodes copy only id, label, type and directions. `A --> B:2` is rejected: a span follows a standalone node only                                                                                                 | `nodeStatement` rules              | `grammar::node_statement`                       |
| B6  | `style` for an id that no block has yet is an error; `style A,B …` is an error (ids are not split); `style A` with no definition is not an error                                                                                                                  | `addStyle2Node`, `if (stylesStr)`  | `db::add_style_to_node`, `Db::populate`         |
| B7  | `class` before the declaration creates a DB entry outside the tree, so the later node is not drawn (Q11). `class A, B` with a blank before `B` looks up ` B` untrimmed, and replaces an existing `B` entry: the tree's `B` gets no class                          | `setCssClass`                      | `db::set_css_class`                             |
| B8  | `class A` with no class name applies the empty class `""`                                                                                                                                                                                                         | `cssClassStatement`, `setCssClass` | `grammar::statement`, `db::set_css_class`       |
| B9  | `classDef x` followed by a line: the style options come from the next line, since the blank after the name takes the line break. `classDef DEFAULT` (uppercase) is rejected; `classDef default` names the class `default`. `classDef my-class` is a lexical error | `CLASSDEF` rules                   | `scanner::class_def`, `scanner::initial`        |
| B10 | Class styles: the first `;` of each style is removed; a style that mentions `color` also gives a text style, with the first `fill` made `bgFill` and then the first `color` made `fill`. The test is unanchored: `background-color` gives `background-fill`       | `addStyleClass`                    | `db::add_style_class`                           |
| B11 | `root` is registered from the start: a node or a class named `root` is absorbed by the root entry, and `class A, root` replaces that entry, so the root's `columns` are absent from the model                                                                     | `clear`, `setCssClass`             | `Db::new`, `Db::into_model`                     |
| B12 | A node declared again keeps its first place and span; its later label replaces the first when it differs from the id, and its type changes unless the new type is `na` (Q24, Q29)                                                                                 | populate step 5                    | `db::add_block`                                 |
| B13 | Edge ids count per key: `1-A-B`, `2-A-B`                                                                                                                                                                                                                          | `edgeCount`                        | `db::add_edge`                                  |
| B14 | `columns 0` is kept: the model has `0` (the renderer rejects it, Q12). `columns auto` gives `-1`; `columns 00007` gives `7`; `columns 3x` gives `3` and the id `x`                                                                                                | `columnsStatement`, `getColumns`   | `scanner::columns_*`, `grammar::statement`      |
| B15 | An anonymous `block … end` gets a generated id. `block:A-->B` (a chain as the header) gives a composite with no id, no label and no span: the spread of an array (Q30)                                                                                            | `blockStatement`                   | `grammar::id_block`, `composite_from`           |
| B16 | `space:0` gives no cell; `space` is one cell; a cell's id is `<id>-<j>` and its label is the generated id; the space itself is not in the tree                                                                                                                    | populate step 5, `space`           | `db::add_space_cells`                           |
| B17 | Labels are sanitised with the reference's DOMPurify, edges too; a label that sanitises to empty takes the id. Entity placeholders are kept (Q20, Q21); `<br>` is text for the renderer (Q22)                                                                      | `sanitizeText`, populate step 1    | `sanitize_text`, `db::add_block`                |
| B18 | Block arrow directions and styles keep the blanks as written: `( right , down )` gives `[" right ", "down "]`; `style A fill:#f00, stroke:#000` gives `[…, " stroke:#000"]`. The renderer expands `x` and `y` (Q28)                                               | `dirList`, `addStyle2Node`         | `scanner::arrow_dir`, `db::add_style_to_node`   |
| B19 | `accDescr {` followed by `}` is accepted and ignored (no token). A non-empty block, and `accTitle:` or `accDescr:` with a value, are rejected: no production uses them (Q14)                                                                                      | `acc_descr_multiline`, the tokens  | `scanner::accessibility_block`, `Token::Unused` |
| B20 | `linkStyle`, `interpolate` and a bare `default` are rejected; so are markdown strings (``"`…`"``, Q13) and the empty label `A[""]`                                                                                                                                | the tokens and the grammar         | `Token::Unused`, `grammar::node`                |
| B21 | Labels are quoted, with no escapes (`"a\"b"` is rejected, Q15), no blank after `[` (Q15), no unquoted text (`A[x]`), and no shape without a label (`A()`)                                                                                                         | `NODE` state                       | `scanner::node`, `grammar::node`                |
| B22 | A link label must be quoted and non-empty (`A -- x --> B` and `-- "" -->` are rejected). A block arrow's direction must be `right`, `left`, `up`, `down`, `x` or `y` (Q27); any other word is a lexical error                                                     | `LLABEL`, `ARROW_DIR`              | `scanner::link_label`, `scanner::arrow_dir`     |

## 9. Specs

| Reference spec                           | Rust file                                                 |        Tests | Notes                                                                                                                                                                    |
| ---------------------------------------- | --------------------------------------------------------- | -----------: | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `diagrams/block/parser/block.spec.ts`    | `crates/ferrigura-syntax/tests/block_spec.rs`             | 26 (25 pass) | The DB is read from the model. The `dompurifyConfig` test is ignored: the sanitiser runs DOMPurify's default configuration. The warning test uses `parse_with_warnings`. |
| `diagrams/block/blockDB.spec.ts`         | `crates/ferrigura-syntax/tests/block_db_spec.rs`          |     4 (pass) | The `getConfig` spy has no Rust form: the last test checks that sanitising happens during each parse.                                                                    |
| `diagrams/block/layout.spec.ts`          | `crates/ferrigura-render/tests/block_layout_spec.rs`      |   2 (3 pass) | The layout is the renderer's. The file also has the grid-of-zero-columns error.                                                                                          |
| `diagrams/block/blockColorIndex.spec.ts` | `crates/ferrigura-render/tests/block_color_index_spec.rs` |   8 (8 pass) | The colour slots are the renderer's: the `colorIndex` values from the parser, and the palette rules of the style sheet.                                                  |

## 10. Differential

`tools/port/export-block.ts` runs the reference's Jison parser with `parser.yy` set to the DB, after `preprocessDiagram`, `encodeEntities` and the line break, and reads the DB in the order of the renderer. The explicit cases in `cases.ts` hold the quirks above, the probes that the notes name, and the inputs of the accepted and rejected sets. The collected inputs come from the e2e fixtures, the spec strings and the Markdown examples: 264 inputs in all, 218 accepted by the reference, no difference.

Beyond the committed inputs, the port was also checked on about 12,000 generated inputs (random token sequences, and statements of the grammar with occasional noise), which were not committed. None differed.

## 11. Open questions and gaps

1. The `dompurifyConfig` of a site configuration is not read by the sanitiser; the one spec that tests it is ignored. The configuration port (M2) owns the setting.
2. The sanitiser runs DOMPurify once where the reference's `sanitizeMore` runs `removeScript` first; the flowchart port makes the same choice, and the differential shows no difference on the block inputs.
3. The renderer (`crates/ferrigura-render/src/block/`) is checked by `pnpm port:geometry block` (58 of 58 within tolerance). Its own pieces are `layout.rs` (the arithmetic of `layout.ts`), `nodes.rs` (`getNodeFromBlock`, the measure and placed passes, `insertEdges`), `style.rs` (`styles.ts`), and the shared shapes it adds under `unified/shapes/` (`composite.rs`, `block_arrow.rs`, with the sixteen point tables of `blockArrow.spec.ts`). Behaviour the geometry comparison does not state is in `tests/block_render_spec.rs`. The shapes that read the node's own width or height (the hexagon, the lean and trapezoid shapes, the double circle, the cylinder) take it now; the flowchart never sets one, so its output is unchanged (298 of 298).
4. Quirks that the renderer reproduces: the `undefined` block and the edge with no start of a chained link (B2, Q2); `height="NaN"` for a drawing of only spaces without `useMaxWidth` (a named exception: the attribute is written as the reference writes it); a hexagon label that wraps at equality (Q4); the repeated classes of a composite; `style="undefined;;;undefined"` on every edge. Not reproduced: the production default of HTML labels (decision 0004 draws SVG text, Q-B), and the generated ids, which are the port's own (section 7).
