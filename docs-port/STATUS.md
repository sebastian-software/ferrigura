# Port status

Last updated: 2026-10-15. Planning is done; M4 Stage A is done (ELK source pinned, inventory and plan in `elk/`); no Rust code exists yet.

Each session updates its section when it stops: what changed, what is next, what blocks it. Test counts per diagram type are meant to be written by a script once M1 delivers it; until then they are maintained by hand.

## Milestones

| Milestone                          | Status       | Strand | Notes                                                   |
| ---------------------------------- | ------------ | ------ | ------------------------------------------------------- |
| M1 Parser foundation: pie and info | not started  | main   | Starts first; builds the shared infrastructure          |
| M2 First slice: pie to SVG         | not started  | —      | After M1, in parallel with M3                           |
| M3 Flowchart parser                | not started  | —      | After M1, in parallel with M2                           |
| M4 ELK Layered port                | Stage A done | elk    | Stage B waits for coordinator review; see `elk/PLAN.md` |
| M5 Flowchart rendering             | not started  | —      | Needs M2, M3 and M4                                     |
| M6 Remaining P1 diagrams           | not started  | —      | After M5                                                |

## Strands

| Strand | Session            | Worktree                                                                       | Working on                                                                                                                         | Next                                                                                       | Blocked by                                                                 |
| ------ | ------------------ | ------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------- |
| elk    | port-m4-elk-fe5cfa | `.claude/worktrees/port-m4-elk-fe5cfa` (branch `sebastian/port-m4-elk-fe5cfa`) | M4 Stage A: ELK source pinned to `9bc93474e` (inferred, see `elk/SOURCES.md`), Mermaid's option set, Java inventory, `elk/PLAN.md` | Stage B (differential corpus and harness) and Stage C (port) once the coordinator confirms | Coordinator review of the scope rule and the source pin (`elk/PLAN.md` §8) |

## P1 diagram types

Baseline from the JS reference at commit `5a220b30e`: the grammar, the spec files under `packages/mermaid/src/diagrams/<type>/`, an approximate count of their test cases, and the e2e fixtures under `e2e/diagrams/`. pie, info and gitGraph also have parser tests in `packages/parser/tests/`.

| Type      | Grammar (JS)                                     | Spec files | ≈ Tests | Fixtures | Parser | Specs ported / passing | Differential parse | Layout | Render | Parity |
| --------- | ------------------------------------------------ | ---------: | ------: | -------: | ------ | ---------------------- | ------------------ | ------ | ------ | ------ |
| flowchart | `diagrams/flowchart/parser/flow.jison`           |         20 |     318 |      352 | —      | —                      | —                  | —      | —      | —      |
| sequence  | `diagrams/sequence/parser/sequenceDiagram.jison` |         10 |     198 |      193 | —      | —                      | —                  | —      | —      | —      |
| class     | `diagrams/class/parser/classDiagram.jison`       |          5 |     255 |      247 | —      | —                      | —                  | —      | —      | —      |
| state     | `diagrams/state/parser/stateDiagram.jison`       |          9 |     121 |      145 | —      | —                      | —                  | —      | —      | —      |
| ER        | `diagrams/er/parser/erDiagram.jison`             |          3 |     142 |      206 | —      | —                      | —                  | —      | —      | —      |
| gantt     | `diagrams/gantt/parser/gantt.jison`              |          2 |      40 |       52 | —      | —                      | —                  | —      | —      | —      |
| pie       | `packages/parser/…/pie/pie.langium`              |          2 |      16 |       24 | —      | —                      | —                  | —      | —      | —      |
| timeline  | `diagrams/timeline/parser/timeline.jison`        |          1 |      11 |       30 | —      | —                      | —                  | —      | —      | —      |
| gitGraph  | `packages/parser/…/gitGraph/gitGraph.langium`    |          1 |      69 |      135 | —      | —                      | —                  | —      | —      | —      |
| journey   | `diagrams/user-journey/parser/journey.jison`     |          2 |       8 |       18 | —      | —                      | —                  | —      | —      | —      |
| info      | `packages/parser/…/info/info.langium`            |          1 |       4 |        2 | —      | —                      | —                  | —      | —      | —      |

## Open items

- Trademark check for `ferrigura` (DPMA, EUIPO, USPTO) before the first release; then create `sebastian-software/ferrigura` and rename the repository.
- Open `help wanted` issues for every P3 diagram type and for math in labels once the GitHub repository exists.
- Decide the mindmap layout when P2 starts.
- M4: confirm the scope rule (Mermaid-reachable options only) and the ELK commit pin, see `elk/PLAN.md` §8. Seven fixtures use non-layered ELK algorithms and are excluded from the ELK differential run.
- Replace the inherited changesets setup with release-please when the first crate is published.
