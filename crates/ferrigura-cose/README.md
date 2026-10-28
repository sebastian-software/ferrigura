# ferrigura-cose

`ferrigura-cose` holds two force-directed layouts ported from their JavaScript sources: cytoscape-cose-bilkent 4.1.0, on the tree path that Mermaid's mindmaps use, and cytoscape-fcose 2.2.0, which architecture diagrams use. Each layout is checked against the JavaScript engine on captured layout inputs.

This crate is a part of [ferrigura](https://github.com/sebastian-software/ferrigura). Its API may change between versions. To render diagrams, use the `ferrigura` crate.

## What is ported

The two engines come from different library versions that do not share a model, so each has its own graph, forces and constraint code:

| Engine                       | Packages (all MIT)                                                          | Used by                                        | Module                                                           |
| ---------------------------- | --------------------------------------------------------------------------- | ---------------------------------------------- | ---------------------------------------------------------------- |
| cose-bilkent, tree path only | `cytoscape-cose-bilkent` 4.1.0 on `cose-base` 1.0.3 and `layout-base` 1.0.2 | Mindmaps (`layout: cose-bilkent`, the default) | the crate root (`layout`: `engine`, `forest`, `forces`, `graph`) |
| fcose                        | `cytoscape-fcose` 2.2.0 on `cose-base` 2.2.0 and `layout-base` 2.0.1        | Architecture diagrams                          | `fcose`                                                          |

`NOTICE.md` lists which source file each module ports. The two versions of `layout-base` and `cose-base` are separate copies in Mermaid's bundle, each with its own state (cose-bilkent loads 1.0.3 and 1.0.2, not the 2.x versions that fcose uses); only `geometry.rs` is shared, for the functions the two versions have in common. Mermaid reaches the engines like this:

- **cose-bilkent** only through a mindmap. A mindmap is one tree, so Mermaid gives the engine a forest with one tree, which cose-base lays out by its radial path: the center of the tree is found by peeling leaves, the nodes are placed on rings around it, and the spring embedder starts from there. That path draws no random numbers, so the output depends on the input alone. The random path (`positionNodesRandomly`, tree reduction and the growth of pruned nodes, the draws of `RandomSeed` and `Math.random`) is not reachable from Mermaid and is not ported: a graph with a cycle is `Error::NotAForest`, and two or more nodes without an edge, which cose-base would group under a dummy compound, are `Error::TilingNotPorted`.
- **fcose** through `layoutArchitecture`, which runs the layout twice per render: the second run starts from the first run's output and the bounds after the segment weights the renderer computes. Mermaid always passes an alignment object, so constraints are on, tiling and tree reduction are off, the constraint handler runs, and the spring embedder runs in its incremental branch. `randomize: true` runs the spectral start before the layout. The random draws come from the mulberry32 generator of Mermaid's own `architectureSeed.ts`, seeded per run from the seed in the configuration (seeds from 1; seed 0 keeps the page's `Math.random` and cannot be reproduced). Not ported: tiling, tree reduction, hidden children, the packing of components, `draft` quality and the fixed-node constraint, none of which Mermaid reaches.

## How the port follows the sources

The results are compared with a tolerance of zero, so the port keeps what the arithmetic depends on:

- **Operation order.** Every expression keeps the operand order and the tree of the source (`WORLD_CENTER_X + (r * (maxX - minX)) + minX`, `(repulsionForce * distanceX) / distance`, `offset + lenRatio * disp`), because floating-point results depend on it.
- **Math functions.** `sin` and `cos` (the radial placement), `log` and `pow` (the cooling factor) come from `ferrigura-jsmath`, which gives Chromium's bits; the platform's functions differ in the last bit for a few percent of inputs, and the radial start would then differ from the first node on.
- **Iteration order.** `Set` and `Map` iterate in insertion order, and the order of the forces is the order of `getAllEdges` and `getAllNodes`, so floating-point sums come out in that order. JavaScript's plain objects have quirks that show: fcose's `relocateComponent` walks `Object.keys(idToLNode)`, and a node whose id is `__proto__` sets the object's prototype instead of adding a key, so the walk misses that node and does not move it.
- **Quirks of the sources are kept.** `findCenterOfTree` splices its list while it iterates it, which gives the wrong center for many trees (a path of five nodes has its center at the second node, not the third); `branchRadialLayout` divides `(end - start + 1) / 2`; the bounds start at ±2,147,483,647; `shiftToLastRow` reads `node.width`, which is undefined for layout nodes, so that branch never runs.
- **Edge endpoints are not in the crate.** cytoscape writes the endpoints of each edge (`rscratch`) for its default 30 by 30 node style, not for Mermaid's box sizes, so every end of a mindmap edge lies on a circle of radius 15 around its node's center. The mindmap renderer produces them (`ferrigura-render`, `mindmap/layout.rs`); the cose-bilkent corpus records them so the renderer can check its own port.
- **Input.** Initial positions are not read: Mermaid passes zero for every node, and the radial start sets every center. Self loops and parallel edges are dropped, as cytoscape-cose-bilkent's adapter drops them.

## The corpora and the checks

The cases were captured from Mermaid 12.2.0 in Chromium: the layout's input as the Mermaid integration handed it to the engine (the node sizes it measured, the edges in order) and the positions the engine wrote. Each case is one JSON file and `manifest.json` lists them.

| Corpus               | Cases | Holds                                                                                                                                                                                                                         |
| -------------------- | ----: | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `tests/corpus`       |   328 | 28 mindmap fixtures of `e2e/diagrams/mindmap` and 300 generated mindmaps (shapes: random, path, star, broom, balanced; every node shape); `input` has `nodes` and `edges`, `output` has `positions` and `edgePoints`          |
| `tests/fcose-corpus` |   419 | 20 architecture fixtures, 10 diagrams from Mermaid's `svgDraw.spec.ts` and 389 generated architectures; each case has its two `runs`, with the options, the seed, the constraints, the bounds and the positions the run wrote |

The generated cases are texts that the generators of `tools/port/crates/cose` produce from seed 20261017 (`generate-trees.ts`: 300 mindmaps; `generate-architectures.ts`: 400 architectures with groups nested to three levels, services with and without icons, junctions, edges of every side pair and arrow form, `{group}` modifiers and alignment hints, and seeds 1 to 9; the diagrams that Mermaid does not render are skipped, which leaves 389). The same seed and count give the same texts.

- `tests/corpus.rs` compares the node centers of every mindmap case with the engine's; the edge endpoints are not compared (they belong to the renderer).
- `tests/fcose_corpus.rs` compares the positions of the leaves of every run (fcose writes those; cytoscape places a compound from its children) and the order of the nodes.
- `tests/api.rs` covers the crate's interface and its errors.
- Both corpus tests read JSON numbers exactly (`serde_json` with `float_roundtrip`), because the sizes are `getBBox` values with many digits, and compare with `==`, since `JSON.stringify` writes `-0` as `0`. The case floors only go up (328 and 419).

The corpus is fixed test data: the files are what the tests read, and the repository's formatter leaves them alone. A case file is the input and the output of one call of the engine, as Mermaid makes the call, so a new case is added by rendering its text in the reference's page and recording the layout call. The repository holds the generators of the texts, but not a script that records the calls.

The fcose corpus was captured on a page that measured labels in the system's Helvetica Neue, so 19 of its first-run bounds differ by half a pixel from what the harness's mapped bundled face gives. The engine tests replay the captured input and are unaffected; a recapture through the harness page would align them.

## License

MIT, as the workspace and as the ported packages are. `NOTICE.md` lists the packages, their versions and their copyright lines.
