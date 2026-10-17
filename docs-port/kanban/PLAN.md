# P2 kanban parser: plan

Status: the parser, the DB and the renderer are ported (section 10 covers the renderer), with the model, the exporter, the specs and the differentials. Reference: the TypeScript reference of this branch (`packages/mermaid/src`, `diagrams/kanban/`). The research and the probes behind it are in [NOTES.md](./NOTES.md). This page says what the port does and which quirks it keeps. Quirk ids `K1`-`K14` are used in the code comments and in `tests/kanban_quirks_spec.rs`.

One correction to the research: NOTES.md section 2.2 says the keyword `kanban` has no word boundary. The generated lexer says otherwise: `^(?:kanban\b)` with the case-insensitive flag (the reference's `kanban.jison` is compiled by Jison, which adds `\b` to a literal that ends in a word character). The probes agree with the boundary, not with NOTES' stated reason. `kanbanX` is rejected because `KANBAN` is not matched, and `kanban-x` is a section because `-` ends the word. Quirk K1 records the behaviour as the probes show it.

## 1. Sources

| File                                       | Used for                                                                        |
| ------------------------------------------ | ------------------------------------------------------------------------------- |
| `diagrams/kanban/parser/kanban.jison`      | Lexer rules (lines 22-88) and grammar (97-162), in Jison's first-match order    |
| `diagrams/kanban/kanbanDb.ts`              | `addNode` (99-165), `getSection` (25-48), `decorateNode` (202-214), `getType`   |
| `diagrams/common/common.ts`                | `sanitizeText`: DOMPurify with `FORBID_TAGS: ['style']` (71-83)                 |
| `diagram-api/comments.ts`, `preprocess.ts` | `cleanupComments`, front matter and directives, as `Diagram.fromText` runs them |

`getData` (`kanbanDb.ts:54-97`) is the renderer's input. It is not ported here (section 6).

## 2. Text before the lexer

`prepare` removes the front matter and normalises the line breaks, then `remove_directives` and `cleanup_comments` run as in `preprocess.ts`, `encode_entities` replaces the entities by placeholders, and one `\n` is appended (`Diagram.ts:77`). The front matter's title is read and dropped: the kanban DB has no title.

## 3. Scanner

The scanner is a state-stack scanner (`scanner.rs`, the shared `StateStack` of `scan.rs`), because the grammar has start conditions. Rules are tried in the order of `kanban.jison`, and the first match wins. The start conditions are the eight of the grammar: `INITIAL`, and the exclusive states `NODE`, `NSTR`, `NSTR2`, `ICON`, `CLASS`, `shapeData`, `shapeDataStr` and `shapeDataEndBracket`. The last is declared by the grammar and entered by no rule.

| Start state  | Rule (line)                                         | Token and effect                                                                                               |
| ------------ | --------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| INITIAL      | `\@\{` (24)                                         | `SHAPE_DATA` with empty text; enters `shapeData`                                                               |
| shapeData    | `"` (27)                                            | `SHAPE_DATA` with the quote; enters `shapeDataStr`                                                             |
| shapeDataStr | `"` (32)                                            | `SHAPE_DATA` with the quote; leaves                                                                            |
| shapeDataStr | `[^\"]+` (35)                                       | `SHAPE_DATA`; each line break and the blanks after it become `<br/>` (37-38)                                   |
| shapeData    | `[^}^"]+` (40)                                      | `SHAPE_DATA`: no `}`, no `^` (K5), no quote                                                                    |
| shapeData    | `}` (44)                                            | leaves, no token                                                                                               |
| INITIAL      | `\s*%%.*` (48)                                      | `SPACELINE`, the blanks before it, line breaks included (K2)                                                   |
| INITIAL      | `kanban` with `\b` (50)                             | `KANBAN` (K1), case-insensitive                                                                                |
| INITIAL      | `:::` (51)                                          | enters `CLASS`                                                                                                 |
| INITIAL      | `::icon(` (55)                                      | enters `ICON`, case-insensitive                                                                                |
| INITIAL      | `[\s]+[\n]` (56)                                    | `SPACELINE`: the blanks up to the last line break of the run, when that break is not the first character (K14) |
| INITIAL      | `[\n]+` (57)                                        | `NL`                                                                                                           |
| ICON         | `[^\)]+`, `\)` (58-59)                              | `ICON` with its text, which may hold line breaks; `)` leaves                                                   |
| CLASS        | `.+` (52), `\n` (53)                                | `CLASS` with the rest of the line, which leaves; a line break alone leaves, no token                           |
| INITIAL      | `-)`, `(-`, `))`, `)`, `((`, `{{`, `(`, `[` (60-67) | `NODE_DSTART`; enters `NODE`. The bracket is not matched with its closing one (K12)                            |
| INITIAL      | `[\s]+` (68)                                        | `SPACELIST`, whose length is the level                                                                         |
| INITIAL      | `[^\(\[\n\)\{\}@]+` (70)                            | `NODE_ID`: blanks, `%`, quotes, `]`, `-` and `:` are part of it (K3)                                           |
| INITIAL      | end of the input (71)                               | `EOF`                                                                                                          |
| NODE         | `"` followed by a backquote (72), `"` (75)          | enters `NSTR2` or `NSTR`                                                                                       |
| NSTR2        | `[^`"]+` (73), `` `" `` (74)                        | `NODE_DESCR`; leaves                                                                                           |
| NSTR         | `[^"]+` (76), `"` (77)                              | `NODE_DESCR`, line breaks included; leaves                                                                     |
| NODE         | `))`, `)`, `]`, `}}`, `(-`, `-)`, `((`, `(` (78-85) | `NODE_DEND`; leaves                                                                                            |
| NODE         | `[^\)\]\(\}]+` (86)                                 | `NODE_DESCR`                                                                                                   |
| NODE         | `.+` (87)                                           | `NODE_DESCR` to the end of the line: only a lone `}` reaches it (K13)                                          |

A character that no rule takes is a lexical error (`a@b`, `a{b}`, `^` outside shape data). The end of the input is handled as Jison's `next()` does, with its `done` flag: once the input is empty, the rules of the current state are tried on it. In INITIAL the end rule gives `EOF`, and every later call, and every call in another state, gives Jison's raw end, which the grammar accepts only after `EOF`. A text that ends inside a node, an icon, a class or shape data is therefore rejected (K9).

## 4. Grammar and DB calls

```text
start      : spaceLines? mindMap
mindMap    : KANBAN NL? document
document   : (statement stop)+
stop       : (NL | EOF | SPACELINE) (NL | EOF)*
statement  : SPACELIST node shapeData? | SPACELIST ICON | SPACELIST CLASS | SPACELIST
           | SPACELINE | node shapeData? | ICON | CLASS
node       : NODE_ID (NODE_DSTART NODE_DESCR NODE_DEND)? | NODE_DSTART NODE_DESCR NODE_DEND
shapeData  : SHAPE_DATA+ (concatenated)
```

The grammar is a recursive descent over the tokens (`mod.rs`). `mindMap` returns after its document, so a text is accepted only when the whole list is consumed. The DB calls come at the reductions, in document order: `addNode(level, id, descr, shapeData)` for a node statement (the level is the length of its `SPACELIST`, 0 without one), `decorateNode({icon})` for `ICON` and `decorateNode({class})` for `CLASS`. The bracket type of a node only sets the padding of the DB, which the model does not keep, so the port does not compute it.

A node without an id takes its description as its id (`nodeWithoutId`, `kanban.jison:148`). A node with an id and no bracket takes its id as its description.

## 5. DB

### 5.1 `addNode`

1. `id` is `sanitizeText(id)`, or `kbn<n>` when that is empty; the counter `n` starts at 0 for each parse and moves only for those ids (K11).
2. `label` is `sanitizeText(descr)`.
3. The shape data, when there is one (K6, K7, K10): the text is wrapped as a flow mapping when it has no line break, and gets a line break after it otherwise; `yaml::load` with `JSON_SCHEMA`. An empty document or `null` is a rejection (`doc.shape` throws). A scalar or a sequence has none of the fields and changes nothing. A mapping sets, in this order: the shape check, then `label`, `icon`, `assigned`, `ticket` and `priority`, each when its value is truthy.
4. Placement: `getSection(level)` (below). A null section makes the node a section; otherwise its `parentId` is the section's id.

### 5.2 `getSection` (K8)

`sectionLevel` is the level of the first node. Every node that exists is checked against it, and a node with a lower level is an error (`Items without section detected`). The section is the last node at `sectionLevel`, and the new node is a section when its level equals that node's level. Because the check runs over the nodes that already exist, an error appears at the line after the shallow node.

### 5.3 Shape data

- The `shape` field is checked and never stored (K7). A truthy value that is not a string is an error (`toLowerCase` is not a function); a string that is not lowercase, or that has `_`, is an error. A lowercase name is accepted. The `kanbanItem` branch of the reference is unreachable, so no shape ever reaches the DB.
- `label`, `priority` are kept as the YAML value, as the configuration value of `ferrigura-model` keeps numbers, booleans, arrays and objects (K6). `icon`, `assigned` and `ticket` are the value's `String()`: `12` is `"12"`, an array joins its items with commas, an object is `"[object Object]"`.
- Truthiness is JavaScript's: `0`, `false`, `""`, `null` and `NaN` are not set, while an empty array or object is.

### 5.4 Decorations (K4)

`decorateNode` writes to the last node of the list, whatever its level. A decoration with no node before it is a rejection, which the reference reports as a `TypeError`.

## 6. Model

`KanbanDiagram { version, nodes }`, where `nodes` is every node the DB holds, in the order it was added, sections and items alike. A node has `id`, `level`, `label`, `parentId` (absent for a section), `isSection`, and the shape data's `icon`, `assigned`, `ticket`, `priority`, and the decorations' `cssClasses`. Absent fields are left out of the JSON.

Decisions:

- **The shape is not in the model.** The DB never stores one (K7). The renderer picks the section or item shape from `isSection`, which is the reference's `sections` list: a section is the same object as its node.
- **Width and padding are left out.** The DB sets them from `mindmap` (`kanbanDb.ts:101-113`), and `getData` drops them. Nothing else reads them, so the model does not depend on the configuration.
- **`getData` is the renderer's stage.** It drops `priority`, `assigned`, `icon` and `cssClasses` for sections, sanitises labels a second time, and lists the children of each section with their section's id. Its children are the nodes with that `parentId`, which the specs read as `parent_id`. Two sections with the same id list their children under both (probed, NOTES 3.4).
- **Non-finite numbers** in a label or a priority are written as the strings `"NaN"`, `"Infinity"` and `"-Infinity"`, as the differential writes them. Reading such a string back gives a string, so a text label that reads `Infinity` cannot be told apart from the number, as in the differential.

## 7. Quirks reproduced

1. **`K1` The keyword has a word boundary.** `kanbanX` is a rejection; `kanban-x` is a section with the id `-x` at level 0 (`kanban.jison:50`, the generated `\b`).
2. **`K2` A comment takes the blanks before it, line breaks included.** `\s*%%.*` (line 48) starts at a line break: `\n%%c` is one `SPACELINE`.
3. **`K3` Ids keep their trailing blanks, comments and quotes.** `a %% note` is the id `a %% note`; `Todo  ` keeps its blanks. A comment after a bracket form is removed (`a[x] %% note`).
4. **`K4` Decorations apply to the last node,** whatever its level, and a decoration before any node is a rejection.
5. **`K5` `^` is no character of shape data.** Single quotes are ordinary there (only `"` starts a string).
6. **`K6` Numeric and boolean values keep their type** in `label` and `priority`; `ticket`, `assigned` and `icon` are strings through `String()`.
7. **`K7` `shape` is checked and not stored.** The check rejects a capitalised name, a name with `_` and a non-string value; a lowercase name passes.
8. **`K8` The level check looks one node behind.** A last line shallower than the first node is a child, and the error comes with the next line.
9. **`K9` The end of the input.** `kanban` without a newline is a rejection, and `kanban` followed by a newline is accepted with no node. A text that ends inside a node, a class, an icon or shape data is a rejection.
10. **`K10` Empty and odd shape data.** `@{}` is accepted with no effect. `@{` followed by a line break and `}` is a rejection (the document is `undefined`). A document that is a scalar or a sequence is accepted with no effect. A `null` document is a rejection.
11. **`K11` The counter.** An id that sanitises to nothing is `kbn<n>`, and `<script>x</script>` gives an empty label.
12. **`K12` Brackets are not paired.** `a[x)`, `a(x]`, `((x)` are accepted, and the bracket type only sets the padding.
13. **`K13` A lone `}` in a node takes the rest of the line** as one description. `a[x}y]` is a rejection, since the bracket then has two descriptions.
14. **`K14` Blanks that end in a line break are one `SPACELINE`.** `\n\n` and `\n  \n` are single statements, while a single `\n` is an `NL`; the level of a node is the length of the blanks in front of it, counted in characters, so ` ` is one level.

Sanitising uses the default configuration. The reference's `sanitizeText` reads `dompurifyConfig` from the configuration (`common.ts:75`), and the port's `sanitize_text` does not take one. No kanban input in the differential sets it, so this is not reproduced; it is open (section 9).

## 8. Specs and differential

- `tests/kanban_spec.rs`: the 22 tests of `when parsing a kanban`. The children of a section are the nodes whose `parent_id` is its id, which is `getData`'s filter; `getData` itself is not ported. The expected throw of KNBN-6 becomes a rejection at the statement that raises it (decision 0002).
- `tests/kanban_db_spec.rs`: the 8 tests of `item data data`, read from the DB.
- `tests/kanban_quirks_spec.rs`: one assertion per quirk of section 7 that the specs do not reach.
- `tools/port/export-kanban.ts`: the reference's own grammar and DB, run on each input. The DB has no accessor for its node list, so the exporter reads it through a line its loader appends to `kanbanDb.ts` (`exposeNodes`, `openReference`'s extra plugin; the reference's source is not changed).
- `tools/port/cases.ts`: the explicit kanban inputs, one for each rule of section 3 and each branch of section 5, and the inputs the research probed. `KANBAN` and `Kanban` are undetected cases: the reference's detector is case-sensitive.
- `diff-config.ts` now compares the kanban section, which the reference computes for its inputs; it agrees.

Differential, at the time of writing: 337 inputs, 264 accepted by the reference, 0 differences, 0 named exceptions. A random run of about 12,000 generated inputs (mostly valid documents, and a share of mutations) agreed on every input as well; those inputs are not committed.

## 9. Open questions

- **`dompurifyConfig`.** The sanitiser of the port takes no configuration (section 7). A kanban text that sets it is not reproduced.
- **The renderer's data.** `getData` (section 6) is the renderer stage's input. Its duplicate-child behaviour and its dropped section fields belong to that stage.
- **Front matter and directives** change the configuration, which `ferrigura-syntax`'s `config` module reads (checked by `pnpm port:diff config`). The model does not read the configuration, apart from the sanitiser (above).

## 10. Renderer

The renderer is `kanbanRenderer.ts` (`draw`) with `getData` (`kanbanDb.ts:54-97`), the `kanbanSection` and `kanbanItem` shapes and `styles.ts`. Its files: `crates/ferrigura-render/src/kanban/` (`mod.rs` for the layout, `data.rs` for `getData`, `style.rs` for the rules) and `crates/ferrigura-render/src/unified/shapes/kanban_section.rs` and `kanban_item.rs`. The labels are `unified::label`, Markdown and start-anchored, as the kanban stylesheet does not centre `.label text`.

Checks, at the time of writing:

- `pnpm port:geometry kanban`: 29 of 29 within tolerance. The 11 fixtures, and 18 probes under `tools/port/geometry/probes/kanban/` (items, priorities, tickets, assignees, markdown and HTML in labels, special characters, blank and wrapped titles, duplicate section ids, an empty board, the section width, the `neo` look, the dark theme and a ticket base URL). The probes are not e2e fixtures: every `.mmd` under `e2e/diagrams` is an image snapshot test.
- `pnpm port:diff kanban-data`: `getData` on 354 inputs (the fixtures, the probes, the specs, the docs and the explicit cases), 281 accepted by the reference, 0 differences.
- `pnpm port:geometry-flowchart`: 298 of 298, unchanged.
- Specs: `crates/ferrigura-render/tests/kanban_render.rs` (the reference's numbers for a few fixtures and probes, and the markup of the link, the icon background and the priority bars) and `kanban_data_spec.rs` (what `getData` keeps and drops).

Quirks reproduced, numbered here (the code comments name the reference's lines):

1. `R1` A section's class is `section-N`, numbered from 1, and its colour is `cScale<N+1>`: the rules number the slots from `i - 1` (`styles.ts` 24-26).
2. `R2` Classes are dropped: a section's group is `cluster undefined section-N`, an item's is `node undefined`, and no item has a `data-look`.
3. `R3` A repeated section id lists its children under each copy, so the items are drawn once per copy (`kanbanRenderer.ts` 66-67).
4. `R4` A section's rectangle is `sectionWidth` wide whatever its title, and its title is laid out at that width.
5. `R5` An item is placed from the box of its group, which a long ticket can take beyond the rectangle (`kanbanRenderer.ts` 81-84).
6. `R6` An item with an icon gives its title a background, which makes the item 4 px taller. The icon itself is not drawn (`labelHelper`'s `addBackground`).
7. `R7` A priority draws a bar. `Medium`, a value that is not a name, and a number have no stroke colour (`colorFromPriority`).
8. `R8` The ticket link replaces the first `#TICKET#` as `String.replace` does, expanding its `$` patterns. The link has no `target` in the reference's output.
9. `R9` The sections' and items' labels are start-anchored; a section's title is centred over its rectangle by its group's translation (`clusters.js` 391-396).

Not covered:

- `look: handDrawn`: the harness excludes it, and the port draws the classic geometry (as `unified/shapes/mod.rs` does for the other types).
- `kanban.padding`: the reference reads it only as a fallback that the defaults never reach (section 6.3).
- The link's URL is written as configured. The reference's output goes through its sanitiser, which may change an unsafe URL; not probed.
- `dompurifyConfig` in the sanitiser, as in the parser (section 9).
