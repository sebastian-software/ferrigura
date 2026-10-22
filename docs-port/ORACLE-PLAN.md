# Checking the port against upstream Mermaid

The plan for checking ferrigura against Mermaid itself, pinned to an upstream commit, instead of against this repository's fork of it: why, what a measurement against upstream 12.2.0 showed, how the differences are handled, and the steps that retire the fork from the repository. Status: a plan, written on 2026-10-22 with Sebastian, who decided its open points the same day (see the end); the measurement is done, the steps are not.

## Why

Since decision [0003](./decisions/0003-differential-testing.md), every change of the port is checked against the running reference: `packages/`, this repository's fork of Mermaid 12.2.0. The fork removed deprecated keywords, options and the dagre layout, and fixed some bugs of Mermaid (see [FORK.md](../FORK.md)). The port was checked against the fork, so it follows those changes.

Checking against Mermaid itself is the better promise: ferrigura renders what Mermaid renders, measured against the original, not against a fork of our own. A pinned upstream commit also gives a clear way to follow Mermaid: moving the pin to a newer release shows, as differences, exactly what ferrigura has to catch up on. And the fork can leave the repository: `packages/`, its build tooling and Mermaid's history are no longer needed in it, as upstream holds them.

Regressions during development and refactoring are caught in two layers: the differential runs against the pinned reference, and hashes of ferrigura's own SVG output, which catch every change of the output, including those the geometry check cannot see.

## The measurement of 2026-10-22

The harness took the reference from an upstream checkout of `e28fbd9de` (upstream `develop` at 12.2.0, where the fork starts), built with upstream's own toolchain. The change that makes the reference configurable is on the branch `sebastian/upstream-oracle` (`REFERENCE_ROOT`, 9 files; with the variable unset, the runs are unchanged). All 33 model exporters, the 6 layout data exporters and the configuration and theme exporters ran on upstream's modules without a change.

**Every difference comes from a change the fork made.** No input shows ferrigura behind upstream.

| Run                                   | Differences against upstream                                                                                         |
| ------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| 30 of the 33 model types              | 0                                                                                                                    |
| flowchart, flowchart-data             | 3 each: the removed `flowchart-elk` keyword                                                                          |
| sankey                                | 1: the removed `sankey-beta` keyword                                                                                 |
| architecture                          | 5: the fork's fix of the graduated `architecture` keyword (4), and `service architecture`, which only upstream takes |
| er-, state-, class-, requirement-data | 0                                                                                                                    |
| kanban-data, yaml, pie header rules   | 0                                                                                                                    |
| config                                | Every input: options the fork removed (`legacyMathML`, `er.minEntityHeight`, `state.*`), and three config bugs       |
| theme                                 | 99 of 110: the theme variables `attributeBackgroundColorEven` and `…Odd`, which the fork removed                     |

Inputs that only the fork's corpus has (the tests of its fixes) add 6 differences: the `architecture` keyword (3), `topAxis` in gantt, the `constructor` shape and the `__proto__` subgraph member.

Geometry: of the comparable fixtures, 49 are outside the tolerance, and nothing else differs; most types match bit for bit.

| Cause                                                                                                                                                                                                               | Fixtures                                                                                                   |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `htmlLabels`: upstream's triangle and swimlane lane labels read `flowchart.htmlLabels`, which upstream leaves unset, so they are drawn as HTML labels; the fork made them follow the root option, as ferrigura does | 43 (swimlanes 38, flowchart triangles 5); all within when the reference gets `flowchart.htmlLabels: false` |
| Railroad options in directives, which upstream drops (a bug the fork fixed)                                                                                                                                         | 4 probes                                                                                                   |
| The `wardley-beta` configuration section, which upstream never applies (a bug the fork fixed)                                                                                                                       | 1 probe                                                                                                    |
| `layout: dagre`, which upstream lays out with dagre and ferrigura with ELK                                                                                                                                          | 1 probe                                                                                                    |

Limits of the measurement: upstream has no ELK capture hook, so the ELK inputs of the flowchart harness are not compared; the input collector recognises only the fork's keywords, so 20 upstream texts with `-beta` keywords were not collected; and the corpus has no fixture with `layout: dagre`.

## How differences are handled

Every difference is one of three kinds, and each kind has one place:

1. **ferrigura is behind upstream.** It shows up when the pin moves. Each difference gets an issue; until the fix, it is a pending exception that names the issue, so CI stays green and nothing is forgotten. A pending exception without an issue is an error.
2. **Mermaid has a bug that ferrigura does not.** The fix is applied to the pinned checkout as a patch before it is built, so the reference is "Mermaid with this fix", whether the bug touches one input or hundreds. The fork's own fixes are such patches already. Each patch names the bug and, where there is one, the upstream issue; when upstream fixes the bug, the patch goes with the next move of the pin.
3. **ferrigura does not support something on purpose.** For example the dagre layout, or HTML labels (decision [0004](./decisions/0004-svg-text-labels.md)). Such a feature is a rule in the comparison (the reference's forced configuration, or which inputs a check compares), with the reason next to it.

What the measurement found, sorted into these kinds:

| Difference                                                                                              | Kind and handling                                                                                                                                |
| ------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| The keywords `flowchart-elk`, `sankey-beta`, `packet-beta`, `xychart-beta`, `block-beta`                | Behind upstream: users write them, so ferrigura accepts them again (issue)                                                                       |
| `service architecture` as an id after `architecture-beta`                                               | Behind upstream: ferrigura accepts it (issue)                                                                                                    |
| The removed options `legacyMathML`, `er.minEntityHeight`, `state.*` and the two theme variables         | Not supported on purpose: no code reads them, upstream's included; the configuration run leaves these keys out, one rule with its reason         |
| The `architecture` keyword, `topAxis`, the `constructor` shape, `__proto__`, the detector word boundary | Mermaid bugs, a few inputs each: the fork's fixes as patches                                                                                     |
| Railroad options, the `wardley-beta` section, the `c4.flowchart` copy                                   | Mermaid bugs, many inputs each (417, 219 and 176 in the config run): the fork's fixes as patches                                                 |
| `htmlLabels` of triangles and lane labels                                                               | Not supported on purpose: the reference's forced configuration gets `flowchart.htmlLabels: false` next to the `htmlLabels: false` it has already |
| `layout: dagre`                                                                                         | Not supported on purpose: inputs that name dagre are left out of the geometry check, and compared as configuration                               |
| The ELK capture hook (`__mermaidElkCapture`), which only the fork has                                   | A patch on the pinned checkout, so the flowchart harness can compare the ELK inputs again                                                        |

So the pinned reference is upstream plus nine small patches (the fork's eight fixes and the capture hook), and the comparison has three rules (`htmlLabels`, dagre and the inert options). No input needs an exception of its own; the exception file starts empty and holds the pending cases of later moves of the pin.

## The design

- **The pin.** `tools/reference/pin` holds the upstream repository and commit; `tools/reference/patches/` holds the patches, each with a header that says why it exists and which upstream issue it belongs to. A script fetches the pinned commit (shallow, one commit), applies the patches, installs and builds it into `target/reference/<commit>`, and the harness runs with `REFERENCE_ROOT` pointing there. CI caches the build, keyed by the commit, the patches and the lockfile.
- **The inputs.** The corpus of this repository (`e2e/diagrams`, the dev diagrams, the probes and the cases of `tools/port`) plus the spec literals and docs examples of the pinned checkout, as today. Moving the pin brings Mermaid's new tests and docs examples in as inputs by itself.
- **The exceptions and rules.** One file, each entry with its kind (pending with an issue, or unsupported with a reason) and the inputs or the rule it covers; Mermaid bugs are patches, not entries. The runs fail on a difference that no entry covers and on an entry that covers nothing any more.
- **ferrigura's own output.** A Rust test renders the corpus for each configuration and colour scheme (3 × 4) and compares, per corpus file, one SHA-256 over its 12 renders with `tests/snapshots/svg.tsv`: 2,625 lines, about 0.3 MB of text. `UPDATE_SNAPSHOTS=1` rewrites it; a tool shows which renders of a changed file moved, and runs the geometry check on them. This test needs neither Node nor the reference, so it runs in `cargo test`.
- **Moving the pin.** A pull request moves `tools/reference/pin`; the runs show what is new in Mermaid. Each difference is fixed in that pull request or becomes a pending exception with its issue. Patches that upstream has made unnecessary go.

Nothing large is checked in: the reference is fetched and built from upstream, and its answers live only in CI's cache. The SVG hashes are the only new data in the repository.

## Steps

Each step is one change with CI green. Until step 4 the fork keeps running in CI next to upstream, so every step is checked against both.

1. **Catch up with upstream where the fork took things away.** ferrigura accepts the removed keywords (`flowchart-elk` as a flowchart with ELK, the `-beta` names as aliases of their types) and `architecture` as a name after the header again. Done on 2026-10-22: against upstream the keyword differences are gone and the collector takes the `-beta` keywords too; against the fork, which CI still runs, the three inputs it rejects are named exceptions that go in step 3.
2. **Set up the pinned reference.** The pin, the patches (the fork's fixes of the `architecture` keyword, in the form step 1 gave ferrigura: the graduated keyword is the header only, and a name after it, `topAxis`, the `constructor` shape, `__proto__`, the detector word boundary, the railroad options, the `wardley-beta` section and the `c4.flowchart` copy, and the ELK capture hook), the script, `flowchart.htmlLabels: false` in the forced configuration, the dagre rule, the rule for the inert options, and the merge of `sebastian/upstream-oracle`. The input collector learns upstream's keywords. Done on 2026-10-22: [tools/reference/](../tools/reference/README.md) holds the pin, nine patches and `setup.sh`; against that reference every diff run shows 0 differences and every comparable fixture renders within the tolerance, swimlanes and the flowchart triangles included.
3. **Switch CI to the pinned reference.** The exception file replaces `tools/port/exceptions.ts`; `port-diff` and `port-geometry` run against `target/reference/<commit>`. Check: every run is clean against upstream with the exceptions it lists. Done on 2026-10-22: `port-diff` and `port-geometry` build the reference with `tools/reference/setup.sh` and cache it per pin; the exceptions of the fork are gone, and an exception is `pending` with its issue or `unsupported` with its reason.
4. **Pin ferrigura's output.** The SVG hash test and its tools, in `cargo test`.
5. **Retire the fork.** `packages/`, the fork's build tooling (`.esbuild`, `.build`, `.vite`, the Jison and Langium setup) and the dependencies only it needs go; the harness keeps what it needs to load and render the pinned checkout (Vite, tsx, jsdom, Playwright), and takes the Jison and schema plugins from the pinned checkout's `.vite`, as it does since step 1. `vite.config.ts` becomes a small Vitest configuration for the tests of the harness (the environment and the excludes), without the plugins, `define`, `includeSource` and `build` options that Mermaid's own tests needed; the swimlane capture tool's Vitest configuration takes its plugins from the pinned checkout too. `tsconfig.json` keeps what tsx and the typed lint of `tools/port` and the TypeScript tools of the crates need, without the template comments and Mermaid's settings (`useDefineForClassFields`, the `vitest/globals` types); `tsconfig.eslint.json` loses `.build`, `.esbuild` and `.vite`. The schema copy in `ferrigura-model` is checked against the pinned checkout's schema; the Rust tests that read files from `packages/` get their own copies. The last commit with the fork gets the tag `fork-mermaid-12.2.0`.
6. **Record and document.** Decision record 0010: the reference is upstream Mermaid at a pinned commit, with patches and exceptions as above. README, CONTRIBUTING, docs-port, the glossary and `tools/port/README.md` follow; FORK.md stays as the record of the fork.
7. **Later, optional.** Move the pin to the current Mermaid release, as the first real catch-up. A geometry measurer in Rust would take Chromium out of the checks of ferrigura's own output; it is not needed for this plan.

A new repository without Mermaid's history, as Sebastian considers, fits after step 5: nothing in it would need that history any more.

## Decisions

Decided by Sebastian on 2026-10-22:

1. **Compatibility:** ferrigura accepts the removed keywords (`flowchart-elk`, `sankey-beta`, `packet-beta`, `xychart-beta`, `block-beta`) and `architecture` as a name again, as upstream does. The removed configuration options and theme variables stay removed: no code reads them, upstream's included, so the configuration run leaves them out by one rule.
2. **Mermaid bugs:** every fix of the fork becomes a patch on the pinned checkout, eight in all, and the ELK capture hook a ninth.
3. **Snapshots:** one hash line per corpus file in the repository, about 0.3 MB.
4. **The pin:** it stays at 12.2.0 (`e28fbd9de`) until the fork is retired; then it follows Mermaid's releases, one pull request per minor release, with an issue for everything to catch up on.
