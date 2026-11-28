# Third-party notice

This crate contains a Rust translation of portions of D2's TALA layout
engine, copyright 2022 Terrastruct Inc., under the Mozilla Public License
2.0. The upstream source is D2 v0.10.0 at
`b463ea5d47272f4da6e1e08012dbb15a6a4dbcde`.
[Source](https://github.com/d2lang/d2/tree/b463ea5d47272f4da6e1e08012dbb15a6a4dbcde/d2layouts/d2talalayout) ·
[Upstream license](https://github.com/d2lang/d2/blob/b463ea5d47272f4da6e1e08012dbb15a6a4dbcde/LICENSE.txt).

| Rust file                   | D2 source                                                                                                                                                                                                                                                                                              |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `src/balance.rs`            | `d2layouts/d2talalayout/internal/routing/{postprocess,balance_route_guard,tuning}.go`, `lib/geo/segment.go`                                                                                                                                                                                            |
| `src/connected.rs`          | `d2layouts/d2talalayout/internal/layoutgraph/{graph,geometry_policy}.go`, `internal/placement/{compaction,gapreduction,stages}.go`, `internal/placementcost/geometry.go`, `internal/routing/postprocess.go`, `internal/labeling/placement.go`, `d2layouts/d2talalayout/patch.go`, `lib/label/label.go` |
| `src/connected/parallel.rs` | `d2layouts/d2talalayout/internal/placement/stages.go`, `internal/routing/{coordinator,postprocess}.go`, `lib/label/label.go`                                                                                                                                                                           |
| `src/hierarchy.rs`          | `d2layouts/d2talalayout/internal/layoutgraph/{node,graph}.go`, `internal/packing/guard.go`                                                                                                                                                                                                             |
| `src/hierarchy/boundary.rs` | `d2layouts/d2talalayout/internal/layoutgraph/{hierarchy_access,graph,node}.go`, `internal/placement/{node_placement,compaction,gapreduction}.go`, `internal/routing/postprocess.go`                                                                                                                    |
| `src/hierarchy/siblings.rs` | `d2layouts/d2talalayout/internal/placement/node_placement.go`, `internal/packing/{combine,binpack,guard}.go`, `internal/layoutgraph/{node,graph}.go`                                                                                                                                                   |
| `src/initialize.rs`         | `d2layouts/d2talalayout/internal/placement/{initialize,metrics,sizeless_optimizer}.go`, `internal/layoutgraph/{node,graph}.go`, `internal/placementcost/{edge_length,geometry}.go`, `lib/geo/math.go`                                                                                                  |
| `src/normalize.rs`          | `d2layouts/d2talalayout/internal/placement/stages.go`                                                                                                                                                                                                                                                  |
| `src/ovg_seed.rs`           | `d2layouts/d2talalayout/internal/routing/{ovg,ovg_fastpath,ovg_node,ovg_resource}.go`, `internal/nodeshape/{policy,snappoints}.go`, `internal/layoutgraph/{geometry_policy,node}.go`                                                                                                                   |
| `src/placement.rs`          | `d2layouts/d2talalayout/internal/packing/{combine,binpack,bounds,score}.go`, `internal/layoutgraph/node.go`, `internal/placement/{node_placement,stages,tuning}.go`                                                                                                                                    |
| `src/routing.rs`            | `d2layouts/d2talalayout/internal/loops/loops.go`, `internal/nodeshape/snappoints.go`, `internal/layoutgraph/node.go`, `internal/placement/stages.go`, `d2layouts/d2talalayout/patch.go`, `lib/label/label.go`, `lib/geo/route.go`                                                                      |
| `src/simplify.rs`           | `d2layouts/d2talalayout/internal/routing/{edge_simplify,geometry,edge_simplify_safety_test}.go`, `internal/layoutgraph/node.go`                                                                                                                                                                        |
| `src/stress_initialize.rs`  | `d2layouts/d2talalayout/internal/placement/stress_initialize.go`                                                                                                                                                                                                                                       |

The crate archive contains the covered Rust source. Distributors of native
binaries that enable TALA must tell recipients how to obtain that source at
the exact release revision, including modifications, as required by
[MPL-2.0 §3.2](https://www.mozilla.org/en-US/MPL/2.0/).
