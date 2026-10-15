# Themes are CSS custom properties with attribute fallback

A theme is a set of semantic colour tokens; derived colours are computed in Rust, not with CSS `color-mix()`. The SVG uses CSS custom properties in a `<style>` block for every colour, and each element also carries its concrete colour as a presentation attribute, so viewers without CSS variable support (resvg, some PDF converters) still render correctly. `colorScheme: light | dark | auto` selects the output; `auto` writes both token sets with a `prefers-color-scheme` media query, so one SVG follows the viewer's light or dark mode. `themeVariables` set token values.

Only `redux-color` (light) and `redux-dark-color` (dark), Mermaid's current defaults, ship at first, which keeps the geometry comparison with the JS reference meaningful; an own design can replace them later. Other theme names fall back to light or dark with a warning.
