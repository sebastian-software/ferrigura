# Railroad: the parser and the DB

The parser task of P3 for railroad. It ports the four syntaxes (`railroad-beta`, `railroad-ebnf-beta`, `railroad-abnf-beta`, `railroad-peg-beta`) with their hand-written parsers, one model and one DB. The research is in [NOTES.md](./NOTES.md) (sections 2 to 5 and 8), the parser rules in [P2-PARSERS.md](../P2-PARSERS.md), and the decisions of [P3-PLAN.md](../P3-PLAN.md) apply. The renderer (its layout, its stylesheet and the configuration section) is the renderer stage and is not in this task.

## 1. What is ported

| Part                                                                                                                                               | Where                                                                                          |
| -------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| The model: `RailroadDiagram`, `RailroadRule`, `RailroadExpr`                                                                                       | `crates/ferrigura-model/src/railroad.rs`                                                       |
| The scanner: the token tables of the four grammars                                                                                                 | `crates/ferrigura-syntax/src/railroad/scanner.rs`                                              |
| The four grammars, by recursive descent, with the reference's AST transforms                                                                       | `crates/ferrigura-syntax/src/railroad/grammar.rs`                                              |
| The DB: the setters, the sanitising, `addRule`                                                                                                     | `crates/ferrigura-syntax/src/railroad/db.rs`                                                   |
| The entry point: preparation, the choice of the syntax from the header                                                                             | `crates/ferrigura-syntax/src/railroad/mod.rs`                                                  |
| The exporter (`Diagram.fromText` through the reference)                                                                                            | `tools/port/export-railroad.ts`                                                                |
| The canonical arm of `railroad` (detection first, then the parser)                                                                                 | `crates/ferrigura-syntax/examples/canonical.rs`                                                |
| The specs: the parser specs of the four syntaxes (65 tests), the DB spec through `parse` (11), the quirks of section 5 (46)                        | `crates/ferrigura-syntax/tests/railroad_spec.rs`, `railroad_db_spec.rs`, `railroad_grammar.rs` |
| The renderer and styles specs as ignored tests (6 and 19), naming the renderer stage                                                               | `crates/ferrigura-render/tests/railroad_render_spec.rs`                                        |
| The inputs: the probes of the research (212), the edge cases of the port (79), the front matter and detection cases (11), and the collected inputs | `tools/port/cases.ts`                                                                          |

The shared code that the port reuses: `preprocess.rs` (`prepare`, `remove_directives`, `cleanup_comments`, `encode_entities`, `strip_front_matter`), `lexer.rs` (`directive_len` and `yaml_len`, the directive and YAML block rules of the other grammars), `common.rs` (`plain_text` and `descr_block_text`, the header converter), `sanitize.rs` (`sanitize_text`, the strict path of `sanitizeText`), `text.rs`, and `skip_hidden_prefix` of `lib.rs`, the same function `detect` uses.

The shared files changed: `crates/ferrigura-model/build.rs` (the sixteen railroad keys without a default, section 8) and `tools/port/diff-config.ts` (the railroad section compared on its inputs). Neither `lib.rs` nor the other shared helpers changed.

## 2. The scanner

The four Langium grammars lex with Chevrotain, which takes the first token type whose pattern matches at the position, in this order: the keywords of the grammar, then its terminals, then its hidden tokens. A keyword with a `LONGER_ALT` gives way to the alternative when the alternative matches more text. The scanner runs that order (`step`: `keyword`, `terminal`, `hidden`):

- **Header.** The header keyword carries the boundary `(?=%%)|(?!\S)`, so `railroad-betax` is an identifier, and `railroad-beta%%c` is the header. The header is case-sensitive; the detector is not (`syntax_of` picks the grammar from the text, in any case, and the grammar then needs the exact header).
- **IR words.** `nonterminal`, `zeroOrMore`, `oneOrMore`, `sequence`, `optional`, `terminal`, `special` and `choice` are keywords only when the identifier at the position is the word itself (the identifier is their `LONGER_ALT`). So `sequenceX` is a name, and `sequence` as a rule name is an error.
- **Terminals.** `TITLE`, `ACC_TITLE` and `ACC_DESCR` come before the identifier, so a name that starts with `title` splits (quirk 2). `ACC_DESCR` is the colon form, or the braces over any number of lines (`\s*{…}`), and otherwise the identifier `accDescr`.
- **Punctuation with a `LONGER_ALT`.** EBNF `(` gives way to the ISO comment `(*…*)`, `?` to the special sequence `?…?`, `-` to a closed YAML block. ABNF `;` gives way to the comment `;…` when a character follows it on the line (quirk 8).
- **Hidden.** White space (`[\t \r\n]`), a YAML block, a directive, a `%%` comment line, a block comment (IR and EBNF), a `#` comment (PEG). Lines end at `\n` or `\r` only, as the reference's `[^\n\r]`; a U+2028 stays in a comment or a title.
- **Errors.** A character that no rule matches is a lexer error at its position.

The ABNF grammar has its own terminals: the rule name `[A-Za-z][A-Za-z0-9-]*`, the string `"[^"]*"` (no escapes, may span lines), the numeric value `%[xXdDbB]` with its hex runs and ranges, and the repeat `[0-9]*\*[0-9]*` or a count `[0-9]+`, both kept as written.

## 3. The grammars and the AST

The four grammars are recursive descent over the tokens, and each builds the reference's AST directly (`transform*` in `railroad*Parser.ts`). The collapse rules, the postfix order and the bounds are the reference's:

- **IR**: `sequence(…)` and `choice(…)` collapse when they have one element; `optional`, `oneOrMore` (min 1, max `Infinity`) and `zeroOrMore` (min 0, max `Infinity`) take one expression; `terminal`, `nonterminal` and `special` take a string.
- **EBNF**: a choice of sequences; a sequence of terms with an optional comma before each term after the first (a comma with no term after it is an error); a term is a primary with its postfixes applied in their order: `?` optional, `*` and `{ … }` repetitions of 0 to `Infinity`, `+` a repetition of 1 to `Infinity`, and `- primary` a three-element sequence with the text `-` (quirk 5). `[ … ]` is an optional, `( … )` a group that collapses.
- **ABNF**: alternation with `/`, concatenation of one or more elements, an element with an optional repeat. The repeat's bounds follow `parseRepeat`: `a*b` with an empty `a` at 0 and an empty `b` at `Infinity`, and a count at both bounds. A repeat of `0*1` is an optional, and any other bounds make a repetition (min above max is kept).
- **PEG**: an ordered choice of sequences of prefixes; a prefix is `&` or `!` on a suffix, and that makes a special whose text is the operator and a label of the suffix: a quoted terminal (not escaped), a rule name, a special's text, or `(...)` for any other expression. A suffix is one of `?`, `*` and `+`, and a primary is a string, a name, a group or `.` (a special `.`).

## 4. The DB and what the model holds

The DB is one module for the four syntaxes (`railroadDb.ts`). `populateCommonDb` sets the accessibility descriptions, the accessibility title and the title, each only when the grammar's value is non-empty, and the rules follow in order.

- **Title.** The grammar's last `title` line, sanitised, or `""`. The last line wins even when it is empty (X-29), and an earlier title is lost.
- **Front matter.** Dropped. `Diagram.fromText` sets the front matter title and then the parser's own `db.clear()` removes it, in all four syntaxes (probed, Q8: the front matter `title: FM` gives `""` for `railroad`, `railroadEbnf`, `railroadAbnf` and `railroadPeg`, and a grammar title `G` gives `G`).
- **Accessibility texts.** `accTitle` loses its leading whitespace after sanitising; `accDescr` loses the whitespace after each line break (`\n\s+` becomes `\n`).
- **Rules.** Every rule of the text, in order, duplicates included (R-19). The name and each leaf are sanitised with `sanitize_text`, which is the strict path of the reference's `sanitizeText`: `<script>` is removed, `&` becomes `&amp;`, and the entity placeholders are kept (the renderer decodes them).
- **Left out.** `RailroadRule.comment` and `RepetitionNode.separator`: no grammar sets them.

## 5. Quirks reproduced

Each has its probe in `tests/railroad_grammar.rs` under the same id, and its input in `tools/port/cases.ts`.

1. **The header.** Case-sensitive, followed by a blank, a line end or `%%` (R-30, X-05, X-06). The detector is case-insensitive, so `RAILROAD-BETA` is detected and then rejected.
2. **A title eats the prefix of a word.** `titles = terminal("a") ;` is the rule `s` with the title `""` (R-05).
3. **Word keywords need the longer identifier.** `sequenceX` is a name; `sequence` as a rule name is an error (R-06, R-07, R-56); a capital is not a keyword (R-48).
4. **Header lines come first.** `title`, `accTitle` and `accDescr` after a rule are errors (R-47, X-17, X-22). The last line of each kind wins, even when it is empty (X-29, X-30).
5. **The EBNF exception is text.** `A - B` is the sequence `A`, `"-"`, `B` (E-11).
6. **The EBNF special sequence.** `?…?` needs a non-blank character between the marks and no `;`, is trimmed, and keeps a line break inside (E-13, E-14, E-28).
7. **Postfixes nest** in their order: `{ "a" }*` is a repetition of a repetition, `[ "a" ]?` an optional of an optional (E-23, E-35).
8. **ABNF `;` is a comment when text follows it on its line.** `r = "a" ; comment`, `r = "a" ; ` and `r = "a";x` are errors (A-15, A-16, A-18); `r = "a";` at the end of its line and a `;` on the next line are accepted (A-17, A-19); a comment line between rules is accepted (A-40). This is the Q2 decision.
9. **ABNF repeats keep their bounds, and the renderer keeps only min** (Q5). `*1` and `0*1` are optionals (A-10, A-11), `*0` is a repetition from 0 to 0 (A-37), `2*1` is accepted as a repetition from 2 to 1 (A-38).
10. **ABNF numeric values and quoted titles are not decoded.** `%x41-5A` is one terminal with its text (A-03), `%x41-` is an error (A-26), and a quoted ABNF title keeps its backslashes (A-32).
11. **PEG lookahead labels are lossy.** `&"a"*` and `!"x"?` are labelled `(...)` (P-14, P-15); a quoted label does not escape its quote (P-31).
12. **PEG prefixes and suffixes are single.** `&&"a"`, `! & "a"` and `"a"?*` are errors (P-10, P-11, P-12).
13. **String escapes.** IR, EBNF and PEG decode only `\n`, `\r` and `\t`; other escapes stand for their letter, so `\b`, `\f`, `\v` and `\0` are the letters (X-23). ABNF decodes nothing. A raw line break inside a string is accepted (R-59), and a backslash before one is an error.
14. **Every value is sanitised** (section 4): `<script>` removed, `&` written `&amp;`, placeholders kept (R-18, X-26, X-27, X-28).
15. **Hidden text.** Comments, directives and a closed YAML block are hidden anywhere between tokens; an unclosed YAML block is an error (R-37, R-38, R-25, R-27, R-28). Two rules on one line are accepted (R-57).
16. **Names.** IR, EBNF and PEG names are `[A-Za-z_][\w-]*` (so `_r` and `r-2` are names and `1r` is an error, R-40, R-41); ABNF names start with a letter.
17. **Quoted titles** are unquoted in IR, EBNF and PEG, with the same escapes as strings; ABNF keeps them (X-13, R-21, A-32).

## 6. The open questions

- **Q1** (the measurement) is the renderer's.
- **Q2** (ABNF `;`): reproduced, quirk 8.
- **Q3** (escapes): the reference's rule is kept for IR, EBNF and PEG, and the canonical text rule 3 is recorded as a deviation in [canonical-model.md](../canonical-model.md), Railroad.
- **Q4** (sanitised values): recorded in the same section.
- **Q5** (max): the model keeps `min` and `max`; the renderer reads `min` only.
- **Q6** (inert configuration keys): the renderer's typed view. The configuration differential compares the railroad section on its own inputs (`pnpm port:diff config`): the sixteen colour and typography keys that the reference's `defaultConfig.ts` leaves `undefined` have no default in `build.rs` (`RAILROAD_KEYS_WITHOUT_DEFAULT`), as the class keys have none (section 8).
- **Q7** (default configuration): the renderer's.
- **Q8** (front matter title): dropped, as the reference does (probed above).
- **Q9** (one model and one DB): done. The exporter reads the same DB for the four detector ids.
- **Q10** (the entity decoding of the whole SVG string) is the renderer's common SVG step; the parser keeps the placeholders the DB keeps.
- **Q11** (probe gaps): closed by the `Diagram.fromText` probe of Q8 and by the explicit cases.

## 7. Checks

- `pnpm port:diff railroad`: 418 inputs (the collected fixtures, specs and docs, the 212 probes, the 79 edge cases and the 11 front matter cases), 339 accepted by the reference, 0 differences.
- Fuzzing, not committed: a random-token fuzz of 3,000 texts (189 accepted by the reference), two structured fuzzes of the four grammars with the escapes, the repeats, the comments and the header variants (2,500 texts with 1,272 accepted, and 3,000 with 1,532), and the 79 edge cases above (59 accepted): 0 differences.
- `pnpm port:diff config`: 0 differences, with the railroad section compared on its 418 inputs and every other group's counts unchanged.
- The gates: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --no-fail-fast`, `pnpm exec prettier --check tools/port`, `pnpm exec eslint tools/port`.

## 8. The configuration section

The reference's `railroad` section, as `getConfig().railroad` holds it after `Diagram.fromText` (probed for `railroad`, `railroadEbnf` and `railroadPeg`), has nine keys: `useMaxWidth` true, `compactMode` false, `padding` 10, `verticalSeparation` 8, `horizontalSeparation` 10, `arcRadius` 10, `strokeWidth` 2, `showMarkers` true and `markerRadius` 5. Its sixteen colour and typography keys are present with the value `undefined` (`defaultConfig.ts`, 295 to 314: `fontSize`, `fontFamily`, the terminal, non-terminal, line, marker, comment, special and rule-name colours), so the theme supplies them. The schema gives them defaults (`fontFamily` `monospace`, `fontSize` 14, and the colours), which the reference's section does not hold.

ferrigura takes the same route as the class section: `RAILROAD_KEYS_WITHOUT_DEFAULT` in `crates/ferrigura-model/build.rs` makes those sixteen fields `Option`s without a default, so the effective section holds the same nine keys. `ts_defaults.json` holds defaults that the reference adds in TypeScript, not defaults it removes, so it is not used here.

`pnpm port:diff config` compares the section on the railroad inputs and finds no difference.
