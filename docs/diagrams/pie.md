# Pie chart diagram

A pie diagram draws labeled values as slices of a circle, with a legend and an optional title. It starts with the keyword `pie`, optionally followed by `showData` to print the values in the legend. ferrigura parses it with a hand-written parser on the shared `logos` lexer (`crates/ferrigura-syntax/src/pie.rs`) that follows the Langium grammar `pie.langium` and the DB of `pieDb.ts`, and draws it with a renderer of its own (`crates/ferrigura-render/src/pie.rs`). There is no layout engine: the layout is arithmetic, with the slice angles of d3's pie.

## Model

`PieDiagram` (`crates/ferrigura-model/src/pie.rs`), version 1, with these fields:

- `title`: the grammar's `title` when it is not empty, else the front matter's title, else `""`.
- `accTitle`, `accDescr`: the header texts, `""` when absent. `accTitle` has no leading blanks, and an indented continuation of `accDescr` is joined, as the DB's setters do.
- `showData`: true only when the keyword directly follows `pie` on the same line.
- `sections`: the slices as `{ label, value }`, in the order of their first appearance.

The texts are the source text after the grammar's value rules: unsanitized and unescaped ([canonical-model.md](../canonical-model.md#pie)). The DB's setters sanitize header texts with DOMPurify, which needs a DOM, so the exporter applies only the two whitespace rules of the setters (`tools/port/pie-header.ts`).

A label given twice keeps its first value and its first position, as `addSection` does. A negative value is an error, and it is checked before the duplicate test, so a negative value under a repeated label is an error too. A value of `-0` is kept as written.

## Parsing

The parser reads the tokens of the shared lexer (`lexer.rs`) with a cursor: `pie`, `showData`, then any mix of `title`, `accTitle` and `accDescr` lines and slices (`"label" : number`) until the end. The shared rules of `common.rs` read the header lines, as `common.langium` does ([0007](../decisions/0007-hand-written-parsers.md)).

- Keywords are words that the parser recognizes, not tokens: `pie` and `showData` are plain words to the lexer.
- A label is a double or single quoted string whose escapes (`\n`, `\t`, `\\` and the others) are resolved.
- A number is `-?[0-9]+(\.[0-9]+)?`. A leading zero that stands for more than `0` (`007`) and a trailing dot (`1.`) are lexer errors, as the grammar's `INT` makes them.
- Comments (`%%`) and directives are hidden tokens. A `---` block that is not at the start of the text is hidden too, as the grammar's `YAML` terminal is.
- A slice ends at the end of its line; two slices on one line are an error.

## Layout and rendering

`pie.rs` ports `pieRenderer.ts` and `pieStyles.ts` step by step, so that the geometry can be compared with the reference.

- **Sizes.** The diagram is 450 px high, the pie 450 px wide with a margin of 40 px, so the radius is 185 px. A legend entry is a square of 18 px with 4 px of spacing. `pie.textPosition` places the slice labels along the radius, and `pie.donutHole`, when it is above 0 and at most 0.9, cuts a hole of that fraction of the radius.
- **Slices.** The angles come from d3's pie over the slices that are at least one percent of the total, in input order, sharing the full circle between them. A slice below one percent is not drawn and has no label; its legend entry stays. A label is the percentage in whole numbers, as `toFixed(0)` writes it.
- **Legend.** `pie.legendPosition` is `right` (the default), `left`, `top`, `bottom` or `center`. The legend is laid out by the box of its widest entry, and the root grows to hold it. With `showData` an entry reads `label [value]`.
- **Highlight.** `pie.highlightSlice` names a slice to enlarge, or is `hover` to enlarge the slice under the pointer.
- **Colors.** A slice takes the theme color `pie1` to `pie12` for its position among all sections, wrapping after twelve. Every color is a CSS custom property declared on the diagram's id, and the element carries the same color as a presentation attribute ([0011](../decisions/0011-themes-as-css-variables.md)). `colorScheme: auto` writes the light set and, under `prefers-color-scheme: dark`, the dark set.
- **Text.** The slice labels, the legend and the title are SVG text in the bundled face, with the glyphs used embedded as a subset ([0009](../decisions/0009-svg-text-labels.md), [0010](../decisions/0010-text-measurement-and-fonts.md)). Every run has an explicit `x`; the title is anchored at its start, at `-advance / 2`.
- **Root.** The viewBox starts at the smaller of 0 and the left edge of the title, and is as wide as the larger of the drawing and the title. With `useMaxWidth` the root has `width="100%"` and a `max-width`. `accTitle` and `accDescr` become `<title>` and `<desc>`, and the root refers to them with `aria-labelledby` and `aria-describedby` (`a11y.rs`).

## Differences from Mermaid

- Labels are SVG text; there are no HTML labels ([0009](../decisions/0009-svg-text-labels.md)).
- Colors are custom properties with attribute fallbacks, and only the two redux themes ship. The reference's `default`, `forest`, `neutral` and `dark` themes are drawn with the redux theme their name falls back to ([0011](../decisions/0011-themes-as-css-variables.md)).
- A slice carries a class `pie-slot-<n>` of ferrigura's own, for its color slot. The geometry key leaves it out.
- The width of the root is rounded to Chromium's layout unit (see Learnings). The reference's width is already on that grid, because the browser measured it.
- The id of a diagram is a hash of its text and configuration unless the caller gives one, and every selector starts with it, so several diagrams can share a page.

## Known gaps

- A pie without sections has no meaningful geometry in the reference (its legend width is `-Infinity`); ferrigura keeps it at zero. No fixture covers it.
- Scripts the bundled faces do not cover (CJK, kana, Arabic) need a registered fallback font ([0010](../decisions/0010-text-measurement-and-fonts.md)).

## Checking it

- `pnpm -C tools/port port:diff pie` compares the canonical models on every input: the fixtures, the string literals and docs of the reference, and the cases in `tools/port/cases.ts`.
- `pnpm -C tools/port port:check-header` compares the header rules of `pie-header.ts` with the real DB setters, with jsdom as the DOM.
- `pnpm -C tools/port port:geometry pie` and the specialized `pnpm -C tools/port port:geometry-pie` compare the geometry in Chromium: the viewBox, the outer circle, the angles of every slice (within 1e-5 rad), the label boxes, the title and the legend. `--images <dir>` writes side-by-side images.
- `pnpm -C tools/port port:scope-pie` inlines three pies with different ids in one page under a stylesheet that paints every path, text and rectangle, and checks that each keeps its own colors under light, under dark and with style sheets off.
- `pnpm -C tools/port port:diff config` covers the `pie` section.
- The Rust specs are `crates/ferrigura-syntax/tests/pie_spec.rs` (`pie.spec.ts`) and `pie_grammar.rs` (`packages/parser/tests/pie.test.ts`). `crates/ferrigura/tests/pie.rs` checks the SVG, and the pie fixtures are in the snapshots. The reference's `pieRenderer.spec.ts` checks the DOM of the reference's own renderer and is not ported; the geometry run takes its place. Two of the three configuration tests of `pie.spec.ts` are `it.todo` in the reference and stay ignored.

## Learnings

- **A width off the 1/64 px grid changes the text.** Chromium lays text out in units of 1/64 px. A root width that is not on that grid makes it scale the drawing by a fraction and lay the text out at the scaled size, which moves widths by a few hundredths of a pixel. `layout_unit` snaps the width.
- **The reference writes path numbers with three decimals** (d3-path). An angle can therefore be off by up to 2.7e-6 rad on the outer radius, and the geometry run compares angles at 1e-5 rad.
- **Both sides must measure after the embedded faces have loaded.** The geometry page loads the bundled face as a web font and waits for `document.fonts.ready`.
- **The legend and the title are measured as boxes, placed by advances.** The reference reads the legend through `getBoundingClientRect`, so its width is the box of the glyphs, and the title's width in the viewBox is a box too, while the title is placed by its advance. Both come from `ferrigura-text` (`measure_box`, `layout_width`).
- **The color slot is the position among all sections.** A slice that is too small to be drawn still advances the colors of the slices after it.
