# 0019 Optimized rendering with Mermaid compatibility

**Scope:** the per-render compatibility switch and the optimizations it controls: flowchart attachment policy and sequence participant boxes. It does not change parsing, Mermaid configuration or the layout engines.

## Decision

`compat` is the only render-mode option. `RenderOptions.compat` defaults to `false`: Ferrigura uses optimized rendering. `compat: true` selects Mermaid-compatible geometry across diagram types. Node.js uses the same `compat` boolean; the CLI uses `--compat`. This is a render option, not a Mermaid configuration key: front matter and init directives cannot set it.

With `compat: false`, flowcharts center their ports with label-aware spacing and neo sequence participant boxes fit their measured labels plus padding. With `compat: true`, flowcharts keep the reference's attachment policy and sequence boxes span the shared row. Compact header bottoms and footer tops remain on the same lifeline datum: message positions, row height and wrapping do not change.

Font, fill and unintended-shadow corrections apply in both modes. Flowchart edge-label backgrounds also remain at full opacity in both modes, matching the opaque paragraph background of HTML labels; reference patch 11 applies the same paint-only correction. Explicit theme color transparency still applies.

Adaptive routing changes the measured ELK input, before layout:

- Resolve each labeled edge's container as ELK does: the lowest common ancestor of its endpoints, or the parent of a self-loop. Include the intermediate containers on both endpoint branches, so a route crossing a frame gets consistent label-aware attachment spacing inside and outside. Edge storage does not determine these containers; Mermaid stores edges on the root before ELK imports them.
- For a parent traversed by a measured labeled edge, set `elk.portAlignment.default: CENTER` on its child nodes, including subgraph nodes.
- Set that parent's `elk.spacing.portPort` to `max(20, 2 × maximum measured edge-label height + 2)` in SVG user units. For vertical layouts, also reserve `maximum measured edge-label width / 2 + 2`: an inline label extends sideways toward the neighboring route. Read the direction from the container, inheriting it from its parent when absent or when `INCLUDE_CHILDREN` makes the nested direction inert. Label measurements include wrapping and font settings.
- Visit nested parents with the same rule. Without a positive measured label height on a route through the parent, leave the parent's child alignments and port spacing unchanged. This avoids adding steps to already straight parallel routes and dense unlabeled graphs.
- Let ELK fit the port group into the available node side, compress spacing when necessary, place nodes and route around obstacles. Keep its size constraints, the existing outline clipping, marker offsets, label guards and terminal straightener.

Both modes draw edge-label background rectangles at full opacity instead of applying Mermaid's extra factor of 0.5. The fill remains `edgeLabelBackground`, including its alpha channel: an explicitly transparent or translucent theme color remains so. Icon and image captions and other diagram types retain their reference styling.

The policy reduces unnecessary attachment offsets. It does not require every edge to be straight, freeze node positions or guarantee fewer bends for every graph. Both policies honor `elk.straightenEdges`; turning that option off changes post-processing, not the input handed to ELK.

## Why

Distributed attachment points on differently sized nodes can introduce small steps even in a simple review cycle. Centered ports with a gap based on actual labels give the existing straightener room to remove those steps. A fixed 44-unit gap works for the default review-cycle labels, but adds bends with long horizontal labels. Centering without label-aware spacing can retain detours in dense graphs. A wide inline label can also force a side step on a vertical route when the port gap accounts only for label height.

Mermaid's normal HTML-label path gives the paragraph inside an edge label its own background color. Its SVG path instead dims the background rectangle to half opacity, allowing the edge to show through the text. Ferrigura always uses SVG text, so full-opacity backgrounds give its default the line masking of Mermaid's HTML labels. The pinned reference carries the same paint-only correction in patch 11. See [the upstream styles](https://github.com/mermaid-js/mermaid/blob/e28fbd9ded0a07ba9569e51be4764c1ad4daa8f6/packages/mermaid/src/diagrams/flowchart/styles.ts).

Centered flowchart ports and compact participant boxes are deliberate choices beyond Mermaid parity. An explicit compatibility mode lets callers retain the reference's attachment policy. Both modes share the paint-only label correction, which is also applied to the pinned reference. Changing ELK itself would make the engine diverge; replacing bends with curves would conceal the attachment offsets.

## Consequences

- The optimized default changes flowchart routing and sequence participant-box geometry. Both modes use full-opacity flowchart edge-label backgrounds. Other diagram types retain their output.
- The configuration and theme caches remain shared; the rendering mode belongs to each render and cannot leak between calls.
- Compatibility geometry is compared with the unmodified reference input. Adaptive geometry is compared with the reference after the harness applies the same documented attachment policy to its measured input. ELK layout, clipping and SVG measurement still run on both sides; there is no geometry exception for the policy.
- The reference's input hook is test instrumentation, inactive without a callback. It does not turn the adaptive policy into a Mermaid bug fix. This is the explicit flowchart exception to [0017](./0017-reproduce-the-reference.md).
- The stylesheet comparison checks Mermaid mode rule by rule. Public render tests cover background treatment across the shipped themes, preserve explicit color transparency and switch both optimizations per render without leaking cached state.
- Snapshots pin both modes: the default hash is the second column of `svg.tsv`, and flowcharts and sequences have a third column for the compatibility-mode hash.

## Related

[0012](./0012-layout-engines-ported-exactly.md), [0014](./0014-upstream-reference-at-a-pin.md), [0015](./0015-differential-testing.md), [0017](./0017-reproduce-the-reference.md), [flowchart](../diagrams/flowchart.md#rendering-mode).
