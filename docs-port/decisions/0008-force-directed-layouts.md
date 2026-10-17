# Port cose-bilkent and fcose, and Mermaid's tidy tree beside them

Mindmaps lay out with cose-bilkent by default, and architecture diagrams with fcose; both are force-directed layouts of the cytoscape ecosystem (`cytoscape-cose-bilkent` on `cose-base` and `layout-base`, `cytoscape-fcose` on `cose-base`). ferrigura ports them from the JavaScript sources, as M4 ported ELK Layered, so that the same text gives the same diagram. The reference is not fully deterministic: cose-bilkent starts from random positions (`randomize: true`), `layout-base` draws from a seeded generator whose state lives for the whole page, and `cose-base` calls `Math.random`. ferrigura draws from the same generators with a fixed start for every render, which is what the reference gives for a diagram alone on a fresh page; the geometry harness renders these types on a fresh page with the same seed.

Mermaid also ships a deterministic, bidirectional tidy tree for mindmaps (`@mermaid-js/layout-tidy-tree`, chosen with `layout: tidy-tree`, on `non-layered-tidy-tree-layout`). The fork removed the package (FORK.md, commit `2d85261d6`); ferrigura ports it as well, so `layout: tidy-tree` gives Mermaid's tidy tree. Which layout should be the mindmap default in the long run is decided once both can be compared.

Considered and not chosen: ELK Layered as the mindmap default (no new layout code, but not what Mermaid draws by default) and a layout of our own (deterministic, but with no reference to check against, and against the compatibility promise).

## Consequences

- The layout engines live in their own crate, `ferrigura-cose` (MIT, as their sources), with a differential test against the JavaScript engines on captured layout inputs, as `ferrigura-elk` has.
- The tidy tree is a layout algorithm of the unified renderer, beside ELK and cose-bilkent.
- A render of these types depends on the order of the generator's draws; that order is part of what the port reproduces.
