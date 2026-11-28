# 0019 Mermaid's geometry with `compat`

**Scope:** the per-render switch that draws the reference's geometry and look instead of ferrigura's own, and the corrections that hold in both renderings. What the default rendering draws is [0023](./0023-the-default-rendering.md). It does not change parsing, Mermaid configuration or the layout engines.

## Decision

`RenderOptions.compat` defaults to `false`: a render draws ferrigura's own look and measures ([0023](./0023-the-default-rendering.md)). `compat: true` draws what the reference draws, for every diagram type: its layout input, its geometry, its colors and its look. Node.js uses the same `compat` boolean; the CLI uses `--compat`. This is a render option, not a Mermaid configuration key: front matter and init directives cannot set it. There are two renderings, the default and `compat`, and no third.

In `compat`, flowcharts keep the reference's attachment policy, sequence participant boxes span the shared row, and a mindmap is laid out as the reference lays it out, by cose-bilkent unless its configuration names another layout. ferrigura's own steps, the measures, the placement, the side exits, the notes and self-loops beside their nodes and the mindmap's tree, do not run.

Font, fill and unintended-shadow corrections apply in both renderings. Edge-label backgrounds in the shared renderer are drawn at full opacity instead of applying Mermaid's extra factor of 0.5, matching the opaque paragraph background of HTML labels; each diagram keeps its own theme fill, including its alpha channel, so an explicitly transparent or translucent theme color remains so. The correction is applied once in the shared edge-label painter, for flowchart, class, state and ER diagrams. Icon and image captions and node backgrounds keep their reference styling. Reference patches 11 and 12 apply the same paint-only correction, so the comparison still holds. The default masks its edges under the label instead, and draws no background ([0023](./0023-the-default-rendering.md)).

## Why

ferrigura's promise is two-sided. Its default draws a diagram that reads as designed, and `compat` is the drawing that is checked against the reference: a user who needs Mermaid's picture gets it, and every check of the port has a rendering to run on ([0015](./0015-differential-testing.md)). One switch for every type keeps that choice simple: a caller does not need to know which types depart from the reference, and a document is either in ferrigura's look or in Mermaid's throughout.

Mermaid's normal HTML-label path gives the paragraph inside an edge label its own background color. Its SVG path instead dims the background rectangle to half opacity, allowing the edge to show through the text. Ferrigura always uses SVG text, so full-opacity backgrounds give `compat` the line masking of Mermaid's HTML labels. The pinned reference carries the same paint-only correction in patches 11 and 12. See [the upstream styles](https://github.com/mermaid-js/mermaid/blob/e28fbd9ded0a07ba9569e51be4764c1ad4daa8f6/packages/mermaid/src/diagrams/flowchart/styles.ts).

Weighed and not chosen: an option per type or per feature, which would let one page mix both drawings; and keeping a geometry between the two, the reference's look with some of ferrigura's measures, which would be a third rendering that neither matches the reference nor reads as designed.

## Consequences

- `compat` is compared with the unmodified reference input, for the layout data, the layout coordinates and the rendered geometry ([0015](./0015-differential-testing.md)). The default is not; its own tests and the snapshots hold it.
- The configuration and theme caches remain shared; the rendering belongs to each render and cannot leak between calls.
- The stylesheet comparison checks `compat` rule by rule. Public render tests cover background treatment across the shipped themes, preserve explicit color transparency and switch the rendering per render without leaking cached state.
- Snapshots pin both renderings: the default hash is the second column of `svg.tsv`, the `compat` hash the third, for every corpus file.

## Related

[0012](./0012-layout-engines-ported-exactly.md), [0014](./0014-upstream-reference-at-a-pin.md), [0015](./0015-differential-testing.md), [0017](./0017-reproduce-the-reference.md), [0023](./0023-the-default-rendering.md).
