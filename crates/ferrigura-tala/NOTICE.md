# Third-party notice

This crate contains a Rust translation of portions of D2's TALA layout
engine, copyright 2022 Terrastruct Inc., under the Mozilla Public License
2.0. The upstream source is D2 v0.10.0 at
`b463ea5d47272f4da6e1e08012dbb15a6a4dbcde`.
[Source](https://github.com/d2lang/d2/tree/b463ea5d47272f4da6e1e08012dbb15a6a4dbcde/d2layouts/d2talalayout) ·
[Upstream license](https://github.com/d2lang/d2/blob/b463ea5d47272f4da6e1e08012dbb15a6a4dbcde/LICENSE.txt).

| Rust file          | D2 source                                                                                                                                                                                                                         |
| ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/normalize.rs` | `d2layouts/d2talalayout/internal/placement/stages.go`                                                                                                                                                                             |
| `src/placement.rs` | `d2layouts/d2talalayout/internal/packing/{combine,binpack,bounds,score}.go`, `internal/layoutgraph/node.go`, `internal/placement/{node_placement,stages,tuning}.go`                                                               |
| `src/routing.rs`   | `d2layouts/d2talalayout/internal/loops/loops.go`, `internal/nodeshape/snappoints.go`, `internal/layoutgraph/node.go`, `internal/placement/stages.go`, `d2layouts/d2talalayout/patch.go`, `lib/label/label.go`, `lib/geo/route.go` |

The crate archive contains the covered Rust source. Distributors of native
binaries that enable TALA must tell recipients how to obtain that source at
the exact release revision, including modifications, as required by
[MPL-2.0 §3.2](https://www.mozilla.org/en-US/MPL/2.0/).
