# 0010 Text measurement and bundled fonts

**Scope:** how ferrigura measures and wraps text, which fonts it ships, and how they reach the viewer (`ferrigura-text`). It does not cover how labels are written into the SVG ([0009](./0009-svg-text-labels.md)).

## Decision

**Only text is measured; every other size is calculated.** Measuring needs the exact font the viewer sees, so ferrigura bundles its fonts and embeds the glyphs a diagram uses as a subset in the SVG by default.

- **Faces:** Lexica Ultralegible (regular, italic, bold, bold italic; based on Atkinson Hyperlegible) for text and Atkinson Hyperlegible Mono for code, under the SIL Open Font License. For what they lack: Noto Sans Math for the arrows (U+2190–21FF, its arrows block only), Noto Sans Symbols 2 for the symbols of U+2600–27BF, and a cut of Twemoji Mozilla for the color emoji with emoji presentation.
- **Fallback:** as a browser does with a `font-family` list, a run is split where a face lacks a character and the next face has it; each part is shaped and measured in its own face, and the text names every family it uses, so the viewer draws what was measured. Further fonts can be registered, with a fallback chain and a warning for missing glyphs.
- **Measurement:** `harfrust` (HarfBuzz's Rust port) for shaping and `read-fonts` for metrics, with kerning and ligatures, rounding ascent and descent to whole pixels as browsers do.
- **Wrapping:** a port of Mermaid's own algorithm for SVG labels (break at spaces, pack greedily, split an overlong word into graphemes with `unicode-segmentation`), so lines break where Mermaid breaks them.
- **Subsets:** made with `skera` (fontations' port of `hb-subset`); each embedded subset gets a family name of its own.

## Why

Mermaid's default fonts (Trebuchet MS, Verdana, Arial) are system fonts that cannot be shipped, and a font that differs from the measured one breaks the layout. Embedded subsets make every viewer render exactly what was measured, and keep the text selectable, searchable and accessible. Widths match Chromium within its 1/64 px rounding, text box heights exactly, and wrapping is identical ([LEARNINGS.md](../LEARNINGS.md)).

Weighed and not chosen: naming a font family without embedding it (smaller, but the viewer may substitute other metrics); converting text to paths (exact, but not selectable or accessible); `cosmic-text` (much heavier than the parts needed); `rustybuzz` and `ttf-parser` (unmaintained in favor of HarfRust); and wrapping at UAX #14 break opportunities (reads better for some text, but breaks lines differently from Mermaid).

## Consequences

- `@font-face` rules are global to an HTML page, so two diagrams with different subsets under one family name would break each other's labels; hence the unique names per subset.
- WebKit lays SVG text out from unkerned per-character widths and paints it kerned, so it can place a kerned run away from where it was measured. The labels of the unified renderer are therefore placed run by run with an explicit `x` and `text-anchor="start"` (`ferrigura-render/src/unified/label.rs`); the custom renderers keep the reference's `middle` and `end` anchors for their titles and labels, which WebKit may draw slightly off.
- CJK, kana, Arabic and the emoji and symbols the bundled faces lack need registered fallback fonts.
- The reference loads the same font files as web fonts for the geometry comparison ([0015](./0015-differential-testing.md)).

## Related

[0009](./0009-svg-text-labels.md), [0013](./0013-licenses-of-ported-code.md) for the fonts' licenses, [LEARNINGS.md](../LEARNINGS.md).
