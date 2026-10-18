# P3 venn parser: plan

Status: the parser, the DB and the model are ported. The differential shows no difference on the 277 inputs of `tools/port/cases.ts` and the collected fixtures (198 accepted by the reference). Reference: `packages/mermaid/src` in this worktree. Research and probes: [NOTES.md](./NOTES.md), sections 2 to 5 and 8, with the three corrections of section 2 below. Quirk ids `V1`-`V21` are used in the code comments, in the spec names and in the differential's `cases.ts` entries.

## 1. Sources

| File                                                             | Used for                                                                                  |
| ---------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| `diagrams/venn/parser/venn.jison`                                | Lexer rules (1-46) and grammar (48-135), in Jison's first-match order                     |
| `diagrams/venn/vennDB.ts`                                        | `addSubsetData`, `addTextData`, `addStyleData`, `validateUnionIdentifiers`, `customClear` |
| `diagrams/common/commonDb.ts`                                    | `setDiagramTitle`: the title is sanitised                                                 |
| `diagrams/common/common.ts`                                      | `sanitizeText`: DOMPurify, then `FORBID_TAGS: ['style']`                                  |
| `Diagram.ts` (49-83)                                             | Order of `clear`, front matter title, `encodeEntities`, `+ "\n"`, `parser.yy = db`        |
| `preprocess.ts`, `diagram-api/comments.ts`, `utils.ts` (874-906) | Front matter, directives, comment lines, `trimStart`, `encodeEntities`                    |

The Rust code: `crates/ferrigura-syntax/src/venn/` (`mod.rs` the grammar and the entry point, `scanner.rs` the lexer, `db.rs` the DB), the model in `crates/ferrigura-model/src/venn.rs`, the exporter `tools/port/export-venn.ts`.

## 2. Corrections to the research

The notes of section 2 and 3 are read against `venn.jison` and the Jison code, and three points differ:

1. **A number in `text` takes no label.** The grammar has `TEXT identifierList NUMERIC` without a bracket label (`venn.jison` 81-85). NOTES 2.2 gave it one. `text A 12["x"]` is rejected (V13).
2. **The keywords need a word boundary.** Jison's rule builder (`lex-parser`, `case 31`) appends `\b` to a pattern that ends in a word character. So `venn-beta`, `set`, `union`, `text` and `style` are keywords only before a non-word character, and `setA`, `settings`, `unionA` and `textual` are identifiers (V11). NOTES 2.1 read them as prefixes.
3. **The styles are a JavaScript object.** `addStyleData` writes `styles[key]`, so a repeated key keeps its first place and takes its last value, and the key `__proto__` is dropped (V14). NOTES 3.2 gave a list.

Two more points the notes did not have: rule 8 (`[^\}]\%\%`) matches one UTF-16 unit, so a character outside the BMP before `%%` is an error (V15); and a line that begins with `%%{` clears the indent mode, as rule 19 does, because the directive comment is taken by rule 22, after the `bol` rules (V1).

## 3. Scanner

The rules are tried in the order of `venn.jison`, first match wins. The start condition `bol` is inclusive; `bol` is set by a newline and cleared by rule 19 or by the `text` lookahead.

| Rule (line)    | Token or effect                                                                                                                                                                                             |
| -------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 7, 22          | `%%` to the end of the line is skipped (rule 7 excludes `%%{`, which rule 22 takes after the `bol` rules)                                                                                                   |
| 8              | A character that is not `}` and is in the BMP, then `%%`: that character and the comment are skipped (V2, V15)                                                                                              |
| 10, 11, 18, 19 | In `bol`: a line of blanks is skipped; blanks before `text\b` return `INDENT_TEXT` in indent mode; other blanks are skipped; any other non-newline character clears the indent mode and is lexed again (V1) |
| 21             | `[\n\r]+` is `NEWLINE` and begins `bol`                                                                                                                                                                     |
| 23             | Blanks are skipped                                                                                                                                                                                          |
| 26             | `"title"\s[^#\n;]+`: `TITLE`, with the rest of the line; the `\s` may be a newline (V4)                                                                                                                     |
| 27-31          | `venn-beta`, `set`, `union`, `text`, `style`, each with a word boundary (V11). `text` is consumed after `INDENT_TEXT`                                                                                       |
| 33, 34         | Brackets: `["…"]` (quotes removed) and `[…]` (trimmed); neither takes `"` in the plain form                                                                                                                 |
| 36-41          | `NUMERIC`, `HEXCOLOR` (3 to 8 digits), `RGBACOLOR`, `RGBCOLOR`, `IDENTIFIER`, `STRING` (quotes kept)                                                                                                        |
| 43, 44         | `,` and `:`                                                                                                                                                                                                 |
| 24             | End of the text                                                                                                                                                                                             |

Any other character is a lexical error.

## 4. Grammar and DB calls

The grammar is a recursive descent with one token of lookahead. The DB calls run where Jison's reductions run: a `set` or `union` turns the indent mode on after its last token has been read, before the newline that ends it is consumed, so the next line is lexed with the mode on. Grammar actions:

| Statement                                | DB call                                                                                                    |
| ---------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `title …`                                | `setDiagramTitle`, with the text after the keyword and its blank (`substr(6)`)                             |
| `set id [label] [: n]`                   | `addSubsetData([id], label, n)`; indent mode on                                                            |
| `union id, id… [label] [: n]`            | arity check (two or more), `validateUnionIdentifiers`, `addSubsetData`; indent mode on                     |
| `text ids id [label]`, `text ids number` | `addTextData(ids, id, label)`; a number takes no label (V13)                                               |
| `INDENT_TEXT id [label]`                 | `addTextData(currentSets, id, label)`; an error when no `set` or `union` came before (`text requires set`) |
| `style ids field, field…`                | `addStyleData(ids, fields)`, where a value is a string or one or more tokens joined by one space           |

## 5. DB state and the model

- **Title**: sanitised by the DOMPurify subset (`sanitize.rs`). The last `title` wins, even when it sanitises to nothing; a front matter title is set first (V19).
- **Subsets**: the ids are normalised (trimmed, one pair of quotes removed) and sorted by UTF-16 code units, as `Array.prototype.sort` sorts strings (V12). The size is the statement's, or `10 / n²` for the `n` identifiers as written (V8). A single id becomes a known set.
- **Text nodes**: the sets are normalised and sorted (V7); the id and the label are normalised, and the label is `""` when absent.
- **Styles**: the targets are normalised and sorted; the properties are ordered pairs with the object semantics of V14.
- **Texts are the DB's**: markup is raw, and an entity placeholder (`ﬂ°name¶ß`, from `#name;`) stays in the text (V17).

Set order. The subsets stay in declaration order, which is what the DB holds and what the layout receives. The DB has no object keyed by set id, so `Object.keys` does not apply to it: the order arises in the library's keyed objects (the circles, NOTES L6), and the layout port applies it with `ferrigura_model::js_object::JsObject`. The model does not reorder the subsets (Q4). A union's ids are sorted by `.sort()`, not by `Object.keys`, so `union "10","2"` stays `["10","2"]`.

## 6. Canonical model

`VennDiagram` with `version` 1 and camelCase keys: `title`, `subsets` (`sets`, `size`, `label`), `textNodes` (`sets`, `id`, `label`) and `styles` (`targets`, `styles` as `[key, value]` pairs). A label is `""` when absent. A size that is not finite is the sentinel string (`"Infinity"` for a digit run that overflows a double); a negative size is a number (V10, Q7). The layout, the geometry and the configuration are not in the model (NOTES section 4 leaves them out).

## 7. Quirks reproduced

| Id  | Quirk                                                                                                                                                                                                                                                 | Where                               |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------- |
| V1  | Indent mode: a `set` or `union` turns it on; a line that begins with a non-blank character turns it off; a comment line or a `x%%` line does not. So a `style` or `text` line at column 0 ends the indented texts, and `%%{` at a line start does too | rules 8, 10-19, 22                  |
| V2  | A comment glued to a token takes the character before `%%`: `set A%% c` is an error, `set A %% c` is not                                                                                                                                              | rule 8                              |
| V3  | Statements need no separator: `set A set B` and `venn-beta set A` are two statements                                                                                                                                                                  | grammar                             |
| V4  | A title stops at `#` or `;` (an error after it), keeps its trailing blanks, and may contain `%%`                                                                                                                                                      | rule 26                             |
| V5  | A union needs two identifiers, each a `set` given before it; the error names every unknown id                                                                                                                                                         | `union`, `validateUnionIdentifiers` |
| V6  | Duplicate statements are kept, and a union may repeat an id (`union A,A`)                                                                                                                                                                             | `addSubsetData`                     |
| V7  | A text node's sets are sorted, so `text B,A` is `[A, B]`                                                                                                                                                                                              | `addTextData`                       |
| V8  | The default size is `10 / n²` for the identifiers as written: 10 for a set, 2.5 for a pair                                                                                                                                                            | `addSubsetData`                     |
| V9  | Quotes are removed after trimming, so `["  x  "]` is `x`, and `[ ]` and `[""]` are empty labels                                                                                                                                                       | `normalizeText`                     |
| V10 | A negative or zero size is kept (Q7). The renderer draws `NaN` for it, as the reference does, and the renderer task reproduces that                                                                                                                   | `addSubsetData`                     |
| V11 | The keywords need a word boundary after them (Jison's `\b`): `setA` is an identifier, and so are `settings`, `textual` and `unionA`                                                                                                                   | rules 27-31                         |
| V12 | Set ids inside a subset are sorted as strings (`"10"` before `"2"`), not in `Object.keys` order                                                                                                                                                       | `addSubsetData`                     |
| V13 | A number in a `text` statement takes no label; the grammar has no such rule                                                                                                                                                                           | grammar, `text`                     |
| V14 | Styles are an object: a repeated key keeps its first place and its last value, and `__proto__` is dropped                                                                                                                                             | `addStyleData`                      |
| V15 | A character outside the BMP before `%%` is an error: JavaScript's `[^\}]` matches one code unit                                                                                                                                                       | rule 8                              |
| V16 | The parser reads the keywords without regard to case (`VENN-BETA` parses), though the detector does not take it; `venn-betax` is an error                                                                                                             | rule 27, `vennDetector.ts`          |
| V17 | Labels and text ids are raw: markup stays, and an entity placeholder (`#quot;` is `ﬂ°quot¶ß`) stays                                                                                                                                                   | `normalizeText`                     |
| V18 | The title is sanitised: `<b>x</b> & y` is `<b>x</b> &amp; y`, and a `<script>` goes with its content                                                                                                                                                  | `setDiagramTitle`                   |
| V19 | Front matter title first; a grammar `title` replaces it, even when the grammar's sanitises to nothing                                                                                                                                                 | `Diagram.fromText`                  |
| V20 | A bare `venn-beta`, with no statements, is accepted; the renderer's error (Q8) is the renderer's                                                                                                                                                      | grammar                             |
| V21 | A non-breaking space is not a blank for the lexer: it begins no statement                                                                                                                                                                             | rule 19                             |

## 8. Specs

| Reference spec                                  | Rust file                                             | Tests           | Notes                                                                                             |
| ----------------------------------------------- | ----------------------------------------------------- | --------------- | ------------------------------------------------------------------------------------------------- |
| `diagrams/venn/parser/venn.spec.ts` (17)        | `crates/ferrigura-syntax/tests/venn_spec.rs`          | 17, all passing | The DB has no public surface: each test parses and checks the model; the texts are the spec's own |
| The quirks above (probed through the reference) | `crates/ferrigura-syntax/tests/venn_quirks_spec.rs`   | 36, all passing | One or more tests per quirk, with the probe's input                                               |
| `diagrams/venn/vennRenderer.spec.ts` (12)       | `crates/ferrigura-syntax/tests/venn_renderer_spec.rs` | 12, ignored     | The renderer and `ensurePairwiseSubsets` (the layout data), the renderer stage's                  |
| `diagrams/venn/vennPalette.spec.ts` (7 titles)  | `crates/ferrigura-syntax/tests/venn_renderer_spec.rs` | 7, ignored      | The palette is the renderer's; the spec expands over the 11 themes, giving 18 runs                |

The ignored tests panic with their reason if they are run with `--ignored`.

## 9. Differential

`tools/port/export-venn.ts` runs the reference's Jison parser with `parser.yy = vennDB` after `preprocessDiagram`, `encodeEntities` and the line break, as `Diagram.fromText` does. It writes the DB state as the model, with a size that is not finite as its sentinel string (`JSON.stringify` would write `NaN` as `null`). The explicit inputs are in `tools/port/cases.ts`, each probe of this document and each quirk test, and the collected inputs are the fixtures and the spec literals. A rejection on both sides is agreement, and the error texts are ferrigura's own.

`undetectedCases.venn` lists five inputs that the reference's detector does not take (`VENN-BETA`, an empty text, a bare newline, a title before the header), so the configuration run expects no scope for them.

## 10. Open questions for the coordinator

1. **Q4, the set order.** The model keeps the declaration order, and the layout applies the `Object.keys` order in its keyed objects. Q4 asks that the layout follow `Object.keys`; the DB has no keyed object, so the parser cannot give that order by itself. Confirm that the layout port applies it.
2. **Q1, Q2, Q3, Q6, Q9 to Q12** are the renderer's and the layout's (text nodes as `foreignObject`, the padding of 8 and 15, the randomised branch from four sets, the hand-drawn look, the configuration layers, the crate). They are not decided here.
3. **Q7** is decided for the parser: negative and zero sizes are kept (V10). The renderer must reproduce the `NaN` output, or reject, as the coordinator decides there.
