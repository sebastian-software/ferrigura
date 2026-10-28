# Kanban Diagram

The kanban draws a board of columns with cards. It starts with `kanban`; each line is a node, and its indentation decides whether it is a column (a section, the level of the first node) or a card (an item) in the section above it. A node is `id`, `id[Text]` or `[Text]`, followed by shape data `@{ assigned: "x", ticket: T-1, priority: "High" }` and decorations (`::icon(…)`, `:::class`) on the lines below. ferrigura parses it with a hand-written state-stack scanner and recursive-descent grammar in `ferrigura-syntax/src/kanban/`, and draws it with the type's own renderer in `ferrigura-render/src/kanban/`, whose section and item shapes are in the unified renderer's shape folder. No layout engine is involved: the layout is arithmetic on the sizes of the labels.

## Model

`ferrigura_model::kanban::KanbanDiagram` is the DB state after the parse ([canonical-model.md](../canonical-model.md#kanban) has the conventions). It holds `nodes`: every node the DB holds, in the order it was added, sections and items alike. A node has:

- `id`: the sanitized id, with the blanks and comments the lexer kept. An id that sanitizes to nothing is `kbn` and a counter that starts at 0 for each parse and moves only for those ids.
- `level`: the number of blanks in front of the node, counted in characters.
- `label`: a text label is sanitized; a label from the shape data keeps its YAML type (a number, a boolean, an array or an object). A non-finite number is its sentinel string.
- `parentId`: the id of the section the node belongs to; absent for a section. `isSection` marks a section.
- `icon`, `assigned`, `ticket`, `priority`, `cssClasses`: absent when not set. `icon`, `assigned` and `ticket` from shape data are the `String()` of the YAML value (`12` is `"12"`, an array joins its items with commas, an object is `"[object Object]"`); `priority` keeps its YAML value; `icon` and `cssClasses` from `::icon(…)` and `:::…` are sanitized.

Left out: the shape, because the DB never stores one (its only shape check leaves it out; the renderer picks the section or item shape from `isSection`); the width and padding the DB takes from the mindmap configuration, which `getData` drops; the title, which the kanban DB does not have, so the front matter's title is dropped. The layout data that the renderer reads is `getData`, compared by its own run (below).

## Parsing

`kanban/mod.rs` runs the preprocessing steps of `Diagram.fromText` and a recursive descent over the tokens of `scanner.rs`; the DB calls (`db.rs`) happen at the reductions in document order: `addNode(level, id, descr, shapeData)` for a node (the level is the length of its leading blanks, 0 without), `decorateNode({icon})` and `decorateNode({class})` for decorations. The grammar returns after its document, so a text is accepted only when all of it is consumed.

```text
start      : spaceLines? mindMap
mindMap    : KANBAN NL? document
document   : (statement stop)+
statement  : SPACELIST node shapeData? | SPACELIST ICON | SPACELIST CLASS | SPACELIST
           | SPACELINE | node shapeData? | ICON | CLASS
node       : NODE_ID (NODE_DSTART NODE_DESCR NODE_DEND)? | NODE_DSTART NODE_DESCR NODE_DEND
shapeData  : SHAPE_DATA+
```

The scanner ports `kanban.jison` rule by rule, in Jison's first-match order, with the shared `StateStack` of `scan.rs`: the states are the initial one and the exclusive `NODE`, `NSTR`, `NSTR2`, `ICON`, `CLASS`, `shapeData`, `shapeDataStr` (the grammar also declares `shapeDataEndBracket`, which no rule enters). A character no rule takes is a lexical error. The end of the input follows Jison's `done` flag: in the initial state it gives `EOF`, in any other state it gives Jison's raw end symbol, which the grammar accepts only after `EOF`, so a text that ends inside a node, an icon, a class or shape data is rejected. Shape data is read by `yaml.rs` as js-yaml's JSON schema.

Quirks that ferrigura keeps:

- The keyword has a word boundary (Jison adds `\b` to a literal that ends in a word character): `kanbanX` is rejected, `kanban-x` is a section with the id `-x`. `KANBAN` and `Kanban` are parsed but not detected: the detector is case-sensitive. `kanban` without a newline is rejected, and `kanban` followed by a newline is accepted with no node.
- A comment takes the blanks before it, line breaks included (`\s*%%.*`): `\n%%c` is one `SPACELINE`. Blanks that end in a line break are one `SPACELINE`, so `\n\n` and `\n  \n` are single statements, while a single `\n` is an `NL`.
- Ids keep their trailing blanks, comments and quotes: `a %% note` is the id `a %% note`, and `Todo  ` keeps its blanks. A comment after a bracket form is removed. An id may hold blanks, `%`, quotes, `]`, `-` and `:`.
- Brackets are not paired: `a[x)`, `a(x]` and `((x)` are accepted, since the bracket type only sets a padding the model leaves out. A lone `}` in a node takes the rest of the line as one description, so `a[x}y]` is rejected.
- A node without an id takes its description as its id; a node with an id and no bracket takes its id as its description.
- `^` is no character of shape data, and single quotes are ordinary there (only `"` starts a string, in which each line break and the blanks after it become `<br/>`).
- Shape data is wrapped as a flow mapping when it has no line break and gets a line break after it otherwise. An empty document or `null` is rejected (`doc.shape` throws), and `@{` followed by a line break and `}` is rejected. A scalar or a sequence is accepted and changes nothing. A mapping sets, in order, the shape check, then `label`, `icon`, `assigned`, `ticket` and `priority`, each when its value is truthy (`0`, `false`, `""`, `null` and `NaN` are not; an empty array or object is). `shape` is checked and never stored: a non-string value, a capitalized name and a name with `_` are errors, a lowercase name passes.
- `getSection` looks one node behind: the first node's level is the section level, and every node that already exists is checked against it. A line shallower than the first node is a child of the last section, and `Items without section detected` appears with the next line. A decoration applies to the last node, whatever its level, and one before any node is rejected.

Error messages are ferrigura's own ([0007](../decisions/0007-hand-written-parsers.md)); the reference's expected throw becomes a rejection at the statement that raises it.

## Layout and rendering

`data.rs` is `getData` (`kanbanDb.ts`): a flat list in which each section is followed by its children, a child being every node whose parent id equals the section's id. A section keeps its ticket and drops its priority, assignee, icon and classes; an item keeps all of them; both drop their classes. A section id that the diagram repeats lists its children under each copy, and the renderer, which filters the whole list by parent id, draws them once per copy.

`mod.rs` is `kanbanRenderer.ts` (`draw`). The width of a section is `sectionWidth || 200`; the sections stand side by side at `x = width * n + ((n - 1) * padding) / 2` with the constant padding 10 (the source says "TODO set padding"); an item is `width - 1.5 * padding` wide and its label wraps at 175. The tallest section title sets a band that starts at 25 px, the items are stacked below it, each as tall as the box of its own group, and the rectangle of a section is `max(y - top + 3 * padding, 50)` plus the title's excess. The root's `viewBox` is the box of the drawing grown by the mindmap section's padding, which is how the reference reads it (`kanban.padding` is dead configuration).

The shapes are `unified/shapes/kanban_section.rs` and `kanban_item.rs`, ports of `kanbanSection` (`clusters.js`) and `kanbanItem`. An item is a rectangle, the title label, the ticket label and the assignee label (each written even when empty), and a priority line two pixels inside the left edge: `Very High` red, `High` orange, `Low` blue, `Very Low` light blue, and no stroke for `Medium`, a value that is not a name, or a number. The labels are `unified::label`, Markdown and start-anchored. The style sheet (`style.rs`) is `styles.ts` for the redux themes, one custom property per color on the diagram's id.

## Differences from Mermaid

- **Labels are SVG text** ([0009](../decisions/0009-svg-text-labels.md)). The reference's default for kanban is HTML labels (its `htmlLabels = false` assignment is made on a copy of the configuration and has no effect); the harness forces `htmlLabels: false`, which is the path ferrigura compares.
- **`look: handDrawn`** is excluded by the harness, and ferrigura draws the classic geometry.
- **The ticket link's URL** is written as configured; the reference's output goes through its sanitizer, which may change an unsafe URL. Not probed.
- **Sanitizing reads no configuration**: the reference's `sanitizeText` reads `dompurifyConfig`, and ferrigura's takes none. No input in the differential sets it.
- **Colors** are custom properties with presentation attributes ([0011](../decisions/0011-themes-as-css-variables.md)).

## Known gaps

None known for this type.

## Checking it

- `pnpm -C tools/port port:diff kanban` compares the canonical models on every fixture, probe, spec string, documentation example and case of `cases.ts`; `export-kanban.ts` runs the reference's grammar and DB and reads the node list through a line its loader appends to `kanbanDb.ts` (the reference's source is not changed). `port:diff config` compares the `kanban` section.
- `pnpm -C tools/port port:diff kanban-data` compares `getData` field by field (`export-kanban-data.ts`, `layout-data-diff.ts`) against the `kanban_data` example of `ferrigura-render`.
- `pnpm -C tools/port port:geometry kanban` compares the geometry of the fixtures and of the probes in `tools/port/geometry/probes/kanban/`. The fixtures are flat lists of sections; the probes cover what they miss: items, priorities, tickets and assignees, Markdown and HTML in labels, special characters, blank and wrapped titles, duplicate section ids, an empty board, the section width, the `neo` look, the dark theme and a ticket base URL.
- The Rust specs: `ferrigura-syntax/tests/kanban_spec.rs` ports the reference's parser tests (a section's children are the nodes whose `parent_id` is its id, which is `getData`'s filter), `kanban_db_spec.rs` its item-data tests, `kanban_quirks_spec.rs` has one assertion per quirk above; `ferrigura-render/tests/kanban_data_spec.rs` pins what `getData` keeps and drops, and `kanban_render.rs` the reference's numbers for a few fixtures and probes and the markup of the link, the icon background and the priority bars.

## Learnings

- **Every line of the fixtures has the same indentation**, so every node is a section and no fixture draws an item. The metadata in the fixtures sits on sections, which do not draw it. Items, `kanbanItem` and the priority lines are exercised only by the probes, which is why the probes live beside the harness and not in `e2e/diagrams`.
- **A section is numbered from 1, and its color is `cScale<N+1>`**: the style rules number their slots from `i - 1`, so `.section-1` holds `cScale2`, and `.section--1` and `.section-0` are never used.
- **Classes are dropped**: a section's group is `cluster undefined section-N`, an item's is `node undefined ` (with a trailing blank), and no item has a `data-look`. The class literal `__APA__` is in the reference's rectangle.
- **A section's width does not follow its title**: the width comparison uses `node.padding`, which is undefined for a section, so the rectangle stays `sectionWidth` wide, and the title is laid out at that width. Its label is not sanitized at render, only decoded by `createText`.
- **An item is placed from the box of its group**, which a long ticket can take beyond the rectangle.
- **The ticket link replaces the first `#TICKET#`** as `String.replace` does, expanding its `$` patterns, and its anchor has `class="kanban-ticket-link"` and an `xlink:href`, with no `target`.
- **An item with an icon draws no icon**, but the icon gives the title a background, which makes the item 4 px taller.
- **An empty board** has `viewBox="-10 -10 20 20"` and `max-width: 20px`: the padding around an empty box.
- **Each item's empty ticket and assignee labels are written**, as empty `g.label` groups with a background rectangle and an empty text; an item's height is 17 plus 20 for one line.
- **Duplicate DOM ids** occur in the fixtures (a repeated section id), so a harness that pairs elements by id must pair them by position.
- **`getData` sanitizes labels a second time** and drops the fields of sections that only items use.
