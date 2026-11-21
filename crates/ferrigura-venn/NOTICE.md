# Notice: sources and licenses of ferrigura-venn

The crate combines MIT-licensed venn.js with BSD-3-Clause-licensed fmin. It ports the layout of
`@upsetjs/venn.js` 2.0.0, an MIT package (`LICENSE`, the copyright lines of Ben Frederickson and Samuel Gratzl), which Mermaid
12.2.0 bundles. What runs is the package's `build/venn.esm.js`, which inlines the `fmin` functions
the layout calls (build lines 262-622). The port follows that build, the code that runs.

| Source (`build/venn.esm.js`)                                                                                                                          | Lines     | Ported in                                                                                                  |
| ----------------------------------------------------------------------------------------------------------------------------------------------------- | --------- | ---------------------------------------------------------------------------------------------------------- |
| `intersectionArea`, `containedInCircles`, `getIntersectionPoints`, `circleArea`, `distance`, `circleOverlap`, `circleCircleIntersection`, `getCenter` | 1-268     | `src/geometry.rs`                                                                                          |
| `bisect`, `nelderMead`, `wolfeLineSearch`, `conjugateGradient`, `dot`, `norm2`                                                                        | 262-622   | `src/fmin.rs` (the `fmin` functions as inlined)                                                            |
| `venn`                                                                                                                                                | 623-672   | `src/placement.rs` (`venn`)                                                                                |
| `distanceFromIntersectArea`, `addMissingAreas`                                                                                                        | 684-768   | `src/placement.rs` (`distance_from_intersect_area`, `add_missing_areas`)                                   |
| `getDistanceMatrices`, `constrainedMDSGradient`, `bestInitialLayout`, `constrainedMDSLayout`                                                          | 770-932   | `src/placement.rs` (`distance_matrices`, `constrained_gradient`, `best_initial_layout`, `constrained_mds`) |
| `greedyLayout`                                                                                                                                        | 934-1077  | `src/placement.rs` (`greedy`)                                                                              |
| `lossFunction` (the squared error only)                                                                                                               | 1079-1101 | `src/placement.rs` (`loss`)                                                                                |
| `orientateCircles`, `disjointCluster`, `getBoundingBox`, `normalizeSolution`, `scaleSolution`, `toObjectNotation`, `fromObjectNotation`               | 1134-1447 | `src/arrange.rs`, and `src/js.rs` (`JsObject`) for the key order                                           |
| `VennDiagram`'s pipeline: the removal of zero-size singles, `venn`, `normalizeSolution`, `scaleSolution`, `computeTextCentres`                        | 1503-1526 | `src/diagram.rs` (`chart`)                                                                                 |
| `circleMargin`, `computeTextCentre`, `getOverlappingCircles`, `computeTextCentres`                                                                    | 1874-2042 | `src/text.rs`                                                                                              |
| `layout`                                                                                                                                              | 2146-2212 | `src/diagram.rs` (`layout`), without the arcs and path strings                                             |

The JavaScript behaviours the order of results depends on are in `src/js.rs`: `Object.keys` order,
V8's `Array.prototype.sort` (binary insertion for arrays under 64 elements, which gives the same
order as V8 for every comparator, NaN included), `Math.min` and `Math.max` on NaN and signed zero,
and the UTF-16 order of strings.

## Numbers

`sin`, `cos`, `atan2` and `acos` come from `ferrigura-jsmath`, whose notice is in its own
`NOTICE.md`. They are Chromium's functions, bit for bit, which is what the reference runs in.

## fmin

The inlined functions come from `fmin` 0.0.4, Copyright 2016 Ben Frederickson,
under BSD-3-Clause; its complete license is in `LICENSE-fmin`. The published
[npm tarball](https://registry.npmjs.org/fmin/-/fmin-0.0.4.tgz) has SHA-512
`18379b05d8b35ba8351792b19bfcbf0f46ca8dcf3305e21e1845e9803b867a333279a69fb8b90a651c44cd62aa7a10d18a9c50f21d1c2e1743442b31e93d2de3`.
The [venn.js 2.0.0 package manifest](https://github.com/upsetjs/venn.js/blob/350c835aab4a92a7570963c28f725cf9f6e5f258/package.json)
selects that version with a [Yarn patch](https://github.com/upsetjs/venn.js/blob/350c835aab4a92a7570963c28f725cf9f6e5f258/.yarn/patches/fmin-npm-0.0.4-e439f499bd.patch).
The patch declares the reduction loop variable in `nelderMead` and adjusts module
metadata. The bundled `bisect`, `zeros`, `dot`, `norm2`, `scale`, `weightedSum`,
`nelderMead`, `wolfeLineSearch`, and `conjugateGradient` match the corresponding
published source functions after that patch, apart from removed `export` keywords.

## The corpus

`tests/corpus` holds the cases written by `tools/port/crates/venn/capture-corpus.ts`:

- 21 fixtures: every file of `e2e/diagrams/venn`, parsed with the reference's own Jison grammar and
  DB, and given the reference's `ensurePairwiseSubsets` (`vennRenderer.ts`). The box is the renderer's
  default (800 by 450, less the title's 24 px), the layout's padding 8, the chart's padding 15.
- 26 probes: hand-written texts (numeric and alphanumeric ids, duplicates, zero and negative sizes,
  overlaps of every kind, a set named `constructor`, a set named `__proto__`) and area lists given to
  the library directly (no areas, an area with no single, an empty area).
- 400 generated set systems (`tools/generate-set-systems.ts`, seed 20261018): two to six sets, the
  five id schemes, sizes of every kind, duplicated sets and unions, through the same parser and
  renderer function. They cycle through four boxes and layout paddings.

The reference runs in Chromium through the geometry harness (`withPage` in
`tools/port/geometry/common.ts`). The library's build is served to the page with one line appended
that exports `addMissingAreas` and `constrainedMDSLayout`; the build exports the rest. The chart's
drawing needs d3 and is not run: its pipeline is replicated from the build's exported functions
(the lines above). `layout()` is called as the build exports it.

Random numbers: `Math.random` is replaced by the harness's generator (mulberry32, `renderReference`
in `tools/port/geometry/common.ts`), and each draw is recorded. The stages and the chart each start
the generator at seed 1, as a render does; the layout continues from the chart's draws, as the
renderer's second call does, and starts again from seed 1 only when the chart throws. The tests seed
a port of that generator the same way, and check that its draws are the recorded ones.

## Not ported

The options the renderer does not pass: `distinct`, `scaleToFit`, `symmetricalTextCentre`,
`orientationOrder`, the logarithmic and custom losses, custom layouts, the weight of an area, the
history options, `restarts` and `maxIterations` (fixed at 10 and 500, as `venn()` sets them). The
path strings, arcs and `distinctPath` of `layout()`. The drawing. The `console.log` warnings. A set
named `__proto__` is refused (`Error::ReservedName`).
