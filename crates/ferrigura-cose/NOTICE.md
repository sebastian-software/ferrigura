# Notice: sources and licenses of ferrigura-cose

The crate is MIT-licensed, as its sources are. It ports the cose-bilkent engine, from MIT packages
at the versions Mermaid's bundle loads: the tree path of three packages.

| Package                  | Version | Copyright                                         | Ported in                                                                                                                                                                                             |
| ------------------------ | ------- | ------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `cytoscape-cose-bilkent` | 4.1.0   | Copyright (c) 2016-2018, The Cytoscape Consortium | `src/graph.rs`, `src/engine.rs` (`index.js`: the adapter and `getUserOptions`)                                                                                                                        |
| `cose-base`              | 1.0.3   | Copyright (c) 2019 - present, iVis@Bilkent        | `src/forest.rs`, `src/forces.rs`, `src/engine.rs` (`CoSELayout.js`, `CoSENode.js`)                                                                                                                    |
| `layout-base`            | 1.0.2   | Copyright (c) 2019 iVis@Bilkent                   | `src/geometry.rs`, `src/graph.rs`, `src/forces.rs`, `src/forest.rs` (`Layout.js`, `LGraph.js`, `LNode.js`, `LEdge.js`, `FDLayout.js`, `util/IGeometry.js`, `util/RectangleD.js`, `util/Transform.js`) |

The three packages are under the MIT license. The text and their copyright notices are in
`LICENSE`.

The trigonometric and logarithm functions come from `ferrigura-jsmath`, whose notice is in its own
`NOTICE.md`.
