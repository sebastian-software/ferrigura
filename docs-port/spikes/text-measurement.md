# Spike: measuring text without a browser

**Question.** Can ferrigura measure label text in Rust, without a browser, closely enough to lay diagrams out like the JS reference does in Chromium?

**Answer.** Yes. With the same font files, Rust and Chromium agree on every width to within Chromium's own layout rounding, on text box heights exactly, and a port of Mermaid's SVG line wrapping produces the same lines in every case tested.

Code and how to run it: [spikes/text-measure/](../../spikes/text-measure/README.md). Run on 2026-10-15.

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

## Part 3: WebKit, and kerning

**Question.** Do other engines draw the embedded labels at the Rust width too?

**Answer.** WebKit does not, as long as the text is kerned: it lays out SVG `text` without kerning and without ligatures, while its HTML and canvas text keep both. Two ways make WebKit match exactly. Firefox could not be tested.

Setup: `embed.rs` also writes `all-plain.svg`, with `font-kerning: none` and `font-variant-ligatures: none` on `text` and the Rust widths measured with `kern`, `liga`, `clig` and `calt` off, and `all-length.svg`, which is `all.svg` with every label's Rust width set as `textLength` (`lengthAdjust="spacing"`). `embed-check.ts <dir> webkit` runs the checks in WebKit 26.5 (Playwright). Run on 2026-10-16.

| Check, 1,690 labels                                                  | Chromium                             | WebKit                                                  |
| -------------------------------------------------------------------- | ------------------------------------ | ------------------------------------------------------- |
| `all.svg`, kerned: width vs Rust                                     | max 0.0155 px                        | 956 labels off, up to 16 px (3.2 px on ordinary labels) |
| Same labels as HTML text or canvas, kerned                           | (Part 1)                             | HTML max 0.74 px (spaces case), canvas max 0.0003 px    |
| `all-plain.svg`, kerning and ligatures off on both sides             | max 0.0155 px                        | max 0.0003 px                                           |
| `all-length.svg`, kerned with `textLength`: character extent vs Rust | max 0.0155 px; pixels shift subpixel | max 0.0155 px                                           |
| Text box height vs Rust (ascent and descent rounded to whole pixels) | exact                                | off by 0.16–0.38 px, depending on size                  |
| `<img>` vs document                                                  | pixel-identical                      | pixel-identical                                         |

The largest WebKit differences are the kerning test line (`AVATAR Type WAVE Yo Te LT`, 16 px wider at 24 px) and the ligature test line (`office fluffy fjord affinity`). WebKit does not round ascent and descent to whole pixels the way Chromium does, so the text box sits up to 0.4 px differently. In Chromium, `getBBox` includes ink that overhangs the advance (a leading `j`, italics), so the checks compare character positions and `getComputedTextLength`, not the box width.

Two options for M2:

- **Plain text: kerning and ligatures off**, in the measurement and in the SVG's CSS. Exact in both engines, nothing per label, and every browser draws the same glyphs. The cost is typography: Lexica's kerning and its `fi`/`fl` ligatures are gone.
- **Kerned text with `textLength`** on every text line. Chromium draws kerned text; WebKit draws unkerned glyphs, spread to the measured width. Exact in both, but the two engines show different spacing, and `textLength` has to be set per line, which mixed styles inside a line make harder.

The geometry comparison with the JS reference (M5) is unaffected either way, as long as the reference run uses the same font settings.

## Not covered yet

- Mixed styles within one line (bold or italic words from Markdown); a straightforward extension of the per-word measurement.
- Fallback fonts for scripts Lexica does not cover (CJK, Arabic, emoji).
- Caching of shaper data and per-word widths; not needed at 5 µs per measurement.
- Whether `Intl.Segmenter` and `unicode-segmentation` agree on graphemes for the newest emoji sequences.
- Firefox: Playwright's Firefox 153 build (1538) hangs at launch on macOS 27.0.1, also outside the shell sandbox and with software rendering; to be run on Linux, for example in CI.
- WOFF2: the subsets are embedded as TTF; WOFF2 would roughly halve them, but needs a Rust encoder that has not been evaluated.
