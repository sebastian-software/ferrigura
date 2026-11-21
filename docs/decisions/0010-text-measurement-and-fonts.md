# 0010 Text measurement and bundled fonts

**Scope:** how ferrigura measures and wraps text, which fonts it ships, and how they reach the viewer (`ferrigura-text`). It does not cover how labels are written into the SVG ([0009](./0009-svg-text-labels.md)).

## Decision

**Only text is measured; every other size is calculated.** Measuring needs the exact font the viewer sees, so ferrigura bundles its fonts and embeds the glyphs a diagram uses as a subset in the SVG by default.

- **Registered faces:** the text and renderer APIs accept explicit local TTF or OTF bytes or paths through `FontRegistry`. Each resolved `Arc<Face>` owns its bytes and explicit fallback chain. Family lists resolve against registered names; weights select the nearest design in the requested style, with upright fallback for missing italic. Missing characters are measured with the primary face’s missing glyph and are observable through `FontRegistry::missing_glyphs`; viewers may substitute a system font, so deterministic layout requires registered coverage. No system font discovery or downloads occur.
- **Compatible defaults when no fonts are supplied:** Lexica Ultralegible (regular, italic, bold, bold italic; based on Atkinson Hyperlegible) for text and Atkinson Hyperlegible Mono for code, under the SIL Open Font License. For what they lack: Noto Sans Math for the arrows (U+2190–21FF, its arrows block only), Noto Sans Symbols 2 for the symbols of U+2600–27BF, and a cut of Twemoji Mozilla for the color emoji with emoji presentation.
- **Fallback:** as a browser does with a `font-family` list, a run is split where a face lacks a character and the next face has it; each part is shaped and measured in its own face, and the text names every family it uses, so the viewer draws what was measured. The text registry accepts further fonts with an explicit fallback chain and returns diagnostics for missing glyphs.
- **Measurement:** `harfrust` (HarfBuzz's Rust port) for shaping and `read-fonts` for metrics, with kerning and ligatures, rounding ascent and descent to whole pixels as browsers do.
- **Wrapping:** a port of Mermaid's own algorithm for SVG labels (break at spaces, pack greedily, split an overlong word into graphemes with `unicode-segmentation`), so lines break where Mermaid breaks them.
- **Subsets:** made with `skera` (fontations' port of `hb-subset`); each embedded subset gets a family name of its own.

Supplied fonts belong to each immutable renderer, including their owned bytes, fallback chain,
and CSS identities. Concurrent renderers cannot change one another's selection. Their caches
include the supplied inputs. Global and per-type family settings resolve registered aliases;
implicit per-type defaults inherit the global primary, while explicit unknown names fail with
registration guidance. [The font API](../fonts.md) defines the host-facing contract.

Each supplied design has a distinct embedded family, and `font-synthesis: none` prevents the
viewer inventing a weight or slant that was not measured. Variable faces are fixed to all their
default axes with an `@font-face` descriptor. CFF and CFF2 faces embed complete because the
subsetter cannot serialize their outlines; their metrics and ink bounds still use the real font.

## Why

Mermaid's default fonts (Trebuchet MS, Verdana, Arial) are system fonts that cannot be shipped, and a font that differs from the measured one breaks the layout. With registered character coverage, embedded subsets let viewers render the measured designs, and keep the text selectable, searchable and accessible. Widths match Chromium within its 1/64 px rounding, text box heights exactly, and wrapping is identical ([LEARNINGS.md](../LEARNINGS.md)).

Weighed and not chosen: naming a font family without embedding it (smaller, but the viewer may substitute other metrics); converting text to paths (exact, but not selectable or accessible); `cosmic-text` (much heavier than the parts needed); `rustybuzz` and `ttf-parser` (unmaintained in favor of HarfRust); and wrapping at UAX #14 break opportunities (reads better for some text, but breaks lines differently from Mermaid).

## Consequences

- `@font-face` rules are global to an HTML page, so two diagrams with different subsets under one family name would break each other's labels; hence the unique names per subset.
- WebKit lays SVG text out from unkerned per-character widths and paints it kerned, so it can place a kerned run away from where it was measured. The labels of the unified renderer are therefore placed run by run with an explicit `x` and `text-anchor="start"` (`ferrigura-render/src/unified/label.rs`); the custom renderers keep the reference's `middle` and `end` anchors for their titles and labels, which WebKit may draw slightly off.
- CJK, kana, Arabic and the emoji and symbols the bundled faces lack need registered fallback fonts.
- The reference loads the same font files as web fonts for the geometry comparison ([0015](./0015-differential-testing.md)).

## Related

[0009](./0009-svg-text-labels.md), [0013](./0013-licenses-of-ported-code.md) for the fonts' licenses, [LEARNINGS.md](../LEARNINGS.md).
