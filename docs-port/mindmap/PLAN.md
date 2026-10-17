# P2 mindmap parser: plan

Status: the parser, the DB and the model are ported, with the differential run and the specs (sections 7 and 8). Reference: the TypeScript reference of this worktree. Research and the probes behind it are in [NOTES.md](./NOTES.md); this page says what the port does and which quirks it keeps. Quirk ids `M1`-`M16` are used in the code comments and in the differential's `cases.ts` entries. The renderer stage (`getData`, the cose-bilkent hand-off, the shapes and the style sheet) is not in this page.

## 1. Sources

| File                                                             | Used for                                                                                          |
| ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `diagrams/mindmap/parser/mindmap.jison`                          | Lexer rules (lines 19-62) and grammar (70-126), in Jison's first-match order                      |
| `diagrams/mindmap/mindmapDb.ts`                                  | `addNode` (77-127), `getType` (129-147), `decorateNode` (156-169), `getMindmap` (73-75)           |
| `diagrams/common/common.ts`                                      | `sanitizeText` (71-83) and `sanitizeMore` (56-69): DOMPurify, then `FORBID_TAGS: ['style']`       |
| `Diagram.ts` (19-85), `preprocess.ts`, `diagram-api/comments.ts` | Order of `clear`, `encodeEntities`, `+ "\n"`; line breaks, directives, comment lines, `trimStart` |
| `utils.ts` (874-906), `assignWithDepth.ts` (30-80)               | `encodeEntities`; the merge of the configuration, which keeps a number over an array or object    |

The Rust code: `crates/ferrigura-syntax/src/mindmap/` (`mod.rs` entry point and grammar, `scanner.rs` lexer, `db.rs` DB), the model in `crates/ferrigura-model/src/mindmap.rs`, the exporter `tools/port/export-mindmap.ts`.

## 2. Text before the lexer

The steps of `Diagram.fromText` after `preprocessDiagram`, in the reference's order, are those of the journey port: CRLF and CR to LF and attribute quotes (`prepare`, shared); front matter and directives removed (`remove_directives`, shared); comment lines removed and the text trimmed at the start (`cleanup_comments`, shared); `encodeEntities` (`encode_entities`, shared: `#name;` becomes `ﬂ°name¶ß`); one `\n` appended.

The configuration is read from the same text (`config::read`) before the parse, because the DB reads `mindmap.padding` and `mindmap.maxNodeWidth` when it adds each node (section 4.2, M13).

## 3. Scanner

Jison's `next()` takes the first rule that matches at the current position, not the longest. The rules run in the order of `mindmap.jison`. The states are exclusive (`%x`): a rule without a state belongs to the initial state only. The keyword `mindmap` is case-insensitive and needs a word boundary after it (M3).

Initial state:

| #   | Rule                                                                        | Token or action                                                                      |
| --- | --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| 1   | `\s*%%.*`                                                                   | SPACELINE: the blanks and line breaks before the `%%`, and the rest of the line (M1) |
| 2   | `mindmap` and a word boundary                                               | MINDMAP (M3)                                                                         |
| 3   | `:::`                                                                       | enters CLASS                                                                         |
| 4   | `::icon(`                                                                   | enters ICON                                                                          |
| 5   | `[\s]+[\n]`: white space whose last line break is after its first character | SPACELINE, through that line break (M2)                                              |
| 6   | `[\n]+`                                                                     | NL                                                                                   |
| 7   | `-)`, `(-`, `))`, `)`, `((`, `{{`, `(`, `[`                                 | NODE_DSTART with the text, enters NODE (M7)                                          |
| 8   | `[\s]+`                                                                     | SPACELIST: its length in characters is the level                                     |
| 9   | `[^\(\[\n\)\{\}]+`                                                          | NODE_ID: everything up to a bracket, spaces and `:`, `%`, `]` included (M6)          |
| 10  | end of the text                                                             | EOF                                                                                  |

NODE (a shape, after its opening token): `"` followed by a backtick enters NSTR2; `"` enters NSTR; `))`, `)`, `]`, `}}`, `(-`, `-)`, `((`, `(` leave the state with NODE_DEND; `[^\)\]\(\}]+` is NODE_DESCR, and `.+` (the rest of the line) takes a lone `}` (M8).

NSTR2 (between backticks): `[^`"]+` is NODE_DESCR; `` `" ``leaves. NSTR (between double quotes):`[^"]+`is NODE_DESCR;`"` leaves.

ICON: `[^\)]+` is ICON, which may span lines; `)` leaves (M5). CLASS: `.+` is CLASS and leaves; a line break leaves with no token (M4).

Jison's `done` flag: when the text is used up, the rules of the current state are read once more. Only the initial state has the end rule, so a text that ends inside a shape, an icon or a class gives Jison's raw end symbol, which the grammar does not accept (M10).

## 4. Grammar and DB calls

```
start      : mindMap | spaceLines mindMap
spaceLines : SPACELINE (SPACELINE | NL)*
mindMap    : MINDMAP [NL] document
document   : (statement stop)+
stop       : (NL | EOF | SPACELINE) (NL | EOF)*
statement  : SPACELIST node          addNode(length of SPACELIST, node)
           | node                    addNode(0, node)
           | SPACELIST ICON | ICON   decorateNode({icon})
           | SPACELIST CLASS | CLASS decorateNode({class})
           | SPACELINE | SPACELIST   nothing
node       : NODE_ID [NODE_DSTART NODE_DESCR NODE_DEND] | NODE_DSTART NODE_DESCR NODE_DEND
```

The grammar is read from the tokens by a recursive recogniser (`parse_tokens`). A `SPACELINE` after a stop is the first token of a statement, since a stop takes only line breaks after its first token. A statement that is `SPACELIST` alone must be followed by a stop (M4, M14).

### 4.1 Calls and their order

The grammar reads the whole text first, and the DB calls then run in document order. A rejected text gives no model; the only difference this makes is which error a text with two faults reports, and the reference's own text is not compared (section 9).

### 4.2 `addNode(level, id, descr, type)`

- The first node is the root: `baseLevel` is its level, and its own level is 0 (M11).
- Later nodes: `level - baseLevel`. The parent is the last node in the DB's list with a smaller level (`getParent`). With no parent, the text is rejected with `There can be only one root. No parent could be found for ("<descr>")`, the exact message of the reference, with the sanitised descr (M11).
- `nodeId` and `descr` are `sanitizeText` of the id and the description (M16).
- `width` is `mindmap.maxNodeWidth` (200 by default); `padding` is `mindmap.padding` (10), doubled for shapes 1, 2 and 6 (M13).

### 4.3 `getType(start, end)`

| Start | End       | Type                                 |
| ----- | --------- | ------------------------------------ |
| `[`   | any       | 2 RECT                               |
| `(`   | `)`       | 1 ROUNDED_RECT                       |
| `(`   | any other | 4 CLOUD                              |
| `((`  | any       | 3 CIRCLE                             |
| `)`   | any       | 4 CLOUD                              |
| `))`  | any       | 5 BANG                               |
| `{{`  | any       | 6 HEXAGON                            |
| other | any       | 0 DEFAULT (`(-` and `-)` starts, M7) |

### 4.4 `decorateNode`

`decorateNode({icon})` and `decorateNode({class})` set the last node added, replacing an earlier value; the value is `sanitizeText`ed (M15). With no node yet, the reference throws a `TypeError` on the `undefined` node when the value is not empty, so the text is rejected (M12).

### 4.5 Sanitising

`sanitizeText` runs the DOMPurify subset of `sanitize.rs` once. The reference runs DOMPurify twice under the default configuration (`removeScript`, then the `style` removal), as the journey port notes (J15); the differential shows that one pass is enough for every input collected and every probe (M16).

## 5. Model

`MindmapDiagram` with `version` 1 and `root`: the first node, or `null` when the text has no node. A node has `id` (the DB's counter, the position in the order the nodes were added), `nodeId`, `descr`, `type` (the shape number), `level` (the indentation relative to the root), `width`, `padding`, `icon` and `class` (`""` when absent), and `children`. Field names are camelCase.

`width` and `padding` are the configuration's values as the reference's JavaScript holds them: a number, unless the configuration gave a value of another type (M13). The model holds the DB's values, so a text that is not sanitised by the DB keeps its entity placeholders (`ﬂ°°35¶ß`), which the renderer decodes.

Left out, because the DB holds them only after the renderer's stage, or because they follow from the others: `isRoot` (the root is the first node), `section`, `x`, `y`, `height` and `labelType` (set by `getData` and the layout), `getData`'s `shapes`, `markers`, `direction`, `nodeSpacing`, `rankSpacing`, `type` and `diagramId` (dead, or set by `getData`: NOTES section 3.5).

## 6. Quirks reproduced

| Id  | Quirk                                                                                                                                                                                                                                                                                          | Where                     |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------- |
| M1  | A comment takes the blanks and line breaks before its `%%`: `root`, a line break and `  %% c` leave one SPACELINE. After a shape, `%%` starts a comment (MMP-22); after a plain id it is text (`child %% trailing` is the id)                                                                  | rule 1, NODE_ID           |
| M2  | White space with a line break after its first character is one SPACELINE: blank rows collapse, and the indentation of the row after them is a SPACELIST of its own. A lone line break is NL                                                                                                    | rules 5, 6                |
| M3  | `mindmap` is a keyword only as a whole word, case-insensitively: `mindmapper` is an id, `MindMap` the keyword                                                                                                                                                                                  | rule 2                    |
| M4  | `:::` followed at once by a line break takes the line break with no token. The indentation of the next line then follows the statement's indentation, so `root`, `  :::`, `  child` is rejected                                                                                                | CLASS, statement          |
| M5  | `::icon()` sets nothing; an icon may span lines (`::icon(a⏎b)` is `a⏎b`); the last icon wins                                                                                                                                                                                                   | ICON, decorateNode        |
| M6  | An id is everything up to a bracket, so it keeps spaces, trailing blanks, `:`, `%`, `]` and `:::` (`child:::hot` is one id); `root-)` has the id `root-`; `}` cannot be in an id                                                                                                               | rule 9                    |
| M7  | The shape is decided by the opening and closing tokens (`getType`, table 4.3). `root(-Root-)` is DEFAULT with the text `Root-`, since `(-` is not in `getType`                                                                                                                                 | rule 7, `getType`         |
| M8  | Descriptions: `"…"` may hold brackets and span lines; `"`…`"` keeps the backticks out (``"`**b**`"`` is `**b**`) while `[`…`]` keeps them (`` `**b**` ``); `(` ends a description, and a lone `}` takes the rest of the line                                                                   | NODE, NSTR, NSTR2         |
| M9  | Text after a shape is an error (`root(a)b`), and so is a description with a quote in it (`root["a"b]`)                                                                                                                                                                                         | grammar                   |
| M10 | A text that ends inside a shape, an icon or a class is rejected: the end is Jison's raw end symbol, which the grammar does not accept                                                                                                                                                          | `done` flag               |
| M11 | The first node is the root, whatever its indentation. A node is attached to the nearest earlier node of smaller indentation; no such node is the root error. Levels are columns, a tab is one column                                                                                           | `addNode`                 |
| M12 | An icon or a class before any node is rejected: the reference's `decorateNode` throws on `undefined`                                                                                                                                                                                           | `decorateNode`            |
| M13 | `mindmap.padding` is doubled for shapes 1, 2 and 6 (`*= 2`, with JavaScript's `ToNumber`). A configuration value that is not a number is kept as it was given (a string, a boolean). An array or an object does not replace the default number (`assignWithDepth`)                             | `settings_of`, `add_node` |
| M14 | The header takes a statement, and a statement needs a stop. A blank row right after `mindmap` is a statement with no stop before the next node, so `mindmap`, a blank row and `root` is rejected; `mindmap` alone is rejected, and `mindmap` followed by one line break is a text with no node | grammar, `stop`           |
| M15 | The last icon and the last class of a node win, and each is sanitised                                                                                                                                                                                                                          | `decorate_*`              |
| M16 | Entities stay as the DB keeps them (`#35;` is `ﬂ°°35¶ß`); `<`, `>` and `&` are sanitised as the flowchart does (`<script>x</script>y` is `y`, `<br/>` is `<br>`, `a < b` is `a &lt; b`)                                                                                                        | `sanitize_text`           |

## 7. Specs

| Reference spec                               | Rust file                                                |  Tests | Live | Notes                                                                                            |
| -------------------------------------------- | -------------------------------------------------------- | -----: | ---: | ------------------------------------------------------------------------------------------------ |
| `diagrams/mindmap/mindmap.spec.ts`           | `crates/ferrigura-syntax/tests/mindmap_spec.rs`          |     26 |   26 | the reference drives the Jison parser; the Rust reads the model, `toThrow` is an `Err`           |
| `diagrams/mindmap/mindmapColors.spec.ts`     | `crates/ferrigura-syntax/tests/mindmap_colors_spec.rs`   | 8 runs |    2 | the colour sections are `getData`'s and are ignored; the classes, icons and empty trees are live |
| `diagrams/mindmap/mindmapDb.getData.test.ts` | `crates/ferrigura-syntax/tests/mindmap_get_data_spec.rs` |     12 |    0 | `getData` is the renderer stage (section 5): ignored with the reason                             |
| `diagrams/mindmap/mindmapLayout.spec.ts`     | `crates/ferrigura-syntax/tests/mindmap_layout_spec.rs`   |      3 |    0 | the layout selection is `getData`'s: ignored with the reason                                     |

49 runs in all: 28 passing and 21 ignored. The ignored ones name the renderer stage. The renderer has no spec of its own in the reference that the parser task touches (NOTES section 5), so no render test is added here.

## 8. Differential

`tools/port/export-mindmap.ts` runs the reference's Jison parser with `parser.yy` set to the reference's `MindmapDB`, after the config steps of `processAndSetConfigs` (reset, the detected scope, the directives), the text steps of `Diagram.fromText`, and `encodeEntities` with a line break. The model is the tree from `getMindmap()`, with `width` and `padding` written as JSON (a non-finite number as its sentinel string, `value.rs`). The reference's rejections are rejections on both sides.

`cases.ts` holds the probes of NOTES section 2.4 and the quirks above (M1 to M16), the configuration cases (padding and width as strings, booleans, arrays and objects, and the front matter forms), and the case-sensitive `MindMap` in `undetectedCases`, which the detector does not take.

Result (`pnpm port:diff mindmap`): 248 inputs, 212 accepted by the reference, 0 differences, no named exception. `pnpm port:diff config`: the mindmap section is compared for all 248 mindmap inputs of the configuration run, with no difference.

Beyond the collected inputs, the port was checked outside the repository (scratch files, not committed) against the reference's own parser: every text of up to four characters over a 14-character alphabet of the syntax characters after `mindmap\n` (121,158 texts, 15,241 accepted), and 12,000 generated documents of random and well-formed trees with icons, classes, quoted and backtick descriptions, comments, blank rows and configuration. None differs in acceptance or in the model.

## 9. Open questions

1. A configuration that puts an array in place of the `mindmap` section is merged element by element by `assignWithDepth`, which the port does not do: `mindmap: [{padding: 3}]` gives padding 3 in the reference. That is the configuration layer's merge (`config::read`), not the mindmap's; it is left to the configuration task.
2. The error texts of Jison and of the engine's `TypeError` are not reproduced; the differential compares acceptance only for them. The DB's own message is reproduced exactly, as the specs check it.
