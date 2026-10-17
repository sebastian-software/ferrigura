# Port cose-bilkent and fcose, and Mermaid's tidy tree beside them

Mindmaps lay out with cose-bilkent by default, and architecture diagrams with fcose; both are force-directed layouts of the cytoscape ecosystem. ferrigura ports them from the JavaScript sources, as M4 ported ELK Layered, so that the same text gives the same diagram. They are two engines, not one: cose-bilkent 4.1.0 runs on `cose-base` 1.0.3 and `layout-base` 1.0.2, fcose 2.2.0 on `cose-base` 2.2.0 and `layout-base` 2.0.1, and their force phases differ.

Both are deterministic where Mermaid reaches them (probed, `cose/NOTES.md` and `architecture/NOTES.md`). A mindmap is always a tree, so cose-bilkent takes its tree path, which starts from a radial layout and draws no random numbers; its random path is unreachable from Mermaid and is not ported. fcose draws from a mulberry32 generator seeded for every run (`architectureSeed.ts`), not from page state. To give the same numbers, the ports compute `sin`, `cos`, `log` and `pow` as V8 does (V8's port of fdlibm), since the platform's functions differ from V8's in the last bit for a few percent of inputs.

Mermaid also ships a deterministic, bidirectional tidy tree for mindmaps (`@mermaid-js/layout-tidy-tree` 1.0.1, chosen with `layout: tidy-tree`, on `non-layered-tidy-tree-layout`). The fork removed the package (FORK.md, commit `2d85261d6`), so the fork's bundle draws `layout: tidy-tree` as cose-bilkent. ferrigura ports the package, so `layout: tidy-tree` gives Mermaid's tidy tree, and the geometry harness registers the package from the git history in the reference page. Which layout should be the mindmap default in the long run is decided once both can be compared.

Considered and not chosen: ELK Layered as the mindmap default (no new layout code, but not what Mermaid draws by default) and a layout of our own (deterministic, but with no reference to check against, and against the compatibility promise).

## Consequences

- The engines live in their own crate, `ferrigura-cose` (MIT, as their sources), with a differential test against the JavaScript engines on captured layout inputs, as `ferrigura-elk` has. cose-bilkent comes first; fcose follows with architecture.
- The tidy tree is a layout algorithm of the unified renderer, beside ELK and cose-bilkent.
- The V8 math functions are shared code; other renderers that compute angles (ishikawa) use them too.
