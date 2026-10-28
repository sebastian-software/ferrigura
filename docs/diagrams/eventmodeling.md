# Event modeling diagram

An event modeling diagram draws a timeline of frames as boxes in swimlanes: user interface and processors on top, commands and read models in the middle, events at the bottom, with arrows from a box to the box it follows from. The keyword is `eventmodeling`. ferrigura reads it with a hand-written scanner and parser of the Langium grammar (`ferrigura-syntax`, `src/event_modeling`) and draws it with a renderer of its own (`ferrigura-render`, `src/event_modeling`); there is no layout engine, the layout is arithmetic on the measured size of each box's text.

## Model

`ferrigura_model::event_modeling::EventModelingDiagram` is the DB state after the parse, without the geometry and the measured box texts, version 1:

| Field          | Holds                                                                                                                         |
| -------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| `title`        | The grammar's last `title` token, sanitized, else the front matter title                                                      |
| `accTitle`     | The accessibility title token, sanitized, without leading blanks                                                              |
| `accDescr`     | The accessibility description token, sanitized, with the blanks after each line break removed                                 |
| `frames`       | The boxes in text order (below)                                                                                               |
| `dataEntities` | The `data` blocks `{ name, dataBlockValue }` in text order, with the value the grammar gives, braces and line breaks included |
| `lanes`        | The swimlanes `{ index, label }` the frames use, in index order                                                               |
| `relations`    | `{ sourceFrame, targetFrame }`, frame indices, in the order the DB creates them                                               |

A frame is `{ name, kind, modelEntityType, entityIdentifier, entityName, namespace, lane, sourceFrames, dataInline, dataReference, data }`: its number as written (`01` and `1` are different frames), its kind (`time` for `tf` and `timeframe`, `reset` for `rf` and `resetframe`), the entity type keyword as written, the qualified name, the DB's split of that name into name and namespace (the namespace is `null` unless the name has exactly two parts), the index of its lane, the source numbers after each `->>`, the inline data as written, the name inside `[[ ]]`, and the data text.

`data` is the data text the DB shows: the referenced block's text when that block exists, otherwise the inline data's, cut and sanitized as `calculateTextProps` cuts it, before the line breaks and the `&nbsp;` joins that measurement needs. `null` means the frame shows no data and `""` a quoted inline value, which the reference shows as an empty code block.

Left out are the box geometry and texts (they need the measured text, and `data` is the part that does not), the `entity`, `note` and `gwt` statements (the DB never reads them, though the parser checks their syntax), and the `dataType` of backticked types, which the reference parses and never reads. The conventions of the whole family are in [canonical-model.md](../canonical-model.md#event-modeling).

## Parsing

`event_modeling::parse` runs the steps of `Diagram.fromText`: it prepares the text (line breaks, quote rewriting, front matter and its title), removes directives, keeps the comment lines (this DB sets `preserveCommentsWhenParsing`) unless the text is longer than the default `maxTextSize` of 50 000 UTF-16 units, encodes entities and adds a line break. The scanner and the grammar build a syntax tree, the DB places the frames and creates the relations, and the model holds the result ([0007](../decisions/0007-hand-written-parsers.md)). Positions in errors count the lines of the original input, front matter included.

The scanner runs the terminals of the Chevrotain lexer in the order Langium builds them, and the first that matches wins: the keywords (longest first), `EM_ID`, `EM_FID` (one to three digits), the inline data, the data block, the `accDescr`, `accTitle` and `title` tokens, white space, and the hidden YAML, directive and comment terminals. Each regular expression is matched by hand as JavaScript runs it at the current position. A word keyword is a token only when the identifier run that starts there is exactly the keyword, and `eventmodeling` also needs `%%`, white space or the end after it. The grammar is `eventmodeling` followed by statements, each starting with a token no other statement starts with.

The quirks that ferrigura keeps because the reference has them:

- **Titles work only after another token on the same line.** `EM_ID` and the white space rule come before the header rules, so a `title`, `accTitle:` or `accDescr` at the start of a line is an identifier and the parse fails; `eventmodeling title Foo` and `tf 01 ui A title Foo` work. The header texts are the raw token texts with their keyword and leading blanks (` title Foo`); the common setters then sanitize them and strip the leading blanks of `accTitle`. The last of each wins.
- **Keywords cannot be identifiers:** `tf 01 evt text`, `tf 01 evt data`, `A.ui` and `A.text` fail.
- **Frames are not line-based:** `tf 01 ui A tf 02 cmd B` on one line parses. The order inside a frame is fixed: sources, then `[[ref]]`, then the inline data.
- **A frame number is a string.** `01`, `1` and `001` are three frames, and a repeated number is an error, reported at the later frame's keyword.
- **A data block's closing brace must start a line.** An indented `  }` fails, and `data D { a: 1 }` fails because the one-line form is lexed as inline data.
- **Inline data is greedy** to the last closing character on the line.
- **A qualified name** joins its parts without blanks (`A . B` is `A.B`).
- **Names keep the entity placeholders** (`ﬂ°quot¶ß`); the DB decodes nothing, and the renderer does.
- **Lanes.** A frame's lane is set by its entity type: UI, processor and `pcr` at index 0 (`UI/Automation`), command and read model at 100 (`Command/Read Model`), events at 200 (`Events`). A namespaced frame takes the next free index in the range of its kind, after the largest index in it, and its label is `UI/A: `, `C/RM: ` or `Stream: ` plus the namespace. Namespaced frames never share a lane: the DB looks for a lane with the same namespace, but the lanes it creates have no namespace field, so every namespaced frame gets a new one, even when the namespace repeats.
- **Relations.** A frame with explicit sources gets one relation per listed frame, in text order, when that frame is already placed; a source that comes later in the text, or names no frame, gives none, and a self source gives a self relation. A frame without sources gets one from the nearest earlier box in another lane, unless it is the first frame or a reset frame.
- **The data cuts.** Inline data loses the character before its closing `}` (`{x: 1}` shows `x: `). A block with blanks after its `{` has no `{\n`, so the cut removes the brace alone and keeps the blanks and the line break after it.
- **The validator is registered but never runs.** The reference's source checks (which entity types may feed which) are registered with Langium's validation registry, but `parse()` reads only the lexer and parser errors, so an `ui` fed by an `evt` renders. `validator.rs` holds the checks as functions of their own, tested alone and not called by `parse`.

## Layout and rendering

The renderer ports `renderer.ts`, `styles.js` and the text of `db.ts` (`calculateTextProps`). `layout.rs` holds the box texts, the placement, the lanes and the relations, `label.rs` the box text as SVG text, `style.rs` the colors, and `mod.rs` the root and the elements.

- **Measurement.** A box is sized by `calculateTextDimensions` of the literal content (`<b>name</b>`, `<br/><br/>`, `<code style="…">data</code>`): the markup, the `&nbsp;` joins and the placeholders are measured as characters, at 16 px bold in the family list the reference names in code (`"trebuchet ms", verdana, arial, sans-serif`), which ferrigura measures in the bundled face ([0010](../decisions/0010-text-measurement-and-fonts.md)). Per line the width and height are rounded; the width of the whole is the widest line, the height the sum. When a box shows data its text width is a third of the measured width, not rounded. A name or data longer than 430 px wraps at that width, and a word wider than that breaks inside with a hyphen.
- **Boxes.** The width is the text width plus 20, clamped to 80 to 450, plus 20; the height the same with 80 to 750. The first box starts at x 250. A box in the lane of the previous box follows that lane's last box plus 10; any other box starts 70 px left of the right edge of the last drawn box, whatever its lane.
- **Lanes.** A lane is the tallest of its boxes (at least 70) plus 30 high, and each lane starts 10 below the one above. Every lane rectangle is `maxR + 15` wide, where `maxR` is the largest lane right edge before the last frame and that frame's own `r`, not the widest box; a box can stick out of the lanes. A lane's label is bold at the root's size, 30 px from the left and 30 px below the lane's top, and is not measured.
- **Relations.** A relation runs from two thirds of the source box's width to one third of the target's. It leaves the bottom of the source and enters the top of the target, or leaves the top of the source and enters the bottom of the target when the source's lane is strictly lower; a relation inside one lane goes down. The lane's current `y` is used, not the box's stale `y`.
- **Box text.** The reference writes the text as HTML in a `foreignObject`; ferrigura draws SVG text in the lines the browser breaks that HTML into ([0009](../decisions/0009-svg-text-labels.md)), wrapped in a `switch` that the label comparison reads. The name is bold and centered in the cell, the empty line of the second `<br/>` takes the strut's height, and the data is in the mono face, left at the cell's edge, at most 430 px wide, breaking only where its text has a blank or a newline (`&nbsp;` does not break). A code line is a list of runs, each in the weight and slant of its tags (`b` and `strong` bold, `i` and `em` italic, nested tags add up) and measured in the faces its runs are drawn in; other tags draw nothing of their own. A code block widens its cell to its widest unbreakable piece.
- **Colors.** Entity colors are the code's fallbacks, since neither redux theme defines an `em` key: UI `white` with `#dbdada`, processor `#edb3f6` with `#b88cbf`, read model `#d3f1a2` with `#a3b732`, command `#bcd6fe` with `#679ac3`, event `#ffb778` with `#c19a0f`; lanes `rgb(250,250,250)` with `rgb(240,240,240)`. They are custom properties on the diagram's id and also presentation attributes ([0011](../decisions/0011-themes-as-css-variables.md)). The dark theme draws the same colors.
- **Root.** The root's box is the union of the lane rectangles, the lane labels, the box rectangles and the relation points, rounded to float32 as `getBBox` returns it, padded by `eventmodeling.padding` (30); `useMaxWidth` is true by default. A box's text is not in it. An empty diagram has the viewBox `-30 -30 60 60`. The arrowhead marker is `em-arrowhead-<id>`.

## Differences from Mermaid

- The box text is SVG text, not HTML in a `foreignObject` ([0009](../decisions/0009-svg-text-labels.md)), and the fonts are the bundled faces: the names in Lexica Ultralegible and the data in Atkinson Hyperlegible Mono ([0010](../decisions/0010-text-measurement-and-fonts.md)). The reference's `code` element takes the machine's `monospace`; the harness pins it to the bundled mono face so both sides lay the code out in one face.
- Error messages are ferrigura's own, with the position in the original input; the duplicate-frame message is `frame id "01" is already used by an earlier frame`, where the reference counts the front matter out of its line number ([0007](../decisions/0007-hand-written-parsers.md)).
- A data cut can split a surrogate pair (`{ 😀}` cut before the closing brace). The reference keeps the lone surrogate; a Rust `String` cannot, so ferrigura writes U+FFFD. No input of the comparison has one.

## Known gaps

- The tags that DOMPurify keeps and the data can reach, beyond bold, italic and the plain ones, are not drawn as the reference draws them: `sub` and `sup` (a shifted baseline and a smaller size), `small` and `big` (a size), `kbd` (the machine's `monospace`) and the block tags `p` and `div` (each starts a line). No probe passes through them yet; `u`, `s`, `span`, `mark` and `code` are within tolerance.
- The browser's per-glyph rounding of advances is not emulated; it shows as one layout unit (1/64 px) on some lines.
- The configuration's `maxTextSize` is not read by the parse pipeline, for any type; a directive that sets it is not honored.
- The `accTitle` and `accDescr` elements and the order of the `aria-labelledby` attributes are not compared with the reference (the harness skips `title` and `desc`).

## Checking it

- `pnpm -C tools/port port:diff eventmodeling` compares the canonical models of the fixtures, the string literals of Mermaid's specs and documentation, and the probed cases of `tools/port/cases.ts` (every quirk above has an input there). `pnpm -C tools/port port:diff config` compares the `eventmodeling` section.
- `pnpm -C tools/port port:geometry eventmodeling` compares the geometry of the fixtures and of the probes under `tools/port/geometry/probes/eventmodeling/`: the blocks of Mermaid's syntax documentation (`docs-01` to `docs-13`) and probes for long names and data, many lanes, namespaces, upward relations, empty and quoted data, markup, placeholders, tall boxes, a configuration override, the chained state change, and bold and italic data (mixed, nested, next to code, wrapping, and an unbreakable bold word wider than the cell). The box text is compared line by line (`journey-labels.ts`, with the `foreignObject` as the label, and a newline read as the blank it is drawn as on both sides). The harness maps the families the reference names in code (`trebuchet ms`, `verdana`, `arial`) onto the bundled faces.
- Rust specs: `event_modeling_spec` (the port of Mermaid's diagram spec), `event_modeling_grammar` (the parser's spec, with the validator registration ignored because Langium's registry is not ported, plus a test that `parse` does not run the checks), `event_modeling_validator` (the checks on their own) in `ferrigura-syntax`; `event_modeling_render` in `ferrigura-render`.

## Learnings

- **Boxes are sized in one font and drawn in another size.** The measurement is 16 px bold in a family list named in code, not the configured font, and the text is drawn at the root's size (14 px in the redux themes), so a box is larger than its text. The configured `fontFamily` never reaches the measurement; ferrigura measures in the bundled face and the harness maps the reference's family names onto it, so both sides measure the same text.
- **The measurement reads the literal markup.** The width and height come from the whole string including `<b>`, `<code style="…">`, `&nbsp;` and the entity placeholders, split on `<br>`; an empty line becomes U+200B and still has its height. The reference measures twice, in `sans-serif` and in the named family, and takes the second unless the first is larger in all three values, which is never the case in practice because its height is smaller.
- **Heights do not depend on the text:** 19 px per line in the named family on the reference machine, 18 px in `sans-serif`. Wrapping, by contrast, depends on the widths, so a stubbed measurement (the reference's own spec stubs `getBBox` to 100 by 20) cannot test it.
- **`getBBox` of the root is float32.** The lane rectangle's width is a double (`540.3333333333334`), but the viewBox and `max-width` use the float32 value plus the padding (`600.3333129882812`), so the root's box is rounded to float32 before the padding is added.
- **The `/3` is not rounded.** A box with data is a third of the measured data width plus 40, for example 265.33 px.
- **`maxR` is not the widest box.** It is read before the last frame moves its lane, so the lanes can be narrower than a box.
- **A newline in a data block is a break opportunity,** because the reference writes the block's own newline into the HTML while `&nbsp;` does not break; a block of several lines can show one more line than the measurement counted.
- **The harness leaves `foreignObject` sizes out,** so the box rectangles carry the same numbers and show a size error; the text is compared as labels.
- **Dead code in the reference:** `findSwimlaneByNamespace` can never find a lane, and `rowHeight`, `boxDefaultY` and `boxTextFontWeight` are read by nothing.
