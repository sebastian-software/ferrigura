# Cynefin (P3): research notes for the port

Status: research only. Nothing in the repository changed except this file, and nothing is committed. Reference: `packages/mermaid/src` and `packages/parser/src` at HEAD `f3a1a94b0` (branch `sebastian/port-p3`). Plan: [../P3-PLAN.md](../P3-PLAN.md). Guide: [../P2-RESEARCH.md](../P2-RESEARCH.md). Model of finished notes: [../kanban/NOTES.md](../kanban/NOTES.md).

Every rule cites the reference as `diagrams/cynefin/<file>:<line>` unless the file is named in full. **Probed** means the rule was run through the reference's own code. The Langium lexer and parser ran from `packages/parser/src` under tsx (`createCynefinServices`). The parser and DB ran through `tools/port/reference.ts` (Vite SSR), with `preprocessDiagram` and `encodeEntities` applied as `Diagram.fromText` applies them. The theme blocks ran through the theme modules. The rendering ran in Chromium through the harness's own page (`withPage`, `renderReference` and `patchText` from `tools/port/geometry/common.ts`) on the bundle `packages/mermaid/dist`, with the harness's forced settings unless a row says otherwise. **Unverified** means read or inferred only.

The probe material is outside the repository in `/private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/p3/cynefin/`: `lexer-order.mts`; `parse-corpus.mts` with `corpus1.json` to `corpus3.json`; `db-probe.mts` with `db-corpus1.json` to `db-corpus5.json`; `theme-probe.mts`; `detect-probe.mts`; `chrome-probe.mts` with its `chrome-inputs*.json` and `chrome-results*.json`, and the SVGs in `svg/`; `seed_check.py`, which transcribes the boundary code and reproduces the probed paths; and `cmap.py`, which reads the font tables of the bundled faces.

## 0. Summary

Cynefin is the smallest Langium type left. Its grammar has one header, domain blocks with string items, and transitions. Its DB holds four pieces of state. Its renderer places fixed shapes by arithmetic, and it measures one thing: the width of each item badge. The only randomness is the seed of the boundary waves, which is a hash of the SVG id unless the configuration sets a seed.

| Item                | Value                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Reference, non-spec | `cynefinRenderer.ts` (436 lines), `cynefinBoundaries.ts` (169), `cynefinDb.ts` (96), `styles.ts` (95), `types.ts` (34), `cynefinParser.ts` (20), `cynefinDetector.ts` (22), `cynefinDiagram.ts` (12): 884 lines in total. The grammar `packages/parser/src/language/cynefin/cynefin.langium` is 30 more                                                                                                                                                                                      |
| Shared code it uses | Preprocessing and `encodeEntities` (`Diagram.ts`), `sanitizeText` (`diagrams/common/common.ts`), `populateCommonDb`, the Langium token builder and value converter (`packages/parser/src/language/common/`), the theme modules and `cleanAndMerge`                                                                                                                                                                                                                                           |
| Specs               | `cynefin.spec.ts` (191 lines, 27 tests) and `cynefin.integration.spec.ts` (275 lines, 15 tests). There is no renderer and no style spec                                                                                                                                                                                                                                                                                                                                                      |
| Fixtures            | 12 `.mmd` files: 10 in `pr-check/`, 2 in `nightly-check/`                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| Parser              | Langium 4.4.0, one grammar, 14 Chevrotain token types. A hand port, as decision [0002](../decisions/0002-hand-written-parsers.md) says: a scanner that runs the token table in its written order, and recursive descent for the rules                                                                                                                                                                                                                                                        |
| DB                  | Module state: a map of domains, a list of transitions, and the common title and accessibility texts. Labels are stored as written. The title and accessibility texts go through `sanitizeText`                                                                                                                                                                                                                                                                                               |
| Renderer            | Fixed arithmetic for every position, one `getBBox` per item badge, and cubic and quadratic paths. No layout engine                                                                                                                                                                                                                                                                                                                                                                           |
| Configuration       | Six keys in `cynefin`, and `useMaxWidth` from the base section. The theme gives five background colours and ten style keys (section 6.8)                                                                                                                                                                                                                                                                                                                                                     |
| Riskiest parts      | (1) The subtitles contain `→` (U+2192), which the bundled Lexica faces do not have, so 11 of the 12 fixtures draw a glyph the port cannot match. (2) Titles go through DOMPurify, so entities and markup show as text. (3) The redux themes, which ferrigura ships, use the default theme's colour block. (4) A seed of 0 is a hash of the SVG id, so the `--self` pass of the harness reports differences. (5) The harness compares the box of every text element, not only the item badges |
| Effort              | Medium. About 1,000 to 1,300 lines of Rust without tests and without a sanitiser, 400 to 500 lines of tests, and 0 to 400 more lines for a DOMPurify-equivalent sanitiser if ferrigura has none (section 8.2)                                                                                                                                                                                                                                                                                |

No exporter exists yet (`tools/port/export-cynefin.ts` is absent), so `pnpm port:diff cynefin` needs the scaffold first. `tools/port/diff-config.ts:265` lists `cynefin` in `UNPORTED_SECTIONS`, and that entry comes out when the configuration is ported.

## 1. Sources

| File                                                                                                           | Lines                                    | What it holds                                                                                                                                                                                                      |
| -------------------------------------------------------------------------------------------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `packages/parser/src/language/cynefin/cynefin.langium`                                                         | 2                                        | `import "../common/common"`, so the common terminals come after the grammar's own                                                                                                                                  |
|                                                                                                                | 4-15                                     | `entry Cynefin`: the header (7), then the body loop (9-14)                                                                                                                                                         |
|                                                                                                                | 17-20                                    | `DomainBlock`: `DOMAIN_NAME`, then any number of items and newlines                                                                                                                                                |
|                                                                                                                | 22-24                                    | `DomainItem`: a `STRING` label                                                                                                                                                                                     |
|                                                                                                                | 26-28                                    | `Transition`: `DOMAIN_NAME '-->' DOMAIN_NAME`, an optional `: STRING` label, then `EOL`                                                                                                                            |
|                                                                                                                | 30                                       | `terminal DOMAIN_NAME`, the five names, with no word end                                                                                                                                                           |
| `packages/parser/src/language/cynefin/module.ts`                                                               | 30-38, 40-55                             | `CynefinModule` (token builder and value converter), `createCynefinServices`                                                                                                                                       |
| `packages/parser/src/language/cynefin/tokenBuilder.ts`                                                         | 3-6                                      | the keyword list `['cynefin-beta']`                                                                                                                                                                                |
| `packages/parser/src/language/common/tokenBuilder.ts`                                                          | 6-28                                     | the word-end guard that the keyword list selects (line 24)                                                                                                                                                         |
| `packages/parser/src/language/common/valueConverter.ts`                                                        | 12-71, 73-81                             | the common conversion for `ACC_DESCR`, `ACC_TITLE` and `TITLE`, and the empty subclass                                                                                                                             |
| `packages/parser/src/language/common/common.langium`                                                           | 6-34                                     | `EOL` (6), `TitleAndAccessibilities` (8-10), `ACC_DESCR` (14), `ACC_TITLE` (15), `TITLE` (16), `STRING` (22), `NEWLINE` (29), `WHITESPACE` (31, hidden), `YAML` (32), `DIRECTIVE` (33), `SINGLE_LINE_COMMENT` (34) |
| `packages/parser/src/language/common/matcher.ts`                                                               | 4, 9, 14                                 | the regexes of accessible description, accessible title and title                                                                                                                                                  |
| `packages/parser/src/parse.ts`                                                                                 | 18, 110-114, 131                         | the import, the `cynefin` initialiser of the shared parse entry, and its overload                                                                                                                                  |
| `diagrams/cynefin/cynefinParser.ts`                                                                            | 8-12, 14-20                              | `populate` (the order of the DB calls, section 3.1), and `parser.parse`                                                                                                                                            |
| `diagrams/cynefin/cynefinDetector.ts`                                                                          | 9-11                                     | the detection regex (line 10)                                                                                                                                                                                      |
| `diagrams/cynefin/cynefinDiagram.ts`                                                                           | 7-12                                     | the definition: parser, db, renderer and styles                                                                                                                                                                    |
| `diagrams/cynefin/cynefinDb.ts`                                                                                | 18-26, 28-32                             | the data shape, `getDomains` and `getTransitions`                                                                                                                                                                  |
|                                                                                                                | 34-48, 50-69                             | `setDomains` (the map), `setTransitions` (the self-loop filter at 55-63)                                                                                                                                           |
|                                                                                                                | 71-76, 78-81, 83-96                      | `getConfig`, `clear`, the `db` object                                                                                                                                                                              |
| `diagrams/cynefin/cynefinRenderer.ts`                                                                          | 1-16, 18-29, 31-50, 52-66, 68-69         | imports, `DOMAIN_META`, the domain layouts, the colour lookup, `MAX_CONFUSION_ITEMS`                                                                                                                               |
|                                                                                                                | 71-434                                   | `draw`, in sections 6.2 to 6.6                                                                                                                                                                                     |
|                                                                                                                | 436                                      | `renderer`                                                                                                                                                                                                         |
| `diagrams/cynefin/cynefinBoundaries.ts`                                                                        | 5-10, 15-23, 32-41                       | `seededRandom`, `hashString`, `resolveSeed`                                                                                                                                                                        |
|                                                                                                                | 52-84, 94-126, 135-151, 161-169          | the fold wave, the horizontal wave, the cliff, the confusion ellipse                                                                                                                                               |
| `diagrams/cynefin/styles.ts`                                                                                   | 11-27, 29-93                             | `getCynefinTheme` (the keys it reads), `styles` (the rules)                                                                                                                                                        |
| `diagrams/cynefin/types.ts`                                                                                    | 4-34                                     | `DomainName`, the item and transition shapes, the `CynefinDB` interface                                                                                                                                            |
| `diagrams/cynefin/cynefin.spec.ts`                                                                             | 1-191                                    | 27 tests (section 5)                                                                                                                                                                                               |
| `diagrams/cynefin/cynefin.integration.spec.ts`                                                                 | 1-275                                    | 15 tests (section 5)                                                                                                                                                                                               |
| `rendering-util/multi-diagram-id-uniqueness.spec.ts`                                                           | 201                                      | a cynefin sample with a title and four domains and no transitions                                                                                                                                                  |
| `diagrams/common/common.ts`                                                                                    | 56-69, 71-83                             | `sanitizeMore` and `sanitizeText` (the title sanitiser, section 6.12)                                                                                                                                              |
| `diagrams/common/commonDb.ts`                                                                                  | 10-14, 16-29                             | `clear` and the setters: `setAccTitle` strips leading blanks (17), `setAccDescription` collapses `\n\s+` (23)                                                                                                      |
| `diagrams/common/populateCommonDb.ts`                                                                          | 4-14                                     | the common title and accessibility calls, each only when the value is truthy                                                                                                                                       |
| `Diagram.ts`                                                                                                   | 49, 52-53, 77                            | `db.clear`, the front-matter title, and `encodeEntities(source) + '\n'`                                                                                                                                            |
| `preprocess.ts`                                                                                                | 33, 58-62                                | `processDirectives` and `preprocessDiagram` (front matter is 58-91, as the kanban notes describe)                                                                                                                  |
| `diagram-api/comments.ts`                                                                                      | 6-8                                      | `cleanupComments`: whole-line `%%` comments                                                                                                                                                                        |
| `utils.ts`                                                                                                     | 734-743, 841-843, 874-895, 904-906       | `InitIDGenerator`, `cleanAndMerge` (lodash `merge`), `encodeEntities`, `decodeEntities`                                                                                                                            |
| `mermaidAPI.ts`                                                                                                | 319-334, 596-601, 606-608, 623, 359      | `createUserStyles`; the style element, inserted before the draw; the draw; `addA11yInfo`; `decodeEntities` of the whole SVG string                                                                                 |
| `accessibility.ts`                                                                                             | 45-66                                    | `addSVGa11yTitleDescription`, which inserts `title` and `desc` as the first children                                                                                                                               |
| `mermaid.ts`                                                                                                   | 159, 174                                 | the id generator of `run`: `mermaid-${idGenerator.next()}`                                                                                                                                                         |
| `setupGraphViewbox.js`                                                                                         | 23-33, 43-47                             | `calculateSvgSizeAttrs` and `configureSvgSize`. `setupGraphViewbox` (49) is not called by cynefin                                                                                                                  |
| `config.ts`                                                                                                    | 133-137, 171-172, 324-326                | the resolved `themeVariables` of the active theme, and `getEffectiveHtmlLabels` (`htmlLabels`, default true)                                                                                                       |
| `defaultConfig.ts`                                                                                             | 9, 338-340                               | the defaults come from the schema (`?only-defaults=true`), and the `cynefin` section                                                                                                                               |
| `schemas/config.schema.yaml`                                                                                   | 432-433, 457-470, 3063-3102              | the reference, `BaseDiagramConfig` (with `useMaxWidth`, default at 470), and `CynefinDiagramConfig`                                                                                                                |
| `config.type.ts`                                                                                               | 348, 2500-2531                           | the `cynefin` field and the `CynefinDiagramConfig` interface                                                                                                                                                       |
| `themes/theme-default.js`                                                                                      | 294-310, 503-507                         | the cynefin block (the keys at 296-310) and `getThemeVariables`                                                                                                                                                    |
| `themes/theme-base.js` 254-271, `theme-dark.js` 265-282, `theme-forest.js` 257-274, `theme-neutral.js` 282-299 |                                          | the other cynefin blocks                                                                                                                                                                                           |
| `themes/theme-redux*.js`, `theme-neo*.js`                                                                      |                                          | no cynefin block (grep of the theme folder)                                                                                                                                                                        |
| `diagram-api/diagram-orchestration.ts`                                                                         | 35, 120                                  | the import and the registration of the detector                                                                                                                                                                    |
| `tools/port/geometry/common.ts`                                                                                | 61-68, 95-104, 115-127, 682-713, 720-745 | the forced settings, `forcedSiteConfig`, `patchText`, `withPage`, `renderReference`                                                                                                                                |
| `tools/port/geometry/diagram-geometry.ts`                                                                      | 351, 369, 391                            | the reference id `ref-<slug>`, and the self pass `self-<slug>`                                                                                                                                                     |
| `e2e/helpers/util.ts`                                                                                          | 44-46                                    | the e2e viewer sets `cynefin.seed` to 1 when the fixture does not                                                                                                                                                  |
| `e2e/diagrams/cynefin/`                                                                                        |                                          | the 12 fixtures (section 7)                                                                                                                                                                                        |

## 2. Parser

### 2.1 The grammar

```
Cynefin      : NEWLINE* ('cynefin-beta' | 'cynefin-beta:') NEWLINE*
               ( TitleAndAccessibilities | DomainBlock | Transition | NEWLINE )*
DomainBlock  : DOMAIN_NAME NEWLINE* ( DomainItem NEWLINE* )*
DomainItem   : STRING
Transition   : DOMAIN_NAME '-->' DOMAIN_NAME ( ':' STRING )? EOL
TitleAndAccessibilities : ( ( ACC_DESCR | ACC_TITLE | TITLE ) EOL )+
EOL          : NEWLINE+ | EOF
```

The grammar is `cynefin.langium:4-28` and `common.langium:6-9`. The DB calls come from `cynefinParser.ts:8-12`, not from actions in the grammar, so the grammar has no semantic code of its own. The only decision that needs lookahead is `DOMAIN_NAME` (a block or a transition): a transition has `-->` as its second token (probed: a block followed by a transition on the next line parses as two statements).

### 2.2 Lexer: token order

The reference's lexer is Chevrotain, built by Langium's `DefaultTokenBuilder.buildTokens`. The token order below is the order of `lexerDefinition` in the Chevrotain lexer (probed: `lexer-order.mts`, which reads `services.Cynefin.parser.Lexer.chevrotainLexer.lexerDefinition`). Chevrotain takes the first match in this order, not the longest. The keywords come first and are sorted by descending length (Langium 4.4.0, `lib/parser/token-builder.js:84`; probed by the order of the first four rows).

| #   | Token                 | Pattern                                                              | Source                   | Effect                                                                                                                                                 |
| --- | --------------------- | -------------------------------------------------------------------- | ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 0   | `cynefin-beta:`       | the keyword, no word end                                             | `tokenBuilder.ts:5`      | header in the colon form. The end guard does not apply, because the guard only applies to tokens named `cynefin-beta` (`common/tokenBuilder.ts:22-24`) |
| 1   | `cynefin-beta`        | `cynefin-beta(?:(?=%%)\|(?!\S))`                                     | the same, with the guard | header. The word must end at `%%`, or at a blank or the end of the text                                                                                |
| 2   | `-->`                 | the keyword                                                          | `cynefin.langium:27`     | the arrow of a transition                                                                                                                              |
| 3   | `:`                   | the keyword                                                          | `cynefin.langium:27`     | introduces a transition label                                                                                                                          |
| 4   | `DOMAIN_NAME`         | `complex\|complicated\|clear\|chaotic\|confusion`                    | `cynefin.langium:30`     | a domain. No word end, so the name is taken as a prefix (section 2.6, item 1)                                                                          |
| 5   | `ACC_DESCR`           | `[\t ]*accDescr(?:[\t ]*:([^\n\r]*?(?=%%)\|[^\n\r]*)\|\s*{([^}]*)})` | `common.langium:14`      | accessible description, one line or a block. Takes leading blanks                                                                                      |
| 6   | `ACC_TITLE`           | `[\t ]*accTitle[\t ]*:(?:[^\n\r]*?(?=%%)\|[^\n\r]*)`                 | `common.langium:15`      | accessible title. The colon is required                                                                                                                |
| 7   | `TITLE`               | `[\t ]*title(?:[\t ][^\n\r]*?(?=%%)\|[\t ][^\n\r]*\|)`               | `common.langium:16`      | title. Takes leading blanks, so a title can start after a domain on the same line (section 2.6, item 5)                                                |
| 8   | `STRING`              | `"([^"\\]\|\\.)*"\|'([^'\\]\|\\.)*'`                                 | `common.langium:22`      | an item or a transition label. May span lines                                                                                                          |
| 9   | `NEWLINE`             | `\r?\n`                                                              | `common.langium:29`      | line end                                                                                                                                               |
| 10  | `WHITESPACE`          | `[\t ]+`                                                             | `common.langium:31`      | skipped                                                                                                                                                |
| 11  | `YAML`                | `---[\t ]*\r?\n(?:[\S\s]*?\r?\n)?---(?:\r?\n\|(?!\S))`               | `common.langium:32`      | hidden                                                                                                                                                 |
| 12  | `DIRECTIVE`           | `[\t ]*%%{[\S\s]*?}%%(?:\r?\n\|(?!\S))`                              | `common.langium:33`      | hidden                                                                                                                                                 |
| 13  | `SINGLE_LINE_COMMENT` | `[\t ]*%%[^\n\r]*`                                                   | `common.langium:34`      | hidden                                                                                                                                                 |

Notes on the table:

- The common terminals come after the grammar's own, because the import is considered after the importing grammar (`common.langium:2-4`). `DOMAIN_NAME` is therefore before the terminals of `common`.
- No token has a `LONGER_ALT`. The keyword `cynefin-beta` has its guard in the pattern itself.
- A lexical error is reported at the first character that no token takes. Chevrotain skips it and goes on, and the wrapper then throws (`packages/parser/src/parse.ts:146-148`).
- `ID` is not a token of this grammar. Every word that is not a keyword, a domain or a title is a lexical error.

### 2.3 Value converter

Two converters act in order (probed through the tokens of the corpus, section 2.5):

1. `CommonValueConverter` (`common/valueConverter.ts:27-42`) first runs `runCommonConverter` for the rules named `ACC_DESCR`, `ACC_TITLE` and `TITLE` (lines 44-70), which apply the regexes of `common/matcher.ts`. The single-line value is trimmed, and each run of two or more blanks becomes one blank (line 59). A block `accDescr { … }` takes each line trimmed, drops trailing blanks, collapses blank runs, and reduces two or more newlines to one (lines 63-67). A title with no text gives the empty string, and the DB then keeps it as nothing (`populateCommonDb.ts:11`).
2. Otherwise Langium's `DefaultValueConverter` runs (Langium 4.4.0, `lib/parser/value-converter.js:24-37`). `STRING` goes to `convertString`, which strips the quotes and maps the escapes `\b \f \n \r \t \v \0` to the control characters. Any other `\c` gives `c` (lines 41-60). `DOMAIN_NAME` returns its text unchanged, because its rule type is `string` (line 35).

### 2.4 Preprocessing and detection

`Diagram.fromText` (`Diagram.ts:49-77`) runs the shared steps: the front matter and the directives (`preprocess.ts:58-62`), `cleanupComments` (`comments.ts:6-8`), and `encodeEntities(cleaned) + '\n'` (`Diagram.ts:77`). The cynefin DB does not set `preserveCommentsWhenParsing`, so the parser gets the comment-stripped text. The kanban notes (section 2.3) describe the shared steps in full.

`encodeEntities` (`utils.ts:874-895`) turns `#name;` into `ﬂ°name¶ß` and `#123;` into `ﬂ°°123¶ß`, before the parser sees the text. The decoding happens on the finished SVG string (`mermaidAPI.ts:359`, section 6.12).

Detection (`cynefinDetector.ts:10`) is `/^\s*cynefin-beta(?:[\s:]|$)/` on the text after the directives are read (probed: `detect-probe.mts`). It accepts `cynefin-beta`, `cynefin-beta:`, `cynefin-beta x`, `cynefin-beta:x`, `cynefin-beta<TAB>`, and a leading blank line or tab. It rejects `cynefin-betax`, `cynefin-beta-x`, `cynefin-beta.`, `cynefin-beta%%x`, `CYNEFIN-BETA` and `xcynefin-beta`. A rejection is `UnknownDiagramError` from `detectType` (probed).

### 2.5 Probed behaviour

Each row is an input to the raw Langium parser (`LangiumParser.parse`, probed: `parse-corpus.mts`) unless it says DB. "Error" means a lexer or a parser error, and the wrapper would throw it. The DB rows went through `Diagram.fromText`'s steps (`db-probe.mts`).

| Input                                                                                          | Result                                                                                  |
| ---------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| `cynefin-beta` alone, with or without a newline                                                | Accepted, with no domains                                                               |
| `cynefin-beta:` then `complex "A"`                                                             | Accepted, the same as the bare form                                                     |
| `cynefin-beta complex "A"` (one line)                                                          | Accepted: header and first block on one line                                            |
| `complex "A" "B"`                                                                              | Two items, on one line                                                                  |
| `complex "A" complicated --> clear`                                                            | Accepted: a block, a block and a transition on one line                                 |
| `complex ""`                                                                                   | Item `""`                                                                               |
| `Complex`                                                                                      | Error at the first letter: domain names are case-sensitive                              |
| `complexity`, `clearly`, `complex-`                                                            | Error at the first character after the name                                             |
| `simple` (an unknown word)                                                                     | Error at its first letter                                                               |
| `cynefin-betaX`                                                                                | Error at `c`, and a parse error. The detector rejects it too                            |
| `cynefin-beta:x`                                                                               | Error at `x`. The detector accepts it                                                   |
| `cynefin-beta%%note` (the raw parser)                                                          | Accepted. The detector rejects it, so `mermaid.render` never reaches the parser with it |
| `cynefin-beta` twice                                                                           | Parse error at the second header                                                        |
| `"A"` before any domain                                                                        | Parse error                                                                             |
| `complex`, `"A"`, `title T`, `"B"`                                                             | Parse error at `"B"` (line 5, column 1). An item cannot follow a title line             |
| `complex --> clear : "x"`, then `"A"`                                                          | Parse error at `"A"`. An item cannot follow a transition                                |
| `complex --> complicated : "x"`, `: 'x'`, `:"x"` (the colon with or without a blank before it) | Accepted with label `x`                                                                 |
| `complex --> clear : x` (unquoted)                                                             | Error at `x`, and parse errors                                                          |
| `complex --> clear "x"` (no colon)                                                             | Parse error                                                                             |
| `complex --> clear : "x" "y"`                                                                  | Parse error                                                                             |
| `complex --> clear --> chaotic`                                                                | Parse error                                                                             |
| `complex --> complicated : ""`                                                                 | Transition with label `""`. The DB stores `undefined` (section 3.2)                     |
| `complex --> foo`                                                                              | Error at `f`: `foo` is not a domain                                                     |
| A transition to a domain with no block (`complex --> clear`, `clear` never declared)           | Accepted, and the arrow is drawn (probed in Chromium)                                   |
| `complex title x` (after a domain)                                                             | Domain `complex` and title `x`                                                          |
| `complex complicated` (one line, no arrow)                                                     | Two blocks                                                                              |
| `title` alone                                                                                  | Title `""`                                                                              |
| `title   Spaces   here   `                                                                     | Title `Spaces here` (trimmed, blank runs collapsed)                                     |
| `title a %% comment`                                                                           | Title `a`                                                                               |
| Two `title` lines                                                                              | The last one wins: `Second`                                                             |
| `accTitle:` with nothing                                                                       | Accessible title `""`                                                                   |
| `accDescr {`, then `  line one`, `    line two`, a blank line, `  line three`, then `}`        | `line one\nline two\nline three` (lines trimmed, blank line gone)                       |
| `accDescr: A map` (one line)                                                                   | Accessible description `A map`                                                          |
| `accTitle x` (no colon)                                                                        | Error at `a`, and at `x`                                                                |
| `titleX`                                                                                       | Error at `X`                                                                            |
| `"a\"b\\c\td"`                                                                                 | `a"b\c` then a tab then `d`                                                             |
| `"a\nb"` (escape) and a raw newline inside quotes                                              | A line break in both cases                                                              |
| `'it\'s'`, `'say "hi"'`                                                                        | `it's`, `say "hi"`                                                                      |
| `"a\qb\u0041c\x41d"`                                                                           | `aqbu0041cx41d`: unknown escapes give the letter, and `u` and `x` are kept              |
| `"abc` (unterminated)                                                                          | Error at the quote                                                                      |
| `"Ü ζ → x"`                                                                                    | Accepted, the text kept                                                                 |
| CRLF line ends, tabs, and leading blanks before the header                                     | Accepted                                                                                |
| `#amp;` in an item (DB)                                                                        | Stored as `a ﬂ°amp¶ß b`. The SVG shows `&` (section 6.12)                               |
| `title A #amp; B` (DB)                                                                         | Stored as `A ﬂ°amp¶ß B`                                                                 |
| `title` with a front-matter title, and an empty body `title`                                   | Front-matter title kept (`populateCommonDb.ts:11` tests truthiness)                     |
| front-matter title, then a body `title Body`                                                   | `Body` wins                                                                             |
| `complex` then a blank line, two blanks, then `"A"`                                            | Accepted: blank lines and a comment line may come between a domain and its items        |

### 2.6 Quirks to reproduce

1. **Domain names have no word end.** `DOMAIN_NAME` (line 30) matches as a prefix, so `complexity` lexes as `complex` followed by an error at `i`. Names are case-sensitive (`Complex` is an error). The port must match the prefix, and then fail on the rest.
2. **Two spellings of the header, and two word ends.** `cynefin-beta` has the guard `(?:(?=%%)|(?!\S))`. `cynefin-beta:` has no guard, so `cynefin-beta:x` is a lexical error at `x` (probed). The detector is a different regex: `[\s:]|$` (`cynefinDetector.ts:10`), so `cynefin-beta%%note` is parsed by the parser but never detected.
3. **Items follow their domain.** A `STRING` after a title or a transition is an error (probed). Blank lines and comment lines between a domain and its items are accepted (probed).
4. **Domains and transitions share lines.** `complex "A" complicated --> clear` is accepted (probed). The DB then holds the three domains in the order they first appear.
5. **A title can start after a domain on the same line.** `complex title x` sets the title to `x`, because `TITLE` takes leading blanks (probed).
6. **Strings keep escapes in an odd way.** `\n`, `\r`, `\b`, `\f`, `\t`, `\v` and `\0` become control characters, and any other `\c` becomes `c`, so `\u0041` is `u0041` (probed; Langium 4.4.0 `lib/parser/value-converter.js:41-60`).
7. **Transition labels can be empty.** `""` is stored as `undefined`, because `t.label || undefined` (`cynefinDb.ts:67`).
8. **The title of the diagram is the last one.** Two `title` lines give the last, and an empty `title` does not replace a front-matter title (section 3.2).

## 3. DB

### 3.1 Calls in order

| Step | Call                                                                            | Effect                                                                                                                                                                                |
| ---- | ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1    | `db.clear()` (`Diagram.ts:49`, `cynefinDb.ts:78-81`)                            | `commonClear` resets the title, the accessible title and the accessible description to `''` (`commonDb.ts:10-14`). The cynefin `data` is replaced with an empty map and an empty list |
| 2    | `setDiagramTitle(metadata.title)` (`Diagram.ts:52-53`)                          | Only when the front matter has a title. Sanitised by the setter                                                                                                                       |
| 3    | `parser.parse(text)` (`cynefinParser.ts:15-19`)                                 | The parse, then `populate` (lines 8-12)                                                                                                                                               |
| 3a   | `populateCommonDb` (`cynefinParser.ts:9`, `populateCommonDb.ts:4-14`)           | `accDescr` (5-7), then `accTitle` (8-10), then `title` (11-13). Each only when truthy. Each goes through its setter, which sanitises                                                  |
| 3b   | `setDomains(ast.domains)` (`cynefinParser.ts:10`, `cynefinDb.ts:34-48`)         | One entry per block                                                                                                                                                                   |
| 3c   | `setTransitions(ast.transitions)` (`cynefinParser.ts:11`, `cynefinDb.ts:50-69`) | Self-loops dropped, the rest copied                                                                                                                                                   |
| 4    | `draw` reads (`cynefinRenderer.ts:72-78`)                                       | `getDomains`, `getTransitions`, `getDiagramTitle`, `getAccTitle`, `getAccDescription`, `getConfig`                                                                                    |

### 3.2 State and each setter

The state is `{ domains: Map<DomainName, { name, items: [{ label }] }>, transitions: [{ from, to, label? }] }` (`cynefinDb.ts:18-26`). It is module state: one `let data` (line 28), replaced by `clear`.

- `setDomains` (`cynefinDb.ts:34-48`): `if (!blocks) return` (35-37). For each block, `items` is a fresh list of `{ label }` (40-42), and `domains.set(name, …)` (43-46). A second block with the same name replaces the items of the first and keeps its place in the map (probed: `complex "A"` then `complex "B"` gives `["B"]`; a second `complex` with no items gives `[]`). Labels are not sanitised here, and not decoded.
- `setTransitions` (`cynefinDb.ts:50-69`): the self-loops are filtered, and each one gives a warning (`log.warn`, lines 55-63). `label: t.label || undefined` (67). The order of declaration is kept.
- `setAccTitle` (`commonDb.ts:16-18`) sanitises and strips leading blanks. `setAccDescription` (22-24) sanitises and turns each `\n` plus blanks into `\n`. `setDiagramTitle` (28-30) sanitises.

### 3.3 Configuration

`getConfig` (`cynefinDb.ts:71-76`) returns `cleanAndMerge({ ...DEFAULT_CONFIG.cynefin, ...commonGetConfig().cynefin })`. The defaults are in the schema (section 6.9). Probed defaults: `{"useMaxWidth":true,"width":800,"height":600,"padding":40,"showDomainDescriptions":true,"boundaryAmplitude":8,"seed":0}`. A directive or front-matter value replaces the default, and an unknown key is not in the result (probed with `colour`). A non-number is not rejected: `"width":"wide"` comes out unchanged (probed).

### 3.4 Quirks of the DB

1. **Duplicate domain blocks overwrite.** The second block replaces the items of the first, and the map keeps the first position (`cynefinDb.ts:43-46`, probed).
2. **Self-loops are dropped at the DB,** with a warning, so the renderer's own guard (`cynefinRenderer.ts:378-382`) is never reached.
3. **Titles and accessible texts are sanitised; labels are not.** `sanitizeText` (`common.ts:71-83`) runs on the title, the accessible title and the accessible description (section 6.12). Items and transition labels are stored as they were parsed, with their placeholders.
4. **The title precedence.** The front-matter title is set before the parse (`Diagram.ts:52-53`). A body `title` with text replaces it. An empty body title does not (probed).
5. **Declarations that name no block still count for transitions.** The DB only adds the domains of blocks, but the renderer has a position for all five names (`cynefinRenderer.ts:40-50`), so an arrow to an undeclared domain is drawn (probed).

## 4. Canonical model proposal

Following [canonical-model.md](../canonical-model.md): a version field, and camelCase keys.

```text
CynefinDiagram { version: 1, title, accTitle, accDescr, domains: [Domain], transitions: [Transition] }
Domain         { name, items: [string] }
Transition     { from, to, label: string | null }
```

| Field                           | Rule                                                                                                                                                                                                                                |
| ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `title`, `accTitle`, `accDescr` | The DB value after `sanitizeText` and the setter's own steps. `''` when unset. These are the only values that carry sanitised markup, so the port must match DOMPurify's output on them (section 6.12)                              |
| `domains`                       | Every entry of the DB map, in the map's order (first appearance of the name). A name appears once. The items are the labels as stored, with their placeholders (`ﬂ°amp¶ß`), because the reference's DB holds them that way (probed) |
| `items`                         | Strings, in declaration order. An empty label is `""`                                                                                                                                                                               |
| `transitions`                   | The DB list after the self-loop filter, in declaration order                                                                                                                                                                        |
| `label`                         | The DB value: `null` for `undefined`, which covers both an absent label and `""` (section 3.2)                                                                                                                                      |

Left out, and why:

- **The configuration.** It is an input of the renderer, not part of the parse. It is checked through the geometry harness and through `port:config` (once the section is ported), and the DB model would otherwise depend on the defaults.
- **`DOMAIN_META`, the `MAX_CONFUSION_ITEMS` cap, and the positions.** These are renderer constants and arithmetic. The geometry harness covers them.
- **The `Map` key.** It always equals `name`.
- **The log warnings.** They are not output.

A `getData`-style model is not needed. The renderer reads the DB directly (`cynefinRenderer.ts:72-78`).

## 5. Specs

| File                          | Lines   | Tests | What they test                                                                                                                                                                                                                                            |
| ----------------------------- | ------- | ----- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `cynefin.spec.ts`             | 22-103  | 11    | the DB: empty start, domains from blocks, transitions with and without label, self-loop filtering, a list of only self-loops, `null` blocks, `clear`, five domains, items, `getConfig` returns an object                                                  |
| `cynefin.spec.ts`             | 105-163 | 10    | the boundary functions: `seededRandom` is deterministic and in [0, 1), `hashString` is consistent and differs for different strings, the fold, horizontal and cliff paths are valid strings and deterministic, the ellipse path uses the centre and radii |
| `cynefin.spec.ts`             | 165-191 | 6     | `resolveSeed`: a non-zero seed is used, zero and `undefined` fall back to `hashString(id)`, an explicit seed ignores the id, different ids give different fallbacks, `NaN` and `Infinity` fall back                                                       |
| `cynefin.integration.spec.ts` | 5-79    | 5     | the parser (through `parser.parse`, which is the real parser): basic domains, five domains, empty domains, items, comments                                                                                                                                |
| `cynefin.integration.spec.ts` | 81-132  | 3     | transitions with and without a label, several transitions                                                                                                                                                                                                 |
| `cynefin.integration.spec.ts` | 134-163 | 3     | `accTitle`, `accDescr`, `title`                                                                                                                                                                                                                           |
| `cynefin.integration.spec.ts` | 165-213 | 1     | a full diagram with all features                                                                                                                                                                                                                          |
| `cynefin.integration.spec.ts` | 215-243 | 2     | self-loops dropped, all self-loops give none                                                                                                                                                                                                              |
| `cynefin.integration.spec.ts` | 245-275 | 1     | a second parse after `clear` starts fresh                                                                                                                                                                                                                 |

The total is 42 tests: 27 in `cynefin.spec.ts` and 15 in `cynefin.integration.spec.ts`.

Not covered by any spec: the escapes of strings, the parser's errors, the placeholders and entity decoding, the sanitiser, the renderer (all of it), the styles, the confusion overflow, the transitions' geometry, the seed through the renderer, `useMaxWidth`, and the front-matter title precedence. The integration specs use the reference's parser through `@mermaid-js/parser`, so they test the reference's behaviour, not the port's.

## 6. Renderer

### 6.1 Pipeline

The generic steps are those of the kanban and packet notes. The specific facts for cynefin:

- The style element is created from `createUserStyles` and inserted as the first child of the svg before `draw` (`mermaidAPI.ts:596-601`, probed: `style` is the first child).
- `draw` is awaited (`mermaidAPI.ts:606-608`), and the cynefin draw does no asynchronous work.
- `addA11yInfo` runs after the draw (`mermaidAPI.ts:623`). It sets `aria-labelledby` and `aria-describedby` and inserts `title` and `desc` at the start (`accessibility.ts:45-66`).
- The whole SVG string goes through `decodeEntities` (`mermaidAPI.ts:359`).
- The cynefin code never reads `htmlLabels` (grep of the folder). The labels are always SVG text.

### 6.2 Root and the viewBox

`draw` computes `totalWidth = width + 2 padding` and `totalHeight = height + 2 padding` (`cynefinRenderer.ts:88-89`). It calls `configureSvgSize(svg, totalHeight, totalWidth, useMaxWidth ?? true)` (line 101), then sets `viewBox = "0 0 W H"` (line 102).

- With `useMaxWidth` true (the default), the root has `width="100%"` and `style="max-width: Wpx;"` (`setupGraphViewbox.js:23-33`, probed: `max-width: 880px;`). There is no `height` attribute.
- With `useMaxWidth` false, the root has `width="W"` and `height="H"`, and no style (probed).
- `setupGraphViewbox` and its `getBBox` of the root are not used, so the viewBox does not depend on any measurement.

The root attributes come out in this order (probed): `id`, `width`, `xmlns`, `style`, `viewBox`, `role="graphics-document document"`, `aria-roledescription="cynefin"`.

### 6.3 Element structure, in DOM order (probed)

```text
svg
  title, desc                  accessibility, mermaid's own: only with accTitle or accDescr
  style                        the generic style sheet, scoped to #id (mermaidAPI.ts:596-601)
  g                            empty (generic; not from cynefin)
  title, desc                  cynefin's own: only with accTitle or accDescr (cynefinRenderer.ts:104-110)
  g  transform="translate(padding, padding)"               cynefinRenderer.ts:112
    g.cynefin-backgrounds      four rect.cynefinDomain (118-132)
    g.cynefin-boundaries       path.cynefinBoundary twice, then path.cynefinCliff (135-154)
    path.cynefinConfusion      the ellipse (161-165)
    g.cynefin-labels           four text.cynefinDomainLabel, then the Confusion label (168-189)
    g.cynefin-subtitles        eight quadrant texts and one for confusion: only with showDomainDescriptions (193-226)
    g.cynefin-items            one g per item: rect.cynefinItem, then text.cynefinItemText (229-349). An overflow badge is rect.cynefinItemOverflow
    g.cynefin-arrows           per transition kept: path.cynefinArrowLine, and text.cynefinArrowLabel when labelled (369-420). Only when a transition is kept
    text.cynefinTitle          only with a title (423-433)
  defs > marker                only when a transition is kept, appended after the root g (353-367)
```

The marker is `marker id="cynefin-arrow-<id>" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse"`, with one `path d="M 0 0 L 10 5 L 0 10 z" class="cynefinArrowHead"` (probed).

The attributes of each element are the ones in `cynefinRenderer.ts` (for example `rect` gets `class`, `x`, `y`, `width`, `height`, `fill`, `fill-opacity`, `stroke="none"`). The probed SVG lists them in that order.

### 6.4 Layout arithmetic

With the defaults (`width` 800, `height` 600, `padding` 40, so `hw` 400 and `hh` 300):

| Quantity              | Formula                                                                                                                                                                                                                  | Default                                                | Source                     |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------ | -------------------------- |
| total size            | `width + 2 padding`, `height + 2 padding`                                                                                                                                                                                | 880 by 680                                             | `cynefinRenderer.ts:88-89` |
| root translate        | `(padding, padding)`                                                                                                                                                                                                     | (40, 40)                                               | 112                        |
| quadrant layouts      | complex: centre `(hw/2, hh/2)`, box `(0, 0, hw, hh)`. complicated: centre `(hw + hw/2, hh/2)`, box `(hw, 0)`. chaotic: centre `(hw/2, hh + hh/2)`, box `(0, hh)`. clear: centre `(hw + hw/2, hh + hh/2)`, box `(hw, hh)` | centres (200, 150), (600, 150), (200, 450), (600, 450) | 40-50                      |
| confusion centre      | `(hw, hh)` (its box is not drawn)                                                                                                                                                                                        | (400, 300)                                             | 48                         |
| quadrant rects        | the four boxes, `fill-opacity` 0.4, `stroke` none                                                                                                                                                                        |                                                        | 118-132                    |
| confusion ellipse     | centre `(width/2, height/2)`, radii `0.15 width`, `0.15 height`                                                                                                                                                          | (400, 300), rx 120, ry 90                              | 158-165                    |
| quadrant labels       | `(cx, cy - 30)` with descriptions, `(cx, cy)` without. `text-anchor` middle, `dominant-baseline` middle                                                                                                                  | complex (200, 120)                                     | 168-179                    |
| confusion label       | `(width/2, height/2 - 10)` with descriptions, `(width/2, height/2)` without                                                                                                                                              | (400, 290)                                             | 182-189                    |
| subtitles             | model at `(cx, cy - 10)`, practice at `(cx, cy + 5)`                                                                                                                                                                     |                                                        | 198-214                    |
| confusion practice    | `(width/2, height/2 + 8)`                                                                                                                                                                                                | (400, 308)                                             | 217-225                    |
| item start, quadrant  | `cy + 25` with descriptions, `cy + 15` without                                                                                                                                                                           | 175 (complex), 475 (clear)                             | 258                        |
| item start, confusion | `cy + 22` with descriptions, `cy + 14` without                                                                                                                                                                           | 322                                                    | 255-256                    |
| item pitch            | `26 + 4`                                                                                                                                                                                                                 | 30                                                     | 230, 263                   |
| item group            | `translate(cx - badge/2, startY + index * 30)`                                                                                                                                                                           |                                                        | 262-289                    |
| item rect             | `(0, 0, badge, 26)`, `rx` and `ry` 4, `fill` the domain colour, `fill-opacity` 0.95                                                                                                                                      |                                                        | 292-301                    |
| item text             | `(badge/2, 13)`, `text-anchor` middle, `dominant-baseline` central                                                                                                                                                       |                                                        | 304                        |
| badge width           | measured text width + 20 (`itemPaddingX` 10 on each side)                                                                                                                                                                |                                                        | 276-286                    |
| confusion cap         | first 3 items; overflow badge `+N more` at `startY + 3 * 30`, `fill-opacity` 0.6, class `cynefinItemOverflow`                                                                                                            |                                                        | 247-250, 307-348           |
| arrow path            | `M x1,y1 Q cpx,cpy x2,y2`, from the centre of `from` to the centre of `to`. Control point: the midpoint plus `0.15 * len` along the normal `(-dy/len, dx/len)`                                                           | e.g. `M200,150 Q400,210 600,150`                       | 385-407                    |
| arrow label           | `(cpx, cpy - 6)`, `text-anchor` middle, `dominant-baseline` auto                                                                                                                                                         |                                                        | 415-418                    |
| title                 | `(width/2, -padding/2)` in root coordinates, `text-anchor` middle, `dominant-baseline` middle                                                                                                                            | (400, -20)                                             | 423-433                    |

The probed positions match these formulas: for example the complex item at `translate(172.0234375, 175)` with a 55.953125 badge, the clear item at y 475, and the confusion overflow at y 412 with five items.

Items are drawn per domain in a fixed order, complex, complicated, chaotic, clear, confusion (`cynefinRenderer.ts:233`), and in declaration order within a domain (probed: the all-five fixture). Items inside a quadrant are not capped and not clipped (the many-items fixture has three per quadrant and draws them all).

`-padding / 2` with padding 0 gives `-0`, which is written as `y="0"` (probed). The port must not print `-0`.

### 6.5 Boundary geometry

Everything in this section is in `cynefinBoundaries.ts`.

- **`seededRandom(seed)`** (lines 5-10): `t = (seed + 0x6d2b79f5) | 0`, then `t = imul(t ^ (t >>> 15), t | 1)`, then `t ^= t + imul(t ^ (t >>> 7), t | 61)`, then `((t ^ (t >>> 14)) >>> 0) / 2^32`. The `| 0` truncates a fractional seed toward zero, so seeds 1 and 1.5 give the same paths (probed: identical paths for seed 1 and seed 1.5). The `t + imul(...)` sum is converted to int32 by `^`.
- **`hashString(str)`** (15-23): `hash = (hash << 5) - hash + ch`, with `|= 0` after each step, over the UTF-16 code units. Probed value: `hashString('ref-seed-a') = -1042255233`, and the Python transcription in `seed_check.py` reproduces the probed id-seeded path.
- **`resolveSeed(configured, id)`** (32-41): a finite, non-zero configured number is used as it is. Anything else gives `hashString(id)`. The configured value is used directly, so a negative seed works (probed: seed -3).
- **The fold** (`generateFoldPath`, 52-84): `cx = width/2`, `amplitude` is the configured `boundaryAmplitude` (the `??` at line 59 keeps 0, so 0 is straight, probed), and 7 segments with `segHeight = height/7`. Point `i` is `(cx + (seededRandom(seed + 17i) * 2A - A), i * segHeight)`. There are eight points (`i` from 0 to 7). Each segment `i` has control points at the mid height, offset horizontally from its two end points by `1.5 A dir seededRandom(seed + 31i + 7)`, where `dir` is +1 for even `i` and -1 for odd (`cp1x = p0.x + offset`, `cp2x = p1.x - offset`; `cynefinBoundaries.ts:74-80`). The path string starts `M x0,y0` and adds `C cp1 cp2 p1` per segment.
- **The horizontal line** (94-126): the same with `seed + 100` (renderer line 146), points `(i * width/7, cy + (seededRandom(seed + 23i) * 2A - A))`, offsets from `seededRandom(seed + 37i + 11)`, and the control points at the mid width, offset vertically by the same kind of term (`cynefinBoundaries.ts:116-121`).
- **The cliff** (135-151): uses `width * 0.03` as its amplitude, not the configured one, and has no seed. Its points are fixed.
- **The ellipse** (161-169): two arcs of `rx, ry`, `0 1,1`.
- **The default amplitude** (`amplitudeOverride ?? width * 0.015`, line 59 and the horizontal line 101) only applies when the configured value is `null` or `undefined`. The schema default is 8, so this is not reached with the defaults.

Test vector (probed, and checked against the Python transcription): with width 800, height 600, seed 42 and amplitude 8, the fold begins `M401.6176600307226,0 C406.1051963418722,42.857142857142854 393.83792743459344,42.857142857142854 398.32546374574304,85.71428571428571`. Seed 1 begins `M402.0331830494106,0 C403.9087250875309,42.857142857142854 396.49504977371544,42.857142857142854 398.37059181183577,85.7…`. The probed fold for id `ref-seed-a` with seed 0 begins `M405.8679441511631,0 …`, which is the same arithmetic with seed `hashString('ref-seed-a')`.

The Python check matched every digit of the probed paths. Python writes `0.0` where JavaScript writes `0`, and both use the shortest round-trip form, so the port needs that form. Rust's `Display` for `f64` gives the same digits in this range, but it writes `-0` for negative zero, which JavaScript writes as `0`.

### 6.6 Text measurement

The only measurement in cynefin is the width of an item's text, which sets the badge width (`cynefinRenderer.ts:276-286`, and the overflow badge at 322-332). The code asks the text element for `getBBox().width`, and falls back to `label.length * 7` (the UTF-16 length) if the width is 0 or the method is missing (lines 277-284). The text element has the class `cynefinItemText`, which the style sheet sets to 12 px in Lexica Ultralegible (`styles.ts:50-53`). The style sheet is in the document before the draw (section 6.1), so the measurement is at 12 px.

Probed: the rect width of each item is its text box plus 20 (for example `Item A` has a text box of 35.953125 and a rect of 55.953125; `Investigate root cause` has 118.359375 and 138.359375). An empty label has a box of 0, and so the fallback width of 0 gives a badge of 20 (probed: `""` gives a rect of 20 with the text `""`).

The harness compares the box of every `text` element (`diagram-geometry.ts` header, and `common.ts` `geometry`). So the other texts are also measured by the harness, and their boxes must match Chromium's: the domain labels (bold 16 px, `styles.ts:35-39`), the subtitles (italic 11 px, `styles.ts:40-44`), the arrow labels (11 px, `styles.ts:83-86`), and the title (bold 18 px, `styles.ts:87-91`). The renderer does not measure these, because they are centred with `text-anchor` middle. Their boxes still depend on the glyph advances of the faces they use.

### 6.7 Styles

`styles` (`styles.ts:29-93`) returns one string of rules, which `compileCSS` scopes with the element id (`mermaidAPI.ts:319-334`, probed: each rule begins `#<id> .cynefinX{…}`). The rules and the theme keys they read:

| Rule                   | Declarations                                                                            | Theme keys (`styles.ts`)                   | Lines |
| ---------------------- | --------------------------------------------------------------------------------------- | ------------------------------------------ | ----- |
| `.cynefinDomain`       | `stroke: none`                                                                          | none                                       | 32-34 |
| `.cynefinDomainLabel`  | `font-size` domainFontSize px, `font-weight: bold`, `fill` labelColor                   | `domainFontSize` (36), `labelColor` (38)   | 35-39 |
| `.cynefinSubtitle`     | `font-size` itemFontSize minus 1 px, `fill` textColor, `font-style: italic`             | `itemFontSize` (41), `textColor` (42)      | 40-44 |
| `.cynefinItem`         | `fill-opacity: 0.95`, `stroke` boundaryColor, `stroke-width: 1`                         | `boundaryColor` (47)                       | 45-49 |
| `.cynefinItemText`     | `font-size` itemFontSize px, `fill` textColor                                           | `itemFontSize` (51), `textColor` (52)      | 50-53 |
| `.cynefinItemOverflow` | `fill-opacity: 0.6`, `stroke` boundaryColor, `stroke-width: 1`, `stroke-dasharray: 3 2` | `boundaryColor` (56)                       | 54-59 |
| `.cynefinBoundary`     | `stroke` boundaryColor, `stroke-width` boundaryWidth, `stroke-dasharray: 6 3`           | `boundaryColor` (61), `boundaryWidth` (62) | 60-64 |
| `.cynefinCliff`        | `stroke` cliffColor, `stroke-width` cliffWidth                                          | `cliffColor` (66), `cliffWidth` (67)       | 65-68 |
| `.cynefinConfusion`    | `stroke` boundaryColor, `stroke-width: 1.5`, `stroke-dasharray: 4 2`                    | `boundaryColor` (70)                       | 69-73 |
| `.cynefinArrowLine`    | `stroke` arrowColor, `stroke-width` arrowWidth, `fill: none`                            | `arrowColor` (75), `arrowWidth` (76)       | 74-78 |
| `.cynefinArrowHead`    | `fill` arrowColor, `stroke: none`                                                       | `arrowColor` (80)                          | 79-82 |
| `.cynefinArrowLabel`   | `font-size` itemFontSize minus 1 px, `fill` textColor                                   | `itemFontSize` (84), `textColor` (85)      | 83-86 |
| `.cynefinTitle`        | `font-size` domainFontSize plus 2 px, `font-weight: bold`, `fill` labelColor            | `domainFontSize` (88), `labelColor` (90)   | 87-91 |

The fills of the rectangles, the ellipse and the item badges are attributes set in `draw`, not CSS: the domain colours (`cynefinRenderer.ts:129, 164, 300, 344`) and the opacities 0.4, 0.5, 0.95 and 0.6 (130, 165, 301, 345). The style function ignores the options it is passed (`mermaidAPI.ts:327-333` passes `themeVariables`, `theme` and `look`): it reads the theme through `getThemeVariables()` and the global configuration (`styles.ts:11-14`), which the renderer also does. The two agree (section 6.8).

The font family of the text is not set by cynefin. The generic root rule of the style sheet sets `font-family` from the theme's `fontFamily` (probed: `#<id>{font-family:"Lexica Ultralegible";font-size:14px;fill:#28253D;}`). The generic rule is shared, so it is not part of this type's work, but the text boxes depend on it.

### 6.8 Theme variables

The renderer and the styles both do the same thing (`cynefinRenderer.ts:61-66` and `styles.ts:11-14`): `cleanAndMerge(getThemeVariables(), getConfig().themeVariables).cynefin`. The first argument is the default theme, with no overrides. The second is the active theme's resolved variables (`config.ts:133-137`), which include the active theme's `cynefin` block when it has one. `cleanAndMerge` is lodash `merge` (`utils.ts:841-843`), which skips `undefined` values, so a theme with no block keeps the default block, key by key.

Keys read by the renderer: `complexBg`, `complicatedBg`, `clearBg`, `chaoticBg`, `confusionBg` (`cynefinRenderer.ts:91-97`). Keys read by the styles: `domainFontSize`, `itemFontSize`, `boundaryColor`, `boundaryWidth`, `cliffColor`, `cliffWidth`, `arrowColor`, `arrowWidth`, `textColor`, `labelColor` (`styles.ts:15-26`, read at the lines in section 6.7).

The values the blocks give (probed: `theme-probe.mts`, the merged block for each case):

| Theme                                     | Block                      | `domainFontSize`, `itemFontSize`            | `boundaryColor`, `arrowColor`                       | `boundaryWidth`, `arrowWidth` | `cliffColor`, `cliffWidth` | Backgrounds (complex, complicated, chaotic, clear, confusion) | `textColor`, `labelColor` |
| ----------------------------------------- | -------------------------- | ------------------------------------------- | --------------------------------------------------- | ----------------------------- | -------------------------- | ------------------------------------------------------------- | ------------------------- |
| default                                   | `theme-default.js:294-310` | 16, 12                                      | `#333333` (the default `lineColor`, lines 298, 302) | 2, 2                          | `#8B0000`, 4               | `#E8F5E9`, `#E3F2FD`, `#FBE9E7`, `#FFF8E1`, `#F3E5F5`         | `#333`, `#131300`         |
| base                                      | `theme-base.js:254-271`    | 16, 12                                      | `#0b0b0b`                                           | 2, 2                          | `#8B0000`, 4               | as default                                                    | `#333`, `#333`            |
| dark                                      | `theme-dark.js:265-282`    | 16, 12                                      | `lightgrey`                                         | 2, 2                          | `#FF6B6B`, 4               | `#1B5E20`, `#0D47A1`, `#BF360C`, `#F57F17`, `#4A148C`         | `#ccc`, `#e0dfdf`         |
| forest                                    | `theme-forest.js:257-274`  | 16, 12                                      | `#000000`                                           | 2, 2                          | `#8B4513`, 4               | `#C8E6C9`, `#DCEDC8`, `#FFE0B2`, `#FFF9C4`, `#D7CCC8`         | `#000000`, `#321b67`      |
| neutral                                   | `theme-neutral.js:282-299` | 16, 12                                      | `#666`                                              | 2, 2                          | `#8B0000`, 4               | as default                                                    | `#000000`, `#111111`      |
| redux-color, redux-dark-color, redux, neo | none                       | the default block, as the table's first row |                                                     |                               |                            |                                                               |                           |

Probed results for the themes ferrigura ships:

- **redux-color.** The merged block is the default block (`#E8F5E9` and the rest, `#333333`, `#131300`). The redux theme's own line colour (`#000000`) and text colour (`#28253D`) are not used. The probed Chromium render has these fills and label colours (`chrome-results.json`, the simple fixture).
- **redux-dark-color.** The same default block, so the labels are `#131300` on the pastel backgrounds. This is what the reference draws for the dark redux theme (probed in Chromium: `rgb(19, 19, 0)` for the labels).
- **dark** (not shipped by ferrigura, but the harness maps `…dark` names to redux-dark-color, so the dark block is only a reference point): the dark backgrounds and `lightgrey` boundaries are used (probed).
- **base with overrides** (the e2e fixture): the base block, then the overrides of `complexBg`, `clearBg` and `boundaryColor` (probed: fills `#FFE4B5` and `#E6E6FA`, boundary `#FF0000`).

None of the redux themes (`redux`, `redux-color`, `redux-dark`, `redux-dark-color`) or the neo themes has a cynefin block (grep of `packages/mermaid/src/themes`). Probed for `redux`, `redux-color`, `redux-dark-color` and `neo`.

### 6.9 Configuration

| Key                              | Default      | Source                             | Read at                                      | Notes                                                                                |
| -------------------------------- | ------------ | ---------------------------------- | -------------------------------------------- | ------------------------------------------------------------------------------------ |
| `cynefin.width`                  | 800          | schema `3070-3074`                 | `cynefinRenderer.ts:83`                      | total width, quadrant widths, all x positions                                        |
| `cynefin.height`                 | 600          | schema `3075-3079`                 | line 84                                      | total height, quadrant heights, all y positions                                      |
| `cynefin.padding`                | 40           | schema `3080-3084`                 | line 85                                      | root translate, title y                                                              |
| `cynefin.showDomainDescriptions` | true         | schema `3085-3088`                 | line 86 (used at 175, 186, 192, 255, 258)    | subtitles on or off, and the item offsets                                            |
| `cynefin.boundaryAmplitude`      | 8            | schema `3089-3094` (min 0, max 50) | line 87 (used at 140, 146)                   | 0 gives straight boundaries (probed)                                                 |
| `cynefin.seed`                   | 0            | schema `3095-3102`                 | line 115 (`resolveSeed`)                     | 0 gives the id hash (section 6.5)                                                    |
| `cynefin.useMaxWidth`            | true         | base section `457-470`             | line 101 (`?? true`, which is never reached) | read from the cynefin section only: a top-level `useMaxWidth` has no effect (probed) |
| `themeVariables`                 | active theme | `config.ts:133-137`                | renderer 63-64, styles 13-14                 | the `cynefin` block (section 6.8)                                                    |

The renderer reads the typed view from `db.getConfig()` (`cynefinDb.ts:71-76`), so the defaults come from the schema through `DEFAULT_CONFIG.cynefin` (`defaultConfig.ts:338-340`). `htmlLabels`, `look`, `fontFamily`, `securityLevel` and `handDrawnSeed` are not read by the cynefin code.

A value of the wrong type is not rejected: `"width":"wide"` gives the viewBox `0 0 wide80 680`, `max-width: wide80px`, and `NaN` in every path and rect (probed). The schema says `number`, and the reference does not check it.

### 6.10 Randomness and the clock

- **The seed** is the only randomness, and it is deterministic given the id. The id is the `id` argument of `draw`, which is the SVG id (`cynefinRenderer.ts:71, 115`).
- **`mermaid.run` ids are time-based.** `InitIDGenerator.next` returns `Date.now()` unless `deterministicIds` is set (`utils.ts:734-743`), and `run` builds the id as `mermaid-${…}` (`mermaid.ts:159, 174`). A seed-0 diagram rendered through `run` therefore differs from one render to the next (**unverified**: read, not probed).
- **No `Math.random`, no `Date`, no timers** appear in the cynefin code (grep of the folder).
- **The harness** sets `Math.random` and `Date` only when a clock is given (`common.ts`, `window.renderReference`). Cynefin does not use either, so the clock is not needed.

### 6.11 DOM dependencies a server renderer must replace

- `getBBox` of each item text (`cynefinRenderer.ts:279-284` and `324-329`), behind a `typeof … === 'function'` check. A server renderer needs the text advances of Lexica Ultralegible at 12 px (section 6.6).
- `svg.select('defs')` and `svg.append` (lines 353, 106, 109), which need a DOM. The port writes the elements directly.
- `selectSvgElement(id)` (line 99) and `configureSvgSize` (setupGraphViewbox.js:43-47), which set attributes on the element that mermaid created. There is no measured root.
- The harness's own `getBBox` of every text element and path (section 6.6), which the port's output must match.

### 6.12 Sanitising and entities

Two rules shape the text, and both are visible in the output:

1. **Entities.** `encodeEntities` turns `#amp;` into `ﬂ°amp¶ß` and `#35;` into `ﬂ°°35¶ß` before parsing (`utils.ts:874-895`), in every text of the diagram, including labels. `decodeEntities` (`utils.ts:904-906`) runs on the whole SVG string (`mermaidAPI.ts:359`) and turns `ﬂ°°` into `&#`, `ﬂ°` into `&`, and `¶ß` into `;`. So `#amp;` shows as `&`, `#35;` shows as `#`, and the probed item `ﬂ°amp¶ß` also shows as `&`, because the decoding does not know where a placeholder came from (probed: item `a #35; b` shows `a # b`, `title A #amp; B` shows `A & B`).
2. **DOMPurify on titles.** `sanitizeText` (`common.ts:71-83`) runs `sanitizeMore` (56-69) and then `DOMPurify.sanitize(…, { FORBID_TAGS: ['style'] })`. DOMPurify returns the input unchanged when it has no `<` (`dompurify@3.4.12`, `dist/purify.es.mjs:2250-2252`). Any `<` makes it parse and serialise the text. The result is then drawn with `.text()`, which does not parse markup. So the visible text is the serialised text:
   - `title A & B` shows `A & B` (no `<`, unchanged).
   - `title a < b` stores `a &lt; b` and shows `a &lt; b` (probed).
   - `title <b>x</b> & y` stores `<b>x</b> &amp; y` and shows the tags and `&amp;` as text (probed in Chromium).
   - `title <img src=x onerror=alert(1)> T` stores `<img src="x"> T` (probed, DB).
   - `title A<script>alert(1)</script>B` stores `AB` (probed, DB).
   - The accessible title and description go through the same steps (`commonDb.ts:16-24`), and `accTitle: A & B` shows `A & B` (probed).
   - `sanitizeMore` does nothing when `htmlLabels` is false (`common.ts:57`, `config.ts:324-326`), which is the harness's setting. With the default (`htmlLabels` true, `securityLevel` strict), it removes `<script>` before DOMPurify runs (lines 58-60). Item and transition labels are not sanitised at all, so `<b>x</b> & y` in an item is drawn as the text `<b>x</b> & y` (probed).

## 7. Fixtures

| File (in `e2e/diagrams/cynefin/`)                                                         | Features                                                                              | viewBox (probed) | What the render shows                                       |
| ----------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- | ---------------- | ----------------------------------------------------------- |
| `nightly-check/should-render-cynefin-with-straight-boundaries-when-amplitude-is-zero.mmd` | `boundaryAmplitude: 0`; four domains, one item each; title                            | `0 0 880 680`    | Straight boundaries (x = 400 on every point, probed)        |
| `nightly-check/should-render-cynefin-without-domain-descriptions.mmd`                     | `showDomainDescriptions: false`; two domains; title                                   | `0 0 880 680`    | No subtitles. The only fixture with no `→`                  |
| `pr-check/should-render-a-cynefin-diagram-with-transitions.mmd`                           | four domains and four labelled transitions around the cycle; title                    | `0 0 880 680`    | Defs, four arrows, four labels                              |
| `pr-check/should-render-a-simple-cynefin-diagram-with-all-five-domains.mmd`               | five domains, eight items, one in confusion; title                                    | `0 0 880 680`    | All five domains                                            |
| `pr-check/should-render-an-empty-cynefin-framework.mmd`                                   | four domains, no items; title                                                         | `0 0 880 680`    | No item groups                                              |
| `pr-check/should-render-cynefin-deterministically-with-an-explicit-seed-override.mmd`     | `seed: 42`; four items                                                                | `0 0 880 680`    | The same paths for any id (probed)                          |
| `pr-check/should-render-cynefin-with-accessibility-directives.mmd`                        | `accTitle` and `accDescr`; four items                                                 | `0 0 880 680`    | Two `title` and two `desc` elements (section 6.3)           |
| `pr-check/should-render-cynefin-with-config-override.mmd`                                 | `width: 1000`, `height: 700`, `boundaryAmplitude: 15`; two domains                    | `0 0 1080 780`   | Custom size                                                 |
| `pr-check/should-render-cynefin-with-confusion-domain-items-without-overflow.mmd`         | confusion with five items; title                                                      | `0 0 880 680`    | Three items and `+2 more`. The name says "without overflow" |
| `pr-check/should-render-cynefin-with-many-items-per-domain.mmd`                           | four domains, eleven items; title                                                     | `0 0 880 680`    | Two or three items per quadrant, none clipped               |
| `pr-check/should-render-cynefin-with-self-loop-transitions-silently-dropped.mmd`          | one kept transition, one self-loop                                                    | `0 0 880 680`    | One arrow and the defs                                      |
| `pr-check/should-render-cynefin-with-theme-override.mmd`                                  | front matter `theme: base` with `complexBg`, `clearBg` and `boundaryColor`; two items | `0 0 880 680`    | Under the harness: redux-color with the overrides (probed)  |

The viewBox is the configured size plus twice the padding, so it does not depend on any measurement. Every fixture is `0 0 880 680` except the config override.

### 7.1 What the generic geometry harness will have to exclude or handle

1. **The arrow in the subtitles.** `DOMAIN_META` has `→` (U+2192) in four model strings (`cynefinRenderer.ts:24-27`). The bundled faces do not have it: the `cmap` of `LexicaUltralegible-Regular`, `-Bold` and `-Italic` lacks U+2192 and has U+2014, U+00DC, U+03B6, U+00E9 and U+2019 (`cmap.py`, probed). Chromium therefore draws the glyph in another face, and the box of each subtitle depends on it. This affects every fixture with `showDomainDescriptions` on, which is 11 of 12. The harness's `UNCOVERED_SYMBOLS` check (`common.ts`, the exclusion rules) reads the fixture text, and the arrows are not in the fixture text, so the check does not catch it. The exclusion has to be added by name for this type, or the fallback-face question has to be settled first.
2. **The seed.** The geometry harness does not set `cynefin.seed`. The e2e viewer sets it to 1 (`e2e/helpers/util.ts:46`). The harness renders each fixture with the id `ref-<slug>` (`diagram-geometry.ts:351`), and ferrigura must take the same id. The self pass renders `self-<slug>` (`diagram-geometry.ts:391`), and a seed-0 fixture gets wave paths that depend on the id (probed: the same text with `ref-seed-a` and `ref-seed-b` gives different paths, while the seed-42 text gives the same paths for both ids). So `--self` reports differences that are not noise in the port.
3. **Text boxes.** The harness compares the box of every text element (section 6.6), so the bold, italic and bold-italic faces at 11, 12, 16 and 18 px all matter, not only the badges.
4. **The theme.** `theme: base` in the front matter is replaced by redux-color, and the overrides still apply (probed).
5. **No exclusions needed for:** `handDrawn` (none of the fixtures uses `look`), tickets, `htmlLabels` (forced to false, and the cynefin code does not read it), or non-ASCII text in the fixture files (grep finds none). The fixtures use Latin letters only.

## 8. Risks, effort, open questions

### 8.1 Risks

| Risk                           | Level                                  | Why                                                                                                                                                                                                                 |
| ------------------------------ | -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The `→` glyph in the subtitles | High for the harness, low for the code | Eleven fixtures draw a glyph that the bundled faces lack. The port's text measurement must match whatever face Chromium uses on the reference machine, which is **unverified** across machines (probed on this one) |
| Text boxes of five kinds       | Medium                                 | Every text box is compared. The M5 text rules (decision [0005](../decisions/0005-text-measurement-and-fonts.md)) must cover bold, italic and bold-italic at four sizes                                              |
| Titles and DOMPurify           | Medium                                 | The visible output of a title with `<` is the serialised text. A sanitiser that differs from DOMPurify's serialisation shows different text. Only the fast path (no `<`) is easy                                    |
| Seed and id                    | Medium                                 | The port must take the id as an input (ferrigura's `--id` does). The `--self` pass is not a noise baseline for seed-0 fixtures                                                                                      |
| Redux theme colours            | Low for the code, high for review      | The reference draws the default block under the redux themes. A port that uses the redux values does not match                                                                                                      |
| Number formatting              | Low                                    | `-0` must print as `0`, and the numbers must use the shortest round-trip form. Checked with Python for the seed path                                                                                                |
| Entity decoding                | Low                                    | The decoding is on the whole SVG, so a label containing a placeholder letter is changed too                                                                                                                         |

### 8.2 Effort

A rough estimate in Rust lines, not a measurement:

| Part                                                                                           | Lines      |
| ---------------------------------------------------------------------------------------------- | ---------- |
| Scanner: the 14 token rules in their order, the keyword guard, the string escapes              | 120 to 160 |
| Parser: the header, blocks, items, transitions, title and accessibility lines, error positions | 100 to 140 |
| Value converter: the escapes, the title and description normalisation                          | 60 to 80   |
| DB: the domain map, transitions and the self-loop filter, the config merge, title precedence   | 100 to 130 |
| Sanitiser for titles, only if ferrigura has none: DOMPurify's fast path and serialisation      | 0 to 400   |
| Renderer: the layout, the items and the badge measurement, the overflow, the arrows, the title | 350 to 450 |
| Boundaries: seeded random, hash, fold, horizontal, cliff, ellipse                              | 100 to 130 |
| Styles: the 13 rules and the theme reads                                                       | 80 to 100  |
| Configuration and theme views ([0009](../decisions/0009-configuration-resolved-once.md))       | 60 to 100  |
| Tests: 42 spec ports and the probes of sections 2 to 6                                         | 400 to 500 |

Without tests and without a sanitiser the total is 1,000 to 1,300 lines. The renderer is the largest part. Against the kanban estimate (1,600 to 2,100 lines), both the parser and the renderer are smaller.

### 8.3 Open questions for the coordinator

- **Q1. The `→` glyph.** Exclude the eleven fixtures with subtitles, by name, until the fallback-face question is settled? Or settle the fallback first? The harness's source-text rule does not catch this case, so the exclusion has to be added for the type. Recommendation: a named exclusion for the subtitles, with the cause recorded, so the other fixtures of the type are still compared.
- **Q2. The seed in the harness.** Add `cynefin.seed` to the forced settings of the geometry harness, as the e2e viewer does (seed 1), or keep the seed 0 and the id? And what should `--self` do for seed-0 fixtures? Recommendation: set the seed in the geometry harness for this type, so both sides draw the same waves, and record why the e2e viewer and the harness differ.
- **Q3. Titles with `<`.** Reproduce DOMPurify's serialisation (`&lt;`, `&amp;`, and the markup shown as text), or only the fast path, with the other cases recorded as a named difference? This depends on whether ferrigura has a sanitiser that matches DOMPurify. No fixture uses `<` in a title. Recommendation: reproduce the fast path now, and decide the rest with the sanitiser work.
- **Q4. Redux colours.** Confirm that the port's cynefin tokens for `redux-color` and `redux-dark-color` are the default block (as probed), and that the typed tokens of decision 0007 carry these values. The `redux-dark-color` theme then shows `#131300` labels, which is what the reference does.
- **Q5. Non-numeric configuration.** The reference renders `NaN` for `"wide"`. A typed view cannot hold that. Recommendation: reject a non-number at the configuration stage with an error, and accept the numbers the reference accepts (zero and negative values are not checked by the reference either; not probed).
- **Q6. The canonical model.** Build the DB model first (section 4), and rely on the geometry harness for the layout. Agree that the model holds the placeholders as the reference does.
- **Q7. The fixture name.** The fixture `…with-confusion-domain-items-without-overflow` draws `+2 more`. This is a snapshot test (e2e), so a rename or a change of its expectation is the coordinator's call. The port does not need a change.
- **Q8. Tooling.** `tools/port/export-cynefin.ts` is needed for `port:diff`, and `tools/port/diff-config.ts:265` should drop `cynefin` from `UNPORTED_SECTIONS` once the configuration is ported.
- **Q9. Seed 0 through `run`.** The reference's `run` gives a time-based id (section 6.10), so seed-0 output is not reproducible through `run`. The port keeps the id as an input, which is correct. The coordinator may want this written into the port's documentation.
