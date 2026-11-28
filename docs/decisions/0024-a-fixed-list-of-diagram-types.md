# 0024 A fixed list of diagram types

**Scope:** which of Mermaid's diagram types ferrigura draws, what it does with the text of any other type, and what follows for the checks against the reference. It does not cover how a type is parsed, laid out or drawn, which the type's page under [diagrams/](../diagrams/README.md) and the other records describe, nor which parts of a supported type's syntax ferrigura leaves out on purpose ([0014](./0014-upstream-reference-at-a-pin.md)).

## Decision

ferrigura draws ten diagram types, in three groups:

| Group  | Types                         | Drawn by                                                                                                                          |
| ------ | ----------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Graphs | flowchart, state, class, ER   | The unified renderer and ELK Layered, with the default rendering's measures and placement around ELK                              |
| Single | mindmap, sequence, gantt      | The mindmap on the unified shapes with the default's tree, cose-bilkent in `compat`; sequence and gantt by renderers of their own |
| Charts | XY chart, pie, quadrant chart | Renderers of their own                                                                                                            |

- **A type is supported completely or not at all.** A supported type draws what the reference draws, and is compared with it in `compat` ([0019](./0019-mermaid-geometry-with-compat.md)), and drawn in ferrigura's own look by default ([0023](./0023-the-default-rendering.md)): its specs are ported, its models, layout data and geometry are compared on every input, and its output is pinned by the snapshots ([0015](./0015-differential-testing.md)). A type that cannot be held to that is not supported.
- **The list is in one place.** `DiagramType` of `ferrigura-syntax` has one variant per supported type, and `DiagramType::ALL` lists them in the order of the groups. The facade registers one plugin for each ([0020](./0020-diagram-types-as-plugins.md)), the Node addon has one feature for each, and the error below lists them from `ALL`.
- **A text of another type gets an error that names it.** The headers of Mermaid's other diagram types are detected as the reference detects them (`ferrigura-syntax`, `unsupported.rs`), and such a text is refused with `Error::Unsupported`: "ferrigura does not draw sankey diagrams; it draws flowchart, state, class, ER, mindmap, sequence, gantt, XY chart, pie and quadrant chart diagrams". The CLI reports it and exits with 1, the Node addon throws it. A text whose header no detector takes is a parse error. The detection of those headers stays; nothing else of those types is in the code.
- **The reference is compared for these types only.** The pin moves with Mermaid ([0014](./0014-upstream-reference-at-a-pin.md)), and its parsers, renderers and specs are compared for the ten types. The parts of the reference that only other types use are not compared, and their patches go once nothing that is compared depends on them; the configuration run still compares the whole set of configuration keys and the defaults, and the theme run the whole map of theme variables.

## Why

ferrigura puts the quality of a drawing first. A type it draws is one it draws completely: the same geometry as the reference in `compat`, the same output on every machine, and a look of its own by default. Each type costs its parser, its renderer, often a layout engine, its corpus, its harness and its share of every change to shared code; a type that is drawn but not maintained to that standard lowers the promise for all of them.

A clear error is better than a confusing one: a text of a type ferrigura does not draw is valid Mermaid, so a parse error would blame the text. Naming the type and the list tells the user what ferrigura does instead.

Weighed and not chosen: keeping every type and marking some experimental (the promise would differ per type, and the shared code would still carry them), cargo features per type on the library (a larger build matrix for the same effect as the plugins of [0020](./0020-diagram-types-as-plugins.md)), and falling back to a parse error for the other types (it hides that the text is valid).

## Consequences

- Adding a type is a decision of its own: it comes with its parser, its renderer, its corpus, its specs and its runs against the reference, and it extends `DiagramType`, `ALL`, the plugins and the addon features; its header leaves the table of `unsupported.rs`.
- `DiagramType` is the public list of the facade ([0022](./0022-stable-public-api.md)); the error's wording may improve within a major version, its variant does not change.
- The corpus under `e2e/` holds fixtures of the ten types only, and the snapshot manifest one line per fixture.
- A shape of the unified renderer stays only while a supported type can reach it. A flowchart can name a shape whose name is lowercase and has no `_`, so `composite` stays and draws what a flowchart node gives it; the shapes that only the nodes of other types carried (`requirementBox`, `block_arrow`, the use case shapes, `kanbanItem`) are gone.

## Related

[0012](./0012-layout-engines-ported-exactly.md), [0014](./0014-upstream-reference-at-a-pin.md), [0015](./0015-differential-testing.md), [0020](./0020-diagram-types-as-plugins.md), [0022](./0022-stable-public-api.md), [0023](./0023-the-default-rendering.md), [diagrams/](../diagrams/README.md).
