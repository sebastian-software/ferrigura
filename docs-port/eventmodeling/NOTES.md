# Event modeling (P2): research notes for the port

Status: research only. Nothing in the repository was changed, and nothing is committed. Reference: the TypeScript at HEAD `aa5c4338f` of `sebastian/port-p2`.

Conventions. Citations such as `db.ts:139-151` are under `packages/mermaid/src/diagrams/eventmodeling/`. Grammar citations are under `packages/parser/src/language/eventmodeling/`. Every other path is written in full. A rule marked **(probed)** was checked by running the reference: the built bundle `packages/mermaid/dist/mermaid.esm.mjs` in Chromium, through the harness page of `tools/port/geometry/common.ts` (`withPage`, `renderReference`, `patchText`), or the Langium parser source through `tsx`. The probe scripts and outputs are in the session scratchpad (`em-probe/`), outside the repository; Appendix A lists them. A rule marked **unverified** was inferred from the source and not run.

## 0. Summary

| Item              | Value                                                                                                                                                                                                                              |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Reference sources | 7 TypeScript files, 1,098 lines without the spec (`packages/mermaid/src/diagrams/eventmodeling/`), spec 133 lines                                                                                                                  |
| Grammar           | `event-modeling.langium` (70 lines), validator (66), module (62), token builder (7), index (1)                                                                                                                                     |
| Parser tests      | `packages/parser/tests/eventmodeling.test.ts`, 338 lines, 31 tests                                                                                                                                                                 |
| Diagram spec      | `eventmodeling.spec.ts`, 10 tests                                                                                                                                                                                                  |
| Docs inputs       | 13 blocks in `packages/mermaid/src/docs/syntax/eventmodeling.md` (12 `mermaid-example`, 1 `mermaid`); all 13 render (probed)                                                                                                       |
| Other inputs      | The diagram spec's 10 tests use 11 texts, and `rendering-util/multi-diagram-id-uniqueness.spec.ts` has one more. All render except the duplicate-ID text, which is meant to fail (probed)                                          |
| Fixtures          | 6 `.mmd` files, all in `e2e/diagrams/eventmodeling/pr-check/`; `visual-check` and `nightly-check` do not exist for this type                                                                                                       |
| Parser            | Langium, not Jison. Ferrigura needs a hand-written parser that copies Langium's token order, because that order is observable (section 2.2)                                                                                        |
| Renderer          | d3 selection calls only (no d3 layout, no d3 shape). The layout is the DB's own arithmetic. Boxes are HTML inside `foreignObject`, always, because `htmlLabels` does not apply here                                                |
| Biggest risk      | Every box size comes from text measured in **Trebuchet MS Bold** (the first family of `"trebuchet ms", verdana, arial, sans-serif`, which resolves on the reference Mac). Ferrigura does not ship that font (sections 6.7 and 8.1) |
| Effort            | About the size of the journey port (1,151 TypeScript lines without specs, here 1,098), plus a grammar of about 200 lines. An estimate, unverified; section 8.2                                                                     |

The type draws events as boxes in swimlanes. Each frame of the text becomes one box. The box goes into the lane of its entity type (UI, command or read model, event) or into a lane named by its namespace. A box is placed 10 px right of the last box of its own lane; when its lane differs from the previous frame's, it is placed with its left edge 70 px left of the previous box's right edge. Relations join a box to the nearest earlier box in another lane, and each explicit `->>` source adds one more relation. Box text is the entity name in bold, plus the inline or referenced data in a `<code>` block.

## 1. Sources

| File                                                                     | Lines | What it holds                                                                                                                                                              |
| ------------------------------------------------------------------------ | ----- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `diagrams/eventmodeling/db.ts`                                           | 702   | State machine: frame positioning, swimlanes, relations, text measurement, the DB object (`db.ts:683-702`)                                                                  |
| `diagrams/eventmodeling/renderer.ts`                                     | 167   | `draw`: swimlanes, boxes, relations, marker, viewBox (`renderer.ts:128-163`)                                                                                               |
| `diagrams/eventmodeling/types.ts`                                        | 165   | Interfaces for the DB context, boxes, lanes, relations, the event and command types of the DB (`types.ts:16-165`). The command and event constants are used by the DB only |
| `diagrams/eventmodeling/parser.ts`                                       | 28    | `parse` calls `@mermaid-js/parser`, then `db.setAst` and `populateCommonDb` (`parser.ts:8-15`)                                                                             |
| `diagrams/eventmodeling/detector.ts`                                     | 21    | Detection by `/^\s*eventmodeling/` (`detector.ts:7`). It also accepts `eventmodelingX`, which then fails in the parser (probed)                                            |
| `diagrams/eventmodeling/diagram.ts`                                      | 12    | Wires parser, db, renderer and styles (`diagram.ts:7-12`)                                                                                                                  |
| `diagrams/eventmodeling/styles.js`                                       | 3     | Returns the empty string (`styles.js:1`). The diagram adds no CSS of its own                                                                                               |
| `diagrams/eventmodeling/eventmodeling.spec.ts`                           | 133   | 10 tests (section 5)                                                                                                                                                       |
| `packages/parser/src/language/eventmodeling/event-modeling.langium`      | 70    | Grammar                                                                                                                                                                    |
| `packages/parser/src/language/eventmodeling/event-modeling-validator.ts` | 66    | Source-type checks; registered, but never run by `parse()` (section 2.5)                                                                                                   |
| `packages/parser/src/language/eventmodeling/module.ts`                   | 62    | Services. `ValueConverter` is the common one, and `TokenBuilder` (`tokenBuilder.ts`) adds the `%%` boundary to `eventmodeling` only                                        |
| `packages/parser/tests/eventmodeling.test.ts`                            | 338   | Parser and validator tests (section 5)                                                                                                                                     |
| `packages/mermaid/src/docs/syntax/eventmodeling.md`                      | 336   | The syntax documentation and 13 example blocks. A copy at `docs/syntax/eventmodeling.md` is different; the `packages/` copy is taken as the input                          |

Other places that name the type: `packages/mermaid/src/defaultConfig.ts:282-284` (the `eventmodeling` section), `config.type.ts:2172-2185`, `schemas/config.schema.yaml:2726-2741`, `diagram-api/diagram-beta-policy.spec.ts:42` (in `NEVER_BETA_DIAGRAMS`: a stable name that never had a beta phase), `rendering-util/multi-diagram-id-uniqueness.spec.ts:226-230` (one input), `tools/port/diff-config.ts:231` (already in the configuration differential).

## 2. Parser

### 2.1 What happens before the grammar

The text takes these steps before Langium sees it. All of them are the common pipeline; only the last two are specific to this type.

1. `Diagram.fromText` (`Diagram.ts:19-85`) detects the type, clears the DB (`Diagram.ts:49`, `db.ts:58-61`), and sets the front matter title (`Diagram.ts:52-54`).
2. `preprocessDiagram` (`preprocess.ts:58-81`):
   - `cleanupText` (`preprocess.ts:6-17`) turns CRLF and CR into LF, and rewrites `="x"` inside tags to `='x'`. A data value such as `{ <a href="x">y</a> }` therefore reaches the parser with single quotes.
   - Front matter is read and removed (`preprocess.ts:19-31`; `diagram-api/frontmatter.ts:24`). Its lines are **not** counted in the line numbers the parser reports (probed, `fm_dupe`: the duplicate of frame `01` is on line 4 of the text that follows the front matter).
   - Directives are read (`detectInit`, `wrap`) and removed by `removeDirectives` (`preprocess.ts:33-45`, `utils.ts:203-205`). A removed directive keeps its line, so it still counts (probed, `dir_dupe`: line 5, with two directive lines above the header, one of them added by the harness).
   - `withComments` is the text with directives removed (`preprocess.ts:64`). `cleanedCode` removes the `%%` lines (`preprocess.ts:65`, `diagram-api/comments.ts:6-8`).
3. Source choice (`Diagram.ts:66-76`): this DB sets `preserveCommentsWhenParsing: true` (`db.ts:685`), so the parser receives `withComments`, with its `%%` lines. If `withComments` is longer than `maxTextSize` (default 50,000; `schemas/config.schema.yaml:83-86`), the parser gets `cleaned` instead, and the line numbers then no longer count the comments (`Diagram.ts:68-73`).
4. `encodeEntities` (`Diagram.ts:77`, `utils.ts:874-897`) turns `#name;` into the placeholder `ﬂ°name¶ß`, and `#123;` into `ﬂ°°123¶ß`. A newline is appended. The reverse runs on the finished SVG (`mermaidAPI.ts:359`, `utils.ts:904-906`). Probed: `#quot;` in data is drawn as `"` and `#38;` as `&amp;` (`entity_quot`, `entity_num_entity`).
5. `parse('eventmodeling', text)` (`packages/parser/src/parse.ts:70-74` for the initialiser, the overload at `parse.ts:123`). If the lexer or the parser reports an error, the call throws `MermaidParseError` (`parse.ts:146-147`). Nothing else is checked: validation is not run.
6. `populateCommonDb` (`parser.ts:13`, `diagrams/common/populateCommonDb.ts:4-14`) sets the accessibility texts and the title if they are non-empty.

### 2.2 Tokens, in the order Langium builds them

Decision 0002 (`docs-port/decisions/0002-hand-written-parsers.md`) applies: the port is a hand-written parser, and a grammar whose lexer takes the first rule in its written order is run by a scanner over the rule table (`ferrigura-syntax/src/scan.rs`, as the journey scanner does), not by `logos`, which takes the longest match. Error messages are the port's own, and specs assert failure and position (0002). This section gives the rule table that the scanner must run.

Langium puts the keyword tokens first, sorted by length, descending, and then the terminals in the order of the grammar's terminal rules (`langium/lib/parser/token-builder.js:21-35`, `77-84`; here that order is the grammar's own). Chevrotain tries the tokens in that order and takes the first match, not the longest one: `title Foo` at a line start lexes as the identifier `title`, because `EM_ID` comes before `EM_TITLE` and matches a shorter prefix (probed). A keyword that is also a prefix of `EM_ID` carries `LONGER_ALT` = `EM_ID` (`token-builder.js:91-95`). The order was read from the built lexer (probed, `tokens-probe.mts`, 47 tokens).

Keywords, by length: `eventmodeling`, `resetframe`, `readmodel`, `processor`, `timeframe`, `command`, `entity`, `event`, `jsobj`, `figma`, `given`, `json`, `salt`, `html`, `text`, `data`, `note`, `when`, `then`, `rmo`, `cmd`, `evt`, `pcr`, `uri`, `gwt`, `ui`, `md`, `tf`, `rf`, `[[`, `]]`, `` ` ``, `.`, `->>`. Keywords are case-sensitive.

`eventmodeling` carries a boundary: it must be followed by whitespace, by `%%`, or by the end of the text (`tokenBuilder.ts:3-5`, `common/tokenBuilder.ts:21-26`). The other keywords have no boundary, but `LONGER_ALT` makes `tfoo` an identifier.

Terminals, in order:

| #   | Token                    | Pattern (verbatim)                                                     | Note                                                                       |
| --- | ------------------------ | ---------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| 1   | `EM_ID`                  | `/[_a-zA-Z][\w_]*/`                                                    | ASCII only. Used for entity names and entity parts                         |
| 2   | `EM_FID`                 | `/\d{1,3}/`                                                            | Frame numbers: one to three digits                                         |
| 3   | `EM_DATA_INLINE`         | `/\{(.*)\}\|"(.*)"\|'(.*)'/`                                           | Greedy to the last `}`, `"` or `'` on the line                             |
| 4   | `EM_DATA_BLOCK`          | `/\{[\t ]*\r?\n(?:[\S\s]*?\r?\n)?\}(?:\r?\n\|(?!\S))/`                 | Opening `{`, optional spaces, a newline; the closing `}` must start a line |
| 5   | `EM_ACC_DESCR`           | `/[\t ]*accDescr(?:[\t ]*:([^\n\r]*?(?=%%)\|[^\n\r]*)\|\s*{([^}]*)})/` | Accessibility description, line or block form                              |
| 6   | `EM_ACC_TITLE`           | `/[\t ]*accTitle[\t ]*:(?:[^\n\r]*?(?=%%)\|[^\n\r]*)/`                 | Accessibility title                                                        |
| 7   | `EM_TITLE`               | `/[\t ]*title(?:[\t ][^\n\r]*?(?=%%)\|[\t ][^\n\r]*\|)/`               | Title                                                                      |
| 8   | `EM_WS`                  | `/\s+/`                                                                | Skipped. Includes newlines                                                 |
| 9   | `EM_YAML`                | `/---[\t ]*\r?\n(?:[\S\s]*?\r?\n)?---(?:\r?\n\|(?!\S))/`               | Hidden                                                                     |
| 10  | `EM_DIRECTIVE`           | `/[\t ]*%%{[\S\s]*?}%%(?:\r?\n\|(?!\S))/`                              | Hidden                                                                     |
| 11  | `EM_SINGLE_LINE_COMMENT` | `/[\t ]*%%[^\n\r]*/`                                                   | Hidden                                                                     |
| 12  | `EM_ML_COMMENT`          | `/\/\*[\s\S]*?\*\//`                                                   | Hidden                                                                     |
| 13  | `EM_SL_COMMENT`          | `/\/\/[^\n\r]*/`                                                       | Hidden                                                                     |

Consequences, all probed:

- **Titles only work after another token on the same line.** `EM_WS` (position 8) comes after `EM_TITLE` and matches a newline together with the indentation that follows it. So a `title`, `accTitle:` or `accDescr:` at the start of a line is an identifier, and the parse fails (`title_col0`, `title_indented`, `accTitle_col0`, `accDescr_block`). After the header or a frame on the same line they work: `eventmodeling title Foo` gives title ` title Foo`, `eventmodeling accTitle: Foo bar` gives accTitle ` accTitle: Foo bar` (both raw, with the keyword in the value), and `tf 01 ui A title Foo` works.
- `tf 01 evt title` fails: the space before `title` is on the same line, so `EM_TITLE` takes it. `tf 01 evt` followed by a newline and `title` works and names the entity `title`.
- A keyword cannot be an identifier: `tf 01 evt text` and `tf 01 evt data` fail, and so do `A.ui` and `A.text`.
- Frames are not line-based. `tf 01 ui A tf 02 cmd B` on one line and `tf 01` / `ui` / `A` on three lines both parse.
- A data block closes with a `}` at column 0. An indented `  }` fails, and so does `  a: 1}` (probed: `data_close_indented`, `block_inline_close`).
- `data D { a: 1 }` fails, because the one-line form is lexed as `EM_DATA_INLINE`, which the block rule does not accept (probed: `data_one_line`). `data D {}` fails the same way.

### 2.3 Grammar and what the DB receives

| Rule (`event-modeling.langium`)   | Accepts                                                                                                              | DB reads                                                                                                |
| --------------------------------- | -------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `EventModel` (9-11)               | `eventmodeling`, then zero or more of: `accDescr`, `accTitle`, `title`, entity, frame, data, note, gwt, in any order | title, accTitle, accDescr (through `populateCommonDb`), frames, data entities                           |
| `EmModelEntityType` (14-15)       | exactly `rmo readmodel ui cmd command evt event pcr processor`                                                       | `modelEntityType`: lane choice, colours, and nothing else                                               |
| `EmDataType` (17-18)              | exactly `json jsobj figma salt uri md html text`, in backticks                                                       | not read (`dataType` is never used)                                                                     |
| `EmDataInline` (20)               | optional `` `type` ``, then `EM_DATA_INLINE`                                                                         | `dataInlineValue` (the text, quotes and braces included)                                                |
| `EmDataBlock` (22)                | optional `` `type` ``, then `EM_DATA_BLOCK`                                                                          | `dataBlockValue` of the data entity                                                                     |
| `QualifiedName` (24-25)           | `EM_EID ('.' EM_EID)*`, any depth                                                                                    | `entityIdentifier`                                                                                      |
| `EmTimeFrame` (27-28)             | `tf`/`timeframe`, `EM_FI`, type, identifier, `('->>' source)*`, `('[[' ref ']]')?`, inline data?                     | `$type`, name, type, identifier, `sourceFrames[].$refText`, `dataReference.$refText`, `dataInlineValue` |
| `EmResetFrame` (30-31)            | the same with `rf`/`resetframe`                                                                                      | the same, plus the reset flag (`db.ts:597`)                                                             |
| `EmModelEntity` (36-37)           | `entity` plus a qualified name                                                                                       | nothing (`db` never reads `modelEntities`, by source; the parser keeps the entity, probed)              |
| `EmDataEntity` (39-40)            | `data` name `EmDataBlock`                                                                                            | name and `dataBlockValue`                                                                               |
| `EmNoteEntity` (42-43)            | `note` frame-ref `EmDataBlock`                                                                                       | nothing (`db` never reads `noteEntities`)                                                               |
| `EmGwt`, `EmGwtStatement` (45-49) | `gwt` frame-ref `given` statements, optional `when`, `then` statements                                               | nothing (`db` never reads `gwtEntities`)                                                                |
| `EM_FI`, `EM_EID` (51-55)         | `EM_FID` (1-3 digits) and `EM_ID`                                                                                    | strings, so `001` and `1` are different frames                                                          |

The sources (`->>`) and the references (`[[ ]]`) are Langium references. `parse()` only parses and returns the AST (`parse.ts:144-150`), with no linking step, so the DB works with the written text (`$refText`), and unresolved names are silently ignored (probed: `missing_source`, `block_missing`). The DB reads `$refText` at `db.ts:111` and `db.ts:360`.

The order inside a frame is fixed: sources first, then `[[ref]]`, then inline data (probed):

| Input                         | Result |
| ----------------------------- | ------ |
| `tf 02 rmo B ->> 01 { x: 1 }` | parses |
| `tf 02 rmo B ->> 01 [[D]]`    | parses |
| `tf 02 rmo B ->> 01 "x"`      | parses |
| `tf 02 rmo B { x: 1 } ->> 01` | fails  |
| `tf 02 rmo B [[D]] ->> 01`    | fails  |
| `tf 01 evt A [[D]] { x: 1 }`  | parses |
| `tf 01 evt A { x: 1 } [[D]]`  | fails  |

### 2.4 Acceptance, probed

Each row is an input the reference was run on (`parse-probe.ts`, `parse-probe2.ts`, the `Parsing failed` message is shortened).

| Input (after `eventmodeling`)                            | Result | Note                                                                                                            |
| -------------------------------------------------------- | ------ | --------------------------------------------------------------------------------------------------------------- |
| `tf 01 ui A`                                             | parses | baseline                                                                                                        |
| `tf 1 ui A`, `tf 001 ui A`, `tf 000 ui A`                | parses | names are strings                                                                                               |
| `tf 1234 ui A`, `tf a1 ui A`, `tf01 ui A`                | fails  | `EM_FID` is 1 to 3 digits; `tf01` is one identifier                                                             |
| `tf 01 evt tfoo`, `tf 01 evt _Foo`, `tf 01 evt A.B.C.D`  | parses | `LONGER_ALT`; any depth of qualified names                                                                      |
| `tf 01 evt 1Foo`, `tf 01 evt Foo-Bar`, `tf 01 evt Größe` | fails  | `EM_ID` is ASCII and cannot start with a digit                                                                  |
| `tf 01 evt A.`, `tf 01 evt .A`, `tf 01 evt A.ui`         | fails  | the dot needs a name on both sides                                                                              |
| `tf 01 UI A`, `tf 01 foo A`, `TF 01 ui A`                | fails  | types and keywords are lowercase and closed                                                                     |
| `eventmodelingX`, `EventModeling`, `resetFrame 01 evt A` | fails  | keywords are exact                                                                                              |
| `eventmodeling` twice                                    | fails  | one header only                                                                                                 |
| `eventmodeling\ttf 01 ui A`                              | parses | any whitespace after the header                                                                                 |
| `tf 01 ui A tf 02 cmd B` (one line)                      | parses | frames are not line-based                                                                                       |
| `tf 01 ui A %% comment`, `%%` line, `/* */`, `//`        | parses | hidden tokens                                                                                                   |
| `eventmodeling title Foo`                                | parses | title is ` title Foo` (raw)                                                                                     |
| `eventmodeling accTitle: Foo`                            | parses | accTitle is ` accTitle: Foo` (raw)                                                                              |
| `eventmodeling accDescr: Foo`                            | parses | accDescr is ` accDescr: Foo` (raw)                                                                              |
| `title Foo` on its own line                              | fails  | section 2.2                                                                                                     |
| `tf 01 ui A ->> 09` (missing source)                     | parses | the DB drops the source                                                                                         |
| `tf 01 evt A` then `tf 02 rmo B ->> 01 ->> 01`           | parses | sources are a set in the DB                                                                                     |
| `tf 01 cmd A { x: 1 }`, `{x: 1}`, `{ x: 1 } { y: 2 }`    | parses | one token: the greedy match takes everything to the last `}`                                                    |
| `tf 01 cmd A "hello"`, `'hello'`                         | parses | the quotes stay in the value                                                                                    |
| `` `json`{ x: 1 } ``                                     | parses | type `json`                                                                                                     |
| `` `xml`{ x: 1 } ``                                      | fails  | the type is not in the list                                                                                     |
| `{ x: 1` (unclosed), `'x`                                | fails  |                                                                                                                 |
| `{ x:` line break `1 }`                                  | fails  | the inline rule reads one line                                                                                  |
| `data D {` newline `  a: 1` newline `}`                  | parses | value `{\n  a: 1\n}\n`                                                                                          |
| `data D {` newline `}`                                   | parses | value `{\n}\n`                                                                                                  |
| `data D {  ` (trailing spaces) newline ... `}`           | parses | value keeps the spaces; the DB then cuts at `{\n`, which is not there (section 6.4.1, unverified in the render) |
| `data D{` newline ...                                    | parses |                                                                                                                 |
| `data D` `` `md` `` `{` ...                              | parses | the type is kept and not used                                                                                   |
| `data D {}`, `data D { a: 1 }`                           | fails  | section 2.2                                                                                                     |
| `data D {` newline `  a: 1` newline `  }`                | fails  | the closing brace must start the line                                                                           |
| `data D {` ... `}` then ` tf 02 cmd B` on the same line  | parses | the closing brace may be followed by whitespace                                                                 |
| `tf 01 evt A [[Nope]]`                                   | parses | no data; no error                                                                                               |
| `note 01 `md` {` ... `}`                                 | parses | notes are parsed and dropped                                                                                    |
| `note 01 { x: 1 }`                                       | fails  | a note needs a block                                                                                            |
| `gwt 01` with `given` and `then`, no `when`              | parses |                                                                                                                 |
| `gwt 01` without `then`                                  | fails  |                                                                                                                 |
| `entity Foo.Bar`                                         | parses | dropped                                                                                                         |
| `%%{init: {"theme": "dark"}}%%` before the header        | parses | the directive is removed first                                                                                  |
| `---` front matter before the header                     | parses | removed first                                                                                                   |
| a `%%` line between frames                               | parses |                                                                                                                 |

Frame order and duplicates: frames keep their text order; the DB rejects a repeated frame name, not a repeated line (section 3.2, call 3).

### 2.5 Validation is never run

`EventModelingValidator.checkSourceFrameTypes` (`event-modeling-validator.ts:24-46`) rejects, for example, a `ui` fed by an `evt`. It is registered (`event-modeling-validator.ts:11-21`), but `parse()` only reads `lexerErrors` and `parserErrors` (`parse.ts:146`), so no validation runs. Probed: `tf 01 ui A` then `tf 02 evt B ->> 01` renders, and so does `tf 01 rmo A` then `tf 02 evt B ->> 01` (both with no error). The 26 validator tests in `packages/parser/tests/eventmodeling.test.ts` call the validator directly. A port can keep the validator as a function that is tested on its own, but it does not belong to the render path.

### 2.6 Errors

- A lexer or parser error throws `MermaidParseError` with `Parsing failed: ` and one line per error: `Lexer error on line L, column C: …` or `Parse error on line L, column C: Expecting …` (`parse.ts:146-147` and the class after it). The expected-token text is Chevrotain's (for example `Expecting token of type 'EM_ID' but found \`text\``). The text is not a contract of the type, only the failure and its first position are (proposal, section 8.3).
- The DB reports a duplicate frame name (`db.ts:139-151`): `Duplicate event modeling frame ID "01" on line 4`. The line is `frame.$cstNode.range.start.line + 1` in the text the parser saw, so it counts directive lines and comment lines and not front matter (probed; the spec, test 8, checks a comment line).
- `db.getState()` throws `No data for EventModel` when no AST is stored (`db.ts:92-94`). `setAst` clears the stored AST before it checks for duplicates (`db.ts:140`), so a failed parse leaves the DB empty (spec test 7).
- `decidePositionRelation` throws `Target box not found for frame …` (`db.ts:589-591`). It cannot happen, because a frame's own box is added first (`db.ts:99-104`; unverified that no path skips it).
- A diagram with no frames renders an empty SVG with a viewBox of `-30 -30 60 60` and `max-width: 60px` (probed, `empty`).

## 3. DB

### 3.1 State and clear

- `store` (`db.ts:81-85`) holds one thing: the parsed AST. The positions are not stored. `getState()` (`db.ts:87-137`) recomputes everything from the AST on every call, so two calls give two equal copies.
- `clear()` (`db.ts:58-61`) resets `store` to `{}` and calls the common `clear` (`diagrams/common/commonDb.ts:10-14`), which resets the accessibility title and description and the diagram title.
- `setOptions` and `getOptions` are no-ops (`db.ts:50-56`). `getConfig` (`db.ts:71-79`) merges the defaults and is not used by the layout code, which reads `commonGetConfig()` instead (`db.ts:291`, `db.ts:335`).
- `getDirection` returns `'LR'` (`db.ts:67-69`). The renderer does not use it (unverified against every consumer of `getDirection`).
- `preserveCommentsWhenParsing: true` (`db.ts:685`). It decides the parser's input (section 2.1, step 3).

### 3.2 Calls in order

| #   | Call                                   | What it does                                                                         | Source                                  |
| --- | -------------------------------------- | ------------------------------------------------------------------------------------ | --------------------------------------- |
| 1   | `db.clear()`                           | Empties the store and the common texts                                               | `Diagram.ts:49`, `db.ts:58-61`          |
| 2   | `setDiagramTitle(front matter title)`  | Sanitised, not trimmed                                                               | `Diagram.ts:52-54`, `commonDb.ts:28-30` |
| 3   | `db.setAst(ast)`                       | Duplicate check, then stores the AST                                                 | `parser.ts:10-12`, `db.ts:139-151`      |
| 4   | `populateCommonDb`                     | Sets accDescr, accTitle and title when non-empty                                     | `populateCommonDb.ts:4-14`              |
| 5   | `getDiagramProps()`                    | Returns the constants of section 3.4                                                 | `db.ts:177-179`                         |
| 6   | `getState()`                           | Dispatches one position command per frame, then one per source, then sorts the lanes | `db.ts:87-137`                          |
| 7   | `getAccTitle()`, `getAccDescription()` | Read by the accessibility code, after the draw                                       | `mermaidAPI.ts:621-622`                 |

The common setters (`commonDb.ts:16-30`) are: `setAccTitle` sanitises and removes leading whitespace; `setAccDescription` sanitises and turns `\n\s+` into `\n`; `setDiagramTitle` only sanitises. So the accessibility title is `accTitle: Hello there` in the output, with the keyword (probed, `accTitle_header`). The description keeps its leading space (` accDescr: Desc here`, probed). The diagram title is never drawn (probed, `title_header`; the renderer does not call `getDiagramTitle`).

### 3.3 The sanitising

`sanitizeText` (`diagrams/common/common.ts:71-83`) is applied to:

- the entity name, after `extractName` (`db.ts:336`). Identifiers cannot hold markup, so this does nothing in practice;
- the inline data value (`db.ts:353`) and the data block value (`db.ts:367`). These are the only texts with markup;
- the three common texts (section 3.2).

With the default configuration (`htmlLabels` true, `securityLevel` strict) `sanitizeText` runs `DOMPurify.sanitize` twice: once with the default configuration (`removeScript`, `diagrams/common/common.ts:56-68`), and once with `FORBID_TAGS: ['style']`. Probed (in the reference, through the harness): a `<script>` element is removed with its content; `<img onerror>` is removed; `<b>x</b>`, `<i>`, `<u>`, `<em>`, `<p>` and `<div>` survive; `&` becomes `&amp;`; `<` becomes `&lt;`; `"` stays.

The other tags do not survive because of what happens after sanitising, not because of DOMPurify. `wrapLabel` splits the data text on spaces and `replaceAll(' ', '&nbsp;')` joins it again (`db.ts:354-355`). So a tag with an attribute, `<a href="…">`, `<span style="…">` or `<b class="…">`, has its inner space turned into `&nbsp;`. The reference then draws the text alone (probed: `tag_a_dq`, `tag_span_style`, `tag_b_attr`, `tag_img_onerror`). The mechanism is inferred from the string; the browser's parse of it was not inspected (unverified). A port must produce the same string, and the same browser will then do the same thing. A port does not need to parse the markup in the way a browser does. Unverified: how far the flowchart's DOMPurify subset (`crates/ferrigura-syntax/src/sanitize.rs`) covers the tags used here.

### 3.4 The defaults

`diagramProps` (`db.ts:153-175`), used everywhere in the DB and the renderer:

| Key                            | Value                                                                                                                      | Used for                                                                                                                              |
| ------------------------------ | -------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `swimlaneMinHeight`            | 70                                                                                                                         | Initial lane `height` and `maxHeight` (`db.ts:461-463`)                                                                               |
| `swimlanePadding`              | 15                                                                                                                         | Box top offset in its lane (`db.ts:499`, `renderer.ts:17`), lane height (`db.ts:494-495`) and the lane rect width (`renderer.ts:115`) |
| `swimlaneGap`                  | 10                                                                                                                         | Gap between lanes (`db.ts:461`, `db.ts:535`)                                                                                          |
| `boxPadding`                   | 10                                                                                                                         | Gap between boxes in a lane, and around the box size (`db.ts:429`, `db.ts:479`, `db.ts:489`)                                          |
| `boxOverlap`                   | 90                                                                                                                         | Overlap of a box in another lane with the last drawn box (`db.ts:436`)                                                                |
| `boxDefaultY`                  | 0                                                                                                                          | Unused                                                                                                                                |
| `boxMinWidth`, `boxMaxWidth`   | 80, 450                                                                                                                    | Clamp of the text width (`db.ts:474-478`)                                                                                             |
| `boxMinHeight`, `boxMaxHeight` | 80, 750                                                                                                                    | Clamp of the text height (`db.ts:481-485`)                                                                                            |
| `contentStartX`                | 250                                                                                                                        | x of the first box, and of the first box in a lane with no earlier box (`db.ts:426`, `db.ts:433`)                                     |
| `textMaxWidth`                 | 430                                                                                                                        | Wrap width of names and data (450 - 2 \* 10)                                                                                          |
| `boxTextFontWeight`            | `bold`                                                                                                                     | Unused: names are written with `<b>`                                                                                                  |
| `boxTextPadding`               | 10                                                                                                                         | Padding of the text inside the box size (`db.ts:404-405`)                                                                             |
| `swimlaneTextFontWeight`       | `bold`                                                                                                                     | Lane label (`renderer.ts:121`)                                                                                                        |
| label strings                  | `UI/Automation` (`labelUiAutomation`), `UI/A: ` prefix; `Command/Read Model`, `C/RM: ` prefix; `Events`, `Stream: ` prefix | Lane labels (`db.ts:169-174`)                                                                                                         |

Lane indices (`db.ts:233-288`). Without a namespace, UI and processor entities go to lane 0 (UI/Automation), command and read model entities to lane 100 (Command/Read Model), and event entities to lane 200 (Events). A namespaced entity gets the next free index in the range of its kind: 1-99 for UI and processor, 101-199 for command and read model, 201-299 for events. The next free index is the largest existing key in the range plus one, or the range's lower bound plus one when the range is empty (`findNextAvailableIndex`, `db.ts:215-231`).

### 3.5 Frames, step by step

Each frame is handled as follows, in AST order.

1. **Text** (`calculateTextProps`, `db.ts:330-397`). Section 6.7 gives the measurement.
2. **Box size** (`decidePositionFrame`, `db.ts:399-417`): the text width plus 2 × 10, and the text height plus 2 × 10.
3. **Lane** (`calculateSwimlaneProps`, `db.ts:233-288`, section 3.4). A new lane gets `r: 0`, `y = index * 70 + 10` (corrected later), `height: 70`, `maxHeight: 70` (`db.ts:457-464`). The label is set when the lane is created and never changed (`db.ts:457-464`).
4. **x** (`calculateX`, `db.ts:419-437`):
   - no earlier frame: 250;
   - the earlier frame is in the same lane and the lane's `r` is set: the lane's `r` (the right edge of its last box) plus 10;
   - otherwise: the last drawn box's `r` minus 90, plus 10. Since `r` is that box's right edge plus 10, the new box's left edge is 70 px left of the last drawn box's right edge (probed: the events box at x 250 with width 121 is followed by a UI box at x 301). The rule is about the last drawn box, not the last box of the lane.
5. **Box size, clamped** (`db.ts:473-486`): width = clamp(dimension width, 80, 450) + 20; height = clamp(dimension height, 80, 750) + 20. The dimension of step 2 already includes the 20 of `boxTextPadding`; the 20 added here is `boxPadding` on each side.
6. `r = x + width + 10`, the lane's `r = x + width`, the lane's `maxHeight` grows to the box height, and its `height` becomes `max(70, maxHeight) + 30` (`db.ts:489-495`).
7. **Lane positions** (`db.ts:523-536`): the first lane in index order gets `y = 0`, and each later lane gets the previous lane's `y + height + 10`. This runs after every frame, so the final values are the ones used.
8. `maxR` is the largest `r` of all boxes (`db.ts:490`). A lane's `r` is always smaller than its last box's `r`, by reading the code (unverified in every case).
9. The box's own `y` is `15 + the lane's y at that moment` (`db.ts:499`), which is stale when a later lane moves the lane. The renderer does not use it (`renderer.ts:17`), and neither do the relation `source`/`target` coordinates (`db.ts:625-632`).

### 3.6 Relations

Relations come from `decidePositionRelation` (`db.ts:580-615`) and `evolveRelationPositioned` (`db.ts:617-642`):

- A frame that is the first frame of the text and has no source gets no relation (`db.ts:583-585`).
- A frame with explicit sources (`->>`): one relation per source frame, in AST order (`db.ts:107-121`). The sources are found by name among all frames of the AST, so a source that comes later in the text has no box yet and gives no relation. A source name that does not exist gives none (probed: `forward_source`, `missing_source`). A source that is the frame itself gives a self relation (probed: `self_source`, `first_frame_with_src`). When the frame has sources, the inferred relation is not made, even if no source resolves (probed: `missing_source`).
- A frame with no sources, which is not a reset frame: the nearest earlier box in a **different lane** is the source (`findBoxByLineIndex`, `db.ts:561-578`), searched backwards from the previous frame. A frame in the same lane as the previous frame therefore gets no relation from that previous frame, and an earlier box in another lane is used instead (probed: `relation_prev_other_lane`, `relation_same_lane_chain`, `order_lanes_out_of_sequence`, where the relation of a third frame goes to the box two positions back).
- A reset frame (`rf`/`resetframe`) with no sources gets no inferred relation (`db.ts:597-599`, probed: `reset_no_src`).
- The relation's stroke is `#000` and fill `none` (`db.ts:620-624`).

### 3.7 Frame order and the namespace quirk

`calculateSwimlaneProps` reads the namespace from the entity identifier (`extractNamespace`, `db.ts:189-195`) and looks for an existing lane with the same `namespace` (`findSwimlaneByNamespace`, `db.ts:205-213`). The lane objects are created without a `namespace` field (`db.ts:457-464`), so the lookup never finds one. Every namespaced frame therefore gets a new lane, even when the namespace repeats (probed):

| Input                                                                                               | Lanes (index order), label                                |
| --------------------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| `rf 01 evt Inventory.A`, `rf 02 evt Inventory.B`, `rf 03 evt Inventory.C`                           | three lanes, all `Stream: Inventory`, at y 0, 140, 280    |
| `rf 01 evt Inv.A`, `rf 02 evt Ext.B`                                                                | `Stream: Inv`, `Stream: Ext`                              |
| `tf 01 ui X.Screen`, `tf 02 cmd X.Add`, `tf 03 evt X.Added`, `tf 04 cmd Y.Add`, `tf 05 evt Y.Added` | `UI/A: X`, `C/RM: X`, `C/RM: Y`, `Stream: X`, `Stream: Y` |
| `tf 01 ui X.A`, `tf 02 ui Y.B`, `tf 03 ui X.C`                                                      | `UI/A: X`, `UI/A: Y`, `UI/A: X`                           |

The docs say that "each combination of the Namespace and Entity type forms new Swimlane" (`packages/mermaid/src/docs/syntax/eventmodeling.md`, line 298). The code does not do that. The port should reproduce the code (the M6 rule: quirks are reproduced and listed), and this is a quirk to list.

The lanes of an entity without a namespace are the three fixed ones: 0, 100, 200 (`db.ts:255`, `db.ts:271`, `db.ts:286`). A dotted name with three or more parts has no namespace, and its name is the whole identifier (`extractName`, `db.ts:197-203`). This is unverified, but the code is simple.

## 4. Canonical model proposal

The differential compares the model in camelCase, as the P1 models do. The fields below are a proposal for the coordinator.

```text
version: 1
title, accTitle, accDescr          raw texts, the common setters' rules applied (no trimming here)
frames[]                           { index, name, kind: "time" | "reset", modelEntityType,
                                     entityIdentifier, sourceFrames: [name as written],
                                     dataInline: text | null, dataReference: name | null }
dataEntities[]                     { name, dataBlockValue }
modelEntities[], notes[], gwts[]   parsed and kept for the parser differential, not used by the DB
layout                             only when the DB is run (section 4.1)
  lanes[]                          { index, label, y, height }, in index order
  boxes[]                          { frameIndex, laneIndex, x, width, height, fill, stroke, html }
  relations[]                      { sourceFrameIndex, targetFrameIndex }
  maxR
```

What is left out, and why:

- The DB's own `box.y` and the relation `source`/`target` points: stale and unused (section 3.5 step 9).
- The `previousFrame` and `previousSwimlaneNumber` state: internal.
- The measured widths and heights as such: they depend on the font (section 4.1). Only their results (the box sizes and the `html`) go into the model.
- The `dataType` of data entities and `EmDataType`: parsed, never read.

### 4.1 Text measurement in the differential

The DB needs `getBBox` (through `calculateTextDimensions`, section 6.7), and the Node exporters do not have a DOM. The spec stubs `getBBox` to 100 × 20 (`eventmodeling.spec.ts:8-23`), and with that stub every word measures the same, so the wrapping and the `html` would not be the reference's. Two options for the coordinator:

- (a) The exporter runs in Chromium, with the reference's DB bundled from source, so the real measurement is used. This is the most faithful, and the cost is a bundle step.
- (b) The exporter takes a table of measured lines (`text` → width, height), recorded in Chromium with the reference's font list, and the port uses the same table. The layout logic is then tested without a font, and the text metrics are tested by the table. Recommended, with the table generated by the probe in Appendix A.

## 5. Specs

### 5.1 Files and counts

| File                                           | Tests | What they test                                                                                         |
| ---------------------------------------------- | ----- | ------------------------------------------------------------------------------------------------------ |
| `diagrams/eventmodeling/eventmodeling.spec.ts` | 10    | Parser (tests 1-6, 10), DB state (7, 9), the full pipeline (8)                                         |
| `packages/parser/tests/eventmodeling.test.ts`  | 31    | Parser AST (4), the registration of the validator (1), the validator (25), a multiple-source model (1) |

The diagram spec stubs `SVGElement.prototype.getBBox` to `{x:0, y:0, width:100, height:20}` for every test (`eventmodeling.spec.ts:8-23`). It has no SVG test and no rendering test: the renderer is not covered by the reference's own specs.

### 5.2 `eventmodeling.spec.ts` by test

| Line | Test name (keep it)                                                                | Level                                                                                     |
| ---- | ---------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| 30   | should handle a simple eventmodeling definition                                    | parser                                                                                    |
| 38   | should handle data inline in frames                                                | parser                                                                                    |
| 45   | should handle data block references                                                | parser                                                                                    |
| 57   | should handle qualified names                                                      | parser                                                                                    |
| 65   | should handle multiple source frames                                               | parser                                                                                    |
| 74   | should handle reset frames                                                         | parser                                                                                    |
| 82   | should reject a frame id reused by a reset frame                                   | parser (duplicate, message with line) and DB (`getState` throws `No data for EventModel`) |
| 95   | should report the original line number when comments precede a duplicate frame     | the whole pipeline: `mermaidAPI.parse`, comment lines counted                             |
| 104  | should keep explicit sources on a reset frame without inferring the previous frame | DB relations: `03 → 04` present, `04 → 05` absent                                         |
| 119  | should handle all entity types                                                     | parser                                                                                    |

The file calls `beforeEach` at line 26 without importing it from `vitest` (line 1 imports the other names), so it relies on the global test API. Before each test it calls `clear()`.

### 5.3 `packages/parser/tests/eventmodeling.test.ts` by group

- Four parse tests (lines 11, 74, 91, 111): a complex model with inline, block, note, gwt and every frame form; the minimal `timeframe` form; qualified names; a reset frame.
- Line 135: the validator is registered with the validation registry.
- Lines 174-310: 25 validator tests (`collectErrors` over `checkSourceFrameTypes`), which fix the allowed sources: `evt` from `cmd`; `event` from `command`; `cmd` from `ui`, `pcr` or `processor`; `ui` and `rmo` as listed in `event-modeling-validator.ts:24-46`; `pcr` from `rmo` and `processor` from `readmodel`. A frame with no sources is not validated (the last of the 25, line 308).
- Line 314: a multiple-source model, with `rf 03 readmodel … ->> 01 ->> 02 { a: true }`.

The test helper `eventModelingParse` is built with the GitGraph services object (`packages/parser/tests/test-util.ts:116-125`), which does not change the result.

## 6. Renderer

### 6.1 Pipeline

`draw` (`renderer.ts:128-163`) does this, in order:

1. Reads the configuration (`getConfig()`), the theme variables, and the id. The root element is found with `select('[id="${id}"]')` (`renderer.ts:136`).
2. `db.getDiagramProps()` and `db.getState()` (`renderer.ts:138-139`).
3. Appends one `g.em-swimlane` per lane, in index order (`renderer.ts:144-146`).
4. Appends one `g.em-box` per box, in frame order (`renderer.ts:147`).
5. Appends one `path.em-relation` per relation, in creation order (`renderer.ts:148`).
6. Appends `defs > marker` (`renderer.ts:150-160`).
7. Calls `setupGraphViewbox(undefined, diagram, padding, useMaxWidth)` (`renderer.ts:162`).

The surrounding SVG, outside `draw`, is set by the common code: the `svg` with `id`, `width="100%"` and `xmlns` and an empty `g` (`appendDivSvgG`, `mermaidAPI.ts:397-421`, with the `g` at line 419), then the style element inserted before the first child (`mermaidAPI.ts:596-600`), then `draw` (`mermaidAPI.ts:606-609`), then the accessibility attributes (`addA11yInfo`, `mermaidAPI.ts:750`), then the `xmlns` on the `foreignObject` children, then serialisation with `decodeEntities` (`mermaidAPI.ts:359`).

### 6.2 The DOM, element by element (probed)

The reference output for `state-view-pattern.mmd` (the fixture in section 7), in document order, with attributes in their written order:

```text
svg#id  width="100%"  xmlns  style="max-width: 728px;"  viewBox="-30 -30 728 470"
        role="graphics-document document"  aria-roledescription="eventmodeling"
  style                                  (the common style sheet, section 6.8)
  g                                      (empty; mermaidAPI.ts:419)
  g.em-swimlane                          (one per lane, index order)
    rect   x="0" y=lane.y rx="3" width=maxR+15 height=lane.height fill="rgb(250,250,250)" stroke="rgb(240,240,240)"
    text   font-weight="bold" x="30" y=lane.y+30   text: the label
  g.em-box                               (one per box, frame order)
    rect   x=box.x y=lane.y+15 rx="3" width=box.w height=box.h stroke=… fill=…
    foreignObject  x=box.x+10 y=lane.y+25 width=box.w-20 height=box.h-20
      div  xmlns="http://www.w3.org/1999/xhtml" style="display: table; height: 100%; width: 100%;"
        span style="display: table-cell; text-align: center; vertical-align: middle;"
          (the html of the box, section 6.4)
  path.em-relation  fill="none" stroke="#000" stroke-width="1" marker-end="url(#em-arrowhead-<id>)" d="M… L…"
  defs > marker  id="em-arrowhead-<id>" markerWidth="10" markerHeight="7" refX="10" refY="3.5" orient="auto"
    polygon  points="0 0, 10 3.5, 0 7" fill="#000000"
```

When the diagram has an accessibility title or description, the SVG has a `title` or `desc` element and the root gets `aria-labelledby="chart-title-<id>"` or `aria-describedby="chart-desc-<id>"` (probed, `accTitle_header`, `accDescr_header`; the position of the `title` element in the child order was not checked, unverified). The `stroke`/`fill` of a box is its entity colour (section 6.4). The `text` element of a lane has no `class` and no `style`.

### 6.3 Lanes (`renderer.ts:99-126`, `db.ts:457-536`)

- Lane `rect`: `x = 0`, `y = lane.y`, `rx = 3`, `width = maxR + 15`, `height = lane.height`, `fill = themeVariables.emSwimlaneBackgroundOdd ?? 'rgb(250,250,250)'`, `stroke = emSwimlaneBackgroundStroke ?? 'rgb(240,240,240)'` (`renderer.ts:108-118`). Every lane has the same width, the one of the widest box.
- Lane label: `text` with `font-weight="bold"`, `x = 30`, `y = lane.y + 30`, text = the label (`renderer.ts:120-124`). The label is not measured.
- Lane `height` = `max(70, maxBoxHeight) + 30`. Lane `y` = the sum of the heights of the lanes above, with a gap of 10 (section 3.5 step 7).

Probed: the `state-view-pattern` lanes are at y 0, 140 and 280 with height 130. The lane is 130 high for a box of 100 (`max(70, 100) + 30`). `block_empty` gives a lane of 146 for a box of 116.

### 6.4 Boxes (`renderer.ts:12-50`, `db.ts:497-509`)

- `rect`: `x = box.x`, `y = lane.y + 15`, `rx = 3`, `width = box.dimension.width`, `height = box.dimension.height`, `stroke = box.visual.stroke`, `fill = box.visual.fill` (`renderer.ts:21-28`).
- Entity colours (`db.ts:290-328`), all `??` fallbacks because the redux themes do not define the `em*` keys (section 6.9):

| Entity type        | fill      | stroke    |
| ------------------ | --------- | --------- |
| `ui`               | `white`   | `#dbdada` |
| `pcr`, `processor` | `#edb3f6` | `#b88cbf` |
| `rmo`, `readmodel` | `#d3f1a2` | `#a3b732` |
| `cmd`, `command`   | `#bcd6fe` | `#679ac3` |
| `evt`, `event`     | `#ffb778` | `#c19a0f` |

The `default` branch (red and black) cannot be reached: the grammar accepts only the nine types (section 2.3).

- `foreignObject`: `x = box.x + 10`, `y = lane.y + 25` (the rect's `y` plus 10; `renderer.ts:32-33`), `width = box.w - 20`, `height = box.h - 20`. The `foreignObject` is not in the geometry harness's measured set (section 7.2).
- `div` and `span` are as in section 6.2. The `span` gets `box.text`, set with `innerHTML` (`renderer.ts:48`).
- The `box.text` is `<b>NAME</b>`, then with data `<br/><br/><code style="text-align: left; display: block;max-width:430px">VALUE</code>` (section 6.4.1).
- This box text is HTML in the reference. Decision 0004 says labels are SVG text, and the journey port drew its `fo` labels that way (M6-PLAN, "Journey labels"). Whether the port follows that route for this type is open question 8 of section 8.3; section 6.4.2 describes what the SVG route would draw.

#### 6.4.1 The text of a box (`db.ts:330-397`)

1. `name` = `sanitizeText(extractName(entityIdentifier))` (`db.ts:336`).
2. `wrappedName` = `wrapLabel(name, 430, {fontSize: 16, fontWeight: 700, fontFamily: '"trebuchet ms", verdana, arial, sans-serif', joinWith: '<br/>'})` (`db.ts:339-346`).
3. `content` = `<b>${wrappedName}</b>` (`db.ts:347`).
4. Inline data (`db.ts:349-356`): take the text after the first `{`, cut at `lastIndexOf('}') - 1` (this removes the `}` and the character before it), sanitise, `wrapLabel` at 430, replace each space with `&nbsp;`. The value is `toHtml`. A quoted value (`"…"`, `'…'`) has no `{`, so the cut leaves nothing and the value is the empty string (probed, `inline_quoted`: an empty `<code>`).
5. Data reference (`db.ts:358-372`): look the data entity up by name. If found: the text after `indexOf('{\n') + 2`, cut at `lastIndexOf('}') - 1`, sanitise, `wrapLabel`, spaces to `&nbsp;`, then `+= '<br/>'`. If the entity is missing, nothing is added (probed, `block_missing`).
6. `hasRenderedData` = `toHtml !== undefined` (`db.ts:374`). When true, `content += '<br/><br/><code style="text-align: left; display: block;max-width:430px">' + toHtml + '</code>'` (`db.ts:376-378`).
7. Measure `content` (section 6.7). The width is `dimensions.width / 3` when there is data, else `dimensions.width` (`db.ts:385-388`). The `/3` is not rounded (probed: `inline_normal` gives a box width of 265.33…).
8. `props` = `{ content, width, height }` (`db.ts:390-394`). The box dimension is then `width + 20` (`db.ts:404`) and `height + 20` (`db.ts:405`), and the clamps of section 3.5 step 5 apply.

The `html` that the reference writes is `content`, unchanged (probed). Examples:

- `<b>A</b>`, for a plain frame named `A`;
- `<b>A</b><br><br><code style="text-align: left; display: block;max-width:430px">productId:&nbsp;7</code>`, for `{ productId: 7 }`;
- `<b>A</b><br><br><code style="text-align: left; display: block;max-width:430px"><br></code>`, for `data D {` newline `}` (the closing `<br/>` of the reference branch makes the empty value a line);
- a name longer than 430 px breaks inside the word with a hyphen: `<b>Supercalifragilisticexpialidocious_and_then_some_more_-<br>letters_here_to_exceed</b>` (probed, `name_long_one_word`).

The serialised `<br>` is the browser's form of the string `<br/>`; `cleanUpSvgCode` turns `<br>` into `<br/>` (`mermaidAPI.ts:362`), so the port writes `<br/>`.

#### 6.4.2 What the browser lays out (for the SVG route)

Under decision 0004, the port draws the box text as SVG text in the lines the browser breaks the HTML into, as `journey` does (`tools/port/geometry/journey-labels.ts` describes the method). For the reference's box text these lines are:

- the `<br/>`s of the content split it into lines: the name, an empty line, then the data lines. The measurement counts the same lines (`block_long_line`: six lines, 6 × 19 = 114, the sixth being the line that holds only the closing tag);
- inside the `code` element, a break is possible only where the text has a normal space or a newline. The spaces of a data value are `&nbsp;` and do not break. A newline is different: the reference writes the data block's own newline into the HTML (probed: `block_simple` gives `productId:&nbsp;7,` then a literal newline, then `&nbsp;quantity:&nbsp;1`), and a newline is a break opportunity for the browser. So a data block with several lines can show one more line break than the measurement counts (the measure splits only on `<br>`; `block_simple` is measured as four lines, 76 px). Unverified in a render;
- the name is one word, so it does not break; a name longer than 430 px is broken by `wrapLabel` with a hyphen, which is an explicit `<br>` (section 6.4.1);
- the drawn font is the root font (14 px in the redux theme, section 6.8), not the 16 px bold of the measurement (section 6.7).

### 6.5 Relations (`renderer.ts:52-97`)

- The source and target lane tops: `sourceBoxY = source.swimlane.y + 15`, `targetBoxY = target.swimlane.y + 15` (`renderer.ts:63-64`). The lane's current `y`, not the box's stale `y`.
- `upwards = sourceBoxY > targetBoxY` (strict; `renderer.ts:52-54`, `66`). A relation in one lane is not upwards.
- `sourceX = source.x + source.w * 2 / 3`; `targetX = target.x + target.w / 3` (`renderer.ts:68-69`).
- Upwards: `sourceY = sourceBoxY`, `targetY = targetBoxY + target.h`. Otherwise: `sourceY = sourceBoxY + source.h`, `targetY = targetBoxY` (`renderer.ts:78-84`).
- `d = "M" + sourceX + " " + sourceY + " L" + targetX + " " + targetY` (`renderer.ts:95`). The numbers are JavaScript numbers (shortest round-trip form), for example `M373.3333333333333 115 L420 155` (probed).
- Probed: `order_lanes_out_of_sequence` has a relation from the events lane up to the UI lane, and it starts at the top of the source box.

### 6.6 The root (`setupGraphViewbox.js:49-72`, `renderer.ts:162`)

- `svgBounds = svgElem.node().getBBox()`: the union of the children's boxes, in user units. The children are the lane rects (x 0, width `maxR + 15`, y from 0 to the bottom of the last lane), the lane labels, the boxes and the relations. Markers are not in it.
- `width = bbox.width + 2 * padding`, `height = bbox.height + 2 * padding`, with `padding = config.padding ?? 30` (`renderer.ts:162`; default 30, `schemas/config.schema.yaml`, `EventModelingDiagramConfig`).
- `useMaxWidth` (default true): `width="100%"` and `style="max-width: <width>px;"`. With `false`: `height` and `width` as numbers and no style (probed, `useMaxWidth_false`).
- `viewBox = "<bbox.x - padding> <bbox.y - padding> <bbox.width + 2 * padding> <bbox.height + 2 * padding>"`.
- Float32: `getBBox` returns float32 values. The lane `rect` width is a double (`540.3333333333334`), but the root's viewBox and `max-width` use the float32 value plus 60: `600.3333129882812` (probed, `inline_normal`). The port must round the bounding box to float32 before adding the padding.
- Empty diagram: bounding box zero, so `viewBox="-30 -30 60 60"`, `max-width: 60px` (probed).
- The padding changes the viewBox origin, for example `-50 -50` with `padding: 50` (probed, `padding50`).

### 6.7 Text measurement

Every width and height that changes a box comes from `calculateTextDimensions` (`utils.ts:665-735`), applied to the `content` of section 6.4.1. Its rules are:

1. The text is split on `<br>`-like tags (`diagrams/common/common.ts:8`, `utils.ts:681`). Each line is measured as the literal string: the markup is text, so the `<b>` and `</b>` and the whole `<code style="…">` start tag are measured as characters (`utils.ts:686-706`, `drawSimpleText` puts the line in a `tspan` by `textContent`, `utils.ts:491-531`). An empty line becomes U+200B (`utils.ts:698`).
2. For each line, an SVG `text` element is created in a temporary `svg` in `body`, with `font-size: 16px`, `font-weight: 700` and the family of the pass. Its `getBBox()` gives the width and the height (`utils.ts:700-707`).
3. Per line: `width = round(bbox.width)`, `height = round(bbox.height)`. Over the lines: `width` = the largest rounded width; `height` = the sum of the rounded heights; `lineHeight` = the largest rounded height (`utils.ts:707-712`).
4. Two passes, one for `sans-serif` and one for the family `"trebuchet ms", verdana, arial, sans-serif`. The result is the second pass unless the first is strictly larger in all three values, in which case it is the first (`utils.ts:720-729`). A `NaN` in the second pass selects the first.
5. The returned width and height are integers (rounding in step 3). The data width is then divided by 3 without rounding (`db.ts:388`).

The family list is set in code (`db.ts:342`, `db.ts:383`), not from the configuration. So the harness's `fontFamily: "Lexica Ultralegible"` (`tools/port/geometry/common.ts:59-63`) does not reach these measurements. The text that is drawn does use the configured family, through the root `font-family` (section 6.8), so the boxes are sized for one font and drawn in another.

Two decisions apply to this measurement, and they point in different directions. M6-PLAN's text measurement row says a renderer "measures text where and how the reference does", and "the layout must see the reference's numbers". Decision 0005 (`docs-port/decisions/0005-text-measurement-and-fonts.md`) says text is measured with bundled fonts only, because "measuring needs the exact font the viewer will see", and it names Trebuchet MS as one of "Mermaid's defaults … system fonts we may not ship". This type measures in a font that it does not draw with, so the two cannot both hold without a decision (section 8.3, question 1).

Measured on the reference Mac, Chromium, 16 px bold (probed, `measure-lines.ts`, `fonts.json`):

| Text (literal)                                                                            | `"trebuchet ms", …` width | `sans-serif` width | Note                                      |
| ----------------------------------------------------------------------------------------- | ------------------------- | ------------------ | ----------------------------------------- |
| `<b>A</b>`                                                                                | 72.4844 (rounded 72)      | 72.9609            | box of `A` is 112 wide: 72 + 20 + 20      |
| `<b>CartScreen</b>`                                                                       | 144.8906 (145)            | 146.7812           | `state-view-pattern`: the box is 185 wide |
| `<code style="text-align: left; display: block;max-width:430px">productId:&nbsp;7</code>` | 676.2969 (676)            | 681.5703           | `inline_normal`: 676 / 3 + 40 = 265.33    |
| `<b>Supercalifragilisticexpialidocious_and_then_some_more_-</b>`                          | 499.8594 (500)            | 508.6719           |                                           |
| U+200B                                                                                    | 0                         | 0                  | an empty line still has its height        |

Heights, which do not depend on the text: 19 px for the Trebuchet pass and 18 px for `sans-serif` (probed, the same run). So on this machine the `sans-serif` pass is never selected: its height is smaller in every case, and the rule needs all three values to be larger. The result is always the Trebuchet pass, with a line height of 19.

The reference's own numbers confirm the rule: `state-view-pattern` (box `CartScreen` 185 wide and 100 high) is the Trebuchet width 145 plus 40, and `block_long_line` has six lines of 19 px (114) plus 40 for a box height of 154.

The width the port needs is therefore the advance width of Trebuchet MS Bold at 16 px for each literal line, rounded to the nearest integer, and the line height 19 px. The shaping and the rounding rules of Chromium are the M5 rules (`ferrigura-text`); they still have to be checked against this font. Section 8.1 covers the options.

Other text rules that feed the layout:

- `wrapLabel` (`utils.ts:539-583`): splits on single spaces with empty words dropped (`label.split(' ').filter(Boolean)`), measures `word + ' '` against the line budget, and breaks a word that is too wide character by character (`breakString`, `utils.ts:585-615`, which appends `-` to each piece but the last). Returns the input unchanged when it already has a `<br>` (`diagrams/common/common.ts:102-106`; probed, `data_explicit_br`: no wrapping, and the spaces stay as `&nbsp;`).
- `calculateTextWidth` and `calculateTextHeight` are `calculateTextDimensions(...).width` and `.height` (`utils.ts:649-660`).
- The font size as a style: `parseFontSize` (`utils.ts:823-826`) turns the number 16 into `[16, '16px']`.
- The memoisation of the three functions does not change a result.

### 6.8 Styles

- `styles.js` returns the empty string (`styles.js:1`). The diagram adds no rules.
- The common style sheet is the same as for every diagram: `#id` with `font-family: <fontFamily>; font-size: <fontSize>; fill: <textColor>` (the header, `styles.ts:36-46`), the keyframes, the edge and marker rules and the `:root` variable. The reference output for the harness has `#id{font-family:"Lexica Ultralegible";font-size:14px;fill:#28253D;}` (probed). Ferrigura's shared code in `crates/ferrigura-render/src/common/style.rs` is the place for this (unverified that it emits every rule of this output).
- Decision 0007 (`docs-port/decisions/0007-themes-as-css-variables.md`, and the Styles row of `docs-port/M6-RENDERERS.md`) asks for every colour as a custom property on the diagram's id, with the concrete colour also as a presentation attribute. The reference writes this type's colours as attributes only. The port adds the custom properties; the structure comparison does not see them.
- The box text is drawn with the root font (14 px in the redux theme, `theme-redux-color.js:27`), because the `span` inherits it. The measurement uses 16 px bold (section 6.7), so the text is drawn smaller than the box was sized for. The port reproduces this, because the sizes are the reference's.

### 6.9 Theme variables and the configuration

Theme variables read by this type: `emUiFill`, `emUiStroke`, `emProcessorFill`, `emProcessorStroke`, `emReadModelFill`, `emReadModelStroke`, `emCommandFill`, `emCommandStroke`, `emEventFill`, `emEventStroke`, `emSwimlaneBackgroundOdd`, `emSwimlaneBackgroundStroke`, `emArrowhead`, `emRelationStroke` (`db.ts:290-328`, `renderer.ts:86,108-109,142`). Neither redux theme defines any of them (`theme-redux-color.js` and `theme-redux-dark-color.js` contain no `em` key, source grep). So every colour is the fallback in the code. Probed: the dark theme (`redux-dark-color`, `dark_theme`) draws the same colours as the light one.

Configuration (`EventModelingDiagramConfig`, `config.type.ts:2172-2185`; schema `config.schema.yaml:2726-2741`; defaults `defaultConfig.ts:282-284`):

| Key                                                        | Default        | Used               | Note                                                                 |
| ---------------------------------------------------------- | -------------- | ------------------ | -------------------------------------------------------------------- |
| `padding`                                                  | 30             | `renderer.ts:162`  | Probed: 50 gives `-50 -50`                                           |
| `rowHeight`                                                | 32             | nowhere            | Not read by any code of this type (grep). Probed: 40 changes nothing |
| `useMaxWidth` (base)                                       | true           | `renderer.ts:162`  | Probed: false gives `width="447"` and no `max-width`                 |
| `theme`, `themeVariables`, `fontFamily`, `fontSize` (base) | from the theme | `styles`, root CSS |                                                                      |

`htmlLabels` has no effect on this type. Boxes are always `foreignObject` (`renderer.ts:30-48`). Its effect on the `sanitizeText` path is covered in section 3.3.

### 6.10 Randomness and the clock

None. The folder has no `Math.random`, `Date` or `new Date`, and the ids come from the render id only (`renderer.ts:141`, `mermaidAPI.ts` a11y ids). Two renders of the same text are equal (probed in the harness with the same settings).

### 6.11 DOM dependencies a server renderer must replace

| Dependency                                              | Where                             | What the port does                                                      |
| ------------------------------------------------------- | --------------------------------- | ----------------------------------------------------------------------- |
| `select('[id="…"]')` and d3 append/attr/style/text/html | `renderer.ts:136-162`             | Writes the elements in order (section 6.2)                              |
| `getBBox` on the root                                   | `setupGraphViewbox.js:50`         | The union of the children, rounded to float32 (section 6.6)             |
| `getBBox` on text, in a temporary SVG                   | `utils.ts:665-735`                | The measurement of section 6.7 (font data needed)                       |
| `DOMPurify`                                             | `diagrams/common/common.ts:71-83` | A string-level port for the texts that can contain markup (section 3.3) |
| `innerHTML` of the box text                             | `renderer.ts:48`                  | The string itself, as the browser then reads it                         |
| `xmlns` on `foreignObject` children                     | `mermaidAPI.ts` (`serializeSvg`)  | Written as an attribute on the `div`                                    |

## 7. Fixtures

### 7.1 The six fixtures

All six are in `e2e/diagrams/eventmodeling/pr-check/`. Each has a front matter `description` (folded YAML) and the `eventmodeling` header. None has a theme, a look or a layout. Probed: all six render; the numbers come from the reference's SVG.

| Fixture                              | Frames | Features                                                                                                    | Lanes | Boxes | Relations | viewBox (w × h) |
| ------------------------------------ | ------ | ----------------------------------------------------------------------------------------------------------- | ----- | ----- | --------- | --------------- |
| `state-change-pattern.mmd`           | 5      | UI, two command/event pairs, inferred relations                                                             | 3     | 5     | 4         | 989 × 470       |
| `state-view-pattern.mmd`             | 3      | UI, command, event                                                                                          | 3     | 3     | 2         | 728 × 470       |
| `translation-pattern.mmd`            | 5      | a read model with `->> 03` (explicit source)                                                                | 3     | 5     | 4         | 1028 × 470      |
| `with-data-block-reference.mmd`      | 3      | inline data (`{ productId: 7 }`), a data block reference (`[[ItemAddedData]]`) with two lines               | 3     | 3     | 2         | 955 × 486       |
| `with-multiple-source-relations.mmd` | 4      | one event with two sources (`->> 02 ->> 03`)                                                                | 3     | 4     | 4         | 951 × 470       |
| `with-qualified-names.mmd`           | 3      | `Inventory.AddItem` and `Inventory.ItemAdded`: two namespace lanes (`C/RM: Inventory`, `Stream: Inventory`) | 3     | 3     | 2         | 728 × 470       |

In `with-data-block-reference`, the `ItemAdded` box has the data block `productId: 7,` and `quantity: 1`, which the reference writes as `productId:&nbsp;7,` and a literal newline, then `&nbsp;quantity:&nbsp;1` (probed). The `AddItem` box has the inline `{ productId: 7 }`. The taller box makes the viewBox 486 high.

### 7.2 What the generic harness has to handle

- **Not in the harness yet.** `TYPE_FOLDERS` in `tools/port/geometry/diagram-geometry.ts:97-108` has no `eventmodeling` entry. The entry is `eventmodeling: ['eventmodeling']`. The configuration differential (`tools/port/diff-config.ts:231`) already has the type.
- **The forced settings do nothing here.** `FORCED_SETTINGS` (`common.ts:59-63`) sets `htmlLabels: false`, which this type ignores (section 6.9). The `fontFamily` is not used by the measurement (section 6.7).
- **Text in a font the bundle does not have.** The measurement uses Trebuchet MS (section 6.7), which is not in `crates/ferrigura-text/fonts/`. The M6 rule (`uncoveredScriptReason` and `exclusionReason`, `common.ts:176-224`) excludes text the bundled faces do not cover, by character. It does not look at the font the reference measured with. Under the present rule none of the six fixtures would be excluded, and all six would be compared with wrong sizes. This is open question 1 of section 8.3, not a bug of the harness.
- **`foreignObject` is not measured.** The harness measures the elements in `BOXED` (`rect`, `circle`, `ellipse`, `line`, `polygon`, `polyline`, `image`, `text`, `diagram-geometry.ts:120`, `measureSource`). A `foreignObject` has no box in the comparison, and its `div`, `span`, `b` and `code` elements are compared only by tag and class. Their sizes are not compared. The box `rect`s carry the same numbers, so a size error shows there.
- **Box text needs a label mode if the SVG route is taken.** Under decision 0004 the port's box text is `text` elements, and the reference's is `foreignObject` HTML, so the element structure cannot pair them. Journey solved this with a label mode (`tools/port/geometry/journey-labels.ts`, `journey` in `diagram-geometry.ts`): each label is left out of the element comparison and its lines (text and box, from the browser's character ranges) are compared instead. The same mode would serve this type, with the `foreignObject` as the label.
- **No randomness, no clock.** `--self` should find no differences between two renders, and no fixture needs a seed.
- **No handDrawn, no non-layered layout, no CJK.** `exclusionReason` (`common.ts:197-214`) has nothing to exclude for these fixtures.

## 8. Risks, effort, open questions

### 8.1 Risks, most serious first

1. **The font.** Every box width and height in every fixture comes from Trebuchet MS Bold, measured on the reference machine. Ferrigura does not have the font, and the M6 rule would not exclude these fixtures. Decision 0005 rules out shipping it. The options are: (a) a metrics table of Trebuchet MS Bold (advance widths and the 19 px line height for the characters the fixtures and the docs use), which is metric data rather than the font, and needs a licence decision; (b) measure with the bundled Lexica face, as 0005 asks: this is the font the port draws with, but every box size changes, so the port breaks the compatibility promise (M6-PLAN, "the same text renders the same diagram") and the fixtures are no longer the reference's; (c) exclude the type from the geometry check, with a named reason, until a decision is made. The reference is also machine-dependent: on Linux, `"trebuchet ms"` falls back to another face and the numbers change, so the machine that defines the reference must be named.
2. **The literal-markup measurement.** The width and height of a box are measured on the whole literal string, `<b>`, `<code style="…">`, `&nbsp;`, the `ﬂ°` placeholders and the newlines of a data block included. A port that measures the text without its markup gets every width wrong. The rounding rules (per-line integers, the unrounded `/3`) and the float32 root box need the same care.
3. **The lexer.** Ferrigura's shared lexer (`crates/ferrigura-syntax/src/lexer.rs`, logos) is a longest-match lexer that treats keywords as words. It does not reproduce the Chevrotain order of section 2.2: `title` at a line start, `title` after an identifier on the same line, and the `EM_WS` against the titles. Decision 0002 gives the route: a scanner over the rule table in order (`scan.rs`), and that is what this type needs, or the parser will accept and reject different inputs.
4. **The box text, and decision 0004.** The reference draws the box text as HTML in `foreignObject`. Decision 0004 draws labels as SVG text, and the journey port applied it to `fo` labels. Under the SVG route the port must draw the lines the browser breaks the HTML into (section 6.4.2), and the strings of section 6.4.1 are only the source of those lines. The sanitiser parity for the data text matters only for input with `<`, `&` or markup, and the `&nbsp;` joins decide which tags survive (section 3.3). Unverified: whether the flowchart's `sanitize.rs` is enough.
5. **Namespace quirk and dead titles.** Both are reproducible and tested here (section 3.7, section 2.2). They are quirks, not bugs of the port, and they must be listed (the M6 rule).
6. **Geometry harness gaps.** `foreignObject` sizes are not compared, and box text needs a label mode under the SVG route (section 7.2). The type is not in the harness's map yet.

### 8.2 Effort, relative to the flowchart

The reference TypeScript is 1,098 lines without the spec. The grammar folder adds 206 lines (grammar, validator, module, token builder, index). The journey port, the M6 type of the same size class, has 1,151 TypeScript lines without its specs, and its notes put it at about a tenth of the flowchart's Rust. Estimate for this port (unverified, rough): a parser with the scanner and the grammar of section 2.3, about 700 to 900 Rust lines; the DB and the model, 400 to 600; the measurement and wrapping, 150 to 250 plus the font data; the renderer, 250 to 350 (more with the label route of section 6.4.2); the specs ported (10 diagram tests, 31 parser tests), 800 or more. The total is about the size of the journey port.

### 8.3 Open questions for the coordinator

1. **Font decision (blocking).** M6-PLAN wants the reference's numbers; decision 0005 wants bundled fonts only. Which of options (a), (b), (c) of section 8.1 resolves this, and which machine defines the reference for this type? (The choice also decides whether the Trebuchet metric table is acceptable under the licence check.)
2. **Differential design.** The Node exporter needs text measurement. Option (a) or (b) of section 4.1? The spec's 100 × 20 stub is not enough for the wrapping and the `html`.
3. **Parser errors.** Decision 0002 already says error messages are the port's own and specs assert failure and position. Proposed: the port reports the failure and the first error position only, and does not copy Chevrotain's expectation texts. Confirm.
4. **Validator.** Port it as a unit with its 25 tests, without wiring it into parse or render (proposed), since the reference never runs it?
5. **Quirks.** Reproduce all of them, as the M6 rule says: the namespace lanes, the raw titles, the `&nbsp;` joins, the cut of the last character in inline data, the empty value of a quoted inline value, the `}`-at-column-0 rule. (Proposed: yes; list each one in `PLAN.md`.)
6. **Sanitiser.** Reuse `crates/ferrigura-syntax/src/sanitize.rs` for the data text, or write a subset for this type?
7. **Harness.** Add `eventmodeling` to `TYPE_FOLDERS`, and decide whether `foreignObject` sizes should join the measured set (`BOXED`, `tools/port/geometry/diagram-geometry.ts:120`).
8. **Box text route (decision 0004).** Draw the box text as SVG text in the browser's lines, with a label mode in the harness as journey has (proposed, and it is what 0004 asks for), or keep the reference's `foreignObject` HTML as an exception to 0004? The first needs the line emulation of section 6.4.2.

### 8.4 Unverified items

- The `getDirection` consumers (section 3.1).
- Whether a `[[ref]]` and an inline value can both apply to the same frame, and the order in which the reference branch replaces the inline value (`db.ts:349-372`), beyond the parse probe `block_then_inline_order`.
- The order of the relations for sources written in a different order from the AST (section 3.6). The filter is in AST order (`db.ts:109`), so the written order is not used.
- The effect of a trailing space after `data D {` on the reference's cut (`db.ts:365`, where `indexOf('{\n')` returns -1 and the first character is lost). The parse is probed; the render is not.
- Front matter is removed without counting its lines (probed), but a front matter that contains a directive has not been probed.
- The DOMPurify subset needed for the data text (section 3.3).
- Whether Chromium's width for a literal line of 1,000 px or more matches `ferrigura-text`'s measure. Not needed for the fixtures.
- The reference machine for the geometry check (section 8.1).

## Appendix A. Probe material (outside the repository)

Everything is in the session scratchpad: `/private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/em-probe/`. Run from the worktree root with `node_modules/.bin/tsx <script>`.

- `render-fixtures.ts <outdir>`: renders the six fixtures through `withPage`, `renderReference` and `patchText` from `tools/port/geometry/common.ts`, and measures the font families (`fonts.json`).
- `render-cases.ts <cases.json> <outdir>`: the same for any list of inputs, writing one SVG per input and an `index.json` with ok/error. The case files (`cases1.json` to `cases5.json`) hold the inputs quoted in this note. `cases2.json` holds the docs blocks and the spec inputs (extracted from the docs and the spec with a regex).
- `analyze.py <svg…>`: reads lanes, boxes, relations, the root and the marker from an SVG.
- `parse-probe.ts`, `parse-probe2.ts`: the Langium parser through `packages/parser/src/parse.ts` on the inputs of section 2.4 (AST summary or error).
- `tokens-probe.mts`: the lexer's token list, in order, from `createEventModelingServices` (section 2.2).
- `measure-lines.ts`: the literal content lines of section 6.7 measured with `getBBox` in both font lists.
- `purify-probe.cjs`: DOMPurify under jsdom, as a cross-check of the sanitiser (section 3.3; it keeps `<a>` and `<span style>`, which the reference drops, because of the `&nbsp;` join).

Reading the reference's behaviour in Chromium needs the same bundle the harness uses (`packages/mermaid/dist/mermaid.esm.mjs`). The bundle was not rebuilt for this research; it is the one in the checkout.
