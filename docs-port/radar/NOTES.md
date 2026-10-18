# Radar (P3): research notes for the port

Status: research only. Nothing in the repository changed except this file, and nothing is committed. Reference: `packages/mermaid/src/diagrams/radar/`, the grammar `packages/parser/src/language/radar/` and its shared `common/` files, at HEAD `f3a1a94b0` (branch `sebastian/port-p3`). Plan: [../P3-PLAN.md](../P3-PLAN.md). Guide: [../P2-RESEARCH.md](../P2-RESEARCH.md). The model for the sections is [../kanban/NOTES.md](../kanban/NOTES.md); the lexer section follows [../architecture/PLAN.md](../architecture/PLAN.md) section 2.

Every rule cites the reference as `diagrams/radar/<file>:<line>` unless the file is named in full. **Probed** means the rule was run through the reference's own code: the parser source with `tsx` (Langium 4.4.0 through `packages/parser/src`), the DB and the parser through `tools/port/reference.ts` (Vite SSR of the TypeScript sources, with `preprocessDiagram` and `encodeEntities` applied as `Diagram.fromText` applies them), and the rendering through the bundle `packages/mermaid/dist` in Chromium, through the geometry harness's own page (`tools/port/geometry/common.ts`: `withPage`, `renderReference`, `patchText`, `measureSvg`), so the forced settings are the harness's. **Unverified** means read or inferred only; where a value was computed in Node with V8, the row says so. Probe rows are named `Q1` to `Q25` (the render matrix) and `D1` to `D38` (the DB probe); open questions are `O1` to `O11` (section 8.3). Probe material is outside the repository in `/private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/radar/`: `lexer-dump.mts` (token order), `parse-probe.mts` and `parse-probe2.mts` (acceptance, with `parse-out.txt`), `db-probe.mts` (with `db-out.txt`), `render-probe.mts` (the matrix, with `render-out.json`, and the readers `outline.py`, `brief.py`, `styles.py`), `computed-probe.mts`, `title-probe.mts`, and `theme-probe.mts` and `theme-probe2.mts`.

## 0. Summary

Radar is a Langium grammar (ported by hand, decision 0002) with a DB of plain arrays and an options record, both held as module state, and a renderer made of arithmetic: graticule circles or polygons, one line and one label per axis, one closed curve per series, a legend and a title. The renderer does not measure text, does not lay anything out, and reads no randomness and no clock. The reference's visible output is therefore decided by its arithmetic (which is simple) and by a set of quirks in the lexer, the DB, the theme fallback and the CSS. Most of the port's work is reproducing those quirks. Most have a probe below, and the rest are marked as derived.

| Item                | Value                                                                                                                                                                                                                                                                                                                                                                                 |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Reference, non-spec | `db.ts` (134), `detector.ts` (22), `diagram.ts` (12), `parser.ts` (23), `renderer.ts` (246), `styles.ts` (75), `types.ts` (47): 559 lines, as the P3 plan counts. Grammar `radar.langium` (55), `module.ts` (76), `tokenBuilder.ts` (7), `index.ts` (1)                                                                                                                               |
| Shared code it uses | `common/common.langium` (34), `common/tokenBuilder.ts` (31), `common/valueConverter.ts` (81), `common/matcher.ts` (14), `diagrams/common/populateCommonDb.ts`, `diagrams/common/commonDb.ts`, `diagrams/common/common.ts` (`sanitizeText`), `setupGraphViewbox.js` (`configureSvgSize`), `selectSvgElement.ts`, `preprocess.ts`, `Diagram.ts`                                         |
| Specs               | `radar.spec.ts`: 271 lines, 20 tests (9 parse and DB, 6 `relativeRadius`, 3 `closedRoundCurve`, 2 `draw` that assert nothing)                                                                                                                                                                                                                                                         |
| Fixtures            | 6: 5 in `e2e/diagrams/radar/pr-check/`, 1 in `nightly-check/`. `visual-check/` holds only `.gitkeep`. None is excluded by the harness                                                                                                                                                                                                                                                 |
| Parser              | Langium 4.4.0. The lexer has 28 token types (15 keywords and 13 terminals, section 2.2). The grammar is small, but the lexer's first-match order and the keyword and terminal prefixes produce a long list of rejections (section 2.5)                                                                                                                                                |
| DB                  | Module state: `axes`, `curves`, `options`, plus the shared header texts. Four getters, three setters, `clear`, and `getConfig` (section 3)                                                                                                                                                                                                                                            |
| Renderer            | Arithmetic and d3 selection only: `select`, `append`, `attr`, `text` (section 6.3). The root `svg` is sized by `configureSvgSize`. No `getBBox` and no computed style are read                                                                                                                                                                                                        |
| Riskiest parts      | (1) The lexer's prefix and keyword quirks, which decide acceptance. (2) The theme fallback: every theme without a `radar` section takes the default theme's values, and the CSS semantics around them. (3) Numbers written into attributes (JavaScript's number text). (4) The harness's `getBBox` of the labels, which depends on the text engine (decision 0005), not on radar code |
| Effort              | About 900 to 1,250 lines of Rust for the lexer, parser, DB, renderer, styles and theme views, plus tests (section 8.2)                                                                                                                                                                                                                                                                |

No exporter exists yet (`tools/port/export-radar.ts` is absent), and `tools/port/reference.ts` `registerDetectors` does not register the radar detector (section 5.3).

## 1. Sources

| File                                                                           | Lines                                                                   | What it holds                                                                                                                                                                                                                                                                                        |
| ------------------------------------------------------------------------------ | ----------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `packages/parser/src/language/radar/radar.langium`                             | 2                                                                       | `import "../common/common"`                                                                                                                                                                                                                                                                          |
|                                                                                | 4-15                                                                    | `Radar` entry: the header (6), the statement loop (8-14): title and accessibility, `axis`, `curve`, options, NEWLINE                                                                                                                                                                                 |
|                                                                                | 17-19                                                                   | `Label`: `'[' label=STRING ']'`                                                                                                                                                                                                                                                                      |
|                                                                                | 21-23                                                                   | `Axis`: `name=ID (Label)?`                                                                                                                                                                                                                                                                           |
|                                                                                | 25-27                                                                   | `Curve`: `name=ID (Label)? '{' Entries '}'`                                                                                                                                                                                                                                                          |
|                                                                                | 29-32                                                                   | `Entries`: all `NumberEntry` or all `DetailedEntry`, with `NEWLINE*` around the commas                                                                                                                                                                                                               |
|                                                                                | 34-43                                                                   | `Entry` interface (`axis?: @Axis`, `value: number`), `DetailedEntry` (39: `axis=[Axis:ID] ':'? value=NUMBER`), `NumberEntry` (42)                                                                                                                                                                    |
|                                                                                | 45-53                                                                   | `Option`: `showLegend` BOOLEAN, `ticks`, `max`, `min` NUMBER, `graticule` GRATICULE                                                                                                                                                                                                                  |
|                                                                                | 55                                                                      | `terminal GRATICULE returns string: 'circle' \| 'polygon'`                                                                                                                                                                                                                                           |
| `packages/parser/src/language/radar/module.ts`                                 | 40-45                                                                   | `RadarModule`: `RadarTokenBuilder`, `CommonValueConverter`                                                                                                                                                                                                                                           |
| `packages/parser/src/language/radar/tokenBuilder.ts`                           | 3-6                                                                     | `RadarTokenBuilder` with the keyword set `['radar-beta']` (5)                                                                                                                                                                                                                                        |
| `packages/parser/src/language/common/common.langium`                           | 6-34                                                                    | `EOL` (6), `TitleAndAccessibilities` (8-10), the terminals `BOOLEAN` (12), `ACC_DESCR` (14), `ACC_TITLE` (15), `TITLE` (16), `FLOAT` (18), `INT` (19), `NUMBER` (20), `STRING` (22), `ID` (27), `NEWLINE` (29), `WHITESPACE` (31, hidden), `YAML` (32), `DIRECTIVE` (33), `SINGLE_LINE_COMMENT` (34) |
| `packages/parser/src/language/common/tokenBuilder.ts`                          | 14-28                                                                   | `AbstractMermaidTokenBuilder.buildKeywordTokens`: the word-end suffix `(?:(?=%%)\|(?!\S))` on the keywords of the set (24)                                                                                                                                                                           |
| `packages/parser/src/language/common/valueConverter.ts`                        | 27-42, 44-70, 73-80                                                     | `runConverter`: the Mermaid rules first (`TITLE`, `ACC_TITLE`, `ACC_DESCR`), then Langium's default; `CommonValueConverter` returns no custom value                                                                                                                                                  |
| `packages/parser/src/language/common/matcher.ts`                               | 4, 9, 14                                                                | `accessibilityDescrRegex`, `accessibilityTitleRegex`, `titleRegex`                                                                                                                                                                                                                                   |
| `packages/parser/node_modules/langium/lib/parser/token-builder.js`             | 19-27, 79-93, 104                                                       | `buildTokens` (keywords, then terminals in order, 26), `buildKeywordTokens` sorted by descending length (83), `LONGER_ALT` (92), `findLongerAlt` (104)                                                                                                                                               |
| `packages/parser/node_modules/langium/lib/parser/value-converter.js`           | 24-37, 41, 89, 93                                                       | `runConverter` (`STRING`, then the rule's type `number` and `boolean`), `convertString`, `convertNumber`, `convertBoolean`                                                                                                                                                                           |
| `packages/parser/src/parse.ts`                                                 | 124 (the radar overload), 133-151 (`parse`), 156- (`MermaidParseError`) | `parse('radar', text)`; any lexer or parser error throws `MermaidParseError` (Langium's messages, which the port does not copy)                                                                                                                                                                      |
| `diagrams/radar/detector.ts`                                                   | 7, 9-11                                                                 | `id = 'radar'`; `/^\s*radar-beta/` (10): no word boundary, so `radar-betaX` is detected and then fails                                                                                                                                                                                               |
| `diagrams/radar/diagram.ts`                                                    | 7-12                                                                    | `diagram` = `{ parser, db, renderer, styles }`                                                                                                                                                                                                                                                       |
| `diagrams/radar/parser.ts`                                                     | 8-15, 17-23                                                             | `populate`: `populateCommonDb` (9), then `setAxes` (12), `setCurves` (13), `setOptions` (14); `parse` calls the Langium parser (19)                                                                                                                                                                  |
| `diagrams/radar/db.ts`                                                         | 18-24                                                                   | `defaultOptions`: `showLegend` true, `ticks` 5, `max` null, `min` 0, `graticule` `'circle'`                                                                                                                                                                                                          |
|                                                                                | 26-27                                                                   | `MAX_TICKS = 32`                                                                                                                                                                                                                                                                                     |
|                                                                                | 29-37                                                                   | `defaultRadarData`, the module `data`, `DEFAULT_RADAR_CONFIG = DEFAULT_CONFIG.radar` (37)                                                                                                                                                                                                            |
|                                                                                | 39-45                                                                   | `getConfig`: `cleanAndMerge({ ...DEFAULT_RADAR_CONFIG, ...commonGetConfig().radar })`                                                                                                                                                                                                                |
|                                                                                | 47-49                                                                   | `getAxes`, `getCurves`, `getOptions`                                                                                                                                                                                                                                                                 |
|                                                                                | 51-58                                                                   | `setAxes`: `label = axis.label ?? axis.name`                                                                                                                                                                                                                                                         |
|                                                                                | 60-68                                                                   | `setCurves`: `label = curve.label ?? curve.name`, entries from `computeCurveEntries`                                                                                                                                                                                                                 |
|                                                                                | 70-86                                                                   | `computeCurveEntries`: the first entry decides the form (72), references use `$refText` (80), the missing-entry error (82)                                                                                                                                                                           |
|                                                                                | 88-112                                                                  | `setOptions`: the last option of each name wins (90-96), defaults with `??` (98-104), the clamp to `MAX_TICKS` with `log.warn` (106-111)                                                                                                                                                             |
|                                                                                | 114-117                                                                 | `clear`: the shared header setters are cleared (`commonClear`), and the data is reset with `structuredClone`                                                                                                                                                                                         |
|                                                                                | 119-134                                                                 | the `db` object                                                                                                                                                                                                                                                                                      |
| `diagrams/radar/renderer.ts`                                                   | 8-46                                                                    | `draw`: the frame (19), the maximum and minimum (23-25), the radius (26), graticule (29), axes (32), curves (35), legend (38), title (41-45)                                                                                                                                                         |
|                                                                                | 50-62                                                                   | `drawFrame`: the total size (51-52), the centre (53-56), `configureSvgSize` (57), `viewBox` and `overflow` (59), the translated `g` (61)                                                                                                                                                             |
|                                                                                | 64-93                                                                   | `drawGraticule`: circles (71-76) and polygons (77-92)                                                                                                                                                                                                                                                |
|                                                                                | 95-133                                                                  | `drawAxes`: angle (105), line (109-114), label anchor and baseline (118-119), label (125-131)                                                                                                                                                                                                        |
|                                                                                | 135-173                                                                 | `drawCurves`: skip of a curve with the wrong entry count (148-151), points (153-159), `path` (161-165) or `polygon` (166-171)                                                                                                                                                                        |
|                                                                                | 175-183                                                                 | `relativeRadius` (exported)                                                                                                                                                                                                                                                                          |
|                                                                                | 185-207                                                                 | `closedRoundCurve` (exported): Catmull-Rom control points, `Z` (206)                                                                                                                                                                                                                                 |
|                                                                                | 209-244                                                                 | `drawLegend`: position (220-221), spacing (222), one `g` per curve (225-227), box (231-234), text (237-242)                                                                                                                                                                                          |
|                                                                                | 246                                                                     | `export const renderer`                                                                                                                                                                                                                                                                              |
| `diagrams/radar/styles.ts`                                                     | 7-31                                                                    | `genIndexStyles`: `.radarCurve-i` (16-22) and `.radarLegendBox-i` (23-27) for `i < THEME_COLOR_LIMIT`                                                                                                                                                                                                |
|                                                                                | 33-41                                                                   | `buildRadarStyleOptions`: `themeVariables` = theme-default's variables merged with `getConfig().themeVariables` (35-37); `radarOptions` = `themeVariables.radar` merged with the argument (38)                                                                                                       |
|                                                                                | 43-73                                                                   | `styles`: `.radarTitle` (46-51), `.radarAxisLine` (52-55), `.radarAxisLabel` (56-59), `.radarGraticule` (60-65), `.radarLegendText` (66-70), then `genIndexStyles` (71)                                                                                                                              |
|                                                                                | 75                                                                      | `export default styles`                                                                                                                                                                                                                                                                              |
| `diagrams/radar/types.ts`                                                      | 5-20, 30-41, 43-47                                                      | `RadarAxis`, `RadarCurve`, `RadarOptions`, `RadarDB`; `RadarStyleOptions` (with `legendBoxSize`, 39, which nothing reads, section 6.9); `RadarData`                                                                                                                                                  |
| `diagrams/radar/radar.spec.ts`                                                 | 1-271                                                                   | 20 tests (section 5)                                                                                                                                                                                                                                                                                 |
| `schemas/config.schema.yaml`                                                   | 2810-2862                                                               | `RadarDiagramConfig`: `width` (2817), `height` (2822), `marginTop` (2827), `marginRight` (2832), `marginBottom` (2837), `marginLeft` (2842), `axisScaleFactor` (2847), `axisLabelFactor` (2852), `curveTension` (2857)                                                                               |
|                                                                                | 424-425                                                                 | the `radar` key of the top-level properties, a `$ref` to the section                                                                                                                                                                                                                                 |
|                                                                                | 457-470                                                                 | `BaseDiagramConfig.useMaxWidth`, default `true` (468-470)                                                                                                                                                                                                                                            |
|                                                                                | 83-86                                                                   | `maxTextSize`, default 50000 (the text limit that bounds the curve entries, section 6.11)                                                                                                                                                                                                            |
|                                                                                | 303-304                                                                 | `logLevel` default 5 (`fatal`): `log.warn` is silent at the default (logger.ts:18-22, 32-46)                                                                                                                                                                                                         |
| `defaultConfig.ts`                                                             | 289-291                                                                 | `radar: { ...defaultConfigJson.radar }`                                                                                                                                                                                                                                                              |
| `config.type.ts`                                                               | 2247-2285                                                               | `RadarDiagramConfig` (the typed form of the schema section)                                                                                                                                                                                                                                          |
| `themes/theme-default.js`                                                      | 33, 36, 46, 195, 338-350                                                | `fontSize` `'16px'` (33), `THEME_COLOR_LIMIT = 12` (36), `titleColor` (195: `textColor`), the `radar` section (338-350)                                                                                                                                                                              |
| `themes/theme-base.js`, `theme-dark.js`, `theme-forest.js`, `theme-neutral.js` | the `radar` section                                                     | the only other themes with a `radar` section (base 273-284, dark 337-348, forest 310-321, neutral 345-356); redux, redux-dark, redux-color, redux-dark-color, neo and neo-dark have none                                                                                                             |
| `mermaidAPI.ts`                                                                | 319-334                                                                 | `createUserStyles`: `getStyles(type, css, { ...config.themeVariables, theme, look }, svgId)`                                                                                                                                                                                                         |
|                                                                                | 359                                                                     | `cleanedUpSvg = decodeEntities(cleanedUpSvg)`: the placeholders of `encodeEntities` are decoded in the final SVG text                                                                                                                                                                                |
| `diagram-api/comments.ts`                                                      | 6-8                                                                     | `cleanupComments`: removes whole-line `%%` comments that are not `%%{`, and trims the start                                                                                                                                                                                                          |
| `preprocess.ts`                                                                | 6-19, 58-91                                                             | `cleanupText` (CRLF to LF, quotes in tags), `preprocessDiagram`: front matter (`processFrontmatter`), directives, `cleanAndMerge` of the configuration, then `cleanupComments`                                                                                                                       |
| `Diagram.ts`                                                                   | 19-85                                                                   | `fromText`: the type (31), `db.clear()` (49), the front matter title (52-54), `encodeEntities(source) + '\n'` (77), `parser.parse` (83)                                                                                                                                                              |
| `utils.ts`                                                                     | 841-843, 874, 904                                                       | `cleanAndMerge` (a deep merge), `encodeEntities`, `decodeEntities`                                                                                                                                                                                                                                   |

Two more files set the context. `diagrams/common/populateCommonDb.ts` (4-14) sets the accessibility texts and the title when they are non-empty. `diagrams/common/commonDb.ts` (8-32) holds the header texts and their setters. The shared rules are those of the packet notes, sections 2.4 and 6.1.

## 2. Parser

### 2.1 The grammar, as written

```
Radar       : NEWLINE* ('radar-beta' | 'radar-beta:' | 'radar-beta' ':') NEWLINE*
              ( TitleAndAccessibilities
              | 'axis' axes+=Axis (',' axes+=Axis)*
              | 'curve' curves+=Curve (',' curves+=Curve)*
              | options+=Option (',' options+=Option)*
              | NEWLINE )*
Axis        : name=ID Label?
Curve       : name=ID Label? '{' Entries '}'
Entries     : NEWLINE* NumberEntry (',' NEWLINE* NumberEntry)* NEWLINE*
            | NEWLINE* DetailedEntry (',' NEWLINE* DetailedEntry)* NEWLINE*
DetailedEntry : axis=[Axis:ID] ':'? value=NUMBER
NumberEntry   : value=NUMBER
Option      : 'showLegend' BOOLEAN | 'ticks' NUMBER | 'max' NUMBER | 'min' NUMBER | 'graticule' GRATICULE
Label       : '[' label=STRING ']'
TitleAndAccessibilities : ((ACC_DESCR | ACC_TITLE | TITLE) EOL)+
EOL         : NEWLINE+ | EOF
```

Statements need no separator between them, but the items of one `axis` or `curve` statement and of one option list are separated by commas (`radar.langium:4-15`). `Entries` has no empty form, so `{}` is rejected.

What the DB receives from the AST: `populate` (`parser.ts:8-15`) passes the `axes`, `curves` and `options` lists, and `populateCommonDb` passes the header texts. Section 3.1 lists the calls in order.

### 2.2 Lexer: token order

The reference's lexer, read from `createRadarServices().Radar.parser.Lexer.definition` (probed, `lexer-dump.mts`), has 28 token types in this order. Chevrotain takes the first match, not the longest, so the order decides everything.

| #   | Token                 | Kind and pattern                                               | Notes                                                                                           |
| --- | --------------------- | -------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| 0   | `radar-beta:`         | keyword, 11 characters                                         | Sorted first by length. No `LONGER_ALT`, and no word-end suffix                                 |
| 1   | `radar-beta`          | keyword, `radar-beta(?:(?=%%)\|(?!\S))`                        | The suffix is the Mermaid token builder's (`common/tokenBuilder.ts:24`). `LONGER_ALT` ID        |
| 2   | `showLegend`          | keyword                                                        | `LONGER_ALT` ID                                                                                 |
| 3   | `graticule`           | keyword                                                        | `LONGER_ALT` ID                                                                                 |
| 4   | `curve`               | keyword                                                        | `LONGER_ALT` ID                                                                                 |
| 5   | `ticks`               | keyword                                                        | `LONGER_ALT` ID                                                                                 |
| 6   | `axis`                | keyword                                                        | `LONGER_ALT` ID                                                                                 |
| 7   | `max`                 | keyword                                                        | `LONGER_ALT` ID. Equal length to ID, so the keyword wins on the exact word                      |
| 8   | `min`                 | keyword                                                        | `LONGER_ALT` ID, as `max`                                                                       |
| 9   | `:`                   | keyword                                                        |                                                                                                 |
| 10  | `,`                   | keyword                                                        |                                                                                                 |
| 11  | `[`                   | keyword                                                        |                                                                                                 |
| 12  | `]`                   | keyword                                                        |                                                                                                 |
| 13  | `{`                   | keyword                                                        |                                                                                                 |
| 14  | `}`                   | keyword                                                        |                                                                                                 |
| 15  | `GRATICULE`           | `circle\|polygon`                                              | No `LONGER_ALT`: `circlex` is `circle` then ID `x`                                              |
| 16  | `BOOLEAN`             | `true\|false`                                                  | No `LONGER_ALT`: `trueX` is `true` then ID `X`. Case-sensitive                                  |
| 17  | `ACC_DESCR`           | `[\t ]*accDescr(?:[\t ]*:(...)\|\s*{(...)})`                   | Needs `:` or `{` after the word                                                                 |
| 18  | `ACC_TITLE`           | `[\t ]*accTitle[\t ]*:(...)`                                   | Needs `:`                                                                                       |
| 19  | `TITLE`               | `[\t ]*title(?:[\t ][^\n\r]*?(?=%%)\|[\t ][^\n\r]*\|)`         | Leading blanks are part of the token. A tab or a space after `title` takes the rest of the line |
| 20  | `NUMBER`              | `(?:[0-9]+\.[0-9]+(?!\.))\|(?:0\|[1-9][0-9]*(?!\.))`           | Before ID, so a word that starts with a digit is a number first                                 |
| 21  | `STRING`              | `"([^"\\]\|\\.)*"\|'([^'\\]\|\\.)*'`                           | Single or double quotes                                                                         |
| 22  | `ID`                  | `[\w]([-\w]*\w)?`                                              | No dash at the end, no dot, no non-ASCII letter                                                 |
| 23  | `NEWLINE`             | `\r?\n`                                                        | A lone `\r` is a lexer error (probed)                                                           |
| 24  | `WHITESPACE`          | `[\t ]+`, skipped                                              |                                                                                                 |
| 25  | `YAML`                | `---[\t ]*\r?\n(?:[\S\s]*?\r?\n)?---(?:\r?\n\|(?!\S))`, hidden | A block of dashes is hidden. Without a closing fence the dashes are a lexer error (probed)      |
| 26  | `DIRECTIVE`           | `[\t ]*%%{[\S\s]*?}%%(?:\r?\n\|(?!\S))`, hidden                |                                                                                                 |
| 27  | `SINGLE_LINE_COMMENT` | `[\t ]*%%[^\n\r]*`, hidden                                     |                                                                                                 |

Rules that the table implies, all probed:

- Keywords come first and are sorted by descending length, with equal lengths in grammar order (`token-builder.js:83`). A keyword that is also a prefix of an ID is an ID when the ID is longer: `maximum`, `axisX`, `curvex` and `ticksy` are IDs (probed `curvex`, `ticksy`). A keyword that matches the whole word is the keyword: `axis max` and `axis curve` are rejected.
- The terminals that have no `LONGER_ALT` (`GRATICULE`, `BOOLEAN`, `NUMBER`, `TITLE`) take a prefix of any word they match: `polygonal`, `circlex`, `trueX`, `falseAlarm`, `1A`, `12` and `title1` all fail, and `axis accTitle` and `axis accDescr` are IDs because the accessibility terminals need `:` or `{`.
- The header keyword ends at a blank, the end of the text, or `%%`. `radar-beta{` and `radar-betaX` lex as one ID and fail (probed).
- `GRATICULE`, `BOOLEAN` and the rest of the terminals sit in the grammar's own order, which the keywords precede. Keywords in terminal rules are regexes, so `circle`, `polygon`, `true` and `false` are not keyword tokens (`lexer-dump.mts`).

### 2.3 Value converter

The Mermaid converter runs first (`common/valueConverter.ts:27-42`), then Langium's default (`value-converter.js`). Per token:

| Token       | Converter                                                                                                                                                                                                            | Value (probed)                                                                     |
| ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| `TITLE`     | `titleRegex` on the token (`matcher.ts:14`): the text after the first blank, trimmed, and each run of two or more blanks or tabs to one space (`valueConverter.ts:58-59`)                                            | `title   My  Title` is `My Title`; `title` alone is `''` (populate skips it)       |
| `ACC_TITLE` | `accessibilityTitleRegex` (`matcher.ts:9`), trimmed and collapsed                                                                                                                                                    | `accTitle: Hello there` is `Hello there`                                           |
| `ACC_DESCR` | `accessibilityDescrRegex` (`matcher.ts:4`); single line trimmed; the block form (`{ … }`) loses the blanks at each line's start and end, collapses blank runs, and collapses blank lines (`valueConverter.ts:61-68`) | `accDescr {` / `  first line` / `  second line` / `}` is `first line\nsecond line` |
| `STRING`    | Langium `convertString`: the quotes are removed and `\b \f \n \r \t \v \0` are unescaped; any other escaped character stands for itself                                                                              | `"a\"b"` is `a"b`; `'x y'` is `x y`; `"a\tb"` holds a tab                          |
| `NUMBER`    | Langium `convertNumber`: `Number(input)` (`value-converter.js:89-90`)                                                                                                                                                | `2.5` is 2.5; `1234…890` (30 digits) is `1.2345678901234568e+29`                   |
| `BOOLEAN`   | Langium `convertBoolean`: `input.toLowerCase() === 'true'` (`value-converter.js:93`)                                                                                                                                 | `true` and `false` only, since the token is case-sensitive                         |
| `GRATICULE` | none: a string                                                                                                                                                                                                       | `circle` or `polygon`                                                              |
| `ID`        | none (the `^` removal of Langium is unreachable here)                                                                                                                                                                | the text as written, so `a-1` and `_x_1` are kept                                  |

`NUMBER` gives a JavaScript double, so a 400-digit integer is `Infinity` (node). The canonical JSON form is then the string `"Infinity"` (canonical-model.md, Numbers).

### 2.4 Preprocessing and detection

The text goes through `preprocessDiagram` (`preprocess.ts:58-91`), which `mermaidAPI.ts:73` calls before `Diagram.fromText` (`mermaidAPI.ts:571-572`). The steps of the packet notes, section 2.4, run in this order (probed with the same steps in `db-probe.mts`):

1. `cleanupText` (`preprocess.ts:6-17`): CRLF and CR become LF; inside a tag, double-quoted attributes become single-quoted. A label written as `<a href="x">` is read by the grammar as `<a href='x'>`, and a label keeps that form. A title is sanitised afterwards.
2. Front matter (`processFrontmatter`, `preprocess.ts:19-31`): its `title` and its `config`.
3. Directives (`processDirectives`, `preprocess.ts:33-45`): the `%%{init}%%` configuration, merged over the front matter with `cleanAndMerge`, then the directives are removed.
4. `cleanupComments` (`diagram-api/comments.ts:6-8`): whole-line `%%` comments go, and a line of only `%%` stays.

Then `Diagram.fromText` (`Diagram.ts:19-85`): `db.clear()` (`Diagram.ts:49`), the front matter title (`Diagram.ts:52-54`), `encodeEntities(cleaned) + '\n'` (`Diagram.ts:77`), and the parser (`Diagram.ts:83`).

The detector is `/^\s*radar-beta/` (`detector.ts:10`). The port's detection must use the same test, so `radar-betaX` selects the radar type, and the grammar then rejects it at the header.

### 2.5 Probed acceptance

Each row is a source text given to `parse('radar', text)`, with the AST values it produced (probed, `parse-probe.mts` and `parse-probe2.mts`). "Reject" means the parser or the lexer threw.

**Header and statements.** Accept: `radar-beta`, `radar-beta:`, `radar-beta :`, `radar-beta %% note`, `radar-beta%% note`, the header alone, the header without a newline, the header and a statement on one line (`radar-beta axis A curve c{1}`), leading blanks, a comment before the header, leading and trailing blank lines, CRLF, and tabs as separators. Reject: `radar-beta{`, `radar-betaX`, the empty text, `pie`.

**Statements.** Accept: `axis A,B,C`, `axis A, B, C`, two curves on one line (`curve c1{1,2} curve c2{3,4}`), `ticks 3 max 5` (no comma between options), a duplicate axis, and a duplicate option. Reject: `axis A B` (no comma), `axis A,` and `axis A, B,` (a trailing comma), `axis` alone, a label without brackets (`axis A "label"`), a label on the next line (`axis A` / `["x"]`), `curve c` / `{1,2}` (the brace on the next line), `axis A,` / `B` (a newline after the comma), `showLegend` / `true` (a newline before the value), and `ticks 5,` / `max 3`.

**Names.** Accept: `a-1`, `_x_1`, `curvex`, `ticksy`, `accTitle`, `accDescr`, `showLegend2` (as axis names), and `radar-beta` as a curve name (`curve radar-beta{1}`). Reject: `max`, `min`, `axis`, `curve`, `ticks` (keyword as name), `circle`, `trueX`, `falseAlarm`, `true`, `title1`, `1A`, `12`, `A-` (lexer error), `a.b` (lexer error), `Äx` (lexer error).

**Entries.** Accept: `c{1,2,3}`, `c { 1, 2 }`, `c{1 ,2}`, `c{1,\n2}` (a newline after a comma), `c{\n1,\n2\n}`, `c{0.0001}`, `c{123…890}`, `c{A 1, B 2}` and `c{A: 1, B: 2}` (the detailed form, with or without `:`). Reject: `c{}`, `c{1,}`, `c{1 2}` (no comma), `c{1\n,2}` (a newline before the comma), `c{01}` (`0` then `1`), `c{1e3}`, `c{.5}` and `c{1.}` (lexer errors), `c{1.5.2}` (lexer error), `c{-1}` (lexer error: there is no minus token), `c{A 1, 2}` and `c{1, B 2}` (the two forms mixed).

**Options.** Accept: `showLegend true` and `false`, `ticks 5`, `ticks 2.5`, `ticks 0`, `graticule polygon`, `graticule circle`, `showLegend true, ticks 3`. Reject: `showLegend 1`, `showLegend True`, `min -1` (lexer error), `graticule square`, `graticule polygonal`.

**Titles, comments, directives, the body.** Accept: `title My Title`, `title Spaced    Title` (collapsed to `Spaced Title`), `title` alone (`''`), `title Best [v2] (x), y`, `title T %% c` (`T`), `accTitle: Hello there`, the block `accDescr`, a comment line in the body, a directive in the body, and a YAML-like block in the body (`---` … `---` is hidden). Reject: an unclosed `---` in the body (lexer error), a lone `\r` (lexer error). Also accepted, and worth a note: `title\taxis A` is the title `axis A` with no axis, since a tab after `title` takes the rest of the line (probed).

### 2.6 Quirks the port must reproduce

1. The header must be followed by a blank, the end of the text, `%%`, or the colon (section 2.5). A word that starts with `radar-beta` and goes on is an ID, so the detector accepts text that the grammar rejects.
2. Statements need no separator. The items of a list are separated by commas, and a line break is allowed only where the grammar has a `NEWLINE` slot: inside `{ … }` around the entry commas, and at the end of a statement (section 2.1).
3. An ID cannot begin with a digit, cannot end with `-`, and cannot contain `.` or a non-ASCII letter. A word that begins with `circle`, `polygon`, `true`, `false`, `title` or a number is split by the lexer, and the rest of the word is then a second token.
4. The keywords (`axis`, `curve`, `ticks`, `showLegend`, `graticule`, `max`, `min`) are not names when they stand alone, and they are names when a longer word begins with them.
5. Entries are either all numbers or all named. A named entry names an axis by its name, not its label (`A` in `A["Alpha"]`).
6. Numbers have no sign and no exponent, and `0` followed by a digit is two numbers. `2.5` is a number; `2.` and `.5` are lexer errors.
7. Duplicate axes, duplicate curves and duplicate options are accepted by the grammar. The DB decides (section 3.6).
8. `title` takes the rest of the line after its first blank or tab, and ends at `%%`. A blank last `title` line is a `title` with the empty value, which the front matter's title replaces (section 3.5).
9. A YAML-like block in the body is hidden as a token, and a directive in the body is hidden too. The preprocessing removes the directives earlier, so the body directive is seen only by the grammar when the text goes straight to the parser.

## 3. DB

### 3.1 Calls in order

| Step | Call                                                                       | Effect                                                                                                                 |
| ---- | -------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| 1    | `db.clear()` (`Diagram.ts:49`, `db.ts:114-117`)                            | Clears the header texts (`commonClear`) and the data, which becomes a fresh copy of the defaults                       |
| 2    | front matter title (`Diagram.ts:52-54`)                                    | `setDiagramTitle` when the front matter has a `title`                                                                  |
| 3    | `parser.parse(text)` (`Diagram.ts:83`), then `populate` (`parser.ts:8-15`) | `populateCommonDb` (accessibility texts first, then the title if non-empty), then `setAxes`, `setCurves`, `setOptions` |
| 4    | `draw` (`renderer.ts:8-46`)                                                | Reads `getAxes`, `getCurves`, `getOptions`, `getConfig` and `getDiagramTitle`, and nothing else                        |

Axes are set before curves whatever the text order, so a curve may name an axis that the text declares later (probed D5: `curve c{A 1, B 2}` before `axis A, B` gives entries `[1, 2]`). `setCurves` reads `getAxes()`, which is why the order matters.

### 3.2 The three setters

- `setAxes` (`db.ts:51-58`): one entry per axis, in text order, `{ name, label }`, with `label = axis.label ?? axis.name`. Duplicate names are kept (probed D11: `axis A, A, B` gives two `A`).
- `setCurves` (`db.ts:60-68`): one entry per curve, `{ name, label, entries }`, with `label = curve.label ?? curve.name`.
- `computeCurveEntries` (`db.ts:70-86`): if the first entry has no axis reference (`entries[0].axis == undefined`, line 72), the values are the entries in text order. Otherwise there must be at least one axis (throw at 77), and for each axis in axis order the first entry whose `$refText` equals the axis name supplies the value (80). An axis with no entry throws `Missing entry for axis <label>` (82). The message uses the label, not the name.
  - Probed: `curve c{A 1}` with no axes throws `Axes must be populated before curves for reference entries` (D6). `axis A, B` with `curve c{A 1}` throws `Missing entry for axis B` (D7), and with `B["Bee"]` it throws `Missing entry for axis Bee` (D7b).
  - Probed: an entry for an unknown axis is dropped (D8: `curve c{A 1, B 2, Z 9}` gives `[1, 2]`), and the first of two entries for one axis wins (D9: `A 1, A 5, B 2` gives `[1, 2]`).
  - Probed: a number list is kept as it is, whatever its length (D10: `curve c{1,2}` for three axes gives `[1, 2]`). The renderer then skips the curve (section 6.2).
- `setOptions` (`db.ts:88-112`): the options are reduced into a map by name, so the last one wins (probed D15: `ticks 3` then `ticks 4` gives 4). Each field takes its value with `??` from the default (98-104): `showLegend` true, `ticks` 5, `max` null, `min` 0, `graticule` `'circle'`. A `ticks` above 32 is set to 32 (106-111, probed D14), with a `log.warn` that is silent at the default log level (section 3.6).

### 3.3 Sanitising

The header texts go through the shared setters (`diagrams/common/commonDb.ts:16-30`), which run `sanitizeText` (`diagrams/common/common.ts:71-83`): DOMPurify with `FORBID_TAGS: ['style']`. `setAccTitle` also removes leading blanks, and `setAccDescription` removes the blanks after each line break (`commonDb.ts:17, 23`).

| Text                       | Sanitised?             | Probed value (DB)                     | Probed rendered text                                                                            |
| -------------------------- | ---------------------- | ------------------------------------- | ----------------------------------------------------------------------------------------------- |
| title                      | yes                    | `Hello <b>bold</b> &amp; more` (D18)  | `Hello <b>bold</b> &amp; more` as text: the `&amp;` shows (section 6.5)                         |
| title with `<img onerror>` | yes, `onerror` removed | `<img src="x">T` (D19)                | `<img src="x">T` as text                                                                        |
| accTitle, accDescr         | yes                    | `A <b>t</b>`, `x<img src="x">y` (D20) | Not drawn as text. The shared pipeline writes `<title>` and `<desc>` (packet notes 6.1, step 4) |
| axis label, curve label    | **no**                 | `<b>x</b> & y`, `<i>k</i>` (D21)      | `x & y <i>` as text, serialised as `x &amp; y &lt;i&gt;` (title-probe)                          |
| ids                        | no id texts            | n/a                                   | Ids are not drawn: the labels are                                                               |

The labels are not sanitised, and the renderer writes them with `.text()`, so their markup is literal text. That is a difference from the titles, which were sanitised and then written as text, and so show the sanitised entities (section 6.5).

### 3.4 Defaults and what `clear` resets

`clear` resets the header texts and `data` (`db.ts:114-117`). It does not reset the site configuration, which is read through `getConfig` (`db.ts:39-45`), and `getConfig` is merged on every call, so a configuration set after `clear` but before `draw` is seen. The defaults are in the table of section 6.8.

Probed (D29, D29b): a title set by one parse is gone after the next parse of a diagram without one.

### 3.5 Title precedence

`Diagram.fromText` sets the front matter title first, then the grammar's title replaces it when non-empty (`populate`, `diagrams/common/populateCommonDb.ts:11-13`). Probed (D24 to D26): the front matter `FM Title` alone gives `FM Title`; with `title Body Title` in the body the title is `Body Title`; with an empty `title` line in the body the title is `FM Title`.

### 3.6 Quirks of the DB

1. `ticks` is clamped to 32, and the warning (`db.ts:107-109`) is silent at the default `logLevel` (5, `fatal`), because `log.warn` is a no-op there (`logger.ts:22`). Probed: 40 and 33 give 32 (D13, D14).
2. `ticks` keeps fractions and zero (D16, D17: 2.5 and 0). The renderer uses them (section 6.2).
3. A `max` of 0 is kept, and so is a `min` of 0 (D30). `null` only means that no `max` was written.
4. Front matter and directive configuration reach `getConfig` (D27, D28). The values are not validated: width 0, a negative `axisScaleFactor`, an unknown key and a `curveTension` of 2 are kept (D34 to D37). Only the renderer's arithmetic decides what they draw (section 6.11).
5. An entity such as `#35;` reaches the DB as the placeholder `ﬂ°°35¶ß` (D22, D23). The placeholder becomes the entity text `&#35;` in the final SVG text (`mermaidAPI.ts:359`, `utils.ts:904-906`), which the browser reads as `#`, so the rendered label is `a # b` (probed Q9).

## 4. Canonical model proposal

The DB is the only render input: `draw` reads five getters and nothing else (`renderer.ts:9-14`), so one model serves the differential and the renderer. The model follows [canonical-model.md](../canonical-model.md): camelCase keys, `"version": 1`, header texts as `""` when absent, numbers as JSON numbers.

```text
RadarDiagram { version: 1, title, accTitle, accDescr,
  axes:   [{ name, label }],
  curves: [{ name, label, entries: [number] }],
  options: { showLegend, ticks, max, min, graticule } }
```

| Field                           | Rule                                                                                                                                                            |
| ------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `title`, `accTitle`, `accDescr` | The DB's values after the setters (section 3.3). `""` when absent                                                                                               |
| `axes`                          | In text order. `label` is the DB's label, so the name is the fallback (section 3.2). Duplicates are kept                                                        |
| `curves`                        | In text order. `entries` are in axis order for the named form, and in text order for the number form. The length is not checked (D10)                           |
| `options.showLegend`            | Boolean                                                                                                                                                         |
| `options.ticks`                 | The DB's number after the clamp (32 at most). A fraction is kept as a JSON number                                                                               |
| `options.max`                   | A number, or JSON `null` when no `max` was written. It is the one nullable field                                                                                |
| `options.min`                   | A number, default 0                                                                                                                                             |
| `options.graticule`             | `"circle"` or `"polygon"`                                                                                                                                       |
| numbers                         | `Infinity` (a 400-digit number) and `-0` are written as the strings `"Infinity"` and `"0"`, as the canonical rule says (node probe). Neither is in the fixtures |

Left out, and why: the configuration (`width`, the margins and the rest of section 6.8), which `pnpm port:diff config` compares; the theme (section 6.9), which is typed per decision 0009; and everything the renderer derives (the radius, the maximum used for drawing, the angles, the points), which the geometry harness compares.

## 5. Specs

### 5.1 `radar.spec.ts` (271 lines, 20 tests)

| Lines    | Tests                                                                                                                                                                                                    | What they test                                                                                                          |
| -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| 24       | `should handle a simple radar definition`                                                                                                                                                                | Parse only (`resolves.not.toThrow`)                                                                                     |
| 31       | `should handle diagram with data and title`                                                                                                                                                              | Parse, then the title, the accessibility texts, the axes with labels, the curve, the default options (inline snapshots) |
| 83       | `should handle a radar diagram with options`                                                                                                                                                             | Parse, then the options (`ticks 10`, `showLegend false`, `graticule polygon`, `min 1`, `max 10`)                        |
| 103      | `should handle curve with detailed data in any order`                                                                                                                                                    | Parse, then the entries in axis order (`C: 3, A: 1, B: 2`)                                                              |
| 123      | `should handle radar diagram with comments`                                                                                                                                                              | Parse only, with `%%` lines                                                                                             |
| 133      | `should handle radar diagram with config override`                                                                                                                                                       | Parse only, with a `%%{init}%%` directive that is not JSON (single quotes)                                              |
| 143      | `should parse radar diagram with theme override`                                                                                                                                                         | Parse only                                                                                                              |
| 153      | `should handle radar diagram with radar style override`                                                                                                                                                  | Parse only                                                                                                              |
| 163      | `should limit the number of ticks to MAX_TICKS`                                                                                                                                                          | Parse, then `ticks` 12, 32 and 33 give 12, 32 and 32                                                                    |
| 180-200  | `relativeRadius`, 6 tests: `(5, 0, 10, 100)` is 50; `(0, 0, 10, 100)` is 0; `(10, 0, 10, 100)` is 100; `(-5, 0, 10, 100)` is 0 (clipped); `(15, 0, 10, 100)` is 100 (clipped); `(5, -10, 10, 100)` is 75 | The renderer's scale, exported                                                                                          |
| 206-230  | `closedRoundCurve`: 3 tests: tension 0 gives straight `C` segments; tension 0.5 with two points; tension 0.5 with four points                                                                            | The path string, exact (`"M0,0 C50,-50 50,-50 100,0 …  Z"`)                                                             |
| 246, 255 | `draw` simple and complex                                                                                                                                                                                | Run the renderer on a DOM, with no assertion. They pin that the calls do not throw                                      |

Not covered by any spec: the rendered structure, the attributes, the style sheet, the theme fallback, the sanitising, the lexer's rejections, the value converters, the entity and title quirks, and every case of section 6.11. The port's tests for these are the probes of this note, pinned as cases (sections 2.5, 3.2 and 6.11).

### 5.2 The render check is the harness

The reference's own specs check almost no output. The rendered structure is checked by the geometry harness (section 7). The port's tests should pin the probes of sections 2.5, 3.2 and 6.11, which are listed with their results above.

### 5.3 Tooling gaps for the coordinator

- There is no `tools/port/export-radar.ts`. The exporters of the other types (for example `export-architecture.ts`, `exporter.ts` `runExporter`) are the model.
- `tools/port/reference.ts` `registerDetectors` does not register the radar detector. Without it, a `%%{init}%%` directive throws `No diagram type detected matching given configuration`. The probe registers it with `detectType.registerLazyLoadedDiagrams(radar)` from `diagrams/radar/detector.ts` (probed, D28 after the fix). The coordinator decides whether the shared registration gets radar or only the exporter does.

## 6. Renderer

### 6.1 Pipeline

The generic steps are those of the packet notes, section 6.1 (`mermaidAPI.ts:397-421`, `319-334`, `596-608`, `623`, `750-758`, `344-364`, `629-650`). `draw` is synchronous and returns nothing (`renderer.ts:8`).

Probed, for the simple fixture (`render-out.json`, structure read with `outline.py`): the root is `svg` with the attributes `id`, `width="100%"`, `xmlns`, `style="max-width: 700px;"`, `viewBox="0 0 700 700"`, `overflow="visible"`, `role="graphics-document document"` and `aria-roledescription="radar"`. Its children are the `style` element (first, from the pipeline), an empty `g` (from the pipeline, the same as the other types), and the translated `g` that `drawFrame` returns.

### 6.2 `draw`: the placement (`renderer.ts`)

| Quantity                            | Formula                                                                                                                                            | Source                           | Probed value (simple fixture)                                            |
| ----------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------- | ------------------------------------------------------------------------ |
| `maxValue`                          | `options.max ?? max over curves of max(entries)`. For no curves, `Math.max()` is `-Infinity`                                                       | 23-24                            | 3                                                                        |
| `minValue`                          | `options.min`                                                                                                                                      | 25                               | 0                                                                        |
| `radius`                            | `Math.min(width, height) / 2`                                                                                                                      | 26, 145                          | 300                                                                      |
| total width and height              | `width + marginLeft + marginRight`; `height + marginTop + marginBottom`                                                                            | 51-52                            | 700 and 700                                                              |
| centre                              | `(marginLeft + width / 2, marginTop + height / 2)`, the translation of the `g`                                                                     | 53-56, 61                        | `translate(350, 350)`                                                    |
| root size                           | `configureSvgSize(svg, H, W, useMaxWidth ?? true)`: `width="100%"` and `style="max-width: Wpx;"`, or `width` and `height` attributes when false    | 57, `setupGraphViewbox.js:23-46` | `max-width: 700px;`                                                      |
| `viewBox`, `overflow`               | `0 0 W H`; `overflow` `visible`                                                                                                                    | 59                               | `0 0 700 700`                                                            |
| graticule circle `i`                | `r = (radius * (i + 1)) / ticks` for `i < ticks`, a `circle` with `r`                                                                              | 71-76                            | 60, 120, 180, 240, 300                                                   |
| graticule polygon `i`               | the same `r`; vertices `(r cos a, r sin a)` for `a = (2 * j * PI) / numAxes - PI / 2`, `j` over the axes                                           | 77-92                            | `points` with five vertices for a five-axis graticule                    |
| axis `i`                            | `angle = (2 * i * PI) / numAxes - PI / 2`; `cosA`, `sinA`                                                                                          | 103-107                          | axis 0 is `-PI/2`; axis 1 of three is `PI/6`                             |
| axis line `i`                       | `x2 = radius * axisScaleFactor * cosA`, `y2 = radius * axisScaleFactor * sinA`, from `(0, 0)`                                                      | 109-114                          | axis 0: `(1.8369701987210297e-14, -300)`                                 |
| axis label anchor                   | `cosA > 0.01` `start`; `cosA < -0.01` `end`; else `middle`                                                                                         | 118                              | A `middle`, B `start`, C `end`                                           |
| axis label baseline                 | `sinA > 0.01` `hanging`; `sinA < -0.01` `auto`; else `central`                                                                                     | 119                              | A `auto`, B and C `hanging`                                              |
| axis label position                 | `x = radius * axisLabelFactor * cosA + labelPad * cosA`; `y` the same with `sinA`; `labelPad = 4`                                                  | 123, 127-128                     | A `(1.9533116446400285e-14, -319)`                                       |
| curve point `i`                     | `angle = (2 * PI * i) / numAxes - PI / 2`; `r = relativeRadius(entry, min, max, radius)`; `(r cos, r sin)`                                         | 154-157                          | axis 0 of c1: `(6.123233995736766e-15, -100)`                            |
| curve `path` (circle graticule)     | `closedRoundCurve(points, curveTension)`                                                                                                           | 163-165                          | `M6.12…e-15,-100 C…`                                                     |
| curve `polygon` (polygon graticule) | the points, joined by spaces                                                                                                                       | 168-170                          | Q13, Q20                                                                 |
| curve skip                          | a curve whose entry count is not the number of axes draws nothing (its index is kept)                                                              | 148-151                          | Q2, Q8                                                                   |
| `relativeRadius(v, min, max, R)`    | `clip = min(max(v, min), max)`; `(R * (clip - min)) / (max - min)`                                                                                 | 175-183                          | Q11, Q12                                                                 |
| `closedRoundCurve(points, t)`       | `M p0`; for each `i`, with `p0..p3` the cyclic neighbours: `cp1 = p1 + (p2 - p0) * t`, `cp2 = p2 - (p3 - p1) * t`, then ` C cp1 cp2 p2`; then ` Z` | 185-207                          | Q16 with `t = 0` is straight                                             |
| legend position                     | `legendX = ((width / 2 + marginRight) * 3) / 4`; `legendY = (-(height / 2 + marginTop) * 3) / 4`; spacing 20                                       | 220-222                          | `translate(262.5, -262.5)` and `translate(262.5, -242.5)` for the second |
| legend item                         | `g` translated, `rect` 12 by 12 (`radarLegendBox-i`), `text` at `x = 16`, `y = 0` (`radarLegendText`)                                              | 225-242                          | (the 12 is a constant, section 6.9)                                      |
| title                               | `text` class `radarTitle`, `x = 0`, `y = -height / 2 - marginTop`, the title text                                                                  | 41-45                            | `y = -350`                                                               |

The DOM order of the translated `g` is the order of the statements (probed, all fixtures): the graticule (all circles, or all polygons), then for each axis in order its `line` and its `text`, then the curves in order (each one only if its entry count is right), then, when `showLegend` is set, one legend `g` per curve (including the curves that were skipped for drawing), then the title `text`. The title is the last child, and it is always present, even when empty (probed Q7 and Q19; `renderer.ts:41-45` appends it unconditionally).

Probed: for `ticks 3` with a polygon graticule, the output has three graticule polygons (`r = 100, 200, 300`, Q13), and a `ticks 40` is drawn as 32 circles at multiples of 9.375 (Q3).

### 6.3 Trigonometry, arithmetic and d3

**Math functions.** Every call into the platform's maths in radar code is in this list:

| Function               | Where                              | Argument                                    |
| ---------------------- | ---------------------------------- | ------------------------------------------- |
| `Math.cos`, `Math.sin` | 85-86 (graticule polygon vertices) | `(2 * j * Math.PI) / numAxes - Math.PI / 2` |
| `Math.cos`, `Math.sin` | 106-107 (axes)                     | `(2 * i * Math.PI) / numAxes - Math.PI / 2` |
| `Math.cos`, `Math.sin` | 156-157 (curve points)             | `(2 * Math.PI * i) / numAxes - Math.PI / 2` |
| `Math.min`, `Math.max` | 24, 26, 145, 181                   | exact on doubles (comparisons and spreads)  |

There is no `atan2`, `sqrt`, `tan`, `pow` or `log`. The three angle expressions are the same double for every `i` and `numAxes` (probed in Node for `numAxes` from 3 to 32: 0 of 525 pairs differ, and the cosines agree too, since multiplying by 2 is exact). The port's `cos` and `sin` must be `ferrigura_jsmath`'s (`crates/ferrigura-jsmath/src/trig.rs:17, 37`, the pair the ishikawa renderer uses, `crates/ferrigura-render/src/ishikawa/mod.rs:20`, and the decision that the angle arithmetic uses V8's functions, P2-PLAN.md row "Math functions"). The arithmetic is written left to right, so the port keeps the reference's order: `radius * axisScaleFactor * cosA` is `(radius * axisScaleFactor) * cosA`, and `(2 * j * PI) / n` is `((2 * j) * PI) / n`.

**d3.** The renderer uses four d3 pieces, and no other d3 module: `select` from `d3` (`selectSvgElement.ts:1, 14-20`: `select('body')`, then `.select('#id')`, or the iframe body in sandbox mode), and the selection methods `append`, `attr` and `text` (`renderer.ts`, throughout; `configureSvgSize` uses `attr`, `setupGraphViewbox.js:9-13, 43-46`). `text(value)` sets the text content (d3-selection's documented behaviour; **unverified** by reading d3's source). `attr` with a number writes the number's JavaScript text (section 6.4).

### 6.4 Number text in attributes

Every coordinate reaches the SVG through `attr` or a template literal, so it is written with `Number.prototype.toString`: the shortest text that reads back to the same double, with an exponent below 1e-6 and from 1e21 (node). Probed values in the output: `6.123233995736766e-15`, `1.8369701987210297e-14`, `1e-7`, `-0` written as `0` (the complex fixture's `0,0` polygon vertex, from `0 * cos(…)`), `259.8076211353316`, `149.99999999999994`. The port's shared writer is `svg::number` (`crates/ferrigura-render/src/svg.rs:27`). Whether it writes these forms is **unverified**; the values above are the cases to check.

### 6.5 Text: labels and the title

No radar code calls `getBBox`, `getComputedTextLength`, `getBoundingClientRect` or `calculateTextDimensions` (grep of `diagrams/radar/`). The positions of the labels come from constants and the radius only, so the renderer's output does not depend on the font. The geometry harness does measure: every `text` element's box is compared (`diagram-geometry.ts:145-200`, the `BOXED` set), so the port's text engine has to produce the reference's boxes for the axis labels (three anchors and three baselines, with the font of the fixtures) and for the legend and the title. That is decision 0005's `measure_box`, not radar code, and it is the one place where radar depends on text metrics.

Text content, as probed:

- The axis label is `label` (`renderer.ts:126`), written with `.text()`, so raw HTML in a label is literal text: `<b>x</b>` shows as `<b>x</b>` (Q10).
- The title is the sanitised DB text, written with `.text()`. The sanitised text holds `&amp;`, which `.text()` does not decode, so the shown title has `&amp;` where the author wrote `&`: `Hello <b>bold</b> &amp; more` (title-probe, computed text content). The serialised SVG then holds `&amp;amp;`. The output's `decodeEntities` (`mermaidAPI.ts:359`, `utils.ts:904-906`) rewrites only the placeholders (`ﬂ°°` to `&#`, `ﬂ°` to `&`, `¶ß` to `;`), so this text stays as it is. The port must reproduce it (section 8.3, O5).
- An entity `#35;` in a label is decoded at serialisation, so the label shows `#` (Q9).

### 6.6 The root `svg` and `viewBox`

Probed matrix (`render-out.json`): default `max-width: 700px;` and `viewBox="0 0 700 700"`; `useMaxWidth: false` gives `width="700"` and `height="700"`, with no style (Q15); `width: 400` and `height: 300` give `max-width: 500px;` and `viewBox="0 0 500 400"`, with the radius 150 and the title at `y = -200` (Q14); the config override of marginTop 100 gives `viewBox="0 0 700 750"` (fixture 3); a width of 0 gives `max-width: 100px;` and `viewBox="0 0 100 700"`, with every circle at radius 0 (Q25).

### 6.7 Style sheet (`styles.ts`)

The sheet is built by `styles` and prefixed by the pipeline's `compileCSS` with the diagram id (`#<id> .radarTitle`, probed). Its rules, with the values the probes show under `redux-color` (the harness's theme):

| Rule                | Declarations                                                                                                             | Source | redux-color value (probed)                                       |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------ | ------ | ---------------------------------------------------------------- |
| `.radarTitle`       | `font-size: fontSize`; `color: titleColor`; `dominant-baseline: hanging`; `text-anchor: middle`                          | 46-51  | `font-size:14px; color:#000000`                                  |
| `.radarAxisLine`    | `stroke: axisColor`; `stroke-width: axisStrokeWidth`                                                                     | 52-55  | `stroke:#333333; stroke-width:2`                                 |
| `.radarAxisLabel`   | `font-size: axisLabelFontSize px`; `color: axisColor`                                                                    | 56-59  | `font-size:12px; color:#333333`                                  |
| `.radarGraticule`   | `fill: graticuleColor`; `fill-opacity: graticuleOpacity`; `stroke: graticuleColor`; `stroke-width: graticuleStrokeWidth` | 60-65  | `fill:#DEDEDE; fill-opacity:0.3; stroke:#DEDEDE; stroke-width:1` |
| `.radarLegendText`  | `text-anchor: start`; `font-size: legendFontSize px`; `dominant-baseline: hanging`                                       | 66-70  | `font-size:12px`                                                 |
| `.radarCurve-i`     | `color`, `fill`, `stroke` = `cScale<i>`; `fill-opacity` and `stroke-width` from `curveOpacity` and `curveStrokeWidth`    | 16-22  | `.radarCurve-0` `fill:#f4a8ff; fill-opacity:0.5; stroke-width:2` |
| `.radarLegendBox-i` | `fill`, `stroke` = `cScale<i>`; `fill-opacity` = `curveOpacity`. No stroke width                                         | 23-27  | `fill:#f4a8ff; fill-opacity:0.5`                                 |

`genIndexStyles` writes the two curve rules for `i` from 0 to `THEME_COLOR_LIMIT - 1`, which is 12 (`theme-default.js:36`, and 12 in the redux themes). Probed: a thirteenth curve (`radarCurve-12`) has no rule, so it is filled with the root's fill (`rgb(40, 37, 61)`), at opacity 1, with no stroke (computed-probe).

**CSS semantics, probed in Chromium (computed-probe).**

- `color` does not paint SVG text. The axis labels and the title are filled with the root's `fill` (`#28253D` in redux-color), inherited from `#probe-N` (`fill:#28253D`), and the computed `color` of the title (`rgb(0, 0, 0)`) has no effect on what is drawn.
- A numeric `fontSize` (fixture 5 sets `fontSize: 80`) gives `font-size:80;` (unitless, not valid CSS), which the browser drops. The title's computed size is 16px (the inherited body size), not the theme's 14px (computed-probe, `fontSize80`).
- The axis lines use `stroke`, which paints.

### 6.8 Configuration

The renderer reads these keys of `radar` (`getConfig`, `db.ts:39-45`, the defaults from `config.schema.yaml:2817-2857`, `useMaxWidth` from `schema:468-470`):

| Key               | Default | Read at (`renderer.ts`)                     | Probed effect                                                          |
| ----------------- | ------- | ------------------------------------------- | ---------------------------------------------------------------------- |
| `width`           | 600     | 26, 51, 54, 145, 220                        | Radius, total width, centre x, legend x (Q14)                          |
| `height`          | 600     | 26, 52, 56, 45, 145, 221                    | Radius, total height, centre y, title y, legend y (fixture 3, Q14)     |
| `marginTop`       | 50      | 45, 52, 56, 221                             | Title y, total height, centre y, legend y (fixture 3)                  |
| `marginRight`     | 50      | 51, 220                                     | Total width, legend x                                                  |
| `marginBottom`    | 50      | 52                                          | Total height                                                           |
| `marginLeft`      | 50      | 51, 54                                      | Total width, centre x                                                  |
| `axisScaleFactor` | 1       | 112-113                                     | Axis line length (fixture 3: 150 for 300 radius at 0.5)                |
| `axisLabelFactor` | 1.05    | 127-128                                     | Label distance                                                         |
| `curveTension`    | 0.17    | 164 (passed to `closedRoundCurve`), 197-202 | Tension 0 gives straight segments (Q16)                                |
| `useMaxWidth`     | true    | 57 (`?? true`)                              | `width="100%"` with `max-width` (default); attributes when false (Q15) |

Not read by radar: `useWidth` and the other `BaseDiagramConfig` keys (`theme` and `look` are read by the shared pipeline only). The top-level `themeVariables` is read by the styles (section 6.9).

### 6.9 Theme variables

The style provider gets `{ ...config.themeVariables, theme, look }` (`mermaidAPI.ts:327-331`). `styles.ts` then reads the default theme's variables (`getThemeVariables()` from `theme-default.js`) merged with `getConfig().themeVariables`, which is the active theme's variables with the user's overrides (`styles.ts:35-37`, and `config.ts:133-137, 171` for how the active theme is computed). The radar section is `themeVariables.radar`, merged with the argument (`styles.ts:38`).

The keys the sheet reads (`styles.ts`):

| Key                          | Read at      | Default theme (`theme-default.js`) | redux-color (probed)                                                                                                               | redux-dark-color (probed)                             |
| ---------------------------- | ------------ | ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| `fontSize`                   | 47           | `'16px'` (33)                      | `'14px'`                                                                                                                           | `'14px'`                                              |
| `titleColor`                 | 48           | `'#333'` (195)                     | `'#000000'`                                                                                                                        | `rgb(222.9999999999, 223.6666666666, 223.9999999999)` |
| `THEME_COLOR_LIMIT`          | 12           | 12 (36)                            | 12                                                                                                                                 | 12                                                    |
| `cScale0` … `cScale11`       | 14           | `hsl(240, 100%, 76.27%)` first     | `#f4a8ff`, `#46ecd5`, `#ffb86a`, `#dab2ff`, `#7bf1a8`, `#c4b4ff`, `#ffa2a2`, `#ffdf20`, `#a3b3ff`, `#bbf451`, `#74d4ff`, `#ffa1ad` | the same twelve                                       |
| `radar.axisColor`            | 38, 53, 58   | `#333333` (= `lineColor`)          | none: fallback to the default, `#333333`                                                                                           | none: fallback, `#333333`                             |
| `radar.axisStrokeWidth`      | 54           | 2                                  | 2 (fallback)                                                                                                                       | 2 (fallback)                                          |
| `radar.axisLabelFontSize`    | 57           | 12                                 | 12 (fallback)                                                                                                                      | 12 (fallback)                                         |
| `radar.curveOpacity`         | 19, 26       | 0.5                                | 0.5 (fallback)                                                                                                                     | 0.5 (fallback)                                        |
| `radar.curveStrokeWidth`     | 21           | 2                                  | 2 (fallback)                                                                                                                       | 2 (fallback)                                          |
| `radar.graticuleColor`       | 61, 63       | `#DEDEDE`                          | `#DEDEDE` (fallback)                                                                                                               | `#DEDEDE` (fallback)                                  |
| `radar.graticuleOpacity`     | 62           | 0.3                                | 0.3 (fallback)                                                                                                                     | 0.3 (fallback)                                        |
| `radar.graticuleStrokeWidth` | 64           | 1                                  | 1 (fallback)                                                                                                                       | 1 (fallback)                                          |
| `radar.legendFontSize`       | 68           | 12                                 | 12 (fallback)                                                                                                                      | 12 (fallback)                                         |
| `radar.legendBoxSize`        | **not read** | 12 (`theme-default.js:348`)        | —                                                                                                                                  | —                                                     |

Facts from the probes:

- **Fallback.** `theme-redux-color.js`, `theme-redux-dark-color.js`, `theme-redux.js`, `theme-redux-dark.js`, `theme-neo.js` and `theme-neo-dark.js` have no `radar` section. Their `radar` is `null` in `getThemeVariables()` (probed for the two redux colour themes), so the default theme's `radar` values are used. Probed under redux-color: `lineColor` is `#000000` and the axis line is `#333333`. Under redux-dark-color, `lineColor` is `#cccccc`, the background is `#333`, and the axis line is still `#333333`, which is close to the background. The last claim is read from the values and not seen on screen (**unverified** visually).
- **Overrides** through `themeVariables.radar` merge onto the fallback (fixture 4: `#FF0000` on the axis and on the label, Chromium).
- **Overrides** of `cScale0` (fixture 5) replace the first curve colour (`.radarCurve-0` is `#FF0000`), and `fontSize` replaces the title's font size, unless it is numeric (section 6.7).
- The `base` theme's `radar` section (`theme-base.js:273-284`) gives `#0b0b0b` for the axis, and it applies to the base theme only. The harness never renders `base` (section 7).

### 6.10 Randomness, clock and DOM

- **Randomness and clock.** None. Grep of `diagrams/radar/` finds no `Math.random` and no `Date`.
- **DOM dependencies a server renderer has to replace.** `selectSvgElement` (the root lookup, section 6.3), the element creation and attribute writes of `d3`, `text` content, and the pipeline's serialisation. No `getBBox`, no `getCTM`, no computed style (section 6.5 for the harness, which is not part of the renderer).
- **Sandbox.** In `securityLevel: 'sandbox'`, `selectSvgElement` looks inside an iframe (`selectSvgElement.ts:15-18`). The port needs no equivalent.

### 6.11 Quirks the renderer reproduces

1. **A maximum equal to the minimum gives `NaN`.** All-zero values with no `max` write `MNaN,NaN CNaN,NaN …` (probed Q1). The same happens for `min 2 max 2`, by the formula (derived).
2. **A curve with the wrong number of entries is skipped, and its legend item is drawn** (Q2, Q8). The other curves keep their own index in the list.
3. **`ticks`:** 40 draws 32 circles (Q3), and the DB clamps 33 the same way (D14); 2.5 draws three circles at 120, 240 and 360, which are beyond the 300 frame (Q4); 0 draws none (Q5).
4. **`showLegend false` removes the legend** and nothing else (Q6).
5. **A header-only diagram** draws the five graticule circles and the title, and no axis and no curve (Q7).
6. **Values are clipped** to `[min, max]`: below the minimum they draw the centre (Q11), above the maximum they draw the frame (Q12).
7. **The graticule polygon** draws curves as polygons, with `points` (Q13, Q20).
8. **Labels are literal text.** Raw HTML in an axis label is shown as typed (Q10). A `#35;` in a label shows `#` (Q9).
9. **The title is double-encoded**: an `&` shows as `&amp;` (section 6.5).
10. **The title element is always there**, empty if there is no title (Q19).
11. **`legendBoxSize` is ignored**: the legend box is 12 by 12 whatever the theme says (Q17, probed with 30).
12. **A numeric `fontSize` is dropped by the browser**, so the title is 16px (computed-probe).
13. **A thirteenth curve is unstyled** (computed-probe).
14. **The fallback of section 6.9**: the redux themes draw the default theme's radar values.
15. **`color` does not paint** (section 6.7).
16. **A width of 0 gives a radius of 0**, and every circle, curve and label is at the centre plus the label offset (Q25).
17. **`useMaxWidth false`** writes the absolute `width` and `height` (Q15).
18. **`curveTension` 0** draws straight segments (Q16). A tension above 1 is accepted and not clamped (DB, D37), so the curve overshoots (derived).
19. **One axis** draws a degenerate closed curve through the one point (Q22), and two axes draw a closed curve through two points (Q23).
20. **A negative `axisScaleFactor`** puts the axis line on the other side of the centre (derived from the formula; not rendered).
21. **Very long curves**: `Math.max(...entries)` throws `RangeError` at about 125,000 entries in Node (probed in Node). The text is capped at `maxTextSize` (default 50,000 characters, `schema:83-86`, `mermaidAPI.ts:48, 495-498`), and a longer text is replaced by the error diagram, so a curve has at most about 25,000 entries at the default. The default configuration cannot reach the limit; a configuration that raises `maxTextSize` can. The port has no such limit, and does not reproduce this one (open question O9).

## 7. Fixtures

The six fixtures, rendered in Chromium through the harness page (`render-out.json`). Their names are as in `e2e/diagrams/radar/`.

| File (under `e2e/diagrams/radar/`)                                   | Features                                                                                                                                                                                                                                                                                                                                                               | Probed viewBox |
| -------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------- |
| `pr-check/should-render-a-simple-radar-diagram.mmd`                  | Title, three axes, one curve `c1{1, 2, 3}`, no configuration                                                                                                                                                                                                                                                                                                           | `0 0 700 700`  |
| `pr-check/should-render-a-radar-diagram-with-multiple-curves.mmd`    | Title, three axes written `A, B, C`, two curves `c1` and `c2` (the legend has two items, at 262.5/-262.5 and -242.5)                                                                                                                                                                                                                                                   | `0 0 700 700`  |
| `pr-check/should-render-radar-diagram-with-config-override.mmd`      | Front matter `config.radar`: `marginTop: 100`, `axisScaleFactor: 0.5`. Title, one curve `mycurve`                                                                                                                                                                                                                                                                      | `0 0 700 750`  |
| `pr-check/should-handle-radar-diagram-with-radar-style-override.mmd` | Front matter `theme: base` (replaced by `redux-color`, see the harness settings below) and `themeVariables.radar.axisColor: '#FF0000'`. Three axes, one curve. No title                                                                                                                                                                                                | `0 0 700 700`  |
| `pr-check/should-parse-radar-diagram-with-theme-override.mmd`        | Front matter `theme: base` (replaced), `themeVariables.fontSize: 80` (numeric), `cScale0: '#FF0000'`. Three axes, one curve. No title                                                                                                                                                                                                                                  | `0 0 700 700`  |
| `nightly-check/complex-radar-diagram.mmd`                            | Title `My favorite ninjas`. Five axes with labels (`Stam["Stamina"]`, `Intel["Intelligence"]`) and comma spacing (`Stam["Stamina"] , Intel`); three curves: a labelled named form (`Ninja1["Naruto Uzumaki"]{ Agility 2, … }`, multi-line), a labelled number form, and an unlabelled number form; `showLegend true`, `ticks 3`, `max 8`, `min 0`, `graticule polygon` | `0 0 700 700`  |

**Harness settings for these fixtures** (`tools/port/geometry/common.ts`). Both sides get `htmlLabels: false`, the font `"Lexica Ultralegible"` and `securityLevel: 'strict'` (`geometry/common.ts:61-65`), and the theme `redux-color` (`geometry/common.ts`: `forcedDirective`, 79-88; `forcedSiteConfig`, 95-104). Fixtures 4 and 5 name `theme: base`, which is not shipped, so the harness replaces it with `redux-color` on both sides. The `base` values (section 6.9) are then not compared, which is the harness's standard fallback and is in line with its comment that colours are not compared (`geometry/common.ts:71-77`).

**What the generic harness must handle for radar.**

- None of the fixtures uses `look: handDrawn`, a non-layered layout, a script the bundled faces do not cover, or randomness. The text is ASCII (`geometry/common.ts:162-183, 215`), so no fixture is excluded, and the P3 rule of named exclusions has nothing to name.
- The harness measures every element below the root, including `path` (17 samples along the path) and every `text` (its box and its content) (`diagram-geometry.ts:145-200`). For radar the paths are the curves, and their samples depend on the Catmull-Rom arithmetic. The text boxes depend on the text engine (section 6.5).
- The `numeric fontSize` of fixture 5 is a real case for the box of the title. The reference's title is 16px, so the port's title must come out 16px too.
- No duplicate DOM ids: the elements carry classes, and the only id is the pipeline's root.

## 8. Risks, effort, open questions

### 8.1 Risks

| Risk                                                         | Level  | Why                                                                                                                                                                                                                                                                   |
| ------------------------------------------------------------ | ------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The lexer's order and prefix rules (section 2.2)             | Medium | Acceptance depends on the first-match order and the word-end rules. Most rules have a probe above; the port needs them all, and a wrong order accepts or rejects a whole class of names                                                                               |
| The theme fallback and the CSS semantics (sections 6.7, 6.9) | Medium | Four rules are easy to miss: the default radar values under the redux themes, `color` that does not paint, the numeric `fontSize` that the browser drops, and the thirteenth curve                                                                                    |
| Number text in attributes (section 6.4)                      | Medium | The reference writes JavaScript's number text, with exponents at 1e-6 and below. The port's writer must match it exactly, or the path strings differ                                                                                                                  |
| The labels' boxes in the harness (section 6.5)               | Medium | The renderer does not measure text, but the harness compares the boxes of the labels, the legend and the title. Those depend on decision 0005's text engine and its three baselines                                                                                   |
| Trigonometry                                                 | Low    | The three angle forms are the same double for every case probed, and the cosines and sines come from `ferrigura_jsmath`. The geometry tolerance is 0.05 px, so a last-bit difference in a cosine is invisible to the harness, and the model holds no derived geometry |
| The title's double encoding (section 6.5)                    | Low    | Easy to reproduce once known; it is visible in the output, so the harness would catch it                                                                                                                                                                              |
| The tooling gap (section 5.3)                                | Low    | No exporter and no radar detector in the shared registration                                                                                                                                                                                                          |

### 8.2 Effort

A rough estimate in Rust lines, not a measurement:

| Part                                                                                                           | Lines      |
| -------------------------------------------------------------------------------------------------------------- | ---------- |
| Lexer: the 28 token types as scanners, the word-end suffix, the hidden tokens (section 2.2)                    | 120 to 160 |
| Parser: the rules of section 2.1, the newline slots, the entries                                               | 120 to 160 |
| Value converters: `STRING`, `NUMBER`, `BOOLEAN`, the title and accessibility regexes                           | 40 to 60   |
| DB: the setters, `computeCurveEntries`, `setOptions` with the clamp, the sanitising, `clear`                   | 100 to 130 |
| Renderer: `draw`, the graticule, the axes, the curves and `closedRoundCurve`, the legend, the title, the root  | 220 to 280 |
| Styles and theme views: the sheet, the twelve curve classes, the fallback and the typed tokens (decision 0009) | 100 to 140 |
| Tests: the 20 specs and the probes of sections 2.5, 3.2, 3.3 and 6.11                                          | 200 to 300 |

The total is 900 to 1,250 lines. Radar has no text measurement and no layout; the kanban estimate, for comparison, is 1,600 to 2,100 lines ([kanban/NOTES.md](../kanban/NOTES.md), section 8.2).

### 8.3 Open questions

- **O1. The theme fallback.** The redux themes have no `radar` section, and the reference uses the default theme's values (section 6.9). Reproduce the fallback as the reference does, so that the typed token for `radar` under redux-color and redux-dark-color is the default's (recommended), or model a per-theme `radar` with a fallback in the typed view (decision 0009)? Under the dark theme the values give a `#333333` axis on a `#333` background (not seen on screen).
- **O2. `legendBoxSize`.** No reference code reads it. The theme sets it (12), and the docs give its default as 10 (`docs/syntax/radar.md:184`). Drop it from the typed view, or keep it as an unused field? The output is the same either way.
- **O3. A numeric `fontSize`.** The reference writes the number without a unit and the browser drops it (section 6.7). The typed token for `fontSize` is a string today. Should the port accept a number and write the same invalid declaration, or reject it at the boundary? Either way the title is 16px in the reference's output.
- **O4. Colours and decision 0007.** Radar's colours are class rules with concrete values, as the reference writes them. Decision 0007 asks for CSS custom properties with each concrete value also as a presentation attribute. Should radar's curves, legend boxes and graticule follow 0007 (so each `path` and `rect` also carries `fill` and `stroke` as attributes), or keep the class rules? The harness does not compare colours, so the choice is for the port's own consistency.
- **O5. The title's double encoding.** Reproduce it (recommended: the harness compares text exactly, and the reference shows `&amp;`), or correct it? Decide before the title's text is written.
- **O6. The `ticks` warning.** The clamp's warning is silent at the default `logLevel`. Does the port have a logger that the warning should reach, or is it dropped with no output?
- **O7. The rejection policy.** The differential counts two rejections as agreement (architecture PLAN, section 4). Radar's rejections have many causes, and the messages differ. Confirm that only acceptance and the model are compared.
- **O8. The exporter and the detector.** `tools/port/reference.ts` `registerDetectors` lacks the radar detector (section 5.3). Add it to the shared registration, or only to the radar exporter?
- **O9. Edge cases not pinned.** Three cases are derived and not probed: `max` below `min` (the formula divides by a negative), a 400-digit number (`Infinity` in the entries, whose point comes out as `NaN`), and the 125,000-entry `RangeError` (section 6.11, item 21), which the default configuration cannot reach. Probe them, or leave them as the port's own behaviour?
- **O10. The harness's fallback of `base`.** Fixtures 4 and 5 name `base`, which the harness replaces with `redux-color`. Accepted as the harness's standard behaviour, so the `base` values are not checked against the reference? (Their colours are not compared in any case.)
- **O11. The canonical model's `max`.** `max` is the one nullable number in the model (section 4). Is JSON `null` acceptable for it, given that the canonical model writes absent header texts as `""`?
