# ferrigura-tala

A native Rust port of a bounded subset of the TALA layout algorithm in D2.
The experimental Ferrigura integration covers:

- Up to twelve disconnected rectangles.
- An isolated rectangle with one labeled or up to 32 unlabeled self-loops.
- One directed connection between two congruent rectangles, subject to size
  and caption limits.
- Two to four target-arrow connections with the same source and target between
  congruent rectangles with integral dimensions. This requires an explicit
  flow direction. Horizontal connections can carry captions that share the
  same measured height and fit between the edge routes.
- Straight paths of three to five standard rectangles with a single target
  arrow on each connection and enough room for the measured edge captions.
- One chain of nested containers around a single rectangular leaf, supported
  unlabeled self-loops, or a supported unlabeled connection between two
  congruent rectangles in the innermost container.
- One unlabeled one-way edge between an outer rectangle and a container or
  its only child. This case requires standard-size rectangles and an
  explicit flowchart direction.
- Two disconnected unary container chains, optionally inside one common
  outer container, with one rectangular leaf per chain.

The public Ferrigura renderer uses its existing engine without an explicit
TALA request. Branching, multi-node cycles, more complex cross-container connections, and
additional combinations of shapes, measurements, and edges return an
unsupported-graph error. The Go reference and the stage replay instructions
are in the [development harness](../../tools/tala-pilot/README.md).

The source pin, attribution, and file map are in [NOTICE.md](./NOTICE.md).
The TALA port uses [MPL-2.0](./LICENSE). Its Go-compatible random source
uses [BSD-3-Clause](./LICENSE-GO).
