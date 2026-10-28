# Treemap Diagram

The treemap draws a hierarchy as nested rectangles whose area follows the value. It starts with `treemap` or `treemap-beta`; each row is a quoted name, indented to give its depth, and a leaf has a value: `"Leaf": 20`. A row may carry a class with `:::name`, and `classDef` rows define the classes. ferrigura parses it with a hand-written lexer and recursive-descent grammar in `ferrigura-syntax/src/treemap/` (entry point `treemap.rs`) and draws it with the type's own renderer in `ferrigura-render/src/treemap/`, over a port of the layout of `d3-hierarchy`. No layout engine crate is involved.

## Model

`ferrigura_model::treemap::TreemapDiagram` is the DB state after the parse and `buildHierarchy` ([canonical-model.md](../canonical-model.md#treemap) has the conventions):

- `title`, `accTitle`, `accDescr`: sanitized by the DB's setters. The title is the grammar's when it is non-empty, else the front matter's. `accTitle` loses its leading blanks; `accDescr` loses the blanks after each line break.
- `classes`: the classes in the order of their first definition, each with `styles` and `textStyles` (the DB's lists). A class defined twice keeps its place and appends its styles.
- `tree`: the root of `getRoot()`, with the name `""` and the top-level nodes as children. A section has `children` (empty when it has no rows); a leaf has `value` and no `children`. `classSelector` is present when the row gives one, and `cssCompiledStyles` when the DB defines that class. A value that is not finite is its sentinel string (`"NaN"` for a value of only separators, `"Infinity"` for a digit run that overflows).
- Names are the DB's names: the quoted text without its quotes, nothing sanitized, the entity placeholders kept (`#quot;` is `ﬂ°quot¶ß`).

Left out: the indentation levels and the flat node list (the tree holds the same information), the DB's `levels` map, and the configuration.

## Parsing

`treemap.rs` runs the preprocessing steps of `Diagram.fromText`, tokenizes with `scanner.rs`, parses the rows with `grammar.rs` and builds the model with `db.rs`. The DB's order follows Mermaid's `populate`: the header texts first, then every `classDef` row in order, then the items (each with the styles of its class as they stand after all the definitions, so a class defined after its use styles the item), then `buildHierarchy`.

The scanner ports `treemap.langium` as Chevrotain runs it: the first terminal that matches wins, not the longest, in the order of the grammar's terminals. The accessibility and title terminals come first, since they begin with the blanks before them; then the keyword, `classDef`, `:::`, `:`, `,`, the blanks, comments and line breaks (hidden), `ID2`, `NUMBER2` and `STRING2`. Terminals no rule uses (`BOOLEAN`) are not in Langium's lexer. The grammar is `Treemap: KEYWORD (ACC_DESCR | ACC_TITLE | TITLE | TreemapRow)*`, `TreemapRow: INDENTATION? (Leaf | Section | CLASS_DEF)`, `Leaf: STRING2 INDENTATION? (":" | ",") INDENTATION? NUMBER2 (":::" ID2)?`, `Section: STRING2 (":::" ID2)?`.

Quirks that ferrigura keeps:

- A name must be quoted (`A: 1` is rejected). It may be empty, single-quoted or span lines, and has no escapes.
- Values: commas are removed before `parseFloat`, so `1,2,3` is 123; `1_000` is 1; `1..2` is 1; `.5` is 0.5; `._.` and `.` are `NaN`; `_` alone and `1e3` are rejected; a digit run that overflows is `Infinity`.
- A blank run that ends a row is the indentation of the next row, so `"A": 1 ⏎ "B": 2` puts `B` one level deeper. Two indentation tokens in a row are rejected, and so is a trailing blank at the end of the text. Tabs count one character each. A comment line takes its leading blanks with it; a comment after a value leaves its blanks as a trailing blank.
- A `classDef` name needs two characters (`c` is rejected, `_x` is fine). A bare `classDef` followed by a line takes that line as its style text (`\s` crosses the line break), so `"A"` disappears; a bare `classDef` at the end gives the style `''`. A style text ends at `;`, its commas split the styles, `\,` keeps a comma. Only the exact pieces `color`, `font-size` and the other label properties are text styles; `color:#fff` without a space is not.
- Class selectors are identifiers: `true` and `false` are plain names. `treemap`, `title` and a `classDef` after `:::` are rejected, and so is a blank before `:::`.
- `title` alone is the empty title; a title may sit inside a row (`"A" title T`) or follow the keyword with no blank (`title"A": 1` is the title `""` and a leaf). The last title wins; an empty grammar title keeps the front matter's.
- `treemapX` is rejected (the keyword followed by an identifier), while `treemap "A": 1` on one line is accepted. Two roots are accepted: the multiple-root validator is registered but never runs in `parse()`.
- Names and values are not sanitized (`<b>C</b>` stays as written); entities are placeholders before the parse.

Error messages are ferrigura's own ([0007](../decisions/0007-hand-written-parsers.md)); the differential run compares accept and reject.

## Layout and rendering

`mod.rs` ports `renderer.ts` (`draw`), `style.rs` the colors, the class declarations and the inline style, `text.rs` the measured text, and `config.rs` the `treemap` section. The layout is `d3-hierarchy` in `common/d3/`: `hierarchy.rs` (`sum`, `sort`, the traversals), `treemap.rs` (the paddings, `squarify` with the golden ratio, `dice`, `slice`, the rounding), together with `format.rs` (`d3-format`, with `toFixed`, `toExponential` and `toPrecision` rounded on the exact binary value) and `ordinal.rs` (`scaleOrdinal`).

The hierarchy is summed from the last child to the first, sorted descending by value (a stable sort), and laid out with `size([1000, 400])` for the defaults (`nodeWidth` and `nodeHeight` in tenths of a pixel), `round(true)`, `paddingInner(padding)`, and for each node with children a `paddingTop` of 35 (a 25 px header plus the padding) and the padding on the other sides. A section is a node with children; a node without children is a leaf, including a section with no rows (a zero-value leaf), and an empty `treemap` draws its root as a leaf. Each section is a group with a header rectangle, a clip path, the body rectangle, a bold label and an italic value; each leaf is a group with a rectangle, a clip path, a label and a value.

Colors are three ordinal scales (the section fill and stroke, and the label), assigned in the order of the first call: sections in breadth-first order (the root's name `""` is the first call and takes `transparent`), leaves in depth-first order. A leaf's fill and stroke use its parent's name.

Leaf labels and values are fitted by shrinking the font, and hidden when they do not fit: the label's size drops by 1 while the text is wider than the leaf allows, the value's is derived from the label's, and a section label is truncated with `...` to the header's width minus the value's room, and emptied when even the dots do not fit. A layout of more than 20 leaves uses the complex constants. The decisions read the advance of the text, so `text.rs` measures as Chromium does (`getComputedTextLength` snapped to the 1/64 px grid, and the `getBBox` of one line). The root's `viewBox` is the union of the boxes that are drawn, plus `diagramPadding`; an element with `display: none` (the root section) is left out.

The facade passes `TreemapSettings::of(&effective)`: the typed configuration has no treemap section, because the schema does not name its keys ([0008](../decisions/0008-canonical-model-and-typed-views.md)).

## Differences from Mermaid

- **Label truncation counts characters**, where the reference counts UTF-16 units, so an astral character may be cut differently. This is not probed.
- **A font family in a class** that is not one of the families the harness maps is measured in the bundled face; the reference measures it in that font.
- **Text outside the bundled faces** is measured with the face's `.notdef`; the geometry run leaves such fixtures out ([0010](../decisions/0010-text-measurement-and-fonts.md)).
- **Colors** are custom properties with presentation attributes ([0011](../decisions/0011-themes-as-css-variables.md)).

## Known gaps

- The `.treemapNode.section` and `.treemapNode.leaf` rules of Mermaid's styles are left out of the sheet: no element carries those classes.

## Checking it

- `pnpm -C tools/port port:diff treemap` compares the canonical models on every fixture, spec string, documentation example and case of `cases.ts`; `export-treemap.ts` runs the reference's parser and DB with a window for DOMPurify. `port:diff config` compares the `treemap` section.
- `pnpm -C tools/port port:geometry treemap` compares the geometry of the fixtures and of the probes in `tools/port/geometry/probes/treemap/` (class definitions, value formats, zero and non-finite values, long labels, many leaves, empty sections, the dark theme, titles and more). The fixture with emoji is excluded for its uncovered glyphs.
- The Rust specs: `ferrigura-syntax/tests/treemap_spec.rs` ports the parser tests of Mermaid's parser package (the two `class myNode myClass` cases are asserted as rejections, as the reference reports parser errors there), `treemap_db_spec.rs` the `buildHierarchy` tests and the DB's class and title rules, `treemap_grammar.rs` the quirks above; `ferrigura-render/tests/treemap_render.rs` holds the rules the markup and the errors show. `d3-format` was checked against the library on thousands of generated specifiers and values.

## Learnings

- **`valueFormat` has two failure modes.** The three `$` branches build their formatter inside the value callback, outside the `try`, so an invalid specifier after `$` (`$xyz`) fails the whole render, but only when a value is written. An invalid specifier without `$` falls back to `,`. `$0,0` is `$` plus `,` (decimals kept); a `$` with a comma takes the first `.n` of the string as precision (`$,.2f` gives `$1.6e+6`); `$` alone gives `$1234567`. `0` shows no value text.
- **d3-format rounds on the binary value**: `.2f` of `1.005` is `1.00`, `.0f` of `2.5` is `3`, the default type is `.12~g`, and negative numbers use U+2212. A treemap value cannot be negative.
- **The layout measures names with the entity placeholders in them**, and the SVG carries them decoded: the reference decodes the whole SVG string after the layout. The font subset is built from the decoded characters.
- **A class's declarations go into the inline style with the reference's rules.** A property given again moves to the place of the later declaration; an `!important` one keeps its value (`font-size:20px !important` on a leaf sets its label to 20 px); a d3 `style(name, value)` replaces the value and clears the flag. `color` becomes `fill` on labels. A label's and a value's font is the one its inline style gives, so a class's `font-weight` and `font-style` change the measured text and the embedded face.
- **The text boxes of the viewBox follow Chromium's baselines**: `middle` is half the x-height and `hanging` is 0.8 of the rounded ascent, both on the 1/64 px grid.
- **A `NaN` or infinite value** (`"A": .` or a 400-digit number) gives `NaN` coordinates, which the browser reads as zero in the box, and a `NaN` room cuts no label (`Math.max` keeps `NaN`, and `length > NaN` is false).
- **Clipping does not change `getBBox`**, so a clipped label counts in full in the viewBox.
- **Three configuration keys are never read**: `borderWidth`, `valueFontSize` and `labelFontSize` have defaults but the renderer uses constants (`labelFontSize: 30` leaves the labels at 38 px). `nodeWidth` and `nodeHeight` fall back to 960 and 500 px when falsy, `padding` and `diagramPadding` fall back only when missing, and `showValues` is off only for `false`.
- **The simple and the complex constants** switch at more than 20 leaves (base label 38, base value 28, minimum label 8 versus 16, 14, 4).
- **The section's value is the sum after `sum`**, the root included; the leaf's `class` ends in `x` (`leaf0x`, `leaf1 ccx` for a class `cc`).
- **`d3-hierarchy`'s sum order matters**: children are summed from the last to the first, which fixes the floating-point result.
