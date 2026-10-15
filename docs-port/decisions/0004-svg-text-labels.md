# Labels are SVG text, not HTML in foreignObject

Labels are written as SVG `<text>` and `<tspan>`. Markdown in labels (bold, italic, code, line breaks) is parsed by Ferromark and written as styled `tspan` runs. The `htmlLabels` option goes away. With `foreignObject`, we would measure the box but the viewer's browser would lay out and wrap the HTML with its own fonts, so text could be clipped or overflow; and many SVG consumers (PDF converters, Inkscape, resvg) do not render `foreignObject` at all. With SVG text we decide every line break ourselves, and the result looks the same everywhere.

## Consequences

- Output looks different from Mermaid's default, which uses HTML labels.
- Math in labels (KaTeX produces HTML and MathML) is out of scope.
