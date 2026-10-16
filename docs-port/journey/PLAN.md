# M6 user journey parser: plan

Status: plan written before the Rust code. Reference: the JS reference of this worktree. Research and the probes behind it are in [NOTES.md](./NOTES.md); this page says what the port does and which quirks it keeps. Quirk ids `J1`-`J20` are used in the code comments and in the differential's `cases.ts` entries.

## 1. Sources

| File                                              | Used for                                                                   |
| ------------------------------------------------- | -------------------------------------------------------------------------- |
| `diagrams/user-journey/parser/journey.jison`      | Lexer rules (lines 6-36) and grammar (44-67), in Jison's first-match order |
| `diagrams/user-journey/journeyDb.js`              | `addSection`, `addTask`, `getTasks`, `getActors`, the default export       |
| `diagrams/common/commonDb.ts`                     | `setDiagramTitle`, `setAccTitle`, `setAccDescription` (sanitising)         |
| `diagrams/common/common.ts`                       | `sanitizeText`: DOMPurify, then `FORBID_TAGS: ['style']`                   |
| `Diagram.ts` (19-85)                              | Order of `clear`, front matter title, `encodeEntities`, `+ "\n"`           |
| `preprocess.ts`, `diagram-api/comments.ts`        | Line breaks, attribute quotes, directives, comment lines, `trimStart`      |
| `utils.ts` (874-906)                              | `encodeEntities`, `decodeEntities`                                         |
| `svgDraw.js` (274-390), `mermaidAPI.ts` (344-365) | How a text reaches the reader: `.text(content)`, then `decodeEntities`     |

The Rust code: `crates/ferrigura-syntax/src/journey/` (`mod.rs` grammar and entry point, `scanner.rs` lexer, `db.rs` DB and text conversion), the model in `crates/ferrigura-model/src/journey.rs`, the entity table in `crates/ferrigura-syntax/src/entities.rs`.

## 2. Text before the lexer

The steps of `Diagram.fromText` after `preprocessDiagram`, in the reference's order:

1. CRLF and CR to LF; `="x"` to `='x'` inside tags; front matter removed, its `title` read (`prepare`, shared).
2. `%%{ … }%%` directives removed (`remove_directives`, shared with the flowchart).
3. Comment lines removed and the text trimmed at the start (`cleanup_comments`, shared).
4. `encodeEntities` (`encode_entities`, shared): the `style` and `classDef` lines lose their last `;` (J13); `#name;` and `#digits;` become the placeholders `ﬂ°name¶ß` and `ﬂ°°digits¶ß`.
5. One `\n` is appended, and the scanner reads the result.

The front matter title is not encoded: it is set on the DB before the parse (J14).

## 3. Scanner

Jison's `next()` takes the first rule that matches at the current position, not the longest. The rules run in the order of `journey.jison`. Unannotated rules belong to the initial state only, because the states `acc_title`, `acc_descr` and `acc_descr_multiline` are exclusive (`%x`). Keywords are case-insensitive (`%options case-insensitive`) and `journey` needs a word boundary after it (J1).

| #   | Rule (initial state unless stated)                                                               | Token                                                            |
| --- | ------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------- |
| 1   | `%%` not followed by `{`, then the line                                                          | skipped                                                          |
| 2   | one character other than `}`, then `%%`, then the line (consumes the character, J2)              | skipped                                                          |
| 3   | one or more `\n`                                                                                 | NEWLINE                                                          |
| 4   | one or more JavaScript white space (`\s`, with newlines)                                         | skipped                                                          |
| 5   | `#` and the line                                                                                 | skipped                                                          |
| 6   | `journey` and a word boundary                                                                    | journey                                                          |
| 7   | `title`, one white space character, then one or more characters other than `#`, `\n`, `;`        | title, the value is the text after the first 6 characters (J5)   |
| 8   | `accTitle`, white space, `:`, white space (may cross lines)                                      | acc_title, enters `acc_title` (J3)                               |
| 9   | (`acc_title`) the rest of the line, possibly empty                                               | acc_title_value, leaves the state                                |
| 10  | `accDescr`, white space, `:`, white space                                                        | acc_descr, enters `acc_descr`                                    |
| 11  | (`acc_descr`) the rest of the line                                                               | acc_descr_value, leaves the state                                |
| 12  | `accDescr`, white space, `{`, white space                                                        | none, enters `acc_descr_multiline`                               |
| 13  | (`acc_descr_multiline`) `}`                                                                      | none, leaves the state                                           |
| 14  | (`acc_descr_multiline`) everything up to `}`, possibly empty                                     | acc_descr_multiline_value                                        |
| 15  | `section`, one white space character, then one or more characters other than `#`, `:`, `\n`, `;` | section, the value is the text after the first 8 characters (J6) |
| 16  | one or more characters other than `#`, `:`, `\n`, `;`                                            | taskName (blanks at the end kept, J7)                            |
| 17  | `:` and one or more characters other than `#`, `\n`, `;`                                         | taskData (with its colon)                                        |
| 18  | `:`                                                                                              | colon: an error in the grammar                                   |
| 19  | end of input                                                                                     | EOF                                                              |
| 20  | any other character (`;`, a lone `}`, …)                                                         | INVALID: an error in the grammar                                 |

Jison's `next()` also sets a `done` flag when the input is empty. The rules of the current state are read once more at that point: in the initial state rule 19 gives the grammar's `EOF`, but in an exclusive state the value rule matches the empty text and gives an empty value. After that, every call returns Jison's raw end symbol (`1`), which is not the grammar's `EOF` terminal, so the parser rejects the text (J4). The scanner reproduces the flag, so the quirk comes out of the same logic.

## 4. Grammar and DB calls

```
start     : journey document EOF
document  : (statement | NEWLINE | EOF)*
statement : title                       setDiagramTitle(value)
          | acc_title acc_title_value   setAccTitle(trim(value))
          | acc_descr acc_descr_value   setAccDescription(trim(value))
          | acc_descr_multiline_value   setAccDescription(trim(value))
          | section                     addSection(value)
          | taskName taskData           addTask(taskName, taskData)
```

The grammar builds no tree: each statement calls the DB at once, in document order. A second `journey` keyword, a colon, an invalid character, or a task name that is not followed by its data is an error (J18). The parse rejects the text and the model is not built.

## 5. DB

| Method                 | Behaviour                                                                                                                                                                                     |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `setDiagramTitle`      | `sanitizeText(text)`; the last title line wins, even when the result is `""` (J14)                                                                                                            |
| `setAccTitle`          | `sanitizeText(text)` without leading white space                                                                                                                                              |
| `setAccDescription`    | `sanitizeText(text)` with each line break and the white space after it reduced to one line break                                                                                              |
| `addSection`           | `currentSection = text`, pushed unchanged (J6)                                                                                                                                                |
| `addTask(descr, data)` | `pieces = data.substr(1).split(':')`; `score = Number(pieces[0])`; `people = pieces.length == 1 ? [] : pieces[1].split(',')`, each trimmed; pieces after the second are ignored (J8, J9, J10) |
| `getTasks`             | the tasks in the order they were added                                                                                                                                                        |
| `getActors`            | the unique names of all people, sorted by UTF-16 code units, after `getTasks` (J11)                                                                                                           |

`sanitizeText` runs the DOMPurify subset of `sanitize.rs` (J15): text without `<` comes back unchanged, and text with `<` goes through the parser of the subset. The reference runs DOMPurify twice under the default configuration (`removeScript`, then the `style` removal); the port runs the second pass only, and the differential decides whether that is enough.

`Number()` is JavaScript's `StringToNumber` on the encoded text: blank text is 0; `0x`, `0o` and `0b` take no sign; `Infinity` with a sign; decimal literals with an exponent; `NaN`, `inf` and `1_000` are NaN. Scores are kept as `f64` and written as the sentinel strings `"NaN"`, `"Infinity"` and `"-Infinity"` (section 7).

## 6. Text as the reader sees it

The DB keeps the placeholders. The text reaches the reader this way: the renderer sets the text with `.text(…)` (`svgDraw.js` 274-390), which escapes `&`, `<` and `>` on serialisation; `decodeEntities` replaces `ﬂ°°` by `&#`, `ﬂ°` by `&` and `¶ß` by `;` in the whole SVG string (`mermaidAPI.ts` 359); the HTML parser reads the result back.

The canonical text is therefore:

```
display(value) = decode_references( decode_marks( escape(value) ) )
escape: & -> &amp;, < -> &lt;, > -> &gt;
decode_marks: ﬂ°° -> &#, ﬂ° -> &, ¶ß -> ;
decode_references: the HTML character references of the text state (named, numeric, legacy without ;)
```

Raw text stays as written: `Task &amp; x` displays `&amp;` (the escape is undone by the parser). Only the placeholders, and any marks the user typed, change. Unknown names stay as `&name;` (J12). This is the reading of "entities as the DB decodes them" in the brief: the DB itself holds the placeholders, so the canonical text is the value after the reader's decoding.

The named table is the full HTML5 list (2,125 names with `;`, and the 106 legacy names without it) in `entities.rs`, generated from `character-entities@2.0.2` and `character-entities-legacy@1.1.4`, the two packages of the reference's dependency tree. `sanitize.rs` uses the same table.

## 7. Canonical model

`JourneyDiagram` with `version` 1 and camelCase keys: `title`, `accTitle`, `accDescr`, `sections` (every section in order, duplicates kept), `tasks` (`section`, `task`, `score`, `people`), `actors`. The DB's `type` field of a task equals `section` and is left out. A score that is not finite is the string `"NaN"`, `"Infinity"` or `"-Infinity"` (the convention of the configuration differential, `value.rs`); the JS exporter writes the same.

## 8. Quirks reproduced

| Id  | Quirk                                                                                                                                                                                                                                     | Where                               |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------- |
| J1  | Keywords match at the start of any word: `journeyfoo: 5: me` is a task; `journey` needs a boundary after it                                                                                                                               | rules 6, 7, 15                      |
| J2  | The comment rule consumes one character before `%%` at a token start (`x%%y` skips the line)                                                                                                                                              | rule 2                              |
| J3  | `accTitle:` and `accDescr:` take the rest of the line; the white space after the colon may cross lines                                                                                                                                    | rules 8-11                          |
| J4  | A text that ends inside an accessibility value is rejected: `accTitle:` followed by nothing, or an `accDescr { …` block without its `}`. The value rule runs at the end of the input, and the parser then receives Jison's raw end symbol | rules 9, 11, 14 and the `done` flag |
| J5  | `title` drops the character after the keyword: `title  X` is `" X"`; a title that sanitises to `""` replaces the front matter title                                                                                                       | rule 7, `setDiagramTitle`           |
| J6  | `section` drops the character after the keyword; the name keeps its blanks and is not sanitised                                                                                                                                           | rule 15, `addSection`               |
| J7  | A task name keeps trailing blanks, and the task data must follow it at once: a comment or a line break between them is an error                                                                                                           | rules 16-17                         |
| J8  | Scores are `Number()` of the text: blanks and `""` are 0, `0x10` is 16, `1e1` is 10, `+3` is 3, `abc` and `2rem` are NaN, `Infinity` is infinite                                                                                          | `addTask`                           |
| J9  | Only the first two pieces of the task data split at `:` are read                                                                                                                                                                          | `addTask`                           |
| J10 | People split at `,` and trimmed; an empty name (`E task: 5:`) and duplicates are kept                                                                                                                                                     | `addTask`                           |
| J11 | Actors are sorted by UTF-16 code units, on the encoded names, so a placeholder sorts by its marks                                                                                                                                         | `getActors`                         |
| J12 | Entities: a known name or number is decoded, an unknown name stays as `&name;`, a number outside Unicode is U+FFFD; a mark typed by the user is decoded as well                                                                           | `display`                           |
| J13 | The last `;` of a line with `style` or `classDef`, followed by `:`, a non-blank run and `#`, is removed before lexing                                                                                                                     | `encodeEntities`                    |
| J14 | The front matter title is set before the parse and is not encoded; the last grammar title wins, even when empty                                                                                                                           | `Diagram.fromText`                  |
| J15 | Header texts are sanitised by the DOMPurify subset, which runs once here where the reference runs it twice                                                                                                                                | `sanitizeText`                      |
| J16 | Line breaks in labels are literal text (`<br>`); the renderer's concern, the model keeps the text                                                                                                                                         | renderer (M6 later)                 |
| J17 | Raw references in the source are text: `&amp;` reads as `&amp;`, because only the placeholders are decoded                                                                                                                                | `display`                           |
| J18 | `journey` is a keyword anywhere, so a second header line, or a task named `Journey …`, is an error                                                                                                                                        | rule 6                              |
| J19 | The text is trimmed at the start before the lexer (`cleanupComments`), which changes what `\s*` in `accTitle:` sees                                                                                                                       | step 3                              |
| J20 | A title with `;` or `#` in it stops at that character, and the rest of the line is an error or a comment                                                                                                                                  | rule 7                              |

## 9. Specs

| Reference spec                                 | Rust file                                          | Tests | Notes                                                                         |
| ---------------------------------------------- | -------------------------------------------------- | ----: | ----------------------------------------------------------------------------- |
| `diagrams/user-journey/journeyDb.spec.js`      | `crates/ferrigura-syntax/tests/journey_db_spec.rs` |     9 | the DB has no public surface: each test drives `parse` (flowchart convention) |
| `diagrams/user-journey/parser/journey.spec.js` | `crates/ferrigura-syntax/tests/journey_spec.rs`    |     7 | `not.toThrow()` becomes `parse(...).is_ok()`; DB state is read from the model |

The journey reference has no render or DOM spec (NOTES section 6), so no test is ignored for rendering. The `it.each` rows of `journeyDb.spec.js` become one test each, named after the row, with a `_2` suffix for the second block's repeats.

## 10. Differential

`tools/port/export-journey.ts` runs the reference's Jison parser with `parser.yy = journeyDb`, after `preprocessDiagram`, `encodeEntities` and the `\n`, reads the DB in the order of the renderer (`getTasks`, then `getActors`), and writes the model with `display` applied through the reference's `decodeEntities` and a parser of its own (jsdom). `cases.ts` holds the quirks above that the specs do not reach.

## 11. Open questions

1. The canonical text of an entity (section 6) is the reader's text, not the DB value. The DB value would keep the placeholders, which are internal to Mermaid.
2. The reference runs DOMPurify twice on header texts (J15). The port runs the subset once, and the differential shows whether the second pass changes any input.
3. The renderer (text placement, legend wrap, the line height of `fo` labels) is not in this plan: this milestone ports the parser and the model.
