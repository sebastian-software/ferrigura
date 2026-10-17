# P2 requirement: parser and DB plan

Status: the parser, the DB, the model and the specs are ported; `pnpm port:diff requirement` shows no difference on its 266 inputs. The renderer is ported too (section 11). Reference: `packages/mermaid/src/diagrams/requirement/` (the Jison grammar, `requirementDb.ts`). Research and the probes behind this page are in [NOTES.md](./NOTES.md). Quirk ids `Q1`-`Q44` are NOTES section 2.5; `Q45`-`Q53` are the quirks that the port found by reading the lexer rules or by the differential, and the probes of `cases.ts` carry the same ids in their comments.

The renderer stage (`getData`, the shape, the markers, the style sheet, the layout data) is not part of the parser task: the model is the DB state after the parse. The renderer task is section 11.

## 1. Sources

| File                                                             | Used for                                                                                                             |
| ---------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `diagrams/requirement/parser/requirementDiagram.jison`           | Lexer rules (lines 6-103) and grammar (107-265), in Jison's first-match order                                        |
| `diagrams/requirement/requirementDb.ts`                          | The fields, `addRequirement`, `addElement`, `addRelationship`, `setCssStyle`, `setClass`, `defineClass`              |
| `diagrams/common/commonDb.ts` (16-29)                            | `setAccTitle` (leading white space removed), `setAccDescription` (white space after a line break), `setDiagramTitle` |
| `diagrams/common/common.ts` (71-83)                              | `sanitizeText`: DOMPurify                                                                                            |
| `Diagram.ts` (19-77), `preprocess.ts`, `diagram-api/comments.ts` | Order of `clear`, front matter title, `encodeEntities`, `+ "\n"`, directives, comment lines                          |
| `utils.ts` (874-906)                                             | `encodeEntities`, `decodeEntities`                                                                                   |

The Rust code: `crates/ferrigura-syntax/src/requirement/` (`mod.rs` the grammar and the entry point, `scanner.rs` the lexer, `db.rs` the DB), the model in `crates/ferrigura-model/src/requirement.rs`. The exporter is `tools/port/export-requirement.ts`.

## 2. Text before the lexer

The steps of `Diagram.fromText` after `preprocessDiagram`, in the reference's order: CRLF and CR to LF, `="x"` to `='x'` inside tags, front matter removed and its `title` read, `%%{ … }%%` directives removed, comment lines removed and the text trimmed at the start (`cleanupComments`), then `encodeEntities` (`style` and `classDef` lines lose their last `;`, `#name;` and `#digits;` become the placeholders `ﬂ°name¶ß` and `ﬂ°°digits¶ß`), and one `\n` is appended. The front matter title is set on the DB before the parse, and is not encoded.

The front matter title (Q37), a `#` line before the header (Q32) and a `%%` line before it (Q34) are probed; the rest of the steps are the shared ones.

## 3. Scanner

Jison's `next()` takes the first rule that matches at the current position. The rules are tried in the order of the `.jison` file, and the rules without a start condition apply in the initial state only. Keyword literals that end in a word character take an implicit `\b` (Q4). Keywords match without regard to case (`%options case-insensitive`; names keep their case, Q6).

| #     | Rule (initial state unless stated)                                                                                                                                                                  | Token                                                                                                                     | Notes                                                                               |
| ----- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| 18    | `title\s[^#\n;]+`                                                                                                                                                                                   | `title`, an error in the grammar                                                                                          | Also at the start of a value (Q46)                                                  |
| 19    | `accTitle\s*":"\s*`                                                                                                                                                                                 | `acc_title`, enters `acc_title`                                                                                           | The white space after the colon may cross lines                                     |
| 20    | (`acc_title`) the rest of the line, possibly empty                                                                                                                                                  | `acc_title_value`, leaves the state                                                                                       |                                                                                     |
| 21-22 | `accDescr\s*":"\s*`, then (`acc_descr`) the rest of the line                                                                                                                                        | `acc_descr`, `acc_descr_value`                                                                                            |                                                                                     |
| 23-25 | `accDescr\s*"{"\s*`, then (`acc_descr_multiline`) up to `}`                                                                                                                                         | `acc_descr_multiline_value`                                                                                               | `}` leaves the state; `accDescr {}` gives no value (Q48)                            |
| 26-29 | `.*direction\s+TB[^\n]*`, then BT, RL, LR                                                                                                                                                           | `direction_*`                                                                                                             | The greedy `.*` takes the last keyword of the line, from any token start (Q13, Q47) |
| 30    | `(\r?\n)+`                                                                                                                                                                                          | `NEWLINE`                                                                                                                 |                                                                                     |
| 31    | `\s+`                                                                                                                                                                                               | skipped                                                                                                                   | Runs over line breaks when it starts with a blank (Q2, Q49)                         |
| 32    | `\#[^\n]*`                                                                                                                                                                                          | skipped                                                                                                                   | A `#` at a token start is a comment (Q12, Q33)                                      |
| 33    | `\%%[^\n]*`                                                                                                                                                                                         | skipped                                                                                                                   | One `%` starts a comment, not two (Q45)                                             |
| 34    | end of the input                                                                                                                                                                                    | `EOF`                                                                                                                     |                                                                                     |
| 36    | `requirementDiagram`                                                                                                                                                                                | `RD`                                                                                                                      |                                                                                     |
| 38-41 | `{`, `}`, `:::` (three colons), `:`                                                                                                                                                                 | `STRUCT_START`, `STRUCT_STOP`, `STYLE_SEPARATOR`, `COLONSEP`                                                              |                                                                                     |
| 43-75 | the keywords: `id`, `text`, `risk`, `verifyMethod`, the six kinds, `low`, `medium`, `high`, `analysis`, `demonstration`, `inspection`, `test`, `element`, the seven relationships, `type`, `docref` | the keyword's token                                                                                                       | Each takes the word boundary (Q4, Q44)                                              |
| 77    | `style`                                                                                                                                                                                             | `STYLE`, enters `style`                                                                                                   |                                                                                     |
| 78-86 | (`style`) `\w+`, `:`, `;`, `%`, `-`, `#`, a space, `"`, `\n`                                                                                                                                        | `ALPHA`, `COLON`, `SEMICOLON`, `PERCENT`, `MINUS`, `BRKT`; a space is skipped; `"` enters `string`; `\n` leaves the state | Any other character is an error (Q19, Q20). The line break makes no token (Q51)     |
| 88-89 | `classDef`, `class`                                                                                                                                                                                 | `CLASSDEF`, `CLASS`, enter `style`                                                                                        |                                                                                     |
| 91-93 | `<-`, `->`, `-`                                                                                                                                                                                     | `END_ARROW_L`, `END_ARROW_R`, `LINE`                                                                                      |                                                                                     |
| 95-97 | `"` (enters `string`), then `"` leaves it and `[^"]*` is a `qString`                                                                                                                                | `qString`                                                                                                                 | An empty pair makes no token (Q26); an unclosed one ends with the raw end (Q53)     |
| 99    | `[\w][^:,\r\n\{\<\>\-\=]*`                                                                                                                                                                          | `unqString`, trimmed                                                                                                      | ASCII `\w` only (Q43); spaces, `#`, `"`, `}` and `;` are inside it (Q9-Q11)         |
| 103   | `,`                                                                                                                                                                                                 | `COMMA`                                                                                                                   | In every state                                                                      |

Everything else is an error: `;`, `(`, `'`, a non-ASCII letter at a token start, `=`, `<` or `>` alone. Rules 101-102 (`\w+` and `[0-9]+`) never make a token: a word character takes rule 99 in the initial state and rule 78 in the style state.

The scanner reproduces Jison's `next()`, including the `done` flag: once the input is empty, the rules of the current state are read once more, and after that every call returns Jison's raw end symbol. A value at the end of the text is therefore followed by a raw end, which the grammar does not accept (Q52).

## 4. Grammar and DB calls

The grammar is read as a recursive descent over the tokens. Each production makes its DB calls where Jison's actions run, and a block's properties are collected first, because the actions of a body run before the action of its block.

```
start       : directive NEWLINE start | directive start | RD NEWLINE diagram EOF
directive   : acc_title acc_title_value | acc_descr acc_descr_value | acc_descr_multiline_value
diagram     : (requirementDef | elementDef | relationshipDef | directive | direction
              | styleStatement | classDefStatement | classStatement | NEWLINE)*
requirementDef : kind name '{' NEWLINE requirementBody          addRequirement(name, kind)
               | kind name ':::' idList '{' NEWLINE requirementBody   addRequirement; setClass([name], idList)
requirementBody: (ID ':' id | TEXT ':' text | RISK ':' level | VERIFYMTHD ':' method) NEWLINE | NEWLINE)* '}'
elementDef  : ELEMENT name '{' NEWLINE elementBody  (and the ':::' form, as requirementDef)
elementBody : (TYPE ':' type | DOCREF ':' ref) NEWLINE | NEWLINE)* '}'
relationshipDef: id '<-' relationship '-' id    addRelationship(relationship, second, first)
               | id '-' relationship '->' id    addRelationship(relationship, first, second)
classStatement : id ':::' idList                setClass([id], idList)
               | 'class' idList idList          setClass(first, second)
classDefStatement: 'classDef' idList stylesOpt  defineClass(idList, stylesOpt)
styleStatement : 'style' idList stylesOpt       setCssStyle(idList, stylesOpt)
direction   : direction_tb | direction_bt | direction_rl | direction_lr   setDirection
```

A property takes its first value, and a block whose name exists keeps its first block (Q7, Q8). A relationship's endpoints are in the order the grammar gives them: `X <- T - Y` stores `Y T X` (probed: `e <- traces - r` stores `r traces e`).

## 5. DB

| Field or method              | Behaviour                                                                                                                                                                                    |
| ---------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `direction`                  | `TB` at the start; each `direction` statement replaces it (Q35). `clear()` does not reset it (Q36), and a new DB starts at `TB`                                                              |
| `addRequirement(name, kind)` | A new name takes the block's properties; a name that exists keeps its block (Q8). Classes start as `default`. The latest properties are reset after the call                                 |
| `addElement(name)`           | The same rule for `type` and `docref`                                                                                                                                                        |
| `addRelationship`            | Appended; the endpoints are not checked against the names (Q14)                                                                                                                              |
| `setCssStyle(ids, styles)`   | Each id in turn: a name that is no requirement or element stops the call, so the ids after it are not styled (Q16, Q17). A requirement is looked up before an element (Q15)                  |
| `setClass(ids, classes)`     | Each node that exists gets each class name, and the styles of a class defined so far (Q24, Q25)                                                                                              |
| `defineClass(ids, styles)`   | Creates the class, appends the styles, and appends them to each node already carrying the class (Q24). A style with `color` is also a text style, with the first `fill` replaced by `bgFill` |
| `setDiagramTitle`            | `sanitizeText`                                                                                                                                                                               |
| `setAccTitle`                | `sanitizeText`, then the leading white space removed                                                                                                                                         |
| `setAccDescription`          | `sanitizeText`, then the white space after each line break removed (Q31)                                                                                                                     |

The DB does not sanitise requirement, element, relationship or class texts; they are what the grammar gives. For `text: "a &amp; <b>x</b> #lt; #9829;"` the model holds `a &amp; <b>x</b> ﬂ°lt¶ß ﬂ°°9829¶ß`: the raw references stay as written, and the entities are placeholders (`cases.ts`). The model keeps them so (open question 5 of NOTES, answered: the placeholders stay).

## 6. Model

`RequirementDiagram` with `version` 1 and camelCase keys: `direction` (`TB`, `BT`, `RL`, `LR`), `requirements` (`name`, `type`, `requirementId`, `text`, `risk`, `verifyMethod`, `classes`, `cssStyles`), `elements` (`name`, `type`, `docRef`, `classes`, `cssStyles`), `relationships` (`type`, `src`, `dst`), `classes` (`id`, `styles`, `textStyles`), `title`, `accTitle`, `accDescr`. The lists are in the order of the DB's maps and arrays. `type` of a requirement is the display string (`Requirement`, `Functional Requirement`, and so on); `risk` is `Low`, `Medium` or `High`, `verifyMethod` is `Analysis`, `Demonstration`, `Inspection` or `Test`, and an unset one is `""`. The reference's `getData` is not part of the model.

## 7. Quirks reproduced

| Id     | Quirk                                                                                                                                     | Where                 |
| ------ | ----------------------------------------------------------------------------------------------------------------------------------------- | --------------------- |
| Q1-Q44 | As NOTES section 2.5: the parse results and the DB results of each probe are in `cases.ts`, with the id in its comment                    | the rules above       |
| Q45    | A single `%` at a token start starts a comment: the reference reads `\%%` as one `%` (probed: `%x` and `%` alone are accepted)            | rule 33               |
| Q46    | A value that starts with `title ` is the `title` token, an error: `text: title x` is rejected, `text: my title x` is one text             | rule 18               |
| Q47    | `direction` anywhere on the line of a token start makes the token a direction, with the rest of the line; `direction\nTB` spans two lines | rules 26-29           |
| Q48    | `accDescr {}` after the header is accepted with no value; the empty block leaves no token, and the description stays empty                | rules 23-25           |
| Q49    | A header followed by blanks and a line break is rejected: the blank run takes the line break, so the header has no `NEWLINE`              | rule 31, `RD NEWLINE` |
| Q50    | `accTitle:` at the end of the text is rejected: the value is empty and the raw end follows it                                             | the `done` flag       |
| Q51    | A `style`, `classDef` or `class` line makes no `NEWLINE` token: its line break leaves the style state                                     | rule 86               |
| Q52    | A value at the end of the text (`accDescr {` with no `}`, `accTitle:`) is rejected, because the raw end follows the empty value           | the `done` flag       |
| Q53    | An unclosed `"` takes the text to the end, and the raw end follows: the text is rejected                                                  | rule 97               |

## 8. Specs

| Reference spec                                           | Rust file                                              | Tests | Notes                                                                                                               |
| -------------------------------------------------------- | ------------------------------------------------------ | ----: | ------------------------------------------------------------------------------------------------------------------- |
| `diagrams/requirement/parser/requirementDiagram.spec.js` | `crates/ferrigura-syntax/tests/requirement_spec.rs`    |    43 | Every test passes; the two loops over `__proto__` and `constructor` are four tests, named `proto` and `constructor` |
| `diagrams/requirement/requirementDb.spec.ts`             | `crates/ferrigura-syntax/tests/requirement_db_spec.rs` |    10 | 9 pass, and 1 is ignored: the edge ids are `getData`'s, the renderer's stage                                        |

The DB spec calls the DB directly, which the public API does not expose; each test drives `parse` with the text that makes the same call. Two values differ from the spec's literals because a style is its tokens joined with no spaces: `stroke-width: 8px` is `stroke-width:8px`, and `stroke: yellow` is `stroke:yellow` (Q21). The texts of these tests are in `cases.ts`, so the differential checks their values against the reference.

## 9. Differential

`tools/port/export-requirement.ts` runs the reference's Jison parser with `yy` set to a new `RequirementDB` for each input, after `preprocessDiagram`, `encodeEntities` and the line break, and writes the model with the texts as the DB holds them. The 266 inputs are the 172 e2e fixtures (170 accepted; the two malformed visual-check fixtures are rejected by both sides, which is what the reference makes of them, and no fixture was changed), 14 spec and documentation inputs, and the explicit cases of `cases.ts` (Q1-Q53, the DB spec's texts, and the header blanks). The result is 232 accepted, 0 named exceptions, 0 differences.

The configuration run (`pnpm port:diff config`) compares the `requirement` section now that the type is ported: the sections of `diff-config.ts` lose `requirement`, and the two inputs that the detector does not take (`# comment` before the header, and the empty text) are in `undetectedCases`.

Random inputs were run through both sides as well: 10000 generated texts (token soup, structured blocks, and mutated fixtures), with one difference. One difference, on a front matter whose YAML the reference's reader rejects and the port's shared reader accepts (`---`, `: >-`, `---`). The journey parser has the same gap with the same input, so it is in `yaml.rs`, not in this type. It is not in the 266 inputs, and it is open question 1 below.

## 10. Open questions

1. **Front matter YAML that the reference rejects.** A front matter with a value after a key-less `:` (`---\n: >-\n  a\n---`) is an error in the reference (`yaml` package: "incomplete explicit mapping pair") and accepted by the port's shared reader. Affects every type; not fixed here.
2. **Undeclared relationship endpoint** (Q14). The parser keeps the relation, as the DB does. The reference fails in ELK at render time; the render stage decides.
3. **A requirement and an element with the same name** (Q15). Both are stored, as the DB stores them. The reference's render writes two nodes with one id; the render stage decides.
4. **Error messages.** The port reports the position and the token it rejects, with its own text; the Jison `Expecting` lists are not reproduced (decision 0002).
5. **The texts keep the placeholders.** Answered by the port: the model holds what the DB holds (`ﬂ°…¶ß`), as NOTES section 4 proposed. The renderer decodes them.
6. **The `state` section quirk** (title margin and `useMaxWidth`, NOTES 6.8) is the renderer's, and is not reproduced here.

## 11. Renderer

The renderer is on the unified pipeline, as `class`, `state` and `er` are. The research and the reference's lines are in [NOTES.md](./NOTES.md), section 6.

- **Layout data** (`pnpm port:diff requirement-data`): `getData` in `crates/ferrigura-render/src/requirement/data.rs`; the reference's exporter is `tools/port/export-requirement-data.ts`, and the example is `requirement_data`. 266 inputs, 232 accepted by the reference, 0 differences. A requirement or an element carries its fields in `Node.requirement` (`RequirementFields`, `unified/data.rs`), as a class carries `Node.class`.
- **Shape** (`unified/shapes/requirement_box.rs`, `requirementBox`): it draws its own labels, as `erBox` does. Each text is measured with `getBBox` plus 6 in height, the outline is the box of the texts plus 20, and the labels are moved by their index. Divider: a polygon in neo, a line in classic.
- **Markers** (`unified/markers.rs`, `requirement_markers`): `requirement_arrow` and `requirement_contains` in the classic and neo forms; the edges name them through `Kind::RequirementArrow` and `Kind::RequirementContains`.
- **Style sheet** (`requirement/style.rs`): the reference's rules in its order, rule by rule (checked against a capture: the same selectors and properties, and the same values once the custom properties of decision 0007 are resolved). The font families are the embedded subsets, as the ER sheet has them.
- **Root** (`requirement/mod.rs`): the title at `-state.titleTopMargin`, the viewport from `state.useMaxWidth`, the class `requirementDiagram` and `aria-roledescription="requirement"`.

Findings that the research did not have:

- **Bold and other weights.** The reference measures every word of a label in the weight its group's style declares: the name is bold because its group's style says so (measured: 51.086 px against 46.130 px for `sys_req`), and a `font-weight:bold` on a node or in a class is measured the same way. The port applies a `font-weight` of the group's style to every word (`style_weight`, `requirement_box.rs`).
- **Body alignment.** The reference's `center` branch (`requirementBox.ts` 30 and 238-243, for a `layout` other than `elk`) writes no `text-anchor`, and a node's text is never centred by `createText` (`centerText` is false for nodes). So the body starts at the origin for every layout, and the port does the same. Q8 of section 8.3 is answered by this reading; the harness draws only `elk` (the other layouts are excluded).
- **`htmlLabels`.** Ignored, as decision 0004 has it: labels are SVG text.
- **Config.** `state.titleTopMargin` and `state.useMaxWidth` are read (Q6, reproduced); the `requirement.rect_*`, `text_color`, `fontSize` and `line_height` keys are read nowhere (Q6, accepted).

Answers to section 8.3 (the question numbers are NOTES' own):

- **Q1.** The two malformed fixtures stay "reference failed" in the harness; their text is not the port's question.
- **Q2 and Q3.** Named exceptions: `render` refuses a relationship to a name that is no requirement or element, and a requirement and an element with one name (`requirement/mod.rs`, `check_names`). The reference fails in ELK for the first and writes duplicate ids for the second. No fixture has either.
- **Q4.** Not applicable: the renderer has no parse errors.
- **Q5.** The texts keep the placeholders in the model, and the renderer draws `sanitize_text(decode_entities(…))`.
- **Q6.** Reproduced, as above.
- **Q7.** Neo is the default look, resolved by the requirement scope.
- **Q8.** See the body alignment above.
- **Q9.** Not ported, as decision 0004 has it.
- **Q10.** The direction is kept in the model and the layout; `clear()` has no effect, since a new DB starts at `TB`.

Counts of `pnpm port:geometry requirement` (172 fixtures): within tolerance 146, outside 0, not rendered 0, excluded 24 (handDrawn), reference failed 2 (the malformed fixtures of section 7.3). The DB spec's edge-id test is ported to `crates/ferrigura-render/tests/requirement_get_data_spec.rs`. The syntax spec's parallel-relationship test is no longer ignored: it checks the order of the relationships, which is what the model holds.
