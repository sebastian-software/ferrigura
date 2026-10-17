# P2 packet parser: plan

Status: the parser, the DB, the model, the exporter and the parser and DB specs are ported; `pnpm port:diff packet` and `pnpm port:diff config` show no difference. The renderer is not ported, and its specs are ignored stubs. Reference: `packages/mermaid/src` of this branch (`diagrams/packet/`, `packages/parser/src/language/packet/`). The research is in [NOTES.md](./NOTES.md). This page says what the port does and which quirks it keeps. Quirks are numbered 1 to 9 here, and the code and the tests cite them by number.

## 1. Sources

| File                                                  | Used for                                                                     |
| ----------------------------------------------------- | ---------------------------------------------------------------------------- |
| `packages/parser/src/language/packet/packet.langium`  | The grammar (`Packet`, `PacketBlock`)                                        |
| `packages/parser/src/language/common/common.langium`  | `TITLE`, `ACC_TITLE`, `ACC_DESCR`, `STRING`, `NEWLINE`, the hidden terminals |
| `packages/parser/src/language/common/tokenBuilder.ts` | The keyword boundary `(?:(?=%%)\|(?!\S))`                                    |
| `diagrams/packet/parser.ts`                           | `populate` (11-53) and `getNextFittingBlock` (55-91)                         |
| `diagrams/packet/db.ts`                               | `getConfig` (21-30), `pushWord` (36-40), `clear` (42-45)                     |
| `diagrams/common/commonDb.ts`, `populateCommonDb.ts`  | The header texts: the sanitised setters, and which of them `populate` calls  |

The renderer's files (`renderer.ts`, `styles.ts`) are not used.

## 2. Text before the scanner

`prepare` removes the front matter and normalises the line breaks. `remove_directives` and `cleanup_comments` then run as in `preprocess.ts`, `encode_entities` replaces the entities by placeholders, and one `\n` is appended (`Diagram.ts:77`). The front matter's title is kept aside, and the DB sets it before the parse (quirk 7).

## 3. Scanner

The scanner (`packet/scanner.rs`) tries the terminals in the grammar's order, with the first match winning. Its own rules, with the reason for each:

- **`packet`** is a keyword only when a blank, a line break, `%%` or the end of the text follows (the boundary of `tokenBuilder.ts`). `packet0`, `packet-0`, `packet+8` and `packet"x"` are not keywords, and every other word is an error, since no word terminal is used by a packet (quirk 3).
- **Numbers** are the unsigned `INT` `0|[1-9][0-9]*(?!\.)`. A lone `0` is one token; a non-zero digit takes the whole run. `0.5` is therefore `0` and then an error at `.`, and `01` is two numbers, which the grammar rejects (quirk 4). The shared lexer cannot be used, because its `-?[0-9]+` would read `0-10` as `0` and `-10`.
- **`title`, `accTitle`, `accDescr`** are read to the end of their line, or to a `%%` on it, as the regexes of `common.langium` read them. `accTitle` needs its `:`, and `accDescr` its `:` or a `{ … }` block that closes (both through the shared `colon_form` and `descr_block_text`).
- **Strings** are `"…"` or `'…'`, and may span lines; an escape cannot escape a line break (`.` does not match one).
- **Hidden**: blanks, `%%` comments and directives, and a `---` block that is not at the start of the text. A lone `\r` is an error.

## 4. Grammar

A recursive descent over the tokens (`packet/grammar.rs`):

```text
Packet:      NEWLINE* "packet" ( TitleAndAccessibilities | PacketBlock | NEWLINE )*
PacketBlock: ( INT ( '-' INT )? | '+' INT ) ':' STRING EOL
EOL:         NEWLINE+ | EOF
```

A header line needs its `EOL`, and a block needs its `EOL` too, so two blocks on one line are an error (quirk 5). The `packet` keyword may be followed by a block on its own line. The last `title`, `accTitle` and `accDescr` of the diagram win, and an empty value is kept as empty, as the grammar's assignment keeps it (quirk 7).

## 5. DB

`db.rs` is `populate` and `getNextFittingBlock`, step by step, with the arithmetic in the reference's order and on doubles, so that the degenerate `bitsPerRow` values give the reference's rows. The steps:

1. The header texts (quirk 6) and the title precedence (quirk 7) are set before the rows, as `populateCommonDb` does.
2. For each block, in order: the `end < start` check (only when both are written), `start` from `lastBit + 1` for the `+bits` form, the contiguity check, the zero-bit check, `end` from the bits, and `bits` from the end. Each check throws the reference's message, at the block.
3. The block is split at each row boundary (quirk 1) while the word's length is at most `bitsPerRow + 1` and fewer than 10,000 rows are kept (quirk 2). A row is pushed when its last bit is reached.
4. The last word is pushed if it is not empty.

Quirks of the DB:

1. **`bits` is one less than the width of each piece.** `parser.ts:82` and `:88` give `rowEnd - start` and `end - rowStart`. `0-40` at 32 bits gives `{0, 31, bits 31}` and `{32, 40, bits 8}`. The spec snapshots (`should split into multiple rows`) pin it.
2. **The row cap drops input without an error.** Once 10,000 rows are kept, the rest of the current block and every later block are dropped. The later blocks are still checked, so `5-3` after the cap is still rejected. A partial word is never pushed at the cap, because a row is complete when the cap is reached.
3. **The keyword needs its boundary** (section 3). `PACKET` is not the keyword: the detector is case-sensitive, and the scanner matches the same spelling.
4. **Unsigned numbers.** `0.5` and `01` are rejected (section 3).
5. **Blocks on one line.** `0-7: "a" 8-15: "b"` is rejected, and `packet 0-7: "a"` is accepted.
6. **Labels** are the grammar's string, with the escapes resolved and the entity placeholders kept (`ﬂ°amp¶ß`), and the markup is raw, since the DB does not sanitise labels. A label may hold a line break. The three header texts are sanitised (`sanitize_text`), `accTitle` loses its leading blanks, and `accDescr` loses the blanks after each line break.
7. **Title precedence.** The grammar's title, if non-empty, is sanitised and used; otherwise the front matter's title, sanitised, if it has one; otherwise `""`. The front matter title is not passed through the entity encoding, which only the parser's text gets.
8. **Errors.** A parse error is reported before any DB error, because `populate` runs only after a successful parse. A DB error is reported at the block that causes it, with the reference's message, and the position is ferrigura's own (decision 0002).
9. **`bitsPerRow` is read from the effective configuration**, as the reference reads it in `db.getConfig()`: `effective(None, Some("packet"), config::read(text))`, with the front matter, the `packet` directive and the default of 32. The reading follows JavaScript: a number is used as it is, `null` is 0, a boolean is 0 or 1, and a string is converted with `Number()` in `row * bitsPerRow` and with `"<string>1"` in `bitsPerRow + 1`. The configuration keeps the default in place of an array or an object (the probes `[16]` and `{}` read 32 on both sides), so their string form is never read. Whatever the reference's arithmetic makes of 0, a negative, a fraction or `.inf` is reproduced by the same arithmetic: 0 gives 10,000 rows, a negative value gives none, and `.inf` puts a block in one row, with the sentinel string in its `end` and `bits` (`docs-port/canonical-model.md`).

## 6. Model

`ferrigura_model::packet::PacketDiagram`: `version`, `title`, `accTitle`, `accDescr`, and `rows`, a list of rows, each a list of `PacketBlock { start, end, bits, label }`. The numbers are doubles, written as numbers when finite and as the sentinel strings otherwise. The rows are the DB's words; an empty row never appears. Left out: `bitsPerRow` and the other configuration, and the styles, which are the renderer's (quirk 9 says where the rows depend on the configuration).

## 7. Exporter and specs

- `tools/port/export-packet.ts` runs the reference's `parser.ts` and `PacketDB` with the configuration of each input (the steps of `Diagram.fromText`, with `encodeEntities` and a line break). The model is read out of the DB.
- Differential: `pnpm port:diff packet` runs 115 inputs, 79 of them accepted by the reference, and reports no difference. The inputs are the reference's fixtures, the string literals of its specs and docs, and the explicit cases of `tools/port/cases.ts`, which pin every probe of NOTES section 2.3 and the degenerate configurations above. Three inputs that the detector does not take are in `undetectedCases`.
- `pnpm port:diff config` compares the packet section, with `packet` no longer in the list of sections that are not compared.
- Specs: `crates/ferrigura-syntax/tests/packet_spec.rs` (the 14 cases of `packet.spec.ts`, its parser and DB tests and its seven errors), `packet_grammar.rs` (the four cases of `packages/parser/tests/packet.test.ts`, and the grammar's rules of section 3 and 4), and `packet_db_spec.rs` (quirks 1, 2, 6, 7, 9). `crates/ferrigura-render/tests/packet_render_spec.rs` holds the eleven cases of `renderer.spec.ts` as ignored stubs, with the reason in each.

## 8. Not ported

- The renderer (`renderer.ts`) and its styles. `styles.ts` reads `themeVariables.packet` (NOTES Q4); the route is part of the renderer and comes with it.
- `PacketData` in `types.ts` is unused by any code, and is not ported.

## 9. Corrections to NOTES

- Section 3.4, item 3 (the word-length guard is unreachable for a whole `bitsPerRow` of 1 or more): true for a whole number, and the guard is kept as written. Whether a non-integer `bitsPerRow` reaches it is not established; no input pins its effect.
- Section 3.4, item 4 (`bitsPerRow` of 0 or below is unverified): now verified by the differential. Zero gives 10,000 rows of bits -1 and a negative value gives no rows (`cases.ts`).
- The probes of section 2.3 are the first group of `cases.ts`, and agree with the reference.
