# Tree view (`treeView-beta`): research notes for the port

Status: research only. Nothing in the repository changed except this file, and nothing is committed. Reference: `packages/mermaid/src` and `packages/parser/src` in this worktree (branch `sebastian/port-p3`, HEAD `f3a1a94b0`). Plan: [../P3-PLAN.md](../P3-PLAN.md). Guide: [../P2-RESEARCH.md](../P2-RESEARCH.md). Model for the shared steps: [../kanban/NOTES.md](../kanban/NOTES.md) and [../architecture/PLAN.md](../architecture/PLAN.md) section 2 (the lexer description).

Every rule cites the reference as `diagrams/treeView/<file>:<line>` unless the file is named in full. **Probed** means the rule was run through the reference's own code, with the method named in the row or the paragraph. **Unverified** means read or inferred only.

Probe material is outside the repository in `/private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/tree-view/`:

- `probe-parse.mts` with `cases-parse.json`, `cases-2.json`, `cases-title.json` (the parser, the DB and the preprocessing, run as `Diagram.fromText` runs them: `preprocessDiagram`, `encodeEntities(cleaned) + "\n"`, `parser.parse`, then `db.getRoot()`), loaded through `tools/port/reference.ts` (`openReference`, Vite SSR). The outputs are `parse-log*.txt` and `parse-out*.json`.
- `probe-lexer.mts`, `probe-lexer2.mts`, `probe-lexer3.mts` and `probe-lexer4.mts`: the Langium lexer of `packages/parser/src/language/treeView/index.ts`, its token list, `LONGER_ALT` and tokens of given inputs.
- `probe-render.mts` with `render-cases.json` (the 15 fixtures and 25 probes) and `cases-render-2.json`: the bundle through the harness's own page, `withPage`, `renderReference` and `patchText` from `tools/port/geometry/common.ts`, with the harness settings (`htmlLabels: false`, Lexica Ultralegible, `securityLevel: strict`, `redux-color`). The outputs are `render-out.json` and `render-out-2.json`. Each entry has the SVG string, and an outline with the `getBBox` of every text element, measured after the SVG is inserted again.

Probed numbers in this file come from the `viewBox`, the attributes and the outlines. Where the outline's `getBBox` and the render-time value differ, the render-time value is the one used (section 6.3).

## 0. Summary

A tree view is a Langium grammar over a line-oriented token set. Its DB is a stack of nodes built from the indentation level of each line. Its renderer places one row per node by arithmetic on the measured height and width of each label, draws connectors and vertical trunks, and sizes the root `svg` from the totals. The only icons are two built-in SVG paths (`folder`, `file`) and an "unknown" box with a question mark. There is no layout library and no randomness.

Three findings matter most for the port:

1. **The reference always draws a root row `/`.** `renderer.ts:182` starts the walk at `db.getRoot()`, the synthetic node the DB creates at `db.ts:22-33`. Every diagram has that row: a bold directory labelled `/`, with the user's top-level nodes as its children. This holds for an empty diagram too (probed, `P-root-one`, `P-empty`). The reference's own docs show no such row (`packages/mermaid/src/docs/syntax/treeView.md`, section "Syntax", whose examples start at the first user node).
2. **The lexer and the preprocessing have many quirks that change the tree silently.** A name that starts with `title` is split (`titlepage.md` becomes a node `page.md`). A whitespace-only line gives the next node its indentation. A name with a `├` inside an indent-based file loses the text before the character. Section 2.7 lists them, each with a test input.
3. **Text measurement is the geometry.** Every row height is the label's `getBBox` height plus `2 * paddingY`, and every width comes from `getBBox` of a label that is bold for directories, italic for descriptions, and `white-space: pre`. Entities in names are measured as their placeholders (`ﬂ°amp¶ß`), not as the characters they draw (probed: `A #amp; B` makes the viewBox 116.92 wide; `A & B` makes it 65.17).

| Item                 | Value                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Reference, non-spec  | `diagrams/treeView/`: `boxDrawingPreprocessor.ts` (209 lines), `db.ts` (91), `detector.ts` (21), `diagram.ts` (12), `icons.ts` (100), `parser.ts` (57), `renderer.ts` (242), `styles.ts` (58), `types.ts` (50). 840 lines in total                                                                                                                                                                                                        |
| Parser glue, Langium | `packages/parser/src/language/treeView/`: `treeView.langium` (64 lines), `valueConverter.ts` (38), `tokenBuilder.ts` (7), `module.ts` (80), `index.ts` (1). The grammar is the specification; the port replaces the glue by a hand-written scanner (decision [0002](../decisions/0002-hand-written-parsers.md))                                                                                                                           |
| Shared code it uses  | `populateCommonDb.ts`, `commonDb.ts` (title and accessibility setters), `common.ts` (`sanitizeText`, DOMPurify), `selectSvgElement.ts`, `setupGraphViewbox.js` (`configureSvgSize`), `rendering-util/icons.ts` (`registerIconPacks`, `getIconSVG`), `@iconify/utils` 3.0.2 (`iconToSVG`, `iconToHTML`, `replaceIDs`), the core's `preprocess.ts`, `diagram-api/comments.ts`, `utils.ts` (`encodeEntities`, `cleanAndMerge`), `Diagram.ts` |
| Specs                | 4 files, 116 `it` calls: `parser.spec.ts` 33, `db.spec.ts` 18, `icons.spec.ts` 23, `boxDrawingPreprocessor.spec.ts` 42. Counted by grep; not run. No spec for the renderer or the styles                                                                                                                                                                                                                                                  |
| Fixtures             | 15 `.mmd` files: 5 in `nightly-check/`, 10 in `pr-check/` (plus a `.gitkeep`). `visual-check/` is empty. The generic harness excludes 2 of the 15 (emoji, section 7)                                                                                                                                                                                                                                                                      |
| Parser               | Langium 4.4.0 with Chevrotain 13.2.0. Hand port with a first-match scanner, the keyword with a longer-alternative rule (section 2.2). The DB is a stack; `populate` calls `addNode` once per node                                                                                                                                                                                                                                         |
| DB                   | Module state: a counter and a stack, reset by `clear`. `getRoot` is the synthetic `/`. Config from `getCommonConfig().treeView` merged onto the defaults                                                                                                                                                                                                                                                                                  |
| Renderer             | Custom arithmetic. Rows top to bottom; the connector and the trunk for each node; a description column; highlight rectangles sized last. `getBBox` on labels and descriptions; `innerHTML` for icons                                                                                                                                                                                                                                      |
| Riskiest parts       | (1) `getBBox` parity for bold, italic and placeholder texts, and the icon's `?` drawn in ArialMT (section 8.1). (2) The lexer quirks, which must stay exact. (3) The order of the arithmetic for highlights and the description column                                                                                                                                                                                                    |
| Effort               | Medium. Rough estimate: 1,300 to 1,650 lines of Rust for the scanner, the parser, the preprocessor, the DB, the icons, the renderer and the styles, plus about 900 to 1,100 lines of tests and a differential exporter (section 8.2)                                                                                                                                                                                                      |

No exporter exists (`tools/port/export-tree-view.ts` is absent). `tools/port/diff-config.ts:260` lists `treeView` among the diagram keys of the configuration diff.

## 1. Sources

| File                                                               | Lines                       | What it holds                                                                                                                                                           |
| ------------------------------------------------------------------ | --------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| `packages/mermaid/src/diagrams/treeView/detector.ts`               | 6-8                         | The detector `/^\s*treeView-beta/`. It matches `treeView-betaX` too, and the parse then fails (section 2.7)                                                             |
| `diagrams/treeView/diagram.ts`                                     | 7-12                        | The definition: `db`, `renderer`, `parser`, `styles`                                                                                                                    |
| `diagrams/treeView/parser.ts`                                      | 11-41                       | `populate`: `populateCommonDb`, then one `db.addNode` per AST node; level, directory suffix, class, icon, description                                                   |
| `diagrams/treeView/parser.ts`                                      | 43-57                       | `parser.parse`: `preprocessBoxDrawing` (line 45), `parse('treeView', text)` (line 47), error remapping (lines 50-55)                                                    |
| `diagrams/treeView/boxDrawingPreprocessor.ts`                      | 9-14                        | Character classes: all box chars, branch chars, dashes, decoration-only lines, the metadata and comment patterns                                                        |
| `diagrams/treeView/boxDrawingPreprocessor.ts`                      | 16                          | `INDENT_UNIT = '    '` (4 spaces per output depth level)                                                                                                                |
| `diagrams/treeView/boxDrawingPreprocessor.ts`                      | 18-23                       | `PreprocessResult`: the text and a line map                                                                                                                             |
| `diagrams/treeView/boxDrawingPreprocessor.ts`                      | 28-30                       | `isBoxDrawingFormat`: any content line with a box character                                                                                                             |
| `diagrams/treeView/boxDrawingPreprocessor.ts`                      | 37-45                       | `inferSegmentWidth`: the column of the first branch character at a column above 0, else 4                                                                               |
| `diagrams/treeView/boxDrawingPreprocessor.ts`                      | 50-56                       | `remapErrorLines`: rewrites `line N` in a message through the line map                                                                                                  |
| `diagrams/treeView/boxDrawingPreprocessor.ts`                      | 67-209                      | `preprocessBoxDrawing`: keyword line, content detection, segment width, output by line                                                                                  |
| `diagrams/treeView/db.ts`                                          | 17-38                       | The state (`cnt`, `stack`) and `clear`                                                                                                                                  |
| `diagrams/treeView/db.ts`                                          | 40-50                       | `getRoot`, `getCount`, `getConfig`                                                                                                                                      |
| `diagrams/treeView/db.ts`                                          | 52-75                       | `addNode`                                                                                                                                                               |
| `diagrams/treeView/db.ts`                                          | 77-91                       | The DB object: the tree functions and the common setters and getters                                                                                                    |
| `diagrams/treeView/types.ts`                                       | 12-25                       | `NodeType`, `Node` (the DB node; the renderer adds `BBox`)                                                                                                              |
| `diagrams/treeView/types.ts`                                       | 27-48                       | `TreeViewDB`, `TreeViewDiagramStyles`                                                                                                                                   |
| `diagrams/treeView/icons.ts`                                       | 15-27                       | `treeViewIcons`: prefix `mermaid-treeview`, 24 by 24, the two bodies `folder` (line 21) and `file` (line 24)                                                            |
| `diagrams/treeView/icons.ts`                                       | 29-34, 43-55                | `IconDetectionConfig`, `detectIcon` (exact filename, then extension, lower-cased)                                                                                       |
| `diagrams/treeView/icons.ts`                                       | 58-66                       | `qualifyIcon`: a built-in name wins, then `defaultIconPack`, then the built-in prefix                                                                                   |
| `diagrams/treeView/icons.ts`                                       | 77-100                      | `getNodeIcon`: the rules of section 6.2                                                                                                                                 |
| `diagrams/treeView/renderer.ts`                                    | 10-15                       | `registerIconPacks` at module load (a side effect)                                                                                                                      |
| `diagrams/treeView/renderer.ts`                                    | 17-19                       | `ICON_SIZE` 14, `ICON_GAP` 4, `DESC_GAP` 16                                                                                                                             |
| `diagrams/treeView/renderer.ts`                                    | 28-50                       | `resolveNodeIcons`: `getIconSVG` for every node that has an icon, in parallel                                                                                           |
| `diagrams/treeView/renderer.ts`                                    | 52-110                      | `positionLabel`: the icon group, the label, the row height and width, the highlight rectangle                                                                           |
| `diagrams/treeView/renderer.ts`                                    | 112-128                     | `positionLine`                                                                                                                                                          |
| `diagrams/treeView/renderer.ts`                                    | 130-216                     | `drawTree`: `drawNode`, `processNode`, descriptions, highlights                                                                                                         |
| `diagrams/treeView/renderer.ts`                                    | 218-236                     | `draw`: the viewBox and `configureSvgSize`                                                                                                                              |
| `diagrams/treeView/styles.ts`                                      | 5-13                        | The default style values                                                                                                                                                |
| `diagrams/treeView/styles.ts`                                      | 15-28                       | `styles`: reads `options.treeView` and merges the defaults                                                                                                              |
| `diagrams/treeView/styles.ts`                                      | 29-55                       | The style sheet text                                                                                                                                                    |
| `packages/parser/src/language/treeView/treeView.langium`           | 13-63                       | The grammar: terminals (lines 15-17, 40-57), `entry TreeView` (26-30), `TitleAndAccessibilities` (32-33), `TreeNode` (59-63)                                            |
| `packages/parser/src/language/treeView/valueConverter.ts`          | 4-37                        | `TreeViewValueConverter.runCustomConverter`: the values of the terminals                                                                                                |
| `packages/parser/src/language/treeView/tokenBuilder.ts`            | 3-6                         | `TreeViewTokenBuilder` with the keyword `treeView-beta`                                                                                                                 |
| `packages/parser/src/language/treeView/module.ts`                  | 41-49                       | Services: the token builder and the value converter                                                                                                                     |
| `packages/parser/src/language/common/tokenBuilder.ts`              | 14-28                       | The keyword suffix `(?:(?=%%)                                                                                                                                           | (?!\S))` |
| `packages/parser/src/language/common/valueConverter.ts`            | 27-70                       | `runConverter`: common converter first (titles, accessibility), then the custom one                                                                                     |
| `packages/parser/src/language/common/matcher.ts`                   | 4-14                        | `titleRegex`, `accessibilityTitleRegex`, `accessibilityDescrRegex`                                                                                                      |
| `packages/parser/src/parse.ts`                                     | 55-59, 133-150              | The treeView initializer and `parse`: any lexer or parser error throws `MermaidParseError` (146-148)                                                                    |
| `packages/parser/src/parse.ts`                                     | 152-174                     | `MermaidParseError`: `Lexer error on line L, column C`, `Parse error on line L, column C`, `line ?` when the location is missing                                        |
| `packages/parser/src/language/generated/grammar.ts`                | 49                          | The serialised grammar (a JSON string); it matches the `.langium` file                                                                                                  |
| `packages/parser/src/language/generated/ast.ts`                    | 2703-2732                   | `TreeNode` and `TreeView` as generated                                                                                                                                  |
| `node_modules/.pnpm/langium@4.4.0/.../lib/parser/token-builder.js` | 19-30                       | `buildTokens`: keyword tokens first, then the terminals in declaration order, hidden ones included                                                                      |
| `node_modules/.pnpm/langium@4.4.0/.../lib/parser/token-builder.js` | 80-112                      | `buildKeywordToken` and `findLongerAlt` (`LONGER_ALT` is every terminal whose anchored pattern matches the keyword's text)                                              |
| `node_modules/.pnpm/langium@4.4.0/.../lib/parser/lexer.js`         | 23-31                       | The Chevrotain lexer: `positionTracking: 'full'`                                                                                                                        |
| `packages/mermaid/src/Diagram.ts`                                  | 19-86                       | `fromText`: `db.clear` (49), `init` (50), the frontmatter title (52-54), `encodeEntities(source) + '\n'` (77), `parser.parse` (83)                                      |
| `packages/mermaid/src/preprocess.ts`                               | 6-17, 19-31, 33-45, 58-91   | `cleanupText` (CRLF and HTML attribute quotes), frontmatter, directives, `preprocessDiagram` (cleaned text at line 65)                                                  |
| `packages/mermaid/src/diagram-api/comments.ts`                     | 6-8                         | `cleanupComments`: removes lines `^\s*%%(?!{)[^\n]+` (at least one character after `%%`)                                                                                |
| `packages/mermaid/src/utils.ts`                                    | 874-897, 904-906, 841-843   | `encodeEntities`, `decodeEntities`, `cleanAndMerge` (lodash `merge` onto `{}`)                                                                                          |
| `packages/mermaid/src/diagrams/common/common.ts`                   | 56-69, 71-83                | `sanitizeMore` (only with HTML labels), `sanitizeText` (DOMPurify, `FORBID_TAGS: ['style']`)                                                                            |
| `packages/mermaid/src/diagrams/common/commonDb.ts`                 | 4-32                        | The title and accessibility setters, which sanitise                                                                                                                     |
| `packages/mermaid/src/diagrams/common/populateCommonDb.ts`         | 4-14                        | Sets accessibility and title only when the AST value is truthy                                                                                                          |
| `packages/mermaid/src/utils/imperativeState.ts`                    | 24-37                       | `ImperativeState`: `records` and `reset()`                                                                                                                              |
| `packages/mermaid/src/rendering-util/icons.ts`                     | 20-24, 29-46, 48-77, 88-106 | `unknownIcon`, `registerIconPacks`, `getRegisteredIconData`, `getIconSVG` (sanitised at line 105)                                                                       |
| `packages/mermaid/src/rendering-util/selectSvgElement.ts`          | 11-22                       | `selectSvgElement`: `select('body')`, or the sandbox document                                                                                                           |
| `packages/mermaid/src/setupGraphViewbox.js`                        | 23-46                       | `calculateSvgSizeAttrs` and `configureSvgSize`                                                                                                                          |
| `packages/mermaid/src/styles.ts`                                   | 11-164, 34-38, 166-170, 226 | `getStyles`: the global sheet (the template from line 42), the diagram's own `themes[type]` with `svgId` (lines 34-38), `addStylesForDiagram` (166), `compileCSS` (226) |
| `packages/mermaid/src/mermaidAPI.ts`                               | 327-333                     | `createUserStyles`: the options are `{ ...config.themeVariables, theme, look }` (line 330), the result is `compileCSS(svgId, allStyles)` (line 333)                     |
| `packages/mermaid/src/mermaidAPI.ts`                               | 359, 419, 623               | `decodeEntities` on the final SVG string (359), the wrapper `svg` with an empty `g` (419), the accessibility title and description (623)                                |
| `node_modules/.pnpm/@iconify+utils@3.0.2/.../lib/svg/id.js`        | 17, 23, 31-44               | `replaceIDs`: a random prefix from `Date.now()` and `Math.random()` at module load (line 23), used only when the body has `id="…"`                                      |
| `node_modules/.pnpm/@iconify+utils@3.0.2/.../lib/svg/html.js`      | 4-8                         | `iconToHTML`: `<svg xmlns …>` with the attributes and the body                                                                                                          |
| `packages/mermaid/src/schemas/config.schema.yaml`                  | 422-423, 2742-2808, 464-468 | `treeView` in the config (422), `TreeViewDiagramConfig` (2742-2808, keys 2749-2808), `useMaxWidth` of the base (464-468)                                                |
| `packages/mermaid/src/config.type.ts`                              | 2188-2245                   | `TreeViewDiagramConfig` (the TypeScript type)                                                                                                                           |
| `packages/mermaid/src/defaultConfig.ts`                            | 285-288                     | `treeView: { ...defaultConfigJson.treeView, useWidth: undefined }`                                                                                                      |
| `packages/mermaid/src/docs/syntax/treeView.md`                     | 1-277                       | The user documentation: syntax, box-drawing input, annotations, comments, examples, config and theme variables                                                          |

## 2. Parser

### 2.1 The grammar

The grammar is `treeView.langium` (`grammar TreeViewGrammar`). Its rules, as the generated grammar has them (`grammar.ts:49`, the rule order is the file order):

```
terminal ACC_DESCR  : /[\t ]*accDescr(?:[\t ]*:([^\n\r]*?(?=%%)|[^\n\r]*)|\s*{([^}]*)})/;   (15)
terminal ACC_TITLE  : /[\t ]*accTitle[\t ]*:(?:[^\n\r]*?(?=%%)|[^\n\r]*)/;                   (16)
terminal TITLE      : /[\t ]*title(?:[\t ][^\n\r]*?(?=%%)|[\t ][^\n\r]*|)/;                    (17)

entry TreeView returns TreeView:                                                              (26)
    "treeView-beta"  TitleAndAccessibilities?  nodes+=TreeNode*                               (27-29)
fragment TitleAndAccessibilities:
  ((accDescr=ACC_DESCR | accTitle=ACC_TITLE | title=TITLE))+                                  (32-34)

terminal CLASS_ANNOTATION : /[ \t]+:::[ \t]*[A-Za-z_][\w-]*/;                                 (40)
terminal ICON_ANNOTATION  : /[ \t]+icon\([\w-]*(?::[\w-]+)?\)/;                              (42)
terminal DESC_ANNOTATION  : /[ \t]+##[^\n\r]*/;                                              (43)
terminal INDENTATION      : /[ \t]{1,}/;                                                     (46)
terminal QUOTED_NAME      : /"[^"]*"|'[^']*'/;                                               (49)
hidden terminal WS        : /[ \t]+/;                                                        (51)
hidden terminal ML_COMMENT: /\%\%[^\n]*/;                                                    (52)
hidden terminal NL        : /\r?\n/;                                                         (53)
terminal BARE_NAME: /(?!:::|icon\(|##)[^ \t\n\r"'](?:(?![ \t]+:::[ \t]*[A-Za-z_]|[ \t]+icon\(|[ \t]+##)[^\n\r])*/;   (57)

TreeNode:                                                                                    (59-63)
    indent=INDENTATION?
    (name=QUOTED_NAME | name=BARE_NAME)
    (classAnnotation=CLASS_ANNOTATION | iconAnnotation=ICON_ANNOTATION | descAnnotation=DESC_ANNOTATION)*
```

The AST interfaces are `TreeView { nodes, title?, accTitle?, accDescr? }` (`treeView.langium:19-24`) and `TreeNode { indent?, name, classAnnotation?, iconAnnotation?, descAnnotation? }` (`ast.ts:2703-2711`). The grammar has no `%%{…}%%` directive rule and no front-matter rule. The common grammar's `YAML`, `DIRECTIVE`, `SINGLE_LINE_COMMENT` and `WHITESPACE` are not imported. Directives and front matter are removed by the core before the parser runs (`preprocess.ts:33-45`, `preprocess.ts:19-31`).

Each line is one node, or one header line, or nothing. `NL` is hidden, so the parser never sees line breaks. This is why an indentation token that stands alone is attached to the next node (section 2.7, Q4).

### 2.2 Lexer rules

The token list is what `buildTokens` returns (`token-builder.js:19-30`), read in Langium 4.4.0 with `createTreeViewServices().TreeView.parser.TokenBuilder.buildTokens` (probed, `probe-lexer4.mts`):

| Order | Token              | Pattern                           | Group   | `LONGER_ALT`  |
| ----- | ------------------ | --------------------------------- | ------- | ------------- |
| 1     | `treeView-beta`    | `treeView-beta(?:(?=%%)\|(?!\S))` | keyword | `[BARE_NAME]` |
| 2     | `ACC_DESCR`        | grammar line 15                   | token   | none          |
| 3     | `ACC_TITLE`        | grammar line 16                   | token   | none          |
| 4     | `TITLE`            | grammar line 17                   | token   | none          |
| 5     | `CLASS_ANNOTATION` | grammar line 40                   | token   | none          |
| 6     | `ICON_ANNOTATION`  | grammar line 42                   | token   | none          |
| 7     | `DESC_ANNOTATION`  | grammar line 43                   | token   | none          |
| 8     | `INDENTATION`      | grammar line 46                   | token   | none          |
| 9     | `QUOTED_NAME`      | grammar line 49                   | token   | none          |
| 10    | `WS`               | grammar line 51                   | skipped | none          |
| 11    | `ML_COMMENT`       | grammar line 52                   | hidden  | none          |
| 12    | `NL`               | grammar line 53                   | skipped | none          |
| 13    | `BARE_NAME`        | grammar line 57                   | token   | none          |

Chevrotain 13.2.0 takes the first pattern that matches, in this order, unless a token's `LONGER_ALT` matches a longer text at the same position. The keyword's `LONGER_ALT` is `[BARE_NAME]`, because `BARE_NAME` matches the keyword's text. Probed (`probe-lexer.mts`, `probe-lexer2.mts`): `treeView-beta\na` lexes as the keyword, then `a`. `treeView-beta foo`, `treeView-beta  ` (trailing spaces) and `treeView-betaX` lex as one `BARE_NAME`, because `BARE_NAME` matches longer. A minimal Chevrotain lexer without `LONGER_ALT` lexes `treeView-beta foo` as keyword then name (`chev-exp.mjs`), which is what identifies the `LONGER_ALT` as the cause.

Consequences the port keeps (section 2.7):

- A word that starts with `title` (case-sensitive) lexes as `TITLE` when the lexer reaches it, whatever follows. `TITLE` has an empty alternative, so `title` alone matches (`grammar line 17`). `titlepage.md` is `TITLE("title")` then `BARE_NAME("page.md")`, probed.
- `ACC_TITLE` and `ACC_DESCR` need a colon (or a brace), so `accTitles.md` and `accDescribed.md` are names (probed).
- Leading spaces are part of `TITLE`, `ACC_TITLE` and `ACC_DESCR`, and of `INDENTATION`. Since `TITLE` comes before `INDENTATION`, an indented name that starts with `title` loses its indentation to `TITLE` (probed: `src/` then `    titles.ts` is a parse error).
- `WS` never matches. `INDENTATION` (`[ \t]{1,}`) comes first and takes every run of spaces and tabs. So trailing spaces after a quoted name are an `INDENTATION` token, which the parser cannot place (probed, Q7).
- `ML_COMMENT` is `%%` and the rest of the line, with no leading spaces. `cleanupComments` has already removed the lines that start with `%%` and have text after them (section 2.4), so `ML_COMMENT` only sees `%%` alone and `%%` inside a line.

### 2.3 Value converter

The custom converter (`valueConverter.ts:4-37`) runs after the common converter (`common/valueConverter.ts:27-42`). The common one handles `ACC_DESCR`, `ACC_TITLE` and `TITLE` through `matcher.ts`, and returns `undefined` for the other terminals.

| Terminal            | Converter                                                                                      | Lines                            | Result                                                                                     |
| ------------------- | ---------------------------------------------------------------------------------------------- | -------------------------------- | ------------------------------------------------------------------------------------------ |
| `INDENTATION`       | `input.length` (characters; a tab is 1)                                                        | `valueConverter.ts:10-12`        | the level of the node. `TreeNode` gets `0` when the indentation is absent (`parser.ts:14`) |
| `QUOTED_NAME`       | `input` without its first and last character                                                   | `13-16`                          | the name. `""` gives `""`                                                                  |
| `BARE_NAME`         | `input` without trailing `[\t ]+`                                                              | `17-20`                          | the name; inner runs of spaces stay                                                        |
| `CLASS_ANNOTATION`  | trim, drop `:::`, trim                                                                         | `21-25`                          | the class name (`my-cls1` for `:::my-cls1`)                                                |
| `ICON_ANNOTATION`   | trim, drop `icon(` and `)`                                                                     | `26-30`                          | the icon name; `""` for `icon()`                                                           |
| `DESC_ANNOTATION`   | trim, drop `##`, trim                                                                          | `31-35`                          | the description; `""` for `##` alone                                                       |
| `TITLE`             | `titleRegex` (`matcher.ts:14`) on the token: trim, collapse runs of two or more spaces or tabs | `common/valueConverter.ts:57-60` | the title; `""` for `title` alone (probed)                                                 |
| `ACC_TITLE`         | `accessibilityTitleRegex` (`matcher.ts:9`), trim, collapse runs                                | `common/valueConverter.ts:57-60` | the accessible title                                                                       |
| `ACC_DESCR` (block) | `accessibilityDescrRegex` (`matcher.ts:4`), the block rules                                    | `common/valueConverter.ts:62-68` | the accessible description                                                                 |

Notes that a port must follow:

- The level is a character count. A tab in indent mode is 1, not 4. Box-drawing mode turns tabs into four spaces first (`boxDrawingPreprocessor.ts:97, 155`).
- `ICON_ANNOTATION` takes everything between `icon(` and `)`. The token pattern `[\w-]*(?::[\w-]+)?` only allows a name, a colon and a name (`treeView.langium:42`). So `icon(a.b)` is a lexer error (probed).
- The `TITLE` value of a line with a `%%` comment is cut before the comment, because the `TITLE` pattern stops before `%%` (`[\t ][^\n\r]*?(?=%%)`). `title Foo %% note` gives `Foo`.

### 2.4 Preprocessing

The text the parser receives is produced by the core, then by the treeView preprocessor.

1. **Core, `Diagram.fromText`** (`Diagram.ts:19-86`). `detectType` matches the detector (`detector.ts:7`). `db.clear()` (line 49). `init`. If the frontmatter has a title, `db.setDiagramTitle` (lines 52-54). The source is the cleaned text, or with comments when `preserveCommentsWhenParsing` is set (not set for this type; lines 66-76). Then `encodeEntities(source) + "\n"` (line 77), and `parser.parse` (line 83).
2. **Core, `preprocessDiagram`** (`preprocess.ts:58-91`). `cleanupText`: CRLF and CR become LF (`preprocess.ts:10`), and HTML attributes get single quotes (`12-16`). Frontmatter comes out first (`19-31`), then the directives (`33-45`, with `removeDirectives`), then `cleanupComments` (`preprocess.ts:65`, `diagram-api/comments.ts:7`). That regex is `^\s*%%(?!{)[^\n]+\n?` with the `m` flag, so it removes whole lines that start with `%%` and have at least one more character, together with the blank lines before them. A line that is only `%%` is kept.
3. **Core, `encodeEntities`** (`utils.ts:874-897`). `style…:…#…;` and `classDef…` are cut before the last `;` (lines 877-882). Then `#\w+;` becomes `ﬂ°` plus the name plus `¶ß`, or `ﬂ°°` plus the digits for a number (lines 884-895). The placeholders reach the parser. `decodeEntities` (`utils.ts:904-906`) runs on the final SVG string in `mermaidAPI.ts:359`, so the characters appear in the SVG, but the layout has measured the placeholders (section 6.3).
4. **TreeView, `preprocessBoxDrawing`** (`parser.ts:45`, `boxDrawingPreprocessor.ts:67-209`). Described in section 2.5. It runs on the text of step 3, and only then does the parser see it.

### 2.5 Box-drawing input

The preprocessor is a second parser that runs before the grammar. Its rules, from `boxDrawingPreprocessor.ts`:

- **Keyword line** (lines 71-83). The first line whose `trim()` is exactly `treeView-beta` is the keyword line. If there is none, the text is returned unchanged. Lines before the keyword are copied (113-117).
- **Content lines** (lines 86-98). Lines after the keyword that are blank, `%%` comments, metadata (`title`, `accTitle:`, `accDescr:` or `accDescr {`, lines 13 and 90) or decoration-only (`^[\s│┃]+$`, line 93) are skipped. The others have tabs turned into four spaces (line 97).
- **Box mode** (lines 101-103). Box mode is on when any content line contains one of `─━│┃└┗├┣`, anywhere, including inside a name. In indent mode the text is returned unchanged.
- **Segment width** (line 106, lines 37-45). The column of the first branch character (`└┗├┣`) at a column above 0. Otherwise 4.
- **Output** (lines 120-206), line by line:
  - blank, comment and metadata lines pass through (lines 125-147);
  - decoration-only lines are dropped (lines 150-152);
  - a line with a branch character gets depth `Math.round(branchCol / segmentWidth) + 1` (lines 160-163) and an indent of `4 * depth` spaces. The content is what follows the branch, its dashes and its spaces (lines 165-173). An empty content throws `Line N: Empty node — expected a filename or directory name after the box-drawing prefix` (lines 175-179);
  - a line that is only box characters and spaces is dropped (lines 185-187);
  - a line with a box character but no branch character passes through as written, with its indentation (lines 188-193);
  - a line with leading whitespace and no box character throws `Line N: Unexpected indentation without box-drawing characters. In box-drawing format, use ├── or └── prefixes for indented nodes.` (lines 194-199);
  - any other line passes through as a root item (lines 200-205).

The output keeps the original line numbers in `lineMap`. `parser.ts:51-53` rewrites `line N` in a Langium error through that map, but only when the map is not empty (box mode). The two thrown messages above are plain `Error`s thrown before the `try` block (`parser.ts:45-46`), so they are not remapped and not wrapped in `MermaidParseError`.

Probed (`probe-parse.mts`, `cases-2.json`):

| Label                                              | Input (after `treeView-beta`)                                                           | Result                                                                                    |
| -------------------------------------------------- | --------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| `box-std`                                          | `.`, `├── src`, `│   ├── index.ts`, `│   └── util`, `│       └── a.ts`, `└── README.md` | `.` at 0 with children at 4 and 8 and 12; `README.md` at 4                                |
| `box-heavy`                                        | `.`, `┣━━ src`, `┃   ┗━━ a.ts`, `┗━━ README.md` (heavy variants)                        | `src` at 4 with `a.ts` at 8; `README.md` at 4                                             |
| `box-seg3`                                         | `├─ a`, `│  └─ b`                                                                       | segment width 3: `b` at depth 2                                                           |
| `box-col2-branch`                                  | `├── a`, `  └── b`                                                                      | `b` at column 2: `round(0.5) + 1 = 2`, so `b` is a child of `a`                           |
| `box-empty-node`                                   | `├── a`, `├── `                                                                         | error `Line 4: Empty node — …`                                                            |
| `box-plain-indented-error`                         | `├── a`, `    plain`                                                                    | error `Line 4: Unexpected indentation …`                                                  |
| `box-no-branch-with-box-char`                      | `Section ─ A.txt`, `└── b`                                                              | root item `Section ─ A.txt` with child `b`                                                |
| `box-plain-root-after`                             | `├── a`, `root2`                                                                        | `root2` is a root item                                                                    |
| `box-tab`                                          | `\t├── a`, `\t│\t└── b`                                                                 | tabs become 4 spaces: `a` at depth 2 under `.`, `b` at depth 3                            |
| `indent-with-box-char-in-name-then-plain-indented` | `src`, `    a ─ b.md`, `    c.txt`                                                      | error `Line 4: Unexpected indentation …` (box mode, then an indented plain line)          |
| `indent-branch-char-in-name`                       | `src`, `    b ├ c`                                                                      | the name is `c` at depth 2; `b` is lost (the text before the branch character is dropped) |
| `box-title`                                        | `title T`, `.`, `└── a`                                                                 | title `T`; the tree is unchanged                                                          |

The spec `boxDrawingPreprocessor.spec.ts` asserts the messages of the two thrown errors with `toThrow` at lines 382, 413 and 421, so their wording is part of the spec.

### 2.6 Errors

- A lexer error (a character no token can start, for example `"` that does not close, `:::` at the start of a line, `icon(` at the start of a line, `icon(a.b)`) and a parser error both throw `MermaidParseError` (`parse.ts:146-148`). The message is `Parsing failed: ` followed by the lexer errors and the parser errors, with their line and column (`parse.ts:156-173`). A missing location prints `?`, for example `Parse error on line ?, column ?` at the end of the text (probed, `quoted-trailing-spaces-eof`).
- Chevrotain's recovery is on, but Mermaid throws whenever a lexer or parser error is reported, so there is no partial tree.
- `decision 0002` says that the error messages of the Rust port are its own, and that specs that assert the Jison text are ported to assert failure and the position. The treeView spec asserts the preprocessor's messages (section 2.5), so those strings stay.

### 2.7 Quirks to reproduce, with test inputs

The "Probed" column names the probe case label in `cases-parse.json`, `cases-2.json` or `cases-title.json`, or a render probe in `render-cases.json`. The inputs are written as they appear after `treeView-beta` unless a row says otherwise.

| #    | Input                                                                                                                         | Reference result (probed)                                                                                                                   | Cause                                                                                                                                         |
| ---- | ----------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| Q1   | a top-level line `titlepage.md`                                                                                               | node `page.md`; title stays `""`                                                                                                            | `TITLE` (empty alternative) before `BARE_NAME` (grammar 17; `title-prefix-bare`)                                                              |
| Q1b  | a top-level line `title.txt`                                                                                                  | node `.txt` (`title-dot-bare`)                                                                                                              | same                                                                                                                                          |
| Q1c  | `src/`, then `    titles.ts`                                                                                                  | parse error (`title-prefix-nested`)                                                                                                         | `TITLE` takes the indentation too                                                                                                             |
| Q1d  | `a`, then `title X`                                                                                                           | parse error (`title-after-node`)                                                                                                            | `TITLE` only in `TitleAndAccessibilities`                                                                                                     |
| Q1e  | `title T`, then `titles.md`                                                                                                   | node `s.md`, and the title is `""`, not `T` (`title-prefix-after-title`); with frontmatter `FM` the title stays `FM` (`fm-and-titles-bare`) | as Q1: the AST keeps the last `TITLE` value, which is empty, and `populateCommonDb` does not set an empty title (`populateCommonDb.ts:11`)    |
| Q2   | `treeView-beta  ` (trailing spaces on the keyword line), or `treeView-beta foo`, or `treeView-beta %% c`, or `treeView-betaX` | parse error (`keyword-trailing-spaces`, `keyword-with-text`, `keyword-line-comment`, `keyword-glued`)                                       | `LONGER_ALT` of the keyword is `BARE_NAME`                                                                                                    |
| Q3   | `    treeView-beta` as a node                                                                                                 | parse error (`keyword-as-name`)                                                                                                             | the keyword token is not allowed as a name                                                                                                    |
| Q4   | `a`, `   `, `b` (a line with only spaces between them)                                                                        | `b` is a child of `a` at level 3 (`ws-only-line-between`)                                                                                   | `INDENTATION` is not hidden; the next node takes it                                                                                           |
| Q4b  | `a`, a line with a tab only, `b`                                                                                              | `b` is a child of `a` at level 1 (`tab-only-line-between`)                                                                                  | as Q4                                                                                                                                         |
| Q4c  | a line with spaces only, then `title T`                                                                                       | parse error (`ws-only-line-before-title`)                                                                                                   | `INDENTATION` before `TITLE`                                                                                                                  |
| Q5   | `a`, `    %%`, `    b` (an indented `%%` alone)                                                                               | parse error (`pct-pct-indented-alone`)                                                                                                      | `cleanupComments` keeps `%%` alone; `INDENTATION` then `ML_COMMENT`                                                                           |
| Q5b  | `a`, `%%`, `b` (unindented `%%` alone)                                                                                        | `a` and `b` are siblings (`pct-pct-alone`)                                                                                                  | no indentation token                                                                                                                          |
| Q5c  | `a`, `    %% note`, `    b`                                                                                                   | removed by the core; `b` is a child of `a` (`indented-comment-content`)                                                                     | `cleanupComments` (`comments.ts:7`)                                                                                                           |
| Q6   | `a %% note`, `b`                                                                                                              | a node named `a %% note` (`inline-comment-name`); `a %%` is `a %%` (`name-with-double-colon-only`)                                          | `%%` is not a comment inside a line                                                                                                           |
| Q7   | `"a"  ` (quoted, then spaces), with or without a next line                                                                    | parse error (`trailing-spaces-quoted`, `quoted-trailing-spaces-eof`)                                                                        | `INDENTATION` after a quoted name. Bare names keep their trailing spaces out of the name (`trailing-spaces-bare`, `bare-trailing-spaces-eof`) |
| Q8   | `"a"b"`                                                                                                                       | two nodes: `a` and `b"` (`quoted-inner-quote`)                                                                                              | `QUOTED_NAME` stops at the second quote; `BARE_NAME` then takes `b"`                                                                          |
| Q8b  | `"abc` (no closing quote)                                                                                                     | lexer error (`quoted-unterminated`)                                                                                                         | `BARE_NAME` cannot start with a quote                                                                                                         |
| Q8c  | `""`                                                                                                                          | a file named `""` (empty) (`quoted-empty`)                                                                                                  | empty quoted name                                                                                                                             |
| Q8d  | `"src/"`                                                                                                                      | a directory `src` (`quoted-dir`)                                                                                                            | `name.endsWith('/')` applies to every name (`parser.ts:20`)                                                                                   |
| Q9   | `/` (a bare slash)                                                                                                            | a directory with name `""` (`bare-slash`); its row has an empty label of height 0 (`P-root-slash`)                                          | as Q8d                                                                                                                                        |
| Q9b  | `a//`                                                                                                                         | a directory `a/` (`bare-double-slash`)                                                                                                      | one slash is removed                                                                                                                          |
| Q10  | `a :::1cls` and `a :::`                                                                                                       | names `a :::1cls` and `a :::` (`ann-class-digit`, `ann-class-only`)                                                                         | the class token needs a letter or `_` after `:::`                                                                                             |
| Q10b | `f :::a :::b` and `f icon(x) icon(y)`                                                                                         | class `b` and icon `y` (`ann-two-classes`, `ann-two-icons`)                                                                                 | a single-valued field assigned twice; the last wins                                                                                           |
| Q10c | `e ## desc :::cls` and `a ## b ## c`                                                                                          | descriptions `desc :::cls` and `b ## c` (`ann-desc-then-class`, `name-hash-in-middle`)                                                      | `DESC_ANNOTATION` takes the rest of the line                                                                                                  |
| Q11  | a line that starts with `:::x`, `icon(x)` or `## x`                                                                           | lexer error (`start-class`, `start-icon`, `start-desc`)                                                                                     | no name at the start, and the annotations need leading spaces                                                                                 |
| Q11b | `g icon(a.b)`, `l icon(a:b:c)`                                                                                                | lexer error (`ann-icon-dot`, `ann-icon-3part`)                                                                                              | `ICON_ANNOTATION` pattern (grammar 42)                                                                                                        |
| Q11c | `h icon()` and `i icon(none)`                                                                                                 | icon `none` for both (`ann-icon-empty`, `ann-icon-none`), hidden                                                                            | `parser.ts:33`                                                                                                                                |
| Q12  | `A #amp; B`                                                                                                                   | the DB name is `A ﬂ°amp¶ß B`; the SVG text is `A & B`; the layout uses the placeholder (`P-entity`, section 6.3)                            | `encodeEntities` before the parser; `decodeEntities` only on the final SVG                                                                    |
| Q13  | `title A & <b>B</b>` (the title line)                                                                                         | the DB title is `A &amp; <b>B</b>` (`title-sanitised`)                                                                                      | `sanitizeText` in the title setter (`commonDb.ts:28-30`); DOMPurify serialises text with `<`                                                  |
| Q13b | `a ## a & b` and `a ## <b>x</b>`                                                                                              | descriptions `a & b` and `<b>x</b>` (`desc-ampersand`, `desc-tag`)                                                                          | DOMPurify returns text without `<` unchanged; with `<` it keeps `<b>` and shows it as text                                                    |
| Q13c | `a <b>x</b>` (a name)                                                                                                         | the name is `a <b>x</b>` (`name-html`)                                                                                                      | names are not sanitised                                                                                                                       |
| Q14  | frontmatter `title: FM`, then `title Line`                                                                                    | title `Line` (`fm-and-line`)                                                                                                                | `populateCommonDb` runs after `Diagram.fromText` sets the frontmatter title                                                                   |
| Q14b | frontmatter `title: FM`, then `title` alone                                                                                   | title `FM` (`fm-and-empty-title-line`)                                                                                                      | an empty title is not set                                                                                                                     |
| Q15  | `a`, then `    b` (4 spaces), then `c` (no indent)                                                                            | `b` is a child of `a`; `c` is a root item (`q15-child-then-root`)                                                                           | the stack pops while `level <= top.level` (`db.ts:60-62`); a level only has to grow to make a child                                           |
| Q15a | `a`, `    b`, `        c`, `    d`                                                                                            | `c` is a child of `b`; `d` is a second child of `a` (`q15-deeper-then-middle`)                                                              | `d` (level 4) pops `c` and `b`                                                                                                                |
| Q15b | `a`, `  b`, ` c` (levels 0, 2, 1)                                                                                             | `c` is a child of `a`, not a sibling of `b` (`uneven-indent`)                                                                               | `c` (level 1) pops `b` (level 2); `a` (level 0) stays, so `c` is its child                                                                    |
| Q15c | `        a`, `    b`, `c` (the first node indented by 8)                                                                      | all three are root items (`first-node-indented`)                                                                                            | the stack starts with the root at level -1                                                                                                    |
| Q15d | tabs: `\ta`, `    b` (tab is 1, spaces are 4)                                                                                 | `b` is a child of `a` (`tab-indent`)                                                                                                        | a tab counts as one character                                                                                                                 |
| Q16  | box mode with a branch character inside a name: `    b ├ c` after a `src` node                                                | `c` at depth 2; `b` is lost (`indent-branch-char-in-name`)                                                                                  | `boxDrawingPreprocessor.ts:166-173` takes the text after the branch character                                                                 |
| Q17  | `frontmatter` with `config: treeView: rowIndent: 20`, or `%%{init}%%` with `showIcons`, or `themeVariables.treeView`          | config is merged; the frontmatter and the directive work (`frontmatter-config`, `directive-config`)                                         | `Diagram.fromText` and `preprocessDiagram`; see section 6.8 for the directive and `themeVariables`                                            |
| Q18  | a `title` line between the keyword and a box-drawing tree (`treeView-beta`, `title T`, `.`, `└── a`)                          | the title is set and the tree is unchanged (`box-title`)                                                                                    | `METADATA_LINE` passes it through (`boxDrawingPreprocessor.ts:13`, `142-146`)                                                                 |
| Q19  | a name that contains a space run: `ÉÜ  double  space`                                                                         | the name keeps both runs (`unicode-name`)                                                                                                   | `BARE_NAME` keeps inner runs; `white-space: pre` in the render (section 6.3)                                                                  |

Other points, probed or read:

- The detector matches `treeView-beta` at the start of the text after leading spaces (`detector.ts:7`). `no-keyword` (`a` then `b`) is a parse error (`Expecting token of type 'treeView-beta'`).
- An empty tree (`treeView-beta` alone, or with a comment) parses to no nodes (`empty-tree`, `only-comment`, `empty-tree-nl`). `getCount()` is 1 (the root).
- `getCount()` is the number of nodes plus one, because the counter starts at 1 (`db.ts:23`) and each node takes the next number (`db.ts:64`). The root is 0 and is not counted.
- Blank lines between nodes are harmless (`blank-lines`). CRLF is handled by `cleanupText` (`crlf`).
- `class-hyphen` (`a :::my-cls1`) gives `my-cls1`. `dir-with-class-and-icon` (`src/ :::hl icon(folder)`) gives both.
- Multiple `title` lines: the last one wins, and an empty last one wins too, with the result `""` (`title-twice`, `title-empty`).

## 3. DB

### 3.1 Calls in order

For a diagram, `Diagram.fromText` makes these calls (section 2.4 for the core side):

1. `db.clear()`: `state.reset()` and the common `clear` (`db.ts:35-38`). The common clear resets the title, the accessible title and the accessible description (`commonDb.ts:10-14`).
2. `db.setDiagramTitle(metadata.title)` if the frontmatter has a title (`Diagram.ts:52-54`).
3. `parser.parse(text)`: `preprocessBoxDrawing`, then the grammar, then `populate` (`parser.ts:43-57`).
4. `populate` (`parser.ts:11-41`): `populateCommonDb(ast, db)` (`populateCommonDb.ts:4-14`: each of `accDescr`, `accTitle`, `title` is set only when the AST value is truthy), then one `db.addNode` for each node in source order (`parser.ts:13-40`). The AST's `nodes` is flat: it holds every node line, whatever its depth, and the depth is only in `indent`.
5. `draw` (`renderer.ts:218-236`): `db.getRoot()`, `db.getConfig()`.

The DB is module state (`ImperativeState`, `db.ts:22`, `utils/imperativeState.ts:24-37`). `clear` replaces it. The port needs no global; one value per parse is enough.

### 3.2 `addNode` (`db.ts:52-75`) step by step

1. `while (level <= stack.top.level) stack.pop()` (lines 60-62). The stack starts with the synthetic root at level -1 (lines 24-31), so the root is never popped (levels are at least 0).
2. The node takes the next counter value as `id` (`cnt++`, line 64). The counter starts at 1 (line 23).
3. The node is appended to `stack.top.children` and pushed (lines 73-74).

The effect is that the parent of a node is the nearest node above it with a smaller level. A node whose level equals a previous sibling's level pops it. Levels are compared as numbers, so any increase makes a child (Q15, Q15b).

`parser.ts:14` takes the level from `node.indent` (`typeof === 'number' ? … : 0`). `parser.ts:17` takes the name. `parser.ts:20-24` strips one trailing `/` and sets `nodeType` to `directory`, otherwise `file`. `parser.ts:27` sets `cssClass` from `classAnnotation` (`undefined` when empty). `parser.ts:32-33` sets `icon`: `undefined` when there is no annotation, `none` when the annotation is `""`, otherwise the name. `parser.ts:36-37` sets `description` through `sanitizeText(rawDesc, getConfig())` (`undefined` when empty).

### 3.3 Sanitising

| Text                                  | Sanitised                                  | Where                         | Probed                                                               |
| ------------------------------------- | ------------------------------------------ | ----------------------------- | -------------------------------------------------------------------- |
| diagram title                         | yes, `sanitizeText`                        | `commonDb.ts:28-30`           | Q13                                                                  |
| accessible title                      | yes                                        | `commonDb.ts:16-18`           | `accTitle-sanitised`: `A <i>x</i>` kept                              |
| accessible description                | yes, then `\n\s+` becomes `\n`             | `commonDb.ts:22-24`           | not probed                                                           |
| node description                      | yes                                        | `parser.ts:37`                | Q13b                                                                 |
| node name                             | no                                         | `parser.ts:17`                | Q13c; the renderer sets it with `.text`, so markup shows as text     |
| class name                            | no                                         | `parser.ts:27`                | not applicable: the grammar allows only letters, digits, `_` and `-` |
| icon name                             | no                                         | `parser.ts:32-33`             | the name is used in a registry lookup, not in markup                 |
| icon SVG (the output of `getIconSVG`) | yes, `sanitizeText` with the global config | `rendering-util/icons.ts:105` | the output is the sanitised SVG string                               |

`sanitizeText` (`common.ts:71-83`) calls `sanitizeMore` first (`common.ts:56-69`, which does work only when HTML labels are effective, and `htmlLabels` is false in the harness), then DOMPurify with `FORBID_TAGS: ['style']` (or the configured `dompurifyConfig`). DOMPurify returns text without `<` unchanged (probed: `a & b`, `a &amp; b`). It serialises markup (probed: `<b>x</b>` is kept). Script tags and event handlers are removed by DOMPurify's default rules (read, not probed). The canonical model's sanitising rules (`docs-port/canonical-model.md`, Sanitisation) and `ferrigura-syntax`'s `sanitize.rs` cover this.

### 3.4 Defaults and `clear`

| Field                           | Default after `clear`                                                                             | Source                                              |
| ------------------------------- | ------------------------------------------------------------------------------------------------- | --------------------------------------------------- |
| `cnt`                           | 1                                                                                                 | `db.ts:23`                                          |
| `stack`                         | `[root]` where the root is `{ id: 0, level: -1, name: '/', nodeType: 'directory', children: [] }` | `db.ts:24-31`                                       |
| `title`, `accTitle`, `accDescr` | `""`                                                                                              | `commonDb.ts:4-6` and `clear` (`commonDb.ts:10-14`) |

`clear` resets everything above. It does not reset the config, which the DB reads from the common configuration at each call of `getConfig` (`db.ts:48-50`).

### 3.5 Quirks of the DB

- The root `/` is part of every tree and is drawn (section 6.11, R1). `getRoot` returns it (`db.ts:40-42`).
- `getCount()` is nodes plus one (section 2.7).
- The renderer writes `node.BBox` into the DB's nodes (`renderer.ts:95`). The port must not model it, and a second render of the same DB overwrites it.
- Frontmatter title first, then the `title` line when it is not empty (Q14, Q14b).

## 4. Canonical model proposal

The model the differential compares is the DB after a parse, in the camelCase form of `docs-port/canonical-model.md`. The shape:

```json
{
  "version": 1,
  "title": "",
  "accTitle": "",
  "accDescr": "",
  "count": 2,
  "root": {
    "id": 0,
    "level": -1,
    "name": "/",
    "nodeType": "directory",
    "icon": "",
    "cssClass": "",
    "description": "",
    "children": [
      {
        "id": 1,
        "level": 0,
        "name": "root",
        "nodeType": "directory",
        "icon": "",
        "cssClass": "",
        "description": "",
        "children": []
      }
    ]
  }
}
```

Rules:

- Strings are always present. An absent title is `""` (as the DB holds it). For the node fields, an absent value is `""` as well (the mindmap's rule, `canonical-model.md:197`): `icon` is `""` when there is no annotation, `cssClass` is `""` when there is no class, `description` is `""` when there is none. This is safe, because the grammar never gives an empty class, description or icon name (the empty icon becomes `none`, `parser.ts:33`).
- `icon` keeps the three states of the DB: `""` (absent), `"none"` (suppressed) and the name as written (`"fa:bell"`, `"folder"`).
- `name` is the grammar's value with the entity placeholders kept (Q12). The directory suffix is already removed, and `nodeType` says which it was.
- `description` is the sanitised text (section 3.3).
- `id` values are the counter's; the root is 0.
- `count` is `getCount()`. It is in the model because the public DB has it.
- Left out: `BBox` (set by the renderer, `renderer.ts:95`), the configuration (checked by `port:diff config`, as the canonical model's "Left out" section says), the directives, the frontmatter, and the source positions.

## 5. Specs

| File                             | `it` calls                                                         | What it tests                                                                                                                                                                                                                                                                                              | Which port test takes it                                            |
| -------------------------------- | ------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| `parser.spec.ts`                 | 33 in 11 `describe` blocks, plus the box-drawing equivalence block | the grammar, the value converter, `populate` and the DB, through `parser.parse` and `db`: bare and quoted labels, `:::`, `icon()`, `##`, combined annotations, spaces, the default icon rule (`parser.spec.ts:197`), icon suppression, and the same tree from box-drawing and indent input (lines 270-380) | the scanner, the parser and the DB, through the parser's public API |
| `db.spec.ts`                     | 18 in 7 `describe` blocks                                          | `clear`, `getRoot`, `getCount`, `addNode` (levels, siblings, deep trees, ids), `getConfig` fields, the accessible and title setters                                                                                                                                                                        | the DB                                                              |
| `icons.spec.ts`                  | 23 in 5 `describe` blocks                                          | `treeViewIcons` (prefix, the two icons, `currentColor`), `detectIcon` (filename, extension, case, dotfiles, multi-dot names), `getNodeIcon` (none, prefixed, `defaultIconPack`, `showIcons`, the maps)                                                                                                     | the icon functions                                                  |
| `boxDrawingPreprocessor.spec.ts` | 42 in 16 `describe` blocks                                         | `isBoxDrawingFormat`, `preprocessBoxDrawing` (trees, flexible prefixes, heavy variants, annotations, comments, metadata, decoration, line mapping, the error messages at lines 382, 413, 421, tabs), `remapErrorLines`                                                                                     | the preprocessor, with its messages                                 |

Not covered by any spec: the renderer (`renderer.ts`), the styles (`styles.ts`), and the layout arithmetic. The only renderer coverage is the 15 fixtures, through the harness. The port needs its own render tests for the probed numbers of section 6, which the probe outputs provide.

The specs use `vitest`. They were counted, not run, in this task.

## 6. Renderer

### 6.1 Pipeline

`draw` (`renderer.ts:218-236`):

1. `log.debug`, then `db = diagObj.db`, `root = db.getRoot()`, `config = db.getConfig()` (lines 219-223).
2. `svg = selectSvgElement(id)` (line 225): `select('body')`, then `#id`; in sandbox mode the iframe's document (`selectSvgElement.ts:11-22`).
3. `treeElem = svg.append('g')` with class `tree-view` (lines 227-228). The core has already appended an empty `<g>` to the svg (`mermaidAPI.ts:419`).
4. `iconSVGs = await resolveNodeIcons(root, config)` (line 230): every node with an icon, in parallel, through `getIconSVG` (lines 28-50).
5. `drawTree(treeElem, root, config, iconSVGs)` (line 231, lines 130-216). It returns `totalHeight` and `totalWidth`.
6. `svg.attr('viewBox', '-' + lineThickness/2 + ' 0 ' + totalWidth + ' ' + totalHeight)` (lines 232-234).
7. `configureSvgSize(svg, totalHeight, totalWidth, useMaxWidth)` (line 235; section 6.6).

`drawTree` (lines 130-216):

- `processNode(root)` (line 182), recursive. For each node: `drawNode` (lines 140-161) draws the row; then the children, in order (lines 163-167); then the trunk line from the node's row (lines 168-179).
- Phase 2 (lines 184-200): the descriptions, in a second pass over all rows.
- Phase 3 (lines 202-213): the highlight rectangles, sized last.

### 6.2 Icons

The built-in pack is `mermaid-treeview` (`icons.ts:16`, `renderer.ts:10-15`), with two icons, both 24 by 24 and drawn with `currentColor` (`icons.ts:21`, `icons.ts:24`). It is registered at module load. The renderer gets an icon SVG from `getIconSVG(name, {height: 14, width: 14})` (`renderer.ts:42-45`).

`getNodeIcon` (`icons.ts:77-100`) decides the icon of a node:

| Case                                                                                                                                                                                   | Result                                                                                                                                                                                                                               | Lines        |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------ |
| `icon === 'none'` (from `icon()` or `icon(none)`)                                                                                                                                      | no icon                                                                                                                                                                                                                              | 81-83        |
| an explicit `icon` (any other value)                                                                                                                                                   | `qualifyIcon(icon, defaultIconPack)`: a name with `:` as is; a built-in name (`folder`, `file`) gets the built-in prefix even when `defaultIconPack` is set; another name gets `defaultIconPack:` when set, else the built-in prefix | 84-86, 58-66 |
| no explicit icon, `showIcons` false                                                                                                                                                    | no icon                                                                                                                                                                                                                              | 87-89        |
| file, `showIcons` true, the name matches `filenameIcons` (exact) or `extensionIcons` (the extension lower-cased, with or without the dot; the last dot counts, and a dotfile has none) | the mapped value; `none` hides it; otherwise `qualifyIcon`                                                                                                                                                                           | 90-97, 43-55 |
| anything else with `showIcons` true                                                                                                                                                    | the built-in `folder` for a directory (`mermaid-treeview:folder`), the built-in `file` for a file                                                                                                                                    | 99           |

`getIconSVG` (`rendering-util/icons.ts:88-106`) splits the reference, looks the pack up in the registry, and returns the body as an SVG string. Lookups that fail (an unknown pack, a missing icon, an invalid name) log and use `unknownIcon` (`rendering-util/icons.ts:20-24`, catch at `94-99`). That icon is a blue (`#087ebf`) square with a white `?` in `ArialMT, Arial` at 67.75 px, 80 by 80. The body is wrapped by `iconToSVG` and `iconToHTML` into `<svg xmlns width height viewBox>` (probed, `P-showIcons`: `width="14" height="14" viewBox="0 0 24 24"`; `P-unknown-icon-nopack`: `viewBox="0 0 80 80"`). The string goes through `sanitizeText` (`rendering-util/icons.ts:105`).

The renderer inserts the string with `.html()` into a `g.treeView-node-icon` group translated to `(x + paddingX, y + paddingY)` (`renderer.ts:73-79`).

Probed: the built-in icons and the unknown icon of the fixtures are the ones above. The fixtures with `fa:bell`, `bell` under `defaultIconPack: fa`, `typescript` under `material-icon-theme`, and `unregistered:icon` all render the unknown icon, because no pack of those names is registered (section 7). The `folder` and `file` icons are used in the fixtures that use `showIcons` (section 7).

Randomness (read, not probed): `@iconify/utils` `replaceIDs` computes a random prefix at module load (`svg/id.js:23`), and draws more random numbers and the clock only when an icon body has `id="…"` (`id.js:31-44`). None of the built-in or unknown icon bodies has an id. A registered pack with ids would make a render random, which the reference does not control (section 6.10).

Built-in symbols and the notes of other renderers: the built-in symbols are the two bodies at `icons.ts:21` and `icons.ts:24` (`folder` and `file`), and nothing else. A search of `docs-port/` for tree view, treeView, folder, symbol, built-in icon and compat finds no compatibility note that mentions tree view symbols. The grep for `<symbol` finds only the C4 notes (`docs-port/c4/NOTES.md` lines 500-502) and the sequence notes (`docs-port/sequence/NOTES.md` line 584), which describe the C4 shapes' own symbols, not tree view icons.

### 6.3 Measurement

The renderer measures with `getBBox` in three places:

| Call                  | Lines             | What it measures                                                                                                                                                                                                       |
| --------------------- | ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| label `getBBox`       | `renderer.ts:87`  | the label's height and width. The text has the class `treeView-node-label` (bold for `treeView-node-dir`), and `font-size: labelFontSize` (16 px by default) and `white-space: pre` from `styles.ts:30-34` and `35-37` |
| description `getBBox` | `renderer.ts:197` | the description's width. The class `treeView-node-description`: italic, `labelFontSize`, `white-space: pre` (`styles.ts:44-49`)                                                                                        |

The label is measured before its `x` and `y` are set (`renderer.ts:82-87`). Position does not change either size.

Measured values (probed, `P-root-one` and the others):

- **Height.** The label's height is 20 px for 16 px Lexica Ultralegible, in regular and bold (probed on labels). The italic description's height is not used by the layout, so it is not probed here. The row height is `20 + 2 * 5 = 30`. Every probed row is 30 px high, and the viewBox heights are multiples of 30 (for example 60 for two rows, 120 for four). The outlines re-measure the height as 20.0052 in one case (`P-desc2`), which is the re-insertion of the SVG, not the render (see the note at the top).
- **Widths.** Examples from the viewBoxes, as `labelWidth = viewBoxWidth − indent − 2 * paddingX`: `a` 8.421875, `/` (bold) 7.078125, `src` 20.984375, `src/` (bold) 22.828125, `A  B` (two spaces) 28.890625, `A B` 24.40625, `A_B` 26.265625. The values are multiples of 1/64 px, as decision [0005](../decisions/0005-text-measurement-and-fonts.md) found. One exception was seen: the italic description `x` in `P-desc2` gives a width of 9.088005…, which is not a multiple of 1/64. The cause is unverified.
- **Empty label.** The label of an empty name has a box of 0 by 0 (probed, `P-root-slash`). Its row height is `0 + 10 = 10`.
- **Consecutive spaces.** `white-space: pre` keeps them, and the width grows with each space (probed, `Q-two-spaces`).
- **Entities.** The measured text is the placeholder. For `A #amp; B` the viewBox is 116.921875 wide, which gives a label width of 91.921875 (`116.921875 − 15 − 10`). The text `A & B` measures 40.171875 (`P-entity-literal-amp`: 65.171875 − 15 − 10). So the box is sized for `A ﬂ°amp¶ß B`, and the drawn text is `A & B` (`mermaidAPI.ts:359`).
- **Icon text.** The unknown icon's `?` is measured as a `<text>` inside the icon's own `svg` (probed `P-unknown-icon-nopack`: its box is `[0, -62.8571, 37.6786, 80]` in icon units, because `getBBox` is in the icon's coordinates). The icon's `getBBox` is not used for layout (`renderer.ts` never measures the icon), so the question mark affects the layout only through the harness's comparison.

The tree view always draws its labels as SVG `text`. It never reads `htmlLabels`, and its own styles have no HTML rules. Decision [0004](../decisions/0004-svg-text-labels.md) covers SVG text labels in general.

### 6.4 Layout arithmetic

Constants: `rowIndent` 10, `paddingX` 5, `paddingY` 5, `lineThickness` 1 (section 6.8); `ICON_SIZE` 14, `ICON_GAP` 4, `DESC_GAP` 16 (`renderer.ts:17-19`). `iconOffset = ICON_SIZE + ICON_GAP = 18` (line 70).

For a node at depth `d` (the root is depth 0), with `totalHeight` the height so far (the sum of the rows drawn before it):

```
indent      = d * (rowIndent + paddingX)                       renderer.ts:146   (15 per level)
x, y        = indent, totalHeight                              renderer.ts:147
showIcon    = getNodeIcon(node) !== undefined                  renderer.ts:71-72
labelX      = x + paddingX + (showIcon ? 18 : 0)               renderer.ts:89
labelHeight = getBBox(label).height
height      = labelHeight + 2 * paddingY                       renderer.ts:88
label.y     = y + height / 2                                   renderer.ts:91
labelWidth  = getBBox(label).width
labelRight  = labelX + labelWidth                              renderer.ts:93
width       = labelWidth + 2 * paddingX + (showIcon ? 18 : 0)  renderer.ts:94
BBox        = { x, y, width, height }                          renderer.ts:95
icon group  = translate(x + paddingX, y + paddingY), size 14   renderer.ts:77
connector   = line (indent − rowIndent, cy) to (indent, cy)    renderer.ts:150-157, cy = y + height/2
totalWidth  = max(totalWidth, indent + width)                  renderer.ts:159
totalHeight += height                                          renderer.ts:160
```

After the children of a node, its trunk (renderer.ts:168-179): from `(x + paddingX, y + height)` to `(x + paddingX, endY + endHeight/2 + lineThickness/2)`, where `endY` and `endHeight` are the last child's `BBox`. Its stroke width is `lineThickness`.

Phase 2, descriptions (renderer.ts:184-200): when at least one node has a description, `descX = max(labelRight over all rows) + 16`. The column is the same for every row, including rows without a description. Each description is a text at `(descX, cy)`, and `totalWidth = max(totalWidth, descX + descWidth + paddingX)`.

Phase 3, highlights (renderer.ts:202-213): for each row in order, when its class list has `highlight` (`node.cssClass.split(/\s+/).includes('highlight')`, line 204), the rectangle is sized `rectWidth = totalWidth − BBox.x + 8`, then `totalWidth = max(totalWidth, BBox.x + rectWidth + 2)`. Since `BBox.x + rectWidth + 2 = totalWidth + 10`, every highlighted row adds 10 to `totalWidth`, and each later rectangle is sized from the already-grown value. This order dependence is the most likely thing to get wrong.

Viewbox and size (renderer.ts:232-235, section 6.6): `viewBox = (-lineThickness/2) 0 totalWidth totalHeight`.

The rectangle's attributes (renderer.ts:99-107 and 205-209): `x = BBox.x`, `y = BBox.y + 1`, `width` from phase 3, `height = height − 2`, `rx = 3`, class `treeView-highlight-bg`. It is inserted as the first child of the row's group (`insert('rect', ':first-child')`, line 100).

Probed examples:

- `P-root-one` (`treeView-beta` then `a`): the viewBox is `-0.5 0 33.421875 60`. Row `/` at `y = 0`: label `x = 5`, `y = 15`. Row `a` at `y = 30`: label `x = 20`, `y = 45`, connector `(5, 45)` to `(15, 45)`. Trunk of `/`: `x = 5`, from `y = 30` to `45.5`. Root connector: `(-10, 15)` to `(0, 15)`.
- `P-highlight2` (`a :::highlight`, `b :::highlight`, `c`): the viewBox is `54.0625` wide. The rectangles are 27.0625 and 37.0625 wide. The first gives `34.0625 − 15 + 8`; the second `44.0625 − 15 + 8`. The plain tree is 34.0625 wide.
- `P-desc2` (`a ## x`, `longname.txt`, `    b ## y`): `descX = 110.21875 + 16 = 126.21875` for both descriptions, the viewBox width is 140.3067…, which is `126.21875 + 9.088005 + 5`.

### 6.5 DOM, in order

The probe of `P-nest` (`root/` with children `a` and `sub/`, which has `c`) gives this structure, with the attributes that matter:

```
<svg id width="100%" xmlns viewBox="-0.5 0 67.203125 150" style="max-width: 67.203125px;" role aria-roledescription>
  <g></g>                                         core, mermaidAPI.ts:419
  <g class="tree-view">
    <g>                                           row "/"  (depth 0)
      <text class="treeView-node-label treeView-node-dir" x=5 y=15 dominant-baseline=middle>/</text>
    <line x1=-10 y1=15 x2=0 y2=15 class="treeView-node-line" stroke-width=1>
    <g>                                           row "root/"  (depth 1)
      <text class="… treeView-node-dir" x=20 y=45>root</text>
    <line x1=5 y1=45 x2=15 y2=45>
    <g> <text x=35 y=75>a</text> </g>             row "a"  (depth 2)
    <line x1=20 y1=75 x2=30 y2=75>
    <g> <text class="… treeView-node-dir" x=35 y=105>sub</text> </g>
    <line x1=20 y1=105 x2=30 y2=105>
    <g> <text x=50 y=135>c</text> </g>
    <line x1=35 y1=135 x2=45 y2=135>
    <line x1=35 y1=120 x2=35 y2=135.5>           trunk of "sub"
    <line x1=20 y1=60 x2=20 y2=105.5>            trunk of "root"
    <line x1=5 y1=30 x2=5 y2=45.5>               trunk of "/"
  </g>
</svg>
```

Order: each row's group, then its connector line, then its children, then its trunk. The core's `<style>` and the accessibility `<title>` and `<desc>` come before the empty `<g>`, and `<title>` and `<desc>` are the first children when set (probed, `P-title-acc`). A row with an icon has the icon group before the label. A highlighted row has its rectangle first. A row with a description has the description text after the label, in the same group.

Probed: the `title` line (`title My Tree`) is stored and never drawn. The string `My Tree` is not in the SVG (`P-title-acc`). The accessible title (`T1`) is in `<title>`.

### 6.6 Sizing of the root `svg`

`configureSvgSize` (`setupGraphViewbox.js:43-46`, `calculateSvgSizeAttrs` at 23-33):

- `useMaxWidth` true (the default): `width="100%"` and `style="max-width: {totalWidth}px;"`. No `height` attribute (probed, `P-root-one`).
- `useMaxWidth` false: `height="{totalHeight}"` and `width="{totalWidth}"` (probed, `P-useMaxWidth-false`: `width="33.421875" height="60"`, no style).
- The viewBox is set in `draw` (`renderer.ts:234`), with `-lineThickness/2` as its x.
- The core adds `role="graphics-document document"`, `aria-roledescription="treeView"`, and `aria-labelledby` and `aria-describedby` when titles are set (`mermaidAPI.ts:623`; probed).

The empty diagram has the root row alone, so the viewBox is `-0.5 0 17.078125 30` (`P-empty`).

### 6.7 Style sheet (`styles.ts`)

The tree view's sheet (`diagrams/treeView/styles.ts:29-55`) is appended to the global sheet, which `getStyles` builds in `packages/mermaid/src/styles.ts:11-164`. Line 38 of that file calls the diagram's own style function with `{ ...options, svgId }`. `createUserStyles` (`mermaidAPI.ts:327-333`) passes `{ ...config.themeVariables, theme, look }` (line 330). The tree view reads only `options.treeView` (`diagrams/treeView/styles.ts:16-18`, `28`).

Rules, in order: `.treeView-node-label` (font size, fill, `white-space: pre`), `.treeView-node-dir` (bold), `.treeView-node-line` (stroke), `.treeView-node-icon` (colour, which the icons' `currentColor` picks up), `.treeView-node-description` (font size, fill, italic, `white-space: pre`), `.treeView-highlight-bg` (fill, stroke, width 1).

Probed: the compiled sheet of each render is prefixed with `#<svg id>` for every rule, except the first one, `.treeView-node-label`, which is emitted without the prefix (`P-root-one`, `P-theme-vars`, fixture `custom-config`). The cause is in the core's CSS compilation (`compileCSS` at `styles.ts:226`, called from `createUserStyles` at `mermaidAPI.ts:333`), not in the tree view. The first rule of the tree view's text is the one that loses the prefix (`styles.ts:30`). The geometry harness does not compare CSS text. Unverified: which core stage produces the unscoped rule.

### 6.8 Configuration

All keys are read through `db.getConfig()`, which merges `DEFAULT_CONFIG.treeView` (`db.ts:46-50`, `defaultConfig.ts:285-288`) with the common `treeView` section (`cleanAndMerge`, `utils.ts:841-843`). The defaults are the schema's (`config.schema.yaml:2749-2808`). `useMaxWidth` is the base diagram default (`config.schema.yaml:464-468`), probed as `true` in `treeView` (`no-config` probe).

| Key               | Default | Schema                         | Read at (file:line)                                                        | Use                                            |
| ----------------- | ------- | ------------------------------ | -------------------------------------------------------------------------- | ---------------------------------------------- |
| `rowIndent`       | 10      | `config.schema.yaml:2749-2753` | `renderer.ts:146`, `152`                                                   | indentation per level; the connector length    |
| `paddingX`        | 5       | `2754-2758`                    | `renderer.ts:77`, `89`, `94`, `146`, `173`, `175`, `198`                   | horizontal padding; trunk x; description total |
| `paddingY`        | 5       | `2759-2763`                    | `renderer.ts:77`, `88`                                                     | vertical padding; row height; icon y           |
| `lineThickness`   | 1       | `2764-2768`                    | `renderer.ts:156`, `176`, `177`, `234` (and the parameter at `118`, `126`) | stroke width; trunk end; viewBox x             |
| `showIcons`       | false   | `2769-2774`                    | `icons.ts:87` (through `getNodeIcon`, called at `renderer.ts:31`, `71`)    | default icons on                               |
| `defaultIconPack` | `''`    | `2775-2783`                    | `icons.ts:85`, `96`, `58-66`                                               | pack of unprefixed names                       |
| `filenameIcons`   | `{}`    | `2784-2795`                    | `icons.ts:44` (through `detectIcon`, line 91)                              | exact file names                               |
| `extensionIcons`  | `{}`    | `2796-2808`                    | `icons.ts:51-52`                                                           | extensions                                     |
| `useMaxWidth`     | true    | base `464-468`                 | `renderer.ts:235`                                                          | the size attributes (section 6.6)              |

The type `TreeViewDiagramConfig` (`config.type.ts:2188-2245`) has the same keys and `BaseDiagramConfig` for `useMaxWidth`. `useWidth` is set to `undefined` in the defaults (`defaultConfig.ts:287`), and nothing reads it.

Global settings the tree view reads, outside the `treeView` section:

| Setting                                                                  | Read at                                 | Use                                                               |
| ------------------------------------------------------------------------ | --------------------------------------- | ----------------------------------------------------------------- |
| `securityLevel`                                                          | `rendering-util/selectSvgElement.ts:12` | the sandbox document (not used by the harness)                    |
| `htmlLabels` (effective)                                                 | `diagrams/common/common.ts:57`          | `sanitizeMore` only                                               |
| `dompurifyConfig`                                                        | `diagrams/common/common.ts:75`          | the DOMPurify config (default: `FORBID_TAGS: ['style']`, line 79) |
| `theme`, `look`, `themeVariables`                                        | `mermaidAPI.ts:330`                     | the style options (`styles.ts:38`)                                |
| `fontFamily`, `fontSize`, `textColor` (top-level, from `themeVariables`) | `styles.ts:43-45` (the global sheet)    | the diagram's root font and fill, not tree specific               |

Directive path, probed: `%%{init: {"treeView": {"showIcons": true, "paddingX": 7}}}%%` sets both values (`directive-config`). The frontmatter path works too (`frontmatter-config`).

### 6.9 Theme variables (`themeVariables.treeView`)

`styles.ts` reads one object, `treeView`, from the style options, which are `config.themeVariables`. The values are strings, and a missing key takes its default (`styles.ts:20-28`, `cleanAndMerge` at line 28).

| Key                | Default                     | Default at     | Read at                    | Used for                        |
| ------------------ | --------------------------- | -------------- | -------------------------- | ------------------------------- |
| `labelFontSize`    | `'16px'`                    | `styles.ts:6`  | `styles.ts:21`, `31`, `45` | label and description font size |
| `labelColor`       | `'black'`                   | `styles.ts:7`  | `styles.ts:22`, `32`       | label fill                      |
| `lineColor`        | `'black'`                   | `styles.ts:8`  | `styles.ts:23`, `39`       | connector and trunk stroke      |
| `iconColor`        | `'#546e7a'`                 | `styles.ts:9`  | `styles.ts:24`, `42`       | `currentColor` of the icons     |
| `descriptionColor` | `'#6a9955'`                 | `styles.ts:10` | `styles.ts:25`, `46`       | description fill                |
| `highlightBg`      | `'rgba(255, 193, 7, 0.15)'` | `styles.ts:11` | `styles.ts:26`, `51`       | highlight rectangle fill        |
| `highlightStroke`  | `'#ffc107'`                 | `styles.ts:12` | `styles.ts:27`, `52`       | highlight rectangle stroke      |

None of the seven is in the shipped themes (`themes/` has no `treeView` key), and none is in the typed theme tokens of [0007](../decisions/0007-themes-as-css-variables.md). They are user overrides with defaults in `styles.ts`.

Probed: the frontmatter form (`config: themeVariables: treeView: labelFontSize: '20px', labelColor: '#FF0000', lineColor: '#00FF00'`, fixture `should-render-with-custom-config-and-quoted-labels`) changes the sheet as expected. The init-directive form (`%%{init: {"themeVariables": {"treeView": {...}}}}%%`) did not change the sheet in the probe (`P-theme-vars`: defaults remained). The cause is unverified, and the config diff (`port:diff config`) should decide the correct behaviour for `themeVariables` in directives.

### 6.10 Randomness, the clock, and DOM dependencies

- **Randomness and the clock.** None in the tree view's own code. The only random source on this path is in `@iconify/utils` (`svg/id.js:23`), and it is used only when an icon body has ids (section 6.2). The built-in and unknown bodies have none, so a render with the default icons is deterministic.
- **DOM dependencies a server renderer must replace.**
  - `getBBox` of the label (`renderer.ts:87`) and of the description (`renderer.ts:197`): text measurement (section 6.3).
  - `select('body')` and the sandbox document (`selectSvgElement.ts:14-18`): not needed for a server renderer.
  - `.html(svgString)` for the icon group (`renderer.ts:78`): the port emits the string as written, after `sanitizeText` (`icons.ts:105`).
  - `insert(':first-child')` (`renderer.ts:100`), `select(...)` and `empty()` (`renderer.ts:205-206`): tree order only.
  - `getBBox` of the root svg is not used by the tree view (`setupGraphViewbox` is not called; `renderer.ts:235` uses `configureSvgSize`).
- **The DB object is mutated by the renderer** (`renderer.ts:95`). The port keeps the layout in its own structure.

### 6.11 Renderer quirks to reproduce

| #   | Behaviour                                                                                                                                                                                                              | Where                                                         | Probed                                                                      |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- | --------------------------------------------------------------------------- |
| R1  | The root row `/` is always drawn: bold (`treeView-node-dir`), the folder icon with `showIcons`, and the top-level nodes at depth 1. An empty diagram is one row, viewBox `-0.5 0 17.078125 30`                         | `renderer.ts:182`, `222`                                      | `P-root-one`, `P-empty`, `P-showIcons`                                      |
| R2  | A connector starts at `indent − rowIndent`, so the connector of a root-level row runs from x = −10 to 0, and only the part inside the viewBox (from −0.5) is visible                                                   | `renderer.ts:150-157`                                         | `P-root-one` (`x1="-10"`)                                                   |
| R3  | The icon group is top-aligned at `(x + paddingX, y + paddingY)`, not centred: its centre is 3 px above the label's centre                                                                                              | `renderer.ts:73-79`                                           | `P-showIcons` (`translate(5, 5)` in a 30 px row)                            |
| R4  | Each highlighted row adds 10 to `totalWidth`, and the rectangles' widths depend on the rows before them                                                                                                                | `renderer.ts:203-213`                                         | `P-highlight1`, `P-highlight2`                                              |
| R5  | The descriptions share one column for all rows: the largest label right edge + 16                                                                                                                                      | `renderer.ts:185-188`                                         | `P-desc2`, `P-desc-amp` (x = 44.421875)                                     |
| R6  | The viewBox starts at −lineThickness/2                                                                                                                                                                                 | `renderer.ts:232-234`                                         | fixture `custom-config` (−1.5)                                              |
| R7  | With `useMaxWidth` false the svg has `width` and `height` attributes and no `style`                                                                                                                                    | `setupGraphViewbox.js:25-31`                                  | `P-useMaxWidth-false`                                                       |
| R8  | An empty label measures 0 high, so its row is `2 * paddingY` high                                                                                                                                                      | `renderer.ts:87-88`                                           | `P-root-slash` (viewBox height 40 for two rows)                             |
| R9  | Every icon the built-in pack does not have (`fa:`, `material-icon-theme:`, `unregistered:`, a `defaultIconPack` that is not registered) draws the 80 by 80 unknown icon: blue `#087ebf`, white `?` in `ArialMT, Arial` | `rendering-util/icons.ts:20-24`, `94-99`                      | `P-unknown-icon-nopack`, `P-unknown-icon-prefix`, the fixtures of section 7 |
| R10 | The `title` line is stored but not drawn. `accTitle` and `accDescr` become the svg's `<title>` and `<desc>`                                                                                                            | `mermaidAPI.ts:623`                                           | `P-title-acc`                                                               |
| R11 | Consecutive spaces are kept in measurement and in the drawing (`white-space: pre`)                                                                                                                                     | `styles.ts:33`, `48`                                          | `Q-two-spaces`                                                              |
| R12 | The first rule of the sheet, `.treeView-node-label`, has no svg id scope; the other rules are scoped                                                                                                                   | `styles.ts:30`, and the core's `compileCSS` (`styles.ts:226`) | `P-root-one`, fixture `custom-config`                                       |

## 7. Fixtures

| Fixture (under `e2e/diagrams/tree-view/`)                                      | Features                                                                        | Rows | Desc | Icons (unknown) | Highlight | Probed viewBox                    |
| ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------- | ---- | ---- | --------------- | --------- | --------------------------------- |
| `nightly-check/should-hide-default-icons-with-icon-none-and-icon`              | `showIcons`, `icon(none)`, `icon()`, default icons on `src/` and `package.json` | 5    | 0    | 3 (0)           | 0         | 134.765625 × 150                  |
| `nightly-check/should-render-a-simple-treeview-diagram-with-quoted-labels`     | one quoted root                                                                 | 2    | 0    | 0               | 0         | 68.3125 × 60                      |
| `nightly-check/should-render-combined-annotations`                             | `:::highlight`, `icon(fa:bell)`, `##` three times                               | 9    | 3    | 1 (1)           | 1         | 297.953125 × 270                  |
| `nightly-check/should-render-icon-overrides`                                   | `icon(fa:bell)`, `icon(folder)` on files                                        | 6    | 0    | 2 (1)           | 0         | 118.09375 × 180                   |
| `nightly-check/should-resolve-unprefixed-icon-overrides-via-default-icon-pack` | `icon(bell)` with `defaultIconPack: fa`                                         | 4    | 0    | 1 (1)           | 0         | 118.578125 × 120                  |
| `pr-check/should-pick-file-icons-from-filename-and-extension-config-maps`      | `filenameIcons`, `extensionIcons`, `none`, `defaultIconPack`                    | 7    | 0    | 6 (3)           | 0         | 158.46875 × 210                   |
| `pr-check/should-preserve-consecutive-spaces-and-unicode-in-labels`            | spaces and emoji (U+1F493)                                                      | 4    | 0    | 0               | 0         | 160.390625 × 122                  |
| `pr-check/should-render-a-complex-treeview-diagram-with-quoted-labels`         | four levels, quoted                                                             | 13   | 0    | 0               | 0         | 122.5625 × 390                    |
| `pr-check/should-render-bare-unquoted-labels-with-icons`                       | bare labels, `.gitignore`, `showIcons`                                          | 11   | 0    | 11 (0)          | 0         | 149.953125 × 330                  |
| `pr-check/should-render-class-annotations-for-highlighting`                    | `:::highlight` twice                                                            | 8    | 0    | 0               | 2         | 136.953125 × 240                  |
| `pr-check/should-render-descriptions`                                          | `##` four times                                                                 | 7    | 4    | 0               | 0         | 283.890625 × 210                  |
| `pr-check/should-render-emoji-as-icons-with-the-default-icons-hidden`          | emoji (U+1F680, U+1F4E6, U+1F3A8, U+1F4DD)                                      | 5    | 0    | 0               | 0         | 153 × 158                         |
| `pr-check/should-render-the-unknown-icon-fallback-for-unregistered-icons`      | `icon(unregistered:icon)`                                                       | 3    | 0    | 1 (1)           | 0         | 110.921875 × 90                   |
| `pr-check/should-render-with-custom-config-and-quoted-labels`                  | `rowIndent: 80`, `lineThickness: 3`, `themeVariables.treeView` (frontmatter)    | 12   | 0    | 0               | 0         | 323.0625 × 420 (x starts at −1.5) |
| `pr-check/should-render-with-multiple-roots-and-quoted-labels`                 | several roots, quoted                                                           | 12   | 0    | 0               | 0         | 107.5625 × 360                    |

Counts from the probe outputs (`render-out.json`): "Rows" is the number of `text` elements of class `treeView-node-label`, root included. "Icons" counts the `treeView-node-icon` groups; the number in brackets is how many of them are the unknown icon. Every fixture renders in the reference (`render-log.txt`, 15 of 15).

Harness exclusions (`tools/port/geometry/common.ts`, `exclusionReason` and `uncoveredScriptReason`, used by `diagram-geometry.ts:383-385`): no fixture has `look: handDrawn`, a non-layered layout, or CJK. Two fixtures contain characters in the emoji block U+1F000 to U+1FAFF (`UNCOVERED_SYMBOLS`, `common.ts:183-184`), which the harness excludes as "text the bundled faces do not cover": `should-preserve-consecutive-spaces-and-unicode-in-labels` and `should-render-emoji-as-icons-with-the-default-icons-hidden`. So 13 of 15 fixtures are compared.

Notes on the fixtures:

- `should-pick-file-icons-from-filename-and-extension-config-maps` describes `unmapped.bin` as falling back to the configured `material-icon-theme` pack. The reference draws the built-in `file` icon for it (`icons.ts:99`), and the 6 icons and 3 unknown icons of the probe match that rule. The description is wrong, and the reference's output is what the harness compares.
- `should-render-combined-annotations` and the other `fa:` and `material-icon-theme:` fixtures draw the unknown icon. The reference logs an error for each (`rendering-util/icons.ts:94-99`); the harness compares the SVG only.
- The spaces check that the fixture `should-preserve-consecutive-spaces-and-unicode-in-labels` makes is in an excluded fixture, so the compared set has no test of consecutive spaces. The port's own tests should cover it without emoji (section 8.3, question 6).

## 8. Risks, effort, open questions

### 8.1 Risks

1. **Text measurement of labels and descriptions** (high). Every row height and width depends on `getBBox` of 16 px Lexica text in four faces (regular, bold for directories, italic for descriptions) and on the 1/64 px rounding. The port's text stage (decision 0005) gives exact heights (20 px) and widths within 1/64 px. The description width in `P-desc2` is not on the 1/64 grid, which the port has to cover within the harness tolerance (0.05 px, `common.ts:34`).
2. **Entity placeholders in names** (medium). The layout must measure `ﬂ°amp¶ß`, not `&`. The port must keep the placeholder in the model and the measurement, and decode only the final SVG string (`mermaidAPI.ts:359`). The entity fixtures do not exist in the 15, so this needs a probe case in the port's own tests (`P-entity` gives the numbers).
3. **The unknown icon's `?` in ArialMT** (medium). Five compared fixtures draw it (`combined-annotations`, `icon-overrides`, `resolve-unprefixed…`, `pick-file-icons`, `unregistered`). Ferrigura does not ship Arial, and `STATUS.md:152` records the same gap for the architecture diagram's unknown icon. The text box of the `?` decides whether the harness passes for these five.
4. **The lexer's quirks** (medium, many). The rows Q1 to Q19 of section 2.7 (about 30 cases) must be reproduced exactly. None is hard alone; together they need the scanner to follow the Chevrotain order and the `LONGER_ALT` rule, not a regex that reads better.
5. **The box-drawing preprocessor** (medium). Its arithmetic (`Math.round`, the segment width, tab normalisation) and its two error messages, which the spec asserts (lines 382, 413, 421).
6. **The arithmetic of highlights and descriptions** (low, but silent). The +10 per highlight and the description column over all rows are easy to get wrong in the order of the phases.
7. **The first CSS rule without the svg id** (low for the geometry; it is in the output text). Whether the port reproduces it depends on the core's CSS stage.
8. **DOMPurify parity** for descriptions with `<` (low). `sanitize.rs` in `ferrigura-syntax` covers the flowchart's texts, and the tree view needs the same function (Q13b).

### 8.2 Effort

| Part                                                                                                    | Rust lines (estimate) | Basis                                                                 |
| ------------------------------------------------------------------------------------------------------- | --------------------- | --------------------------------------------------------------------- |
| Scanner: 13 tokens, first match, the keyword's longer alternative, the value rules                      | 250 to 300            | grammar lines 13-63; section 2.2                                      |
| Parser: entry, header fragment, node rule, annotations, value conversion                                | 150 to 200            | section 2.1, 2.3                                                      |
| Box-drawing preprocessor                                                                                | 200 to 230            | `boxDrawingPreprocessor.ts`, 209 lines                                |
| DB: stack, `addNode`, setters, `getCount`, `getConfig`                                                  | 120 to 150            | `db.ts`, 91 lines                                                     |
| Icons: built-in pack, `getNodeIcon`, `detectIcon`, the unknown icon, the SVG string                     | 150 to 200            | `icons.ts`, 100 lines; `rendering-util/icons.ts` for the unknown icon |
| Renderer: layout, phases, icon groups, the SVG tree, the viewBox                                        | 300 to 400            | `renderer.ts`, 242 lines, plus the text measurement calls             |
| Styles and typed view of the configuration and the seven theme keys                                     | 100 to 150            | `styles.ts`, 58 lines; decision 0009                                  |
| Total, without tests                                                                                    | about 1,300 to 1,650  |                                                                       |
| Tests: 116 spec cases ported, 15 fixtures through the harness, probe cases of section 6 as render tests | about 900 to 1,100    | section 5, section 6.3                                                |
| Differential exporter (`tools/port/export-tree-view.ts`)                                                | about 150             | the pattern of the other `export-*.ts` files                          |

The estimate depends on the text stage: if the port reuses the label measurement of the other SVG-text diagrams (decision 0004), the renderer's share falls.

### 8.3 Open questions for the coordinator

1. **The root row.** The reference always draws `/` (section 0, finding 1; section 6.11, R1). The differential needs it. Should the port reproduce it as it is, or should the fork fix it first? The reference's own user docs show no such row. Recommendation: reproduce it, and record it as a quirk of this fork.
2. **The lexer and preprocessing quirks.** Section 2.7 lists about 30 cases, the most visible being that `titlepage.md` becomes `page.md`. The differential run needs them. Should they all be reproduced, including the silent ones (Q1, Q4, Q15, Q16)? Recommendation: yes, with a test for each quirk, because the P3 plan's done-when is no difference from the reference on every collected input.
3. **Entity placeholders in measurement** (Q12, section 6.3). The layout measures the placeholder. Is the port's entity pipeline (`entities.rs`) already doing the same for the other diagrams?
4. **`themeVariables.treeView`.** The seven keys are not in the shipped themes or in the typed theme tokens of [0007](../decisions/0007-themes-as-css-variables.md). Should the port give them a typed section with the defaults of `styles.ts:6-12`, as decision 0009 allows? Also: the init-directive form of `themeVariables` did not change the probed sheet, while the frontmatter form did. The config diff should say which form the reference honours.
5. **The unknown icon's font** (section 8.1, risk 3). Is the harness's comparison of the `?` text in five fixtures acceptable with a known gap (a named exclusion), or should ferrigura draw the question mark with a metric-compatible font? The architecture notes take the first option (`STATUS.md:152`).
6. **Two excluded fixtures.** `should-preserve-consecutive-spaces-and-unicode-in-labels` and `should-render-emoji-as-icons-with-the-default-icons-hidden` are excluded for emoji. The first is the only fixture for consecutive spaces. Should the port add a fixture without emoji to its own tests, or wait for the fallback-font decision (open since M2)?
7. **The first CSS rule.** `.treeView-node-label` is emitted without the svg id (section 6.7). Is that the core's stage to reproduce, and who owns it?
8. **Fixture text.** `should-pick-file-icons-…` says that `unmapped.bin` falls back to `material-icon-theme`. The reference uses the built-in `file` icon. The harness compares against the reference, so nothing changes, but the description should not be used as the spec.
9. **Error messages.** The preprocessor's two messages are asserted by the spec (`boxDrawingPreprocessor.spec.ts:382, 413, 421`), and they are plain `Error`s, not `MermaidParseError`s. Should the port keep them as they are, with the same `Line N:` prefix?
10. **No exporter yet.** `tools/port/export-tree-view.ts` does not exist. The model in section 4 is the proposal for the exporter and the `port:diff treeView` check.
