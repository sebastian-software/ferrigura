# Wardley (P3): research notes for the port

Status: research only. Nothing in the repository changed except this file, and nothing is committed. Reference: `packages/mermaid/src` at HEAD `f3a1a94b0` (branch `sebastian/port-p3`). Plan: [../P3-PLAN.md](../P3-PLAN.md). Guide: [../P2-RESEARCH.md](../P2-RESEARCH.md). Model: [../kanban/NOTES.md](../kanban/NOTES.md). Lexer and converter layout: [../architecture/PLAN.md](../architecture/PLAN.md) section 2.

Every rule cites the reference as `diagrams/wardley/<file>:<line>` unless the file is named in full. Parser files are `packages/parser/src/language/...`, and core files are `packages/mermaid/src/...`. **Probed** means the rule was run through the reference's own code. The parser rules ran through the Langium services of `packages/parser/src` (`parse` in `parse.ts`, the same call the diagram makes), and the rendering through the built bundle `packages/mermaid/dist` in Chromium with `tools/port/geometry/common.ts` (`withPage`, `renderReference` and `patchText`), so the harness's forced settings apply: `htmlLabels: false`, Lexica faces, `securityLevel: strict`, and the theme the harness forces (see section 7). **Unverified** means read or inferred only. The probe scripts and their output are outside the repository, in `/private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/p3-wardley/`: `probe-lexer.mts` (the token order), `probe-parse*.mts` (acceptance and the AST), `probe-render*.mts` with `render-out*.txt` (the fixtures, the quirks and the DOM), `probe-font.mts`, `probe-style.mts`, `probe-svg.mts` and `probe-dark.mts`.

## 0. Summary

Wardley maps are a Langium grammar (`wardley-beta`) whose statements place named components on a two-axis map. The DB keeps the placed nodes, links, trends, pipelines, annotations, notes and arrows in the order the text gives them. The renderer is one function, `draw`, of about 1,070 lines that projects each coordinate to the plot area by arithmetic, emits the SVG element by element with d3 selections, and measures text in one place only (the annotation box). There is no layout engine: no position depends on a text width, and no randomness or clock is read.

The port is small compared with kanban or use case, but it has more quirks per line than most: thirty-three behaviours of the lexer, the parser and the DB that the reference shows and a port must keep (section 2.5). Three findings change the plan and are the first things to decide (section 8.3):

1. **The stylesheet is never in the reference output.** `draw` removes every child of the root `svg` before drawing (`wardleyRenderer.ts:85`), and the `<style>` that mermaid inserted there goes with them. The 90-line style provider (`styles.ts`) runs and its result is dropped (probed: no `<style>` in any render; the flowchart keeps its style). So the attributes of the SVG are the whole appearance.
2. **The wardley text has no font.** The root has no `font-family`, so the text takes the page default. In the harness that page is `Times` (probed: computed font family of every label is `Times`, the annotation line measures 95.625 px there and 101.75 px in Lexica Ultralegible). The harness's `fontFamily` setting does not reach the wardley text.
3. **The redux themes have no wardley tokens.** Neither `redux-color` nor `redux-dark-color` (the two ferrigura ships, and the two the harness forces for every fixture) defines `themeVariables.wardley`. The renderer therefore uses its own hard-coded fallbacks (`#fff`, `#000`, `rgba(100, 100, 100, 0.2)`, `#dc3545`) for the component colours. Only `base`, `dark`, `default`, `forest` and `neutral` define the tokens, and the harness never renders those.

| Item                    | Value                                                                                                                                                                      |
| ----------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source lines (no specs) | 1,825: renderer 1,071, builder 223, parser 245, DB 157, styles 90, detector, diagram and types 39 (`P3-PLAN.md` gives the same count)                                      |
| Grammar                 | `wardley.langium`, 154 lines, with `common/common.langium` imported; value converter 19 lines                                                                              |
| Specs                   | 2 files, 24 tests (20 parser and DB, 4 builder), no renderer spec                                                                                                          |
| Fixtures                | 10 in `e2e/diagrams/wardley/pr-check/` (the `nightly-check` and `visual-check` folders hold only `.gitkeep`)                                                               |
| Port size               | About 2,750 to 3,600 lines of Rust, section 8.2                                                                                                                            |
| Riskiest parts          | The font of the text (section 8.1), the lexer's keyword and prefix rules (section 2.2), the renderer's DOM order and attribute values, which must match with no stylesheet |

## 1. Sources

| File                                                                      | Lines | What it holds                                                                                                                                                                 |
| ------------------------------------------------------------------------- | ----- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `packages/mermaid/src/diagrams/wardley/wardleyParser.ts`                  | 245   | `toPercent` (8-17), `toCoordinates` (19-28), flow helpers (30-66), `populateDb` (68-227), the parser object (229-245)                                                         |
| `packages/mermaid/src/diagrams/wardley/wardleyDb.ts`                      | 157   | The DB facade over the builder (16-131) and its exports (133-157)                                                                                                             |
| `packages/mermaid/src/diagrams/wardley/wardleyBuilder.ts`                 | 223   | The state: node map, links, trend map, pipelines, annotations, notes, arrows, axes, size; `build` (187-208), `clear` (210-222)                                                |
| `packages/mermaid/src/diagrams/wardley/wardleyRenderer.ts`                | 1,071 | `draw` (67-1066): theme (31-50), configuration (52-65), every element of section 6                                                                                            |
| `packages/mermaid/src/diagrams/wardley/styles.ts`                         | 90    | The style provider (21-88); its output is dropped (section 6.9)                                                                                                               |
| `packages/mermaid/src/diagrams/wardley/wardleyDetector.ts`                | 24    | The detector `/^\s*wardley-beta/i` (line 10)                                                                                                                                  |
| `packages/mermaid/src/diagrams/wardley/wardleyDiagram.ts`                 | 12    | The `DiagramDefinition`: parser, db, renderer, styles                                                                                                                         |
| `packages/mermaid/src/diagrams/wardley/wardleyTypes.ts`                   | 3     | `WardleyDB` type                                                                                                                                                              |
| `packages/mermaid/src/diagrams/wardley/wardleyParser.spec.ts`             | 345   | 20 tests (section 5)                                                                                                                                                          |
| `packages/mermaid/src/diagrams/wardley/wardleyBuilder.spec.ts`            | 38    | 4 tests of `resolveNodeId`                                                                                                                                                    |
| `packages/parser/src/language/wardley/wardley.langium`                    | 154   | The grammar: the entry (4-12), statements (14-27), the rules (29-104), terminals (109-154)                                                                                    |
| `packages/parser/src/language/wardley/valueConverter.ts`                  | 19    | `LINK_LABEL` (6-18)                                                                                                                                                           |
| `packages/parser/src/language/wardley/module.ts`                          | 77    | The services; `WardleyModule` (39-46)                                                                                                                                         |
| `packages/parser/src/language/common/common.langium`                      | 34    | `EOL`, `TitleAndAccessibilities` (8-10), `ACC_DESCR` (14), `ACC_TITLE` (15), `TITLE` (16), `INT` (19), `STRING` (22), `ID` (27), `NEWLINE` (29), the hidden terminals (31-34) |
| `packages/parser/src/language/common/valueConverter.ts`                   | 81    | `runConverter` (27-42) and `runCommonConverter` (44-70)                                                                                                                       |
| `packages/parser/src/language/common/matcher.ts`                          | 14    | The regexes of `ACC_DESCR` (4), `ACC_TITLE` (9) and `TITLE` (14)                                                                                                              |
| `packages/parser/src/parse.ts`                                            |       | `wardley` initializer (105-108), the generic `parse` (133-) and `MermaidParseError`                                                                                           |
| `packages/parser/node_modules/langium/lib/parser/value-converter.js`      |       | Langium 4.4.0 `DefaultValueConverter`: `runConverter` (24-37), `convertString` (41-55), `convertNumber` (89)                                                                  |
| `packages/mermaid/src/diagrams/common/commonDb.ts`                        |       | `clear` (10-14) and the setters that sanitise (16-29)                                                                                                                         |
| `packages/mermaid/src/diagrams/common/populateCommonDb.ts`                | 14    | The header texts, each set only when truthy (4-14)                                                                                                                            |
| `packages/mermaid/src/Diagram.ts`                                         |       | `db.clear` (49), the front matter title (53), `encodeEntities` (77), `parser.parse` (83)                                                                                      |
| `packages/mermaid/src/preprocess.ts`                                      |       | `preprocessDiagram` (58-80): front matter, directives, comment lines                                                                                                          |
| `packages/mermaid/src/mermaidAPI.ts`                                      |       | `getStyles` call (327-333); the `<style>` inserted into the svg before `draw` (596-601)                                                                                       |
| `packages/mermaid/src/rendering-util/selectSvgElement.ts`                 | 22    | d3 `select` of the svg by id, and the sandbox branch (11-22)                                                                                                                  |
| `packages/mermaid/src/setupGraphViewbox.js`                               |       | `calculateSvgSizeAttrs` (23-33) and `configureSvgSize` (43-46)                                                                                                                |
| `packages/mermaid/src/diagram-api/diagramConfigKeys.ts`                   |       | `wardley: 'wardley-beta'` (line 15)                                                                                                                                           |
| `packages/mermaid/src/schemas/config.schema.yaml`                         |       | The section key (430-431), the definition (3009-3061) with its defaults                                                                                                       |
| `packages/mermaid/src/config.type.ts`                                     |       | The section key (347) and `WardleyDiagramConfig` (2465-2498)                                                                                                                  |
| `packages/mermaid/src/themes/theme-{base,dark,default,forest,neutral}.js` |       | The `wardley` tokens: base 287-301, dark 351-365, default 352-366, forest 324-338, neutral 359-373                                                                            |

The renderer uses d3 from `d3` ^7.9.0 (`packages/mermaid/package.json:78`), through `selectSvgElement.ts:1` (`select`) and `import type * as d3` (`wardleyRenderer.ts:1`). No other d3 module is used.

## 2. Parser

The parser is Langium 4.4.0 (decision [0002](../decisions/0002-hand-written-parsers.md) makes it hand-written in ferrigura). The pipeline is `Diagram.fromText` (`Diagram.ts:19-83`): `db.clear()`, then the front matter title, then `encodeEntities(source) + '\n'`, then `parser.parse`. The wardley DB does not set `preserveCommentsWhenParsing`, so the parser reads the comment-stripped text (`Diagram.ts:57-82`, read). `preprocessDiagram` (`preprocess.ts:58-80`) removes the front matter (its `config` is merged), the `%%{...}%%` directives and the whole comment lines. A `%%` in the middle of a line is not removed there.

### 2.1 Modes and entry

Langium has one lexer mode. The entry is `Wardley` (`wardley.langium:4-12`):

```text
NEWLINE* KW_WARDLEY ( NEWLINE | TitleAndAccessibilities | Statement )*
```

- The header is `wardley-beta` with no word end after it. `wardley-betax` is an error, not a header (probed). The detector is case-insensitive (`wardleyDetector.ts:10`) but the keyword is not: `Wardley-Beta` is detected and then rejected (probed: "Expecting token of type 'KW_WARDLEY'"). A port must do the same: detect with `^\s*wardley-beta` ignoring case, then parse case-sensitively.
- The header may share its line with a statement or a header title: `wardley-beta component A [0.2, 0.4]` and `wardley-beta title Inline` are accepted (probed).
- `TitleAndAccessibilities` (`common.langium:8-10`) may appear between statements, any number of times.
- Every statement ends with `EOL`, which is one or more newlines or the end of the input (`common.langium:6`). Two statements on one line are rejected.

### 2.2 The lexer: token order

Langium builds the Chevrotain lexer with `DefaultTokenBuilder`. The order below is the reference's own lexer (`createWardleyServices().Wardley.parser.chevrotainLexer.lexerDefinition`, probed with `probe-lexer.mts`). Chevrotain takes the first pattern that matches at the position, not the longest, unless the token has a `LONGER_ALT`. Only one token has one here: `-` (LONGER_ALT `ARROW`, `LINK_ARROW`, `YAML`).

| No.      | Token                                                                                                                                                                                                          | Pattern (as Chevrotain has it)                                                                                        | Notes                                                                                                                                                                                                                                            |
| -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1 to 10  | `[` `,` `]` `@` `/` `(` `)` `-` `{` `}`                                                                                                                                                                        | literal                                                                                                               | The punctuation of the grammar, in the order it first appears in the grammar (read).                                                                                                                                                             |
| 11       | `WARDLEY_NUMBER`                                                                                                                                                                                               | `[0-9]+\.[0-9]+`                                                                                                      | Decimals only (`wardley.langium:109`).                                                                                                                                                                                                           |
| 12       | `ARROW`                                                                                                                                                                                                        | `->`                                                                                                                  |                                                                                                                                                                                                                                                  |
| 13       | `LINK_PORT`                                                                                                                                                                                                    | `\+<>\|\+>\|\+<`                                                                                                      |                                                                                                                                                                                                                                                  |
| 14       | `LINK_ARROW`                                                                                                                                                                                                   | `-->\|-\.->\|>\|\+'[^']*'<>\|\+'[^']*'<\|\+'[^']*'>`                                                                  |                                                                                                                                                                                                                                                  |
| 15       | `LINK_LABEL`                                                                                                                                                                                                   | `;[^\n\r]+`                                                                                                           | Needs at least one character after `;`.                                                                                                                                                                                                          |
| 16       | `STRATEGY`                                                                                                                                                                                                     | `build\|buy\|outsource\|market`                                                                                       | Before the names (see the prefix rule below).                                                                                                                                                                                                    |
| 17 to 30 | `KW_WARDLEY`, `KW_SIZE`, `KW_EVOLUTION`, `KW_ANCHOR`, `KW_COMPONENT`, `KW_LABEL`, `KW_INERTIA`, `KW_EVOLVE`, `KW_PIPELINE`, `KW_NOTE`, `KW_ANNOTATIONS`, `KW_ANNOTATION`, `KW_ACCELERATOR`, `KW_DEACCELERATOR` | the word                                                                                                              | Declared as terminals, not as keyword literals, so they have no `LONGER_ALT`.                                                                                                                                                                    |
| 31       | `NAME_WITH_SPACES`                                                                                                                                                                                             | `(?!title\s\|accTitle\|accDescr)[A-Za-z](?:[A-Za-z0-9_()&]\|-(?!>))*(?:[ \t]+[A-Za-z(](?:[A-Za-z0-9_()&]\|-(?!>))*)*` | `wardley.langium:143`. A name may hold spaces, `(`, `)`, `&`, and a `-` that is not followed by `>`.                                                                                                                                             |
| 32       | `WS`                                                                                                                                                                                                           | `[ \t]+`                                                                                                              | Skipped (`wardley.langium:154`).                                                                                                                                                                                                                 |
| 33       | `ACC_DESCR`                                                                                                                                                                                                    | `[\t ]*accDescr(?:[\t ]*:([^\n\r]*?(?=%%)\|[^\n\r]*)\|\s*{([^}]*)})`                                                  | From `common.langium:14`.                                                                                                                                                                                                                        |
| 34       | `ACC_TITLE`                                                                                                                                                                                                    | `[\t ]*accTitle[\t ]*:(?:[^\n\r]*?(?=%%)\|[^\n\r]*)`                                                                  | `common.langium:15`.                                                                                                                                                                                                                             |
| 35       | `TITLE`                                                                                                                                                                                                        | `[\t ]*title(?:[\t ][^\n\r]*?(?=%%)\|[\t ][^\n\r]*\|)`                                                                | `common.langium:16`. The lazy `(?=%%)` cuts the title at a `%%`.                                                                                                                                                                                 |
| 36       | `INT`                                                                                                                                                                                                          | `0\|[1-9][0-9]*(?!\.)`                                                                                                | `common.langium:19`.                                                                                                                                                                                                                             |
| 37       | `STRING`                                                                                                                                                                                                       | `"([^"\\]\|\\.)*"\|'([^'\\]\|\\.)*'`                                                                                  | Either quote.                                                                                                                                                                                                                                    |
| 38       | `ID`                                                                                                                                                                                                           | `[\w]([-\w]*\w)?`                                                                                                     | ASCII word characters only.                                                                                                                                                                                                                      |
| 39       | `NEWLINE`                                                                                                                                                                                                      | `\r?\n`                                                                                                               |                                                                                                                                                                                                                                                  |
| 40       | `WHITESPACE`                                                                                                                                                                                                   | `[\t ]+`                                                                                                              | Skipped.                                                                                                                                                                                                                                         |
| 41 to 43 | `YAML`, `DIRECTIVE`, `SINGLE_LINE_COMMENT`                                                                                                                                                                     | `---` block, `%%{...}%%`, `%%...`                                                                                     | Hidden. A `---` block in the body is accepted and hidden (probed). `preprocessDiagram` removes the directives and the comment lines before the lexer runs (read), so those two are rarely seen; a `%%` comment in the body is accepted (probed). |

Not in the lexer, because the grammar does not reach them: `BOOLEAN`, `FLOAT` and `NUMBER` of `common.langium`.

Consequences, each probed with `probe-parse*.mts` unless marked:

- **Prefix rule for the keywords and the strategies.** Tokens 16 to 30 come before `NAME_WITH_SPACES` and `ID`, so a word that begins with one of them is split. Rejected (probed): a component named `marketing`, `buyer` or `builder`; `notes`, `evolutionary`, `componentA`; `title` as a name. `titlebar` and `accTitlex` are names, because the lookahead of `NAME_WITH_SPACES` needs the space (`title\s`) or the whole word (`accTitle`) and `ID` takes the rest.
- **Digits.** `INT` (36) comes before `ID` (38), so a name that starts with a digit is rejected (`2fast` probed). `_x` is an id (probed).
- **Non-ASCII letters** are a lexer error (`Café`, probed).
- **Hyphens.** A `-` in a name is kept unless it is followed by `>`, so `foo-`, `foo--bar`, `real-time processing` are names and `a->b` is an arrow (probed; `a->b` as a name is a parse error).
- **Arrows.** `->`, `-->`, `-.->`, `>`, `+<`, `+>`, `+<>`, `+'label'>`, `+'label'<`, `+'label'<>` are arrows. `A -- B`, `A ----> B`, `A<-B`, `A .-. B`, `A +'x' B` and `A +> B ;` are rejected (probed). `.-.` is in the reference's `isDashed` test (`wardleyParser.ts:175`), but the lexer never produces it, so that branch is dead (probed by the rejection).
- **Leading blanks.** `WS` (32) comes before `TITLE` (35), so the blanks before a `title` line are skipped first and the `TITLE` pattern starts at the word.

### 2.3 The grammar rules

| Rule                           | Source                  | Form                                                                  | Reads as                                                                                                                                                                                                                                                        |
| ------------------------------ | ----------------------- | --------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Size`                         | `wardley.langium:29-31` | `size [INT, INT] EOL`                                                 | Integers only (`size [1200.5, 900]` is rejected, probed). The last `size` wins (probed).                                                                                                                                                                        |
| `Evolution`                    | `:33-35`                | `evolution Stage (-> Stage)+ EOL`                                     | At least two stages. A single stage is rejected (probed). The last `evolution` wins (probed).                                                                                                                                                                   |
| `EvolutionStage`               | `:37-39`                | `name ('@' WARDLEY_NUMBER)? ('/' name)?`                              | The boundary comes before the second name (`Genesis@0.3 / Concept` probed; `Genesis / Concept@0.3` is an error). A boundary is a decimal (`@0` and `@1` are rejected; `@1.0` is 1).                                                                             |
| `Anchor`                       | `:41-43`                | `anchor name [WARDLEY_NUMBER, WARDLEY_NUMBER] EOL`                    | No label or decorator (probed).                                                                                                                                                                                                                                 |
| `Component`                    | `:45-53`                | `component name [v, e] label? decorator? (inertia \| (inertia))? EOL` | The order is fixed: `label`, then the decorator, then `inertia`. `component A [0.5,0.5] (build) (inertia)` is accepted, `(inertia) (build)` and `(build) label [..]` are rejected (probed). The bare `inertia` word and `(inertia)` are both accepted (probed). |
| `Label`                        | `:55-57`                | `label [-? INT, -? INT]`                                              | Offsets are integers, the sign is a flag (`label [-1, -2]` gives `negX`, `negY` and 1, 2). `label [1.5, 2]` is rejected (probed).                                                                                                                               |
| `Decorator`                    | `:59-61`                | `( STRATEGY )`                                                        | `build`, `buy`, `outsource`, `market`.                                                                                                                                                                                                                          |
| `Link`                         | `:63-66`                | `from fromPort? arrow? to toPort? linkLabel? EOL`                     | Both ports may appear (`A +> B +<`, probed), and an arrow may take a port after it (`A -> B +>`, probed).                                                                                                                                                       |
| `Evolve`                       | `:68-70`                | `evolve name WARDLEY_NUMBER EOL`                                      |                                                                                                                                                                                                                                                                 |
| `Pipeline`                     | `:72-76`                | `pipeline name { NEWLINE+ PipelineComponent+ } EOL`                   | At least one component. A component on the same line as the brace is rejected (probed). Blank lines inside are accepted.                                                                                                                                        |
| `PipelineComponent`            | `:78-80`                | `component name [WARDLEY_NUMBER] label? EOL`                          | No decorator (probed) and no inertia (read from the rule).                                                                                                                                                                                                      |
| `Note`                         | `:82-84`                | `note STRING [WARDLEY_NUMBER, WARDLEY_NUMBER] EOL`                    | Decimals only (`note "x" [1, 0]` is rejected, probed).                                                                                                                                                                                                          |
| `Annotations`                  | `:86-88`                | `annotations [CoordinateValue, CoordinateValue] EOL`                  | `CoordinateValue` is a decimal or an integer (`:94-96`).                                                                                                                                                                                                        |
| `Annotation`                   | `:90-92`                | `annotation INT, [CoordinateValue, CoordinateValue] STRING EOL`       | The text is required, and may be `""` (probed).                                                                                                                                                                                                                 |
| `Accelerator`, `Deaccelerator` | `:98-104`               | `accelerator name [WARDLEY_NUMBER, WARDLEY_NUMBER] EOL`               | Decimals only; a name with spaces or quotes is accepted (probed).                                                                                                                                                                                               |

The entry's `Statement` fragment (`:14-27`) is a choice, and each alternative is a feature of the `Wardley` node. `size` and `evolution` are single values (last wins). The others are arrays: `anchors`, `components`, `links`, `evolves`, `pipelines`, `notes`, `annotations`, `annotation`, `accelerators` and `deaccelerators`. The DB reads only the first `annotations` block (section 3.1).

The name rule: a name is a `STRING`, an `ID` or a `NAME_WITH_SPACES`, in that order (read: `:42`, `:46`, `:64`). The `ID` alternative covers the names that begin with `_` and the names that `NAME_WITH_SPACES` rejects (`accTitlex`). Spaces inside a name are kept as written: `Data  Store` keeps its two spaces (probed).

### 2.4 The value converter

Two layers run for every token (`common/valueConverter.ts:27-42`):

1. `runCommonConverter` (`common/valueConverter.ts:44-70`) for the rules named `ACC_DESCR`, `ACC_TITLE` and `TITLE`, with the regexes of `matcher.ts`:
   - `TITLE` and `ACC_TITLE` (single line): the first capture, trimmed, with each run of two or more spaces or tabs made into one space.
   - `ACC_DESCR` single line: the same with `accDescr:`.
   - `ACC_DESCR` block (`accDescr { ... }`): each line's leading and trailing whitespace removed, runs of blanks made into one space, and runs of newlines made into one (read; probed for `accDescr {\n line one\n line two\n}` gives `line one\nline two`).
   - `title 50%% done` is `50`: the token stops at `%%` and the converter trims it (probed).
2. `WardleyValueConverter.runCustomConverter` (`wardley/valueConverter.ts:6-18`) for `LINK_LABEL`: drop the leading `;`, trim. So `A -> B ;  label; more  ` gives `label; more` (probed).

If neither applies, Langium's `DefaultValueConverter.runConverter` runs (`value-converter.js:24-37`):

- `INT` is `parseInt`.
- `STRING` strips the quotes and reads the escapes `\b \f \n \r \t \v \0`, and any other escaped character is itself (`convertString`, `:41-55`). `"A\"B"` is `A"B` (probed).
- `ID` drops a leading `^` (never present, since `ID` cannot start with `^`).
- A rule whose type is `number` (`WARDLEY_NUMBER`, `CoordinateValue`) is `Number(input)`. `1.0` is `1`.
- Everything else (`NAME_WITH_SPACES`, `LINK_ARROW`, `LINK_PORT`, `STRATEGY`, `KW_*`) is the raw text.

`EOL` returns a string, and nothing reads it.

### 2.5 What the parser hands to the DB, and the quirks a port reproduces

`parser.parse` (`wardleyParser.ts:234-244`) calls `parse('wardley', input)` from `@mermaid-js/parser`. Any lexer or parser error throws `MermaidParseError` (`parse.ts`), with the Langium message. Errors are described in section 8.3 (Q5).

Quirks, with the probe that shows them. The first group is the lexer and the grammar.

| No. | Input                                                   | Result                                                                                                     | Source of the rule                                   |
| --- | ------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- | ---------------------------------------------------- |
| L1  | `component marketing [0.5, 0.5]`                        | rejected                                                                                                   | prefix rule, section 2.2                             |
| L2  | `component notes [0.5, 0.5]`                            | rejected                                                                                                   | prefix rule                                          |
| L3  | `component 2fast [0.5, 0.5]`                            | rejected                                                                                                   | `INT` before `ID`                                    |
| L4  | `Wardley-Beta` header                                   | rejected                                                                                                   | case-sensitive keyword                               |
| L5  | `wardley-betax`                                         | rejected                                                                                                   | no word end on the header                            |
| L6  | `component titlebar [0.2, 0.3]`                         | accepted, name `titlebar`                                                                                  | the `title\s` lookahead                              |
| L7  | `component Size [0.2, 0.3]`                             | accepted                                                                                                   | keywords are case-sensitive                          |
| L8  | `component foo- [0.2, 0.3]`                             | accepted, name `foo-`                                                                                      | a trailing `-` is part of a name                     |
| L9  | `A .-. B`, `A -- B`                                     | rejected                                                                                                   | no such arrow                                        |
| L10 | `A +> B ;`                                              | lexer error                                                                                                | `LINK_LABEL` needs a character                       |
| L11 | `component A [0, 0.5]`, `[1, 0.5]`                      | rejected                                                                                                   | component coordinates are decimals                   |
| L12 | `size [1200.5, 900]`                                    | rejected                                                                                                   | size is integers                                     |
| L13 | `evolution A@1 -> B`                                    | rejected                                                                                                   | boundaries are decimals                              |
| L14 | `evolution A -> B` (two stages)                         | accepted                                                                                                   | at least two                                         |
| L15 | `component A [0.5,0.5] (inertia) (build)`               | rejected                                                                                                   | order is fixed                                       |
| L16 | `annotations [1, 0]` and `annotation 1,[1, 0] "x"`      | accepted                                                                                                   | `CoordinateValue` takes integers (fixture 6 uses it) |
| L17 | `title One` then `title Two`                            | title `Two`                                                                                                | last value of a single feature                       |
| L18 | `title ` (blank)                                        | the title stays as it was (read: `populateCommonDb` sets only a truthy value, `populateCommonDb.ts:11-13`) |                                                      |
| L19 | `component "A\"B" [0.1, 0.2]`, `'Single'`               | `A"B`, `Single`                                                                                            | `convertString`                                      |
| L20 | `title 50%% done`                                       | title `50`                                                                                                 | the `TITLE` token ends at `%%`                       |
| L21 | `wardley-beta component A [0.2, 0.4]`                   | accepted                                                                                                   | a statement may follow the header on the same line   |
| L22 | CRLF line endings, tabs between words, no final newline | accepted                                                                                                   | `NEWLINE` is `\r?\n`                                 |
| L23 | `accTitle: My title`, `accDescr {...}`                  | `My title`, `line one\nline two`                                                                           | the common converter                                 |

The second group is the DB, described in section 3:

| No. | Input                                                     | Result                                                                                                              |
| --- | --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| D1  | Two components with the same name                         | one node, at the second position; the first position is kept in the node map (probed: `duplicate-component`)        |
| D2  | Two `pipeline` blocks for one parent                      | the second replaces the first's components, so only its box is drawn; both sets of component nodes exist (probed)   |
| D3  | `X -> Y` with neither declared                            | no error; no link is drawn (probed)                                                                                 |
| D4  | `evolve Nope 0.8`                                         | no error; no trend (probed)                                                                                         |
| D5  | `evolve A 0.8` before `component A`                       | works, because components are added before evolves (probed)                                                         |
| D6  | `Queue -> K` where `Queue` is a pipeline component of `K` | resolved by label; the link is dropped by the renderer (probed)                                                     |
| D7  | `pipeline Nope { ... }`                                   | error `Pipeline "Nope" must reference an existing component with coordinates.` (probed)                             |
| D8  | Two `annotations` blocks                                  | only the first positions the box (`wardleyParser.ts:200-204`, read)                                                 |
| D9  | `component A [1.5, 1.0]`                                  | accepted as 1.5 (not scaled, since 1.5 is above 1), and 1.0 becomes 100 (probed)                                    |
| D10 | `flow` from a label that contains `<`                     | `A +'a<b'> B` gets a start marker, not an end marker, because `extractFlowFromArrow` tests `includes('<')` (probed) |

### 2.6 Errors

The DB's errors are the reference's text, and the lexer and parser errors are Langium's. The DB messages (probed unless marked):

- `Component "A" evolution must be between 0-1 (decimal) or 0-100 (percentage). Received: 100.5` (for a value above 100, probed). The check is on evolution first, then on visibility (`toCoordinates`, `wardleyParser.ts:19-28`), so `[100.5, 100.5]` reports the evolution (probed).
- The same message with `visibility` for the first number, and with `Anchor "X"`, `Note "text"`, `Pipeline component "X" evolution`, `Evolve target for "X"`, `Annotations box`, `Annotation N`, `Accelerator "X"` and `Deaccelerator "X"` as the context (`wardleyParser.ts:94`, `:103`, `:133`, `:157`, `:194`, `:202`, `:208`, `:214`, `:220`). `Evolve target for "X" must be ... Received: 100.5` is probed.
- `Pipeline "X" must reference an existing component with coordinates.` (`wardleyParser.ts:141-143`, probed).
- `Node "X" is missing coordinates` (`wardleyBuilder.ts:191`) cannot be reached from the grammar, since every node is added with coordinates (read).

The message texts are not part of the differential (section 8.3, Q5).

## 3. DB

### 3.1 Calls in order

`populateDb` (`wardleyParser.ts:68-227`) runs these calls, in this order. Each one is a method of the DB (`wardleyDb.ts`), which forwards to the builder.

| Step | Source     | Call                                                                                                                                                                     | Effect                                                                                                                                                                                                         |
| ---- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1    | `:69`      | `populateCommonDb`                                                                                                                                                       | `setAccDescription`, `setAccTitle`, `setDiagramTitle`, each only when the text is truthy (`populateCommonDb.ts:4-13`). Sanitised (3.3).                                                                        |
| 2    | `:72-74`   | `setSize(width, height)` when `size` was given                                                                                                                           | Replaces the size.                                                                                                                                                                                             |
| 3    | `:77-90`   | `updateAxes({ stages, stageBoundaries })` when `evolution` was given                                                                                                     | `stages` are the names, trimmed, with a second name as `Name / Second` (`:78-84`). `stageBoundaries` are the boundaries that are present, in order (`:85-87`). `setAxes` merges (`wardleyBuilder.ts:155-160`). |
| 4    | `:93-96`   | `addNode(name, name, x, y, 'anchor')`                                                                                                                                    | One per anchor.                                                                                                                                                                                                |
| 5    | `:99-129`  | `addNode(name, name, x, y, 'component', labelX, labelY, inertia, strategy)`                                                                                              | Per component. The label offsets are `(neg ? -1 : 1) * offset` (`:105-110`). The strategy is the decorator's (`:111-116`).                                                                                     |
| 6    | `:132-135` | `addNote(text, x, y)`                                                                                                                                                    | Per note.                                                                                                                                                                                                      |
| 7    | `:138-171` | `getNode(parent)`, `startPipeline(parent)`, then `addNode(parent_name, name, x, parentY, 'pipeline-component', labelX, labelY)` and `addPipelineComponent` per component | The parent must be a node with a `y` (`:139-144`). The component's `y` is the parent's (`:164`), and its `x` is the evolution (`:157`).                                                                        |
| 8    | `:174-187` | `addLink(resolve(from), resolve(to), dashed, label, flow)`                                                                                                               | Per link. `isDashed` is `-.->` or `.-.` (`:175`). The flow is the port's, else the arrow's (`:177-181`). The label is the arrow's `+'label'`, else the `;` label (`:183-184`).                                 |
| 9    | `:190-197` | `addTrend(name, target, node.y)` when the node exists and has a `y`                                                                                                      | `getNode` looks at the id only (`:192`).                                                                                                                                                                       |
| 10   | `:200-204` | `setAnnotationsBox(x, y)` for the first `annotations`                                                                                                                    |                                                                                                                                                                                                                |
| 11   | `:207-210` | `addAnnotation(number, [{x, y}], text)` per annotation                                                                                                                   |                                                                                                                                                                                                                |
| 12   | `:213-216` | `addAccelerator(name, x, y)`                                                                                                                                             |                                                                                                                                                                                                                |
| 13   | `:219-226` | `addDeaccelerator(name, x, y)`                                                                                                                                           |                                                                                                                                                                                                                |

The order matters, as the probes show: components are all added before pipelines and evolves, so an `evolve` may name a component declared later (D5). Pipelines are added after the components, so a pipeline parent must be declared, anywhere in the text, before its pipeline is processed (the statements are read in source order, but the calls in the order above). Links come after pipelines, so a link may name a pipeline component by its label (D6).

Coordinates are converted by `toCoordinates(visibility, evolution)` (`:19-28`): `x` is the evolution and `y` the visibility, each through `toPercent` (`:8-17`). The grammar's names `x` and `y` for the annotations box, the annotation and the accelerators mean `[visibility, evolution]`, as `toCoordinates` reads them (`:200-214`). The spec confirms it: `annotations [0.10, 0.90]` gives a box at `x = 90, y = 10` (`wardleyParser.spec.ts:220`).

### 3.2 The builder (`wardleyBuilder.ts`)

- `addNode` (`:92-102`): `existing ?? { id, label }`, then the new fields spread over it. `className`, `labelOffsetX` and `labelOffsetY` keep the old value when the new one is `undefined` (`??`). `inertia` and `sourceStrategy` are always overwritten, even with `undefined`, because the call passes them as keys (read; not probed). The map keeps the first position (`Map.set` on an existing key keeps the order; D1 probed).
- `addLink`, `addAnnotation`, `addNote`, `addAccelerator`, `addDeaccelerator` push to lists, so the order is the text order.
- `addTrend` (`:108-110`) is a map keyed by `nodeId`, so the last `evolve` for a component wins (probed: `self-evolve-unknown-pipeline`, one trend).
- `startPipeline` (`:112-120`) replaces the parent's entry, so its component list is empty again (D2), and marks the node `isPipelineParent`.
- `addPipelineComponent` (`:122-133`) appends the id and marks the node `inPipeline`.
- `setAxes` merges the given keys (`:155-160`). `setSize` replaces (`:162-164`). `setAnnotationsBox` replaces (`:151-153`).
- `resolveNodeId` (`:175-185`): an exact id first, then the first node whose label equals the name, else the name itself. A pipeline component is found by its label, which is how `Queue -> Z` works (probed: `pipeline-link-to-other`).
- `build` (`:187-208`): the nodes in map order, the links, the trends and the pipelines in insertion order, the annotations, the notes, the accelerators, the deaccelerators, the box, a copy of the axes, and the size (`undefined` when absent).

### 3.3 Sanitising and defaults

- The three header texts go through `sanitizeText` in the common setters (`commonDb.ts:16-29`). The sanitiser is the common one of the reference (`common.js`), not shown here (unverified beyond the call).
- Node names, component labels, note texts, annotation texts, accelerator and deaccelerator names, and link labels are not sanitised. The renderer writes them with `.text()` (`wardleyRenderer.ts:9`, the comment), so `<b>x</b>` is shown as typed (probed: `note "a <b>x</b>"`).
- `encodeEntities` runs on the whole text before the parse (`Diagram.ts:77`, `utils.ts:874` onwards). `#35;` in a note, a name or a title is shown as `#` (probed: `Cost #35; here` gives `Cost # here`). `&amp;` in a name is shown literally as `&amp;` (probed: `"R&amp;D"`).
- Defaults: an absent link label or flag is `undefined` or `false`; an absent `size` is `undefined`, and the renderer then uses the configuration (`wardleyRenderer.ts:81-82`); an absent `evolution` leaves `axes` without `stages`, and the renderer uses `DEFAULT_STAGES` (`wardleyRenderer.ts:12`, `:214-215`). `xLabel` and `yLabel` are never set by any statement (read: the only writers are the builder's interface and the renderer's defaults, `wardleyRenderer.ts:186-187`), so the axis labels are always `Evolution` and `Visibility`.

### 3.4 `clear`

`clear` (`wardleyDb.ts:128-131`) calls `builder.clear()` (`wardleyBuilder.ts:210-222`), which resets the nodes, links, trends, pipelines, annotations, notes, accelerators, deaccelerators, the box, the axes and the size, and `commonClear()` (`commonDb.ts:10-14`), which resets the three header texts. `Diagram.fromText` calls `db.clear()` before each parse (`Diagram.ts:49`). Nothing else is kept between parses. `wardleyDb.getConfig` (`:16-18`) reads the section for `wardley-beta` and is exported, but nothing in `packages/mermaid/src` calls it (grep; the other packages are unverified).

### 3.5 Quirks of the DB

They are listed in section 2.5 (D1 to D10) with their probes. In addition, these are read and not probed:

- `inertia` and `sourceStrategy` are overwritten by a second declaration of the same component, even by `undefined` (section 3.2).
- A pipeline component's id is `Parent_Name`, so a component that is named `Parent_Name` clashes with it (read).
- The `isDashed` test has a `.-.` branch that the lexer never produces (section 2.2).

## 4. Canonical model proposal

The differential compares the DB state after the parse, as `getWardleyData()` returns it. There is no separate layout-data layer, as there is in kanban, because `draw` reads the DB state and projects it. The model follows [canonical-model.md](../canonical-model.md): `"version": 1`, strings always present (`""` when absent), `null` for an absent value that is not a string, and camelCase.

```text
WardleyMap {
  version: 1,
  title, accTitle, accDescr,        // strings, "" when absent
  size: { width, height } | null,
  axes: { stages: [string] | null, stageBoundaries: [number] | null },
  nodes: [Node],                    // map order
  links: [Link],
  trends: [{ nodeId, targetX, targetY }],
  pipelines: [{ nodeId, componentIds: [string] }],
  annotations: [{ number, coordinates: [{ x, y }], text }],
  notes: [{ text, x, y }],
  accelerators: [{ name, x, y }],
  deaccelerators: [{ name, x, y }],
  annotationsBox: { x, y } | null
}
Node  { id, label, x, y, className: string | null, labelOffsetX: number | null,
         labelOffsetY: number | null, inertia: boolean | null, sourceStrategy: string | null,
         inPipeline: boolean | null, isPipelineParent: boolean | null }
Link  { source, target, dashed: boolean, label: string | null, flow: string | null }
```

| Field                            | Rule                                                                                                                                                                                                               |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `nodes`                          | The DB's node order (the first insertion of each id). Anchors, then components, then pipeline components, as the calls of section 3.1 insert them.                                                                 |
| `x`, `y`                         | The chart percentages (0 to 100), as `toPercent` gives them. The model keeps the DB's `x` (evolution) and `y` (visibility), not the statement's order (section 8.3, Q8).                                           |
| `className`                      | `anchor`, `component` or `pipeline-component`. `evolved` is never produced (read; `wardleyRenderer.ts:811` tests it, no path reaches it).                                                                          |
| `labelOffsetX`, `labelOffsetY`   | The signed offsets. `null` when the DB has none.                                                                                                                                                                   |
| `inertia`                        | `false` for a component without the word, `true` for one with it (passed at `wardleyParser.ts:126`), and `null` for anchors and pipeline components, whose `addNode` calls pass no inertia (`:93-96`, `:160-168`). |
| `sourceStrategy`                 | `build`, `buy`, `outsource`, `market` or `null`.                                                                                                                                                                   |
| `inPipeline`, `isPipelineParent` | Set by the builder's marks; `null` when unset.                                                                                                                                                                     |
| `links[].dashed`                 | `true` only for `-.->`.                                                                                                                                                                                            |
| `links[].flow`                   | `forward`, `backward`, `bidirectional` or `null`.                                                                                                                                                                  |
| `axes.stages`                    | `null` when there is no `evolution`, so the renderer uses its defaults.                                                                                                                                            |
| `axes.stageBoundaries`           | `null` when there is no `evolution`; `[]` when the evolution has no boundary; otherwise the present boundaries, in order (`wardleyParser.ts:85-87`).                                                               |
| `annotations[].text`             | The text as read, `""` when empty (`annotation 3,[0.2, 0.3] ""`).                                                                                                                                                  |
| `size`                           | `null` when absent. The configuration's size is not in the model.                                                                                                                                                  |

Left out: the derived node positions after the renderer's pipeline step (renderer state, section 6.2), the configuration (compared by `pnpm port:diff config` in the `wardley-beta` group, `tools/port/diff-config.ts:264`), the `xLabel` and `yLabel` (never set), and the function-valued members of the DB facade.

## 5. Specs

Two spec files, 24 `it` tests.

**`wardleyParser.spec.ts` (20 tests, 345 lines).** It runs the parser through `diagram.parser`, with the DB as `parser.parser.yy`, and reads the state through `db.getWardleyData()`. Each test is a parse plus a state check. The tests, in order:

| Test                                                                               | Covers                                                                                                      |
| ---------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| parses components and links                                                        | nodes, one link, the title                                                                                  |
| parses custom evolution stages                                                     | stages                                                                                                      |
| parses dual-label evolution stages with slashes                                    | `secondName`                                                                                                |
| parses pipeline blocks with single-coordinate components                           | pipelines, the y inheritance, label offsets (`label [-60, 35]` in the input)                                |
| parses custom stage widths with boundary notation                                  | `stageBoundaries`                                                                                           |
| parses notes with quoted text                                                      | notes                                                                                                       |
| parses annotations with quoted text                                                | annotations                                                                                                 |
| parses anchors                                                                     | anchors                                                                                                     |
| parses evolve statements                                                           | trends                                                                                                      |
| parses component decorators                                                        | `sourceStrategy`                                                                                            |
| parses size directive                                                              | size                                                                                                        |
| handles quoted identifiers, inline labels, and converts coordinates to percentages | strings, `+<>`, flow, label, notes, box, annotation, accelerator, deaccelerator (the coordinate conversion) |
| accepts hyphens in unquoted component names                                        | names with `-`                                                                                              |
| accepts hyphens in link endpoints                                                  | link endpoints                                                                                              |
| accepts hyphens in anchor names                                                    | anchors                                                                                                     |
| still tokenises A->B as an arrow (no-space regression)                             | the `-` lookahead                                                                                           |
| parses foo-bar->baz as link from foo-bar to baz                                    | the `-` lookahead                                                                                           |
| accepts hyphens in pipeline component names with label overrides                   | pipeline names with `-`                                                                                     |
| accepts consecutive hyphens inside a name                                          | `foo--bar`                                                                                                  |
| accepts a trailing hyphen in a name                                                | `foo-`                                                                                                      |

**`wardleyBuilder.spec.ts` (4 tests, 38 lines).** `WardleyBuilder.resolveNodeId`: the exact id, the label fallback for a pipeline's synthetic id, an unknown name returned as it is, and the id winning over a label.

What they cover for the port: the parser and the DB together (20), `resolveNodeId` (4). Nothing tests the DB calls one by one, the layout (there is none), or the renderer. The renderer has no spec in its folder. Two specs outside the folder use wardley: `config.appearance.spec.ts:142-145` (the `wardley-beta` section reaches the diagram), `:170-176` (the front matter does too), and `rendering-util/multi-diagram-id-uniqueness.spec.ts:182` (a sample map in a list of diagrams).

The port should keep the 24 tests as the first ports, and add a test per quirk in section 2.5 (the probe inputs are the tests).

## 6. Renderer

### 6.1 Pipeline

`diagram.renderer.draw(text, id, version, diag)` is called by `mermaidAPI.ts` after the `<style>` has been inserted as the svg's first child (`mermaidAPI.ts:596-601`). `draw` (`wardleyRenderer.ts:67`):

1. Reads the configuration (`getConfigValues`, `:52-65`) and the theme (`getTheme`, `:31-50`).
2. Reads the DB: `getWardleyData()` and `getDiagramTitle()` (`:77-78`).
3. Chooses the width and height: `size` if given, else the configuration (`:81-82`).
4. Selects the root svg by id and removes all its children (`:84-85`). **This is the step that removes the stylesheet** (section 6.9).
5. Sets the root's size and viewBox (`:87-88`).
6. Appends `g.wardley-map` and then `defs` (the three markers) as its sibling, and fills `g.wardley-map` in the DOM order of section 6.3.

Everything after step 5 is arithmetic on the DB state, and every element is appended in the order the code calls `append`. The DOM is therefore a function of the code's order, and the port reproduces it by emitting the same sequence.

### 6.2 Placement (projection, no layout)

Constants: `chartWidth = width - 2*padding` and `chartHeight = height - 2*padding` (`:147-148`); `projectX(v) = padding + v/100 * chartWidth` and `projectY(v) = height - padding - v/100 * chartHeight` (`:165-166`). Visibility grows upward and evolution to the right, so the origin of the plot is the bottom-left at `(padding, height - padding)`.

- Components, anchors, pipeline components and notes are at `(projectX(x), projectY(y))` (`:297-304`). Pipeline components have the parent's `y`.
- **Pipeline parents move.** For each pipeline with components, the parent's position becomes `x = (minX + maxX)/2` and `y = boxTop - squareSize/6`, where `boxTop = y - nodeRadius*4/2` and `squareSize = nodeRadius*1.6` (`:353-364`). The box is `minX - 15` to `maxX + 15`, and `nodeRadius*4` high (`:354-377`). This move happens in the pipeline step, before the links, the trends and the nodes are drawn, so they all use the moved position.
- Stage dividers are at `padding + boundary * chartWidth`. With no custom boundaries, or with a count that differs from the stages, they are equal: `i/n` (`:221-238`; probed: `stage-partial-boundaries` gives equal stages). A custom last boundary below 1 leaves a gap at the end (probed: `stage-last-boundary-below-1`, the last stage's label at 711.3 px).
- Link endpoints are shortened along the direction by a radius. The source uses `squareSize/√2` if it is a pipeline parent, else `nodeRadius`; the target does the same (`:410-457`).
- A link label sits 8 px to the side of the midpoint, with the normal `(dy/d, -dx/d)` (`:481-504`), and is rotated by `atan2(dy, dx)` in degrees, plus 180 when outside ±90 (`:509-529`).
- A trend starts at the node's position and ends at `(projectX(targetX), projectY(node.y))`, shortened by `nodeRadius + 2` when it is longer than that (`:540-567`). The end uses the DB `y`, not the moved parent position (section 6.8).
- Label offsets: for a component, `x = pos.x + (labelOffsetX ?? 8 (+10 with a strategy and no offset))` and `y = pos.y + (labelOffsetY ?? -8 (-10 with a strategy and no offset))` (`:785-808`). An anchor's label is at `x = pos.x + labelOffsetX` when set, else `pos.x`, and `y = pos.y + labelOffsetY` when set, else `pos.y - 3` (`:784-800`; probed: anchor at 530.4, 297).
- Annotations: each coordinate is `(projectX, projectY)`, and the number is centred on it (`:853-877`).

Probed values: a component at `[0.2, 0.4]` with the default size is at `(369.6, 451.2)`, and its label at `(377.6, 443.2)` (`layout-A`). The grid lines are at `x = 249` and `y = 426` for one quarter (`grid`). The inertia line is 6 px wide, from `x = pos.x + 21` (`inertia`).

### 6.3 DOM order

Probed with the sample in `probe-render2.mts` (render-out2.txt), abbreviated. The order is the order of the `append` calls.

```text
svg (root: width, style or height/width, viewBox; id, xmlns, role and aria-roledescription are the core's)
  g.wardley-map
    rect.wardley-background
    text.wardley-title                        (only when a title is set)
    g.wardley-axes
      line, line, text.wardley-axis-label.wardley-axis-label-x, text.wardley-axis-label.wardley-axis-label-y
    g.wardley-stages                          (always: the default stages when none are given)
      text.wardley-stage-label, line, text.wardley-stage-label, line, ...   (a line before each stage but the first)
    g.wardley-grid                            (only with showGrid: 3 vertical then 3 horizontal lines, per ratio)
    g.wardley-pipelines                       (only with pipelines)
      rect.wardley-pipeline-box               (one per pipeline with components)
    g.wardley-pipeline-links                  (only with pipelines)
      line.wardley-pipeline-evolution-link    (one per consecutive pair)
    g.wardley-links
      line.wardley-link [.wardley-link--dashed]   (one per valid link)
      text.wardley-link-label                     (one per valid link with a label)
    g.wardley-trends
      line.wardley-trend                      (one per trend)
    g.wardley-nodes
      g.wardley-node.wardley-node--{className}  (one per node, in map order)
        circle.wardley-{outsource|buy|build|market}-overlay   (for the strategy, first)
        circle                                (normal, pipeline children and components; not anchors, parents or market)
        line.wardley-market-line x3, circle.wardley-market-dot x3   (market only)
        rect                                  (pipeline parents: the square)
        line.wardley-inertia                  (with inertia)
        text.wardley-node-label               (always last)
    g.wardley-annotations                     (only with annotations)
      line.wardley-annotation-line            (per annotation with several coordinates)
      g.wardley-annotation > circle, text     (per coordinate)
      g.wardley-annotations-box               (only with a box)
        rect                                  (only when some annotation has text)
        text                                  (per annotation with text, sorted by number)
    g.wardley-notes                           (only with notes)
      text
    g.wardley-accelerators                    (only with accelerators)
      path, text                              (interleaved per accelerator)
    g.wardley-deaccelerators                  (only with deaccelerators)
      path, text
  defs
    marker#arrow-{id} > path                  (trend arrow)
    marker#link-arrow-end-{id} > path
    marker#link-arrow-start-{id} > path
```

Probed notes: the marker ids carry the diagram id, which is what keeps several diagrams on one page apart (`multi-diagram-id-uniqueness.spec.ts`). A box with annotations that have no text leaves an empty `g.wardley-annotations-box` (probed: `annotation-empty-text`).

### 6.4 Element by element

Attributes are the values the code sets, in the order of the calls. Colour names are the theme tokens of section 6.10; `W` and `H` are the size, `P` the padding, `R` the node radius, `S` the square size `1.6R`.

| Element                                                  | Attributes (call order)                                                                                                                                                                                                                                                                                                                                                  | Source                                                   |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------- |
| root `svg`                                               | `width="100%"` and `style="max-width: {W}px;"` (`useMaxWidth` true), or `height="{H}"` and `width="{W}"` (false); then `viewBox="0 0 {W} {H}"`                                                                                                                                                                                                                           | `setupGraphViewbox.js:23-46`, `wardleyRenderer.ts:87-88` |
| `g.wardley-map`                                          | none                                                                                                                                                                                                                                                                                                                                                                     | `:90`                                                    |
| `rect.wardley-background`                                | `width`, `height`, `fill=backgroundColor` (class first)                                                                                                                                                                                                                                                                                                                  | `:140-145`                                               |
| `text.wardley-title`                                     | `x=W/2`, `y=P/2`, `fill=axisTextColor`, `font-size=axisFontSize*1.05` (12.600000000000001 at 12), `font-weight=bold`, `text-anchor=middle`, `dominant-baseline=middle`; text                                                                                                                                                                                             | `:151-163`                                               |
| `g.wardley-axes` > `line` (bottom)                       | `x1=P`, `x2=W-P`, `y1=y2=H-P`, `stroke=axisColor`, `stroke-width=1`                                                                                                                                                                                                                                                                                                      | `:168-176`                                               |
| `line` (left)                                            | `x1=x2=P`, `y1=P`, `y2=H-P`, `stroke=axisColor`, `stroke-width=1`                                                                                                                                                                                                                                                                                                        | `:177-184`                                               |
| `text.wardley-axis-label-x`                              | `x=P+cw/2`, `y=H-P/4`, `fill=axisTextColor`, `font-size=axisFontSize`, `font-weight=bold`, `text-anchor=middle`; text `Evolution`                                                                                                                                                                                                                                        | `:189-198`                                               |
| `text.wardley-axis-label-y`                              | `x=P/3`, `y=P+ch/2`, same, `transform=rotate(-90 {P/3} {P+ch/2})`; text `Visibility`                                                                                                                                                                                                                                                                                     | `:199-212`                                               |
| `g.wardley-stages` > `line` (divider)                    | `x1=x2=start`, `y1=P`, `y2=H-P`, `stroke=#000`, `stroke-width=1`, `stroke-dasharray=5 5`, `opacity=0.8`                                                                                                                                                                                                                                                                  | `:246-258` (a literal `#000`)                            |
| `text.wardley-stage-label`                               | `x=centre`, `y=H-P/1.5`, `fill=axisTextColor`, `font-size=axisFontSize-2`, `text-anchor=middle`; text                                                                                                                                                                                                                                                                    | `:261-269`                                               |
| `g.wardley-grid` > `line`                                | vertical: `x1=x2`, `y1=P`, `y2=H-P`, `stroke=gridColor`, `stroke-dasharray=2 6`; horizontal: `x1=P`, `x2=W-P`, `y1=y2`, the same                                                                                                                                                                                                                                         | `:273-295`                                               |
| `rect.wardley-pipeline-box`                              | `x=minX-15`, `y=boxTop`, `width=maxX-minX+30`, `height=4R`, `fill=none`, `stroke=axisColor`, `stroke-width=1.5`, `rx=4`, `ry=4`                                                                                                                                                                                                                                          | `:366-377`                                               |
| `line.wardley-pipeline-evolution-link`                   | `x1,y1,x2,y2` (component centres), `stroke=linkStroke`, `stroke-width=1`, `stroke-dasharray=4 4`                                                                                                                                                                                                                                                                         | `:327-336`                                               |
| `line.wardley-link`                                      | `class` (`wardley-link` plus `wardley-link--dashed` for `-.->`), `x1,y1,x2,y2` (shortened), `stroke=linkStroke`, `stroke-width=1`, `stroke-dasharray=6 6` (dashed only; absent otherwise), `marker-end=url(#link-arrow-end-{id})` (forward or bidirectional; absent otherwise), `marker-start=url(#link-arrow-start-{id})` (backward or bidirectional; absent otherwise) | `:404-472`                                               |
| `text.wardley-link-label`                                | `x`, `y` (8 px beside the midpoint), `fill=axisTextColor`, `font-size=labelFontSize`, `text-anchor=middle`, `dominant-baseline=middle`, `transform=rotate({angle} {x} {y})`; text                                                                                                                                                                                        | `:475-530`                                               |
| `line.wardley-trend`                                     | `x1,y1` (origin), `x2,y2` (target, shortened), `stroke=evolutionStroke`, `stroke-width=1`, `stroke-dasharray=4 4`, `marker-end=url(#arrow-{id})`                                                                                                                                                                                                                         | `:569-582`                                               |
| `g.wardley-node`                                         | `class="wardley-node"` plus `wardley-node--{className}` when set, joined with a space                                                                                                                                                                                                                                                                                    | `:584-594`                                               |
| `circle.wardley-outsource-overlay`                       | `cx`, `cy`, `r=2R`, `fill=#666`, `stroke=componentStroke`, `stroke-width=1`                                                                                                                                                                                                                                                                                              | `:597-606`                                               |
| `circle.wardley-buy-overlay`                             | `r=2R`, `fill=#ccc`, `stroke=componentStroke`, `stroke-width=1`                                                                                                                                                                                                                                                                                                          | `:609-618`                                               |
| `circle.wardley-build-overlay`                           | `r=2R`, `fill=#eee`, `stroke=#000`, `stroke-width=1`                                                                                                                                                                                                                                                                                                                     | `:621-630`                                               |
| `circle.wardley-market-overlay`                          | `r=2R`, `fill=white`, `stroke=componentStroke`, `stroke-width=1`                                                                                                                                                                                                                                                                                                         | `:636-644`                                               |
| `circle` (node)                                          | `cx`, `cy`, `r=R`, `fill=componentFill`, `stroke=componentStroke`, `stroke-width=1`                                                                                                                                                                                                                                                                                      | `:647-658`                                               |
| `line.wardley-market-line` x3                            | triangle sides with `d = R*1.2`, `cos(π/6)`, `sin(π/6)`: top to bottom-left, bottom-left to bottom-right, bottom-right to top; `stroke=componentStroke`, `stroke-width=1`                                                                                                                                                                                                | `:666-696`                                               |
| `circle.wardley-market-dot` x3                           | `cx`, `cy` at the triangle's corners, `r=0.7R`, `fill=white`, `stroke=componentStroke`, `stroke-width=2`                                                                                                                                                                                                                                                                 | `:699-729`                                               |
| `rect` (pipeline parent)                                 | `x=cx-S/2`, `y=cy-S/2`, `width=S`, `height=S`, `fill=componentFill`, `stroke=componentStroke`, `stroke-width=1`                                                                                                                                                                                                                                                          | `:732-741`                                               |
| `line.wardley-inertia`                                   | `x1=x2=cx+off`, `y1=cy-h/2`, `y2=cy+h/2` with `off = (parent ? S/2 + 15 : R + 15)`, plus `R + 10` with a strategy, and `h = parent ? S : 2R`; `stroke=componentStroke`, `stroke-width=6`                                                                                                                                                                                 | `:744-777`                                               |
| `text.wardley-node-label`                                | `x`, `y` (section 6.2), `class`, `fill` (anchor `#000`; `evolved` evolutionStroke, unreachable; else componentLabelColor), `font-size=labelFontSize`, `font-weight` (anchor bold, else normal), `text-anchor` (anchor middle, else start), `dominant-baseline` (anchor middle, else auto); text is the label                                                             | `:779-823`                                               |
| `g.wardley-annotations` > `line.wardley-annotation-line` | `x1..y2`, `stroke=axisColor`, `stroke-width=1.5`, `stroke-dasharray=4 4`                                                                                                                                                                                                                                                                                                 | `:837-850`                                               |
| `g.wardley-annotation` > `circle`                        | `cx`, `cy`, `r=10`, `fill=white`, `stroke=axisColor`, `stroke-width=1.5`                                                                                                                                                                                                                                                                                                 | `:857-864`                                               |
| `g.wardley-annotation` > `text`                          | `x`, `y`, `text-anchor=middle`, `dominant-baseline=central`, `font-size=10`, `fill=axisTextColor`, `font-weight=bold`; text is the number                                                                                                                                                                                                                                | `:867-876`                                               |
| `g.wardley-annotations-box` > `rect`                     | `x`, `y`, `width`, `height`, `fill=white`, `stroke=axisColor`, `stroke-width=1.5`, `rx=4`, `ry=4`; inserted before the first text                                                                                                                                                                                                                                        | `:943-953`                                               |
| `g.wardley-annotations-box` > `text`                     | `x=boxX+10`, `y=boxY+10+(i+1)*16`, `font-size=11`, `fill=axisTextColor`, `text-anchor=start`, `dominant-baseline=middle`; text `{number}. {text}`                                                                                                                                                                                                                        | `:899-907`                                               |
| `g.wardley-notes` > `text`                               | `x`, `y`, `text-anchor=start`, `font-size=11`, `fill=axisTextColor`, `font-weight=bold`; text                                                                                                                                                                                                                                                                            | `:966-974`                                               |
| `g.wardley-accelerators` > `path`                        | `d` (the arrow: 60 by 30, head 20 wide), `fill=white`, `stroke=componentStroke`, `stroke-width=1`                                                                                                                                                                                                                                                                        | `:992-1008`                                              |
| `g.wardley-accelerators` > `text`                        | `x=accX+30`, `y=accY+30`, `text-anchor=middle`, `font-size=10`, `fill=axisTextColor`, `font-weight=bold`; text is the name                                                                                                                                                                                                                                               | `:1011-1019`                                             |
| deaccelerator                                            | the same, with the arrow pointing left from `(decX, decY)`                                                                                                                                                                                                                                                                                                               | `:1024-1064`                                             |
| `marker#arrow-{id}`                                      | `viewBox="0 0 10 10"`, `refX=9`, `refY=5`, `markerWidth=6`, `markerHeight=6`, `orient=auto-start-reverse`; `path d="M 0 0 L 10 5 L 0 10 z"`, `fill=evolutionStroke`, `stroke=none`                                                                                                                                                                                       | `:96-108`                                                |
| `marker#link-arrow-end-{id}`                             | `refX=9`, `refY=5`, `markerWidth=5`, `markerHeight=5`, `orient=auto`; path as above, `fill=linkStroke`                                                                                                                                                                                                                                                                   | `:111-123`                                               |
| `marker#link-arrow-start-{id}`                           | `refX=1`, `refY=5`, `markerWidth=5`, `markerHeight=5`, `orient=auto`; `path d="M 10 0 L 0 5 L 10 10 z"`, `fill=linkStroke`                                                                                                                                                                                                                                               | `:126-138`                                               |

Notes on the table:

- The accelerator path's `d` is a template literal with line breaks and indentation (`:992-1001`). An SVG parser ignores them, so a port may write the path compactly. A port that compares bytes must keep them.
- Numbers are written by d3 as `String(value)`, so `12.600000000000001` and `286.38301946629423` are the shortest round-trip strings. `NaN` is written as `NaN` (section 6.11). A negative zero would be written `0` (JavaScript); a port must do the same.
- `attr` with `null` removes the attribute: `marker-end` and `marker-start` on a link without that flow (probed: `flow-gt-arrow-no-marker`). The `stroke-dasharray` of a solid link is the same rule (read).
- A link between two points at the same position divides by zero and writes `NaN` (probed: `nan-links`, for the endpoints). The link label divides by the same distance, so it writes `NaN` too (read; no label was probed). The trend does not: `adjustedX2` falls back to the target at zero length (`:556-557`), so the trend line is drawn with finite values (probed: `trend-into-same-x`, one trend).

### 6.5 Text measurement

The renderer measures text in one place only: the annotation box (`wardleyRenderer.ts:896-922`). Each annotation line is drawn first at its unclamped position, then:

- `maxWidth` is the largest `getComputedTextLength()` of the lines (`:918-919`).
- `maxHeight` is the largest `getBBox().height` (`:920-921`).
- `boxWidth = maxWidth + 2*10 + 105` (`:925`), and `boxHeight = n*16 + 2*10 + maxHeight/2` (`:926`). The 105 is a fixed buffer (the comment says "for safety").
- The box is clamped to the plot's margins (`:928-935`).

Probed in the harness (the page's default font, section 8.1): the annotation box `annotation-box` gives a rect of width `220.625 = 95.625 + 125` and height `58.5 = 2*16 + 20 + 13/2`, placed at `(631.375, 493.5)`, which is the clamped corner (`maxX = 900 - 48 - 220.625`, `maxY = 600 - 48 - 58.5`). So the measures are `95.625` for `1. Critical component` at 11 px and `13` for the line height in the reference's font.

Nothing else is measured. Node labels, link labels, titles, stage labels and notes are placed without their widths, so their overlap is not avoided (read). That is what makes the layout a pure projection.

### 6.6 Layout

There is no layout algorithm. Positions are the projection of section 6.2, plus the pipeline parent's move. The only "layout" data is the DB's coordinates and the text measure of section 6.5.

### 6.7 d3 pieces

The renderer uses only `d3-selection`, through `select` (`selectSvgElement.ts:1`) and the `Selection` type (`wardleyRenderer.ts:14`):

| Method                                           | Uses                                                                                  | Notes                                                                                                          |
| ------------------------------------------------ | ------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `select('body').select('#{id}')`                 | `selectSvgElement.ts:14-20`                                                           | In `sandbox` the selection goes into the iframe's document (`:15-19`).                                         |
| `selectAll('*').remove()`                        | `:85`                                                                                 | Removes the stylesheet too (section 6.9).                                                                      |
| `append(tag)`                                    | 61 call sites                                                                         | Appends to the last selection.                                                                                 |
| `attr(name, value)`                              | 319 call sites                                                                        | A string, a number (written as `String(n)`), a function of the datum, or `null` (removes the attribute).       |
| `text(value)`                                    | 12 call sites                                                                         | Sets the text node; the value is written as a string, not as HTML.                                             |
| `selectAll(sel).data(array).enter().append(tag)` | `:404-408` (links), `:475-479` (link labels), `:569-573` (trends), `:585-589` (nodes) | Each datum is passed to the attribute functions. Each selection starts empty, so `enter()` is the whole array. |
| `filter(predicate)`                              | 13 call sites                                                                         | On `nodeEnter`, for each overlay, market, pipeline parent and inertia node.                                    |
| `insert('rect', 'text')`                         | `:944`                                                                                | Inserts before the first `text` child.                                                                         |
| `node()`                                         | `:916`                                                                                | The DOM element, for `getComputedTextLength` and `getBBox`.                                                    |
| `remove()`                                       | `:85`                                                                                 |                                                                                                                |

Without d3, the port needs: an element tree with ordered children, attribute maps that keep insertion order (the output order), a `remove` of all children, a `filter` over a list of nodes, and the `insert before` of one element.

### 6.8 Root size and viewBox

- `useMaxWidth` (default `true`) selects `width="100%"` and `style="max-width: {W}px;"`. With `false`, `height="{H}"` and `width="{W}"` are set and there is no style (probed: `maxwidth-false`).
- `W` and `H` are the `size` values if given, else the configuration (`:81-82`). The viewBox is `0 0 W H` (`:88`). The root `id`, `xmlns`, `role` and `aria-roledescription` (= the diagram type `wardley`) come from the core (probed: the render output).
- `setupGraphViewbox` (the bounding-box version) is not used.

### 6.9 The stylesheet (dropped)

`styles.ts` returns a CSS string for `.wardley-*` classes, built from the theme's `wardley` tokens (section 6.10). `mermaidAPI.ts:327-333` calls it on every render, and `mermaidAPI.ts:596-601` inserts the result as a `<style>` as the svg's first child. Then `draw` removes every child (`wardleyRenderer.ts:85`). The output has no `<style>` element (probed in `probe-style.mts` and `probe-svg.mts`: the wardley render has none in strict or loose security, while a flowchart keeps its `<style>` in both).

Because the stylesheet is gone, the attributes are the appearance. This matters for the rules the stylesheet would have changed if it had survived (read, not probed, since it does not survive):

- `.wardley-link--dashed { stroke-dasharray: 4 4 }` (`styles.ts:55-57`) would override the attribute `6 6`. The output has `6 6` (probed: `dashed-link`).
- `.wardley-node circle { fill; stroke }` (`styles.ts:45-48`) would paint the four overlay circles with the component fill. The output keeps `#666`, `#ccc`, `#eee`, `white` (probed: `overlays`).
- `.wardley-node-label { fill }` (`styles.ts:49-51`) would make anchor labels take the component label colour. The output keeps `#000` (probed: `anchor-label`).

If a port emits CSS, these three rules must be left out, or the attributes must be made equal to them. The recommendation is in section 8.3 (Q2).

### 6.10 Theme variables the renderer reads

`getTheme` (`wardleyRenderer.ts:31-50`) reads `getConfig().themeVariables`, the full resolved theme of the current render. For each token it takes `themeVariables.wardley?.X`, else a fallback, which is another theme token, else a constant.

| Token                         | Read at  | Fallback chain                    | Constant                   |
| ----------------------------- | -------- | --------------------------------- | -------------------------- |
| `wardley.backgroundColor`     | `:34`    | `themeVariables.background`       | `#fff`                     |
| `wardley.axisColor`           | `:35`    |                                   | `#000`                     |
| `wardley.axisTextColor`       | `:36-37` | `themeVariables.primaryTextColor` | `#222`                     |
| `wardley.gridColor`           | `:38`    |                                   | `rgba(100, 100, 100, 0.2)` |
| `wardley.componentFill`       | `:39`    |                                   | `#fff`                     |
| `wardley.componentStroke`     | `:40`    |                                   | `#000`                     |
| `wardley.componentLabelColor` | `:41-42` | `themeVariables.primaryTextColor` | `#222`                     |
| `wardley.linkStroke`          | `:43`    |                                   | `#000`                     |
| `wardley.evolutionStroke`     | `:44`    |                                   | `#dc3545`                  |
| `wardley.annotationStroke`    | `:45`    |                                   | `#000`                     |
| `wardley.annotationTextColor` | `:46-47` | `themeVariables.primaryTextColor` | `#222`                     |
| `wardley.annotationFill`      | `:48`    | `themeVariables.background`       | `#fff`                     |

The global tokens read: `themeVariables.background` (`:34`, `:48`) and `themeVariables.primaryTextColor` (`:37`, `:42`, `:47`). Constants written in the code, not tokens: `#000` for the stage dividers (`:254`), the overlay fills `#666`, `#ccc`, `#eee` and `white` (`:604`, `:616`, `:628`, `:642`), the build overlay's stroke `#000` (`:629`), the anchor label `#000` (`:815`), and `white` for the market dots (`:705`), the annotation circles (`:862`), the annotations box (`:949`) and the accelerator arrows (`:1006`, `:1051`).

Probed under the harness's `redux-color`: component fill `#fff`, component stroke `#000`, background `#ffffff` (from `background`), axis text `#28253D` (from `primaryTextColor`), title `#28253D`, grid `rgba(100, 100, 100, 0.2)`, link stroke `#000` (`theme-redux-color`, `grid`, `dashed-link`). Under `redux-dark-color` the background is `#333`, the title `#e0dfdf`, the component fill `#fff` and the stroke `#000`, so a dark map gets white nodes (`probe-dark.mts`). The reason is that `redux-color` and `redux-dark-color` define no `wardley` tokens (`theme-redux-color.js`, `theme-redux-dark-color.js`, and the grep of the whole `src` tree, which finds `wardley` only in the five themes listed in section 1).

The `themeVariables.wardley` tokens of the five themes that define them: `theme-base.js:287-301`, `theme-dark.js:351-365` (`wardleyEvolutionColor` `#ff6b6b`), `theme-default.js:352-366` and `theme-forest.js:324-338` (each `wardleyEvolutionColor` `#dc3545`, except the dark one), and `theme-neutral.js:359-373`. Each defines the twelve tokens of the table as `this.wardley?.X || <a global token>`. The renderer reads `wardleyEvolutionColor` nowhere; only the theme file uses it to make `evolutionStroke`.

`styles.ts` reads the same twelve tokens, from `cleanAndMerge(getThemeVariables() of the default theme, config.themeVariables).wardley` merged with its own `wardley` option (`styles.ts:24-27`). For a theme without `wardley`, the stylesheet's colours come from the default theme, while the attributes use the fallbacks above. The two disagree, but since the stylesheet is dropped, only the attributes count (section 6.9).

### 6.11 Configuration the renderer reads

`getConfigValues` (`wardleyRenderer.ts:52-65`) reads `getConfig()['wardley-beta']` (`:53`), the section of the resolved configuration. Each key has a `??` default. The schema's defaults are the same (`config.schema.yaml`), and the type is `WardleyDiagramConfig` (`config.type.ts:2465-2498`).

| Key               | Read at | Default | Schema line                      | Used for                                                                                                                                                    |
| ----------------- | ------- | ------- | -------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `width`           | `:55`   | 900     | `3027`                           | The canvas width, unless `size` overrides it (`:81`).                                                                                                       |
| `height`          | `:56`   | 600     | `3032`                           | The canvas height, unless `size` overrides it (`:82`).                                                                                                      |
| `padding`         | `:57`   | 48      | `3037`                           | The margin of the plot, the title's y (`P/2`), the axis labels, and the annotation box's margin.                                                            |
| `nodeRadius`      | `:58`   | 6       | `3042`                           | The circles, the square (`1.6R`), the overlays (`2R`), the market dots (`0.7R`) and the triangle (`1.2R`), the pipeline box (`4R`), and the link endpoints. |
| `nodeLabelOffset` | `:59`   | 8       | `3047`                           | The default label offset (`:787`, `:801`).                                                                                                                  |
| `axisFontSize`    | `:60`   | 12      | `3052`                           | The title (`1.05×`), the axis labels, and the stage labels (`-2`, `:267`).                                                                                  |
| `labelFontSize`   | `:61`   | 10      | `3057`                           | The node labels (`:819`) and the link labels (`:506`).                                                                                                      |
| `showGrid`        | `:62`   | `false` | `3061`                           | The grid group (`:273`).                                                                                                                                    |
| `useMaxWidth`     | `:63`   | `true`  | base `470` (`BaseDiagramConfig`) | The root's size attributes (`:87`, `setupGraphViewbox.js:25-31`).                                                                                           |

Also read, outside the wardley code:

- `securityLevel`, in `selectSvgElement.ts:12`, for the sandbox branch.
- `getConfig().themeVariables`, in `wardleyRenderer.ts:32`, and `getConfigAPI().themeVariables` and the default theme in `styles.ts:24-26`.
- `look` and `fontFamily` are not read by this type. The docs say `handDrawn` is not supported (`docs/syntax/wardley.md:461`), and the renderer has no rough branch (read).

Fixed values, not configuration: the annotation numbers (10), the annotation lines (11), the notes (11), the accelerator labels (10), the stroke widths (1, 1.5, 2, 6), the arrow sizes (60 by 30, head 20, arrow 8), the buffer (105), the box's line height (16), the pipeline padding (15), the trend's shortening (`R + 2`), the link label offset (8), the grid ratios (1/4, 2/4, 3/4), and the default stages (`Genesis`, `Custom Built`, `Product`, `Commodity`, `:12`).

The section key is `wardley-beta`, not `wardley` (`diagramConfigKeys.ts:15`; `config.type.ts:347`; `config.schema.yaml:430`). `wardleyDb.getConfig` (`wardleyDb.ts:16-18`) reads the same key, but nothing in `packages/mermaid/src` calls it (grep).

### 6.12 Randomness, the clock, and the DOM

- **Randomness and the clock.** None. `draw` calls neither `Math.random` nor `Date` (checked by grep). The `Math` calls are `min`, `max`, `sqrt` (distances), `atan2` (the link label angle, `:523`), `cos` and `sin` (the triangle, `:671-694`), and `PI`.
- **Floating point.** `atan2`, `cos` and `sin` may differ in the last bit between V8 and a Rust `libm`. That changes only the strings of the rotation and the triangle's coordinates by about 1e-15, which the geometry tolerance accepts and a byte comparison would not (unverified against a Rust `libm`).
- **DOM dependencies a server renderer has to replace:**
  1. `select` of the root by id, in the body or the sandbox iframe (`selectSvgElement.ts`).
  2. The removal of the children of the root (`:85`), which is also the stylesheet's removal.
  3. `getComputedTextLength` and `getBBox` of the annotation lines (`:918`, `:920`), which need the font's advances and the line box (section 6.5).
  4. `text()` writes, which escape `&`, `<` and `>` in the output, as the serialiser does for every renderer.
  5. The core's serialisation and `DOMPurify.sanitize` in strict mode (`mermaidAPI.ts`, `serializeSvg`), which keep the `NaN` attributes and the `marker` references (probed).

## 7. Fixtures

Ten fixtures, all in `e2e/diagrams/wardley/pr-check/` (the other two folders hold only `.gitkeep`). The harness renders each with the front matter and the forced directive (`patchText`).

| Fixture                                      | Nodes (probed) | What it exercises                                                                                                                      |
| -------------------------------------------- | -------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| `1-should-render-tea-shop`                   | 9              | Title, `size [1100, 800]`, two anchors, components, label offsets (3), eight arrows, two `evolve`, three notes.                        |
| `2-should-render-data-evolution-stages`      | 5              | Four plain evolution stages (no boundaries, no second names), four arrows, two `evolve`.                                               |
| `3-should-render-pipelines`                  | 6              | One pipeline (`Kettle`, three components with label offsets), its box and its evolution links, two arrows.                             |
| `4-should-render-link-types-and-annotations` | 6              | Arrows `+>`, `+<>`, `+<`, `+'backup'>`, and `;` labels (five labels). No annotation statement, despite the name.                       |
| `5-should-render-custom-canvas-size`         | 5              | `size [600, 800]`, one `evolve`.                                                                                                       |
| `6-should-render-gpt-tokeniser-architecture` | 36             | The largest: three pipelines, `annotations [1, 0]` (integers), five annotations, a note, a deaccelerator, 22 label offsets, 23 arrows. |
| `theme-test-base`                            | 5              | Front matter `theme: base`. Harness: `redux-color` (see below).                                                                        |
| `theme-test-dark`                            | 5              | `theme: dark`. Harness: `redux-dark-color`.                                                                                            |
| `theme-test-forest`                          | 5              | `theme: forest`. Harness: `redux-color`.                                                                                               |
| `theme-test-neutral`                         | 5              | `theme: neutral`. Harness: `redux-color`.                                                                                              |

The harness forces every fixture's theme to a shipped one (`forcedDirective`, `tools/port/geometry/common.ts:79-87`): a name ending in `dark` becomes `redux-dark-color`, and the rest `redux-color`. So none of the four theme fixtures renders with its own theme, and none of them exercises the `wardley` tokens of `base`, `dark`, `forest` or `neutral`.

Not covered by any fixture (each probed through the scratch inputs unless marked):

- Accelerators (`accelerator`), inertia (`inertia` in either form), decorators (`build`, `buy`, `outsource`, `market`), and the market triangle.
- The dashed arrow `-.->`.
- Custom stage boundaries (`@`) and second names (`/`). The equal-stage fallback for a partial set of boundaries is probed, the rest is read.
- `showGrid`, `useMaxWidth: false`, `width` and `height` in the configuration.
- Empty annotation text, duplicate names, unknown link endpoints, a link between two points at one position (`NaN`), a pipeline whose parent is an anchor, a title with `%%`, and `Wardley-Beta` (rejected, probed).

The harness's `geometry` check will therefore not see these features unless new fixtures are added (section 8.3, Q5).

## 8. Risks, effort, open questions

### 8.1 Risks

| Risk                                     | Level       | Why                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ---------------------------------------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| The font of the text                     | High        | The reference writes no `font-family` for the wardley text (the stylesheet that names it is dropped), so the text takes the page default. In the harness that is `Times` (probed for every label and the annotation line). The annotation box's size depends on it: 95.625 px in Times, 101.75 px in Lexica Ultralegible (`probe-font.mts`). Ferrigura ships no Times, and the default serif of a user's browser is not reproducible. Section 8.3, Q1. |
| The lexer's keyword and prefix rules     | Medium      | About thirty quirks (section 2.5), each a rule of the token order (section 2.2). They are mechanical once the order is copied, but each needs a test.                                                                                                                                                                                                                                                                                                  |
| The DOM order and the attributes         | Medium      | About forty element kinds and attribute sets, with no stylesheet to fall back on (section 6.4). A missed attribute is visible. The DOM outline of section 6.3 is the first test.                                                                                                                                                                                                                                                                       |
| The theme fallbacks                      | Medium      | The redux themes have no `wardley` tokens, so the output is the fallbacks (section 6.10). A typed token view (decision [0009](../decisions/0009-configuration-resolved-once.md)) must allow the absent group, or the redux output changes.                                                                                                                                                                                                             |
| `NaN` and negative zero                  | Low         | Links between coincident points write `NaN` (probed). A JavaScript negative zero writes `0`. A Rust formatter must do both.                                                                                                                                                                                                                                                                                                                            |
| The pipeline's move of the parent        | Low, subtle | The move happens before the links, trends and nodes are drawn, and the trend's end does not move with it (probed: `pipeline-trend`, the trend slants). A port that moves the parent later draws different trends.                                                                                                                                                                                                                                      |
| Duplicate declarations and the map order | Low         | The first position is kept and the last values are used (D1, section 3.2). Insertion order must be a map that keeps the first position.                                                                                                                                                                                                                                                                                                                |
| Float text                               | Low         | Shortest round-trip, as section 6.4 notes.                                                                                                                                                                                                                                                                                                                                                                                                             |

### 8.2 Effort

A rough estimate in Rust lines, not a measurement:

| Part                                                                                    | Lines        |
| --------------------------------------------------------------------------------------- | ------------ |
| Scanner with the token order of section 2.2, and the regexes as functions               | 300 to 400   |
| Hand-written parser: the statements of section 2.3, the error positions                 | 350 to 450   |
| Value conversion and the header texts (section 2.4)                                     | 80 to 120    |
| DB and builder: the calls of section 3.1, the checks, the pipelines and the flows       | 350 to 450   |
| Model and exporter (`tools/port/export-wardley.ts`, as the architecture exporter)       | 150 to 200   |
| Renderer: projection, the element tree, the elements of section 6.4, the annotation box | 900 to 1,100 |
| Typed configuration and the theme view (section 6.10, 6.11)                             | 120 to 180   |
| Tests: the 24 specs, one test per quirk, the DOM outline                                | 500 to 700   |

The total is 2,750 to 3,600 lines, with the renderer about a third of it. The renderer's stage waits for issue [#4](https://github.com/sebastian-software/ferrigura/issues/4) (P3-PLAN.md), so the parser and the DB can start first.

### 8.3 Open questions

- **Q1. Which font does the wardley text get?** The reference uses the page default (`Times` in the harness), which no ferrigura face matches. Options: (a) ship a face with the same advances as Times (Liberation Serif or Tinos are metric-compatible, but would need checking against the 95.625 px line) and use it for the wardley text only; (b) write the configured `fontFamily` as a `font-family` attribute, which diverges from the reference's output; (c) keep the reference's text and exclude the annotation box from the geometry check with a named reason. This note recommends deciding before the renderer stage, because it sets the test for the box.
- **Q2. Emit the stylesheet or not?** The reference drops it (section 6.9). Recommendation: do not emit it, port `styles.ts` as a function that is not attached, and record the reason in the port's plan. Emitting it would change the dashed links, the overlay fills and the anchor labels (section 6.9).
- **Q3. Keep the redux fallbacks?** Recommendation: yes. The typed token view has an optional `wardley` group for each theme, with the fallbacks of section 6.10 used when it is absent. Adding wardley tokens to `redux-color` or `redux-dark-color` would change the output, and the port would then differ from the reference.
- **Q4. Reproduce the quirks or fix them?** Recommendation: reproduce all of section 2.5 and record each in the port's plan, with its probe as the test, because the differential is the acceptance test. The quirks that look like bugs (the prefix rule for names, the trend that does not move with the pipeline parent, the `NaN` links, the `%%` cut, the flow from a `<` in a label) should be reported to the reference's owner separately.
- **Q5. Add fixtures for the features no fixture covers?** Section 7 lists them. Recommendation: a set of probe inputs under `tools/port/geometry/probes/wardley` in the style of architecture's (the 13 hand-written and 12 generated probes there), covering the accelerators, the inertia and the decorators, the dashed arrow, the boundaries and second names, the grid, the `useMaxWidth` and size settings, and the quirks that change the drawing.
- **Q6. Compare parse errors?** The differential could compare acceptance only, as architecture does (`architecture/PLAN.md`, section 11). Recommendation: yes, and the message texts stay ferrigura's.
- **Q7. Detect the header case-insensitively and then reject `Wardley-Beta`?** Recommendation: yes (section 2.1), which keeps the reference's behaviour: the detector accepts and the parser rejects.
- **Q8. Which names does the canonical model use for the chart coordinates?** The DB stores `x` as the evolution and `y` as the visibility, while the statements write `[visibility, evolution]`, and the grammar's own field names for the annotation and accelerator coordinates are `x` and `y` in the statement order (section 3.1). Recommendation: keep the DB's `x` and `y` in the model, as section 4 proposes, and say so in the model's page.
- **Q9. Is the DB state the whole differential?** There is no `getData` layer. Recommendation: yes, the model of section 4 is the differential, and the renderer is checked by `port:geometry` only.

Summary for the coordinator: the parser and the DB are ready to start from this note once Q4 and Q7 are answered. Q1 and Q2 must be answered before the renderer stage, and Q3 before the typed theme view of issue #4.
