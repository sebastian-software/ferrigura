# P2 treemap parser: plan

Status: the parser, the DB and the model are ported, with the exporter, the specs and the differential (sections 7 and 8). Reference: the TypeScript reference of this branch (`packages/mermaid/src`, `packages/parser/src/language/treemap/`). The research and the probes behind it are in [NOTES.md](./NOTES.md); this page says what the port does and which quirks it keeps. The renderer stage (`renderer.ts`, the layout, the value formatting, the style sheet) is not in this page.

## 1. Sources

| File                                                                                           | Used for                                                                                                                    |
| ---------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| `packages/parser/src/language/treemap/treemap.langium`                                         | Terminals (17-89) and rules (44-87), in the order Chevrotain takes them                                                     |
| `packages/parser/src/language/common/valueConverter.ts`, `matcher.ts`                          | The title and accessibility converters (`runCommonConverter`, the regexes of `matcher.ts`)                                  |
| `packages/parser/src/language/treemap/valueConverter.ts`                                       | `NUMBER2` (`parseFloat` after the commas), `STRING2` (quotes removed), `INDENTATION` (length), `classDef` (`classDefRegex`) |
| `packages/mermaid/src/diagrams/treemap/parser.ts`                                              | `populate`: the common DB, the classes, the items, `buildHierarchy`, the nodes                                              |
| `packages/mermaid/src/diagrams/treemap/db.ts`                                                  | `addClass` (50-70), `addNode` (37-44), `getRoot` (46-48), `clear` (80-87)                                                   |
| `packages/mermaid/src/diagrams/treemap/utils.ts`                                               | `buildHierarchy` (8-64)                                                                                                     |
| `packages/mermaid/src/diagrams/common/commonDb.ts`, `populateCommonDb.ts`                      | The header setters (`sanitizeText`, then the leading-blank rule) and the truthy checks of `populateCommonDb`                |
| `packages/mermaid/src/diagrams/common/common.ts`, `rendering-util/.../handDrawnShapeStyles.ts` | `sanitizeText` (DOMPurify with `FORBID_TAGS: ['style']`), `isLabelStyle` (53-73)                                            |
| `packages/mermaid/src/Diagram.ts` (55-77), `preprocess.ts`, `utils.ts` (874-893)               | The order of `clear`, the front matter title, `encodeEntities` and the `\n`                                                 |

The Rust code: `crates/ferrigura-syntax/src/treemap.rs` (entry point), `treemap/scanner.rs` (lexer), `treemap/grammar.rs` (rules and the syntax tree), `treemap/db.rs` (DB and `buildHierarchy`). The model is `crates/ferrigura-model/src/treemap.rs`, the exporter `tools/port/export-treemap.ts`.

## 2. Text before the lexer

`prepare` normalises the line breaks and the attribute quotes and removes the front matter (shared). `remove_directives` and `cleanup_comments` run as in `preprocessDiagram`, then `encode_entities` and one `\n` (`Diagram.ts` 77, shared). The treemap parser has no wrapper, so the Langium parser reads that text. The front matter's `title` is read and set on the DB before the parse (`Diagram.ts`, after `clear`).

## 3. Scanner

Chevrotain takes the first terminal that matches at a position, in the order of the grammar's terminals, not the longest; the probes of section 6 confirm the order where it matters (`:::treemap`, `:::title` and `:::classDef x` are rejected, so the keywords come before `ID2`). The order is that of `treemap.langium`: the accessibility and title terminals first, since they begin with the blanks before them (so a blank run in front of `accTitle:` or `title` is part of the token), then the keyword, `classDef`, `:::`, `:`, `,`, the blanks, the comment and the line break (both hidden), `ID2`, `NUMBER2` and `STRING2`.

`BOOLEAN` is not in the lexer: no rule uses it, and Langium's lexer holds only the reachable terminals. `"A":::true` is therefore a class `true`, as the probes show (quirk 7 below).

Terminals that matter for the port:

| Terminal          | Port (`scanner.rs`)                                                                                                                                                                      |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ACC_DESCR`       | `acc_descr_at`: blanks, `accDescr`, then `:` and the rest of the line up to a `%%`, or `\s*{…}` over lines                                                                               |
| `ACC_TITLE`       | `acc_title_at`: blanks, `accTitle`, `[\t ]*:` and the rest (`colon_form`, shared with git graph)                                                                                         |
| `TITLE`           | `title_at`: blanks, `title`, then a blank and the rest of the line up to a `%%`; without a blank, `title` alone                                                                          |
| `TREEMAP_KEYWORD` | `treemap-beta`, then `treemap`; no word boundary (`treemapX` is the keyword and then an identifier)                                                                                      |
| `CLASS_DEF`       | `class_def_at`: `classDef`, `\s+`, a name of two or more characters, then `\s+` and the text up to `;` or a line break, and an optional `;`. The `\s` of the pattern crosses line breaks |
| `INDENTATION`     | `[ \t]+`, counted in characters                                                                                                                                                          |
| `ID2`             | `[a-zA-Z_][a-zA-Z0-9_]*`                                                                                                                                                                 |
| `NUMBER2`         | `[0-9_.,]+`, converted by `parse_number`: commas removed, then the longest decimal prefix (`parseFloat`)                                                                                 |
| `STRING2`         | `"…"` or `'…'`, which may span lines; the quotes are removed and nothing else                                                                                                            |

A character that no terminal takes is a lexer error, as are the unclosed quotes.

## 4. Grammar and DB calls

```text
Treemap      : TREEMAP_KEYWORD ( ACC_DESCR | ACC_TITLE | TITLE | TreemapRow )*
TreemapRow   : INDENTATION? ( Leaf | Section | CLASS_DEF )
Leaf         : STRING2 INDENTATION? ( ':' | ',' ) INDENTATION? NUMBER2 ( ':::' ID2 )?
Section      : STRING2 ( ':::' ID2 )?
```

The parser (`grammar.rs`) is recursive descent over the tokens. After a name it reads a leaf when a separator follows, with at most one blank before it (`peek_at(1)`), and a section otherwise. The header texts take the last value given (the grammar assigns a non-array property, so the last one stays).

The DB calls follow `parser.ts`'s `populate` (`db.rs::build`): the header texts first (`populateCommonDb`), then every `classDef` row in order (`addClass`), then the items, each with the styles of its class as they stand after all the class definitions, then `buildHierarchy`, then the nodes.

## 5. Model

- `version`, `title`, `accTitle`, `accDescr`, as the DB holds them. `title` is the grammar's title when it is non-empty, else the front matter's, both sanitised. `accTitle` loses its leading blanks, and `accDescr` loses the blanks after each line break (the two setters' rules).
- `classes`: the classes in the order of their first definition, each with `styles` and `textStyles`, which are the DB's lists. A class defined twice keeps its place and appends its styles.
- `tree`: the root of `getRoot()` (empty name, the top-level nodes as children). A section has `children` (empty when it has no rows), a leaf has `value` and no `children`. `classSelector` is present when the row gives one, and `cssCompiledStyles` when the DB defines that class.
- A non-finite `value` is the sentinel string (`"NaN"`, `"Infinity"`, `"-Infinity"`), as `value.rs` writes it.
- Names are the DB's names: quoted text without the quotes, and the entity placeholders that `encode_entities` writes stay (`ﬂ°quot¶ß`), since the DB keeps them.
- Left out: the indentation levels and the flat node list (the tree holds the same information), the DB's `levels` map, and the configuration, which the configuration port owns (the `treemap` section of `defaultConfig.ts` is compared by `pnpm port:diff config`).

## 6. Quirks reproduced

Each quirk is probed on the reference (`NOTES.md`, sections 2.5 and 2.8). The explicit inputs of `tools/port/cases.ts` cover them, and `tests/treemap_grammar.rs` asserts the ones with a direct result.

| #   | Quirk                                                                                                                                                                                                                                                            | Where                                         |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| 1   | A name must be quoted; `A: 1` is rejected. A name may be empty, single-quoted, or span lines. Names have no escapes                                                                                                                                              | `grammar.rs`, `scanner.rs` (`string_len`)     |
| 2   | Values: commas are removed before `parseFloat`, so `1,2,3` is 123; `1_000` is 1; `1..2` is 1; `.5` is 0.5; `._.` and `.` are NaN; `_` alone and `1e3` are rejected; `1,` takes its comma                                                                         | `parse_number`, `ID2` before `NUMBER2`        |
| 3   | A blank run that ends a row is the indentation of the next row, so `"A": 1 ⏎ "B": 2` puts `B` one level deeper                                                                                                                                                   | `grammar.rs` (`row`), `treemap_grammar.rs`    |
| 4   | Two indentation tokens in a row are rejected (`"A": 1  ⏎  "B": 2`), and a trailing blank at the end of the text is rejected                                                                                                                                      | `grammar.rs` (`row`)                          |
| 5   | Tabs count one character each. Empty lines are hidden, and a run of blanks before a row is that row's indentation. A comment line takes its leading blanks with it (`cleanup_comments`); a comment after a value leaves its blanks as a trailing blank (quirk 3) | `scanner.rs` (`blanks_len`), `treemap.rs`     |
| 6   | A `classDef` name needs two characters (`c` and `a-b` are rejected; `_x` is fine). A bare `classDef` followed by a line takes that line as its style text (`\s` crosses the line break), so `"A"` disappears. A bare `classDef` at the end gives the style `''`  | `scanner.rs` (`class_def_at`)                 |
| 7   | Class selectors are identifiers. `true` and `false` are plain names (`BOOLEAN` is not in the lexer). `treemap`, `title` and a `classDef` with a name after `:::` are rejected. A blank before `:::` is rejected                                                  | `scanner.rs`, `grammar.rs` (`class_selector`) |
| 8   | A style text ends at `;`, and its commas split the styles. `\,` keeps a comma. A repeated `classDef` appends to the class. A class defined after its use styles the item (the classes are added first)                                                           | `db.rs` (`add_class`, `build`)                |
| 9   | Only the exact pieces `color`, `font-size`, ... are text styles; `color:#fff` is not                                                                                                                                                                             | `db.rs` (`LABEL_STYLES`)                      |
| 10  | `title` alone is the empty title; a title may sit inside a row (`"A" title T`), or follow the keyword with no blank (`title"A": 1` is a title `""` and a leaf); the last title wins; an empty grammar title keeps the front matter's                             | `scanner.rs` (`title_at`), `db.rs::build`     |
| 11  | Header texts are sanitised by the DB's setters; names and values are not (`<b>C</b>` stays as written)                                                                                                                                                           | `db.rs`, `sanitize.rs`                        |
| 12  | Entities: `#name;` is an entity placeholder before the parse, and the names keep it                                                                                                                                                                              | `treemap.rs` (`encode_entities`)              |
| 13  | Two roots are accepted: the multiple-root validator is registered but not run by `parse()`                                                                                                                                                                       | `grammar.rs`                                  |
| 14  | `treemapX` is rejected (the keyword is followed by an identifier); `treemap "A": 1` on one line is accepted                                                                                                                                                      | `scanner.rs` (`keyword_len`)                  |
| 15  | A digit run that overflows is `Infinity`; `1e400` is rejected                                                                                                                                                                                                    | `parse_number`                                |

## 7. Specs and the differential

- `tests/treemap_spec.rs`: the 17 cases of `packages/parser/tests/treemap.test.ts`. The two `class myNode myClass` cases are asserted as rejections: the reference reports them as parser errors and its spec does not check them.
- `tests/treemap_db_spec.rs`: the 4 cases of `utils.test.ts` (`buildHierarchy`), written as the texts that give the same items, and the class and title rules of the DB.
- `tests/treemap_grammar.rs`: the quirks of section 6 with a direct result.
- `tools/port/export-treemap.ts` runs the reference's parser and DB on every input (`parser.ts`, `db.ts`, the `preprocessDiagram` and `encodeEntities` of `Diagram.fromText`, with a window for DOMPurify). `cases.ts` holds the probed inputs of section 6. `pnpm port:diff treemap` reports 211 inputs, 161 accepted by the reference, and no difference.
- `exceptions.ts`: none. The reference has no garbage output for this type; `NaN` is the sentinel of section 5.
- `pnpm port:diff config` compares the `treemap` section now that the type is ported (`diff-config.ts`).

## 8. Open questions

1. **Error text.** The error messages are ferrigura's own, with the position (decision 0002). The reference's Chevrotain messages are not reproduced.
2. **Multiple roots** are kept as the reference parses them (quirk 13). The validator stays out of the port.
3. **`$xyz` and the value formats** (`valueFormat`) are the renderer's (`NOTES.md` 6.5).
4. **The three keys the renderer does not read** (`borderWidth`, `valueFontSize`, `labelFontSize`) are left to the configuration, as the reference defines them; the model holds no configuration, and `pnpm port:diff config` passes with them as they are.
