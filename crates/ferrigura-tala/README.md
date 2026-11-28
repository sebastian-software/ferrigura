# ferrigura-tala

A native Rust port of a bounded subset of the TALA layout algorithm in D2.
The experimental Ferrigura integration currently covers flat, disconnected
rectangles, one labeled self-loop on an isolated rectangle, and up to 32
unlabeled self-loops on an isolated rectangle. The crate reports unsupported
topology as an error. The public Ferrigura renderer uses its existing engines
unless the caller asks for TALA.

The source pin, attribution, and file map are in [NOTICE.md](./NOTICE.md).
The license is [MPL-2.0](./LICENSE).
