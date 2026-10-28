# Sankey Diagram

The sankey diagram draws flows between nodes as bands whose width follows the value. Its text is CSV: a header line `sankey` (or `sankey-beta`), then one `source,target,value` record per line. ferrigura parses it with a hand-written scanner in `ferrigura-syntax/src/sankey/` and draws it with the type's own renderer in `ferrigura-render/src/sankey/`, whose layout is a port of `d3-sankey` 0.12.3. It does not use the unified renderer or a layout engine crate.

## Model

`ferrigura_model::sankey::SankeyDiagram` is the DB state after the parse ([canonical-model.md](../canonical-model.md#sankey) has the conventions):

- `title`: the front matter's title, sanitized, or `""`. Nothing draws it, but the DB holds it. An entity in it is not encoded and stays as written.
- `accTitle`, `accDescr`: always `""`. The grammar has no statement for them.
- `nodes`: `{id}` in creation order: the source of a record before its target, by first appearance. A node id is the name as the DB sanitizes it, so two names that sanitize alike are one node.
- `links`: `{source, target, value}` in input order, with the ids of the ends. Duplicates are kept. A value that is not finite is its sentinel string.

Left out: the layout (`x0`, `y0`, `layer`, `width`, the node values) and the configuration.

## Parsing

`sankey/mod.rs` runs the preprocessing steps of `Diagram.fromText`, sets the front matter title, and then applies the parser wrapper's `prepareTextForParsing`: the blanks at the very start and end of the text go (a blank that starts or ends another line stays), every run of line breaks becomes one line break, inside quoted fields too, and the text is trimmed. Blank lines vanish, while a line of blanks in the middle stays and is an error. `scanner.rs` then reads the whole text into tokens, and the grammar (`start: SANKEY NEWLINE csv`, `record: field COMMA field COMMA field`) calls the DB once per record: the source node, the target node, then the link.

The scanner ports `sankey.jison`: the initial state reads the header, a `csv` state reads the rest, and a quoted field enters `escaped_text`. A field is printable ASCII (U+0020 to U+007E) without the double quote and the comma. Jison's rules can match the empty text, and the grammar never accepts an empty field, so the scanner reports the first character no rule takes (a tab, a non-ASCII character, the end of the text inside quotes) as an error at once. The accept and reject results are Jison's.

Quirks that ferrigura keeps:

- The header is `sankey` or `sankey-beta` alone on its line, in any case for the parser (the detector is case-sensitive). Other text on the line, and a word glued to the keyword (`sankeyfoo`), is an error. A header without a record is an error.
- A record has exactly three fields. An empty field is an error (`,b,1`, `a,b,`); a field of blanks is the empty name `""`, and a value of blanks is `NaN`. At the very end of the text the blanks are removed, so such a field is empty and the text an error.
- Quoted fields: `""` is one quote (resolved after the trim); a line break inside is kept, and a run of them collapses; nothing may follow the closing quote; the field must close.
- Text outside printable ASCII is an error, quoted or not (`Käse`). An entity fails: `#name;` and `#digits;` become placeholders that are not ASCII, while `&quot;` without the hash is text.
- The value is `parseFloat` of the field: `10abc` is 10, `1e` is 1, `0x10` is 0, `1_000` is 1, `"1,2"` is 1; `x`, `NaN` and `-` are `NaN`; `Infinity` and `2e308` are infinite.
- A name is sanitized as the DB does: `<script>` goes with its content, `<img onerror>` loses the handler, an open tag swallows the rest (`a<b` is `a`), `a<` is `a&lt;`, and `a&b` and `a=b` are kept.
- A cycle parses; the layout rejects it. Prototype names (`__proto__`, `constructor`) are ordinary ids.
- The grammar has no `title`, `accTitle` or `accDescr` statement, so such a line is an error; `sankey` is an ordinary name inside a record. Comment lines and directives are removed; a `%%` in the middle of a line is text, and a line of `%%` alone is an error.

Error messages are ferrigura's own ([0007](../decisions/0007-hand-written-parsers.md)): the differential run compares accept and reject, not Jison's text. Lines are counted in the text the grammar reads, after the blank lines were removed, plus the front matter's lines.

## Layout and rendering

`layout.rs` is `d3-sankey` 0.12.3 (`sankey.js`, `align.js`), run with Mermaid's settings: node ids, `nodeWidth`, `nodePadding`, the alignment (`left`, `right`, `center` or `justify`) and the extent `[[0, 0], [width, height]]`; the defaults of the library that Mermaid leaves alone are six iterations and the default sorts. The phases and their order are the library's, because the last bits of the positions are part of the geometry: node links, node values (`max(sum(out), sum(in))`, where the sum skips `NaN` and zero), depths and heights by breadth-first search (a cycle throws `circular link`), layers, the initial breadths, and the relaxation sweeps with `alpha = 0.99^i` and `beta = max(1 - alpha, (i + 1) / iterations)`, with the collision resolution of the library (a move only for more than `1e-6`) and its comparators on a stable sort. The sorts and sums are the shared d3 subset in `common/d3/`.

`mod.rs` is `sankeyRenderer.ts` and `styles.js`. It writes the root, the style sheet (the common sheet and the sankey rules, `own_rules` in `style.rs`), `g.nodes` with one `g.node` and `rect` per node, `g.node-labels` with the labels, and `g.links` with one `g.link` per link: a `linearGradient` in gradient mode and a `path` with d3's `linkHorizontal` (`M x0,y0 C xm,y0 xm,y1 x1,y1`, `xm = (x0 + x1) / 2`), stroke width `max(1, width)`. Node colors come from `nodeColors` or the Tableau 10 ordinal scale, assigned in node order. A label is SVG text; with `labelStyle: outlined` each label is two texts, `sankey-label-bg` then `sankey-label-fg`.

The root's size is the bounding box of the drawing, as `setupGraphViewbox` reads `getBBox`: the layout's `width` and `height` only set the extent, and labels that spill past it widen the `viewBox`. ferrigura computes the box from the node rectangles, the link bands and the label boxes (`ferrigura_text::measure_box`), leaving out the elements with `NaN` geometry.

## Differences from Mermaid

- **Prototype names.** `nodeColors` is a plain object in the reference, so a node named `constructor` or `__proto__` reads the member of `Object.prototype` as its color, and the reference writes that member's text into the attribute (`[object Object]`, `function Object() { [native code] }`). ferrigura writes the same strings (`object_prototype_member`).
- **`useMaxWidth`.** The schema says `false` for sankey, but the reference's built default is `true`. ferrigura follows the reference (`ts_defaults.json`).
- **Detector.** `sankeyfoo` is not a sankey diagram. Upstream's detector takes any word that starts with the keyword, which is a bug of Mermaid; the reference carries a patch that ends the detector at a word boundary, as ferrigura's does ([0014](../decisions/0014-upstream-reference-at-a-pin.md)).
- **Themes.** Only the shipped themes exist ([0011](../decisions/0011-themes-as-css-variables.md)). The dark custom properties are written but not compared, since the harness forces `redux-color`.

## Known gaps

None known for this type. A probe the reference cannot render (a cycle gives `circular link`) counts as a reference failure in the geometry run, not as a difference; ferrigura fails on a cycle too.

## Checking it

- `pnpm -C tools/port port:diff sankey` compares the canonical models on every fixture, spec string, documentation example and case of `cases.ts`; `export-sankey.ts` runs the reference's parser and DB after `prepareTextForParsing`. `port:diff config` compares the `sankey` section.
- `pnpm -C tools/port port:geometry sankey` compares the geometry of the fixtures and of the probes in `tools/port/geometry/probes/sankey/` (cycles, zero, negative, `NaN` and infinite values, prototype names, spilling and outlined labels, every alignment, link colors, prefix and suffix, many links).
- The Rust specs: `ferrigura-syntax/tests/sankey_spec.rs` ports the reference's spec, `sankey_grammar.rs` has a test per quirk above, and `ferrigura-render/tests/sankey_layout_d3.rs` compares the layout to the bit with `d3-sankey` on random graphs (zero, negative, `NaN` and infinite values, every alignment, cycles) that the real library laid out.

## Learnings

- **Zero, negative, `NaN` and infinite values do not throw.** The sums skip `NaN` and zero, so a node's value is 0 and `ky` is `Infinity` or `NaN`: every position becomes `NaN`, the rect gets `height="NaN"`, the path `d="M10,NaNC…"` and `stroke-width="NaN"`. A negative value gives `stroke-width="1"` (`Math.max(1, -Infinity)`). Chromium reads `y="NaN"` of a text as 0, so such labels still count in the root box. The render must write the same strings and succeed.
- **The label's newline collapses to a space.** The text is `name\nvalue`, which SVG draws as one line, so the box is that of `a 8`. The embedded subset must cover the collapsed text, space included: a subset cut from `a`, `\n` and `8` has no space glyph, and Chromium draws the space with a fallback glyph of another width, which put the boxes 0.42 px off.
- **Values are rounded with `Math.round(value * 100) / 100`** and printed with JavaScript's number rules: `0.597` is `0.6`, `2.675` is `2.68` (the product is a double that rounds up), `1e21` is `999999999999999900000`, `1e-7` is `0`. `Math.round` rounds half up (`-2.5` is `-2`), which Rust's `round` does not.
- **The ordinal scale assigns colors in order of first use.** The rectangles call it in node order; the gradient stops come after (every source stop, then every target stop) and add no new value. A non-custom node's color therefore depends on how many non-custom nodes precede it, and the 11th takes `#4e79a7` again.
- **Node ids are `node-<n>` and `linearGradient-<n>`** from one counter that the reference never resets, so a second diagram on a page continues the numbering. ferrigura numbers each render from 1; the harness pairs elements without their ids.
- **The legacy label has no `class` attribute**; the `x` and `y` of `g.node` are written though nothing draws them; the `height` attribute of the root is written after `xmlns` and the `width` after the `id`.
- **`nodePadding` is `nodePadding + 15` when values are shown**, and `width` and `height` of the configuration set only the extent: `width: 300` gives a root 300 wide because the nodes span it.
- **Floating-point residue is part of the output**: the justified layout writes `viewBox="0 -2.664530515715871e-14 410 400"`. Only the library's order of operations reproduces it.
- **`nodeColors` passes the directive filter**: a value must be a hex color, `rgb(…)`, `hsl(…)` or a plain word (`rgba(…)` is removed), and a key that starts with `__` or contains `proto` or `constr` is removed.
- **A latent bug stays latent**: `height` falls back to the default width, but the merged defaults always provide `height`.
