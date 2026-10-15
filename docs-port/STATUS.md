# Port status

Last updated: 2026-10-15. M1 is implemented on branch `sebastian/port-m1-parser-4d51c2`, not merged; pie and info parse and match the reference on every collected input.

Each session updates its section when it stops: what changed, what is next, what blocks it. Test counts and the differential column are written by `pnpm port:status`, which reads the Rust specs and the last `pnpm port:diff` run; run it after both.

## Milestones

| Milestone                          | Status      | Strand | Notes                                                     |
| ---------------------------------- | ----------- | ------ | --------------------------------------------------------- |
| M1 Parser foundation: pie and info | in review   | main   | Branch `sebastian/port-m1-parser-4d51c2`; see M1 handover |
| M2 First slice: pie to SVG         | not started | —      | After M1, in parallel with M3                             |
| M3 Flowchart parser                | not started | —      | After M1, in parallel with M2                             |
| M4 ELK Layered port                | not started | elk    | Independent; can start on day one                         |
| M5 Flowchart rendering             | not started | —      | Needs M2, M3 and M4                                       |
| M6 Remaining P1 diagrams           | not started | —      | After M5                                                  |

## Strands

| Strand | Session                              | Worktree                                  | Working on                                                                   | Next                                                 | Blocked by                                                 |
| ------ | ------------------------------------ | ----------------------------------------- | ---------------------------------------------------------------------------- | ---------------------------------------------------- | ---------------------------------------------------------- |
| main   | M1 session (`port-m1-parser-4d51c2`) | `.claude/worktrees/port-m1-parser-4d51c2` | M1: `ferrigura-syntax`, `ferrigura-model`, the JS exporters, the harness, CI | Review and merge; then M2 takes the pie model to SVG | Answers to the canonical-model questions (see M1 handover) |

## M1 handover

Done: a Cargo workspace (`crates/ferrigura-model`, `crates/ferrigura-syntax`, stable toolchain pinned); the shared lexer on `logos`, the shared rules of `common.langium` (`title`, `accTitle`, `accDescr`), minimal front matter (CRLF, quote normalisation, the front matter `title`); the pie and info parsers with the pie DB rules; the canonical models `PieDiagram` and `InfoDiagram` (version 1, camelCase JSON); the pie and info specs ported with their test names; JS exporters in `tools/port/` run with tsx; the differential harness `pnpm port:diff <pie|info>`; `pnpm port:status`; CI jobs `rust` and `port-diff`.

Verified: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test --workspace` pass; `pnpm port:diff pie` reports 79 inputs and `pnpm port:diff info` 15 inputs, both with no difference; eslint and prettier pass on `tools/port`. Hand-picked edge cases (front matter, CRLF, directives, escapes, duplicate labels, negative values, malformed numbers) agree as well.

Deferred to M2: `pieRenderer.spec.ts` (SVG and DOM); the three configuration tests in `pie.spec.ts` (`getConfig`, and the two `it.todo` cases), which need the configuration model.

Open decisions, sent to the coordinating session with defaults applied: header texts in the canonical model are unsanitised source text, because DOMPurify needs a DOM and would HTML-serialise them, so escaping is M2's job; the info model keeps the grammar's header and not the build's version; the front matter title is the fallback for pie only.

Known gaps: the front matter `title` is read as a plain or quoted scalar, not with the full YAML reader; comment lines inside a multi-line `accDescr` block are not removed before lexing, as the reference does; sanitisation of header and label text is not ported; the CI jobs have not run on GitHub yet.

Next: review, then merge. M2 needs `ferrigura-text` for measurement and the pie layout; the configuration model is the first M2 item because two pie specs wait for it.

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
| pie       | `packages/parser/…/pie/pie.langium`              |          2 |      16 |       24 | ferrigura-syntax | 26 / 29 (3 ignored)    | 79 inputs, no difference | —      | —      | —      |
| timeline  | `diagrams/timeline/parser/timeline.jison`        |          1 |      11 |       30 | —                | —                      | —                        | —      | —      | —      |
| gitGraph  | `packages/parser/…/gitGraph/gitGraph.langium`    |          1 |      69 |      135 | —                | —                      | —                        | —      | —      | —      |
| journey   | `diagrams/user-journey/parser/journey.jison`     |          2 |       8 |       18 | —                | —                      | —                        | —      | —      | —      |
| info      | `packages/parser/…/info/info.langium`            |          1 |       4 |        2 | ferrigura-syntax | 6 / 6                  | 15 inputs, no difference | —      | —      | —      |

## Spikes

| Spike                                                     | Result                                                                                                                                                                                                                                                                     |
| --------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Text measurement](./spikes/text-measurement.md) (for M2) | Widths match Chromium within its 1/64 px rounding, text box heights exactly, Mermaid's wrapping in 270 of 270 cases; about 5 µs per text. Embedded font subsets (`skera`) draw every label at the Rust width, also through `<img>`; family names must be unique per subset |

## Open items

- Trademark check for `ferrigura` (DPMA, EUIPO, USPTO) before the first release; then create `sebastian-software/ferrigura` and rename the repository.
- Open `help wanted` issues for every P3 diagram type and for math in labels once the GitHub repository exists.
- Decide the mindmap layout when P2 starts.
- Replace the inherited changesets setup with release-please when the first crate is published.
