# 0023 A design mode beside the reference's look

**Scope:** the per-render option that draws a diagram in ferrigura's own look and with its own measures, what both change, and how the mode relates to the compatibility switch. It does not change parsing, Mermaid configuration, text measurement or the layout engines.

## Decision

`RenderOptions.design` selects the design mode. It defaults to `false`, and an absent option preserves the output byte for byte. Node.js takes `design: true`; the CLI takes `--design`. It is a render option, not a Mermaid configuration key: front matter and init directives cannot set it.

The mode has its own look and its own measures. The look is paint: colors, outlines, corners, arrowheads. The measures change what the layout engine is given: node sizes, spacings, paddings and options. The engines themselves stay exact ports ([0012](./0012-layout-engines-ported-exactly.md)), and the mode starts from the optimized default's layout input ([0019](./0019-adaptive-flowchart-mode.md)).

- **Colors are theme variables.** The mode lays its own values for the shared color tokens under the user's `themeVariables`, which win key by key. Elements and custom properties keep reading the theme tokens ([0011](./0011-themes-as-css-variables.md)). It has a light and a dark set: the color scheme chooses, and without one the configured theme's name does.
- **Color follows a node's role, not its order.** A shape with a conventional meaning carries a `data-role` attribute that the style sheet colors: `terminal` for the stadium and the circles, `decision` for the diamond, `data` for the cylinder. Other shapes stay neutral. The theme's palette is not used: groups are neutral.
- **Outlines are hairlines.** A node outline is 1.25 px and an edge 1.5 px, so that nodes stand in front of their connections. A thick edge is 3 px.
- **Edges are drawn anew through the points of their route.** A rounded edge turns on a radius of 10 instead of 5, also where it hops over another edge. It is drawn without the points the layout leaves on its straight parts: a corner takes at most half of each part that meets in it, so a point shortly before a corner would make that turn tighter than the others. The arrowhead is slimmer than the reference's and notched at the back; its tip stays on the end of the path. An edge label interrupts its edge: the edges are masked in the label's box, which is its text's box and 5 more on each side with a corner radius of 6, so the label needs no background and reads on any ground. The layout is given that box's width and 6 of room at both ends, so that it does not touch a node beside it. A group's frame is pulled in around its members after the layout and can then cross a label's box; such a label slides along the straight part of its edge, the shorter way, until the box is 6 clear of the frame.
- **A node gives its edges room.** A node grows along the side its edges meet until its busiest side has 20 for each edge and 24 at both ends: along its height in a layout from left to right, along its width from top to bottom. The edges into a node count for one side, the edges out of it for the other. A shape that takes no size of its own, such as the stadium and the diamond, stays as it is.
- **The edges of a side meet the node around its middle.** They sit 20 apart, as a bundle in the middle of the side, however long the side is: the layout is given 20 as the distance between ports in every graph, and the center alignment for the ports of every node. The default spreads the edges over the side, which puts the outer ones where the corner turns. The 20 replace the default's label-aware distance ([0019](./0019-adaptive-flowchart-mode.md)).
- **Spacing is wider.** The base value of the root's spacings is 48 where the reference has 40. A group pads its members by 32, and by 44 at the top, where its title has a row of its own, 12 below the frame's edge.
- **Nodes stand on axes.** ELK decides the layers and the order of the nodes in them; where the nodes stand along their layer is the mode's own placement, run after ELK's network simplex placement in every graph. An edge that is in the middle of its side at both of its nodes is an axis edge: the edge of a chain, the middle edge of a fan. Its nodes stand on one line and it runs straight, before any other edge does. Of two edges in the middle of an even side, the one that runs with the flow and passes fewer layers is the axis edge, and the bundle moves by half the distance between two edges, so that the axis edge leaves from the middle of the node; a main edge beside the middle of an odd side takes the axis from an edge against the flow. A node between two equals stands in their middle. An edge between two nodes counts for more than the end of a long edge, and an edge against the flow for less, so the main flow lines up and what goes around it bends. The placement is ferrigura's own and lives in the renderer, not in the ELK port: the port hands it the layers as plain data after its own placement and takes the places back (`ferrigura_elk::placement`), so the port stays a port ([0012](./0012-layout-engines-ported-exactly.md)) under its own license ([0013](./0013-licenses-of-ported-code.md)). The default and `compat` never run it. A `nodePlacementStrategy` in the configuration is the user's choice and stays, without the mode's placement.
- **Corners and shadows are soft.** A rectangle node has a corner radius of 8, a group frame of 12. The look's drop shadow is short and blurred, and a group frame has none.
- **A group title keeps clear of the edges.** It sits at the left of its frame, 16 from the edge, and moves right past every edge that crosses its row. When the row has no free place wide enough, it stays at the left.
- **Contrast is checked.** In both sets, text reaches 4.5 to 1 on its ground, and outlines and edges reach 3 to 1 on the background and inside a group (WCAG 2.1, 1.4.3 and 1.4.11). The background is the mode's `background` variable: the mode cannot know the page behind a transparent SVG.
- **A `classDef` or `style` of the diagram keeps the last word.** The mode's rules come before the class rules.

Flowcharts support the mode. Another diagram type returns an option error when the mode is requested, as an unsupported draw animation does ([0021](./0021-optional-svg-draw-animation.md)).

`design` and `compat` exclude each other; the combination returns an option error. The design mode builds on the optimized default and is free to depart from the reference's geometry.

## Why

The default look follows Mermaid's, which reads as generated: every stroke has the same weight, color cycles with the order of the elements, and a dark scheme is white outlines on black. A diagram that looks designed needs choices the reference does not make. A separate mode keeps both promises: the default stays comparable with the reference, and the design mode is free to differ.

A render option instead of a theme name or a `look` value keeps diagram texts portable: the same text renders in Mermaid, in ferrigura's default and in the design mode. Laying the colors under the user's variables keeps one path for colors and keeps overrides working.

The reference sets a diagram tightly: four edges meet a node 41 high, and a group's title shares its row with the edges that enter the group. A drawing that reads as designed gives lines room. Growing the node, not only spreading its ports, is what a person drawing it by hand would do.

The measures are given to the engine as its own options and as node sizes, the way the adaptive port policy is ([0019](./0019-adaptive-flowchart-mode.md)). Changing the engine would end its comparison with the reference; changing its input does not.

The title moves because the centered title is where edges enter a frame from above. The fix belongs to this mode: the default reproduces the reference's title, and changing it there needs a patch on the reference ([0017](./0017-reproduce-the-reference.md)).

Weighed and not chosen: changing the default look, which breaks the comparison with the reference for every user; and a shipped theme pair under a new `theme` name, which cannot carry stroke widths, radii, shadows or roles.

## Consequences

- The reference has no counterpart to this mode, and its geometry is not compared with the reference's. The engine that lays it out is: the default and the compatibility rendering run the same code against the reference. The mode's own tests cover the room a node gives its edges, the group's padding and title, the roles, both color schemes, the contrast ratios and the precedence of the user's styles.
- A diagram in the mode is larger than in the default rendering.
- A diagram type gains the mode in its style sheet and shapes; the option error lists nothing and names no types, so adding a type changes no message.
- Roles are read from the shape alone. A diagram that uses one of those shapes for every node is colored throughout.

## Related

[0011](./0011-themes-as-css-variables.md), [0012](./0012-layout-engines-ported-exactly.md), [0017](./0017-reproduce-the-reference.md), [0019](./0019-adaptive-flowchart-mode.md), [0021](./0021-optional-svg-draw-animation.md), [flowchart](../diagrams/flowchart.md#design-mode).
