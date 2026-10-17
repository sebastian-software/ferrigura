# sankey parser and DB: plan

Status: ported (parser, DB, model, specs, differential). Reference: this worktree's `packages/mermaid/src/diagrams/sankey/` (`parser/sankey.jison`, `sankeyDB.ts`, `sankeyUtils.ts`, `sankeyDiagram.ts`). Research and probes are in [NOTES.md](./NOTES.md); this page says what the port does, which quirks it keeps and which it leaves to the renderer. Quirk ids `S1`-`S15` are used in the code comments and in the `cases.ts` entries.

Scope: the parser, the DB and the model. The layout (`d3-sankey`), the renderer, the styles and the configuration's rendering use are the renderer stage's (NOTES, sections 6 and 8).

## 1. Sources

| File                                       | Used for                                                                            |
| ------------------------------------------ | ----------------------------------------------------------------------------------- |
| `diagrams/sankey/parser/sankey.jison`      | Lexer rules (lines 19-36) and grammar (44-66), in Jison's first-match order         |
| `diagrams/sankey/sankeyDB.ts`              | `findOrCreateNode`, `addLink`, `getGraph`, `clear`, the header setters              |
| `diagrams/sankey/sankeyUtils.ts`           | `prepareTextForParsing`, the whitespace step that the parser's `parse` wrapper runs |
| `diagrams/common/commonDb.ts`, `common.ts` | `setDiagramTitle` and `sanitizeText` (DOMPurify, then `FORBID_TAGS: ['style']`)     |
| `Diagram.ts` (49-77)                       | Order of `clear`, the front matter title, `encodeEntities`, and `+ "\n"`            |
| `preprocess.ts`, `diagram-api/comments.ts` | Line breaks, attribute quotes, front matter, directives, comment lines              |
| `utils.ts` (874-896)                       | `encodeEntities`: `#name;` and `#digits;` become placeholders that are not ASCII    |

The Rust code: `crates/ferrigura-syntax/src/sankey/` (`mod.rs` the entry point, `prepareTextForParsing` and the grammar; `scanner.rs` the lexer; `db.rs` the DB), the model in `crates/ferrigura-model/src/sankey.rs`, and `js_parse_float` (`parseFloat`) in `crates/ferrigura-syntax/src/text.rs`. The shared steps are `preprocess.rs` (`prepare`, `remove_directives`, `cleanup_comments`, `encode_entities`) and `sanitize.rs`.

## 2. Text before the scanner

The steps of `Diagram.fromText`, in the reference's order: CRLF and CR to LF, `="x"` to `='x'` in tags, front matter removed and its `title` read (`prepare`, shared); `%%{ … }%%` directives removed; comment lines removed and the text trimmed at the start (`cleanup_comments`, shared); `encodeEntities` (shared); one `\n` appended. The title is set on the DB here, before the parse.

Then the parser's `parse` wrapper runs `prepareTextForParsing` (`sankeyDiagram.ts` 9-10), which the port applies in `prepare_text_for_parsing`:

1. the blanks (white space other than a line break) at the very start and the very end of the text go; a blank that starts or ends another line stays;
2. every run of line breaks, CR or LF, becomes one LF, inside quoted fields too;
3. the text is trimmed.

So a line of blanks in the middle stays and is an error, while blank lines are removed (S2).

## 3. Scanner

Jison takes the first rule that matches at the position, and the rules run in the grammar's order. The initial state reads the header keyword; `csv` reads the rest of the text, and a quoted field enters `escaped_text`.

| #   | Rule (`sankey.jison`)                                       | State(s)     | Token              | Notes                                                               |
| --- | ----------------------------------------------------------- | ------------ | ------------------ | ------------------------------------------------------------------- |
| 1   | `"sankey"` (29), case-insensitive                           | INITIAL      | `SANKEY`           | Pushes `csv`. No word boundary after it; the detector adds one (S1) |
| 2   | `<<EOF>>` (30)                                              | INITIAL, csv | `EOF`              | Matches only at the end of the text                                 |
| 3   | `{CRLF}` or `{LF}` (31)                                     | INITIAL, csv | `NEWLINE`          | After the preparation there is no CR                                |
| 4   | `{COMMA}` (32)                                              | INITIAL, csv | `COMMA`            |                                                                     |
| 5   | `{DQUOTE}` (33)                                             | INITIAL, csv | `DQUOTE`           | Pushes `escaped_text`                                               |
| 6   | `{TEXTDATA}*` (34)                                          | INITIAL, csv | `NON_ESCAPED_TEXT` | Can match the empty text; the empty match is an error (below)       |
| 7   | `{DQUOTE}(?!{DQUOTE})` (35)                                 | all three    | `DQUOTE`           | Pops                                                                |
| 8   | `({TEXTDATA}\|{COMMA}\|{CR}\|{LF}\|{DQUOTE}{DQUOTE})*` (36) | all three    | `ESCAPED_TEXT`     | Greedy: text, commas, line breaks and `""` pairs, as many as come   |

`TEXTDATA` is printable ASCII (U+0020 to U+007E) without the double quote and the comma (lines 19-25). Rule 6 can match the empty text, and so can rule 8: at a character that neither takes (a tab, a non-ASCII character, or the end of the text inside quotes) the scanner returns the empty token, and the grammar never accepts an empty field (its token is followed by the same position again). The port reports that character as an error at once. The accept and reject results are the same as Jison's, and the position is the first such character.

The scanner returns the tokens of the whole text, ending with `EOF`.

## 4. Grammar and DB calls

```
start    : SANKEY NEWLINE csv opt_eof
csv      : record (NEWLINE record)*
record   : field COMMA field COMMA field       (the action: below)
field    : DQUOTE ESCAPED_TEXT DQUOTE | NON_ESCAPED_TEXT
opt_eof  : EOF | empty      (the scanner always gives EOF; see below)
```

The record's action (`sankey.jison` 51-56) runs once the three fields are read, in this order: `findOrCreateNode` of the source, `findOrCreateNode` of the target, `addLink(source, target, parseFloat(value.trim()))`. A name is `field.trim().replaceAll('""', '"')`; the value is the field's text as written, trimmed. The grammar needs at least one record. The scanner always ends the tokens with `EOF`, so `opt_eof` is taken in every text the port accepts; a text without that token is not reached.

## 5. DB

| Method                   | Behaviour                                                                                             |
| ------------------------ | ----------------------------------------------------------------------------------------------------- |
| `clear`                  | Empties the links and the nodes; the common header texts are reset (`commonDb.ts`)                    |
| `setDiagramTitle`        | `sanitizeText(title)`; called only for a truthy front matter title, before the parse                  |
| `findOrCreateNode(name)` | `id = sanitizeText(name)`; the node is created when no node has that id, and its position is returned |
| `addLink(s, t, value)`   | Pushes the link, duplicates included                                                                  |
| `getGraph`               | The nodes in creation order, with their ids; the links in input order, with the ids of their ends     |

`sanitizeText` runs the DOMPurify subset of `sanitize.rs` (S10). The DB has no defaults of its own. Its `nodesMap` property is dead (NOTES, section 3) and the port does not copy it.

## 6. Model

`SankeyDiagram`, `version` 1, camelCase keys:

- `title`: the front matter title, sanitised, or `""`. The sankey never draws it, but the DB holds it (S13).
- `accTitle`, `accDescr`: always `""`. The grammar has no statement for them, and the model holds them so that it has the DB's header texts.
- `nodes`: `{ id }` for each node, in creation order (the source of a record before its target, in order of first appearance).
- `links`: `{ source, target, value }` in input order, with the ids of the ends. A value that is not finite is its sentinel string (`"NaN"`, `"Infinity"`, `"-Infinity"`), as the other models write it.

Left out: the layout (`x0`, `y0`, `layer`, `width`, the node values), the colours, and the configuration (`config.sankey`, which the configuration module owns).

## 7. Quirks reproduced

| Id  | Quirk                                                                                                                                                                                                                                                 | Where (NOTES)                                 | Tests (`sankey_grammar.rs` unless named)                                                                                                         |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| S1  | The header is `sankey` alone on its line. Other text on the line, or `sankey-beta` and `sankeyfoo`, is an error. The parser takes the keyword in any case; the detector does not (`undetectedCases`)                                                  | P02-P04, P45                                  | `a_header_with_text_on_its_line_is_rejected`, `a_word_glued_to_the_keyword_is_rejected`, `the_keyword_is_taken_in_any_case_by_the_parser`        |
| S2  | Blank lines are removed. A line of blanks between records is an error, and blanks at the very end of the text are removed. A header without a record is an error                                                                                      | P05-P07, P48, P50, P56                        | `blank_lines_are_removed_at_the_start_and_the_end`, `a_line_of_blanks_between_records_is_rejected`, `blanks_at_the_very_end_are_removed`         |
| S3  | CRLF reads as LF                                                                                                                                                                                                                                      | P25                                           | `crlf_reads_as_lf`                                                                                                                               |
| S4  | A record has exactly three fields                                                                                                                                                                                                                     | P20, P21, P57                                 | `a_record_has_exactly_three_fields`                                                                                                              |
| S5  | An empty field is an error (`,b,1`, `a,b,`, `""`). A field of blanks is not: it is the empty name `""`, and a value of blanks is `NaN`; at the very end of the text the blanks are removed, so the field is empty and the text is an error            | P13-P15, P58, P59                             | `an_empty_field_is_rejected`, `a_field_of_blanks_is_the_empty_name`, `a_value_of_blanks_in_the_middle_is_not_a_number`                           |
| S6  | Quoted fields: `""` is one quote (resolved after the trim); a line break is kept and a run of them collapses; nothing may follow the closing quote; the field must close                                                                              | P10-P12, P37, P46, P47, P53, P54              | `a_quoted_field_keeps_its_commas_and_its_quotes_as_written`, `a_quoted_field_takes_nothing_after_its_closing_quote`, `a_quoted_field_must_close` |
| S7  | Text outside printable ASCII is an error, quoted or not (`Käse`), and so is a tab inside a field; a tab at the very end is removed                                                                                                                    | P22-P24                                       | `non_ascii_text_is_rejected_quoted_or_not`, `a_tab_inside_a_field_is_rejected`                                                                   |
| S8  | An entity fails: `#name;` and `#digits;` become placeholders that are not ASCII. `&quot;` without `#` is text                                                                                                                                         | E01-E09                                       | `an_entity_in_a_name_is_rejected`, `a_named_reference_without_the_hash_is_text`                                                                  |
| S9  | The value is `parseFloat` of the field: `10abc` is 10, `1e` is 1, `0x10` is 0, `1_000` is 1, `"1,2"` is 1; `x`, `NaN` and `-` are NaN; `Infinity` and `2e308` are infinite (sentinels)                                                                | P16-P19, P27, P39-P41, P51, P52, P61-P63      | `a_value_is_read_as_parse_float_reads_it`, `a_value_that_is_not_a_number_is_nan`, `a_value_beyond_the_double_range_is_infinite`                  |
| S10 | A name is sanitised as the DB sanitises it: `<script>` is removed with its content, `<img onerror>` loses the handler, an open tag swallows the rest (`a<b` is `a`), `a<` is `a&lt;`, `a&b` and `a=b` are kept. Nodes are keyed by the sanitised name | P33-P35, D02-D11                              | `a_name_with_markup_is_sanitised`, `an_open_tag_swallows_the_rest_of_the_name`, `names_that_sanitise_alike_are_one_node`                         |
| S11 | Duplicate links are kept, and a cycle parses; the layout rejects a cycle with `circular link`, not the parser                                                                                                                                         | P28-P30, P60                                  | `duplicate_links_are_kept`, `a_cycle_parses`                                                                                                     |
| S12 | Prototype names are ordinary ids in the DB (`__proto__`, `constructor`, `hasOwnProperty`). Their colours are the renderer's (NOTES 6.7)                                                                                                               | P31, P32, Q03                                 | `prototype_names_are_ordinary_ids`; `sankey_spec`: `allows_proto_as_id`                                                                          |
| S13 | The front matter title is the DB title, sanitised; an entity in it is not encoded, so it stays as written. Nothing draws it                                                                                                                           | V09; probes of `<b>x</b>`, `a<b`, `FM #9829;` | `the_front_matter_title_is_the_db_title`, `the_front_matter_title_is_sanitised`, `an_entity_in_the_front_matter_title_is_kept_as_written`        |
| S14 | Comment lines and directives are removed before the parse. A `%%` in the middle of a line is text, and a line that is `%%` alone is an error                                                                                                          | P26, P27, P64-P66                             | `comment_lines_and_directives_are_removed`, `a_comment_line_of_one_percent_pair_alone_is_rejected`                                               |
| S15 | The grammar has no `title`, `accTitle` or `accDescr` statement, so such a line is an error. `sankey` is an ordinary name inside a record                                                                                                              | P43, P44                                      | `a_title_or_accessibility_line_is_rejected`, `the_keyword_is_a_plain_name_in_a_record`                                                           |

Two quirks are the port's own probes, not NOTES': a field of blanks is the empty name (S5), and a value of blanks in the middle of a text is `NaN` (S5). Both come from the reference's own result and are in `cases.ts`.

## 8. Specs and differential

- `crates/ferrigura-syntax/tests/sankey_spec.rs`: the reference's five tests of `parser/sankey.spec.ts` (`parses csv`, its duplicate, the two `__proto__` tests, the issue #7528 sample). The reference has no DB spec and no render spec for sankey. The `energy.csv` sample is read from the reference's folder.
- `crates/ferrigura-syntax/tests/sankey_grammar.rs`: one test per quirk above, with the reference's result as the assertion.
- Unit tests: `js_parse_float` (`text.rs`), `prepare_text_for_parsing` (`sankey/mod.rs`), the header texts and the sentinels (`ferrigura-model/src/sankey.rs`).
- `tools/port/export-sankey.ts` runs the reference's Jison parser with the DB, after `preprocessDiagram`, `encodeEntities` and `prepareTextForParsing`, and writes the model. `pnpm port:diff sankey`: 174 inputs (the e2e fixtures, the spec literals, the docs' blocks and `cases.ts`), 127 accepted by the reference, 0 differences. The explicit cases in `cases.ts` are the quirks above, and `undetectedCases` holds the three keyword forms the detector does not take.
- `pnpm port:diff config`: the sankey section is compared (`diff-config.ts`, `NOT_PORTED_SECTIONS`), with one shared change: `ts_defaults.json` sets `sankey.useMaxWidth` to `true`, the reference's effective default (section 9).

## 9. Open questions and what is not reproduced

- **Error text.** Jison's messages (`Parse error on line N … Expecting …`) are not reproduced. The port's messages are its own, with the position, as decision 0002 says. The differential compares only accept and reject.
- **Error positions.** Lines are counted in the text the grammar reads, after `prepareTextForParsing` has removed its blank lines, plus the front matter's lines.
- **`useMaxWidth`.** The schema's `SankeyDiagramConfig.useMaxWidth` says `default: false` (`config.schema.yaml` 2623-2625), but the reference's built defaults say `true` (NOTES 6.8). The port follows the reference (`ts_defaults.json`, the same way the treemap's default is set). The schema comment is for the docs owner to fix.
- **Title in the model.** Included, since the DB holds it. `accTitle` and `accDescr` are included as always empty.
- **The layout's home.** A module of `ferrigura-render/src/sankey/`, or a crate of its own: the renderer stage decides (NOTES section 8, question 3).
- **The latent `height` fallback** (`sankeyRenderer.ts` 91) and the sandbox branch (77-86) are the renderer's.
- **Entities** fail, as in the reference (S8). The reference's users may expect `&amp;`-style names to work; `&quot;` without `#` is text.
