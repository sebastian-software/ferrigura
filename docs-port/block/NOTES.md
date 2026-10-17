# BLOCK diagram: research notes for P2

Research only, for the port of `block` to ferrigura (P2, stage 1). Nothing in the repository was changed; this file is the only output. Reference: the TypeScript tree at `aa5c4338f`, the commit of this worktree (`sebastian/port-p2`).

Conventions: **[probed]** = run on the reference, either the Jison grammar and the DB through the port harness's own Vite loader (`tools/port/reference.ts` plugins), or the built bundle `packages/mermaid/dist` in Chromium through the geometry harness's page (`tools/port/geometry/common.ts`: `withPage`, `renderReference`, `patchText`, the forced settings). **[source]** = read in the code. **[unverified]** = neither could confirm it. Paths are relative to `packages/mermaid/src/` unless a repository path is given.

Scratch material (not committed): `/private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/p2-block/`.

- `ref.mts`: the harness's Vite SSR setup with its cache moved to the scratch directory.
- `probe-full.mts '<json array>'`: `mermaidAPI.getDiagramFromText` (preprocessing, detection, parse into the real DB); prints blocks, edges, classes and warnings. `probe-db.mts` is the same without preprocessing.
- `render-text.mts '<json array>'`: renders texts through `renderReference` with the forced settings and prints the root and node boxes; `render-all.mts` renders the 58 fixtures (`render-summary.jsonl`); the SVGs are in `svg/`; `dom.py` dumps an SVG's structure.

---

## 0. Summary

`block` is a grid diagram. The text sets a column count (`columns N`, or `columns auto`, or none: one column per row). Each block takes a span (`A:2`). Empty cells are `space` and `space:N`. A composite (`block:id` … `end`, or `block` … `end`) lays out its own children with its own `columns`. The shapes are `[]` (a square), `()`, `(())`, `>]`, `{}`, `{{}}`, `([])`, `[[]]`, `[()]`, `((()))`, `[//]`, `[\\]`, `[/\]` and `[\/]`, 14 in all, plus the block arrow `<["text"]>(right)`. A plain id with no brackets is drawn as a square too. Edges join blocks with `-->`, `---`, `-.->`, `==>`, `--x`, `--o`, `<-->`, `====` and labels (`-- "t" -->`). `classDef`, `class` and `style` set the look.

The positions are not computed by a graph engine. `layout.ts` does its own arithmetic in three steps: sizes, positions, bounds. The renderer uses the `rendering-util` shapes, markers, edge paths and labels that the flowchart uses. `dagre-d3-es`'s graphlib is only a keyed store here (`renderHelpers.ts:222-242`); no layout runs on it.

Size of the reference, non-spec: 1,746 lines in `diagrams/block/` (grammar 292, DB 379, layout 369, renderHelpers 302, styles 209, renderer 86, types 76, detector and entry 33). Shared code it reaches: about 1,500 lines, most of it already ported for the flowchart (labelHelper and the rect family, `insertEdge`, `insertEdgeLabel`, markers, the palette gate). What the flowchart port does not have: the composite shape (45 lines), the block arrow (217 lines, with its 16 point tables) and the polygon insertion (20 lines).

Rough size of the port: parser and DB about 1,200 to 1,500 lines of Rust, layout about 400, renderer 800 to 1,000 (composite and block arrow about 350 of it), styles about 150; 2,500 to 3,000 lines with tests. Medium-large, about one P1 type. This is a guess from the line counts.

Fixtures: 58 (28 `nightly-check`, 10 `pr-check`, 20 `visual-check`). All 58 render on the reference under the forced settings, with no console warnings or errors [probed]. None needs an exclusion for the look (no `handDrawn`), for text (no non-ASCII in any diagram body) or for links and callbacks (none).

### Findings at a glance

| #   | Finding                                                                                                                                                                                                                                                                                        | Where                                                                 |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| 1   | Positions come from the type's own arithmetic (rows, spans, padding), not from a graph engine.                                                                                                                                                                                                 | `layout.ts:72-369`                                                    |
| 2   | Sizes come from `getBBox` of the rendered node groups, in a first pass where nothing wraps. A second pass places the nodes at their final width, and labels wrap at that width.                                                                                                                | `renderHelpers.ts:137-155`, `158-187`; `shapes/util.ts:98`            |
| 3   | Node labels do not wrap at 200 px [probed]: a 680 px label stays on one row. The 200 px default (`createText.ts:319`) reaches edge labels only [probed: a 190 px edge label wraps into rows]. The e2e test BL38 says "200 px wrap threshold", but it checks overlap, and its premise is wrong. | `createText.ts:319`; `e2e/rendering/block/pr-check/block.spec.js:6-9` |
| 4   | A hexagon label wraps on a float tie: in two equal polygons, `Opening tag` wraps and `Closing tag` does not [probed]. Seven fixtures contain wrapped labels.                                                                                                                                   | `hexagon.ts:40-41`, `46-49`; `createText.ts:172-173`                  |
| 5   | A composite's class is `node default flowchart-label default flowchart-label`. The classes are repeated, and the geometry key counts them.                                                                                                                                                     | `composite.ts:14`; `renderHelpers.ts:23`                              |
| 6   | Every block edge has `style="undefined;;;undefined"`, and its path id is doubled (`b10-b10-1-A-E`).                                                                                                                                                                                            | `edges.js:812`, `853-860`; `renderHelpers.ts:258`, `280`              |
| 7   | Several inputs fail or give garbage: a style before its node (TypeError), `columns 0` (render error), chained edges (a node with id `undefined`), markdown strings, `accTitle`, `linkStyle`, a standalone `default`, `o--o` after a space.                                                     | section 2                                                             |
| 8   | Generated ids (`Math.random`) name unnamed composites and space cells. They reach the DOM but not the structural key.                                                                                                                                                                          | `blockDB.ts:289-293`; `block.jison:214`, `244-245`                    |
| 9   | The production default is HTML labels: `htmlLabels` is unset, and `getEffectiveHtmlLabels` reads it as true. The harness forces `htmlLabels: false`, and decision 0004 draws SVG text.                                                                                                         | `defaultConfig.ts:24`; `config.ts:324-326`                            |
| 10  | With `useMaxWidth: false`, the `height` attribute carries a magic factor the viewBox does not [probed: 117 against 116]. A diagram of only spaces gets `height="NaN"` [probed].                                                                                                                | `blockRenderer.ts:70-79`                                              |

---

## 1. Sources

| File                                                                                                    | Lines          | What it holds                                                                                                                                                                                                                                                                          |
| ------------------------------------------------------------------------------------------------------- | -------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `diagrams/block/parser/block.jison`                                                                     | 292            | Lexer: `%x` states 12-27, rules 39-160 (`%lex` 11-162). Grammar 164-292.                                                                                                                                                                                                               |
| `diagrams/block/blockDB.ts`                                                                             | 379            | Warnings and populate (105-192), `clear` (197-209), type and edge helpers (211-287), `generateId` (289-293), getters (295-376).                                                                                                                                                        |
| `diagrams/block/layout.ts`                                                                              | 369            | `calculateBlockPosition` (11-36), `getMaxChildSize` (38-70), `setBlockSizes` (72-207), `layoutBlocks` (209-322), `findBounds` (324-349), `layout` (351-369).                                                                                                                           |
| `diagrams/block/renderHelpers.ts`                                                                       | 302            | `getNodeFromBlock` (16-136), `calculateBlockSize` (137-155), `insertBlockPositioned` (158-187), `performOperations` (189-205), `insertBlocks` (207-213), `insertEdges` (215-302).                                                                                                      |
| `diagrams/block/blockRenderer.ts`                                                                       | 86             | `draw` (16-81): the passes, the root svg, the viewBox. `getClasses` (12-14).                                                                                                                                                                                                           |
| `diagrams/block/styles.ts`                                                                              | 209            | Palette rules `genColor` (41-64), the sheet `getStyles` (78-207).                                                                                                                                                                                                                      |
| `diagrams/block/blockTypes.ts`                                                                          | 76             | `BlockType` (3-30), `Block` (32-68), `ClassDef` (70-74).                                                                                                                                                                                                                               |
| `diagrams/block/blockDetector.ts`, `blockDiagram.ts`                                                    | 20, 13         | Detector `/^\s*block(?![\w-])/` (blockDetector.ts:6); the entry (blockDiagram.ts:8-13).                                                                                                                                                                                                |
| `rendering-util/rendering-elements/shapes/composite.ts`                                                 | 45             | The composite shape (10-42).                                                                                                                                                                                                                                                           |
| `rendering-util/rendering-elements/shapes/blockArrow.ts`                                                | 217            | Direction expansion (28-48), 16 point tables (53-181), `getArrowPoints` (183-198), `block_arrow` (200-217).                                                                                                                                                                            |
| `rendering-util/rendering-elements/shapes/insertPolygonShape.ts`                                        | 20             | `polygon.label-container` with `translate(-w/2, h/2)` (3-20).                                                                                                                                                                                                                          |
| `rendering-util/rendering-elements/shapes/util.ts`                                                      | 387            | `labelHelper` (50-217; `createText` call 93-101; `getBBox` 135-138; translate 145-150; the empty `rect` 152), `updateNodeBounds` (220-247), `getNodeClasses` (249-250).                                                                                                                |
| `rendering-util/rendering-elements/shapes/drawRect.ts`, `squareRect.ts`, `roundedRect.ts`, `hexagon.ts` | 90, 33, 20, 90 | The rect family and the hexagon. Label padding: squareRect 9-10, roundedRect 16-17, hexagon 35-36.                                                                                                                                                                                     |
| `rendering-util/rendering-elements/nodes.ts`                                                            | 100            | `insertNode` (14-53: rect choice 22-29, `data-look` 55); `positionNode` (77-100, transform 97).                                                                                                                                                                                        |
| `rendering-util/rendering-elements/edges.js`                                                            | 1,091          | `resolveEdgeCurveType` (43-45), `insertEdgeLabel` (74-134; `createText` 88), `resolveEdgeLabelPosition` (284-299), `positionEdgeLabel` (301-377), `insertEdge` (624-967).                                                                                                              |
| `rendering-util/rendering-elements/edgeMarker.ts`                                                       | 139            | `addEdgeMarkers` (13-37), `arrowTypesMap` (39-57), margin list (59-69), `addEdgeMarker` (71-), the no-arrow case (84-86), the unknown-type warning (91-93).                                                                                                                            |
| `rendering-util/rendering-elements/markers.js`                                                          | 1,022          | `insertMarkers` (6-10), `point` (323-388), `circle` (389-464), `cross` (465-533), the table (997-1021).                                                                                                                                                                                |
| `utils/lineWithOffset.ts`                                                                               | -              | `markerOffsets` (6-19), `markerOffsets2` (21-25), `getLineFunctionsWithOffset` (56-).                                                                                                                                                                                                  |
| `rendering-util/labelTransform.ts`                                                                      | -              | `computeLabelTransform`, the SVG branch (27-36).                                                                                                                                                                                                                                       |
| `rendering-util/insertLookDefs.ts`                                                                      | 73             | The two drop-shadow filters (46-47) and the gradient (49-73).                                                                                                                                                                                                                          |
| `rendering-util/createText.ts`                                                                          | 419            | `computeWidthOfText` (114-125), `createFormattedText` (152-), its wrap test (172-173), its background (182-190), `createText` (306-), default `width = 200` (319).                                                                                                                     |
| `diagrams/common/colorThemeGate.ts`                                                                     | 125            | `COLOR_THEMES` (17), `hasPalette` (33), `paletteSlotCount` (83), `stampColorSlot` (114-126).                                                                                                                                                                                           |
| `setupGraphViewbox.js`                                                                                  | -              | `calculateSvgSizeAttrs` (23-), `configureSvgSize` (43-).                                                                                                                                                                                                                               |
| `mermaidAPI.ts` (`src/`)                                                                                | -              | `createCssStyles` (142-196; the classDef CSS, shape list 163), `compileCSS` (226-), `createUserStyles` (319-334), the placeholder `g` (419), the renderer's `getClasses` call (594).                                                                                                   |
| `utils.ts` (`src/`)                                                                                     | -              | `getStylesFromArray` (423-439).                                                                                                                                                                                                                                                        |
| `utils/subGraphTitleMargins.ts`, `config.ts`, `defaultConfig.ts`, `schemas/config.schema.yaml`          | -              | The edge-label y offset (3-16), `getEffectiveHtmlLabels` (324-326), `htmlLabels: undefined` (24), the block and base defaults (`BlockDiagramConfig` 2714-2724, `BaseDiagramConfig.useMaxWidth` 464-470, `flowchart.curve` default 'basis' near 2521, `subGraphTitleMargin` 2464-2476). |

---

## 2. Parser

### 2.1 Pipeline

- The text goes through the shared preprocessing first. A `%%` comment line is removed [probed]. Front matter is removed, and entities are encoded (see Q20). Then the Jison parser runs with `yy = db`.
- The grammar's only DB call is `yy.setHierarchy($2)` at the end of `start` (`block.jison:184-186`). The populate step runs inside it. Other `yy` calls come from the actions: `getLogger`, `generateId`, `typeStr2Type`, and the four `edgeStr*` helpers (`block.jison:214`, `224-227`, `231`, `244-245`).
- Newlines separate nothing. The lexer skips whitespace and newlines (`block.jison:43-44`), and both come before the `NL` rule (`:46`). So `NL`, `separator`, `stop` and `spaceLines` (`:169-195`) are unreachable. Statements follow each other directly: `A:2 B` on one line is two statements [probed].
- Lexing is by the first rule in written order, not the longest match. ADR 0002 says this is how the Jison grammars with start conditions behave. Keywords written as strings match whole words only: `blockA`, `spaceship`, `defaults`, `columnsX`, `endpoint`, `classy`, `stylish` are all node ids [probed]. The mechanism (an implicit `\b` after string keywords) is inferred from these results [unverified as a mechanism].

### 2.2 Lexer states (`%x`, `block.jison:12-27`)

| State                                                                               | Used by                            | Note                                                                                                                     |
| ----------------------------------------------------------------------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `acc_title`, `acc_descr`, `acc_descr_multiline`                                     | 74-80                              | The tokens exist, but no production uses them (Q14).                                                                     |
| `string`                                                                            | 53-54, 135-136                     | Quoted text. No escapes: `\"` ends the string at the backslash (Q15).                                                    |
| `space`                                                                             | declared at 16                     | No rule uses it.                                                                                                         |
| `md_string`                                                                         | 49-51, 129-132                     | Markdown strings. In `INITIAL` they give `MD_STR`; in `NODE` they give `NODE_DESCR`. The grammar takes only `STR` (Q13). |
| `NODE`                                                                              | 83-100 (end), 102-121 (start), 133 | Inside a shape. The end tokens pop the state.                                                                            |
| `BLOCK_ARROW`, `ARROW_DIR`                                                          | 123, 134, 138-145                  | Block arrows `<["…"]>(dirs)`.                                                                                            |
| `LLABEL`                                                                            | 152-159                            | Edge label after `-- `, `-. `, `== `.                                                                                    |
| `CLASSDEF`, `CLASSDEFID`, `CLASS`, `CLASS_STYLE`, `STYLE_STMNT`, `STYLE_DEFINITION` | 61-72                              | The `classDef`, `class` and `style` statements.                                                                          |

### 2.3 Tokens

| Token                                                | Pattern (line)                                      | Value                                                                                           |
| ---------------------------------------------------- | --------------------------------------------------- | ----------------------------------------------------------------------------------------------- | -------------------------------- |
| `id-block`                                           | `"block:"` (39)                                     | A composite header with an id.                                                                  |
| `BLOCK_DIAGRAM_KEY`                                  | `"block"` (40)                                      | The header and the anonymous composite.                                                         |
| `COLUMNS`                                            | `"columns"\s+"auto"` (47), `"columns"\s+[\d]+` (48) | `-1` for auto, else the number.                                                                 |
| `SPACE_BLOCK`                                        | `space[:]\d+` (55), `space` (56)                    | The number of cells; a bare `space` is 1.                                                       |
| `DEFAULT`, `LINKSTYLE`, `INTERPOLATE`                | 57-59                                               | No production uses `LINKSTYLE` or `INTERPOLATE`. `DEFAULT` is used only after `classDef` (62).  |
| `classDef` … `CLASSDEF_STYLEOPTS`                    | 61-64                                               | `classDef id opts`; `default` is allowed as the id.                                             |
| `class`, `CLASSENTITY_IDS`, `STYLECLASS`             | 66-68                                               | `class A,B name`.                                                                               |
| `style`, `STYLE_ENTITY_IDS`, `STYLE_DEFINITION_DATA` | 70-72                                               | `style A,B opts`.                                                                               |
| `end`                                                | `"end"\b\s*` (81)                                   | Closes a composite.                                                                             |
| `NODE_DSTART`                                        | 103-121                                             | The opening of a shape: `-)`, `(-`, `))`, `)`, `(((`, `((`, `{{`, `{`, `>`, `([`, `(`, `[[`, `[ | `, `[(`, `)))`, `[\`, `[/`, `[`. |
| `NODE_DEND`                                          | 83-100                                              | The closing of a shape (in state `NODE`).                                                       |
| `BLOCK_ARROW_START`                                  | `"<["` (123)                                        |                                                                                                 |
| `NODE_ID`                                            | `[^\(\[\n\-\)\{\}\s\<\>:=]+` (125)                  | Any run of other characters, `~` and `.` included.                                              |
| `EOF`                                                | `<<EOF>>` (126)                                     |                                                                                                 |
| `STR`, `NODE_DESCR`                                  | 133-136                                             | Text in quotes.                                                                                 |
| `DIR`                                                | 139-144                                             | `right`, `left`, `x`, `y`, `up`, `down`, with an optional leading comma.                        |
| `BLOCK_ARROW_END`                                    | `"]>"\s*"("` then `")"\s*` (138, 145)               |                                                                                                 |
| `LINK`                                               | 148-151                                             | `--`, `==`, `-.-`, `~~~` with optional `x`, `o`, `<` at the start and `x`, `o`, `>` at the end. |
| `START_LINK`, `LINK_LABEL`                           | 152-156                                             | The start of a labelled edge.                                                                   |
| `SIZE`                                               | `':'\d+` (160)                                      | Only after a standalone node (Q17).                                                             |

### 2.4 Grammar (`block.jison:164-292`)

| Production                                                 | Line    | What the DB receives                                                                                                                                                                                                                  |
| ---------------------------------------------------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `start: BLOCK_DIAGRAM_KEY document EOF`                    | 184-186 | `setHierarchy(document)`. The block must have at least one statement (Q1).                                                                                                                                                            |
| `document: statement \| statement document`                | 198-201 | An array of statements.                                                                                                                                                                                                               |
| `link: LINK \| START_LINK LINK_LABEL STR LINK`             | 203-208 | `{edgeTypeStr, label}`.                                                                                                                                                                                                               |
| `statement`                                                | 210-219 | One of: `nodeStatement`, `columnsStatement`, `SPACE_BLOCK` (a `space` node with a generated id and `width`), `blockStatement`, `classDefStatement`, `cssClassStatement`, `styleStatement`.                                            |
| `nodeStatement: nodeStatement link node`                   | 222-233 | Three items: the left node, an edge `{id: "<start>-<end>", start, end, label, type: 'edge', thickness, pattern, directions, arrowTypeEnd, arrowTypeStart}`, and the right node. The left side is a node only for the first link (Q2). |
| `nodeStatement: node SIZE`                                 | 234     | The node with `widthInColumns = parseInt(SIZE)`.                                                                                                                                                                                      |
| `nodeStatement: node`                                      | 235     | The node with `widthInColumns = 1`.                                                                                                                                                                                                   |
| `columnsStatement: COLUMNS`                                | 239-241 | `{type: 'column-setting', columns}`.                                                                                                                                                                                                  |
| `blockStatement: id-block nodeStatement document end`      | 244     | A composite: the node's fields, `type: 'composite'`, `children`. The generated id on line 244 is computed and dropped.                                                                                                                |
| `blockStatement: BLOCK_DIAGRAM_KEY document end`           | 245     | A composite with `id` generated, `label: ''`.                                                                                                                                                                                         |
| `node: NODE_ID \| NODE_ID nodeShapeNLabel`                 | 248-256 | `{id, label, typeStr, directions}`.                                                                                                                                                                                                   |
| `nodeShapeNLabel`                                          | 262-267 | `NODE_DSTART STR NODE_DEND` gives `typeStr = start + end`; `BLOCK_ARROW_START STR dirList BLOCK_ARROW_END` gives `typeStr = '<[]>'` and `directions`.                                                                                 |
| `dirList`                                                  | 258-260 | The directions as written.                                                                                                                                                                                                            |
| `classDefStatement`, `cssClassStatement`, `styleStatement` | 270-290 | `{type: 'classDef' \| 'applyClass' \| 'applyStyles', id, css / styleClass / stylesStr}`.                                                                                                                                              |

The rules `spaceLines`, `separator` and `stop` (169-195) are unreachable (see 2.1).

### 2.5 Quirks that a port reproduces

Each row is a reference result. "Probed" means the full pipeline gave it; "source" means the code gives it.

| ID  | Input                                                                                    | Reference result                                                                                                                                                                                                        | Status and where                                                                                                                                                    |
| --- | ---------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Q1  | `block` alone                                                                            | Parse error.                                                                                                                                                                                                            | Probed. `block.jison:184`.                                                                                                                                          |
| Q2  | `A --> B --> C`                                                                          | Blocks `[{}, C]`; edge `1-undefined-C` with no start. In the render: a node with id `undefined` and an empty label, the edge dropped. `A -- "x" --> B -- "y" --> C` gives the same, with label `y`.                     | Probed. Garbage (an `undefined` id). The cause is `$1.id` on an array (`block.jison:222-231`). Named exception candidate (section 8).                               |
| Q3  | `A o--o B`                                                                               | `A` is a node; the edge is `o` → `B` with `arrow_open` start and `arrow_circle` end. `A --o B` is fine.                                                                                                                 | Probed. The space is skipped (`:43`) and `o` matches `NODE_ID` (`:125`) before the link rule (`:149`).                                                              |
| Q4  | `A ~~~ B`                                                                                | The node `~~~`, and no edge.                                                                                                                                                                                            | Probed. `NODE_ID` (`:125`) comes before the `~~` link rule (`:151`).                                                                                                |
| Q5  | `A --- B`                                                                                | Edge with `arrowTypeEnd: ''`.                                                                                                                                                                                           | Probed. `edgeStrToEdgeData` (`blockDB.ts:250-262`).                                                                                                                 |
| Q6  | `A ==== B`                                                                               | Edge, `thick`, no arrow.                                                                                                                                                                                                | Probed.                                                                                                                                                             |
| Q7  | `A -. "t" .-> B`                                                                         | Edge, `dotted`, label `t`, `arrow_point` end.                                                                                                                                                                           | Probed.                                                                                                                                                             |
| Q8  | `A <--> B`                                                                               | Edge with `arrow_point` at both ends.                                                                                                                                                                                   | Probed. `blockDB.ts:264-276`.                                                                                                                                       |
| Q9  | Any edge with no leading `<`, `x` or `o`                                                 | `arrowTypeStart: 'arrow_open'`, which has no marker. The marker code logs `Unknown arrow type: arrow_open`.                                                                                                             | Probed for the value; the warning is source (`edgeMarker.ts:91-93`). Every block edge without a leading mark goes through it.                                       |
| Q10 | `style B fill:#f00` before `B` is declared, or for an id that never is                   | Render and parse fail: `Cannot set properties of undefined (setting 'styles')`.                                                                                                                                         | Probed. `blockDB.ts:61-66`, run in list order at `:133-138`.                                                                                                        |
| Q11 | `class Q black` for an id that is declared later or never                                | No error. `Q` becomes a `na` entry in the DB map, not in the tree, so it is not drawn.                                                                                                                                  | Probed. `blockDB.ts:76-89`.                                                                                                                                         |
| Q12 | `columns 0`                                                                              | Render fails: `Columns must be an integer !== 0.` `getColumns('root')` gives 1, the number of children, because 0 is falsy.                                                                                             | Probed. `layout.ts:14-16`; `blockDB.ts:306-312`.                                                                                                                    |
| Q13 | `A["`**bold**`"]`                                                                        | Parse error: `Expecting 'STR', got 'MD_STR'`. Markdown labels are not supported.                                                                                                                                        | Probed. `block.jison:49`, `129-132`, `262-264`.                                                                                                                     |
| Q14 | `accTitle: x`, `accDescr: x`, `linkStyle 0 stroke:#f00`, a bare `default`, `interpolate` | Parse error for each.                                                                                                                                                                                                   | Probed. Tokens exist (`:58-59`, `74-80`), no production uses them.                                                                                                  |
| Q15 | `A[ "spaced" ]`, `A["a\"b"]`                                                             | Lexical error (a space after `[`; a backslash in a string).                                                                                                                                                             | Probed. `block.jison:83-136` has no rule for either.                                                                                                                |
| Q16 | `x-y`                                                                                    | Lexical error. `a.b` is a node named `a.b`.                                                                                                                                                                             | Probed. `-` is excluded from `NODE_ID` (`:125`).                                                                                                                    |
| Q17 | `A --> B:2`                                                                              | Parse error. `SIZE` only follows a standalone node (`:234`).                                                                                                                                                            | Probed.                                                                                                                                                             |
| Q18 | `A:2 B`; `block:X:2` … `end`                                                             | `A` has span 2, `B` span 1. The composite `X` has span 2.                                                                                                                                                               | Probed.                                                                                                                                                             |
| Q19 | `space:0`; `space`; `space:3`                                                            | `space:0` gives no cell; `space` gives one cell with a generated id; `space:3` gives three 1-column cells, ids `<generated>-0`, `-1`, `-2`.                                                                             | Probed. `blockDB.ts:177-185`.                                                                                                                                       |
| Q20 | `A["x #35; y &amp; z"]`                                                                  | The DB keeps the placeholder `x ﬂ°°35¶ß y …`. The rendered text reads `x`, `&#35;`, `y`, `&`, `z`.                                                                                                                      | Probed. Entities are encoded in preprocessing and decoded at render (`labelHelper` decodes, `shapes/util.ts:95`). The rendered `&#35;` is what the reference draws. |
| Q21 | `A["<script>x</script>y <b>b</b>"]`                                                      | Label `y <b>b</b>`. In the SVG the `<b>` is literal text, `&lt;b&gt;`.                                                                                                                                                  | Probed. The label is sanitised (`blockDB.ts:122-124`), and SVG text does not parse tags.                                                                            |
| Q22 | `A["line<br>break"]`; a literal `\n` in a label                                          | Two rows in both cases.                                                                                                                                                                                                 | Probed. `createText` splits rows.                                                                                                                                   |
| Q23 | `classDef dd color:#fff,fill:#000;`                                                      | `styles: ['color:#fff', 'fill:#000']`, `textStyles: ['fill:#fff']`. The text style takes `color` and turns it into `fill`. The regex is unanchored, so `background-color:` would also give `background-fill:` [source]. | Probed. `blockDB.ts:44-48`.                                                                                                                                         |
| Q24 | `A` … later `A["Ay"]`                                                                    | `A` keeps its first position with the label `Ay`; the second mention is not a child.                                                                                                                                    | Probed. `blockDB.ts:155-172`.                                                                                                                                       |
| Q25 | The same edge statement twice                                                            | Two edges, ids `1-A-B` and `2-A-B`.                                                                                                                                                                                     | Probed. `blockDB.ts:141-145`.                                                                                                                                       |
| Q26 | `block-beta` as the first line                                                           | Not detected as block.                                                                                                                                                                                                  | Source. The detector's negative lookahead (`blockDetector.ts:6`).                                                                                                   |
| Q27 | `A<["a"]>(foo)`                                                                          | Lexical error. Directions are `right`, `left`, `up`, `down`, `x`, `y` only.                                                                                                                                             | Probed.                                                                                                                                                             |
| Q28 | `A<["a"]>(x)`                                                                            | The DB keeps `['x']`. The renderer expands `x` to `right` and `left`.                                                                                                                                                   | Probed (DB); expansion source (`blockArrow.ts:28-48`).                                                                                                              |
| Q29 | `block:g` … `end` followed by `g:2`                                                      | The span of the later mention is ignored: the composite keeps its own span.                                                                                                                                             | Probed (render). A later mention of an existing id never changes it (`blockDB.ts:155-172`).                                                                         |
| Q30 | An anonymous `block` … `end`                                                             | A composite with a generated id, `label: ''`.                                                                                                                                                                           | Probed. `block.jison:245`.                                                                                                                                          |
| Q31 | `block:e` followed directly by `end`                                                     | Parse error: a composite needs a document.                                                                                                                                                                              | Probed.                                                                                                                                                             |
| Q32 | `classDef default fill:#f00`                                                             | The class id is `default`. Nodes without classes use the class string `default` in the DOM, but no style is applied from this class.                                                                                    | Probed (DB); the DOM class string is `renderHelpers.ts:19-23`.                                                                                                      |

### 2.6 Errors

- A Jison parse error reads `Parse error on line N:` with the line, a caret and an `Expecting …` list. A lexical error reads `Lexical error on line N. Unrecognized text.` [probed]. ADR 0002 says the Rust port reports the failure and its position, not the text. The list of expected tokens depends on the LALR states and is not copied.
- The two DB errors of Q10 happen in populate, during the parse. The `columns 0` error of Q12 happens in layout, during the render.

---

## 3. DB

### 3.1 Calls, in order

| When                   | Call                                                                                                                                                               | Effect                                                              |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------- |
| Parse (each space)     | `generateId()` (`block.jison:214`)                                                                                                                                 | Id `id-<random>-<n>`.                                               |
| Parse (each composite) | `generateId()` (`:244`, the value is dropped; `:245`, the id is used)                                                                                              | Id for `block` composites.                                          |
| Parse (each node)      | `typeStr2Type` (`:231`, `:234-235`)                                                                                                                                | Shape type.                                                         |
| Parse (each edge)      | `edgeStrToEdgeData` (`:224`), `edgeStrToEdgeStartData` (`:225`), `edgeStrToThickness` (`:226`), `edgeStrToPattern` (`:227`)                                        | Arrow types, thickness, pattern.                                    |
| End of parse           | `setHierarchy(document)` (`:185`)                                                                                                                                  | Root children, then `populateBlockDatabase` (`blockDB.ts:295-299`). |
| Render                 | `setDiagramId(id)` (`blockRenderer.ts:24`)                                                                                                                         | Prefix of the DOM ids.                                              |
| Render                 | `getBlocks`, `getBlocksFlat`, `getEdges`, `getBlock`, `setBlock`, `getDiagramId`, `getClasses` (`blockRenderer.ts:50-52`, `renderHelpers.ts`, `mermaidAPI.ts:594`) | Reads and the size writes.                                          |

### 3.2 Populate, step by step (`blockDB.ts:105-192`)

Items are processed in list order, one list per level. For each item:

1. Warning, if the item has a span and the first `column-setting` of its list is smaller than the span: `Block <id> width <w> exceeds configured column width <n>` (`:110-121`). Probed through the logger.
2. Label sanitising, for any non-empty label, edge labels included (`:122-124`).
3. `classDef` → `addStyleClass` (`:125-128`, `31-52`).
4. `applyClass` → `setCssClass` for each id of the comma list. An unknown id becomes a `na` entry in the DB map, not in the tree (`:129-132`, `76-89`).
5. `applyStyles` → `addStyle2Node`: `blockDatabase.get(id)!.styles = …`. An unknown id throws (`:133-138`, `61-66`; Q10).
6. `column-setting` → `parent.columns = block.columns ?? -1`. Not a child (`:139-141`).
7. `edge` → count-prefixed id (`edgeCount` keyed by the unprefixed id, `:142-144`) and pushed to `edgeList` (`:145`). Edges are not children.
8. Anything else (`:146-190`):
   - The label defaults to the id, and to `''` for composites (`:147-154`).
   - An id seen before keeps its first position: its type changes unless the new type is `na`, and its label is replaced when the new label differs from the id (`:155-172`). The block is not pushed again.
   - A new composite takes `colorIndex = nextColorIndex++` before its children are walked, so an outer composite gets a lower index (`:160-162`).
   - The block goes into the DB map (`:163`), then its children are populated (`:174-176`).
   - A space with span `w` becomes `w` clones with ids `<id>-<j>`, each put in the map and the children (`:177-185`). The original is in the map but not the children.
   - Anything else is pushed to the children (`:186-188`).
9. `parent.children = children` (`:191`).

### 3.3 What is stored, and what is not

- Root: `{id: 'root', type: 'composite', children, columns: -1}` (`:195`, `200`).
- `blockDatabase` (a Map of all blocks, including style and class targets and the originals of spaces), `classes` (a Map id → `{id, styles, textStyles}`), `edgeList`, `edgeCount`, `diagramId`, `nextColorIndex`.
- Sanitising: `sanitizeText` with the current config (`:22`, `:123`), which is DOMPurify with `dompurifyConfig` [probed: `<script>` removed, `<b>` kept]. Class names, `classDef` strings and style strings are not sanitised here. The render sanitises the label again after decoding (`shapes/util.ts`, labelHelper).
- Defaults:
  - `label`: the id, or `''` for composites.
  - `type`: `'na'` for a plain id, or the shape from `typeStr2Type` (`:211-248`). `'na'` for a class-only entry.
  - `widthInColumns`: 1 for a plain node (`block.jison:235`), N after `:N` (`:234`), and a composite takes the span of its id node (`:244`). A `block` composite has none (`:245`). The renderer treats a missing span as 1 (`renderHelpers.ts:131`, `layout.ts:61`, `116`, `133`, `237`).
  - `columns`: -1 for the root (`:195`). A composite has none unless it says `columns`, and the layout treats that as -1 (`layout.ts:130`, `213`).
  - `directions`: only for block arrows (`block.jison:229-231`, `266`).
- `clear` (`:197-209`) resets the root, `blockDatabase`, `blocks`, `classes`, `edgeList`, `edgeCount`, `diagramId` and `nextColorIndex`. It does not reset `cnt` (`:289`), so generated ids keep counting across parses on one page [source]. The common accessibility fields are cleared by `commonClear` (`:199`).
- `getColumns` (`:301-313`) returns `block.columns` if truthy, else the number of children (Q12). The renderer does not call it. Only the specs do.

### 3.4 DB facts a port has to keep

- Edge ids are the count of that edge key plus the key, so they are unique per page and not per diagram: `1-A-B`, `2-A-B`.
- The first mention of an id decides its place in the tree. A later mention can change its label and type but never its place or its span.
- Colour indices count composites only, across the whole parse, in pre-order (`blockColorIndex.spec.ts` checks this).
- `classDef` before or after `class` makes no difference to the rendered style, because the class map is read at render (`renderHelpers.ts:24-26`). Only `style` depends on order (Q10).

---

## 4. Canonical model proposal

The differential compares the DB's output after the parse. Fields follow the P1 models (camelCase; `undefined` is absent).

```
BlockDiagram {
  columns: number            // root's columns; -1 when not set or auto
  blocks: Block[]            // the root's children, in order
  edges: Edge[]              // edgeList, in order
  classDefs: ClassDef[]      // classes Map, in insertion order
}

Block {
  id: string | null          // generated ids replaced by placeholders (below); null for the undefined id of Q2
  type: string               // the BlockType name, 'na' when none
  label?: string             // absent for space cells (their label is the generated id)
  widthInColumns?: number    // as stored; absent when the parser left it undefined
  columns?: number           // composites only
  directions?: string[]      // block arrows, as written (x and y kept)
  classes?: string[]         // applyClass order, duplicates kept
  styles?: string[]          // applyStyles: split on ','
  colorIndex?: number        // composites only
  children?: Block[]         // composites and the root
}

Edge {
  id: string                 // "<n>-<start>-<end>", or "<n>-undefined-<end>" for Q2
  start?: string             // absent when undefined (Q2)
  end?: string
  label: string              // '' when none
  arrowTypeStart: string     // arrow_open, arrow_point, arrow_cross, arrow_circle
  arrowTypeEnd: string       // the same, or '' for no arrow
  thickness: 'normal' | 'thick'
  pattern: 'solid' | 'dotted'
}

ClassDef { id: string, styles: string[], textStyles: string[] }
```

Placeholders: generated ids (`id-<random>-<n>`) are replaced by `$gen1`, `$gen2`, … in order of appearance, on both sides, as P1 does for auto ids. A space cell's label is excluded with its generated id. `width` on expanded spaces is excluded (the renderer uses `widthInColumns`). Excluded: `size`, `intersect`, `node`, `parent` and the flat map, which are render state. A reference rejection (Q1, Q10, Q13, Q14, Q15, Q16, Q17, Q31) is a rejected input, as the exporter records errors for the other types. Q2 is a reproduced quirk (section 8).

---

## 5. Specs

| File                                                          | Tests                                             | What it tests                                                                                                                                                                                                                                               | Layer                    |
| ------------------------------------------------------------- | ------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------ |
| `diagrams/block/parser/block.spec.ts`                         | 26 `it`                                           | Parse and DB: ids and labels, shapes, edges, `columns`, composites, block arrows, `:N` spans, space cells, `classDef`, `class`, `style`, the column warning, `__proto__` and `constructor` ids.                                                             | Parser, DB               |
| `diagrams/block/blockDB.spec.ts`                              | 4                                                 | Edge thickness, pattern and arrow helpers. `getConfig` read at sanitising time, not at import.                                                                                                                                                              | DB                       |
| `diagrams/block/layout.spec.ts`                               | 2                                                 | `calculateBlockPosition` for columns 2 and 1; `layout` reads `block.padding` at call time (width 212 with padding 4, 260 with 20).                                                                                                                          | Layout                   |
| `diagrams/block/blockColorIndex.spec.ts`                      | 8                                                 | Colour indices: declaration order, outer before inner, one counter, simple blocks take none. Palette rules: one per entry, no `color-2` for a 2-entry palette, a theme without a palette emits nothing, a `look` that would break the selector is rejected. | DB, styles               |
| `rendering-util/rendering-elements/shapes/blockArrow.spec.ts` | 18 runtime (16 direction cases in a loop, plus 2) | `getArrowPoints` for each direction key, the expansion of `x` and `y`, deduplication, the natural width.                                                                                                                                                    | Renderer shape           |
| `styles.spec.ts:92-112`                                       | shared                                            | Block styles are valid for each diagram theme.                                                                                                                                                                                                              | Styles                   |
| `diagrams/common/colorThemeGate.spec.ts:164`                  | shared                                            | Mentions block styles as an interpolation point.                                                                                                                                                                                                            | Styles                   |
| `e2e/rendering/block/pr-check/block.spec.js`                  | 1 (Playwright, BL38)                              | In a composite with a long label, `A`, `B`, `C` do not overlap. It reads the rect `x` and `width` and the `translate`.                                                                                                                                      | Renderer (reference e2e) |

The specs check the parser and DB well, the layout data a little, and the renderer hardly at all. The renderer is covered by the fixtures and by the geometry harness.

---

## 6. Renderer

### 6.1 Pipeline (`blockRenderer.ts:16-81`)

1. `db.setDiagramId(id)` (`:24`). In sandbox mode the root is found through the iframe (`:26-37`); that mode is out of scope.
2. The root `svg` already exists. `mermaidAPI` created it with `width="100%"`, the xmlns, the id, and an empty `g` (`mermaidAPI.ts:418-419`). The style element is added by `createUserStyles` (section 6.9).
3. `insertLookDefs(svg, config)` (`:42`). Always two drop-shadow filters: `<id>-drop-shadow` (`dx=4`, `dy=4`, `stdDeviation=0`, `flood-opacity=0.06`, `height`/`width` 130%) and `<id>-drop-shadow-small` (`dx=2`, `dy=2`, 150%). The flood colour is `#000000`, or `#FFFFFF` on a dark theme. A gradient is added only when `themeVariables.useGradient` is set (`insertLookDefs.ts:24-73`). [probed: two `defs` with one filter each, no gradient on the redux themes.]
4. `insertMarkers(svg, ['point', 'circle', 'cross'], 'block', id)` (`:45-48`). Twelve `marker` elements, direct children of the `svg` (no `defs` wrapper for these three; `markers.js:323-533`) [probed]: `<id>_block-pointEnd`, `pointStart`, `pointEnd-margin`, `pointStart-margin`, the same four for `circle`, and for `cross` (class `marker cross block`). The `-margin` variants are the neo ones.
5. `nodes = svg.insert('g').attr('class', 'block')` (`:54`).
6. `calculateBlockSizes(nodes, bl, db)` (`:55`): the measure pass (section 6.2).
7. `layout(db)` (`:56`): sizes and positions, in the arithmetic of section 6.4. Returns the bounds, used only as a fallback.
8. `insertBlocks(nodes, bl, db)` (`:57`): the placed pass (section 6.5). Composites and other blocks, pre-order; a space is skipped (`renderHelpers.ts:162`).
9. `insertEdges(nodes, edges, blArr, db, id)` (`:58`): section 6.7.
10. `renderedBounds = nodes.node().getBBox()` (`:62`), the bounds of `g.block` with its nodes and edges, and the fallback to the layout bounds when they are not finite (`:63-68`).
11. `magicFactor = max(1, round(0.125 * width / height))`, `height = bounds.height + magic + 10`, `width = bounds.width + 10` (`:70-72`).
12. `configureSvgSize(svg, height, width, useMaxWidth)` (`:74`). With `useMaxWidth` (default true): `width="100%"` and `style="max-width: <width>px;"` (`setupGraphViewbox.js:23-35`). Otherwise `height` and `width` attributes (`:36-41`).
13. `viewBox = "<x-5> <y-5> <w+10> <h+10>"` from `bounds2` (`:76-79`). The magic factor is not in the viewBox, so with `useMaxWidth: false` the height and the viewBox differ by the magic factor [probed: `height="117"`, `viewBox="-5 -58 337.4375 116"`].

Not called: `insertTitle`, `setupViewPortForSVG`, `setupGraphViewbox`, the accessibility title and description (none is set by block).

### 6.2 Measurements, and what they feed

| Where                                             | Call                                             | Measures                                              | Feeds                                                                                                                  | Pass                                              |
| ------------------------------------------------- | ------------------------------------------------ | ----------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------- |
| `labelHelper` (`shapes/util.ts:135-138`)          | `text.getBBox()` on the SVG text                 | The label's bounding box                              | The node's size (`drawRect` 21-22, hexagon 46-49), and the label's translate (`util.ts:145-150`: `translate(0, -h/2)`) | Both                                              |
| `createText` (`createText.ts:114-125`, `172-173`) | `getComputedTextLength()` of a test tspan        | The width of a candidate line                         | The wrap decision `width <= limit` (`<=`, so a tie does not wrap)                                                      | Both; the limit is `Infinity` in the measure pass |
| `createFormattedText` (`createText.ts:182-190`)   | `getBBox()` of the text                          | The background rect: the bbox grown by 2 on each side | `rect.background` in each label and edge label                                                                         | Both                                              |
| `calculateBlockSize` (`renderHelpers.ts:149-154`) | `getBBox()` of the whole node group              | The block's measured width and height                 | `obj.size`, the input of the layout                                                                                    | Measure only                                      |
| `insertEdgeLabel` (`edges.js:121-125`)            | `getBBox()` of the label group and of its text   | The label's bbox                                      | `computeLabelTransform` (`labelTransform.ts:27-36`), so the label's translate                                          | Once per edge label                               |
| `blockRenderer` (`:62`)                           | `getBBox()` of `g.block`                         | The drawn bounds                                      | The root `svg` size and viewBox                                                                                        | After edges                                       |
| `composite` (`composite.ts:37`)                   | `updateNodeBounds` (reads `getBBox` of the rect) | Only when the size is not known                       | The node's width and height, which the layout does not read                                                            | Both                                              |
| `insertEdge` (`edges.js:879`)                     | `getTotalLength()` of the path                   | The path's length                                     | Only the neo dash arrays (`edges.js:883-886`)                                                                          | Placed, neo only                                  |

The two passes matter. In the measure pass a node has no width and no height, so `wrappingWidth` is `Infinity` (`renderHelpers.ts:125`) and the label does not wrap [probed: a 680 px label is one row, and its block is 680 px]. In the placed pass a node has its final width, and the label wraps at the width the shape leaves for it. Shapes that change `node.width` first (the hexagon) change the wrap limit too (section 6.5).

### 6.3 Node data (`renderHelpers.ts:16-136`)

- `classStr` = the block's classes joined by spaces, or `default`, plus ` flowchart-label` (`:19-23`).
- `cssCompiledStyles` = the styles of the classes (`:24-26`).
- `shape` by type (`:33-97`), table in 6.6.
- `labelStyle`, `style` from `getStylesFromArray(vertex.styles)` (`:99`; `utils.ts:423-439`).
- `label` = the sanitised label (`:102`).
- `bounds = vertex.size`, or zeros (`:104`). `domId = <diagramId>-<id>` (`:120`).
- `width`, `height` = `bounds` or `undefined` when zero (`:123-124`). `wrappingWidth = bounds.width || Infinity` (`:125`).
- `x`, `y` = the block's centre from the layout (`:126-127`).
- `padding = block.padding ?? 8` (`:130`; schema 2724). `widthInColumns` (`:131`). `colorIndex` (`:132`). `look = getConfig().look` (`:133`).

### 6.4 Layout arithmetic (`layout.ts`)

Read in this order. `p` is `block.padding` (default 8, read at call time: `layout.ts:357`, `renderHelpers.ts:130`). A space's size is never used for its own column, only for the row height of its cell.

**Sizes, bottom up** (`setBlockSizes`, 72-207). For a parent with children `c_i` of span `s_i`:

1. `maxWidth = max over non-space children of (w_i / s_i)`, and `maxHeight = max over non-space children of h_i` (`getMaxChildSize`, `58-67`).
2. Each child, spaces too: `w_i' = maxWidth * s_i + p * (s_i - 1)`, `h_i' = maxHeight` (`:115-117`). Then each child is sized with `siblingWidth = maxWidth`, `siblingHeight = maxHeight` (`:127`).
3. `numItems = Σ s_i` (`:133`). `xSize = n` (number of children), or `columns` when `0 < columns < numItems` (`:137-140`). `ySize = ceil(numItems / xSize)` (`:142`).
4. `width = xSize * (maxWidth + p) + p`, `height = ySize * (maxHeight + p) + p` (`:144-145`).
5. If `width < siblingWidth`: `width = siblingWidth`, `height = siblingHeight`, and each child becomes `((siblingWidth - xSize·p - p)/xSize, (siblingHeight - ySize·p - p)/ySize)` (`:147-168`). Note that the children get the sibling size, not a natural size.
6. If `width < block.size.width` (the width the parent assigned in step 2): `width` is that assigned width, and each child's width becomes `(width - num·p - p) / num`, with `num = min(n, columns)` for a finite `columns` (`:176-189`). Only the width changes here.
7. `block.size = {width, height, x: 0, y: 0}` (`:191-196`).

A leaf keeps the measured size from the measure pass (`renderHelpers.ts:152`). The root is sized too (`:358`) but not drawn.

**Positions, top down** (`layoutBlocks`, 209-322). Called with the root, then each composite:

1. Row of each child: `calculateBlockPosition(columns, colPos)` (`:11-36`): `columns < 0` gives `(pos, 0)`; `columns == 1` gives `(0, pos)`; otherwise `(pos % columns, floor(pos / columns))`. A `columns` of 0 throws (`:14-16`; Q12). `colPos` advances by the span, clipped to the end of the row for a finite `columns` (`:310-315`).
2. Row heights: the largest child height per row (`:224-243`). A row's y offset is the sum of the heights and paddings of the rows above it (`:244-252`).
3. `startingPosX` is the parent's left edge (`parent.x - parent.width/2`) for a composite, or `-p` for the root. It resets at each new row (`:257`, `:267-271`).
4. `child.x = startingPosX + p + w/2`, then `startingPosX = child.x + w/2` (`:277`, `:292`). So the next child starts `p` after the previous one's right edge.
5. `child.y = parent.y - parent.height/2 + rowYOffset + rowHeight/2 + p` (`:296-297`). The row height is the row's, not the child's.
6. Recurse into composite children (`:307-309`).

**Bounds** (`findBounds`, 324-349; `layout`, 351-369): the extremes over every block except the root, spaces included. The renderer does not use them unless the DOM bounds are not finite (`blockRenderer.ts:63-68`).

**Check** [probed, by hand from the arithmetic]: in `A`, `space`, `A -- "apa" --> E`, `A` is centred at 20.38, the space at 69.15 and `E` at 117.91. These are the values the reference draws (`edges-should-handle-labels` fixture).

**Quirks**: `columns 0` throws (Q12). A composite whose centre is exactly 0 takes the `-p` branch at `layout.ts:257`, because `0` is falsy. No fixture reaches that, because children start at `p + w/2 > 0` [source]. The `-x / 2 || 0` at `:257` and `:269` is `-(w/2)` with a fallback that is never reached for a finite width [source].

### 6.5 Node shapes in the placed pass (`renderHelpers.ts:158-187`)

`insertBlockPositioned` builds the node (6.3) with the placed size, calls `insertNode`, and stamps composites with `stampColorSlot` (`:173-183`). It sets `block.intersect = node.intersect` (`:184`) and `positionNode` (`:185`), which sets `transform="translate(x, y)"` (`nodes.ts:97`). The shape writes `node.intersect`, so every shape can clip an edge; I checked each block shape and all set it [source].

### 6.6 Shapes

| Block type            | Shape in `getNodeFromBlock` (`renderHelpers.ts`)                   | Handler (`shapes.ts` alias)                                                   | Status in ferrigura (`unified/shapes/registry.rs`)                                |
| --------------------- | ------------------------------------------------------------------ | ----------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| `na`, `square`        | `rect` (`:95-96`, `:43-44`) → `squareRect` (`nodes.ts:22-29`)      | `squareRect.ts`                                                               | `SquareRect` (`"rect"`, `"squareRect"`)                                           |
| `round`               | `rect` with `rx = ry = 5` (`:34-37`) → `roundedRect`               | `roundedRect.ts` (radius from themeVariables, default 5; label padding = `p`) | `RoundedRect`                                                                     |
| `circle`              | `circle` (`:73-75`)                                                | `circle.ts`                                                                   | `Circle`                                                                          |
| `doublecircle`        | `doublecircle` (`:92-94`)                                          | `doubleCircle.ts`                                                             | `DoubleCircle`                                                                    |
| `diamond`             | `question` (`:46-48`)                                              | `question.ts`                                                                 | `Question`                                                                        |
| `hexagon`             | `hexagon` (`:49-51`)                                               | `hexagon.ts` (Q4 wrap)                                                        | `Hexagon`                                                                         |
| `stadium`             | `stadium` (`:80-82`)                                               | `stadium.ts`                                                                  | `Stadium`                                                                         |
| `subroutine`          | `subroutine` (`:83-85`)                                            | `subroutine.ts`                                                               | `Subroutine`                                                                      |
| `cylinder`            | `cylinder` (`:86-88`)                                              | `cylinder.ts`                                                                 | `Cylinder`                                                                        |
| `lean_right`          | `lean_right` (`:58-60`)                                            | `leanRight.ts`                                                                | `LeanRight`                                                                       |
| `lean_left`           | `lean_left` (`:61-63`)                                             | `leanLeft.ts`                                                                 | `LeanLeft`                                                                        |
| `trapezoid`           | `trapezoid` (`:64-66`)                                             | `trapezoid.ts`                                                                | `Trapezoid`                                                                       |
| `inv_trapezoid`       | `inv_trapezoid` (`:67-69`)                                         | `invertedTrapezoid.ts`                                                        | `InvTrapezoid`                                                                    |
| `rect_left_inv_arrow` | `rect_left_inv_arrow` (`:55-57`, `:70-72`)                         | `rectLeftInvArrow.ts`                                                         | `RectLeftInvArrow` (`"odd"`, `"rect_left_inv_arrow"`)                             |
| `composite`           | `composite` (`:38-41`)                                             | `composite.ts`                                                                | **Missing.** Section 6.8.                                                         |
| `block_arrow`         | `block_arrow` (`:52-54`)                                           | `blockArrow.ts`                                                               | **Missing.** Section 6.10.                                                        |
| `ellipse`             | `ellipse` (`:76-79`)                                               | Not registered (`shapes.ts`)                                                  | Unreachable: no grammar path makes it. The reference would throw `No such shape`. |
| `group`               | `rect` (`:89-91`); `calculateBlockSize` returns early (`:143-145`) | none                                                                          | Unreachable: no path makes it.                                                    |
| `space`               | not drawn (`:162`)                                                 | none                                                                          | Measured only (`:150`).                                                           |

Shape names in the table are the ones `getNodeFromBlock` returns. The `typeStr2Type` values are in `blockDB.ts:211-248`, and their mapping to shapes is the `switch` in `renderHelpers.ts:33-97`.

### 6.7 Edges (`renderHelpers.ts:215-302`, `edges.js:624-967`)

- The graphlib instance gets `rankdir TB` and spacing (`:222-232`), and a node for each block with a size, carrying `{width, height, intersect}` (`:234-242`). Nothing calls `layout`, so the graph is a keyed store [source].
- An edge with `start` and `end` and both sizes is drawn (`:246-251`); otherwise it is dropped silently. That is why Q2's edge is missing.
- Points: `[start centre, midpoint, end centre]` (`:253-257`).
- Classes: `edge-thickness-normal|thick edge-pattern-solid|dotted flowchart-link LS-a1 LE-b1` (`:260-264`).
- `insertEdge(elem, {...edge, id: <id>-<edge.id>, points, classes}, {}, 'block', g.node(start), g.node(end), id)` (`:266-281`).

Inside `insertEdge`:

- The diagram id must be set, or it throws (`:635-637`).
- The layout is the configured `layout` (`:640-641`), not a parameter of block. With `layout: swimlane` the swimlane clipping branch runs instead (`:658`, unverified in a probe). The default `layout` is `elk` (schema near 82), which takes the other branch.
- Clipping: `points = [tail.intersect(mid), mid, head.intersect(mid)]` (`:699-703`). Both intersect functions are the shapes' own (6.5).
- `data-points` is `btoa(JSON.stringify(points))` (`:705`, `:899`). The JSON formatting of numbers must match for that attribute.
- The line: the default curve `basis` (`resolveEdgeCurveType` returns `flowchart.curve`, `:43-45`; default `'basis'`, schema near 2521). `fixCorners` is applied to non-rounded curves (`:726-730`). The path is `curveBasis` through the points (`:732-770`), with the x and y accessors from `getLineFunctionsWithOffset` (`:777`).
- Offsets (`lineWithOffset.ts:6-19`, `56-`): the first and last points move back along the line by `markerOffsets[arrowType] * cos(angle)`. Only `arrow_point` has an offset here (4); `arrow_circle` and `arrow_cross` have none in `markerOffsets` (their entries are commented out), though `markerOffsets2` (12.5) gives the dash arithmetic [source]. [probed: the path of `A`→`E` ends at 93.53, which is `E`'s left edge 97.53 minus 4.]
- Stroke classes: `strokeClasses` from thickness and pattern (`:780-794`), then the edge's `classes` appended (the class attribute is at `:861-866`). The result repeats `edge-thickness-normal edge-pattern-solid` [probed].
- `style`: `edge.style` is undefined for block edges, so `edgeStyles = [undefined]` (`:812`). `pathStyle` becomes `undefined;;;undefined` (`:853-856`) [probed]. The stroke colour is then empty.
- The path: `id = <diagramId>-<edge.id>` (`:860`), which doubles the prefix [probed: `b10-b10-1-A-E`]. `data-edge="true"`, `data-et="edge"`, `data-id`, `data-points`, `data-look` (`:896-900`). The `d` attribute is the curve.
- Markers: `addEdgeMarkers` (`:947`) gives `marker-start` and `marker-end` as `url(#<id>_block-<type><Start|End>[-margin])` (`edgeMarker.ts:71-112`). `url` is empty unless `flowchart.arrowMarkerAbsolute` (`:934-937`). Block edges have no colour, so no coloured copies are made. The `-margin` ids are used when the look is neo and the marker is point, circle or cross (`:946`, `edgeMarker.ts:59-69`).
- `arrow_open` (every edge without a leading mark) has no marker, and logs `Unknown arrow type: arrow_open` (`edgeMarker.ts:84-93`) [source].

Labels (`renderHelpers.ts:282-298`, `edges.js:74-134`, `301-377`):

- The label style is `stroke: #333; stroke-width: 1.5px;fill:none;` (`:286`).
- `insertEdgeLabel`: `g.edgeLabel` appended to `elem` (`edges.js:81`), then `g.label` with `data-id = edge.id` (`:84`), then `createText` with `addSvgBackground: true` and no `width` (`:88-97`), so the default width of 200 (`createText.ts:319`) applies and the label wraps at 200 [probed: a 190 px box in several rows].
- The label transform: `computeLabelTransform(textEl.getBBox())` = `translate(-(x + w/2), -(y + h/2))` (`edges.js:130`, `labelTransform.ts:27-36`). [probed: `translate(0, -9.500000953674316)`.]
- `edgeLabels.set(edge.id, edgeLabel)` (`:133`). `positionEdgeLabel` then sets `g.edgeLabel`'s transform to `translate(mid.x, mid.y + subGraphTitleTotalMargin/2)` (`:301-377`). The margin is `flowchart.subGraphTitleMargin` top plus bottom, 0 by default (`subGraphTitleMargins.ts:12-14`; schema 2464-2476).
- The label group is after its path in the DOM, and both are after all nodes.

### 6.8 Composite (`composite.ts:10-42`)

- Class: `['node', cssClasses, class]` joined by spaces (`:14`). `cssClasses` and `class` are both `default flowchart-label` when no class is set, so the attribute reads `node default flowchart-label default flowchart-label` [probed].
- `labelHelper(parent, node, classes)` (`:15`). The composite's label is `''`, so the text is empty and the label group is `translate(0, 0)` [probed].
- `rect` inserted first (`:16`), with `class="basic cluster composite label-container"`, `rx`/`ry` (null when unset, so absent), `style` (null when unset), `x`, `y`, `width`, `height` (`:27-37`). Positioned: `width = node.width`, `height = node.height`, `x = -w/2`, `y = -h/2` (`:18-25`). `intersect` is the rect's (`:39-41`).
- DOM: `g.node` (`id`, `data-look`, `data-color-id` when coloured, `transform`), with `rect.basic.cluster.composite.label-container` and then `g.label` [probed].
- DOM order: composites are siblings of their children, not parents. Each block is a child of `g.block`, in pre-order, so a composite's children are next to it, not inside its group [probed: `g id="b46-inner1"` is a sibling of `g id="b46-outer"`]. So the children paint over their container.
- Palette: `stampColorSlot(shapeSvg, colorIndex, theme, borderColorArray)` (`renderHelpers.ts:173-183`; `colorThemeGate.ts:114-126`) sets `data-color-id="color-<i mod P>"`, where `P` is the palette length (12 in both redux themes). It is set only when `colorIndex` is defined and the theme is `redux-color` or `redux-dark-color` (`colorThemeGate.ts:17`, `:33`). Any other theme gets no slot and no palette rules.

### 6.9 Style sheet (`styles.ts`, `mermaidAPI.ts:142-334`)

- `createUserStyles(config, graphType, classDefs, svgId)` (`mermaidAPI.ts:319-334`): `createCssStyles` (142-196) for the classDef rules, then block's `getStyles` with `{...themeVariables, theme, look}`, then `compileCSS` (226-) with `stylis`, which prefixes every rule with `#<id>` and flattens the nested `.edgeLabel p` and `.edgeLabel rect` rules [probed: `#b1 .edgeLabel p{margin:0;padding:0;display:inline;}`].
- Classdef rules (`mermaidAPI.ts:161-196`): for each class, `#<id> .<cls> rect|polygon|ellipse|circle|path {…!important}` (the htmlLabels-off list, `:163`), and `#<id> .<cls> tspan {fill…!important}` for each `textStyles` entry. The browser's CSSOM rewrites the colours in `rgb()` form [probed: `rgb(102, 102, 255)`].
- Block's own rules (`styles.ts:78-207`): `.label` font and colour (79-83); `.cluster-label` (84-89); `.label text, span` (93-96); `.node rect, circle, ellipse, polygon, path` with `fill: mainBkg`, `stroke: nodeBorder`, `stroke-width: strokeWidth px` (98-110; the comment at 105-108 says why it is not pinned to 1 px); `.flowchart-label text { text-anchor: middle }` (111-113); `.node .label { text-align: center }` (121-123); `.node.clickable` (124-126); `.arrowheadPath` (128-130); `.edgePaths .path` (132-135); `.flowchart-link` (137-140); `.edgeLabel` with nested `p` and `rect` (142-161); `.labelBkg` (164-166); `.node .cluster` with `fade(clusterBkg, 0.5)` and `fade(clusterBorder, 0.2)` (168-174; khroma `rgba` output [probed: `rgba(249, 249, 251, 0.5)`]); `.cluster text`, `span` (176-182); `div.mermaidTooltip` (187-199); `.flowchartTitleText` (201-205); `getIconStyles()` (206).
- The palette rules (`genColor`, `styles.ts:41-64`): one rule per slot `i < P`, `[data-look="<look>"][data-color-id="color-<i>"].node rect.composite {stroke: border[i]; fill: bkg[i]}` (53-61). Not `!important`, so an inline `style` wins. `look` is checked by `safeLook` (`colorThemeGate.ts`, see the spec). Only composites match (`rect.composite`).
- Theme variables read: `arrowheadColor`, `border2`, `clusterBkg`, `clusterBorder`, `edgeLabelBackground`, `fontFamily`, `lineColor`, `mainBkg`, `nodeBorder`, `nodeTextColor`, `strokeWidth`, `tertiaryColor`, `textColor`, `titleColor` (`styles.ts:6-28`); `theme`, `look`, `borderColorArray`, `bkgColorArray`, `THEME_COLOR_LIMIT` (palette). `radius` is read by `roundedRect.ts:10`. `useGradient`, `gradientStart`, `gradientStop` are read by `insertLookDefs.ts:26`, `49-73`.
- Per-node inline styles: `drawRect` uses `styles2String` (`handDrawnShapeStyles.ts:75-104`), which merges the classes' styles and the node's `style` statements (`compileStyles` 31-41), keeps the label ones (`color`, `font-*` and the other `isLabelStyle` keys, 53-73) and appends `!important` to each. [probed: `style="fill:#66f !important;stroke:#333 !important;stroke-width:2px !important"`.] `styles2Map` splits on `:`, so a value with a second colon loses everything after it [source].

### 6.10 Block arrow (`blockArrow.ts`)

- `labelHelper` with `getNodeClasses` (`:203`; `util.ts:249-250`): the class is `node <cssClasses> ` with a trailing space.
- Height `bbox.height + 2p`; `midpoint = height/2`; `naturalWidth = bbox.width + 2·midpoint + p`; `isSpanning` when the node is placed, its span is above 1 and its width exceeds the natural width; then the width is the node width (`:207-213`).
- `getArrowPoints(directions, bbox, node, width)` (`:183-198`): `padding = p/2`; `h = bbox.height + 4·padding`; `midpoint = h/2`; `width` as given or `bbox.width + 2·midpoint + 2·padding`. The key is the set of directions, expanded (`x` → right and left, `y` → up and down) and written in the order right, left, up, down (`:28-50`). A key with no entry, or no direction, gives the single point `(0, 0)` (`:50-51`, `:180`).
- The 16 keys with their tables (`:53-181`): `right|left|up|down`, `right|left|up`, `right|left|down`, `right|up|down`, `left|up|down`, `right|left`, `up|down`, `right|up`, `right|down`, `left|up`, `left|down`, `right`, `left`, `up`, `down`, and `point`.
- Shape: `insertPolygonShape` gives `polygon.label-container` with `points` and `transform="translate(-w/2, h/2)"` (`insertPolygonShape.ts:3-20`). The `style` is the node's style. `intersect` is `intersect.polygon` on those points (`:217`).
- Probed: the five fixtures with block arrows render (`block-arrow-should-span-multiple-columns…`, `should-handle-block-arrows-in-different-directions`, `should-handle-block-arrows-and-spece-statements`, `should-render-all-arrow-types-in-9-columns-grid`, and `look-neo` with its `right<["Yes"]>(right)`).

### 6.11 Configuration section and defaults

| Key                                                           | Default (reference)                                                      | Read by                                                              | Notes                                                                                                                                                              |
| ------------------------------------------------------------- | ------------------------------------------------------------------------ | -------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `block.padding`                                               | 8 (`schema 2721-2724`)                                                   | `layout.ts:357`, `renderHelpers.ts:130`, `shapes` label padding      | Read at call time (the layout spec checks it). Composites use 0 for their own `padding`.                                                                           |
| `block.useMaxWidth`                                           | true (`BaseDiagramConfig`, `schema 464-470`)                             | `blockRenderer.ts:73-74`                                             | Decides `width="100%"` and `max-width` (6.1 step 12).                                                                                                              |
| `theme`                                                       | the diagram's own default, then the top level (`BaseDiagramConfig`)      | `styles.ts`, `colorThemeGate.ts`                                     | Only `redux-color` and `redux-dark-color` have a palette.                                                                                                          |
| `look`                                                        | `classic` (`getConfig().look`)                                           | `data-look` (`nodes.ts:55`), `renderHelpers.ts:133`, the neo rules   | Neo changes the padding of squares and hexagons, the marker margins and the edge dash arithmetic.                                                                  |
| `htmlLabels`                                                  | `undefined`, read as `true` (`defaultConfig.ts:24`; `config.ts:324-326`) | `labelHelper` (`util.ts:61`), `insertEdgeLabel`                      | The production default draws HTML labels. The harness forces `false`. Decision 0004 draws SVG text.                                                                |
| `flowchart.curve`                                             | `'basis'` (schema near 2521)                                             | `edges.js:43-45`                                                     | Used by every block edge, since `edge.curve` is never set.                                                                                                         |
| `flowchart.subGraphTitleMargin`                               | top 0, bottom 0 (`schema 2464-2476`)                                     | `positionEdgeLabel` (`edges.js:301-377`)                             | Moves every edge label's y by half the sum.                                                                                                                        |
| `flowchart.wrappingWidth`                                     | 120 (schema 2419-2426)                                                   | `util.ts:98` (only when a node has neither width nor wrapping width) | Never reached by a block node, which always has `wrappingWidth` [source]. The 200 of `createText.ts:319` is a separate default, and it is the one edge labels use. |
| `flowchart.arrowMarkerAbsolute`                               | false                                                                    | `edges.js:934-937`                                                   | Adds the page URL to the marker `url()`.                                                                                                                           |
| `themeVariables.radius`                                       | theme value, default 5                                                   | `roundedRect.ts:10`                                                  |                                                                                                                                                                    |
| `themeVariables.useGradient`, `gradientStart`, `gradientStop` | theme values                                                             | `insertLookDefs.ts`                                                  | Gradient defs only when `useGradient` is set.                                                                                                                      |

The block section has no other keys. The `dompurifyConfig` is read for labels (`blockDB.ts:22`).

### 6.12 Randomness and the clock

- `generateId` (`blockDB.ts:289-293`) draws `Math.random()` and appends a counter `cnt`, which `clear` does not reset. It is used for space cells (`block.jison:214`, and the suffixes `-0`, `-1`, … at `blockDB.ts:182`), for `block` composites without an id (`:245`), and by the discarded `id2` (`:244`), which still advances `cnt`.
- The ids reach the DOM as `g id="<svg>-id-<random>-<n>"` for composites, for example `b12-id-haohqkaa9cq-7` [probed]. Space cells are not drawn, so their ids do not reach the DOM.
- No clock is read.
- The harness seeds `Math.random` when a clock is given (`common.ts:457-466`). The structural key ignores ids (`diagram-geometry.ts:19-20`).
- `btoa` is deterministic. Nothing else is random.

### 6.13 DOM dependencies a server renderer has to replace

- `getBBox` on SVG text, on label groups, on the node groups, and on `g.block` (section 6.2).
- `getComputedTextLength` on test tspans for the wrap decision (`createText.ts:122`).
- `getTotalLength` on the edge path, for neo edges only (`edges.js:879`).
- `getBoundingClientRect` only on the HTML label path (`createText.ts:73`, `util.ts:126-131`), not used with `htmlLabels: false`.
- `document.getElementById` in the coloured-marker path (`edgeMarker.ts:109-111`), not reached by block (no stroke colour).
- `window.location` only with `arrowMarkerAbsolute`.
- `CSSStyleSheet` and the browser's CSSOM rewriting of the classDef colours (6.9).

### 6.14 DOM structure, in order (the geometry key)

For one render with `id` and one composite, one plain block and one edge:

```
svg#<id>  width="100%" xmlns  style="max-width: …px;"  viewBox  role="graphics-document document"  aria-roledescription="block"
  style                                   (mermaidAPI; the sheet)
  g                                       (mermaidAPI placeholder; stays empty)
  defs > filter#<id>-drop-shadow > feDropShadow
  defs > filter#<id>-drop-shadow-small > feDropShadow
  marker.marker.block  ×12                (point, circle, cross × start, end, margin variants; direct children)
  g.block
    g.node.default.flowchart-label#<id>-<cid>  data-look  data-color-id?  transform
      rect.basic.cluster.composite.label-container
      g.label  transform="translate(0, 0)"
        rect                              (empty, first child)
        g
          rect.background  style="stroke: none"
          text[y=-10.1]
            tspan.text-outer-tspan.row[x=0, y=-0.1em, dy=1.1em]   (no inner tspans for '')
    g.node.default.flowchart-label#<id>-<bid>
      rect.basic.label-container[x, y, width, height, rx=0, ry=0, style]   (or polygon.label-container, path …)
      g.label  transform="translate(0, -h/2)"
        rect
        g
          rect.background  style="stroke: none"
          text[y=-10.1]
            tspan.text-outer-tspan.row[x=0, y=-0.1em, dy=1.1em]  (one per row)
              tspan.text-inner-tspan[font-style=normal, font-weight=normal]   (one per word)
    path#<id>-<id>-<edgeId>  class="edge-thickness-normal edge-pattern-solid edge-thickness-normal edge-pattern-solid flowchart-link LS-a1 LE-b1"
         style="undefined;;;undefined"  data-edge  data-et="edge"  data-id  data-points  marker-start?  marker-end?  d
    g.edgeLabel  transform="translate(x, y)"
      g.label  data-id  transform="translate(…)"
        g
          rect.background  x y width height
          text[y=-10.1, text-anchor=middle]
            tspan.text-outer-tspan.row[…, text-anchor=middle]
              tspan.text-inner-tspan …
```

Notes on the list:

- Edges come after every node, and each edge's label comes after its path [probed].
- The stadium's shape is `g.basic.label-container.outer-path` with two `path` children [probed].
- The `g` inside `g.label` is the createText group; the `rect.background` is its first child [probed].
- Attributes are not in the key; classes are, as a sorted list of the `classList` tokens, so the repeated tokens count (`diagram-geometry.ts:19-20`).

---

## 7. Fixtures

58 files: `nightly-check` 28, `pr-check` 10, `visual-check` 20. Features counted from the sources and from the 58 reference SVGs.

| Feature                                            | Fixtures                                                                                                                        | Where it shows                                                                                                                                                                                                                                                |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `columns N`                                        | 46                                                                                                                              | Every row layout (`layout.ts:11-36`).                                                                                                                                                                                                                         |
| Composites (`block:` or anonymous `block … end`)   | 32 SVGs with a composite rect                                                                                                   | `composite.ts`; the palette in the `visual-check` set.                                                                                                                                                                                                        |
| Space cells                                        | 15                                                                                                                              | Not drawn; they only take cells.                                                                                                                                                                                                                              |
| Spans `:N`                                         | 13                                                                                                                              | `layout.ts:61`, `115`, `176-189`.                                                                                                                                                                                                                             |
| Block arrows `<["…"]>`                             | 5                                                                                                                               | `blockArrow.ts`.                                                                                                                                                                                                                                              |
| Edges (any)                                        | 8 SVGs with an edge path                                                                                                        | `insertEdge`.                                                                                                                                                                                                                                                 |
| Edge labels                                        | 1 SVG (`edges-should-handle-labels`)                                                                                            | `insertEdgeLabel`.                                                                                                                                                                                                                                            |
| Edge variants (`-.->`, `==>`, `--x`, `--o`, `---`) | 3 files                                                                                                                         | The arrow and pattern rules.                                                                                                                                                                                                                                  |
| Wrapped labels (more than one row)                 | 7 SVGs: the hexagon `pr-check` fixture, the two `look-neo` fixtures (`Decision`), and four `shapes-inside-a-container` hexagons | Q4, `hexagon.ts`, `createText.ts:172-173`.                                                                                                                                                                                                                    |
| `style` statements                                 | 7 files                                                                                                                         | `styles2String` inline `!important`.                                                                                                                                                                                                                          |
| `classDef` / `class`                               | 1 file (`should-style-statements-and-class-statements`)                                                                         | Class CSS rules and the `!important` inline style.                                                                                                                                                                                                            |
| `look: neo`                                        | 22 files                                                                                                                        | Margin markers, neo dash arrays, `drop-shadow` filters, the hexagon padding.                                                                                                                                                                                  |
| Theme names                                        | 22 files                                                                                                                        | The 20 `visual-check` fixtures name `redux`, `redux-dark`, `redux-color` and `redux-dark-color`; the `redux` and `redux-dark` names are not shipped, so they fall back as the harness says. The `neo` and `neo-dark` names (pr-check) fall back the same way. |

What the generic harness must handle:

- No fixture needs `handDrawn` exclusion (0 files).
- No fixture needs a font exclusion: no non-ASCII characters in any diagram body (the 46 with non-ASCII are in the front matter description only).
- Randomness: the 32 composite ids. Ids are excluded from the key, and the reference is seeded by the harness.
- The reference renders all 58 without errors under the forced settings [probed].
- Outside the 58 are four layout playgrounds in `e2e/platform/dev-diagrams/layout-tests/block-diagrams/` (`test1.mmd`–`test4.mmd`). `test4.mmd` has `%% BL21`. They are not in the harness.

---

## 8. Risks, effort, open questions

### Riskiest parts

1. **Two passes, float boxes.** Every size is a `getBBox` of a group, read as float32 in Chromium (M5 rules). A one-ulp difference in a size moves every later block. The geometry tolerance (0.05 px) hides most of it, but not the wrap decisions (next item).
2. **Wrap decisions at a tie.** `createText` wraps when `getComputedTextLength() > width` (`createText.ts:172-173`), and the width is the node's width minus the padding the shape takes away (`hexagon.ts:40-41`). With the final width equal to the label plus the padding, the comparison is a tie decided by 1/64 px rounding. Seven fixtures have wrapped labels (section 7). The port must measure a text run with the same ceiling as Chromium, and do the same float arithmetic in the same order.
3. **Shapes that write to the node.** `hexagon.ts` changes `node.width` and `node.height` before it measures the label (`:40-41`), and the layout reads the node's width after that. The unified shape API must let a shape change the node, and the measure pass must give `Infinity` as the wrap limit.
4. **Block arrows.** A new shape with 16 direction tables and the spanning rule (`blockArrow.ts:207-213`). Five fixtures. The port can check it with the 16 spec cases and the geometry harness.
5. **Composites.** A new shape, its palette rules, the repeated class string, and the flat DOM order (6.8). Thirty-two fixtures.
6. **Edges.** Reuse the flowchart's `insertEdge` path, with a three-point input and the default `basis` curve. The offsets depend on `Math.cos` and `Math.atan`, which may differ in the last bit from V8 (inside tolerance) [unverified].
7. **Parser.** A hand-written lexer that takes the first rule in order, with start conditions, plus about 30 quirks (section 2.5). The errors are positions only (ADR 0002).
8. **Shared stylesheet.** The `stylis` pass and the classDef rules are shared with the flowchart; block adds its own rules and the palette.

### Effort (rough)

Parser and DB about 1,200 to 1,500 lines of Rust with the specs (the block folder's 40 specs and the 18 arrow specs). Layout about 400 lines. Renderer 800 to 1,000 lines, of which the composite and the block arrow are about 350. Styles about 150. Total 2,500 to 3,000 lines, and the fixtures and the harness on top. The shared parts are already in place for the flowchart, so the new work is the block-specific code.

### Open questions for the coordinator

- **Q-A. Garbage outputs.** Q2 (chained edges) draws a node with id `undefined`, an empty label and no edge. The `useMaxWidth: false` NaN (findings table, item 10) is the same kind. Reproduce them, or name them as exceptions? The recommendation is to reproduce Q2 (it is deterministic and in the DB model), and name the NaN as an exception, since NaN in an attribute is what the M6 rule names.
- **Q-B. Production HTML labels.** The default `htmlLabels` reads as true, so production block output uses HTML labels, which the harness never sees. The recommendation is to keep the decision-0004 SVG text for block as for the other types, and to say so in the port's docs.
- **Q-C. Generated ids.** Their reference values are random. Recommendation: the port uses one counter in the same format position (`id-<n>`), and the differential replaces ids by placeholders (section 4). Confirm.
- **Q-D. Repeated class tokens and `undefined;;;undefined`.** The class tokens count in the geometry key, so the port must repeat them. The `style` attribute is not in the key. Recommendation: reproduce both, since they are cheap.
- **Q-E. The e2e BL38 premise.** The "200 px wrap" in its name is not what the reference does (finding 3). Recommendation: keep the test as an overlap check and rename it in the port's spec port, not in the reference.
- **Q-F. The four playground files** under `e2e/platform/dev-diagrams/layout-tests/block-diagrams/`. Add them to the harness or leave them out?
- **Q-G. Parse error text.** Confirm that the port reports the failing position only (ADR 0002), with no expected-token list.

### Workspace notes

- While this ran, two untracked folders appeared in this worktree that I did not create: `docs-port/sankey/` (from 15:21) and `docs-port/tidy-tree/`. I left them alone.
- My probes created `target/geometry` (the harness's `mkdir`, `common.ts:580`). I removed it, since it was empty.
