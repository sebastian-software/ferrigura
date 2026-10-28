# Packet Diagram

The packet diagram draws a network packet or any bit layout as rows of fields. It starts with `packet` or `packet-beta`, then one block per line: `0-15: "Source Port"`, a single bit `5: "Flag"`, or a width in bits `+8: "Flags"`. ferrigura parses it with a hand-written scanner and recursive-descent grammar in `ferrigura-syntax/src/packet/` and draws it with the type's own renderer in `ferrigura-render/src/packet/`. Nothing is measured and no layout engine runs: every rectangle, label and bit number is arithmetic on the rows.

## Model

`ferrigura_model::packet::PacketDiagram` ([canonical-model.md](../canonical-model.md#packet) has the conventions) holds `title`, `accTitle`, `accDescr` and `rows`: a list of rows, each a list of `{start, end, bits, label}` in left-to-right order. The numbers are doubles, written as JSON numbers when finite and as the sentinel strings otherwise. The rows are the DB's words, already split at the row boundaries, and an empty row never appears.

- The header texts are sanitized; `accTitle` loses its leading blanks and `accDescr` the blanks after each line break. The title is the grammar's when it is non-empty, otherwise the front matter's, sanitized.
- A label is the grammar's string with the escapes resolved and the entity placeholders kept (`#amp;` is `ﬂ°amp¶ß`). Markup stays raw, because the DB does not sanitize labels. A label may hold a line break.
- Left out: `bitsPerRow` and the rest of the configuration, which only the parse and the renderer read, and the styles.

## Parsing

`packet/mod.rs` runs the preprocessing steps of `Diagram.fromText`, parses with `grammar.rs` over the tokens of `scanner.rs`, and fills the rows with `db.rs`. `parse` reads `bitsPerRow` from the effective configuration of the text's own front matter and directives; the facade calls `parse_with` with the whole effective configuration, which also holds the site configuration, so a `bitsPerRow` given to the `Renderer` splits the rows as the reference does.

The grammar is `Packet: NEWLINE* "packet" (TitleAndAccessibilities | PacketBlock | NEWLINE)*` and `PacketBlock: (INT ("-" INT)? | "+" INT) ":" STRING EOL`. The scanner tries the terminals in the grammar's order, and the first match wins. Quirks that ferrigura keeps:

- `packet` is a keyword only when a blank, a line break, `%%` or the end of the text follows (the boundary of Mermaid's keyword token builder). `packet0-7`, `packet-0`, `packet+8` and `packet"x"` are errors. `PACKET` is not the keyword: the detector is case-sensitive. A block may follow the keyword on its line (`packet 0-7: "a"`); `packet;` is an error.
- Numbers are the unsigned `INT` `0|[1-9][0-9]*(?!\.)`. `0.5` is `0` and then an error at the dot, and `01` is two numbers, which the grammar rejects. The shared lexer cannot be used: its `-?[0-9]+` would read `0-10` as `0` and `-10`.
- A block needs its `EOL`, so two blocks on one line are an error. The last block may end the text.
- A string is `"…"` or `'…'`, may span lines, and an escape cannot escape a line break. An unquoted label is an error.
- `title`, `accTitle` and `accDescr` are read to the end of their line or to a `%%` on it. The last of each wins, and an empty value is kept as empty.
- A parse error is reported before any DB error, because the DB runs only after a successful parse.

The DB is `populate` and `getNextFittingBlock` of Mermaid's `parser.ts`, step by step with the arithmetic in the reference's order and on doubles. Each block goes through the `end < start` check (only when both are written), `start` from the last bit for the `+bits` form, the contiguity check, the zero-bit check, `end` from the bits, and `bits` from the end; each check fails with the reference's message at the block (`Packet block 8 - 7 is invalid. End must be greater than start.`). The block is then split at each row boundary. The DB keeps these quirks:

- **`bits` is one less than the width of each split piece.** `0-40` at 32 bits per row gives `{0, 31, bits 31}` and `{32, 40, bits 8}`. The renderer does not read `bits`; it uses `end - start + 1`.
- **The row cap drops input without an error.** Once 10,000 rows are kept, the rest of the current block and every later block are dropped; the later blocks are still checked, so `5-3` after the cap is still rejected.
- **`bitsPerRow` is read as JavaScript reads it.** A number is used as is, `null` is 0, a boolean is 0 or 1, and a string is converted with `Number()` in `row * bitsPerRow` and by concatenation in `bitsPerRow + 1`. The configuration keeps the default in place of an array or an object. 0 gives 10,000 rows of bits -1, and a negative value gives no rows. Because the schema names `bitsPerRow` as a number but the reference's arithmetic accepts anything, the section has a hand-written type beside the generated ones ([0008](../decisions/0008-canonical-model-and-typed-views.md)).

## Layout and rendering

`render` ports `renderer.ts` (`draw`, `drawWord`) and `styles.ts` (`style.rs`). The size follows from the number of rows: `totalRowHeight = rowHeight + paddingY`, where `paddingY` is raised by 10 when the bit numbers are shown; `svgHeight = totalRowHeight * (rows + 1) - (title ? 0 : rowHeight)`; `svgWidth = bitWidth * bitsPerRow + 2`. With `useMaxWidth` the root has `width="100%"` and a `max-width`; without it, `width` and `height`.

A block's rectangle starts at `column * bitWidth + 1` with the width `bits * bitWidth - paddingX`, where `bits = end - start + 1` and the column is `start % bitsPerRow`; with `bitOrder: descending` the column is mirrored and the bit numbers swap sides. Each block is a `rect`, its label (`packetLabel`, centered), and its bit numbers (`packetByte start`, and `packetByte end` when the block is wider than one bit). Each row is a `g` at `rowNumber * (rowHeight + paddingY) + paddingY`. The title is the last child, centered below the rows. The text is drawn with the bundled sans face embedded as a subset ([0009](../decisions/0009-svg-text-labels.md), [0010](../decisions/0010-text-measurement-and-fonts.md)); a label longer than its box is neither wrapped nor truncated.

The renderer takes the JavaScript number of the effective `bitsPerRow` (`PacketSection::bits_per_row`), not the typed configuration, which replaces a string that is not a number by the default; the reference computes `NaN` with such a value, and so does ferrigura.

## Differences from Mermaid

- **Colors.** Each color is a custom property on the diagram's id, and each colored element also carries it as a presentation attribute ([0011](../decisions/0011-themes-as-css-variables.md)); the reference writes no color attribute on these elements. The harness does not compare them.
- **Detector.** `packet0-7` is not a packet diagram. Upstream's detector takes any word that starts with the keyword, which is a bug of Mermaid; the reference carries a patch that ends the detector at a word boundary, as ferrigura's does ([0014](../decisions/0014-upstream-reference-at-a-pin.md)).
- **Error messages.** Parse errors are ferrigura's own ([0007](../decisions/0007-hand-written-parsers.md)); the DB's messages are the reference's, because its spec asserts them.

## Known gaps

None known for this type.

## Checking it

- `pnpm -C tools/port port:diff packet` compares the canonical models on every fixture, spec string, documentation example and case of `cases.ts`, which pins the grammar's probes and the degenerate configurations; `export-packet.ts` runs the reference's parser and DB with each input's configuration. `port:diff config` compares the `packet` section.
- `pnpm -C tools/port port:geometry packet` compares the geometry of the fixtures and of the probes in `tools/port/geometry/probes/packet/`: wide and narrow `bitsPerRow`, blocks across rows, long and multi-line labels, a title with markup and entities, many rows, descending order, `showBits: false`, `useMaxWidth: false`, fractional sizes, an odd `bitsPerRow`, numeric and non-numeric string values, and the theme's packet options.
- The Rust specs: `ferrigura-syntax/tests/packet_spec.rs` ports the reference's spec and its seven errors, `packet_grammar.rs` the parser tests and the grammar's rules, `packet_db_spec.rs` the DB's quirks; `ferrigura-render/tests/packet_render_spec.rs` ports the renderer spec and pins the sizes, the title, the accessible names, the entities and the style sheet.

## Learnings

- **The style options come from `themeVariables.packet`**, not from the diagram's `packet` section: the reference's `getStyles` passes `{...config.themeVariables, theme, look, svgId}` to the packet styles. The directive sanitizer keeps only the configuration's keys, so `blockFillColor` written in a directive is dropped on both sides while `labelColor` survives; options given to `initialize` are kept. A `packet` that is not an object merges nothing. The packet styles read no theme token, so a packet has the same colors (`#efefef` fill, black strokes and text) in light and dark.
- **The rows depend on the site configuration.** A parser that read only the front matter and the directives would split rows differently from the boxes the renderer draws (a 20-bit field drawn 635 px wide in a 514 px root).
- **A label placeholder is decoded to the character it stands for** (`#amp;` is shown as `&`); any other `&` or `<` is shown as written.
- **The root attribute order** is the reference's (`width`, namespace, then `height` or `style`).
- **The unreachable guard stays**: the word-length guard `word.length <= bitsPerRow + 1` cannot trigger for a whole `bitsPerRow` of 1 or more; it is kept as written, because whether a fractional value reaches it is not established.
