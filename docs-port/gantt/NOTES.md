# M6 gantt: research notes

Research only. No repository file was changed. Reference: this worktree at `04b952b4e` (`sebastian/port-m6-p1-diagrams`).

Labels used below:

- **read**: taken from the source; the file and line are cited.
- **probed**: run in Node, in the scratchpad `m6-gantt/` (outside the repo). Node v24.21.0, dayjs 1.11.21, d3 7.9.0 (d3-scale 4.0.2, d3-time 3.1.0, d3-time-format 4.1.0, d3-axis 3.0.0), jison 0.4.18, jison-lex 0.3.4, jsdom 26.1.0. The DB is `ganttDb.js` copied with its imports stubbed. The renderer is `ganttRenderer.js` copied with its imports stubbed and run under jsdom. `getBBox` is replaced by a Courier-like estimate (0.6 em per character) and `offsetWidth` by 1200.
- **unverified**: not confirmed in a browser or against an e2e baseline.

Unless a probe says otherwise, the DB runs with the clock pinned to `1010-10-10T00:00Z` and the zone `TZ=UTC`, the same clock as `e2e/rendering/mmd-snapshots.spec.ts:34-36`.

---

## 0. Summary

- Gantt is a single Jison grammar (`parser/gantt.jison`, 188 lines), one DB (`ganttDb.js`, 899 lines) and one renderer (`ganttRenderer.js`, 900 lines). It uses d3-scale, d3-time, d3-time-format and d3-axis, and dayjs for dates. There is no layout engine, no shape registry and no Markdown.
- The hard part is the date semantics, not the drawing. They depend on dayjs strict parsing, on V8's `new Date(string)` fallback, on the clock, and on the time zone. Section 3 covers them.
- Renderer DOM dependencies: `offsetWidth` (width, only when `useWidth` is unset) and `getBBox().width` (task label placement). Everything else is arithmetic.
- All 52 fixtures parse and compile in the reference with no error and no unresolved task (probed, `TZ=UTC`, clock pinned). Errors come only from malformed input (2.6).
- No renderer or style spec exists. The 2 spec files (37 `it` calls, 82 cases once `it.each` rows are counted) are DB and parser tests. The renderer is verified only through e2e screenshots, so the port needs its own geometry comparison (section 8).

---

## 1. Sources

### 1.1 Files

| Role                    | File                                                     | Lines                                                                                  | Notes                                                  |
| ----------------------- | -------------------------------------------------------- | -------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| Lexer and grammar       | `packages/mermaid/src/diagrams/gantt/parser/gantt.jison` | 6-99 lexer, 101-188 grammar                                                            | `%options case-insensitive` (8)                        |
| Parser spec             | `…/gantt/parser/gantt.spec.js`                           | 1-295                                                                                  | 15 `it`, 3 `it.each`                                   |
| DB                      | `…/gantt/ganttDb.js`                                     | 1-899                                                                                  | see section 3                                          |
| DB spec                 | `…/gantt/ganttDb.spec.ts`                                | 1-620                                                                                  | 22 `it`, 6 `it.each`                                   |
| Detector                | `…/gantt/ganttDetector.ts`                               | 10                                                                                     | `/^\s*gantt/`, case-sensitive                          |
| Diagram definition      | `…/gantt/ganttDiagram.ts`                                | 8-13                                                                                   | parser, db, renderer, styles                           |
| Renderer                | `…/gantt/ganttRenderer.js`                               | 1-900                                                                                  | see section 6                                          |
| Styles                  | `…/gantt/styles.js`                                      | 1-290                                                                                  | theme variables, 6.12                                  |
| Pipeline                | `packages/mermaid/src/Diagram.ts`                        | 26-29, 31, 44-54, 66-77, 83                                                            | clear, title, encode, parse                            |
| Preprocess              | `packages/mermaid/src/preprocess.ts`                     | 6-17, 19-31, 33-45                                                                     | CRLF, attribute quotes, front matter, directives       |
| Front matter            | `packages/mermaid/src/diagram-api/frontmatter.ts`        | 24-69 (`displayMode` 55-57)                                                            | `displayMode` goes to config, not to the DB            |
| Comments                | `packages/mermaid/src/diagram-api/comments.ts`           | 6-8                                                                                    | `cleanupComments`                                      |
| Entities                | `packages/mermaid/src/utils.ts`                          | 874-896 encode, 904-906 decode                                                         | decode applied to the final SVG at `mermaidAPI.ts:359` |
| Common DB               | `packages/mermaid/src/diagrams/common/commonDb.ts`       | 8, 16-18, 22-24, 28-30                                                                 | accTitle, accDescr, title setters                      |
| Sanitise                | `packages/mermaid/src/diagrams/common/common.ts`         | 8 `lineBreakRegex`, 48-53, 56-68, 71-82                                                | DOMPurify path                                         |
| Container               | `packages/mermaid/src/mermaidAPI.ts`                     | 397-418 `appendDivSvgG`, 503-509, 672, 757                                             | div and svg created; a11y attributes                   |
| Accessibility           | `packages/mermaid/src/accessibility.ts`                  | 19, 22-33                                                                              | `role`, `aria-roledescription`                         |
| Size and viewBox helper | `packages/mermaid/src/setupGraphViewbox.js`              | 23-33, 43-47                                                                           | `width="100%"` + `max-width`                           |
| Defaults                | `packages/mermaid/src/defaultConfig.ts`                  | 84-88                                                                                  | `tickInterval` and `useWidth` undefined                |
| Config schema           | `packages/mermaid/src/schemas/config.schema.yaml`        | 2026-2146 (`GanttDiagramConfig`), 1023 (`titleTopMargin` 25), 470 (`useMaxWidth` true) | section 1.4                                            |
| Config types            | `packages/mermaid/src/config.type.ts`                    | 837                                                                                    | generated shape                                        |

### 1.2 Libraries

| Library                 | Installed                   | Used for                                                                          | Where                                |
| ----------------------- | --------------------------- | --------------------------------------------------------------------------------- | ------------------------------------ |
| dayjs                   | 1.11.21                     | parse, format, add, diff, `isoWeekday`, `dayjs.duration`                          | `ganttDb.js:2`, `ganttRenderer.js:1` |
| dayjs customParseFormat | same                        | strict parse with a format                                                        | `ganttDb.js:4,21`                    |
| dayjs isoWeek           | same                        | `isoWeekday()` (weekends)                                                         | `ganttDb.js:3,20,192`                |
| dayjs advancedFormat    | same                        | the strict round trip for `X` and `x` (needed, probed)                            | `ganttDb.js:5,22`                    |
| dayjs duration          | same                        | tick count estimate                                                               | `ganttRenderer.js:2,32,632`          |
| d3                      | 7.9.0                       | `scaleTime`, `rangeRound`, `axisBottom/Top`, `timeFormat`, time intervals         | `ganttRenderer.js:4-27`              |
| d3-interpolate          | 3.0.1                       | `interpolateHcl` imported and used only by a dead colour scale (208-211, 224-225) | `ganttRenderer.js:10`                |
| @braintree/sanitize-url | 7.1.2                       | link URLs (not loose)                                                             | `ganttDb.js:1,714-716`               |
| dompurify               | 3.4.12 (installed)          | title and accessibility texts                                                     | `common.ts:1,48-82`                  |
| js-yaml                 | root                        | front matter                                                                      | `frontmatter.ts:4`                   |
| jison                   | 0.4.18 (`package.json:110`) | grammar to parser; lexer from jison-lex 0.3.4                                     |                                      |

Dead in the reference: `interpolateHcl`, `colorScale`, and the `theColorScale` parameter (never read). No d3-color or HCL arithmetic is needed.

### 1.3 Build and detection

- `ganttDiagram.ts:8-13` registers `{parser, db, renderer, styles}`. The loader imports `./parser/gantt.jison` (Jison-generated).
- Detection runs on the cleaned text (`Diagram.ts:31`). `ganttDetector.ts:10` is `/^\s*gantt/`, which is case-sensitive and a prefix match (`ganttX` also matches).

### 1.4 Config section (`config.schema.yaml`, `GanttDiagramConfig` 2026-2146)

| Key                  | Default                     | Line      | Used at                                    |
| -------------------- | --------------------------- | --------- | ------------------------------------------ |
| titleTopMargin       | 25 (via GitGraph ref, 1023) | 2048-2050 | 194, 869-870                               |
| barHeight            | 20                          | 2054      | 140, 203, 295, 435                         |
| barGap               | 4                           | 2059      | 140, 204                                   |
| topPadding           | 50                          | 2065      | 125, 205                                   |
| rightPadding         | 75                          | 2071      | 164, 262                                   |
| leftPadding          | 75                          | 2077      | 164, 223                                   |
| gridLineStartPadding | 35                          | 2082      | 304, 319, 429, 599, 658, 732               |
| fontSize             | 11                          | 2087      | 399, 435                                   |
| sectionFontSize      | 11                          | 2094      | 839                                        |
| numberSectionStyles  | 4                           | 2099      | 268, 349, 455, 843                         |
| axisFormat           | `%Y-%m-%d`                  | 2107      | 654 (fallback only)                        |
| tickInterval         | none                        | 2108-2118 | 663 (`defaultConfig.ts:86` sets undefined) |
| topAxis              | false                       | 2123      | 730                                        |
| displayMode          | `''` (`compact` enables)    | 2133      | 126                                        |
| weekday              | sunday                      | 2146      | 675, 744                                   |
| useMaxWidth (base)   | true                        | 470       | 188 (via `configureSvgSize`)               |
| useWidth (base)      | none                        | 462       | 105-107                                    |

`BaseDiagramConfig` also gives `theme`, `look` and `layout` (no defaults for gantt). `useWidth` is undefined by default (`defaultConfig.ts:86-87`), so the width comes from the DOM unless the user sets it.

### 1.5 Theme variables (styles.js, read)

| Variable                                                | styles.js lines                 |
| ------------------------------------------------------- | ------------------------------- |
| fontFamily                                              | 4, 48, 61, 89, 95, 286          |
| excludeBkgColor                                         | 8                               |
| sectionBkgColor / sectionBkgColor2 / altSectionBkgColor | 17 / 21 / 26                    |
| titleColor                                              | 31, 35, 39, 43, 285             |
| gridColor                                               | 55                              |
| textColor                                               | 62 (and 285 when no titleColor) |
| todayLineColor                                          | 74                              |
| taskTextDarkColor                                       | 93, 172, 191, 250, 279          |
| taskTextOutsideColor                                    | 149, 154, 204, 262              |
| taskTextColor                                           | 135                             |
| taskBkgColor / taskBorderColor                          | 142 / 143                       |
| activeTaskBkgColor / activeTaskBorderColor              | 164, 224 / 165                  |
| doneTaskBkgColor / doneTaskBorderColor                  | 183, 233 / 182                  |
| critBkgColor / critBorderColor                          | 215 / 214, 223, 232             |
| vertLineColor                                           | 266, 272                        |
| taskTextClickableColor                                  | 112, 118, 124                   |

All 21 names are defined in `themes/theme-redux-color.js` (and in the other theme files). Ferrigura ships only `redux-color` and `redux-dark-color` (STATUS M2), so other theme names fall back by darkness. Colours are not part of the geometry comparison.

### 1.6 e2e harness

| Fact                                                                                           | Where                                                                                                                                                          | Consequence                                                                                              |
| ---------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| 52 fixtures: nightly-check 33, pr-check 11, visual-check 8                                     | `e2e/diagrams/gantt/`                                                                                                                                          |                                                                                                          |
| Every fixture has YAML front matter with `description` (and sometimes `config`, `displayMode`) | fixture files                                                                                                                                                  | `description` is tooling only (`e2e/helpers/mmd-snapshots.ts:16-20`)                                     |
| Clock pinned to `1010-10-10` for `gantt`                                                       | `e2e/rendering/mmd-snapshots.spec.ts:30-36`, `:73-76`                                                                                                          | `page.clock.install({time})`; time keeps running after it                                                |
| The comment names `rendering/gantt/gantt.spec.js`, which is not in this tree                   | `mmd-snapshots.spec.ts:31-32`; `e2e/rendering/gantt/` holds only `.gitkeep`                                                                                    | stale reference (read)                                                                                   |
| Fixture options: `fontFamily: 'courier'`, `fontSize: '16px'`                                   | `e2e/helpers/util.ts:72-76`                                                                                                                                    | task and axis text is Courier, not the bundled faces                                                     |
| Viewport 1440×1024, `deviceScaleFactor: 2`                                                     | `playwright.config.ts:52, 54`                                                                                                                                  | DPR 2 changes d3-axis offsets (6.7)                                                                      |
| Container: `div.mermaid#block` in body (viewer), `div#d<id>` in body (mermaid)                 | `e2e/platform/viewer.js:80-82, 260-261`; `mermaidAPI.ts:404-410`                                                                                               | `offsetWidth` is the body content width: 1440 - 2×8 = 1424 with the default body margin (**unverified**) |
| TZ                                                                                             | no `timezoneId` in `playwright.config.ts`; `.github/workflows/ci.yml:45` runs `ganttDb.spec.ts` with `TZ=America/Los_Angeles`, everything else on ubuntu (UTC) | which TZ produced the e2e baselines is **unverified** (the e2e workflow files are not in this tree)      |

---

## 2. Parser

### 2.1 Text pipeline before Jison (reference order)

| #   | Step                                                                                                                            | Where                                         | Effect on gantt                     |
| --- | ------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------- | ----------------------------------- |
| 1   | `detectType(code.cleaned)`                                                                                                      | `Diagram.ts:31`, `ganttDetector.ts:10`        | selects gantt                       |
| 2   | `cleanupText`: CRLF or CR to LF; `<tag a="x">` to `<tag a='x'>`                                                                 | `preprocess.ts:6-17`                          | `<br />` etc. are unaffected        |
| 3   | `processFrontmatter`: js-yaml `JSON_SCHEMA`; `title` (string), `config` (object), `displayMode` into `config.gantt.displayMode` | `preprocess.ts:19-31`, `frontmatter.ts:42-63` | front matter removed from the text  |
| 4   | `processDirectives`: `%%{init}%%` and `wrap` removed                                                                            | `preprocess.ts:33-45`                         |                                     |
| 5   | `cleanupComments`: drop lines matching `^\s*%%(?!{)[^\n]+\n?`, then `trimStart`                                                 | `comments.ts:6-8`                             | full-line `%%` comments             |
| 6   | `db.clear()`; `db.yy = db`; `db.setDiagramTitle(frontMatterTitle)` if set                                                       | `Diagram.ts:44-54`                            | front matter title set before parse |
| 7   | `textToParse = encodeEntities(source) + '\n'`                                                                                   | `Diagram.ts:77`                               |                                     |
| 8   | `parser.parse(textToParse)`                                                                                                     | `Diagram.ts:83`                               |                                     |

`encodeEntities` (`utils.ts:874-896`): removes the trailing `;` of `style…:\S*#…;` and `classDef…:\S*#…;`; rewrites `#\w+;` as `ﬂ°°N¶ß` (integer) or `ﬂ°name¶ß`. `decodeEntities` (`utils.ts:904-906`) turns them back into `&#N;` or `&name;` in the final SVG string (`mermaidAPI.ts:359`). So `#quot;` in a task name reaches the SVG as `&quot;` (**read**, not probed). A `#` with no `;` is untouched.

### 2.2 Lexer rules (`gantt.jison:6-99`), in order

The lexer uses first-match (see 2.3). Rules that return no token are skipped.

| Line  | Pattern                                                                | Token or action                      | Notes                                                                      |
| ----- | ---------------------------------------------------------------------- | ------------------------------------ | -------------------------------------------------------------------------- |
| 18    | `\%\%\{`                                                               | `open_directive`                     | no production; directives are removed earlier                              |
| 20    | `accTitle\s*":"\s*`                                                    | `acc_title`, start state `acc_title` | `\s*` also eats a following newline (quirk 2.5 Q2)                         |
| 21    | `<acc_title>(?!\n\|;\|#)*[^\n]*`                                       | `acc_title_value`, pop               | the lookahead is zero-width under `*`, so it has no effect                 |
| 22-23 | `accDescr\s*":"\s*` / `<acc_descr>…`                                   | `acc_descr`, `acc_descr_value`       | same                                                                       |
| 24-26 | `accDescr\s*"{"\s*`, `<acc_descr_multiline>[\}]`, `[^\}]*`             | block value, may be empty            | `\s*` after `{` eats newlines                                              |
| 28    | `\%\%(?!\{)*[^\n]*`                                                    | skip                                 | comment                                                                    |
| 29    | `[^\}]\%\%*[^\n]*`                                                     | skip                                 | **any character followed by `%` starts a comment that eats the line** (Q1) |
| 30    | `\%\%*[^\n]*[\n]*`                                                     | skip                                 | also eats following newlines                                               |
| 32    | `[\n]+`                                                                | `NL`                                 |                                                                            |
| 33    | `\s+`                                                                  | skip                                 | can swallow newlines after leading blanks                                  |
| 34    | `\%%[^\n]*`                                                            | skip                                 | unreachable (28 comes first)                                               |
| 42-45 | `"href"[\s]+["]` then `<href>["]`, `<href>[^"]*`                       | `href` in state `href`               | `href` needs a quote (2.6)                                                 |
| 54-59 | `"call"[\s]+`, `<callbackname>\([\s]*\)`, `\(`, `[^(]*`, `\)`, `[^)]*` | `callbackname`, `callbackargs`       | `call f` without parentheses is an error                                   |
| 67-69 | `"click"[\s]+`, `<click>[\s\n]`, `<click>[^\s\n]*`                     | `click` (the id)                     |                                                                            |
| 71    | `"gantt"`                                                              | `gantt`                              | `\b` added (2.3)                                                           |
| 72    | `"dateFormat"\s[^#\n;]+`                                               | `dateFormat`                         | value stops at `#`, `;` and newline                                        |
| 73    | `"inclusiveEndDates"`                                                  | `inclusiveEndDates`                  | trailing text is a parse error                                             |
| 74    | `"topAxis"`                                                            | `topAxis`                            | same                                                                       |
| 75    | `"axisFormat"\s[^#\n;]+`                                               | `axisFormat`                         |                                                                            |
| 76    | `"tickInterval"\s[^#\n;]+`                                             | `tickInterval`                       |                                                                            |
| 77    | `"includes"\s[^#\n;]+`                                                 | `includes`                           |                                                                            |
| 78    | `"excludes"\s[^#\n;]+`                                                 | `excludes`                           |                                                                            |
| 79    | `"todayMarker"\s[^\n;]+`                                               | `todayMarker`                        | allows `#`                                                                 |
| 80-86 | `weekday\s+monday` … `weekday\s+sunday`                                | `weekday_<day>`                      |                                                                            |
| 87-88 | `weekend\s+friday`, `weekend\s+saturday`                               | `weekend_<day>`                      | `weekend monday` is an error                                               |
| 89    | `\d\d\d\d"-"\d\d"-"\d\d`                                               | `date`                               | no production: a bare date at statement start is an error                  |
| 90    | `"title"\s[^\n]+`                                                      | `title`                              | allows `;` and `#`                                                         |
| 91    | `"accDescription"\s[^#\n;]+`                                           | `accDescription`                     | no production: error                                                       |
| 92    | `"section"\s[^\n]+`                                                    | `section`                            | allows `#`, `;`, `%%`                                                      |
| 93    | `[^:\n]+`                                                              | `taskTxt`                            | task name; may end in blanks                                               |
| 94    | `":"[^#\n;]+`                                                          | `taskData`                           | includes the colon; no `#` or `;`                                          |
| 95    | `":"`                                                                  | `:`                                  | no production                                                              |
| 96    | `<<EOF>>`                                                              | `EOF`                                |                                                                            |
| 97    | `.`                                                                    | `INVALID`                            | no production: error                                                       |

Keyword matching is case-insensitive (`%options case-insensitive`, line 8, probed: `GANTT` and `DATEFORMAT` work). The DB tag regexes are not (2.5 and 3.8).

### 2.3 Jison matching semantics (verified in `node_modules`)

- First match wins. `jison-lex/regexp-lexer.js:367-383`: a rule that matches sets the match, and unless `options.flex` is set the loop breaks at once.
- Keyword literals get a word boundary. `lex-parser/lex-parser.js:167` appends `\b` to a literal that ends in a word character, unless `flex` is set. So `"gantt"` does not match the start of `ganttX`. `ganttX :1d` is a task (probed).
- Consequence: a task whose name starts with a keyword followed by a blank is a keyword statement, and the rest of the line is parsed as that keyword (Q3, probed: `click : 1d` gives a parse error).

### 2.4 Grammar (`gantt.jison:107-186`) and the DB calls it makes

Productions in order, with the DB call each one makes.

| Production                                        | Lines   | DB call and argument                   |
| ------------------------------------------------- | ------- | -------------------------------------- |
| `start: gantt document EOF`                       | 107-109 | returns the statement list             |
| `document: ε \| document line`                    | 111-114 | statements need no separator           |
| `line: SPACE statement \| statement \| NL \| EOF` | 116-121 | `SPACE` is never produced (dead)       |
| `weekday_<d>`                                     | 123-131 | `setWeekday("<d>")`                    |
| `weekend_friday/saturday`                         | 133-136 | `setWeekend("<d>")`                    |
| `dateFormat`                                      | 139     | `setDateFormat($1.substr(11))`         |
| `inclusiveEndDates`                               | 140     | `enableInclusiveEndDates()`            |
| `topAxis`                                         | 141     | `enableTopAxis()`                      |
| `axisFormat`                                      | 142     | `setAxisFormat($1.substr(11))`         |
| `tickInterval`                                    | 143     | `setTickInterval($1.substr(13))`       |
| `excludes`                                        | 144     | `setExcludes($1.substr(9))`            |
| `includes`                                        | 145     | `setIncludes($1.substr(9))`            |
| `todayMarker`                                     | 146     | `setTodayMarker($1.substr(12))`        |
| `title`                                           | 149     | `setDiagramTitle($1.substr(6))`        |
| `acc_title acc_title_value`                       | 150     | `setAccTitle($2.trim())`               |
| `acc_descr acc_descr_value`                       | 151     | `setAccDescription($2.trim())`         |
| `acc_descr_multiline_value`                       | 152     | `setAccDescription($1.trim())`         |
| `section`                                         | 153     | `addSection($1.substr(8))`             |
| `clickStatement`                                  | 162-173 | `setClickEvent` and `setLink`, see 2.7 |
| `taskTxt taskData`                                | 155     | `addTask($1, $2)`, `$1` untrimmed      |

The `substr` offsets are the keyword length plus one blank: `dateFormat` 10+1, `axisFormat` 10+1, `tickInterval` 12+1, `excludes` 8+1, `includes` 8+1, `todayMarker` 11+1, `title` 5+1, `section` 7+1. Consequences (probed):

- More than one blank after the keyword stays in the value. `dateFormat  D` gives `getDateFormat()` = `" D"`, and the axis rule `=== 'D'` (6.7) then does not match: the axis shows `1010-10-01`, not `01` (**probed**, `mini/dateformat-two-spaces-D`).
- `axisFormat  %d/%m` gives `" %d/%m"`, so labels carry a leading blank (probed, labels like `"  10-01"`).
- `title     Spaced   title  ` gives `"    Spaced   title  "` (probed). The DB does not trim.

Dead productions: `clickStatementDebug` (175-186) has no reference, and the `SPACE` alternative (117) is unreachable.

### 2.5 Quirks of the lexer and grammar (probed unless marked)

| Id  | Input                                                                                       | Result in the reference                                                                                                                 |
| --- | ------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Q1  | line `A%B :a, 2014-01-01, 1d` (or `1%-done …`, or indented)                                 | the line is silently dropped (rule 29 eats it); the next line parses (probed)                                                           |
| Q2  | `accTitle:` then newline then `section X`                                                   | `setAccTitle("section X")`; the section is lost (probed)                                                                                |
| Q3  | `click : 1d`, `call x`, `title x`, `section x` as task names                                | keyword statement; `click : 1d` is a parse error (probed)                                                                               |
| Q4  | `section A %% hi`                                                                           | section name `A %% hi` (probed); `%%` is a comment only at a token start                                                                |
| Q5  | `;` or `#` in task data (`:a1, 2014-01-01; 3d`, `… # c`)                                    | parse error (probed). In names (`;A task`, `#A task`) and in titles, sections and accDescr they are text (probed)                       |
| Q6  | `excludes weekends; 2014-01-01`                                                             | parse error (probed)                                                                                                                    |
| Q7  | `dateFormat` alone, `section` alone, `todayMarker` alone, `title ` (empty), `weekday` alone | parse error (probed)                                                                                                                    |
| Q8  | `accDescription hello`                                                                      | parse error (probed; the token has no production)                                                                                       |
| Q9  | `tickInterval 1day ` (trailing blank)                                                       | the value keeps the blank; the custom tick regex fails and the default ticks are drawn (probed: 61 ticks with `1day`, 9 with the blank) |
| Q10 | `CRLF` input                                                                                | handled by `cleanupText` before the lexer (read); a raw parse keeps `\r` in names (probed without cleanup)                              |

### 2.6 Error behaviour

Three classes. Only (b) depends on the date semantics.

**(a) Parse errors (Jison).** `parser.parse` throws. `Diagram.fromText` throws (`Diagram.ts:83`), and the error diagram is shown. Probed examples: no-colon task `foo bar`, empty `title `, `dateFormat` alone, `;` or `#` in data, `click` without id, `call fn` without parentheses, `href https://x` without quotes, empty task name ` :1d`, `weekend monday`, `accDescription`. The text `Parse error on line N:` is a Jison message and need not match.

**(b) Errors raised while compiling dates.** They happen in `getTasks`, which the renderer calls (`ganttRenderer.js:109`), so `parse` succeeds and `render` fails.

| Case                                                                  | Where                                                                             | Probed                                     |
| --------------------------------------------------------------------- | --------------------------------------------------------------------------------- | ------------------------------------------ |
| `:3d` or `:` as the first task (no previous end)                      | `compileTask` reads `prevTask.endTime` (`ganttDb.js:664-665`), TypeError          | E1, E5                                     |
| tags only, `:active`, `:done`                                         | `getTaskTags` reads `data[0]` after all tags are removed (`885-898`), TypeError   | E2, E3                                     |
| four or more fields, or a non-leading tag, or `Done` (case)           | `startTime` undefined, `compileTask` reads `.type` (`662`), TypeError             | E4, E19, E21                               |
| strict parse fails and `new Date(str)` is NaN or outside ±10000 years | `Invalid date:<str>` (`368-372`)                                                  | E9 (`202304` with `YYYYMMDD`), E16 (`foo`) |
| every day excluded (10000 days)                                       | `Failed to find a valid date … after 10,000 iterations` (`279-283`)               | E12                                        |
| `after` or `until` id that does not match `[\d\w- ]+`                 | `Invalid date:after :ps` (the regex fails, so the string reaches the date parser) | I1, I3                                     |

**(c) Warnings only.** `log.warn`, no output change: unknown `after`/`until` id (`299-303`, probed E6, E7, E24), unparseable duration (`456-458`, probed E17, E18, D36-D39), milestone without duration, tick count over 10000 (`684-688`), exclude span over five years (`563-567`).

Errors (b) happen at render. Warnings are not part of any output.

### 2.7 Click, href and call

| Form                                  | DB calls (`gantt.jison:163-172`) |
| ------------------------------------- | -------------------------------- |
| `click id call f()`                   | `setClickEvent(id, "f", null)`   |
| `click id call f(args)`               | `setClickEvent(id, "f", "args")` |
| `click id href "url"`                 | `setLink(id, url)`               |
| forms with `call` and `href` together | both calls, in grammar order     |

- `setClickEvent` (`ganttDb.js:817-822`) calls `setClickFun` (744-778), which returns at once unless `securityLevel === 'loose'` (745-747). In the default (strict) mode no callback is stored. Both `setClickEvent` and `setLink` call `setClass(ids, 'clickable')` (821, 726), so `call` plus `href` gives `classes: ["clickable","clickable"]` (probed I9).
- `setLink` (712-727) sanitises the URL unless `securityLevel` is `loose` (714-716). Probed: `javascript:alert(1)` becomes `about:blank`; `https://example.org/x y` becomes `https://example.org/x%20y`. It stores `links` only for tasks that exist (719-724), and registers a click handler (720) that runs `window.open(url, '_self')`.
- That handler is returned as `bindFunctions` (`mermaidAPI.ts:672`, `types.ts:148-153`), so in strict mode the reference binds a JavaScript click that navigates in the same window. The SVG has no `<a>` (the anchors are created only in sandbox mode, `ganttRenderer.js:508-531`).
- None of the 52 fixtures uses `click` (checked by grep). The spec does (`gantt.spec.js:210-240`).

### 2.8 Comments, directives and front matter as the parser sees them

- `%%` lines are removed before the parser (`comments.ts:6-8`). The lexer also skips them (rules 28-30 and 34), which matters for parser-only input (`gantt.spec.js:54-56`).
- `%%{ … }%%` directives are removed before the parser (`preprocess.ts:33-45`). A `%%{` that survives is an `open_directive` token and an error.
- Front matter is removed before lexing (`preprocess.ts:20`, `frontmatter.ts:24-69`).
- `displayMode` (`frontmatter.ts:55-57`) goes to `config.gantt.displayMode` (`preprocess.ts:21-27`). `db.setDisplayMode` is never called from the pipeline (only the DB defines it), so `getDisplayMode()` is always `''` in practice. The renderer checks both (`ganttRenderer.js:126`).

### 2.9 Dead code (safe to leave out)

- `clickStatementDebug` (`gantt.jison:175-186`); `SPACE` alternative (117); `date` token (89); `accDescription` token (91); `open_directive` and `:` tokens (18, 95).
- `compileData` (`ganttDb.js:487-534`) and `addTaskOrg` (637-656) are exported but nothing in the grammar or renderer calls them. `lastTask` (592-593, 653-654) belongs to `addTaskOrg`.
- `colorScale`, `interpolateHcl`, `theColorScale` (208-211, 224-225, 238).

---

## 3. DB and date semantics

### 3.1 Module state and `clear()`

| State                                           | Default                                      | Reset by `clear()` (47-71) |
| ----------------------------------------------- | -------------------------------------------- | -------------------------- |
| `dateFormat`                                    | `''` (25)                                    | yes (56)                   |
| `axisFormat`                                    | `''` (26)                                    | yes (57)                   |
| `tickInterval`                                  | undefined (27)                               | yes (59)                   |
| `todayMarker`                                   | `''` (28)                                    | yes (60)                   |
| `includes`, `excludes`                          | `[]` (29-30)                                 | yes (61-62)                |
| `links`                                         | `new Map()` (31)                             | yes (66)                   |
| `sections`, `tasks`, `currentSection`, `funs`   | empty (32-34, 37)                            | yes (48-51)                |
| `rawTasks`, `lastTaskID`, `lastTask`, `taskCnt` | `[]`, undefined, undefined, 0 (468, 592-594) | yes (52-55)                |
| `displayMode`                                   | `''` (35)                                    | yes (58)                   |
| `inclusiveEndDates`, `topAxis`                  | false (39-40)                                | yes (63-64)                |
| `lastOrder`                                     | 0 (45)                                       | yes (65)                   |
| `weekday`                                       | `'sunday'` (41)                              | yes (69)                   |
| `weekend`                                       | `'saturday'` (42)                            | yes (70)                   |
| **`taskDb`** (`{}`, 595)                        | `{}`                                         | **no**                     |

`taskDb` maps id to position in `rawTasks`. Because it is not cleared, an id that a later diagram does not define can resolve to a slot of the new diagram. Observed in a probe run: a stale entry from an earlier case changed the result of a later one. The port must not carry state across renders.

### 3.2 Task lifecycle

1. **Parse**, `addTask(descr, data)` (596-630): creates the raw task (`section`, `type` = current section, `processed` false, `manualEndTime` false, `renderEndTime` null, `raw.data`, `task` = `descr` untrimmed, `classes` []). `parseData` fills `raw.startTime`, `raw.endTime`, `id`, `prevTaskId` (= last id), and the flags. `order`: vert gives -1, otherwise `lastOrder++` (618-623). Then `rawTasks.push`, `lastTaskID = id`, `taskDb[id] = pos - 1` (625-629).
2. **`parseData`** (536-590): strip one leading `:`; split on `,`; `getTaskTags` (885-899) removes leading tags; trim each field. By count of remaining fields:

| Count   | id                                   | startTime                              | endTime                            |
| ------- | ------------------------------------ | -------------------------------------- | ---------------------------------- |
| 1       | `parseId()` = `task<n>`              | `{type:'prevTaskEnd', id: prevTaskId}` | `{data: f0}`                       |
| 2       | `parseId()`                          | `{type:'getStartDate', startData: f0}` | `{data: f1}`                       |
| 3       | `parseId(f0)` (empty string is kept) | `{type:'getStartDate', startData: f1}` | `{data: f2}`                       |
| 0 or 4+ | undefined                            | undefined                              | undefined (later TypeError, 2.6 b) |

3. **`getTasks`** (169-181): `compileTasks()` once, then up to 10 more passes while any task is unprocessed. So at most 11 passes. The result is `tasks = rawTasks`, the same array, so renderer sorts change the DB order (6.3).
4. **`compileTasks`** (658-704): every raw task in array order runs `compileTask`; `allProcessed` is the conjunction.
5. **`compileTask(pos)`** (659-695):

```
if raw.startTime.type == 'prevTaskEnd':  task.startTime = findTaskById(task.prevTaskId).endTime   # TypeError if absent
if raw.startTime.type == 'getStartDate': s = getStartDate(raw.startTime.startData); if s: task.startTime = s
if task.startTime:
    task.endTime = getEndDate(task.startTime, raw.endTime.data, inclusiveEndDates)
    if task.endTime:
        processed = true
        manualEndTime = strictParse(raw.endTime.data, 'YYYY-MM-DD').isValid()   # independent of dateFormat
        checkTaskDates(task)
return processed
```

Every pass recomputes from the raw data, so repeated passes do not compound excludes shifts (read; the chain probes D29/D31 agree).

### 3.3 Start and end dates

**`getStartDate(prev, fmt, str)`** (305-375), in this order:

1. `str = trim(str)`.
2. If `fmt.trim()` is `x` or `X` and `str` matches `^\d+$`: `new Date(Number(str))`. Both mean milliseconds here (315-317). Probed: `dateFormat X` with `000` is 0 ms.
3. If `str` matches `^after\s+([\d\w- ]+)` (319): split the ids on single blanks and drop empties. For each id, `findTaskById` (last definition wins). Unknown ids warn (299-303). The result is the largest `endTime` among the found tasks (`>`, so the first wins a tie). With no found task, `today 00:00` local (345-347). Probed: `after zz` gives 1010-10-10T00:00 (`D33`).
4. Else strict `dayjs(str, fmt.trim(), true)`. If valid, `toDate()` (351-353).
5. Else `log.debug`, then `d = new Date(str)` (359). If `d` is NaN, or `getFullYear` is below -10000 or above 10000, throw `Invalid date:<str>` (368-372). Otherwise return `d`. This is the legacy path (3.5).

**`getEndDate(prev, fmt, str, inclusive)`** (409-466), in this order:

1. `str = trim(str)`.
2. If `^until\s+([\d\w- ]+)` (413): the smallest `startTime` among the found ids, `<` (first wins). Unknown ids warn. None found gives today 00:00 local (439-441). The `inclusive` flag does not apply to `until`.
3. Else strict `dayjs(str, fmt.trim(), true)`. If valid: add one day when `inclusive` (447-449), return `toDate()`. `inclusive` is `inclusiveEndDates` (528, 681).
4. Else `parseDuration(str)`. If NaN, warn (456-458) and return `prev` as the end (zero length).
5. Else `dayjs(prev).add(value, unit)`, return `toDate()` (460-465).

**`parseDuration`** (399-407): `/^(\d+(?:\.\d+)?)([Mdhmswy]|ms)$/` on the trimmed string; no sign, no exponent, no unit gives NaN. Units: `M` month, `d` day, `h` hour, `m` minute, `s` second, `w` week, `y` year, `ms` millisecond. Probed: `-1d`, `1e3d`, `3D`, `3H` are all NaN.

**dayjs `add`** (1.11.21, `esm/index.js:255-280`), probed:

| Unit                | Arithmetic                                                     | Probed example                                       |
| ------------------- | -------------------------------------------------------------- | ---------------------------------------------------- |
| `ms`, `s`, `m`, `h` | `getTime() + n × step` (fixed milliseconds, fractions allowed) | `0.5h` from 00:00 gives 00:30                        |
| `d`                 | `setDate(getDate() + Math.round(n))`, local calendar           | `1.5d` from 2014-01-31 gives 2014-02-02 (rounded up) |
| `w`                 | `setDate(getDate() + Math.round(7n))`                          | `1w` gives +7 days                                   |
| `M`                 | `setMonth(month + n)`, clamped to the month length             | `1M` from 2014-01-31 gives 2014-02-28                |
| `y`                 | `setFullYear(year + n)`, clamped                               | `1y` gives 2015-01-31                                |

**Strict parsing** (customParseFormat, `esm/plugin/customParseFormat/index.js`, read):

- The parser fills missing parts from the clock (`parseFormattedInput`, 217-256): day = `now.getDate()` when there is no year and no month, else 1; year = `now.getFullYear()`; month = `now.getMonth()` unless the input has a year and no month; hours, minutes, seconds, ms default to 0. Then `new Date(y, M, d, h, m, s, ms)` in local time (256). So a format with no date part depends on today. Probed: `dateFormat D` with `1` gives 1010-10-01; `HH:mm` with `17:30` gives 1010-10-10T17:30 (the clock's day).
- Strict check (307-309): `date != this.format(format)` makes the result invalid. The round trip must reproduce the input exactly. Probed: `ss` with `0` is invalid (the output is `00`), so the start falls back to `new Date('0')`; `HH:mm` with `9:05` is invalid; `YYYY-MM-DD` with `2014-1-1` is invalid; `2024-02-30` rolls over and is invalid.
- `format('')` gives the default ISO-like string, so an empty `dateFormat` never matches. Probed: `strict('2014-01-01', '')` is false. Hence a diagram without `dateFormat` (fixture issue-1060) sends every date to the legacy path.
- The `X` and `x` round trip needs `advancedFormat` (probed: without it `120` with `X` is invalid).

### 3.4 Excludes, includes and weekends

**`isInvalidDate(date, fmt, excludes, includes)`** (183-201), in order:

```
fd = date.format(fmt.trim())          # empty fmt gives the ISO default, so no token matches
d1 = date.format('YYYY-MM-DD')
if includes ∋ fd or includes ∋ d1:                                  return false   # 187-189
if excludes ∋ 'weekends' and isoWeekday ∈ {W, W+1}:                  return true    # 190-196
   where W = {friday: 5, saturday: 6}[weekend]
if excludes ∋ lower(date.format('dddd')):                            return true    # 197-199
return excludes ∋ fd or excludes ∋ d1                                              # 200
```

- `weekends` is a token, not a keyword in the DB. Only the literal `weekends` triggers the weekend rule. `weekdays` matches nothing (probed D21). Seven fixtures use `excludes weekdays <date>`, so only the date part works for them.
- Day names are compared in lower case with the English `dddd` (`excludes sunday`, probed D22).
- `includes` win over `excludes` and over the weekend rule (probed D18).

**`checkTaskDates(task)`** (227-254): returns at once if `excludes` is empty or the end was given as a date (`manualEndTime`) (228). Otherwise:

```
s = startTime + 1 day                                  # 237: the start day itself is never tested
(e, render) = fixTaskDates(s, endTime)
task.endTime = e ; task.renderEndTime = render          # 252-253
```

**`fixTaskDates(s, e)`** (268-288):

```
invalid = false ; render = null ; max = e + 10000 days
while s <= e:
    if not invalid: render = e                         # 274: end at the last valid day seen
    invalid = isInvalidDate(s)
    if invalid:
        e = e + 1 day
        if e > max: throw 'Failed to find a valid date ...'      # 279-283
    s = s + 1 day
return (e, render)
```

Worked example, Friday 2019-02-01 for 5 days, excludes `weekends` (probed D16): the raw end is Wednesday 02-06. The loop moves the end past Saturday and Sunday to Friday 02-08. `renderEndTime` is 02-08, because 02-08 is the last valid day the loop sees.

Worked example, Friday 2019-02-01 for 1 day (probed D15): the raw end is Saturday 02-02. The end moves to Monday 02-04, and `renderEndTime` stays at 02-02, the last valid day before the excluded tail. The renderer stops the bar at 02-02 (`ganttRenderer.js:315, 402`), while successors start at 02-04.

Other consequences (probed): zero-length task on an excluded day gives `renderEndTime` null (D20); a manual end date gives `renderEndTime` null and no shift (D14); a task that ends on a valid day has `renderEndTime` = end (D19).

### 3.5 The legacy `new Date(str)` path (V8 behaviour)

This path decides the results of 17 strings across the fixtures (probed by scanning the 52 fixtures, `fallback-scan.mjs`). In TZ=UTC:

| Input                                    | Where                                        | Result                     | Rule                                                                                |
| ---------------------------------------- | -------------------------------------------- | -------------------------- | ----------------------------------------------------------------------------------- |
| `2014-01-06`, `2014-01-09`               | visual-check (`dateFormat :YYYY-MM-DD`, 3.6) | 2014-01-06T00:00Z          | ISO date-only is **UTC**                                                            |
| `2019-05-10` … `2019-09-17` (15 strings) | issue-1060 (no `dateFormat`)                 | UTC midnight               | ISO date-only is UTC                                                                |
| `2019-09-1`                              | issue-1060                                   | 2019-09-01 00:00 **local** | non-ISO form is local                                                               |
| `202-12-01`                              | typo fixture                                 | year 202 (local midnight)  | year below 1000 kept                                                                |
| `0`                                      | seconds-only fixture (`dateFormat ss`)       | 2000-01-01 00:00 **local** | V8 legacy: a bare small number is read as a year in the 2000s (probed for `0` only) |

Other probed results, not in the fixtures: `202304` gives year 202304 (throws, E9); `2014/01/02` gives local midnight; `2014-02-30` rolls to 2014-03-02 (V8 rollover); `foo` gives Invalid Date (throws).

The DB spec depends on these: `ganttDb.spec.ts:542-546` expects `Invalid date:202304`, and `:558-568` expects `getFullYear()` 202 for `202-12-01`.

ECMA-262 leaves this format implementation-defined. A port must choose (section 8): implement the rules above for these shapes, or reject them. Rejecting would change 2 fixtures' output and the spec expectations.

### 3.6 Time zone and clock

Where the reference reads the clock or the zone:

| Place                                                            | Dependency                          |
| ---------------------------------------------------------------- | ----------------------------------- |
| dayjs strict parse of partial formats (3.3)                      | today's date (clock and local zone) |
| `after`/`until` with no known id (`ganttDb.js:345-347, 439-441`) | today 00:00 local                   |
| `drawToday` (`ganttRenderer.js:863`)                             | `new Date()`                        |
| d3 time intervals and time formats (renderer)                    | local calendar and local hours      |
| dayjs strict parse of full dates                                 | local midnight                      |
| `new Date('YYYY-MM-DD')` (fallback)                              | **UTC** midnight                    |
| `new Date('YYYY/MM/DD')`, `new Date('202-12-01')` (fallback)     | local                               |

Probed effect (same pinned clock, three zones; `pipeline.mjs`, `fx-*.jsonl`): compared with UTC, the local date and time of at least one task changes for 14 fixtures under `America/Los_Angeles` and 8 under `Europe/Berlin`. The causes, by fixture:

- Epoch formats (`dateFormat x` and `X`, and the milliseconds fixture): the instant is fixed, but the local time shown changes with the zone. Both zones.
- Clock-day formats (`HH:mm`, `ss`, `SSS`, `D`) take the date from the clock. The pinned instant is 1010-10-09 17:00 in LA, so these tasks move a day (compact, vertical markers, seconds and milliseconds fixtures). Under Berlin the instant is 01:00 on the same day, so the day does not move.
- ISO date-only strings through the fallback (issue-1060, and the four `:YYYY-MM-DD` visual-check fixtures) are UTC midnight, so they show a different local time in every other zone.

**Port decision.** Fix the zone to UTC and take `now` as an explicit input (default: the system clock, as the reference). Compare with the reference under `TZ=UTC` and the clock at `1010-10-10T00:00Z`. Which TZ the e2e baselines use is **unverified** (section 1.6).

### 3.7 Task order, sections, tags and milestones

- `order`: non-vert tasks take 0, 1, 2 … in creation order; vert tasks take -1 and do not advance the counter (`ganttDb.js:618-623`).
- Sections: `addSection` sets `currentSection` and pushes the name (160-163). Tasks before any section get `''`. Duplicate names stay in `sections` (probed I13). The renderer's categories are unique names (`checkUnique`, 884-895).
- Tags (`getTaskTags`, 885-899): loops over `active`, `done`, `crit`, `milestone`, `vert` in that order. Each removes `data[0]` if it matches `^\s*TAG\s*$`. The regex has no `i` flag, so `Done` is not a tag and `done` is (probed E19, E20). A tag after the first field is not a tag (E21). Several tags in any order at the start all count (E22).
- `milestone` is only a flag. The DB does not change dates; the renderer draws the diamond.

### 3.8 `after` and `until` in detail

- Ids are matched by `[\d\w- ]+`, so only ASCII letters, digits, `_`, `-` and blanks. A dot, colon or non-ASCII letter in an id is not referenced. `after v1.2` matches `v1`, warns and falls back to today, silently (probed I2). `after :ps` throws (I1). `after Ärger` throws (I3). `after a_b-c` works (I4).
- Ids are duplicated by design: `taskDb[id]` is overwritten, so a reference resolves to the **last** definition in the file, even when it comes earlier in the file (probed E15, I7).
- Resolution order: the `after` target may not be compiled yet, so its `endTime` is undefined. Then no candidate is taken and the call returns undefined. The task stays unprocessed and is retried on the next pass. Chains of up to 11 links resolve, 12 do not (probed D29, D30, D31).
- An unresolved task keeps `startTime` undefined. The renderer then writes `NaN` into `x`, `width` and `transform-origin` (probed `chain12`: `x="NaN" width="NaN"` on the first task, warnings for each unknown id). The reference therefore draws invalid SVG for such input. The port must decide (section 8).

### 3.9 Other DB rules that reach the output

- Title (`setDiagramTitle`, `commonDb.ts:28-30`): sanitised, no trim. The grammar's title (last one wins) overrides the front matter title, which is set first (`Diagram.ts:52-54`). A grammar title cannot be empty (Q7), so the two rules never clash.
- accTitle (`commonDb.ts:16-18`): sanitised, then leading whitespace removed. accDescr (22-24): sanitised, then `\n\s+` to `\n`. The grammar already trims both (150-152).
- Sanitising (`common.ts:56-82`): with `htmlLabels` on (default) and `securityLevel` `strict`, `antiscript` or `sandbox`, `removeScript` runs first (48-53). Then DOMPurify with `FORBID_TAGS: ['style']` (or `dompurifyConfig`).
- Task names (`addTask`, 596-630) and section names (`addSection`, 160-163) are **not** sanitised.
- `excludes` and `includes` (`mergeTokens`, 133-139): lower case, split on blanks and commas, de-duplicated, first-seen order (probed I15, I16).
- `todayMarker`: stored raw (probed I14, with the leading blank). The renderer turns `,` into `;` for the `style` attribute (`ganttRenderer.js:873-875`). `off` disables the line (858).
- `tickInterval`: stored raw. The renderer's regex (661) must match exactly, so a trailing blank silently disables the custom ticks (Q9).
- `weekend`: only `friday` and `saturday` are accepted by the grammar (133-136).
- `topAxis`: DB flag or config (`ganttRenderer.js:730`).

### 3.10 Probe results (selected, TZ=UTC, clock 1010-10-10)

| Id           | Input                                                            | Reference result                                                                           |
| ------------ | ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| D1           | `dateFormat D`, `1, 1d`                                          | 1010-10-01 to 1010-10-02 (month and year from the clock)                                   |
| D2           | `dateFormat ss`, `rtt, 0, 20`                                    | start 2000-01-01T00:00 (legacy `new Date('0')`), end 1010-10-10T00:00:20 (today)           |
| D5 / D6 / D7 | `x` / `X` / `x` with `0`, `120`                                  | D6: end is 120 **s**; D7: 120 ms                                                           |
| D10          | `1w 1M 1y 1.5d 0.5h 2m 3s 5ms` from 2014-01-31                   | +7 d, Feb 28, 2015-01-31, Feb 2 (1.5 d rounded to 2), 00:30, 00:02, 00:00:03, 00:00:00.005 |
| D13          | `inclusiveEndDates`, `2019-02-01, 1d` / `2019-02-01, 2019-02-03` | ends 02-02 / 02-04 (fixed end +1, manual)                                                  |
| D14          | fixed end with `excludes weekends`                               | no shift; `renderEndTime` null                                                             |
| D15          | Fri 02-01, 1d, `weekends`                                        | end Mon 02-04; render Sat 02-02                                                            |
| D16          | Fri 02-01, 5d, `weekends`                                        | end and render Fri 02-08                                                                   |
| D18          | `includes 2019-02-02`                                            | Saturday is a working day                                                                  |
| D24          | `dateFormat :YYYY-MM-DD`, `2014-01-06, 2014-01-08`               | start UTC midnight via fallback; end is the start (the end date fails the format; warning) |
| D26          | `2024-02-30`, YYYY-MM-DD                                         | 2024-03-01 (fallback rollover)                                                             |
| D29 / D31    | chain of 11 / 12 `after` links, reverse order                    | 11 resolve; the 12-link chain gives an unresolved head                                     |
| E15          | duplicate id `x`, later `after x`                                | uses the later definition                                                                  |
| E17          | milestone with end `0`                                           | zero length, warning                                                                       |

---

## 4. Canonical model proposal

Follows `canonical-model.md`: `version: 1`, camelCase keys, empty strings instead of `null` for absent texts, JSON numbers, text rules of section 1-6 of that page. Dates are epoch milliseconds (integers, as `Date.getTime()`), negative allowed; `null` for an unresolved value.

```json
{
  "version": 1,
  "title": "",
  "accTitle": "",
  "accDescr": "",
  "dateFormat": "",
  "axisFormat": "",
  "tickInterval": null,
  "todayMarker": "",
  "topAxis": false,
  "inclusiveEndDates": false,
  "weekday": "sunday",
  "weekend": "saturday",
  "includes": [],
  "excludes": [],
  "sections": [],
  "links": {},
  "tasks": [
    {
      "id": "a1",
      "task": "A task ",
      "section": "",
      "order": 0,
      "vert": false,
      "milestone": false,
      "active": false,
      "done": false,
      "crit": false,
      "classes": [],
      "manualEndTime": false,
      "startTime": 1388534400000,
      "endTime": 1388707200000,
      "renderEndTime": null
    }
  ]
}
```

Rules:

| Field                                                     | Rule                                                                                                                                       | Why                                                                                |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------- |
| `title`, `accTitle`, `accDescr`                           | the sanitised value (DOMPurify path, 3.9); `title` keeps leading and trailing blanks; `accTitle` and `accDescr` are trimmed by the grammar | the DB sanitises them (`commonDb.ts`) and the title keeps its blanks (`substr(6)`) |
| `dateFormat`, `axisFormat`, `todayMarker`, `tickInterval` | raw DB value, blanks included; `tickInterval` `null` when unset                                                                            | the renderer's `=== 'D'` and the tick regex compare the raw value (2.4, 3.9)       |
| `includes`, `excludes`                                    | lower case, de-duplicated, first-seen order                                                                                                | `mergeTokens` (133-139)                                                            |
| `sections`                                                | raw names, duplicates kept                                                                                                                 | `addSection` (160-163)                                                             |
| `links`                                                   | task id to sanitised URL (strict semantics: `sanitizeUrl`); only for existing tasks                                                        | `setLink` (712-727)                                                                |
| `task`                                                    | raw text, untrimmed, not sanitised                                                                                                         | `addTask` (596-630)                                                                |
| `startTime`, `endTime`                                    | the `getTasks()` result; `null` when the task stays unresolved                                                                             | the renderer's inputs                                                              |
| `renderEndTime`                                           | `null` or the value from `fixTaskDates`                                                                                                    | `ganttRenderer.js:315, 402`                                                        |
| `classes`                                                 | in order, duplicates kept (`["clickable","clickable"]` is real)                                                                            | `setClass`                                                                         |

**Left out, and why**

| Left out                                                      | Reason                                                                           |
| ------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| `type`                                                        | always equals `section` (`ganttDb.js:598-599`)                                   |
| `raw` (descriptors), `prevTaskId`                             | internal to start and end resolution; the resolved values are in the model       |
| `processed`                                                   | derivable: `startTime` and `endTime` are both non-null                           |
| `funs`, click callback names and args                         | no effect in strict mode (`setClickFun` returns early); loose mode is not ported |
| `taskDb`, `taskCnt`, `lastOrder`, `lastTask`, `diagramId`     | state, not input                                                                 |
| `displayMode`, `useWidth`, `useMaxWidth`, the `gantt` section | configuration; the config model (M2) owns them, and the renderer reads them      |
| the renderer's `order` changes (compact)                      | render-time; the model keeps the DB's `order`                                    |
| `renderEndTime` of an `after` chain (none)                    | n/a                                                                              |

Sanitisation: gantt uses the DOMPurify subset of `ferrigura-syntax/src/sanitize.rs` for `title`, `accTitle` and `accDescr`, the same port as the flowchart uses for its texts. It does **not** apply to task and section names. The canonical-model page says the other diagrams hold unsanitised text, so this is a gantt-specific exception that must be written into that page when M6 lands.

Title precedence: the grammar's title (last one) when it is set, else the front matter title, else `""` (`Diagram.ts:52-54` plus the grammar). The rule matches `canonical-model.md`.

---

## 5. Specs

### 5.1 Files and counts

| File                      | `it` calls | `it.each` calls | Cases (expanded)     | Level                                                                               |
| ------------------------- | ---------- | --------------- | -------------------- | ----------------------------------------------------------------------------------- |
| `ganttDb.spec.ts`         | 22         | 6               | 53 (one gated by TZ) | DB (getters, `getTasks`, `parseDuration`, `isInvalidDate` via `getTasks`, warnings) |
| `parser/gantt.spec.js`    | 15         | 3               | 29                   | parser with the DB as `yy`, plus DB getters and DB-call spies                       |
| `ganttRenderer`           | 0          | 0               | 0                    | none                                                                                |
| `styles`, `ganttDetector` | 0          | 0               | 0                    | none                                                                                |

Totals: 37 `it` calls, 9 `it.each` tables, 82 cases. `STATUS.md` gives about 40 (the `it` calls, rounded). The counts are static (read); vitest was not run.

No spec uses the DOM. `ganttDb.spec.ts` imports `ganttDb.js`, which imports `getConfig` and `utils.js`; the DB needs only the config's `securityLevel` (`setLink`) and the logger (`log.warn`).

### 5.2 `ganttDb.spec.ts` by level

| Lines        | Cases | Level                                                                | Needs               |
| ------------ | ----- | -------------------------------------------------------------------- | ------------------- |
| 13-18, 20-31 | 1 + 5 | DB (`parseDuration`)                                                 | none                |
| 34-62        | 10    | DB (getters after `clear`)                                           | none                |
| 65-84        | 8     | DB (fixed dates, durations, ids)                                     | none                |
| 87-119       | 5     | DB (`after`); row 2 expects `new Date().setHours(0,0,0,0)` (line 90) | clock               |
| 121-140      | 1     | DB (`dateFormat x`)                                                  | none                |
| 142-212      | 4     | DB (relative ids, `until`)                                           | none                |
| 214-269      | 1     | DB (weekends, excludes, `renderEndTime`)                             | none                |
| 271-284      | 1     | DB (weekend friday)                                                  | none                |
| 286-303      | 1     | DB (throws `Failed to find a valid date`)                            | none                |
| 305-322      | 3     | DB (token merging, `getExcludes`, `getIncludes`)                     | none                |
| 324-454      | 1     | DB (order, sections, `24h`, `20h`, `after`)                          | none                |
| 456-474      | 1     | DB (month end, `11d`, `20d`)                                         | none                |
| 485-507      | 1     | DB, **gated** by `TZ=America/Los_Angeles`                            | a zone with DST     |
| 510-531      | 1     | DB (inclusive end dates)                                             | none                |
| 533-540      | 2     | DB (today marker get and set)                                        | none                |
| 542-546      | 1     | DB (throws `Invalid date:202304`)                                    | legacy `Date` parse |
| 548-556      | 1     | DB (`dateFormat ss`, only id and task checked)                       | none                |
| 558-569      | 1     | DB (`202-12-01`, year 202)                                           | legacy `Date` parse |
| 572-617      | 4     | DB (warnings through a spy on `log.warn`)                            | a warning hook      |

### 5.3 `parser/gantt.spec.js` by level

| Lines                   | Cases | Level                                                         | Notes                          |
| ----------------------- | ----- | ------------------------------------------------------------- | ------------------------------ |
| 17, 23, 34, 43, 77, 103 | 6     | parse only (`not.toThrow`)                                    |                                |
| 28-33                   | 1     | parse + DB (`topAxisEnabled`)                                 |                                |
| 49-68                   | 1     | parse + DB (`getExcludes`)                                    | `%%` comments in the source    |
| 69-76                   | 1     | parse + DB-call spy (`setTodayMarker('off')`)                 | spy                            |
| 113-145                 | 1     | parse + DB tasks                                              | `Design jison grammar:des1, …` |
| 146-177                 | 1     | parse + DB tasks                                              | `after`, `until`               |
| 178-208                 | 5     | parse + DB flags (`tags`)                                     | `it.each` rows                 |
| 209-222                 | 1     | parse + DB-call spy (`setClickEvent`)                         | spy                            |
| 223-241                 | 1     | parse + DB-call spy (`setClickEvent` args)                    | spy                            |
| 243-257                 | 1     | parse + DB getters (`accTitle`, `accDescr`)                   |                                |
| 258-275                 | 1     | parse + DB getters (multiline `accDescr`)                     |                                |
| 277-283                 | 7     | parse + DB (`getWeekday`)                                     | `it.each` rows                 |
| 285-294                 | 2     | parse only (`__proto__` and `constructor` ids; `not.toThrow`) | `it.each` rows                 |

### 5.4 Test titles that a port must keep

Mapped to the port's test names (translated mechanically, as `flowchart/PLAN.md` section 10 does):

- DB, `it.each` titles use `$str`, `$testName`, `$fn`, `$type`; keep the row text.
- The TZ test (`ganttDb.spec.ts:488`) is ignored with reason `port: fixed UTC zone` (or ported with a zone parameter; a decision, section 8).
- The `Invalid date` and `Failed to find` messages are asserted by text (545, 293). Per the test strategy, change them to "fails with the message's class" where the port's error text differs.

---

## 6. Renderer

### 6.1 Entry, inputs and DOM use

`draw(text, id, version, diagObj)` (`ganttRenderer.js:81`). It reads the DB and the config and writes the SVG into the existing `<svg id>` (`98`, `152`).

| Read                                                                                    | Lines              |
| --------------------------------------------------------------------------------------- | ------------------ |
| `getConfig().gantt`                                                                     | 82                 |
| `securityLevel` (sandbox branches only)                                                 | 86-96, 508-531     |
| `db.setDiagramId(id)` (the prefix of the click-handler selectors, `ganttDb.js:790-800`) | 84                 |
| `db.getTasks()`                                                                         | 109                |
| `db.getDisplayMode()`, `conf.displayMode`                                               | 126                |
| `db.getExcludes()`, `db.getIncludes()`                                                  | 220-221            |
| `db.getTodayMarker()`                                                                   | 857                |
| `db.getDateFormat()`                                                                    | 570, 646           |
| `db.getAxisFormat()`                                                                    | 647                |
| `db.getTickInterval()`, `conf.tickInterval`                                             | 663                |
| `db.getWeekday()`, `conf.weekday`                                                       | 675, 744           |
| `db.topAxisEnabled()`, `conf.topAxis`                                                   | 730                |
| `db.getLinks()`                                                                         | 278 (sandbox only) |
| `db.getDiagramTitle()`                                                                  | 192                |
| `db.isInvalidDate(...)`                                                                 | 575                |

DOM: `doc.getElementById(id)` (98), `elem.parentElement.offsetWidth` (99), `root.select(...)` (152), `getBBox().width` (412, 445), `doc.createElementNS` (813-817), `setAttribute` on the viewBox (151).

### 6.2 Dimensions and scale

```
W = conf.useWidth ?? parent.offsetWidth          # 98-107; the reference's own fallback is 1200 (101-103)
rows = count(non-vert tasks)                      # 116-148
H  = 2*topPadding + rows*(barHeight + barGap)     # 125, 144
# compact: H = 2*topPadding + Σ_category (maxIntersections_c + 1)*(barHeight + barGap)   # 126-142
viewBox = "0 0 W H"                               # 151
scale = scaleTime()
        .domain([min startTime, max endTime])     # 155-163, over ALL tasks, vert included
        .rangeRound([0, W - leftPadding - rightPadding])   # 164
```

- `min` and `max` ignore undefined (d3-array), so unresolved tasks do not move the domain.
- `rangeRound` uses `Math.round(a*(1-t) + b*t)` (`d3-interpolate/src/round.js:3`). Every scale output is therefore an integer. The text is placed at half-integers (`(endX-startX)/2 + startX`), as the output shows (`881.5`).
- `sidePad` is `leftPadding` for every x (`224` passes `leftPadding`); `rightPadding` only enters the range and the background width.

Verified numbers, `tick-is-1-day` fixture (W = 1200): H = 196 = 100 + 4×24; range = 1050; bars and labels as in 6.4.

### 6.3 Ordering

1. `categories`: unique section names of non-vert tasks, in DB order (116-122).
2. `taskArray.sort(taskCompare)` by `startTime`. Stable; ties keep DB order; undefined compares equal (170-184). This sorts the DB array in place (`getTasks` returns the same array, `ganttDb.js:178`), so the DB order changes after `draw`. Nothing in `draw` reads `taskDb` afterwards.
3. `drawRects` sorts vert tasks last, stably (240).

### 6.4 Bars and labels

Notation: `X(t) = scale(t)`, `sp = leftPadding`, `bh = barHeight`, `gap = barHeight + barGap`, `tp = topPadding`, `gl = gridLineStartPadding`. `order` is the DB value, or the compact value (6.5).

| Attribute          | Non-milestone, non-vert                                                     | Milestone                                                  | Vert                            |
| ------------------ | --------------------------------------------------------------------------- | ---------------------------------------------------------- | ------------------------------- |
| `id`               | `${id}-${task.id}` (285)                                                    | same                                                       | same                            |
| `rx`, `ry`         | 3, 3 (287-288)                                                              | same                                                       | same                            |
| `x`                | `X(start) + sp` (298)                                                       | `X(start) + sp + 0.5*(X(end)-X(start)) - 0.5*bh` (291-296) | same as non-milestone           |
| `y`                | `order*gap + tp` (306)                                                      | same                                                       | `gl` (304)                      |
| `width`            | `X(renderEnd \|\| end) - X(start)` (315)                                    | `bh` (310)                                                 | `0.08*bh` (313) = 1.6           |
| `height`           | `bh` (321)                                                                  | `bh`                                                       | `nonVertCount*gap + 2*bh` (319) |
| `transform-origin` | `"{X(start)+sp+0.5*(X(end)-X(start))}px {order*gap+tp+0.5*bh}px"` (323-337) | same                                                       | same                            |
| `class`            | see 6.4a                                                                    | same plus `milestone`                                      | `vert`                          |

Note the `end` in `transform-origin` is `endTime`, not `renderEndTime`.

**6.4a Bar class string** (338-388). Built in this order, then `secNum` is appended, then `' ' + classStr`:

```
taskClass = ''
if active:   taskClass = crit ? ' activeCrit' : ' active'
elif done:   taskClass = crit ? ' doneCrit'   : ' done'
elif crit:   taskClass += ' crit'
if taskClass == '': taskClass = ' task'
if milestone: taskClass = ' milestone ' + taskClass
if vert:      taskClass = ' vert ' + taskClass
taskClass += secNum                           # secNum = categoryIndex % numberSectionStyles
taskClass += ' ' + classStr                   # classStr = classes.join(' ') (may be '')
class = 'task' + taskClass
```

Examples from the output: `task task0 ` (trailing blank), `task milestone  crit0 ` (two blanks), `task vert  task0 `.

**6.4b Labels** (391-506). One `<text>` per task, same parent `<g>` as the bars, with `id="${id}-${task.id}-text"`, `font-size = fontSize`, `text = task.task` (raw).

`x` (400-424), with `textWidth = getBBox().width` (412):

```
startX = X(start); endX = X(renderEnd || end)
if milestone: startX += 0.5*(X(end)-X(start)) - 0.5*bh ; endX = startX + bh
if vert:      x = X(start) + sp
elif textWidth > endX - startX:
      x = (endX + textWidth + 1.5*leftPadding > W) ? startX + sp - 5 : endX + sp + 5
else:         x = (endX - startX)/2 + startX + sp
```

`y` (425-436): vert gives `gl + nonVertCount*gap + 60`; otherwise `order*gap + bh/2 + (fontSize/2 - 2) + tp`. With `fontSize` 11 the term `(fontSize/2 - 2)` is 3.5, so the first task's label is at `0 + 10 + 3.5 + 50 = 63.5`, as the output shows.

`text-height` attribute = `bh` (437).

`class` (438-506), **uses different geometry**: `startX = X(start)` (no milestone shift), `endX = X(end)` (milestone: `startX + bh`) (439-443). `textWidth` again from `getBBox`. Then:

```
taskType = active ? (crit ? 'activeCritText' : 'activeText') + secNum : ''
if done:  taskType += (crit ? ' doneCritText' : ' doneText') + secNum
elif crit: taskType += ' critText' + secNum
if milestone: taskType += ' milestoneText'
if vert:      taskType += ' vertText'
if textWidth > endX - startX:
    if endX + textWidth + 1.5*leftPadding > W: class = classStr + ' taskTextOutsideLeft taskTextOutside' + secNum + ' ' + taskType
    else:                                      class = classStr + ' taskTextOutsideRight taskTextOutside' + secNum + ' ' + taskType + ' width-' + textWidth
else:                                          class = classStr + ' taskText taskText' + secNum + ' ' + taskType + ' width-' + textWidth
```

The `x` and `class` branches can disagree: `x` uses `renderEndTime` (and the milestone shift), while `class` uses `endTime` and no shift. Replicate both. Probed example: `deadlineB` (milestone) has `taskTextOutsideLeft` in class and `x = 1110`; `deadlineA` has `taskTextOutsideRight` and `x = 806`.

`width-` is `textWidth` as a JS number. The reference reads float32 from `getBBox` (flowchart notes, RENDER-PLAN §Decisions), so the printed digits are float32 values.

**Text content**: the label is the raw task name, including trailing blanks (for example `"A task           "`). Whether Chromium's `getBBox` counts trailing blanks is **unverified**; section 8.

### 6.5 Section rows and compact layout

Background rects (238-273): one `<rect>` per unique `order` value:

```
x = 0 ; y = order*gap + tp - 2 ; width = W - rightPadding/2 ; height = gap
class = 'section section' + (categoryIndex(task.type) % numberSectionStyles)     # 265-272
```

Non-compact: `order` is the DB order (0 … n-1), so each task has its own row (`categoryHeights[c]` = task count, 145-147).

Compact (`displayMode: compact`, 126-142, `getMaxIntersections` 59-77):

```
for each category c in Object.keys(categoryElements):           # JS key order (6.9)
    maxJ = getMaxIntersections(tasks_c, offset)                  # offset = sum of earlier categories' heights
    categoryHeight_c = maxJ + 1
    H += categoryHeight_c * gap
getMaxIntersections(tasks, offset):
    timeline = [-Infinity] * len(tasks)
    sorted = tasks sorted by (startTime - startTime) || (order - order)   # Date subtraction
    for e in sorted:
        for j in 0 ..:
            if e.startTime >= timeline[j]:  timeline[j] = e.endTime ; e.order = j + offset ; maxJ = max(maxJ, j) ; break
    return maxJ
```

Note: this writes `order` into the DB's task objects (67), so the compact row numbers are render-time and change the task `order` seen by later draws.

### 6.6 Section labels

`vertLabels` (799-848). One `<text>` per category, in `Object.keys(categoryHeights)` order:

```
rows = name.split(/<\/?br\s*\/?>/gi)                 # common.ts:8
dy = -(rows.length - 1)/2 em                           # 811
y  = (count_c * gap)/2 + prevGap*gap + tp              # 829-838; prevGap = sum of earlier counts (cumulative)
text: x="10", dy on the text element; each tspan alignment-baseline="central" x="10", tspans after the first with dy="1em"
font-size = sectionFontSize ; class = 'sectionTitle sectionTitle' + (categoryIndex % numberSectionStyles)
```

Verified: a section named `A section<br>multiline` gives `dy="-0.5em"` and two tspans (probed). Tasks without a section give the name `''` (probed: `sectionTitle sectionTitle0` at y = 62). Note the odd `prevGap` closure: it accumulates once per index (829-835), which is the cumulative sum described.

### 6.7 Axes and grid

`makeGrid` (645-793):

```
axisFormat = db.axisFormat || (db.dateFormat === 'D' ? '%d' : conf.axisFormat ?? '%Y-%m-%d')    # 646-655
bottom = axisBottom(scale).tickSize(-H + tp + gl).tickFormat(timeFormat(axisFormat))           # 657-659
custom ticks: re = /^([1-9]\d*)(millisecond|second|minute|hour|day|week|month)$/ on (db.tickInterval || conf.tickInterval)   # 661-664
   every = parseInt(n) ; interval = unit ; weekday = db.weekday || conf.weekday                   # 667-675
   est = ceil((max - min) / dayjsDuration(unit: every).asMilliseconds())                           # 627-637, 681
   if est > 10000: warn, default ticks (683-689)
   else bottom.ticks(d3 interval): millisecond→timeMillisecond.every(n), second→timeSecond, minute→timeMinute,
        hour→timeHour, day→timeDay, week→timeMonday…timeSunday (weekday).every(n), month→timeMonth   # 691-713
svg.append('g').attr('class','grid').attr('transform', `translate(${sp}, ${H - 50})`)            # 718-721
   .call(bottom).selectAll('text').style('text-anchor','middle').attr('fill','#000').attr('stroke','none').attr('font-size',10).attr('dy','1em')   # 723-728
if topAxis (db or conf):
   top = axisTop(scale).tickSize(-H + tp + gl).tickFormat(...)                                     # 731-733
   same custom-tick logic, without the warning; applied only if est <= 10000                       # 753
   svg.append('g').attr('class','grid').attr('transform', `translate(${sp}, ${tp})`).call(top)...  # 781-790 (no dy)
```

Default ticks: `scale.ticks(10)` (d3 default count, `d3-scale/src/time.js:47-50`) → `d3-time/src/ticks.js` `tickInterval` (43-55): target = |stop - start| / 10; the interval is the table entry (`ticks.js:14-33`) nearest to the target by ratio, with `year.every(tickStep(...))` for targets above a year and `millisecond.every(max(tickStep, 1))` below a second. Table (ms): 1 s, 5 s, 15 s, 30 s, 1 min, 5, 15, 30, 1 h, 3, 6, 12, 1 day, 2 days, 1 week (Sunday), 1 month (30 days), 3 months, 1 year.

`every(n)` (`d3-time/src/interval.js:58-65`): `interval.filter(field ? d => field(d) % n === 0 : d => interval.count(0, d) % n === 0)`. The field is the last argument of each unit's `timeInterval` (`minute.js:11` `getMinutes`, `hour.js:11` `getHours`, `day.js:8` `getDate() - 1`, `month.js:11` `getMonth`, `year.js:11` `getFullYear`; weeks have none and use `count`). `d3-time/src/duration.js:6` sets a month to 30 days, so the d3 tick table uses 30-day months, while the reference's estimate (6.7, `dayjs.duration`) uses 30.4167 days. `millisecond.every(k)` (`millisecond.js:12-…`) floors `date / k`.

Axis markup (`d3-axis/src/axis.js`):

- `tickSizeOuter` = `tickSizeInner` (138-146) = `-H + tp + gl`. The domain path is `M{r0},{k·size}V{offset}H{r1}V{k·size}` (bottom, k = 1; top uses k = -1). With H = 196: `M0.5,-111V0.5H1050.5V-111`.
- `offset` (line 38): `window.devicePixelRatio > 1 ? 0 : 0.5`. The **reference's e2e DPR is 2** (`playwright.config.ts:54`), so e2e output has offset 0. The jsdom probe has no `devicePixelRatio`, so it uses 0.5. Both the probe output and the e2e output are valid for their own settings; the port must state which one it reproduces.
- Tick: `<g class="tick" opacity="1" transform="translate(x+offset,0)">` with `<line y2 = k·size>` and `<text fill="currentColor" y = k·3 dy="0.71em">`. The gantt overrides `fill` (`#000`), `stroke`, `font-size` (10) and `dy` (`1em`) (723-728).
- `text-anchor` is set by the gantt through `style` (`style="text-anchor: middle;"`).

Number formats: `timeFormat` from d3-time-format (`locale.js`, en-US, local time). Directives used by the fixtures: `%d %m %H %M %L %S %s %Y %p` and literal text, with d3's padding rules. `%s` is `Math.floor(ms/1000)` (`d3-time-format/src/locale.js:695-697`), `%L` is the three-digit milliseconds and `%S` the two-digit seconds (528-546).

### 6.8 Excluded ranges

`drawExcludeDays` (543-616), run before the grid (`makeGantt`, 213-222) and only when excludes or includes are set:

```
minTime = min startTime, maxTime = max endTime   (over all tasks, vert included; 547-557)
if dayjs(maxTime).diff(dayjs(minTime), 'year') > 5: warn, return     # 563-568
d = dayjs(minTime)                                 # keeps the time of day
while d <= maxTime:                                # 574
    if isInvalidDate(d, fmt, excludes, includes):  start a range or extend it (576-583)
    else: close the range (585-588)
    d = d + 1 day                                  # 590 (local)
rect per range:
    id = `${id}-exclude-${start.format('YYYY-MM-DD')}`        # 597
    x = X(start.startOf('day')) + sp ;  y = gl                 # 598-599
    width = X(end.endOf('day')) - X(start.startOf('day'))      # 600
    height = H - tp - gl                                       # 602
    transform-origin = `${X(start)+sp+0.5*(X(end)-X(start))}px ${i*gap + 0.5*H}px`   # 603-614
    class = 'exclude-range'
```

The reference uses `start` (not `startOf`) in `transform-origin` and `startOf` in `x` and `width`; replicate both. `isInvalidDate` is the DB function (3.4), so the same tokens apply.

### 6.9 Today marker

`drawToday` (856-876):

```
if todayMarker == 'off': return                              # 858
g.today (class 'today') with line:
   x1 = x2 = X(now) + sp            # 863-868
   y1 = titleTopMargin ; y2 = H - titleTopMargin               # 869-870
   class = 'today'
   if todayMarker != '': style = todayMarker.replace(/,/g, ';')   # 873-875
```

`now` is `new Date()` (863). Probed: with the clock at 1010-10-10 and `W = 1200`, the line is at `x = 600` for the domain of `should-style-today-marker`. For the compact fixture the line is at x = -3883 (outside the chart, as expected).

### 6.10 Title

`svg.append('text').text(db.getDiagramTitle()).attr('x', W/2).attr('y', titleTopMargin).attr('class', 'titleText')` (190-195). Appended after `makeGantt`. It is always present, with empty text when there is no title.

### 6.11 SVG root and wrapper

The root `<svg>` is created by `appendDivSvgG` (`mermaidAPI.ts:404-418`): `id`, `width="100%"`, `xmlns`, and an empty first `<g>` child (418). The gantt renderer then sets `viewBox` (151) and, through `configureSvgSize` (188; `setupGraphViewbox.js:43-47`), `width="100%"` and `style="max-width: Wpx;"` when `useMaxWidth` is true (default), or `height` and `width` attributes when it is false (the `usemaxwidth-false` fixture). `addA11yInfo` (`mermaidAPI.ts:757`) adds `role="graphics-document document"` and `aria-roledescription="gantt"` (`accessibility.ts:19-33`) and `<title>`/`<desc>` from accTitle/accDescr (`accessibility.ts:45-…`). The `decodeEntities` pass runs over the serialised string (`mermaidAPI.ts:359`).

The top-level order of the children of `<svg>`, as the probe writes them (checked with an XML parse of `svg-utc/`, sections 6.7-6.8 explain each part):

```
<svg>
  <g/>                                  # empty, from appendDivSvgG (reference only, mermaidAPI.ts:418)
  <g>  exclude-range rects              # drawExcludeDays, only if excludes or includes are set
  <g class="grid">  bottom axis         # makeGrid (run before drawRects)
  <g class="grid">  top axis            # makeGrid, only if topAxis (db or config)
  <g>  section background rects         # drawRects
  <g>  task rects, then task texts      # drawRects (rects and texts in one <g>)
  <g>  section label texts              # vertLabels
  <g class="today"> line                # drawToday, unless todayMarker is 'off'
  <text class="titleText">              # appended last (190-195)
```

`makeGrid` runs before `drawRects`, so the top axis is written before the section rects. Verified on the top-axis fixture (2 `grid` groups, then 2 `section` rect groups) and on a fixture with no excludes (no leading `<g>` with rects).

Probed attributes on a task rect: `id`, `rx`, `ry`, `x`, `y`, `width`, `height`, `transform-origin`, `class`, in that order. Text: `id`, `font-size`, `x`, `y`, `text-height`, `class`, then the text.

### 6.12 Styles (`styles.js`)

- `.section { stroke:none; opacity:0.2 } .section0 { fill: sectionBkgColor } .section2 { fill: sectionBkgColor2 } .section1,.section3 { fill: altSectionBkgColor; opacity:0.2 }` (11-28).
- `.task { stroke-width: 2 }`, `.taskText { text-anchor: middle }`, `.taskTextOutsideRight { text-anchor: start }`, `.taskTextOutsideLeft { text-anchor: end }` (83-101).
- `.milestone { transform: rotate(45deg) scale(0.8,0.8) }` (239-241). **This is a CSS transform**, applied by the browser around `transform-origin`. A port without CSS must write the transform into the geometry (the reference's getBBox does not include it, so the comparison needs the transformed box).
- `.clickable` and `.taskText.clickable` set `cursor:pointer` and bold fill (104-126).
- `.today { fill:none; stroke: todayLineColor; stroke-width:2px }` (72-76), overridden by the inline `style` when given.
- `.exclude-range { fill: excludeBkgColor }` (7-9).
- `.grid .tick { stroke: gridColor; opacity:0.8; shape-rendering: crispEdges }`, `.grid .tick text { font-family; fill: textColor }`, `.grid path { stroke-width: 0 }` (54-67).
- `.titleText { text-anchor: middle; font-size: 18px; fill; font-family }` (282-287).
- `.vert { stroke: vertLineColor }`, `.vertText { font-size: 15px; text-anchor: middle; fill: vertLineColor !important }` (265-273).
- `.sectionTitle { text-anchor: start; font-family }` (46-49).

### 6.13 Number formatting

Coordinates are JS numbers printed with `toString()` (shortest round trip): `337.5px`, `63.5`, `1162.5`, `-111`, `NaN` (unresolved). The port's writer must print the same digits (Rust's `{}` for `f64` gives the same shortest digits for these ranges; check `-0`, exponent forms and `NaN`). Text widths in class names (`width-112.19999999999999`) are the same kind of number.

---

## 7. Fixtures

52 files, all with front matter (`description`). Counts are from a script over the files (read) plus the probes.

| Feature                                                                                           | Fixtures | Examples / notes                                                                                                                                          |
| ------------------------------------------------------------------------------------------------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `dateFormat YYYY-MM-DD` (plain)                                                                   | most     |                                                                                                                                                           |
| `dateFormat` with time (`HH:mm`, `HH:mm:ss`, `YYYY-MM-DD HH:mm:ss`)                               | 5        | compact, excluding-friday-saturday (ss), excluding-specific-date                                                                                          |
| `dateFormat` `x`, `X`, `ss`, `D`, `SSS`                                                           | 8        | milliseconds, numeric-timestamps (x, X), seconds-only, `dateFormat D`, 2-milliseconds, 2-seconds                                                          |
| `dateFormat :YYYY-MM-DD` (leading colon)                                                          | 4        | visual-check `dark`, `default`, `forest`, `neutral` (date fallback, 3.5)                                                                                  |
| no `dateFormat`                                                                                   | 1        | issue-1060 (all dates through the fallback, UTC)                                                                                                          |
| `axisFormat`                                                                                      | 27       | `%d/%m`, `%m-%d`, `%d`, `%L`, `%Lms`, `%Ss`, `%s`, `%H:%M`, `%d %H:%M`, `%Hh%M`, `%Y-%m-%d` (default)                                                     |
| `tickInterval`                                                                                    | 14       | millisecond, second, minute (15), hour (6, 6hour), day (1day, with topAxis), week (1week, with monday), month (1month), 250millisecond (x and X fixtures) |
| `topAxis`                                                                                         | 2        | both set `config.gantt.topAxis: true` in front matter (`tick-is-1-day-and-topaxis-is-true`, `data-labels-at-the-top-when-topaxis-is-true`)                |
| `excludes weekends`                                                                               | 16       | counted in the fixture bodies                                                                                                                             |
| `excludes weekdays <date>` (token `weekdays`, no effect)                                          | 7        |                                                                                                                                                           |
| `excludes` with date or day                                                                       | several  | `2014-01-10`, `2025-07-07`, `2014-01-15 2014-01-16`, multi-line excludes (issue-6270)                                                                     |
| `weekend friday` / `weekend saturday`                                                             | 4        |                                                                                                                                                           |
| `weekday monday`                                                                                  | 1        |                                                                                                                                                           |
| `todayMarker off`                                                                                 | 2        |                                                                                                                                                           |
| `todayMarker <style>`                                                                             | 1        | `stroke-width:5px,stroke:#00f,opacity:0.5`                                                                                                                |
| `displayMode: compact` (front matter)                                                             | 1        |                                                                                                                                                           |
| `useMaxWidth: false` (config)                                                                     | 1        |                                                                                                                                                           |
| `config.theme` (dark, default, forest, neutral, redux-color, redux-dark-color, redux, redux-dark) | 3 + 8    | dark-theme regression fixtures (3) and visual-check (8)                                                                                                   |
| `accTitle` / `accDescr`                                                                           | 4        | visual-check `dark`, `default`, `forest`, `neutral`                                                                                                       |
| `crit`, `done`, `active`                                                                          | 17       |                                                                                                                                                           |
| `milestone` keyword                                                                               | 1        | multiple-dependencies                                                                                                                                     |
| `vert`                                                                                            | 2        | vert-tag, vertical-markers (18 vert tasks, duplicate ids `v1`, `v2`, `v3`)                                                                                |
| `after` with two ids, `until` with two ids                                                        | 2 + 1    | `after b a`, `after onboarding litreview`, `until deadlineA bossaway`                                                                                     |
| `%%` comment lines                                                                                | 2        | and `%% week` between `excludes` lines                                                                                                                    |
| `<br>` variants in section names                                                                  | 1        | `<br>`, `<br/>`, `<br />`, `<br\t/>`                                                                                                                      |
| `#` or `;` in names and titles                                                                    | 7        | hashtag and semicolon fixtures (text, not entity codes)                                                                                                   |
| `click`                                                                                           | 0        | none                                                                                                                                                      |
| Date beyond 2000 years (year 9999)                                                                | 1        | pr-check very-large-intervals (exclude scan skipped, 5-year rule)                                                                                         |
| Date typo (`202-12-01`)                                                                           | 1        | typo fixture (legacy path)                                                                                                                                |
| `dateFormat X`, large ticks                                                                       | 1        | numeric-timestamps-x-2: 481 ticks                                                                                                                         |

Features that need a browser or font ferrigura does not have: none in the parser. The renderer needs `getBBox` for label widths, and every fixture uses the `courier` font (`e2e/helpers/util.ts:74`), which the bundled fonts do not contain (section 8). Fallback fonts are not needed for ASCII text.

Every fixture renders in the reference through the jsdom probe with no error (52 of 52, `render-utc.log`).

---

## 8. Risks, effort and open questions

### 8.1 Risks (most serious first)

| #   | Risk                                                                                                                 | Mitigation                                                                                                                                                                                                                              |
| --- | -------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | Date semantics are not one rule: dayjs strict round trip, clock defaults, V8 `Date` fallback, local vs UTC (3.3-3.6) | Port each rule with its probe; fixed zone UTC; `now` as input; a table test over the 17 fallback strings and the 15 probe rows of 3.10                                                                                                  |
| R2  | V8 legacy `Date` parsing (`0`, `202-12-01`, `2019-09-1`, `202304`)                                                   | Implement the forms listed in 3.5 exactly; reject the rest with `Invalid date` (differs from V8 only outside the list)                                                                                                                  |
| R3  | Text width: reference uses Courier (`courier` font) measured by Chromium; ferrigura measures only its bundled faces  | The harness forces one font on both sides (as M5 does for flowchart: `htmlLabels` off, the same face); the fixture's Courier is replaced. Pixel values of the reference change, so the port's own goldens must come from the forced run |
| R4  | `getBBox` and trailing blanks (`"A task           "`): Chromium may not count them                                   | **Unverified**: measure in Chromium before the width rule is written                                                                                                                                                                    |
| R5  | Width: reference width is the body's `offsetWidth` (about 1424 e2e)                                                  | Harness sets `useWidth` on both sides; the port's default is 1200 (the reference's own fallback at 101-103)                                                                                                                             |
| R6  | d3-axis `offset` depends on DPR (0.5 at DPR 1, 0 at DPR 2)                                                           | State DPR in the harness; the e2e baselines use DPR 2                                                                                                                                                                                   |
| R7  | `draw` mutates the DB order (`taskArray.sort`, `drawRects` sort, compact `order` writes)                             | Port `draw` on a copy; keep the model immutable                                                                                                                                                                                         |
| R8  | Quirks that must be reproduced, not fixed (Q1-Q10, 2.5; `taskDb` leak 3.1; `:YYYY` fixtures; `dateFormat  D`)        | Each gets a test that pins the reference output; the compatibility promise (README) requires it                                                                                                                                         |
| R9  | Unresolved tasks write `NaN` into SVG                                                                                | Decide: reproduce `NaN` (compatibility) or error (safer). Not in any fixture                                                                                                                                                            |
| R10 | Links in strict mode are a JavaScript click (`window.open`), not an anchor                                           | Decide between `<a>` (README: links stay as `<a>`) and nothing. No fixture uses `click`                                                                                                                                                 |
| R11 | Integer-like section names order: `Object.keys` puts integer keys first (6.5)                                        | Replicate JS key order in the port's section map                                                                                                                                                                                        |
| R12 | Sanitiser coverage for titles and accessibility texts (DOMPurify subset)                                             | Reuse `sanitize.rs`; the fixtures use `;` and `#` only                                                                                                                                                                                  |
| R13 | `Object.keys` and JS number printing in the output                                                                   | Replicate; check `NaN`, `-0` and float32 `width-` values                                                                                                                                                                                |

### 8.2 Effort estimate

Relative to the flowchart port (M3 parser plus M5 renderer, with ELK, shapes, Markdown and link handling):

| Area                              | Estimate                                      | Reason                                                                                                    |
| --------------------------------- | --------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Lexer and grammar (`gantt.jison`) | about a third of the flowchart parser         | small grammar, but 20 lexer rules with state and quirks to reproduce (2.2-2.5)                            |
| DB and date semantics             | medium-high; the largest single item          | dayjs strict parse, `add`, V8 fallback, clock, zone (section 3)                                           |
| Canonical model and exporter      | small                                         | 16 fields plus the title rules (section 4)                                                                |
| Specs (82 cases)                  | small to medium                               | DB and parser level only; no DOM (section 5)                                                              |
| Renderer (`ganttRenderer.js`)     | about a third to a half of the flowchart's M5 | no layout engine, no shapes; d3-time ticks and time formats, axis markup, and `getBBox` placement are new |
| Geometry harness                  | medium                                        | Chromium run; forced font; forced width; comparison of bar, label, axis and today positions               |

Overall: roughly a third to a half of the flowchart's M3 plus M5 work (**estimate**, not measured).

### 8.3 Open questions for the coordinating session

1. **Zone and clock.** Fix the port to UTC with an explicit `now`? Confirm which zone and clock the e2e baselines used (the e2e workflow is not in this tree).
2. **V8 fallback.** Implement the 3.5 forms, or reject them? (Rejecting changes the output of two fixtures and two DB spec expectations.)
3. **Raw blanks.** Keep `dateFormat`, `axisFormat`, `title` and `todayMarker` raw, with their blanks and their quirks, as the compatibility promise implies? (Recommended: yes.)
4. **Width in the harness.** Set `useWidth` on both sides? (Recommended: yes; it removes the DOM dependency from the comparison.)
5. **Font in the harness.** Force the bundled Lexica face on both sides, as M5 does? (Recommended: yes; the courier baseline is then not comparable and is replaced by a forced run.)
6. **DPR.** Compare at DPR 1 (offset 0.5) or DPR 2 (offset 0)? The port must write the offset the harness uses.
7. **Links.** Emit `<a>` for `click … href` (as the sandbox branch does), or drop the links? No fixture uses them.
8. **Unresolved tasks.** Reproduce `NaN` or fail the render?
9. **Sanitising.** Confirm that gantt sanitises titles and accessibility texts but not task or section names (as read), and that the canonical-model page gets a gantt exception.
10. **The `TZ=America/Los_Angeles` DB test** (`ganttDb.spec.ts:488`): ignore it, or give the port a zone parameter?
11. **Stale e2e comment.** `mmd-snapshots.spec.ts:31-32` refers to `rendering/gantt/gantt.spec.js`, which does not exist here. Should a gantt e2e spec be added, or the comment fixed?

### 8.4 Unverified items (not confirmed in a browser or against a baseline)

- `getBBox().width` for every label, and whether trailing blanks count (R4). The probe used `0.6 × fontSize × length`, which is a guess.
- The e2e container width (about 1424) and the e2e zone.
- The d3-axis markup in Chromium at DPR 2.
- Chromium's rendering of `transform: rotate(45deg) scale(0.8,0.8)` on milestones (the port must reproduce the transformed box).
- V8's exact legacy `Date` rules beyond the probed inputs (3.5).
- Compact layout on real label widths (the probe's compact fixture uses estimated widths).

---

## Appendix A. Probe material (outside the repository)

Path: `/private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/m6-gantt/`

| File                                                            | What it does                                                                                |
| --------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `db/ganttDb.mjs`                                                | copy of `ganttDb.js` with stub imports (logger, config, utils, commonDb)                    |
| `db/runcases.mjs`, `db/dump.mjs`                                | runner: each case in a fresh module instance (the module state leaks otherwise)             |
| `db/cases-errors.mjs`, `db/cases-dates.mjs`, `db/cases-ids.mjs` | cases E*, D*, I\* in section 2.6, 3.3-3.8                                                   |
| `db/clock.mjs`                                                  | `--import` preload that pins `new Date()` to `FAKE_NOW`                                     |
| `p1-parser.cjs`, `p2-lexer.cjs`, `p3-lexer.cjs`                 | Jison parse traces with a recording `yy` (DB calls in order) and token dumps                |
| `pipeline.mjs`, `pipeline-lib.mjs`                              | front matter, entities, parse, `getTasks`; `fixtures-utc.jsonl`, `fx-*.jsonl` (three zones) |
| `fallback-scan.mjs`                                             | the 17 legacy-path strings across the fixtures                                              |
| `render/ganttRenderer.mjs`                                      | copy of `ganttRenderer.js` with stub imports                                                |
| `render/jsdom-render.mjs`                                       | renders a fixture to SVG under jsdom (`getBBox` estimate, `offsetWidth` 1200)               |
| `svg-utc/`                                                      | 52 rendered reference SVGs (TZ=UTC, clock pinned)                                           |
| `mini/`                                                         | minimal inputs used for 3.10 and 6.7 (ticks with blank, `dateFormat  D`, 12-link chain)     |

Rerun, for example: `cd …/m6-gantt && TZ=UTC FAKE_NOW=1010-10-10T00:00:00.000Z node --import ./db/clock.mjs db/runcases.mjs db/cases-dates.mjs`.

Not run: vitest, Chromium, the e2e suite, any `pnpm` command. No repository file was touched.
