# Ishikawa diagram

An ishikawa (fishbone) diagram draws a cause tree: the effect is the head of a fish, the spine runs from the tail into the head, and the causes branch off the spine at a fixed angle, alternating above and below it, each with its sub-causes. The keywords are `ishikawa` and `ishikawa-beta`. ferrigura reads it with a hand-written Jison-style scanner and parser (`ferrigura-syntax`, `src/ishikawa`) and draws it with a renderer of its own (`ferrigura-render`, `src/ishikawa`); there is no layout engine, the layout is arithmetic on an angle and on the measured boxes of the labels.

## Model

`ferrigura_model::ishikawa::IshikawaDiagram` is the DB state after the parse (`ishikawaDb.ts`), version 1:

| Field      | Holds                                                                                    |
| ---------- | ---------------------------------------------------------------------------------------- |
| `title`    | The diagram title: the head's text, or the front matter title when no statement was read |
| `accTitle` | Always `""`: no rule of the grammar sets it                                              |
| `accDescr` | Always `""`, for the same reason                                                         |
| `root`     | The head as an `IshikawaNode`, or `null` when no statement was read                      |

A node is `{ text, children }`, with the children in source order. The first statement after the header is the head; a cause is a child of the head or of another cause, and its depth is its nesting.

The model keeps the DB's quirks:

- **The title is the head's text, sanitized twice.** The DB sanitizes the text when it adds a node, and `setDiagramTitle` sanitizes it again. A head replaces the front matter title even when its text sanitizes to nothing, and a diagram with no head keeps the front matter title.
- **An empty head is a root.** The DB tests the node, not its text, so a head whose text sanitizes to `""` is a root with an empty text.
- **Texts are sanitized once** with the flowchart's DOMPurify subset (`sanitize.rs`) and keep the entity placeholders the DB keeps.

Left out are the stack of open nodes and the base level, which are the parse state, and the layout, which is the renderer's. The conventions of the whole family are in [canonical-model.md](../canonical-model.md#ishikawa).

## Parsing

`ishikawa::parse` runs the steps of `Diagram.fromText`: it prepares the text (line endings, HTML attribute quotes, front matter and its title), removes directives and comment lines, encodes entities and adds a line break. The scanner reads that text, and the grammar calls the DB as it reads each statement, in document order ([0007](../decisions/0007-hand-written-parsers.md)).

The scanner tries the rules of `ishikawa.jison` in the order of the file and takes the first that matches, not the longest. It has one state:

| Rule                                       | Token     | Reads                                                            |
| ------------------------------------------ | --------- | ---------------------------------------------------------------- |
| `\s*%%.*`                                  | SPACELINE | blanks, `%%` and the rest of the line                            |
| `ishikawa-beta\b`, `ishikawa\b` (any case) | ISHIKAWA  | the header, if the next character is not a word character        |
| `[\s]+[\n]`                                | SPACELINE | blanks up to the last line break that is not the first character |
| `[\n]+`                                    | NL        | a run of line breaks                                             |
| `[\s]+`                                    | SPACELIST | blanks; their count is the indentation                           |
| `[^\n]+`                                   | TEXT      | the rest of the line                                             |

The grammar is `start: spaceLines? ISHIKAWA NL? document`; a statement is `SPACELIST TEXT` (a cause with its indentation), `TEXT` (the head, or a cause at level 0), or a blank line that calls no DB method.

The DB places a node by its indentation. The first statement is the head and starts the stack of open nodes. The first cause fixes the base level, so the head's own indentation does not count; a node's level is its indentation minus the base plus one, and a level of 0 or less is 1. The stack pops while its top is at least the new level (never the head), and the node becomes a child of the top.

The quirks that ferrigura keeps because the reference has them:

- A bare `ishikawa` with no line break is an error, since the grammar needs a statement; `ishikawa\n` is accepted with no root, because the input's line break and the one the parser adds are one SPACELINE.
- `ishikawaEffect` is text, not the header, and the detector rejects it as well. `ishikawa-betafoo` is the header followed by the text `-betafoo`. Blanks after the header are an error, and a second header is an error.
- A directive that the reference's loose directive pattern takes up to the header (`%%{ bad\nishikawa`) leaves the diagram undetected, and ferrigura's `remove_directives` is the same pattern.
- Each tab is one level unit. `%%` inside a text is text, and so is `;`. `accTitle:`, `accDescr:` and `title` lines are causes.
- `<br>` and `<br/>` stay as written, and a text with `<` is sanitized (`&` becomes `&amp;`), while a text without `<` is unchanged.
- The header is case-insensitive in ASCII only, as the lexer's `i` flag folds.

## Layout and rendering

The renderer ports `ishikawaRenderer.ts` (`draw`) and `ishikawaStyles.ts`. `mod.rs` holds the document, the spine and the head, `bones.rs` a cause, its label and its descendants, `text.rs` the label boxes and the wrapping, `style.rs` the style sheet.

- **Angle.** Causes leave the spine at 82 degrees. `cos` and `sin` of that angle come from `ferrigura-jsmath`, which gives V8's values to the last bit ([0012](../decisions/0012-layout-engines-ported-exactly.md)).
- **Sides.** Causes at even indices are the upper side, the others the lower one. Each side's length is its share of 500 by its number of descendants, at least 75, and at least the most descendants under one cause times twice the font size; with no descendants both are 250. The spine's height is the upper length or 250, whichever is larger.
- **Branches.** A cause with children is a line of the side's length, one without a fifth of it. Descendants are placed by depth: even depths are horizontal bones in pre-order, odd depths diagonal bones in post-order, so each stays in its parent's wedge. The upper side reads its children in reverse at every depth, which mirrors the lower side.
- **Spine.** Each pair of causes sets the spine's left end to the leftmost text of that pair, so the spine starts at the last pair's text, not at the leftmost of all.
- **Boxes.** The reference reads four boxes back with `getBBox`: the root's, the pair's texts, the head's and the cause's label. ferrigura computes them from the font ([0010](../decisions/0010-text-measurement-and-fonts.md)): the horizontal extent of each line from its anchor with the glyph ink where it passes the advance, and the vertical extent from the baseline that the stylesheet's `dominant-baseline` places. The root's box takes the tight bounds of the head's quadratic curve, whose extreme is 1.2 times the width, not the control point's 2.4 times.
- **Wrapping.** The head wraps at `max(6, floor(110 / (fontSize * 0.6)))` characters, a sub-cause at 15, and a cause's label does not wrap. Lengths are UTF-16 lengths and the split is at JavaScript white space. `<br>` splits a label in all three.
- **Text sizes.** The layout spaces lines with the configured font size (16 px) and draws the labels at the theme's size (14 px in the shipped themes); the two are kept apart. The head is bold: its weight 600 maps to the bold face.
- **Root.** `ishikawa.diagramPadding` (20) pads the box, and `ishikawa.useMaxWidth` is true by default, so the root takes `width="100%"` and a `max-width`.
- **Empty lines.** A line without characters makes no `tspan` fragment and takes no line step, so the next line follows the last one with characters; a label of empty lines only has the zero box at the origin, which is why an empty head is 40 px tall.

## Differences from Mermaid

- `look: handDrawn` draws the classic geometry: roughjs is not ported, and the reference draws it from `Math.random` at the default seed, so no run could compare it. The geometry run leaves such fixtures out ([0015](../decisions/0015-differential-testing.md)).
- Labels are SVG text and the fonts are embedded subsets ([0009](../decisions/0009-svg-text-labels.md), [0010](../decisions/0010-text-measurement-and-fonts.md)).
- The sanitizing of texts uses the default configuration. A directive that changes `securityLevel` or `htmlLabels` changes the reference's first pass but not ferrigura's.

## Known gaps

- `look: handDrawn` (see above).
- The labels at the 16 px of Mermaid's classic default theme are not compared; only the redux themes ship ([0011](../decisions/0011-themes-as-css-variables.md)), and the harness forces one of them. The formulas take the size from the theme and do not depend on it.

## Checking it

- `pnpm -C tools/port port:diff ishikawa` compares the canonical models of the fixtures, the string literals of Mermaid's spec and the cases of `tools/port/cases.ts` (the grammar and DB quirks above). `pnpm -C tools/port port:diff config` compares the `ishikawa` section of the configuration.
- `pnpm -C tools/port port:geometry ishikawa` compares the geometry of the fixtures and of the probes under `tools/port/geometry/probes/ishikawa/` (many causes, deep nesting, long heads and texts, `<br>`, unbalanced sides, dark theme, padding and width, a header with no statement, empty labels).
- Rust specs: `ishikawa_spec` (the port of Mermaid's spec), `ishikawa_grammar` (one test per quirk of the scanner and grammar), `ishikawa_db_spec` (placement, title, sanitizing) in `ferrigura-syntax`; `ishikawa_render` in `ferrigura-render` pins the reference's numbers for the first fixture and the markup, and the unit tests of `text.rs` and `mod.rs` cover wrapping, baselines and side lengths.

## Learnings

- **The effective `useMaxWidth` is true, the schema's is false.** `IshikawaDiagramConfig` says `false`, but the reference's effective value is `true` (most likely the base diagram config's default winning the merge, which was not traced), and every render uses the max-width form. The value is in `crates/ferrigura-model/src/config/ts_defaults.json`, the file for the reference's defaults that its schema does not hold, and `port:diff config` compares the section.
- **The head's anchor comes from the stylesheet.** The head's `text-anchor="start"` attribute loses to `.ishikawa-head-label { text-anchor: middle }`, so the box is centered on its `x`. The same holds for the baselines: the `up` labels declare `dominant-baseline: baseline`, which is not a keyword, so Chromium drops it and uses the alphabetic baseline. ferrigura writes the declaration as the reference does and computes the box from the alphabetic baseline.
- **Boxes sit on the 1/64 px grid.** `getBBox` of a text in Chromium gives widths on that grid and heights from the rounded ascent; a view scaled by `max-width` gives off-grid values, so the geometry run compares at scale 1.
- **The weight 600 is the bold face.** `Blurry` is 39.3125 px wide at 600 and 35.40625 px at 400 in the bundled face.
- **A header with no statement draws almost nothing.** The reference returns before the layout, so there is no `g.ishikawa` and no viewBox on the root.
- **Layout and drawing take different font sizes.** Spacing follows `fontSize` of the configuration, the labels follow the theme's, and mixing them shifts every label by a fraction.
