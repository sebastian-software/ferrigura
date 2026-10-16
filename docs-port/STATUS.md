# Port status

Last updated: 2026-10-16. M1 is merged into `main`: pie and info parse and match the reference on every collected input. M4 is in progress on branch `sebastian/port-m4-elk-fe5cfa`.

Each session updates its section when it stops: what changed, what is next, what blocks it. Test counts and the differential column are written by `pnpm port:status`, which reads the Rust specs and the last `pnpm port:diff` run; run it after both.

## Milestones

| Milestone                          | Status      | Strand                         | Notes                                                                                                                                            |
| ---------------------------------- | ----------- | ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| M1 Parser foundation: pie and info | done        | main                           | Merged on 2026-10-16; see M1 handover                                                                                                            |
| M2 First slice: pie to SVG         | in progress | m2                             | Branch `sebastian/determined-bartik-42e25d`; Stage A done, see M2 handover                                                                       |
| M3 Flowchart parser                | in progress | sebastian/blissful-cori-9818ff | Steps 1 to 6 done (nodes, edges, subgraphs, styles, interactions); step 7 (shape data) in progress, see [flowchart/PLAN.md](./flowchart/PLAN.md) |
| M4 ELK Layered port                | in progress | elk                            | Branch `sebastian/port-m4-elk-fe5cfa`; see `elk/PLAN.md`                                                                                         |
| M5 Flowchart rendering             | not started | —                              | Needs M2, M3 and M4                                                                                                                              |
| M6 Remaining P1 diagrams           | not started | —                              | After M5                                                                                                                                         |

## Strands

| Strand | Session | Worktree                   | Working on                                          | Next                                           | Blocked by |
| ------ | ------- | -------------------------- | --------------------------------------------------- | ---------------------------------------------- | ---------- |
| main   | —       | —                          | M1 done and merged                                  | M2 (pie to SVG) and M3 (flowchart parser)      | —          |
| m2     | —       | `determined-bartik-42e25d` | Stage A: configuration and text, checkpoint pending | Stage B after the coordinating session answers | —          |

## M1 handover

Done: a Cargo workspace (`crates/ferrigura-model`, `crates/ferrigura-syntax`, stable toolchain pinned); the shared lexer on `logos`, the shared rules of `common.langium` (`title`, `accTitle`, `accDescr`), minimal front matter (CRLF, quote normalisation, the front matter `title`); the pie and info parsers with the pie DB rules; the canonical models `PieDiagram` and `InfoDiagram` (version 1, camelCase JSON); the pie and info specs ported with their test names; JS exporters in `tools/port/` run with tsx; the differential harness `pnpm port:diff <pie|info>`; `pnpm port:status`; CI jobs `rust` and `port-diff`.

Verified: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test --workspace` pass; `pnpm port:diff pie` reports 87 inputs and `pnpm port:diff info` 17 inputs, both with no difference; `pnpm port:check-header` finds no difference on 82 inputs; eslint and prettier pass on `tools/port`. Hand-picked edge cases (front matter, CRLF, directives, escapes, duplicate labels, negative values, malformed numbers) agree as well.

Deferred to M2: `pieRenderer.spec.ts` (SVG and DOM); the three configuration tests in `pie.spec.ts` (`getConfig`, and the two `it.todo` cases), which need the configuration model.

Canonical-model conventions are confirmed by the coordinating session and written down in [canonical-model.md](./canonical-model.md): unsanitised text, the info model without the build version, and title precedence with the front matter as a fallback for pie only. `pnpm port:check-header` compares the pie header rules with the real DB setters.

Known gaps: the front matter `title` is read as a plain or quoted scalar, not with the full YAML reader; comment lines inside a multi-line `accDescr` block are not removed before lexing, as the reference does; sanitisation of header and label text is not ported; the CI jobs have not run on GitHub yet.

Merged into `main` on 2026-10-16 after review; the checks above pass on `main`. Next: M2 needs `ferrigura-text` for measurement and the pie layout; the configuration model is the first M2 item because two pie specs wait for it.

## M2 handover

Stage A (configuration and text) is written; Stages B (themes, pie renderer, geometry comparison) and C (CLI, Node addon) wait for the coordinating session's answer.

Configuration, done: `crates/ferrigura-model/build.rs` generates the configuration types, their defaults and `CONFIG_KEYS` from `packages/mermaid/src/schemas/config.schema.yaml`, so the schema stays the single source. `src/config/ts_defaults.json` holds the TypeScript-only defaults (`pie.useWidth`, `treemap`); `src/config/ts_keys.txt` lists the keys the reference accepts that the schema does not name, which are the theme variables of the default theme and the TypeScript-only sections. `ferrigura_model::config::effective` applies the layers in the order of `mermaidAPI` (site config, front matter, directives; `assignWithDepth`, the key gating of `sanitizeDirective`, the removal of secure keys, the resolution of `theme`, `look` and `layout` through the diagram section). `ferrigura_syntax::config::read` reads the front matter `config` block (serde_yaml_ng) and the `init`, `initialize` and `wrap` directives, with the reference's error behaviour: invalid JSON in any directive drops all of them.

Verified: `pnpm port:diff config` runs 120 inputs (the 87 pie inputs and 33 explicit cases in `tools/port/config-cases.ts`) through the reference's steps and through ferrigura: no difference, and the key sets are equal. `pnpm port:diff pie` and `info` still report no difference.

Compared: the top level and the `pie` section. The other diagram sections are compared when their type is ported, for two reasons: the reference fills them with its own generator (`.build/jsonSchema.ts`, which applies a section's own defaults only), and `defaultConfig.ts` replaces some sections whole. Ferrigura takes their defaults from the schema, so for example `class` and `railroad` carry defaults that the reference does not give them. `dompurifyConfig` is not a key in the reference and is left out of `CONFIG_KEYS`; `$ref` is left out of the comparison (a leftover of the generator in `xyChart`).

Specs: the pie `get_config` spec is ported and passes. `set_config` and `reset_config` stay ignored, because they are `it.todo` in the reference with their bodies commented out, and the pie DB has no `setConfig`.

YAML, done: `crates/ferrigura-syntax/src/yaml.rs` reads YAML as js-yaml 4 does with `JSON_SCHEMA` (typing of plain scalars, string keys, errors for duplicate keys, extra documents and unresolvable core tags, `undefined` for a stream without a document). The structure comes from `saphyr-parser`; the typing is ours. `pnpm port:diff yaml` runs 4269 inputs: the front matter of the e2e fixtures (2073, all accepted by js-yaml), the flowchart shape data of the fixtures and of the flowchart specs (2112), and 84 explicit cases. No difference. The front matter (its title, `config` and `displayMode`) is read with this module, so a front matter that does not parse is an error for every diagram. Shared with M3 from step 7 onward.

Text, done: `crates/ferrigura-text` (fonts, measurement with harfrust, line metrics, Mermaid's wrapping, styled runs, subsetting with per-subset family names) is merged from its own worktree. Verified against Chromium in a fixture (180 widths, 24 boxes, 27 wraps, 7 styled-run lines) and against the spike's Rust binary. Deviations from the spike: the subset family name is a 16-hex-digit FNV-1a hash, not six digits; `wrap_line` takes one face, although Mermaid wraps styled words per word (styled runs are measured per run); no fallback fonts, WOFF2 or caching yet. The spike's generator for the Chromium fixture is not committed.

Known gaps in the configuration: `themeVariables` are not computed (the theme port does that in Stage B), and they are left out of the comparison; non-finite numbers (`.inf`, `.nan`) are carried as the strings `"Infinity"`, `"-Infinity"` and `"NaN"`, because a serde_json value cannot hold them (a theme reads `THEME_COLOR_LIMIT` from there); a `config` key in a directive maps to the section of pie and info only; a directive whose arguments are not an object is ignored, where the reference passes the value on; the front matter `config` block is ignored when it is not an object.

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
| pie       | `packages/parser/…/pie/pie.langium`              |          2 |      16 |       24 | ferrigura-syntax | 27 / 29 (2 ignored)    | 87 inputs, no difference | —      | —      | —      |
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
