# Swimlanes: the layout engine (P3 milestone)

Status: the default path of the layout engine is ported in `crates/ferrigura-swimlane` and matches the reference corpus exactly, with two documented gaps in the sort (below). The parser and the renderer are later tasks. Research: [NOTES.md](NOTES.md), Part II (L0 to L8). Plan context: [../P3-PLAN.md](../P3-PLAN.md), decisions 2 and 3.

## 1. The crate

`crates/ferrigura-swimlane` is MIT, as the Mermaid sources are (`NOTICE.md`, `LICENSE`). It ports `packages/mermaid/src/rendering-util/layout-algorithms/swimlanes/` on the default path (`ignoreCrossLaneEdges: true`, `automaticLaneOrdering: false`). About 14,700 lines of Rust in `src/`, with no `unsafe`.

| Rust module                                                                                                                       | Reference                                                                                              | Role                                                                                                                                                                       |
| --------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `layout_core.rs`                                                                                                                  | `layoutCore.ts`                                                                                        | The seam: `run_swimlane_layout_core(&mut LayoutData) -> Result<Direction, Error>`.                                                                                         |
| `model.rs`                                                                                                                        | `rendering-util/types.ts` (the fields the layout reads and writes)                                     | `LayoutData`, `Node`, `Edge`, `Point`, `Rect`, `Config`, `Direction`. Other own properties are kept, in order, in an `extra` map. `undefined` is `None` and an absent key. |
| `helpers.rs`                                                                                                                      | `helpers.ts` (`toGraphView`, `writeBackToLayoutData`, `assignTopLaneTitleRect`, the lane width solver) | The graph view and the write-back.                                                                                                                                         |
| `phase0.rs`, `phase1.rs`, `phase2_layering.rs`, `phase2_dummies.rs`, `phase2_options.rs`, `phase3.rs`, `phase4.rs`, `pipeline.rs` | `phase0.helpers.ts` to `phase4.coordinates.ts`, `pipeline.ts`                                          | The Sugiyama phases on the default path.                                                                                                                                   |
| `router/`                                                                                                                         | `orthogonalRouter/router.ts`                                                                           | The orthogonal router: setup, ports, initial routing and search, crossing reduction, rebuild.                                                                              |
| `direction/`                                                                                                                      | `direction/*.ts`, `postProcessing.ts`                                                                  | The geometry passes, in the order of `postProcessing.ts` (section 3).                                                                                                      |
| `post_processing.rs`                                                                                                              | `postProcessing.ts`                                                                                    | The driver: the gating of the transforms, the order of the passes, the repeated passes.                                                                                    |
| `js.rs`, `math.rs`                                                                                                                | (none: JavaScript semantics)                                                                           | Section 4.                                                                                                                                                                 |
| `error.rs`                                                                                                                        | (none)                                                                                                 | `NonDefaultOption`, `NonAsciiId`, `NotPrintableAscii`, `UnknownDirection`.                                                                                                 |

The seam checks its input before any work: a non-default option, a direction other than `TB`, `LR`, `BT` or `RL`, and an id that is not printable ASCII are errors. The core reads the configuration with the reference's fallbacks (`nodeSpacing` 40, `rankSpacing` 100) and runs: the graph view, the Sugiyama phases, the write-back, the deletion of the edge points, the router, the curve rule (`rounded` where the curve is absent or `basis`), the post-processing. `validateSwimlanesLayout` is not called: it only logs and changes no field (`layoutCore.ts:55`).

## 2. What the port reproduces, and how it is checked

The port keeps each expression's operand order, the sort comparisons and the insertion orders (`indexmap` where the reference uses `Map`, `Set` or object key order). Equal output is the aim, not a tolerance: a difference in the last bit is a bug.

The reference hooks that make the corpus are guarded and change nothing when absent: `__mermaidSwimlaneStage` is called from `layoutCore.ts` (stages `input`, `sugiyama`, `writeback`, `routed`, `output`), from `postProcessing.ts` (one `post:<pass>` per pass that changed something, `#k` when a pass runs more than once), and helpers in `stageReport.ts`. The reference specs run the same with the hook (314 tests, 311 passed, 3 skipped, as in NOTES section 5).

## 3. The passes, in the order of `postProcessing.ts`

1. LR, RL or BT direction transform (`lr_transform.rs`), only with content nodes; an early return when the transform reports false.
2. `orthogonalizePolyline` and `simplifyPolyline` on every edge with two or more points (`geometry.rs`).
3. `simplifyDetouredEdges` (`detour_simplification.rs`).
4. `straightenCollinearSiblingDetours` (`sibling_shared_face_routing.rs`); `portSwapToLShape` (`port_swap.rs`).
5. `anchorLabelsToPolyline` (`label_anchoring.rs`), which runs five times in the reference's positions.
6. `clipEdgeEndpointsToNodeBoundaries` (`endpoint_clip.rs`); `collapseShortTerminalStub` (`terminal_stub.rs`).
7. `nudgeSharedInteriorSubpaths` (`shared_track_nudging.rs`), which runs twice.
8. The materialized passes (`direction/materialized/`): `separateSharedRenderedTerminalLanes`, `collapseRedundantRectangularDoglegs`, `liftObstacleHuggingSameSideRails`, `swapDestinationTerminalTailsToReduceCrossings`, then `finalizeRenderedEdges` (`resolveRenderedOrthogonalCrossings`, `reassignCrossingExternalRailChannels`, `shortcutRedundantOrthogonalJogs`, the anchoring and `prepareEdgeEndpointsForRenderer`). The driver runs the second `nudgeSharedInteriorSubpaths` (item 7) and a second `finalizeRenderedEdges` after this group.
9. The title-band passes `liftTopLaneTitleBandsAboveRails` and `shiftLeftLaneTitleBandsLeftOfRails`, each twice, in `materialized/title_bands.rs`.

The pair-candidate sort in `crossings.rs` and the other numeric sorts use `js::js_sort_by` (section 4). Sorts of strings use `locale_compare`, a total order.

## 4. JavaScript semantics (`js.rs`, `math.rs`)

- `to_fixed(x, digits)`: `Number.prototype.toFixed` by the specification, deciding on the exact binary value, ties rounded up in magnitude, and `Number::toString` for `|x| >= 1e21`. Checked against Node's V8 on 2,033 cases (`tests/data/to-fixed.txt`).
- `js_round(x)`: `Math.round`, half toward `+Infinity`, with V8's sign of zero. Checked against Node's V8 on 2,030 cases.
- `locale_compare` and `try_locale_compare`: `a.localeCompare(b)` for printable ASCII, as ICU's root collation at tertiary strength, written as a table of the 95 characters in ICU order. Checked in Chromium 151 on all 9,121 strings of length 0 to 2 and on 200,000 pairs, and against Node's V8 on the same data. Control characters and non-ASCII ids are refused (`NotPrintableAscii`, `NonAsciiId`); the collation of other ids is not ported.
- `js_sort_by(items, compare)`: `Array.prototype.sort` as Chromium 151 runs it (section 4.1).
- `math.rs`: `js_max`, `js_min` (JavaScript's `Math.max` and `Math.min` for NaN and zeros) and `js_sign_order`. `Math.hypot` is `ferrigura_jsmath::hypot`, V8's two-argument formula.

Reference: the browser of the reference is Playwright's Chromium 151.0.7922.34, locale en-US. Its `localeCompare` order equals V8's for printable ASCII (checked on the same data).

### 4.1 The sort, and its two gaps

Chromium 151's `Array.prototype.sort` makes different comparator calls from V8 13.6's (Node 24), even for consistent comparators, and gives different orders for inconsistent ones (the threshold comparators of the router, and NaN keys). The port follows Chromium where it has been found:

- 17 to 129 elements: exact, at every call site. Minimum run length `n` shifted right until below 64; the merge collapse rule `run[n-2] >= run[n-1] + run[n]`, with a tie merging the lower pair; a merge with a one-element run is a binary search and a rotation.
- 0 and 1: trivial.
- 2 to 15: a binary insertion sort from the second element, with no run detection, as a function's first call makes it (gap 1 applies to 2 to 16).
- 16 to 129: the TimSort variant above, exact from 17 (16 is in gap 1 for the call sites).
- Gap 1, 2 to 16 elements: Chromium's comparisons depend on the JIT state of the code that calls `sort`. A first call, a call site that has run thousands of sorts, and code made by `new Function` make different calls; `new Function` code sometimes repeats the first comparison. No single order is the reference's, so the port takes the first call's. Test: `js_sort_by_matches_chromium_from_2_to_16_elements` (ignored; 162 mismatches over 180 cases and 108 logs).
- Gap 2, 130 elements and more: Chromium's rules are not reproduced. The port keeps V8 13.6's TimSort there. The first difference is at `n = 130`, call 228. Test: `js_sort_by_matches_chromium_from_130_elements` (ignored; 1,883 of 2,088 cases differ).
- Effect: only the comparator call sequence and the final order under an inconsistent comparator. A consistent comparator gives the same order from every stable sort. The corpus reaches neither gap. The coordinator accepted both gaps as documented limits.

The oracle data is `tests/data/js-sort.txt` (Chromium's output, 2.27 MB), made by `tools/js-sort.ts` in Chromium, each case in code of its own. The passing test is `js_sort_by_matches_chromium_from_17_to_129_elements` (1,380 cases and 24 logs).

## 5. The corpus

`crates/ferrigura-swimlane/tests/corpus/`: 131 cases and `manifest.json` (32 MB). One case per call of `runSwimlaneLayoutCore`:

- `fixture`: 44, the e2e swimlane fixtures rendered in Chromium (`e2e/diagrams/swimlanes/`), with the reference's clock and seeded `Math.random` (as `tools/port/geometry/common.ts`). Fixtures set `htmlLabels: false` and `useMaxWidth: false`.
- `ddlt`: 27, the layout-test inputs of `e2e/platform/dev-diagrams/layout-tests/swimlanes/` run through `loadDdltFixture` under vitest (`tools/capture-ddlt.spec.ts`, `tools/vitest.config.ts`).
- `generated`: 60, seeded diagrams (`tools/generate-swimlanes.ts`, seed 20261018), with the same front matter as the fixtures.

Each case file has `input`, `stages` (`sugiyama`, `writeback`, `routed`, then the `post:` stages that changed something), `output` and `returnedDirection`. The manifest lists the key set (3,460 identifiers of the swimlane sources), the config keys (`flowchart`, `look`, `swimlane`) and the exclusions.

Excluded, with the reason in the manifest (11):

- `mermaid-work` and `14-messy-layout`, in the e2e and the DDLT sets: non-default options (`ignoreCrossLaneEdges: false`, `automaticLaneOrdering: true`).
- DDLT inputs without a `.sizes.json` (the discovery takes only fixtures with sizes): `9-edge-labels`, `issue-7802-6`, `pebr-2-edge-intrudes-lane-label`, `sales-process`, and the regressions `unconnected-source-order` and `unconnected-source-order-five`.
- `11-commant2.sizes.json`: a sizes file without its `.mmd` (the source is `.xmmd`).

Within the cases, a missing `post:` stage means the pass changed nothing. The comparison is exact: numbers with `==` on `f64`, an absent key is not `null`, nodes and edges by index (`tests/support/corpus.rs`).

Regenerate, from the repository root: `pnpm exec tsx crates/ferrigura-swimlane/tools/capture-corpus.ts` (about 8 minutes; it starts `pnpm dev`), and the generator `pnpm exec tsx crates/ferrigura-swimlane/tools/generate-swimlanes.ts`. The sort data: `pnpm exec tsx crates/ferrigura-swimlane/tools/js-sort.ts`. The data of `js-semantics.ts` (`to-fixed.txt`, `js-round.txt`, `ascii-*.txt`) is Node's V8 13.6, and the ASCII order is also checked in Chromium (section 4). `to_fixed` and `js_round` are checked against Node only.

## 6. Tests

- `tests/corpus.rs`: the sugiyama and writeback stages (exact on all 131), and `output_matches_the_reference_after_routing`, the whole core on every case: output snapshot and returned direction, exact.
- `tests/corpus_router.rs` (routed stage), `corpus_geometry.rs`, `corpus_materialized.rs` and `corpus_post.rs` (each pass on the state before its stage; the whole chain from `routed`).
- `tests/corpus_manifest.rs`: the manifest matches the files, the groups and the exclusions.
- `tests/hand_made.rs`: small layouts worked out by hand from the reference sources.
- `tests/js_semantics.rs`, `tests/js_sort.rs`: the JavaScript data, and the sort (two ignored tests for the gaps).

Gates: `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace --no-fail-fast`; `cargo test -p ferrigura-swimlane -- --ignored` runs the two sort gaps, which fail by design. The materialized and whole-chain corpus tests take about 50 s each in a debug build.

## 7. Not ported, and why

- `prepareLayoutForSwimlanes` and `createEdgeLabelNodes` (`helpers.ts` 98-140, `edgeLabelNodes.ts`): they run before the seam in the browser and the DDLT harness. The corpus input is after them. Their port needs the pre-seam capture, which is not in the corpus.
- `validateSwimlanesLayout` (`direction/validation.ts`): it only logs. The warnings are not reproduced.
- The non-default options: `phase2.gravity.ts`, `phase2.longestPath.ts`, `phase2.crossOptimization.ts`, `phase2.crossLaneAdjust.ts`, `phase2.crossCounts.ts`, `driving-tree.ts`, `phase2.multitree.*.ts`, `laneOrdering.ts` (about 1,270 lines). The seam refuses them with `NonDefaultOption`; two fixtures use them and are excluded.
- The parser and the renderer (lane shape, edge clipping and the swimlane branch of `insertEdge`, line hops, the `swimlane` configuration section): later tasks, as the brief says.
- The rough.js randomness of the renderer: not part of the layout.

## 8. Known limits beyond the sort

- Point sharing: the reference shares point objects in places. The port holds points as values. The router writes its own points; the materialized passes write fresh copies. By identity on the reference's output over 6,131 layouts, no point object or array is shared between edges. The corpus agrees, but this is evidence, not a proof.
- A cycle of parents in the lanes: the reference recurses without end; the port visits each node once and does not fail. A flowchart cannot produce one.
- NaN in the input: JSON cannot carry NaN, so only a direct call can reach the NaN paths. They follow the JavaScript semantics, and no test covers them.
- The random layouts checked during the router's work were run through the Node-run reference, not Chromium. The same caveat as for the sort applies to them.
