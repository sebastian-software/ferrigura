# M6 timeline parser: plan

Status: parser, DB and model plan, written before the code. The renderer is out of this plan (section 9). The research behind it is [NOTES.md](./NOTES.md); every rule here cites the reference tree of `packages/mermaid` 12.2.0 in this worktree. Line numbers are those of the files named in each row.

## 1. Scope and sources

Parser, DB, canonical model, type detection, the 11 DB-level specs of `diagrams/timeline/timeline.spec.js`, and the differential run over every timeline input.

| Concern           | Reference file (`packages/mermaid/src/…`)                              | What matters                                           |
| ----------------- | ---------------------------------------------------------------------- | ------------------------------------------------------ |
| Lexer and grammar | `diagrams/timeline/parser/timeline.jison`                              | Rules 6-41, grammar 43-86                              |
| DB                | `diagrams/timeline/timelineDb.js`                                      | Whole file (114 lines)                                 |
| Detection         | `diagrams/timeline/detector.ts`                                        | `/^\s*timeline/`, case-sensitive                       |
| Header texts      | `diagrams/common/commonDb.ts` 16-30, `diagrams/common/common.ts` 56-83 | Sanitising and the two whitespace rules of the setters |
| Pipeline          | `preprocess.ts` 6-91, `Diagram.ts` 19-85, `utils.ts` 874-906           | Steps before the lexer (section 2)                     |
| Specs             | `diagrams/timeline/timeline.spec.js`                                   | 11 tests, all DB level                                 |

Not in this plan: the two renderers, the styles, the `timeline` configuration keys, and the specs outside the folder that touch the timeline (`multi-diagram-id-uniqueness.spec.ts`, `paletteCssGeneration.spec.ts`, `colorThemeGate.spec.ts`, `styles.spec.ts`, `diagram-orchestration.spec.ts`, `diagram-beta-policy.spec.ts`). The first four and the style spec are rendering or CSS; the last two check the detector and the beta policy list, which the registration step of section 2 covers.

## 2. Pipeline before the lexer

The steps and their order are those of the flowchart port (`flowchart/mod.rs`, `parse_with_warnings`), which the timeline shares:

1. CRLF and CR become LF, and HTML attribute quotes are rewritten (`prepare`, shared).
2. The front matter is removed. Its `title` is read and then dropped, because the timeline DB has no `setDiagramTitle` (`Diagram.ts` 52-54 calls it with `?.`). Its `config` is not modelled (section 8).
3. Directives are removed (`removeDirectives`, shared with the flowchart).
4. Comment lines are removed and the leading blanks trimmed (`cleanupComments`, shared).
5. Detection runs on this text. The order is pie, info, flowchart, timeline, as the reference registers them. A capitalised `Timeline` is not detected (Q1).
6. `encodeEntities` runs, and a line break is appended (`Diagram.ts` 77). The parser sees this text, and the model keeps its entities.

Steps 3, 4 and 6 are the flowchart's `remove_directives`, `cleanup_comments` and `encode_entities`. They become `pub(crate)` in `flowchart/mod.rs` for this port; moving them to a shared module is an open question (section 10).

## 3. Lexer

Jison takes the first rule in file order that matches at the current position, not the longest. The rules are hand-written in `timeline/scanner.rs` on the helpers of `scan.rs`, as decision [0002](../decisions/0002-hand-written-parsers.md) says. Keywords are case-insensitive, as `%options case-insensitive` makes them. The rule numbers are those of the generated lexer (NOTES 2.2).

| #   | Rule (source line)                                            | Token and effect                                                                          |
| --- | ------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| 0   | `%` not followed by `{`, to the end of the line (14)          | skipped                                                                                   |
| 1   | one character other than `}`, then `%%`, to the line end (15) | skipped, including the character before `%%` and a line break in that position            |
| 2   | `\n+` (16)                                                    | NEWLINE                                                                                   |
| 3   | `\s+` (17)                                                    | skipped, and it takes the line breaks after blanks                                        |
| 4   | `#` to the end of the line (18)                               | skipped                                                                                   |
| 5   | `timeline[ \t]+LR\b` (20)                                     | `timeline_lr`                                                                             |
| 6   | `timeline[ \t]+TD\b` (21)                                     | `timeline_td`                                                                             |
| 7   | `timeline\b` (22)                                             | `timeline`                                                                                |
| 8   | `title\s[^\n]+` (23)                                          | `title`; the value starts at character 6. `\s` may be a line break                        |
| 9   | `accTitle\s*:\s*` (24)                                        | `acc_title`, enters the `acc_title` state. The `\s*` may take the following lines         |
| 10  | in `acc_title`: `[^\n]*` (25)                                 | `acc_title_value`, leaves the state                                                       |
| 11  | `accDescr\s*:\s*` (26)                                        | `acc_descr`                                                                               |
| 12  | in `acc_descr`: `[^\n]*` (27)                                 | `acc_descr_value`, leaves the state                                                       |
| 13  | `accDescr\s*{\s*` (28)                                        | enters `acc_descr_multiline`, no token                                                    |
| 14  | in the block: `}` (29)                                        | leaves the state, no token                                                                |
| 15  | in the block: `[^}]*` (30)                                    | `acc_descr_multiline_value`; may be empty at the end of the input                         |
| 16  | `section\s[^:\n]+` (31)                                       | `section`; the name starts at character 8                                                 |
| 17  | `:\s(?:[^:\n]\|:(?!\s))+` (34)                                | `event`; the text starts at character 2. The `\s` may be a line break                     |
| 18  | `[^#:\n]+` (35)                                               | `period`; keeps trailing blanks                                                           |
| 19  | end of input, outside the value states (38)                   | `EOF`                                                                                     |
| 20  | any other character (39)                                      | `INVALID`, which the grammar rejects. In practice only a `:` that does not start an event |

Two facts of the generated lexer matter for the port (NOTES 2.2, 2.6):

- At the end of the input, a value state (10, 12, 15) matches the empty string once, and the lexer then answers the end of input rather than `EOF`. An `accTitle:` or `accDescr:` at the end, or an unclosed `accDescr {`, is therefore a parse error (Q10, Q14).
- The text always ends with a line break, because `Diagram.ts` appends one, so a value state at the end is reached only when the rule before it consumed the last line break.

## 4. Grammar and DB

The grammar is `start: timeline_header document EOF`. A line is a statement or a NEWLINE, and statements need no line break between them (Q18). The actions run in input order and call the DB as follows (NOTES 2.3):

| Production                  | Action                                    | DB effect                                                                         |
| --------------------------- | ----------------------------------------- | --------------------------------------------------------------------------------- |
| header `timeline_lr`        | `setDirection('LR')`                      | direction `LR`                                                                    |
| header `timeline_td`        | `setDirection('TD')`                      | direction `TD`                                                                    |
| header `timeline`           | none                                      | direction stays as `clear` left it, `LR`                                          |
| `title`                     | `setDiagramTitle(value from character 6)` | sanitised, no trim, replaces the previous title, even when blank (Q3, Q4)         |
| `acc_title acc_title_value` | `setAccTitle(value.trim())`               | sanitised, then leading whitespace removed                                        |
| `acc_descr acc_descr_value` | `setAccDescription(value.trim())`         | sanitised, then `\n\s+` becomes `\n`                                              |
| `acc_descr_multiline_value` | `setAccDescription(value.trim())`         | as above                                                                          |
| `section`                   | `addSection(value from character 8)`      | the name is the current section and is appended, untrimmed, duplicates kept (Q16) |
| `period`                    | `addTask(period, 0, '')`                  | a period of the current section, untrimmed, with no events                        |
| `event`                     | `addEvent(value from character 2)`        | appended to the last period; with no period it is an error (Q17)                  |
| `NEWLINE`                   | none                                      | none                                                                              |

Other grammar facts: a header token anywhere after the first token is a parse error, and so is an `INVALID` token. The grammar's `line: SPACE statement` is unreachable, because no rule returns `SPACE` (NOTES 2.2), and it is not ported.

## 5. Model

`TimelineDiagram` in `crates/ferrigura-model/src/timeline.rs`, version 1, camelCase JSON:

- `direction`: `"LR"` or `"TD"`;
- `title`, `accTitle`, `accDescr`: the DB texts, sanitised, empty when absent;
- `sections`: the names in creation order, duplicates kept, untrimmed, encoded;
- `periods`: `{ section, period, events }` in creation order. `section` is the current section at creation, `""` before the first one. `period` and `events` are untrimmed and encoded.

The DB's `tasks` are named `periods` here, as the brief and the grammar name them. Left out: `id` (a global counter that `clear` does not reset, R19), `type` (always the section), `score` (always 0), the `processed` flag and the duplicates of `getTasks()` (NOTES 3.4). The header texts follow the timeline's own rules in `canonical-model.md`, which differ from the pie's (no blank collapsing).

## 6. Quirks reproduced

Each quirk is kept on purpose, because the reference defines the behaviour. The "Check" column names the case that pins it: a Rust test in `timeline_grammar.rs` or a case in `tools/port/cases.ts`, which the differential run compares with the reference.

| Id  | Quirk                                                                                                                                               | Reference                                  | Check                                       |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------ | ------------------------------------------- |
| Q1  | `Timeline` (capital) is not detected: the detector is case-sensitive, and the text is then rejected                                                 | `detector.ts` 9-11, `detectType` 36-51     | cases: `Timeline`, `timelineX`              |
| Q2  | The front matter title is ignored: the timeline DB has no title setter                                                                              | `Diagram.ts` 52-54                         | `timeline_grammar`: front matter title      |
| Q3  | The last `title` line wins, and a blank one counts: `title  ` (two blanks) gives the title ` ` (one blank)                                          | grammar 72, `setDiagramTitle`              | `timeline_grammar`: blank last title        |
| Q4  | Title, section, period and event texts are not trimmed: `section A ` is `A `, and a title keeps its leading and trailing blanks                     | grammar 72, 76, 80-85, `timelineDb` 52-63  | `timeline_grammar`: blanks kept             |
| Q5  | The `title` and `section` keywords need one blank character after them: `title ` with nothing after it is a period                                  | rules 8 and 16 (23, 31)                    | `timeline_grammar`: `title ` period         |
| Q6  | Keywords have no word boundary except `timeline`: `timelineX` is a period and fails (no header); `timeline TDx` is a header and a period            | rules 5-7 (20-22)                          | `timeline_grammar`: `timeline TDx`          |
| Q7  | An event needs a colon followed by a blank. `2004 : a:b` is the event `a:b`; a colon elsewhere is `INVALID`                                         | rules 17, 20 (34, 39)                      | `timeline_grammar`: `a:b`, `2004:x`         |
| Q8  | An event attaches to the last period created, wherever it is: a section line between the two does not change that                                   | `timelineDb` 65-70                         | `timeline_grammar`: multi-line events       |
| Q9  | `accTitle:` takes the next line as its value, and the indentation of that line is removed with the line break                                       | rule 9 (24)                                | `timeline_grammar`: value on next line      |
| Q10 | `accTitle:` or `accDescr:` as the last line is a parse error (the value state is at the end of the input)                                           | rules 10, 12 (25, 27) and 2.6 of the notes | `timeline_grammar`                          |
| Q11 | `#` starts a comment at the start of a token, and ends a period. In titles, sections and events it is text                                          | rules 4, 18, 16, 17 (18, 35, 31, 34)       | `timeline_grammar`: `title #x#`, `2004 # x` |
| Q12 | `%%` after exactly one character takes that character with it; after two or more characters the period keeps the `%%`                               | rule 1 (15), rule 18                       | `timeline_grammar`: `x%%c`, `abc%%c`        |
| Q13 | The `accDescr { … }` block has no comment rules: `#` and `%%` inside it are text, and the block ends at the first `}`                               | rules 13-15 (28-30)                        | `timeline_grammar`                          |
| Q14 | An unclosed `accDescr {` is a parse error; `accDescr {}` gives nothing                                                                              | rules 13-15, notes 2.5                     | `timeline_grammar`                          |
| Q15 | The block text is trimmed as a whole, then each line break followed by blanks becomes one line break (the setter), so no line keeps its indentation | grammar 75, `commonDb` 22-24               | `timeline_grammar`: block indentation       |
| Q16 | Section names are kept with duplicates; tasks of a repeated name are drawn under each occurrence (drawing: R5, not in this plan)                    | `timelineDb` 29-32                         | `timeline_grammar`: repeated section        |
| Q17 | An event with no period before it is an error: the reference throws a `TypeError` in `addEvent`                                                     | `timelineDb` 65-70, verified in Chromium   | `timeline_grammar`                          |
| Q18 | Statements need no line break between them: `timeline section A` is a header and a section                                                          | grammar `document` 43-86                   | `timeline_grammar`                          |
| Q19 | A plain `timeline` keeps the direction of `clear`, `LR`; a second header is a parse error                                                           | grammar 53-57, `timelineDb` 4, 17          | `timeline_spec`: default LR                 |
| Q20 | Entities are encoded before parsing and the model keeps the encoded text (`#quot;` is `ﬂ°quot¶ß`). Only the header texts are sanitised              | `Diagram.ts` 77, `utils.ts` 874-895        | `timeline_grammar`: entities                |
| Q21 | Blank lines and blanks between statements have no effect                                                                                            | rules 2-3 (16-17)                          | `timeline_grammar`: blank lines             |

## 7. Specs

`timeline.spec.js` has 11 tests, all at DB level, and no test renders. They become `crates/ferrigura-syntax/tests/timeline_spec.rs`, with the reference's titles in snake case. The reference's loops over `forEach` and `switch` become exact checks on the model, which is stronger than the reference (its loops pass for an empty task list). Each test parses the text and reads the model, as the spec reads the DB.

The quirks of section 6 are pinned in `crates/ferrigura-syntax/tests/timeline_grammar.rs`. No render or DOM test exists in the folder, so no test is ignored for timeline.

## 8. Differential run

`tools/port/export-timeline.ts` runs the reference's real preprocessor, detector, `encodeEntities`, Jison parser and DB, and reads the DB's state after the parse. The inputs are the 30 fixtures, the spec strings, the 14 docs blocks and `cases.ts`, collected by `inputs.ts`. The configuration group of `pnpm port:diff config` takes the timeline inputs too.

Known limit, the same as the flowchart's: the header texts are sanitised with the default configuration. A `securityLevel` or `htmlLabels` set by a directive or by the front matter changes that sanitising only for a text with `<`, and the port does not apply it. The canonical model's sanitisation section records this.

No named exception is planned for the parser. The renderer's defects (R1-R22, NOTES 5.12) are for the render port.

## 9. Not in this plan

The layout, the wrapping, the two renderers, the classic theme (which 22 of the 30 fixtures need, NOTES 7.3 (1)), the accessibility output (R7) and the render defects. These are the M6 renderer work and need their own plan.

## 10. Open questions

1. The helpers `remove_directives`, `cleanup_comments` and `encode_entities` now serve two parsers. Move them to a shared module, or keep them in `flowchart`? This plan keeps them where they are and makes them `pub(crate)`.
2. The field name `periods` replaces the notes' `tasks`. Confirm before the renderer port reads the model.
3. A directive's `securityLevel` or `htmlLabels` on header texts (section 8). Port the configuration to the sanitiser, or keep the limit? The same question applies to the flowchart.
