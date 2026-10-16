# Port status

Last updated: 2026-10-16. M1 is merged into `main`: pie and info parse and match the reference on every collected input. M4 is in progress on branch `sebastian/port-m4-elk-fe5cfa`: 120 of the 145 corpus cases are supported, and all 120 match elkjs exactly in coordinates, edge sections, label boxes and trace (`crates/ferrigura-elk`, see `elk/PLAN.md` §10).

Each session updates its section when it stops: what changed, what is next, what blocks it. Test counts and the differential column are written by `pnpm port:status`, which reads the Rust specs and the last `pnpm port:diff` run; run it after both.

## Milestones

| Milestone                          | Status      | Strand | Notes                                                                                                                                                                                         |
| ---------------------------------- | ----------- | ------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| M1 Parser foundation: pie and info | done        | main   | Merged on 2026-10-16; see M1 handover                                                                                                                                                         |
| M2 First slice: pie to SVG         | not started | —      | After M1, in parallel with M3                                                                                                                                                                 |
| M3 Flowchart parser                | not started | —      | After M1, in parallel with M2                                                                                                                                                                 |
| M4 ELK Layered port                | in progress | elk    | 120 of 145 corpus cases supported, all 120 match elkjs exactly in coordinates, edge sections, label boxes and trace; nested graphs next (22 left), then explicit ports; see `elk/PLAN.md` §10 |
| M5 Flowchart rendering             | not started | —      | Needs M2, M3 and M4                                                                                                                                                                           |
| M6 Remaining P1 diagrams           | not started | —      | After M5                                                                                                                                                                                      |

## Strands

| Strand | Session            | Worktree                                                                       | Working on                                                                                                                                                                                     | Next                                             | Blocked by |
| ------ | ------------------ | ------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ | ---------- |
| main   | —                  | —                                                                              | M1 done and merged                                                                                                                                                                             | M2 (pie to SVG) and M3 (flowchart parser)        | —          |
| elk    | port-m4-elk-fe5cfa | `.claude/worktrees/port-m4-elk-fe5cfa` (branch `sebastian/port-m4-elk-fe5cfa`) | M4: Layered pipeline with edges, centred labels and the input-model order; 120 of 145 corpus cases supported, all 120 match elkjs exactly in coordinates, edge sections, label boxes and trace | nested graphs (22 left), then explicit ports (1) | None       |

## M1 handover

Done: a Cargo workspace (`crates/ferrigura-model`, `crates/ferrigura-syntax`, stable toolchain pinned); the shared lexer on `logos`, the shared rules of `common.langium` (`title`, `accTitle`, `accDescr`), minimal front matter (CRLF, quote normalisation, the front matter `title`); the pie and info parsers with the pie DB rules; the canonical models `PieDiagram` and `InfoDiagram` (version 1, camelCase JSON); the pie and info specs ported with their test names; JS exporters in `tools/port/` run with tsx; the differential harness `pnpm port:diff <pie|info>`; `pnpm port:status`; CI jobs `rust` and `port-diff`.

Verified: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test --workspace` pass; `pnpm port:diff pie` reports 87 inputs and `pnpm port:diff info` 17 inputs, both with no difference; `pnpm port:check-header` finds no difference on 82 inputs; eslint and prettier pass on `tools/port`. Hand-picked edge cases (front matter, CRLF, directives, escapes, duplicate labels, negative values, malformed numbers) agree as well.

Deferred to M2: `pieRenderer.spec.ts` (SVG and DOM); the three configuration tests in `pie.spec.ts` (`getConfig`, and the two `it.todo` cases), which need the configuration model.

Canonical-model conventions are confirmed by the coordinating session and written down in [canonical-model.md](./canonical-model.md): unsanitised text, the info model without the build version, and title precedence with the front matter as a fallback for pie only. `pnpm port:check-header` compares the pie header rules with the real DB setters.

Known gaps: the front matter `title` is read as a plain or quoted scalar, not with the full YAML reader; comment lines inside a multi-line `accDescr` block are not removed before lexing, as the reference does; sanitisation of header and label text is not ported; the CI jobs have not run on GitHub yet.

Merged into `main` on 2026-10-16 after review; the checks above pass on `main`. Next: M2 needs `ferrigura-text` for measurement and the pie layout; the configuration model is the first M2 item because two pie specs wait for it.

## P1 diagram types

Baseline from the JS reference at commit `5a220b30e`: the grammar, the spec files under `packages/mermaid/src/diagrams/<type>/`, an approximate count of their test cases, and the e2e fixtures under `e2e/diagrams/`. pie, info and gitGraph also have parser tests in `packages/parser/tests/`.

| Type      | Grammar (JS)                                     | Spec files | ≈ Tests | Fixtures | Parser           | Specs ported / passing | Differential parse       | Layout | Render | Parity |
| --------- | ------------------------------------------------ | ---------: | ------: | -------: | ---------------- | ---------------------- | ------------------------ | ------ | ------ | ------ |
| flowchart | `diagrams/flowchart/parser/flow.jison`           |         20 |     318 |      352 | —                | —                      | —                        | —      | —      | —      |
| sequence  | `diagrams/sequence/parser/sequenceDiagram.jison` |         10 |     198 |      193 | —                | —                      | —                        | —      | —      | —      |
| class     | `diagrams/class/parser/classDiagram.jison`       |          5 |     255 |      247 | —                | —                      | —                        | —      | —      | —      |
| state     | `diagrams/state/parser/stateDiagram.jison`       |          9 |     121 |      145 | —                | —                      | —                        | —      | —      | —      |
| ER        | `diagrams/er/parser/erDiagram.jison`             |          3 |     142 |      206 | —                | —                      | —                        | —      | —      | —      |
| gantt     | `diagrams/gantt/parser/gantt.jison`              |          2 |      40 |       52 | —                | —                      | —                        | —      | —      | —      |
| pie       | `packages/parser/…/pie/pie.langium`              |          2 |      16 |       24 | ferrigura-syntax | 26 / 29 (3 ignored)    | 87 inputs, no difference | —      | —      | —      |
| timeline  | `diagrams/timeline/parser/timeline.jison`        |          1 |      11 |       30 | —                | —                      | —                        | —      | —      | —      |
| gitGraph  | `packages/parser/…/gitGraph/gitGraph.langium`    |          1 |      69 |      135 | —                | —                      | —                        | —      | —      | —      |
| journey   | `diagrams/user-journey/parser/journey.jison`     |          2 |       8 |       18 | —                | —                      | —                        | —      | —      | —      |
| info      | `packages/parser/…/info/info.langium`            |          1 |       4 |        2 | ferrigura-syntax | 6 / 6                  | 17 inputs, no difference | —      | —      | —      |

## Spikes

| Spike                                                     | Result                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| [Text measurement](./spikes/text-measurement.md) (for M2) | Widths match Chromium within its 1/64 px rounding, text box heights exactly, Mermaid's wrapping in 270 of 270 cases; about 5 µs per text. Embedded font subsets (`skera`) draw every label at the Rust width, also through `<img>`; family names must be unique per subset. Kerned labels are drawn at the Rust width in Chromium, Safari and Firefox; WebKit lays SVG text out unkerned but paints it kerned, so text runs get an explicit `x` instead of `text-anchor` |

## Open items

- Trademark check for `ferrigura` (DPMA, EUIPO, USPTO) before the first release; then create `sebastian-software/ferrigura` and rename the repository.
- Open `help wanted` issues for every P3 diagram type and for math in labels once the GitHub repository exists.
- Decide the mindmap layout when P2 starts.
- Replace the inherited changesets setup with release-please when the first crate is published.
