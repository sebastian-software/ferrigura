# ferrigura-cose

`ferrigura-cose` holds the force-directed layout of Mermaid's mindmaps, ported from its JavaScript sources: cytoscape-cose-bilkent 4.1.0, on the tree path that the mindmaps use. It is checked against the JavaScript engine on captured layout inputs.

This [Ferrigura](https://github.com/sebastian-software/ferrigura) implementation crate exposes an experimental API. The [`ferrigura` facade](https://crates.io/crates/ferrigura) provides the [supported Rust API](https://github.com/sebastian-software/ferrigura/blob/main/docs/decisions/0022-stable-public-api.md).

## What is ported

| Engine                       | Packages (all MIT)                                                          | Used by                                        | Module                                                           |
| ---------------------------- | --------------------------------------------------------------------------- | ---------------------------------------------- | ---------------------------------------------------------------- |
| cose-bilkent, tree path only | `cytoscape-cose-bilkent` 4.1.0 on `cose-base` 1.0.3 and `layout-base` 1.0.2 | Mindmaps (`layout: cose-bilkent`, the default) | the crate root (`layout`: `engine`, `forest`, `forces`, `graph`) |

`NOTICE.md` lists which source file each module ports. Mermaid reaches the engine only through a mindmap. A mindmap is one tree, so Mermaid gives the engine a forest with one tree, which cose-base lays out by its radial path: the center of the tree is found by peeling leaves, the nodes are placed on rings around it, and the spring embedder starts from there. That path draws no random numbers, so the output depends on the input alone. The random path (`positionNodesRandomly`, tree reduction and the growth of pruned nodes, the draws of `RandomSeed` and `Math.random`) is not reachable from Mermaid and is not ported: a graph with a cycle is `Error::NotAForest`, and two or more nodes without an edge, which cose-base would group under a dummy compound, are `Error::TilingNotPorted`.

## How the port follows the sources

The results are compared with a tolerance of zero, so the port keeps what the arithmetic depends on:

- **Operation order.** Every expression keeps the operand order and the tree of the source (`WORLD_CENTER_X + (r * (maxX - minX)) + minX`, `(repulsionForce * distanceX) / distance`, `offset + lenRatio * disp`), because floating-point results depend on it.
- **Math functions.** `sin` and `cos` (the radial placement), `log` and `pow` (the cooling factor) come from `ferrigura-jsmath`, which gives Chromium's bits; the platform's functions differ in the last bit for a few percent of inputs, and the radial start would then differ from the first node on.
- **Iteration order.** `Set` and `Map` iterate in insertion order, and the order of the forces is the order of `getAllEdges` and `getAllNodes`, so floating-point sums come out in that order.
- **Quirks of the sources are kept.** `findCenterOfTree` splices its list while it iterates it, which gives the wrong center for many trees (a path of five nodes has its center at the second node, not the third); `branchRadialLayout` divides `(end - start + 1) / 2`; the bounds start at ±2,147,483,647; `shiftToLastRow` reads `node.width`, which is undefined for layout nodes, so that branch never runs.
- **Edge endpoints are not in the crate.** cytoscape writes the endpoints of each edge (`rscratch`) for its default 30 by 30 node style, not for Mermaid's box sizes, so every end of a mindmap edge lies on a circle of radius 15 around its node's center. The mindmap renderer produces them (`ferrigura-render`, `mindmap/layout.rs`); the cose-bilkent corpus records them so the renderer can check its own port.
- **Input.** Initial positions are not read: Mermaid passes zero for every node, and the radial start sets every center. Self loops and parallel edges are dropped, as cytoscape-cose-bilkent's adapter drops them.

## The corpus and the checks

The cases were captured from Mermaid 12.2.0 in Chromium: the layout's input as the Mermaid integration handed it to the engine (the node sizes it measured, the edges in order) and the positions the engine wrote. Each case is one JSON file and `manifest.json` lists them.

| Corpus         | Cases | Holds                                                                                                                                                                                                                |
| -------------- | ----: | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `tests/corpus` |   328 | 28 mindmap fixtures of `e2e/diagrams/mindmap` and 300 generated mindmaps (shapes: random, path, star, broom, balanced; every node shape); `input` has `nodes` and `edges`, `output` has `positions` and `edgePoints` |

The generated cases are texts that `tools/port/crates/cose/generate-trees.ts` produces from seed 20261017: 300 mindmaps. The same seed and count give the same texts.

- `tests/corpus.rs` compares the node centers of every case with the engine's; the edge endpoints are not compared (they belong to the renderer).
- `tests/api.rs` covers the crate's interface and its errors.
- The corpus test reads JSON numbers exactly (`serde_json` with `float_roundtrip`), because the sizes are `getBBox` values with many digits, and compares with `==`, since `JSON.stringify` writes `-0` as `0`. The case floor only goes up (328).

The corpus is fixed test data: the files are what the tests read, and the repository's formatter leaves them alone. A case file is the input and the output of one call of the engine, as Mermaid makes the call, so a new case is added by rendering its text in the reference's page and recording the layout call. The repository holds the generator of the texts, but not a script that records the calls.

## License

MIT, as the workspace and as the ported packages are. `NOTICE.md` lists the packages, their versions and their copyright lines.
