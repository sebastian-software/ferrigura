# P3: the remaining diagram types

P3 ports the nine diagram families that Mermaid 12.2.0 has and ferrigura does not yet: agentflow, cynefin, radar, railroad, swimlanes, tree view, use case, venn and wardley. With them, ferrigura covers all 33 families. For each: the parser and its DB, the canonical model, the specs and the renderer. The research per type is in `<type>/NOTES.md`, the parser decisions and the quirks each port reproduces in `<type>/PLAN.md`.

**Done when** each type passes its ported specs, `pnpm port:diff <type>` shows no difference on every collected input, and `pnpm port:geometry <type>` is within tolerance on every comparable fixture. Fixtures are excluded only with a named reason, as in M5, M6 and P2.

| Type      | Keywords                                                                         | Grammar (reference)                             | Source lines | Spec files | Fixtures |
| --------- | -------------------------------------------------------------------------------- | ----------------------------------------------- | -----------: | ---------: | -------: |
| agentflow | `agentflow-beta`                                                                 | Jison                                           |        4,013 |         33 |       39 |
| cynefin   | `cynefin-beta`                                                                   | Langium                                         |          884 |          2 |       12 |
| radar     | `radar-beta`                                                                     | Langium                                         |          559 |          1 |        6 |
| railroad  | `railroad-beta`, `railroad-ebnf-beta`, `railroad-abnf-beta`, `railroad-peg-beta` | Langium (EBNF, ABNF) and more, per the research |        2,140 |         11 |        9 |
| swimlanes | `swimlane-beta`                                                                  | the flowchart's, with the swimlane layout       |           63 |          2 |       46 |
| tree view | `treeView-beta`                                                                  | Langium                                         |          840 |          4 |       15 |
| use case  | `usecase-beta`                                                                   | Chevrotain                                      |        5,378 |         10 |       32 |
| venn      | `venn-beta`                                                                      | Jison                                           |          676 |          3 |       21 |
| wardley   | `wardley-beta`                                                                   | Langium                                         |        1,825 |          2 |       10 |

Source lines are the type's TypeScript under `packages/mermaid/src/diagrams/<type>/` without specs; layouts elsewhere (`rendering-util/layout-algorithms/swimlanes`, `ddlt`) and libraries are counted by the research.

## Decisions

| Topic             | Decision                                                                                                                                                                                                         | Source                                                                                                                        |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Renderer contract | The P3 renderers are written against the read-only typed views of decision 0009: their own config section, the globals they read, the theme as typed tokens. Their renderer stage starts when issue #4 is merged | [0009](./decisions/0009-configuration-resolved-once.md), issue [#4](https://github.com/sebastian-software/ferrigura/issues/4) |
| Working rules     | As in P2: [P2-RESEARCH.md](./P2-RESEARCH.md) for research, [P2-PARSERS.md](./P2-PARSERS.md) for parsers, [P2-RENDERERS.md](./P2-RENDERERS.md) for renderers                                                      | P2                                                                                                                            |
| Layout engines    | Open until the research: what swimlanes, agentflow (`ddlt`?), venn and railroad lay out with, and whether a library must be ported                                                                               | Research                                                                                                                      |

## Stages

1. **Research**, one task per type, and one per layout engine the research finds: `<type>/NOTES.md`, every rule with the reference's file and line, in the sections of [P2-RESEARCH.md](./P2-RESEARCH.md).
2. **Registry scaffold**, one task, merged before the parsers.
3. **Parsers**, one task per type: model, parser and DB, specs, differential.
4. **Layout engines and shared code**, as the research asks for them.
5. **Renderers**, after issue #4, each checked with `pnpm port:geometry <type>`.
