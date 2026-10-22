# Freezing the reference into ferrigura's own tests

The plan for checking ferrigura without running Mermaid: what the reference in `packages/` is used for today, how its answers become checked-in data, how large that data is, and in which order the steps go. Status: a plan, written on 2026-10-22 at Sebastian's request; nothing is changed yet, and the open decisions at the end are his.

## Why

Since decision [0003](./decisions/0003-differential-testing.md), every change of the port is checked against the running reference: Mermaid 12.2.0 as this repository's fork, built and run by the JavaScript toolchain in CI. All 33 diagram types have reached parity: the differential parse shows 0 differences with no named exception, and every comparable fixture renders within 0.05 px of the reference. From here on the reference is needed mainly to keep it that way.

That does not need the reference to run. Its answers can be recorded once and checked in, and ferrigura's own output can be pinned by snapshots. Then CI needs neither `packages/` nor most of the JavaScript toolchain, and the reference can leave the working tree. It stays in the history under a tag, for the work that needs Mermaid as an oracle again.

## What the reference does today

| Check                                                                       | Compares                                                                                                     | Inputs                                                                                     | Needs `packages/` | Needs Chromium |
| --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------ | ----------------- | -------------- |
| `port:diff <type>`, 33 types                                                | The canonical model of each input                                                                            | 8,636, collected by `tools/port/inputs.ts`                                                 | Yes               | No             |
| `port:diff <type>-data`, 6 runs                                             | The layout data (`getData`) of flowchart, class, ER, state, requirement and kanban                           | The inputs of those types                                                                  | Yes               | No             |
| `port:diff config`, `yaml`, `theme`                                         | The effective configuration and scope, the front matter, the theme variables                                 | 6,455, 4,269 and 110                                                                       | Yes               | No             |
| `port:check-header`                                                         | The pie header rules against the reference's DB setters                                                      | Its own cases                                                                              | Yes               | No             |
| `port:geometry <type>`, `port:geometry-pie`, `-flowchart`, `port:scope-pie` | Boxes, origins, path samples and text of both SVGs, measured in one Chromium page                            | 2,459 `.mmd` files: `e2e/diagrams` (2,058) and `tools/port/geometry/probes` (401)          | Yes               | Yes            |
| Rust tests `schema_copy`, `sankey_spec`, `use_case_docs_spec`               | The schema copy against its source; the sankey CSV and the use case docs as inputs                           | Files under `packages/`                                                                    | Yes               | No             |
| Captured corpora in the crates                                              | ELK, cose-bilkent, fcose, venn, swimlane, path bounds, `Math`, text widths, shapes, styles, edges, tidy tree | Already checked in as data; only their generators (`crates/*/tools`, `tools/port`) need it | For regeneration  | Some           |

Of the 8,636 diff inputs, 2,044 are `e2e/diagrams` files, 12 come from `e2e/platform`, 4,282 from `tools/port` (generated cases and probes), and 2,298 from `packages/`: string literals of the reference's specs (1,728) and the examples of its docs (570). Those 2,298 exist only inside `packages/`.

The captured corpora show the pattern this plan extends: the reference or Chromium answered once, the answer is checked in, and a Rust test compares against it.

## The principle

Two kinds of expected data, kept apart:

1. **Mermaid's answer, frozen.** The models, layout data, configurations and geometry the reference gives, written by its last run and never written again. Where ferrigura is right to differ, the difference is a named exception with a reason, as `tools/port/exceptions.ts` is today, not a change of the frozen file. Because these files never change, they are stored compressed.
2. **ferrigura's own output, pinned.** A hash of every SVG the corpus renders to, for each configuration and colour scheme. It catches every change of the output, including the ones the geometry check cannot see (markup, styles, embedded fonts). An intended change rewrites the hashes in the same commit, after the geometry check has shown that the new output still matches Mermaid. Because these hashes change with the code, they are stored as text, one line per render, so a diff shows which renders changed.

## Sizes

Measured on 2026-10-22 from `main`; the geometry from captures of class, pie and journey (289 fixtures) and scaled to the corpus.

| Data                                          | Entries                                     | JSON, compact | gzip    | Kind         |
| --------------------------------------------- | ------------------------------------------- | ------------- | ------- | ------------ |
| The inputs of the diff runs                   | 8,636, plus the config and YAML inputs      | 4.2 MB        | –       | Frozen, text |
| Canonical models, 33 types                    | 8,636                                       | 9.5 MB        | 0.49 MB | Frozen       |
| Layout data, 6 runs                           | 2,876 inputs of their types                 | 6.2 MB        | 0.17 MB | Frozen       |
| Configurations, as deltas to the common value | 6,455                                       | 38 MB         | 0.79 MB | Frozen       |
| Front matter and theme variables              | 4,269 and 110                               | 1.7 MB        | 0.14 MB | Frozen       |
| Reference geometry                            | about 2,400 comparable fixtures, 50 KB each | about 120 MB  | ~10 MB  | Frozen       |
| SVG hashes, 3 configurations × 4 schemes      | 31,500 renders of 2,625 files               | about 3.8 MB  | –       | Pinned, text |

The frozen files come to about 12 MB compressed; the inputs and the hash manifest add about 8 MB of text. For comparison, the captured corpora in the crates take 86 MB today, and `packages/` holds about 10 MB of tracked sources.

The SVGs themselves are not stored: one render of the corpus is 93 MB (19 MB compressed), mostly the embedded font subsets, and the full matrix of 12 renders per file would be about 1.1 GB. The geometry captures of the reference are dominated by the structural keys of the elements, so rounding the numbers saves little (class: 15.7 MB compact, 14.1 MB rounded to 0.001 px).

Rendering the 2,625 corpus files once takes 43 s with the release build (one process per file); in one process, the full matrix should take a few minutes in a release test.

## Steps

Each step is one change with CI green, in this order. Until step 7 the reference keeps running in CI, so every step is checked against it before it takes over.

1. **Freeze the inputs.** Write the inputs of every diff run (source and text) to `tests/golden/inputs/<run>.jsonl`, including the spec literals and the docs examples from `packages/` and the generated cases of `tools/port`. `inputs.ts` then reads these files instead of walking `packages/`. Check: every run collects the same inputs as before.
2. **Freeze the models and the other diff runs.** Run each diff run against the reference one last time and write its answers to `tests/golden/reference/<run>.json.gz`: models, layout data, configurations (as deltas to the common values), front matter, theme variables and the pie header cases. Rust integration tests compare ferrigura's answers to them, with the named exceptions in `tests/golden/exceptions.toml`. Check: the tests and the live runs agree; then the `port-diff` matrix in CI goes.
3. **Freeze the reference geometry.** Capture every type with `pnpm port:geometry <type> --capture` and the pie and flowchart harnesses likewise, and write the reference's geometry per type to `tests/golden/reference/geometry/<type>.json.gz`. The harness gets a mode that measures ferrigura's SVG in Chromium and compares it with the frozen geometry, without building or running the reference. Check: the frozen mode and the live mode report the same classes for every fixture; then CI's geometry job runs the frozen mode, with Node and Playwright but without `packages/`.
4. **Pin ferrigura's output.** A Rust test renders the corpus for each configuration and scheme and compares the SHA-256 of each SVG with `tests/golden/snapshots/svg.tsv`. `UPDATE_SNAPSHOTS=1` rewrites the manifest; a small tool renders the changed entries before and after the change and runs the frozen geometry check on them, so that a reviewer sees what moved. Check: the manifest is stable across two runs and across macOS and Linux (`ferrigura_jsmath` makes the math the same on both, but the venn layout is known to round differently without fused multiply-adds; if a difference shows, the manifest records one platform and CI runs it there).
5. **Copy what the Rust tests read.** The schema copy in `ferrigura-model` becomes the source and `schema_copy` goes; `energy.csv` and the docs examples of the use case test move next to their tests; `ts_keys.txt` stays as it is, frozen with the reference.
6. **Record the decision and set the tag.** Decision record 0010 states that the reference is frozen as data, what that gives up, and how to bring it back. The last commit with the live reference gets the tag `reference-mermaid-12.2.0`.
7. **Remove the reference.** `packages/`, the parts of `tools/port` that load the reference (`reference.ts`, the exporters, the diff runs), the capture tools under `crates/*/tools` that need it, `.esbuild`, `.build`, `.vite`, the Jison and Langium tooling and their dependencies go; `package.json` keeps what the frozen geometry check needs (tsx, Playwright). `pnpm-lock.yaml` is written anew. Optionally the corpus moves from `e2e/` to `tests/corpus/`.
8. **Update the documents.** README, CONTRIBUTING, docs-port (README, STATUS, decision 0003), the glossary and `tools/port/README.md`.
9. **Later, optional: measure in Rust.** A geometry measurer in Rust (transforms, element boxes, text boxes from `ferrigura-text`, path length and samples as Skia measures them) would take Chromium and Node out of CI entirely. It is checked against the Chromium measurements of the frozen corpus before it replaces them.

## What is given up, and how to get it back

- **New inputs cannot be checked against Mermaid directly.** A bug report of the kind "this renders differently in Mermaid" needs the reference again: `git worktree add ../ferrigura-reference reference-mermaid-12.2.0`, then `pnpm install` and the harness there, as today.
- **Work that needs Mermaid as an oracle** goes before step 7 or uses that worktree: Mermaid's classic themes (default, dark, forest, neutral), the hand-drawn look (rough.js) and math in labels.
- **The generators of the captured corpora** (ELK, cose, venn, swimlane, text and shape fixtures) run only in that worktree. The corpora themselves stay.

## Decisions for Sebastian

1. **Before or after the freeze:** port the classic themes and the hand-drawn look while the reference still runs, or later from the tag.
2. **Snapshots:** hashes only (as planned), or the full SVG of a sample of the corpus as well, so that a diff shows the markup.
3. **Measuring:** keep Chromium for the geometry check after the freeze (steps 3 and 7), or build the Rust measurer of step 9 before removing Node.
4. **The corpus folder:** keep `e2e/` or move it to `tests/corpus/`.
