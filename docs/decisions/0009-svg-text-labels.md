# 0009 Labels are SVG text

**Scope:** how every label, title and text of a diagram is written into the SVG. It does not cover how text is measured and which fonts are used ([0010](./0010-text-measurement-and-fonts.md)).

## Decision

Labels are written as SVG `<text>` and `<tspan>`, never as HTML in `foreignObject`. Markdown in labels (bold, italic, code, line breaks) is read with ferrigura's port of marked's lexer, the one Mermaid reads it with, and written as styled `tspan` runs. ferrigura decides every line break. The option `htmlLabels: true` is drawn as SVG text too.

## Why

With `foreignObject`, ferrigura would measure the box, but the viewer's browser would lay out and wrap the HTML with its own fonts, so text could be clipped or overflow. Many SVG consumers (PDF converters, Inkscape, resvg) do not render `foreignObject` at all. With SVG text, the output looks the same everywhere.

## Consequences

- The output differs from Mermaid's default, which uses HTML labels. The reference is run with `htmlLabels: false` for the comparison ([0015](./0015-differential-testing.md)).
- Math in labels (`$$…$$`, which KaTeX renders as HTML and MathML) is out of scope.
- `pnpm -C tools/port port:diff markdown` compares the lines and words of the Markdown reader with marked's.

## Related

[0006](./0006-rust-only-no-browser.md), [0010](./0010-text-measurement-and-fonts.md).
