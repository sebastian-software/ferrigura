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

## Not covered yet

- Mixed styles within one line (bold or italic words from Markdown); a straightforward extension of the per-word measurement.
- Fallback fonts for scripts Lexica does not cover (CJK, Arabic, emoji).
- Caching of shaper data and per-word widths; not needed at 5 µs per measurement.
- Whether `Intl.Segmenter` and `unicode-segmentation` agree on graphemes for the newest emoji sequences.
