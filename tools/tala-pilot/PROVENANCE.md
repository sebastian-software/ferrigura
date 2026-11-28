# TALA pilot provenance

The reference is D2 v0.10.0 at commit
`b463ea5d47272f4da6e1e08012dbb15a6a4dbcde`. That revision identifies the
layout implementation, copyright notice, and license used for the port.
[Pinned release](https://github.com/d2lang/d2/releases/tag/v0.10.0) ·
[Pinned code](https://github.com/d2lang/d2/tree/b463ea5d47272f4da6e1e08012dbb15a6a4dbcde/d2layouts/d2talalayout) ·
[Pinned license](https://github.com/d2lang/d2/blob/b463ea5d47272f4da6e1e08012dbb15a6a4dbcde/LICENSE.txt)

## Source map

- `d2layouts/d2talalayout/internal/placement/stages.go` defines `Normalize`.
  Its unpinned branch is translated in `crates/ferrigura-tala/src/normalize.rs` under MPL-2.0.
  Node top-left coordinates participate directly in the minimum. Route
  points and edge-label top-left coordinates are floored. The stage
  subtracts the minimum from node positions and route points.
- `d2layouts/d2talalayout/internal/engine/pipeline.go` calls `Normalize`
  after edge routing and label placement.
- `d2layouts/d2talalayout/internal/layoutgraph/edge.go` and
  `lib/label/label.go` calculate and round the label top-left coordinate
  used for the minimum. Go can recalculate the rounded label coordinate
  after translating the route, so the input label coordinate is not a
  parity target for this Rust stage.
- `d2layouts/d2talalayout/layout.go` runs the full Go layout used as the
  product reference. It can refine the selected seed after the last
  stage of the seed engine.
- `d2layouts/d2talalayout/adapter.go`, `patch.go`, and `result.go`
  implement D2's graph translation, result application, and seed scoring.
  Ferrigura's measured-graph and SVG adapters are project code under MIT.

The Rust slice does not use the fixed-position branch of `Normalize` or the
separate `NormalizeGaps` stage. The Go reference owns the upstream graph,
shape and label policies, grouping, placement, packing, routing, and
scoring. Ports of those stages need their own source mapping and tests.
[Upstream architecture](https://github.com/d2lang/d2/blob/b463ea5d47272f4da6e1e08012dbb15a6a4dbcde/d2layouts/d2talalayout/ARCHITECTURE.md)

## Reference dependencies

The Go TALA adapter imports D2 graph, target, and sequence-layout code.
The engine packages also use D2's shared `lib/geo`, `lib/label`,
`lib/shape`, and `lib/xsort` packages. The public adapter pulls in D2
parser, target, and text measurement dependencies transitively. The
pinned Go module and its own third-party notices identify those
dependencies. The Rust slice consumes measured geometry and does not
import these Go packages or copy their dependencies.
[Pinned Go module](https://github.com/d2lang/d2/blob/b463ea5d47272f4da6e1e08012dbb15a6a4dbcde/go.mod) ·
[Pinned third-party notices](https://github.com/d2lang/d2/blob/b463ea5d47272f4da6e1e08012dbb15a6a4dbcde/THIRD_PARTY_NOTICES.txt)

The comparison uses original Mermaid inputs and Ferrigura-generated SVGs.
Upstream D2 example diagrams, icons, fonts, and test fixtures have separate
reuse questions and stay outside this pilot.

## Distribution boundary

The translated Rust file carries the MPL source notice. The accompanying
license and attribution preserve the notice from the pinned D2 source.
MPL-2.0 sections 3.1–3.4 cover distribution of the covered source and
executable forms, larger works, and preservation of notices. A native
artifact containing the port includes the MPL notice and points to the
exact Ferrigura source for that artifact, including the covered modifications.
The D2 pin alone identifies upstream provenance.
[Rust notice](../../crates/ferrigura-tala/NOTICE.md) · [MPL-2.0](https://www.mozilla.org/en-US/MPL/2.0/)
