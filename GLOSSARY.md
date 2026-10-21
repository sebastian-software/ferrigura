# ferraid and ferrigura

ferraid is a fork of Mermaid trimmed to its core; ferrigura is its port to Rust, which replaces the TypeScript implementation diagram type by diagram type.

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
The TypeScript implementation in `packages/`, kept unchanged as the behaviour the port is checked against.
_Avoid_: upstream (that is mermaid-js/mermaid), legacy

**Canonical model**:
The versioned JSON form of a parsed diagram type that both the Rust model and the JS exporter produce, so the two can be compared.

**Differential run**:
Feeding the same inputs to the JS reference and to ferrigura and comparing the canonical models, ELK coordinates or rendered geometry.

**Parity**:
The state of a diagram type whose ported specs pass and whose differential runs show no difference beyond tolerance.

**Tier**:
The porting priority of a diagram type: P1 was ported first, then P2, then P3.

**Strand**:
One line of porting work that runs in its own session and worktree and reports in `docs-port/STATUS.md`.
