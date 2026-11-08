# 0011 Themes as CSS custom properties

**Scope:** how colors reach the SVG, which themes ship, and how light and dark output is chosen. It does not cover the look (`classic`, `neo`, `handDrawn`), which shapes strokes, not colors.

## Decision

A theme is a set of semantic color tokens; derived colors are computed in Rust, as the reference computes them (a port of `khroma`), not with CSS `color-mix()`.

- The SVG writes every color as a CSS custom property in its `<style>` block, and each element also carries its concrete color as a presentation attribute, so viewers without support for CSS variables (resvg, some PDF converters) still render it.
- `colorScheme: light | dark | auto` selects the output. `auto` writes every color that differs between the two token sets as `light-dark(light, dark)`, which resolves against the `color-scheme` the SVG inherits: inline in a page it follows the page, and as a standalone document `svg:root` declares `color-scheme: light dark`, so it follows the viewer. A value that `light-dark()` cannot hold (not a color, or declared only by the dark set) stays under `prefers-color-scheme: dark`.
- `themeVariables` set token values.
- `redux-color` (light) and `redux-dark-color` (dark), Mermaid's current defaults, are the shipped themes. Other theme names fall back to the light one, or to the dark one for a name that ends in `dark`.

## Why

Custom properties let one SVG follow the viewer's color scheme and let a page restyle diagrams, while the attributes keep simple renderers working. `light-dark()` instead of a media query lets a page with its own theme switch (a `color-scheme` set from a toggle) recolor its diagrams; a media query only sees the operating system's setting. Shipping Mermaid's own defaults keeps the geometry comparison with the reference meaningful.

## Consequences

- The tokens are typed ([0008](./0008-canonical-model-and-typed-views.md)); `pnpm -C tools/port port:diff theme` compares the computed variables with the reference's.
- Mermaid's classic themes (`default`, `dark`, `forest`, `neutral`, `base`) are not ported.
- An inline diagram in a page that declares no `color-scheme` stays light; a page that supports dark mode declares it. `light-dark()` needs a browser from 2024 or later; renderers without it fall back to the presentation attributes.

## Related

[0008](./0008-canonical-model-and-typed-views.md).
