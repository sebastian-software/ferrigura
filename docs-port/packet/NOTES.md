# Packet (P2): research notes for the port

Status: research only. Nothing in the repository changed except this file, and nothing is committed. Reference: `packages/mermaid/src` at HEAD `aa5c4338f` (branch `sebastian/port-p2`). Plan: [../P2-PLAN.md](../P2-PLAN.md). Guide: [../P2-RESEARCH.md](../P2-RESEARCH.md).

Every rule cites the reference as `diagrams/packet/<file>:<line>` unless the file is named in full. **Probed** means the rule was run through the reference's own code: the DB and parser through `tools/port/reference.ts` (Vite SSR of the real `parser.ts`, `db.ts`, `preprocess.ts` and `config.ts`, with `preprocessDiagram` and `encodeEntities` applied as `Diagram.fromText` applies them), and the rendering through the built bundle `packages/mermaid/dist` in Chromium with the harness's settings. **Unverified** means read or inferred only. Probe material is outside the repository in `/private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/p2/`: `probe-db.mts` with the case files `cases*.ts`, `render-probe.mjs` with its input files, and `outline.cjs`, which prints the DOM of a rendered SVG.

## 0. Summary

Packet is the smallest P2 type. The reference is a seven-rule Langium grammar, a DB that stores rows of bit ranges, and a renderer that draws rectangles from bit arithmetic. It has no layout engine, measures no text, draws no random numbers, and takes its colours from its own fixed style sheet, not from the theme.

| Item                | Value                                                                                                                                                                                                                             |
| ------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Reference, non-spec | 7 TypeScript files, 385 lines (`detector.ts` 22, `diagram.ts` 14, `parser.ts` 107, `db.ts` 53, `types.ts` 29, `renderer.ts` 113, `styles.ts` 47), and the grammar `packages/parser/src/language/packet/packet.langium` (21 lines) |
| Specs               | `packet.spec.ts`: 14 tests, 7 for the parser and DB and 7 for error messages. `renderer.spec.ts`: 11 test cases, 2 ascending, 6 descending and 3 for `showBits`                                                                   |
| Fixtures            | 5 `.mmd` files in `e2e/diagrams/packet/pr-check/`. `nightly-check/` and `visual-check/` hold only `.gitkeep`                                                                                                                      |
| Parser              | Langium, to be ported by hand as pie is (`crates/ferrigura-syntax/src/pie.rs`)                                                                                                                                                    |
| DB                  | Rows of `{start, end, bits, label}`, plus title, accTitle and accDescr. The three header texts are sanitised                                                                                                                      |
| Renderer            | d3 attributes and arithmetic. No `getBBox`, no theme variable, no random draw, no `look`                                                                                                                                          |
| Riskiest parts      | The split blocks carry `bits` one less than they should, and both spec snapshots encode it. Input past 10,000 rows is dropped without an error. The shared lexer's number token cannot be used (section 2)                        |
| Effort              | Small. Rough estimate: 600 to 900 lines of Rust for parser, DB, renderer and tests                                                                                                                                                |

No exporter exists yet (`tools/port/export-packet.ts` is absent), so `pnpm port:diff packet` and `pnpm port:geometry packet` need the P2 scaffold first.

## 1. Sources

| File                                                    | Lines                                              | What it holds                                                                                                                                        |
| ------------------------------------------------------- | -------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `diagrams/packet/detector.ts`                           | 9-11                                               | Detection: `/^\s*packet(?![\w-])/` (line 10), on the cleaned text                                                                                    |
| `diagrams/packet/diagram.ts`                            | 7-14                                               | The definition: parser, renderer, styles, and `get db()` returning a new `PacketDB` (9-11)                                                           |
| `diagrams/packet/parser.ts`                             | 9                                                  | `maxPacketSize = 10_000`                                                                                                                             |
|                                                         | 11-53                                              | `populate`: the common DB calls (12), the row split (18-51), the last push (52)                                                                      |
|                                                         | 55-91                                              | `getNextFittingBlock`: the split of a block at a row boundary (75-89). The `bits` values are at 82 and 88                                            |
|                                                         | 93-107                                             | `parser`: the `ParserDefinition` with `parse` (96-106). The DB is read from `parser.parser.yy` (98)                                                  |
| `diagrams/packet/db.ts`                                 | 16                                                 | The packet defaults, `DEFAULT_CONFIG.packet`                                                                                                         |
|                                                         | 21-30                                              | `getConfig`: `cleanAndMerge` of the defaults and `commonGetConfig().packet`, with `paddingY += 10` when `showBits` (26-28)                           |
|                                                         | 36-40                                              | `pushWord`: an empty word is dropped (37)                                                                                                            |
|                                                         | 42-45, 47-52                                       | `clear` (commonDb's clear, then `packet = []`) and the header setters                                                                                |
| `diagrams/packet/renderer.ts`                           | 8-34                                               | `draw`: sizes (14-16), the viewBox (19), `configureSvgSize` (20), the rows (22-24), the title (26-33)                                                |
|                                                         | 36-112                                             | `drawWord`: the row group (50), the row offset (51), the mirrored column (57-58), the rectangle (62-68), the label (71-78), the bit numbers (80-110) |
| `diagrams/packet/styles.ts`                             | 5-16, 18-19, 21-44                                 | The style defaults, the merge with `packet`, and the CSS text                                                                                        |
| `diagrams/packet/types.ts`                              | 6-7, 9-12, 14-25, 27-29                            | `PacketBlock`, `PacketWord`, the `PacketDB` interface, `PacketStyleOptions`. `PacketData` (27-29) is not used by any code                            |
| `packages/parser/src/language/packet/packet.langium`    | 4-12, 14-20                                        | The `Packet` entry rule and the `PacketBlock` rule                                                                                                   |
| `packages/parser/src/language/packet/tokenBuilder.ts`   | 3-6                                                | The keyword list: `packet` only                                                                                                                      |
| `packages/parser/src/language/packet/module.ts`         | 45-49                                              | The services: the packet token builder and `CommonValueConverter`, with no custom converter                                                          |
| `packages/parser/src/language/common/common.langium`    | 6, 8-10, 14-16, 18-20, 22, 29, 31-34               | `EOL`, `TitleAndAccessibilities`, the accessibility and title terminals, `INT`, `STRING`, `NEWLINE`, and the hidden terminals                        |
| `packages/parser/src/language/common/tokenBuilder.ts`   | 14-28                                              | The keyword boundary: each keyword gets `(?:(?=%%)\|(?!\S))` appended (22-25)                                                                        |
| `packages/parser/src/language/common/valueConverter.ts` | 44-70                                              | The header values: single line trimmed with runs of blanks collapsed (58-60), and the block `accDescr` (62-67)                                       |
| `packages/parser/src/language/generated/grammar.ts`     | 25                                                 | The generated `PacketGrammar`. Its rules match `packet.langium`; checked by reading the JSON                                                         |
| `packages/parser/src/parse.ts`                          | 133-151, 156-174                                   | `parse`, which throws `MermaidParseError` on any lexer or parser error, and its message                                                              |
| `Diagram.ts`                                            | 19-85                                              | `fromText`: detection (31), `db.clear` (49), the front matter title (52-54), `textToParse` (77), `parse` (83)                                        |
| `preprocess.ts`                                         | 6-17, 19-31, 33-45, 58-91                          | CRLF to LF (10), front matter, directives, and `preprocessDiagram`                                                                                   |
| `utils.ts`                                              | 841-843, 874-906                                   | `cleanAndMerge`, `encodeEntities` (placeholders at 884-893), `decodeEntities`                                                                        |
| `diagrams/common/common.ts`                             | 56-83                                              | `sanitizeMore` (56-69) and `sanitizeText` (71-83), which runs DOMPurify                                                                              |
| `diagrams/common/commonDb.ts`                           | 8-30                                               | The sanitised setters: `setAccTitle` (16-18, leading blanks removed), `setAccDescription` (22-24), `setDiagramTitle` (28-30)                         |
| `diagrams/common/populateCommonDb.ts`                   | 4-14                                               | The three common DB calls, each only when the grammar's value is truthy                                                                              |
| `mermaidAPI.ts`                                         | 72-87, 319-334, 344-364, 397-421, 470-690, 750-758 | Config setup, the style sheet, SVG clean-up, the root `div` and `svg`, the render steps, and the accessibility attributes                            |
| `accessibility.ts`                                      | 27-32, 45-65                                       | `role`, `aria-roledescription`, and the `title` and `desc` elements                                                                                  |
| `setupGraphViewbox.js`                                  | 23-47                                              | `calculateSvgSizeAttrs` and `configureSvgSize`                                                                                                       |
| `rendering-util/selectSvgElement.ts`                    | 11-22                                              | Selection of the `svg` by id, with the sandbox case                                                                                                  |
| `schemas/config.schema.yaml`                            | 2663-2712                                          | `PacketDiagramConfig` and its defaults                                                                                                               |
| `config.type.ts`                                        | 2123-2156                                          | The TypeScript form of `PacketDiagramConfig`                                                                                                         |

The schema lists seven keys and sets `unevaluatedProperties: false` (2663-2669). The style options are not in it (section 6.5).

## 2. Parser

### 2.1 Grammar

```
Packet:      NEWLINE* "packet" ( TitleAndAccessibilities | blocks+=PacketBlock | NEWLINE )*
PacketBlock: ( start=INT ( '-' end=INT )? | '+' bits=INT ) ':' label=STRING EOL
EOL:         NEWLINE+ | EOF                       (common.langium:6)
TitleAndAccessibilities: ( (accDescr=ACC_DESCR | accTitle=ACC_TITLE | title=TITLE) EOL )+   (common.langium:8-10)
```

The grammar has no `EOL` before the first block, so a block may follow the keyword on the same line. The block itself needs `EOL`. Probed (section 2.3).

### 2.2 Lexer

| Terminal                          | Pattern                                                  | Where                   | Note                                                                                                       |
| --------------------------------- | -------------------------------------------------------- | ----------------------- | ---------------------------------------------------------------------------------------------------------- |
| keyword `packet`                  | `packet` followed by `(?=%%)` or `(?!\S)`                | `tokenBuilder.ts:22-25` | The boundary makes `packet0`, `packet-0`, `packet+8` and `packet"x"` lexer errors (probed)                 |
| `INT`                             | `0\|[1-9][0-9]*(?!\.)`                                   | `common.langium:19`     | The lookahead `(?!\.)` belongs to the second alternative only. `0.5` lexes as `0`, then `.` fails (probed) |
| `STRING`                          | `"([^"\\]\|\\.)*"` or the same with `'`                  | `common.langium:22`     | Spans lines: `"a` newline `b"` is one label (probed). Escapes are resolved by the default converter        |
| `NEWLINE`                         | `\r?\n`                                                  | `common.langium:29`     | CRLF is already removed by `cleanupText` (`preprocess.ts:10`)                                              |
| `:`, `+`, `-`                     | literals                                                 | the rules               |                                                                                                            |
| `TITLE`, `ACC_TITLE`, `ACC_DESCR` | regexes that read to the end of the line                 | `common.langium:14-16`  | The `accDescr { ... }` block form spans lines                                                              |
| hidden                            | `WHITESPACE`, `YAML`, `DIRECTIVE`, `SINGLE_LINE_COMMENT` | `common.langium:31-34`  | `%%` comments are removed earlier by `cleanupComments` (`preprocess.ts:65`, `diagram-api/comments.ts:6-8`) |

Unused by packet: `BOOLEAN`, `FLOAT`, `NUMBER` (`common.langium:12, 18, 20`), `ID` (27).

The shared Rust lexer (`crates/ferrigura-syntax/src/lexer.rs`, `Token::Number`) reads `-?[0-9]+(\.[0-9]+)?`. Packet cannot use it: `0-10` would lex as `0` and `-10`. The packet scanner needs its own unsigned `INT`, a `-` token and `+`, and must not change pie's token set.

### 2.3 Probed behaviour (the reference's parser and DB, `probe-db.mts`)

Inputs are the packet text as the parser sees it, after `preprocessDiagram` and `encodeEntities`. A rejection is the reference's own message, shortened.

| Input                                                             | Result                                                                                               |
| ----------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `0-10: "test"` then `11: "single"`                                | Accepted: `{start 0, end 10, bits 11}`, `{start 11, end 11, bits 1}`                                 |
| `0: "x"`, `0-0: "x"`                                              | Accepted: `start = end = 0`, `bits = 1`                                                              |
| `0.5: "x"`                                                        | Lexer error at `.`                                                                                   |
| `01: "x"`                                                         | Parse error: `0` then `1`                                                                            |
| `0-7 "x"` (no colon)                                              | Parse error, expecting `:`                                                                           |
| `0-7: x` (unquoted label)                                         | Lexer error at `x`                                                                                   |
| `0-7: 'single'`                                                   | Accepted                                                                                             |
| `0-7: "a" 8-15: "b"` (two blocks on one line)                     | Parse error: EOL expected                                                                            |
| `packet0-7: "a"`, `packet-0-7: "a"`, `packet+8: "a"`, `packet"x"` | Errors: the keyword is not followed by a boundary                                                    |
| `packet%%c` then `0-7: "a"`                                       | Accepted                                                                                             |
| `packet 0-7: "a"` (block on the keyword's line)                   | Accepted                                                                                             |
| `packet` alone                                                    | Accepted, no rows                                                                                    |
| `packet;`                                                         | Lexer error                                                                                          |
| `+0: "zero"` after `+8`                                           | `Packet block 8 is invalid. Cannot have a zero bit field.`                                           |
| `8-7: "b"`                                                        | `Packet block 8 - 7 is invalid. End must be greater than start.`                                     |
| `9-15` after `0-7`                                                | `Packet block 9 - 15 is not contiguous. It should start from 8.`                                     |
| `7-15` after `0-7`                                                | `Packet block 7 - 15 is not contiguous. It should start from 8.`                                     |
| A string with a real line break inside the quotes                 | Label is `a`, a line break, `b`                                                                      |
| A string with an escaped quote and tab: `"a\nb\tc\"d"`            | Escapes resolved: `a`, a line break, `b`, a tab, `c"d`                                               |
| `""`                                                              | Label `""`                                                                                           |
| `#amp;` and `#35;` in a label                                     | Stored as placeholders: `ﬂ°amp¶ß`, `ﬂ°°35¶ß` (see section 3)                                         |
| `<b>bold</b> & x` in a label                                      | Stored raw                                                                                           |
| CRLF line endings, tab after the colon                            | Accepted                                                                                             |
| `0-7: "a"` then `5-3 "x"`                                         | Parse error. Parse errors come before DB errors, because populate runs only after a successful parse |
| `0-7: "a"` then `5-3: "x"`                                        | DB error `Packet block 5 - 3 is invalid`                                                             |

Parse errors surface as `MermaidParseError` with `Parsing failed:` and the lexer and parser messages (`parse.ts:146-148` throws it, the message is at 156-174; probed). The differential counts a rejection on both sides as agreement (`tools/port/README.md`), so the message text is not compared; only the DB-level messages in `packet.spec.ts` are fixed by the specs.

### 2.4 Preprocessing

`Diagram.fromText` gives the parser `encodeEntities(source) + '\n'` (`Diagram.ts:77`). `source` is `code.cleaned`, which is the text after front matter, directives and comment removal (`preprocess.ts:58-91`). The trailing newline matters only for the last block, since `EOL` also accepts EOF.

`encodeEntities` (`utils.ts:874-896`) rewrites `#name;` to `ﬂ°name¶ß` and `#123;` to `ﬂ°°123¶ß`. It also protects `style` and `classDef` lines that contain `#`. These rules run on all of the text, so every label and every title passes through them.

Front matter and directives are read before the parse (`mermaidAPI.ts:72-87`). Probed: the descending fixture's front matter `bitsPerRow: 16` reaches the DB (`bitsPerRow 16`), and the same value given as a directive splits `0-3` and `4-40` into three rows (probe `span-rows-bpr16`). So the parser must read the effective configuration, not only the defaults.

### 2.5 Quirks the port must reproduce

1. **The last block must end the line.** Probed as above.
2. **Keyword boundary.** Probed as above, and the rule is in `tokenBuilder.ts:22-25`.
3. **Unsigned integers and the lookahead.** `0.5` is an error and `01` is an error.
4. **Multi-line labels.** A label may span lines (`STRING` allows newlines).
5. **Entities in labels and titles** are placeholders in the DB (section 3).
6. **Title precedence.** See section 3.

## 3. DB

### 3.1 Call order

| Step | Call                                               | Effect                                                                                                                                            |
| ---- | -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1    | `Diagram.fromText`, `db.clear()` (`Diagram.ts:49`) | `commonClear` resets `accTitle`, `accDescription` and `diagramTitle` (`commonDb.ts:10-14`). `packet = []` (`db.ts:42-45`)                         |
| 2    | front matter title (`Diagram.ts:52-54`)            | `setDiagramTitle(metadata.title)` if the front matter has a title                                                                                 |
| 3    | `parser.parse(text)` (`parser.ts:96-106`)          | Errors throw here, before any DB call. Then `populate`                                                                                            |
| 4    | `populateCommonDb(ast, db)` (`parser.ts:12`)       | `setAccDescription` if `ast.accDescr` is truthy, `setAccTitle` if truthy, `setDiagramTitle` if `ast.title` is truthy (`populateCommonDb.ts:4-14`) |
| 5    | `db.getConfig()` once (`parser.ts:16`)             | The config that sets `bitsPerRow` for the split                                                                                                   |
| 6    | blocks, in order (`parser.ts:18-51`)               | Validation (19-32), defaults for `start`, `end` and `bits` (22-34), then the split and `db.pushWord` (41-45, 52)                                  |

### 3.2 What the DB stores

| Field                | Value                                                                                                                                        | Source and probe                                                                                      |
| -------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| `packet`             | The rows, a list of words. A word is a list of blocks in left-to-right order                                                                 | `parser.ts:41-52`. Probed for every fixture                                                           |
| block `start`, `end` | Absolute bit numbers, inclusive                                                                                                              | `parser.ts:33-35`                                                                                     |
| block `bits`         | See the off-by-one below                                                                                                                     | `parser.ts:82, 88`                                                                                    |
| block `label`        | The string value, with entities as placeholders and markup raw                                                                               | Probed: `ﬂ°amp¶ß`, `<b>bold</b>`                                                                      |
| `title`              | Sanitised (`commonDb.ts:28-30`, `common.ts:71-83`)                                                                                           | Probed: `title <b>T</b> &amp;` gives `<b>T</b> &amp;`; `title <script>alert(1)</script>Hi` gives `Hi` |
| `accTitle`           | Sanitised, then leading blanks removed (`commonDb.ts:16-18`). The grammar's value collapses runs of blanks first (`valueConverter.ts:58-60`) | Probed: `accTitle:    spaced   title  ` gives `spaced title`                                          |
| `accDescr`           | Sanitised, then `\n\s+` becomes `\n` (`commonDb.ts:22-24`). A block `accDescr { }` loses the blanks of each line (`valueConverter.ts:62-67`) | Probed: a block with blank lines gives `line one\nline two`                                           |

Labels are not sanitised by the DB (`db.ts:36-40` stores the word as it is). The renderer writes the label with `.text(...)` (`renderer.ts:78`), so the markup is text in the SVG. Escaping happens in the serialisation (section 6).

### 3.3 Title precedence (probed)

The grammar keeps only the last `title` line of a diagram (Langium assigns the feature each time). `populateCommonDb` sets the title only when that value is truthy, and the front matter title is set before the parse. So:

| Input                                        | Title                                               |
| -------------------------------------------- | --------------------------------------------------- |
| front matter `FM`, and `title Text`          | `Text`                                              |
| front matter `FM`, and no title line         | `FM`                                                |
| front matter `FM`, and an empty `title` line | `FM` (the empty last title does not overwrite)      |
| `title Text`, then an empty `title` line     | `""` (the last title wins; the DB never saw `Text`) |

This matches [canonical-model.md](../canonical-model.md), section "Title precedence".

### 3.4 Quirks in the DB

1. **`bits` is one less than the true width for each split piece.** `parser.ts:82` gives `bits: rowEnd - block.start` and `parser.ts:88` gives `bits: block.end - rowStart`. Both should add one. Probed: `0-40: "wide"` at 32 bits per row gives `{start 0, end 31, bits 31}` and `{start 32, end 40, bits 8}`. At one bit per row the value is 0 (`bits 0` for every row in the probe). The spec snapshot `packet.spec.ts:129-161` ("should split into multiple rows") records `bits: 20` for `11-31` and `bits: 26` for `64-90`, so the specs fix the value. The renderer does not read `bits` (it uses `block.end - block.start + 1`, `renderer.ts:54`), so only the DB output carries the error.
2. **The row cap drops input without an error.** `maxPacketSize = 10_000` (`parser.ts:9`) is checked in the loop condition (`parser.ts:38`) against the number of finished rows. Once it is reached, the remaining bits of the current block are dropped, and so is every later block, because the loop no longer runs. Probed: `0-10010: "x"` at one bit per row gives 10,000 rows, and bits 10000 to 10010 are gone. No error is raised.
3. **The inner guard on the word length is not reachable.** `parser.ts:38` also checks `word.length <= bitsPerRow + 1`. Every block has at least one bit, so a word cannot hold more than `bitsPerRow` blocks. This is by reading, **unverified** by probe.
4. **`bitsPerRow` changes the output.** The split depends on the config (`parser.ts:16`). A `bitsPerRow` of 0 or below is not probed; the schema says `minimum: 1` (`config.schema.yaml:2680-2684`), and the reading of the loop suggests it would only reach the row cap. **Unverified.**
5. **Empty words are never stored**, because `pushWord` checks the length (`db.ts:37`). A word is pushed only when its row completes (`parser.ts:41-45`), so the last word is pushed at the end (`parser.ts:52`), and the DB never holds an empty row.

## 4. Canonical model proposal

The model is the DB state after parsing, as [canonical-model.md](../canonical-model.md) describes it for the other types.

```text
PacketDiagram { version: 1, title, accTitle, accDescr, rows: [[Block]] }
Block         { start, end, bits, label }
```

| Field                  | Rule                                                                                                                |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------- |
| `version`              | 1                                                                                                                   |
| `title`                | The DB value, after the title rules of section 3.3                                                                  |
| `accTitle`, `accDescr` | The DB values, sanitised with the port's `sanitize.rs`, with the DB's whitespace rules applied after the sanitising |
| `rows`                 | Every row of `db.getPacket()`, in order. Empty rows never appear                                                    |
| `start`, `end`, `bits` | As the DB has them. `bits` keeps the off-by-one of section 3.4                                                      |
| `label`                | The DB's string, with entities as placeholders. See the open question below                                         |

Left out:

- Config: `rowHeight`, `bitWidth`, `paddingX`, `paddingY`, `showBits`, `bitOrder`. These affect only the render, and the canonical rule is that render-only fields stay out (canonical-model.md, "Left out").
- `bitsPerRow` is not in the model, but the rows depend on it. The differential has to give both sides the same value, which is question Q1 in section 8.3.
- Styles.

Open questions for the coordinator: (a) keep labels encoded, as the timeline model does, or decode them. This note recommends encoded, because the DB holds that form and `decodeEntities` is applied to the SVG string (`mermaidAPI.ts:359`). (b) Whether `bitsPerRow` is a parser option (as `maxEdges` is for flowchart), or is read from the effective config by the Rust parser. The reference reads it from the config during the parse (`parser.ts:16`), and a front matter fixture needs it.

## 5. Specs

### 5.1 `packet.spec.ts` (268 lines, 14 tests)

| Lines   | Test                                                                      | Tests                                                                    |
| ------- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| 14-18   | `should handle a packet definition`                                       | The parser on `packet` alone gives no rows                               |
| 20-43   | `should handle diagram with data and title`                               | Title, accTitle, accDescr and one block                                  |
| 45-69   | `should handle single bits`                                               | `0-10` then `11`                                                         |
| 71-95   | `should handle bit counts`                                                | `+8` and `+16`                                                           |
| 97-121  | `should handle bit counts with bit or bits`                               | The same input as the test at 71. A duplicate, both in name and in input |
| 123-163 | `should split into multiple rows`                                         | The snapshot encodes the off-by-one (`bits 20`, `31`, `26`)              |
| 165-197 | `should split into multiple rows when cut at exact length`                | A split at the boundary (`17-63`)                                        |
| 199-258 | Seven tests: five non-contiguous variants, `end` before `start`, and `+0` | The exact message text, through `toThrowErrorMatchingInlineSnapshot`     |

The specs are the DB and parser oracle. They pin the off-by-one, so the port must keep it. They also pin the message formats: five tests for the contiguity check, one for the range check and one for the zero-bit check.

### 5.2 `renderer.spec.ts` (240 lines, 11 test cases)

These specs render into `document.body` and read the rendered SVG back: `x` and `width` of each rectangle, the label text, the bit numbers with their `text-anchor` (`left`, `center`, `right`), and the rows. They are the geometry oracle for the renderer.

| Lines                        | Group            | Cases                                                                                                                                                              |
| ---------------------------- | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 105, 129                     | Ascending order  | Fields from bit 0 (`x 1`, `width 251`, numbers `0`/`7` and `8`/`15`), and an explicit `ascending` is equal to the default                                          |
| 137, 161, 177, 183, 194, 205 | Descending order | Mirrored rows; mixed widths numbered from the top bit; widths unchanged; a single bit centred; each row mirrored on its own; a short row flush right (`x 24*32+1`) |
| 222 (two cases), 231         | `showBits`       | No numbers with `showBits: false` in both orders; still mirrored with `showBits: false`                                                                            |

Neither spec file covers the config defaults, `styles.ts`, the root `svg` attributes, the title, or the `paddingY` change for `showBits`. The port needs its own checks for those, and section 6 gives the values.

## 6. Renderer

### 6.1 Pipeline

The steps below are the generic ones of `mermaidAPI.ts` `renderDiagram` (470-680), which every type uses.

1. `appendDivSvgG` (`mermaidAPI.ts:397-421`) appends `div#d<id>` containing `svg#<id>` with `width="100%"` and `xmlns`, and an empty `g` inside it.
2. The style sheet (`mermaidAPI.ts:319-334`, `createUserStyles`) is built and inserted as the first child of the `svg` (596-600).
3. `diag.renderer.draw` (`mermaidAPI.ts:606-608`) runs the packet `draw` (section 6.2).
4. `addA11yInfo` (`mermaidAPI.ts:623`, `750-758`) sets `role="graphics-document document"` and `aria-roledescription="packet"` on the `svg`. When `accTitle` or `accDescr` is set, it inserts `<title>` or `<desc>` as the first children and sets `aria-labelledby` or `aria-describedby` (`accessibility.ts:27-65`). Probed for the first two attributes; the title and desc elements are **unverified** because no fixture sets them.
5. Serialisation (`mermaidAPI.ts:629-650`): `cleanUpSvgCode` (344-364) decodes the placeholders with `decodeEntities` (`utils.ts:904-906`), then DOMPurify runs with `ADD_TAGS: ['foreignobject']`, `ADD_ATTR: ['dominant-baseline']` and `HTML_INTEGRATION_POINTS` (645-649), unless the security level is loose or sandbox.

Probed effects of step 5: a title `A #amp; B` is written `A &amp; B`, and a label `a #35;` reads back as `a #` (the DOMPurify serialiser writes the text node). The string that `render` returns is therefore the DOMPurify form, not the placeholder form.

### 6.2 `draw` (`renderer.ts:8-34`)

Sizes, with the defaults of section 6.4 and `showBits` true:

| Quantity         | Formula                                                                                                                                                                                                                                      | Default value            |
| ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------ |
| `totalRowHeight` | `rowHeight + paddingY` (14), `paddingY` already `+10` when `showBits` (`db.ts:26-28`)                                                                                                                                                        | 32 + 15 = 47             |
| `svgHeight`      | `totalRowHeight * (rows + 1) - (title ? 0 : rowHeight)` (15)                                                                                                                                                                                 | one row with a title: 94 |
| `svgWidth`       | `bitWidth * bitsPerRow + 2` (16)                                                                                                                                                                                                             | 32 \* 32 + 2 = 1026      |
| viewBox          | `0 0 svgWidth svgHeight` (19)                                                                                                                                                                                                                |                          |
| size             | `configureSvgSize(svg, svgHeight, svgWidth, useMaxWidth)` (20). With `useMaxWidth` (the base default, true) the attributes are `width="100%"` and `style="max-width: <W>px;"`. Otherwise `height` and `width` (`setupGraphViewbox.js:23-47`) |                          |

Probed: the simple fixture gives `viewBox="0 0 1026 94"` and `style="max-width: 1026px;"`. The complex fixture with `showBits` false gives `viewBox="0 0 1026 296"`, which is `(32 + 5) * (7 + 1)`, the title adding nothing.

The title (26-33): a `text` with `x = svgWidth / 2`, `y = svgHeight - totalRowHeight / 2`, `dominant-baseline="middle"`, `text-anchor="middle"` and `class="packetTitle"`. It is appended to the `svg`, after the row groups. Probed: the simple fixture gives `y="70.5"` for `94 - 47/2`.

Rows (22-24): one `g` per row in `renderer.ts:50`. Its `y` offset is `rowNumber * (rowHeight + paddingY) + paddingY` (51). Probed: the first row's rectangles start at `y=15` with `showBits` true.

### 6.3 `drawWord` (`renderer.ts:36-112`), per block in DOM order

| Element            | Attributes                                                                                                                                                                                 | Lines   |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------- |
| `rect`             | `x = column * bitWidth + 1`, `y = wordY`, `width = bits * bitWidth - paddingX`, `height = rowHeight`, `class="packetBlock"`                                                                | 59-68   |
| `text` (label)     | `x = blockX + width / 2`, `y = wordY + rowHeight / 2`, `class="packetLabel"`, `dominant-baseline="middle"`, `text-anchor="middle"`, text = `label`                                         | 71-78   |
| `text` (start bit) | `x = blockX` (or the centre for one bit), `y = wordY - 2`, `class="packetByte start"`, `dominant-baseline="auto"`, `text-anchor` `start` (or `middle` for one bit), text = the leading bit | 89-98   |
| `text` (end bit)   | `x = blockX + width`, same `y`, `class="packetByte end"`, `text-anchor="end"`, text = the trailing bit. Only when `bits > 1`                                                               | 100-110 |

`bits` in these formulas is `block.end - block.start + 1` (54), not the DB's `bits` field. The column is `block.start % bitsPerRow` (57). With `bitOrder: descending` the column is `bitsPerRow - firstColumn - bits` (58), and the leading bit is `block.end`, the trailing bit `block.start` (84-86). Probed: the descending fixture gives `DATA` at `x 257`, width 251, with `7` on the left and `0` on the right.

Ordering: the rectangle, then the label, then the bit numbers, for each block in the row's order. The `g` of the row is appended after the previous row's `g`, so the DOM order is `style`, the empty `g` from step 1, then one `g` per row, then the title `text`.

Probed DOM of the simple fixture: `svg > style, g (empty), g > rect, text, text, text`, with the title `text` as the last child of the `svg`, and the `rect`'s `x="1" y="15" width="347" height="32"`.

### 6.4 Configuration

| Key           | Default                 | Schema line              | Read by                              |
| ------------- | ----------------------- | ------------------------ | ------------------------------------ |
| `rowHeight`   | 32                      | 2670-2674                | `renderer.ts:11`                     |
| `bitWidth`    | 32                      | 2675-2679                | `renderer.ts:11`                     |
| `bitsPerRow`  | 32                      | 2680-2684                | `parser.ts:16`, `renderer.ts:11`     |
| `showBits`    | true                    | 2685-2688                | `db.ts:26`, `renderer.ts:11`         |
| `bitOrder`    | `ascending`             | 2689-2702                | `renderer.ts:52` (`descending` only) |
| `paddingX`    | 5                       | 2703-2707                | `renderer.ts:60`                     |
| `paddingY`    | 5 (+10 with `showBits`) | 2708-2712, `db.ts:26-28` | `renderer.ts:14`                     |
| `useMaxWidth` | the base default, true  | `BaseDiagramConfig`      | `setupGraphViewbox.js`               |

The config comes from `db.getConfig()`, which merges the defaults and the user's `packet` section (`db.ts:21-25`). The line ranges are those of `config.schema.yaml`; each `minimum` is inside its range.

### 6.5 Style sheet and themes

`styles.ts` (18-45) returns a CSS string from `{ packet }`, merged over the defaults (`defaultPacketStyleOptions`, 5-16). The defaults are `byteFontSize 10px`, `startByteColor black`, `endByteColor black`, `labelColor black`, `labelFontSize 12px`, `titleColor black`, `titleFontSize 14px`, `blockStrokeColor black`, `blockStrokeWidth 1`, `blockFillColor #efefef`.

The packet styles read no theme variable. Probed with `redux-color` and `redux-dark-color`: the `.packetBlock` rule is `stroke:black; stroke-width:1; fill:#efefef` in both.

The options come from the object `getStyles` passes to the function, which is `{...config.themeVariables, theme, look, svgId}` (`mermaidAPI.ts:327-333`, `src/styles.ts:35-38`). The `packet` key is therefore read from `themeVariables`, not from the diagram's config. Probed: `site.themeVariables.packet.blockFillColor = 'red'` changes the fill to `red`, and `site.packet.blockFillColor = 'red'` does not change it. The config key is not in the schema anyway (`unevaluatedProperties: false`). Whether the port should reproduce the `themeVariables.packet` path is an open question.

The generic sheet (`packages/mermaid/src/styles.ts` 11-172) reads these theme values for every type: `fontFamily`, `fontSize`, `textColor`, `errorBkgColor`, `errorTextColor`, `lineColor`, `nodeBorder`, `strokeWidth` and the gradient and shadow options. They apply to packet's `svg` text and to the error classes only; packet's own elements use the fixed colours above.

### 6.6 Text measurement, layout, sizing, randomness, DOM

- **Text measurement.** None. The packet code never calls `getBBox`, `getComputedTextLength` or `calculateTextDimensions` (checked by reading `renderer.ts` and `db.ts`). Its label boxes are the bit widths. Labels longer than their box are not wrapped or truncated: the code has no wrap step (`renderer.ts:71-78`). The overflow itself is **unverified** by probe, because no fixture has a label wider than its box.
- **Layout.** None. Positions are the arithmetic of section 6.2.
- **Sizing.** The root size is `configureSvgSize` (section 6.2). The viewBox is computed, not measured.
- **Randomness and the clock.** None. Grepped in `diagrams/packet/`: no `Math.random`, no `Date`.
- **`look` and `handDrawn`.** Not read. The packet renderer draws plain `rect`s whatever `look` says (`renderer.ts`, read in full).
- **DOM dependencies a server renderer must replace.** The root element and the style insertion (`mermaidAPI.ts:397-421, 596-600`); `selectSvgElement`, which selects `#id` in `body` (`selectSvgElement.ts:11-22`); d3's `append`, `attr`, `text`, `insert` and `classed`, which build the tree; `innerHTML` and DOMPurify in the serialisation (`mermaidAPI.ts:629-650`); and `document` for the accessibility elements (`accessibility.ts`). The sandbox iframe (`mermaidAPI.ts:540-560`) is not used by ferrigura.

## 7. Fixtures

| File (in `e2e/diagrams/packet/pr-check/`)                        | Features                                                                                             | Probed viewBox (default config) |
| ---------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- | ------------------------------- |
| `should-render-a-simple-packet-diagram.mmd`                      | `title Hello world` in the text; one range `0-10`                                                    | `0 0 1026 94`                   |
| `should-render-a-simple-packet-diagram-without-ranges.mmd`       | Single-bit fields `0:` and `1:`                                                                      | `0 0 1026 62`                   |
| `should-render-a-complex-packet-diagram.mmd`                     | A TCP-like header of 224 bits in 7 rows; bit numbers shown; no title, so the height is `47 * 8 - 32` | `0 0 1026 344`                  |
| `should-render-a-complex-packet-diagram-with-showbits-false.mmd` | Same fields; front matter `title: "Packet Diagram"` and `config.packet.showBits: false`              | `0 0 1026 296`                  |
| `should-render-a-packet-diagram-with-descending-bit-order.mmd`   | Front matter `title: "Status Register"`, `bitOrder: descending`, `bitsPerRow: 16`                    | `0 0 514 94`                    |

The fixtures are the only geometry input for packet. Each is Latin text inside the bundled faces, so no fixture needs an exclusion. None uses `look`, `accTitle` or `accDescr`, so the accessibility elements are not in the fixtures; the specs cover the parser for them. The front matter config is used by 2 fixtures, so the harness's `FORCED_SETTINGS` (which set `htmlLabels` and `fontFamily`) do not affect packet, which draws no labels with HTML.

Probed: all five render in Chromium with the bundle. The two complex fixtures have seven rows (`224 / 32`).

## 8. Risks, effort, open questions

### 8.1 Risks

| Risk                                                                                    | Level                                 | Why                                                                                                                      |
| --------------------------------------------------------------------------------------- | ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| Parser edge cases (keyword boundary, unsigned integers, lookahead, the same-line block) | Low to medium                         | Each is a rule of the Langium grammar and has a probed input (section 2.3)                                               |
| The off-by-one `bits`, and the row cap                                                  | Low once known                        | Both are small and both are pinned by a spec or a probe                                                                  |
| Titles and accTitle through the sanitiser                                               | Low to medium                         | Needs `sanitize.rs` to match DOMPurify on the markup that titles can carry. Probed inputs: `<b>`, `<script>`, `&amp;`    |
| Serialisation of the final SVG (DOMPurify's form)                                       | Low                                   | Only affects strings with `&`, `<` or `#`. The differential compares models, not strings; the harness compares structure |
| `bitsPerRow` from the front matter and the directives                                   | Medium, because it is a design choice | The parser needs the effective configuration, see Q1                                                                     |
| The `themeVariables.packet` path                                                        | Low                                   | Reachable only through an undocumented key                                                                               |

### 8.2 Effort

About 250 to 350 lines for the scanner and the recursive descent, with the populate loop; 80 for the DB model; 150 for the renderer (`draw`, `drawWord`, the bit numbers, the mirrored column); about 200 for the specs (the 25 test cases of section 5, ported as Rust tests). The total is about 600 to 900 lines, which is an estimate and not a measurement.

### 8.3 Open questions

- **Q1. How does the Rust parser get `bitsPerRow`?** Options: read it from the effective config in `ferrigura-model`, or take it as a parser option with a default of 32 that the front matter and the directives can override. The reference reads it from the config during the parse (`parser.ts:16`), and the fixture `descending` needs it.
- **Q2. Are labels in the model encoded or decoded?** This note recommends encoded, which is the timeline's rule. The DB stores the encoded form, and `decodeEntities` runs at the string level.
- **Q3. Reproduce the off-by-one and the row cap?** Both are reproduced in this proposal, as the M6 decision on quirks requires. Confirm.
- **Q4. Reproduce the `themeVariables.packet` route?** It is reachable, and a user can set `blockFillColor` through it. Low cost to port; it is a choice of behaviour, not of structure.
- **Q5. Which differential runs on packet?** `pnpm port:diff packet` needs an exporter: parse `packet.spec.ts`'s inputs and the fixtures with the reference's `parser.ts` and `db.ts`, through the same `preprocessDiagram` and `encodeEntities` as above. The probe file `probe-db.mts` is a working starting point, and the geometry harness can take the fixtures as they are.
