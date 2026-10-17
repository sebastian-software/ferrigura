# Notice: sources and licences of ferrigura-cose

The crate is MIT-licensed, as its sources are. It ports the tree path of three MIT packages, all
at the versions Mermaid's bundle loads:

| Package                  | Version | Copyright                                         | Ported in                                                                                                                                                                                             |
| ------------------------ | ------- | ------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `cytoscape-cose-bilkent` | 4.1.0   | Copyright (c) 2016-2018, The Cytoscape Consortium | `src/graph.rs`, `src/engine.rs` (`index.js`: the adapter and `getUserOptions`)                                                                                                                        |
| `cose-base`              | 1.0.3   | Copyright (c) 2019 - present, iVis@Bilkent        | `src/forest.rs`, `src/forces.rs`, `src/engine.rs` (`CoSELayout.js`, `CoSENode.js`)                                                                                                                    |
| `layout-base`            | 1.0.2   | Copyright (c) 2019 iVis@Bilkent                   | `src/geometry.rs`, `src/graph.rs`, `src/forces.rs`, `src/forest.rs` (`Layout.js`, `LGraph.js`, `LNode.js`, `LEdge.js`, `FDLayout.js`, `util/IGeometry.js`, `util/RectangleD.js`, `util/Transform.js`) |

The three packages are under the MIT licence, whose text and copyright notices are in `LICENSE`.

The trigonometric and logarithm functions come from `ferrigura-jsmath`, whose notice is in its own
`NOTICE.md`.
