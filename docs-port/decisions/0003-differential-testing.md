# Differential testing against the JS reference

Besides porting every spec file one to one, the port is checked against the running TypeScript implementation: per diagram type, a JS exporter turns the diagram DB into a canonical, versioned JSON model, which is exactly what the Rust model serializes to, and every input we have (e2e fixtures, spec inputs, documentation examples) is parsed by both and compared. The canonical model is ours, not a dump of the JS DB, so Rust never has to mirror JavaScript internals.

The same principle applies further down the pipeline: the ELK port and elkjs get the same ELK JSON graph and their coordinates are compared, and rendering is compared by geometry (node positions and sizes, edge points, label boxes, within a tolerance) against a JS reference configured like ferrigura, with `htmlLabels: false` and the bundled font loaded as a web font. Pixel comparison is not possible once fonts and label technique differ.

## Consequences

The JS reference, its build and the e2e harness stay working until a diagram type reaches parity; the JavaScript toolchain is removed only after the last type it serves.

## Addendum, 2026-10-21

Every diagram type has reached parity, and the JS reference stays: it is how every later change of the port is checked, by the same differential runs in CI. It is kept unchanged as Mermaid 12.2.0 in this repository's fork, with the part of the JavaScript toolchain that builds and runs it. Mermaid's own unit tests and its Playwright e2e suites no longer run and are removed; the `.mmd` fixtures stay as the inputs of the differential runs and the tests.
