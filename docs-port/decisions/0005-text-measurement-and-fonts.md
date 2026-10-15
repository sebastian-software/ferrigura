# Measure only text, with bundled fonts

Text is the only thing ferrigura measures; every other size is calculated. Measuring needs the exact font the viewer will see, and Mermaid's defaults (Trebuchet MS, Verdana, Arial) are system fonts we may not ship. We bundle Lexica Ultralegible (regular, italic, bold, bold italic; based on Atkinson Hyperlegible) and Atkinson Hyperlegible Mono for code, both under the SIL Open Font License, and embed the glyphs a diagram uses as a subset in the SVG by default, so every viewer renders what we measured. Further fonts can be registered, with a fallback chain and a warning for missing glyphs. Measurement uses `harfrust` (the HarfBuzz project's Rust port) for shaping and the `read-fonts` model for metrics, rounding ascent and descent to whole pixels as browsers do. Line wrapping ports Mermaid's own algorithm for SVG labels (break at spaces, pack greedily, split an overlong word into graphemes with `unicode-segmentation`), so lines match the JS reference.

## Considered Options

- **Reference a font family by name without embedding:** smaller files, but the viewer may substitute a font with different metrics.
- **Convert text to paths:** exact, but not selectable, searchable or accessible.
- **`cosmic-text`:** shaping, fallback and layout in one, but much heavier than the parts we need.
- **`rustybuzz` and `ttf-parser`:** the first plan; `rustybuzz` has had no release since November 2024, while HarfRust is maintained by HarfBuzz itself.
- **Wrapping at UAX #14 break opportunities (`unicode-linebreak`):** reads better for dates and hyphenated words, but breaks lines differently from the JS reference.

## Consequences

- Lexica covers Latin scripts only; CJK, Arabic and emoji need registered fallback fonts.
- Subsets are made with `skera` (fontations' port of `hb-subset`), and every embedded subset gets a font family name of its own, because `@font-face` rules are global to an HTML page and two diagrams with different subsets under one name break each other's labels.
- Revised on 2026-10-15 after the [text measurement spike](../spikes/text-measurement.md), which found widths within Chromium's 1/64 px rounding, exact text box heights, and identical wrapping.
