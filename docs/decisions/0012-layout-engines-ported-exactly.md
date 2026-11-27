# 0012 Layout engines ported exactly

**Scope:** the layout engines that Mermaid uses for the diagram types ferrigura draws ([0024](./0024-a-fixed-list-of-diagram-types.md)), and the browser arithmetic their results depend on: ELK Layered, cose-bilkent, the tidy tree, JavaScript's `Math` and Chromium's path bounds. It does not cover the layouts that the custom renderers (sequence, gantt, pie and the like) compute inline, which are ported with their renderers.

## Decision

Every layout engine that Mermaid reaches for these types is ported from its own sources, so that the same input gives the same numbers, and is checked against the original on captured inputs.

| Engine                                                                                    | Source                                                        | Crate              |
| ----------------------------------------------------------------------------------------- | ------------------------------------------------------------- | ------------------ |
| ELK Layered, with Mermaid's ELK integration (`rendering-util/layout-algorithms/elk`)      | ELK's Java sources (elkjs is the same code compiled with GWT) | `ferrigura-elk`    |
| cose-bilkent 4.1.0 on `cose-base` 1.0.3 and `layout-base` 1.0.2 (mindmaps)                | JavaScript                                                    | `ferrigura-cose`   |
| The tidy tree of `@mermaid-js/layout-tidy-tree` 1.0.1 (mindmaps with `layout: tidy-tree`) | TypeScript, on `non-layered-tidy-tree-layout` 2.0.2           | `ferrigura-render` |
| `Math.sin`, `cos`, `log`, `pow` and the others as V8 computes them (V8's port of fdlibm)  | C++                                                           | `ferrigura-jsmath` |
| Chromium's bounding box of an SVG path                                                    | C++ (Skia)                                                    | `ferrigura-path`   |

Only the paths Mermaid reaches are ported. Of ELK, that is Layered: the other ELK algorithms (`elk.stress`, `elk.force`, `elk.mrtree` and the rest) fall back to Layered with a warning. Of cose-bilkent, that is the tree path, since a mindmap is always a tree; its random path is not reachable from Mermaid.

## Why

The compatibility promise is that the same text gives the same diagram, with the explicit flowchart attachment policy of [0019](./0019-adaptive-flowchart-mode.md). That policy changes the engine's input, not the engine. A layout of ferrigura's own would be deterministic too, but would have no reference to check against. Layout engines amplify the last bit of a number into visibly different positions, so the arithmetic must match as well: the platform's `sin` or `log` differs from V8's in the last bit for a few percent of inputs.

Weighed and not chosen: ELK Layered for every graph, including mindmaps (no new layout code, but not what Mermaid draws), and a port of dagre, which Mermaid still offers as `layout: dagre`. ELK is Mermaid's default layout, and ferrigura lays out `layout: dagre` with ELK as well; the geometry runs leave such fixtures out.

## Consequences

- Each engine crate has a differential test against the original on a captured corpus (`tests/corpus`), with a tolerance of zero where the arithmetic is ported.
- The engines are deterministic where Mermaid reaches them: the tree path of cose-bilkent draws no random numbers.
- `ferrigura-jsmath` rounds with fused multiply-adds on aarch64, as V8 does there, so some outputs differ between aarch64 and x86-64; the snapshots are taken on aarch64 ([0005](./0005-quality-gates.md)).
- An engine under a license other than MIT lives in a crate of its own ([0013](./0013-licenses-of-ported-code.md)).

## Related

[0013](./0013-licenses-of-ported-code.md), [0015](./0015-differential-testing.md), [LEARNINGS.md](../LEARNINGS.md).
