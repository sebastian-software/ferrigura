# Venn (`venn-beta`): research notes for the port

Status: research only. Nothing in the repository changed except this file, and nothing is committed. Reference: `packages/mermaid/src` in this worktree (branch `sebastian/port-p3`, HEAD `f3a1a94b0`). Library: `@upsetjs/venn.js` 2.0.0, the version `packages/mermaid/package.json:73` pins and `pnpm-lock.yaml:1952` resolves. Plan: [../P3-PLAN.md](../P3-PLAN.md). Guide: [../P2-RESEARCH.md](../P2-RESEARCH.md). Model of finished notes: [../kanban/NOTES.md](../kanban/NOTES.md).

Every rule cites the reference as `diagrams/venn/<file>:<line>` unless the file is named in full. Library lines are `src/` lines of the 2.0.0 package (`node_modules/.pnpm/@upsetjs+venn.js@2.0.0/node_modules/@upsetjs/venn.js/src/`). The build is what Mermaid bundles, so the Math calls are also given with their `build/venn.esm.js` lines (see L1). **Probed** means the behaviour was run through the reference's own code. The parser and DB ran through `tools/port/reference.ts` (Vite SSR of the real `venn.jison`, `vennDB.ts`, `preprocess.ts`, `utils.ts` and `config.ts`). The rendering ran through the built bundle `packages/mermaid/dist` in Playwright's Chromium, with the harness's settings from `tools/port/geometry/common.ts` (`forcedSiteConfig`, `patchText`, `htmlLabels: false`, Lexica Ultralegible, `securityLevel: strict`, `redux-color` or `redux-dark-color`). The layout library ran from `build/venn.esm.js`, served to the same Chromium. **Unverified** means read only.

Probe material is outside the repository in `/private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/p3-venn/`: `probe-db.mts` with `cases-db*.json` (parser and DB), `fixtures.mts` (the 21 fixtures as the harness patches them), `probe-render.mjs` (all fixtures, determinism, layout against drawn circles), `probe-variants.mjs` with `variants*.json` (configuration and edge cases), `probe-config.mts` (the configuration layers and theme variables), `probe-random*.mjs`, `probe-branch*.mjs`, `probe-dup.mjs` and `probe-mismatch.mjs` (the randomised branch), and `khroma-probe.mjs`. Output files and SVGs are beside them.

Part A is the diagram type (sections 0 to 8). Part B is the layout engine that places the circles, `@upsetjs/venn.js` (sections L0 to L8).

---

# Part A: the diagram type

## 0. Summary

| Item                  | Value                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it is            | Area-proportional Venn diagrams: sets are circles, unions are intersections, each with a size; optional labels, text nodes inside regions, `style` fills and colours, a title                                                                                                                                                                                                                                                                            |
| Reference, TypeScript | `vennRenderer.ts` 437 lines, `vennDB.ts` 133, `vennTypes.ts` 36, `styles.ts` 33, `vennDiagram.ts` 13, `vennDetector.ts` 24: 676 lines                                                                                                                                                                                                                                                                                                                    |
| Reference, grammar    | `parser/venn.jison` 135 lines (lexer 1-46, grammar 48-135)                                                                                                                                                                                                                                                                                                                                                                                               |
| Layout library        | `@upsetjs/venn.js` 2.0.0 (MIT). Its `src/` is 1,869 lines (`layout.js` 820, `diagram.js` 783, `circleintersection.js` 266), and Mermaid uses most of them. Mermaid bundles `build/venn.esm.js` (2,214 lines, with `fmin` inlined)                                                                                                                                                                                                                        |
| Specs                 | 3 files, 36 `it`/`test` blocks written out. 47 tests run, because `vennPalette.spec.ts` expands over the 11 themes (section 5)                                                                                                                                                                                                                                                                                                                           |
| Fixtures              | 21 in `e2e/diagrams/venn/`. Three are `look: handDrawn` and the harness excludes them, so 18 are compared (section 7)                                                                                                                                                                                                                                                                                                                                    |
| Text measurement      | None. The one call that could measure, `getComputedTextLength` in `wrapText`, returns 0 on the detached element the renderer draws into (probed), so every set label is one `tspan`                                                                                                                                                                                                                                                                      |
| Randomness            | `Math.random` in the library, only when there are eight or more areas, which needs four or more sets, or duplicated statements (L4). No fixture reaches it                                                                                                                                                                                                                                                                                               |
| Riskiest parts        | (1) The circle layout: a Nelder-Mead and conjugate-gradient optimiser over floating point, with `Object.keys` order deciding the result for numeric set ids (L6). (2) The reference lays out the circles and the text nodes with two different padding values, 15 and 8 (6.5). (3) The Jison lexer's indent mode and comment rules (2.5). (4) Text nodes are HTML in `foreignObject` whatever `htmlLabels` says, which decision 0004 does not allow (Q1) |
| Effort                | Medium to large. Rough estimate: 2,200 to 2,700 lines of Rust: parser and DB about 600, the layout library about 1,000, the renderer and styles about 500, tests about 400 (section 8.2)                                                                                                                                                                                                                                                                 |

## 1. Sources

| File                                 | Lines           | What it holds                                                                                                                                                      |
| ------------------------------------ | --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `diagrams/venn/parser/venn.jison`    | 1-2             | `%lex`, `%options case-insensitive`                                                                                                                                |
|                                      | 4               | `%s bol`: the one start condition, inclusive                                                                                                                       |
|                                      | 7-8             | Comment rules: `\%\%(?!\{)[^\n]*` and `[^\}]\%\%[^\n]*` (the second eats one character before `%%`)                                                                |
|                                      | 10-19           | The `bol` rules: whitespace-only lines, indented `text` (INDENT_TEXT), leading whitespace, and the rule that clears indent mode                                    |
|                                      | 21-24           | Newline (begins `bol`), comment, blank, EOF                                                                                                                        |
|                                      | 26-31           | Keywords: `title` (with a regex, `"title"\s[^#\n;]+`), `venn-beta`, `set`, `union`, `text`, `style`                                                                |
|                                      | 33-34           | Bracket labels: `["..."]` and `[...]`                                                                                                                              |
|                                      | 36-41           | `NUMERIC`, `HEXCOLOR`, `RGBACOLOR`, `RGBCOLOR`, `IDENTIFIER`, `STRING`                                                                                             |
|                                      | 43-44           | `COMMA`, `COLON`                                                                                                                                                   |
|                                      | 52-54           | `start`: `optNewlines VENN document 'EOF'`                                                                                                                         |
|                                      | 61-69           | `document` and `line`                                                                                                                                              |
|                                      | 71-88           | `statement`: title (72), set (73-76), union (77-80), text (81-85), indented text (86), style (87)                                                                  |
|                                      | 90-96           | `indentedTextTail`: the text of an indented line, attached to the current sets (91-95)                                                                             |
|                                      | 98-123          | `stylesOpt`, `styleField`, `styleValue`, `valueTokens`, `valueToken`                                                                                               |
|                                      | 125-133         | `identifierList` and `identifier` (`IDENTIFIER` or `STRING`)                                                                                                       |
| `diagrams/venn/vennDB.ts`            | 16-18           | Module state: `subsets`, `textNodes`, `styleEntries`, `knownSets`, `currentSets` (19-20), `indentMode` (21)                                                        |
|                                      | 23-36           | `addSubsetData`                                                                                                                                                    |
|                                      | 38-40           | `getSubsetData`                                                                                                                                                    |
|                                      | 42-52           | `normalizeText` (42-48) and `normalizeStyleValue` (50-52)                                                                                                          |
|                                      | 54-61           | `addTextData`                                                                                                                                                      |
|                                      | 63-74           | `addStyleData` and `getStyleData`                                                                                                                                  |
|                                      | 76-78           | `normalizeIdentifierList`                                                                                                                                          |
|                                      | 80-86           | `validateUnionIdentifiers` (the throw is at 84)                                                                                                                    |
|                                      | 88-96           | `getTextData`, `getCurrentSets`, `getIndentMode`, `setIndentMode`                                                                                                  |
|                                      | 98-102          | `getConfig`: `cleanAndMerge(DEFAULT_CONFIG.venn, commonGetConfig().venn)`                                                                                          |
|                                      | 104-112         | `customClear`: the common clear, then every array and flag above                                                                                                   |
|                                      | 114-133         | The `db` object                                                                                                                                                    |
| `diagrams/venn/vennRenderer.ts`      | 17-29           | `buildStyleByKey`: key is the sorted targets joined by `\|`; later entries for a key override earlier ones (`Object.assign`)                                       |
|                                      | 31-36           | `draw`                                                                                                                                                             |
|                                      | 37-61           | Configuration and theme read (`getConfig` at 39), the eight palette colours (41-50), title, sets, text nodes, styles, `renderSets` (61)                            |
|                                      | 64-70           | Sizes: `svgWidth` (64), `svgHeight` (65), `REFERENCE_WIDTH` 1600 (66), `scale` (67), `titleHeight` (68), `defaultTextColor` (70)                                   |
|                                      | 73-95           | Root `viewBox` (74), title (76-87), the detached dummy root and the library's chart (90-95)                                                                        |
|                                      | 97-106          | Hand-drawn setup (97-99) and the layout call `venn.layout` (102-106)                                                                                               |
|                                      | 109-117         | `layoutByKey` (109-113) and `renderTextNodes` (115-117)                                                                                                            |
|                                      | 120-169         | Set circles: palette colour (128), base colour (129), class (130), opacity and stroke (131-133), the path style (154-160), the label style (163-168)               |
|                                      | 172-230         | Union labels and paths (the hand-drawn branch at 172-206, the plain branch at 207-230)                                                                             |
|                                      | 233-240         | The group that receives the chart's nodes (233-239) and `configureSvgSize` (240)                                                                                   |
|                                      | 243-245         | `stableSetsKey`                                                                                                                                                    |
|                                      | 247-374         | `renderTextNodes`: debug layout (255), grouping (260-269), the area and its centre (272-287), the grid (308-318), each node (321-372)                              |
|                                      | 376-435         | `ensurePairwiseSubsets` (doc 376-392; body 393-434)                                                                                                                |
|                                      | 437             | `renderer` export                                                                                                                                                  |
| `diagrams/venn/styles.ts`            | 1-6             | `VennChartStyleOptions`; `textColor` (5) is declared and never read                                                                                                |
|                                      | 8-31            | `getStyles`: `.venn-title` (10-14), `.venn-circle text` (16-19), `.venn-intersection text` (21-25), `.venn-text-node` (27-30)                                      |
| `diagrams/venn/vennTypes.ts`         | 4-36            | `VennData`, `VennTextData`, `VennStyleData`, `VennDB`                                                                                                              |
| `diagrams/venn/vennDiagram.ts`       | 8-12            | The definition: `parser`, `db`, `renderer`, `styles`                                                                                                               |
| `diagrams/venn/vennDetector.ts`      | 10              | `/^\s*venn-beta/`: no word boundary                                                                                                                                |
| `schemas/config.schema.yaml`         | 2978-3007       | `VennDiagramConfig`: `theme` 'redux-color' (2987), `look` 'neo' (2990), `width` 800 (2995), `height` 450 (3000), `padding` 8 (3004), `useDebugLayout` false (3007) |
|                                      | 457-470         | `BaseDiagramConfig.useMaxWidth`, default `true` (470)                                                                                                              |
|                                      | 71-75           | `handDrawnSeed`, default 0                                                                                                                                         |
| `config.type.ts`                     | 2424-2458       | `VennDiagramConfig` interface                                                                                                                                      |
| `defaultConfig.ts`                   | 335-337         | `venn: {...defaultConfigJson.venn}`, the defaults the schema gives                                                                                                 |
| `config.ts`                          | 76-107          | `resolveAppearance`: `theme`, `look` and `layout` resolved onto the top level, the diagram's own section first (101-103 keeps the section in step)                 |
|                                      | 144-152         | `setDiagramConfigScope` (149)                                                                                                                                      |
| `Diagram.ts`                         | 49              | `db.clear()`                                                                                                                                                       |
|                                      | 53              | `db.setDiagramTitle` from front matter                                                                                                                             |
|                                      | 66-83           | The text the parser gets: `code.cleaned`, `encodeEntities(...) + '\n'` (77), `parser.parse` (83)                                                                   |
| `preprocess.ts`                      | 58-91           | `preprocessDiagram`: front matter, directives, `cleanupComments`                                                                                                   |
| `diagram-api/comments.ts`            | 6-8             | `cleanupComments`: whole-line `%%` comments removed, then `trimStart`                                                                                              |
| `utils.ts`                           | 874-            | `encodeEntities`                                                                                                                                                   |
| `diagrams/common/commonDb.ts`        | 10-13, 28-30    | `clear`; `setDiagramTitle` sanitises (`sanitizeText`)                                                                                                              |
| `diagrams/common/common.ts`          | 71-84           | `sanitizeText` (DOMPurify)                                                                                                                                         |
| `mermaidAPI.ts`                      | 319-332         | `createUserStyles`: `{...config.themeVariables, theme, look}` is the options object of the diagram's `styles` (330)                                                |
|                                      | 697-704         | `fontFamily` copied into `themeVariables.fontFamily` when the theme does not set one (699-703)                                                                     |
| `setupGraphViewbox.js`               | 23-47           | `calculateSvgSizeAttrs` (23) and `configureSvgSize` (43). The venn renderer does not call `setupGraphViewbox` (no `getBBox`)                                       |
| `rendering-util/selectSvgElement.ts` | 11-22           | `selectSvgElement`; reads `securityLevel` (12)                                                                                                                     |
| `tools/port/geometry/common.ts`      | 60-104, 564-570 | The harness's forced settings and patched text; the seeded `Math.random` replacement of `renderReference`                                                          |

Two more files set the context: `Diagram.ts` (19-85) and the generic render path, which the packet notes describe in section 6.1 (`packet/NOTES.md`). The venn renderer adds nothing to that path except the style call above.

## 2. Parser

### 2.1 Lexer

The lexer has one start condition beyond INITIAL: `bol`, which is inclusive (`%s`, line 4). A rule without a condition applies in both. Jison takes the first rule that matches, in written order (`venn.jison`, not `flex`), so the order of rules 7 to 44 decides. Indent mode (`db.getIndentMode`, set by `set` and `union` and cleared by a line whose first character is not blank) decides whether an indented `text` is INDENT_TEXT.

| State | Line  | Pattern                                           | Token and effect                                                                                         |
| ----- | ----- | ------------------------------------------------- | -------------------------------------------------------------------------------------------------------- | --------- |
| all   | 7     | `\%\%(?!\{)[^\n]*`                                | Skipped: a `%%` comment, not a `%%{` directive                                                           |
| all   | 8     | `[^\}]\%\%[^\n]*`                                 | Skipped. Consumes the character before `%%` as well, which may be any character but `}` (2.5, item 2)    |
| bol   | 10    | `[ \t]+(?=[\n\r])`                                | Skipped: a line of blanks                                                                                |
| bol   | 11-17 | `[ \t]+(?=text\b)`                                | If indent mode: consumes the blanks, begins INITIAL, returns `INDENT_TEXT`, and sets `consumeIndentText` |
| bol   | 18    | `[ \t]+`                                          | Skipped: leading blanks                                                                                  |
| bol   | 19    | `[^ \t\n\r]`                                      | Clears indent mode, begins INITIAL and puts the character back (`unput`)                                 |
| all   | 21    | `[\n\r]+`                                         | `NEWLINE`, and begins `bol`                                                                              |
| all   | 22    | `\%\%[^\n]*`                                      | Skipped                                                                                                  |
| all   | 23    | `[ \t]+`                                          | Skipped                                                                                                  |
| all   | 24    | `<<EOF>>`                                         | `EOF`                                                                                                    |
| all   | 26    | `"title"\s[^#\n;]+`                               | `TITLE` with the rest of the line. A `#` or `;` in the title is a lexical error (probed)                 |
| all   | 27-31 | `venn-beta`, `set`, `union`, `text`, `style`      | Keywords. `text` returns no token when `consumeIndentText` is set                                        |
| all   | 33    | `\[\"[^\"]*\"\]`                                  | `BRACKET_LABEL`, quotes removed                                                                          |
| all   | 34    | `\[[^\]\"]+\]`                                    | `BRACKET_LABEL`, trimmed                                                                                 |
| all   | 36    | `[+-]?(\d+(\.\d+)?                                | \.\d+)`                                                                                                  | `NUMERIC` |
| all   | 37-39 | `#` hex of 3 to 8 digits, `rgba(...)`, `rgb(...)` | `HEXCOLOR`, `RGBACOLOR`, `RGBCOLOR`                                                                      |
| all   | 40    | `[A-Za-z_][A-Za-z0-9\-_]*`                        | `IDENTIFIER`                                                                                             |
| all   | 41    | `\"[^\"]*\"`                                      | `STRING`, quotes kept (`normalizeText` removes them later)                                               |
| all   | 43-44 | `,` and `:`                                       | `COMMA`, `COLON`                                                                                         |

A character that no rule takes is a lexical error (probed: `#amp;` in a text line, and `<b>` in a set id).

### 2.2 Grammar

```
start        : optNewlines VENN document EOF
document     : (empty) | document line
line         : NEWLINE | statement
statement    : TITLE                                   -> setDiagramTitle(text after 'title ')
             | SET identifier                          -> addSubsetData([id]); indent mode on
             | SET identifier BRACKET_LABEL            -> addSubsetData([id], label); indent mode on
             | SET identifier COLON NUMERIC            -> addSubsetData([id], undefined, size); indent mode on
             | SET identifier BRACKET_LABEL COLON NUMERIC
             | UNION identifierList (+ the same four forms)
                   -> throws if fewer than two identifiers; validateUnionIdentifiers; addSubsetData; indent mode on
             | TEXT identifierList IDENTIFIER|STRING|NUMERIC [BRACKET_LABEL]   -> addTextData
             | INDENT_TEXT indentedTextTail
             | STYLE identifierList stylesOpt         -> addStyleData
indentedTextTail : IDENTIFIER | STRING | NUMERIC | IDENTIFIER BRACKET_LABEL | STRING BRACKET_LABEL
                   -> throws 'text requires set' if there is no current set; addTextData(current sets, ...)
stylesOpt    : styleField (COMMA styleField)*
styleField   : IDENTIFIER COLON styleValue
styleValue   : STRING | valueToken+        (joined with one space)
identifierList : identifier (COMMA identifier)*
identifier   : IDENTIFIER | STRING
```

Statements need no separator: two statements on one line are accepted (probed: `set A set B`). The grammar's line 73 sets indent mode through `yy.setIndentMode`, and `union` (77-80) validates before it adds.

### 2.3 Preprocessing and detection

`Diagram.fromText` runs `preprocessDiagram` (`preprocess.ts:58-91`), which removes front matter and directives and calls `cleanupComments` (`comments.ts:6-8`). It then gives the parser `encodeEntities(code.cleaned) + '\n'` (`Diagram.ts:77`). The venn DB does not set `preserveCommentsWhenParsing`, so the comment-stripped text is the one parsed. Detection is `/^\s*venn-beta/` (`vennDetector.ts:10`), so `VENN-BETA` is not detected, though the lexer is case-insensitive. `venn-betax` is detected, because the regex has no word boundary, and by reading of the grammar it fails to parse (unverified).

### 2.4 Probed behaviour

Each row gives the input after preprocessing, run through the reference's parser and DB (`probe-db.mts`), unless it says "render". "Node" means a subset, text node or style entry.

| Input                                                        | Result                                                                                                                                                                     |
| ------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `title foo bar`, `set A`, `set B`, `union A,B`               | Title `foo bar`; subsets `A` (10), `B` (10), `A,B` (2.5)                                                                                                                   |
| `set A%% c`                                                  | Parse error on line 2: the `A` is taken by rule 8 with the `%%`                                                                                                            |
| `union A,B%% c`                                              | Parse error on line 4, the same rule                                                                                                                                       |
| `set A %% c`                                                 | Accepted: the comment is skipped                                                                                                                                           |
| `%% c` on its own line between two sets                      | Accepted; the newline before the comment is consumed by rule 8, which is harmless                                                                                          |
| `set A:-5`                                                   | Accepted, size -5. Render: the circles and labels are `NaN` (39 `NaN` values in the SVG), with no error                                                                    |
| `set A:0` and `union A,B`                                    | Accepted. Render: the union is dropped from the chart (section 6.13)                                                                                                       |
| `union A,B` before `set A`, `set B`                          | Error `unknown set identifier: A, B` (all unknown names are listed, `vennDB.ts:84`)                                                                                        |
| `union A`                                                    | Error `union requires multiple identifiers`                                                                                                                                |
| Indented `text "x"` directly after `venn-beta` (no set)      | Parse error: indent mode is off, so `text` is a TEXT token with no identifier list                                                                                         |
| `set A`, `style A fill:#f00`, then indented `text "x"`       | Parse error on line 4: `style` at the start of a line clears indent mode                                                                                                   |
| `set A`, then indented `text A "x"`                          | Parse error on line 3: in indent mode `text` is INDENT_TEXT, and the identifier list is not accepted there                                                                 |
| `set A`, `set B`, `union A,B`, then indented `text A,B "x"`  | Parse error (same reason)                                                                                                                                                  |
| The same `text A "x"` at column 0                            | Accepted, stored on `A`                                                                                                                                                    |
| `set A`, then indented `text "Item 1"`, `text B1["Lbl"]`     | Text nodes `Item 1` (no label) and `B1` with label `Lbl`, both on `A`                                                                                                      |
| Indented `text "Shared"` after `union A, B`                  | Stored on `A,B` (the union's sets, sorted)                                                                                                                                 |
| `text A,C "Kept"` with no union `A,C`                        | Stored in the DB; dropped by the renderer (section 6.5)                                                                                                                    |
| `set A`, `set A`, `set B`, `set C`                           | Two subsets named `A` (duplicates kept)                                                                                                                                    |
| `union A,A`                                                  | Subset `['A', 'A']`, size 2.5                                                                                                                                              |
| `title <b>x</b> & y  `                                       | `<b>x</b> &amp; y  ` (sanitised, trailing blanks kept)                                                                                                                     |
| `title T   `                                                 | `T   ` (trailing blanks kept)                                                                                                                                              |
| `set A["Set A label"]`, `set B[Bee]`, `union A,B["A and B"]` | Labels `Set A label`, `Bee`, `A and B`                                                                                                                                     |
| `set A[Set A]`                                               | Label `Set A`                                                                                                                                                              |
| `set A["<b>x</b> &amp; y"]`                                  | Label kept raw: `<b>x</b> &amp; y` (not sanitised)                                                                                                                         |
| `set A["<script>x</script>y"]`                               | Label kept raw, `<script>` included                                                                                                                                        |
| `text <i>q</i>` on `A`                                       | Text id kept raw                                                                                                                                                           |
| `style A fill:rgb(1,2,3),stroke:rgba(1,2,3,0.5)`             | `{fill: "rgb(1,2,3)", stroke: "rgba(1,2,3,0.5)"}`                                                                                                                          |
| `style A fill:#ff0000 stroke-width:2`                        | Parse error: style fields are comma-separated                                                                                                                              |
| `style A fill:red,stroke-width:2`                            | `{fill: "red", "stroke-width": "2"}`                                                                                                                                       |
| `style A color:"#ff0000"`                                    | `{color: "#ff0000"}`: the quotes are removed                                                                                                                               |
| `set "2"`, `set "10"`, `set "a"`, `union "2","a"`            | Ids `2`, `10`, `a`. The union is `['2', 'a']`                                                                                                                              |
| `SET A`, `UNION A,B`                                         | Accepted (case-insensitive lexer)                                                                                                                                          |
| `set my-set`, `union my-set,B`                               | Accepted; the union is `['B', 'my-set']`                                                                                                                                   |
| `set A[Lbl]:12`, `set A["Lbl"]:12`                           | Size 12, label `Lbl`                                                                                                                                                       |
| `set A:+3.5`, `set B:.5`                                     | Accepted by the pattern (not probed separately)                                                                                                                            |
| `union A,B:1e3`                                              | Parse error: `1e3` is not a number token                                                                                                                                   |
| `set` size omitted                                           | Size `10 / n²` for `n` identifiers: 10 for a set, 2.5 for a pair, 10/9 for a triple                                                                                        |
| `venn-beta` alone                                            | Accepted, no subsets. Render: error `Cannot read properties of undefined (reading 'set')` (from `greedyLayout`, `src/layout.js:396`, which the chart guard does not cover) |
| `venn-beta` with one `set A`                                 | Accepted. Render: one circle and its label                                                                                                                                 |
| Lexical `title A;B #c`                                       | Lexical error: `;` and `#` end the title pattern, so `;` is unrecognised                                                                                                   |
| `set <b>A</b>`                                               | Lexical error (`<` and `>` are not identifier characters)                                                                                                                  |

### 2.5 Quirks to reproduce

1. **Indent mode.** After any `set` or `union`, a line that begins with `text` after blanks is an indented text line, and it attaches to the most recent subset. The inline form `text A "x"` is accepted only at column 0. Any line that starts with a non-blank character clears the mode (rule 19), and a `set` or `union` line sets it again. So a `style` or `title` line, indented or not, turns it off, and an indented text after `style` is a parse error (probed). A port must do the same: clear the mode on every line that starts with a non-blank, and set it on `set` and `union`.
2. **A comment glued to a token eats the token.** `[^\}]\%\%[^\n]*` (line 8) takes the character before `%%`. `set A%% c` fails (probed), while `set A %% c` works.
3. **Statements need no separator.** `set A set B` is two statements (probed).
4. **Title pattern.** A title stops at `#` or `;` and is a lexical error if they follow (probed).
5. **Union arity and validation.** A union needs two identifiers, and every identifier must be a `set` declared before it (`knownSets` takes only single-set statements, `vennDB.ts:27-29`). A name that only appears in unions is unknown.
6. **Duplicates are kept.** A second `set A` is a second subset (probed). The renderer counts it in the layout (L4).
7. **Text is attached by its sets, sorted.** `text B,A "x"` stores `['A', 'B']` (`vennDB.ts:57`), the same key a union would use.
8. **Size defaults use the list length.** `10 / n²`, where `n` is the number of identifiers in the statement (`vennDB.ts:25`).
9. **Quotes are stripped by `normalizeText`** for ids, labels and styles (`vennDB.ts:42-48`). It trims first, so `["  x  "]` becomes `x`. Labels and text ids are not sanitised, but the title is (section 3.2).
10. **Negative sizes are accepted and give `NaN` output** (probed). A port may keep this or reject the input; decide before the parser (Q7).

## 3. DB

### 3.1 Calls in order

| Step | Call                                                       | Effect                                                                                                                                                                                                          |
| ---- | ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1    | `db.clear()` (`Diagram.ts:49`)                             | `customClear` (`vennDB.ts:104-112`): the common clear (title, accessibility texts), then the arrays, `knownSets`, `currentSets` and `indentMode`                                                                |
| 2    | `db.setDiagramTitle(front matter title)` (`Diagram.ts:53`) | Sanitised (`commonDb.ts:28-30`). A `title` statement later replaces it                                                                                                                                          |
| 3    | `parser.parse(text)` (`Diagram.ts:83`)                     | The actions in 2.2 call the DB: `setDiagramTitle` (grammar line 72), `addSubsetData` (73-80), `validateUnionIdentifiers` (77-80), `addTextData` (81-86, 91-95), `addStyleData` (87) and `setIndentMode` (73-80) |
| 4    | `draw` (`vennRenderer.ts:31`)                              | Reads `getDiagramTitle`, `getSubsetData`, `getTextData`, `getStyleData` and `getConfig`. It does not write to the DB                                                                                            |

### 3.2 What is stored, and sanitising

| Store                      | Written by                              | Sanitised?                         | Notes                                                                                                                                 |
| -------------------------- | --------------------------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Title                      | `setDiagramTitle` (`commonDb.ts:28-30`) | Yes, DOMPurify (`common.ts:71-84`) | Probed: `title AT&T` gives `AT&T`; `title <b>x</b> & y` gives `<b>x</b> &amp; y`, and the renderer writes that as literal text (6.13) |
| Subset `sets`              | `addSubsetData` (24)                    | No                                 | Sorted with the default `Array.prototype.sort` (UTF-16 code units)                                                                    |
| Subset `size`              | `addSubsetData` (25)                    | No                                 | `size ?? 10 / n²`. A `0` is kept (`??`)                                                                                               |
| Subset `label`             | `addSubsetData` (34)                    | No                                 | `normalizeText`, or `undefined` when empty. Probed: `<script>` is kept                                                                |
| Text `sets`, `id`, `label` | `addTextData` (54-61)                   | No                                 | `id` and `label` go through `normalizeText`                                                                                           |
| Style `targets`            | `addStyleData` (64)                     | No                                 | Sorted                                                                                                                                |
| Style `styles`             | `addStyleData` (66-68)                  | No                                 | `normalizeStyleValue` strips quotes and trims                                                                                         |

`normalizeText` (42-48) trims and removes one pair of surrounding double quotes. Empty labels become `undefined` (34, 59), which the renderer treats the same as no label (6.4).

### 3.3 Defaults and `getConfig`

`getConfig` (98-102) merges `DEFAULT_CONFIG.venn` with `commonGetConfig().venn`. The defaults are the schema's (`config.schema.yaml:2985-3007`). The renderer reads the top-level `getConfig()` for `themeVariables`, `look` and `handDrawnSeed` (`vennRenderer.ts:39`), and the venn section for the rest (6.9).

## 4. Canonical model proposal

Follows [canonical-model.md](../canonical-model.md): `"version": 1`, camelCase, strings always present (`""` for no label), numbers as JSON numbers.

```text
VennDiagram { version: 1, title: string, subsets: [Subset], textNodes: [TextNode], styles: [StyleEntry] }
Subset      { sets: [string], size: number, label: string }      // label "" when absent
TextNode    { sets: [string], id: string, label: string }         // label "" when absent
StyleEntry  { targets: [string], styles: [[key, value]] }         // value is the string as written, quotes removed
```

Left out: `knownSets`, `currentSets` and `indentMode` (parser state), and `accTitle` and `accDescr` (the renderer does not read them). The layout is a second model, proposed below; it is not part of this one.

Notes on the rules:

- `label: undefined` and `label: ""` are the same to the renderer: the library tests the label for truth (`src/diagram.js:103-105`), and the renderer tests the area's label for length (`vennRenderer.ts:310`). So `""` loses nothing. The port must reproduce the fallback: a single set with no label shows its id, and a pair shows nothing (`src/diagram.js:109-116`).
- The model keeps duplicates and the order of the subsets (both the parser and the renderer depend on that order: the set class index and the palette colour follow the DOM order).
- `size` is a JSON number. A `NaN` or `Infinity` cannot be written as JSON: the model needs a rule (Q7).
- Sets are sorted with JavaScript's default string order, which the port must reproduce.

Layout model (proposal for a second differential, only for inputs that avoid the randomised branch, L4): `VennLayout { circles: [{ set: string, x, y, radius }], areas: [{ sets, text: {x, y} | null, circles: [...] }] }`, with the layout's `padding` taken from the configuration. The layout is where the numbers are, so it is the input that a bit-exact comparison needs.

## 5. Specs

| File                   | Lines | Tests          | What they test                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ---------------------- | ----- | -------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `parser/venn.spec.ts`  | 255   | 17             | The grammar and the DB through `venn.parse` (the DB is `vennDB`, set as `yy`): `simple` (13), bracket labels (29, 43, 71), size suffix (57), text nodes with bracket labels (90), indented text nodes without a label (107), style statements for a set (124), a union (137) and a text node (150), several properties (162), rgb and rgba colours (178), indent mode with style (192), and the errors for `set` with two identifiers (220), `union` with one (227) and an unknown identifier (234), and quoted identifiers (242) |
| `vennRenderer.spec.ts` | 251   | 12             | `draw` with a stub DB (`createDiagram`, lines 6-25, whose configuration is `padding: 15`, so the spec never sees the default 8: lines 9 and 153): the title (28), text colours from style data (40), theme colours on circles (58), user overrides over the theme (72), contrast text on dark backgrounds (109), debug layout (149), and six tests of `ensurePairwiseSubsets` (167-246)                                                                                                                                           |
| `vennPalette.spec.ts`  | 117   | 18 at run time | The theme palette: every theme with a palette defines all eight `venn` slots (38), the themes without one define none (48), redux palettes are taken in order and are distinct (55, 62), and a renderer with no palette falls back to one colour (67-117). `describe.each` over the 11 themes gives 7 with a palette and 4 without, and 2 for the redux pair, so 18 in all                                                                                                                                                        |

The parser and renderer specs cover the parser and the renderer. Nothing tests the DB directly, and nothing tests the library's layout: `ensurePairwiseSubsets` is the only pure function in the renderer that a spec checks. The renderer specs also use a stub DB whose `padding` is 15, not the schema default (section 6.5).

## 6. Renderer

### 6.1 Pipeline

The generic steps are those of packet's section 6.1: `appendDivSvgG`, the style sheet as the first child of the `svg`, `draw`, `addA11yInfo` (`role` and `aria-roledescription="venn"`), and serialisation with DOMPurify (`ADD_TAGS: ['foreignobject']`). `draw` is synchronous. Probed: the output starts `<svg id=... width="100%" xmlns=... viewBox=... style="max-width: ...">`, then `<style>`, then an empty `<g></g>` (the generic group), then the venn output.

### 6.2 The layout calls

`draw` lays the sets out twice, on the same `renderSets` (`vennRenderer.ts:61`):

1. `dummyD3root.datum(renderSets).call(vennDiagram)` (line 95): the library's chart. It calls `venn()`, normalises, scales to the box, computes the set label centres, and draws the circles and the intersection paths into a detached `div`. Its padding is the library default, 15, because the renderer does not pass one (`vennRenderer.ts:91-94` set only `width` and `height`).
2. `venn.layout(renderSets, { width, height: svgHeight - titleHeight, padding: config?.padding ?? 15 })` (lines 102-106). It calls `venn()` again, and returns the circles and the text centre of every area. The padding is the configuration's, which is 8 by default, so the `?? 15` is never reached (probed, 6.5).

Each call draws random numbers when there are eight or more areas (L4). The chart's draws come first, then the layout's, within one render.

### 6.3 Element structure, in DOM order (probed)

Fixture 6 (`pr-check/6-…`, two sets and a union with text nodes), reduced to the element structure:

```text
svg#id  width="100%"  viewBox="0 0 800 450"  style="max-width: 800px;"
                      role="graphics-document document"  aria-roledescription="venn"
  style                                  one stylesheet, with the venn rules of 6.7 at the end
  g                                      empty, the generic group
  g transform="translate(0, 0)"          added at line 233; translate(0, titleHeight)
    g.venn-area.venn-circle.venn-set-0   data-venn-sets="A"
      path  d="M x y \nm -r 0 \na r r 0 1 0 2r 0 \na r r 0 1 0 -2r 0"
            style="fill-opacity: 0.1; fill: rgb(…); stroke: rgb(…); stroke-width: 2.5; stroke-opacity: 0.95;"
      text.label  class="label" text-anchor="middle" dy=".35em" x y  style="fill: rgb(…); font-size: 24px;"
        tspan  x y dy="0.35em"             the label, one tspan (6.6)
    g.venn-area.venn-circle.venn-set-1   data-venn-sets="B"        (same children)
    g.venn-area.venn-intersection        data-venn-sets="A_B"      no venn-set class
      path  d="M x y \nA r r 0 l s x y \nA …"  style="fill-opacity: 0; fill: transparent;"
      text.label  …  tspan  (empty text when the union has no label)
    g.venn-text-nodes                      appended to the dummy svg after the chart's nodes (`vennRenderer.ts:257`)
      g.venn-text-area  font-size="20px"   one per area with text nodes, in area order
        foreignObject.venn-text-node-fo  width height x y overflow="visible"
          span (xhtml)  class="venn-text-node"  style="display: flex; width: 100%; height: 100%; white-space: normal; align-items: center; justify-content: center; text-align: center; overflow-wrap: normal; word-break: normal;" [+ color]
            "Item 1"
```

Order rules, probed: the areas follow `renderSets`, which is the subsets in declaration order followed by the synthetic pairs of `ensurePairwiseSubsets` (6.4). The `venn-set-N` index counts the circles in DOM order (`vennRenderer.ts:121` and 128, 130), so the palette colour of a set is decided by the order of the circles, not by its name. A title (6.8) is a `text.venn-title` before the group.

Title, probed: `<text class="venn-title" font-size="16px" text-anchor="middle" dominant-baseline="middle" x="50%" y="16" style="fill: rgb(0, 0, 0);">` (`vennRenderer.ts:76-87`; `font-size` is `32 * scale`, `y` is `32 * scale`).

### 6.4 Sets and intersections (the chart)

The chart (`diagram.js`, read as `src/diagram.js:74-265`) does this for each area of its data:

- It removes zero-size singles and every area that contains one (`src/diagram.js:78-84`). This is the only place that does so. `layout()` does not (6.13).
- It normalises the solution (`normalize` true, orientation π/2, `src/diagram.js:92-94`) and scales it into `width` by `height - 2 * padding` (`:96`).
- It computes each area's label centre with `computeTextCentres` (`:97`), and writes it as `Math.floor(x)` and `Math.floor(y)` (`:228-229`).
- It draws each area's path: circles as `circlePath` (`:661-668`), intersections as the arcs of `intersectionAreaPath` (`:713-715`). The `round` argument of the chart is handed to `attr` and not to the path function (`:221`), so the paths are not rounded.
- Labels: `label(d)` returns the label of the datum, or for a single set its id (`:109-116`).

The renderer then styles them (`vennRenderer.ts:121-230`):

| Element            | Attribute or style | Value                                                                                                   | Source       |
| ------------------ | ------------------ | ------------------------------------------------------------------------------------------------------- | ------------ |
| Circle path        | `fill-opacity`     | `customStyle['fill-opacity'] ?? 0.1`                                                                    | 131, 157     |
| Circle path        | `fill`             | `customStyle.fill`, else the palette colour `themeColors[i % len]`, else `primaryColor`                 | 128-129, 156 |
| Circle path        | `stroke`           | `customStyle.stroke`, else the base colour                                                              | 132, 158     |
| Circle path        | `stroke-width`     | `customStyle['stroke-width'] \|\| 5 * scale`                                                            | 133, 159     |
| Circle path        | `stroke-opacity`   | `0.95`                                                                                                  | 160          |
| Circle label       | `fill`             | `customStyle.color`, else `lighten(base, 30)` on a dark background or `darken(base, 30)` on a light one | 163-164, 168 |
| Circle label       | `font-size`        | `48 * scale` px (24px at width 800)                                                                     | 167          |
| Intersection path  | `fill-opacity`     | `1` if `customStyle.fill`, else `0`                                                                     | 220-223      |
| Intersection path  | `fill`             | `customStyle.fill`, else `transparent`                                                                  | 225-229      |
| Intersection label | `fill`             | `customStyle.color`, else `themeVariables.vennSetTextColor`, else `primaryTextColor` or `textColor`     | 205, 215     |
| Intersection label | `font-size`        | `48 * scale` px                                                                                         | 210          |

Two rules follow from the code and the probes. A style key is the sorted set ids joined by `|` (`buildStyleByKey`, 17-29), so `style A,B fill:…` styles the pair `A_B` (key `A|B`) and none of the single sets. Later statements for the same key override the earlier ones, property by property.

`scale` is `width / 1600` (`vennRenderer.ts:67`): 0.5 at 800, so `stroke-width` is 2.5 and the set label 24px. The title is `32 * scale` px and its band is `48 * scale` px.

### 6.5 Text nodes (layout padding, `foreignObject`)

`renderTextNodes` (`vennRenderer.ts:247-374`) places each text node in a grid inside its area:

1. The nodes are grouped by their sets (260-269). A group whose area is not in `layoutByKey` is skipped (272-276, probed: the `text A,C "Kept"` of 2.4 is dropped).
2. The centre is the layout's `area.text`. The inner radius is the smallest `r - distance(centre, circle centre)` over the area's circles (278-283). A zero inner radius becomes 0.6 times the smallest radius (285-287).
3. `innerWidth = max(80 * scale, innerRadius * 2 * 0.95)`, and the same for the height (308-309). The grid moves down by `labelOffset` (311-314): `min(32 * scale, innerRadius * 0.25)` when the area has a label, and 0 otherwise, plus `30 * scale` when the area has two nodes or fewer.
4. The grid has `cols = ceil(sqrt(n))` columns and `rows = ceil(n / cols)` rows (315-318). Each node sits at the centre of its cell (324-325), and its box is 90% of the cell (341-342). The box is a `foreignObject` with `overflow="visible"` (345-352).
5. Inside the box, an `xhtml:span` holds the text (355-367), with the `display: flex` styles listed in 6.3, and a `color` if a style sets one (354, 369-371).

The layout centre (step 2) and the circles it is measured against come from the `venn.layout` call, which uses `config.padding` (8 by default). The circles the reader sees come from the chart call, which uses 15. The result is a visible offset. Probed on fixture 6 (`probe-render.mjs`, `layout` probe):

| Quantity                                  | Padding 15 (drawn circles, set labels) | Padding 8 (text nodes)                                  |
| ----------------------------------------- | -------------------------------------- | ------------------------------------------------------- |
| Circle A: x, radius                       | 266.71, 210                            | 262.27, 217                                             |
| Label or text centre of the A-only region | 190 (set label `A`)                    | 183.00 (grid of the two `Item` nodes, centred on 183.0) |

The text boxes are 117.76 wide, placed from the padding-8 geometry. The port must choose one of: reproduce the offset (the harness compares against the reference, so this is the default), or correct both to one padding. Q2 asks the coordinator.

`debug layout` (`useDebugLayout: true`, probed) adds a `circle.venn-text-debug-circle` at the centre with radius `innerRadius`, `stroke="purple"`, `stroke-width` `1.5 * scale`, `stroke-dasharray` `6*scale 4*scale`, and a `rect.venn-text-debug-cell` per cell in `teal` (`:294-339`). Probed values: `stroke-width="0.75"`, `stroke-dasharray="3 2"` at width 800.

### 6.6 Measurements

The renderer measures nothing. Probed: `getComputedTextLength()` returns 0 on a text element in a detached `div`, and 297.7 for the same text in the document. The renderer draws into a detached `div` (`vennRenderer.ts:90`), and `wrapText` (`src/diagram.js:384-436`) calls `getComputedTextLength` (418) only inside the `joined.length > minChars` test. With 0 the test never passes, so every set label is one `tspan` holding the whole label. The port must produce a single `tspan` for each label (the same as the reference), and must not try to wrap.

Two more effects of `wrapText` (probed): `split(/\s+/)` collapses every run of whitespace in a set label to one space (`set A["a    b  c"]` gives `a b c`), and an empty label gives an empty `tspan`. Text nodes are not wrapped, so their runs of spaces stay (`x    y` in the span).

The `tspan` attributes are `x` and `y` from the text, and `dy` of `0.35em` (`initial = 0.35 - 0 * 1.1 / 2`, `src/diagram.js:427-433`).

### 6.7 Styles and colours

`styles.ts` (`getStyles`, 8-31) returns four rules, compiled by stylis under the SVG's id (probed, the `<style>` of fixture 6):

```css
#id .venn-title {
  font-size: 32px;
  fill: <vennTitleTextColor>;
  font-family: <fontFamily>;
}
#id .venn-circle text {
  font-size: 48px;
  font-family: <fontFamily>;
}
#id .venn-intersection text {
  font-size: 48px;
  fill: <vennSetTextColor>;
  font-family: <fontFamily>;
}
#id .venn-text-node {
  font-family: <fontFamily>;
  color: <vennSetTextColor>;
}
```

The inline styles of 6.4 override the 48px in the first two rules (probed: the labels are 24px inline).

Colour values reach the SVG as the browser writes them. The renderer passes hex, named and `hsl(...)` strings to d3's `style`, and Chromium serialises them as `rgb(r, g, b)` (probed: `#E879F9` appears as `rgb(232, 121, 249)`, and a `hsl` from `darken` as `rgb(181, 9, 208)`). `darken` and `lighten` (khroma 2.1.0, `packages/mermaid/node_modules/khroma`) return `hsl(...)` strings with ten decimal places (`khroma-probe.mjs`). The port needs the same HSL arithmetic and the same rounding to 8-bit channels. `crates/ferrigura-model/src/theme/khroma.rs` has `darken`, `lighten` and `is_dark` (Rust port, not checked here).

An invalid colour makes the render fail: `style A fill:notacolour` gives `Unsupported color format: "notacolour"` from khroma (probed). `isDark` runs on `background` (`:120`).

### 6.8 Root sizing

`svg.attr('viewBox', '0 0 W H')` with `H` the full height, title included (`:73-74`). Then `configureSvgSize(svg, svgHeight, svgWidth, useMaxWidth)` (`:240`), which with `useMaxWidth` true gives `width="100%"` and `style="max-width: Wpx;"`, and with it false gives `width="W"` and `height="H"` (`setupGraphViewbox.js:23-47`). Probed: `venn: {useMaxWidth: false}` gives `width="800" height="450"`. The top-level `useMaxWidth: false` is ignored, because the renderer reads the venn section only (6.9).

Size: `W` is `config.width` (800), `H` is `config.height` (450), and the title takes `48 * scale` off the library's height, not off the viewBox. The `titleHeight` is 0 without a title.

### 6.9 Configuration

Every key the renderer or DB reads, with its default and where it is read. The table gives the layer each one comes from (decision 0009's typed views take them as listed).

| Key                           | Layer           | Default                        | Read by                                                                                                | Effect                                                                                                                                                                  |
| ----------------------------- | --------------- | ------------------------------ | ------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `venn.width`                  | diagram section | 800                            | `vennRenderer.ts:64`                                                                                   | Canvas width, and `scale` through `width / 1600` (67)                                                                                                                   |
| `venn.height`                 | diagram section | 450                            | `vennRenderer.ts:65`                                                                                   | Canvas height                                                                                                                                                           |
| `venn.padding`                | diagram section | 8                              | `vennRenderer.ts:105` (`?? 15` never reached)                                                          | Padding of the text layout (6.5). The drawn circles keep the library's 15                                                                                               |
| `venn.useMaxWidth`            | diagram section | `true` (`BaseDiagramConfig`)   | `vennRenderer.ts:240` (`?? true`)                                                                      | Root sizing (6.8)                                                                                                                                                       |
| `venn.useDebugLayout`         | diagram section | `false`                        | `vennRenderer.ts:255` (`?? false`)                                                                     | Debug shapes in text areas (6.5)                                                                                                                                        |
| `venn.theme`                  | diagram section | `redux-color`                  | Not read by the renderer. Resolved by `config.ts:79-107` into the top-level `theme` for a venn diagram | Theme for a venn diagram, unless the user sets the top level (the probe shows `redux-color` without a site setting, and `default` with `initialize({theme:'default'})`) |
| `venn.look`                   | diagram section | `neo`                          | Not read by the renderer. Resolved into the top-level `look` (`config.ts:79-107`)                      | Only `handDrawn` changes the output. `classic` and `neo` give identical SVGs (probed, after id normalisation)                                                           |
| `look` (top level)            | global          | from the venn section          | `vennRenderer.ts:39-40` (`isHandDrawn`)                                                                | Hand-drawn look only (6.14)                                                                                                                                             |
| `handDrawnSeed` (top level)   | global          | 0 (`config.schema.yaml:71-75`) | `vennRenderer.ts:39`, `:141`, `:186`                                                                   | Seed of rough.js. 0 means random (6.14)                                                                                                                                 |
| `securityLevel` (top level)   | global          | `strict`                       | `rendering-util/selectSvgElement.ts:12`                                                                | Sandbox root lookup only                                                                                                                                                |
| `fontFamily` (top level)      | global          | the theme's font               | Through `themeVariables.fontFamily` (`mermaidAPI.ts:699-703`), read by `styles.ts:13,18,24,28`         | Font in the style sheet                                                                                                                                                 |
| `theme` (top level)           | global          | from the venn section          | `vennRenderer.ts:39` (`themeVariables`)                                                                | Theme variables (6.10)                                                                                                                                                  |
| `dompurifyConfig` (top level) | global          | none                           | `common.ts:75` (DB title sanitising)                                                                   | DOMPurify options for the title                                                                                                                                         |

The renderer and DB read no other key. The renderer reads `width`, `height`, `padding`, `useMaxWidth` and `useDebugLayout` only from the venn section (`vennRenderer.ts:64-65`, `:105`, `:240`, `:255`), so a top-level value of those names does not reach it (from the code; only `useMaxWidth` was probed).

Top-level `useMaxWidth` is not read for venn (probed: `useMaxWidth: false` at the top level leaves `width="100%"`). Venn's section decides, and its default is the top-level default of 6.8.

### 6.10 Theme variables

Every `themeVariables` key that the renderer or the styles read, with file and line.

| Token                | Read at                                              | Use                                                        | Default in `redux-color` (probed)                                                      |
| -------------------- | ---------------------------------------------------- | ---------------------------------------------------------- | -------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- | --------- |
| `venn1` to `venn8`   | `vennRenderer.ts:41-50`, filtered for truth (50)     | The palette: the k-th circle takes `venn[k mod len]`       | `#E879F9`, `#2DD4BF`, `#FB923C`, `#22D3EE`, `#4ADE80`, `#A78BFA`, `#F87171`, `#FACC15` |
| `primaryColor`       | `vennRenderer.ts:129`                                | Circle colour when there is no palette and no style        | `#cccccc`                                                                              |
| `primaryTextColor`   | `vennRenderer.ts:70`                                 | The default text colour (`defaultTextColor`, 205, 215)     | `#28253D`                                                                              |
| `textColor`          | `vennRenderer.ts:70` (fallback)                      | The same, when `primaryTextColor` is unset                 | `#28253D`                                                                              |
| `titleColor`         | `vennRenderer.ts:86` (fallback)                      | Title fill when `vennTitleTextColor` is unset              | `#000000`                                                                              |
| `vennTitleTextColor` | `vennRenderer.ts:86`; `styles.ts:12`                 | Title fill: the inline style (86) and the style sheet (12) | `#000000`                                                                              |
| `vennSetTextColor`   | `vennRenderer.ts:205`, `:215`; `styles.ts:23`, `:29` | Intersection labels and text nodes                         | `#28253D`                                                                              |
| `background`         | `vennRenderer.ts:120` (`                             |                                                            | '#f4f4f4'`)                                                                            | Whether the theme is dark (`isDark`), which picks `lighten` or `darken` for circle labels | `#ffffff` |
| `fontFamily`         | `styles.ts:13`, `:18`, `:24`, `:28`                  | Font in the style sheet                                    | `"Recursive Variable", arial, sans-serif` (the harness overrides it)                   |

Not read: `theme` and `look` (passed to `styles` at `mermaidAPI.ts:330`, which the venn styles ignore), `textColor` in `styles.ts:5` (declared, never used), and `borderColorArray`. Themes without `venn1` to `venn8` (`neo`, `neo-dark`, `redux`, `redux-dark`) give flat circles in `primaryColor` (`vennPalette.spec.ts`, probed: `neo` has `primaryColor` `#cccccc`).

For decision 0007, the eight palette slots are the venn-specific tokens. `redux-color` and `redux-dark-color` set them from `borderColorArray` (`theme-redux-color.js:332-333`), and the default theme sets them from `adjust` (`theme-default.js:283-290`), giving `hsl(...)` strings.

### 6.11 Randomness and the clock

- **Renderer.** None. `vennRenderer.ts` never reads `Math.random` or the clock.
- **Library.** `Math.random` in one place, the constrained layout's starting points (L4). The harness replaces it with a seeded generator for both sides (`tools/port/geometry/common.ts:564-570`, seeded with 1 by `diagram-geometry.ts:368`).
- **rough.js.** The hand-drawn look only: `Math.random` when the seed is 0 (roughjs 4.6.6, `bin/math.js:9-13`).

### 6.12 DOM dependencies a server renderer must replace

| Dependency                                                                 | Where                                | What the port does                                               |
| -------------------------------------------------------------------------- | ------------------------------------ | ---------------------------------------------------------------- |
| A detached `div` and d3 selections (`vennRenderer.ts:90-95`, `diagram.js`) | Renderer, chart                      | Not needed: the port writes the elements in order (6.3)          |
| `getComputedTextLength` in `wrapText`                                      | `src/diagram.js:418`                 | Returns 0 in the reference (probed), so no wrapping: one `tspan` |
| `text.textContent`, `createElementNS`, `ownerDocument`                     | `src/diagram.js:398-407`             | Not needed beyond one `tspan`                                    |
| `foreignObject` and `xhtml:span`                                           | `vennRenderer.ts:345-367`            | Q1                                                               |
| DOMPurify of the serialised SVG                                            | `mermaidAPI.ts:645-649` (packet 6.1) | The port's own escaping (the generic step)                       |
| `console.log` (`src/layout.js:769`, `src/diagram.js:606`)                  | Library                              | Not output                                                       |

### 6.13 Quirks of the renderer (probed)

1. **Two padding values.** The circles use 15 and the text nodes use the configuration's 8 (6.5). Probed on fixture 6.
2. **Zero-size singles.** `set A:0` removes `A` and every union with `A` from the chart, but the layout still has `A`, so a text node under `A` is drawn, at the position of a zero-radius circle. Probed: with `set A:0`, `set B`, `union A,B` and `text "inA"` under `A`, the text's `foreignObject` has its centre at x 621, outside the drawn circle of B, which spans x 190 to 610.
3. **Disjoint labelled union.** A union whose circles do not overlap gets its label at `(0, -1000)`, and its path is `M 0 0` (probed: `union A,B["AB"]:0`). Source: `src/diagram.js:532-534`. The label is off the canvas.
4. **Empty diagram.** `venn-beta` alone renders with the error of 2.4.
5. **Negative sizes.** `NaN` throughout the SVG, no error (2.4).
6. **Invalid colour.** Render error from khroma (6.7).
7. **Title markup.** The title is sanitised in the DB and written with `text(...)`, so `&amp;` is shown as the characters `&amp;` (probed).
8. **Numeric set ids change the layout.** The library orders its object keys with `Object.keys`, and integer-like keys come first, in ascending order. Sets `"2"`, `"10"`, `"1"` give circles at different places from sets `"x2"`, `"x10"`, `"x1"` in the same declaration order (probed; L6).
9. **Duplicated statements change the branch.** Two `set A` lines with `B` and `C` give 10 areas, which takes the randomised branch (L4, probed).
10. **Text nodes ignore `htmlLabels`.** With `htmlLabels: false` (the harness), the text nodes are still `foreignObject`, as the fixtures show. Decision 0004 says labels are SVG text, so the venn text nodes differ from it (Q1).
11. **The top-level `useMaxWidth`, `padding` and `width` are not read** (6.9).
12. **Set order decides the colours.** The palette index follows the order of the circles in the DOM, and the zero-size removal shifts it (6.3, order rules).

### 6.14 Hand-drawn look (read only; out of the harness's scope)

With `look: handDrawn`, the renderer swaps each circle path for a rough.js circle (`vennRenderer.ts:135-152`: roughness 0.7, `seed: handDrawnSeed`, hachure fill at angle `-41 + 60 * i` with gap 8, weight 2), and each intersection for a rough path with cross-hatching (172-206), only when the set has a `fill`. The three handDrawn fixtures are excluded by `EXCLUDED_LOOK` (`common.ts`). The port's scope needs a named decision (Q6). Rough.js is a dependency (`roughjs` 4.6.6 in `packages/mermaid/node_modules`).

## 7. Fixtures

21 `.mmd` files under `e2e/diagrams/venn/` (`pr-check` 10, `nightly-check` 9 at the top and 2 in `handdrawn/`). Each has a front matter `description`, and some a `config:` block.

| Fixture                                                      | Features                                                                      | Status                            |
| ------------------------------------------------------------ | ----------------------------------------------------------------------------- | --------------------------------- |
| `pr-check/1-…simple-two-set`                                 | Two sets and a union                                                          | Compared                          |
| `pr-check/2-…three-set`                                      | Three sets, three pairs, the triple                                           | Compared (7 areas, greedy branch) |
| `pr-check/3-…with-a-title`                                   | Three sets, all unions, title `Skills Overlap`                                | Compared                          |
| `pr-check/6-…text-nodes`                                     | Two sets with text nodes (`Item 1`, `Item 2`, `Item 3`, `Shared`)             | Compared. Exercises 6.5           |
| `pr-check/7-…custom-colors-via-style`                        | `style` on `A`, `B` and `A,B`                                                 | Compared                          |
| `pr-check/8-…string-identifiers`                             | Quoted set ids, title, a union with a missing pair (synthetic, 0-size)        | Compared                          |
| `pr-check/11-…custom-text-colors-via-style`                  | Text nodes `A1`, `B1`, `AB1` with `style` colours                             | Compared. Exercises 6.5           |
| `pr-check/15-…handdrawn-look-and-title`                      | Three sets, `look: handDrawn`, `handDrawnSeed: 1`, `courier`                  | Excluded (`look handDrawn`)       |
| `pr-check/16-…3-set-union-without-explicit-pairwise-subsets` | Triple union only; the three pairs are synthetic                              | Compared (7 areas)                |
| `pr-check/17-…partial-pairwise-subsets`                      | Triple and two pairs; the third pair is synthetic                             | Compared (7 areas)                |
| `nightly-check/4-…custom-set-sizes`                          | Sizes 20/15/10, pairs 5/3/2, triple 1                                         | Compared                          |
| `nightly-check/5-…with-labels`                               | Bracket labels on two sets and the union, with a title                        | Compared                          |
| `nightly-check/9-…dark-theme`                                | `config.theme: dark` (the harness maps it to `redux-dark-color`), three pairs | Compared                          |
| `nightly-check/10-…many-text-nodes`                          | Title, three and two text nodes on the sets, one on the union                 | Compared. Exercises 6.5           |
| `nightly-check/12-…asymmetric-sizes`                         | Sizes 30, 10 and 5                                                            | Compared                          |
| `nightly-check/13-…labels-text-nodes-and-styles`             | Bracket labels, text nodes on every region, styles on each set                | Compared. Exercises 6.5           |
| `nightly-check/explicit-set-style-over-redux-palette`        | `config.theme: redux-color`, `style A fill:#00ff00`                           | Compared                          |
| `nightly-check/three-sets-redux-color-theme`                 | Triple union with label `Innovation`                                          | Compared                          |
| `nightly-check/three-sets-redux-dark-color-theme`            | The same, `redux-dark-color`                                                  | Compared                          |
| `nightly-check/handdrawn/14-…`                               | Two sets, `look: handDrawn`, `courier`                                        | Excluded (`look handDrawn`)       |
| `nightly-check/handdrawn/18-…`                               | Hand-drawn, text nodes and styles                                             | Excluded (`look handDrawn`)       |

The 18 compared fixtures render in Chromium, and the reference output is the same on every render, in the same page and in a second page (probed: `probe-render.mjs`, all 18). Their layouts take the greedy branch, because none has eight areas: three sets give at most seven (3 singles, 3 pairs, 1 triple), and the fixtures have no duplicated statement (L4). So no compared fixture draws random numbers.

Nothing in the fixtures uses a character outside the bundled faces: `uncoveredScriptReason` is null for all 21 (probed: `fixture-checks.mts`), and `exclusionReason` excludes only the three handDrawn fixtures.

What the geometry harness must handle: the text nodes are `foreignObject` (`x`, `y`, `width`, `height` compare as boxes, their HTML content does not, Q1); the set labels are SVG `text` with one `tspan`; the circles are `path` with `d`; the intersections are `path` with arcs. No fixture repeats a `set` or `union` statement (probed: `fixture-checks.mts`), so the DOM ids of one diagram are unique.

## 8. Risks, effort, open questions

### 8.1 Risks

| Risk                                                        | Level                              | Why                                                                                                                                            |
| ----------------------------------------------------------- | ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| The circle layout (the library)                             | High                               | Greedy placement, Nelder-Mead and conjugate gradient in floating point, with `Math`, sorts and key order all mattering (Part B)                |
| The padding mismatch                                        | Medium                             | A visible offset on the four fixtures with text nodes (6.5). Simple to reproduce once seen                                                     |
| The randomised branch                                       | Medium for users, low for fixtures | Four or more sets, or duplicated statements, take it. The result can vary between renders (L4, probed), so a comparison needs the harness seed |
| Indent mode and comments in the lexer                       | Medium                             | Several quirks, each probed (2.4, 2.5)                                                                                                         |
| `foreignObject` text nodes against decision 0004            | Medium                             | A decision, not a port difficulty (Q1)                                                                                                         |
| The DOM order and the palette index                         | Low once written down              | Deterministic (6.3)                                                                                                                            |
| Colour arithmetic (khroma) and browser colour serialisation | Medium                             | The text colours come from `hsl` strings with ten decimals (6.7). `khroma.rs` exists                                                           |
| Hand-drawn look                                             | Out of scope unless decided        | Random when the seed is 0 (6.14)                                                                                                               |

### 8.2 Effort

Rough estimate in Rust lines, not a measurement:

| Part                                                                                                                                                                         | Lines        |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ |
| Scanner with the `bol` state and the indent mode (2.1)                                                                                                                       | 200 to 260   |
| Parser: the statements, the arity checks, the errors (2.2)                                                                                                                   | 150 to 200   |
| DB: `addSubsetData`, text, styles, `validateUnionIdentifiers`, title sanitising                                                                                              | 150 to 200   |
| Library, Part B: `venn`, greedy, constrained MDS with the seeded draws, Nelder-Mead, conjugate gradient with Wolfe search, normalise, scale, text centres, intersection arcs | 900 to 1,100 |
| Renderer: chart paths and labels, text nodes (grid, debug layout), styles, the palette, colour strings                                                                       | 450 to 550   |
| Tests: 36 specs, the probe inputs of sections 2, 6 and L4                                                                                                                    | 350 to 450   |

The total is 2,200 to 2,700 lines. The library is the largest part and the hardest to match bit for bit.

### 8.3 Open questions for the coordinator

- **Q1. Text nodes are HTML.** The reference draws each text node in a `foreignObject` with an `xhtml:span`, and does so with `htmlLabels: false` as well (6.13, item 10). Decision 0004 says labels are SVG text. Options: (a) draw SVG text inside the box, and say the venn text nodes differ from the reference in their markup; (b) keep `foreignObject`, a second exception to 0004. The geometry harness compares the boxes either way. The reference's line breaks come from the browser's HTML layout, which ferrigura does not run: option (a) needs ferrigura's own line breaking, and option (b) leaves the breaks to the viewer. Decide before the renderer.
- **Q2. Padding 8 for the text, 15 for the circles.** Reproduce the offset (the default: the harness compares with the reference, and the four fixtures with text nodes would otherwise move), or make both use one value and record the difference. The second changes the reference's output for every venn with text nodes.
- **Q3. The randomised branch.** Four or more sets take it. Options: (a) port the harness's generator exactly, with the same draw order (the chart first, then the layout), so that the reference under the harness matches for every input; (b) use a fixed seed in the port and say so; (c) remove the randomness, which changes outputs for inputs where the constrained layout wins (44 of 120 random configurations in the probe). None of the 21 fixtures reaches the branch, so the harness cannot check it. Recommend (a), plus one or two fixtures with four sets, so the branch is covered.
- **Q4. Numeric set ids.** They change the layout (6.13, item 8). Reproduce JavaScript's key order (integer-like ids first, ascending), or treat it as a reference quirk to record. Recommend reproduce: it costs one helper.
- **Q5. Duplicated statements.** They change the subset list and the branch (2.5, item 6). Reproduce as written.
- **Q6. Hand-drawn look.** Out of the harness's scope. Name it as excluded with a reason, or port rough.js (`rough.rs` exists in `unified/shapes/` for flowchart, not checked for this).
- **Q7. Invalid numbers and colours.** Negative sizes give `NaN` output, and an invalid colour is a render error (2.4, 6.7). Reproduce, or reject the input with an error in the parser. The model cannot store `NaN` as JSON.
- **Q8. Empty diagram.** A bare `venn-beta` renders as an error. Reproduce the error, or render an empty `svg`.
- **Q9. Venn's own `theme` and `look` defaults.** Decision 0009's layering has to resolve diagram-scoped appearance defaults (`config.ts:79-107`): `venn.theme` is `redux-color` and `venn.look` is `neo`, and these reach the top level only for a venn diagram. Confirm the port's config layer does the same.
- **Q10. Top-level `useMaxWidth`, `padding` and `width`.** The renderer reads only the venn section (6.9). Reproduce, which is simplest, or read the top level as well.
- **Q11. Library port and licence.** `@upsetjs/venn.js` is MIT (its `package.json:20`). Its `fmin` dependency is a patched 0.0.4 (`package.json:82`, with a yarn patch that is not in the repository). Keep the copyright notices, as `crates/ferrigura-jsmath/NOTICE.md` does for its ports.
- **Q12. Layout as its own crate or module.** The library is about 900 to 1,100 lines of Rust. Decide whether it is a `ferrigura-venn` crate (like `ferrigura-cose` and `ferrigura-elk`) or a module of the renderer.

---

# Part B: the layout engine, `@upsetjs/venn.js` 2.0.0

## L0. Summary

`venn()` places circles so that the pairwise and higher overlaps match the given sizes. Steps: add the missing pairwise areas as size 0; a greedy placement of each set against the already placed ones, by testing a few candidate points; if there are at least eight areas, a constrained multidimensional-scaling start from ten random restarts, kept if it beats the greedy result by 1e-8; a Nelder-Mead minimisation of the squared overlap error; then `normalizeSolution` (orientation, clusters arranged in a grid) and `scaleSolution` (into the box, with padding); and a separate search for each area's label centre, which maximises the distance to the area's boundary, with Nelder-Mead again.

Mermaid uses the chart (`VennDiagram`, with `width`, `height`) for the drawn circles and `layout()` for the text centres. Both call `venn()`.

## L1. Sources

Version: `@upsetjs/venn.js` 2.0.0, MIT (`"license": "MIT"`), author Ben Frederickson and contributor Samuel Gratzl (the package's `package.json`). The lockfile pins it at `pnpm-lock.yaml:1952`. Mermaid imports it as `import * as venn from '@upsetjs/venn.js'` (`vennRenderer.ts:11`), which the package's `exports.import` maps to `build/venn.esm.js`.

| File                         | Lines                          | What                                                                                                                                                                                                                                                                  |
| ---------------------------- | ------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/index.js`               | 1-23                           | The exports Mermaid uses: `VennDiagram`, `layout`, `venn`, `lossFunction`, `normalizeSolution`, `scaleSolution`, `bestInitialLayout`, `greedyLayout`, `disjointCluster`, `distanceFromIntersectArea`                                                                  |
| `src/layout.js`              | 11-60                          | `venn()`                                                                                                                                                                                                                                                              |
|                              | 62-79                          | `SMALL`, `distanceFromIntersectArea`                                                                                                                                                                                                                                  |
|                              | 89-149                         | `addMissingAreas`                                                                                                                                                                                                                                                     |
|                              | 158-194                        | `getDistanceMatrices`                                                                                                                                                                                                                                                 |
|                              | 197-230                        | `constrainedMDSGradient`                                                                                                                                                                                                                                              |
|                              | 236-254                        | `bestInitialLayout`                                                                                                                                                                                                                                                   |
|                              | 261-313                        | `constrainedMDSLayout`                                                                                                                                                                                                                                                |
|                              | 322-457                        | `greedyLayout`                                                                                                                                                                                                                                                        |
|                              | 467-489                        | `lossFunction` (483-486: the squared error)                                                                                                                                                                                                                           |
|                              | 491-514                        | `logRatioLossFunction` (not reachable from Mermaid, which never passes `loss: 'logRatio'`)                                                                                                                                                                            |
|                              | 522-584                        | `orientateCircles`                                                                                                                                                                                                                                                    |
|                              | 591-639                        | `disjointCluster`                                                                                                                                                                                                                                                     |
|                              | 645-652                        | `getBoundingBox`                                                                                                                                                                                                                                                      |
|                              | 661-746                        | `normalizeSolution`                                                                                                                                                                                                                                                   |
|                              | 760-799                        | `scaleSolution`                                                                                                                                                                                                                                                       |
|                              | 805-820                        | `toObjectNotation`, `fromObjectNotation`                                                                                                                                                                                                                              |
| `src/circleintersection.js`  | 1-266                          | `intersectionArea` (18-143), `containedInCircles`, `getIntersectionPoints`, `circleArea` (181), `distance` (191), `circleOverlap` (204), `circleCircleIntersection` (229), `getCenter` (257). Line-identical in the build                                             |
| `src/diagram.js`             | 20-375                         | `VennDiagram` (the chart); defaults at 21-35; the data filter 78-84; the layout call 90; normalise 92-94; scale 96; text centres 97; labels 102-116; the path tween 142-161 (unused without a previous render); the join and the attributes 164-229; the wrap 231-243 |
|                              | 384-436                        | `wrapText`                                                                                                                                                                                                                                                            |
|                              | 445-462                        | `circleMargin`                                                                                                                                                                                                                                                        |
|                              | 473-547                        | `computeTextCentre`                                                                                                                                                                                                                                                   |
|                              | 551-573                        | `getOverlappingCircles`                                                                                                                                                                                                                                               |
|                              | 575-610                        | `computeTextCentres`                                                                                                                                                                                                                                                  |
|                              | 661-668                        | `circlePath`                                                                                                                                                                                                                                                          |
|                              | 689-715                        | `arcsToPath`, `intersectionAreaPath`                                                                                                                                                                                                                                  |
|                              | 717-783                        | `layout`                                                                                                                                                                                                                                                              |
| `fmin`, inlined in the build | build 262-622 (no source here) | The `fmin` functions used: `bisect`, `zeros`, `dot`, `norm2`, `scale`, `weightedSum`, `nelderMead`, `wolfeLineSearch`, `conjugateGradient`. The source imports them from `fmin` (`src/layout.js:1`), which is not installed here                                      |
| `build/venn.esm.js`          | 1-2214                         | What runs. Same code, `fmin` inlined at 262-622. Line offsets from `src/`: `layout.js` +612, `diagram.js` +1429, `circleintersection.js` 0                                                                                                                            |

`fmin` is a devDependency at 0.0.4, with a yarn patch (`package.json:82`), and the patch is not in the repository. The build's copy is what runs. The `fmin` version is **unverified**: the package pins 0.0.4 with a patch, but the build's copy was read, not compared with 0.0.4.

The build and the source were read side by side for the functions the renderer reaches. They were not diffed, and the probes ran the build. **Unverified**: that the build's code equals the source in every branch.

## L2. Input

What Mermaid builds (`renderSets`, `vennRenderer.ts:61`): a list of `{ sets: string[] (sorted), size: number, label?: string }`. Singles come first in declaration order, then the unions in declaration order, then the synthetic pairs that `ensurePairwiseSubsets` adds for each missing pair of a union of three or more sets (`vennRenderer.ts:393-435`).

The two calls:

| Call                                                                                                    | Options Mermaid passes                      | Library defaults that apply                                                                                                                                                   |
| ------------------------------------------------------------------------------------------------------- | ------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `VennDiagram().width(W).height(H - titleH)` then `.call` on `renderSets` (`vennRenderer.ts:91-95`)      | `width`, `height`                           | `padding` 15, `orientation` π/2, `normalize` true, `scaleToFit` null, `wrap` true, `styled` true, `distinct` false, `symmetricalTextCentre` false, `lossFunction` the default |
| `venn.layout(renderSets, { width, height, padding: config.padding ?? 15 })` (`vennRenderer.ts:102-106`) | `width`, `height`, `padding` (8 by default) | `normalize` true, `orientation` π/2, `scaleToFit` false, `symmetricalTextCentre` false, `round` 2 (affects only the `path` strings, which Mermaid does not read)              |

The chart removes zero-size singles and every area that contains one (`src/diagram.js:78-84`). `layout()` does not. So the text of a zero-size set is laid out and drawn (6.13, item 2).

## L3. Algorithm

Constants are as written in the source. Each line is `src/`.

### L3.1 `venn(sets, parameters)` (`layout.js:11-60`)

1. `parameters.maxIterations = parameters.maxIterations || 500` (`:12`). This **mutates the caller's object**. Mermaid passes a fresh object each time, so this is harmless to it.
2. `addMissingAreas` (`:18`), below.
3. `initialLayout = parameters.initialLayout || bestInitialLayout` (`:14`), then `circles = initialLayout(areas, parameters)` (`:21`).
4. The circles' centres go into a flat vector `[x0, y0, x1, y1, …]` in the order of `Object.keys(circles)` (`:24-30`). The radius stays fixed (`:41`).
5. `nelderMead(loss, initial, parameters)` (`:33-49`), and the result's `x` is written back (`:52-57`).

### L3.2 `addMissingAreas` (`:89-149`)

Adds `{ sets: [a, b], size: 0 }` for every pair of single sets that has no 2-set area. The single-set ids are sorted with `<` on strings (`:137`, which is code-unit order) and the pairs are taken with `i < j`. With `distinct` (`:97-119`, not used by Mermaid) the sizes are recomputed.

### L3.3 `greedyLayout` (`:322-457`)

1. Every single set starts at `x: 1e10, y: 1e10` (`:333-334`).
2. `setOverlaps[set]` is the list of its 2-set areas, each with `weight` 1, or 0 when the overlap is at least the smaller set (`:347-359`, the completely overlapped case, `SMALL` 1e-10).
3. `mostOverlapped`: for each set, `sum(size * weight)` over its overlaps (`:363-370`), then a stable sort, descending (`:376`).
4. The first set goes to `(0, 0)` (`:396`).
5. For each later set, in the sorted order (`:401-454`): the already placed overlaps are sorted by size, descending (`:405`). For each one, `d1 = distanceFromIntersectArea(r_set, r_p1, size)` (`:417`), and four candidate points are `(p1.x ± d1, p1.y)` and `(p1.x, p1.y ± d1)` (`:420-423`). For each pair of such overlaps, the intersection points of the two circles of radius `d1` and `d2` are candidates too (`:427-435`).
6. Each candidate is scored with `lossFunction` on the current circles (`:443-451`). The first candidate wins a tie, and `bestLoss` starts at `1e50`.

`distanceFromIntersectArea(r1, r2, overlap)` (`:72-79`): if the smaller circle is inside the overlap, `|r1 - r2|`; otherwise `bisect` on `circleOverlap(r1, r2, d) - overlap` over `[0, r1 + r2]`, with the default tolerance 1e-10 and 100 iterations (`build` 270-299).

### L3.4 `bestInitialLayout` (`:236-254`)

`initial = greedyLayout(areas, params)`. If `areas.length >= 8`, it runs `constrainedMDSLayout` and takes its result when `constrainedLoss + 1e-8 < greedyLoss` (`:244-251`). Otherwise the greedy result stays.

### L3.5 `constrainedMDSLayout` (`:261-313`)

1. `restarts = params.restarts || 10` (`:262`).
2. The distance matrix from `getDistanceMatrices` (`:158-194`): for each pair with an overlap area, the distance from `distanceFromIntersectArea`, and a constraint of `1` (subset), `-1` (disjoint) or `0`.
3. The distances are divided by `norm = norm2(row norms) / n` (`:278-279`).
4. The objective `constrainedMDSGradient` (`:197-230`): for each pair, `delta = d² - dij²`, skipped when the constraint is satisfied (`constraint > 0 && distance <= dij`, or `constraint < 0 && distance >= dij`). Otherwise the loss adds `2 * delta²` and the gradient follows (`4 * delta * (xi - xj)`).
5. For each restart: start at `Math.random` values for all `2n` coordinates (`:285`), run `conjugateGradient(obj, initial, params)` (`:287`), keep the best by `fx`, with a strict `<` (`:288`).
6. The positions are multiplied back by `norm`, and the radii are `sqrt(size / π)` (`:293-305`).

### L3.6 `conjugateGradient` and `wolfeLineSearch` (`build` 488-614)

Polak-Ribière, with `beta = max(0, y·g_next / g·g)` (`build` 590). The step is found by `wolfeLineSearch` with `c1 = 1e-6` and `c2 = 0.1` (`build` 497-498), up to 10 expansions and 16 zoom steps (`build` 501, 526). If the search fails, the direction is reset to `-g` (`build` 584). It stops when `|g| <= 1e-5` (`build` 599) or after `maxIterations` (`build` 562: `params.maxIterations`, which `venn()` set to 500). Restarts: 10.

### L3.7 `lossFunction` (`:467-489`)

`sum over areas with at least 2 sets of (overlap - size)²`, weight 1. For a pair, `overlap = circleOverlap(r1, r2, distance)`. For more sets, `intersectionArea` (`:476-482`). A single set contributes nothing (`:470-473`).

### L3.8 `nelderMead` (`build` 339-479)

Parameters: `nonZeroDelta` 1.05, `zeroDelta` 0.001 (for a zero coordinate), `rho` 1, `chi` 2, `psi` -0.5, `sigma` 0.5 (`build` 343-350). The iteration limit is `maxIterations` (500 from `venn()`, 500 from `computeTextCentre`) (`build` 342). The stop test (`build` 407) is `|f0 - fN| < minErrorDelta` and `maxDiff < minTolerance`. `minErrorDelta` defaults to `1e-6` (`build` 345) and `minTolerance` is `parameters.minErrorDelta || 1e-5` (`build` 346), so the tolerance is 1e-5 by default and 1e-10 when `minErrorDelta` is 1e-10 (as `computeTextCentre` sets it).

The simplex is sorted by `fx` (stable, `build` 374, 382) at each iteration, and it reflects, expands, contracts (inside or outside) or reduces, as usual.

### L3.9 `normalizeSolution` (`:661-746`)

1. `orientation` defaults to π/2 (`:662-664`).
2. The circles are copied, and `disjointCluster` (`:591-639`) groups them by union-find, joining two circles when `distance + 1e-10 < r1 + r2` (`:614-616`). The clusters are in order of first appearance (a `Map`, `:623-630`).
3. Each cluster is oriented by `orientateCircles` (`:522-584`): sorted by radius, descending (stable, `:524`), shifted so the largest is at the origin (`:529-538`), a special case of two circles with one inside the other (`:540-548`), then rotated so that the second circle's `atan2` is at `orientation` (`:552-563`), and mirrored when the third circle is above the line (`:567-583`).
4. The clusters are sorted by bounding-box area, descending (`:680`).
5. Clusters are placed in a grid: to the right, below, and below-right of the largest, with a spacing of one fiftieth of its width (`:685`, `:692-742`).

### L3.10 `scaleSolution` (`:760-799`)

`width -= 2 * padding` and `height -= 2 * padding` (`:763-764`). The bounding box is `x ± r` and `y ± r` (`:766`). With `scaleToFit` false (`:782-784`), `scaling = min(width / xRange, height / yRange)` (`:786`). Then the solution is centred: `xOffset = (width - xRange * scaling) / 2` (`:788-789`) and every circle becomes `padding + offset + (x - xMin) * scaling` with radius `scaling * r` (`:791-798`). A zero range returns the input unscaled, with a `console.log` (`:768-770`).

### L3.11 Text centres (`computeTextCentres`, `computeTextCentre`, `circleMargin`; `src/diagram.js:445-610`)

1. For each area: the **interior** is the area's own circles. The **exterior** is every other circle, except those that contain one of the area's circles. `getOverlappingCircles` (`:551-573`) lists the containing circles, and treats `d + r_b <= r_a + 1e-10` as "b lies inside a".
2. `circleMargin(p, interior, exterior)` is the minimum of `r - d` over the interior circles and of `d - r` over the exterior ones (`:445-462`).
3. Candidate points: the centre of each interior circle and the points at `±r/2` on each axis (`:477-484`). The best candidate is the one with the largest margin, with `>=` (a later point wins a tie, `:491-494`).
4. Nelder-Mead maximises the margin from that start, with `{ maxIterations: 500, minErrorDelta: 1e-10 }` (`:497-502`).
5. The result is valid if it lies inside every interior circle and outside every exterior one (`:508-521`). If not: one interior circle gives its centre (`:526-528`); no arcs in the interior's intersection give `{x: 0, y: -1000, disjoint: true}` (`:532-534`); one arc gives that circle's centre (`:535-537`); with exterior circles the search runs again without them (`:538-540`); otherwise the mean of the arc points (`:546`).

The chart writes `Math.floor` of these centres for the set labels (`src/diagram.js:228-229`). `layout()` returns them unfloored (`:743`).

## L4. Randomness

**Every draw.** One place: `src/layout.js:285`, `zeros(distances.length * 2).map(Math.random)`, once per restart, `restarts` (10) times. The arrays have `2n` entries, where `n` is the number of single-set entries in `areas` (`sets` in `constrainedMDSLayout`, `:265-272`, which includes duplicated sets). So one `venn()` call with the constrained branch draws `20n` numbers. The map passes `Math.random` the value, index and array, and `Math.random` ignores them.

**When the branch runs.** Only when `areas.length >= 8`, after `addMissingAreas`, and only then does the comparison decide (`:244-251`). The count is of the list the library receives, so duplicates count:

| Input                                            | Areas (probed) | Branch |
| ------------------------------------------------ | -------------- | ------ |
| Two sets and one union                           | 3              | Greedy |
| Three sets, all pairs and the triple (fixture 2) | 7              | Greedy |
| Three sets, the triple only (fixture 16)         | 7              | Greedy |
| Three sets with `set A` twice                    | 10             | Random |
| Three sets, one union declared three times       | 8              | Random |
| Four sets, no unions                             | 10             | Random |
| Four sets, sizes and unions (`probe-branch.mjs`) | 11             | Random |

Three sets give at most seven areas without duplicates, so no fixture reaches the branch. Four or more sets always do, because the pairs are added (`n` singles and `n(n-1)/2` pairs make at least 10).

**Draws per render.** Two `venn()` calls per render, so `40n` numbers. Probed: the four-set diagram with no unions consumed 160 numbers (`probe-random.mjs`, `40 × 4`).

**Whether the draws change the result.** They change it only when the constrained result wins (L3.4). In `probe-branch.mjs` the constrained branch won in 44 of 120 random configurations of four and five sets, with pairwise and triple unions drawn at random. Among those 44, three calls with different draws gave different final circles in 43 (`probe-branch2.mjs`, compared bit for bit). In the four-set no-union case and in the four-set case with sizes 20, 15, 10 and 5 and the unions of `probe-branch.mjs`, the single check made for each found greedy the winner. Four diagrams (those two, a five-set case and a four-set case with text) rendered five times each in fresh pages gave one output each (`probe-random-many.mjs`, after removing the ids). One of the 44 configurations, rendered by Mermaid six times, gave six different sets of circles (`probe-mismatch.mjs`). The text node in that diagram stayed inside its set's circle in all six renders.

**What survives between calls on one page.** Nothing in the library: there is no module state. The page's `Math.random` generator keeps its state between calls, so an unseeded result depends on how many numbers the page has drawn before. This follows from V8's generator and was not probed separately.

**The harness.** `renderReference` (`tools/port/geometry/common.ts:553-570`) replaces `Math.random` for the duration of one render with a generator seeded by `clock.seed`, which `diagram-geometry.ts:368` sets to 1. The generator is:

```text
state = (state + 0x6d2b79f5) | 0
t = imul(state ^ (state >>> 15), 1 | state)
t = (t + imul(t ^ (t >>> 7), 61 | t)) ^ t
return ((t ^ (t >>> 14)) >>> 0) / 4294967296
```

With this generator the reference is repeatable (probed: the same SVG in the same page and in another page, and 160 draws for the four-set diagram). Ferrigura reproduces the reference under the harness only if it draws the same numbers in the same order: the chart's `venn()` (`vennRenderer.ts:95`) before the layout's (`:102`), `2n` numbers per restart, restarts in order. A port with another generator matches the reference only where greedy wins. (Q3.)

## L5. Output

What Mermaid reads back:

- From the chart: the scaled circles, through the `d` string of each set's path (`circlePath(x, y, r)`, `src/diagram.js:661-668`), and each area's label centre (`textCentres`, floored, `:228-229`). The intersection paths are the arcs of `intersectionAreaPath`, with no rounding (`arcsToPath` with `round` undefined, `:689-706`). These are the values the reference writes, and the harness compares them.
- From `layout()`: for each area of `renderSets`, `{ data, text, circles, arcs, path, distinctPath }` (`:773-781`). Mermaid reads `text` (`{x, y}`, not floored) and `circles` (`x`, `y`, `radius`, the radius of each set in the area). It does not read `arcs`, `path` or `distinctPath`.

`circlePath` builds the string with `join(' ')` over `['\nM', x, y, '\nm', -r, 0, '\na', r, r, 0, 1, 0, 2r, 0, '\na', r, r, 0, 1, 0, -2r, 0]`, so the serialised `d` is `M x y \nm -r 0 \na r r 0 1 0 2r 0 \na r r 0 1 0 -2r 0` (probed). The leading newline of the library's string does not appear in the rendered SVG; the cause was not traced (**unverified**), and the geometry harness parses `d`, so it does not matter for comparison.

## L6. Determinism and floating point

### L6.1 Transcendental functions

`Math.atan2`, `Math.sin`, `Math.cos` and `Math.acos` are in the hot paths (intersection arcs, orientation, circle areas). `Math.hypot` is used by the renderer (`vennRenderer.ts:282`). `Math.pow` appears in the DB (`vennDB.ts:25`, `n²`, exact) and in `arcsToPath` (`src/diagram.js:693`, `10^0`, exact). `Math.sqrt`, `Math.floor`, `Math.ceil`, `Math.abs`, `Math.min`, `Math.max` and `Math.PI` are exact or correctly rounded, so no port needs a special version of them. `Math.log` is in `logRatioLossFunction` only (`src/layout.js:509`), which Mermaid does not use. `Math.round` is in `arcsToPath` when `round` is set (`src/diagram.js:694`), which only `layout()`'s unread paths do.

The port takes `atan2`, `sin`, `cos`, `acos` and `hypot` from `ferrigura-jsmath`, which matches Chromium's results bit for bit (`crates/ferrigura-jsmath/NOTICE.md`). The reference runs in Chromium, so the default module is the right one.

| Call                    | Build line                                                                                  | Used for                                           |
| ----------------------- | ------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| `Math.atan2`            | 38, 61, 62 (src `circleintersection.js` 38, 61-62), `layout.js:553, 568` (build 1165, 1180) | Sorting intersection points; orientation           |
| `Math.sin`, `Math.cos`  | 73-74 (circle arcs); build 1166-1167 (rotation)                                             | Arc midpoints; rotation                            |
| `Math.acos`             | 182 (`circleArea`)                                                                          | Circular segments, in every loss                   |
| `Math.sqrt`             | 192, 240, 176-177 (src), build 788-789, 825, 915, 950, 1390                                 | Distances, radii                                   |
| `Math.hypot` (renderer) | `vennRenderer.ts:282`                                                                       | The inner radius of a text area                    |
| `Math.random`           | build 897 (src `layout.js:285`)                                                             | The starting points of the constrained layout (L4) |

### L6.2 Operation order

- **Object key order.** `Object.keys` gives integer-like keys first, in ascending numeric order, and the other keys in insertion order. Every `circles` and `setOverlaps` object in the library is keyed by set id, so `venn()`'s vector, `greedyLayout`'s candidate order and `normalizeSolution`'s output order all depend on the ids. Probed: sets `"2"`, `"10"`, `"1"` declared in that order give the same circles as declared `"1"`, `"2"`, `"10"`. For non-numeric ids the insertion order counts: sets `a1`, `a2`, `a10` declared in that order give the numeric result, and `x2`, `x10`, `x1` declared in the numeric order give a different one (probed). A port needs a map that yields keys in this order (Q4).
- **Stable sorts.** Every `sort` the library calls is stable in V8 (`mostOverlapped` at `:376`, `overlap` at `:405`, `circles` at `:524`, `clusters` at `:680`, and the simplex at each iteration, `build` 382, 477). The comparators are `b.size - a.size`, `b.radius - a.radius`, `a.fx - b.fx`. A `NaN` in a comparator makes the order unspecified (negative sizes, Q7).
- **String order.** `addMissingAreas` sorts the ids with `<`, which is UTF-16 code-unit order (`:137`). The DB sorts with the default `sort()`, which is the same order (`vennDB.ts:24`).
- **Summation.** Sums run in array order, left to right: `loss` (`:467-489`), `dot` (`build` 314-320), the gradient (`:197-230`).
- **Iteration limits.** `bisect`: 100 iterations, tolerance 1e-10. `nelderMead`: 500 (venn and the text centre). `conjugateGradient`: 500 (from `venn()`). `wolfeLineSearch`: 10 expansions, 16 zoom steps. Restarts: 10. Greedy: none.
- **Tolerances.** `SMALL` 1e-10 (`src/circleintersection.js:1`, `src/layout.js:62`). Complete overlap `1e-10` (`:353` for greedy, `:185` for the constraints). Containment `1e-10` (`src/diagram.js:565`, `:567`). Union-find `1e-10` (`src/layout.js:615`). The constrained-versus-greedy margin `1e-8` (`:249`). Gradient stop `1e-5` (`build` 599).
- **Number formatting.** The `d` strings and attributes are JavaScript numbers in template strings, so the port needs JavaScript's shortest round-trip conversion, including exponent form for very small magnitudes. Probed values: `x="0"`, `y="-1000"` and `NaN`. Exponent forms were not checked across the fixtures; a port must still follow JavaScript's rules.
- **Floor.** The chart floors set-label centres with `Math.floor` (`src/diagram.js:228-229`) before they are written, so the label `x` is an integer string.

### L6.3 Quirks a port must reproduce

1. The `minTolerance` line reads `minErrorDelta` (`build` 346). The effective tolerance for the text-centre search is 1e-10, not the 1e-5 its name suggests.
2. `venn()` mutates `parameters.maxIterations` (`:12`).
3. The chart and `layout()` both call `venn()` on the same data, so the same layout is computed twice, and they match only where the layout is deterministic (L4).
4. `computeTextCentre` returns `{x: 0, y: -1000, disjoint: true}` for a disjoint area (`src/diagram.js:532-534`), which Mermaid writes out as the label position (6.13, item 3).
5. A zero-size single with no label gives a zero radius (`src/layout.js:303`); `layout()` keeps it (6.13, item 2).

## L7. How to test a port

**The layout, bit for bit.** The reference runs in Chromium. Serve `build/venn.esm.js` from the package and import it in a page (the probes did this). For each input, build `renderSets` as the DB does (sorted sets, the defaults of 2.4), and call `venn.layout(renderSets, {width, height, padding})` and `VennDiagram()` on a detached `div` with `width` and `height`. Record every circle's `x`, `y` and `radius`, and each area's text centre, as 64-bit hex. The Rust layout must match with no ulp difference for inputs on the greedy branch. For inputs on the random branch, run the reference with the harness's generator (L4) in the same page, so the draws line up, and compare the same way.

**The renderer.** `pnpm port:geometry venn` (the script is `package.json:73`, through `tools/port/geometry/diagram-geometry.ts`). It compares the `d` of the circles and intersections, the set-label positions, and the text boxes, at the tolerance of 0.05. The four fixtures with text nodes will show the padding offset (6.5) unless the port reproduces it.

**Inputs for the random branch.** The fixtures have no branch-taking input. Add at least one four-set fixture and run both sides with the harness seed, so the branch is checked (Q3).

**Inputs for the key-order rule.** A fixture with numeric set ids (`set "2"`, `set "10"`, `set "1"`) checks Q4.

## L8. Risks, effort, open questions

| Risk                                         | Level  | Why                                                                                                                                                                    |
| -------------------------------------------- | ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Bit-exact Nelder-Mead and conjugate gradient | High   | Long chains of floating-point operations and tolerance tests; any change of order moves the last bits, and the simplex sorting is stable only with the same comparator |
| Random restarts                              | Medium | The branch needs the harness generator and the draw order (L4). Nothing in the fixtures reaches it                                                                     |
| `Object.keys` order                          | Medium | Easy to miss; changes the layout for numeric ids (L6.2)                                                                                                                |
| Transcendental functions                     | Low    | `ferrigura-jsmath` covers `atan2`, `sin`, `cos`, `acos` and `hypot`                                                                                                    |
| The text-centre search                       | Medium | Its starting points and ties (`>=`) decide the label position (L3.11)                                                                                                  |

**Effort.** The part of the library Mermaid reaches is about 900 to 1,100 lines of Rust, without tests (section 8.2): `venn`, `addMissingAreas`, `greedyLayout`, the constrained branch with `conjugateGradient` and `wolfeLineSearch`, `nelderMead`, `lossFunction`, `normalizeSolution` with its helpers, `scaleSolution`, and the text centres with `intersectionArea`. The chart's d3 code and `wrapText` are not needed, beyond a single `tspan`.

**Open questions for the layout.** Q3 (the branch), Q4 (key order), Q11 (licence and the `fmin` patch) and Q12 (crate or module).

---

## Summary of what the coordinator must decide

Q1 (text nodes as HTML against decision 0004), Q2 (padding 8 for text and 15 for circles), Q3 (the randomised branch and its seed), Q4 (numeric id order), Q7 (negative sizes, invalid colours, empty diagrams), Q9 (diagram-scoped `theme` and `look` defaults for the typed views), Q12 (crate or module for the layout). Q5, Q6, Q8, Q10 and Q11 are smaller and listed in section 8.3.
