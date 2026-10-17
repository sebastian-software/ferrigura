# ARCHITECTURE parser and DB: plan and decisions

The parser task of `architecture-beta` (P2). The research, with its sources and probes, is in [NOTES.md](./NOTES.md); this page records what the port does, the decisions it took, and every quirk of the reference that it reproduces. The parser is `crates/ferrigura-syntax/src/architecture/` (scanner, grammar, DB), the model is `crates/ferrigura-model/src/architecture.rs`, and the exporter is [`tools/port/export-architecture.ts`](../../tools/port/export-architecture.ts).

The reference is the grammar `packages/parser/src/language/architecture/` with Langium 4.4.0, and the DB `packages/mermaid/src/diagrams/architecture/architectureDb.ts` and `architectureParser.ts`. The decision to write the parser by hand is [0002](../decisions/0002-hand-written-parsers.md).

## 1. Scope

The parser task covers the grammar, the preprocessing it shares with the other types, the DB calls with their checks and messages, and the model. The layout (fcose), the drawing (SVG, icons, styles, the seeded generator) and the configuration's use in the renderer are the renderer stage, sections 6 and 8 of NOTES.md.

## 2. The lexer

Langium 4.4.0 builds the lexer with `DefaultTokenBuilder.buildTokens` (`lib/parser/token-builder.js`). The order of the token types, read from the reference's own lexer (`createArchitectureServices().Architecture.parser.Lexer.definition`), is:

1. The keywords, sorted by descending length: `architecture-beta`, `architecture`, `junction`, `service`, `column`, `group`, `align`, `row`, `--`, `in`, `:`, `-`. The two header keywords get the mermaid word-end suffix `(?:(?=%%)|(?!\S))`.
2. The terminals of the reachable rules, in the order of the grammar: `ARROW_DIRECTION` (`L|R|T|B`), `ARROW_GROUP` (`{group}`), `ARROW_INTO` (`<|>`), `ACC_DESCR`, `ACC_TITLE`, `TITLE`, `STRING`, `ID`, `NEWLINE`, `WHITESPACE` (skipped), `YAML`, `DIRECTIVE` and `SINGLE_LINE_COMMENT` (hidden), `ARCH_ICON`, `ARCH_TITLE`.

Chevrotain takes the first match, not the longest. A keyword has a `LONGER_ALT`: the ID terminal for a word (`in`, `group`, `row`, …, both headers), and the YAML block for `--` and `-`. The alternative wins only when it matches longer. A terminal has no alternative, and `BOOLEAN`, `INT` and `FLOAT` of the common grammar are not reachable, so they are not in the lexer.

The scanner in `architecture/scanner.rs` is written in that order, and each terminal's regex is written as its own function: `id_len`, `string_len`, `icon_len`, `arch_title_len` (the quoted form first, then the run up to `[`, `]` or a line break), `acc_descr_at`, `acc_title_at`, `title_at`, and the comment and YAML lengths that `crate::lexer` already has.

Consequences that the reference shows, and the port keeps (section 9):

- A word that begins with `L`, `R`, `T` or `B` is a side letter and then the rest, so `Lx` and `Bob` cannot be ids.
- A word that begins with `title` is the `title` line and then the rest, so `title1` and `title` as ids are rejected. `accTitle` and `accDescr` are ids unless a colon or a brace follows.
- `architecture` is an id when a parenthesis follows it, and a keyword only before a blank, a comment or the end of the text.
- A block that starts with `---` on its own line is hidden as YAML in the body, and an unclosed one is a lexer error.

## 3. The model

The model is the DB state after the parse, as NOTES.md section 4 proposes, camelCase, with `version: 1`:

- `title`, `accTitle`, `accDescr`: the sanitised header texts (section 4, the sanitising rules).
- `groups`, `services`, `junctions`, `edges`, `alignments`: the DB's lists, in the order the DB holds them. Groups are in text order. Services and junctions are separate lists, each in text order.
- Each group, service and junction has `in` (the JSON name of the enclosing id, `""` at the top) and the fields the DB keeps (`icon`, `iconText`, `title`). Absent strings are `""`, as the canonical rule says.
- Each edge has `lhsId`, `lhsDir`, `lhsInto`, `lhsGroup`, `rhsId`, `rhsDir`, `rhsInto`, `rhsGroup` and `title`. The flags are `false` when the token was not there.
- `derived`: `groupAlignments` and `spatialMaps`, computed as `getDataStructures` does (section 6).

Left out, and why: the icon's drawing (names stay names, the renderer draws them, section 8 of NOTES.md); the node's `edges` lists, which are a cycle in the model; the adjacency map, which is reproduced through the spatial maps; the element map and the diagram id (renderer objects); the configuration (compared by `pnpm port:diff config`, section 7).

## 4. The DB calls and their checks

`populateDb` (`architectureParser.ts`) runs in this order, and the first failing call is the error:

1. The header texts, when non-empty: `setAccDescription`, `setAccTitle`, `setDiagramTitle`. The front matter title is set before the parse, and the grammar's title replaces it when non-empty. The setters sanitise with DOMPurify (`sanitize.rs`); `setAccTitle` removes the leading whitespace, and `setAccDescription` the whitespace after each line break.
2. Every group, in text order: `addGroup`. The id must be free. The parent, when given, must be a group that is registered already, and not the group itself.
3. Every service, in text order: `addService`. The same id and parent checks, and the parent must be a group.
4. Every junction, in text order: `addJunction`. The same checks as a service.
5. Every edge, in text order: `addEdge`. Both ends must be registered. An end that is a group is rejected (section 9, Q26). A `{group}` mark must cross two groups: when both ends have a parent and the parents are equal, the mark is an error.
6. Every alignment, in text order: `addLayoutHint`. Two members at least, each a registered service or junction, none twice.

Groups are added before services and junctions whatever the text order, so a service may name a group declared later in the text, and a group may not name a group declared later (section 9, O1 to O7).

Errors: the DB messages of the reference are kept, with the position of the statement that caused them. The grammar's errors are the lexer's and the parser's, at the offending token. Any error is a rejection, and the differential counts a rejection on both sides as agreement.

The DB is not shared between parses: `db.clear()` resets the DB and the header texts, as `Diagram.fromText` does before each parse.

## 5. Sanitising and defaults

The header texts are the DB's values. The reference's common setters sanitise every header text, so `title Hello <b>bold</b> & more` is kept as `Hello <b>bold</b> &amp; more` and `accDescr: x<img src=x onerror=…>` loses its `onerror`. The service, group and junction texts are not sanitised by the DB, and the architecture renderer always draws SVG text, so they are kept as written. The entity placeholders (`#35;` becomes `ﬂ°°35¶ß`) are kept.

The defaults: an absent icon, icon text, title or parent is `""`, and an absent edge flag is `false`. The renderer treats `""` and `undefined` alike, so this loses nothing.

## 6. The derived block: `getDataStructures`

Memoised in the reference, derived after the parse, and read by the layout. The port computes it in `db.rs` (`Db::derived`) in the reference's order.

- **Group alignments.** For each node, for each of its edges (a self-loop is listed twice), when the two ends are in different groups and the sides are not a bend (one horizontal and one vertical side), the key `"A"-"B"` (the two ids sorted and JSON-quoted) is set to `horizontal` or `vertical`. A later edge between the same groups overwrites the value and keeps the key's place (pinned by a test).
- **Adjacency.** For each node in the DB's node order (services, then junctions), a map from the side pair to the neighbour, in insertion order. The pair starts at the node's own side. A pair of the same side twice (`LL`, `RR`, `TT`, `BB`) is no pair. A second edge with the same pair from the same node overwrites the neighbour and keeps the pair's place.
- **Spatial maps.** A breadth-first search from the first node that is not yet taken, one map per component. The start is at `(0, 0)`. A node's `visited` flag is set when it is taken from the queue, not when it is queued, so a node can be queued twice and placed again; the last placement wins (pinned: `c` at `(1,-1)` after `a:B -- T:c` and `b:B -- T:c`). A pair that two edges give the same node places it once per pair, and the last placement wins.

The model keeps the group alignments as a list of `[key, axis]` pairs and the spatial maps as lists of `{id, x, y}`, both in the order the reference's maps hold them. The indexed maps are `IndexMap`, whose `insert` keeps the place of a key, as `Map.set` does.

## 7. Configuration

The `architecture` section of the configuration is compared by `pnpm port:diff config` (the group `architecture` in `diff-config.ts`), now that the type is ported; it passes with 281 inputs. The parser reads no configuration: `getConfig` is read by the renderer (fcose, `randomize`, the style options). The DB's own tests of the defaults and of `setConfig` are therefore not in the parser's specs (section 10).

## 8. The renderer stage

The renderer is `crates/ferrigura-render/src/architecture/`: [`scene.rs`](../../crates/ferrigura-render/src/architecture/scene.rs) (the cytoscape state and the functions the renderer reaches: label boxes, compound sizes, endpoints, bends, the element bounds), [`layout.rs`](../../crates/ferrigura-render/src/architecture/layout.rs) (the two runs of fcose through `ferrigura-cose`, and the segment handler between them), [`constraints.rs`](../../crates/ferrigura-render/src/architecture/constraints.rs) (`getAlignments` and `getRelativeConstraints`), [`draw.rs`](../../crates/ferrigura-render/src/architecture/draw.rs) (the SVG, and the boxes that set the viewBox), with [`icons.rs`](../../crates/ferrigura-render/src/architecture/icons.rs) and [`style.rs`](../../crates/ferrigura-render/src/architecture/style.rs).

Checked with `pnpm port:geometry architecture`: the 20 fixtures and the probes under [`tools/port/geometry/probes/architecture`](../../tools/port/geometry/probes/architecture) (the hand-written ones `p01` to `p13`, and twelve generated diagrams `gen01` to `gen12`), 45 of 45 within 0.05 px. A further run of 60 random generated diagrams, added for the check and then removed, was within tolerance too (104 of 104 with the 44 above). The specs that test what the renderer builds (the ids of `svgDraw.spec.ts`, the seed of `architectureSeed.spec.ts`) are in [`crates/ferrigura-render/tests/architecture_render.rs`](../../crates/ferrigura-render/tests/architecture_render.rs).

Reproduced quirks of the renderer (each one measured against the harness):

- A group with children draws its title, but the title does not count in the bounds of the group or of its parent: the group's box is its body, as the reference's rectangles show. A service's title counts. A childless group's title counts too, placed on its centre line (no padding offset).
- The SVG text takes the theme's `fontSize` (14 px in `redux-color`), while cytoscape measures the node labels at the configuration's `fontSize` (16 px) and the edge labels at 16 px: two sizes, one per use.
- The XY edge label uses the `dominant-baseline` `auto` after its turn, and the alphabetic baseline for the group titles (`start` is not a `dominant-baseline` value).
- The unknown icon's question mark keeps the reference's font list, `ArialMT, Arial`. In the harness that name is the machine's Arial, as it is for the flowchart's fallback icon, whose baseline (298 of 298) would move if `ArialMT` were mapped onto the bundled face. The glyph lies inside the 80-unit square in Arial's metrics, so the square bounds it.
- Icons: the unknown icon for a name that is not in the pack, or another prefix.

The fcose corpus in `crates/ferrigura-cose/tests/fcose-corpus` was captured through the e2e viewer, where `Helvetica Neue` is the Mac's system font, so 19 of its 419 renders differ in a label's width by a pixel (for example `Link 5`: 42.99 px in the system font, 43.15 px in the bundled face). The geometry harness maps `Helvetica Neue` to the bundled face, as the task asks, and the renderer matches it; the corpus engine tests are unaffected, because they take the captured inputs.

## 9. Reproduced quirks

Each of these is pinned by a test in `crates/ferrigura-syntax/tests/architecture_grammar.rs` or `architecture_db_spec.rs`, and by the explicit cases of `tools/port/cases.ts`, which the differential compares.

Lexer and grammar (NOTES.md section 2.3, Q1 to Q38):

- Both header keywords are accepted; the header may share its line with a statement; the text may end without a line break.
- Exact keyword ids are rejected (`group`, `in`, `row`, `align`, `junction`, `column`, `service`); a longer word beginning with a keyword is an id (`rowspan`, `columnar`, `inner`, `groupie`).
- An id cannot end with a dash; it may begin with a digit or hold dashes and underscores; keywords are case-sensitive.
- An icon is written in parentheses with no quotes and no spaces, and its name holds colons (`logos:aws`); a service's icon text is a quoted string without parentheses, and it may span lines.
- A title is in brackets, needs one character, and may hold brackets inside quotes, commas and parentheses.
- Two statements on one line are rejected.
- An edge has the form `end:SIDE [<] (-- | -[label]-) [>] SIDE:end`; the sides are the capital letters `L R T B`; the label is never empty; `---` is not an edge; the arrow heads may sit on either side; the same side on both ends is accepted.
- A junction has no title; an alignment needs two members, which are services or junctions.
- A line comment, a directive in the body and tabs are accepted.

Lexer order (section 2):

- `Lx`, `Bob` and every id that begins with `L R T B` are rejected.
- `title1`, `title` and every id that begins with `title` are rejected, as `architecture(x)`, `architecturex` and `architecture-betax` are not headers.
- `service accTitlex` is an id; `service architecture(server)` is an id; `service architecture` is rejected.
- `---` on its own lines is hidden as a block in the body when a closing `---` follows; without the closing fence the dashes are two keywords, and the text is rejected.

DB order and checks (section 2.3, O1 to O7 and the checks of section 4):

- Groups are added first: a service or a junction may name a group declared later, and a group may name an earlier group only.
- A group that names a service is rejected with `parent does not exist`, since the services are not registered yet when the groups are added.
- A service may name only a group: a service declared earlier is `parent is not a group`, a junction is not registered yet (`parent does not exist`), and the service itself is `cannot be placed within itself`.
- A junction may name only a group: a service or an earlier junction is `parent is not a group`.
- An edge may name a service or junction declared later, and a junction declared later.
- An edge to a group is rejected (the reference throws a `TypeError`).
- An edge whose `{group}` mark crosses one group only is rejected.
- An alignment may name members declared later, but never a group, never the same member twice, and never fewer than two.

Header texts (section 5): sanitised by the common setters; a blank last `title` line lets the front matter title through, as the grammar's last value is checked for truth.

Derived block (section 6): the group alignment overwrite; the adjacency overwrite; the visited-on-dequeue placement; the bend that is no alignment; the self-loop that is no alignment.

## 10. Specs and the differential run

- **Specs.** `architecture_spec.rs` ports `architecture.spec.ts` (26 passing; the parser and DB tests, through the public parse, with the DB-level calls written as the statements that make them), and keeps the configuration tests and the stylesheet warnings ignored with their reasons (5 ignored). The renderer's specs (the seed of `architectureSeed.spec.ts` and the ids of `svgDraw.spec.ts`) are in `crates/ferrigura-render/tests/architecture_render.rs`. `architecture_grammar.rs` (51 passing) pins the quirks above. `architecture_db_spec.rs` (20 passing) pins the order, the header texts and the derived block. The alignment check that no statement reaches is a unit test in `architecture/db.rs`.
- **Differential.** `pnpm port:diff architecture`: 281 inputs (the e2e fixtures, the spec literals, the docs, and the explicit cases), 170 accepted by the reference, 0 differences, 0 named exceptions. The explicit cases are the probes of section 9, and an `undetectedCases` entry for the capitalised header.
- **Random checks, not committed.** Beyond the committed inputs, a seeded generator of mostly valid diagrams (groups and nested groups, junctions, edges with every side, arrow form and `{group}` mark, alignments, and rare invalid statements) produced 6000 diagrams, 1092 of the first 1500 accepted, with 966 of those having edges and 131 having group alignments. A mutator that edits valid diagrams with lexer-sensitive pieces produced 15000 mutants, 909 of them accepted. The two sides agreed on every one. The header texts (sanitising, entity placeholders, front matter precedence) were checked on 39 further inputs. None of these random inputs is committed.

## 11. Open questions

- The error messages of the lexer and the parser are ferrigura's own (`unexpected character`, `expected an id`, and so on), as decision 0002 allows. Langium's positions are not reproduced; the line and column are those of the token that the hand-written parser stops at.
- Whether a grammar error of the reference should be reproduced verbatim for the user is not decided; the differential only compares acceptance.
