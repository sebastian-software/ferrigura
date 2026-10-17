# quadrantChart (P2): research notes for the port

Status: research only, nothing in the repository changed except this file. Reference: the TypeScript checkout at this worktree's HEAD (`aa5c4338f`). The bundle `packages/mermaid/dist` was built on 2026-10-16 11:57, after the last change of `packages/mermaid/src/diagrams/quadrant-chart/` (2026-05-12) and of the shared helpers it reads (2026-10-16 09:23), so the probes below used the same code as the source.

Verification legend used below:

- **[V]** verified by running the reference: the parser and DB through the source (`tools/port/reference.ts`, Vite SSR, with `preprocessDiagram` and `encodeEntities` as `Diagram.fromText` gives them), the renderer through the harness's own page (`tools/port/geometry/common.ts`, `withPage` and `renderReference`, with the forced settings), or the library (d3-scale 4.0.2, khroma 2.1.0) in Node;
- **[S]** read from the source, with the file and line;
- **[U]** not verified; stated as an open point.

Probe material, not committed: `/private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/p2-quadrant/`. `probe-parse.mts` runs the reference's Jison parser and DB on a JSON map of inputs (`cases-quadrant*.json`, about 80 inputs; results in `out-parse*.json`). `render-ref.mts` renders `.mmd` files with the reference (`ref-quadrant/` holds the 14 fixtures). The probe for the scale, `d3-scale`'s `scaleLinear` against three candidate formulas on 1,011 inputs, was an inline Node check.

## 0. Summary

| Item              | Value                                                                                                                                                                                                                                                                                                                           |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Reference sources | 7 files without specs, 1,217 lines (`quadrant-chart/`: `quadrantBuilder.ts` 623, `quadrantDb.ts` 174, `quadrantRenderer.ts` 175, `parser/quadrant.jison` 189, `quadrantDetector.ts` 24, `quadrantDiagram.ts` 12, `utils.ts` 20)                                                                                                 |
| Specs             | 2 files, 29 test cases: 24 for the grammar (`parser/quadrant.jison.spec.ts`), 5 for the DB and the style parser (`quadrantDb.spec.ts`). No renderer spec                                                                                                                                                                        |
| Fixtures          | 14 `.mmd` files: 10 in `pr-check/`, 4 in `nightly-check/`; `visual-check/` is empty                                                                                                                                                                                                                                             |
| Parser            | Jison, 11 lexer start states, first-match rules (no longest match). A hand-written port is the recommendation (decision 0002)                                                                                                                                                                                                   |
| Renderer          | SVG built with d3 `select` and `append`. Positions come from the builder's arithmetic and d3's `scaleLinear` on [0, 1]. No text is measured anywhere in the folder                                                                                                                                                              |
| Biggest risks     | (1) The float bits of the point positions: d3 computes `a*(1-t)+b*t`, and `a+t*w` differs in 282 of 1,011 values [V]. (2) Garbage the reference writes for some coordinates (NaN, `translate(undefined, …)`, a hexadecimal `0x5` read as 5) [V]. (3) A NaN colour from khroma in the default point fill of the redux themes [V] |
| Effort            | About 1,100 lines of Rust plus about 300 lines of tests for the 29 specs. The parser is the bulk; the builder and the renderer are plain arithmetic                                                                                                                                                                             |

Everything below is about the reference as it is. Where the reference produces a value a reader would call garbage, the notes say so, because the M6 rules name such exceptions in `PLAN.md`.

## 1. Sources

| File                                            | Lines              | Role                                                                                                                                                                                                                                                                     |
| ----------------------------------------------- | ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `diagrams/quadrant-chart/quadrantDiagram.ts`    | 1-12               | Definition: parser, DB, renderer; `styles: () => ''` (line 11), so the diagram adds no CSS                                                                                                                                                                               |
| `diagrams/quadrant-chart/quadrantDetector.ts`   | 7-10               | Detector `/^\s*quadrantChart/`, case-sensitive (line 10). Loader 13-16                                                                                                                                                                                                   |
| `diagrams/quadrant-chart/parser/quadrant.jison` | 1-77               | Lexer: options (2), start states (4-14), rules (16-77)                                                                                                                                                                                                                   |
| same                                            | 81-136             | Grammar: `idString` (85-93), `style` (95-98), `stylesOpt` (100-104), `classDefStatement` (106-108), `start` (110-114), `document`/`line` (116-123), `statement` (125-136)                                                                                                |
| same                                            | 138-189            | `points` (138-143), `axisDetails` (145-152), `quadrantDetails` (154-159), `eol` (161-165), `text` (167-175), `alphaNum` (177-182), tokens (185-187)                                                                                                                      |
| `diagrams/quadrant-chart/quadrantDb.ts`         | 1-174              | DB: text setters (21-62), `parseStyles` (64-93), `addPoint` (95-106), `addClass` (108-110), `setWidth`/`setHeight` (112-118), `getQuadrantData` (120-145), `clear` (147-150), export (152-174)                                                                           |
| `diagrams/quadrant-chart/quadrantBuilder.ts`    | 1-623              | Types (10-124), defaults (138-199), `clear`/`setData`/`addPoints`/`setConfig` (201-229), `calculateSpace` (231-284), `getAxisLabels` (286-382), `getQuadrants` (384-473), `getQuadrantPoints` (475-514), `getBorders` (516-586), `getTitle` (588-602), `build` (604-622) |
| `diagrams/quadrant-chart/quadrantRenderer.ts`   | 17-175             | `draw`: root and sizing (40-59), the data (62), groups and elements (64-170)                                                                                                                                                                                             |
| `diagrams/quadrant-chart/utils.ts`              | 1-20               | `validateHexCode` (8-10), `validateNumber` (12-14), `validateSizeInPixels` (16-18), `InvalidStyleError` (1-6)                                                                                                                                                            |
| `schemas/config.schema.yaml`                    | 1287-1407, 457-470 | `QuadrantChartConfig`: the defaults (the table in section 6.5). `BaseDiagramConfig.useMaxWidth` default true (470)                                                                                                                                                       |
| `themes/theme-redux-color.js`                   | 338-355            | The quadrant theme variables of the redux-color theme. Point fill ternary at 350-352                                                                                                                                                                                     |
| `themes/theme-default.js`                       | 313-336            | The same for `default`, which the harness does not use                                                                                                                                                                                                                   |
| `mermaidAPI.ts`                                 | 359, 413-419       | Whole-SVG `decodeEntities` (359). The container `svg` with `width="100%"` and the empty `g` (413-419)                                                                                                                                                                    |
| `accessibility.ts`                              | 22-30              | `aria-roledescription` on the root svg                                                                                                                                                                                                                                   |
| `setupGraphViewbox.js`                          | 23-46              | `calculateSvgSizeAttrs` and `configureSvgSize`                                                                                                                                                                                                                           |
| `node_modules/d3-scale`                         | 4.0.2              | `scaleLinear` (`d3` 7.9.0 re-exports it, `packages/mermaid/package.json`)                                                                                                                                                                                                |

## 2. Parser

### 2.1 Lexer

All rules are case-insensitive (`%options case-insensitive`, line 2). The rules are tried in file order and the first match wins, so the order below matters.

| Rules (line)                                                                                                         | Start state                     | Token                                                                    | Notes                                                                                                          |
| -------------------------------------------------------------------------------------------------------------------- | ------------------------------- | ------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------- |
| `%%(?!\{)…` (16), `…%%…` (17), `%%…` (19)                                                                            | any                             | skipped                                                                  | A `%%` comment runs to the end of the line; a trailing `%%` after text also removes the rest of the line [V]   |
| `[\n\r]+` (18)                                                                                                       | any                             | NEWLINE                                                                  |                                                                                                                |
| `title` (21)                                                                                                         | any → `title`                   | title                                                                    | Then `(?!…)*[^\n]*` (22) is the value: the rest of the line, colon and all [V]                                 |
| `accTitle\s*":"\s*` (24) and `accDescr…` (26)                                                                        | any → `acc_title` / `acc_descr` | acc_title, acc_descr                                                     | Values to the end of the line (25, 27)                                                                         |
| `accDescr\s*"{"\s*` (28)                                                                                             | any → `acc_descr_multiline`     | (none)                                                                   | Block form, values to `}` (29-30)                                                                              |
| `" "*"x-axis"" "*` (32), `y-axis` (33), `" "*\-\-+\>" "*` (34)                                                       | any                             | X-AXIS, Y-AXIS, AXIS-TEXT-DELIMITER                                      | The delimiter is `-->`, `--->` and so on [V]                                                                   |
| `" "*"quadrant-1"" "*` … `quadrant-4` (35-38)                                                                        | any                             | QUADRANT_1 … QUADRANT_4                                                  | A word `quadrant-2` anywhere in a text is this token, so it cannot be a label word [V]                         |
| `"classDef"` (39)                                                                                                    | any                             | CLASSDEF                                                                 |                                                                                                                |
| `["][`]`(41),`<md_string>…` (42-43)                                                                                  | any → `md_string`               | MD_STR                                                                   | Text in ``"`…`"`` [V]                                                                                          |
| `["]` (44), `<string>["]` (45), `<string>[^"]*` (46)                                                                 | any → `string`                  | STR                                                                      | Quoted text, may hold colons [V]                                                                               |
| `\:\:\:` (48) and `<class_name>^\w+` (49)                                                                            | any → `class_name`              | class_name                                                               | `:::name`; the name is a word [V]                                                                              |
| `\s*\:\s*\[\s*` (51)                                                                                                 | any → `point_start`             | point_start                                                              | The `:` and `[` of a point                                                                                     |
| `<point_start>(1)\|(0(.\d+)?)` (52)                                                                                  | point_start → `point_x`         | point_x                                                                  | The `.` is unescaped (section 2.3)                                                                             |
| `<point_start>\s*\]" "*` (53)                                                                                        | point_start                     | (none)                                                                   | Pops back to the start state                                                                                   |
| `<point_x>\s*\,\s*` (54)                                                                                             | point_x → `point_y`             | (none)                                                                   |                                                                                                                |
| `<point_y>(1)\|(0(.\d+)?)` (55)                                                                                      | point_y                         | point_y                                                                  |                                                                                                                |
| `" "*"quadrantChart"" "*` (57)                                                                                       | any                             | QUADRANT                                                                 | The header                                                                                                     |
| `[A-Za-z]+` (59), `[^\x00-\x7F]+` (61)                                                                               | any                             | ALPHA, UNICODE_TEXT                                                      |                                                                                                                |
| `:` (62), `+` (63), `,` (64), `=` (65-66), `*` (67), `#` (68), `_` (69), `.` (70), `&` (71), `-` (72), `[0-9]+` (73) | any                             | COLON, PLUS, COMMA, EQUALS, MULT, BRKT, UNDERSCORE, DOT, AMP, MINUS, NUM |                                                                                                                |
| `\s` (74)                                                                                                            | any                             | SPACE                                                                    |                                                                                                                |
| `;` (75)                                                                                                             | any                             | SEMI                                                                     |                                                                                                                |
| `[!"#$%&'*+,-.`?\\\_/]` (76)                                                                                         | any                             | PUNCTUATION                                                              | Most of these characters are caught by earlier rules. `!`, `$`, `%`, `'`, `?`, `\` and `/` reach this rule [S] |
| `<<EOF>>` (77)                                                                                                       | any                             | EOF                                                                      |                                                                                                                |

Characters that match no rule are a lexical error. `<` is one of them [V] (`Point & <b>x</b>` fails at the `<`). `(`, `)` and `{` match no rule either [S].

### 2.2 Grammar

```
start      : eol start | SPACE start | QUADRANT document          (110-114)
document   : /* empty */ | document line                         (116-119)
line       : statement eol                                       (121-123)
eol        : NEWLINE | SEMI | EOF                                (161-165)
statement  : (empty) | classDefStatement | SPACE statement
           | axisDetails | quadrantDetails | points
           | title title_value | acc_title acc_title_value
           | acc_descr acc_descr_value | acc_descr_multiline_value
           | section                                             (125-135)
```

What the DB receives, in order (the calls are in the same lines as the rules):

| Rule (line)                                                        | DB call                                                                                                                                                         |
| ------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `title title_value` (132)                                          | `setDiagramTitle(value.trim())`, the common DB (`commonDb.ts` 28-30, sanitised)                                                                                 |
| `acc_title acc_title_value` (133)                                  | `setAccTitle(value.trim())` (common, sanitised)                                                                                                                 |
| `acc_descr acc_descr_value`, `acc_descr_multiline_value` (134-135) | `setAccDescription(value.trim())` (common, sanitised)                                                                                                           |
| `section` (135)                                                    | `addSection(…)`: unreachable (2.4, quirk Q1)                                                                                                                    |
| `classDefStatement` (106-108)                                      | `addClass(idString, stylesOpt)`                                                                                                                                 |
| `points` (138-143)                                                 | `addPoint(text, className, x, y, styles)`, four alternatives with and without class and styles                                                                  |
| `axisDetails` (145-152)                                            | `setXAxisLeftText`, `setXAxisRightText`, `setYAxisBottomText`, `setYAxisTopText`. A one-text axis with a delimiter gets `" ⟶ "` appended to its text (147, 150) |
| `quadrantDetails` (154-159)                                        | `setQuadrant1Text` … `setQuadrant4Text`                                                                                                                         |

The grammar passes `{text, type}` objects for texts (`text`, 167-175): `type` is `'text'`, or `'markdown'` for ``"`…`"`` strings, which the DB then ignores. The mock DB of the grammar spec (`parser/quadrant.jison.spec.ts`, 13-24) records these calls, so the 24 grammar tests check the call protocol, not the DB state.

### 2.3 Coordinates

The point rule is `(1)|(0(.\d+)?)` in both `point_start` (52) and `point_y` (55). In a JavaScript regular expression, the `.` is any character. Both alternatives feed the DB as text, and the DB converts with `+` (through d3's scale, section 6.4).

| Input                                                                 | Result                                                                                                                      |
| --------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| `[0.5, 0.5]`, `[0.50, 0.5]`, `[0.12345, 0.5]`, `[0.0, 0]`, `[0.5, 1]` | Accepted [V]                                                                                                                |
| `[1.0, 0.5]`, `[1.5, 0.5]`, `[-0.5, 0.5]`, `[.5, 0.5]`                | Lexical error [V]                                                                                                           |
| `[0x5, 0.5]`                                                          | Accepted, `x` = `Number("0x5")` = 5, so the point is at 2455 px, outside the chart [V]                                      |
| `[0,5, 0.5]`                                                          | Accepted, `x` = `Number("0,5")` = NaN. The reference writes `cx` as nothing and `transform="translate(undefined, 255)"` [V] |
| `[0.5, 0a5]`                                                          | Accepted, `y` = NaN, the same garbage on the other axis [V]                                                                 |

The two NaN rows are the reference producing garbage. They are candidates for the named exception of the M6 rules (section 8).

### 2.4 Quirks a port must reproduce

Each row was run through the reference. "Error" is a Jison parse error, a lexical error or a `parseStyles` error, which the reference reports as an exception.

| #   | Input                                                                                        | Reference behaviour                                                                                                                                               |
| --- | -------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Q1  | `section Foo` as a statement                                                                 | Error [V]. The `section` alternative of `statement` (135) names a symbol that is no token, so the rule can never match. (`addSection` is dead.)                   |
| Q2  | `QUADRANTCHART` or `QuadrantChart` as the header                                             | The grammar accepts it (the lexer is case-insensitive) [V]. The detector does not (`/^\s*quadrantChart/` is case-sensitive) [V], so the diagram is not recognised |
| Q3  | a second `quadrantChart` line                                                                | Error [V]                                                                                                                                                         |
| Q4  | `title: Hello`                                                                               | The title is `: Hello`; the colon is part of the value [V]                                                                                                        |
| Q5  | `title A`, then `title B`                                                                    | The last one wins [V]. Trailing blanks are trimmed (`title T   ` gives `T`) [V]                                                                                   |
| Q6  | `x-axis Low` (one text)                                                                      | Left text only, at the left of the x axis [V]                                                                                                                     |
| Q7  | `x-axis Low ---> High`, `x-axis Low --> High`                                                | Two texts; the delimiter is any `-->`-like run [V]                                                                                                                |
| Q8  | `x-axis Low -->` (delimiter, no right text)                                                  | The text becomes `Low ⟶` [V] (the `" ⟶ "` at line 147 is trimmed by the sanitiser)                                                                                |
| Q9  | `x-axis --> High`, `x-axis`, `x-axis a --> b --> c`, `y-axis --> Top`                        | Error [V] (each needs a left text first)                                                                                                                          |
| Q10 | `y-axis Low --> High`                                                                        | Low is the bottom text and High the top text (149). The probe `Bottom --> Top` puts `Bottom` at the bottom and `Top` at the top [V]                               |
| Q11 | `quadrant-1 A`, then `quadrant-1 B`                                                          | The last wins: `B` [V]                                                                                                                                            |
| Q12 | a point with no text, `: [0.5, 0.5]`                                                         | Error [V]                                                                                                                                                         |
| Q13 | `P1 : [0.5, 0.5]`, `P1:[0.5,0.5]`, `P1: [ 0.5 , 0.5 ]`, `P1: [0.5, 0.5   ]`                  | All accepted [V]                                                                                                                                                  |
| Q14 | `P1:::cls: [0.5, 0.5]` with no `classDef cls`                                                | Accepted; no styles [V]                                                                                                                                           |
| Q15 | `classDef hot color: #ff0000` defined after the point that uses it                           | The class still applies: the lookup happens at build time (builder 489-491) [V]                                                                                   |
| Q16 | a point's own `color` against its class                                                      | The point's own value wins [V]                                                                                                                                    |
| Q17 | the same `classDef` twice                                                                    | The second replaces the first (a `Map.set`) [V]                                                                                                                   |
| Q18 | `P1: [0.5, 0.5] radius: 10px`                                                                | Error: `value for radius 10px is invalid, please use a valid number` [V]                                                                                          |
| Q19 | `radius: -3`, `radius: 4.5`, `radius:` (no value)                                            | Error (the first two are a `parseStyles` error and a grammar error, the third is `value for radius  is invalid`) [V]                                              |
| Q20 | `radius: 0`                                                                                  | Accepted, radius 0 [V] (the `??` in the builder keeps it, 497)                                                                                                    |
| Q21 | `color: red`, `stroke-color: blue`                                                           | Error (hex only) [V]                                                                                                                                              |
| Q22 | `color: abcdef` (no `#`)                                                                     | Accepted and written as given: `fill="abcdef"` [V]                                                                                                                |
| Q23 | `color: #abc`                                                                                | Accepted [V]                                                                                                                                                      |
| Q24 | `stroke-width: 3` (no unit)                                                                  | Error [V] (`\d+px` only). A valid value is kept as written, `2px` stays `2px`                                                                                     |
| Q25 | `opacity: 1`                                                                                 | Error: `style named opacity is not supported.` [V]                                                                                                                |
| Q26 | `radius: 6` with the styles separated by a space and not a comma, `color: #ff0000 radius: 6` | Error: `#ff0000 radius` is not a hex code [V]                                                                                                                     |
| Q27 | a trailing comma after the styles                                                            | Error [V]                                                                                                                                                         |
| Q28 | `P1: [0.5, 0.5]` written with `%% trailing`                                                  | Accepted; the comment is dropped [V]                                                                                                                              |
| Q29 | `"`**Bold** label`": [0.5, 0.5]`                                                             | The text `**Bold** label`; the markdown type is dropped [V]                                                                                                       |
| Q30 | `Point & Q`                                                                                  | Accepted as `Point & Q` [V] (the sanitiser's fast path, section 3.2)                                                                                              |
| Q31 | `Point &amp; Q`, `P&amp;Q`                                                                   | Error [V] (`&`, `amp` and `;` are three tokens, and `;` is `SEMI`)                                                                                                |
| Q32 | `Point; a`, a `;` in a label                                                                 | Error [V]                                                                                                                                                         |
| Q33 | `P #35; Q`                                                                                   | Accepted; the DB holds `P ﬂ°°35¶ß Q`, the entity placeholder of `encodeEntities`. The SVG shows `P # Q` because `mermaidAPI.ts` 359 decodes the whole SVG [V]     |
| Q34 | `中文点`, `🚀 Rocket`                                                                        | Accepted [V]                                                                                                                                                      |
| Q35 | CRLF line endings                                                                            | Accepted [V]                                                                                                                                                      |
| Q36 | `quadrantChart` alone, or with blank lines                                                   | Accepted, with no points, no texts and no axes [V]                                                                                                                |
| Q37 | `classDef` alone                                                                             | Error [V]                                                                                                                                                         |
| Q38 | `[0.5, 0.5]` with a class name that contains a digit, `123: [0.5, 0.5]`                      | Accepted [V] (the text is `123`)                                                                                                                                  |

### 2.5 Errors

A parse error carries the Jison message (`Parse error on line N: …`, `Lexical error on line N. Unrecognized text. …`). A style error carries the `InvalidStyleError` text (`utils.ts` 2-5) or the `style named X is not supported.` text (`quadrantDb.ts` 89). The port needs the messages only where mermaid shows them; the differential compares failure, not text [U, it depends on the exporter's error policy].

## 3. DB

### 3.1 Calls and state

| DB call                                                                                          | Stores                                                                                      | Notes                                                                                                                                              |
| ------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `setQuadrant{1..4}Text(obj)` (32-46)                                                             | `quadrantBuilder.data.quadrant{n}Text`                                                      | `textSanitizer` (21-23)                                                                                                                            |
| `setXAxisLeftText`, `setXAxisRightText` (48-54), `setYAxisTopText`, `setYAxisBottomText` (56-62) | the four axis texts                                                                         | All sanitised                                                                                                                                      |
| `addPoint(obj, className, x, y, styles)` (95-106)                                                | `parseStyles(styles)`, then `quadrantBuilder.addPoints([{x, y, text, className, …styles}])` | `x` and `y` are the token texts from the grammar (139-142) [S]. d3 converts them with `+` (verified, section 6.4)                                  |
| `addClass(name, styles)` (108-110)                                                               | `quadrantBuilder.addClass`: a `Map` keyed by name, last definition wins                     |                                                                                                                                                    |
| `setWidth`, `setHeight` (112-118)                                                                | the builder's config                                                                        | Called by the renderer (`quadrantRenderer.ts` 57-59), not by the grammar                                                                           |
| `getQuadrantData()` (120-145)                                                                    | builds the layout (section 6)                                                               | Takes the config from `getConfig().quadrantChart` (123-125) and the theme from `themeVariables` (126-142), then the title from the common DB (143) |
| `clear()` (147-150)                                                                              | the builder (its config, theme, data and classes) and the common DB                         |                                                                                                                                                    |

### 3.2 Sanitising

`textSanitizer(text) = sanitizeText(text.trim(), getConfig())` (DB 21-23) runs on the quadrant texts, the axis texts and the point texts. The title and the accessibility texts go through the common DB's `sanitizeText` (`commonDb.ts` 28-30 and 16-18). Class names, style keys and style values are raw.

`sanitizeText` calls DOMPurify on every text (`common.ts` 71-83). DOMPurify returns a string with no `<` unchanged (its fast path, `purify.es.mjs` 2250, DOMPurify 3.4.12), so `Point & Q` and `A & B` stay as they are [V]. A text with `<` is parsed and serialised, so `&` becomes `&amp;` [V] (the ishikawa case in its notes). The lexer rejects `<` in quadrant texts anyway, so the fast path is the only case that matters here.

### 3.3 Defaults and clear

- The builder keeps its state in a module-level singleton (`quadrantDb.ts` 30). A parse without a `clear` keeps the previous points and texts.
- `clear()` resets the builder's config, theme, data and classes to the builder's defaults (builder 201-207), and the common DB (accTitle, accDescr, title; `commonDb.ts` 10-14).
- `getQuadrantData()` passes the configuration to `setConfig` and the theme to `setThemeConfig` (DB 123-142). The schema defaults make the configuration present, so the builder's own defaults (section 6.5) are only used before that call.
- **Point order.** `addPoints` prepends: `this.data.points = [...points, ...this.data.points]` (builder 214). The DB therefore lists the points in reverse source order [V]: `A`, `B`, `C` in the source come out as `C`, `B`, `A` (the probe `point_order`). The drawing follows that order (section 6.2), so the port must keep it.
- The `classes` map keeps insertion order, but nothing reads it in order.

### 3.4 Config reads

`getQuadrantData` reads `themeVariables` and `quadrantChart` at call time (DB 121-142), so the same parse with a different configuration gives a different layout. The DB spec's last test (`quadrantDb.spec.ts` 54-69) checks that the sanitiser reads the config when it runs, not at module load.

## 4. Canonical model proposal

The model is the DB state after parsing, in the conventions of `docs-port/canonical-model.md` (camelCase, `version: 1`, strings present, numbers as JSON numbers, objects compared without order).

```
{
  "version": 1,
  "title": "",            // common DB diagramTitle, sanitised
  "accTitle": "",         // common DB, sanitised, trimmed
  "accDescr": "",         // common DB, sanitised, trimmed
  "quadrant1Text": "", "quadrant2Text": "", "quadrant3Text": "", "quadrant4Text": "",
  "xAxisLeftText": "", "xAxisRightText": "", "yAxisBottomText": "", "yAxisTopText": "",
  "points": [             // DB order: the last source line first
    { "text": "", "className": "", "x": 0.5, "y": 0.5,
      "radius": 5,        // only when the point or its class sets one
      "color": "#…",      // ditto
      "strokeColor": "#…", "strokeWidth": "3px" }
  ],
  "classes": [ [ "hot", { "color": "#ff0000", "radius": 8 } ] ]   // Map order: first definition
}
```

Notes on the fields:

- `x` and `y` are `Number(token text)`, which is the value d3 uses. `0x5` is 5 and `0,5` is NaN; the canonical convention writes a non-finite number as the sentinel string `"NaN"`, so the NaN point is `"NaN"` there.
- The style fields are optional and absent when unset. The DB spreads only the keys that were set (`parseStyles`, 64-93), so `undefined` must not appear: the model needs "absent" as distinct from "set".
- `radius` is a number (`parseInt`), `color` and `strokeColor` are the text as given (`abcdef` stays `abcdef`), and `strokeWidth` is the text (`3px`).
- `className` is `""` for an unnamed point (the grammar's default).
- The entity placeholders (`ﬂ°°…¶ß`) stay as the DB holds them, unless the coordinator decides on the decoded text (section 8).

Left out, and why: the builder's layout (positions, rectangles, the axis label geometry). That is the renderer's output and is covered by the geometry harness, and by the layout differential proposed in section 8. The configuration is compared by `pnpm port:diff config`, and the theme variables by `pnpm port:diff theme`.

## 5. Specs

| File                            | Tests                                                                                          | What they check                                                                                                                                                                                                                                                                                                                                                                                               |
| ------------------------------- | ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `parser/quadrant.jison.spec.ts` | 24 (14 top level at 38-428, 10 in the nested `Unicode support (CJK + Emoji)` block at 437-590) | The grammar with a mock DB (lines 13-24). Header errors (38, 43), axes (48, 86), the four quadrant texts (130-181), title (197), points (213), whole charts with and without styles (251, 310, 369), a `constructor` class name (428), CJK, Japanese, Korean, emoji, Latin-1 and accented text (438-590). It asserts the calls the grammar makes (145 `expect` calls), so it tests the grammar-to-DB protocol |
| `quadrantDb.spec.ts`            | 5: four `parseStyles` tests (6, 18, 25, 31) and one runtime config test (60)                   | The style parser, its errors, and the sanitiser reading the config at call time                                                                                                                                                                                                                                                                                                                               |

Neither file tests the layout or the SVG. The 24 grammar tests port as parser tests on the protocol, and the 5 DB tests port as-is. The layout needs new tests (the differential and the geometry harness).

## 6. Renderer

### 6.1 Pipeline and DOM order

`draw(txt, id, version, diagObj)` (`quadrantRenderer.ts` 17-171) does this, in order:

1. Selects the root `svg` by id (45) and appends `g.main` (47).
2. Sets the size with `configureSvgSize(svg, height, width, useMaxWidth)` (49-52), where width and height are `chartWidth` and `chartHeight` (default 500) and `useMaxWidth` defaults to true (`?? true`, 52).
3. Sets `viewBox="0 0 W H"` (54).
4. Calls `db.setHeight(height)` and `db.setWidth(width)` (57-59), then `db.getQuadrantData()` (62).
5. Appends five groups to `g.main`, in this order: `g.quadrants` (64), `g.border` (65), `g.data-points` (66), `g.labels` (67), `g.title` (68). The groups are created even when empty [V] (the title group is empty for a chart without title).

The root, as the reference writes it for `should-render-a-complete-quadrant-chart` [V]:

```
<svg id="…" width="100%" xmlns="http://www.w3.org/2000/svg" style="max-width: 500px;"
     viewBox="0 0 500 500" role="graphics-document document" aria-roledescription="quadrantChart">
  <style>…</style>
  <g></g>                        (mermaidAPI.ts 419: the empty group of every diagram)
  <g class="main">
    <g class="quadrants">        4 × <g class="quadrant"> (rect, text)
    <g class="border">           6 × <line>
    <g class="data-points">      n × <g class="data-point"> (circle, text)
    <g class="labels">           n × <g class="label"> (text)
    <g class="title">            <text> or nothing
```

`aria-roledescription` comes from `accessibility.ts` (22-30), and the `role` and the `<style>` from the same wrapper [U, not traced]. Without `useMaxWidth` the root has `width` and `height` in pixels and no `style` (`setupGraphViewbox.js` 25-31) [S]. The ishikawa fixture with `useMaxWidth: false` shows that root shape [V].

### 6.2 Elements, in DOM order

**Quadrants** (`quadrants`, builder 384-473). The four rectangles come in this order: quadrant 1 (top right), quadrant 2 (top left), quadrant 3 (bottom left), quadrant 4 (bottom right) [V].

- `rect`: `x`, `y`, `width`, `height`, `fill` (renderer 104-110).
- `text`: `x="0" y="0"`, `fill`, `font-size`, `dominant-baseline`, `text-anchor="middle"`, `transform="translate(X, Y) rotate(0)"` (112-123).
  - With no points the label is centred in its box (builder 462-464): `y = y + height/2`, `dominant-baseline="middle"`.
  - With points it sits `quadrantTextTopPadding` below the top edge (466-468): `dominant-baseline="hanging"`.
  - The `text` centre is `x + width/2` (460).

**Border** (`border`, builder 516-586; renderer 83-95). Six `line` elements: the external top, right, bottom and left edges (stroke width `quadrantExternalBorderStrokeWidth`), then the internal vertical and horizontal lines (`quadrantInternalBorderStrokeWidth`). Each line has `x1`, `y1`, `x2`, `y2` as attributes and `style="stroke: …; stroke-width: …;"`. The style is set through d3's `.style`, so the browser writes the colour in its own form: `#181818` appears as `stroke: rgb(24, 24, 24);` [V]. The port must write the same form (section 8).

**Data points** (`data-points`, builder 475-514; renderer 143-170), in the DB order (section 3.3):

- `g.data-point` containing:
  - `circle`: `cx`, `cy`, `r` (the radius), `fill`, `stroke` (the stroke colour, which defaults to the point fill), `stroke-width` (an attribute, `0px` by default) (151-157);
  - `text`: `x="0" y="0"`, `fill` (the point text colour), `font-size` (`pointLabelFontSize`, 12), `dominant-baseline="hanging"`, `text-anchor="middle"`, `transform="translate(x, y + pointTextPadding) rotate(0)"` (159-170).

**Labels** (`labels`, builder 286-382; renderer 125-141). One `g.label` per axis text with a `text`: `x="0" y="0"`, `fill`, `font-size`, `dominant-baseline="hanging"` (every axis label has `horizontalPos: 'top'`), `text-anchor` (`start` for a `left` vertical position, `middle` for `center`), and `transform="translate(X, Y) rotate(R)"` with R = -90 for the y axis.

**Title** (`title`, builder 588-602; renderer 70-81). `text` at `translate(chartWidth/2, titlePadding)`, `font-size = titleFontSize`, `dominant-baseline="hanging"`, `text-anchor="middle"`. The title group is appended last and only when there is a title.

### 6.3 Layout arithmetic

All of this is plain arithmetic on the config values. The formulas are the builder's (line numbers in the table). Let `P` = `quadrantPadding`, `T` = `titleSpace.top` (`titleFontSize + 2·titlePadding` when shown, else 0), `Xt`/`Xb` = the x-axis space at the top or bottom (`2·xAxisLabelPadding + xAxisLabelFontSize` when shown there, else 0), `Yl`/`Yr` = the y-axis space at the left or right (`2·yAxisLabelPadding + yAxisLabelFontSize`, else 0).

| Value                   | Formula                          | Builder |
| ----------------------- | -------------------------------- | ------- |
| `quadrantLeft`          | `P + Yl`                         | 256     |
| `quadrantTop`           | `P + Xt + T`                     | 257     |
| `quadrantWidth`         | `chartWidth − 2P − Yl − Yr`      | 258-259 |
| `quadrantHeight`        | `chartHeight − 2P − Xt − Xb − T` | 260-265 |
| half width, half height | `/ 2`                            | 267-268 |

Showing: an axis is shown when its config flag is true (the builder's defaults, 155-157, which the schema does not expose) and it has at least one text (605-609). The title is shown when it has text (609). The x axis is at the **bottom** whenever there are points, whatever `xAxisPosition` says (611) [V] (the complete chart has its x labels at y 479, with points).

Quadrants (384-473): quadrant 1 at `(left + hw, top)`, quadrant 2 at `(left, top)`, quadrant 3 at `(left, top + hh)`, quadrant 4 at `(left + hw, top + hh)`, each `hw × hh`.

Points (475-514): the d3 scales from section 6.4. Point text at `y + pointTextPadding`.

Borders (516-586): the external lines are inset by half the external stroke width at the corners, `halfExternal = quadrantExternalBorderStrokeWidth / 2`. The internal lines run from the inner edges (`quadrantLeft + halfExternal` to `quadrantLeft + quadrantWidth − halfExternal`, and the same vertically).

Axis labels (286-382):

| Label         | x                                                                               | y                                                                                                                         | Anchor                                         | Rotation |
| ------------- | ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------- | -------- |
| x left text   | `left + (middle ? hw/2 : 0)`                                                    | top: `xAxisLabelPadding + titleSpace.top` (314); bottom: `xAxisLabelPadding + quadrantTop + quadrantHeight + P` (333-336) | `center` when a right text exists, else `left` | 0        |
| x right text  | `left + hw + (middle ? hw/2 : 0)`                                               | as the left text                                                                                                          | as the left text                               | 0        |
| y bottom text | `yAxisPosition left ? yAxisLabelPadding : yAxisLabelPadding + left + width + P` | `top + height − (middle ? hh/2 : 0)`                                                                                      | `center` when a top text exists                | −90      |
| y top text    | as the bottom text                                                              | `top + hh − (middle ? hh/2 : 0)`                                                                                          | as the bottom text                             | −90      |

The "middle" flags are `Boolean(xAxisRightText)` (302) and `Boolean(yAxisTopText)` (303).

Title: `x = chartWidth / 2`, `y = titlePadding` (597-598).

### 6.4 The d3 scale

`getQuadrantPoints` (builder 480-486) uses d3's `scaleLinear` with domain `[0, 1]`:

- x: range `[quadrantLeft, quadrantWidth + quadrantLeft]`;
- y: range `[quadrantHeight + quadrantTop, quadrantTop]`.

The output of a linear scale over `[0, 1]` into `[r0, r1]` is `r0·(1−t) + r1·t`, the form d3's `interpolateNumber` uses, not `r0 + t·(r1−r0)`. Checked against d3-scale 4.0.2 on 1,011 values of `t` (every thousandth, plus the test values):

| Candidate                         | Values that differ from d3       |
| --------------------------------- | -------------------------------- |
| `r0·(1−t) + r1·t`                 | 0 of 1,011, for both axes [V]    |
| `r0 + t·W` (x), `(H+T) − t·H` (y) | 282 (x) and 435 (y) of 1,011 [V] |

So the port must use the interpolation form. The results differ in the last bits, which the geometry harness (0.05 px) cannot see, but the differential on the layout data can.

d3 also converts the input with `+`: `xs("0.57") === xs(0.57)` [V]. A text that `Number` reads as NaN gives `undefined` from the scale (d3's `unknown`), which is the garbage of section 2.3.

### 6.5 Configuration and defaults

The schema defaults (`QuadrantChartConfig`, `config.schema.yaml` 1287-1407), confirmed by `getConfig().quadrantChart` [V]:

| Key                                        | Default | Key                                 | Default |
| ------------------------------------------ | ------- | ----------------------------------- | ------- |
| `chartWidth`                               | 500     | `pointTextPadding`                  | 5       |
| `chartHeight`                              | 500     | `pointLabelFontSize`                | 12      |
| `titleFontSize`                            | 20      | `pointRadius`                       | 5       |
| `titlePadding`                             | 10      | `xAxisPosition`                     | `top`   |
| `quadrantPadding`                          | 5       | `yAxisPosition`                     | `left`  |
| `xAxisLabelPadding`, `yAxisLabelPadding`   | 5       | `quadrantInternalBorderStrokeWidth` | 1       |
| `xAxisLabelFontSize`, `yAxisLabelFontSize` | 16      | `quadrantExternalBorderStrokeWidth` | 2       |
| `quadrantLabelFontSize`                    | 16      | `useMaxWidth` (base config, 470)    | true    |
| `quadrantTextTopPadding`                   | 5       |                                     |         |

The builder's own defaults (`quadrantBuilder.ts` 153-179) swap `chartWidth` and `chartHeight` (158-159: each reads the other key's default). Both are 500, and `getQuadrantData` always replaces them, so the swap has no effect [S]. The builder's `||` fallbacks also turn a 0 into the default (`quadrantPadding: 0` is read as 5 by the builder's defaults only), which again only applies before `getQuadrantData` [S].

The configuration keys the renderer reads are `chartWidth`, `chartHeight` and `useMaxWidth` (renderer 49-52), and the DB reads the other keys through `getQuadrantData`.

### 6.6 Theme variables

Read by `getQuadrantData` (DB 126-142) and the builder defaults (181-199): `quadrant1Fill` … `quadrant4Fill`, `quadrant1TextFill` … `quadrant4TextFill`, `quadrantPointFill`, `quadrantPointTextFill`, `quadrantXAxisTextFill`, `quadrantYAxisTextFill`, `quadrantExternalBorderStrokeFill`, `quadrantInternalBorderStrokeFill`, `quadrantTitleFill`. The redux-color values are at `theme-redux-color.js` 338-355 (the fills come from `primaryColor`, `#ECECFE` at line 170, and the adjusted greys). The fixture with theme overrides (`should-use-all-the-theme-variable`) shows that every one of these reaches the SVG [V].

**The NaN fill.** With no override, the point fill is `isDark(q1) ? lighten(q1) : darken(q1)` (`theme-redux-color.js` 350-352), where `q1` is `#ECECFE`. khroma 2.1.0 returns `hsl(240, 90%, NaN%)` for both `darken('#ECECFE')` and `lighten('#ECECFE')` [V], so the reference writes `fill="hsl(240, 90%, NaN%)"` and `stroke` of the same value on every point with the default colour [V]. The condition is written `this.quadrantPointFill || isDark(q1) ? lighten(q1) : darken(q1)`, and JavaScript binds `||` tighter than `?:`. Without a user value the result is `isDark(q1) ? lighten(q1) : darken(q1)`, as intended. With a user value the expression gives `lighten(q1)`, which the user's value then replaces: the fixture `should-use-all-the-theme-variable` shows `fill="#0149ff"` on all six points [V]. The order in which the override is applied is [U]. The NaN belongs to the theme port; the quadrant port copies whatever the theme gives.

### 6.7 Text

No text is measured in this folder. The four `getBBox` sites of the ishikawa renderer do not exist here, and there is no `calculateTextDimensions` call [V, by search of `quadrant-chart/`]. Every box is the config's arithmetic, so a long label is not fitted to its quadrant [S, not seen in a render]. The text is drawn with the font the stylesheet gives (`font-family` from the theme, the configured font size as the `font-size` attribute), so Chromium's layout does not enter the geometry.

### 6.8 Randomness, clock and the DOM

- No randomness and no clock [S, no `Math.random`, no `Date`].
- The sandbox branch (renderer 34-43) selects the iframe's `contentDocument`. A server renderer has no sandbox; the port drops the branch.
- d3 calls to replace: `select('body')` and `select('[id=…]')` (renderer 40-45), `append`, `attr`, `style`, `selectAll(…).data(…).enter()`. A server renderer writes the tree directly. The one serialisation the port must match is the browser's `style` form of the border lines (section 6.2).

## 7. Fixtures

14 fixtures: 10 in `pr-check/`, 4 in `nightly-check/`. None uses `look: handDrawn`, none contains a character outside the bundled faces in its diagram text (the front matter has none either), and none needs randomness. The generic harness (`pnpm port:geometry quadrant-chart`) needs no exclusions. The features are:

| Fixture                                                                                                | What it exercises                                                          |
| ------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------- |
| `pr-check/should-render-a-complete-quadrant-chart`                                                     | Baseline: title, both axes with two texts, four quadrant texts, six points |
| `pr-check/should-render-without-points`                                                                | No points: quadrant texts centred, x axis at its config position           |
| `pr-check/should-render-x-axis-labels-in-the-center-if-x-axis-has-two-labels`                          | Centred x labels (`hw/2` offsets)                                          |
| `pr-check/should-render-y-axis-labels-in-the-center-if-y-axis-has-two-labels`                          | Centred y labels                                                           |
| `pr-check/should-able-to-render-y-axis-on-right-side`                                                  | `yAxisPosition: right`                                                     |
| `pr-check/should-able-to-render-x-axis-on-bottom-and-y-axis-on-right`                                  | Both config positions                                                      |
| `pr-check/it-should-render-data-points-with-styles`                                                    | Inline point styles                                                        |
| `pr-check/it-should-render-data-points-with-styles-classes`                                            | `classDef` and `:::` class names                                           |
| `pr-check/should-use-all-the-config`                                                                   | Every config key the diagram reads                                         |
| `pr-check/should-use-all-the-theme-variable`                                                           | Every quadrant theme variable                                              |
| `nightly-check/should-able-to-render-x-axis-on-bottom`                                                 | `xAxisPosition: bottom` without points                                     |
| `nightly-check/should-render-both-axes-labels-on-the-left-and-bottom-if-both-axes-have-only-one-label` | One label per axis: `left` anchors                                         |
| `nightly-check/should-render-if-only-chart-type-is-provided`                                           | The bare header                                                            |
| `nightly-check/should-render-without-title`                                                            | No title                                                                   |

Fixtures without a theme get the harness's forced `redux-color` (`common.ts` 77-88). The generic harness's exclusions (`common.ts` 172-225: handDrawn, non-layered layouts, uncovered script) match none of these fixtures [S].

## 8. Risks, effort, open questions

### 8.1 Risks

1. **Float bits of the scale** (section 6.4). The harness tolerance (0.05 px) hides the difference, so the layout differential must compare the exact values.
2. **Garbage values** (section 2.3). `0x5` is read as 5, and a NaN coordinate drops `cx` and writes `translate(undefined, …)`. The port must reproduce both or name the NaN case as an exception.
3. **Jison's first-match lexer and its start-state stack.** `begin` pushes and `popState` pops, and the point states rely on that (51-55). A hand-written lexer must keep the order and the stack. Rule order decides `quadrant-2`, `title`, `:::` and the one-character tokens.
4. **Number text.** `translate(…)` and the other attributes are JavaScript numbers in text. Shortest round-trip digits match Rust's `Display` for ordinary values, but JavaScript writes exponents below 1e-6 and from 1e21, and `NaN`/`undefined` as words.
5. **The khroma NaN colour** (section 6.6). It must not be "fixed" in the quadrant port, and the theme port has to reproduce it or name it.
6. **Entities.** The placeholder of `encodeEntities` reaches the DB and is decoded only in the whole-SVG pass (`mermaidAPI.ts` 359). The port's wrapper must do that pass for this type too.
7. **The border `style` form** (section 6.2): the browser's `rgb(…)` serialisation of hex colours.

### 8.2 Effort

About 1,100 lines of Rust: the lexer and the grammar (about 450), the DB and style parser (about 200), the builder (about 350, the types being most of the rest), the renderer (about 180), and the canonical exporter. Plus about 300 lines for the 29 tests. No layout engine, no text measurement and no randomness are needed.

### 8.3 Open questions for the coordinator

1. **Entities in the model.** The DB holds `ﬂ°°35¶ß`, and the SVG shows `#`. Should the model keep the placeholder (what the DB holds) or the decoded text (as the journey model does)? The decoded form is what the reader sees, but it is not the DB state.
2. **The NaN coordinates and the NaN fill.** Name both as exceptions in `PLAN.md`, as the M6 rule allows for NaN, and write the NaN coordinate as `"NaN"` in the model. The `0x5` input is a valid reading (5), so it is reproduced and is not an exception.
3. **The layout differential.** Add a `quadrant-data` differential over `getQuadrantData()` (points, rectangles, axis labels, borders, title) with exact floats? The geometry harness cannot see the last-bit differences of section 6.4. The reference is pure arithmetic, so this is cheap and it is the right test.
4. **The khroma NaN.** Does the theme port reproduce `hsl(240, 90%, NaN%)` for the redux themes, or do we treat the point colour as an exception? The quadrant port follows whichever is chosen.
5. **The border colour form.** Should the port write `rgb(24, 24, 24)` for `#181818` (the browser's form), or the hex? The reference's output is the `rgb` form, and the geometry harness does not compare styles.
6. **The unescaped `.`** (section 2.3). The port reproduces the reference's acceptance: `0x5` is 5 (valid JavaScript number text) and `0,5` is NaN. Confirm that the port should keep both, rather than reject the second.
7. **The configuration differential.** `pnpm port:diff config` covers pie, flowchart, timeline, journey, gitGraph, ER, class, sequence, state and gantt (`tools/port/README.md`), not quadrantChart. Section 6.5's defaults are read from the schema and confirmed by `getConfig()` [V]. Should `quadrantChart` be added to that differential?
