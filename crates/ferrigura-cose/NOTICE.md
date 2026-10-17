# Notice: sources and licences of ferrigura-cose

The crate is MIT-licensed, as its sources are. It ports two engines, both MIT packages at the
versions Mermaid's bundle loads. The cose-bilkent engine (`src/` outside `fcose/`) ports the tree
path of three packages:

| Package                  | Version | Copyright                                         | Ported in                                                                                                                                                                                             |
| ------------------------ | ------- | ------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `cytoscape-cose-bilkent` | 4.1.0   | Copyright (c) 2016-2018, The Cytoscape Consortium | `src/graph.rs`, `src/engine.rs` (`index.js`: the adapter and `getUserOptions`)                                                                                                                        |
| `cose-base`              | 1.0.3   | Copyright (c) 2019 - present, iVis@Bilkent        | `src/forest.rs`, `src/forces.rs`, `src/engine.rs` (`CoSELayout.js`, `CoSENode.js`)                                                                                                                    |
| `layout-base`            | 1.0.2   | Copyright (c) 2019 iVis@Bilkent                   | `src/geometry.rs`, `src/graph.rs`, `src/forces.rs`, `src/forest.rs` (`Layout.js`, `LGraph.js`, `LNode.js`, `LEdge.js`, `FDLayout.js`, `util/IGeometry.js`, `util/RectangleD.js`, `util/Transform.js`) |

The fcose engine (`src/fcose/`, the architecture diagram's layout) ports:

| Package           | Version | Copyright                                     | Ported in                                                                                                                                                                                                                                                                                                                                                                                                             |
| ----------------- | ------- | --------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `cytoscape-fcose` | 2.2.0   | Copyright (c) 2018 - present, iVis-at-Bilkent | `src/fcose/mod.rs` (`index.js` `run`, `cose.js` `coseLayout`, `auxiliary.js` `relocateComponent`, `processChildrenList`, `processEdges`, `calcBoundingBox`), `src/fcose/spectral.rs` (`spectral.js`, the spectral start of `randomize: true`)                                                                                                                                                                         |
| `cose-base`       | 2.2.0   | Copyright (c) 2019 - present, iVis@Bilkent    | `src/fcose/layout.rs` (`CoSELayout.js` `classicLayout`, `tick`, `moveNodes`, `initSpringEmbedder`), `src/fcose/forces.rs` (`FDLayout` forces as cose-base uses them), `src/fcose/model.rs` (`CoSENode.js`, `LGraph` and `LGraphManager` as cose-base uses them), `src/fcose/constraints.rs` (`ConstraintHandler.js`, `initConstraintVariables`, `updateDisplacements`, and the transform block for `randomize: true`) |
| `layout-base`     | 2.0.1   | Copyright (c) 2019 iVis@Bilkent               | `src/fcose/model.rs` (`LGraph.js`, `LGraphManager.js`, `LNode.js`, `LEdge.js`), `src/fcose/forces.rs` (`fd/FDLayout.js`), `src/fcose/linalg.rs` (`util/Matrix.js`, `util/SVD.js`, and the orthogonal Procrustes fit of `ConstraintHandler.js` 578-593), `src/geometry.rs` (`util/IGeometry.js`, `util/RectangleD.js`, `util/Transform.js`, `util/IMath.js`, the same functions as in 1.0.2)                           |

`src/fcose/rng.rs` reproduces the generator of Mermaid's own `architectureSeed.ts` (not a third-party
package).

The three cose-bilkent packages are under the MIT licence, and so are the fcose packages. The text
and the copyright notices of all six are in `LICENSE`.

The trigonometric and logarithm functions come from `ferrigura-jsmath`, whose notice is in its own
`NOTICE.md`.
