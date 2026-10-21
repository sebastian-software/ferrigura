# Spike: measuring text without a browser

**Question.** Can ferrigura measure label text in Rust, without a browser, closely enough to lay diagrams out like the JS reference does in Chromium?

**Answer.** Yes. With the same font files, Rust and Chromium agree on every width to within Chromium's own layout rounding, on text box heights exactly, and a port of Mermaid's SVG line wrapping produces the same lines in every case tested.

Run on 2026-10-15. The spike's code was in `spikes/text-measure/` until 2026-10-21; `crates/ferrigura-text` and `tools/port/text/` took over its work, and the code stays in the history.

## Setup

- **Fonts:** Lexica Ultralegible (regular, italic, bold, bold italic) and Atkinson Hyperlegible Mono (regular), the bundled fonts from [0005](../decisions/0005-text-measurement-and-fonts.md).
- **Rust:** `harfrust` 0.14 for shaping (with the `read-fonts` model it re-exports for metrics), `unicode-segmentation` for graphemes, `unicode-linebreak` for comparison. Widths are the sum of shaped advances, so kerning and ligatures are included.
- **Browser:** Chromium through Playwright, the same font files loaded as data-URL web fonts, measured with SVG `getComputedTextLength`, `getBBox` and canvas `measureText`.
- **Corpus:** 400 labels sampled from 2,093 found in the e2e fixtures, plus 10 edge cases (ligatures, kerning pairs, umlauts, figures, one very long word, leading spaces, typographic punctuation). 1,960 requests: every label at 14, 16 and 24 px in regular, a subset in the other three styles and in mono, and 270 wrapping requests at 80, 140 and 200 px.

## Results

| Check                                                                        | Result                                      |
| ---------------------------------------------------------------------------- | ------------------------------------------- |
| Width, Rust vs Chromium SVG `getComputedTextLength`, all 1,960 requests      | max 0.0155 px, mean 0.0078 px               |
| Width, Chromium canvas vs Chromium SVG, for comparison                       | max 0.0156 px, mean 0.0079 px               |
| Text box height and top, Rust vs SVG `getBBox`, ascent and descent unrounded | max 0.36 px                                 |
| Same, with ascent and descent each rounded to whole pixels                   | exact (0.0000 px)                           |
| Mermaid's SVG wrapping, Rust port vs the JS algorithm measured in Chromium   | 270 of 270 identical                        |
| Greedy wrapping at UAX #14 break opportunities vs Chromium's CSS wrapping    | 254 of 270 identical                        |
| Speed, release build, one measurement                                        | about 5 µs (1,960 in 10 ms), no caching yet |

The remaining width difference is Chromium's layout unit of 1/64 px (0.0156): Chromium's own canvas and SVG measurements differ from each other by the same amount.

Chromium rounds a font's ascent and descent to whole pixels separately before it builds the text box. With the same rounding, Rust reproduces box height and position exactly.

Mermaid does not wrap SVG labels with CSS rules. `createFormattedText` and `splitLineToFitWidth` (`rendering-util/createText.ts`, `splitText.ts`) split a line at single spaces, pack words greedily, measure each candidate line as one `tspan` per word with a leading space, and split a word that does not fit on an empty line into graphemes, at least one per line. The Rust port of exactly that matches in all 270 cases. Chromium's CSS wrapping differs only where it breaks after a hyphen (`2014-01-` | `06`, `(-` | `-//)`), which UAX #14 does not allow.

## Consequences

- **Shaping with `harfrust`, not `rustybuzz`.** `rustybuzz` has had no release since November 2024. HarfRust is the HarfBuzz project's own Rust port (MIT, tracks HarfBuzz 14.5.1, released October 2026), built on Google's fontations (`read-fonts`), whose `skera` also covers the font subsetting M2 needs for embedding. [0005](../decisions/0005-text-measurement-and-fonts.md) is updated.
- **Wrapping ports Mermaid's algorithm**, with `unicode-segmentation` for graphemes. `unicode-linebreak` is not needed. A UAX #14 mode could come later as an option, since it reads better for dates and hyphenated words, but it would break parity with the JS reference.
- **One shaping run per word**, as Mermaid's per-word `tspan`s do, so no kerning is applied across a space.
- **Vertical metrics rounded like Chromium's:** ascent and descent to whole pixels, so text sits where a browser will draw it.

## Part 2: embedding subsets

**Question.** If ferrigura embeds only the glyphs a diagram uses, does a browser still draw every label at the width measured in Rust?

**Answer.** Yes, in Chromium, both inline and through `<img>`, as long as every diagram's fonts get a family name of their own.

Setup: `src/bin/embed.rs` subsets each face with `skera` 0.8 (Google's fontations subsetter, a port of `hb-subset`) to the characters of the labels drawn in it. The subsets keep HarfBuzz's default layout features, so ligatures and kerning survive, and they drop hinting. The program writes them into the SVG as `@font-face` data URLs (TTF, base64), and each label carries its Rust width. `embed-check.ts` opens the SVGs in Chromium. Run on 2026-10-15 with the 1,690 requests of the corpus that have no wrap width.

| Check                                                                           | Result                                                    |
| ------------------------------------------------------------------------------- | --------------------------------------------------------- |
| Rust width with the subset vs with the full font, all 1,690 labels              | identical; vertical metrics identical                     |
| Chromium, SVG opened as a document: width vs Rust                               | max 0.0155 px; text box height exact                      |
| Same SVG through `<img>` (data URL) vs as a document                            | pixel-identical; without the fonts it is not              |
| Two diagrams inline in one HTML page, own family names (`fg1-sans`, `fg2-sans`) | both max 0.0155 px                                        |
| Same, one shared family name                                                    | 33 of 549 labels in the first diagram wrong, up to 1.8 px |
| Size of one face, about 95 characters                                           | 9–12.5 KB, from 52–72 KB                                  |
| A diagram with 57 labels in one face                                            | 26 KB SVG, 7.8 KB gzipped                                 |

Consequences for M2:

- **Family names unique per subset.** `@font-face` rules are global to the HTML page, even inside an inline SVG's `<style>`, and of two rules with the same family and descriptors the later one wins. Name the family after a hash of the subset (for example `fg-3fa9c1-sans`), so diagrams with the same subset share it and others cannot break each other.
- **One family per font, styles as descriptors.** Regular, italic, bold and bold italic are four `@font-face` rules of one family with `font-weight` and `font-style`; a label switches style with `font-weight="700"`, which Chromium matched to the right face.
- **`white-space: pre` on `text`, not on the root.** Chromium computes `white-space: nowrap` for `text` even when the root says `pre`, and drops a label's leading and trailing spaces. The rule must target `text` (or `xml:space="preserve"` on the element).
- **Keep legacy `kern` tables.** `skera`, like `hb-subset`, drops `kern` by default; Lexica keeps its kerning in `GPOS`, but a registered font may not, so ferrigura's subset call does not drop it.

## Part 3: other engines, and kerning

**Question.** Do Safari and Firefox draw the embedded, kerned labels at the Rust width too?

**Answer.** Yes. Safari 27, WebKit 26.5 and Firefox 155 draw kerned and ligated SVG text at the Rust width, as Chromium does. Only WebKit's SVG DOM API reports a different width: `getComputedTextLength` and `getBBox` return the sum of unkerned, unligated advances. A first reading of that API value suggested that WebKit does not kern SVG text; the drawn pixels show that it does. Kerning stays on, and `textLength` is not needed.

Setup, run on 2026-10-16:

- `embed-check.ts <dir> <engine>` checks every label's drawn extent, from the start of the first character to the end of the last (`getStartPositionOfChar`, `getEndPositionOfChar`), in Chromium 151 and WebKit 26.5 (Playwright). `getComputedTextLength` is reported next to it.
- `lab/lab.html`, served by `lab/server.mjs`, draws a kerning line (`AVATAR Type WAVE Yo Te LT`), a ligature line (`office fluffy fjord affinity`) and an ordinary label at 12, 16, 24 and 48 px, in fourteen variants of CSS and markup. It posts the results back, so it runs in the installed browsers as well: Safari 27.0.1 and Firefox 155, plus Chromium 152 in the Claude app's browser.
- Pixels: the kerning and ligature lines at 48 px as SVG and as HTML (kerned, unkerned, without ligatures), with the ink measured in screenshots; and the right and top ink edges of all 549 labels of `half-a.svg`, WebKit against Chromium.

| Check                                                                 | Chromium      | WebKit / Safari                             | Firefox 155          |
| --------------------------------------------------------------------- | ------------- | ------------------------------------------- | -------------------- |
| Drawn extent vs Rust, `all.svg` (1,690 labels, kerned)                | max 0.0155 px | WebKit 26.5: max 0.0003 px                  | (not run, see below) |
| Drawn extent vs Chromium, lab lines, 12–48 px                         | —             | Safari 27: max 0.014 px                     | max 0.08 px          |
| `getComputedTextLength` vs drawn extent, lab lines                    | equal         | larger: up to 32 px (kerning line at 48 px) | equal                |
| Ink of the kerning line at 48 px (unkerned HTML: 629 px, kerned: 597) | 597 px        | WebKit 26.5: 596 px                         | —                    |
| Right and top ink edges of 549 labels vs Chromium                     | —             | WebKit 26.5: all within 1 px (antialiasing) | —                    |
| `getBBox` height vs Rust                                              | exact         | 0.16–0.38 px off; the ink is within 1 px    | —                    |
| `<img>` vs document; shared family name breaks 33 of 549 labels       | yes; yes      | yes; yes                                    | —                    |

None of the lab's CSS switches changes how WebKit draws: `font-kerning`, `font-variant-ligatures`, `font-feature-settings`, `text-rendering` (as property or attribute), `white-space`, `letter-spacing`, `word-spacing`, `text-anchor` and a `tspan` all leave the text kerned; only `text-rendering: optimizeSpeed` drops ligatures, in Chromium and WebKit alike. Firefox reports and draws the same width.

**Why WebKit reports a different width.** WebKit lays SVG text out from advances it measures one character at a time, and paints each text fragment as one shaped run. References are to WebKit `main` at `609e7d3870` (2026-10-15):

- Layout: [`SVGTextMetricsBuilder.cpp`](https://github.com/WebKit/WebKit/blob/609e7d3870021938ba5cd4a30a9c5a0afaf025aa/Source/WebCore/rendering/svg/SVGTextMetricsBuilder.cpp#L170-L212) measures every character on its own (`widthForTextUsingSimplifiedMeasuring(view.substring(i, 1), …)`, line 205), so there is no pair to kern and nothing to ligate. [`SVGTextLayoutEngine.cpp`](https://github.com/WebKit/WebKit/blob/609e7d3870021938ba5cd4a30a9c5a0afaf025aa/Source/WebCore/rendering/svg/SVGTextLayoutEngine.cpp#L124-L135) adds these up as the fragment width. On macOS, CSS does not move Latin text off this path: the kerning check that would force the shaped path is compiled out there.
- What uses the layout width: `getComputedTextLength` ([`SVGTextQuery.cpp`](https://github.com/WebKit/WebKit/blob/609e7d3870021938ba5cd4a30a9c5a0afaf025aa/Source/WebCore/rendering/svg/SVGTextQuery.cpp#L247-L253)), `getBBox` (through `InlineIteratorSVGTextBox.cpp`), and the text chunk length behind `text-anchor` and `textLength` ([`SVGTextChunk.cpp`](https://github.com/WebKit/WebKit/blob/609e7d3870021938ba5cd4a30a9c5a0afaf025aa/Source/WebCore/rendering/svg/SVGTextChunk.cpp#L95-L107)).
- What is shaped: painting ([`SVGTextBoxPainter.cpp`](https://github.com/WebKit/WebKit/blob/609e7d3870021938ba5cd4a30a9c5a0afaf025aa/Source/WebCore/rendering/svg/SVGTextBoxPainter.cpp#L587), one run per fragment) and `getStartPositionOfChar`, `getEndPositionOfChar` and `getSubStringLength`, which measure the prefix as one run ([`SVGTextMetrics.cpp`](https://github.com/WebKit/WebKit/blob/609e7d3870021938ba5cd4a30a9c5a0afaf025aa/Source/WebCore/rendering/svg/SVGTextMetrics.cpp#L39)).
- Letter-spacing, word-spacing, per-character `x`/`y`/`dx`/`dy`/`rotate` lists and `textLength` split the text into one fragment per character, so the ink becomes unkerned as well. That is what `all-length.svg` showed: WebKit met the width with unkerned glyphs.
- Related WebKit bugs: [215321](https://bugs.webkit.org/show_bug.cgi?id=215321) (`text-anchor: middle` misaligned, open; an unmerged pull request measures whole fragments), [138596](https://bugs.webkit.org/show_bug.cgi?id=138596), [100050](https://bugs.webkit.org/show_bug.cgi?id=100050). No report covers kerning missing from `getComputedTextLength` and `getBBox` directly.
- Gecko takes each character's advance from the shaped run (`SVGTextFrame.cpp`), so Firefox's metrics and ink agree.

**Placement.** Because `text-anchor` and the start of a following `tspan` come from the layout width, WebKit misplaces kerned text that the browser aligns itself. `anchor-check.ts` draws the kerning line at 48 px five ways:

| Placement                                                 | WebKit vs Chromium, ink edges   |
| --------------------------------------------------------- | ------------------------------- |
| `text-anchor` start, explicit `x`                         | 0 / −1 px                       |
| `text-anchor="middle"`                                    | −17 / −16 px                    |
| `text-anchor="end"`                                       | −33 / −32 px                    |
| Two `tspan`s, the second without `x`                      | second part 13 px too far right |
| Two `tspan`s, the second with `x` from the measured width | 0 / −1 px                       |

Consequences:

- **Kerning and ligatures stay on.** The Rust measurement with the font's default features is what all three engines draw. `all-plain.svg` (kerning and ligatures off on both sides) and `all-length.svg` (`textLength`) remain in the spike as the experiments that were tried; neither is needed.
- **Never measure with WebKit's SVG text API.** Any check, test or tool that compares widths in WebKit has to use character positions or pixels, not `getComputedTextLength` or `getBBox`. The JS reference uses exactly these calls wherever it measures SVG text: labels with `htmlLabels: false` (`createText.ts`, `rendering-elements/shapes/util.ts`) and diagrams that draw SVG text directly, such as sequence. In Safari it therefore sizes those boxes and wraps those lines for unkerned text, while Safari draws kerned text. HTML labels are measured with `getBoundingClientRect` and are not affected. ferrigura measures without a browser, so its layouts are the same in every engine.
- **Every text run is placed explicitly.** ferrigura writes `text-anchor` start and an `x` computed from its own measurement for every line, and an explicit `x` for every `tspan` that follows another (a style change, a second word run). A centred label gets `x = centre − width / 2`. The browser then never derives a position from its own text width, so WebKit draws where Chromium and Firefox do. Mermaid centres labels with `text-anchor: middle`, so in Safari its SVG labels with noticeable kerning sit off centre.
- **No per-character positions and no `textLength`.** Both split WebKit's text into single characters and drop kerning from the ink.
- **The lab** (`spikes/text-measure/lab/`, in the history since 2026-10-21) checked further engines and font settings without Playwright; the WebKit check of the fallback faces used `tools/port/fonts/webkit-snapshot.swift` instead.

One more observation, not investigated: in WebKit, the lab's HTML reference spans changed width after SVG text with other feature settings had been laid out, as if a cache kept features across font descriptions. ferrigura sets no feature switches, so this does not touch it.

## Not covered yet

- Mixed styles within one line (bold or italic words from Markdown); a straightforward extension of the per-word measurement.
- Fallback fonts for scripts Lexica does not cover (CJK, Arabic, emoji).
- Caching of shaper data and per-word widths; not needed at 5 µs per measurement.
- Whether `Intl.Segmenter` and `unicode-segmentation` agree on graphemes for the newest emoji sequences.
- Firefox through Playwright: its Firefox 153 build (1538) hangs at launch on macOS 27.0.1, also outside the shell sandbox and with software rendering, so `all.svg` was not run in Firefox; the installed Firefox 155 ran the lab. To be run on Linux, for example in CI.
- WOFF2: the subsets are embedded as TTF; WOFF2 would roughly halve them, but needs a Rust encoder that has not been evaluated.
