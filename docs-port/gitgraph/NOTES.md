# GitGraph: research notes for M6

Read-only research. Nothing else in the repository was changed. Reference: the JS reference at `04b952b4e`, which is `main` and the M6 worktree HEAD.

Labels: **code** = read at the cited line. **probe** = run against the reference (scratchpad `m6-gitgraph/`, not committed). **unverified** = neither.

Probe setup: the worktree has no Langium output (`packages/parser/src/language/generated` is gitignored and exists only in the main checkout, same HEAD). Probes loaded the reference of the main checkout through Vite SSR, as `tools/port/reference.ts` does, with the cache outside the repo and `@mermaid-js/parser` aliased to `packages/parser/src/index.ts`. The DB probe calls the reference `parser.parse` and reads the DB. The render probe runs `mermaidAPI.render` in jsdom with `getBBox` stubbed, so widths are not Chromium's; element structure and attribute text are real.

## 0. Headlines

1. The grammar is small (56 lines, 5 statements), but the lexer is Chevrotain's first-match: `title…` and digit-led names fail, and exact keywords cannot be unquoted branch names (§2.3).
2. The DB raises 14 exact error messages and one warning (§2.5). Two quirks must be kept: a merge is refused when the current head commit belongs to the other branch, and a duplicate commit id only warns and makes a commit its own parent.
3. Commit, merge and cherry-pick ids without an explicit id use `Math.random()` (7 hex digits). The `deterministicIds` seed is not used. No fixture relies on them, but spec inputs do (§2.7).
4. Sanitisation has two paths. Ids, messages, tags and branch names use the gitGraph config section, which has no `securityLevel`, so the escape branch of `sanitizeMore` runs (`=` becomes `&equals;`). Title, accTitle and accDescr use the top-level config (§2.6).
5. The renderer is custom (no ELK). Its label boxes and viewBox come from `getBBox`, and its colours and geometry branch on the theme name. 105 of 135 fixtures use `default`, which ferrigura does not ship. The fallback-by-name rule of M2 and M5 does not hold here (§5.14, §7).

## 1. Sources

### 1.1 Files

| File                                                       | Lines                                                                                                                                                                     | What it holds                                                                                                                                                                     |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `packages/parser/src/language/gitGraph/gitGraph.langium`   | 1-56                                                                                                                                                                      | Grammar: entry 5-13, statements 15-21, direction 23-24, commit 26-33, branch 34-37, merge 39-45, checkout 47-48, cherry-pick 50-56                                                |
| `packages/parser/src/language/gitGraph/reference.langium`  | 4                                                                                                                                                                         | `REFERENCE` terminal `\w([-\./\w]*[-\w])?`                                                                                                                                        |
| `packages/parser/src/language/common/common.langium`       | 6-34                                                                                                                                                                      | `EOL`, `TitleAndAccessibilities` (8-10), ACC_DESCR 14, ACC_TITLE 15, TITLE 16, INT 19, STRING 22, NEWLINE 29, hidden WHITESPACE 31, YAML 32, DIRECTIVE 33, SINGLE_LINE_COMMENT 34 |
| `packages/parser/src/language/common/tokenBuilder.ts`      | 14-28                                                                                                                                                                     | Keyword tokens of the listed keywords get the suffix `(?:(?=%%)\|(?!\S))` (24)                                                                                                    |
| `packages/parser/src/language/gitGraph/tokenBuilder.ts`    | 3-7                                                                                                                                                                       | Keyword list is `['gitGraph']`                                                                                                                                                    |
| `packages/parser/src/language/common/valueConverter.ts`    | 6-10, 27-42, 44-70                                                                                                                                                        | Value rules of ACC_DESCR, ACC_TITLE, TITLE: trim, collapse blanks, block form                                                                                                     |
| `packages/parser/src/language/common/matcher.ts`           | 4, 9, 14                                                                                                                                                                  | The three regexes the converter uses                                                                                                                                              |
| `packages/parser/src/parse.ts`                             | 65-69, 144-148, 156                                                                                                                                                       | gitGraph parser; `parse()` throws `MermaidParseError` on any lexer or parser error (147)                                                                                          |
| `packages/mermaid/src/diagrams/git/gitGraphDetector.ts`    | 6-8                                                                                                                                                                       | Detector `/^\s*gitGraph/`                                                                                                                                                         |
| `packages/mermaid/src/diagrams/git/gitGraphDiagram.ts`     | 8-13                                                                                                                                                                      | Diagram: parser, db, renderer, styles                                                                                                                                             |
| `packages/mermaid/src/diagrams/git/gitGraphParser.ts`      | 21-31, 33-48, 50-91, 93-99, 101-249                                                                                                                                       | `populate`, `parseStatement`, converters, `parser.parse`, 6 in-source tests                                                                                                       |
| `packages/mermaid/src/diagrams/git/gitGraphAst.ts`         | 29-38, 41-47, 49-64, 66-68, 105-134, 136-150, 152-259, 261-368, 369-390, 452-455, 457-472, 498-525                                                                        | State, config, DB methods (see §2.5)                                                                                                                                              |
| `packages/mermaid/src/diagrams/git/gitGraphTypes.ts`       | 4-10, 12-48, 50-88, 90-132, 134                                                                                                                                           | `commitType`, DB and AST types, DB interfaces                                                                                                                                     |
| `packages/mermaid/src/diagrams/git/gitGraphRenderer.ts`    | 25-30, 36-48, 56-65, 66-73, 75-82, 84-99, 101-224, 226-329, 331-402, 404-491, 493-509, 511-563, 565-620, 622-651, 653-849, 851-865, 867-984, 986-996, 998-1089, 1095-1489 | Renderer; in-source tests 1095-1489 (see §4)                                                                                                                                      |
| `packages/mermaid/src/diagrams/git/styles.js`              | 2-15, 17-28, 30-101, 103-117, 118-174                                                                                                                                     | Stylesheet generator (see §1.4)                                                                                                                                                   |
| `packages/mermaid/src/schemas/config.schema.yaml`          | 1009-1055                                                                                                                                                                 | `GitGraphDiagramConfig` (see §1.2)                                                                                                                                                |
| `packages/mermaid/src/diagrams/common/common.ts`           | 8, 48-54, 56-69, 71-83, 134-136                                                                                                                                           | `sanitizeText`, `sanitizeMore`, `removeScript`, `breakToPlaceholder`                                                                                                              |
| `packages/mermaid/src/diagrams/common/commonDb.ts`         | 8, 16-18, 22-24, 28-30                                                                                                                                                    | Shared accTitle, accDescr and title setters (top-level config)                                                                                                                    |
| `packages/mermaid/src/diagrams/common/populateCommonDb.ts` | 4-14                                                                                                                                                                      | accDescr, then accTitle, then title, each if non-empty                                                                                                                            |
| `packages/mermaid/src/utils.ts`                            | 453-465, 734-743, 793-813                                                                                                                                                 | `makeRandomHex` and `random` (Math.random); `InitIDGenerator` (not used by gitGraph); `insertTitle`                                                                               |
| `packages/mermaid/src/setupGraphViewbox.js`                | 49-72                                                                                                                                                                     | viewBox and size from the root `getBBox()`                                                                                                                                        |
| `packages/mermaid/src/mermaidAPI.ts`                       | 142-160, 319-335, 596-600, 605-608, 623                                                                                                                                   | Stylesheet creation, style element inserted before draw (596-600), draw (605-608), a11y (623)                                                                                     |
| `packages/mermaid/src/accessibility.ts`                    | 27-32, 45-…                                                                                                                                                               | `role`, `aria-roledescription`, `<title>`, `<desc>`                                                                                                                               |
| `packages/mermaid/src/Diagram.ts`                          | 49, 52-54                                                                                                                                                                 | `db.clear()` after config, then the front matter title                                                                                                                            |
| `packages/mermaid/src/themes/theme-base.js`                | 18-26, 366-419                                                                                                                                                            | Base theme: THEME_COLOR_LIMIT, strokeWidth, font, gradient, git variables                                                                                                         |
| `packages/mermaid/src/themes/theme-default.js`             | 36-38, 136, 397-…                                                                                                                                                         | Default theme's own overrides (`useGradient` false, own git block)                                                                                                                |
| `packages/mermaid/src/themes/theme-redux-color.js`         | 19, 21, 30, 32, 35, 53, 393-447                                                                                                                                           | Redux-color overrides and git block                                                                                                                                               |
| `packages/mermaid/src/themes/theme-redux-dark-color.js`    | 33, 35, 46, 49, 52, 68, 410-463                                                                                                                                           | Redux-dark-color overrides and git block                                                                                                                                          |
| `packages/parser/src/language/generated/ast.ts`            | 169-182                                                                                                                                                                   | Reachable terminals of GitGraphGrammar (main checkout only)                                                                                                                       |
| `packages/parser/src/language/generated/grammar.ts`        | 19                                                                                                                                                                        | Serialised grammar (main checkout only)                                                                                                                                           |

### 1.2 Configuration (gitGraph section)

Defaults come from the schema through `defaultConfig.ts` (its `?only-defaults=true` import, line 9). `defaultConfig.ts` has no gitGraph entry.

| Key                          | Schema line (default) | Default                              | Read at                            | Notes                                                                   |
| ---------------------------- | --------------------- | ------------------------------------ | ---------------------------------- | ----------------------------------------------------------------------- |
| `titleTopMargin`             | 1017 (1023)           | 25                                   | renderer 1083                      | Title `y = -titleTopMargin`                                             |
| `diagramPadding`             | 1025 (1027)           | 8                                    | renderer 1088                      | viewBox padding                                                         |
| `nodeLabel`                  | 1028                  | `{width 75, height 100, x -25, y 0}` | nowhere in git                     | **dead key**, kept in the section                                       |
| `mainBranchName`             | 1036 (1038)           | `'main'`                             | AST state 51, 57-59                | Read at `clear()` (Diagram.ts 49)                                       |
| `mainBranchOrder`            | 1039 (1041)           | 0                                    | AST state 57                       | Order of the main branch                                                |
| `showCommitLabel`            | 1042 (1044)           | true                                 | renderer 340                       | Hides commit labels only (tags stay)                                    |
| `showBranches`               | 1045 (1047)           | true                                 | renderer 1074                      | No branch lines or branch labels                                        |
| `rotateCommitLabel`          | 1048 (1050)           | **true**                             | renderer 371, 945, 954, 1008, 1067 | The renderer's own `?? false` never applies; the schema default is true |
| `parallelCommits`            | 1051 (1053)           | false                                | renderer 575                       |                                                                         |
| `arrowMarkerAbsolute`        | 1055 (anchor 326)     | false                                | nowhere in git                     | unused                                                                  |
| `useMaxWidth`                | BaseDiagramConfig     | true                                 | renderer 1088                      |                                                                         |
| `theme` / `look` (top level) | BaseDiagramConfig     | `default` / `classic`                | renderer 1016, styles 120          | **probe**                                                               |

Probe: the section the DB sees is `{useMaxWidth:true, titleTopMargin:25, diagramPadding:8, nodeLabel:{…}, mainBranchName:'main', mainBranchOrder:0, showCommitLabel:true, showBranches:true, rotateCommitLabel:true, parallelCommits:false, arrowMarkerAbsolute:false}`. It has **no** `securityLevel` and no `htmlLabels` (this matters for §2.6).

### 1.3 Theme variables read by gitGraph

| Variable                                                                             | Used at                                            | Base default (theme-base.js)                                                                                                                    | Redux overrides                                          |
| ------------------------------------------------------------------------------------ | -------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| `git0`…`git7`                                                                        | styles 53-55, 109-113 (normal theme, neo)          | `primaryColor`, `secondaryColor`, `tertiaryColor`, then `adjust(primary, h -30/-60/-90/+60/+120)`; dark mode lightens by 25, else darkens by 25 | redux-color 393-401; redux-dark-color 410-418            |
| `gitInv0`…`gitInv7`                                                                  | styles 54, 111                                     | `invert(git_i)`                                                                                                                                 | same                                                     |
| `gitBranchLabel0`…`gitBranchLabel7`                                                  | styles 52, 109                                     | `branchLabelColor` (black in dark mode, else `labelTextColor`)                                                                                  | same                                                     |
| `THEME_COLOR_LIMIT`                                                                  | styles 21, 63, 78, 104; renderer 874, 881          | 12                                                                                                                                              | redux-color 19; redux-dark-color 33, 40; default 36      |
| `borderColorArray`                                                                   | styles 33, 89-95 (redux colour themes)             | (per theme)                                                                                                                                     | redux-color 55-68                                        |
| `commitLabelFontSize`                                                                | styles 140                                         | `'10px'` (419)                                                                                                                                  | same                                                     |
| `commitLabelColor`, `commitLabelBackground`                                          | styles 140-141 (classic only)                      | 417-418                                                                                                                                         | same                                                     |
| `tagLabelFontSize`                                                                   | styles 142                                         | `'10px'` (416)                                                                                                                                  | same                                                     |
| `tagLabelColor`, `tagLabelBackground`                                                | styles 142-143                                     | 413-414                                                                                                                                         | same                                                     |
| `tagLabelBorder`                                                                     | styles 143                                         | `this.tagBorder \|\| primaryBorderColor` (415). **Bug**: `tagBorder` is never set, so the user value is ignored                                 | same                                                     |
| `strokeWidth`                                                                        | styles 65-68, 84, 136, 153, 164                    | 1                                                                                                                                               | 2 (redux-color 21; redux-dark-color 35)                  |
| `noteFontWeight`                                                                     | styles 65, 68, 81, 84, 91, 140                     | `'normal'`                                                                                                                                      | 600 (redux-color 53; redux-dark-color 68)                |
| `nodeBorder`, `mainBkg`, `primaryColor`, `textColor`, `lineColor`, `commitLineColor` | styles 41-45, 53-70, 83-86, 137, 144, 147-158, 171 | base 57, 56, 49, 137                                                                                                                            | redux `nodeBorder` 30 (#28253D), redux-dark 46 (#FFFFFF) |
| `useGradient`, `gradientStart`, `gradientStop`                                       | renderer 1017-1037; styles 17-28                   | base true (25)                                                                                                                                  | redux false (32, 49); default false (136)                |
| `dropShadow`, `filterColor`                                                          | renderer 1054; styles 143, 941                     | base `drop-shadow(...)` (26)                                                                                                                    | redux `url(#drop-shadow)` (35, 52); filterColor 100, 102 |
| `darkMode` (indirect)                                                                | renderer 237 (DARK_THEMES by name)                 |                                                                                                                                                 |                                                          |

### 1.4 Stylesheet (styles.js, `getStyles` 118-174)

The generator is wrapped under `#id` by `compileCSS`. Every `.x` below is the selector text in the generated sheet.

| Selector                                     | Declarations                                                                                                                 | Lines                                          |
| -------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| `.commit-id, .commit-msg, .branch-label`     | `fill: lightgrey`; `font-family: var(--mermaid-font-family)`                                                                 | 124-131                                        |
| per index i (normal or neo)                  | `.branch-label{i}`, `.commit{i}`, `.commit-highlight{i}`, `.label{i}`, `.arrow{i}`                                           | 103-117 (normal: `i % 8` wraps into git0…git7) |
| per index i (neo)                            | the same, with i=0 using `nodeBorder`                                                                                        | 30-58                                          |
| per index i (redux-color, redux-dark-color)  | i=0 `nodeBorder`; i>0 `borderColorArray[i % len]`                                                                            | 78-98                                          |
| per index i (classic-like, not redux colour) | `.commit{i} stroke nodeBorder`, `.label{i}` with `mainBkg`                                                                   | 60-74                                          |
| `.branch`                                    | `stroke-width: strokeWidth`; `stroke: commitLineColor ?? lineColor`; `stroke-dasharray: 4 2` (neo gen) or `2`                | 135-139                                        |
| `.commit-label`                              | `font-size: commitLabelFontSize`; fill `nodeBorder` (neo gen) or `commitLabelColor`; `font-weight: noteFontWeight` (neo gen) | 140                                            |
| `.commit-label-bkg`                          | `font-size`; `fill: transparent` (neo gen) or `commitLabelBackground`; `opacity` 0.5 (classic)                               | 141                                            |
| `.tag-label`                                 | `font-size: tagLabelFontSize`; `fill: tagLabelColor`                                                                         | 142                                            |
| `.tag-label-bkg`                             | fill `mainBkg` (neo gen) or `tagLabelBackground`; stroke `nodeBorder` or `tagLabelBorder`; `filter: dropShadow` (neo gen)    | 143                                            |
| `.tag-hole`                                  | `fill: textColor`                                                                                                            | 144                                            |
| `.commit-merge`                              | stroke/fill `mainBkg` (neo gen) or `primaryColor`                                                                            | 146-149                                        |
| `.commit-reverse`                            | as merge; `stroke-width: strokeWidth` (neo gen) or 3                                                                         | 150-154                                        |
| `.commit-highlight-inner`                    | stroke/fill `mainBkg` (neo gen) or `primaryColor`                                                                            | 157-160                                        |
| `.arrow`                                     | `stroke-width`: `strokeWidth` for redux-geometry themes, else 8; `stroke-linecap: round`; `fill: none`                       | 162-167                                        |
| `.gitTitleText`                              | `text-anchor: middle`; `font-size: 18px`; `fill: textColor`                                                                  | 168-172                                        |

Neo-gen themes (`NEO_COLOR_GEN_THEMES`, styles 8-15) are redux, redux-dark, redux-color, redux-dark-color, neo and neo-dark. The `normalTheme` path is used for everything else, including `default`.

## 2. Parser

### 2.1 Grammar rules

| Rule                    | Lines       | Productions                                                                                                | DB call (gitGraphParser.ts)                       |
| ----------------------- | ----------- | ---------------------------------------------------------------------------------------------------------- | ------------------------------------------------- |
| GitGraph (entry)        | 5-13        | `NEWLINE*`, header, then `(NEWLINE \| TitleAndAccessibilities \| Statement)*`                              | `populate` 21-31                                  |
| header                  | 7           | `gitGraph`, `gitGraph:`, `gitGraph` Direction `:`                                                          | `setDirection` only when `ast.dir` is set (24-27) |
| Direction               | 23-24       | `LR` \| `TB` \| `BT`                                                                                       |                                                   |
| Statement               | 15-21       | Commit \| Branch \| Merge \| Checkout \| CherryPicking                                                     | `parseStatement` 33-48                            |
| Commit                  | 26-33       | `commit` ( `id:` STRING \| `msg:`? STRING \| `tag:` STRING \| `type:` (NORMAL\|REVERSE\|HIGHLIGHT) )\* EOL | `db.commit` 35, 50-58                             |
| Branch                  | 34-37       | `branch` (REFERENCE \| STRING) (`order:` INT)? EOL                                                         | `db.branch` 36, 60-66                             |
| Merge                   | 39-45       | `merge` (REFERENCE \| STRING) ( `id:` STRING \| `tag:` STRING \| `type:` … )\* EOL                         | `db.merge` 37, 68-76                              |
| Checkout                | 47-48       | (`checkout` \| `switch`) (REFERENCE \| STRING) EOL                                                         | `db.checkout` 38, 78-81                           |
| CherryPicking           | 50-56       | `cherry-pick` ( `id:` STRING \| `tag:` STRING \| `parent:` STRING )\* EOL                                  | `db.cherryPick` 39, 83-91                         |
| TitleAndAccessibilities | common 8-10 | `((accDescr \| accTitle \| title) EOL)+`                                                                   | `populateCommonDb.ts` 4-14                        |
| EOL                     | common 6    | `NEWLINE+` \| EOF                                                                                          |                                                   |

Assignment rules (Langium): `=` keeps the last value (probe: `msg:"first" msg:"second"` gives `second`; `id:"A" id:"B"` gives `B`), `+=` collects (`tag:` repeats), and `'msg:'?` means a bare string is the message (probe: `commit "quoted msg"` works). Option order is free.

### 2.2 Terminals and keywords (generated, main checkout)

Reachable terminals of GitGraphGrammar (`generated/ast.ts` 171-182): ACC_DESCR, ACC_TITLE, TITLE, INT, STRING, NEWLINE, WHITESPACE (hidden), YAML (hidden), DIRECTIVE (hidden), SINGLE_LINE_COMMENT (hidden), REFERENCE. FLOAT, NUMBER, ID and BOOLEAN are not in the lexer of this grammar.

| Terminal  | Regex                                                                                               | Source       |
| --------- | --------------------------------------------------------------------------------------------------- | ------------ |
| ACC_DESCR | `[\t ]*accDescr(?:[\t ]*:([^\n\r]*?(?=%%)\|[^\n\r]*)\|\s*{([^}]*)})`                                | common 14    |
| ACC_TITLE | `[\t ]*accTitle[\t ]*:(?:[^\n\r]*?(?=%%)\|[^\n\r]*)`                                                | common 15    |
| TITLE     | `[\t ]*title(?:[\t ][^\n\r]*?(?=%%)\|[\t ][^\n\r]*\|)`                                              | common 16    |
| INT       | `0\|[1-9][0-9]*(?!\.)`                                                                              | common 19    |
| STRING    | `"([^"\\]\|\\.)*"\|'([^'\\]\|\\.)*'`                                                                | common 22    |
| REFERENCE | `\w([-\./\w]*[-\w])?` (JS `\w` is ASCII only)                                                       | reference 4  |
| NEWLINE   | `\r?\n`                                                                                             | common 29    |
| hidden    | WHITESPACE `[\t ]+`; YAML front matter; DIRECTIVE `%%{…}%%`; SINGLE_LINE_COMMENT `[\t ]*%%[^\n\r]*` | common 31-34 |

Keywords: `commit`, `branch`, `merge`, `checkout`, `switch`, `cherry-pick`, `gitGraph`, `gitGraph:`, `TB`, `LR`, `BT`, `NORMAL`, `REVERSE`, `HIGHLIGHT`, `:`, and the option keywords `id:`, `msg:`, `tag:`, `type:`, `order:`, `parent:` (generated `KeywordNames`, the grammar text, `grammar.ts` 19).

The `gitGraph` keyword gets the suffix `(?:(?=%%)|(?!\S))` (tokenBuilder 24). So `gitGraphX` is not a header; the detector still matches it (`gitGraphDetector.ts` 7).

### 2.3 Lexing: what the reference accepts (probe)

Ordered by the reason. Chevrotain tries terminals in the grammar's order and takes the first match. A keyword has a longer alternative (REFERENCE), so a longer name wins. TITLE, INT and STRING have none.

| Input                                                                                                                         | Reference                                 | Reason                                                                                          |
| ----------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `commit id:abc`                                                                                                               | error: expected STRING                    | `id:` takes only STRING                                                                         |
| `branch café` (also `é`)                                                                                                      | lexer error                               | REFERENCE is ASCII (`\w`)                                                                       |
| `branch "café"`, `commit id:"café"`                                                                                           | accepted                                  | STRING takes any character                                                                      |
| `branch a.` / `branch 10.x-y` / `branch 1.0.`                                                                                 | lexer error                               | REFERENCE must end in `[-\w]`; `10` fails INT's lookahead and backtracks                        |
| `branch a-` / `a..b` / `a/b.c_d-e` / `_x` / `v1.0` / `1.0` / `1.0.1`                                                          | accepted                                  | REFERENCE                                                                                       |
| `branch -a`                                                                                                                   | lexer error                               | REFERENCE starts with `\w`                                                                      |
| `branch 1` / `12` / `0` / `0a` / `1a`                                                                                         | parse error                               | INT wins and REFERENCE is not tried                                                             |
| `branch 1-0`                                                                                                                  | lexer error                               | INT takes `1`, then `-` matches nothing                                                         |
| `branch commit`, `merge`, `checkout`, `switch`, `cherry-pick`, `TB`, `LR`, `BT`, `NORMAL`, `REVERSE`, `HIGHLIGHT`, `gitGraph` | parse error                               | exact keyword token; quoted names are accepted (`branch "commit"`, probe)                       |
| `branch order`, `msg`, `tag`, `type`, `id`, `parent`, `accTitle`                                                              | accepted                                  | the keyword is the word with its colon                                                          |
| `branch commitX`, `checkout02`                                                                                                | accepted                                  | keyword with longer REFERENCE                                                                   |
| `branch title`, `branch titleX`, `title: X` in the body                                                                       | parse error                               | TITLE matches `title` alone (no longer alternative). `title X` (no colon) is a valid title line |
| `branch accTitleX`                                                                                                            | accepted                                  | ACC_TITLE needs `:`                                                                             |
| `gitGraph TB` (no colon)                                                                                                      | parse error                               | header needs `:`                                                                                |
| `gitGraph commit` (one line)                                                                                                  | accepted                                  | header followed by a statement, probe                                                           |
| `commit;` / `;` anywhere                                                                                                      | lexer error                               | no semicolon terminal                                                                           |
| `commit %% comment`, blank lines, CRLF                                                                                        | accepted                                  | hidden tokens; `\r?\n`                                                                          |
| `order: 1.5` / `order: 007` / `order:-1` / `order:xyz`                                                                        | error                                     | INT only                                                                                        |
| `merge` / `branch` / `checkout` / `cherry-pick` with no name                                                                  | parse error (cherry-pick: DB error, §2.5) | required names                                                                                  |
| `commit type:FOO`                                                                                                             | parse error                               | closed set                                                                                      |
| `branch a b`                                                                                                                  | parse error                               | one name                                                                                        |
| `branch ""`                                                                                                                   | accepted, creates a branch named `""`     | STRING, empty                                                                                   |

### 2.4 AST to DB: the order of calls

```
populate(ast, db):
  populateCommonDb(ast, db):           # gitGraphParser.ts 22; populateCommonDb.ts 4-14
    if ast.accDescr: db.setAccDescription(ast.accDescr)
    if ast.accTitle: db.setAccTitle(ast.accTitle)
    if ast.title:    db.setDiagramTitle(ast.title)
  if ast.dir: db.setDirection(ast.dir)  # 24-27; LR otherwise (DB state default 60)
  for stmt in ast.statements (source order):   # 28-30
    Commit      -> db.commit({id, msg: message ?? '', type: commitType[type] ?? NORMAL, tags: tags ?? undefined})    # 50-58
    Branch      -> db.branch({name, order: order ?? 0})                                                             # 60-66
    Merge       -> db.merge({branch, id: id ?? '', type: commitType[type] (undefined if none), tags: tags ?? undefined})  # 68-76
    Checkout    -> db.checkout(branch)                                                                             # 78-81
    CherryPick  -> db.cherryPick({id, targetId: '', tags: tags.length === 0 ? undefined : tags, parent})           # 83-91
```

`targetId` is always `''`, so the "cherry-pick to another commit" branch of the DB (gitGraphAst 307-367 when `targetId` is set) is unreachable from the grammar; only the current-branch path runs. `db.commit` is called with `tags: undefined` when there are none, and the DB makes `[]`.

The DB clear happens before this (Diagram.ts 49), and the front-matter title is set between clear and parse (Diagram.ts 52-54). The initial state reads the config at clear time, so `mainBranchName` from front matter takes effect.

### 2.5 DB methods, state and errors

State (gitGraphAst 29-38): `commits` (Map id to Commit, insertion order), `head` (Commit or null), `branchConfig` (Map name to `{name, order}`), `branches` (Map name to head commit id or null), `currBranch`, `direction` (default LR), `seq` (counter), `options` (dead).

Initial state: `branches = {main: null}`, `branchConfig = {main: {order: mainBranchOrder}}`, `currBranch = mainBranchName`.

| Method                  | Lines   | Condition (in order)                                                             | Exact message                                                                                                                        |
| ----------------------- | ------- | -------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `commit`                | 105-134 | id: `id` after sanitising, else `${seq}-${random7}`                              | none. Duplicate id: `log.warn("Commit ID ${id} already exists")` (128-130), then the Map entry is replaced                           |
| `branch`                | 136-150 | `branches.has(name)`                                                             | `Trying to create an existing branch. (Help: Either use a new name if you want create a new branch or try using "checkout ${name}")` |
| `merge`                 | 152-259 | (a) `currentCommit && otherCommit && currentCommit.branch === otherBranch` (170) | `Cannot merge branch '${other}' into itself.`                                                                                        |
|                         |         | (b) `currBranch === otherBranch` (173)                                           | `Incorrect usage of "merge". Cannot merge a branch to itself`                                                                        |
|                         |         | (c) no current head commit (182)                                                 | `Incorrect usage of "merge". Current branch (${curr})has no commits` (no space before "has")                                         |
|                         |         | (d) other not in `branches` (193)                                                | `Incorrect usage of "merge". Branch to be merged (${other}) does not exist`                                                          |
|                         |         | (e) other head null (204)                                                        | `Incorrect usage of "merge". Branch to be merged (${other}) has no commits`                                                          |
|                         |         | (f) same head (215)                                                              | `Incorrect usage of "merge". Both branches have same head`                                                                           |
|                         |         | (g) `customId` already a commit (224)                                            | `Incorrect usage of "merge". Commit with id:${customId} already exists, use different custom id`                                     |
| `cherryPick`            | 261-368 | (1) no source id or unknown (275)                                                | `Incorrect usage of "cherryPick". Source commit id should exist and provided`                                                        |
|                         |         | (2) `parent` not in source's parents (291)                                       | `Invalid operation: The specified parent commit is not an immediate parent of the cherry-picked commit.`                             |
|                         |         | (3) source is MERGE and no parent (301)                                          | `Incorrect usage of cherry-pick: If the source commit is a merge commit, an immediate parent commit must be specified.`              |
|                         |         | (4) source on current branch (310)                                               | `Incorrect usage of "cherryPick". Source commit is already on current branch`                                                        |
|                         |         | (5) current branch has no head (322, 335)                                        | `Incorrect usage of "cherry-pick". Current branch (${curr})has no commits`                                                           |
| `checkout`              | 369-390 | branch not in `branches` (371)                                                   | `Trying to checkout branch which is not yet created. (Help try using "branch ${branch}")`                                            |
| `setDirection`          | 86-88   | none; stores the value (LR/TB/BT)                                                |                                                                                                                                      |
| `getBranchesAsObjArray` | 457-472 | sort by `order` (stable), name only                                              | the `parseFloat('0.'+i)` fallback (465) is dead: every branch has a number                                                           |

Notes on the messages:

- Check (a) compares the **head commit's branch**, not the current branch name. `commit; branch b; checkout b; merge main` gives "Cannot merge branch 'main' into itself." (probe), because b's head is still main's commit.
- Check (b) is reached for `commit A; branch b; checkout b; merge b` (probe: `Incorrect usage of "merge". Cannot merge a branch to itself`). Check (a) does not fire there, because A's branch is main. Check (c) fires for `gitGraph` then `merge x` with no commit on main (probe: `Current branch (main)has no commits`); `merge main` there hits check (b) first.
- Merge parents are `[head.id, otherBranch head]`, with `head` possibly null (then `[]`). Probe: `commit A; branch b; checkout b; commit B; checkout main; merge b` gives parents `["A","B"]` (no duplicates).
- Cherry-pick parents are `[head.id, source.id]`. The probe `commit A; branch b; checkout b; cherry-pick id:"A"` gives parents `["A","A"]`: a duplicate, kept as is.
- Cherry-pick tags: `tags ? tags.filter(Boolean) : ['cherry-pick:' + src + (merge ? '|parent:' + parent : '')]` (353-359). `tag:""` gives `[]` (probe), and a commit's `tag:""` gives `[""]` (no filter).
- Cherry-pick's message is `cherry-picked ${src.message} into ${curr}` (348). An empty source message gives a double space (probe).
- Duplicate commit id (probe `commit id:"a"` twice): one commit in the Map, `seq` 1, and `parents ["a"]`, a self-loop. The reference keeps this; the port must too, or name it as an exception.
- Merge with no `id:` uses `${seq}-${random7}` (244); with `id:` existing it throws (g).
- `commit` duplicate ids are only a warning; `merge id:` duplicates are an error. The two must differ.

Other methods: `prettyPrint` (446-450) is debug output. `setOptions`/`getOptions` (90-103) are reachable only from the DB object and never from the grammar (dead). `getCommitsArray` sorts by `seq` (480-487). `getHead` (494-496) is `branches[currBranch]`'s commit in every path (derivable).

### 2.6 Sanitisation (probe)

Every gitGraph text goes through `common.sanitizeText(text, config)` (common.ts 71-83), where `config` is the **gitGraph section**, not the top-level config. Titles and headers use the top-level `getConfig()` (commonDb.ts 8).

`sanitizeText(text, config)`:

```
if !text: return text
text = DOMPurify.sanitize(sanitizeMore(text, config), {FORBID_TAGS: ['style']})
sanitizeMore(text, config):
  if getEffectiveHtmlLabels(config):   # config.htmlLabels ?? true  -> true (section has none)
    level = config.securityLevel
    if level in (antiscript, strict, sandbox): text = removeScript(text)     # DOMPurify.sanitize(text)
    elif level != 'loose':                                                  # undefined in the section
      text = breakToPlaceholder(text)   # <br> -> #br#           (common 134-136)
      text = text.replace('<','&lt;').replace('>','&gt;').replace('=','&equals;')
      text = placeholderToBreak(text)   # #br# -> <br/>
```

DOMPurify returns its input unchanged when it contains no `<` (the port already does this: `sanitize.rs` 89-91).

| Field                                                                                                 | Config used                     | Input              | Model value (probe)                                                     |
| ----------------------------------------------------------------------------------------------------- | ------------------------------- | ------------------ | ----------------------------------------------------------------------- |
| commit `id`, `msg`, `tag`; merge `id`, `tag`; cherry `id`, `tag`, `parent`; branch and checkout names | gitGraph section, escape branch | `AT&T`             | `AT&T`                                                                  |
| same                                                                                                  |                                 | `<b>x</b>`         | `&lt;b&gt;x&lt;/b&gt;`                                                  |
| same                                                                                                  |                                 | `a=b`              | `a&equals;b`                                                            |
| same                                                                                                  |                                 | `a<br>b`           | `a<br>b`                                                                |
| same                                                                                                  |                                 | `x<y`, `<tag>`     | `x&lt;y`, `&lt;tag&gt;`                                                 |
| same                                                                                                  |                                 | `a&b<c>`           | `a&b&lt;c&gt;` (the `&` stays: the escape branch runs before DOMPurify) |
| title (`title x`)                                                                                     | top-level strict, removeScript  | `a<b`              | `a` (DOMPurify drops the unfinished tag)                                |
| title                                                                                                 |                                 | `a=b &amp; c`      | unchanged (no `<`)                                                      |
| accTitle, accDescr                                                                                    | strict                          | `x<y`, `p<q`       | `x`, `p`                                                                |
| accTitle, accDescr                                                                                    |                                 | `spaced   title  ` | `spaced title` (grammar collapse plus DB `^\s+`)                        |

Message text: `commit msg:"  padded   msg  "` keeps its spaces (probe), so messages are not trimmed by the DB.

Branch names are sanitised before lookup, so `branch "a=b"` then `checkout "a=b"` matches (probe: the branch is `a&equals;b`, and `checkout` finds it).

### 2.7 Automatic ids

- `getID()` = `random({length: 7})` = seven calls to `Math.random()` (utils.ts 453-465).
- Commit: `id: id ? id : seq + '-' + getID()` (gitGraphAst 118). Merge: `customId \|\| seq + '-' + getID()` (244). Cherry-pick: always `seq + '-' + getID()` (347). The `seq` is read before `seq++`, so the first id is `0-…`.
- The `deterministicIds` / `deterministicIDSeed` config (`InitIDGenerator`, utils.ts 734-743) is **not** used here. Two runs of the same input give different labels, because the commit id is drawn as the commit label (renderer 350).
- No fixture has a commit without `id:` (survey: all 752 `commit` statements in fixture bodies carry an `id:`; the four `commit` lines without one are prose in descriptions). The spec inputs do (for example the "NO params" test, gitGraph.spec.ts 238-254, which only checks `not.toBeNull()`).

Port proposal: a source of ids behind a trait with a deterministic default (for example a 7-hex hash of seq and text). The differential exporter stubs `Math.random` in the reference page, and the comparison normalises ids matching `^\d+-[0-9a-f]{7}$` to `N-*`. Record this as a named exception.

### 2.8 Parser-level spec: recovery and error policy

`packages/parser/tests/gitGraph.test.ts` calls `LangiumParser.parse` directly (test-util 104-113), which returns errors instead of throwing. The mermaid entry throws on any error (parse.ts 147). Three tests need Langium's recovery, which the reference entry rejects (probe):

| Test (line)                                                        | Input                                                              | Reference through the entry point         |
| ------------------------------------------------------------------ | ------------------------------------------------------------------ | ----------------------------------------- |
| "should ignore malformed properties and not break parsing" (42-48) | `commit id:"2" msg:"…" oops:"ignored"`                             | error (`oops` unexpected)                 |
| "should ignore malformed properties in merge statements" (121-126) | `merge feature random:"ignored"`                                   | error                                     |
| "should parse cherry-picking with a commit id" (152-156)           | `commit id:"123" commit id:"321" cherry-pick id:"123"` on one line | error (the second `commit` is unexpected) |

The other 22 tests are AST checks that the reference also accepts. The two error tests (203-211) expect `parserErrors` to be non-empty and must become "fails, at line and column" (ADR 0002).

## 3. Canonical model proposal

Conventions as `docs-port/canonical-model.md`: `"version": 1`, camelCase, strings always present, `null` for a value the DB leaves undefined, numbers exact.

```json
{
  "version": 1,
  "title": "",
  "accTitle": "",
  "accDescr": "",
  "direction": "LR",
  "currentBranch": "main",
  "branches": [
    { "name": "main", "order": 0, "head": "2-5f0c1d9" },
    { "name": "dev", "order": 2, "head": null }
  ],
  "commits": [
    {
      "id": "init",
      "seq": 0,
      "branch": "main",
      "type": "NORMAL",
      "customType": null,
      "customId": null,
      "message": "",
      "tags": ["v0.1"],
      "parents": []
    },
    {
      "id": "M",
      "seq": 3,
      "branch": "main",
      "type": "MERGE",
      "customType": "REVERSE",
      "customId": true,
      "message": "merged branch dev into main",
      "tags": ["v1"],
      "parents": ["m1", "d1"]
    }
  ]
}
```

| Field                           | Source                                        | Rule                                                                                         |
| ------------------------------- | --------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `commits`                       | `db.getCommits()`, insertion order            | Keep the Map order; the renderer sorts by `seq` itself                                       |
| `commits[].id`                  | DB `id` (sanitised, auto id if empty)         | §2.6, §2.7                                                                                   |
| `commits[].seq`                 | DB `seq`                                      | Required: the reroute test and the sort use it                                               |
| `commits[].branch`              | DB `branch` (the current branch when created) | Also the branch-colour index                                                                 |
| `commits[].type`                | `commitType` number mapped to a name          | NORMAL, REVERSE, HIGHLIGHT, MERGE, CHERRY_PICK; names are readable and match the enum        |
| `commits[].customType`          | DB `customType`                               | Set only on merges; `null` when absent. Symbol uses `customType ?? type` (renderer 494, 604) |
| `commits[].customId`            | DB `customId`                                 | Set only on merges. `null` for other commits                                                 |
| `commits[].message`             | DB `msg`                                      | Kept. Merges: `merged branch X into Y` (245). Cherry-pick: `cherry-picked … into …` (348)    |
| `commits[].tags`                | DB `tags`                                     | Kept in source order. Commit tags keep empty strings; cherry-pick tags drop them (§2.5)      |
| `commits[].parents`             | DB `parents`                                  | Keep duplicates and self-loops, in order                                                     |
| `branches`                      | DB `branches` Map plus `branchConfig` order   | Insertion order; `head` is the commit id or `null`                                           |
| `branches[].order`              | `branchConfig.order`                          | Number (main: `mainBranchOrder`; others: `order:` or 0)                                      |
| `currentBranch`                 | `getCurrentBranch()`                          |                                                                                              |
| `direction`                     | `getDirection()`                              | LR by default                                                                                |
| `title`, `accTitle`, `accDescr` | DB getters                                    | Rules below                                                                                  |

Left out: `head` (equal to the current branch's head in every path, so derivable), `options`, `prettyPrint`, the fallback order of `getBranchesAsObjArray` (dead), the DB's global `seq` counter (only each commit's `seq` is kept), the render-only `allCommitsDict`, and all rendering state.

Title and accessibility rules:

- Title precedence: front matter title (set by `Diagram.fromText`, Diagram.ts 52-54), then the grammar `title` if non-empty (populateCommonDb 11-13), else `""`. The grammar's last `title` line wins even if empty, so an empty last line lets the front matter through (as the pie notes).
- Title value: TITLE converter (`titleRegex` `title([\t ][^\n\r]*\|)` with trim and blank collapse). `title: X` with a colon is a syntax error; `title X` is the form.
- accTitle: `accTitle:` value trimmed, blanks collapsed (converter), then `sanitizeText` with the top-level config, then `^\s+` removed (commonDb 17).
- accDescr: single line `accDescr: v` (converter 56-60) or block `accDescr { … }` (converter 63-67: per-line leading and trailing blanks removed, blank runs collapsed, runs of line breaks collapsed to one), then `sanitizeText` and `\n\s+` to `\n` (commonDb 23). The result for `accDescr {\n first\n\n\n second   line\n}` is `first\nsecond line` (probe).
- These are the same rules as pie and info (canonical-model.md 4-5). Titles use the strict sanitiser (§2.6).

Differential exporter (new `tools/port/export-gitgraph.ts`): needs `provideDomForSanitising` (sanitising runs on every id), the gitGraph detector registered (`reference.ts` `registerDetectors` registers pie, info and flowchart only), config scope set per input (front matter `config.gitGraph`, directives), and `Math.random` stubbed (§2.7).

## 4. Specs

| File                                                                | Declared `it`            | Active cases                        | Skipped | What it tests                                                                      | Level                                                                                       |
| ------------------------------------------------------------------- | ------------------------ | ----------------------------------- | ------- | ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `packages/mermaid/src/diagrams/git/gitGraph.spec.ts` (1391 lines)   | 73                       | 70 (the `for` loop at 1301 gives 2) | 4       | Parse and DB                                                                       | Parser and DB; no DOM; `vi.spyOn(log, 'warn')` in one test (1352)                           |
| `packages/parser/tests/gitGraph.test.ts` (213 lines)                | 27                       | 27                                  | 0       | Langium AST                                                                        | Parser only; three rely on recovery (§2.8)                                                  |
| `packages/mermaid/src/diagrams/git/gitGraphParser.ts` (in-source)   | 6                        | 6                                   | 0       | `parseStatement` and `populate` against a mock DB                                  | Parser to DB; no DOM                                                                        |
| `packages/mermaid/src/diagrams/git/gitGraphRenderer.ts` (in-source) | 9 declarations, 30 cases | 30                                  | 0       | `drawText`, branch positions, commit positions, class types, BT parallel positions | Renderer; `drawText` needs DOM (1099); the rest are pure with a stubbed DOMRect (1119-1141) |

Breakdown of `gitGraph.spec.ts` (line ranges by Python count):

| Group                                                     | Lines     | Active      | Skipped | Notes                                                                                                            |
| --------------------------------------------------------- | --------- | ----------- | ------- | ---------------------------------------------------------------------------------------------------------------- |
| basic gitGraph                                            | 9-236     | 10          | 4       | Reset (92, 111) and ff-merge (131, 191) are unsupported syntax (`reset` is not in the grammar), skipped upstream |
| advanced: commits, branches, merges, cherry-picks, errors | 237-1273  | 50          | 0       | Error tests use `rejects.toThrow` (1055-1099, 1119-1273) with exact DB messages (§2.5)                           |
| accessibility                                             | 1274-1299 | 2           | 0       | DB getters                                                                                                       |
| unsafe properties                                         | 1300-1321 | 1 (2 cases) | 0       | `__proto__` and `constructor` as ids and branch names (Map, so fine)                                             |
| warning                                                   | 1322-1360 | 1           | 0       | `Commit ID working on MDR already exists`                                                                        |
| config                                                    | 1361-1391 | 5           | 0       | `db.getConfig()` has the four keys                                                                               |

In-source renderer tests (`gitGraphRenderer.ts`): `drawText` (1098-1104) uses DOM; branchPosition (1106-1143) expects `setBranchPosition` values with two stub widths (34.9921875, 56.421875, from the test's own stub, not a measurement); commitPosition TB (1250-1270) and LR (1271-1291) give 8 cases each, the expected positions of the fixture commits (TB: y 40, 90, …, 390 in steps of 50; x 107.49609375 for feature and 224.03515625 for release); getCommitClassType (1292-1310) gives 8 cases; BT parallel (1311-1478) gives 2 cases and `add` (1478) one, for `findClosestParentBT`. These expected values are useful as port checks.

Other specs that mention gitGraph (not owned by the diagram): `packages/mermaid/src/utils.spec.ts` 299-305 (detection), `mermaidAPI.spec.ts` 862 (type table), `rendering-util/multi-diagram-id-uniqueness.spec.ts` 56 (a fixture string), `diagram-api/diagram-orchestration.spec.ts` 35, `diagram-api/diagram-beta-policy.spec.ts` 38. The M6 port needs only the detector entry from these.

Port classification:

- Parser and DB tests (about 100 cases, including skipped): port as Rust unit tests on the parser and the DB.
- Error tests (about 15): assert that parsing fails with the exact DB message (DB messages stay exact) or at a position (Langium messages do not).
- Config tests (5): the config model, not the DB. Port as "gitGraph section keys and defaults" in the config model.
- In-source renderer tests (30): port as renderer unit tests. `drawText` needs the Rust text measurement.

## 5. Renderer

### 5.1 Pipeline (renderer 998-1089)

```
draw(txt, id, ver, diagObj):
  clear()                                   # 999; branchPos, commitPos, allCommitsDict, maxPos=0, lanes=[], dir='LR'
  cfg = db.getConfig()                      # gitGraph section
  rotate = cfg.rotateCommitLabel ?? false  # 1008
  allCommitsDict = db.getCommits()          # 1009
  branches = db.getBranchesAsObjArray()     # 1010; sorted by order, stable
  dir = db.getDirection()                   # 1011
  {look, theme, themeVariables} = getConfig()          # 1016, top level
  if themeVariables.useGradient:            # 1018-1038
      <defs><linearGradient id=`${id}-gradient` x1=0% y1=0% x2=100% y2=0% gradientUnits=objectBoundingBox>
        <stop offset=0% stop-color=gradientStart/><stop offset=100% stop-color=gradientStop/>
  if look == 'neo' && theme in REDUX_GEOMETRY:  # 1042-1055
      <defs><filter id=`${id}-drop-shadow` height=130% width=130%><feDropShadow dx=4 dy=4 stdDeviation=0 flood-opacity=0.06 flood-color=filterColor/>
  pos = 0
  for (index, branch) in branches:          # 1059-1071: measure only
      bbox = getBBox(<text> with tspans of drawText(branch.name))
      pos = setBranchPosition(branch.name, pos, index, bbox, rotate)
  drawCommits(diagram, commits, modifyGraph=false)    # 1073: pass 1, positions only
  if showBranches: drawBranches(...)                  # 1074-1076: pushes lanes
  drawArrows(diagram, commits)                        # 1077
  drawCommits(diagram, commits, modifyGraph=true)     # 1078: pass 2, draws
  insertTitle(diagram, 'gitTitleText', titleTopMargin, title)   # 1080-1085 (title non-empty only)
  setupGraphViewbox(diagram, diagramPadding, useMaxWidth)       # 1088
```

Output order in the root `<svg>` (probe, reference output): the style element (inserted before draw, mermaidAPI 596-600); an empty `<g>` appended by `mermaidAPI.ts` 419; the optional `<defs>` (gradient, drop shadow); pass 1's `<g class="commit-bullets">` and `<g class="commit-labels">` (both empty); the optional `<g>` of branch lines and labels; `<g class="commit-arrows">`; pass 2's `commit-bullets` and `commit-labels` (drawn); the optional title `<text class="gitTitleText">`; `<title>`/`<desc>` from accessibility (a11y, mermaidAPI 623).

The DOM contains two `commit-bullets` and two `commit-labels` groups. Pass 1's are empty, but they stay in the document.

### 5.2 Constants and state

| Name                           | Value                                            | Line     |
| ------------------------------ | ------------------------------------------------ | -------- |
| `LAYOUT_OFFSET`                | 10                                               | 25       |
| `COMMIT_STEP`                  | 40                                               | 26       |
| `PX`, `PY`                     | 4, 2                                             | 27-28    |
| `THEME_COLOR_LIMIT` (renderer) | 8                                                | 30       |
| `REDUX_BRANCH_LABEL_PADDING_Y` | 12                                               | 39       |
| `defaultPos`                   | 30                                               | 68       |
| `REDUX_GEOMETRY_THEMES`        | redux, redux-dark, redux-color, redux-dark-color | 36       |
| `COLOR_THEMES`                 | redux-color, redux-dark-color                    | 45       |
| `DARK_THEMES`                  | dark, redux-dark, redux-dark-color, neo-dark     | 48       |
| `NEO_THEMES` (styles)          | neo, neo-dark                                    | styles 6 |

State: `branchPos: Map name to {pos, index}`, `commitPos: Map id to {x, y}`, `lanes: number[]` (grows during draw, reset at 80), `maxPos` (max of `pos` after each commit and drawBranches' end point), `dir`.

### 5.3 Colour index

```
calcColorIndex(raw, limit, avoidDefault):          # 56-65
  if avoidDefault and raw > 0: return ((raw - 1) % (limit - 1)) + 1
  return raw % limit
```

- Bullets, arrows and branch classes in commits use `limit` 8 and `avoidDefault` = colour theme (drawCommitBullet 247, 257, 303, 312, 325; drawArrow 676, 847).
- Branch lines and labels (drawBranches 879-881) use `limit` = theme `THEME_COLOR_LIMIT` (12) for redux geometry and 8 otherwise. So redux-color branch 8 to 11 get their own classes (`branch8`…`branch11`), while their commits wrap into 1…7.
- Class names: `commit{i}`, `commit-highlight{i}`, `arrow{i}`, `branch{i}`, `branch-label{i}`, `label{i}`.

### 5.4 Branch positions (setBranchPosition 986-996)

```
setBranchPosition(name, pos, index, bbox, rotate):
  branchPos[name] = {pos, index}
  pos += 50 + (rotate ? 40 : 0) + (dir in {TB, BT} ? bbox.width / 2 : 0)
  return pos
```

- Measured from the label of the **same** branch, before the increment, so the gap after branch i uses branch i's width. With `rotate` true (the default) the step is 90 (LR) and 90 + w/2 (TB/BT).
- Branches are indexed in the sorted order of `getBranchesAsObjArray`.

### 5.5 Commit positions (drawCommits 565-620, getCommitPosition 546-563)

```
drawCommits(svg, commits, modifyGraph, cfg):
  gBullets = svg.append('g.commit-bullets'); gLabels = svg.append('g.commit-labels')   # always
  pos = (dir in {TB, BT}) ? 30 : 0
  sorted = commits sorted by seq (stable)
  if dir == BT:
      if parallel: setParallelBTPos(sorted, pos=30)      # 586
      sorted = reverse(sorted)                           # 588
  for c in sorted:
      if parallel: pos = calculatePosition(c, dir, pos)   # 597
      cp = getCommitPosition(c, pos, parallel)
      if modifyGraph: drawCommitBullet, drawCommitLabel(pos), drawCommitTags(pos)   # 606-608
      commitPos[c.id] = (TB/BT) ? {x: cp.x, y: cp.posWithOffset} : {x: cp.posWithOffset, y: cp.y}   # 610-614
      pos = (BT && parallel) ? pos + 40 : pos + 40 + 10                    # 615
      maxPos = max(maxPos, pos)                                          # 616-618

getCommitPosition(c, pos, parallel):
  posWithOffset = (BT && parallel) ? pos : pos + 10                       # 551
  branchPosVal = branchPos[c.branch].pos
  x = (TB/BT) ? branchPosVal : posWithOffset                             # 553
  y = (TB/BT) ? posWithOffset : branchPosVal + (redux ? 12/2 + 1 : -2)   # 558-561
  return {x, y, posWithOffset}
```

- In LR, a branch is a lane (its `pos`, 50 or 90 apart) and time runs along x: commit i has `posWithOffset` 10 + 50 i without parallel commits.
- In TB/BT, time runs along y and a branch is a column.

calculatePosition (511-544), used only with parallel commits:

```
calculatePosition(c, dir, pos):
  if c.parents.length > 0:
      cp = findClosestParent(c.parents)          # by dir, see below; may be undefined
      if cp is defined:
          pp = commitPos[cp] ?? {0,0}
          TB:  return pp.y + 40
          BT:  return (commitPos[c.id] ?? {0,0}).y - 40
          LR:  return pp.x + 40
  else:
      TB: return 30
      BT: return commitPos[c.id].y - 40 ?? 0 - 40
      LR: return 0
  return 0                                       # no closest parent found
```

Note: the BT branch of the parent case reads the commit's own position (set in the first pass).

findClosestParent (101-124):

```
if dir == BT: cmp = (a <= b), target = +Inf
else:         cmp = (a >= b), target = 0
for p in parents:
   v = (TB or BT) ? commitPos[p].y : commitPos[p].x     (undefined if missing)
   if v !== undefined and cmp(v, target): closest = p; target = v
```

So LR and TB pick the parent furthest along the time axis (ties to the later parent), and BT picks the parent with the smallest y (ties to the later one).

setParallelBTPos (140-182), BT only:

```
setParallelBTPos(sorted, defaultPos=30):
  curPos = defaultPos; maxPosition = defaultPos; roots = []
  for key in sorted (ascending seq):
      if c.parents.length: curPos = calculateCommitPosition(c); maxPosition = max(curPos, maxPosition)   # parent.y + 40
      else: roots.push(c)
      setCommitPosition(c, curPos)                 # {x: branch.pos, y: curPos + 10}
  curPos = maxPosition
  for root in roots: setRootPosition(root, curPos, defaultPos)       # {x: branch.pos, y: curPos + 30}
  for key in sorted:
      if c.parents.length:
          cp = findClosestParentBT(c.parents)      # min y, ties later; throws if no position
          curPos = commitPos[cp].y - 40
          if curPos <= maxPosition: maxPosition = curPos
          commitPos[c.id] = {x: branch.pos, y: curPos - 10}
```

`drawCommits` runs this twice in BT parallel mode (pass 1 and pass 2). Pass 2 reads the positions pass 1 wrote. In a probe with one BT parallel fixture (two branches, merge), every arrow end equals a commit centre, so the two passes agree there. Whether they always agree is **unverified**; the port must run both passes in the same order.

### 5.6 Commit bullets (drawCommitBullet 226-329)

`symbol = commit.customType ?? commit.type`; `typeClass` from `getCommitClassType` (493-509, same symbol); `redux = REDUX_GEOMETRY`; `ci = calcColorIndex(branchIndex, 8, colourTheme)`.

| Symbol                             | Shapes (in order)                                                                                                                                         | Radii and offsets                                                                            | Classes                                                                                                                          |
| ---------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| HIGHLIGHT (2)                      | `rect` outer: x = x-10 (+3 redux), y = y-10 (+3 redux), w = h = 20 (14 redux); `rect` inner: x = x-6 (+2 redux), y = y-6 (+2 redux), w = h = 12 (8 redux) |                                                                                              | outer `commit ${id} commit-highlight${ci} commit-highlight-outer`; inner `commit ${id} commit${ci} commit-highlight-inner`       |
| CHERRY_PICK (4)                    | `circle` at (x, y); two white `circle` at (x-3, y+2) and (x+3, y+2); two `line` (x±3, y+1) to (x, y-5)                                                    | r 10 (7 redux); eyes r 2.75 (2.5 redux); fill `#000000` if dark else `#fff`; stroke the same | all `commit ${id} commit-cherry-pick`                                                                                            |
| NORMAL (0), REVERSE (1), MERGE (3) | `circle` at (x, y); MERGE adds a second `circle` (r 6, 5 redux); REVERSE adds `path` `M x-c,y-c L x+c,y+c M x-c,y+c L x+c,y-c` with c = 5 (4 redux)       | r 10 (7 redux)                                                                               | `commit ${id} commit${ci}`; MERGE inner `commit ${typeClass} ${id} commit${ci}`; REVERSE `commit ${typeClass} ${id} commit${ci}` |

A merge whose `customType` is NORMAL draws only the normal circle. A merge with `customType` REVERSE draws the cross.

### 5.7 Commit labels (drawCommitLabel 331-402)

Drawn when `commit.type != CHERRY_PICK` **and** (`commit.type != MERGE` **or** `commit.customId`) **and** `showCommitLabel`. The test uses `type`, not `customType`.

```
wrapper = g
labelBkg = rect.commit-label-bkg       # inserted first
text = text.commit-label x=pos y=cp.y+25 text=commit.id     (no split, no tspans)
bbox = text.getBBox()                  # MEASURED: commit label, 10px, bold in neo-gen themes
labelBkg.x = cp.posWithOffset - bbox.width/2 - 2     (PY)
labelBkg.y = cp.y + 13.5
labelBkg.width = bbox.width + 4;  height = bbox.height + 4
if TB or BT:
    labelBkg.x = cp.x - (bbox.width + 4*4 + 5);  labelBkg.y = cp.y - 12
    text.x = cp.x - (bbox.width + 16);           text.y = cp.y + bbox.height - 12
else:
    text.x = cp.posWithOffset - bbox.width/2
if rotate:                              # rotate is the setting, default true
    if TB or BT:
        text.transform = labelBkg.transform = rotate(-45, cp.x, cp.y)
    else:
        r_x = -7.5 - ((bbox.width + 10) / 25) * 9.5
        r_y = 10 + (bbox.width / 25) * 8.5
        wrapper.transform = translate(r_x, r_y) rotate(-45, pos, cp.y)     # pos, not posWithOffset
```

The expression order matters: JavaScript evaluates `((w + 10) / 25) * 9.5` left to right in doubles. The probe output shows `translate(-21.939999999999998, 19.520000000000003)` for a 28 px label, so Rust must use the same order.

### 5.8 Tags (drawCommitTags 404-491)

Drawn for every commit with tags, whether or not labels are shown.

```
if tags nonempty:
    tags.reverse()                      # IN PLACE on the commit (410-416)
    yOffset = 0; maxW = 0; maxH = 0; elems = []
    for v in tags:                      # reversed order
        rect = polygon (insert); hole = circle; tag = text.tag-label y = cp.y - 16 - yOffset, text v
        tb = tag.getBBox()              # MEASURED: tag label, 10px
        maxW = max(maxW, tb.width); maxH = max(maxH, tb.height)
        tag.x = cp.posWithOffset - tb.width/2
        elems.push({tag, hole, rect, yOffset}); yOffset += 20
    for e in elems:
        h2 = maxH/2; ly = cp.y - 19.2 - e.yOffset
        rect.class = tag-label-bkg; rect.points = [
            (pos - maxW/2 - 2, ly + 2), (pos - maxW/2 - 2, ly - 2),
            (posWithOffset - maxW/2 - 4, ly - h2 - 2), (posWithOffset + maxW/2 + 4, ly - h2 - 2),
            (posWithOffset + maxW/2 + 4, ly + h2 + 2), (posWithOffset - maxW/2 - 4, ly + h2 + 2) ]
        hole: cy = ly, cx = pos - maxW/2 + 2, r = 1.5, class tag-hole
        if TB or BT:
            yOrigin = pos + e.yOffset
            rect.points = [(cp.x, yOrigin+2), (cp.x, yOrigin-2), (cp.x+10, yOrigin-h2-2),
                           (cp.x+10+maxW+4, yOrigin-h2-2), (cp.x+10+maxW+4, yOrigin+h2+2), (cp.x+10, yOrigin+h2+2)]
            rect.transform = hole.transform = translate(12,12) rotate(45, cp.x, pos)
            hole.cx = cp.x + 2; hole.cy = yOrigin
            tag.x = cp.x + 5; tag.y = yOrigin + 3; tag.transform = translate(14,14) rotate(45, cp.x, pos)
```

Element order per tag: `polygon`, `circle.tag-hole`, `text.tag-label`. Tag labels are appended to the same `commit-labels` group.

### 5.9 Arrows (drawArrows 851-865, drawArrow 653-849, shouldRerouteArrow 622-636, findLane 638-651)

For each commit in Map order, for each parent in order: `drawArrow(parentCommit, commit)`. Output: `<path d=… class="arrow arrow{ci}">` in `g.commit-arrows`.

```
shouldRerouteArrow(A, B, p1, p2, all):
    B_furthest = (TB or BT) ? p1.x < p2.x : p1.y < p2.y
    curveBranch = B_furthest ? B.branch : A.branch
    return exists X in all: A.seq < X.seq < B.seq and X.branch == curveBranch

findLane(y1, y2, depth = 0):           # uses the shared `lanes` list
    cand = y1 + |y1 - y2| / 2
    if depth > 5: return cand           # no push
    if all(|lane - cand| >= 10 for lane in lanes): push cand; return cand
    return findLane(y1, y2 - |y1 - y2| / 5, depth + 1)
```

drawArrow:

- `p1 = commitPos[A]`, `p2 = commitPos[B]` (throws if either is missing).
- `ci = branchPos[B.branch].index`, but if `B.type == MERGE` and `A.id != B.parents[0]`, use `A.branch`'s index (677-679).
- If rerouted: `arc = 'A 10 10, 0, 0, 0,'`, `arc2 = 'A 10 10, 0, 0, 1,'`, r = o = 10. Then **both** `lineY = findLane(...)` and `lineX = findLane(...)` run (688-690), in that order, whatever the direction.
- If not rerouted: r = o = 20, arcs `A 20 20, 0, 0, 0,` and `A 20 20, 0, 0, 1,`.

Path shapes (not rerouted; `M` = move, `L` = line; `B.merge2` = B is MERGE and A is not its first parent):

| Direction | Case                | Path                                            |
| --------- | ------------------- | ----------------------------------------------- |
| TB        | p1.x < p2.x, merge2 | `M p1 L p1.x (p2.y-r) arc (p1.x+o, p2.y) L p2`  |
| TB        | p1.x < p2.x         | `M p1 L (p2.x-r) p1.y arc2 (p2.x, p1.y+o) L p2` |
| TB        | p1.x > p2.x, merge2 | `M p1 L p1.x (p2.y-r) arc2 (p1.x-o, p2.y) L p2` |
| TB        | p1.x > p2.x         | `M p1 L (p2.x+r) p1.y arc (p2.x, p1.y+o) L p2`  |
| TB        | p1.x == p2.x        | `M p1 L p2`                                     |
| BT        | p1.x < p2.x, merge2 | `M p1 L p1.x (p2.y+r) arc2 (p1.x+o, p2.y) L p2` |
| BT        | p1.x < p2.x         | `M p1 L (p2.x-r) p1.y arc (p2.x, p1.y-o) L p2`  |
| BT        | p1.x > p2.x, merge2 | `M p1 L p1.x (p2.y+r) arc (p1.x-o, p2.y) L p2`  |
| BT        | p1.x > p2.x         | `M p1 L (p2.x+r) p1.y arc2 (p2.x, p1.y-o) L p2` |
| BT        | p1.x == p2.x        | `M p1 L p2`                                     |
| LR        | p1.y < p2.y, merge2 | `M p1 L (p2.x-r) p1.y arc2 (p2.x, p1.y+o) L p2` |
| LR        | p1.y < p2.y         | `M p1 L p1.x (p2.y-r) arc (p1.x+o, p2.y) L p2`  |
| LR        | p1.y > p2.y, merge2 | `M p1 L (p2.x-r) p1.y arc (p2.x, p1.y-o) L p2`  |
| LR        | p1.y > p2.y         | `M p1 L p1.x (p2.y+r) arc2 (p1.x+o, p2.y) L p2` |
| LR        | p1.y == p2.y        | `M p1 L p2`                                     |

Rerouted (r = o = 10). The `else` branch also covers equal x, which the code treats as "source right of destination":

| Direction | Case        | Path                                                                                    | Class from |
| --------- | ----------- | --------------------------------------------------------------------------------------- | ---------- |
| TB        | p1.x < p2.x | `M p1 L (lineX-r) p1.y arc2 (lineX, p1.y+o) L (lineX, p2.y-r) arc (lineX+o, p2.y) L p2` | B          |
| TB        | else        | `M p1 L (lineX+r) p1.y arc (lineX, p1.y+o) L (lineX, p2.y-r) arc2 (lineX-o, p2.y) L p2` | A          |
| BT        | p1.x < p2.x | `M p1 L (lineX-r) p1.y arc (lineX, p1.y-o) L (lineX, p2.y+r) arc2 (lineX+o, p2.y) L p2` | B          |
| BT        | else        | `M p1 L (lineX+r) p1.y arc2 (lineX, p1.y-o) L (lineX, p2.y+r) arc (lineX-o, p2.y) L p2` | A          |
| LR        | p1.y < p2.y | `M p1 L p1.x (lineY-r) arc (p1.x+o, lineY) L (p2.x-r, lineY) arc2 (p2.x, lineY+o) L p2` | B          |
| LR        | else        | `M p1 L p1.x (lineY+r) arc2 (p1.x+o, lineY) L (p2.x-r, lineY) arc (p2.x, lineY-o) L p2` | A          |

The "class from" column is the colour rule; in the code some branches set `colorClassNum` again (704, 718, 734).

Lanes: `lanes` is filled by drawBranches (spine y of each branch, 915) and by findLane. Without `showBranches`, the lanes differ, so the reroute positions differ. The port must replicate both.

### 5.10 Branch lines and labels (drawBranches 867-984, setBranchPosition above)

```
drawBranches(svg, branches, cfg, id):
  g = svg.append('g')
  for (index, b) in branches:
      adjust = calcColorIndex(index, redux ? theme.THEME_COLOR_LIMIT : 8, colourTheme)
      pos = branchPos[b.name].pos
      spineY = (TB or BT) ? pos : (redux ? pos + 12/2 + 1 : pos - 2)        # 891-896
      line.branch{adjust}: x1=0, y1=spineY, x2=maxPos, y2=spineY           # 897-901
      if TB: y1=30, x1=pos, y2=maxPos, x2=pos                              # 904-908
      if BT: y1=maxPos, x1=pos, y2=30, x2=pos                              # 909-914
      lanes.push(spineY)                                                    # 915
      label text = drawText(b.name); bbox = label.getBBox()                 # MEASURED: branch name, 16px
      bkg = rect.branchLabelBkg.label{adjust}: rx=ry=(redux ? 0 : 4)
            style = neo ? 'filter:' + (redux ? url(#id-drop-shadow) : dropShadow) : ''    # style="" for classic
            x = -bbox.width - 4 - (rotate ? 30 : 0)
            y = -bbox.height/2 + 10
            width = bbox.width + 18 + (redux ? 16 : 0)
            height = bbox.height + 4 + (redux ? 12 : 0)
            transform = translate(-19, spineY - 12 - (redux ? 6 : 0))           # LR
      label g.label.branch-label{adjust}, transform = translate(
            -bbox.width - 14 - (rotate ? 30 : 0) + (redux ? 8 : 0), spineY - bbox.height/2 - 2)     # LR
      if TB:  bkg.x = pos - w/2 - 10, bkg.y = 0; label.transform = translate(pos - w/2 - 5, 0)
              if redux: bkg.transform = translate(-8 - 3, -12 - 10)  (-labelPaddingX/2 - 3 = -11, -labelPaddingY - 10 = -22)
                        label.transform = translate(pos - w/2 - 5, -2*12 + 7)
      if BT:  bkg.x = pos - w/2 - 10, bkg.y = maxPos; label.transform = translate(pos - w/2 - 5, maxPos)
              if redux: bkg.transform = translate(-8 - 3, 12 + 10); label.transform = translate(pos - w/2 - 5, maxPos + 24 + 4)
```

Notes: the text `<text>` contains one `<tspan xml:space="preserve" dy="1em" x="0" class="row">` per line (`drawText`, 84-99). Labels split on backslash-n, LF or `<br>` (case-insensitive) and trimmed. The branch line class is `branch branch{adjust}`. The reference draws every label; there is no clipping.

### 5.11 Title and viewBox (utils 793-813; setupGraphViewbox 49-72)

```
if title nonempty:
    bounds = diagram.getBBox()        # at this point: all elements drawn so far
    text.gitTitleText text-anchor=middle x = bounds.x + bounds.width/2 y = -titleTopMargin (25) text=title
setupGraphViewbox:
    b = diagram.getBBox()             # whole svg, title included
    width  = b.width + 2*padding (8); height = b.height + 2*padding
    svg: width="100%" style="max-width: {width}px;" (useMaxWidth) — no height attribute
    viewBox = "{b.x - padding} {b.y - padding} {b.width + 2*padding} {b.height + 2*padding}"
```

The viewBox is the union of the children's bounding boxes, in root coordinates. Chromium's `getBBox` gives the tight box of paths (arcs included), of text (glyph cells with the font's ascent and descent), of polygons, circles, rects and lines, and maps children through their own transforms. The transforms here are `rotate` and `translate` on groups and elements. **Unverified:** whether Chromium maps a rotated child by its four corners (an axis-aligned hull) or by the exact shape. This decides the viewBox of every fixture with rotated labels (`rotateCommitLabel` true is the default). M5 found that `getBBox` returns float32 values; the same applies here.

### 5.12 Text measurements (every call to getBBox)

| #   | Element                                     | Font (CSS)                                                                                                        | Used for                                                                                     | Line                               |
| --- | ------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- | ---------------------------------- |
| T1  | branch name `text` with tspans (`drawText`) | 16px (root `font-size`); `font-family var(--mermaid-font-family)`; weight 600 for redux-colour themes (styles 91) | setBranchPosition (TB width) in the measuring loop, and the branch label box in drawBranches | renderer 1065, 929                 |
| T2  | commit id `text.commit-label`               | `commitLabelFontSize` (10px); weight 600 in neo-gen themes (styles 140)                                           | label box and rotation (5.7)                                                                 | 351                                |
| T3  | tag `text.tag-label`                        | `tagLabelFontSize` (10px)                                                                                         | tag box, max width and height                                                                | 424                                |
| T4  | root `svg`                                  | all of the above, plus the title (18px, `.gitTitleText`)                                                          | viewBox and title x                                                                          | 50 (setupGraphViewbox), 18 (utils) |

Each `getBBox` of a `text` gives width = sum of the run advances (Chromium rounds each advance up to 1/64 px, M5 notes), and height = the line box from the rounded ascent and descent (M5: ascent and descent each rounded to a whole pixel, a tie going down, after flooring the size to 1/100 px; `ferrigura-text/src/metrics.rs` 1-80 already does this). A multi-line T1 has `dy = 1em` per tspan: the height is about `16·(N−1) + ascent + descent` (**unverified**, to check in Chromium). Only T1 can be multi-line.

Bold: the neo-gen weight 600 needs a face choice in ferrigura (`fonts.rs` `face(family, weight, style)` at 224). Whether Lexica's Bold face is what Chromium picks for 600 is **unverified**.

### 5.13 SVG structure and attributes (from the probe render)

- Root: `<svg id width="100%" xmlns xmlns:xlink style="max-width: Wpx;" viewBox role="graphics-document document" aria-roledescription="gitGraph">`. No `height` (useMaxWidth).
- Numbers are JS numbers written by d3 `attr` (`String(n)`), and in template strings: `107.49609375`, `-21.939999999999998`, `19.520000000000003`. Rust must reproduce the same doubles (same operation order) and the shortest round-trip text.
- Element names and classes follow §5.5 to §5.10 exactly. Example (LR, probe, stubbed widths): `<path d="M 10 -2 L 10 68 A 20 20, 0, 0, 0, 30 88 L 60 88" class="arrow arrow1">`, `<circle cx="10" cy="-2" r="10" class="commit init commit0">`, `<rect x="50" y="78" width="20" height="20" class="commit d1 commit-highlight1 commit-highlight-outer">`, `<path d="M 105,-7L115,3M105,3L115,-7" class="commit commit-reverse m1 commit0">`, the label `<g class="label branch-label0" transform="translate(-72, -12)">`.
- Tag polygon points are written as multi-line template strings. Parsed as numbers they are the same as the single-line form.
- Empty style attribute on branch background in the classic look: `style=""`.
- Drop-shadow filter `feDropShadow` (camel case, created with `createElementNS` in the browser; jsdom lowercased it in the probe, so the probe output is not authoritative for this one name).
- Accessibility: `<title>` and `<desc>` as first children, `aria-labelledby` and `aria-describedby` (accessibility.ts 45-60).

### 5.14 Theme dependence (the important risk)

The renderer and the stylesheet branch on the **theme name**:

| Name list                                                                  | Used at                                   | Effect                                                              |
| -------------------------------------------------------------------------- | ----------------------------------------- | ------------------------------------------------------------------- |
| `REDUX_GEOMETRY_THEMES` (redux, redux-dark, redux-color, redux-dark-color) | renderer 36, 235, 557, 875, 930-938, 1042 | radii 7 and 5, box offsets, label padding, font weight, drop shadow |
| `COLOR_THEMES` (redux-color, redux-dark-color)                             | renderer 45, 236, 660, 876                | colour cycling without colour 0; styles use `borderColorArray`      |
| `DARK_THEMES` (dark, redux-dark, redux-dark-color, neo-dark)               | renderer 237, styles 7                    | cherry-pick eyes colour; label fills                                |
| `NEO_THEMES` (neo, neo-dark)                                               | styles 6, 35                              | neo palette `git{i}` (no `borderColorArray`)                        |
| `NEO_COLOR_GEN_THEMES`                                                     | styles 8-15, 121                          | neo-style `commit-label`, tag and merge styles                      |

Shipped in ferrigura: `redux-color` and `redux-dark-color` only (M2). The other names fall back by darkness (M2 notes): `redux`, `neo` → `redux-color`; `redux-dark`, `neo-dark` → `redux-dark-color`; **`default` → `redux-color`**.

Fixtures per theme (features script): `default` 105 (no theme key), `neo` 5, `neo-dark` 5, `redux` 5, `redux-dark` 5, `redux-color` 5, `redux-dark-color` 5. So the fallback is exact for 10 fixtures and wrong for 20 (it changes the geometry flags). It is wrong for all 105 `default` fixtures, which need `theme-default.js` (`useGradient` false, own git values, `strokeWidth` 1, THEME_COLOR_LIMIT 12) and the `normalTheme` path. M2 left the `default` theme unported ("drawn with one of the two"). So for gitGraph this is a blocker or a named exclusion, and the M5 rule "replace an unshipped theme by its fallback on both sides" must not be used for gitGraph. The harness must keep the name, or the port must ship `default`.

## 6. Fixtures

e2e: `e2e/diagrams/git/` has 135 `.mmd` files: `nightly-check` 95, `pr-check` 10, `visual-check` 30 (the tier names are CI's, `e2e/rendering/mmd-snapshots.spec.ts`). All 135 have a front matter `description`. The fixtures are screenshot tests in the e2e suite and have no separate geometry; the M6 geometry comparison uses them as inputs.

Other inputs: 39 docs blocks with gitGraph in 5 files (`packages/mermaid/src/docs/syntax/gitgraph.md` 35, and `intro/examples.md`, `config/accessibility.md`, `syntax/examples.md`, `community/contributing.md`, one each). Spec string literals (gitGraph.spec.ts, gitGraph.test.ts, utils.spec.ts).

### 6.1 Feature counts (files)

| Feature                                             | Files                                    | Feature                                         | Files                           |
| --------------------------------------------------- | ---------------------------------------- | ----------------------------------------------- | ------------------------------- |
| Header LR (explicit) / TB / BT / LR default         | 5 / 33 / 36 / 61                         | `look: neo`                                     | 30                              |
| `theme` unset (default)                             | 105                                      | theme neo / neo-dark / redux / redux-dark       | 5 each                          |
| theme redux-color / redux-dark-color                | 5 each                                   | `parallelCommits: true`                         | 17                              |
| `showBranches: false`                               | 10                                       | `showCommitLabel: false`                        | 10                              |
| `rotateCommitLabel` set (front matter or directive) | 24                                       | `mainBranchName` / `mainBranchOrder` set        | 7 / 7                           |
| directive `%%{init}%%` present                      | 9 (the `logLevel: debug` key is ignored) | `themeVariables` gitBranchLabel0–7 in directive | 3                               |
| merge statements in file                            | 72                                       | merge with `id:`                                | 40                              |
| merge with `tag:` / `type:`                         | 36 / 11                                  | commit with `tag:`                              | 35                              |
| commit with two or more tags                        | 2                                        | REVERSE / HIGHLIGHT commit or merge             | 39 / 38                         |
| cherry-pick                                         | 33                                       | cherry-pick with `parent:`                      | 7                               |
| branch with `order:`                                | 9                                        | `switch` used                                   | 2                               |
| quoted branch names / quoted checkout               | 19 / 18                                  | more than 8 branches                            | 7                               |
| front matter title (column-0 `title:`)              | 1                                        | accTitle / accDescr                             | 0                               |
| commit without id (body)                            | 0                                        | semicolons in statements                        | 0 (one `;` is in a description) |

Statement totals (all fixtures): commit 752, branch 256, checkout 423, switch 19, merge 126, cherry-pick 37.

Config comes from front matter (`config:`, 62 files; `look`, `theme`) and directives (9 files).

### 6.2 Browser features needed

- Text measurement with `getBBox`: T1 to T4 (§5.12), including a 600 weight in redux-colour and neo-gen themes.
- The viewBox from the union `getBBox` of all elements with transforms and arcs (§5.11).
- The CSS cascade for measurement: `.commit-label` and `.tag-label` set font sizes through the stylesheet, so the measurement uses those sizes, not the root's 16px.
- Nothing else: no `Math.random` in fixtures, no images, no fonts other than Lexica, no scripts, no ELK.

## 7. Risks, effort, open questions

### 7.1 Risks (highest first)

1. **Theme `default` (105 of 135 fixtures).** Ferrigura ships two themes; `default` is not one. The name-based branches (§5.14) make fallback wrong for geometry. Decision needed: port the `default` theme variables (the code is already in `theme-default.js`; M2's theme port handles the khroma functions) or name a gitGraph exclusion. Recommendation: port it, since the classic look is the default look.
2. **Lexer compatibility (§2.3).** A logos lexer would take the longest match and accept `titleX`, `1a`, and `café`. The Rust lexer must be first-match for TITLE, INT and STRING, and ASCII-only for REFERENCE. Recommendation: hand-written scanner, as flowchart (ADR 0002 amendment as M3 open question 1).
3. **Measured boxes (§5.11, 5.12).** The viewBox and every label box depend on Chromium's getBBox (ascent and descent rounding, 1/64 px advances, bold 600, arc bounds, transformed bounds). Expect the same work as M5 for text plus new arc and transform handling. Unverified items are listed in 7.4.
4. **Two passes in BT parallel mode and the shared `lanes` list (§5.5, 5.9).** Replicating the sequence exactly is required; both are order-dependent.
5. **Sanitiser escape path (§2.6).** The gitGraph section lacks `securityLevel`, so `=` becomes `&equals;` in ids. This is the reference's behaviour; it must be replicated, and ferrigura's `sanitize_text` has no `sanitizeMore` pre-step (it implements only the DOMPurify subset). A helper is needed for this branch.
6. **Random ids (§2.7).** No fixture is affected; spec inputs are. A deterministic scheme and a named exception.
7. **Known quirks to keep:** merge check (a) by head branch; duplicate commit id only warns, leaves a self-loop; cherry-pick duplicate parents; `Current branch (x)has no commits` with no space; `tagLabelBorder` ignores user `tagBorder`; `rotateCommitLabel` default true.

### 7.2 Effort, relative to the flowchart

Estimates, not measurements.

| Part                             | Relative to M3 (flowchart parser) | Relative to M5 (flowchart rendering) | Note                                                                               |
| -------------------------------- | --------------------------------- | ------------------------------------ | ---------------------------------------------------------------------------------- |
| Scanner, grammar, AST            | about 1/5                         |                                      | 56-line grammar, 11 terminals; the lexer traps (§2.3) take most time               |
| DB and errors                    | about 1/4                         |                                      | 13 messages, 2 quirks, one clean method per statement                              |
| Model, exporter, harness glue    | about 1/3                         |                                      | exporter needs DOMPurify and the detector                                          |
| Renderer                         |                                   | about 1/3                            | no ELK, no shapes, no markdown; but getBBox, transforms and two themes of geometry |
| Theme `default` port (if chosen) |                                   |                                      | not counted; M2 already has the khroma functions                                   |
| Specs                            | about 1/3                         |                                      | about 100 parser and DB cases, 30 renderer cases                                   |

Total: about a third of M3 and M5 together, most of it in the renderer's text and bounds work and the lexer traps.

### 7.3 Open questions for the coordinating session

1. `default` theme: port its variables, or exclude 105 fixtures (§5.14)? Recommendation: port.
2. Lexer: hand-written for gitGraph too (as flowchart)? Recommendation: yes; amend ADR 0002 as M3 proposed.
3. Auto ids: deterministic scheme in ferrigura, and normalise `N-hex7` in the differential run? Recommendation: yes, named exception.
4. Parser-level recovery tests (§2.8): assert failure with a position for the three tests? Recommendation: yes.
5. Keep the reference's quirks (§7.1.7) or fix them with named exceptions? Recommendation: keep the duplicate-id and merge-check behaviour (they change the diagram), and keep the spelling "has no commits" as is.
6. The sanitiser helper for the gitGraph section: in `ferrigura-syntax/src/sanitize.rs` next to `sanitize_text`? Recommendation: yes.
7. Does the M6 flowchart-style harness need `securityLevel` and `htmlLabels` pinned for gitGraph too? The section has neither; the top-level values only matter for titles (§2.6). Recommendation: pin the top-level values and leave the section as the reference has it.
8. `setOptions`/`getOptions` (dead) and the `nodeLabel` key (dead): leave both out of the model and the config type? Recommendation: keep `nodeLabel` in the config schema (generated), leave the DB functions out.

### 7.4 Unverified (for the port's first Chromium comparison)

- Whether Chromium maps a rotated child's bbox by its corners or exactly, for the viewBox (§5.11).
- The height of a multi-line branch label `text` (§5.12).
- Whether Lexica's Bold face is what Chromium uses for weight 600 (§5.12).
- Whether BT parallel pass 1 and pass 2 give the same positions in general (§5.5); one probe agreed.
- The exact `getBBox` of `text` with `dy` tspans and of rotated groups (§5.7, 5.8).
- Arc bounds in `getBBox` for the arrow paths (§5.9, 5.11).
- Whether `drawText` splits on a literal backslash-n as the regex `\\n` says (§5.10): from the code only.
- Parser: `branch ""` produces an empty-named branch; not compared with any fixture.
- Whether the reference renders a duplicate-id self-loop without error in every layout (probe checked the DB only).
- The Langium `commit` keyword longer-alternative rule as it applies to every keyword (probed for `commitX`, `checkout02` and `titleX`, not for all keywords).
