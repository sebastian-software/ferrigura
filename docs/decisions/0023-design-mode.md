# 0023 A design mode beside the reference's look

**Scope:** the per-render option that draws a diagram in ferrigura's own look, what that look changes, and how it relates to the compatibility switch. It does not change parsing, Mermaid configuration, text measurement or the layout engines.

## Decision

`RenderOptions.design` selects the design mode. It defaults to `false`, and an absent option preserves the output byte for byte. Node.js takes `design: true`; the CLI takes `--design`. It is a render option, not a Mermaid configuration key: front matter and init directives cannot set it.

The mode changes paint, and the place of a group's title. Node sizes, positions, edge routes and edge-label positions are those of the default rendering ([0019](./0019-adaptive-flowchart-mode.md)), so the layout comparison with the reference keeps covering them.

- **Colors are theme variables.** The mode lays its own values for the shared color tokens under the user's `themeVariables`, which win key by key. Elements and custom properties keep reading the theme tokens ([0011](./0011-themes-as-css-variables.md)). It has a light and a dark set: the color scheme chooses, and without one the configured theme's name does.
- **Color follows a node's role, not its order.** A shape with a conventional meaning carries a `data-role` attribute that the style sheet colors: `terminal` for the stadium and the circles, `decision` for the diamond, `data` for the cylinder. Other shapes stay neutral. The theme's palette is not used: groups are neutral.
- **Outlines are hairlines.** A node outline is 1.25 px and an edge 1.5 px, so that nodes stand in front of their connections. A thick edge is 3 px.
- **Edges are drawn anew through the same points.** A rounded edge turns on a radius of 10 instead of 5, also where it hops over another edge. The arrowhead is slimmer than the reference's and notched at the back; its tip stays on the end of the path. An edge label sits in a chip: its background, 5 wider on each side, with a corner radius of 6 and a thin outline.
- **Corners and shadows are soft.** A rectangle node has a corner radius of 8, a group frame of 12. The look's drop shadow is short and blurred, and a group frame has none.
- **A group title keeps clear of the edges.** It sits at the left of its frame, 16 from the edge, and moves right past every edge that crosses its row. When the row has no free place wide enough, it stays at the left.
- **Contrast is checked.** In both sets, text reaches 4.5 to 1 on its ground, and outlines and edges reach 3 to 1 on the background and inside a group (WCAG 2.1, 1.4.3 and 1.4.11). The background is the mode's `background` variable: the mode cannot know the page behind a transparent SVG.
- **A `classDef` or `style` of the diagram keeps the last word.** The mode's rules come before the class rules.

Flowcharts support the mode. Another diagram type returns an option error when the mode is requested, as an unsupported draw animation does ([0021](./0021-optional-svg-draw-animation.md)).

`design` and `compat` exclude each other; the combination returns an option error. The design mode builds on the optimized default and is free to depart from the reference's geometry.

## Why

The default look follows Mermaid's, which reads as generated: every stroke has the same weight, color cycles with the order of the elements, and a dark scheme is white outlines on black. A diagram that looks designed needs choices the reference does not make. A separate mode keeps both promises: the default stays comparable with the reference, and the design mode is free to differ.

A render option instead of a theme name or a `look` value keeps diagram texts portable: the same text renders in Mermaid, in ferrigura's default and in the design mode. Laying the colors under the user's variables keeps one path for colors and keeps overrides working.

Starting with paint keeps the first step checkable: with identical nodes and routes, the difference between the two renders is the look and nothing else.

The title moves because the centered title is where edges enter a frame from above. The fix belongs to this mode: the default reproduces the reference's title, and changing it there needs a patch on the reference ([0017](./0017-reproduce-the-reference.md)).

Weighed and not chosen: changing the default look, which breaks the comparison with the reference for every user; and a shipped theme pair under a new `theme` name, which cannot carry stroke widths, radii, shadows or roles.

## Consequences

- The reference has no counterpart to this mode. The mode is checked by ferrigura's own tests: nodes and routes equal the default rendering's, a unit test holds both color sets to the contrast ratios, and the public render tests cover the roles, the title, both color schemes and the precedence of the user's styles.
- A diagram type gains the mode in its style sheet and shapes; the option error lists nothing and names no types, so adding a type changes no message.
- Roles are read from the shape alone. A diagram that uses one of those shapes for every node is colored throughout.

## Related

[0011](./0011-themes-as-css-variables.md), [0017](./0017-reproduce-the-reference.md), [0019](./0019-adaptive-flowchart-mode.md), [0021](./0021-optional-svg-draw-animation.md), [flowchart](../diagrams/flowchart.md#design-mode).
