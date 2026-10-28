# User journey diagram

A user journey diagram draws the steps of a person's path through a process, grouped in sections. Each task has a score from 1 to 5, drawn as a face, and the people involved, drawn as colored circles with a legend. It starts with the keyword `journey`. ferrigura parses it with a hand-written parser that ports the Jison grammar `journey.jison` and the DB `journeyDb.js` (`crates/ferrigura-syntax/src/journey/`), and draws it with a renderer of its own (`crates/ferrigura-render/src/journey/`) that ports `journeyRenderer.ts`, `svgDraw.js` and `styles.js`. There is no layout engine.

## Model

`JourneyDiagram` (`crates/ferrigura-model/src/journey.rs`), version 1.

- `title`, `accTitle`, `accDescr`: the header texts after the sanitizer, `""` when absent. The last `title` line of the body wins, even when it sanitizes to empty, and replaces the front matter title.
- `sections`: every section statement in order, duplicates kept.
- `tasks`: in order, each `{ section, task, score, people }`. `section` is the current section, `""` before the first one; `task` keeps its trailing blanks; `people` are trimmed, with empty names and duplicates kept.
- `actors`: the unique names of all people, sorted by UTF-16 code units of their text.

A `score` is `Number()` of the text, so blanks and `""` are 0, `0x10` is 16, `1e1` is 10, `+3` is 3, and `abc` and `2rem` are `NaN`. A score that is not finite is written as the string `"NaN"`, `"Infinity"` or `"-Infinity"`, the convention of the configuration (`value.rs`), and the exporter writes the same. The DB's `type` of a task equals `section` and is left out.

**Texts are the reader's text.** The DB keeps entities as placeholders (`ﬂ°name¶ß`), the renderer sets the text, the serializer escapes `&`, `<` and `>`, `decodeEntities` turns the placeholders back into references in the whole SVG string, and the HTML parser reads the result. The model holds that final text: `#hearts;` is `♥`, an unknown name stays as `&name;`, and raw text such as `&amp;` stays as written, because only the placeholders are decoded. The named table is the full HTML5 list, in `entities.rs`, shared with the sanitizer. The title and the accessibility texts go through the sanitizer (`sanitize.rs`) before; task names, people and sections are not sanitized ([canonical-model.md](../canonical-model.md#journey)).

## Parsing

**Scanner.** `scanner.rs` runs the rules of `journey.jison` in their written order, first match wins, with case-insensitive keywords; the states `acc_title`, `acc_descr` and `acc_descr_multiline` are exclusive. It reproduces Jison's `done` flag: once the input is empty, the next call reads the rules of the current state at the end of the input, and after that every call returns Jison's raw end symbol, which is not the grammar's `EOF`. A text that ends inside an accessibility value (`accTitle:` followed by nothing, an `accDescr {` without its `}`) is therefore rejected.

**Grammar.** `journey document EOF`; the grammar builds no tree, and each statement calls the DB at once, in document order: `title`, `accTitle`, `accDescr`, `section`, and a task name followed by its data. A second `journey`, a colon, an invalid character, or a task name that is not followed at once by its data is an error.

**DB.** `db.rs` follows `journeyDb.js`. `addTask` reads the data as `pieces = data.substr(1).split(':')`: the score is `Number(pieces[0])`, the people are `pieces[1]` split at `,` and trimmed, and later pieces are ignored. `getActors` sorts the encoded names, so a placeholder sorts by its marks.

**Pipeline.** The steps of `Diagram.fromText`: front matter (its title is set on the DB before the parse and is not encoded), directives, comment lines and a trim at the start, entity encoding and a final line break.

Quirks of the reference that the parser keeps on purpose:

- **Keywords match at the start of any word,** except that `journey` needs a boundary after it: `journeyfoo: 5: me` is a task. `journey` is a keyword anywhere, so a second header line or a task named `Journey …` is an error.
- **The comment rule consumes one character before `%%`** at the start of a token (`x%%y` skips the line).
- **`title` and `section` drop the character after the keyword:** `title  X` is `" X"`, and a section name keeps its blanks and is not sanitized. A title with `;` or `#` stops there.
- **A task name keeps trailing blanks,** and the data must follow it at once; a comment or a line break in between is an error.
- **`accTitle:` and `accDescr:` take the rest of the line,** and the blanks after the colon may cross lines.
- **The text is trimmed at the start before lexing,** which changes what the `\s*` of `accTitle:` sees.
- **The last `;` of a `style` or `classDef` line,** followed by `:`, a non-blank run and `#`, is removed before lexing, by `encodeEntities`.

## Layout and rendering

`mod.rs` sizes the root and writes the title and the activity line, `legend.rs` the actors, `tasks.rs` the sections and tasks, `face.rs` the score faces, `text.rs` the labels, and `style.rs` the style sheet. The numbers are the schema defaults unless the theme or the configuration says otherwise.

- **Legend.** Each actor, in the order of `Object.keys` of the reference's actor object (names that are array indices first, ascending, then the others in order of insertion), is a circle at x = 20 with its name at x = 40 on lines of at most `maxLabelWidth` (360). A name wider than that is wrapped at blanks; a word wider than that is cut by characters with a hyphen. The widest legend line over 75 px widens `leftMargin` (150) by its width.
- **Sections.** A section is a band 50 px high at y = 50 over its tasks, with the name at y = 75. Its colors are `section-type-N` with N the index of the section among all, modulo 7, not its name. Tasks before the first section get no band.
- **Tasks.** A task is a box 150 by 50 at y = 110, `taskMargin` (50) plus the width apart, with a dashed line down to y = 450, a face at `300 + (5 - score) · 30`, the circles of its people along the top edge, and its name at y = 135.
- **Faces.** A score above 3 is a smile, below 3 a frown, and 3 (or a score that is not a number) a straight line. The mouths are d3 arcs between 7.5 and 6.818 (15 / 2.2); d3's `arc` swaps radii when the inner is larger, so the code passes them in ascending order, and the path has three decimals.
- **Labels.** The default placement `fo` is HTML in the reference; ferrigura draws SVG text in the lines the browser would break the HTML into (see Differences). `textPlacement: tspan` draws one `text` per `<br>` piece, and `old` one `text` for the label.
- **Root.** The viewBox is `X -25 W H+extra` with `W = leftMargin + stopx + 2 · diagramMarginX` and `H = stopy - starty + 2 · diagramMarginY`; `extra` is 70 with a title. The `height` attribute is `H + extra + 25`. With `useMaxWidth`, `width="100%"` and a `max-width`. The title is bold at `leftMargin`, `titleFontSize` `4ex`, y = 25. The activity line is at `4 · height` from the left margin to 4 units short of the right edge, with an arrowhead.
- **Style.** The rules of `styles.js` that journey markup uses, with the colors as custom properties ([0011](../decisions/0011-themes-as-css-variables.md)); every `line` takes the text color, and `.task-type-N` and `.section-type-N` take `fillTypeN`.

## Differences from Mermaid

- **`fo` labels are SVG text** ([0009](../decisions/0009-svg-text-labels.md)). The renderer breaks a label where the browser breaks the HTML (after a blank, and after a hyphen followed by a letter), centers each line in the cell, and uses the line box of the font. `<br>` is literal text in this placement, as it is in the reference.
- **Fonts.** Every text is set in the bundled face, so `titleFontFamily` and `taskFontFamily` are not read; the geometry harness pins them on the reference side (`journey-labels.ts`) ([0010](../decisions/0010-text-measurement-and-fonts.md)).
- **Only the redux themes are drawn** ([0011](../decisions/0011-themes-as-css-variables.md)).
- **The number of color types is 7,** the length of the schema's `sectionFills`, whatever the configuration says: the reference reads it once when its module loads.

## Known gaps

- Scripts the bundled faces do not cover need a registered fallback font ([0010](../decisions/0010-text-measurement-and-fonts.md)).
- Break opportunities other than blanks and hyphens before letters are not modeled; a word wider than the cell widens it to the width of its widest piece.

## Checking it

- `pnpm -C tools/port port:diff journey` runs the reference's Jison parser with its DB (`export-journey.ts`), after `preprocessDiagram` and `encodeEntities`, reads the DB in the order of the renderer (`getTasks`, then `getActors`), converts every text to the reader's text with the reference's `decodeEntities` and a parser of its own (jsdom), and compares the result with ferrigura's `canonical` example. `tools/port/cases.ts` has an input for each quirk the specs do not reach.
- `pnpm -C tools/port port:diff config` covers the `journey` section.
- `pnpm -C tools/port port:geometry journey` compares the drawing in Chromium: the elements by the generic rule within 0.05 px, and the labels line by line (`journey-labels.ts`), since the reference's are `foreignObject`s.
- The Rust specs: `crates/ferrigura-syntax/tests/journey_db_spec.rs` (`journeyDb.spec.js`, where each test drives `parse`) and `journey_spec.rs` (`parser/journey.spec.js`). `crates/ferrigura/tests/journey.rs` checks the root and the wrapped labels, and the unit tests in `legend.rs`, `text.rs` and `face.rs` pin the legend wrap, the label lines and the mouth paths. The reference has no renderer spec for the journey.

## Learnings

- **`getBoundingClientRect` of an SVG `text` ignores leading and trailing blanks and collapses runs.** `"ab "` and `"ab"` measure the same, and `"a b"` is wider only by the one blank. The legend wrap measures that way, so a trailing blank in a name does not wrap it.
- **The legend wrap has a stray hyphen.** When one character plus its hyphen is already wider than `maxLabelWidth`, the code pushes `broken[:-1] + '-'`, which is just `-`: with `maxLabelWidth: 5` the name `AB` draws `-`, `A-` and `B`.
- **`__proto__` is not an own key.** An actor with that name is missing from the legend, but its color and position still reach the circles on the tasks.
- **The face's mouths come from d3's arc with swapped radii and three decimals** (`d3-path`'s `pathRound(3)`): `M7.5,0A7.5,7.5,0,1,1,-7.5,0L-6.818,0A6.818,6.818,0,1,0,6.818,0Z`. The geometry tolerance does not need the rounding, but the path text does.
- **The visible `fo` label is the `div`,** a table cell with `text-align: center` and `vertical-align: middle` in a 150 by 50 `foreignObject`. The SVG `text` fallback is not drawn by Chromium. A block taller than the box starts at the top and grows down; the cell is never narrower than its widest unbreakable piece.
- **The bounds use the wrong right edge on purpose.** A task inserts `x + diagramMarginX + taskMargin` (`x + 100`), not `x + width`; the viewBox still contains the last box because the width adds `leftMargin` and `2 · diagramMarginX`.
- **A CSS rule overrides the attributes.** `line { stroke: textColor }` makes the task lines and the activity line take the text color, not `#666` or `black`, and `.task-type-N` and `.section-type-N` are all gated by `fillType0`.
- **Widths sit on the 1/64 px grid,** because they are sums of runs rounded up to it: a `leftMargin` of `455.453125` is `150` plus a legend line of `305.453125`.
