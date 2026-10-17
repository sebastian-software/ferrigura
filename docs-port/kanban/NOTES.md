# Kanban (P2): research notes for the port

Status: research only. Nothing in the repository changed except this file, and nothing is committed. Reference: `packages/mermaid/src` at HEAD `aa5c4338f` (branch `sebastian/port-p2`). Plan: [../P2-PLAN.md](../P2-PLAN.md). Guide: [../P2-RESEARCH.md](../P2-RESEARCH.md). Packet's notes are the model for the shared steps: [../packet/NOTES.md](../packet/NOTES.md).

Every rule cites the reference as `diagrams/kanban/<file>:<line>` unless the file is named in full. **Probed** means the rule was run through the reference's own code. The parser and DB ran through `tools/port/reference.ts` (Vite SSR of the real `kanban.jison`, `kanbanDb.ts`, `preprocess.ts` and `config.ts`, with `preprocessDiagram` and `encodeEntities` applied as `Diagram.fromText` applies them, and the default configuration). The rendering ran through the built bundle `packages/mermaid/dist` in Chromium, with the harness's settings (`htmlLabels: false`, Lexica faces, `securityLevel: strict`, `redux-color`) unless a row says otherwise. **Unverified** means read or inferred only. Probe material is outside the repository in `/private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/p2/`: `probe-db.mts` with `cases*.ts`, `render-probe.mjs` with its `render-inputs*.json`, and `outline.cjs`, which prints the DOM of a rendered SVG.

## 0. Summary

Kanban is a Jison grammar with start conditions, a mutable DB whose `getData` is the render input, and a renderer that places cards by arithmetic and by the bounding boxes of its own labels. It shares the unified node and cluster helpers with flowchart and the label code with the other SVG-text labels, and it has two shape kinds of its own (`kanbanItem`, `kanbanSection`).

| Item                           | Value                                                                                                                                                                                                                                                        |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Reference, non-spec            | `kanban.jison` (166 lines), `kanbanDb.ts` (255), `kanbanRenderer.ts` (102), `styles.ts` (111), `kanbanTypes.ts` (3), `detector.ts` (23), `kanban-definition.ts` (13): 673 lines in total                                                                     |
| Shape and cluster code it uses | `shapes/kanbanItem.ts` (163 lines), `kanbanSection` in `clusters.js` (308-417), and the shared helpers `labelHelper`, `insertLabel` and `createText`                                                                                                         |
| Specs                          | `kanban.spec.ts`: 30 tests (22 in `when parsing a kanban`, 8 in `item data data`), 494 lines. No renderer spec. `samples.md` has 6 `mermaid` blocks                                                                                                          |
| Fixtures                       | 11 `.mmd` files: 10 in `e2e/diagrams/kanban/pr-check/` and 1 in `nightly-check/`. `visual-check/` is empty                                                                                                                                                   |
| Parser                         | Jison with start conditions and first-match rules. Hand port with a state-stack scanner, as decision 0002 says for this shape of grammar (the journey and timeline scanners are precedents)                                                                  |
| DB                             | Module state: `nodes`, `sections`, a counter, `getData`. Level-based hierarchy. Sanitised ids and labels, and YAML shape data                                                                                                                                |
| Renderer                       | Custom placement: section x from a width constant, item y from the measured height of each item, section height from the items. `getBBox` of the items and of the root `svg`                                                                                 |
| Riskiest parts                 | (1) Labels as SVG text with markdown lines and per-word `tspan`s, and their `getBBox`. (2) The lexer's quirks. (3) The default configuration renders HTML labels, not SVG text (section 6.5). (4) Section metadata is dropped before rendering (section 3.4) |
| Effort                         | Large, relative to packet. Rough estimate: 1,500 to 2,000 lines of Rust for the lexer, parser, DB, `getData`, the two shapes and the styles, plus tests                                                                                                      |

No exporter exists yet (`tools/port/export-kanban.ts` is absent), so `pnpm port:diff kanban` needs the scaffold first.

## 1. Sources

| File                                                     | Lines            | What it holds                                                                                                                                                                                                                                               |
| -------------------------------------------------------- | ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `diagrams/kanban/parser/kanban.jison`                    | 8                | `%options case-insensitive` (the whole lexer is case-insensitive)                                                                                                                                                                                           |
|                                                          | 13-20            | Start conditions: `NODE`, `NSTR`, `NSTR2`, `ICON`, `CLASS` (exclusive), `shapeData`, `shapeDataStr`, `shapeDataEndBracket`                                                                                                                                  |
|                                                          | 24-87            | The lexer rules, in their written order (table in section 2.2)                                                                                                                                                                                              |
|                                                          | 97-162           | The grammar: `start` (97-101), `mindMap` (109-112), `stop` (114-120), `document` (121-124), `statement` (126-137), `node` (141-155), `shapeData` (157-162)                                                                                                  |
| `diagrams/kanban/kanbanDb.ts`                            | 10-13            | Module state: `nodes`, `sections`, `cnt`, `elements`                                                                                                                                                                                                        |
|                                                          | 15-20            | `clear`                                                                                                                                                                                                                                                     |
|                                                          | 25-48            | `getSection`, which decides whether a node is a section or a child, and throws at 38-40                                                                                                                                                                     |
|                                                          | 50-52            | `getSections`                                                                                                                                                                                                                                               |
|                                                          | 54-97            | `getData`: the section node (62-71), the children (73-93) and the return (96)                                                                                                                                                                               |
|                                                          | 99-165           | `addNode`: the id (110), the label (112), the width and padding from `mindmap` (101-113), the shape data (118-155) and the placement (157-164)                                                                                                              |
|                                                          | 178-196          | `getType`: the bracket to the node type                                                                                                                                                                                                                     |
|                                                          | 202-214          | `decorateNode`: the icon (209) and the class (212), both sanitised                                                                                                                                                                                          |
|                                                          | 216-235          | `type2Str` (not used by the renderer)                                                                                                                                                                                                                       |
|                                                          | 241-253          | The `db` object                                                                                                                                                                                                                                             |
| `diagrams/kanban/kanbanRenderer.ts`                      | 12-98            | `draw`                                                                                                                                                                                                                                                      |
|                                                          | 19               | `conf.htmlLabels = false`, on a copy (section 6.5)                                                                                                                                                                                                          |
|                                                          | 25-27, 32-39     | The DOM ids (`${id}-${node.id}`) and the two groups, `sections` and `items`                                                                                                                                                                                 |
|                                                          | 42-63            | Sections: the constants (42, 45, 47), the position (51), the size (52-56), the class (59), `insertCluster` (60), and the label height (61)                                                                                                                  |
|                                                          | 65-89            | Items: the top (69), `item.x` (78), `item.width` (79), `insertNode` (80), the measured `bbox` (81), `item.y` (82), `positionNode` (83), the next y (84)                                                                                                     |
|                                                          | 86-88            | The section rect height                                                                                                                                                                                                                                     |
|                                                          | 92-97            | `setupGraphViewbox` with `mindmap.padding` and `mindmap.useMaxWidth`                                                                                                                                                                                        |
| `diagrams/kanban/styles.ts`                              | 6-74             | `genSections`: the colour loop (9-16), the section rules (21-72), with the `section-${i - 1}` selectors (24-26)                                                                                                                                             |
|                                                          | 77-110           | `getStyles`: the edge, the section root (83-85), the icon container, and the label colours                                                                                                                                                                  |
| `diagrams/kanban/kanban-definition.ts`                   | 8-12             | The definition: `db`, `renderer`, `parser`, `styles`                                                                                                                                                                                                        |
| `diagrams/kanban/detector.ts`                            | 9                | `/^\s*kanban/`: no word boundary                                                                                                                                                                                                                            |
| `diagrams/kanban/samples.md`                             | 1-105            | Six `mermaid` samples (docs, not specs)                                                                                                                                                                                                                     |
| `rendering-util/rendering-elements/shapes/kanbanItem.ts` | 9-22             | `colorFromPriority`                                                                                                                                                                                                                                         |
|                                                          | 23-163           | `kanbanItem`: the width (34), the ticket link (46-53), the labels (55-81), the restore of the width (82), the sizes (84-89), the label positions (90-105), the rect (107-135), the `__APA__` class (128), the priority line (137-152), the bounds (155-156) |
| `rendering-util/rendering-elements/clusters.js`          | 308-417          | `kanbanSection`: the group (317-321), the label (323-338), the width rule (346-351), `x` and `y` (354-355), the rect (371-389), the label position (391-396), and the bounds (406-409)                                                                      |
|                                                          | 806-816          | The `shapes` map of clusters (`kanbanSection` at 812), and 841-846, `insertCluster`                                                                                                                                                                         |
| `rendering-util/rendering-elements/shapes.ts`            | 79, 562          | `kanbanItem` is registered as a node shape                                                                                                                                                                                                                  |
| `rendering-util/rendering-elements/nodes.ts`             | 14-66, 77-83     | `insertNode` (the shape lookup at 31, the call at 49-51) and `positionNode`                                                                                                                                                                                 |
| `rendering-util/rendering-elements/shapes/util.ts`       | 50-154           | `labelHelper`: the group (63-71), the label text (93-100), the measure (137-138), the centring (140-152)                                                                                                                                                    |
|                                                          | 155-219          | `insertLabel`, the same for the ticket and assigned labels                                                                                                                                                                                                  |
|                                                          | 220-247, 249-250 | `updateNodeBounds`, and `getNodeClasses` (`'node ' + cssClasses + ' ' + extra`)                                                                                                                                                                             |
| `rendering-util/createText.ts`                           | 306-420          | `createText`. The SVG path is 343-420: `markdownToLines` (368), then `createFormattedText`, and the `row` class (415)                                                                                                                                       |
|                                                          | 104, 247, 413    | `tspan.text-outer-tspan`, `tspan.text-inner-tspan`, and the `row` class                                                                                                                                                                                     |
| `rendering-util/handle-markdown-text.ts`                 | 46               | `markdownToLines`                                                                                                                                                                                                                                           |
| `setupGraphViewbox.js`                                   | 23-70            | `calculateSvgSizeAttrs`, `configureSvgSize`, `setupGraphViewbox` (the `getBBox` of the root at 50)                                                                                                                                                          |
| `schemas/config.schema.yaml`                             | 1210-1216        | `mindmap.padding` (default 10) and `mindmap.maxNodeWidth` (default 200)                                                                                                                                                                                     |
|                                                          | 1238-1253        | `KanbanDiagramConfig`: `padding` (1245-1247, default 8), `sectionWidth` (1248-1250, default 200), `ticketBaseUrl` (1251-1253, default `''`)                                                                                                                 |
| `config.ts`                                              | 223-225          | `getConfig` returns `assignWithDepth({}, currentConfig)`, a copy                                                                                                                                                                                            |
| `diagram-api/comments.ts`                                | 6-8              | `cleanupComments`: removes whole-line `%%` comments and trims the start of the text                                                                                                                                                                         |
| `diagram-api/diagramAPI.ts`                              | 22               | `getConfig` is `_getConfig`                                                                                                                                                                                                                                 |

Two more files set the context. `Diagram.ts` (19-85) and `preprocess.ts` (6-91) are the shared steps described in the packet notes, section 2.4. `mermaidAPI.ts` (470-690, 750-758) is the shared render path, and section 6.1 names what it adds.

## 2. Parser

### 2.1 The grammar

```
start      : mindMap | spaceLines mindMap
mindMap    : KANBAN document | KANBAN NL document
document   : document statement stop | statement stop
stop       : NL | EOF | SPACELINE | stop NL | stop EOF
statement  : SPACELIST node shapeData | SPACELIST node | SPACELIST ICON | SPACELIST CLASS
           | SPACELINE | node shapeData | node | ICON | CLASS | SPACELIST
node       : nodeWithId | nodeWithoutId
nodeWithoutId : NODE_DSTART NODE_DESCR NODE_DEND                 (id = descr)
nodeWithId    : NODE_ID | NODE_ID NODE_DSTART NODE_DESCR NODE_DEND
shapeData  : shapeData SHAPE_DATA | SHAPE_DATA                   (concatenated)
```

The `SPACELIST` length is the node's `level` (`kanban.jison:127-128`, `$1.length`). A statement without `SPACELIST` has level 0 (132-133). The actions call `yy.addNode`, `yy.decorateNode`, `yy.getType` and `yy.getLogger`.

The grammar is `packages/mermaid/src/diagrams/mindmap/parser/mindmap.jison` plus shape data. A diff with the mindmap grammar shows these changes only: the keyword (`KANBAN` for `MINDMAP`), the node id class (no `@`, line 70), the shape-data states (24-47), the `shapeData` rule (157-162), and the statements that take it (127-128, 132). The lexer rules are otherwise identical, so the mindmap's lexer notes apply to kanban too.

### 2.2 Lexer rules

The rules are tried in their written order, and the first match wins. The probes show this: a line `  Todo` lexes as `SPACELIST` of length 2 and then `NODE_ID`, not as one `NODE_ID` with the spaces in it, because the `[\s]+` rule (68) comes before the `NODE_ID` rule (70) and both start at the same position.

| Start state  | Line  | Pattern                                                           | Token and effect                                                               |
| ------------ | ----- | ----------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| INITIAL      | 24-26 | `\@\{`                                                            | `SHAPE_DATA` with empty text; begins `shapeData`                               |
| shapeData    | 27-31 | `"`                                                               | `SHAPE_DATA` (the quote); begins `shapeDataStr`                                |
| shapeDataStr | 32-34 | `"`                                                               | `SHAPE_DATA` (the quote); ends the string                                      |
| shapeDataStr | 35-39 | `[^\"]+`                                                          | `SHAPE_DATA`; each newline with the blanks after it becomes `<br/>` (37-38)    |
| shapeData    | 40-43 | `[^}^"]+`                                                         | `SHAPE_DATA`. Excludes `}`, `^` and `"`                                        |
| shapeData    | 44-47 | `}`                                                               | Ends the shape data (no token)                                                 |
| INITIAL      | 48    | `\s*%%.*`                                                         | `SPACELINE`: a comment, with the blanks before it                              |
| INITIAL      | 50    | `kanban`                                                          | `KANBAN`. No word boundary (section 2.4)                                       |
| INITIAL      | 51    | `:::`                                                             | Begins `CLASS`                                                                 |
| CLASS        | 52    | `.+`                                                              | Ends `CLASS`; token `CLASS` with the rest of the line                          |
| CLASS        | 53    | newline                                                           | Ends `CLASS` (no token)                                                        |
| INITIAL      | 55    | `::icon(`                                                         | Begins `ICON`                                                                  |
| INITIAL      | 56    | `[\s]+[\n]`                                                       | `SPACELINE`                                                                    |
| INITIAL      | 57    | `[\n]+`                                                           | `NL`                                                                           |
| ICON         | 58-59 | `[^\)]+`, `)`                                                     | `ICON` with the text; `)` ends `ICON`                                          |
| INITIAL      | 60-67 | `-)`, `(-`, `))`, `)`, `((`, `{{`, `(`, `[`                       | `NODE_DSTART`; begins `NODE`                                                   |
| INITIAL      | 68    | `[\s]+`                                                           | `SPACELIST`, whose length is the level                                         |
| INITIAL      | 70    | `[^\(\[\n\)\{\}@]+`                                               | `NODE_ID`. Spaces, `%`, `"`, `'`, `]` and `-` are all part of it. Excludes `@` |
| INITIAL      | 71    | EOF                                                               | `EOF`                                                                          |
| NODE         | 72-74 | `"` followed by a backquote; then `[^`"]+`; then the closing pair | `NODE_DESCR` inside a Markdown string (`NSTR2`)                                |
| NODE         | 75-77 | `"`; then `[^"]+`; then `"`                                       | `NODE_DESCR` inside a quoted string (`NSTR`). Newlines are allowed             |
| NODE         | 78-85 | `))`, `)`, `]`, `}}`, `(-`, `-)`, `((`, `(`                       | Ends `NODE`, token `NODE_DEND`                                                 |
| NODE         | 86    | `[^\)\]\(\}]+`                                                    | `NODE_DESCR`                                                                   |
| NODE         | 87    | `.+(?!\(\()`                                                      | `NODE_DESCR` (rarely reached, since 86 matches first)                          |

Lexer errors. Jison reports any character that no rule takes, such as `@` outside `@{` (probed: `a@b`, a lexical error). The error text is the Jison message, which the differential does not compare.

### 2.3 Preprocessing

`Diagram.fromText` gives the parser `encodeEntities(cleaned) + '\n'` (`Diagram.ts:77`), the same as packet. Whole-line `%%` comments are removed by `cleanupComments` (`diagram-api/comments.ts:6-8`). That regex also trims the start of the text, so leading blank lines go. The `%%` rule at line 48 handles the comments that remain, such as one after a node id. Front matter is removed earlier (`preprocess.ts:58-91`).

Detection (`detector.ts:9`) reads the cleaned text. `kanbanx` is detected as kanban and then fails to parse (probed).

### 2.4 Probed behaviour

Each row is an input in the kanban grammar, after preprocessing. "Node" means a node of the DB (section or item, section 3). Every row was run through `probe-db.mts` with the default configuration.

| Input                                                                            | Result                                                                                                                                             |
| -------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `kanban` then `  root` (one section)                                             | One section `root`, level 2                                                                                                                        |
| Nested: `root` at 4, `child1` at 6, `leaf1` at 8, `child2` at 6                  | One section; three children, all with `parentId = root` (the depth is not kept)                                                                    |
| `root` at 10, `fakeRoot` at 8, `realRootWrongPlace` at 4                         | Error `Items without section detected, found section ("fakeRoot")`. The error is raised at the third line (section 3.3)                            |
| `root` at 4, `child` at 6, `last` at 2 (the last line)                           | No error. `last` becomes a child of `root`, at level 2                                                                                             |
| The same, plus a fourth line `more` at 4                                         | Error for `more`, naming `last`. The check looks at the nodes that already exist                                                                   |
| Two sections with the same id `docs` (fixture 1)                                 | Two sections with id `docs`, and their children are shared (section 3.4)                                                                           |
| `Todo  ` with two trailing blanks                                                | Id `Todo  ` and label `Todo  `. The trailing blanks stay                                                                                           |
| `Todo List[Label here]`                                                          | Id `Todo List`, label `Label here`                                                                                                                 |
| `Todo List` (no brackets)                                                        | Id and label `Todo List`                                                                                                                           |
| `a(a) %% comment` after `root(Root)`                                             | The comment is a `SPACELINE`. The three children are `Child`, `a`, `b` (as KNBN-22)                                                                |
| `a[x] %% note`                                                                   | Id `a`, label `x`. The comment is removed                                                                                                          |
| `a %% note` (no brackets)                                                        | Id `a %% note`: the comment is part of the id                                                                                                      |
| `  %% note` on its own line                                                      | Skipped                                                                                                                                            |
| `a@b`                                                                            | Lexical error (the `@` of line 70 is excluded and no other rule takes it)                                                                          |
| `a@{ shape: rect }`                                                              | Accepted. The section keeps the shape `kanbanSection` (the DB holds no shape)                                                                      |
| `a@{ shape: kanbanItem }`                                                        | Error `No such shape: kanbanItem. Shape names should be lowercase.` (`kanbanDb.ts:131-133`, so the branch at 136 is never taken)                   |
| `a@{ shape: 1 }`                                                                 | Error `doc.shape.toLowerCase is not a function`                                                                                                    |
| `a@{ shape: Rect }`, `a@{ shape: my_shape }`                                     | Errors: lowercase and no underscore                                                                                                                |
| `a@{ priority: 1 }`, `true`, `'Very High'`                                       | `priority` is the JSON value: `1`, `true`, `"Very High"`                                                                                           |
| `a@{ priority: 0 }`                                                              | Not set (falsy)                                                                                                                                    |
| `a@{ label: 5 }`                                                                 | DB label `5` (a number). The `getData` label is `"5"`                                                                                              |
| `a@{ label: true }`, `a@{ label: [1, 2] }`                                       | DB label `true`, `[1,2]`. `getData` labels `"true"`, `"1,2"`                                                                                       |
| `a[Keep]@{ label: '' }`                                                          | Label `Keep` (an empty label is falsy)                                                                                                             |
| `a@{ ticket: 12 }`                                                               | Ticket `"12"` (`toString`)                                                                                                                         |
| `a@{ assigned: null }`                                                           | Not set                                                                                                                                            |
| `a@{ icon: '' }`                                                                 | Not set                                                                                                                                            |
| `a@{}`                                                                           | Accepted, with no effect                                                                                                                           |
| `a@{ label: x^y }`, and the same with `'x^y'`                                    | Lexical error at `^` (line 40 excludes it, even inside single quotes, because `'` is not a string delimiter here)                                  |
| `a@{ label: 'x' ` (no closing brace) then a newline                              | Parse error                                                                                                                                        |
| `a@{` then `ticket: T-1` and `priority: Low` on their own lines, then `}`        | Accepted, with ticket and priority set                                                                                                             |
| `a@{ label: "two` newline `lines" }`                                             | Label `two<br/>lines` in the DB, `two<br>lines` in `getData`                                                                                       |
| `a[<b>bold</b> & <script>x</script>]`                                            | Label `<b>bold</b> &amp; ` (DOMPurify removes the script and keeps the trailing blank)                                                             |
| `x<b>y</b>[z]`                                                                   | Id `x<b>y</b>` (ids are sanitised, and markup is kept)                                                                                             |
| `<script>x</script>` as the whole id                                             | Id `kbn0` and label `""` (the id sanitises to empty, so the counter is used)                                                                       |
| `a[x #amp; y]`                                                                   | Label `x ﬂ°amp¶ß y` (placeholder kept)                                                                                                             |
| `a[x]y]`                                                                         | Parse error: `y]` is a `NODE_ID` after the node                                                                                                    |
| `a[]`                                                                            | Parse error                                                                                                                                        |
| `a["x` newline `y"]`                                                             | Label `x` newline ` y` (the quoted string keeps the line break)                                                                                    |
| `a["` with backquotes and `**x**`                                                | Label `md **x**` (the backquotes are removed)                                                                                                      |
| `kanbanX` then `  a`                                                             | Parse error: `X` after the keyword                                                                                                                 |
| `kanban-x` then `  a`                                                            | Accepted: a section with id `-x` at level 0, and child `a`                                                                                         |
| `kanban%%c`                                                                      | Accepted                                                                                                                                           |
| `KANBAN` (upper case)                                                            | Accepted (`case-insensitive`)                                                                                                                      |
| `kanban a` (same line)                                                           | Section `a` at level 1                                                                                                                             |
| `kanban` with no newline after it                                                | Parse error. The text the parser gets is `kanban` and the newline that `Diagram.fromText` appends: a single `NL` with no statement after it        |
| `kanban` followed by a newline (`kanban\n`)                                      | Accepted, with no sections. The two newlines lex as one `SPACELINE` (line 56 comes before `NL` at 57), and a `SPACELINE` is a statement (line 131) |
| `kanban` followed by a blank line (`kanban\n\n`)                                 | Accepted, with no sections                                                                                                                         |
| `\n\nkanban` with leading blank lines                                            | Accepted                                                                                                                                           |
| `::icon(...)` before any node                                                    | TypeError `Cannot set properties of undefined (setting 'icon')`: `decorateNode` reads `nodes[-1]` (`kanbanDb.ts:207-209`)                          |
| `  root` then `    ::icon(fa fa-book)` and `    :::urgent`                       | Both decorate `root` (the last node), not the child that follows                                                                                   |
| `<b>c</b> d` in `:::`                                                            | Class `<b>c</b> d` (sanitised, markup kept)                                                                                                        |
| `a[Item]@{ priority: 'Very High' }` and the other four levels, `Medium` included | Lines with a stroke colour, and for `Medium` a line with no stroke attribute (rendering, section 6.4)                                              |
| `root` at 0, then `child` at 2                                                   | Accepted: the first node's level is 0                                                                                                              |

### 2.5 Quirks to reproduce

1. **Lexer keywords have no boundary.** `kanbanX` is a parse error and `kanban-x` is a section (`kanban.jison:50`, `detector.ts:9`).
2. **Ids keep their trailing blanks and their comments.** `NODE_ID` (line 70) takes every character except `(`, `[`, newline, `)`, `{`, `}` and `@`. An end-of-line `%%` after a plain id is part of the id.
3. **Decorations apply to the last node,** whatever its level (`kanbanDb.ts:207`).
4. **A decoration before any node is a TypeError** (probed).
5. **`^` is excluded from shape data, and `'` is an ordinary character** (line 40).
6. **Numeric and boolean YAML values are kept as JSON.** `priority` keeps its type. `ticket`, `assigned` and `icon` are strings through `toString` (`kanbanDb.ts:143-150`).
7. **`shape` never reaches the DB.** The check at 131-133 rejects anything that is not lowercase, and the `kanbanItem` branch at 136-138 is therefore unreachable. Any lowercase shape is accepted and ignored.
8. **The level check looks one node behind.** `getSection` (`kanbanDb.ts:25-48`) throws only when an existing node has a lower level than the first node. A last line that is shallower than the first node is accepted and becomes a child (probed), and the error is raised at the next line instead.
9. **The end of the input has its own rule.** `kanban` with no newline is rejected, and `kanban` with a newline is accepted with no sections. Two newlines lex as one `SPACELINE` (line 56), which is a statement (line 131). One newline is an `NL`, which needs a statement after it (probed).
10. **Single-quoted and unquoted text are the same in shape data.** Only `"` starts a string state (line 27).

## 3. DB

### 3.1 Calls in order

| Step | Call                                                | Effect                                                                                                                  |
| ---- | --------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| 1    | `db.clear()` (`Diagram.ts:49`, `kanbanDb.ts:15-20`) | `nodes`, `sections`, `cnt` and `elements` are reset. The shared config is not                                           |
| 2    | `parser.parse(text)` (`Diagram.ts:83`)              | The actions call `addNode` (grammar lines 127-128, 132-133), `decorateNode` (129-130, 134-135) and `getType` (148, 154) |
| 3    | `draw` (`kanbanRenderer.ts:16`)                     | `db.getData()` once. Everything after that reads the `getData` result                                                   |

The DB is not a class. It is a module with a default export, and the parser keeps a reference to it through `kanban.yy` (`kanban.spec.ts:10`, `kanban-definition.ts:3`).

### 3.2 `addNode` (`kanbanDb.ts:99-165`) step by step

1. `padding` is `mindmap.padding`, default 10 (`kanbanDb.ts:101`). For rectangle, rounded and hexagon types it is doubled (103-107).
2. The node is `{ id, level, label, width, padding, isGroup: false }` (109-116). `id` is `sanitizeText(id) || 'kbn' + cnt++` (110). `label` is `sanitizeText(descr)` (112). `width` is `mindmap.maxNodeWidth`, default 200 (113).
3. Shape data (118-155). The text is wrapped as a flow mapping (`{ … }`) unless it has a newline (122-128). It is parsed with `js-yaml` `load` and `JSON_SCHEMA` (129). Then, in order: the shape check (131-133), `label` if truthy (139-141), `icon` if truthy as a string (142-144), `assigned` (145-147), `ticket` (148-150), and `priority` if truthy, with its JSON value (152-154).
4. Placement (157-164). `getSection(level)` returns the section that the node joins, or null. A null makes the node a section, and it is pushed to `sections` (162). Otherwise `node.parentId = section.id` (160). The node is pushed to `nodes` (164) in every case.

### 3.3 `getSection` (`kanbanDb.ts:25-48`)

`sectionLevel` is the level of `nodes[0]`. The loop goes from the last node to the first. It records `lastSection`, the last node at `sectionLevel`, and throws when any node has a lower level than `sectionLevel` (38-40). If the new node's level equals `lastSection.level`, the node is a section (42-44). Otherwise it is a child of `lastSection`.

Because the check runs over the nodes that already exist, the error is raised one line late (probed: quirk 8, section 2.5).

### 3.4 `getData` (`kanbanDb.ts:54-97`) and what it drops

`getData` builds the render input. For each section in `sections` order (61-62):

| Field                          | Section (62-71)             | Item (75-92)                                     |
| ------------------------------ | --------------------------- | ------------------------------------------------ |
| `id`                           | `section.id`                | `item.id`                                        |
| `parentId`                     | not set                     | `section.id`, the id of the section that owns it |
| `label`                        | `sanitizeText(label ?? '')` | `sanitizeText(label ?? '')`                      |
| `labelType`                    | `'markdown'`                | `'markdown'`                                     |
| `isGroup`                      | `true`                      | `false`                                          |
| `ticket`                       | kept                        | kept                                             |
| `priority`, `assigned`, `icon` | **dropped**                 | kept                                             |
| `cssClasses`                   | **dropped**                 | **dropped**                                      |
| `shape`                        | `'kanbanSection'`           | `'kanbanItem'`                                   |
| `level`                        | kept                        | kept                                             |
| `rx`, `ry`                     | not set                     | 5 and 5                                          |
| `cssStyles`                    | not set                     | `['text-align: left']`                           |
| `width`, `padding`             | not set                     | not set                                          |

Probed (`probe-db.mts`): fixture 8 has a section `docs` with `priority: Very Low` and `assigned: knsv` in the DB. Its `getData` node has `ticket` and nothing else of that kind. The `:::urgent` class and the `::icon` of a section never reach the renderer (probed with `icon-and-class` and `class-on-section`).

Children are `nodes.filter(n => n.parentId === section.id)` (73), so every item whose `parentId` equals a section id is listed under that section. Two sections with the same id therefore list the same children twice (probed: `docs` twice with `x` and `y` below it gives four item nodes).

Ids are not sanitised again in `getData`. Labels are sanitised again there (64, 79), so a yaml label is sanitised once in `getData`, and a descr label twice.

### 3.5 `decorateNode` (`kanbanDb.ts:202-214`)

The last node in `nodes` receives the decoration: `icon` sanitised (209), `class` sanitised into `cssClasses` (212). A decoration with no node raises the TypeError of section 2.4.

### 3.6 Quirks of the DB

1. Ids are sanitised, so `x<b>y</b>` is a valid id with markup (probed).
2. An id that sanitises to empty becomes `kbn<n>` (110), with a counter shared with nothing else. The counter starts at 0 for each parse (`clear`, line 18).
3. `width` and `padding` come from `mindmap` (101, 113), so they depend on configuration, and `getData` drops them. The renderer does not read them (section 6.3 explains where the renderer's padding comes from).
4. `sections` and `nodes` hold the same objects, so a section is also in `nodes`, with `isGroup: false` (the DB never sets `isGroup` to true; `getData` does).

## 4. Canonical model proposal

Two models are possible, and both are useful. The DB model follows [canonical-model.md](../canonical-model.md). The `getData` model is the layout input the renderer reads, and it is where the sanitising and the dropped fields show. Section 8 asks the coordinator which to build first.

### 4.1 DB model

```text
KanbanDiagram { version: 1, nodes: [Node] }
Node { id, level, label, parentId, isSection, icon, assigned, ticket, priority, cssClasses }
```

| Field                                      | Rule                                                                                                                                                                          |
| ------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `nodes`                                    | Every node in the DB's `nodes` order. A section is a node with `isSection: true`; the order of the sections is the order of those nodes in `sections`, which is the same list |
| `id`                                       | The sanitised id, with its blanks and comments as the lexer kept them                                                                                                         |
| `level`                                    | The level as the grammar gives it                                                                                                                                             |
| `label`                                    | The DB value. A descr is sanitised; a yaml label is the JSON value as yaml gives it (`5`, `true`, `[1,2]`)                                                                    |
| `parentId`                                 | `null` for a section                                                                                                                                                          |
| `icon`, `assigned`, `ticket`, `cssClasses` | The strings as the DB holds them, or `null`                                                                                                                                   |
| `priority`                                 | The JSON value, or `null`                                                                                                                                                     |

Left out, because they are config-derived and not read by `getData`: `width` and `padding` (section 3.6, item 3). Also left out: `isGroup` (derived) and `shape` (never set in the DB, quirk 7). Probed: none of these reaches the render (the render probes show no `width` or `padding` in the output).

### 4.2 `getData` model (proposed for a second differential)

```text
KanbanData { nodes: [LayoutNode] }
LayoutNode { id, parentId, label, labelType, isGroup, shape, level, ticket, priority, assigned, icon, rx, ry, cssStyles }
```

Every field is the one `getData` returns (section 3.4). `null` stands for an absent field. The reason for a second model is that the sanitising and the dropped fields are only visible here, and the renderer depends on exactly this list.

### 4.3 Values that need a rule

- `label` may be a number, a boolean or an array in the DB (probed). The `getData` label is the sanitised string, as probed (`"5"`, `"true"`, `"1,2"`).
- `priority` may be a number, a boolean or a string. Its truthiness decides whether it is set (`0` is not).

## 5. Specs

### 5.1 `kanban.spec.ts` (494 lines, 30 tests)

| Lines   | Group            | Tests                                                                                                                                                                                                                     | What they test                                                                                                                                                                          |
| ------- | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 14-106  | `hierarchy`      | 6: KNBN-1 (15), KNBN-2 (24), `3 should handle a simple root definition…` (50), KNBN-4 (59), `5 Multiple sections are ok` (80), KNBN-6 (98)                                                                                | Sections and children through `getSections` and `getData`. KNBN-4 accepts deeper levels, and its expected error (lines 66-69) is commented out. KNBN-6 expects the error of section 3.3 |
| 108-156 | `nodes`          | 3: KNBN-7 (109), KNBN-8 (123), KNBN-9 (140)                                                                                                                                                                               | Ids, types and descriptions (the bracket forms)                                                                                                                                         |
| 158-232 | `decorations`    | 4: KNBN-13 (159), KNBN-14 (177), KNBN-15 (194), KNBN-16 (213)                                                                                                                                                             | `::icon` and `:::` on the node                                                                                                                                                          |
| 234-287 | `descriptions`   | 3: KNBN-17 (235), KNBN-18 (248), KNBN-19 (264)                                                                                                                                                                            | Node syntax in descriptions, and a child after a class assignment                                                                                                                       |
| 289-406 | top level        | 6: KNBN-20 (289, empty rows), KNBN-21 (313, comment lines), KNBN-22 (337, comments at the end of a line), KNBN-23 (360, rows with spaces), KNBN-24 (376, rows above the keyword), KNBN-25 (392, the same without a space) | Blank lines, comment lines, end-of-line comments after a bracket form, and leading rows                                                                                                 |
| 408-493 | `item data data` | 8: KNBN-30 (414, priority), KNBN-31 (426, assigned), KNBN-32 (435, icon), KNBN-33 (444, icon again), KNBN-34 (453, multi-line metadata), KNBN-35 (466, one-line metadata), KNBN-36 (476, label), KNBN-37 (484, ticket)    | The shape data, through `getSections` (the section's own fields)                                                                                                                        |

Several tests read children through `kanban.yy.getData()`, so they check the filtering of children (section 3.4). The `item data data` group reads section fields through `getSections()`, which is the DB and not `getData`, so those tests pass even though `getData` drops the same fields for sections.

Not covered by any spec: the renderer, the shapes, the styles, the sanitising of ids and labels, the duplicate-id case, the `kbn` counter, the lexer errors, and the numeric YAML values.

### 5.2 Other specs and samples

There is no other kanban spec. `samples.md` holds six `mermaid` samples for the docs. They are the input texts of the differential's "docs" collection (`tools/port/inputs.ts`).

## 6. Renderer

### 6.1 Pipeline

The generic steps are those of packet's section 6.1 (`mermaidAPI.ts:397-421`, `319-334`, `596-600`, `606-608`, `623`, `750-758`, `344-364`, `645-650`). The kanban `draw` is async, and `renderDiagram` awaits it (`mermaidAPI.ts:606`).

Probed: the root is `svg` with `id`, `width="100%"`, `xmlns`, `style="max-width: <W>px;"`, `viewBox`, `role="graphics-document document"` and `aria-roledescription="kanban"`. Its children are `style`, the empty `g` of step 1, then `g.sections` and `g.items`. The `style` element is first.

### 6.2 `draw` (`kanbanRenderer.ts:12-98`): the placement

| Quantity                | Formula                                                                           | Value                 | Source                                             |
| ----------------------- | --------------------------------------------------------------------------------- | --------------------- | -------------------------------------------------- |
| `WIDTH`                 | `kanban.sectionWidth \|\| 200`                                                    | 200                   | 47, 68                                             |
| `padding`               | the constant `10` (the source says "TODO set padding")                            | 10                    | 42                                                 |
| `maxLabelHeight`        | starts at 25 and grows to the tallest section label                               | 25 or more            | 45, 61                                             |
| section `cnt` (1-based) | the position among the sections                                                   | 1, 2, 3               | 50                                                 |
| section `x`             | `WIDTH * cnt + ((cnt - 1) * padding) / 2`                                         | 200, 405, 610         | 51 (probed: rect `x` 100, 305, 510 with width 200) |
| section `width`         | `WIDTH`                                                                           | 200                   | 52                                                 |
| section `y`, `height`   | 0 and `WIDTH * 3` = 600 (replaced later)                                          |                       | 53-54                                              |
| section `rx`, `ry`      | 5                                                                                 |                       | 55-56                                              |
| section class           | `cssClasses + ' section-' + cnt`, where `cssClasses` is `undefined` (section 3.4) | `undefined section-1` | 59 (probed)                                        |
| item `top`              | `(-WIDTH * 3) / 2 + maxLabelHeight`                                               | -275 with 25          | 69                                                 |
| item `x`                | the section's `x`                                                                 |                       | 78                                                 |
| item `width`            | `WIDTH - 1.5 * padding`                                                           | 185                   | 79                                                 |
| item `y`                | `y + bbox.height / 2`, where `bbox` is the measured `getBBox` of the item's group |                       | 81-82                                              |
| after an item           | `y = item.y + bbox.height / 2 + padding / 2`                                      |                       | 84                                                 |
| section rect height     | `max(y - top + 3 * padding, 50) + (maxLabelHeight - 25)`                          |                       | 87-88                                              |
| viewBox                 | `setupGraphViewbox(undefined, svg, mindmap.padding, mindmap.useMaxWidth)`         | padding 10            | 92-97                                              |

Probed numbers: fixture 1 has three sections, with rect `x` 100, 305 and 510, height 57.4, and `viewBox="90 -310 630 77.4"`. The viewBox is the box of the sections (`100` to `710`, `-300` to `-242.6`) grown by 10 on each side.

Empty kanban (`kanban` followed by a newline): `viewBox="-10 -10 20 20"` and `max-width: 20px`, which is the padding of an empty box (`getBBox` of an empty group is zero).

### 6.3 Where the padding comes from

`kanbanRenderer.ts:95` uses `mindmap.padding` for the viewBox, and `kanbanDb.ts:101` uses it for the node padding. The kanban's own `padding` (default 8, `config.schema.yaml:1247`) is read only when `mindmap` is missing, which the defaults never allow. As the code reads it, it is dead configuration. Setting it has no effect in the code as read (**unverified** by probe).

### 6.4 `kanbanSection` (`clusters.js:308-417`), one per section

Element structure, in DOM order (probed): `g.cluster.undefined.section-N` (id `domId`, `data-look`) with the class built at 318-321. Inside it: `rect` first (the rect is inserted as `:first-child`, 380), then `g.cluster-label` with the label's `g`.

The label (323-338) is `createText(labelEl, node.label, {style, useHtmlLabels, isNode: true, width: node.width})`. It is **not** sanitised here: `clusters.js:328` passes the label straight to `createText`, which decodes the entities (`createText.ts:366`) and does not sanitise. The entity placeholders are therefore decoded at the label, and the output shows the decoded text (probed: `x ﬂ°amp¶ß y` becomes the tspans `x`, ` &amp;` and ` y`).

The label's bbox (336) gives `maxLabelHeight` (in the renderer, 61). The width rule (346-351) compares `node.width` with `bbox.width + node.padding`. `node.padding` is undefined for a section (section 3.4), so the comparison is false and the width stays 200 (probed: every section rect is 200 wide, even with a long label). The rect's position is `x - width / 2` and `y - height / 2` (354-355), so `y` is -300 with height 600.

The label is moved to `translate(node.x - bbox.width / 2, node.y - node.height / 2 + subGraphTitleTopMargin)` (391-396). `subGraphTitleTopMargin` is 0 in the probes (the label sat at `y=-300`).

The function ends by reading the rect's `getBBox` into `node.width` and `node.height` (406-409). The renderer later sets the rect's height attribute itself and does not read these fields.

### 6.5 Labels

Items and sections use the same label code, with `htmlLabels` false.

- **Which path.** `labelHelper` decides `useHtmlLabels = node.useHtmlLabels || evaluate(getConfig().htmlLabels)` (`util.ts:61`). `kanbanRenderer.ts:19` sets `conf.htmlLabels = false`, but `conf` is a copy from `getConfig()` (`config.ts:223-225`), so the assignment has no effect on later reads. **Probed:** with the default configuration (no `htmlLabels` key) the same diagram renders `foreignObject` labels with `div`, `span.nodeLabel` and `p`, and `<b>` is bold (probe `df1`). With `htmlLabels: false` it renders SVG `text` (probe `kx3`). Decision 0004 says ferrigura draws SVG text in every case, so the forced configuration of the harness is the path to compare. The default-configuration output is a known difference that the coordinator should record (section 8, Q1).
- **Text.** `createText` (`createText.ts:343-420`) runs `markdownToLines` (368, `handle-markdown-text.ts:46`) on the text after `decodeEntities` and `<br>` normalisation (`createText.ts:366-368`), then builds the SVG text (`createFormattedText`). Each line is a `tspan.text-outer-tspan` (104) with the class `row` (415), and each word is a `tspan.text-inner-tspan` (247). Probed: `Create Blog about the new diagram` wraps into two rows of `tspan.row`.
- **Markdown in the SVG path.** The HTML tags are text here: `<b>bold</b> & x` gives the tspans `<b>`, ` bold`, ` </b>`, ` &`, ` x` (probed). The default HTML path makes `bold` bold (above).
- **Width.** The section label wraps at 200 (`clusters.js:328` with `node.width`). An item label wraps at 175: `kanbanItem.ts:34` sets the width to 185 minus 10 before `labelHelper` (40) and the `insertLabel` options (58), and restores 185 after the labels (82).
- **Measurement.** The SVG path measures `text.getBBox()` (`util.ts:137-138` for `labelHelper`, `util.ts:203-204` for `insertLabel`), and the `text` element includes its `tspan`s. The measurements, through the text rules of decision 0005, decide each item's height and so every position below it.
- **Sanitising.** `labelHelper` and `insertLabel` sanitise `decodeEntities(label)` (`util.ts:95` and `util.ts:177`), so an item label is decoded, sanitised and then read as markdown. The section label is not sanitised at render, only decoded by `createText` (section 6.4).
- **Empty labels.** An item always gets its ticket and assigned labels, even when they are empty (`kanbanItem.ts:65-81`). Each empty label is a `g.label` with a background `rect` and an empty `text`. Its bbox height is 0, which the item's height of 37 (17 + 20) implies. Probed: an item with no ticket and no assignee has two empty label groups, at `translate(-82.5, 8.5)` and `translate(82.5, 8.5)`.

### 6.6 `kanbanItem` (`kanbanItem.ts:23-163`), one per item

Element structure (probed, `outline.cjs`): `g.node.undefined` (`id = domId`, `transform = translate(x, y)` set by `positionNode`). The class is `node undefined ` with a trailing blank, because `getNodeClasses` concatenates it (`util.ts:249-250`). Inside, in order: the `rect` (first child, 125), the title `g.label`, the ticket `g.label`, the assigned `g.label`, and the priority `line` if one is drawn (137-152).

| Element                              | Attributes                                                                                                                                                                                                             | Source  |
| ------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------- |
| `rect.basic.label-container.__APA__` | class `basic label-container __APA__` (the literal `__APA__` is in the source, 128), `style` from `nodeStyles` (empty), `rx` and `ry` from the node or 5, `x = -totalWidth/2`, `y = -totalHeight/2`, `width`, `height` | 127-135 |
| title `g.label`                      | `transform = translate(padding - totalWidth/2, -heightAdj - bbox.height/2)`, where `padding` is `node.padding \|\| 10` (41)                                                                                            | 90-93   |
| ticket `g.label`                     | `transform = translate(padding - totalWidth/2, -heightAdj + bbox.height/2)`                                                                                                                                            | 94-97   |
| assigned `g.label`                   | `transform = translate(padding + totalWidth/2 - bboxAssigned.width - 2*10, -heightAdj + bbox.height/2)`, right-aligned                                                                                                 | 98-105  |
| priority `line`                      | `x1 = x2 = x + 2`, `y1 = y + floor(rx/2)`, `y2 = y + totalHeight - floor(rx/2)`, `stroke-width="4"`, `stroke = colorFromPriority(priority)`                                                                            | 137-152 |

The sizes (84-89): `heightAdj = max(ticket bbox height, assigned bbox height) / 2`; `totalHeight = max(title bbox height + 20, node.height \|\| 0) + heightAdj`; `totalWidth = node.width` (185, restored at 82). Probed: a one-line title gives `height 37` and `rect y -18.5`.

Priority colours (`colorFromPriority`, 9-22): `Very High` red, `High` orange, `Medium` no stroke (the attribute is removed, `null`), `Low` blue, `Very Low` lightblue. Any other value gives `undefined`, which d3 removes as well, so the line has no stroke (**read**, not probed). Probed: all five levels except the unknown value; `Medium` gives `<line … stroke-width="4">` with no `stroke`.

The item's height: the renderer's `getBBox` (`kanbanRenderer.ts:81`) is the box of the whole group, and it includes the rect and the labels (which carry their transforms). A title of one line and no ticket gives 37.

`kanbanItem` sets `intersect` (158-160) and the bounds (155-156). Nothing in the renderer reads them.

The ticket link (46-53) is created only when `kanban.ticketBaseUrl` is set. Its default is `''` (`config.schema.yaml:1253`). No fixture sets it. When it is set, the ticket label goes into an `svg:a` with `class="kanban-ticket-link"`, `xlink:href` and `target="_blank"` (lines 48-52, **read**, not probed).

The `handDrawn` look (112-123) draws with rough.js. The seed is `handDrawnSeed` (the config). No fixture sets `look`.

### 6.7 Styles (`kanban/styles.ts`)

`getStyles` (77-110) builds one CSS string from the theme: `.edge` (`stroke-width: 3`), `genSections` (6-74), the section root (83-85), the icon container (89-94), the edge fill, the cluster and label colours (98-101), `.kanban-label` (102-108), and `getIconStyles()`.

`genSections` reads the theme colour list. Its length is `THEME_COLOR_LIMIT`. The Rust palette records twelve for the redux themes (`crates/ferrigura-render/src/git_graph/palette.rs:23`), which is **unverified** against the reference's theme file; the probed style sheet has twelve section rules (`-1` to `10`). It reads `cScale<i>`, `cScaleInv<i>`, `cScaleLabel<i>`, and `lineColor<i>`, which it first sets to `cScaleInv<i>` when missing and then lightens or darkens by 20 (9-16, using `isDark`, `lighten` and `darken` from khroma; `khroma.rs` has all three, `lighten` at 960, `is_dark` at 1074, `darken` at 951, as Rust functions).

The selectors use `section-${i - 1}` (24-26, 31, 34, 38, 41, 44). The probe shows the rule list for indices `-1` to `10`, which is 12 entries. A section is numbered from 1 (`kanbanRenderer.ts:50, 59`), so the rule `.section-1` (the third entry, `i = 2`) holds the colour of `cScale2`, and `.section-N` holds `cScale(N+1)`. The `.section--1` and `.section-0` rules are never used. **Read and probed for the selectors; the colour values are unverified**.

The sheet also reads `options.background`, `nodeBorder`, `git0`, `gitBranchLabel0`, `textColor` and `darkMode` (`styles.ts:19, 56-64, 83-87, 98-101`). Its function receives `{...config.themeVariables, theme, look, svgId}` (`mermaidAPI.ts:327-333`).

### 6.8 Configuration

| Key                    | Default                                    | Read by                                                         |
| ---------------------- | ------------------------------------------ | --------------------------------------------------------------- |
| `kanban.sectionWidth`  | 200                                        | `kanbanRenderer.ts:47, 68`                                      |
| `kanban.ticketBaseUrl` | `''`                                       | `kanbanItem.ts:46-47`                                           |
| `kanban.padding`       | 8                                          | Only as a fallback that is never reached (section 6.3)          |
| `mindmap.padding`      | 10                                         | `kanbanDb.ts:101`, `kanbanRenderer.ts:95`                       |
| `mindmap.maxNodeWidth` | 200                                        | `kanbanDb.ts:113`, not in `getData`, so it has no render effect |
| `mindmap.useMaxWidth`  | true (probed: `max-width` in every render) | `kanbanRenderer.ts:96`                                          |
| `look`                 | `classic`                                  | `getData` (`kanbanDb.ts:70`), `data-look` on sections           |
| `htmlLabels`           | effectively true (section 6.5)             | `labelHelper`, `kanbanSection`                                  |

### 6.9 Randomness, the clock, DOM

- **Randomness and the clock.** None in the code paths read (grepped in `kanban/`, `kanbanItem.ts`, `createText.ts`, `util.ts`, `clusters.js` and `nodes.ts`). The counter `kbn<n>` is deterministic. The `rough.js` seed is `handDrawnSeed` (`clusters.js:18, 85`, `kanbanItem.ts:112-123`), only for `look: handDrawn`.
- **DOM dependencies a server renderer must replace.** `getBBox` in the items (`kanbanRenderer.ts:81`), in the labels (`util.ts:137-138` for `labelHelper`, `util.ts:203-204` for `insertLabel`) and in the section label (`clusters.js:336`), and `getBBox` of the root (`setupGraphViewbox.js:50`). The rect's `getBBox` (`clusters.js:406`) is not used for positions. Text metrics are therefore part of the geometry, not only of the labels.
- **`getBBox` semantics.** The box of a group is the union of its children's boxes, with their transforms. The item's group has labels with transforms, so the union includes them. This is the part to test against Chromium's `getBBox`.

## 7. Fixtures

| File (in `e2e/diagrams/kanban/`)                                                           | Features                                                                                                                                                          | Probed viewBox         |
| ------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------- |
| `pr-check/1-should-render-a-kanban-with-a-single-section.mmd`                              | Three sections (the name says single, the text says three same-level nodes), one duplicate id `docs`                                                              | `90 -310 630 77.4`     |
| `pr-check/2-should-render-a-kanban-with-multiple-sections.mmd`                             | Four sections, one a bare id `id2` (its label is the id), a duplicate `docs`                                                                                      | Rendered; not recorded |
| `pr-check/3-should-render-a-kanban-with-a-single-wrapping-node.mmd`                        | Two sections; the second title is over 100 characters and wraps                                                                                                   | `90 -310 425 108.2`    |
| `pr-check/4-…-at-the-end.mmd`                                                              | Three sections; a wrapping title last                                                                                                                             | Rendered; not recorded |
| `pr-check/5-…-at-the-top.mmd`                                                              | Three sections; a wrapping title second                                                                                                                           | Rendered; not recorded |
| `pr-check/6-should-handle-assignments.mmd`                                                 | Four sections; `assigned` on the last one, which is a section, and `docs` twice                                                                                   | Rendered; not recorded |
| `pr-check/6-…-in-the-middle.mmd`                                                           | Four sections; a wrapping title third                                                                                                                             | Rendered; not recorded |
| `pr-check/7-should-handle-external-tickets.mmd`                                            | Four sections; `ticket` on the last one (a section, so the ticket reaches `getData` and nothing else)                                                             | Rendered; not recorded |
| `pr-check/8-should-handle-assignments-prioritization-and-tickets-ids-in-the-same-item.mmd` | Two sections; the second one has `priority`, `ticket` and `assigned`. `getData` drops the first and the last, and keeps the ticket, which a section does not draw | Rendered; not recorded |
| `pr-check/10-full-example.mmd`                                                             | Fifteen sections, the long title, `priority` and `assigned`, `docs` twice; no items                                                                               | `90 -310 3090 123.6`   |
| `nightly-check/7-should-handle-prioritization.mmd`                                         | Six sections, with five priority levels and one with none                                                                                                         | `90 -310 1245 70`      |

Two things to know about the fixtures. First, every line of these fixtures has the same indentation, so every node is a section: no fixture has an item in the render. The descriptions in the fixtures say so ("each becomes its own top-level column"). Second, the metadata of the fixtures is on sections, so it does not reach the render (section 3.4). Only the labels, the wrapping, the duplicate ids and the layout of the sections are exercised by the render. The items, the `kanbanItem` shape and its priority lines are exercised only by the probe inputs I wrote (`kx3`, `kx4` in `render-inputs.json`, `tk1`, `pri1` and `hl1` in `render-inputs2.json` and `render-inputs4.json`).

Exclusions a geometry harness will need: none of the fixture texts uses a character outside the bundled faces (Latin only). No fixture uses `look`, `ticketBaseUrl` or `htmlLabels`, except through the harness's forced `htmlLabels: false`. The duplicate ids in fixtures 1, 2, 6 (assignments), 7 and 10 give duplicate DOM ids, so a harness that pairs elements by id must pair them by position.

## 8. Risks, effort, open questions

### 8.1 Risks

| Risk                                                    | Level                 | Why                                                                                                                                                                                                                                                                                 |
| ------------------------------------------------------- | --------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The Jison lexer with start states and first-match rules | Medium to high        | Eight start conditions and about 35 rules. The quirks of section 2.5 all come from the rule order, and each needs a probe to keep                                                                                                                                                   |
| Label text measurement and markdown tspans              | High                  | Every item's height and every position below it come from `getBBox` of SVG text with `tspan`s. The M5 text rules and the markdown converter (`unified/markdown.rs`) must match Chromium exactly. `unified/label.rs` covers the flowchart's labels, and its reuse here is unverified |
| Measured positions                                      | High                  | The viewBox and the section heights come from the `getBBox` of groups and of the root. Any difference in a label's height changes the whole layout                                                                                                                                  |
| The default configuration renders HTML                  | Medium                | The reference's default output differs from the SVG path (section 6.5). Decision 0004 accepts that ferrigura draws SVG text in all cases, so it is a recorded consequence, not a new defect                                                                                         |
| YAML in the shape data                                  | Low                   | `crates/ferrigura-syntax/src/yaml.rs` loads YAML 1.2 with the `js-yaml` typing. Its handling of the flow mapping and the multi-line form of the shape data is **unverified**                                                                                                        |
| The quirks of section 3.4 and 6.4                       | Low once written down | Reproducing `undefined section-N`, `node undefined`, `__APA__`, the duplicated children and the dropped section metadata is mechanical                                                                                                                                              |
| `rough.js` for `look: handDrawn`                        | Medium if in scope    | Both shapes have a rough branch. No fixture uses it                                                                                                                                                                                                                                 |

### 8.2 Effort

A rough estimate in Rust lines, not a measurement:

| Part                                                                                        | Lines      |
| ------------------------------------------------------------------------------------------- | ---------- |
| Scanner with the start states (section 2.2)                                                 | 250 to 350 |
| Parser: the grammar rules, the statements, the level                                        | 150 to 200 |
| DB: `addNode`, `getSection`, `decorateNode`, shape data through the YAML module, `getData`  | 250 to 300 |
| Renderer: placement, `kanbanSection`, `kanbanItem`, the labels through the shared text code | 500 to 650 |
| Styles: the section colour rules and the static rules                                       | 100 to 150 |
| Tests: 30 specs, the probes of sections 2 to 6                                              | 350 to 450 |

The total is 1,600 to 2,100 lines.

### 8.3 Open questions

- **Q1. Which label path does the port follow?** The reference's default output is HTML (section 6.5, probed). Decision 0004 says ferrigura draws SVG text. This note assumes 0004, which is also what the harness forces. The coordinator should record the default-configuration difference in the port's notes.
- **Q2. Which model first: the DB or `getData`?** The DB model is the canonical one. The `getData` model shows the dropped section metadata and the sanitising, and it is the renderer's input. This note recommends both, DB first.
- **Q3. Reproduce the dropped section metadata?** `getData` drops `priority`, `assigned`, `icon` and `cssClasses` for sections (section 3.4). The reference renders nothing of them, so a faithful port does too. Confirm.
- **Q4. Reproduce the duplicate-children behaviour?** A repeated section id lists its children under each section (section 3.4). Confirm.
- **Q5. Reproduce the quirks of the lexer and the DB?** The list is in section 2.5. This note assumes yes, as M6 requires. Two of them change what a user sees: the keyword without a boundary (`kanban-x`), and the id that keeps its trailing blanks and comments.
- **Q6. Is `look: handDrawn` in scope?** No fixture uses it. Both shapes have a rough branch, and `rough.rs` exists in `unified/shapes/`. Decide whether to exclude it with a named reason.
- **Q7. Is the ticket link in scope?** No fixture sets `ticketBaseUrl`. The link is an `svg:a` with a `target`. Whether the serialisation's DOMPurify keeps `target` and `xlink:href` on `svg:a` is **unverified**. Decide whether to exclude it with a named reason, or to add a fixture.
- **Q8. How are numeric YAML values modelled?** `priority` and `label` can be numbers or booleans (section 4.3). Choose a JSON value type for them in the model.
- **Q9. Does `getData` count as the model for a differential?** Section 4.2 proposes it. It needs `tools/port`'s layout-data pattern (`diff-class-data.ts` is the precedent).
