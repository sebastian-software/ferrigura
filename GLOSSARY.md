# ferrigura

ferrigura is the port of Mermaid 12.2.0 to Rust. Its reference is upstream Mermaid at a pinned commit. The repository began as a fork of Mermaid, trimmed to its core, whose working name was ferraid.

## Diagrams

**Diagram type**:
One kind of diagram with its own syntax, such as flowchart, sequence or gantt, identified by the keyword that starts the text.
_Avoid_: chart, diagram kind

**Detector**:
The check that decides from the start of the text which diagram type it is.

**Diagram DB**:
The in-memory model the TypeScript parser fills while parsing one diagram: nodes, edges, styles and settings.
_Avoid_: store, state

**Look**:
The stroke and shape style of a diagram (`classic`, `neo`, `handDrawn`), independent of its colours.

**Theme**:
A set of colour tokens that decides how a diagram is coloured.
_Avoid_: skin, palette (for the whole set)

**Fixture**:
A `.mmd` file under `e2e/diagrams/` with one diagram and its front matter, which the differential runs and the tests render.

## Port

**JS reference**:
Upstream Mermaid at the pin, with its patches: the behaviour the port is checked against.
_Avoid_: upstream (that is mermaid-js/mermaid as a whole), fork, legacy

**Pin**:
The upstream Mermaid commit the JS reference is built from. Moving the pin to a newer release is how the port follows Mermaid.

**Patch**:
The fix of one Mermaid bug, applied to the pinned commit before it is built, so that the JS reference does not have the bug.

**Canonical model**:
The versioned JSON form of a parsed diagram type that both the Rust model and the JS exporter produce, so the two can be compared.

**Differential run**:
Feeding the same inputs to the JS reference and to ferrigura and comparing the canonical models, ELK coordinates or rendered geometry.

**Exception**:
An input whose difference a differential run accepts: pending while ferrigura is behind Mermaid, with the issue that tracks the fix, or unsupported when ferrigura leaves the feature out on purpose.
_Avoid_: allowlist, known failure

**Snapshot**:
The hash of ferrigura's own output for one fixture, kept in the repository so that any change of the output shows.

**Parity**:
The state of a diagram type whose ported specs pass and whose differential runs show no difference beyond tolerance.

**Tier**:
The porting priority of a diagram type: P1 was ported first, then P2, then P3.

**Strand**:
One line of porting work that runs in its own session and worktree and reports in `docs-port/STATUS.md`.
