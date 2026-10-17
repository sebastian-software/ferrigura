# P2: the second tier of diagram types

P2 ports mindmap, block, packet, xychart, sankey, quadrantChart, requirement, C4, kanban, architecture, treemap, ishikawa and eventmodeling: for each, the parser and its DB, the canonical model, the specs and the renderer. The research per type is in `<type>/NOTES.md`, the parser decisions and the quirks each port reproduces in `<type>/PLAN.md`. The layout engines that P2 needs are in [decision 0008](./decisions/0008-force-directed-layouts.md).

**Done when** each type passes its ported specs, `pnpm port:diff <type>` shows no difference on every collected input, and `pnpm port:geometry <type>` is within tolerance on every comparable fixture. Fixtures are excluded only with a named reason, as in M5 and M6.

## Decisions

The decisions of [M6-PLAN.md](./M6-PLAN.md) carry over unchanged: only the two redux themes ship, quirks are reproduced and listed, text is measured where and how the reference measures it with Chromium's rules, links stay as `<a>` and callbacks go, and text the bundled faces do not cover is excluded until fallback fonts exist. Added for P2:

| Question            | Decision                                                                                                                                                                                                                                                                        | Why                                                                                                      |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Mindmap layout      | cose-bilkent by default, ported with `layout-base` and `cose-base`; Mermaid's tidy tree when the text sets `layout: tidy-tree`; ELK when it sets `layout: elk`, as the reference does                                                                                           | Decided by Sebastian on 2026-10-17, decision 0008                                                        |
| Architecture layout | fcose, ported after cose-bilkent, on the same `cose-base`                                                                                                                                                                                                                       | Decision 0008                                                                                            |
| Random draws        | The force-directed layouts draw from the reference's generators (`layout-base`'s seeded generator, `Math.random`) with a fixed start for every render. The harness renders these types on a fresh page with the same seed, so the reference starts from the same state          | The reference's generator state lives for the page; a fresh page is what a single diagram on a page sees |
| Layout engines      | `ferrigura-cose` is a crate of its own (MIT), with a differential test of captured layout inputs against the JavaScript engines; the tidy tree is a layout algorithm of the unified renderer                                                                                    | As `ferrigura-elk`: the engine is tested on its own, before the diagrams that use it                     |
| Registries          | One scaffold adds every P2 type to the shared registries first (detection, the `Diagram` and `DiagramType` enums, the facade, the `canonical` example, the `tools/port` lists), with stubs that report the type as not ported; each type's tasks then fill only their own files | M6's parallel parser branches collided in every registry file                                            |

## Stages

1. **Research**, one task per type and one per layout engine (cose-bilkent with `layout-base` and `cose-base`, the tidy tree; fcose with architecture): `<type>/NOTES.md`, every rule with the reference's file and line, in the sections of the M6 notes.
2. **Registry scaffold**, one task, merged before the parsers.
3. **Parsers**, one task per type: model, parser and DB, specs, differential.
4. **Layout engines**: `ferrigura-cose` (cose-bilkent first, then fcose) and the tidy tree, each checked against the JavaScript engine on captured inputs.
5. **Renderers**: requirement and mindmap on the unified renderer, the others as custom renderers on `ferrigura-render/src/common/`, each checked with `pnpm port:geometry <type>`. The working rules are those of [M6-RENDERERS.md](./M6-RENDERERS.md).
