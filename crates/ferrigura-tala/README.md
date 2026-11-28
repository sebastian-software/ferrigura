# ferrigura-tala

A native Rust port of a bounded subset of the TALA layout algorithm in D2.
The experimental Ferrigura integration covers:

- Up to twelve disconnected rectangles.
- An isolated rectangle with one labeled or up to 32 unlabeled self-loops.
- One directed connection between two congruent rectangles, subject to size
  and caption limits.
- Straight paths of three to five standard rectangles with a single target
  arrow on each connection and enough room for the measured edge captions.
- One chain of nested containers around a single rectangular leaf, supported
  unlabeled self-loops, or a supported unlabeled connection between two
  congruent rectangles in the innermost container.

The public Ferrigura renderer uses its existing engine without an explicit
TALA request. Branching, multi-node cycles, cross-container connections, and
additional combinations of shapes, measurements, and edges return an
unsupported-graph error. The Go reference and the stage replay instructions
are in the [development harness](../../tools/tala-pilot/README.md).

The source pin, attribution, and file map are in [NOTICE.md](./NOTICE.md).
The license is [MPL-2.0](./LICENSE).
