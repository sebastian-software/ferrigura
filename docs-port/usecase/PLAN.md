# Use case (`usecase-beta`): the parser plan

The parser, DB, AST and differential of the use case diagram, ported from the Chevrotain parser of the reference. Research: [NOTES.md](./NOTES.md) (sections 2 to 5 and 8 are the parser's); the parser rules are [P2-PARSERS.md](../P2-PARSERS.md) with `P3-PLAN.md` as the plan. The renderer and the layout data (`getData`, its shapes and markers) are not in this port.

## Layout of the port

| File (`crates/ferrigura-syntax/src/use_case/`) | Reference                                       | What it does                                                                                                              |
| ---------------------------------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `scanner.rs`                                   | `parser/usecase.tokens.ts`, `usecase.lexer.ts`  | The three modes, the token order, the `longer_alt`, the custom matchers                                                   |
| `grammar.rs`                                   | `parser/usecase.parser.ts`                      | The 47 rules as recursive descent, with the GATEs and `LA(0)`                                                             |
| `visitor.rs`                                   | `parser/usecase.visitor.ts`                     | The syntax tree into builder drafts and AST statements, the spans                                                         |
| `draft.rs`                                     | `usecaseModelBuilder.ts` 29-150                 | The drafts the visitor hands over                                                                                         |
| `builder.rs`                                   | `parser/usecaseModelBuilder.ts`                 | `finalize`, its order of steps and checks, the header sanitising                                                          |
| `ast.rs`                                       | `usecaseAst.ts`                                 | The graph AST                                                                                                             |
| `json.rs`                                      | `parser/usecaseJson.ts`, `usecaseDb.ts` 149-211 | `JSON.parse` validity and value, the property order, `String(number)`, and the JSON table rows of `getData` (`json_rows`) |

The model is `ferrigura-model/src/use_case.rs`: the DB state after the parse, in camelCase, with the AST. The specs are `crates/ferrigura-syntax/tests/use_case_*.rs`.

## Decisions

1. **The AST is part of the model** (open question 1 of the notes, decided by Sebastian as the research recommended). The three specs assert it, and the differential compares it with the reference's `getAST()`, including the spans in UTF-16 code units.
2. **Error texts are ours**; the differential compares acceptance only, and the specs assert that a text fails and at which position (open question 2).
3. **The 24 documentation examples** (`docs/syntax/usecase.md`, the `mermaid-example` fences) parse in `use_case_docs_spec.rs` (open question 8).
4. **Preprocessing** is the shared one: `prepare` (front matter), `remove_directives`, `cleanup_comments`, `encode_entities`, a line break, as `Diagram.fromText` does.
5. **Lookahead.** Each OPTION, OR and MANY decision is made with the lookahead of the reference: the `statement` order and its GATEs, the OPTION guards of `relationTail` and `entityName`, the metadata separators. Where Chevrotain chooses by its maximum lookahead, the port uses the same tokens; the differential on the probes is the check.

## Quirks reproduced

These are the notes' section 2.5 quirks, each probed through the reference and tested:

- Keywords are not names: `actor`, `end`, `note`, `for`, `class`, `classDef`, `style`, `json`, `direction`, `systemBoundary`, `include`, `extend`, `true`, `false`, `TD` … `RL` and `usecase-beta` are not identifiers. Quoted names are the way out.
- `x--` and `o--` are operators wherever they start a token. `--o`, `--x`, `--` and `-->` are matched longest-first only where the list says so; the rest is first-match.
- The label of `<--` needs the exact image `<--`, and of `--` the exact image `--` (`LA(0)`). `<---` and `---` cannot be labelled.
- The closer of a label must match its family (`<--` and `o--`/`x--`/`--` close with the markerless operator, `--` also with `-->`, `--o`, `--x`).
- `metadataAssignmentStatement` shadows the entity statement when `@{` follows the target, so a relation after such a line is an error.
- Metadata values are identifiers, quoted strings, `true` or `false`. A number is an error; `1mg` is an identifier.
- Only the first lexer error and the first parser error are reported.
- Label text is the source slice, with its inner spacing. `LABEL_SYMBOL` takes any run of non-printable ASCII, so a non-ASCII letter in a label is label text.
- The dead alternative of `CssIdentifier` (`--…`) is dropped: `--` is matched first. `foo-bar` is a CSS identifier.
- `generateId` replaces every code unit that is not a word character with `_`, so `"Café"` is `Caf_`.
- **The front matter title never reaches the model.** `Diagram.fromText` sets it, then the parser clears the DB, and `clear` resets the title (`commonDb.ts` 10-14). The model's `title` is `""`, as the reference's `getDiagramTitle()` is.
- The header texts are sanitised twice, as the reference does (the visitor's setter, then `commit`): `accTitle` loses its leading blanks, `accDescr` its blanks after each line break.
- Labels are raw in the model. The reference sanitises labels only in `getData`, and the rows of a JSON table are sanitised cell by cell (`json_rows` in `json.rs`).
- JSON: `JSON.parse` decides validity; a duplicate key keeps its first position and its last value; `String(number)` gives JavaScript's text (`Infinity`, `-0` as `0`, `1e-7`, `12345678901234567000`).

## Not reproduced

Nothing in the model is left out on purpose except the symbols and counters of the DB, the configuration and the render input. The rendering is not in this port.

## Results

- **Differential**: `pnpm port:diff usecase` gives 291 inputs, 167 accepted by the reference, 0 differences and no named exception. The inputs are the fixtures, the spec and documentation literals, the 105 probes of the research (`cases.ts`), and the `usecase-beta` inputs of the other tests.
- **`pnpm port:diff config`**: 0 differences, the usecase group with 291 of 291 inputs accepted.
- **Specs** (`crates/ferrigura-syntax/tests/use_case_*.rs`): the publication spec (`usecase.parser.spec.ts`) 41 tests, 1 ignored; the main spec (`usecase.spec.ts`) 33 passing and 10 ignored; the DB spec 3 and 1 ignored; the parser spec 73 and 2 ignored; the lexer spec 13; the JSON spec 39; the documentation spec 2 (the 24 examples); the renderer and colour specs 3 ignored.
- **Ignored, with the reasons**: the render input (`getData`) and the stylesheet, which belong to the renderer stage; the CST rule names and the Chevrotain token positions, which are the reference's internals; and the grammar-level tests that the public `parse` cannot isolate, because it also runs the model builder (an undeclared metadata target is rejected there, as the reference's builder does).
- **Found while porting**: the front matter title never reaches the model (the parser clears the DB); the AST reads the accessibility texts before the commit; the reference's AST drops the key `__proto__`; a non-finite number in a JSON table is a sentinel string in the model and `null` in the AST.
