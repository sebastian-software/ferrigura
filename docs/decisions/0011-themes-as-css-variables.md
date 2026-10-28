# 0011 Themes as CSS custom properties

**Scope:** how colors reach the SVG, which themes ship, and how light and dark output is chosen. It does not cover the look (`classic`, `neo`, `handDrawn`), which shapes strokes, not colors.

## Decision

A theme is a set of semantic color tokens; derived colors are computed in Rust, as the reference computes them (a port of `khroma`), not with CSS `color-mix()`.

- The SVG writes every color as a CSS custom property in its `<style>` block, and each element also carries its concrete color as a presentation attribute, so viewers without support for CSS variables (resvg, some PDF converters) still render it.
- `colorScheme: light | dark | auto` selects the output; `auto` writes both token sets with a `prefers-color-scheme` media query, so one SVG follows the viewer's light or dark mode.
- `themeVariables` set token values.
- `redux-color` (light) and `redux-dark-color` (dark), Mermaid's current defaults, are the shipped themes. Other theme names fall back to the light one, or to the dark one for a name that ends in `dark`.

## Why

Custom properties let one SVG follow the viewer's color scheme and let a page restyle diagrams, while the attributes keep simple renderers working. Shipping Mermaid's own defaults keeps the geometry comparison with the reference meaningful.

## Consequences

- The tokens are typed ([0008](./0008-canonical-model-and-typed-views.md)); `pnpm -C tools/port port:diff theme` compares the computed variables with the reference's.
- Mermaid's classic themes (`default`, `dark`, `forest`, `neutral`, `base`) are not ported.

## Related

[0008](./0008-canonical-model-and-typed-views.md).
