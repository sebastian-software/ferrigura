# Measure only text, with bundled fonts

Text is the only thing ferrigura measures; every other size is calculated. Measuring needs the exact font the viewer will see, and Mermaid's defaults (Trebuchet MS, Verdana, Arial) are system fonts we may not ship. We bundle Lexica Ultralegible (regular, italic, bold, bold italic; based on Atkinson Hyperlegible) and Atkinson Hyperlegible Mono for code, both under the SIL Open Font License, and embed the glyphs a diagram uses as a subset in the SVG by default, so every viewer renders what we measured. Further fonts can be registered, with a fallback chain and a warning for missing glyphs. Measurement uses `rustybuzz` for shaping, `ttf-parser` for metrics and `unicode-linebreak` for break opportunities; width-based line breaking is our own code.

## Considered Options

- **Reference a font family by name without embedding:** smaller files, but the viewer may substitute a font with different metrics.
- **Convert text to paths:** exact, but not selectable, searchable or accessible.
- **`cosmic-text`:** shaping, fallback and layout in one, but much heavier than the parts we need.

## Consequences

Lexica covers Latin scripts only; CJK, Arabic and emoji need registered fallback fonts.
