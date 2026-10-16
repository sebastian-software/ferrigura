# M6 ER parser: plan

Status: parser, DB and model written from the reference (`erDiagram.jison`, `erDb.ts`, `Diagram.ts`, `commonDb.ts`) at the base of this branch. Research and the probes behind the claims are in [NOTES.md](./NOTES.md). Every quirk below is reproduced on purpose; the differential run (`pnpm port:diff er`) is the check.

## 1. Scope

- In: the lexer and grammar of `packages/mermaid/src/diagrams/er/parser/erDiagram.jison`, the DB `erDb.ts` (the parts that decide the model), the header setters of `common/commonDb.ts`, and the text pipeline of `Diagram.fromText`.
- Out (M6 renderer, with the specs marked `#[ignore]`): `getData`, `erBox`, markers, styles, the ELK layout, `config.er`.
- The model is the DB state after `parser.parse`, in the canonical form of [canonical-model.md](../canonical-model.md) (section "ER").

## 2. Pipeline

`Diagram.fromText` does, in order: front matter (its `title` goes to `setDiagramTitle`), directive removal, `cleanupComments` (which also trims the start), `encodeEntities`, and a line break. Only then does the parser see the text. `er::parse` runs the same steps; `er::parse_grammar` (the spec entry, as `erDiagram.parser.parse` is called in the specs) runs the lexer, the parser and the DB on the text it is given.

The first token must be `erDiagram`. The detector is case-sensitive (`^\s*erDiagram`, `erDetector.ts` 10), so `erdiagram` on the first line never reaches the parser, although the lexer accepts it (NOTES L16).

## 3. Lexer

Jison with `%options case-insensitive`: every literal and every regular expression is case-insensitive (ASCII letters only, as in JavaScript without the `u` flag). Rules are tried in the order written, and the first match wins (not the longest). Exclusive start conditions (`%x`): a rule without a condition applies in `INITIAL` only.

States: `INITIAL`, `acc_title` (after `accTitle:`), `acc_descr` (after `accDescr:`), `acc_descr_multiline` (after `accDescr {`), `block` (after `{`), `block_bq` (inside backticks in a block), `style` (after `style`, `classDef`). The state stack is a `StateStack` (`scan.rs`): `begin` pushes, `popState` pops, and the base state stays.

A keyword literal gets Jison's implicit `\b` when it ends in a word character. The `\b` is checked after the match only: the lexer matches the rest of the input, so the leading `\b` of `\b(PK|FK|UK)\b` always holds.

End of input follows Jison's `next()`: the first call with no input left sets `done`, then the rules run once more on the empty rest. In `INITIAL` the rule `<<EOF>>` gives the token `EOF` (the grammar's string terminal). In the other states an empty-matching value rule may fire (`acc_title`, `acc_descr`, `acc_descr_multiline`); after `done`, the lexer returns Jison's end marker. A parse that reaches the end marker without an `EOF` token fails. So a value state open at the end of input fails, and a document that ends in `INITIAL` gives `EOF` and then the end marker (NOTES 2.6, probed).

| #   | Quirk                                                                                                                                                                                                                                                                                                        | Rule (erDiagram.jison line) | Reproduced as                                                                                                                                  |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- | --- | ---------------------------------- | ------- | ------------------------------------------------------ |
| L1  | `.*direction\s+WORD[^\n]*` takes the rest of the line from the current token start, so the rest of the line is lost (`A --> B : x direction TB` gives only the direction). The `\s+` may cross a line break. Rules are tried in the order TB, BT, RL, LR, not by position. Dots do not cross `\r`, ` `, ` `. | 19–22                       | `direction_word(rest)`: the last occurrence of `direction` on the line whose word follows, token runs to the next `\n`.                        |
| L2  | Keywords need a word boundary after them, so `endpoint` and `classes` are names. `end\b\s*` also eats the blanks and line breaks that follow.                                                                                                                                                                | 48–57                       | `word_len` plus the trailing blank run for `end`.                                                                                              |
| L3  | `to` and `optionally to` are keywords: a role spelled `to` unquoted fails.                                                                                                                                                                                                                                   | 87–88                       | keyword table.                                                                                                                                 |
| L4  | `1` is ONLY*ONE only before blank and a letter, quote or `*`, before blank and a digit, or before `--`, `..`, `.-`, `-.`. Otherwise ENTITY_ONE (an entity name). A `1` followed by a word character is NUM.                                                                                                  | 74–79                       | the four `1` rules in order, with the lookahead.                                                                                               |
| L5  | `u` is MD_PARENT only before `.`, `-` or `\|`. Otherwise it is a name.                                                                                                                                                                                                                                       | 84                          | lookahead.                                                                                                                                     |
| L6  | A quoted name is ENTITY_NAME `"[^"%\r\n\v\b\\]+"`. Any other quoted text is WORD `"[^"]*"` (may span lines), which is valid only as a role. `"Blo%rf"` as an entity fails.                                                                                                                                   | 25–26                       | two rules.                                                                                                                                     |
| L7  | Unquoted names are `([^\x00-\x7F]\|\w\|-\|\*\|\.)+`. Every non-ASCII character is a name character, NBSP included. `#`, `%`, `"`, `;`, `'` are not.                                                                                                                                                          | 93                          | the same class.                                                                                                                                |
| L8  | In a block, `\s+` skips line breaks, so attributes are not line-separated (`string name int id` is two attributes). An empty block `A {}` and `A {\n}` are valid.                                                                                                                                            | 34, 154–167                 | block rules.                                                                                                                                   |
| L9  | Generic types `([^\s]*)[~].*[~]([^\s]*)` are greedy to the last `~` on the line, spaces included. `map~K,V~ data "x~y"` fails.                                                                                                                                                                               | 36                          | matched by hand: the first `~` is the last one in the run of non-blanks that has another `~` after it, the second is the last `~` of the line. |
| L10 | Attribute keys `\b(PK\|FK\|UK)\b` match in any case and are kept as written (`pk` is a key). `PK` as an attribute name fails.                                                                                                                                                                                | 35                          | ATTRIBUTE_KEY, case-insensitive.                                                                                                               |
| L11 | An attribute comment `"…"` cannot contain `"` (no escapes). A comment must follow the keys: `… "c" PK` fails.                                                                                                                                                                                                | 41, 296–299                 | grammar order.                                                                                                                                 |
| L12 | `string?` is a valid type; the `?` is a single-character token that only follows a type. `string name?` fails.                                                                                                                                                                                               | 44, 289                     | literal `?` token.                                                                                                                             |
| L13 | A backtick name is `[^`]+`, which may span lines. An empty pair ` ` `` gives no token at all.                                                                                                                                                                                                                | 38–40                       | `block_bq` rules.                                                                                                                              |
| L14 | In a block, any other character is a literal token (`yytext[0]`), which only `,` and `?` can satisfy. `[`, `(`, `#`, `:` fail.                                                                                                                                                                               | 44                          | `Literal(char)` token.                                                                                                                         |
| L15 | In `style`, the rules are `[\n]+` (ends the state), `\s+` (blanks, which include line breaks when blanks come first), `:`, `,`, `#`, `;` and the style text `([^\x00-\x7F]\|\w\|-\|\*)+`. There is no catch-all, so `.`, `(`, `"` fail. `;` does not end the state.                                          | 49–53, 91–92                | style rules. The `\s+` swallow is the case `style A color:red ` + newline + next line: the next line joins the style.                          |
| L16 | `accTitle\s*":"\s*` and `accDescr\s*":"\s*` skip line breaks after the colon; the value is the rest of the line (which may be the next line). An empty value is the empty string.                                                                                                                            | 12–15                       | `delimited` form, case-insensitive.                                                                                                            |
| L17 | `accDescr\s*"{"\s*` opens a block ending at the first `}`; the block value is `[^}]*` and may contain line breaks. The grammar trims it.                                                                                                                                                                     | 16–18, 191                  | `acc_descr_multiline` state.                                                                                                                   |
| L18 | `title` is not a token. The grammar's `title` productions are dead.                                                                                                                                                                                                                                          | 188                         | not implemented.                                                                                                                               |
| L19 | A line may hold several statements: there is no separator rule (`A                                                                                                                                                                                                                                           |                             | --o{ B : x C                                                                                                                                   |     | --o{ D : y` is two relationships). | 114–118 | statements follow each other; a role ends a statement. |
| L20 | Any other character in `INITIAL` (a literal `.`) is a token that the grammar never accepts (`;`, `%`, `?`). U+2028 and U+2029 are names, as every non-ASCII character is (the `[^\x00-\x7F]` class): only the `.` rule would reject them, and it never sees them.                                            | 94                          | error.                                                                                                                                         |

## 4. Grammar

Recursive descent over the token stream. The alternatives of `statement` (lines 122–196) are these, with `E` a name (`entityName`: ENTITY_NAME without quotes, UNICODE_TEXT, NUM, DECIMAL_NUM, ENTITY_ONE), `R` a relationship `cardinality relType cardinality`, `S` the `:::` idList (`class names`), and `L` the `role` (WORD, ENTITY_NAME or UNICODE_TEXT, quotes removed):

| Form                                   | DB calls, in reduction order                                                                | Statement value                                                       |
| -------------------------------------- | ------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| `E [S] R E [S] : L`                    | `addEntity(E1)`, `addEntity(E2)`, `addRelationship`, `setClass(E1, S1)`, `setClass(E2, S2)` | `[E1, E2]`                                                            |
| `E { attrs }`, `E [S] { attrs }`       | `addEntity`, `addAttributes`, `setClass`                                                    | `[E]`                                                                 |
| `E { }`, `E [S] { }`                   | `addEntity`, `setClass`                                                                     | `[E]`                                                                 |
| `E [S]`, `E S`                         | `addEntity`, `setClass` (if `S`)                                                            | `[E]`                                                                 |
| `E [A] [S] [{ attrs }]`                | `addEntity(E, A)`, `addAttributes`, `setClass` (alias forms, the alias set if none)         | `E` (Jison's default `$$ = $1` for the forms without an action value) |
| `accTitle: text`, `accDescr: text`     | `setAccTitle(trim)`, `setAccDescription(trim)`                                              | the trimmed text                                                      |
| `accDescr { text }`                    | `setAccDescription(trim)`                                                                   | the trimmed text                                                      |
| `direction DIR`                        | top level: `setDirection`; inside a subgraph: none                                          | none at top level; the `dir` object inside                            |
| `classDef ids styles SEP`              | `addClass(ids, styles)`                                                                     | `classDef` as written                                                 |
| `class ids ids`                        | `setClass(ids1, ids2)`                                                                      | `class` as written                                                    |
| `style ids styles SEP`                 | `addCssStyles(ids, styles)`                                                                 | `style` as written                                                    |
| `subgraph ID [TITLE] SEP document end` | depth `+1` at the header; at `end`: depth `-1`, `addSubGraph(ID, members, TITLE)`           | the subgraph id                                                       |

Details the port keeps:

- The alias is set by `addEntity(name, alias)` only when the entity has none yet (`erDb.ts` 75–78), and `alias` is `''` otherwise.
- `classDef` and `style` take `styles` as concatenations of their tokens without blanks: `fill:#f9f, stroke: red` gives `fill:#f9f` and `stroke:red`. The `classDef` ids are style-state words, so `classDef A,fill:red` gives the ids `A`, `fill` and the style `:red`.
- `class` ids are INITIAL names (UNICODE_TEXT only): `class A B C` is `class A B` followed by the entity `C`.
- `SEP` is NEWLINE or SEMI (style state), or `EOF` in `INITIAL` (after a subgraph header).
- `R` is `cardB` on the left and `cardA` on the right (grammar 312): `A }o--|| B` gives `cardA` `ONLY_ONE`, `cardB` `ZERO_OR_MORE`.
- A nested `subgraph` header adds the subgraph depth before its body, so a `direction` inside a subgraph (at any depth) goes to the member list, not to the diagram.

### 4.1 Members of a subgraph (phantoms)

`document` concatenates the value of each line. Values are not only entity names: the values of `classDef`, `class`, `style`, `accTitle`, `accDescr` and `direction` (as an object) enter the list too, and `addSubGraph` keeps the strings (`uniq`, `erDb.ts` 237–267):

```
subgraph G / A ||--o{ B : x / classDef foo fill:#f9f / end   -> members A, B, classDef
subgraph G / accTitle: T / A / end                              -> members T, A
subgraph G / A / subgraph H / B / end / end                     -> members A, H
```

`uniq` drops empty strings, duplicates (compared trimmed, the first kept) and the direction objects, which set `dir` (the last one wins). A member already in an earlier subgraph is dropped (`makeUniq`, 338–349). An entity named like a phantom (an entity `T` above) is drawn inside `G` by the renderer; the model keeps the phantom in `nodes`.

## 5. DB and model

The DB (`erDb.ts`) is the model. Port: `ErDb` in `er/db.rs`, with the DB's method names in snake case.

| Field                     | Source                                                            | Rule                                                                                                                                                               |
| ------------------------- | ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `title`                   | `setDiagramTitle` from the front matter                           | sanitised (`commonDb.ts` 28–30)                                                                                                                                    |
| `accTitle`, `accDescr`    | `setAccTitle`, `setAccDescription`, the last one wins             | sanitised, then `^\s+` removed (title) and `\n\s+` → `\n` (description) (`commonDb.ts` 16–24). The grammar trims first.                                            |
| `direction`               | `setDirection`                                                    | `TB` by default; the last top-level statement wins                                                                                                                 |
| `entities[]`              | `addEntity`, in creation order                                    | `id` = `entity-NAME-N`, `N` is the count at creation (`erDb.ts` 64)                                                                                                |
| `entities[].alias`        | `addEntity(name, alias)`                                          | `""` if none                                                                                                                                                       |
| `entities[].attributes`   | `addAttributes`                                                   | source order; `keys` as written (L10); `comment` `""` if none                                                                                                      |
| `entities[].classes`      | `setClass`, the `cssClasses` string without its leading `default` | the class names in order, duplicates kept                                                                                                                          |
| `entities[].styles`       | `addCssStyles`, the `cssStyles` list                              | in order                                                                                                                                                           |
| `relationships[]`         | `addRelationship`                                                 | `entityA`/`entityB` are the subgraph id if one of that id exists when the statement is reduced, else the entity id                                                 |
| `relationships[].relSpec` | the grammar's `relSpec`                                           | `cardA`, `cardB` and `relType` as the DB's strings (`MD_PARENT` included)                                                                                          |
| `classes[]`               | `addClass`                                                        | `id`, `styles`, `textStyles` (the styles with `/color/`, with `fill` replaced by `bgFill` once); kept in the model                                                 |
| `subgraphs[]`             | `addSubGraph`                                                     | `id` (trimmed), `title` (sanitised, trimmed), `nodes` (members, phantoms included, nested ids included), `classes`, `styles` (`cssStyles`), `dir` (`null` if none) |

Left out: `shape` (`erBox`), `labelType` (always `markdown` from the grammar), `look` (configuration), `cssCompiledStyles` and `colorIndex` (set by `getData`), `subCount` and `subgraphDepth` (parser state; `subCount` only names a subgraph with no id, which the grammar never gives).

A subgraph id and an entity name share one namespace: `G ||--o{ B` with a subgraph `G` adds an entity `G` (the grammar calls `addEntity` first) and the relationship uses the subgraph id (NOTES 2.5). `getData` is not ported (renderer).

## 6. Sanitisation

The DB sanitises the header texts, the subgraph title and the front-matter title (`sanitize.rs`, the DOMPurify subset). It does not sanitise entity names, aliases, attributes, roles, styles or class names. The exporter runs the real setters with the DOM that `reference.ts` provides (`provideDomForSanitising`), so the sanitised header texts are compared with the reference's.

## 7. Errors

A syntax error is a `ParseError` at the offset of the token that cannot continue (the end of the text for the end marker). No model is returned. The DB throws nothing. The reference's messages are not copied (ADR 0002): the differential compares only whether parsing succeeded.

## 8. Specs

| Reference spec                                                                                                                        | Rust file                             | Ported                                                                                                                                                                                                                                          |
| ------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `er/erDb.spec.js` (8)                                                                                                                 | `er_db_spec.rs`                       | 6 DB tests on `ErDb`; 2 `getData` tests ignored (render data)                                                                                                                                                                                   |
| `er/parser/erDiagram.spec.js` (110 `it`, one `it.each` of 3, one loop of 104 characters × 5 cases, plus the `it` over all characters) | `er_spec.rs`, `er_characters_spec.rs` | the parser tests with the reference's titles; the 520 cases of the character loop are generated (`er_characters_spec.rs`), each one a test with the name of its title; the skipped test (`it.skip`) and the two empty `TODO` bodies are ignored |
| `er/parser/subgraph.spec.js` (18)                                                                                                     | `er_subgraph_spec.rs`                 | all ported                                                                                                                                                                                                                                      |

Two of the reference's throw-checks are vacuous: the `%` and `\` tests read `name`, which is not in scope there, so the check throws whatever the parse does. The port asserts the failure those tests mean (`er_spec.rs`). The throw-checks of the generated cases pass when the parse fails or the entity exists, and the port keeps that shape (`er_characters_spec.rs`).

Test names are the titles in snake case, with `_2`, `_3` … for repeated names. Characters in a title become words: ASCII punctuation by name (`~` → `tilde`), other characters by code point (`u00a1`).

## 9. Differential

`tools/port/export-er.ts` runs the reference's `preprocess`, `encodeEntities` and the Jison parser with the real `ErDB`. `inputs.ts` collects the fixtures, the spec literals, the docs blocks, and the explicit cases of `cases.ts` (the quirks above). The exceptions list starts empty: a difference is a bug in the port unless the reference is shown to produce garbage.

## 10. Open questions

1. Header sanitisation: the flowchart leaves its header texts unsanitised (canonical-model.md). ER sanitises them, as its DB does; the flowchart's exporter replaces the header setters. Both are kept, and the difference is documented.
2. `textStyles` is in the model (not derived), because the DB holds it and the specs check it.
