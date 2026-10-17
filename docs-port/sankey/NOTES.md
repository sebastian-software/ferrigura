# Sankey (P2): research notes for the port

Status: research, and the renderer is ported (`crates/ferrigura-render/src/sankey/`, with the layout in `sankey/layout.rs`; the port's own checks are in the report of its branch). The research below describes the reference and was made before the port. Reference: the TypeScript at this worktree's HEAD `aa5c4338f`; the built bundle in `packages/mermaid/dist` is from 2026-10-16 and its sankey chunk matches the source, including the quirk at `sankeyRenderer.ts` 91 (checked in the built `chunks/mermaid.esm/sankeyDiagram-JTIULAC4.mjs` 1254).

Legend. A rule with a `file:line` citation was read from that code. **Probed** means the rule was run in Chromium on the built bundle (`mermaid.render` through `renderReference` and `patchText` of `tools/port/geometry/common.ts`, with the harness's forced settings unless stated), or the library in `node_modules` was run directly. **Unverified** means inferred from the code and not run. Probe scripts and outputs are outside the repository, in `/private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/p2-sankey/`: `probe-parse.mts` (65 inputs, `out-parse/results.json` and one SVG per accepted input), `probe-ent.mts` (9 entity inputs), `probe-plain.mts` (11 names under the default configuration), `probe-proto.mts` (3 names), `probe-fixtures.mts` (18 fixtures and 13 config variants, `out-fixtures/`), and `d3-check.cjs` (d3-sankey 0.12.3 run on the alignment fixture's links).

## 0. Summary

| Item              | Value                                                                                                                                                                                                                                                                                                                                     |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Reference sources | 6 files, 454 lines of TypeScript and JavaScript without the spec (`packages/mermaid/src/diagrams/sankey/`), 66 lines of Jison, and the layout in `node_modules` (d3-sankey 0.12.3, 369 lines of `sankey.js`)                                                                                                                              |
| Specs             | 1 file, 5 test cases (`parser/sankey.spec.ts`). All parse only; no DB or renderer spec                                                                                                                                                                                                                                                    |
| Fixtures          | 18 `.mmd` files: 15 in `nightly-check/`, 3 in `pr-check/`; `visual-check/` is empty                                                                                                                                                                                                                                                       |
| Parser            | Jison, one lexer with three start states. A hand-written port is recommended (ADR 0002). The grammar is small: one header, then one record per line                                                                                                                                                                                       |
| Layout            | `d3-sankey` 0.12.3 (node alignment, six relaxation iterations, link breadths). Pure arithmetic, no randomness; the port must follow its loop order                                                                                                                                                                                        |
| Renderer          | SVG built by d3 calls, one `path` per link with a `linearGradient` per link. Text is SVG `text` with the value on the same element. The root `svg` size comes from `getBBox`                                                                                                                                                              |
| Biggest risks     | (1) the layout's floating-point order (section 6.5); (2) text boxes, which set the root `viewBox` when a label spills past the layout and the boxes of every label (section 6.4); (3) reproduced quirks: NaN geometry for zero, negative or non-numeric values, prototype names as colours, and entities that fail to parse (section 2.6) |
| Effort            | Medium-small. Rough size in section 8: about 1,000 lines of Rust without tests, the layout is the largest part                                                                                                                                                                                                                            |

## 1. Sources

| File                                                                             | Lines                              | Role                                                                                                                               |
| -------------------------------------------------------------------------------- | ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `diagrams/sankey/parser/sankey.jison`                                            | 11-38 lexer, 40-66 grammar         | Lexer (states `csv`, `escaped_text`), grammar, record action                                                                       |
| `diagrams/sankey/sankeyDiagram.ts`                                               | 9-10                               | Wraps `parser.parse` with `prepareTextForParsing`; exports `{ styles, parser, db, renderer }`                                      |
| `diagrams/sankey/sankeyUtils.ts`                                                 | 1-7                                | `prepareTextForParsing`: the only whitespace and newline normalisation                                                             |
| `diagrams/sankey/sankeyDetector.ts`                                              | 6                                  | Detector `/^\s*sankey(?![\w-])/`, no `i` flag                                                                                      |
| `diagrams/sankey/sankeyDB.ts`                                                    | 14-87                              | Module state, `clear` (20-25), `addLink` (40-42), `findOrCreateNode` (48-58), `getGraph` (63-70), default export (72-87)           |
| `diagrams/sankey/sankeyRenderer.ts`                                              | 1-286                              | `draw`: config (69-102), d3-sankey setup (121-129), the DOM (154-279), viewport (281)                                              |
| `diagrams/sankey/styles.js`                                                      | 1-36                               | Diagram CSS; `options.mainBkg`, `options.background`, `options.textColor`                                                          |
| `diagrams/sankey/parser/sankey.spec.ts`                                          | 1-91                               | 5 tests (section 5)                                                                                                                |
| `diagrams/sankey/parser/energy.csv`                                              | 1-99                               | Input of the spec's CSV tests                                                                                                      |
| `diagrams/common/common.ts`                                                      | 48-83                              | `removeScript` (48), `sanitizeMore` (56-69), `sanitizeText` (71-83): DOMPurify over node names, with `FORBID_TAGS: ['style']` (79) |
| `diagrams/common/commonDb.ts`                                                    | 10-14                              | Common `clear`: resets `accTitle`, `accDescription` and `diagramTitle`                                                             |
| `Diagram.ts`                                                                     | 49-54, 77                          | `fromText`: `db.clear()`, front matter title, `encodeEntities(source) + '\n'`                                                      |
| `utils.ts`                                                                       | 874-896                            | `encodeEntities`: `#name;` becomes `ﬂ°name¶ß`, and `#digits;` becomes `ﬂ°°digits¶ß`                                                |
| `preprocess.ts`                                                                  | 58-65                              | `preprocessDiagram`: `cleanupText`, front matter, directives, `cleanupComments`                                                    |
| `diagram-api/comments.ts`                                                        | 6-8                                | `cleanupComments`: removes `%%` lines that do not start `%%{`                                                                      |
| `setupGraphViewbox.js`                                                           | 23-33, 43-46, 49-72                | `calculateSvgSizeAttrs`, `configureSvgSize`, `setupGraphViewbox` (root size and `viewBox`)                                         |
| `rendering-util/uid.ts`                                                          | 1-18                               | `Uid`: a static counter, never reset (section 6.7)                                                                                 |
| `accessibility.ts`                                                               | 27-31                              | `role` and `aria-roledescription` on the root `svg`                                                                                |
| `mermaidAPI.ts`                                                                  | 142-153, 319-332, 407-419, 596-600 | Common CSS (`createCssStyles`, `:root` font variable), `createUserStyles`, root `svg` and its `g`, style insertion                 |
| `styles.ts`                                                                      | 11-47                              | Common CSS of the diagram root (`& { font-family … }`, `@keyframes`)                                                               |
| `utils/sanitizeDirective.ts`                                                     | 1-27, 38-80                        | Dictionary pattern of `nodeColors` (line 11), the dictionary key filter (17-27), the directive key filter (56-60)                  |
| `schemas/config.schema.yaml`                                                     | 464-470, 2599-2660                 | `BaseDiagramConfig.useMaxWidth` (default true), `SankeyDiagramConfig` (defaults 2599-2660)                                         |
| `defaultConfig.ts`                                                               | 318-323                            | `sankey` = the generated defaults, with `nodeColors: undefined`                                                                    |
| `packages/mermaid/dist/chunks/mermaid.esm/chunk-DJLRBVLP.mjs`                    | 5257-5267                          | The generated sankey defaults as built: `useMaxWidth: true`, see section 6.8                                                       |
| `node_modules/.pnpm/d3-sankey@0.12.3/…/src/sankey.js`                            | 1-369                              | The layout (section 6.5)                                                                                                           |
| `…/d3-sankey/src/align.js`, `sankeyLinkHorizontal.js`, `constant.js`, `index.js` | 1-23, 1-15, 1-5, 1-3               | Alignment functions, link path, default accessors                                                                                  |
| `…/d3-shape@1.3.7/…/src/link/index.js`                                           | 52-55, 71-73                       | `linkHorizontal` = `link(curveHorizontal)`: `moveTo`, then one cubic Bézier with the midpoint as both control x values             |
| `node_modules/.pnpm/d3-array@2.12.1/…/src/sum.js`                                | 1-19                               | `sum` skips values that are NaN or zero (`if (value = +value)`)                                                                    |
| `node_modules/.pnpm/d3-scale@4.0.2/…/src/ordinal.js`                             | 1-40                               | `scaleOrdinal`: domain grows on first call; range index `i % range.length`                                                         |
| `node_modules/.pnpm/d3-scale-chromatic@3.1.0/…/categorical/Tableau10.js`         | 3                                  | The ten colours of `schemeTableau10`                                                                                               |
| `e2e/diagrams/sankey/`                                                           | 18 fixtures                        | Screenshot tests through the generic loader; front matter `config`, `description`                                                  |

Mermaid's `d3` import is `d3` 7.9.0 (`packages/mermaid/package.json`), which gives `d3-scale` 4.0.2 and `d3-scale-chromatic` 3.1.0 (the only versions in the store). `d3-sankey` 0.12.3 depends on `d3-array` 2.12.1 and `d3-shape` 1.3.7 (with `d3-path` 1), not on the d3 bundle's copies, so the layout and the link path are the older libraries.

## 2. Parser

### 2.1 Pipeline before the grammar

For a sankey text, the order is: `cleanupText` (CRLF and CR become LF, `preprocess.ts` 60), front matter (its `config` is read; its `title` goes to the DB), `%%{…}%%` directives removed, `cleanupComments` (`preprocess.ts` 65), then `Diagram.fromText` (`Diagram.ts` 77) applies `encodeEntities` and appends a line break. The parser's `parse` is wrapped (`sankeyDiagram.ts` 9-10), so `prepareTextForParsing` (`sankeyUtils.ts` 1-7) runs last:

1. `text.replaceAll(/^[^\S\n\r]+|[^\S\n\r]+$/g, '')`: no `m` flag, so only the blanks at the very start and the very end of the text go. Trailing blanks on the other lines stay (the actions trim the fields anyway).
2. `text.replaceAll(/([\n\r])+/g, '\n')`: every run of line breaks (CR, LF or both, inside quoted fields too) becomes one LF. Blank lines disappear.
3. `.trim()`.

Consequences, probed (P-numbers are in `probe-parse.mts`):

- A line of blanks in the middle is not removed (step 2 needs a break, not a blank). The lexer then sees a `NON_ESCAPED_TEXT` line without three fields, and the input is an error (P56).
- Empty lines in the middle or at the end are removed (P07, P48, P50).
- CRLF text parses like LF (P25).

### 2.2 Lexer

Jison's lexer takes the first rule that matches, in definition order. The `case-insensitive` option (`sankey.jison` 13) makes the keyword match in any case; the detector is what keeps other cases out (section 2.3).

| #   | Rule (`sankey.jison`)                                                                 | State(s)     | Token              | Notes                                                          |
| --- | ------------------------------------------------------------------------------------- | ------------ | ------------------ | -------------------------------------------------------------- |
| 1   | `<INITIAL>"sankey"` (29)                                                              | INITIAL      | `SANKEY`           | Pushes `csv`                                                   |
| 2   | `<INITIAL,csv><<EOF>>` (30)                                                           | INITIAL, csv | `EOF`              |                                                                |
| 3   | `<INITIAL,csv>({CRLF}\|{LF})` (31)                                                    | INITIAL, csv | `NEWLINE`          |                                                                |
| 4   | `<INITIAL,csv>{COMMA}` (32)                                                           | INITIAL, csv | `COMMA`            |                                                                |
| 5   | `<INITIAL,csv>{DQUOTE}` (33)                                                          | INITIAL, csv | `DQUOTE`           | Pushes `escaped_text`                                          |
| 6   | `<INITIAL,csv>{TEXTDATA}*` (34)                                                       | INITIAL, csv | `NON_ESCAPED_TEXT` | Can match the empty string (section 2.4)                       |
| 7   | `<INITIAL,csv,escaped_text>{DQUOTE}(?!{DQUOTE})` (35)                                 | all three    | `DQUOTE`           | Pops                                                           |
| 8   | `<INITIAL,csv,escaped_text>({TEXTDATA}\|{COMMA}\|{CR}\|{LF}\|{DQUOTE}{DQUOTE})*` (36) | all three    | `ESCAPED_TEXT`     | Can match the empty string; commas and line breaks are allowed |

Macros (lines 19-25): `COMMA` U+002C, `CR` U+000D, `LF` U+000A, `CRLF`, `DQUOTE` U+0022, and `TEXTDATA` = U+0020 to U+0021, U+0023 to U+002B, and U+002D to U+007E (the grammar's `[\u0020-\u0021\u0023-\u002B\u002D-\u007E]`). So `TEXTDATA` is printable ASCII without the double quote (U+0022) and the comma (U+002C). Nothing outside U+0020 to U+007E is a text character, which the probes confirm (section 2.6).

### 2.3 Grammar and the record action

```
start   : SANKEY NEWLINE csv opt_eof          (44)
csv     : record csv_tail                     (46)
csv_tail: NEWLINE csv | /* empty */           (47)
opt_eof : EOF | /* empty */                   (48)
record  : field[source] COMMA field[target] COMMA field[value]   (50-57)
field   : escaped | non_escaped               (59-62)
escaped : DQUOTE ESCAPED_TEXT DQUOTE          (64)
non_escaped: NON_ESCAPED_TEXT                 (66)
```

The action (51-56) runs in this order: `source = findOrCreateNode($source.trim().replaceAll('""', '"'))`, then the same for the target, then `value = parseFloat($value.trim())`, then `addLink(source, target, value)`. So a record creates its source before its target (first-appearance order of nodes, section 3).

Consequences of the grammar, probed:

- A header with anything else on its line is an error (`sankey a,b,1`, P04). A header with no records is an error (P05, P06): the grammar needs at least one record.
- The detector requires the lower-case word `sankey`, so `SANKEY` and `Sankey` never reach the parser and report "No diagram type detected" (P02, P03). `sankey-beta` is not detected either (`sankeyDetector.ts` 6; P45).
- The record needs exactly three fields (P20, P21, P57: two or four are errors).

### 2.4 Quoted and empty fields

- `"a,x"` is one field containing a comma (P11). `"a ""q"" b"` is the name `a "q" b` (P12). `"a"""` is `a"` (P37). Only `""` inside quotes is an escape; the replacement is in the action (line 52-53), after `trim`.
- A line break inside quotes is kept in the name, and a run of blank lines inside quotes is collapsed to one break by step 2 of section 2.1 (P46, P47: the name is `a\nb`, and `a\n\n\nb` gives the same).
- Text between the closing quote and the comma is an error (`"a" ,b,1`, P10; `"a"b,c,1`, P53). An unclosed quote is an error (P54).
- **Empty fields are errors**, quoted or not: `""`, `,b,1`, `a,b,` and `,,1` all fail (P13, P14, P15, P58, P59). The grammar does not allow an empty `NON_ESCAPED_TEXT` or `ESCAPED_TEXT` to stand as a field. The mechanism is unverified; the likely cause is that rules 6 and 8 can match the empty string (section 2.2).

### 2.5 Values

`value = parseFloat($value.trim())` (line 54). The JavaScript `parseFloat` rule applies, so the port needs `parseFloat`'s prefix grammar, not a strict number parse:

| Field                                           | Value    | Probe    |
| ----------------------------------------------- | -------- | -------- |
| `10abc`                                         | 10       | P17      |
| `1e2`                                           | 100      | P18      |
| `.5`                                            | 0.5      | P63      |
| `  5  `                                         | 5        | P41      |
| `0x10`                                          | 0        | P62      |
| `x`, `NaN`                                      | NaN      | P16, P61 |
| `Infinity`                                      | Infinity | P40      |
| `-3`                                            | -3       | P19      |
| `1 %% trailing` (the `%%` is not at line start) | 1        | P27      |

### 2.6 Other rejected and accepted text

Rejected (all "Parse error on line N", probed): non-ASCII anywhere, in quoted or unquoted fields (`Käse`, P22, P23); a tab (P24); a double quote in the middle of an unquoted field (`a"b,c,1`, P36); a line consisting only of `%%` (P65); `accTitle:` and `title:` lines, which the grammar has no rule for (P43, P44). **Entities** are a special case: a `#name;` or `#digits;` in the text is turned into the placeholder `ﬂ°name¶ß` or `ﬂ°°digits¶ß` by `encodeEntities` (`utils.ts` 884-893), and the placeholder holds non-ASCII characters, so the lexer rejects the whole line (`a#quot;x,b,1`, `a#9829;x`, `A#amp;B`, `a#lt;b#gt;`, `a#x;y`, and the quoted form, E01-E05, E07, E09). The same text without `#` passes: `a&quot;x` is the name `a&quot;x` as written (E06).

Accepted, with their result: `%%` comment lines are removed before parsing (P26, P64), and a `%%{init}%%` line is removed as a directive (P66). Semicolons and `#` inside unquoted names are text (P51, P52), and `a#,b,1` is the name `a#` (E08, because no `;` follows). A `<` inside a name goes through the sanitiser (section 3).

### 2.7 Errors

Parse errors are Jison's `parseError` text: `Parse error on line N:`, the source line, a caret line, and `Expecting '…', got '…'` (P36 is `Expecting 'NEWLINE', 'EOF', 'COMMA', got 'DQUOTE'`). A cycle is a render error, not a parse error: `circular link` (section 6.6), raised by `d3-sankey` (`sankey.js` 169 and 187). The renderer's error handling is generic.

### 2.8 Quirks to reproduce (with a test input)

| Quirk                                                                       | Test input                                                 | Reference result                                                                                                                                                                   |
| --------------------------------------------------------------------------- | ---------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Detector is case-sensitive                                                  | `SANKEY\na,b,1`                                            | No diagram type detected (P02)                                                                                                                                                     |
| Header alone on its line; at least one record                               | `sankey a,b,1`, `sankey\n`                                 | Parse error (P04, P05)                                                                                                                                                             |
| Blank lines removed, blank-only lines kept as errors                        | `sankey\n\n\na,b,1\n\n` (ok), `sankey\n  \n a,b,1` (error) | P07, P48 ok; P56 error                                                                                                                                                             |
| Empty fields are errors                                                     | `sankey\n,b,1`                                             | Parse error (P14)                                                                                                                                                                  |
| Entities fail                                                               | `sankey\na#quot;x,b,1`                                     | Parse error (E01)                                                                                                                                                                  |
| Non-ASCII fails                                                             | `sankey\nKäse,b,1`                                         | Parse error (P22)                                                                                                                                                                  |
| `parseFloat` value                                                          | `sankey\na,b,10abc`                                        | value 10 (P17)                                                                                                                                                                     |
| `NaN` and zero and negative values and `Infinity` render, with NaN geometry | `sankey\na,b,0`                                            | Parses; the SVG has `NaN` heights, path and text `y`, and `stroke-width="NaN"` (P39, P16, P40); a negative value gives `stroke-width="1"` (P19)                                    |
| Cycles                                                                      | `sankey\na,a,1`, `sankey\na,b,1\nb,a,1`                    | Parses; render error `circular link` (P28, P29)                                                                                                                                    |
| Duplicate links are kept, node values add up                                | `sankey\na,b,1\na,b,2`                                     | Two paths; node values 3 (P30)                                                                                                                                                     |
| Prototype names as colours                                                  | `sankey\n__proto__,B,1`                                    | `fill="[object Object]"` (Q01); `constructor` gives `fill="function Object() { [native code] }"` (P32); `hasOwnProperty` gives `function hasOwnProperty() { [native code] }` (Q03) |
| `nodeColors` drops keys containing `proto` or `constr`                      | front matter `nodeColors: {constructor: '#123456'}`        | Key dropped: the node keeps the prototype colour (Q02; the filter is `sanitizeDirective.ts` 21 and 56-60, applied to front matter config through `addDirective`, `config.ts` 277)  |

## 3. DB

Calls, in order, for one diagram:

1. `Diagram.fromText` calls `db.clear()` (`Diagram.ts` 49). `clear` (`sankeyDB.ts` 20-25) empties `links`, `nodes`, `nodesMap` and calls the common `clear`, which resets `accTitle`, `accDescription` and `diagramTitle` (`commonDb.ts` 10-14).
2. If the front matter has a `title`, `setDiagramTitle` is called (`Diagram.ts` 52-54). The sankey grammar has no title, accessibility or description statement, so `accTitle` and `accDescription` stay `''`, and the title is only the front matter one.
3. For each record, in input order: `findOrCreateNode(source)`, `findOrCreateNode(target)`, `addLink(source, target, value)` (section 2.3).
4. `findOrCreateNode(ID)` (lines 48-58) first sanitises the ID with `common.sanitizeText(ID, getConfig())`, then looks it up in `nodesMap`; a new node is appended to `nodes`. The sanitised value is the key, so two inputs that sanitise to the same text are one node (unverified: derived from the code, not run).
5. `addLink` (40-42) pushes a `SankeyLink` with the two node objects and the value, in input order. Duplicates are kept.
6. `getGraph` (63-70) returns `{ nodes: [{id}], links: [{source: id, target: id, value}] }` as new objects. The renderer calls it once per draw (`sankeyRenderer.ts` 113), and d3-sankey mutates that copy only.

**Sanitising.** `sanitizeText` (`common.ts`) runs with the configuration at parse time. With `htmlLabels` false (the harness's forced setting), `sanitizeMore` skips its block (`common.ts` 57) and returns the text unchanged, and the text goes to `DOMPurify.sanitize(text, { FORBID_TAGS: ['style'] })` (79). With `htmlLabels` true (the default), `securityLevel` strict runs `removeScript` first. Results, probed on the bundle:

| Name                            | Forced settings (`htmlLabels` false)         | Default settings (`htmlLabels` true) | Probe    |
| ------------------------------- | -------------------------------------------- | ------------------------------------ | -------- |
| `<b>x</b>`                      | `<b>x</b>` (drawn as text, so the tags show) | same                                 | P33, D02 |
| `a<b`                           | `a` (the open tag swallows the rest)         | `a`                                  | P34, D03 |
| `a<`                            | not run                                      | `a&lt;`                              | D11      |
| `a&b`                           | `a&b` (no `<`: DOMPurify's fast path)        | same                                 | P35, D04 |
| `a=b`                           | not run                                      | `a=b`                                | D05      |
| `<script>x</script>n`           | not run                                      | `n`                                  | D06      |
| `<img src=x onerror=alert(1)>n` | not run                                      | `<img src="x">n` (the handler goes)  | D07      |
| `<i>it</i>`                     | not run                                      | `<i>it</i>`                          | D10      |
| `&lt;x&gt;`                     | not run                                      | `&lt;x&gt;` as text                  | D09      |
| `a<br>b`                        | not run                                      | `a<br>b`                             | D08      |
| `"a"""` (the name `a"`)         | `a"`                                         | not run                              | P37      |

The output of the sanitiser is HTML, and the label then sets it as text (`text.textContent`, `sankeyRenderer.ts` 218), so the tags show as written. The port reuses `crates/ferrigura-syntax/src/sanitize.rs` (the flowchart's DOMPurify subset, 2,145 lines; its header lists the reproduced cases). Sankey adds nothing new except the `FORBID_TAGS: ['style']` option, which the flowchart also uses.

**Defaults and `clear`.** The DB has no defaults of its own. The module-level `nodesMap` is replaced by `clear` (line 23), but the exported `nodesMap` property (line 73) keeps the Map from module load. Nothing reads `db.nodesMap` (`grep` over `packages/mermaid/src`), so this is dead and harmless: the port must not copy it.

## 4. Canonical model proposal

The fields of the DB state after `Diagram.fromText`, in the canonical style (camelCase, `version: 1`):

```json
{
  "version": 1,
  "title": "",
  "nodes": [{ "id": "a" }, { "id": "b" }],
  "links": [{ "source": "a", "target": "b", "value": 10 }]
}
```

- `title`: the front matter title, or `""`. It is in the DB (`Diagram.ts` 52-54), so the differential run should include it. Nothing draws it (probed: P-cases with a front matter title show no text other than the labels, V09). The coordinator decides whether a title the sankey never shows belongs in the model (section 8).
- `nodes`: sanitised IDs in first-appearance order (section 3, step 4). A node has only `id` (`getGraph` 63-70).
- `links`: input order, with `source` and `target` as node IDs (strings, sanitised), and `value` as the parsed number. A value that is not finite is written as its sentinel string, per [canonical-model.md](../canonical-model.md#numbers): `"NaN"`, `"Infinity"`, `"-Infinity"`. `-0` and `0` agree.
- Left out: `accTitle` and `accDescr` (the grammar cannot set them, so they are always `""`; include them if the coordinator wants the DB's full state), the d3 fields (`x0`, `y0`, `value` of a node, `layer`, `width`, `uid`), since they are the renderer's, and the configuration, which the configuration module owns.

## 5. Specs

`packages/mermaid/src/diagrams/sankey/parser/sankey.spec.ts` (91 lines), 5 test cases, all parse-only:

| Test (line)                                                | What it checks                                                                                       | Port                                        |
| ---------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- | ------------------------------------------- |
| `parses csv` (16-22)                                       | `energy.csv` parses after `cleanupComments` and `prepareTextForParsing`                              | parser test, no assertion beyond "no throw" |
| `parses csv with sankey syntax` (24-30)                    | Same input and body; a duplicate of the test above                                                   | same                                        |
| `allows __proto__ as id` (32-39)                           | `__proto__,A` and `A,__proto__` (a two-link cycle) parse; the cycle is only detected at render (P31) | parser test                                 |
| `allows __proto__ as id with sankey syntax` (41-48)        | The same, duplicate                                                                                  | same                                        |
| `parses the issue #7528 sample with sankey syntax` (78-89) | 19 nodes and 16 links; the names with `'`, `&`, `/` and `-` are kept as written                      | parser and DB test (`getGraph`)             |

The spec calls `sankey.parser.yy = db` and `db.clear()` (`sankey.spec.ts` 11-14), so the DB is the port's test target. No spec covers the renderer, the layout or the styles. The 19 nodes and 16 links of test 5 match the input at lines 51-65.

The spec was not run, to leave the worktree as it was. Its inputs were checked through the bundle instead: `energy.csv`, prefixed as the spec prefixes it (`sankey\n\n `), renders with 52 nodes and 70 links (probed: the `g.node` and `path` counts), and it has `%%` comment lines, leading and trailing blanks, quoted fields with commas, and `"""Biomass imports"""`, which becomes the name `"Biomass imports"` (with the quotes). The issue 7528 sample has 19 nodes and 16 links in its DB, as the spec expects (P-cases of section 2 cover its shapes). The `__proto__` cycle parses (P31). The port should run the spec as written.

## 6. Renderer

### 6.1 Pipeline

`draw(text, id, _version, diagObj)` (`sankeyRenderer.ts` 67-282). The root `svg` exists before `draw` (`mermaidAPI.ts` 410-419: `svg` with `id`, `width="100%"` and `xmlns`, then an empty `g`). The style element is inserted as the first child of the `svg` (`mermaidAPI.ts` 596-600). The renderer then:

1. reads the configuration (`getConfig()`: `securityLevel`, `sankey`) (69-102);
2. selects the `svg` (77-86: a sandbox branch uses the iframe's body; the output is the same, so the port needs only the page branch);
3. builds the d3-sankey generator and runs it on `getGraph()` (113-134);
4. computes the central layer for outlined labels (137) and the colour scale (140);
5. appends, in order: `g.nodes` with one `g.node` per node and a `rect` (154-172), `g.node-labels` with the labels (206-225), `g.links` with one `g.link` per link, each with a `linearGradient` (gradient mode) and a `path` (228-279);
6. calls `setupGraphViewbox(undefined, svg, 0, useMaxWidth)` (281).

### 6.2 DOM, element by element

Probed on `P01` (`a,b,10`, default settings) and the fixture outputs; the attribute order is the order of the calls.

```
svg#id  width="100%" xmlns  style="max-width: 600px;"  viewBox="0 0 600 400"  role="graphics-document document"  aria-roledescription="sankey"
  style                                                  (the common and diagram CSS, section 6.9)
  g                                                      (empty; mermaidAPI.ts 419)
  g.nodes
    g.node  id="node-N" transform="translate(x0,y0)" x="x0" y="y0"        (154-166)
      rect  height="y1-y0" width="x1-x0" fill="colour"                      (167-172)
  g.node-labels  font-size="14"                                             (206)
    text  x  y="(y0+y1)/2"  dy="0em" | "0.35em"  text-anchor  [no class]  content: name + "\n" + value   (legacy, 208-218)
    -- or, for labelStyle outlined (220-222): all text.sankey-label-bg, then all text.sankey-label-fg, same attributes
  g.links  fill="none" stroke-opacity="0.5"                                  (228-232)
    g.link  style="mix-blend-mode: multiply;"                               (236-237)
      linearGradient  id="linearGradient-N" gradientUnits="userSpaceOnUse" x1="source.x1" x2="target.x0"   (gradient mode, 242-247)
        stop  offset="0%"    stop-color=colour(source)                       (249-252)
        stop  offset="100%"  stop-color=colour(target)                       (254-257)
      path  d="M x0,y0 C … x1,y1" stroke="url(#linearGradient-N)" | colour | "source"/"target" colour, stroke-width="max(1, width)"  (275-279)
```

Rules that matter for a port:

- The `node` `id` is `node-<n>` and the gradient `id` is `linearGradient-<n>`, from one global counter (section 6.7). Node ids are assigned in node order before any gradient.
- `x`, `y` on `g.node` are not drawn but are in the DOM (`attr('x', …)`, 165-166).
- The legacy label has no `class` attribute (`attr('class', null)` removes it; `appendLabel`, 213). The text content is `name\nvalue` with a real line break when `showValues` is on, which SVG draws as one line (P01 shows `a\n1`).
- `stroke` of the path is the gradient URL in gradient mode; with `linkColor` set to a colour, it is that colour and no gradient element exists (`links-use-hex-color`, `links-same-color-as-target-node`, `variant V04`).
- The `d` of the path is d3's `linkHorizontal` (`sankeyLinkHorizontal.js` 1-15, which sets the source and target points; d3-shape 1.3.7 `link/index.js` 52-55 and 71-73): `M x0,y0 C xm,y0 xm,y1 x1,y1`, with `xm = (x0 + x1) / 2`. Numbers are written as JavaScript writes them (for example `-2.664530515715871e-14`).
- The root `svg` has no `height` attribute when `useMaxWidth` is true, and it has `width="100%"` and `style="max-width: Wpx;"` (section 6.8).

### 6.3 Layout inputs and mapping

- Nodes: `nodeId` is the name (`d.id`); `nodeWidth` = `nodeWidth` (default 10); `nodePadding` = `nodePadding + 15` when `showValues` is on, else `nodePadding` (default 12, so 27 with values) (`sankeyRenderer.ts` 99-100, 121-124); `nodeAlign` is `sankeyLeft`, `sankeyRight`, `sankeyCenter` or `sankeyJustify` from the config (26-35, 116); `extent` is `[[0,0],[width,height]]` (125-129).
- `width` and `height` (`conf.width`, `conf.height`, defaults 600 and 400) only set the extent of the layout. The drawn size is the bounding box of the drawing (6.8), so `width: 300` gives an `svg` 300 wide only because the nodes span it (`variant V12`).
- No `nodeSort` or `linkSort` is set, so the default sorts of d3-sankey run (section 6.5).

### 6.4 Text measurement

The renderer does not measure text itself. `setupGraphViewbox` reads `svgElem.node().getBBox()` (`setupGraphViewbox.js` 50) after every element is appended. The root `svg` is measured as a whole, so the text labels take part in the `viewBox` and the `max-width`. Two examples, probed:

- Labels inside the layout do not change the box: `P01` is `0 0 600 400`, with labels at `x=16` and `x=584`.
- Labels that spill change it: `outlined-labels` (`a,b,10` and `b,c,10`) is `[0, 0, 631.625, 400]`; `variant V02`, with `b,c,20` so that the outlined label of `a` sits left of the layout, gives `viewBox="-13.375 0 626.375 400"`; the financial flow gives `[-104.8125, 0, 1055.703125, 500]` (fixture `should-render-complete-financial-flow-with-custom-colors`).

So the port needs the bounding box of each text as Chromium's `getBBox` gives it: the advance width of the string at 14 px in the bundled face, the anchor's offset, and the ascent and descent (the `viewBox` `y` of `V02` is `0`, and of `P16` with NaN geometry, `-13`). The harness compares each text's box within 0.05 px, so every fixture with labels is affected (section 7).

The text is one `text` element with the newline in its content. SVG's whitespace rules collapse the newline to a space, so the value follows the name on one line. The collapsed string is what the box must measure, and it is measured: the box of `"a\n8"` is the box of `"a 8"` (`ferrigura_text::measure_box`), 19.906 px, as Chromium's `getBBox` gives the reference. The embedded font subset must cover the collapsed text, space included: a subset cut from `a`, `\n` and `8` has no space glyph, so Chromium draws the space with a fallback glyph of another width, and the label boxes came out 0.42 px off the reference in the first port (the harness caught it). The renderer subsets `collapse_white_space` of each label.

### 6.5 The layout: d3-sankey 0.12.3

The phases of `sankey(graph)` (`sankey.js` 66-75), with the constants and the loops that a port must reproduce in order:

| Phase                                          | Lines            | What it does                                                                                                                                                                | Constants and details                                                                                                                                                                                                                               |
| ---------------------------------------------- | ---------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `computeNodeLinks`                             | 126-147          | `index` per node and link; `sourceLinks`, `targetLinks` in input order; string ids resolved through a Map                                                                   | `find` throws `missing: id` for an unknown id (never reached: ids are created first)                                                                                                                                                                |
| `computeNodeValues`                            | 149-155          | `value = max(sum(out), sum(in))` (or `fixedValue`)                                                                                                                          | `sum` skips NaN and zero (`d3-array` 2.12.1 `sum.js`)                                                                                                                                                                                               |
| `computeNodeDepths`                            | 157-173          | BFS from the sources; `depth` = step; throws `circular link` when `++x > n`                                                                                                 |                                                                                                                                                                                                                                                     |
| `computeNodeHeights`                           | 175-191          | BFS from the sinks along `targetLinks`; `height`                                                                                                                            | same cycle check                                                                                                                                                                                                                                    |
| `computeNodeBreadths`                          | 233-243          | layers, then `py`, then the initial breadths, then the iterations                                                                                                           | `py = min(dy, (y1-y0)/(max column length - 1))` (235)                                                                                                                                                                                               |
| `computeNodeLayers`                            | 193-209          | `x = max depth + 1`; `kx = (x1-x0-dx)/(x-1)`; `layer = clamp(floor(align(node, x)), 0, x-1)`; `x0 = x0 + layer*kx`; `x1 = x0 + dx`; columns; sort if `sort`                 | the align functions (`align.js` 7-23): left = depth; right = `n-1-height`; justify = depth if the node has outgoing links, else `n-1`; center = depth if it has incoming links, else one before its nearest target if it has outgoing links, else 0 |
| `initializeNodeBreadths`                       | 211-231          | `ky = min over columns of (y1-y0-(len-1)*py)/sum(value)`; stacks each column; `y = (y1-y+py)/(len+1)` centres it; `reorderLinks`                                            | `min` from d3-array ignores NaN                                                                                                                                                                                                                     |
| `relaxRightToLeft` / `relaxLeftToRight`        | 269-289, 246-266 | per column, the target (or source) position from its links, weighted by `value * layer difference`; `sort` by `ascendingBreadth` when no `sort` is set; `resolveCollisions` | `alpha = 0.99^i`, `beta = max(1-alpha, (i+1)/iterations)` (238-239); `iterations = 6` (64)                                                                                                                                                          |
| `resolveCollisions`                            | 291-298          | subject = middle of the column; four sweeps                                                                                                                                 |                                                                                                                                                                                                                                                     |
| `resolveCollisionsTopToBottom` / `BottomToTop` | 301-308, 311-318 | push overlapping nodes; a move happens only when `dy > 1e-6`                                                                                                                |                                                                                                                                                                                                                                                     |
| `reorderNodeLinks` / `reorderLinks`            | 320-329, 331-338 | sort the links of the neighbours by the other end's breadth, with `index` as the tie-break                                                                                  | only when `linkSort` is undefined (it is)                                                                                                                                                                                                           |
| `targetTop` / `sourceTop`                      | 341-366          | ideal positions of a link end                                                                                                                                               |                                                                                                                                                                                                                                                     |
| `computeLinkBreadths`                          | 39-52            | `link.y0` and `link.y1` from the node's stacks, with `width = value * ky`                                                                                                   | also called from `sankey.update`                                                                                                                                                                                                                    |

Comparators: `ascendingBreadth` (13-15) compares `y0`; `ascendingSourceBreadth` and `ascendingTargetBreadth` (5-11) compare the other end's `y0` and then `index`. JavaScript's `Array.prototype.sort` is stable, so a Rust port with a stable sort and the same comparators gives the same order.

Defaults that Mermaid does not override: `dx = 24`, `dy = 8`, `iterations = 6`, `align = justify`, `id = index` (`sankey.js` 56-64); Mermaid sets `nodeId`, `nodeWidth`, `nodePadding`, `nodeAlign` and `extent` (121-129), so only `iterations` (6) comes from the library's defaults.

Probed (`d3-check.cjs`): the library, run with Mermaid's settings on the `align-nodes-left` links, gives `c` at `x0 = 200`, `y0 = 13.288167426099502`, height `373.00000000000006`, the same values as the SVG of the bundle (`translate(200,13.288167426099502)`, `h 373.00000000000006`). The fixture's other nodes match too.

Floating point. Node positions include residues from the centring: the `justify-nodes` fixture has `viewBox="0 -2.664530515715871e-14 410 400"`, and the node `translate` values carry the same kind of residue (`-2.6644e-14`). The harness tolerance (0.05 px) covers these, but the port must follow the same operation order to keep the stacked positions within tolerance after six iterations.

### 6.6 Errors from the layout

- A cycle, including a self-loop (`a,a,1`) and any loop of links (`a,b,1` with `b,a,1`, `a,b`, `b,c`, `c,a`), throws `circular link` from `computeNodeDepths` and `computeNodeHeights` (P28, P29, P31, P60). The parse succeeds, so the error is the render's.
- A zero, negative, `NaN` or infinite value does not throw. The sums skip NaN and zero, so the node values become 0 and `ky` becomes `Infinity` or `NaN`, and every position becomes `NaN`. The probes show: `rect height="NaN"`, path `d="M10,NaNC…"`, `stroke-width="NaN"` (for NaN and zero, P16, P39), `stroke-width="1"` for a negative value (`Math.max(1, -Infinity)`, P19), and text `y="NaN"` (P16, P40). The rectangles and the path, with NaN coordinates, add nothing to the root box. The labels with `y="NaN"` still count: Chromium reads the attribute as 0, so the box is the labels' extent (`P16`: `viewBox="16 -13 568 17"`). The port has to write the same strings; the box then follows from Chromium, as the harness measures both sides in Chromium.

### 6.7 Colours, ids and the global counter

- Node colour: `nodeColors[nodeId] ?? colorScheme(nodeId)` (`sankeyRenderer.ts` 149-151). `colorScheme` is `d3scaleOrdinal(schemeTableau10)` (140). The ordinal scale assigns a range index in the order of its first call with each value (`ordinal.js`: `domain.push(d) - 1`, then `range[i % range.length]`). Calls happen in this order: first the rectangles (node order, custom colours skipped). The gradient stops come after, one selection at a time: every source stop, then every target stop (d3 `append` and `attr` act on the whole selection), and every node already has its colour from the rectangles, so the stops add no new value. (Corrected in the port: this paragraph said source then target per link.) So the colour of a non-custom node depends on how many non-custom nodes came before it in node order. Probed: `fallback-colors` (`nodeColors: {a}`) gives `b` `#4e79a7` and `c` `#f28e2c`; `custom-node-colors` (a, b custom) gives `c` `#4e79a7`; `variant V13` gives the 11th non-custom node `#4e79a7` again (`% 10`).
- `schemeTableau10`: `#4e79a7 #f28e2c #e15759 #76b7b2 #59a14f #edc949 #af7aa1 #ff9da7 #9c755f #bab0ab` (`Tableau10.js` 3; the last colour is `bab0ab`, as the source's `colors("…bab0ab")` string has it).
- Prototype names: `nodeColors` is a plain object (`?? {}` at 102, or the configuration's object), so `nodeColors[name]` for a name of `Object.prototype` (`__proto__`, `constructor`, `toString`, `hasOwnProperty`, and the rest) returns that prototype member, which is not nullish and is written into the attribute. The stop and the rectangle get `[object Object]` or `function … { [native code] }`. Probed in section 2.8. The port should reproduce these strings (the text is the JavaScript `String` of the member) or accept that these names are a documented difference. A user colour for such a name is dropped at configuration (section 2.8, Q02).
- Uid: `Uid.next(name)` returns `new Uid(name + ++count)` (`uid.ts` 6-8), a static count that is never reset. The page's first `sankey` gives `node-1`, `node-2`, `linearGradient-3`, and the next diagram on the same page continues from there (probed: the fixture run reaches `node-149` and `linearGradient-240`). A `Uid` used as an attribute value is converted by `toString()` to `url(#id)` (`uid.ts` 15-17). The harness pairs elements without ids (`docs-port/M6-RENDERERS.md`, "How it is checked"), so the port can use fresh-page numbering, which is what a single diagram sees.

### 6.8 The root `svg`: size and viewBox

`setupGraphViewbox(undefined, svg, 0, useMaxWidth)` (`setupGraphViewbox.js` 49-72):

1. `svgBounds = svg.getBBox()` (50), with padding 0, so `width = bounds.width` and `height = bounds.height`.
2. `configureSvgSize` (43-46): when `useMaxWidth` is true, `width="100%"` and `style="max-width: <width>px;"` (the `height` attribute is not written, and the `width` set by `mermaidAPI` stays in place); when false, `height` and `width` attributes (`calculateSvgSizeAttrs` 23-33). The `height` attribute is written after `xmlns` and `width` after the `id`, as the probes show.
3. `viewBox = "x y w h"` from the bounds (67-71).

The effective default of `useMaxWidth` is `true`: the built defaults say `"useMaxWidth": true` (`chunk-DJLRBVLP.mjs` 5258), and every default probe had `max-width` in the style. The schema file says `default: false` in `SankeyDiagramConfig` (`config.schema.yaml` 2623-2625). The built defaults do not follow it. The base `BaseDiagramConfig.useMaxWidth` (lines 464-470, default true) is the value that reaches the build; how the build merges the two is not traced (unverified). The port uses `true`, and the coordinator should tell the docs owner (section 8).

A probed default example: `P01` has `style="max-width: 600px;"` and `viewBox="0 0 600 400"`.

### 6.9 Style sheet

`styles.js` (1-36) gives the diagram CSS. Its options are the theme variables with the configuration's `fontFamily`, which the theme provides:

- `.label` and `.node-labels`: `font-family: <fontFamily>` (2-8).
- `.sankey-label-bg`: `stroke: <mainBkg || background || '#fff'>; stroke-width: 4px; stroke-linejoin: round; paint-order: stroke;` (10-16).
- `.sankey-label-fg`: `fill: <textColor>` (18-21).
- `.node rect`: `shape-rendering: crispEdges` (23-26); `.link`: `fill: none; stroke-opacity: 0.5; mix-blend-mode: multiply` (28-33).

The style element that reaches the SVG is the common CSS and the diagram CSS, both prefixed with `#<id>` by `stylis` (`mermaidAPI.ts` 211-228, `createUserStyles` 319-332). The common part, which every diagram gets, is `styles.ts` 11-47 plus `createCssStyles` (142-153): the `:root` font variable, `@keyframes` and classes such as `.edge-animation-slow`, `.error-text` and `[data-look="neo"]` rules. In the probed SVG (`V02`) the sankey rules above are the last ones before the `.node` rules of the common part; the rest is shared. The redux-color values that show are `textColor` `#28253D` and the outline `#ffffff`. The dark theme is unverified: no sankey fixture uses it.

The style-sheet port belongs to the shared CSS work (M6 renderers, "Styles"), not to this type; the sankey part is 30-40 lines.

### 6.10 Configuration

`config.sankey` (`schemas/config.schema.yaml` 2599-2660; built defaults `chunk-DJLRBVLP.mjs` 5257-5267; `defaultConfig.ts` 318-323):

| Key                | Default          | Use                                            | Where                                        |
| ------------------ | ---------------- | ---------------------------------------------- | -------------------------------------------- |
| `width`            | 600              | layout extent                                  | 90                                           |
| `height`           | 400              | layout extent                                  | 91 (fallback is `defaults.width`: see below) |
| `useMaxWidth`      | true (effective) | root `svg` size                                | 92, section 6.8                              |
| `nodeAlignment`    | `justify`        | `nodeAlign`                                    | 93, 116                                      |
| `showValues`       | true             | label text and `nodePadding` + 15              | 96, 124, 178                                 |
| `prefix`, `suffix` | `''`             | value text                                     | 94-95, 178                                   |
| `nodeWidth`        | 10               | `dx`                                           | 99                                           |
| `nodePadding`      | 12               | `dy`                                           | 100                                          |
| `labelStyle`       | `legacy`         | label layout                                   | 101, 190                                     |
| `linkColor`        | `gradient`       | link paint; a colour string is used as written | 239-273                                      |
| `nodeColors`       | `{}`             | node colour map                                | 102, 150                                     |

Latent: `height = conf.height ?? defaults.width` (line 91). The fallback is the width's default (600), not the height's (400). The merged defaults always provide `height`, so the fallback is unreachable through a valid configuration; the port may write it either way, and the note is for the record.

The configuration's `nodeColors` filter (`utils/sanitizeDirective.ts`, dictionary pattern at 11, applied at 72): a value must match `^#[\da-f]{3,8}$`, `rgb(…)`, `hsl(…)` or a plain letter word (`red`); any other value (for example `rgba(…)`) is removed, and so is any key that starts with `__` or contains `proto` or `constr`. The port's configuration module already has the pattern (`crates/ferrigura-model/src/config/layers.rs` 430).

### 6.11 Randomness, the clock and the DOM

- No randomness, no clock. `Math`: `Math.max`, `Math.min`, `Math.round`, `Math.pow` (d3-sankey 238). `Math.round` is JavaScript's: it rounds half up, so `Math.round(-2.5)` is `-2` (the Rust `round` rounds half away from zero).
- Labels: `Math.round(value * 100) / 100` (line 178), then the JavaScript `Number` to string. Probed: `0.597` gives `0.6`; `280.322` gives `280.32`; `2.675` gives `2.68` (the product `2.675 × 100` is a double that `Math.round` takes up to 268); `1e21` gives `999999999999999900000` (the product `1e23` divided by 100 is `9.999999999999999e20`, which JavaScript writes in full below `1e21`); `1e-7` gives `0`; `Infinity` gives `Infinity`. A Rust port needs the `Number` string rules, the `svg::number` helper the renderers share (M6 renderers) and this `Math.round`.
- The DOM: `d3select` (root and `svg`), `getBBox` (section 6.4), `document.createElement('style')` (`mermaidAPI.ts` 598), and DOMPurify (section 3). The port replaces each by its own: a tree, the text box of `ferrigura-text`, the stylesheet writer, and `sanitize.rs`.

## 7. Fixtures

18 `.mmd` files under `e2e/diagrams/sankey/`: `nightly-check/` 15, `pr-check/` 3, `visual-check/` empty. Each has front matter with a `description` and usually a `config`. The harness's `patchText` puts the forced settings (`htmlLabels: false`, Lexica font, `securityLevel: strict`, `redux-color`) before and after the body.

| Fixture                                                                      | Features                                                                   | Probe result (root box, labels)                        |
| ---------------------------------------------------------------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------ |
| `nightly-check/align-nodes-left`, `…-right`, `center-nodes`, `justify-nodes` | the same two-path graph at width 410, `useMaxWidth: false`, each alignment | `[0,0,410,400]`; `justify` has the `-2.66e-14` residue |
| `custom-node-colors`                                                         | `nodeColors` for a and b                                                   | colours `#ff0000`, `#00ff00`, `#4e79a7`                |
| `custom-node-width`                                                          | `nodeWidth: 20`, `useMaxWidth: false`                                      | `[0,0,600,400]` (the node is 20 wide)                  |
| `default-node-width`                                                         | the default width, `useMaxWidth: false`                                    | `[0,0,600,400]`                                        |
| `fallback-colors-for-unspecified-nodes`                                      | `nodeColors` for a only                                                    | `b`, `c` take `#4e79a7`, `#f28e2c`                     |
| `legacy-label-style-by-default`                                              | `sankey: {}`                                                               | legacy labels                                          |
| `legacy-plain-labels`                                                        | `labelStyle: legacy`                                                       | legacy labels                                          |
| `links-are-gradient`                                                         | `linkColor: gradient`                                                      | one `linearGradient`                                   |
| `links-same-color-as-source-node`                                            | `linkColor: source`                                                        | stroke is the source colour, no gradient               |
| `links-same-color-as-target-node`                                            | `linkColor: target`                                                        | stroke is the target colour, no gradient               |
| `links-use-hex-color`                                                        | `linkColor: '#636465'`                                                     | stroke `#636465`                                       |
| `outlined-labels`                                                            | `labelStyle: outlined`                                                     | `[0,0,631.625,400]`; six texts                         |
| `pr-check/should-render-a-simple-example`                                    | minimal                                                                    | `[0,0,600,400]`                                        |
| `pr-check/should-render-complete-financial-flow-with-custom-colors`          | 800×500, outlined, `showValues`, `$`/`B`, 13 nodes with colours, 12 links  | `[-104.8125,0,1055.703125,500]`, `$205B` style labels  |
| `pr-check/should-render-labels-with-apple-style-outlined-text`               | the same data, 800×500, outlined, no values                                | `[-87.828125,0,1021.734375,500]`                       |

Harness notes. No fixture uses `handDrawn`, a non-layered layout, or a symbol or script that the bundled faces do not cover (`uncoveredScriptReason`): all their text is plain ASCII. So no fixture needs an exclusion. The fixtures use no randomness and no clock, so the page's seeds do not matter. The `Uid` counter differs with the order of fixtures on one page, and the harness ignores ids. Three fixtures set `nodeColors` in their front matter (`custom-node-colors`, `fallback-colors-for-unspecified-nodes`, and the complete financial flow); their names are plain words or `#` hex values, which pass the configuration filter (section 6.10).

The fixtures do not cover: a name that needs sanitising, a quoted field, a zero or negative value, a cycle, an error, `showValues: false`, `prefix` without `suffix`, `nodeAlignment` other than the four in the fixtures, or the dark theme. These come from `variant` probes (section 2 and 6), which the port's own tests should add.

## 8. Risks, effort, open questions

Risks, in order of cost:

1. **Text boxes.** Every label is compared (within 0.05 px) and the root `viewBox` depends on them when a label spills (outlined labels, the financial flow). `ferrigura-text` needs the advance widths, the ascent and descent of the bundled face at 14 px, with Chromium's rounding (the M6 rules). Medium risk: the text function is shared and already used by the flowchart, so the cost is in checking the 14 px case.
2. **Floating point in the layout.** Six relaxation passes with stable sorts and comparators (section 6.5). Following the library's order of operations keeps the positions to the last bit, which is more than the 0.05 px tolerance needs. Low to medium risk, testable with `d3-check.cjs` as the oracle.
3. **NaN and infinity geometry.** Zero, negative and non-numeric values render, with NaN attributes and a root box that ignores them (section 6.6). The port must write the same strings and skip the same elements in its box calculation. Low frequency in practice, but cheap to test.
4. **Parser quirks.** Empty fields, non-ASCII, entities, whitespace-only lines, `parseFloat` (section 2). A hand-written lexer with the three states reproduces them; the Jison parse error text is the hard part (below).
5. **Prototype names.** The colour lookup on a plain object (section 6.7). Reproduce it, or record it as a difference.
6. **Number formatting.** `Math.round`, JavaScript `Number` strings (`e-14`, `1e21`), the `NaN` and `Infinity` strings (section 6.11). Small, but a silent source of differences.
7. **Sanitising.** `<` in names runs DOMPurify's HTML parse (`a<b` gives `a`). The reference subset in `sanitize.rs` covers it; the port inherits its gaps, if any (unverified for sankey-specific inputs).

Effort. Rust, without tests: lexer and parser about 250 lines (the three states, 8 rules, `parseFloat`); DB about 80; d3-sankey about 400 (`sankey.js` 369 lines, plus the align functions and the link path); renderer about 300 (the DOM of section 6.2, the labels, the colour scale, the root size); styles about 40; the shared number and text helpers reused. About 1,000 lines. Tests: 5 spec cases, the 65 parser inputs and the 18 fixtures, and a differential of the layout against `d3-sankey` on captured inputs (`d3-check.cjs` pattern). The layout is the largest single part and is the most testable.

Open questions for the coordinator:

1. **Error text.** Reproduce Jison's messages (`Parse error on line N: … Expecting …`) or only accept and reject with the line? The canonical model says errors carry positions, not text, so the default would be to keep the message out of the differential.
2. **Title in the model.** The DB holds the front matter title, which nothing draws. Include `title` in the canonical model (as planned in section 4) or leave it out?
3. **Where the layout goes.** A module of `ferrigura-render/src/sankey/` or its own crate, like `ferrigura-elk`? The port of `d3-sankey` is used only by this diagram. A crate would allow a differential of the layout on its own.
4. **`useMaxWidth` default.** The schema's sankey block says `false`, the generated default is `true`. The port uses the effective `true`. The docs owner should fix the schema or the comment (`config.schema.yaml` 2623-2625).
5. **The latent `height` fallback** (line 91). Reproduce it (it cannot fire) or write `defaults.height`?
6. **Prototype colours.** Reproduce the `function Object() { [native code] }` strings, or document them as a difference? Reproducing them is cheap; they appear only for prototype names.
7. **The sandbox branch** (`securityLevel: sandbox`, 77-86) appears to leave the output unchanged: both branches select the same `svg`, and only the root differs (read, not probed). Confirm that the port ignores it.
8. **Entities.** The reference fails on `#name;` in names. The port must fail the same way; confirm that this is intended, since users may expect `&amp;`-style names to work. The probes show that `&quot;` (without `#`) is accepted and shown as written (E06).
