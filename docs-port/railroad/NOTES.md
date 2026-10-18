# P3 research: railroad

Status: research only. Nothing in the repository changed except this file, and nothing is committed. Reference: `packages/mermaid/src` and the Langium grammars in `packages/parser/src/language/railroad*` at HEAD `f3a1a94b0` (branch `sebastian/port-p3`). Guide: [../P2-RESEARCH.md](../P2-RESEARCH.md). Plan: [../P3-PLAN.md](../P3-PLAN.md). Model: [../kanban/NOTES.md](../kanban/NOTES.md) and [../architecture/NOTES.md](../architecture/NOTES.md).

Every rule cites the reference as `diagrams/railroad/<file>:<line>` unless a file is named in full. **Probed** means the rule was run through the reference's own code, as described in the next paragraph. **Unverified** means read or inferred only.

How the probes ran. Parser and DB probes loaded the reference's modules through `tools/port/reference.ts` (Vite SSR, with jsdom for DOMPurify), called `parse()` on each input, and read `db.getRules()`, `getTitle()`, `getAccTitle()` and `getAccDescription()`. The input went through `cleanupComments` and `encodeEntities` plus a newline, as `Diagram.fromText` does (`Diagram.ts:77`). Front matter removal and `processDirectives` were not run, so a front matter `title` is not in these probes, and the lexer hides front matter and directives anyway. The lexer token tables came from `createRailroad*Services()` in the built `packages/parser/dist`. Renderer probes ran the built bundle `packages/mermaid/dist` in Chromium through the geometry harness's page (`tools/port/geometry/common.ts`: `withPage`, `renderReference`, `patchText`), with the harness's forced settings (`htmlLabels: false`, Lexica Ultralegible, `securityLevel: strict`, and a shipped theme) unless a row says otherwise. Probe material is outside the repository in `/private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/p3-railroad/`: `parse-cases.mts` (the inputs, with ids `R-`, `E-`, `A-`, `P-`, `X-`), `probe-parse.mts` and `parse-out.json` (the results), `lexer-defs.mjs` (the token tables), `render-probe.mts` and `render-probe2.mts` with `svg/` (the SVG of each probe) and `results.json`, `analyse.cjs` and `outline.cjs` (summaries and DOM outlines of an SVG), and `vitest.json` (the spec run).

## 0. Summary

Railroad is four syntaxes over one model, one DB and one renderer. Each syntax has a Langium grammar that the port must reproduce by hand (decision [0002](../decisions/0002-hand-written-parsers.md)). The renderer is the type's own arithmetic, with no layout library: the sizes come from the measured width and height of each text, and from constants. So the layout-engine question of P3-PLAN is answered for railroad: **no engine and no library port is needed**.

| Item                | Value                                                                                                                                                                                                                                                                                                                                         |
| ------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Reference, non-spec | 2,140 lines under `packages/mermaid/src/diagrams/railroad/`: `railroadRenderer.ts` 669, `styles.ts` 368, `railroadTypes.ts` 203, `railroadDb.ts` 155, the four parsers 593 (113, 169, 151, 160), and eight detector and diagram files 152. This matches P3-PLAN                                                                               |
| Grammars            | Four Langium grammars under `packages/parser/src/language/`: `railroad/railroad.langium` (113 lines), `railroad-ebnf/railroad-ebnf.langium` (166), `railroad-abnf/railroad-abnf.langium` (114), `railroad-peg/railroad-peg.langium` (111). Each has a `module.ts`, `tokenBuilder.ts` and `valueConverter.ts`                                  |
| Specs               | 11 files under `diagrams/railroad/`, 135 tests. A vitest run at HEAD passes all 135 (probed). `packages/parser/tests/railroad.test.ts` adds 7 tests (110 lines)                                                                                                                                                                               |
| Fixtures            | 9 `.mmd` files in `e2e/diagrams/railroad/pr-check/`: 5 IR, 2 EBNF, 1 ABNF and 1 PEG (section 7). No nightly or visual-check fixtures                                                                                                                                                                                                          |
| Parser              | Four Langium grammars, each with its own lexer (keywords first, then terminals, first match wins with `LONGER_ALT` for word-like keywords). Hand port with one scanner and four small grammars. Lexer errors and parser errors both reject the input                                                                                          |
| DB                  | One module (`railroadDb.ts`) that holds the title, the accessibility texts and the rule list. Every name and value is sanitised with DOMPurify on the way in. No defaults beyond empty strings                                                                                                                                                |
| Renderer            | Own code. Each rule is a horizontal arrangement of elements (terminal, nonterminal, special, sequence, choice, optional, repetition). Sizes are measured text plus `padding`, and the rest is arithmetic on `arcRadius`, `horizontalSeparation` and `verticalSeparation`. The only DOM measurement is `getBBox` of text                       |
| Layout              | None: no `railroad-diagrams` package and no d3 layout. Confirmed by the imports (only mermaid modules, `@mermaid-js/parser` and a type-only `d3`)                                                                                                                                                                                             |
| Shared or separate  | One DB, one renderer, one styles provider, one config section (`railroad`, shared by the four ids through `diagramConfigKeys.ts:12-14`). The four parsers differ only in grammar and in how they map the AST to `ASTNode`                                                                                                                     |
| Riskiest parts      | (1) Text metrics: every size comes from `getBBox` of the text, to 1/64 px (section 6.4). (2) The four lexers, with the `;` and `title` quirks (section 2.9). (3) Visible text: DOMPurify output and the entity placeholders decoded at the SVG string (sections 3.2, 6.1). (4) Theme-derived defaults and the config sanitisers (section 6.8) |
| Effort              | Medium. About 2,050 to 2,650 lines of Rust with tests, of which 1,650 to 2,050 are code (section 8.2)                                                                                                                                                                                                                                         |

## 1. Sources

| File (under `packages/mermaid/src/diagrams/railroad/` unless named) | Lines             | What it holds                                                                                                                                                                                                            |
| ------------------------------------------------------------------- | ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `railroadDetector.ts`                                               | 13-15             | Detector `^\s*railroad-beta`, case-insensitive (`/i`). Loader 20-23, export 28-32, id `railroad`                                                                                                                         |
| `railroadDiagram.ts`                                                | 11-16             | The definition: `parser`, `db`, `renderer`, `styles: getStyles`                                                                                                                                                          |
| `ebnfDetector.ts`, `abnfDetector.ts`, `pegDetector.ts`              | 9-11              | Detectors `railroad-ebnf-beta`, `railroad-abnf-beta`, `railroad-peg-beta`, all `/i`. Ids `railroadEbnf`, `railroadAbnf`, `railroadPeg`, each set on line 7                                                               |
| `ebnfDiagram.ts`, `abnfDiagram.ts`, `pegDiagram.ts`                 | 7-11              | The same four keys as `railroadDiagram.ts`, with the matching parser                                                                                                                                                     |
| `parser/railroadParser.ts`                                          | 26-73             | `transformExpression`: `RailroadTerminalExpr` 28-32, nonterminal 33-37, special 38-42, sequence 43-46 (one element collapses, 45), choice 47-50 (49), optional 51-55, `+` 56-62 (min 1), `*` 63-69                       |
|                                                                     | 75-90             | `transformRule` (75-80) and `populateDb` (82-90): `populateCommonDb` (83), the title (85-87), then `addRule` for each rule (89)                                                                                          |
|                                                                     | 92-111            | `parser.parse`: `db.clear()` (94), the lexer and parser error check (98-100), `populateDb` (105)                                                                                                                         |
| `parser/ebnfParser.ts`                                              | 28-52             | `transformChoice` (28-39, one alternative collapses, 31-33), `transformSequence` (41-52, 44-46)                                                                                                                          |
|                                                                     | 54-88             | `transformPrimary`: terminal 56-60, nonterminal 61-65, special 66-70, group 71-72, optional 73-77, `{ }` 78-84 (min 0)                                                                                                   |
|                                                                     | 90-129            | `transformPostfix`: `?` 92-96, `*` 97-103, `+` 104-110, exception `- X` 111-119 (becomes a sequence with the terminal `"-"`, 115-117), `transformTerm` 125-129 (postfixes in order)                                      |
|                                                                     | 131-163           | `transformRule` (131-136), `populateDb` (138-146), `parse` (149-163, error 154-156)                                                                                                                                      |
| `parser/abnfParser.ts`                                              | 25-61             | `transformAlternation` (25-36), `transformConcatenation` (38-49), `parseRepeat` (51-61): `*` splits min and max, a bare number is exact                                                                                  |
|                                                                     | 63-111            | `transformElement` (63-82: `0*1` becomes optional, 72-74; otherwise repetition 76-81), `transformPrimary` (84-111): string 86-90, numval 91-95 (kept as text), rule name 96-100, groups 101-107                          |
|                                                                     | 113-149           | `transformRule`, `populateDb`, `parse` (error 136-138)                                                                                                                                                                   |
| `parser/pegParser.ts`                                               | 25-64             | `transformOrderedChoice` (25-36), `transformSequence` (38-49), `transformPrefix` (51-64): `&` and `!` become a special node whose text is the label (58-63)                                                              |
|                                                                     | 66-96             | `nodeToLabel` (66-77: terminal `"v"` 68-69, name 70-71, special text 72-73, anything else `(...)` 74-75), `transformSuffix` (79-96: `?`, `*`, `+`)                                                                       |
|                                                                     | 98-158            | `transformPrimary` (98-120: literal 100-104, identifier 105-109, group 110-111, `.` as special 112-116), `transformRule` (122-127), `populateDb` (129-137), `parse` (140-158, error 145-147)                             |
| `railroadTypes.ts`                                                  | 7-55              | The AST: `TerminalNode`, `NonTerminalNode`, `SequenceNode`, `ChoiceNode`, `OptionalNode`, `RepetitionNode` (min, max, and an optional `separator` that no parser sets, 37), `SpecialNode`                                |
|                                                                     | 60-64             | `RailroadRule`: `name`, `definition`, and `comment` (63), which no grammar sets                                                                                                                                          |
|                                                                     | 69-167            | `RailroadStyleOptions` (24 keys) and `DEFAULT_RAILROAD_CONFIG` (123-167), equal to the schema defaults                                                                                                                   |
|                                                                     | 190-203           | `RailroadDB` (`getRule` at 196 is not called by the renderer)                                                                                                                                                            |
| `railroadDb.ts`                                                     | 13-15             | `sanitizeText` through `getConfig()` (`commonSanitizeText`)                                                                                                                                                              |
|                                                                     | 17-38             | `sanitizeAstNode`: terminal value 20, nonterminal name 22, special text 36, and each child                                                                                                                               |
|                                                                     | 43-51             | `clear`: title, accessibility texts, rules and the rule map are reset, then `commonClear()` (49)                                                                                                                         |
|                                                                     | 56-59             | `setTitle` (sanitised)                                                                                                                                                                                                   |
|                                                                     | 71-88             | `addRule`: name 74, definition 75, comment 76 (never set), a duplicate warning (82-84), push (86) and map (87)                                                                                                           |
|                                                                     | 107-125           | `setAccTitle` (sanitise, then `^\s+` removed, 108), `setAccDescription` (sanitise, then `\n\s+` to `\n`, 123)                                                                                                            |
|                                                                     | 134-153           | `setDiagramTitle = setTitle`, and the `db` object                                                                                                                                                                        |
| `railroadRenderer.ts`                                               | 17-56             | `PathBuilder` (M, L, H, V, A and the joined string, trimmed at 54)                                                                                                                                                       |
|                                                                     | 61-93             | `RailroadRenderer`: the constructor (66-69), `measureText` (74-93, `getBBox` at 86, cache 64, 75-77, 90)                                                                                                                 |
|                                                                     | 98-530            | The element renderers, section 6.2 gives each line range: terminal 98, nonterminal 136, sequence 167, choice 222, optional 318, repetition 371, special 473, dispatch 504                                                |
|                                                                     | 535-623           | `renderRule` (535-604), `renderDiagram` (609-623)                                                                                                                                                                        |
|                                                                     | 626-666           | `configureRailroadSvgSize` (626-633), `draw` (638-666: class 643, `getConfigAPI().railroad` 644-645, empty diagram 650-654, renderer 656, size 659)                                                                      |
| `styles.ts`                                                         | 18-46             | `COLOR_VALUE_PATTERN` (18-19), `FONT_FAMILY_PATTERN` (20), `RAILROAD_STYLE_OPTION_KEYS` (21-46)                                                                                                                          |
|                                                                     | 48-83             | `isRailroadStyleOptions`, `extractRailroadOverrides`, `extractThemeOverrides`                                                                                                                                            |
|                                                                     | 85-123            | Sanitisers: colour (85-92), font family (94-101), number (103-112: finite and not negative), theme font size (114-123)                                                                                                   |
|                                                                     | 125-187           | `buildThemeDefaults`: the theme variables mapped to the style keys (section 6.7)                                                                                                                                         |
|                                                                     | 189-254           | `buildRailroadStyleOptions`: theme variables (193-197), `currentConfig.railroad` (199-202), the output (204-253)                                                                                                         |
|                                                                     | 259-366           | `getStyles`: the stylesheet (280-365). Section 6.6 lists the rules                                                                                                                                                       |
| `../../defaultConfig.ts`                                            | 295-314           | The `railroad` section: colour and typography keys set to `undefined`, so the theme supplies them (297-313). `strokeWidth`, `padding`, `arcRadius` and the others keep the schema defaults                               |
| `../../schemas/config.schema.yaml`                                  | 434               | The top-level `railroad` key, referring to `RailroadDiagramConfig`                                                                                                                                                       |
|                                                                     | 457-470           | `BaseDiagramConfig` (457) and its `useMaxWidth` (464, default `true` at 470)                                                                                                                                             |
|                                                                     | 3104-3214         | `RailroadDiagramConfig`: the 24 keys (lines 3111, 3115, 3120, 3125, 3130, 3135, 3140, 3144, 3148, 3152, 3156, 3160, 3164, 3168, 3172, 3177, 3181, 3185, 3189, 3193, 3197, 3201, 3205, 3209), defaults in the same blocks |
| `../../config.type.ts`                                              | 480, 2541-2640    | `BaseDiagramConfig` (480) and `RailroadDiagramConfig` (2541), the same keys as the schema                                                                                                                                |
| `../../diagram-api/diagramConfigKeys.ts`                            | 12-14             | `railroadAbnf`, `railroadEbnf` and `railroadPeg` read the `railroad` section                                                                                                                                             |
| `../common/populateCommonDb.ts`                                     | 4-14              | `accDescr`, `accTitle` and `title` set through the DB's setters, when non-empty                                                                                                                                          |
| `../../themes/theme-default.js`                                     | 503-507           | `getThemeVariables(userOverrides)`: a new `Theme` each call, so `buildRailroadStyleOptions` makes one per call                                                                                                           |
| `../../mermaidAPI.ts`                                               | 319-334           | `createUserStyles`: `getStyles(graphType, css, {...config.themeVariables, theme, look}, svgId)`, then the scope of the stylesheet to `#id`                                                                               |
|                                                                     | 359               | `cleanedUpSvg = decodeEntities(cleanedUpSvg)`: the whole SVG string is decoded (section 6.1)                                                                                                                             |
|                                                                     | 621-622           | The accessibility title and description read from the DB                                                                                                                                                                 |
| `../../utils.ts`                                                    | 874-877, 904-906  | `encodeEntities` (applied to the input) and `decodeEntities` (applied to the output)                                                                                                                                     |
| `../../Diagram.ts`                                                  | 49, 53, 77, 83    | `db.clear()`, the front matter title, `textToParse = encodeEntities(source) + '\n'`, and `parser.parse`                                                                                                                  |
| `packages/parser/src/language/common/tokenBuilder.ts`               | 6-28              | `AbstractMermaidTokenBuilder`: the header keyword gets the suffix `(?:(?=%%)\|(?!\S))` (22-25)                                                                                                                           |
| `packages/parser/src/language/common/valueConverter.ts`             | 6-70              | Regexes for `ACC_TITLE`, `ACC_DESCR` and `TITLE` (6-10), `runConverter` (27-42): common, then custom, then Langium's default. `runCommonConverter` (44-70)                                                               |
| `packages/parser/src/language/common/matcher.ts`                    | 4, 9, 14          | `accessibilityDescrRegex`, `accessibilityTitleRegex`, `titleRegex`                                                                                                                                                       |
| `packages/parser/src/language/railroad/railroad.langium`            | 3-14              | Terminals `TITLE`, `ACC_TITLE`, `ACC_DESCR` (3-5), `RR_ID` and `RR_STRING` (7-8), hidden `RR_WHITESPACE`, `RR_YAML`, `RR_DIRECTIVE`, `RR_SINGLE_LINE_COMMENT`, `RR_BLOCK_COMMENT` (10-14)                                |
|                                                                     | 16-60             | The interfaces (`Railroad`, `RailroadRule`, the expression types)                                                                                                                                                        |
|                                                                     | 62-113            | Entry (62-66), `RailroadRule` (68-70), `RailroadExpression` (72-81), the six call forms (83-113)                                                                                                                         |
| `packages/parser/src/language/railroad-ebnf/railroad-ebnf.langium`  | 3-20              | Terminals (3-8), `EBNF_SPECIAL_SEQUENCE` (9-13, with its comment), hidden terminals (15-20, `EBNF_ISO_COMMENT` at 20)                                                                                                    |
|                                                                     | 22-88             | Interfaces                                                                                                                                                                                                               |
|                                                                     | 90-166            | Entry (90-94), `EbnfRule` (96-98, `=` or `::=`), `EbnfChoice` (100-102), `EbnfSequence` (104-106, commas optional), `EbnfTerm` (108-110), primaries (112-143), postfixes (145-166)                                       |
| `packages/parser/src/language/railroad-abnf/railroad-abnf.langium`  | 7-17              | Terminals `ABNF_RULENAME` (7), `ABNF_STRING` (8), `ABNF_NUMVAL` (9), `ABNF_REPEAT` (10), `ABNF_EXACT_REPEAT` (11), hidden terminals (13-17, `ABNF_COMMENT` at 17)                                                        |
|                                                                     | 66-114            | Entry (66-70), `AbnfRule` (72-74), `AbnfAlternation` (76-78, `/`), `AbnfConcatenation` (80-82), `AbnfElement` (84-86, optional repeat then primary), primaries (88-114)                                                  |
| `packages/parser/src/language/railroad-peg/railroad-peg.langium`    | 3-14              | Terminals, hidden `PEG_LINE_COMMENT` (`#`, 14)                                                                                                                                                                           |
|                                                                     | 64-111            | Entry (64-68), `PegRule` (70-72, `<-`), `PegOrderedChoice` (74-76), `PegSequence` (78-80), `PegPrefix` (82-84), `PegSuffix` (86-88), primaries (90-111)                                                                  |
| `packages/parser/src/language/railroad*/valueConverter.ts`          | see 2.4           | The converters of the four grammars                                                                                                                                                                                      |
| `packages/parser/src/language/generated/ast.ts`                     | 4526 lines in all | The generated types for every grammar (not read in detail)                                                                                                                                                               |

## 2. Parser

### 2.1 Which grammar each syntax uses

| Syntax (keyword)     | Detector id    | Grammar entry (`.langium`)                     | Parser file (`parser/`) | Header token (tokenBuilder)                              | Rule definition form                                                                                                                                                  |
| -------------------- | -------------- | ---------------------------------------------- | ----------------------- | -------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `railroad-beta`      | `railroad`     | `RailroadGrammar`, `railroad.langium`          | `railroadParser.ts`     | `railroad-beta` (`railroad/tokenBuilder.ts:5`)           | `name = expr ;`, with `sequence(...)`, `choice(...)`, `optional(...)`, `oneOrMore(...)`, `zeroOrMore(...)`, `terminal("...")`, `nonterminal("...")`, `special("...")` |
| `railroad-ebnf-beta` | `railroadEbnf` | `RailroadEbnfGrammar`, `railroad-ebnf.langium` | `ebnfParser.ts`         | `railroad-ebnf-beta` (`railroad-ebnf/tokenBuilder.ts:5`) | ISO 14977 and W3C-style: `=` or `::=`, `,` or juxtaposition, `\|`, `[ ]`, `{ }`, `( )`, `?`, `*`, `+`, `- X` exception, `? special ?`                                 |
| `railroad-abnf-beta` | `railroadAbnf` | `RailroadAbnfGrammar`, `railroad-abnf.langium` | `abnfParser.ts`         | `railroad-abnf-beta` (`railroad-abnf/tokenBuilder.ts:5`) | RFC 5234: `=`, `/`, juxtaposition, `[ ]`, `( )`, `%x`, `%d`, `%b`, repetition `*`, `n*m`, `n`                                                                         |
| `railroad-peg-beta`  | `railroadPeg`  | `RailroadPegGrammar`, `railroad-peg.langium`   | `pegParser.ts`          | `railroad-peg-beta` (`railroad-peg/tokenBuilder.ts:5`)   | `Name <- expr ;` with `/`, juxtaposition, `&`, `!`, `?`, `*`, `+`, `( )`, `.`                                                                                         |

All four share the same `ASTNode` model (`railroadTypes.ts:48-55`), the same DB (`railroadDb.ts`), the same renderer and the same styles. The keyword `railroad` is the config section for all four (section 6.8).

### 2.2 Lexer: token order (probed)

Langium 4.4.0 builds each lexer with `DefaultTokenBuilder` (`packages/parser/node_modules/langium`), and `AbstractMermaidTokenBuilder` adds the header suffix (`common/tokenBuilder.ts:22-25`). The token list below comes from `services.parser.Lexer.definition`, read from the built parser (**probed**, `lexer-defs.mjs`). The order is:

1. **Keywords**, in descending length (the probe shows `nonterminal`, `zeroOrMore`, `oneOrMore`, `sequence`, `optional`, `terminal`, `special`, `choice`, then `=`, `;`, `(`, `,`, `)`). The header keyword comes first, because it is the longest word of its grammar.
2. **Terminals** of the grammar, in declaration order: `TITLE`, `ACC_TITLE`, `ACC_DESCR`, then the id, the string and (for EBNF, ABNF and PEG) the rest.
3. **Hidden** tokens last, in declaration order.

Chevrotain takes the first token type whose pattern matches. A keyword with `LONGER_ALT` gives way to the alternative only when the alternative matches more characters. The `LONGER_ALT` entries of the probe:

| Grammar | Keyword with `LONGER_ALT`               | Alternative                                         |
| ------- | --------------------------------------- | --------------------------------------------------- |
| IR      | the header, and all eight word keywords | `RR_ID`                                             |
| EBNF    | the header                              | `EBNF_ID`                                           |
| EBNF    | `(`                                     | `EBNF_ISO_COMMENT` (`(* ... *)`, hidden)            |
| EBNF    | `?`                                     | `EBNF_SPECIAL_SEQUENCE`                             |
| EBNF    | `-`                                     | `EBNF_YAML` (`---` block, hidden)                   |
| ABNF    | the header                              | `ABNF_RULENAME`                                     |
| ABNF    | `;`                                     | `ABNF_COMMENT` (`;` to the end of the line, hidden) |
| PEG     | the header                              | `PEG_ID`                                            |

The full order per grammar:

| Grammar | Keywords (in order)                                                                                                                         | Terminals (in order)                                                                                                                                                                                                 | Hidden (in order)                                                                                                        |
| ------- | ------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| IR      | `railroad-beta`, `nonterminal`, `zeroOrMore`, `oneOrMore`, `sequence`, `optional`, `terminal`, `special`, `choice`, `=`, `;`, `(`, `,`, `)` | `TITLE`, `ACC_TITLE`, `ACC_DESCR`, `RR_ID` `[A-Z_a-z][\w-]*`, `RR_STRING` (double or single quotes, backslash escapes)                                                                                               | `RR_WHITESPACE` `[\t \r\n]+`, `RR_YAML`, `RR_DIRECTIVE`, `RR_SINGLE_LINE_COMMENT` `[\t ]*%%`, `RR_BLOCK_COMMENT` `/* */` |
| EBNF    | `railroad-ebnf-beta`, `::=`, `=`, `;`, `\|`, `,`, `(`, `)`, `[`, `]`, `{`, `}`, `?`, `*`, `+`, `-`                                          | `TITLE`, `ACC_TITLE`, `ACC_DESCR`, `EBNF_ID` `[A-Z_a-z][\w-]*`, `EBNF_STRING`, `EBNF_SPECIAL_SEQUENCE` `\?(?=[^?;]*[^?\s;][^?;]*\?)[^?;]*\?`                                                                         | `EBNF_WHITESPACE`, `EBNF_YAML`, `EBNF_DIRECTIVE`, `EBNF_SINGLE_LINE_COMMENT`, `EBNF_BLOCK_COMMENT`, `EBNF_ISO_COMMENT`   |
| ABNF    | `railroad-abnf-beta`, `=`, `;`, `/`, `(`, `)`, `[`, `]`                                                                                     | `TITLE`, `ACC_TITLE`, `ACC_DESCR`, `ABNF_RULENAME` `[A-Za-z][A-Za-z0-9-]*`, `ABNF_STRING` `"[^"]*"`, `ABNF_NUMVAL` `%[xXdDbB][0-9A-Fa-f]+(?:-…\|\.…)*`, `ABNF_REPEAT` `[0-9]*\*[0-9]*`, `ABNF_EXACT_REPEAT` `[0-9]+` | `ABNF_WHITESPACE`, `ABNF_YAML`, `ABNF_DIRECTIVE`, `ABNF_SINGLE_LINE_COMMENT`, `ABNF_COMMENT` `;[^\n\r]*`                 |
| PEG     | `railroad-peg-beta`, `<-`, `;`, `/`, `&`, `!`, `?`, `*`, `+`, `(`, `)`, `.`                                                                 | `TITLE`, `ACC_TITLE`, `ACC_DESCR`, `PEG_ID` `[A-Z_a-z][\w-]*`, `PEG_STRING`                                                                                                                                          | `PEG_WHITESPACE`, `PEG_YAML`, `PEG_DIRECTIVE`, `PEG_SINGLE_LINE_COMMENT`, `PEG_LINE_COMMENT` `#[^\n\r]*`                 |

The shared terminals `TITLE` `title(?:[\t ][^\n\r]*?(?=%%)\|[\t ][^\n\r]*\|)` (`railroad.langium:3`, and the same line in the other three), `ACC_TITLE` (4) and `ACC_DESCR` (5) have an empty alternative in `TITLE`, so `title` alone matches. The regex matches a prefix of any word that begins with `title`, which the probes show (quirk 2 below).

The header keyword is case-sensitive, because Langium's keywords are, while the detector is not (`railroadDetector.ts:14` has `/i`).

### 2.3 Grammar rules

The rules as written in the `.langium` files. Each ends with `;` unless noted.

```
IR        : 'railroad-beta' (TITLE | ACC_TITLE | ACC_DESCR)* RailroadRule*            (railroad.langium:62-66)
            RailroadRule : RR_ID '=' RailroadExpression ';'                           (68-70)
            Expression   : sequence '(' e (',' e)* ')' | choice '(' a (',' a)* ')'
                         | optional '(' e ')' | oneOrMore '(' e ')' | zeroOrMore '(' e ')'
                         | terminal '(' RR_STRING ')' | nonterminal '(' RR_STRING ')'
                         | special '(' RR_STRING ')'                                    (72-113)
EBNF      : 'railroad-ebnf-beta' (headers)* EbnfRule*                                   (ebnf.langium:90-94)
            EbnfRule     : EBNF_ID ('=' | '::=') EbnfChoice ';'                        (96-98)
            EbnfChoice   : EbnfSequence ('|' EbnfSequence)*                            (100-102)
            EbnfSequence : EbnfTerm (','? EbnfTerm)*                                   (104-106)
            EbnfTerm     : EbnfPrimary EbnfPostfix*                                    (108-110)
            EbnfPrimary  : terminal | special | nonterminal | '(' choice ')' | '[' choice ']' | '{' choice '}'  (112-143)
            EbnfPostfix  : '?' | '*' | '+' | '-' EbnfPrimary                           (145-166)
ABNF      : 'railroad-abnf-beta' (headers)* AbnfRule*                                   (abnf.langium:66-70)
            AbnfRule        : ABNF_RULENAME '=' AbnfAlternation ';'                    (72-74)
            AbnfAlternation : AbnfConcatenation ('/' AbnfConcatenation)*               (76-78)
            AbnfConcatenation : AbnfElement+                                           (80-82)
            AbnfElement     : (ABNF_REPEAT | ABNF_EXACT_REPEAT)? AbnfPrimary           (84-86)
            AbnfPrimary     : string | numval | rulename | '(' alternation ')' | '[' alternation ']'  (88-114)
PEG       : 'railroad-peg-beta' (headers)* PegRule*                                     (peg.langium:64-68)
            PegRule         : PEG_ID '<-' PegOrderedChoice ';'                         (70-72)
            PegOrderedChoice: PegSequence ('/' PegSequence)*                           (74-76)
            PegSequence     : PegPrefix+                                               (78-80)
            PegPrefix       : ('&' | '!')? PegSuffix                                   (82-84)
            PegSuffix       : PegPrimary ('?' | '*' | '+')?                            (86-88)
            PegPrimary      : PEG_STRING | PEG_ID | '(' choice ')' | '.'               (90-111)
```

The header terminals (`TITLE`, `ACC_TITLE`, `ACC_DESCR`) may appear only before the first rule (**probed**: X-17, R-47, X-31). The entry rule takes the last value of each (`title=TITLE` is one assignment, so the last wins: R-22, X-29, X-30).

### 2.4 Value converters

Each grammar has a value converter that extends `AbstractMermaidValueConverter`. Its `runConverter` (`common/valueConverter.ts:27-42`) tries the common regex for the three header terminals first (lines 44-70: a single-line text is trimmed, and each run of two or more blanks becomes one blank, line 59; a block `accDescr { }` loses each line's surrounding blanks, collapses blank runs, and collapses two or more line breaks, lines 62-68), then the grammar's own `runCustomConverter`, then Langium's default.

| Grammar                                  | `TITLE`                                                                 | String terminal                                                                                                                                       | Other terminals                                      |
| ---------------------------------------- | ----------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------- |
| IR (`railroad/valueConverter.ts`)        | If the trimmed text is in quotes, decoded (44-51)                       | `RR_STRING`: quotes removed, then `decodeEscapedString` (5-34, 62-63). `\n`, `\r`, `\t` decoded, every other escaped character stands for itself (25) | none                                                 |
| EBNF (`railroad-ebnf/valueConverter.ts`) | As IR (44-51)                                                           | `EBNF_STRING`: as IR (62-63). `EBNF_SPECIAL_SEQUENCE`: `input.slice(1, -1).trim()` (66-68)                                                            | none                                                 |
| ABNF (`railroad-abnf/valueConverter.ts`) | If in quotes, the quotes are removed and nothing is decoded (13-21, 19) | `ABNF_STRING`: `input.slice(1, -1)` (31-33)                                                                                                           | `ABNF_NUMVAL` and the rule names are kept as written |
| PEG (`railroad-peg/valueConverter.ts`)   | As IR (44-51)                                                           | `PEG_STRING`: as IR (62-63)                                                                                                                           | none                                                 |

The IR, EBNF and PEG escapes differ from the canonical text rule 3 (`docs-port/canonical-model.md`, "Strings"), which says Langium's default converter decodes `\b \f \n \r \t \v \0`. The reference's own converter decodes only `\n`, `\r` and `\t` (**probed**: X-23, X-24, X-25, `\b` in `"a\bc"` gives `b`). The ABNF converter decodes nothing. Section 4 records this.

### 2.5 From the AST to the model

The parsers map the Langium AST to `ASTNode` (`railroadTypes.ts:48-55`):

- **IR** (`railroadParser.ts:26-73`): one-element sequences and choices collapse (45, 49). `oneOrMore` is a repetition with min 1, `zeroOrMore` with min 0, and both have max `Infinity` (56-69). A `terminal`, `nonterminal` or `special` keeps its string.
- **EBNF** (`ebnfParser.ts`): a term is its primary followed by each postfix, in order (125-129). `?` makes an optional, `*` and `{ }` a repetition with min 0, `+` a repetition with min 1 (90-110). The exception `A - B` becomes the sequence `[A, terminal "-", B]` (111-119, probed in E-11). A group is its choice (71-72).
- **ABNF** (`abnfParser.ts`): the repeat string is split by `*` (51-61): `3` is exact (min 3, max 3), `1*3` is min 1 and max 3, `*` is min 0 and max `Infinity`, `*3` is min 0 and max 3. `0*1` (and `*1`, which is `0*1`) becomes an optional (72-74). A `%x41-5A` is a terminal with the text as written (91-95).
- **PEG** (`pegParser.ts`): `&` or `!` on an expression makes a special node whose text is the label (58-63): `&` plus the terminal in quotes, the rule name, the special text, or `(...)` for any other inner node (66-77). `?`, `*`, `+` as in EBNF (79-96). A `.` is a special node with text `.` (112-116).

Each parser then calls `db.addRule(transformRule(rule))` for every rule, in text order (`railroadParser.ts:89`, `ebnfParser.ts:145`, `abnfParser.ts:127`, `pegParser.ts:136`).

### 2.6 Preprocessing and detection

- `Diagram.fromText` removes the front matter and the directives (`preprocess.ts:58-91`), then `cleanupComments` removes whole-line `%%` comments and trims the start (`diagram-api/comments.ts:6-8`), then `encodeEntities(cleaned) + '\n'` (`Diagram.ts:77`). The parser then sees the placeholders (section 3.2).
- The grammar hides directives (`RR_DIRECTIVE`), line comments and block comments (`RR_SINGLE_LINE_COMMENT`, `RR_BLOCK_COMMENT`), and a YAML block (`RR_YAML`). So a directive or front matter that somehow reached the parser would still be skipped (probed in R-28, R-37, X-06).
- Detection is by the regex at the top of each detector (section 1), applied to the cleaned text. A detector match does not mean a parse: `RAILROAD-BETA` and `railroad-betax` are detected and then rejected (R-30, X-05).

### 2.7 Errors

Each parser throws `MermaidParseError` when the Langium result has lexer errors or parser errors (`railroadParser.ts:98-100`, `ebnfParser.ts:154-156`, `abnfParser.ts:136-138`, `pegParser.ts:145-147`). Langium's messages are not compared by the differential, only acceptance (as in P2 and the architecture port). A lexer error names the character (`unexpected character`).

### 2.8 Probed behaviour

Each row is an input for the syntax in the table (`parse-cases.mts`). "Rules" are the DB's rules, as `name = expr`. Compact notation: `T"x"` terminal, `N(x)` nonterminal, `S"x"` special, `SEQ[...]`, `ALT[... | ...]`, `OPT(...)`, `REP{min,max}(...)`, with `inf` for `Infinity`.

| Id               | Input (after the header line)                   | Result                                                            |
| ---------------- | ----------------------------------------------- | ----------------------------------------------------------------- |
| R-01             | `digit = terminal("0") ;`                       | `digit = T"0"`                                                    |
| R-03             | header only                                     | accepted, no rules                                                |
| R-05             | `titles = terminal("a") ;`                      | accepted, the rule is `s = T"a"`, and the title is `""` (quirk 2) |
| R-06             | `sequence = terminal("a") ;`                    | error (keyword as a rule name)                                    |
| R-07             | `sequenceX = terminal("a") ;`                   | accepted, rule `sequenceX`                                        |
| R-08             | `r = terminal("a")` (no `;`)                    | error                                                             |
| R-09             | `terminal("a\"b")`                              | `T"a\"b"` (value `a"b`)                                           |
| R-11             | `terminal("a\nb\tc\\d")`                        | value with LF, TAB, and a backslash                               |
| R-12             | `sequence(terminal("a"))`                       | `T"a"` (collapsed)                                                |
| R-14             | `r = terminal("a") ; junk`                      | error                                                             |
| R-15             | `nonterminal("x y")`                            | `N(x y)`                                                          |
| R-17             | `terminal("")`                                  | `T""`                                                             |
| R-18             | `terminal("<b>x</b> & y")`                      | `T"<b>x</b> &amp; y"` (sanitised, section 3.2)                    |
| R-19             | the same rule name twice                        | both kept, in order                                               |
| R-20             | `title Hello   <b>x</b>`                        | title `Hello <b>x</b>`                                            |
| R-21             | `title "Quoted \"t\""`                          | title `Quoted "t"`                                                |
| R-22             | two `title` lines                               | the last wins: `bar`                                              |
| R-23             | `accTitle: Acc  T` and `accDescr: Desc`         | `Acc T` and `Desc`                                                |
| R-24             | `accDescr {` with lines                         | `line one\nline two`                                              |
| R-25             | `/* c */` before a rule                         | accepted                                                          |
| R-27             | `%%` after a rule                               | accepted                                                          |
| R-28             | a `%%{init}%%` line between rules               | accepted (hidden)                                                 |
| R-30             | `RAILROAD-BETA` header                          | error (case-sensitive)                                            |
| R-31             | CRLF line endings                               | accepted                                                          |
| R-34             | `r = terminal("a"),`                            | error                                                             |
| R-35             | `sequence()`                                    | error                                                             |
| R-37             | `---` YAML block in the body, closed            | accepted                                                          |
| R-38             | `---` without the closing line                  | error (lexer)                                                     |
| R-40             | `r-2`, `_r`, `R_x9`                             | accepted                                                          |
| R-41             | `1r = ...`                                      | error (lexer)                                                     |
| R-47             | `title` after a rule                            | error                                                             |
| R-48             | `Terminal("a")`                                 | error                                                             |
| R-56             | `nonterminal = terminal("a") ;`                 | error (keyword)                                                   |
| R-57             | two rules on one line                           | accepted                                                          |
| R-59             | a line break inside a string                    | accepted, the string holds the LF                                 |
| R-60             | a `title` with a continuation line              | error (`world` is read as a rule name)                            |
| X-29             | `title foo`, then an empty `title` line         | title `""` (the last title wins even when empty)                  |
| X-30             | `accTitle: foo`, then `accTitle:`               | `accTitle` `""`                                                   |
| X-11             | `title Foo %% c`                                | title `Foo`                                                       |
| X-13             | `title 'it'`                                    | title `it`                                                        |
| E-01             | `rule = "a" ;`                                  | `T"a"`                                                            |
| E-02             | `rule ::= "a" ;`                                | `T"a"`                                                            |
| E-03             | `"a" , "b"` and `"a" "b"` (E-04)                | `SEQ[T"a", T"b"]` both                                            |
| E-05             | `[ "a" ]`                                       | `OPT(T"a")`                                                       |
| E-06             | `{ "a" }`                                       | `REP{0,inf}(T"a")`                                                |
| E-08, E-09, E-10 | `"a"?`, `"a"*`, `"a"+`                          | `OPT`, `REP{0,inf}`, `REP{1,inf}`                                 |
| E-11             | `"a" - "b"`                                     | `SEQ[T"a", T"-", T"b"]` (the exception is literal text)           |
| E-12             | `? special ?`                                   | `S"special"`                                                      |
| E-13             | `?a?`                                           | `S"a"`                                                            |
| E-14             | `? ? ;`                                         | error                                                             |
| E-15             | `(* iso comment *) "a"`                         | `T"a"`                                                            |
| E-16             | `(* unclosed`                                   | error                                                             |
| E-17             | `"(*"`                                          | `T"(*"` (a string is not a comment)                               |
| E-22             | a rule with no `;`, then another rule           | error                                                             |
| E-23             | `{"a"}*`                                        | `REP{0,inf}(REP{0,inf}(T"a"))` (postfix nests)                    |
| E-28             | `? a` then a line break then `b ?`              | `S"a\n b"`                                                        |
| E-35             | `[ "a" ]?`                                      | `OPT(OPT(T"a"))`                                                  |
| E-37             | a special after the rule's `;`                  | error                                                             |
| E-41             | `'a'` (single quotes)                           | `T"a"`                                                            |
| E-43             | `---` on its own line in the body               | error                                                             |
| A-01             | `r = "a" ;`                                     | `T"a"`                                                            |
| A-03             | `%x41-5A`                                       | `T"%x41-5A"` (as text)                                            |
| A-04             | `%x41.42`                                       | `T"%x41.42"`                                                      |
| A-05             | `%d65 %b1010`                                   | `SEQ[T"%d65", T"%b1010"]`                                         |
| A-06             | `3DIGIT`                                        | `REP{3,3}(N(DIGIT))`                                              |
| A-07             | `1*3DIGIT`                                      | `REP{1,3}(N(DIGIT))`                                              |
| A-08, A-09       | `*DIGIT`, `1*DIGIT`                             | `REP{0,inf}`, `REP{1,inf}`                                        |
| A-10, A-11       | `*1DIGIT`, `0*1DIGIT`                           | `OPT(N(DIGIT))`                                                   |
| A-12             | `2 DIGIT`                                       | `REP{2,2}`                                                        |
| A-15             | `r = "a" ; comment`                             | error (the comment swallows the terminator)                       |
| A-16             | `r = "a" ;` followed by blanks                  | error (the `;` plus blanks is a comment)                          |
| A-17             | `r = "a";` (no blank)                           | accepted                                                          |
| A-18             | `r = "a";x`                                     | error                                                             |
| A-19             | the `;` on the next line                        | accepted                                                          |
| A-22             | `r = a-b ;`                                     | `N(a-b)`                                                          |
| A-25             | `%X4a`                                          | `T"%X4a"`                                                         |
| A-26             | `%x41-`                                         | error (lexer)                                                     |
| A-29             | `"a ;` (unclosed string)                        | error (lexer)                                                     |
| A-30             | `"a'b"`                                         | `T"a'b"`                                                          |
| A-32             | `title "a\nb"` (quoted)                         | title `a\nb` with a backslash (no decoding)                       |
| A-33, A-34       | `3*"a"`, `*"a"`                                 | `REP{3,inf}`, `REP{0,inf}`                                        |
| A-35             | `r = 3 ;`                                       | error                                                             |
| A-37             | `*0DIGIT`                                       | `REP{0,0}(N(DIGIT))`                                              |
| A-38             | `2*1DIGIT` (min above max)                      | accepted, `REP{2,1}`                                              |
| A-40             | a `;` comment line between rules                | accepted                                                          |
| A-43             | `%x4`                                           | `T"%x4"`                                                          |
| P-03             | `&"a" "b"`                                      | `SEQ[S"&\"a\"", T"b"]`                                            |
| P-04             | `!A B`                                          | `SEQ[S"!A", N(B)]`                                                |
| P-05             | `&(A B)`                                        | `S"&(...)"`                                                       |
| P-06             | `.`                                             | `S"."`                                                            |
| P-07             | `"a"? "b"* "c"+`                                | `SEQ[OPT, REP{0,inf}, REP{1,inf}]`                                |
| P-09             | a `#` comment inside a rule                     | accepted                                                          |
| P-10, P-11, P-12 | `&&"a"`, `! & "a"`, `"a"?*`                     | error                                                             |
| P-14             | `&"a"*`                                         | `S"&(...)"` (the label of a repetition is `(...)`)                |
| P-15             | `!"x"?`                                         | `S"!(...)"`                                                       |
| P-16             | `&A`                                            | `S"&A"`                                                           |
| P-20             | `""`                                            | `T""`                                                             |
| P-23             | `. .`                                           | two specials `.`                                                  |
| P-26             | `R = "a" ;` (equals, not `<-`)                  | error (lexer)                                                     |
| P-29             | the same rule name twice                        | both kept                                                         |
| P-30             | `"\n"`                                          | a terminal with an LF                                             |
| P-31             | `&"a\""`                                        | `S"&\"a\"\""` (the label does not escape the quote)               |
| X-01             | `railroad-ebnf-beta` read by the IR parser      | error                                                             |
| X-02             | `railroad-beta` read by the EBNF parser         | error                                                             |
| X-03, X-04       | the PEG or ABNF header read by the other parser | error                                                             |
| X-05             | `railroad-betax`                                | error                                                             |
| X-06             | `railroad-beta%%c`                              | accepted                                                          |
| X-08             | indented header and rule                        | accepted                                                          |
| X-09             | a `%%` line before the header                   | accepted                                                          |
| X-17             | `accTitle:` after a rule                        | error                                                             |
| X-22             | `title "x"` after a rule                        | error                                                             |
| X-23             | `"a\bc\fd\ve\0f"`                               | `T"abcfdve0f"` (the escapes are not decoded)                      |
| X-26             | `terminal("&lt;b&gt;")`                         | `T"&lt;b&gt;"`                                                    |
| X-27             | `terminal("#amp;")`                             | `T"ﬂ°amp¶ß"` (the placeholder stays in the DB)                    |
| X-28             | `nonterminal("AT&T <b>bold</b>")`               | `N(AT&amp;T <b>bold</b>)`                                         |

### 2.9 Quirks to reproduce

1. **Header case and suffix.** The header is case-sensitive, but the detector is not. The header must be followed by a blank, a line end or `%%`: `railroad-betax` is an error (R-30, X-05, X-06). The same holds for the other three headers.
2. **A header terminal eats the prefix of a word.** `TITLE` matches `title` with an empty tail, so `titles = terminal("a");` is the rule `s` with an empty title (R-05). Any word that starts with `title` splits the same way. `ACC_TITLE` and `ACC_DESCR` need a colon or a brace after the keyword, so `accTitles = ...` is an id by the regexes (read, not probed).
3. **Word keywords need the longer identifier.** `sequenceX` is an id (R-07), but the keyword `sequence` as a rule name is an error (R-06), and so is `nonterminal` (R-56). Capital letters do not make a keyword (R-48).
4. **Headers come first.** `title`, `accTitle` and `accDescr` after the first rule are errors (R-47, X-17, X-22). The last `title` line wins, even when it is empty (X-29).
5. **EBNF exception is text.** `A - B` is the sequence `A`, `"-"`, `B`, drawn as three boxes (E-11). The renderer draws the dash as a terminal.
6. **EBNF special sequence.** `?...?` needs a non-blank character inside and no `;` (E-13, E-14, E-37). It is trimmed, and a line break inside it is kept (E-28).
7. **Postfixes nest.** `{ "a" }*` and `[ "a" ]?` are repetitions and optionals of those (E-23, E-35).
8. **ABNF `;` is a comment when text follows.** The `;` keyword gives way to `ABNF_COMMENT` as soon as a character follows it on the line, so the rule has no terminator. A blank after `;` is enough to break it (A-15, A-16, A-18). A `;` alone at the end of a line works (A-17, A-19), and a comment line between rules works (A-40).
9. **ABNF repeats keep their min and max, and the renderer keeps only min.** `*1` is an optional (A-10), `*0` is a repetition with max 0 (A-37), and `2*1` is accepted (A-38).
10. **ABNF numvals and quoted titles are not decoded.** `%x41-5A` is one terminal with its text (A-03), and a quoted title keeps its backslashes (A-32).
11. **PEG lookahead labels are lossy.** `&` on a suffixed expression, or on a group, is labelled `(...)` (P-14, P-15). The label of a quoted terminal does not escape its own quote (P-31).
12. **PEG prefixes and suffixes are single.** `&&`, `! &` and `"a"?*` are errors (P-10, P-11, P-12).
13. **String escapes.** IR, EBNF and PEG decode only `\n`, `\r` and `\t`. `\b`, `\f`, `\v` and `\0` stand for the letter (X-23, X-24, X-25). A raw line break inside a string is accepted (R-59).
14. **Every value is sanitised.** See section 3.2: `<script>` is removed, `&` becomes `&amp;`, and the entity placeholders are kept.
15. **Nothing else in the lexer.** A `---` YAML block in the body is accepted when closed (R-37) and is an error when not (R-38). Line comments and block comments are hidden anywhere between tokens (R-25, R-26, R-27). Two rules on one line are accepted (R-57).
16. **A rule name is `[A-Za-z_][\w-]*` in IR, EBNF and PEG, and `[A-Za-z][A-Za-z0-9-]*` in ABNF.** So `1r` is an error (R-41), and `_r` is an id in IR (R-40). An ABNF name with a leading `_` is a lexer error by the regex (read, not probed; A-22 shows `a-b` is accepted).

## 3. DB

### 3.1 Calls in order

| Step | Call                                                                                       | Effect                                                                                                              |
| ---- | ------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------- |
| 1    | `db.clear()` (`Diagram.ts:49`)                                                             | Before parsing, the shared DB state is reset                                                                        |
| 2    | `db.setDiagramTitle(frontMatter.title)` (`Diagram.ts:53`), if the front matter has a title | Stored, sanitised                                                                                                   |
| 3    | `parser.parse(text)` (`Diagram.ts:83`)                                                     | Calls `db.clear()` again (`railroadParser.ts:94`). **Unverified:** that clears the front matter title set in step 2 |
| 4    | `populateCommonDb` (`railroadParser.ts:83`)                                                | `setAccDescription`, `setAccTitle` and `setDiagramTitle` for each non-empty value (`populateCommonDb.ts:5-13`)      |
| 5    | `setTitle` (`railroadParser.ts:85-87`), for a non-empty title                              | Stored again, sanitised                                                                                             |
| 6    | `addRule(transformRule(rule))` for each rule (`railroadParser.ts:89`)                      | Stored in order, sanitised (3.2)                                                                                    |
| 7    | `draw` reads `db.getRules()` (`railroadRenderer.ts:647`)                                   | The renderer takes the rules and nothing else                                                                       |

The DB is one module (`railroadDb.ts:140-153`), shared by the four syntaxes. The parsers keep `db` as `yy` (`railroadParser.ts:109`).

### 3.2 What each DB call stores

| Function            | Stores                                                      | Sanitised                                                                                     | Source                  |
| ------------------- | ----------------------------------------------------------- | --------------------------------------------------------------------------------------------- | ----------------------- |
| `setTitle`          | `diagramTitle`                                              | Yes (`sanitizeText`)                                                                          | `railroadDb.ts:56-59`   |
| `setAccTitle`       | `accTitle`                                                  | Yes, then the leading whitespace is removed                                                   | `railroadDb.ts:107-110` |
| `setAccDescription` | `accDescription`                                            | Yes, then each line break's following whitespace is removed                                   | `railroadDb.ts:122-125` |
| `addRule`           | `rules` (array, in order) and `ruleMap` (last one per name) | Name (74), definition (75) through `sanitizeAstNode` (17-38), and the comment (76, never set) | `railroadDb.ts:71-88`   |

`sanitizeAstNode` sanitises the terminal value (20), the nonterminal name (22), the special text (36), and recurses into the children (24-34). The `separator` of a repetition is sanitised too (33), but no parser sets it.

`sanitizeText` is `commonSanitizeText(text, getConfig())` (`railroadDb.ts:13-15`). In `common.ts` it is DOMPurify (with `FORBID_TAGS: ['style']` unless `dompurifyConfig` is set), after `sanitizeMore`. Probed (section 2.8): a `<script>` element and its text are removed (R-54), `&` is written `&amp;` (R-18), and `<b>` is kept as markup (R-20).

**Visible effect (probed, Chromium).** The renderer writes the stored text with `.text(value)` (`railroadRenderer.ts:120`, `:151`, `:488`), so markup is shown as text: `<b>x</b> &amp; y` is drawn with the `&amp;` visible (`svg/c03-markup.svg`). The entity placeholders from `encodeEntities` are decoded in the whole SVG string after rendering (`mermaidAPI.ts:359`, `utils.ts:904-906`). So `#amp;` (stored as `ﬂ°amp¶ß`) is drawn as `&`, and `#35;` as `#` (probed in `render-probe2.mts`, `e01-entity`). But a `&lt;` that the sanitiser keeps is drawn as `&lt;`, because the decode only touches the placeholders. The port's common SVG step must apply the same decode to the whole string.

### 3.3 Defaults and empty values

- No field has a default except the empty string and the empty list. The title, the accessibility texts and `rules` start empty after `clear` (`railroadDb.ts:43-48`).
- `getRules` returns the internal array itself (93-95). The renderer does not copy it, and nothing mutates it after the parse.
- `getRule` (100-102) and `getTitle`/`getDiagramTitle` (64-66, 135) are not used by the renderer or by the core (grep). The title is never drawn (probed: `svg/c02-title.svg` has no title text).

### 3.4 Quirks of the DB

1. **Duplicates are kept.** Two rules with the same name are both stored and both drawn (R-19, c10). `addRule` warns (`railroadDb.ts:82-84`) and does not drop the first.
2. **Names are sanitised.** A nonterminal `AT&T <b>bold</b>` is stored `AT&amp;T <b>bold</b>` (X-28).
3. **The last empty title wins.** The grammar gives `""`, and `populateCommonDb` and `setTitle` skip empty values, so the title is `""` (X-29). An earlier non-empty title is lost, because the DB never sees it.
4. **Accessibility texts are drawn by the core.** `getAccTitle` and `getAccDescription` are read by the core (`mermaidAPI.ts:621-622`), which adds `title` and `desc` with ids `chart-title-<id>` and `chart-desc-<id>`, and sets `aria-labelledby` and `aria-describedby` on the root (probed: `svg/c15-accessible.svg`).

## 4. Canonical model proposal

The model is the DB after the parse. It follows [canonical-model.md](../canonical-model.md): camelCase keys, `"version": 1`, and strings always present.

```text
RailroadDiagram { version: 1, title: string, accTitle: string, accDescr: string, rules: [Rule] }
Rule { name: string, definition: Expr }
Expr =
  { type: "terminal",    value: string }
| { type: "nonterminal", name: string }
| { type: "special",     text: string }
| { type: "sequence",    elements: [Expr] }
| { type: "choice",      alternatives: [Expr] }
| { type: "optional",    element: Expr }
| { type: "repetition",  element: Expr, min: number, max: number | "Infinity" }
```

| Field                   | Rule                                                                                                                                                                                                                                                                                                                                                                |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `rules`                 | Every rule, in text order, duplicates included (3.4, quirk 1)                                                                                                                                                                                                                                                                                                       |
| `definition`            | The `ASTNode` of section 2.5, with one-element sequences and choices already collapsed                                                                                                                                                                                                                                                                              |
| `value`, `name`, `text` | The sanitised DB values (3.2). **Deviation from the canonical text rule 6:** the DB sanitises these, as the flowchart does, so the model stores the sanitised text. Record it in `canonical-model.md` (section 8.3, Q4)                                                                                                                                             |
| `min`, `max`            | As the parser sets them: a `repetition` takes `min` and `max` as the parser gives them. EBNF `*` and `{ }` are `0` to `Infinity`, `+` is `1` to `Infinity`, ABNF repeats keep their numbers, and a min above a max is kept (A-38). EBNF `?` and ABNF `0*1` are `optional` nodes, with no `min` or `max`. `Infinity` is written as `"Infinity"` (canonical, Numbers) |
| string escapes          | The IR, EBNF and PEG converter, not Langium's default (2.4). **Deviation from the canonical text rule 3** (Q3)                                                                                                                                                                                                                                                      |

Left out, and why:

- `RailroadRule.comment` and `RepetitionNode.separator`: no grammar sets them. The DB stores `undefined` for the comment (`railroadDb.ts:76`), so the model leaves the field out rather than writing `""`.
- The title from the front matter: the reference drops it (3.1, step 3, **unverified**). Whether the port keeps the title from the front matter is Q8.
- Which syntax the input is: the detector chooses the parser, and the model is the same for the four. The differential needs the detected type per input, not a field of the model (Q9).
- The styles, the configuration and the theme: the renderer's input, read by the differential for the configuration only (`pnpm port:diff config`), not part of the parse.

## 5. Specs

| File (under `diagrams/railroad/`)                                     |   Lines |  Tests | What they test                                                                                                                                                                                                                                     |
| --------------------------------------------------------------------- | ------: | -----: | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `railroadDetector.spec.ts`                                            |      78 |     14 | The id (1), the detector (9: header, case, leading blanks, newlines, negatives, empty and blank text), and the loader (4)                                                                                                                          |
| `abnfDetector.spec.ts`, `ebnfDetector.spec.ts`, `pegDetector.spec.ts` | 34 each | 6 each | The same shape for each header: the detector and the loader                                                                                                                                                                                        |
| `parser/railroadDiagram.spec.ts`                                      |     417 |     26 | Parser and DB through `db.getRules()`: basic forms (11), complex grammars (3), errors (3), edge cases (7: an empty list, whitespace, underscores and digits in names, deep nesting, a zero-or-more of a choice, block comments), real examples (2) |
| `parser/ebnfDiagram.spec.ts`                                          |     178 |     14 | Basic forms (11, including the title, the ISO forms and a special sequence), an expression grammar (1), and the errors (2)                                                                                                                         |
| `parser/abnfDiagram.spec.ts`                                          |     149 |     12 | Basic forms (9, including a title), a URI-like grammar, the errors (2)                                                                                                                                                                             |
| `parser/pegDiagram.spec.ts`                                           |     176 |     14 | Basic forms (11, including `&` and `!`), an expression grammar, the errors (2)                                                                                                                                                                     |
| `railroadDb.spec.ts`                                                  |     121 |     12 | Rules, titles, accessibility texts, the duplicate case, `clear`, and the sanitising of names and values at the DB boundary                                                                                                                         |
| `railroadRenderer.spec.ts`                                            |     167 |      6 | The viewBox, the marker alignment, and the choice and repetition arcs. **The `getBBox` is stubbed** (`railroadRenderer.spec.ts:11-22`: width is the text length times 8, at least 1, and height 16), so the numbers are not Chromium's             |
| `styles.spec.ts`                                                      |     271 |     19 | `getStyles` and the sanitisers: the defaults, the theme colours, the custom values, the fallbacks for invalid values, and the CSS classes                                                                                                          |

Total: 135 tests, all passing at HEAD (vitest run, probed). `packages/parser/tests/railroad.test.ts` (110 lines, 7 tests) tests the Langium grammar directly and is the other half of the parser's check.

What no spec covers: the real text metrics; the header keywords across syntaxes (X-01 to X-04); the quirks in 2.9; the entity decode of the SVG string; `useMaxWidth: false`; the `railroad` config keys beyond the ones the styles spec names; the empty diagram size (200 by 100).

## 6. Renderer

### 6.1 Pipeline

1. **Entry.** `draw(text, id)` (`railroadRenderer.ts:638`). `selectSvgElement(id)` (642) and the class `railroad-diagram` (643).
2. **Configuration.** `getConfigAPI().railroad` and `useMaxWidth`, default `true` (644-645). Nothing else from the section is read here; the rest is read by `buildRailroadStyleOptions` (section 6.8).
3. **Empty diagram.** With no rules, the size is 200 by 100, and the function returns (650-654). Probed: `svg/c04-empty.svg` has `viewBox="0 0 200 100"` and `max-width: 200px`.
4. **Layout and drawing.** `new RailroadRenderer(svg, buildRailroadStyleOptions())` (656), then `renderDiagram(rules)` (657), which appends one `g.railroad-rule` for each rule (section 6.2).
5. **Size.** `configureRailroadSvgSize` (626-633) calls `configureSvgSize` (`setupGraphViewbox.js:43-46`, which uses `calculateSvgSizeAttrs` at 23-33), which sets `width="100%"` and `style="max-width: Wpx;"` when `useMaxWidth` is true, or `width` and `height` attributes when it is false, then the `viewBox` `0 0 W H` (632). The railroad does not call `setupGraphViewbox`, so it never reads the bounding box of the root.
6. **Generic steps** (`mermaidAPI.ts`, the same for every diagram; packet notes section 6.1 has the order): the `div` and `svg` (`mermaidAPI.ts:397-421`), the `<style>` element from `createUserStyles` (319-334), the empty `g` that comes first (probed), the accessibility `title` and `desc` (621-622), and the decode of the whole SVG string (359).

Probed root of a simple diagram (`svg/c01-simple.svg`): `<svg id="cu" width="100%" xmlns=… class="railroad-diagram" style="max-width: 144.484375px;" viewBox="0 0 144.484375 86.5" role="graphics-document document" aria-roledescription="railroad">`. The `aria-roledescription` is the detector id: `railroadEbnf`, `railroadAbnf` and `railroadPeg` for the other three (probed: c16, c17, c18). The port must write the id, not the keyword.

### 6.2 Layout: the arithmetic

Every number is measured text (section 6.4) or a configuration value (section 6.8), or a constant. The constants are hard-coded and do not follow the configuration:

| Constant                                          | Where                         | Value           |
| ------------------------------------------------- | ----------------------------- | --------------- |
| The rounded corner of a terminal                  | `railroadRenderer.ts:112-113` | `rx = ry = 10`  |
| The gap after the end circle and the line into it | `:569-574`, `:596`, `:602`    | 10              |
| The width added to the rule name                  | `:542`                        | 20              |
| The gap between the name and the definition       | `:543`                        | 20              |
| The minimum baseline                              | `:548`                        | 20              |
| The minimum rule height                           | `:601`                        | 40              |
| The arc height of optional and repetition         | `:321`, `:374`                | `2 * arcRadius` |
| The loop's drop below the element                 | `:415`                        | `arcRadius`     |

The element formulas (`w` and `h` are the width and height of the element, `up` and `down` are the parts above and below its centre line):

| Element (line range)  | Width                                               | Height                                      | Up                                                             | Down          | Children and their offsets                                                                                                                                                              |
| --------------------- | --------------------------------------------------- | ------------------------------------------- | -------------------------------------------------------------- | ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Terminal (98-131)     | `textW + 2 * padding` (100)                         | `textH + 2 * padding` (101)                 | `h / 2` (127)                                                  | `h / 2` (128) | a rect, then a text at `(w/2, h/2)` (116-120)                                                                                                                                           |
| Nonterminal (136-162) | as terminal (138-139)                               | as terminal                                 | `h / 2`                                                        | `h / 2`       | a rect (no `rx`, 144), then a text at `(w/2, h/2)`                                                                                                                                      |
| Special (473-499)     | `textW('? ' + text + ' ?') + 2 * padding` (474-475) | as terminal (476)                           | `h / 2`                                                        | `h / 2`       | a rect, then the text `? text ?` at `(w/2, h/2)`                                                                                                                                        |
| Sequence (167-217)    | `Σw + (n - 1) * horizontalSeparation` (170-180)     | `maxUp + maxDown` (212)                     | `maxUp` (210)                                                  | `maxDown`     | child `i` at `x` and `y = maxUp - up_i` (187), with a line of length `horizontalSeparation` after each child but the last (195-202). `x` advances by `w_i + horizontalSeparation` (205) |
| Choice (222-313)      | `maxW + 4 * arcRadius` (225-237)                    | `Σh + (n - 1) * verticalSeparation` (233)   | `centreY = H / 2` (241)                                        | `H - centreY` | child `i` at `x = 2 * arcRadius + (maxW - w_i) / 2` (248) and `y` the running sum with `verticalSeparation` (301). Two arcs and two lines per child (253-299)                           |
| Optional (318-366)    | `w + 4 * arcRadius` (322)                           | `h + 2 * arcRadius` (323)                   | `centreY = 2 * arcRadius + up` (333)                           | `H - centreY` | the child at `(2r, 2r)` (328-331), a lower line in and out (336-341), and the bypass arc path (345-355)                                                                                 |
| Repetition (371-468)  | `w + 4 * arcRadius` (375)                           | `h + 2r + (min = 0 ? 2r : 0)` (374, 379)    | `centreY = elemY + up` (389), `elemY = min = 0 ? 2r : 0` (385) | `H - centreY` | the child at `(2r, elemY)`, two forward lines (392-412), the loop path below (415-442), and the bypass path only when `min = 0` (445-457)                                               |
| Rule (535-604)        | see below                                           | `max(40, defY + h + 2 * padding)` (600-601) |                                                                |               |                                                                                                                                                                                         |

Rule layout (`renderRule`, `:535-604`): `ruleName = name + ' ='` (541), `nameW = textW(ruleName) + 20` (542), `defX = nameW + 20` (543), `baseline = max(20, up)` (548), `defY = baseline - up` (549), the definition group at `(defX, defY)` (550), the name text at `(0, baseline)` (553-558), the start circle at `(nameW, baseline)` with radius `markerRadius` (561-566), the end circle at `(defX + w + 10, baseline)` (569-574), the start line from `nameW + markerRadius` to `defX` (577-586), the end line from `defX + w` to `defX + w + 10 - markerRadius` (589-598). The rule's width is `defX + w + 10 + markerRadius` (602).

Diagram (`renderDiagram`, `:609-623`): the first rule at `y = padding`; each rule's `y` advances by its height plus `verticalSeparation` (615). The width is the widest rule plus `2 * padding` (620), and the height is the final `y` plus `padding` (621). The final `y` includes one trailing `verticalSeparation`.

**Probed checks (Chromium).** With the harness settings: the rule of `digit = terminal("0")` has a terminal rect of width 29.078125 (= 9.078125 + 20) and height 37 (= 17 + 20), the end circle at `cx = 119.484375`, and a viewBox of `144.484375 × 86.5`. With `arcRadius` at its default, the choice of `a` and `bb` gives `viewBox` height 130 (`svg/c06-choice.svg`), the optional of `a` gives 105, the zero-or-more gives 125, and the one-or-more gives 106.5. Each matches the formulas above. With `padding: 4` and `verticalSeparation: 20` (`g01-padding-sep`), the rect is 17.078125 by 25 and the viewBox height is 68.5, which also matches.

**Repetition ignores max.** `renderExpression` passes only `node.min` (`:521-522`), and the repetition uses `min === 0` alone (378). Probed (`e03-rep-exact`, `e04-rep-range`, `e06-plus`): `3DIGIT`, `1*3DIGIT` and `oneOrMore(a)` render the same as `+` (three paths, no bypass). `*0DIGIT` renders as a `*` with the bypass (`e05-rep-zero-zero`, four paths, height 125).

### 6.3 Element structure and DOM order

Probed (`outline.cjs`) for a rule with a choice (`svg/c06-choice.svg`), in DOM order:

```text
g.railroad-rule (transform translate(0, y))
  g (transform translate(defX, defY))                     the definition group, appended first
    g.railroad-choice
      g.railroad-terminal (transform translate(x, y))     one per alternative, in order
        rect (x=0, y=0, width, height, rx=10, ry=10)
        text (x=w/2, y=h/2)  "a"
      path.railroad-line  (left arc, from the centre line)
      path.railroad-line  (right arc, to the centre line)
  g.railroad-rule-name-group
    text.railroad-rule-name (x=0, y=baseline)  "r ="
  g.railroad-start > circle (cx=nameW, cy=baseline, r=markerRadius)
  g.railroad-end   > circle (cx=defX+w+10, cy=baseline, r=markerRadius)
  path.railroad-line  (start line)
  path.railroad-line  (end line)
```

The other elements have the same pattern: `g.railroad-sequence` holds its children with `path.railroad-line` between them (probed, `c11-sequence`), `g.railroad-optional` holds the child and three paths (probed, `c07-optional`), and `g.railroad-repetition` holds the child, the forward lines, the loop, and the bypass for `min = 0` (probed, `c08-zeroOrMore`). The ids are not set on any element. The `text` of a rule name has no `text-anchor`, so it starts at `x`.

The `g` of each child is moved into its parent with `appendChild` (`:190`, `:249`, `:330`, `:386`), so the element order is the order of those calls. The temporary text of `measureText` is removed before the DOM is used (89).

### 6.4 Text measurement

`measureText(text)` (`:74-93`) appends a temporary `text` to the root `svg` with the attributes `font-family` (`config.fontFamily`) and `font-size` (`config.fontSize`, a number), reads `getBBox()` (86), and removes the text (89). The result is cached per renderer instance by the text alone (64, 75-77, 90), so one text has one measurement for the whole diagram.

The texts measured: a terminal's value (99), a nonterminal's name (137), the special text with its `? … ?` (474), and the rule name with ` =` (542). Nothing else is measured.

Probed metrics (`render-probe.mts`, `results.json.__metrics`), with the harness font `"Lexica Ultralegible"`:

| Text                                      | Font size | `getBBox` width | `getBBox` height |                 `getBBox` y |
| ----------------------------------------- | --------: | --------------: | ---------------: | --------------------------: |
| `0`                                       |        14 |        9.078125 |               17 |                         -13 |
| `digit =`                                 |        14 |        40.40625 |               17 |                         -13 |
| `xx`                                      |        14 |       12.921875 |               17 |                         -13 |
| `? x y ?`                                 |        14 |        38.78125 |               17 |                         -13 |
| `` (empty)                                |        14 |               0 |                0 |                           0 |
| `0`                                       |        16 |          10.375 |               20 |                         -15 |
| `0` (monospace, not in the harness faces) |        20 |       12.015625 |               23 | (from the rect, `g03-font`) |

Three points for the port:

- Widths are multiples of 1/64 px (Chromium's layout precision). A rect's width is `textW + 2 * padding` as written, so the output keeps the fraction (`27.375`, `27.9375`).
- The height is the line box of the font, and it does not depend on the text: `0` and `digit =` both give 17 at 14 px. An empty text gives 0 by 0, so an empty terminal is 20 by 20 (`svg/c12-emptyterminal.svg`).
- A newline in a terminal's value is measured as SVG text, which collapses it to a space. `svg/c13-newline.svg` has a rect of 39.21875 for `a\nb`, so the measured text is 19.21875 wide (`a` is 7.375, `b` is 7.9375, and the space between them is the rest).

`renderRule` measures `name =` with the same font, so the rule name's width moves the whole rule to the right. This is the same kind of text rule as the kanban and architecture labels (decision 0005), and `getBBox` is the one DOM call in the renderer.

### 6.5 The root size

The root's size is the diagram's `width` and `height` from section 6.2, through `configureSvgSize` (`setupGraphViewbox.js:43-46`). `useMaxWidth: true` gives `width="100%"` and `max-width` in the style, and `useMaxWidth: false` gives `width` and `height` attributes (probed: `g02-nomaxwidth`, `width=144.484375 height=86.5`). The viewBox is `0 0 W H` in both cases (632).

### 6.6 The style sheet

`getStyles` (`styles.ts:259-366`) returns the CSS below, built from `buildRailroadStyleOptions()`. The core scopes it to `#id` (probed: `#cu .railroad-terminal rect{…}`):

| Rule (lines)                                                    | Declarations                                                                               |
| --------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| `.railroad-diagram` (281-284)                                   | `font-family`, `font-size` (the diagram's text)                                            |
| `.railroad-terminal rect` (286-290)                             | `fill: terminalFill`, `stroke: terminalStroke`, `stroke-width: strokeWidth`                |
| `.railroad-terminal text` (292-298)                             | `fill: terminalTextColor`, the font, `text-anchor: middle`, `dominant-baseline: middle`    |
| `.railroad-nonterminal rect` (300-304)                          | `fill: nonTerminalFill`, `stroke: nonTerminalStroke`, `stroke-width`                       |
| `.railroad-nonterminal text` (306-312)                          | `fill: nonTerminalTextColor`, the font, `text-anchor: middle`, `dominant-baseline: middle` |
| `.railroad-line` (314-318)                                      | `stroke: lineColor`, `stroke-width`, `fill: none`                                          |
| `.railroad-start circle, .railroad-end circle` (320-323)        | `fill: markerFill`                                                                         |
| `.railroad-comment ellipse`, `.railroad-comment text` (325-338) | **Dead:** no element has the class `railroad-comment` (the renderer never builds one)      |
| `.railroad-special rect` (340-345)                              | `fill: specialFill`, `stroke: specialStroke`, `stroke-width`, `stroke-dasharray: 5,3`      |
| `.railroad-special text` (347-353)                              | `fill: nonTerminalTextColor`, the font, `text-anchor: middle`, `dominant-baseline: middle` |
| `.railroad-rule-name` (355-360)                                 | `font-weight: bold`, `fill: ruleNameColor`, the font                                       |
| `.railroad-group` (362-364)                                     | An empty rule, not used                                                                    |

Probed (`svg/c01-simple.svg`): every rule above is present, with the values of the harness theme.

### 6.7 Theme variables the stylesheet reads

The renderer reads no theme variable itself. `buildRailroadStyleOptions` reads them (`styles.ts:193-197`, `125-187`), from three layers, in this order: `getThemeVariables()` (the default theme, `theme-default.js:503-507`), then `config.themeVariables` (the active theme, computed by `config.ts:137` and `mermaidAPI.ts:715`), then the options passed to `getStyles`, which are the same values (`mermaidAPI.ts:327-333`). The variables and where they feed:

| Theme variable                                    | Read at                                                 | Feeds                       |
| ------------------------------------------------- | ------------------------------------------------------- | --------------------------- |
| `fontFamily`                                      | `styles.ts:126-127`                                     | `fontFamily` (text)         |
| `fontSize`                                        | `styles.ts:130` (parsed, `parseThemeFontSize`, 114-123) | `fontSize`                  |
| `secondBkg` (or `secondaryColor`)                 | `:136-138`                                              | `terminalFill`              |
| `secondaryBorderColor` (or `lineColor`)           | `:140-142`                                              | `terminalStroke`            |
| `secondaryTextColor` (or `textColor`)             | `:144-146`                                              | `terminalTextColor`         |
| `mainBkg` (or `background`)                       | `:148-150`                                              | `nonTerminalFill`           |
| `primaryBorderColor` (or `lineColor`)             | `:152-154`                                              | `nonTerminalStroke`         |
| `primaryTextColor` (or `textColor`)               | `:156-158`                                              | `nonTerminalTextColor`      |
| `lineColor`                                       | `:160`, `:161`                                          | `lineColor`, `markerFill`   |
| `labelBackground` (or `tertiaryColor`)            | `:162-164`                                              | `commentFill` (unused)      |
| `tertiaryBorderColor` (or `lineColor`)            | `:166-168`                                              | `commentStroke` (unused)    |
| `tertiaryTextColor` (or `textColor`)              | `:170-172`                                              | `commentTextColor` (unused) |
| `tertiaryColor` (or `secondaryColor`)             | `:174-176`                                              | `specialFill`               |
| `tertiaryBorderColor` (or `secondaryBorderColor`) | `:178-180`                                              | `specialStroke`             |
| `titleColor` (or `textColor`)                     | `:182-184`                                              | `ruleNameColor`             |

Seventeen distinct names. Probed values (`svg/c01-simple.svg`, harness redux-color): terminal fill `#ffffde`, terminal stroke `hsl(-120, 0%, 70%)`, terminal text `#333333`, nonterminal fill `#ffffff`, nonterminal stroke `hsl(247.5, 0%, 9.2156862745%)`, nonterminal text `#28253D`, line `#000000`, rule name `#000000`, font size 14 px. Probed values (`svg/d01-default-simple.svg`, the default theme with no harness settings): fill `#ffffde`, stroke `hsl(60, 60%, 83.5294117647%)`, text `#000021`, nonterminal fill `#ECECFF`, line `#333333`, rule name `#333`, font size 16 px, family `"trebuchet ms",verdana,arial,sans-serif`.

A colour that does not match `COLOR_VALUE_PATTERN` (`styles.ts:18-19`) falls back to the theme's value (probed: `nonTerminalStroke: 'url(x)'` gives the theme's stroke, `g05-colours`).

### 6.8 Configuration

Every key the renderer and the styles read. "Schema" is the default in `config.schema.yaml`; "effective" is the value after `defaultConfig.ts:295-314`, which sets the colour and typography keys to `undefined` so the theme supplies them.

| Key                                                | Schema default (line)                 | Effective default                          | Read at                                                                                | Effect                                                                                                         |
| -------------------------------------------------- | ------------------------------------- | ------------------------------------------ | -------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `useMaxWidth`                                      | `true` (`BaseDiagramConfig`, 464-470) | `true`                                     | `railroadRenderer.ts:645`                                                              | Size attributes (6.5). Probed (g02)                                                                            |
| `padding`                                          | 10 (3115-3119)                        | 10                                         | `railroadRenderer.ts:100-101`, `:138-139`, `:475-476`, `:601`, `:610`, `:620-621`      | The space around each text and each rule. Probed (g01)                                                         |
| `verticalSeparation`                               | 8 (3120-3124)                         | 8                                          | `:233`, `:301`, `:615`                                                                 | Gap between alternatives and between rules. Probed (g01)                                                       |
| `horizontalSeparation`                             | 10 (3125-3129)                        | 10                                         | `:180`, `:196`, `:205`                                                                 | Gap between sequence items. Read (the formula is probed for `sequence`, c11 is 10)                             |
| `arcRadius`                                        | 10 (3130-3134)                        | 10                                         | `:235`, `:320`, `:373`, `:236` and the arcs                                            | Radius of the choice, optional and repetition arcs. Probed for the default (c06 to c09)                        |
| `fontSize`                                         | 14 (3135-3139)                        | Theme (14 for redux-color, 16 for default) | `styles.ts:216` (via `:130`), `renderer` `measureText` (83-84)                         | Text size. Probed (g03, d01)                                                                                   |
| `fontFamily`                                       | `'monospace'` (3140-3143)             | Theme                                      | `styles.ts:126-129` (theme), `:217` (option), `railroadRenderer.ts:82` (`measureText`) | Text family. Probed (g03)                                                                                      |
| `terminalFill`                                     | `#FFFFC0` (3144-3147)                 | Theme (`secondBkg`)                        | `styles.ts:218`                                                                        | Probed (g05)                                                                                                   |
| `terminalStroke`                                   | `#000000` (3148-3151)                 | Theme                                      | `styles.ts:219-222`                                                                    |                                                                                                                |
| `terminalTextColor`                                | `#000000` (3152-3155)                 | Theme                                      | `styles.ts:223-226`                                                                    |                                                                                                                |
| `nonTerminalFill`                                  | `#FFFFFF` (3156-3159)                 | Theme                                      | `styles.ts:227-230`                                                                    |                                                                                                                |
| `nonTerminalStroke`                                | `#000000` (3160-3163)                 | Theme                                      | `styles.ts:231-234`                                                                    | Probed (g05, the fallback)                                                                                     |
| `nonTerminalTextColor`                             | `#000000` (3164-3167)                 | Theme                                      | `styles.ts:235-238`                                                                    |                                                                                                                |
| `lineColor`                                        | `#000000` (3168-3171)                 | Theme                                      | `styles.ts:239`                                                                        | Probed (g05: `#123`)                                                                                           |
| `strokeWidth`                                      | 2 (3172-3176)                         | 2                                          | `styles.ts:240` (CSS, `:289, :303, :316, :328, :343`)                                  | Probed (g07: 5 px)                                                                                             |
| `markerFill`                                       | `#000000` (3177-3180)                 | Theme (`lineColor`)                        | `styles.ts:241`                                                                        |                                                                                                                |
| `commentFill`, `commentStroke`, `commentTextColor` | 3181-3192                             | Theme                                      | `styles.ts:242-246` and the dead `.railroad-comment` rules                             | No element uses them                                                                                           |
| `specialFill`                                      | `#F0E0FF` (3193-3196)                 | Theme                                      | `styles.ts:248`                                                                        | Read by `.railroad-special rect`                                                                               |
| `specialStroke`                                    | `#8800CC` (3197-3200)                 | Theme                                      | `styles.ts:249`                                                                        | Read by `.railroad-special rect`                                                                               |
| `ruleNameColor`                                    | `#000066` (3201-3204)                 | Theme (`titleColor`)                       | `styles.ts:250`                                                                        | `.railroad-rule-name`                                                                                          |
| `markerRadius`                                     | 5 (3209-3213)                         | 5                                          | `railroadRenderer.ts:566`, `:574`, `:583`, `:596`, `:602`                              | Radius of the start and end circles, and the line lengths. Probed (g04: 8, and the viewBox grows by 3)         |
| `compactMode`                                      | `false` (3111-3114)                   | `false`                                    | `styles.ts:22`, `:205` only                                                            | **No effect.** Read into the options and used nowhere (read; the only probe, g04, also changes `markerRadius`) |
| `showMarkers`                                      | `true` (3205-3208)                    | `true`                                     | `styles.ts:44`, `:251` only                                                            | **No effect.** The markers are always drawn (read; g04 shows the start circle with `showMarkers: false`)       |
| `themeVariables`                                   | (top level)                           | the active theme's                         | `styles.ts:195`                                                                        | The 17 variables of 6.7                                                                                        |
| `railroad` section (the whole object)              | (top level, 434)                      |                                            | `styles.ts:199-202`, `railroadRenderer.ts:644`                                         | `buildRailroadStyleOptions` merges the section over the defaults                                               |

The sanitisers (`styles.ts:85-112`): a colour must match `COLOR_VALUE_PATTERN` (hex of 3, 4, 6 or 8 digits; `rgb`, `rgba`, `hsl`, `hsla`, `hwb`, `lab`, `lch`, `oklab`, `oklch` in function form; or a bare name of letters), otherwise the fallback is used. A font family must match `^[\w "',.-]+$`. A number is taken from a number or a numeric string, must be finite and not negative, and otherwise falls back. Probed: `fontSize: -1` and `padding: 'abc'` fall back (`g06-bad-numbers`). Read but not probed: `arcRadius: '7'` becomes 7. `showMarkers` and `compactMode` are taken as they are (`??`).

Apart from the two inert keys and the comment colours, every key of the schema is read.

The three aliases `railroadAbnf`, `railroadEbnf` and `railroadPeg` map to this section (`diagramConfigKeys.ts:12-14`). The base `railroad` id reads it by its own name.

### 6.9 Randomness, the clock and DOM dependencies

- **Randomness and the clock:** none. The railroad files have no `Math.random` and no `Date`, and nothing uses the seed.
- **`look`:** not read. `getStyles` receives it (`mermaidAPI.ts:330`), but nothing uses it, so `look: handDrawn` is not a railroad feature.
- **DOM dependencies a server renderer must replace:** `getBBox` of a text, in `measureText` (`railroadRenderer.ts:86`). Nothing else is measured: no root `getBBox`, no `getComputedTextLength`, no `getBoundingClientRect`. The DOM building (`append`, `attr`, `appendChild`) is structure only.
- **Other DOM use:** `selectSvgElement` (642), the `class` and the `viewBox` (`configureSvgSize`), and the `<style>` of the core.

### 6.10 Renderer error path

`draw` catches any error, logs it, and rethrows (`:662-665`). A parse error never reaches it, because the parser throws first.

## 7. Fixtures

| File (in `e2e/diagrams/railroad/pr-check/`) | Syntax | Features                                                                                                         | Probed viewBox (harness) |
| ------------------------------------------- | ------ | ---------------------------------------------------------------------------------------------------------------- | ------------------------ |
| `abnf-alternation-and-repetition.mmd`       | ABNF   | `/`, `*( … )` group repetition, two rules, a description in the front matter                                     | `351.46875 × 505`        |
| `ebnf-iso-14977-notation.mmd`               | EBNF   | `{ … }` repetition, `,` sequences, `\|`, three rules                                                             | `336.828125 × 525`       |
| `ebnf-sequences-and-choices.mmd`            | EBNF   | `( … )*`, `+` postfix, `\|` and groups, four rules                                                               | `379.71875 × 521.5`      |
| `ir-a-simple-rule.mmd`                      | IR     | One terminal, the smallest case                                                                                  | `144.484375 × 86.5`      |
| `ir-dark-theme.mmd`                         | IR     | `config.theme: dark` (the harness forces `redux-dark-color`, `common.ts:79-89`), a seven-way choice, three rules | `226.1875 × 641.5`       |
| `ir-multiple-rules-in-one-diagram.mmd`      | IR     | A full JSON grammar: seven rules, optionals and repetitions in every rule                                        | `459.796875 × 1294.5`    |
| `ir-optional-and-repetition-operators.mmd`  | IR     | `optional`, `oneOrMore`, `zeroOrMore`, `choice` of terminals, four rules                                         | `431.484375 × 560`       |
| `ir-sequences-and-choices.mmd`              | IR     | `sequence` inside `zeroOrMore(choice(...))`, a recursive reference, four rules                                   | `379.71875 × 521.5`      |
| `peg-ordered-choice-and-suffixes.mmd`       | PEG    | `/`, `<-`, `+`, `*`, `?`, five rules                                                                             | `384.28125 × 716.5`      |

All nine render with `ok: true` under the harness page (probed). None of the nine uses a symbol outside the bundled faces, so no fixture needs `uncoveredScriptReason`. None uses `look` or a randomness source. The front matter `description` is not drawn and not stored: the reference's output has no `desc` for it (probed in `svg/fixture_*.svg`). The `ir-dark-theme` fixture needs the harness's theme forcing, which `forcedDirective` already does (`common.ts:79-89`).

Exclusions a geometry harness will need: none beyond the usual. The root `viewBox` and the rule `translate` values are the things to compare. Rule order is the DOM order of `g.railroad-rule`, and there are no duplicate ids in these fixtures.

## 8. Risks, effort, open questions

### 8.1 Risks

| Risk                                                        | Level                               | Why                                                                                                                                                                                                                                                                                                   |
| ----------------------------------------------------------- | ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Text metrics of SVG text (`getBBox`)                        | High                                | Every size comes from `getBBox` widths to 1/64 px and the font's line-box height. The port's text rules (decision 0005, `crates/ferrigura-text`) must match Chromium for these fonts. Section 6.4 has the numbers to test against                                                                     |
| The four lexers with `LONGER_ALT` and the first-match order | Medium to high                      | Eight-plus keyword rules in IR, and keywords plus hidden alternatives in EBNF, ABNF and PEG. The quirks of 2.9 (2, 8, 16) all come from the order. Each needs a probe to keep                                                                                                                         |
| Visible text: DOMPurify and the placeholder decode          | Medium                              | The stored text is the sanitised text, drawn literally, and the decode happens on the whole SVG string (`mermaidAPI.ts:359`). The port's common SVG step must do the same, and sanitising must match DOMPurify on the inputs (sanitize.rs covers the flowchart's subset; coverage here is unverified) |
| Theme-derived defaults and the sanitisers                   | Medium                              | Seventeen theme variables, each with a fallback to a second variable and to the default theme. The port's typed views (decision 0009) make them explicit, which is the point, but the order of fallbacks must be kept                                                                                 |
| Repetition arithmetic                                       | Low once written                    | Four formulas with arcs. The arcs are many `A` commands with signs, and an error in a sign shows only in the path string (the specs check two of them)                                                                                                                                                |
| The default configuration                                   | Low for the harness, high for users | With no harness settings the output uses the theme's font and sizes. Under the harness the families are mapped to Lexica (`common.ts:396-403`), so it is comparable there, but not in a real browser (Q7)                                                                                             |

### 8.2 Effort

A rough estimate in Rust lines, not a measurement. The reference has 2,140 lines, of which about 1,040 are the renderer and styles (669 and 368).

| Part                                                                                      | Lines      |
| ----------------------------------------------------------------------------------------- | ---------- |
| Scanner: the shared header, hidden tokens and `LONGER_ALT`, and the four token sets (2.2) | 300 to 400 |
| Parsers: the four grammars (2.3) and the AST transforms (2.5)                             | 400 to 500 |
| Value converters and the header rules (2.4)                                               | 100 to 150 |
| DB: `addRule`, the sanitising of 3.2, the setters and the `clear`                         | 150 to 200 |
| Renderer: placement, the element emitters (6.2, 6.3), the rule and the root (6.5)         | 450 to 550 |
| Styles and options: the sanitisers, the 17 theme tokens (typed, 0009), the CSS (6.6)      | 250 to 300 |
| Tests: the 135 specs as Rust tests, the 2.8 probes as differentials                       | 400 to 600 |

Total: 2,050 to 2,650 lines with tests, and 1,650 to 2,050 without.

### 8.3 Open questions

- **Q1. Which measurement does railroad use?** The same `ferrigura-text` path as the flowchart and kanban (decision 0005), with `getBBox` widths and the line-box height of section 6.4. Confirm that the port reuses it and that its metrics include the 1/64 px rounding.
- **Q2. Reproduce the ABNF `;` quirk?** Recommended yes: a blank after the `;` breaks the rule (A-16), and a comment after it does too (A-15). It is what a user would see, and the differential can pin it.
- **Q3. String escapes and the canonical text rule.** The IR, EBNF and PEG converter decodes only `\n`, `\r` and `\t` (2.4, X-23 to X-25). Keep the reference's rule for these three grammars and record the difference in `canonical-model.md`, or change the canonical rule? The reference is the answer for the port.
- **Q4. Sanitised values in the model.** Record the railroad DB's sanitising in `canonical-model.md` next to the flowchart's, as the model of 4 requires (section 4).
- **Q5. Repetition: keep max and ignore it in the renderer?** Recommended yes. The model keeps `min` and `max` as the DB gives them, and the renderer reads `min` only, as the reference does (6.2; the parse is probed in A-37 and A-38, and the renders in `e03` to `e06`).
- **Q6. Inert configuration keys.** `compactMode`, `showMarkers`, and the three `comment*` colours have no effect. Keep them as typed fields so `pnpm port:diff config` passes, and read them nowhere, or leave them out with a named reason? Decision 0009 makes the typed view explicit, so the typed view should not invent a use for them.
- **Q7. The default configuration.** Without harness settings, the default theme's size (16 px) and colours are the output. In the harness the families are mapped onto Lexica, so the comparison holds there. In a browser they would differ (trebuchet ms). `monospace` (the `fontFamily` schema default) is not mapped and is machine-dependent (probed in g03). Record the default-configuration output as a known difference and exclude `monospace`, or compare it under the harness only?
- **Q8. The front matter title.** The reference's `clear` in `parse` drops the front matter's title (3.1, step 3, **unverified**). The title is never drawn, so this changes only the model. Follow the reference (drop) or keep the canonical precedence (front matter, then grammar)? Verify with a `Diagram.fromText` probe before deciding.
- **Q9. Four syntaxes, one model.** The code shares one model, one DB, one renderer and one styles provider. The port should do the same, with four parsers behind one `RailroadDiagram` model, and the exporter picks the parser by detector id. Confirm.
- **Q10. Entity decoding of the whole SVG.** The output of `railroad` depends on the decode of `ﬂ°`-placeholders at `mermaidAPI.ts:359`, which is part of the common pipeline, not of railroad. Check that the port's common SVG step does the same, or the railroad's `#amp;` text will differ.
- **Q11. Probe gaps.** The parse probes skipped `processDirectives` and front matter removal (section 0). They do not change any row above, because the lexer hides both, but Q8 needs the full pipeline.
