# 0002 Glossary

**Scope:** the terms that code, tests and documentation use for ferrigura's domain, and the words to avoid for them. It does not cover the names of Mermaid's syntax and configuration, which keep Mermaid's meaning, or general Rust and TypeScript vocabulary.

## Decision

These terms mean exactly what is written here, everywhere in the repository.

### Diagrams

**Diagram type**:
One kind of diagram with its own syntax, such as flowchart, sequence or gantt, identified by the keyword that starts the text. Mermaid 12.2.0 has 33 of them; ferrigura draws ten, each one completely, and refuses a text of any other ([0024](./0024-a-fixed-list-of-diagram-types.md)).
_Avoid_: chart, diagram kind, family

**Detector**:
The check that decides from the start of the text which diagram type it is.

**Diagram DB**:
The in-memory model that Mermaid's parser fills while parsing one diagram: nodes, edges, styles and settings.
_Avoid_: store, state

**Look**:
The stroke and shape style of a diagram (`classic`, `neo`, `handDrawn`), independent of its colors.

**Theme**:
A set of color tokens that decides how a diagram is colored.
_Avoid_: skin, palette (for the whole set)

**Unified renderer**:
The renderer that flowchart, class, state and ER share, and that the mindmap builds on: the port of Mermaid's `rendering-util`, with its shapes, clusters, edges and layout algorithms (`ferrigura-render`, `src/unified`).

**Default rendering**:
What a render without options draws: every diagram type in ferrigura's own look, with hairline outlines, color by a node's role, rounded corners and soft shadows, and with its own measures (roomier nodes, spacings and group paddings, and layouts of its own for sequence diagrams and mindmaps), laid out by the same engines as `compat` ([0023](./0023-the-default-rendering.md)). In code it is the tone a render carries (`design: Option<Tone>`, `design.rs`); `None` is `compat`.
_Avoid_: design mode, optimized or adaptive mode (there is no third rendering); theme or look (a theme sets colors, a look is Mermaid's `look` configuration)

**Compat**:
The per-render switch (`compat: true`, `--compat`) that draws the reference's geometry and look for every diagram type. The checks against the reference run on it. Font and readability corrections apply in both renderings ([0019](./0019-mermaid-geometry-with-compat.md)).
_Avoid_: compatibility mode, Mermaid mode

**Role**:
What a node or a part stands for in the default rendering, read from its shape and its diagram type: `terminal`, `decision` or `data` in a flowchart, for example. A role gives it its color ([0023](./0023-the-default-rendering.md)).

**Draw animation**:
A one-time reveal of a diagram's nodes, connections, bars, or sectors, following a drawing order.
It is separate from a moving-dash animation on an already visible edge ([0021](./0021-optional-svg-draw-animation.md)).

**Animation speed**:
A multiplier of the average velocity along a drawn path, bar, or arc. Each
element eases in and out independently. Longer elements take
proportionally longer; speed does not specify the duration of the whole diagram.

### Verification

**Reference**:
Upstream Mermaid at the pin, with its patches: the behavior ferrigura is checked against ([0014](./0014-upstream-reference-at-a-pin.md)).
_Avoid_: upstream (that is mermaid-js/mermaid as a whole), oracle, legacy

**Pin**:
The upstream Mermaid commit the reference is built from. Moving the pin to a newer release is how ferrigura follows Mermaid.

**Patch**:
The fix of one Mermaid bug, applied to the pinned commit before it is built, so that the reference does not have the bug.

**Canonical model**:
The versioned JSON form of a parsed diagram that both ferrigura and the reference's exporter produce, so that the two can be compared ([0008](./0008-canonical-model-and-typed-views.md), [canonical-model.md](../canonical-model.md)).

**Differential run**:
Feeding the same inputs to the reference and to ferrigura and comparing the canonical models, the layout data, the layout coordinates or the rendered geometry ([0015](./0015-differential-testing.md)).

**Fixture**:
A `.mmd` file under `e2e/diagrams/` with one diagram and its front matter, which the differential runs and the tests render.

**Probe**:
A `.mmd` file under `tools/port/geometry/probes/<type>/` that exercises what the fixtures miss; the geometry runs render it like a fixture.

**Corpus**:
The inputs a run reads: the fixtures and probes, or, for a layout crate, the captured inputs and outputs under its `tests/corpus`.

**Exception**:
An input whose difference a differential run accepts: pending while ferrigura is behind Mermaid, with the issue that tracks the fix, or unsupported when ferrigura leaves the feature out on purpose.
_Avoid_: allowlist, known failure

**Snapshot**:
The hash of ferrigura's own output for one fixture, kept in `crates/ferrigura/tests/snapshots/svg.tsv` so that any change of the output shows.

**Parity**:
The state of a diagram type whose ported specs pass and whose differential runs show no difference beyond tolerance.

## Why

The port runs between two codebases, Mermaid's and ferrigura's, and the same word can name different things on the two sides. Fixed terms keep comments, tests and reviews unambiguous.

## Consequences

- A new term that the code starts to use is added here in the same change.
- Code identifiers follow these terms where they name these concepts.

## Related

[0001](./0001-us-english.md).
