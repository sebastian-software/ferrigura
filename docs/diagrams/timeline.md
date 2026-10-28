# Timeline diagram

A timeline diagram draws periods (a year, an era) with the events that happened in them, grouped in optional sections, along an axis. It starts with the keyword `timeline`, optionally followed by a direction: `LR` (the default, left to right) or `TD` (top down). ferrigura parses it with a hand-written parser that ports the Jison grammar `timeline.jison` and the DB `timelineDb.js` (`crates/ferrigura-syntax/src/timeline/`), and draws it with a renderer of its own (`crates/ferrigura-render/src/timeline/`) that ports `timelineRenderer.ts` for the horizontal layout and `timelineRendererVertical.ts` for `TD`. There is no layout engine.

## Model

`TimelineDiagram` (`crates/ferrigura-model/src/timeline.rs`), version 1, is the DB's state after parsing.

- `direction`: `"LR"` or `"TD"`. A plain `timeline` keeps `LR`.
- `title`, `accTitle`, `accDescr`: the header texts, sanitized, `""` when absent.
- `sections`: the section names in the order given, duplicates kept, untrimmed.
- `periods`: in creation order, each `{ section, period, events }`. `section` is the section current at creation, `""` before the first one. `period` and `events` are untrimmed.

The DB calls the periods `tasks`; the model names them as the grammar does. The model leaves out `id` (a counter that `clear` does not reset), `type` (always the section) and `score` (always 0).

The timeline's text rules differ from the other types' ([canonical-model.md](../canonical-model.md#timeline)):

- **The header texts follow the grammar.** It trims `accTitle` and `accDescr` and nothing else, so runs of blanks are kept and a title keeps its leading and trailing blanks. The `accDescr` setter then removes the indentation after each line break.
- **The front matter title does not reach the model,** because the timeline DB has no title setter. The title is the last `title` line of the body, even when it is blank.
- **Text is encoded, not decoded.** Entities become placeholders before parsing (`#quot;` is `ﬂ°quot¶ß`) and the model keeps them. Sections, periods and events are not sanitized; the title and the header texts are.

## Parsing

**Lexer.** `scanner.rs` runs the rules of `timeline.jison` in their written order, over the helpers of `scan.rs`: the first match wins, keywords are case-insensitive, and the rules are numbered as the generated lexer numbers them. The text always ends in a line break, because `Diagram.fromText` appends one, so a value state (`accTitle:` and the other values) is reached at the end of the input only when the line before consumed the last line break.

**Grammar.** `timeline_header document EOF`: a line is a statement or a line break, and statements need no line break between them (`timeline section A` is a header and a section). The actions run in input order: `title`, `accTitle`, `accDescr`, `section`, a period, and an event (`: text`). A header token after the first token and an `INVALID` token are errors.

**Pipeline.** The steps of `Diagram.fromText`: front matter (its title is read and dropped), directives, comment lines, entity encoding and a final line break. Detection is the caller's job (`ferrigura_syntax::detect`).

Quirks of the reference that the parser keeps on purpose:

- **`Timeline` is not detected.** The detector is case-sensitive, and the text is rejected.
- **The last `title` line wins, even when blank,** and `title` and `section` need one blank character after the keyword; the value starts at the next character. `title ` with nothing after it is a period.
- **Only `timeline` has a word boundary.** `timelineX` is a period and fails for lack of a header; `timeline TDx` is a header and a period.
- **An event needs a colon and a blank.** `2004 : a:b` is the event `a:b`; a colon anywhere else is `INVALID`. An event attaches to the last period created, wherever it is; with no period it is an error (a `TypeError` in the reference).
- **`accTitle:` takes the next line as its value,** and the indentation of that line goes with the line break. `accTitle:` or `accDescr:` as the last line, and an unclosed `accDescr {`, are errors, because the value state ends at the end of the input.
- **`#` starts a comment at the start of a token** and ends a period; in titles, sections and events it is text.
- **`%%` after exactly one character takes that character with it;** after two or more the period keeps the `%%`.
- **The `accDescr { … }` block has no comment rules:** `#` and `%%` are text, and the block ends at the first `}`.
- **Blank lines and blanks between statements have no effect.**

## Layout and rendering

The horizontal layout (`horizontal.rs`) places the sections left to right and the tasks under them; the vertical layout (`vertical.rs`, `TD`) places the sections and their tasks down the page and the events to the right of the axis. A node is drawn by `node.rs` (`drawNode`, `getVirtualNodeHeight`, `defaultBkg`), its wrapped text is `text.rs`, and the rules are `style.rs`.

- **Constants of the horizontal layout.** Tasks and events are 150 wide with 20 padding, tasks are 200 apart, the top is at 50, an event is at least 50 high and the next one starts 10 lower. A section is `200 · max(n, 1) - 50` wide. The axis is at `maxSectionH + maxTaskH + 150` (`maxTaskH + 100` without sections) and the viewBox is the box of everything drawn plus `timeline.padding` (50), computed from the boxes of the drawn elements.
- **Constants of the vertical layout.** Tasks are 200 wide, events 300, padding 5, a gap of 20 between a section and its tasks and 30 between tasks; the axis is `(leftMargin + 50) + 230` from the left.
- **Two measurements per node.** The layout first measures every text outside the `section-N` groups (`getVirtualNodeHeight`, in the regular face) to get the largest height, and `drawNode` measures again inside them, where the stylesheet makes the text bold. The pre-pass measures the text `[object Object]` for tasks (the task object, not its text), so the axis and the smallest node heights come from it.
- **Wrapping.** The text is split at blanks and at lowercase `<br>` tokens, one token is added to a line at a time, and a new `tspan` starts when the line is wider than the node. A line's width is the advance of its collapsed text, snapped to the layout unit; every `tspan` is at `x="0"` and centered by `text-anchor: middle`, and later lines are `1.1em` below the one before.
- **Colors.** Sections cycle through the theme's color slots (`THEME_COLOR_LIMIT`, 12), and `section--1` is the first. The redux theme's nodes have square corners and no line under the node. In the `neo` look the nodes carry `data-look="neo"` and a drop shadow, and the horizontal layout defines a gradient when the theme's `useGradient` is set. With `disableMulticolor`, only one color is used; without sections, the color advances per task.
- **Text size.** Text is at the theme's `fontSize` (14 px in the redux themes); node heights read the top-level `fontSize` (16). The title is `4ex`, four times the x-height of the regular face at the CSS size, `ferrigura_text::x_height`.
- **Root.** `width="100%"` with a `max-width` for `useMaxWidth`. `accTitle` and `accDescr` are not drawn: the root has its role and `aria-roledescription="timeline"` and no `<title>` or `<desc>`.

Defects of the reference that the renderer reproduces:

- **A `TD` timeline has no arrows.** Its marker is `undefined-arrowhead`, because `initGraphics` gets no id, and its arrows refer to `url(#arrowhead)`. Its nodes are `undefined-node-N` for the same reason, and no gradient is defined although the style sheet refers to `url(#id-gradient)`.
- **Every horizontal task has its dashed line,** with or without events (`if (task.events)` is always true).
- **Tasks before the first section are not drawn,** in both layouts, and a repeated section name draws its tasks under each occurrence.
- **Only a lowercase `<br>` breaks a line.** `<br/>`, `<br />` and `<BR>` are text. A leading or trailing `<br>` leaves an empty line, an empty `tspan` adds no height, and an overlong word is not split: it takes a line of its own, and the line before it is empty.
- **Wrapping measures the encoded text** (`ﬂ°quot¶ß`) while the glyph boxes are those of the decoded text.
- **A title with markup is escaped twice:** the sanitized `&` becomes `&amp;amp;`.

## Differences from Mermaid

- **Only the redux themes are drawn.** The reference chooses between a classic and a redux branch by the theme's name; a theme that ferrigura does not ship is drawn with the redux theme its name falls back to ([0011](../decisions/0011-themes-as-css-variables.md)), so the redux branch is the one drawn.
- **Colors are custom properties** with the concrete color as an attribute. The axis `stroke` is the node border color, not `black`: the reference's style sheet overrides `black` with the same color.
- **Rules no element can match are not written** (`.edge`, `.node-icon-N`, `.section-edge-N`, `.disabled*` and the like), each for its reason in `style.rs`. The `.lineWrapper line` rule is written once, where the reference writes it per color with the same value.
- **Text the bundled faces do not cover is measured with the bundled glyph,** where Chromium falls back to another font (`♥` from `#9829;`) ([0010](../decisions/0010-text-measurement-and-fonts.md)).

## Known gaps

- A directive's `securityLevel` or `htmlLabels` changes the sanitizing of header texts with markup in the reference; ferrigura sanitizes with the default configuration.
- A fixture whose text has characters no bundled face covers cannot be compared in geometry until a fallback font is registered.

## Checking it

- `pnpm -C tools/port port:diff timeline` runs the reference's preprocessor, detector, Jison parser and DB (`export-timeline.ts`) and ferrigura's `canonical` example over every input, and compares the canonical models. The header texts come from the common DB, which sanitizes, so the reference runs with a window.
- `pnpm -C tools/port port:diff config` covers the `timeline` section.
- `pnpm -C tools/port port:geometry timeline` compares every element within 0.05 px, on the fixtures and on the probe `tools/port/geometry/probes/timeline/p-named-entities.mmd`.
- The Rust specs: `crates/ferrigura-syntax/tests/timeline_spec.rs` ports the DB tests of `timeline.spec.js` with exact checks on the model, and `timeline_grammar.rs` pins each quirk above, with the reference's value. `crates/ferrigura-render/tests/timeline_render.rs` pins the numbers Chromium gives for the fixtures and the reference's markup, defects included. The reference has no renderer spec for the timeline.

## Learnings

- **`getComputedTextLength` collapses blanks.** `a   b` and `a b` have the same width, and leading and trailing blanks count as nothing. The wrapping therefore measures one collapsed run per line, not a sum of words, and the line breaks before the word that makes it wider than the node.
- **A text box is the union of its non-empty lines.** One line is the font's text box; each further line adds `1.1em`. An empty `tspan` adds no height and moves no line.
- **`dominant-baseline: middle` puts the middle of the x-height on the baseline position,** and the first line's `dy` is the `tspan`'s own `1em`: the text element's `dy` does not add to it. Both are measured in Chromium, not derived from the code.
- **The title's `4ex` needs the font's x-height.** The bundled faces declare 496 units per 1000 in `OS/2` (`sxHeight`), which is 27.776 px at the CSS size.
- **The bold face is chosen by weight 600.** `.section-N text` sets 600 and Chromium picks the bold face, so every drawn node is measured bold; only the pre-pass measures regular.
- **The viewBox comes from the drawn boxes,** not from the layout constants: it is the union of path boxes, text boxes and line extents, without `<style>`, `<defs>` or markers.
- **Redux or classic is decided by the theme's name,** in four places, not by the look.
