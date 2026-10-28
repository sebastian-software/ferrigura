# 0015 Differential testing

**Scope:** how ferrigura is checked against the reference ([0014](./0014-upstream-reference-at-a-pin.md)), from the parse to the rendered geometry, and how its own output is pinned. It does not cover the ordinary unit tests of each crate.

## Decision

ferrigura is checked by running the same inputs through the reference and through ferrigura and comparing the results at every stage. The harness in `tools/port` runs the reference in Node.js and Chromium.

1. **Specs, ported one to one.** Every spec file of Mermaid for a diagram type gets a Rust counterpart with the same test names.
2. **Differential parse.** An exporter per diagram type turns the reference's diagram DB into the canonical model ([0008](./0008-canonical-model-and-typed-views.md)), and the two models are compared on every input: the fixtures, the probes, the string literals of Mermaid's specs, the diagrams of its documentation and the cases under `tools/port` (`port:diff <type>`). The configuration, the theme variables, the YAML front matter and the Markdown reader have runs of their own.
3. **Differential layout data.** For the unified renderer, the reference's `getData()` and ferrigura's are compared field by field (`port:diff <type>-data`).
4. **Differential layout.** Each layout engine is compared with the original on captured inputs, with the engine's own corpus in its crate ([0012](./0012-layout-engines-ported-exactly.md)).
5. **Geometry.** Both render every fixture and probe in Chromium, and node positions and sizes, edge points and label boxes are compared, paired by their structural keys, within 0.05 px (`port:geometry <type>`). The reference runs with `htmlLabels: false` and loads ferrigura's fonts as web fonts, so that both draw the same text.
6. **Snapshots.** ferrigura's own output for the corpus is pinned by one hash per file in `crates/ferrigura/tests/snapshots/svg.tsv`, so that any change of the output shows, also where the comparison does not look.

## Why

Pixel comparison is impossible once fonts and label technique differ, and a port that is only reviewed drifts. Comparing at every stage finds a difference where it starts: a wrong number in the parse shows as a model difference, not as a shifted node three stages later. Existing specs and inputs measure progress instead of estimating it.

## Consequences

- The harness is the repository's only JavaScript project (`tools/port`), with its own lint, format and tests ([0005](./0005-quality-gates.md)).
- A geometry run fails on a fixture outside the tolerance and on one ferrigura does not render. Fixtures that use what ferrigura leaves out (`look: handDrawn`, `layout: dagre`, scripts the bundled fonts lack) are left out by rules of the harness, each with its reason.
- The geometry runs on macOS: Chromium there keeps fractional glyph advances, as ferrigura's measurement does.

## Related

[0005](./0005-quality-gates.md), [0008](./0008-canonical-model-and-typed-views.md), [0012](./0012-layout-engines-ported-exactly.md), [0014](./0014-upstream-reference-at-a-pin.md), [canonical-model.md](../canonical-model.md).
