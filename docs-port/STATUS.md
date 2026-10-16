# Port status

Last updated: 2026-10-16. M1 is merged into `main`: pie and info parse and match the reference on every collected input. M4 is in progress on branch `sebastian/port-m4-elk-fe5cfa`.

Each session updates its section when it stops: what changed, what is next, what blocks it. Test counts and the differential column are written by `pnpm port:status`, which reads the Rust specs and the last `pnpm port:diff` run; run it after both.

## Milestones

| Milestone                          | Status      | Strand | Notes                                                    |
| ---------------------------------- | ----------- | ------ | -------------------------------------------------------- |
| M1 Parser foundation: pie and info | done        | main   | Merged on 2026-10-16; see M1 handover                    |
| M2 First slice: pie to SVG         | in progress | m2     | Stage B done for pie; Stage C (CLI, Node addon) next     |
| M3 Flowchart parser                | not started | —      | After M1, in parallel with M2                            |
| M4 ELK Layered port                | in progress | elk    | Branch `sebastian/port-m4-elk-fe5cfa`; see `elk/PLAN.md` |
| M5 Flowchart rendering             | not started | —      | Needs M2, M3 and M4                                      |
| M6 Remaining P1 diagrams           | not started | —      | After M5                                                 |

## Strands

| Strand | Session | Worktree                   | Working on                                   | Next                                      | Blocked by |
| ------ | ------- | -------------------------- | -------------------------------------------- | ----------------------------------------- | ---------- |
| main   | —       | —                          | M1 done and merged                           | M2 (pie to SVG) and M3 (flowchart parser) | —          |
| m2     | —       | `determined-bartik-42e25d` | Stage C: CLI and Node addon, then the report | —                                         | —          |

## M1 handover

Done: a Cargo workspace (`crates/ferrigura-model`, `crates/ferrigura-syntax`, stable toolchain pinned); the shared lexer on `logos`, the shared rules of `common.langium` (`title`, `accTitle`, `accDescr`), minimal front matter (CRLF, quote normalisation, the front matter `title`); the pie and info parsers with the pie DB rules; the canonical models `PieDiagram` and `InfoDiagram` (version 1, camelCase JSON); the pie and info specs ported with their test names; JS exporters in `tools/port/` run with tsx; the differential harness `pnpm port:diff <pie|info>`; `pnpm port:status`; CI jobs `rust` and `port-diff`.

Verified: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test --workspace` pass; `pnpm port:diff pie` reports 87 inputs and `pnpm port:diff info` 17 inputs, both with no difference; `pnpm port:check-header` finds no difference on 82 inputs; eslint and prettier pass on `tools/port`. Hand-picked edge cases (front matter, CRLF, directives, escapes, duplicate labels, negative values, malformed numbers) agree as well.

Deferred to M2: `pieRenderer.spec.ts` (SVG and DOM); the three configuration tests in `pie.spec.ts` (`getConfig`, and the two `it.todo` cases), which need the configuration model.

Canonical-model conventions are confirmed by the coordinating session and written down in [canonical-model.md](./canonical-model.md): unsanitised text, the info model without the build version, and title precedence with the front matter as a fallback for pie only. `pnpm port:check-header` compares the pie header rules with the real DB setters.

Known gaps: the front matter `title` is read as a plain or quoted scalar, not with the full YAML reader; comment lines inside a multi-line `accDescr` block are not removed before lexing, as the reference does; sanitisation of header and label text is not ported; the CI jobs have not run on GitHub yet.

Merged into `main` on 2026-10-16 after review; the checks above pass on `main`. Next: M2 needs `ferrigura-text` for measurement and the pie layout; the configuration model is the first M2 item because two pie specs wait for it.

## M2 handover

Stage A (configuration, text, YAML) is accepted and on main. Stage B (themes, pie renderer, geometry comparison) is done for pie: the two shipped themes are ported and match the reference exactly; the pie renderer and the `render(text, config)` facade are on the branch; the geometry comparison passes for all 24 pie fixtures. Stage C (CLI, Node addon) is next.

Stage B, done: `crates/ferrigura-model/src/theme/` ports the khroma functions the themes use and the two shipped themes, `redux-color` and `redux-dark-color`, in the order of the reference. `pnpm port:diff theme` runs 110 cases (55 override sets for each theme) against the reference: no difference. The configuration takes its `themeVariables` from the theme, computed as `updateCurrentConfig` does, and `pnpm port:diff config` covers them for the two shipped themes. `crates/ferrigura-render` lays out and writes a pie as `pieRenderer.ts` does (slices with d3 angles, the legend in its four positions, the title, the viewBox), with label widths from `ferrigura-text`; `crates/ferrigura` offers `render(text, config) → svg`. `tools/port/geometry/pie-geometry.ts` runs the reference in Chromium (`htmlLabels: false`, Lexica as a web font, redux-color) and ferrigura over the 24 pie fixtures and compares the geometry in one Chromium page. All 24 are within tolerance; the largest deviation is 0.016 px, one layout unit of 1/64 px, against the 0.05 px tolerance, and the angles agree within 1e-5 rad (d3-path writes three decimals). Side-by-side images for review: the notes folder, `stage-b/pie-geometry/`. Two things found on the way: a width that is off the 1/64 px grid makes Chromium scale the SVG and lay out its text at the scaled size, so the pie width is snapped to the grid; and the comparison must wait for the embedded faces to load.

Theme names: `redux-color` and `redux-dark-color` ship. Any other name falls back by darkness (names ending in `dark` take `redux-dark-color`, the rest `redux-color`), so the reference's `default`, `forest`, `neutral` and `dark` themes are drawn with one of the two. The configuration differential therefore compares theme variables only for the two shipped themes.

Known gaps for M2 (Stage B): the empty pie (no sections) gives the reference a `-Infinity` legend width and a broken viewBox, ferrigura uses zero; fallback fonts for scripts Lexica does not cover; the accessibility title and description of the SVG are not written yet; `info` is not rendered; the `default` theme of the reference is not drawn with its own variables.

Configuration, done: `crates/ferrigura-model/build.rs` generates the configuration types, their defaults and `CONFIG_KEYS` from `packages/mermaid/src/schemas/config.schema.yaml`, so the schema stays the single source. `src/config/ts_defaults.json` holds the TypeScript-only defaults (`pie.useWidth`, `treemap`); `src/config/ts_keys.txt` lists the keys the reference accepts that the schema does not name, which are the theme variables of the default theme and the TypeScript-only sections. `ferrigura_model::config::effective` applies the layers in the order of `mermaidAPI` (site config, front matter, directives; `assignWithDepth`, the key gating of `sanitizeDirective`, the removal of secure keys, the resolution of `theme`, `look` and `layout` through the diagram section). `ferrigura_syntax::config::read` reads the front matter `config` block (serde_yaml_ng) and the `init`, `initialize` and `wrap` directives, with the reference's error behaviour: invalid JSON in any directive drops all of them.

Verified: `pnpm port:diff config` runs 120 inputs (the 87 pie inputs and 33 explicit cases in `tools/port/config-cases.ts`) through the reference's steps and through ferrigura: no difference, and the key sets are equal. `pnpm port:diff pie` and `info` still report no difference.

Compared: the top level and the `pie` section. The other diagram sections are compared when their type is ported, for two reasons: the reference fills them with its own generator (`.build/jsonSchema.ts`, which applies a section's own defaults only), and `defaultConfig.ts` replaces some sections whole. Ferrigura takes their defaults from the schema, so for example `class` and `railroad` carry defaults that the reference does not give them. `dompurifyConfig` is not a key in the reference and is left out of `CONFIG_KEYS`; `$ref` is left out of the comparison (a leftover of the generator in `xyChart`).

Specs: the pie `get_config` spec is ported and passes. `set_config` and `reset_config` stay ignored, because they are `it.todo` in the reference with their bodies commented out, and the pie DB has no `setConfig`.

YAML, done: `crates/ferrigura-syntax/src/yaml.rs` reads YAML as js-yaml 4 does with `JSON_SCHEMA` (typing of plain scalars, string keys, errors for duplicate keys, extra documents and unresolvable core tags, `undefined` for a stream without a document). The structure comes from `saphyr-parser`; the typing is ours. `pnpm port:diff yaml` runs 4269 inputs: the front matter of the e2e fixtures (2073, all accepted by js-yaml), the flowchart shape data of the fixtures and of the flowchart specs (2112), and 84 explicit cases. No difference. The front matter (its title, `config` and `displayMode`) is read with this module, so a front matter that does not parse is an error for every diagram. Shared with M3 from step 7 onward.

Text, done: `crates/ferrigura-text` (fonts, measurement with harfrust, line metrics, Mermaid's wrapping, styled runs, subsetting with per-subset family names) is merged from its own worktree. Verified against Chromium in a fixture (180 widths, 24 boxes, 27 wraps, 7 styled-run lines) and against the spike's Rust binary. Deviations from the spike: the subset family name is a 16-hex-digit FNV-1a hash, not six digits; `wrap_line` takes one face, although Mermaid wraps styled words per word (styled runs are measured per run); no fallback fonts, WOFF2 or caching yet. The spike's generator for the Chromium fixture is not committed.

Known gaps in the configuration: `themeVariables` are not computed (the theme port does that in Stage B), and they are left out of the comparison; non-finite numbers (`.inf`, `.nan`) are carried as the strings `"Infinity"`, `"-Infinity"` and `"NaN"`, because a serde_json value cannot hold them (a theme reads `THEME_COLOR_LIMIT` from there); a `config` key in a directive maps to the section of pie and info only; a directive whose arguments are not an object (a word, a number, an array, a string) contributes nothing, as the reference's merge ignores such values, and 13 explicit cases in `config-cases.ts` show it; the front matter `config` block is ignored when it is not an object, which the same cases check.

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

- Trademark check for `ferrigura` (DPMA, EUIPO, USPTO) before the first release. The repository is public as `sebastian-software/ferrigura` since 2026-10-16; the local folder is still called `ferraid`.
- Open `help wanted` issues for every P3 diagram type and for math in labels once the GitHub repository exists.
- Decide the mindmap layout when P2 starts.
- M5 (flowchart labels with Markdown): `ferrigura-text`'s `wrap_line` takes one face, but Mermaid wraps styled words one by one, each in its own face (`createText.ts` gives each word after the first a leading space in its own tspan). The runs of a line are measured per run already; the wrapping must take runs too.
- Replace the inherited changesets setup with release-please when the first crate is published.
