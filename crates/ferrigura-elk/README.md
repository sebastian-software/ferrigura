# ferrigura-elk

`ferrigura-elk` is the ELK Layered layout of the Eclipse Layout Kernel, ported from its Java sources. It covers the options that Mermaid sets, and its results are compared with elkjs 0.9.3 on the same ELK JSON graphs. The other ELK algorithms are not ported.

This crate is a part of [ferrigura](https://github.com/sebastian-software/ferrigura). Its API may change between versions. To render diagrams, use the `ferrigura` crate.

## The source pin

The port has to give what elkjs computes, so it follows the version elkjs was built from, not the latest ELK. Mermaid loads `elkjs/lib/elk.bundled.js` and depends on `elkjs@^0.9.3`.

| Item    | Value                                                                                                                                          |
| ------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| elkjs   | 0.9.3 (repository `kieler/elkjs`, tag `0.9.3`, release commit `a8304cf79fde75bc2ab1a89d28320f53f8637436` of 2024-04-16), built with GWT 2.10.0 |
| ELK     | `eclipse/elk` at commit `9bc93474e1dc649450e8c97abb45d16b7c88e35b`, "Metacompiler fix (#1024)", committed 2024-04-15                           |
| License | EPL-2.0 for both                                                                                                                               |

elkjs does not record the ELK commit it was built from, so the pin is inferred, from three facts. The elkjs bundle contains the top-down layout change of ELK commit `7ca51784e` (`NodeContext` has a `topdownLayout` field, which `NodeSizeCalculator` reads), so the tag `v0.9.1` is too old. The commits after `7ca51784e` change no runtime code: the generated-options commit and its revert cancel out, and the rest is documentation, templates and the metacompiler tooling. And `9bc93474e` is the newest commit on ELK's `master` before the elkjs release commit. Mermaid never sets `elk.topdownLayout`, so the top-down change does not reach it; the pin matters for fidelity, and a wrong pin would show as a difference in the corpus.

Option ids, defaults and enum values are declared in `*.melk` files, not in Java: the metacompiler generates the `*Options` and `*MetaDataProvider` classes at build time, and they are not in the repository at the pin. The port therefore reads the defaults from the melk files of the plugins that elkjs bundles (`Core.melk`, `Labels.melk`, `Layered.melk` and the melk files of the other bundled algorithms, because option suffixes are unique only over the whole registry). `tools/gen_option_registry.py` and `tools/gen_enums.py` write `src/core/data/registry.rs` and `src/core/data/enums.rs` from a checkout at the pin:

```bash
git clone --filter=blob:none --no-checkout https://github.com/eclipse/elk .elk-src/elk
git -C .elk-src/elk checkout 9bc93474e1dc649450e8c97abb45d16b7c88e35b
python3 crates/ferrigura-elk/tools/gen_option_registry.py .elk-src/elk crates/ferrigura-elk/src/core/data/registry.rs
python3 crates/ferrigura-elk/tools/gen_enums.py .elk-src/elk crates/ferrigura-elk/src/core/data/enums.rs
```

Both generated files say so in their first lines and are not edited by hand.
The JSON contract (what goes in, what comes out, how `layoutOptions` keys are resolved) is partly elkjs's own, not ELK's: `JsonAdapter.xtend`, `ElkJs.java` and the `elk-api.js` and `main-api.js` of the elkjs repository at its release commit. The port follows those as well.

## What is ported

Only the paths Mermaid can reach. A value is in scope if a Mermaid user can make Mermaid pass it to elkjs: through the `elk` section of the configuration (its presets `default`, `legacy`, `modelOrder` and `depthFirst` and the strategy keys) or through the live option keys of `rendering-util/layout-algorithms/elk/render.ts`. The option catalog in Mermaid is commented out line by line, so it adds nothing. Options Mermaid does not set keep ELK's defaults, and the port keeps those defaults too.

- **Phases.** Cycle breaking (depth first, greedy, and the model-order variants), layering (network simplex and the other layerers), crossing minimization (layer sweep with barycenters, the two-sided greedy switch and the model order), node placement (Brandes and Köpf with all alignments, the other placers) and orthogonal edge routing, with the intermediate processors each of them adds; node, edge, center and end labels; self loops with one edge; the layer constraints `FIRST` and `LAST`; separately handled connected components; nested graphs laid out level by level and compound layouts (`INCLUDE_CHILDREN`) with edges across the hierarchy; ports on the east and west sides.
- **Which processors run** depends on the graph. The phase classes add processors conditionally (the orthogonal router adds its hyperedge, self-loop, north-south port and label processors only when the graph has such parts), and `AlgorithmAssembler` orders them by slot and ordinal; the processor trace of elkjs is the check that the order is right.
- **Not ported**, each reported as `LayoutError::Unsupported` with its name: other algorithms than `elk.layered` (`elk.box`, `elk.rectpacking` and the rest: the unified renderer lays out with Layered when a diagram asks for another name, [0012](https://github.com/sebastian-software/ferrigura/blob/main/docs/decisions/0012-layout-engines-ported-exactly.md)); hyperedges (a port with several edges, which Mermaid never creates, since its edges name nodes); polyline and spline routing; the one-sided greedy switch; port labels, an explicit port anchor and ports without a side; north and south external port dummies, a fixed order of eastern and western external ports, in-layer constraints, inside self loops and the greedy switch in a compound layout, none of which Mermaid's option set reaches.
- Nested graphs take the direction of the top graph; a nested `elk.direction` is ignored, as in Java.

## How the port maps to the sources

Modules are named after the Java packages they port, and each file keeps the EPL-2.0 notice of its source.

| Java package                                | Module                                                                                                                                                             |
| ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `org.eclipse.elk.core` (options, recursion) | `core` (`data` for the registry and the parsing of option values, `recursive`, `util`)                                                                             |
| `org.eclipse.elk.graph`, `graph.json`       | `graph` (plain structs for the EMF model, `json_import`), `json` (the elkjs JSON types)                                                                            |
| `org.eclipse.elk.alg.common`                | `common` (network simplex, overlap removal)                                                                                                                        |
| `org.eclipse.elk.alg.layered`               | `layered`: `driver`, `configurator`, `importer`, the phases `p1cycles` to `p5edges`, `intermediate` and the processors it names, `compound`, `components`, `loops` |
| Java and GWT semantics                      | `java` (`Random`, the merge sort, `Math.round` and 32-bit integers)                                                                                                |

Java semantics that GWT compiles differently from the JVM are reproduced where results depend on them: `java.util.Random`, the legacy merge sort behind `Collections.sort` (insertion sort below seven elements, a left-preferring merge, so that an inconsistent comparator gives elkjs's order), `Math.round` and the clamping of integer conversions. Where elkjs behaves in a way the Java sources do not show, the code says so at the place and names the probe or log that settled it. Four such behaviors cost the most to find:

- **The layer sweep's two tries share one flag.** The Java declares `SECOND_TRY_WITH_INITIAL_ORDER` with the id of `FIRST_TRY_WITH_INITIAL_ORDER`; properties are keyed by id, so the second try never starts. The port keeps one flag.
- **Unset options read the layered defaults.** An option that targets parents takes the `supports` value of `Layered.melk` when it is unset (an unset `edgeRouting` is orthogonal); one that targets nodes keeps its melk default (an unset `portAlignment.default` is `DISTRIBUTED`, not the layered `JUSTIFIED`).
- **Mermaid's `layeringLayerBound` key works.** `elk.layered.layering.coffmanGraham.layerBound` is a suffix of a registered id, while the shorter spellings are not; suffix lookup runs over the whole registry, as `getOptionDataBySuffix` does.
- **Trace names are Java's designations**: `LEdge.toString` writes `e_<designation>`, where the designation is the ELK edge's id with the ids of its containers in front.

elkjs also mutates its input: it adds `x`, `y`, `width`, `height`, `logging` and a GWT identity hash (`$H`) to the object it is given, so a case stores the input before the layout, and the output without `$H`. JSON numbers are read exactly: `serde_json` runs with `float_roundtrip` at the workspace level, because its default parser is not correctly rounded and a width of `14.013293266296387` would otherwise come out one ulp off and move a Brandes and Köpf alignment ([LEARNINGS.md](https://github.com/sebastian-software/ferrigura/blob/main/docs/LEARNINGS.md#javascript-and-browser-arithmetic)).

## How the corpus is made and checked

`tests/corpus` holds the cases, and `manifest.json` lists them: 169 cases, of which 129 come from Mermaid's fixtures and 40 are synthetic. A case file has the elkjs JSON graph that Mermaid handed elkjs (copied before the layout, since elkjs mutates it), elkjs's result, and elkjs's logging tree, with the processor sequence and each processor's log lines. The 129 fixture cases were captured while the reference rendered the fixtures in Chromium, through the capture hook of the reference's patch 07 (`globalThis.__mermaidElkCapture`, [tools/reference/README.md](https://github.com/sebastian-software/ferrigura/blob/main/tools/reference/README.md)); a case is named after its fixture. The synthetic cases cover the features that no fixture reaches (greedy cycles, components that are not separated, explicit ports, self loops in every ordering, compound edge routing, the model-order variants), each added with its case first, and are laid out by Node's elkjs from the reference's `node_modules`. The generated files are not formatted by the repository's formatter.

The corpus is fixed test data. The harness's geometry page installs the capture hook (`tools/port/geometry/common.ts`) and `tools/port/crates/elk/corpus-model.ts` builds the synthetic graphs, but the repository has no script that writes case files: a new case is the elkjs input, result and logging tree of one call, recorded through the hook or laid out by Node's elkjs, and added to `manifest.json`.

The tests compare the port with elkjs, with a tolerance of zero (the port reproduces the arithmetic, so a difference is a bug):

- `tests/differential.rs`: node boxes, edge sections and label boxes of every case the port lays out (165 of 169; the other four are `elk.box` twice, `elk.rectpacking` and one port with two edges, named in `NOT_PORTED` with their reasons). The floors only go up.
- `tests/trace.rs`: the full processor trace with its logs, on every case the port lays out. It fails on a case that cannot be laid out with the trace enabled, so a missing processor name cannot hide a case.
- `tests/corpus_manifest.rs`: the manifest and the case files agree, so a dropped case shows without a browser.
- `tests/random_graphs.rs` and `tests/random_compound_graphs.rs` (ignored by default): random flat graphs (4 to 18 nodes, all four directions, the model-order strategies, both cycle breakers, labels, self loops; 1200 graphs) and random compound graphs (900) against Node's elkjs, in coordinates and in the trace with logs. The generators are `tools/port/crates/elk/random-graphs.ts` and `random-compound-graphs.ts`; they are seeded and write under `target/`:

```bash
pnpm -C tools/port exec tsx crates/elk/random-graphs.ts --count 1200
cargo test -p ferrigura-elk --test random_graphs -- --ignored --show-output
pnpm -C tools/port exec tsx crates/elk/random-compound-graphs.ts --count 900
cargo test -p ferrigura-elk --test random_compound_graphs -- --ignored --show-output
```

Each tool takes `--seed` and `--out`, and the tests read `FERRIGURA_RANDOM_GRAPHS` or `FERRIGURA_COMPOUND_GRAPHS` for another directory. The generators and `tools/port/crates/elk/corpus-model.ts` (the shared helpers: `$H` stripping, the first difference of two JSON values, the synthetic graphs) load elkjs from the reference's build, so they need `tools/reference/setup.sh` first.

A change to the port runs `cargo test -p ferrigura-elk`, and a change that could move a coordinate should also run the two random sets: the corpus cannot reach what the generators do. Mermaid's post-processing of the ELK result, which is not in this crate, is checked with the flowchart geometry run ([LEARNINGS.md](https://github.com/sebastian-software/ferrigura/blob/main/docs/LEARNINGS.md#the-layout-engines)).

## License

Eclipse Public License 2.0 (`EPL-2.0`), as the Eclipse Layout Kernel is. This crate is the only one of ferrigura under that license, because it is a derivative of the Eclipse sources. The `LICENSE` file has the license text, and `NOTICE.md` has the notices of the Eclipse Layout Kernel project.
