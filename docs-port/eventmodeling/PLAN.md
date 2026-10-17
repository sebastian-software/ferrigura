# Event modeling (P2): parser and DB plan

This is the parser task's plan. The research, with every rule cited, is in [NOTES.md](./NOTES.md). The model is in [canonical-model.md](../canonical-model.md), section "Event modeling". The code is `crates/ferrigura-syntax/src/event_modeling/` (`scanner.rs`, `grammar.rs`, `db.rs`, `validator.rs`, `mod.rs`, `ast.rs`), the model is `crates/ferrigura-model/src/event_modeling.rs`, the exporter is `tools/port/export-eventmodeling.ts`.

## 1. What is delivered

- The model: the DB state after the parse, without the geometry and the measured box texts (section 2).
- The parser: the reference's pipeline (`Diagram.fromText`), a scanner that runs the Langium rule order (`scanner.rs`, decision 0002), and a recursive-descent grammar (`grammar.rs`).
- The DB: lanes, the placement of frames, and the relations (`db.rs`). The validator is a function of its own (`validator.rs`), not called by `parse` (quirk E9).
- The differential: `pnpm port:diff eventmodeling` shows no difference on 220 inputs (161 accepted by the reference, 59 rejected by both). The inputs are the reference's fixtures, spec literals and docs blocks, and the probed inputs of NOTES in `tools/port/cases.ts`.
- The specs: the 10 diagram spec tests (`tests/event_modeling_spec.rs`), and the 31 parser tests in two files: `tests/event_modeling_grammar.rs` (the 6 reference tests, one of them, the validator registration, ignored with its reason because Langium's registry is not in the port, plus the added `validation_is_not_run_by_parse`) and `tests/event_modeling_validator.rs` (25 tests).
- The configuration: the `eventmodeling` section is compared by `pnpm port:diff config` (it is no longer in the not-ported lists of `tools/port/diff-config.ts`).

Not in this task: the renderer, the geometry harness entry (`TYPE_FOLDERS`), and the box text's line breaks. The renderer task needs the font decision of NOTES section 8.1 (see section 7 of this plan).

## 2. The model

The model is the DB state after the parse, as the coordinator's brief lists it: the frames, the entities with their lanes and namespaces, the data blocks, the relations and the header texts. Its fields are in `ferrigura-model/src/event_modeling.rs`.

- **Frames** are the boxes, in text order. Each has its name (`01`, kept as written, so `01` and `1` differ), its kind (`time` or `reset`), its entity type, its qualified identifier, the entity name and namespace (the DB's `extractName` and `extractNamespace`), its lane index, its source frames as written, and its inline data and data reference as written.
- **`data`** is the data text the DB shows: the referenced block's text when the block exists, otherwise the inline data's, cut and sanitised as `calculateTextProps` cuts and sanitises it, before the line breaks and the `&nbsp;` joins. It is `null` when the frame shows no data and `""` for a quoted inline value, which the reference shows as an empty code block (quirks E5 and E6).
- **Data blocks** keep the grammar's value, braces and line breaks included.
- **Lanes** are the swimlanes of the DB in index order, with their labels.
- **Relations** join two frame indices, in the order the DB creates them.
- **Header texts** are the common setters' results: the title (the grammar's last `title`, sanitised, else the front matter's), `accTitle` without its leading blanks, `accDescr` with the blanks after each line break removed.

Left out, and why:

- The box geometry (`x`, `y`, the sizes, the lane `y` and `height`, `maxR`) and the box text (`<b>name</b>` and the wrapped data). They need the measured text (NOTES 4.1, 6.7). The `data` field is the part of the box text that does not depend on the measurement, and that is where the quirks are.
- The `entity`, `note` and `gwt` statements. The DB never reads them (`db.ts`, `modelEntities`, `noteEntities`, `gwtEntities` are unused), so they are outside the DB state. Their syntax is still checked: a text that the reference rejects with them is rejected here too. NOTES section 4 proposed them as "kept for the parser differential"; that is the coordinator's call (open question 1).
- The `dataType` of backticked types, which the reference parses and never reads.

## 3. The parser

### 3.1 Pipeline

`Diagram.fromText`, in the reference's order: `prepare` (line breaks, quote rewriting, front matter removal and its title); `remove_directives`; the comment lines are kept (`preserveCommentsWhenParsing`), unless the comment-preserving text is longer than 50 000 UTF-16 units, then the comment lines are removed (`cleanup_comments`); `encode_entities`; a line break at the end. The positions in errors count the original input's lines, front matter included, so a duplicate frame reports the line in the file the user sees.

`maxTextSize` is the default's 50 000: the crate's pipeline does not read the configuration's `maxTextSize`, for any type. A directive that sets it is not honoured here (shared gap, listed for the coordinator).

### 3.2 Scanner

The rule table is Langium's order (NOTES 2.2): the keywords, then `EM_ID`, `EM_FID`, `EM_DATA_INLINE`, `EM_DATA_BLOCK`, `EM_ACC_DESCR`, `EM_ACC_TITLE`, `EM_TITLE`, then the hidden rules `EM_WS`, `EM_YAML`, `EM_DIRECTIVE`, `EM_SINGLE_LINE_COMMENT`, `EM_ML_COMMENT`, `EM_SL_COMMENT`. Each regex is matched by hand as JavaScript runs it at the current position (`scanner.rs` names each one):

- A word keyword is a token only when the identifier run that starts there is exactly the keyword. `eventmodeling` also needs `%%`, white space or the end after it.
- `EM_DATA_INLINE` is greedy to the last matching closing character on the line (`}`, `"` or `'`).
- `EM_DATA_BLOCK` and `EM_YAML` end at the first closing mark that starts a line and is followed by a line break, the end, or white space; a closing mark right after the opening line is the fallback.
- The header rules take the text to the first `%%` on the line, or to the end of the line. `EM_TITLE` without a blank after the word is the word alone.

### 3.3 Grammar

`'eventmodeling' statement*`, where each statement starts with a token that no other statement starts with, so one token of lookahead decides it. Each rule's sequence is the reference's: a frame is the keyword, the number, the entity type, the qualified name, the `->>` sources, the `[[ ]]` reference, and the inline data (an optional type in backticks first). A `data` or `note` needs its block; `gwt` needs `given`, an optional `when`, and `then`, each with at least one statement.

### 3.4 Errors

The error text is ferrigura's own, with the position of the first token that fails (decision 0002). A duplicate frame number is reported at the later frame's keyword: `frame id "01" is already used by an earlier frame`, at line 4 for the spec's input. The reference's text `Duplicate event modeling frame ID "01" on line 4` counts the front matter out; the port counts it in (the position in the original input). Both sides reject the input, so the differential agrees.

## 4. The DB

- **Lanes.** A frame's lane is set by its entity type: UI, processor and `pcr` at 0 (`UI/Automation`); command and read model at 100 (`Command/Read Model`); events and anything else at 200 (`Events`). A namespaced frame takes the next free index in its range (`UI/A: `, `C/RM: `, `Stream: `), and its label is that prefix and the namespace.
- **Relations.** A frame with explicit sources gets one relation per listed frame, in text order, when that frame is already placed (a later one gives none). A frame without sources gets one from the nearest earlier box in another lane, unless it is the first frame or a reset frame.
- **Geometry** is not computed: `calculateX`, the sizes and the `y` values do not change the lanes, the placement rule or the relations.

## 5. Quirks reproduced

Each is in `tools/port/cases.ts` (the input names are the probed rows of NOTES) and so checked by the differential.

| #   | Quirk                                                                                                                                                                                | NOTES    | Where                                          |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------- | ---------------------------------------------- |
| E1  | `title`, `accTitle:` and `accDescr` at the start of a line (or after indentation on a new line) are identifiers, so the parse fails. After a token on the same line they work        | 2.2      | scanner order: `EM_ID` before the header rules |
| E2  | Keywords cannot be identifiers: `tf 01 evt text`, `data`, `A.ui`, `A.text` fail                                                                                                      | 2.2      | keyword check before `EM_ID`                   |
| E3  | Namespaced entities never share a lane: every namespaced frame gets a new one, and the lookup by namespace never finds a lane                                                        | 3.7      | `db.rs` `swimlane_of`                          |
| E4  | A `UI/A:`, `C/RM:` or `Stream:` lane takes the next index of its range, after the largest index in it                                                                                | 3.4      | `next_available_index`                         |
| E5  | Inline data loses the character before its `}`: `{x: 1}` shows `x: `                                                                                                                 | 6.4.1    | `db.rs` `inline_data_cut`                      |
| E6  | Quoted inline data is an empty code block (`""`)                                                                                                                                     | 6.4.1    | `inline_data_cut` (no `{`)                     |
| E7  | A data block's closing `}` must start a line; an indented `  }` fails, and `data D { a: 1 }` fails (it is an inline token)                                                           | 2.2, 2.4 | `line_fenced_len`                              |
| E8  | A block with blanks after its `{` has no `{\n`, so the cut removes the brace alone and keeps the blanks and the line break that follow it                                            | 6.4.1    | `block_data_cut`                               |
| E9  | The validator is registered but `parse` never runs it: an `ui` fed by an `evt` renders                                                                                               | 2.5      | `validator.rs` is not called from `mod.rs`     |
| E10 | Header texts are raw token texts with their keyword and leading blanks (` title Foo`, ` accTitle: Foo`); the common setters then sanitise and strip the leading blanks of `accTitle` | 2.4, 3.2 | `grammar.rs`, `mod.rs` `to_model`              |
| E11 | The last `title`, `accTitle` or `accDescr` of the text wins                                                                                                                          | 2.3      | `grammar.rs` assignments                       |
| E12 | Frames are not line-based: `tf 01 ui A tf 02 cmd B` on one line parses                                                                                                               | 2.2      | scanner has no line rule                       |
| E13 | A frame number is a string: `01`, `1` and `001` are three frames; a repeated number is an error                                                                                      | 2.4      | `check_frame_ids`                              |
| E14 | A reset frame with no sources gets no inferred relation                                                                                                                              | 3.6      | `db.rs` `state`                                |
| E15 | A source that comes later in the text gives no relation; a source naming no frame gives none; a self source gives a self relation                                                    | 3.6      | `db.rs` (`source_frame <= index`)              |
| E16 | Names keep the entity placeholders (`ﬂ°quot¶ß`); the DB decodes nothing                                                                                                              | 3.3      | `encode_entities` in `mod.rs`                  |
| E17 | A qualified name joins its parts without blanks: `A . B` is `A.B`                                                                                                                    | probe    | `qualified_name`                               |
| E18 | Data is sanitised in `data` only; the data block keeps its raw value, and the entity names are not sanitised (identifiers cannot hold markup)                                        | 3.3      | `to_model`, `data_text`                        |

Each quirk above has at least one input in `tools/port/cases.ts`. The spec's own inputs (`eventmodeling.spec.ts`) and the six fixtures are collected automatically.

## 6. Deliberate differences

- The duplicate-frame message is ferrigura's own, and its position counts the front matter lines (section 3.4).
- The validator is a function of its own, tested by its 25 tests; the Langium registry is not ported.
- The lone surrogate case: a data cut can split a surrogate pair (`{ 😀}` cut before the closing brace). The reference's string keeps the lone surrogate; a Rust `String` cannot, so the port writes U+FFFD. No input of the differential has one. Open question 3.

## 7. Open questions for the coordinator

1. The model leaves out the `entity`, `note` and `gwt` statements, since the DB does not read them. NOTES section 4 proposed them as "kept for the parser differential". Add them if the coordinator wants the statements in the model, which would need no more than their names and values.
2. The renderer task needs the font decision of NOTES section 8.1. P2-PLAN's "Hard-coded fonts" row says ferrigura measures `trebuchet ms` with the bundled face and the harness maps the family onto it. That covers the box sizes, and the model needs no sizes.
3. The lone surrogate of section 6, and the `maxTextSize` of the configuration (section 3.1), are shared gaps with the other types' pipelines. Decide whether a model field, an exception, or a fix in `preprocess.rs` is wanted.
4. NOTES' `findSwimlaneByNamespace` lookup is dead code in the reference; the port does not run it. The differential shows no difference, so the dead branch is not observable.
