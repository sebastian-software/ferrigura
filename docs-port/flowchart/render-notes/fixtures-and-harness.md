# M5: flowchart fixtures and the geometry harness

Status: research notes for milestone M5 (geometry comparison of flowchart fixtures). Read-only pass over the repository at HEAD `e8af08065` on branch `sebastian/port-m5-flowchart-render`. No repository file was changed except this one.

Evidence labels used below:

- **source**: read in the code, with file and line.
- **runtime**: run in Chromium against the built bundle of the main checkout (`/Users/sebastian/Workspace/ferraid/packages/mermaid/dist`, built 11:57, before HEAD's commit time 12:17; whether it matches HEAD is **unverified**).
- **classifier**: a regex and YAML pass over every fixture (heuristic where stated).
- **unverified**: not confirmed.

Scripts and raw output (throwaway, outside the repository): `/private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/m5-fixtures/` (`classify.mjs` writes `features.json`; `summarize.mjs`, `final-stages.mjs` and `gen-notes.mjs` derive the tables; `refdom.mjs`, `refdom2.mjs`, `lookcmp.mjs` are the runtime checks; `lists-full.txt` has every class with its full file list).

## Summary

- 352 fixtures under `e2e/diagrams/flowchart`: nightly-check 195, visual-check 146, pr-check 11. Keyword: flowchart 259, graph 93.
- **Defaults of this fork change what the fixtures mean.** The flowchart section of the schema defaults to `theme: redux-color` and `look: neo`; the root default layout is `elk` (source: `FlowchartDiagramConfig` in `packages/mermaid/src/schemas/config.schema.yaml`, line 2438 on; runtime: nodes carry `data-look="neo"` with no look configured). Only 50 fixtures set `handDrawn`, and none sets `classic`.
- **Default appearance** (look neo, theme redux-color, set or unset): 231 of 352 files. The rest: look handDrawn 50, a theme other than redux-color 71 (look not handDrawn). No fixture sets look classic (0). Section 0 lists them.
- **Geometry depends on the look.** Under `neo` a node of the fixture `9-should-render-subgraphs` is 152 x 40 px; under `classic` it is 180 x 46 px, and the viewBox differs (runtime, four fixtures checked; see section 2.4).
- **Ferrigura renders no flowchart yet.** `render_with` knows only `Pie` and `Info` (source: `crates/ferrigura/src/lib.rs`, `crates/ferrigura-syntax/src/lib.rs`), so a flowchart input is an error today.
- **Hard blockers for ferrigura** (union, 143 of 352 files): `htmlLabels: true` set explicitly (66), `fontFamily: courier` (70), `look: handDrawn` (50), `elk.box` or `elk.rectpacking` (2). Ferrigura has no HTML label mode, no courier metrics, no rough.js, and ELK only as layered.
- **Staging** (section 5): stage 1 has 13 files, stage 1a (shape-alias fixtures) 7, stage 1b (courier, after a font decision) 11. Stage 1 + 1a + 1b is 31, which is "about 30". Stage 2 adds 28 files (subgraphs and edge labels).

## 0. Appearance: default run versus the rest

Flowchart defaults in this fork: `theme: redux-color` and `look: neo`, from the `FlowchartDiagramConfig` section of `packages/mermaid/src/schemas/config.schema.yaml` (line 2438 on), resolved by `resolveAppearance` in `packages/mermaid/src/config.ts` (line 79). A fixture that sets neither runs on the default appearance.

| Appearance                                                         | Files |
| ------------------------------------------------------------------ | ----: |
| Default: look neo (unset or set), theme redux-color (unset or set) |   231 |
| Look handDrawn (theme unset in 49, base in 1)                      |    50 |
| Look classic set explicitly                                        |     0 |
| Look not handDrawn, theme set to another name                      |    71 |

Other themes set explicitly (look not handDrawn), by name: `redux` 14, `redux-dark` 14, `neo` 10, `neo-dark` 10, `default` 4, `dark` 4, `forest` 4, `neutral` 4, `redux-dark-color` 4, `base` 3. Explicit `theme: default` (Mermaid's default colours, not the flowchart default) is 4 files.

Non-default appearance, full list: look handDrawn (50), then the other themes (71).

**look handDrawn (50):**

- `nightly-check/handdrawn/` (50): 01-fdh39-should-render-a-flowchart-when-usemaxwidth-is-false, 02-fdh38-should-render-a-flowchart-when-usemaxwidth-is-true-default, 03-fdh49-should-add-edge-animation, apply-class-called-default-on-node-called-default, apply-default-class-to-all-nodes-which-do-not-have-another-class-assigned-htmllabels-disabled, apply-default-class-to-all-nodes-which-do-not-have-another-class-assigned-htmllabels-enabled, chaining-of-nodes, fontawesome-icons-in-edge-labels-2, fontawesome-icons-in-edge-labels-3, fontawesome-icons-in-edge-labels-4, fontawesome-icons-in-edge-labels, handle-link-click-events-link-anchor-mailto-other-protocol-script, handle-styling-for-all-node-shapes, handle-styling-with-style-expressions, keep-node-label-text-if-already-defined-when-a-style-is-applied, multiple-nodes-and-chaining-in-one-statement-2, multiple-nodes-and-chaining-in-one-statement, possibility-to-style-text-color-of-nodes-and-subgraphs-as-well-as-apply-classes-to-subgraphs, render-a-simple-flowchart-with-nodespacing-set-to-100, render-a-simple-flowchart-with-rankspacing-set-to-100, render-cylindrical-shape, render-multiline-texts, render-stadium-shape, render-subroutine-shape, set-text-color-of-nodes-and-links-according-to-styles-when-html-labels-are-disabled, set-text-color-of-nodes-and-links-according-to-styles-when-html-labels-are-enabled, should-be-able-to-style-default-node-independently, should-honor-minimum-edge-length-as-specified-by-the-user, should-not-slice-off-edges-that-are-to-the-left-of-the-left-most-vertex, should-redirect-a-collapsed-subgraph-using-the-handdrawn-look, should-render-a-flowchart-full-of-circles, should-render-a-flowchart-full-of-icons, should-render-a-flowchart-with-long-names-and-class-definitions, should-render-a-simple-flowchart-no-htmllabels, should-render-a-simple-flowchart-with-comments, should-render-a-simple-flowchart-with-diagrampadding-set-to-0, should-render-a-simple-flowchart-with-htmllabels, should-render-a-simple-flowchart-with-line-breaks, should-render-a-simple-flowchart-with-trapezoid-and-inverse-trapezoid-vertex-options, should-render-color-of-styled-nodes, should-render-escaped-without-html-labels, should-render-hexagons, should-render-labels-with-numbers-at-the-start, should-render-non-escaped-with-html-labels, should-render-styled-subgraphs, should-render-subgraphs-with-a-title-starting-with-a-digit, should-render-subgraphs, should-style-nodes-via-a-class, testing-the-label-width-in-percy, text-color-from-classes

**Other theme (71):**

_Theme `redux` (14)_

- `nightly-check/redux-color-subgraphs/` (4): 01-should-render-subgraphs-subgraph-containers, 02-should-render-nested-subgraph-containers, 03-should-render-collapsed-subgraph-containers, 04-should-render-user-styled-subgraph-containers
- `nightly-check/shape-themes/` (10): 21-connect-all-shapes-with-each-other, 22-connect-all-shapes-with-each-other, 23-connect-all-shapes-with-each-other, 24-connect-all-shapes-with-each-other, 25-connect-all-shapes-with-each-other, 26-connect-all-shapes-with-each-other, 27-connect-all-shapes-with-each-other, 28-connect-all-shapes-with-each-other, 29-connect-all-shapes-with-each-other, 30-connect-all-shapes-with-each-other

_Theme `redux-dark` (14)_

- `nightly-check/redux-color-subgraphs/` (4): 10-should-render-nested-subgraph-containers, 12-should-render-user-styled-subgraph-containers, 13-should-render-subgraphs-subgraph-containers, 15-should-render-collapsed-subgraph-containers
- `nightly-check/shape-themes/` (10): 31-connect-all-shapes-with-each-other, 32-connect-all-shapes-with-each-other, 33-connect-all-shapes-with-each-other, 34-connect-all-shapes-with-each-other, 35-connect-all-shapes-with-each-other, 36-connect-all-shapes-with-each-other, 37-connect-all-shapes-with-each-other, 38-connect-all-shapes-with-each-other, 39-connect-all-shapes-with-each-other, 40-connect-all-shapes-with-each-other

_Theme `neo` (10)_

- `nightly-check/shape-themes/` (10): 01-connect-all-shapes-with-each-other, 02-connect-all-shapes-with-each-other, 03-connect-all-shapes-with-each-other, 04-connect-all-shapes-with-each-other, 05-connect-all-shapes-with-each-other, 06-connect-all-shapes-with-each-other, 07-connect-all-shapes-with-each-other, 08-connect-all-shapes-with-each-other, 09-connect-all-shapes-with-each-other, 10-connect-all-shapes-with-each-other

_Theme `neo-dark` (10)_

- `nightly-check/shape-themes/` (10): 11-connect-all-shapes-with-each-other, 12-connect-all-shapes-with-each-other, 13-connect-all-shapes-with-each-other, 14-connect-all-shapes-with-each-other, 15-connect-all-shapes-with-each-other, 16-connect-all-shapes-with-each-other, 17-connect-all-shapes-with-each-other, 18-connect-all-shapes-with-each-other, 19-connect-all-shapes-with-each-other, 20-connect-all-shapes-with-each-other

_Theme `default` (4)_

- `nightly-check/icon/` (2): 01-should-render-registered-icons-on-theme-default, 02-should-render-icons-from-fontawesome-library-on-theme-default
- `visual-check/` (2): default-theme-legacy-graph, default-theme-new-syntax

_Theme `dark` (4)_

- `nightly-check/icon/` (2): 03-should-render-icons-from-fontawesome-library-on-theme-dark, 05-should-render-registered-icons-on-theme-dark
- `visual-check/` (2): dark-theme-legacy-graph, dark-theme-new-syntax

_Theme `forest` (4)_

- `nightly-check/icon/` (2): 04-should-render-registered-icons-on-theme-forest, 08-should-render-icons-from-fontawesome-library-on-theme-forest
- `visual-check/` (2): forest-theme-legacy-graph, forest-theme-new-syntax

_Theme `neutral` (4)_

- `nightly-check/icon/` (2): 09-should-render-icons-from-fontawesome-library-on-theme-neutral, 10-should-render-registered-icons-on-theme-neutral
- `visual-check/` (2): neutral-theme-legacy-graph, neutral-theme-new-syntax

_Theme `redux-dark-color` (4)_

- `nightly-check/redux-color-subgraphs/` (4): 09-should-render-subgraphs-subgraph-containers, 11-should-render-collapsed-subgraph-containers, 14-should-render-nested-subgraph-containers, 16-should-render-user-styled-subgraph-containers

_Theme `base` (3)_

- `nightly-check/icon/` (2): 06-should-render-icons-from-fontawesome-library-on-theme-base, 07-should-render-registered-icons-on-theme-base
- `visual-check/` (1): 63-title-on-subgraphs-should-be-themeable

## 1. Part 1: how the pie comparison works

### 1.1 Files

| File                                  | Role                                                                                                                                                                                                   |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `tools/port/geometry/pie-geometry.ts` | The geometry comparison (`pnpm port:geometry-pie`).                                                                                                                                                    |
| `tools/port/geometry/pie-scope.ts`    | Colour cascade check, three pies in one page (`pnpm port:scope-pie`). Not geometry.                                                                                                                    |
| `tools/port/reference.ts`             | Loads reference modules through Vite SSR for the **exporters** (`port:diff`). The geometry harness does not use it.                                                                                    |
| `tools/port/inputs.ts`                | Collects inputs for `port:diff`: e2e fixtures, spec string literals, docs mermaid blocks, `cases.ts`. It has a `flowchart` detector (`^\s*(graph\|flowchart)`). The geometry harness does not use it.  |
| `tools/port/README.md`                | Lists the `port:*` commands. Says `port:geometry-pie` renders the pie fixtures with both sides and compares them in Chromium.                                                                          |
| `package.json`                        | `port:geometry-pie` = `tsx tools/port/geometry/pie-geometry.ts`; `port:scope-pie` = `tsx tools/port/geometry/pie-scope.ts`; also `port:diff`, `port:status`, `port:check-header`, `port:text-fixture`. |

### 1.2 The reference, in Chromium

- **Bundle, not Vite.** The page imports `/dist/mermaid.esm.mjs` from `packages/mermaid/dist` (esbuild output of `pnpm build`). A small Node `http` server serves `/` (the harness page), `/dist/` and `/fonts/`.
- **Config** (source, `pie-geometry.ts`): `{ htmlLabels: false, theme: 'redux-color', fontFamily: '"Lexica Ultralegible"', securityLevel: 'strict' }`, applied as `mermaid.initialize({ startOnLoad: false, ...config })` before every `mermaid.render(id, text)`. The id is `ref-<fixture basename>`.
- **Font.** An `@font-face` for "Lexica Ultralegible" from `crates/ferrigura-text/fonts/LexicaUltralegible-Regular.ttf`. Only the Regular face is declared. The harness awaits `document.fonts.load` for each family named in the SVG, then `document.fonts.ready`, before measuring. Bold and italic faces are not declared (source), so a bold label would measure with a fallback face.
- **Browser.** Playwright Chromium launched with `--font-render-hinting=none` (fractional glyph advances, as on macOS), viewport 1200 x 900. The comment says WebKit reports kerned widths differently, so this is a Chromium check only.

### 1.3 Ferrigura's SVG

- `cargo build -q -p ferrigura --example render` (run by the harness), then `target/debug/examples/render` with the diagram text on stdin, SVG on stdout (source: `crates/ferrigura/examples/render.rs`).
- Options of the example: `--config <file.json>` (the `initialize` object), `--id <svg id>`, `--scheme light|dark|auto`. **The geometry harness passes none of them**, so it relies on ferrigura's defaults matching `referenceConfig` (see 1.8).

### 1.4 One page

- A single Chromium page holds both sides, but they are measured **one at a time**: `geometry(svgText)` replaces the content of `#stage` with the SVG, waits for fonts, and reads the geometry. The reference SVG and the ferrigura SVG for the same fixture are measured in turn.
- The scope check is the one place where several SVGs sit in one DOM at the same time (three pies, ids `pie-auto`, `pie-dark`, `pie-custom`).
- `--images <dir>` builds a side-by-side SVG (reference left, ferrigura right, 560 px each) and screenshots it per fixture.

### 1.5 Geometry extracted (pie)

| Quantity                 | Selector and method                                                                                                                        | Compared                     |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------- |
| viewBox                  | `svg` attribute `viewBox`                                                                                                                  | 4 numbers, 0.05 px           |
| Outer circle             | `.pieOuterCircle`: centre `cx, cy` mapped through the CTM; radius = bbox width / 2                                                         | 3 numbers, 0.05 px           |
| Slice angles             | `path.pieCircle`: `d` parsed with `M x,y A ... x,y`; both endpoints mapped by the CTM and turned into angles about the centre with `atan2` | start, end: 1e-5 rad         |
| Slice label box          | `text.slice`: `getBBox()` corners mapped by the CTM, then min/max                                                                          | x, y, width, height, 0.05 px |
| Title box                | `text.pieTitleText` (same method)                                                                                                          | 4 numbers, 0.05 px           |
| Legend                   | `g.legend` `transform` translate; `rect` and `text` boxes                                                                                  | 2 + 4 + 4 numbers, 0.05 px   |
| Max width, width, height | extracted (`maxWidth`, `width`, `height`)                                                                                                  | **not compared**             |

Colours are not part of the geometry comparison; they are checked by `port:scope-pie`.

### 1.6 Tolerance and output

- Angles 1e-5 rad; positions and sizes 0.05 px. The comments justify both: the reference writes path coordinates with three decimals (d3-path), so an endpoint is off by up to 0.0005 px, about 2.7e-6 rad at radius 185 px; Chromium lays text out in 1/64 px steps, so a label width can differ by a few hundredths of a pixel.
- Per fixture: `✓` or `✗`, the worst difference as a percentage of tolerance, and the first five problems. Summary line: `pie geometry: N fixtures, X within tolerance, Y outside`. Exit code 1 if any fixture is outside.
- JSON: `target/geometry/pie-geometry.json` (tolerances and results). `DUMP=<basename>` writes `target/geometry/dump-<name>.json` with both SVG strings and both geometries.
- The pie fixtures are the 24 files under `e2e/diagrams/pie`.

### 1.7 pie-scope (for completeness)

Three ferrigura pies with their own ids in one page, under a page stylesheet that paints every `path`, `text` and `rect` rebeccapurple/lime. For each diagram it compares the computed colours of the title, slice 0, its stroke and legend entry 0 with the values its own style declares, under `light` and `dark` emulated schemes, and with the style sheets disabled (presentation attributes must show the light set). It is a cascade check.

### 1.8 Gotchas for reuse

1. **No dist in the worktree.** `packages/mermaid/dist` does not exist here; the harness needs `pnpm build` (source: `.esbuild/build.ts`, `build:esbuild` also cleans the packages). The main checkout has a dist built at 11:57, which predates HEAD's commit time (12:17). Whether it matches HEAD is **unverified**. Runtime checks below used it.
2. **No `target/`** in the worktree; the cargo build and the harness write into it.
3. **Ids from basenames collide in flowcharts.** The pie harness uses `basename`. Two flowchart fixtures appear twice: `3-a-link-with-correct-arrowhead-to-a-subgraph.mmd` and `4-length-of-edges.mmd` (nightly-check and visual-check). Pie has no duplicates (0). Flowchart ids must come from the path relative to `e2e/diagrams/flowchart`.
4. **The ferrigura call throws.** `execFileSync` throws on a non-zero exit, and `render_with` returns an error for every flowchart (source). The pie loop has no `try`, so the first flowchart would abort the run. A flowchart harness must catch per fixture and report "not rendered yet" as its own class.
5. **Defaults.** Pie passes no config to ferrigura, so it assumes ferrigura's defaults equal `referenceConfig`. For flowcharts this is wrong by default: the reference applies per-diagram defaults (`theme: redux-color`, `look: neo`) that the harness config does not set. Ferrigura's config layers mention `look` and `theme` (`crates/ferrigura-model/src/config/layers.rs`), but no flowchart default is visible there. **Unverified** whether ferrigura applies per-diagram defaults.
6. **Ids are not passed to ferrigura.** Pairing by element id needs `--id` equal to the reference's `diagramId`. Without it the ids differ and the pairing has to fall back to logical ids.
7. **Font faces.** Only the Regular face is declared. The flowchart fixtures use bold in class-styled labels (e.g. `65-2-bold-text-from-classes`), so the harness needs Bold and Italic `@font-face` rules too.
8. **`htmlLabels` precedence.** The reference's `getEffectiveHtmlLabels` reads the root key only, with default `true` when unset (source: `packages/mermaid/src/config.ts` 320-325). The pie harness sets `htmlLabels: false` in `initialize`, so unset fixtures get SVG labels; a fixture that sets `htmlLabels: true` in its own front matter or directive still gets HTML labels, because the fixture config is applied on top of `initialize` (source; unverified at runtime for this fork). No fixture sets `flowchart.htmlLabels` (classifier), so that key is not a trap here.

## 2. The reference's flowchart DOM (for pairing and extraction)

### 2.1 Ids, classes and groups

Verified at runtime for three fixtures (`05-should-include-classes-on-the-edges`, `5a-…-htmllabels-is-false`, `9-should-render-subgraphs`) and from source.

| Element                | Selector                                                                       | Id or class convention                                                                                                                                                                                                                                  | Check                                                                |
| ---------------------- | ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| Root svg               | `svg`                                                                          | `id` = diagramId; `class="flowchart"`; `aria-roledescription="flowchart-v2"`; `viewBox`; `style="max-width: Npx"`; `width="100%"`                                                                                                                       | runtime                                                              |
| Node group             | `g.nodes > g`                                                                  | `id` = `<diagramId>-flowchart-<vertexId>-<counter>`; `class="node default"`; `transform="translate(x, y)"` (centre)                                                                                                                                     | runtime; source `flowDb.ts` 191, `render.ts` 86-92                   |
| Node shape             | `.label-container` inside the node group                                       | e.g. `rect class="basic label-container"` with `x, y, width, height`                                                                                                                                                                                    | runtime for rect; other shapes **unverified**                        |
| Node label             | `g.label` > `g` > `text` > `tspan.text-outer-tspan` > `tspan.text-inner-tspan` | `htmlLabels: false` gives SVG text                                                                                                                                                                                                                      | runtime; source `createText.ts`                                      |
| Edge path              | `g.edgePaths > path`                                                           | `id` = `<diagramId>-<edgeId>`, edge id `L_<start>_<end>_<n>` or the user's `e1@` id; `class` contains `flowchart-link`; `d`; `data-points` (base64 JSON of the layout points); `data-id`; `marker-end="url(#<diagramId>_flowchart-v2-pointEnd-margin)"` | runtime; source `edges.js` 837 and 860, `flowDb.ts` 316-324 and 1325 |
| Edge label             | `g.edgeLabels > g.edgeLabel > g.label[data-id=<edgeId>]`                       | `g.label` carries `transform`; inner `g > rect.background` and `text`                                                                                                                                                                                   | runtime                                                              |
| Cluster                | `g.clusters > g.cluster`                                                       | `id` = `<diagramId>-<subgraphId>` (`One` gave `<diagramId>-One`); anonymous subgraphs are `subGraph<n>` (`flowDb.ts` 734)                                                                                                                               | runtime for `One`; source `clusters.js` 320                          |
| Cluster rect and label | `g.cluster > rect`, `g.cluster-label`                                          | rect `x, y, width, height` in absolute coordinates; label `transform`                                                                                                                                                                                   | runtime                                                              |
| Terminal labels        | `g.edgeTerminals`                                                              | source `edges.js` 142-226                                                                                                                                                                                                                               | **unverified** (no fixture run)                                      |
| Markers                | `defs` children                                                                | ids `<diagramId>_flowchart-v2-<kind>[-margin]`                                                                                                                                                                                                          | runtime                                                              |

Two facts that matter for pairing:

- **Counters have gaps.** In `5a-…` the nodes are `flowchart-A-0`, `flowchart-B-1`, `flowchart-C-3` (runtime). Pair nodes by vertex id (the id without the diagram prefix, the `flowchart-` prefix and the trailing `-<counter>`), not by counter.
- **Edge labels carry the edge id.** The label's `data-id` is the edge id without the diagram prefix, so labels pair with edges by that id.

### 2.2 `data-points`

Each edge path has `data-points`, a base64 JSON array of `{x, y}` (runtime: `[{"x":88,"y":53},{"x":88,"y":89}]` for the first edge of `05`). These are the layout points before curve interpolation. They are the cleanest edge geometry to compare, with `d` as a second check.

### 2.3 Layout selection (source)

- `layout` is read from the root config. Registered names: `swimlane`, `cose-bilkent`, `elk` (algorithm `elk.layered`), and `elk.stress`, `elk.force`, `elk.mrtree`, `elk.sporeOverlap`, `elk.box`, `elk.rectpacking` (`render.ts` 60-70; `elk/algorithms.ts` 5-12).
- Anything else is **not an error**: `getRegisteredLayoutAlgorithm` falls back to `elk` with a warning (`render.ts`, around 115-130). In this fork `dagre` is gone, so `layout: dagre` renders as ELK layered. No fixture names `dagre` (classifier).
- ELK in ferrigura: layered only; M4 reports bit-exact coordinates with elkjs 0.9.3 for layered (`docs-port/STATUS.md`, M4 handover). `elk.box` and `elk.rectpacking` fall back to layered in the port (`docs-port/elk/PLAN.md`).

### 2.4 Look changes geometry (runtime)

Same fixture, same config, only `look` changed (`lookcmp.mjs`):

| Fixture                                                | Node box neo                                                       | Node box classic         | viewBox neo  | viewBox classic | Edge label boxes                       |
| ------------------------------------------------------ | ------------------------------------------------------------------ | ------------------------ | ------------ | --------------- | -------------------------------------- |
| `nightly-check/05-should-include-classes-on-the-edges` | rect 152 x 41                                                      | rect 180 x 47            | 4 4 168 300  | 4 4 196 324     | same                                   |
| `visual-check/9-should-render-subgraphs`               | rect 152 x 40                                                      | rect 180 x 46            | 4 4 216 187  | 4 4 244 199     | same (no labels); cluster rect differs |
| `visual-check/5a-…`                                    | rect 152 x 41                                                      | rect 180 x 47            | 4 4 168 341  | 4 4 196 359     | same                                   |
| `pr-check/1-shapes-layout-default`                     | first node rect 152 x 41 (the rect of B is 150 x 47 in both looks) | first node rect 180 x 47 | 4 4 243 1417 | 4 4 213 1310    | same                                   |

The source of the difference: per-shape label padding under `look === 'neo'`, for example `squareRect.ts` lines 9-10 (`16` and `12` under neo; `nodePadding * 2` and `nodePadding` otherwise), and `subroutine.ts` 18-19, `linedCylinder.ts` 70-71, plus edge margin markers under neo (`edges.js` 883, 946) and cluster drawing (`clusters.js` 478). Ferrigura must implement the neo sizes for every shape it draws, because neo is the default for flowcharts.

## 3. Proposal: a flowchart comparison that reuses the pie harness

### 3.1 Shape of the tool

- A new `tools/port/geometry/flowchart-geometry.ts` with the same skeleton as the pie file: static server, one Chromium page, `harness` with `window.geometry(svg)`, the same tolerance constants, `compare()` returning `worst` and `problems`, JSON output under `target/geometry/`. Move the shared bits (server, font loading, tolerance, compare helper) into `tools/port/geometry/common.ts` when the second harness arrives, not before.
- Inputs: the fixtures under `e2e/diagrams/flowchart`, keyed by path relative to that folder (no basenames). `inputs.ts` (`collectInputs('flowchart')`) would add docs and spec literals; start with e2e only.
- Per fixture: catch errors from the ferrigura call and report them as "not rendered" (class of its own), not as failures. Only fixtures in the current stage are expected to render.

### 3.2 Reference

- Same `referenceConfig` as pie (`htmlLabels: false`, `theme: 'redux-color'`, `fontFamily: '"Lexica Ultralegible"'`, `securityLevel: 'strict'`), **without `look`**: the reference's flowchart default (`neo`) must apply, and ferrigura has to reproduce it. Set `look` explicitly only in a second run with `look: 'classic'` if that is wanted.
- Id: `ref-` + a slug of the relative path (for example `ref-nightly-check-05-should-include-classes-on-the-edges`). Ids start with a letter and use `[A-Za-z0-9_-]`, as ferrigura requires.

### 3.3 Ferrigura

- Pass `--id <same id>` so the element ids match the reference's. Pass `--config` only if the config differs from the reference's; the reference's flowchart defaults should be reproduced by ferrigura, not configured in the harness.

### 3.4 What to extract, per element

| Element    | Key (pairs the two sides)                               | Quantities                                                                                                                                                  | Tolerance         |
| ---------- | ------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------- |
| Root       | one per fixture                                         | viewBox (4); `max-width` in px from `style`                                                                                                                 | 0.05 px           |
| Node       | vertex id (strip prefix, `flowchart-`, trailing `-<n>`) | centre from `transform` (CTM-mapped); `.label-container` box (CTM-mapped bbox: x, y, w, h); `g.label` box                                                   | 0.05 px           |
| Edge       | edge id (strip `<diagramId>-`)                          | `data-points` decoded, compared point by point (count must match); `d` sampled with `getPointAtLength` at N evenly spaced fractions, in viewBox coordinates | 0.05 px per point |
| Edge arrow | edge id                                                 | the marker kind from `marker-start` / `marker-end` (name only, ids stripped)                                                                                | exact string      |
| Edge label | `data-id`                                               | `g.edgeLabel` transform; `rect.background` box; text box                                                                                                    | 0.05 px           |
| Cluster    | subgraph id (strip `<diagramId>-`)                      | `rect` box (x, y, w, h); `g.cluster-label` transform                                                                                                        | 0.05 px           |
| Counts     | per class                                               | nodes, edges, labels, clusters: the two counts must match                                                                                                   | exact             |

- Angles do not apply. The pie tolerances carry over. The M4 bit-exact claim for ELK coordinates suggests the failures will come from text measurement (label sizes feed the layout), so keep a separate "label size" class in the output to see it.
- `data-points` requires ferrigura to write that attribute too. That is a contract to decide in M5 (**open question 4**). Without it, compare the sampled `d` only.

### 3.5 Pairing rules

1. Match the two sets of node keys, edge keys, label keys and cluster keys. A key missing on one side is a failure of its own kind.
2. Node, edge and cluster keys are derived by stripping the diagram id; do not compare counters (section 2.1).
3. Duplicated edge ids are not expected (ids are `L_<start>_<end>_<n>`); if one appears, pair by order and report it.

### 3.6 Output

- One line per fixture: `✓`/`✗`, worst difference as a percentage of tolerance, and the first five problems, as the pie harness does. A class summary: fixtures within tolerance, outside, not rendered, not comparable (the blocker categories of section 5.1).
- JSON in `target/geometry/flowchart-geometry.json`; `--images <dir>` for side-by-side PNGs.

### 3.7 What the pie harness does not do that the flowchart one needs

- It does not compare `max-width` (flowcharts use `width="100%"` and `style="max-width: …"`, so the rendered width is only in the viewBox and the style).
- It declares only the Regular face (section 1.8, point 7).
- It does not pass `--id` (section 1.8, point 6).
- It aborts on the first ferrigura error (section 1.8, point 4).

## 4. Part 2: the fixtures

### 4.1 Where they are

Folders (classifier): `nightly-check` 195, `visual-check` 146, `pr-check` 11.

Subfolders:

| Folder                                   | Files |
| ---------------------------------------- | ----: |
| `nightly-check/handdrawn`                |    50 |
| `nightly-check/shape-alias`              |    42 |
| `nightly-check/shape-themes`             |    40 |
| `nightly-check/redux-color-subgraphs`    |    16 |
| `nightly-check/icon`                     |    10 |
| `nightly-check/v2`                       |     7 |
| `nightly-check/classic`                  |     3 |
| `nightly-check/elk-container-algorithms` |     3 |

- Keyword: flowchart 259, graph 93 (no `swimlane`, `sequenceDiagram` or other type). `flowchart-elk` does not exist in this fork (source: no matches for it in `packages/mermaid/src`).
- Direction: TD 129, TB 87, LR 83, (none) 46, BT 4, RL 3.
- Front matter: present in 347; with a `config` block in 256; absent in 5 (listed in Appendix B). `description` is the only other top-level key in most (347); `title` is top-level in 6 files.
- Init directives: `%%{init}%%` in 23 files. No `%%{wrap}%%`.
- Files with any config (front matter or directive): 261.

### 4.2 Config keys used (front matter and directives, count of files)

| Key path                               | Files |
| -------------------------------------- | ----: |
| `htmlLabels`                           |   113 |
| `look`                                 |    90 |
| `theme`                                |    76 |
| `fontFamily`                           |    70 |
| `securityLevel`                        |    54 |
| `flowchart.titleTopMargin`             |    20 |
| `logLevel`                             |    14 |
| `layout`                               |     7 |
| `flowchart.useMaxWidth`                |     6 |
| `flowchart.subGraphTitleMargin.bottom` |     5 |
| `flowchart.subGraphTitleMargin.top`    |     5 |
| `flowchart.nodeSpacing`                |     4 |
| `flowchart.rankSpacing`                |     4 |
| `markdownAutoWrap`                     |     4 |
| `flowchart.diagramPadding`             |     3 |
| `flowchart.minNodeWidth`               |     3 |
| `flowchart.inheritDir`                 |     2 |
| `flowchart.wrappingWidth`              |     2 |
| `handDrawnSeed`                        |     2 |
| `themeVariables.darkMode`              |     2 |
| `themeVariables.primaryColor`          |     2 |
| `themeVariables.titleColor`            |     2 |
| `elk.keepEntryNodeOnTop`               |     1 |
| `flowchart.curve`                      |     1 |

### 4.3 Values the reference reads (effective)

Layout (the raw value is what the fixture writes; the effective value is what the fork uses):

| Layout (raw)                  | Files |
| ----------------------------- | ----: |
| `elk (unset, schema default)` |   345 |
| `elk (explicit)`              |     5 |
| `elk.box`                     |     1 |
| `elk.rectpacking`             |     1 |

- Explicit `elk.box` and `elk.rectpacking` appear once each; no fixture uses `elk.stress`, `elk.mrtree`, `elk.force`, `elk.sporeOverlap`, `swimlane`, `cose-bilkent` or `dagre`. The ELK plan (`docs-port/elk/PLAN.md` line 242) counts seven fixtures with these layouts; the discrepancy is **unverified** (its input set may include other folders).

Look (effective in the reference: unset means `neo` for flowcharts; the default appearance is 231 files):

| Look (raw)        | Files |
| ----------------- | ----: |
| `classic (unset)` |   262 |
| `handDrawn`       |    50 |
| `neo`             |    40 |

Theme (effective: unset means `redux-color` for flowcharts):

| Theme (raw)        | Files |
| ------------------ | ----: |
| `default (unset)`  |   276 |
| `redux`            |    14 |
| `redux-dark`       |    14 |
| `neo`              |    10 |
| `neo-dark`         |    10 |
| `base`             |     4 |
| `default`          |     4 |
| `dark`             |     4 |
| `forest`           |     4 |
| `neutral`          |     4 |
| `redux-color`      |     4 |
| `redux-dark-color` |     4 |

Root `htmlLabels` (unset means `true` in the reference unless the harness sets `false`):

| Value   | Files |
| ------- | ----: |
| `unset` |   239 |
| `true`  |    66 |
| `false` |    47 |

- `flowchart.htmlLabels`: no fixture. `curve` at root: no fixture (the reference reads `flowchart.curve`, `flowDb.ts` 1340). `flowchart.defaultRenderer`: no fixture (not read in this fork).
- `flowchart.curve`: 1 (`rounded`). `flowchart.useMaxWidth`: 6 files (the reference reads `conf.useMaxWidth`, `flowRenderer-v3-unified.ts` 51).
- `fontFamily`: 70 files, all `courier`. Ferrigura bundles Lexica Ultralegible and Atkinson Hyperlegible Mono only (`crates/ferrigura-text/fonts`), and `crates/ferrigura-text/src/lib.rs` says fallback fonts are not part of the crate yet. How ferrigura measures an unknown family is **unverified**.
- `securityLevel`: loose 53, sandbox 1. Not expected to change geometry (unverified).
- `themeVariables`: 2 files. `themeCSS`: 0. `markdownAutoWrap`: 4 files. `logLevel` in 14 files (no geometric effect).
- `elk.keepEntryNodeOnTop`: 1 file. Other `elk.*` keys: none.

### 4.4 Shapes

Classic delimiters, per file (the regex is a heuristic: text with a parenthesis directly after a word, outside quotes, can match):

| Shape        | Files |
| ------------ | ----: |
| rect         |   129 |
| round        |    80 |
| stadium      |    17 |
| subroutine   |     6 |
| cylinder     |    11 |
| circle       |    12 |
| doublecircle |     4 |
| rhombus      |    63 |
| hexagon      |    13 |
| asymmetric   |    11 |
| lean-trap    |    14 |

- Files that use only classic syntax (no `@{ }`): 243. Files that use `@{ }` nodes: 109 (133 distinct names over 109 files).
- The alias folder `nightly-check/shape-alias` (42 files) is `@{ shape: … }` with aliases of one shape per file.

### 4.5 Subgraphs

| Subgraphs                | Files |
| ------------------------ | ----: |
| no subgraphs             |   251 |
| flat (depth 1)           |    80 |
| nested (depth 2 or more) |    21 |

- Subgraph-level `direction` lines in 14 files.

### 4.6 Edge labels, terminal labels, edge styles

- Pipe labels `|text|`: 75. Text labels `-- text -->`, `== text ==>`, `-. text .->` (and `<--` forms): 22. Either: 95. None: 257.
- Terminal-label candidates (a quoted string directly next to an arrow): 11. **These are heuristic and not checked by eye**; `-- "text" -->` can match as well.
- Invisible links `~~~`: 3; dotted `-.`: 3; thick `==`: 15; ampersand chaining `A & B`: 12.

### 4.7 Markdown, icons, images, links, styling, ids

- Markdown strings (backtick inside double quotes): 21 files, 46 strings in total.
- Icons (`fa:`, `fab:`, `aws:`, `icon:`): 54 files. Images (`<img>`, `img:`): 2 files.
- `click`, `link:`, `href`: 15 files.
- Styling: any 62; `classDef` 38; `class` statement 28; `style` 31; `linkStyle` 13; `:::` shorthand 6.
- Edge ids (`e1@-->`): 4 files. Per-edge `@{}` options: 3 files (`animation` in 2, `curve` in 1).
- `accTitle` / `accDescr`: 8 files. Comment lines (`%%`): 23 files.

### 4.8 Method and confidence

- Front matter parsed with js-yaml 4.3.0 (JSON schema); init directives parsed the same way. Parse errors: 0 (`parse-errors.txt`).
- Config values are the values as written, merged in order (front matter, then directives). The effective values in section 4.3 apply the reference's rules (source: `flowchart` defaults in the schema, `getRegisteredLayoutAlgorithm`, `getEffectiveHtmlLabels`).
- Regex features (shapes, edge labels, terminal labels, icons, styling) are heuristics. The 13 stage-1 files and the 7 alias files were read by eye; the other staged files were not all read.

## 5. Staging

### 5.1 Blockers that apply to every stage

| Blocker                           | Files | Why                                                                                  |
| --------------------------------- | ----: | ------------------------------------------------------------------------------------ |
| `htmlLabels: true` set explicitly |    66 | HTML labels (foreignObject) in the reference; ferrigura has no HTML label mode.      |
| `fontFamily: courier`             |    70 | Courier metrics are not available in ferrigura-text (section 4.3). Needs a decision. |
| `look: handDrawn`                 |    50 | rough.js sketching; not in the port.                                                 |
| `elk.box`, `elk.rectpacking`      |     2 | Not ported; the port falls back to layered.                                          |
| **Union**                         |   143 |                                                                                      |

Not hard blockers, but outside the default appearance: a theme other than redux-color or unset (71 files, section 0), `@{ }` shape names outside the classic set, markdown strings, icons and images, subgraph directions and depth above 2.

### 5.2 Stage 1: classic shapes, no subgraphs, no edge labels (13 files)

Criteria: the default appearance (look neo, theme redux-color, set or unset), classic shapes, no subgraphs, no markdown, no icons, ELK layered, plus the blockers above (`htmlLabels` not true, no handDrawn, no courier) and no edge labels (edge labels are stage 2). Ferrigura must draw neo for these shapes.

Files:

- `nightly-check/05-should-include-classes-on-the-edges.mmd` (9 lines; flowchart TD; shapes none; theme default (unset); htmlLabels unset; look classic (unset))
- `nightly-check/06-should-render-a-flowchart-with-title.mmd` (10 lines; flowchart LR; shapes none; theme default (unset); htmlLabels unset; look classic (unset))
- `nightly-check/07-render-with-stylized-arrows.mmd` (13 lines; flowchart LR; shapes none; theme default (unset); htmlLabels unset; look classic (unset))
- `nightly-check/6088-5-should-handle-diamond-shape-intersections.mmd` (12 lines; flowchart LR; shapes rhombus; theme default (unset); htmlLabels unset; look classic (unset))
- `nightly-check/6647-should-keep-node-order-when-using-layout-unless-it-would-add-crossings.mmd` (12 lines; flowchart TB; shapes none; theme default (unset); htmlLabels unset; look classic (unset))
- `pr-check/1-shapes-layout-default.mmd` (37 lines; flowchart TD; shapes asymmetric, circle, cylinder, doublecircle, hexagon, lean-trap, rect, rhombus, round, stadium, subroutine; theme default (unset); htmlLabels unset; look classic (unset))
- `pr-check/4-classdef-style.mmd` (14 lines; flowchart TD; shapes rect; theme default (unset); htmlLabels unset; look classic (unset))
- `visual-check/1433-should-render-a-titled-flowchart-with-titletopmargin-set-to-0.mmd` (13 lines; flowchart TD; shapes none; theme default (unset); htmlLabels unset; look classic (unset))
- `visual-check/3192-it-should-be-possible-to-render-flowcharts-with-invisible-edges.mmd` (11 lines; flowchart TD; shapes none; theme default (unset); htmlLabels unset; look classic (unset))
- `visual-check/34-testing-the-label-width-in-percy.mmd` (8 lines; graph TD; shapes rect; theme default (unset); htmlLabels unset; look classic (unset))
- `visual-check/36-should-render-escaped-without-html-labels.mmd` (11 lines; graph TD; shapes rect; theme default (unset); htmlLabels false; look classic (unset))
- `visual-check/5-should-render-escaped-without-html-labels.mmd` (11 lines; flowchart TD; shapes rect; theme default (unset); htmlLabels false; look classic (unset))
- `visual-check/should-render-a-stadium-shaped-node.mmd` (10 lines; flowchart TB; shapes rect, stadium; theme default (unset); htmlLabels unset; look classic (unset))

Things these files exercise, read by eye: all eleven classic shapes in `pr-check/1-shapes-layout-default`; chains and `&` fan-out (`6647`, `6088-5`); invisible links and arrow heads (`07`, `3192`); classDef and inline `style` (`pr-check/4`); `titleTopMargin` (`1433`, `3192`, `6088-5`); front matter `title` (`06`); escaped HTML with `htmlLabels: false` (`5`, `36`).

### 5.3 Stage 1a: shape-alias fixtures (7 files)

`@{ shape: … }` with names that map to classic shapes (no classic-syntax nodes, no edges), default appearance. This is the only way to reach about 30 without the courier decision. It adds alias handling in the shape parser, which M3 covers.

- `nightly-check/shape-alias/01-all-process-rect-proc-rectangle-should-render-same-shape.mmd` (shapes proc, process, rect, rectangle)
- `nightly-check/shape-alias/03-all-stadium-pill-terminal-should-render-same-shape.mmd` (shapes pill, stadium, terminal)
- `nightly-check/shape-alias/04-all-fr-rect-subproc-subprocess-framed-rectangle-subroutine-should-render-same-shape.mmd` (shapes fr-rect, framed-rectangle, subproc, subprocess, subroutine)
- `nightly-check/shape-alias/05-all-db-database-cylinder-cyl-should-render-same-shape.mmd` (shapes cyl, cylinder, database, db)
- `nightly-check/shape-alias/06-all-diam-decision-diamond-should-render-same-shape.mmd` (shapes decision, diam, diamond)
- `nightly-check/shape-alias/07-all-hex-hexagon-prepare-should-render-same-shape.mmd` (shapes hex, hexagon, prepare)
- `nightly-check/shape-alias/12-all-dbl-circ-double-circle-should-render-same-shape.mmd` (shapes dbl-circ, double-circle)

### 5.4 Stage 1b: courier fixtures (11 files)

Same criteria as stage 1 (default appearance, alias allowed), but with `fontFamily: courier`. Enter this stage after the font decision (open question 1).

- `pr-check/12-should-render-a-flowchart-with-long-names-and-class-definitions.mmd`
- `visual-check/13-should-render-color-of-styled-nodes.mmd`
- `visual-check/18-chaining-of-nodes.mmd`
- `visual-check/19-multiple-nodes-and-chaining-in-one-statement.mmd`
- `visual-check/25-handle-link-click-events-link-anchor-mailto-other-protocol-script.mmd`
- `visual-check/29-apply-default-class-to-all-nodes-which-do-not-have-another-class-assigned-htmllabels-disabled.mmd`
- `visual-check/31-should-not-slice-off-edges-that-are-to-the-left-of-the-left-most-vertex.mmd`
- `visual-check/5-should-style-nodes-via-a-class.mmd`
- `visual-check/6-should-render-a-flowchart-full-of-circles.mmd`
- `visual-check/8-should-render-labels-with-numbers-at-the-start.mmd`
- `visual-check/stadium-tall-labels.mmd`

Stage 1 + 1a + 1b: **31 files**.

### 5.5 Stage 2: subgraphs and edge labels (28 new files; 25 flat, 3 nested)

Stage 2 adds subgraphs of depth 1 and 2 (no subgraph direction) and edge labels, on top of stage 1 and 1a. Courier, handDrawn and htmlLabels-true stay out.

- `nightly-check/3-a-link-with-correct-arrowhead-to-a-subgraph.mmd` (subgraphs 1, depth 1; pipe false; text false; shapes circle)
- `nightly-check/4-length-of-edges.mmd` (subgraphs 0, depth 0; pipe true; text false; shapes none)
- `nightly-check/6088-6-should-handle-diamond-shape-intersections.mmd` (subgraphs 1, depth 1; pipe false; text false; shapes rhombus)
- `nightly-check/7213-should-render-edges-with-right-angles-not-curves.mmd` (subgraphs 2, depth 1; pipe false; text false; shapes none)
- `nightly-check/8292-elk-edge-label-jog.mmd` (subgraphs 0, depth 0; pipe false; text true; shapes rect, rhombus)
- `nightly-check/8326-should-merge-repeated-subgraph-id-with-nested-subgraph.mmd` (subgraphs 3, depth 2; pipe false; text false; shapes none)
- `nightly-check/8326-should-merge-repeated-subgraph-ids.mmd` (subgraphs 2, depth 1; pipe false; text false; shapes none)
- `nightly-check/recursive-flow-with-keepentrynodeontop-false-keeps-the-default-layout-7827.mmd` (subgraphs 0, depth 0; pipe true; text false; shapes none)
- `nightly-check/redux-color-subgraphs/05-should-render-subgraphs-subgraph-containers.mmd` (subgraphs 5, depth 1; pipe false; text false; shapes cylinder, rect)
- `nightly-check/redux-color-subgraphs/06-should-render-nested-subgraph-containers.mmd` (subgraphs 4, depth 2; pipe false; text false; shapes rect)
- `nightly-check/redux-color-subgraphs/08-should-render-user-styled-subgraph-containers.mmd` (subgraphs 2, depth 1; pipe false; text false; shapes rect)
- `nightly-check/should-keep-the-entry-node-on-top-of-a-recursive-flow-nested-in-a-subgraph-with-keepentrynodeontop-true-7827.mmd` (subgraphs 1, depth 1; pipe true; text false; shapes stadium)
- `nightly-check/should-keep-the-entry-node-on-top-when-a-flow-recurses-with-keepentrynodeontop-true-7827.mmd` (subgraphs 0, depth 0; pipe true; text false; shapes none)
- `nightly-check/should-merge-edges-within-subgraphs-when-mergeedges-is-true-7659.mmd` (subgraphs 2, depth 1; pipe false; text false; shapes none)
- `nightly-check/sub-graphs.mmd` (subgraphs 1, depth 1; pipe false; text true; shapes circle, rect, rhombus)
- `pr-check/3-layout-elk-subgraphs.mmd` (subgraphs 3, depth 1; pipe false; text false; shapes cylinder, rect)
- `pr-check/8368-elk-label-parallel-edge.mmd` (subgraphs 0, depth 0; pipe true; text false; shapes rect, rhombus)
- `visual-check/3-a-link-with-correct-arrowhead-to-a-subgraph.mmd` (subgraphs 1, depth 1; pipe false; text false; shapes circle)
- `visual-check/35-should-honor-minimum-edge-length-as-specified-by-the-user.mmd` (subgraphs 0, depth 0; pipe true; text true; shapes none)
- `visual-check/4-length-of-edges.mmd` (subgraphs 0, depth 0; pipe true; text false; shapes none)
- `visual-check/4b-labeled-open-edges-should-not-have-arrowheads.mmd` (subgraphs 0, depth 0; pipe false; text true; shapes none)
- `visual-check/5059-should-render-when-subgraph-contains-only-subgraphs-has-link-to-outside-and-itself-is-part-of-a-link.mmd` (subgraphs 3, depth 2; pipe false; text false; shapes none)
- `visual-check/5064-should-render-when-subgraph-child-has-links-to-outside-node-and-subgraph.mmd` (subgraphs 1, depth 1; pipe false; text false; shapes none)
- `visual-check/5a-should-render-flowchart-with-edge-labels-centered-when-htmllabels-is-false.mmd` (subgraphs 0, depth 0; pipe true; text false; shapes rect)
- `visual-check/7213-should-render-edges-with-rounded-curve-right-angles-with-rounded-corners.mmd` (subgraphs 0, depth 0; pipe true; text false; shapes rect, rhombus)
- `visual-check/should-keep-the-true-entry-on-top-when-a-back-edge-feeds-it-79.mmd` (subgraphs 0, depth 0; pipe true; text false; shapes rect, rhombus)
- `visual-check/should-render-labeled-self-loops.mmd` (subgraphs 0, depth 0; pipe true; text false; shapes rect)
- `visual-check/subgraph-titles-wider-than-children.mmd` (subgraphs 3, depth 1; pipe false; text false; shapes rect)

### 5.6 Rest (293 files)

Each file is in exactly one bucket (first matching reason, in the order below). The partition of all 352 files is: stage 1 13, stage 1a 7, stage 1b 11, stage 2 new 28, rest 293.

**html labels set to true explicitly** (66)

- `nightly-check/handdrawn/` (13): apply-class-called-default-on-node-called-default, apply-default-class-to-all-nodes-which-do-not-have-another-class-assigned-htmllabels-enabled, fontawesome-icons-in-edge-labels-2, fontawesome-icons-in-edge-labels-3, fontawesome-icons-in-edge-labels-4, fontawesome-icons-in-edge-labels, handle-styling-for-all-node-shapes, handle-styling-with-style-expressions, set-text-color-of-nodes-and-links-according-to-styles-when-html-labels-are-enabled, should-be-able-to-style-default-node-independently, should-render-a-simple-flowchart-with-htmllabels, should-render-non-escaped-with-html-labels, text-color-from-classes
- `nightly-check/v2/` (2): 05-should-wrap-edge-labels-markdownautowrap-true-htmllabels-true, 07-should-wrap-edge-labels-markdownautowrap-false-htmllabels-true
- `pr-check/` (3): 52-handle-nested-subgraphs-in-several-levels, 6-markdown-label-html-true, 60-handle-styling-for-all-node-shapes
- `visual-check/` (48): 2-should-render-a-simple-flowchart-with-htmllabels, 2050-handling-of-different-rendering-direction-in-subgraphs, 2388-handling-default-in-the-node-name, 26-set-text-color-of-nodes-and-links-according-to-styles-when-html-labels-are-enabled, 28-apply-default-class-to-all-nodes-which-do-not-have-another-class-assigned-htmllabels-enabled, 2824-clipping-of-edges, 37-should-render-non-escaped-with-html-labels, 50-handle-nested-subgraphs-in-reverse-order, 53-handle-nested-subgraphs-with-edges-in-and-out, 54-handle-nested-subgraphs-with-outgoing-links, 55-handle-nested-subgraphs-with-outgoing-links-2, 56-handle-nested-subgraphs-with-outgoing-links-3, 57-handle-nested-subgraphs-with-outgoing-links-2, 57-handle-nested-subgraphs-with-outgoing-links-4, 57-x-handle-nested-subgraphs-with-outgoing-links-5, 58-handle-styling-with-style-expressions-2, 58-handle-styling-with-style-expressions, 5824-should-be-able-to-render-string-and-markdown-labels, 59-handle-styling-of-subgraphs-and-links, 6-should-render-non-escaped-with-html-labels, 60-handle-styling-for-all-node-shapes-v2, 61-fontawesome-icons-in-edge-labels-2, 61-fontawesome-icons-in-edge-labels, 62-should-render-styled-subgraphs, 63-title-on-subgraphs-should-be-themeable, 6369-edge-color-should-affect-arrow-head, 65-1-text-color-from-classes, 65-2-bold-text-from-classes, 65-3-bigger-font-from-classes, 66-apply-class-called-default-on-node-called-default, 66-more-nested-subgraph-cases-tb, 67-more-nested-subgraph-cases-rl, 67-should-be-able-to-style-default-node-independently, 68-more-nested-subgraph-cases-bt, 69-more-nested-subgraph-cases-lr, 6a-should-render-complex-html-in-labels-with-sandbox-security, 70-handle-nested-subgraph-cases-tb-link-out-and-link-between-subgraphs, 71-handle-nested-subgraph-cases-rl-link-out-and-link-between-subgraphs, 72-handle-nested-subgraph-cases-bt-link-out-and-link-between-subgraphs, 74-handle-labels-for-multiple-edges-from-and-to-the-same-couple-of-nodes, 76-handle-unicode-encoded-character-with-html-labels-true, issue-4648-nested-subgraph-with-external-connection, issue-4648-sibling-subgraphs-with-different-directions-and-external-connections, new-line-in-node-and-formatted-edge-label, sub-graphs-and-markdown-strings, with-formatting-in-a-node, with-styling-and-classes, wrapping-long-text-with-a-new-line

**layout elk.box or elk.rectpacking (not ported)** (2)

- `nightly-check/elk-container-algorithms/` (2): elk-box-layout-with-edges, elk-rectpacking-layout-with-edges

**look handDrawn (rough.js, not ported)** (37)

- `nightly-check/handdrawn/` (37): 01-fdh39-should-render-a-flowchart-when-usemaxwidth-is-false, 02-fdh38-should-render-a-flowchart-when-usemaxwidth-is-true-default, 03-fdh49-should-add-edge-animation, apply-default-class-to-all-nodes-which-do-not-have-another-class-assigned-htmllabels-disabled, chaining-of-nodes, handle-link-click-events-link-anchor-mailto-other-protocol-script, keep-node-label-text-if-already-defined-when-a-style-is-applied, multiple-nodes-and-chaining-in-one-statement-2, multiple-nodes-and-chaining-in-one-statement, possibility-to-style-text-color-of-nodes-and-subgraphs-as-well-as-apply-classes-to-subgraphs, render-a-simple-flowchart-with-nodespacing-set-to-100, render-a-simple-flowchart-with-rankspacing-set-to-100, render-cylindrical-shape, render-multiline-texts, render-stadium-shape, render-subroutine-shape, set-text-color-of-nodes-and-links-according-to-styles-when-html-labels-are-disabled, should-honor-minimum-edge-length-as-specified-by-the-user, should-not-slice-off-edges-that-are-to-the-left-of-the-left-most-vertex, should-redirect-a-collapsed-subgraph-using-the-handdrawn-look, should-render-a-flowchart-full-of-circles, should-render-a-flowchart-full-of-icons, should-render-a-flowchart-with-long-names-and-class-definitions, should-render-a-simple-flowchart-no-htmllabels, should-render-a-simple-flowchart-with-comments, should-render-a-simple-flowchart-with-diagrampadding-set-to-0, should-render-a-simple-flowchart-with-line-breaks, should-render-a-simple-flowchart-with-trapezoid-and-inverse-trapezoid-vertex-options, should-render-color-of-styled-nodes, should-render-escaped-without-html-labels, should-render-hexagons, should-render-labels-with-numbers-at-the-start, should-render-styled-subgraphs, should-render-subgraphs-with-a-title-starting-with-a-digit, should-render-subgraphs, should-style-nodes-via-a-class, testing-the-label-width-in-percy

**fontFamily courier (no metrics in ferrigura-text)** (27)

- `visual-check/` (27): 1-should-render-a-simple-flowchart-no-htmllabels, 10-should-render-subgraphs-with-a-title-starting-with-a-digit, 11-should-render-styled-subgraphs, 14-should-render-hexagons, 15-should-render-a-simple-flowchart-with-comments, 16-render-stadium-shape, 17-render-multiline-texts, 20-multiple-nodes-and-chaining-in-one-statement, 21-render-cylindrical-shape, 22-render-a-simple-flowchart-with-nodespacing-set-to-100, 23-render-a-simple-flowchart-with-rankspacing-set-to-100, 24-keep-node-label-text-if-already-defined-when-a-style-is-applied, 27-set-text-color-of-nodes-and-links-according-to-styles-when-html-labels-are-disabled, 3-should-render-a-simple-flowchart-with-line-breaks, 30-possibility-to-style-text-color-of-nodes-and-subgraphs-as-well-as-apply-classes-to-subgraphs, 32-render-subroutine-shape, 4-should-render-a-simple-flowchart-with-trapezoid-and-inverse-trapezoid-vertex-options, 68-should-honor-subgraph-direction-when-inheritdir-is-false, 69-should-inherit-global-direction-when-inheritdir-is-true, 69-should-render-subgraphs-with-adhoc-list-headings, 7-should-render-a-flowchart-full-of-icons, 70-should-render-a-subgraph-with-direction-td, 70-should-render-subgraphs-with-markdown-headings, 9-should-render-subgraphs, document-shape-label-clearance, selected-shape-wrapping, v2-16-render-stadium-shape

**theme other than redux-color or unset (default, dark, forest, base, neutral, redux, redux-dark, redux-dark-color, neo, neo-dark)** (70)

- `nightly-check/icon/` (10): 01-should-render-registered-icons-on-theme-default, 02-should-render-icons-from-fontawesome-library-on-theme-default, 03-should-render-icons-from-fontawesome-library-on-theme-dark, 04-should-render-registered-icons-on-theme-forest, 05-should-render-registered-icons-on-theme-dark, 06-should-render-icons-from-fontawesome-library-on-theme-base, 07-should-render-registered-icons-on-theme-base, 08-should-render-icons-from-fontawesome-library-on-theme-forest, 09-should-render-icons-from-fontawesome-library-on-theme-neutral, 10-should-render-registered-icons-on-theme-neutral
- `nightly-check/redux-color-subgraphs/` (12): 01-should-render-subgraphs-subgraph-containers, 02-should-render-nested-subgraph-containers, 03-should-render-collapsed-subgraph-containers, 04-should-render-user-styled-subgraph-containers, 09-should-render-subgraphs-subgraph-containers, 10-should-render-nested-subgraph-containers, 11-should-render-collapsed-subgraph-containers, 12-should-render-user-styled-subgraph-containers, 13-should-render-subgraphs-subgraph-containers, 14-should-render-nested-subgraph-containers, 15-should-render-collapsed-subgraph-containers, 16-should-render-user-styled-subgraph-containers
- `nightly-check/shape-themes/` (40): 01-connect-all-shapes-with-each-other, 02-connect-all-shapes-with-each-other, 03-connect-all-shapes-with-each-other, 04-connect-all-shapes-with-each-other, 05-connect-all-shapes-with-each-other, 06-connect-all-shapes-with-each-other, 07-connect-all-shapes-with-each-other, 08-connect-all-shapes-with-each-other, 09-connect-all-shapes-with-each-other, 10-connect-all-shapes-with-each-other, 11-connect-all-shapes-with-each-other, 12-connect-all-shapes-with-each-other, 13-connect-all-shapes-with-each-other, 14-connect-all-shapes-with-each-other, 15-connect-all-shapes-with-each-other, 16-connect-all-shapes-with-each-other, 17-connect-all-shapes-with-each-other, 18-connect-all-shapes-with-each-other, 19-connect-all-shapes-with-each-other, 20-connect-all-shapes-with-each-other, 21-connect-all-shapes-with-each-other, 22-connect-all-shapes-with-each-other, 23-connect-all-shapes-with-each-other, 24-connect-all-shapes-with-each-other, 25-connect-all-shapes-with-each-other, 26-connect-all-shapes-with-each-other, 27-connect-all-shapes-with-each-other, 28-connect-all-shapes-with-each-other, 29-connect-all-shapes-with-each-other, 30-connect-all-shapes-with-each-other, 31-connect-all-shapes-with-each-other, 32-connect-all-shapes-with-each-other, 33-connect-all-shapes-with-each-other, 34-connect-all-shapes-with-each-other, 35-connect-all-shapes-with-each-other, 36-connect-all-shapes-with-each-other, 37-connect-all-shapes-with-each-other, 38-connect-all-shapes-with-each-other, 39-connect-all-shapes-with-each-other, 40-connect-all-shapes-with-each-other
- `visual-check/` (8): dark-theme-legacy-graph, dark-theme-new-syntax, default-theme-legacy-graph, default-theme-new-syntax, forest-theme-legacy-graph, forest-theme-new-syntax, neutral-theme-legacy-graph, neutral-theme-new-syntax

**@{ shape } names outside the classic set** (55)

- `nightly-check/` (2): 6080-should-handle-diamond-shape-intersections, 6088-4-should-handle-diamond-shape-intersections
- `nightly-check/classic/` (1): 03-40-should-add-edge-animation
- `nightly-check/elk-container-algorithms/` (1): elk-subgraph-with-box-algorithm
- `nightly-check/redux-color-subgraphs/` (1): 07-should-render-collapsed-subgraph-containers
- `nightly-check/shape-alias/` (35): 02-all-event-rounded-should-render-same-shape, 08-all-lean-l-lean-left-out-in-should-render-same-shape, 09-all-lean-r-lean-right-in-out-should-render-same-shape, 10-all-trap-b-trapezoid-bottom-priority-should-render-same-shape, 11-all-trap-t-trapezoid-top-manual-should-render-same-shape, 13-all-notched-rectangle-card-notch-rect-should-render-same-shape, 14-all-sm-circ-small-circle-start-should-render-same-shape, 15-all-lin-rect-lined-rectangle-lin-proc-lined-process-shaded-process-should-render-same-shape, 16-all-fr-circ-framed-circle-stop-should-render-same-shape, 17-all-comment-brace-l-should-render-same-shape, 18-all-fork-join-should-render-same-shape, 19-all-bolt-com-link-lightning-bolt-should-render-same-shape, 20-all-doc-document-should-render-same-shape, 21-all-delay-half-rounded-rectangle-should-render-same-shape, 22-all-h-cyl-das-horizontal-cylinder-should-render-same-shape, 23-all-lin-cyl-disk-lined-cylinder-should-render-same-shape, 24-all-curv-trap-display-curved-trapezoid-should-render-same-shape, 25-all-div-rect-div-proc-divided-rectangle-divided-process-should-render-same-shape, 26-all-extract-tri-triangle-should-render-same-shape, 27-all-win-pane-internal-storage-window-pane-should-render-same-shape, 28-all-f-circ-junction-filled-circle-should-render-same-shape, 29-all-lin-doc-lined-document-should-render-same-shape, 30-all-notch-pent-loop-limit-notched-pentagon-should-render-same-shape, 31-all-flip-tri-manual-file-flipped-triangle-should-render-same-shape, 32-all-sl-rect-manual-input-sloped-rectangle-should-render-same-shape, 33-all-docs-documents-st-doc-stacked-document-should-render-same-shape, 34-all-procs-processes-st-rect-stacked-rectangle-should-render-same-shape, 35-all-flag-paper-tape-should-render-same-shape, 36-all-bow-rect-stored-data-bow-tie-rectangle-should-render-same-shape, 37-all-cross-circ-summary-crossed-circle-should-render-same-shape, 38-all-tag-doc-tagged-document-should-render-same-shape, 39-all-tag-rect-tag-proc-tagged-rectangle-tagged-process-should-render-same-shape, 40-all-collate-hourglass-should-render-same-shape, 41-all-datastore-data-store-should-render-same-shape, 42-all-folder-directory-should-render-same-shape
- `pr-check/` (1): 5-icon-shape-bundled
- `visual-check/` (14): 6617-per-link-curve-styling-using-edge-ids, should-be-possible-to-use-syntax-to-add-labels-on-multi-nodes, should-be-possible-to-use-syntax-to-add-labels-with-trail-spaces-and-edge-link, should-be-possible-to-use-syntax-to-add-labels-with-trail-spaces-and, should-be-possible-to-use-syntax-to-add-labels-with-trail-spaces, should-collapse-nested-subgraphs-to-the-outermost-collapsed-ancestor, should-redirect-an-edge-targeting-a-deeply-nested-node-to-the-outermost-collapsed-subgraph, should-render-a-collapsed-subgraph-alongside-an-expanded-one, should-render-a-collapsed-subgraph-as-a-single-node-with-edges-redirected-to-it, should-render-a-diamond-shaped-node-using-shape-config, should-render-a-rounded-rectangle-and-a-normal-rectangle, should-render-aws-icons-with-labels-and-rect-elements, should-render-multiple-aws-icons-with-rect-elements-in-a-flowchart, should-render-single-aws-icon-with-rect-elements-without-unintended-opacity

**markdown strings or terminal-label candidates** (12)

- `nightly-check/v2/` (3): 03-should-render-raw-strings, 04-should-render-raw-strings-with-htmllabels-false, 06-should-wrap-edge-labels-markdownautowrap-true-htmllabels-false
- `pr-check/` (1): 7-markdown-label-html-false
- `visual-check/` (8): 5b-angle-brackets-should-be-work-without-html-labels, new-line-in-node-and-formatted-edge-label-2, should-not-auto-wrap-when-markdownautowrap-is-false, sub-graphs-and-markdown-strings-2, v2-17-should-apply-class-def-colour-to-edge-label, with-formatting-in-a-node-2, with-styling-and-classes-2, wrapping-long-text-with-a-new-line-2

**icons or images** (13)

- `nightly-check/` (4): 02-1-should-render-a-simple-flowchart, 03-1-should-render-a-simple-flowchart, 6088-1-should-handle-diamond-shape-intersections, 6088-3-should-handle-diamond-shape-intersections
- `nightly-check/classic/` (2): 01-38-should-render-a-flowchart-when-usemaxwidth-is-true-default, 02-39-should-render-a-flowchart-when-usemaxwidth-is-false
- `nightly-check/v2/` (2): 01-7-should-render-a-flowchart-when-usemaxwidth-is-true-default, 02-8-should-render-a-flowchart-when-usemaxwidth-is-false
- `visual-check/` (5): 1-should-render-a-simple-flowchart, 2-should-render-a-simple-flowchart-with-diagrampadding-set-to-0, 33-should-render-a-simple-flowchart-with-diagrampadding-set-to-0, 4023-should-render-html-labels-with-images-and-or-text-correctly, 4439-should-render-the-graph-even-if-some-images-are-missing

**subgraphs deeper than 2 or subgraph direction** (11)

- `nightly-check/` (1): 6088-2-should-handle-diamond-shape-intersections
- `pr-check/` (1): circle-content-sizing
- `visual-check/` (9): 3258-should-render-subgraphs-with-large-nodespacing-and-rankspacing, 3258-should-render-subgraphs-with-main-graph-nodespacing-and-rankspacing, should-render-self-loops-in-non-tb-directions, should-render-self-loops, should-render-subgraphs-with-title-margins-and-edge-labels, should-render-subgraphs-with-title-margins-set-lr-and-htmllabels-set-to-false, should-render-subgraphs-with-title-margins-set-lr, should-render-subgraphs-with-title-margins-set-td, v2-18-should-render-nested-subgraphs-with-edge-from-cluster-containing-extractable-subgraph

## Appendix A: feature classes (overlapping)

Classes with 25 files or fewer are listed in full. Larger classes give the count, and the full list is in `lists-full.txt` in the scratchpad directory above.

- **front matter absent** (5): nightly-check/8292-elk-edge-label-jog, nightly-check/8326-should-merge-repeated-subgraph-id-with-nested-subgraph, nightly-check/8326-should-merge-repeated-subgraph-ids, pr-check/8368-elk-label-parallel-edge, pr-check/circle-content-sizing
- **front matter title** (6): nightly-check/06-should-render-a-flowchart-with-title, nightly-check/v2/05-should-wrap-edge-labels-markdownautowrap-true-htmllabels-true, nightly-check/v2/06-should-wrap-edge-labels-markdownautowrap-true-htmllabels-false, nightly-check/v2/07-should-wrap-edge-labels-markdownautowrap-false-htmllabels-true, visual-check/3258-should-render-subgraphs-with-large-nodespacing-and-rankspacing, visual-check/3258-should-render-subgraphs-with-main-graph-nodespacing-and-rankspacing
- **front matter coveredShapes (non-standard key)** (1): pr-check/1-shapes-layout-default
- **init directive** (23): nightly-check/handdrawn/fontawesome-icons-in-edge-labels-4, pr-check/8368-elk-label-parallel-edge, visual-check/63-title-on-subgraphs-should-be-themeable, visual-check/68-should-honor-subgraph-direction-when-inheritdir-is-false, visual-check/69-should-inherit-global-direction-when-inheritdir-is-true, visual-check/dark-theme-legacy-graph, visual-check/dark-theme-new-syntax, visual-check/default-theme-legacy-graph, visual-check/default-theme-new-syntax, visual-check/forest-theme-legacy-graph, visual-check/forest-theme-new-syntax, visual-check/neutral-theme-legacy-graph, visual-check/neutral-theme-new-syntax, visual-check/new-line-in-node-and-formatted-edge-label-2, visual-check/new-line-in-node-and-formatted-edge-label, visual-check/sub-graphs-and-markdown-strings-2, visual-check/sub-graphs-and-markdown-strings, visual-check/with-formatting-in-a-node-2, visual-check/with-formatting-in-a-node, visual-check/with-styling-and-classes-2, visual-check/with-styling-and-classes, visual-check/wrapping-long-text-with-a-new-line-2, visual-check/wrapping-long-text-with-a-new-line
- **layout explicit elk** (5): nightly-check/elk-container-algorithms/elk-subgraph-with-box-algorithm, pr-check/3-layout-elk-subgraphs, pr-check/8368-elk-label-parallel-edge, visual-check/should-keep-the-true-entry-on-top-when-a-back-edge-feeds-it-79, visual-check/subgraph-titles-wider-than-children
- **layout elk.box** (1): nightly-check/elk-container-algorithms/elk-box-layout-with-edges
- **layout elk.rectpacking** (1): nightly-check/elk-container-algorithms/elk-rectpacking-layout-with-edges
- **look handDrawn** (50): see `lists-full.txt`
- **look neo set explicitly** (40): see `lists-full.txt`
- **htmlLabels false** (47): see `lists-full.txt`
- **theme default set explicitly** (4): nightly-check/icon/01-should-render-registered-icons-on-theme-default, nightly-check/icon/02-should-render-icons-from-fontawesome-library-on-theme-default, visual-check/default-theme-legacy-graph, visual-check/default-theme-new-syntax
- **theme redux-color set explicitly** (4): nightly-check/redux-color-subgraphs/05-should-render-subgraphs-subgraph-containers, nightly-check/redux-color-subgraphs/06-should-render-nested-subgraph-containers, nightly-check/redux-color-subgraphs/07-should-render-collapsed-subgraph-containers, nightly-check/redux-color-subgraphs/08-should-render-user-styled-subgraph-containers
- **securityLevel sandbox** (1): visual-check/6a-should-render-complex-html-in-labels-with-sandbox-security
- **flowchart.curve** (1): visual-check/7213-should-render-edges-with-rounded-curve-right-angles-with-rounded-corners
- **flowchart.useMaxWidth** (6): nightly-check/classic/01-38-should-render-a-flowchart-when-usemaxwidth-is-true-default, nightly-check/classic/02-39-should-render-a-flowchart-when-usemaxwidth-is-false, nightly-check/handdrawn/01-fdh39-should-render-a-flowchart-when-usemaxwidth-is-false, nightly-check/handdrawn/02-fdh38-should-render-a-flowchart-when-usemaxwidth-is-true-default, nightly-check/v2/01-7-should-render-a-flowchart-when-usemaxwidth-is-true-default, nightly-check/v2/02-8-should-render-a-flowchart-when-usemaxwidth-is-false
- **themeVariables** (2): nightly-check/handdrawn/fontawesome-icons-in-edge-labels-4, visual-check/63-title-on-subgraphs-should-be-themeable
- **markdown labels** (21): nightly-check/v2/03-should-render-raw-strings, nightly-check/v2/04-should-render-raw-strings-with-htmllabels-false, nightly-check/v2/05-should-wrap-edge-labels-markdownautowrap-true-htmllabels-true, nightly-check/v2/06-should-wrap-edge-labels-markdownautowrap-true-htmllabels-false, nightly-check/v2/07-should-wrap-edge-labels-markdownautowrap-false-htmllabels-true, pr-check/6-markdown-label-html-true, pr-check/7-markdown-label-html-false, visual-check/5824-should-be-able-to-render-string-and-markdown-labels, visual-check/5b-angle-brackets-should-be-work-without-html-labels, visual-check/70-should-render-subgraphs-with-markdown-headings, visual-check/new-line-in-node-and-formatted-edge-label-2, visual-check/new-line-in-node-and-formatted-edge-label, visual-check/should-not-auto-wrap-when-markdownautowrap-is-false, visual-check/sub-graphs-and-markdown-strings-2, visual-check/sub-graphs-and-markdown-strings, visual-check/with-formatting-in-a-node-2, visual-check/with-formatting-in-a-node, visual-check/with-styling-and-classes-2, visual-check/with-styling-and-classes, visual-check/wrapping-long-text-with-a-new-line-2, visual-check/wrapping-long-text-with-a-new-line
- **icons** (54): see `lists-full.txt`
- **images (<img>)** (2): visual-check/4023-should-render-html-labels-with-images-and-or-text-correctly, visual-check/4439-should-render-the-graph-even-if-some-images-are-missing
- **click or link or href** (15): nightly-check/handdrawn/handle-link-click-events-link-anchor-mailto-other-protocol-script, nightly-check/handdrawn/render-cylindrical-shape, nightly-check/handdrawn/render-stadium-shape, nightly-check/handdrawn/render-subroutine-shape, nightly-check/handdrawn/set-text-color-of-nodes-and-links-according-to-styles-when-html-labels-are-disabled, nightly-check/handdrawn/set-text-color-of-nodes-and-links-according-to-styles-when-html-labels-are-enabled, nightly-check/handdrawn/should-render-hexagons, visual-check/14-should-render-hexagons, visual-check/16-render-stadium-shape, visual-check/21-render-cylindrical-shape, visual-check/25-handle-link-click-events-link-anchor-mailto-other-protocol-script, visual-check/26-set-text-color-of-nodes-and-links-according-to-styles-when-html-labels-are-enabled, visual-check/27-set-text-color-of-nodes-and-links-according-to-styles-when-html-labels-are-disabled, visual-check/32-render-subroutine-shape, visual-check/v2-16-render-stadium-shape
- **classDef** (38): see `lists-full.txt`
- **class statement** (28): see `lists-full.txt`
- **style statement** (31): see `lists-full.txt`
- **linkStyle statement** (13): nightly-check/handdrawn/handle-styling-for-all-node-shapes, nightly-check/handdrawn/keep-node-label-text-if-already-defined-when-a-style-is-applied, nightly-check/handdrawn/render-multiline-texts, nightly-check/handdrawn/set-text-color-of-nodes-and-links-according-to-styles-when-html-labels-are-disabled, nightly-check/handdrawn/set-text-color-of-nodes-and-links-according-to-styles-when-html-labels-are-enabled, pr-check/60-handle-styling-for-all-node-shapes, visual-check/17-render-multiline-texts, visual-check/24-keep-node-label-text-if-already-defined-when-a-style-is-applied, visual-check/26-set-text-color-of-nodes-and-links-according-to-styles-when-html-labels-are-enabled, visual-check/27-set-text-color-of-nodes-and-links-according-to-styles-when-html-labels-are-disabled, visual-check/59-handle-styling-of-subgraphs-and-links, visual-check/60-handle-styling-for-all-node-shapes-v2, visual-check/6369-edge-color-should-affect-arrow-head
- **:::shorthand** (6): nightly-check/handdrawn/multiple-nodes-and-chaining-in-one-statement-2, visual-check/20-multiple-nodes-and-chaining-in-one-statement, visual-check/65-2-bold-text-from-classes, visual-check/65-3-bigger-font-from-classes, visual-check/with-styling-and-classes-2, visual-check/with-styling-and-classes
- **edge ids (e1@-->)** (4): nightly-check/classic/03-40-should-add-edge-animation, nightly-check/handdrawn/03-fdh49-should-add-edge-animation, visual-check/6617-per-link-curve-styling-using-edge-ids, visual-check/v2-17-should-apply-class-def-colour-to-edge-label
- **per-edge @{} options** (3): nightly-check/classic/03-40-should-add-edge-animation, nightly-check/handdrawn/03-fdh49-should-add-edge-animation, visual-check/6617-per-link-curve-styling-using-edge-ids
- **terminal-label candidates (verify by eye)** (11): nightly-check/v2/03-should-render-raw-strings, nightly-check/v2/04-should-render-raw-strings-with-htmllabels-false, nightly-check/v2/05-should-wrap-edge-labels-markdownautowrap-true-htmllabels-true, nightly-check/v2/06-should-wrap-edge-labels-markdownautowrap-true-htmllabels-false, nightly-check/v2/07-should-wrap-edge-labels-markdownautowrap-false-htmllabels-true, visual-check/5824-should-be-able-to-render-string-and-markdown-labels, visual-check/new-line-in-node-and-formatted-edge-label-2, visual-check/new-line-in-node-and-formatted-edge-label, visual-check/sub-graphs-and-markdown-strings-2, visual-check/sub-graphs-and-markdown-strings, visual-check/v2-17-should-apply-class-def-colour-to-edge-label
- **invisible links (~~~)** (3): nightly-check/07-render-with-stylized-arrows, pr-check/circle-content-sizing, visual-check/3192-it-should-be-possible-to-render-flowcharts-with-invisible-edges
- **dotted links (-.)** (3): nightly-check/07-render-with-stylized-arrows, nightly-check/4-length-of-edges, visual-check/4-length-of-edges
- **thick links (==)** (15): nightly-check/07-render-with-stylized-arrows, nightly-check/4-length-of-edges, nightly-check/handdrawn/should-honor-minimum-edge-length-as-specified-by-the-user, visual-check/35-should-honor-minimum-edge-length-as-specified-by-the-user, visual-check/4-length-of-edges, visual-check/59-handle-styling-of-subgraphs-and-links, visual-check/6-should-render-non-escaped-with-html-labels, visual-check/dark-theme-legacy-graph, visual-check/dark-theme-new-syntax, visual-check/default-theme-legacy-graph, visual-check/default-theme-new-syntax, visual-check/forest-theme-legacy-graph, visual-check/forest-theme-new-syntax, visual-check/neutral-theme-legacy-graph, visual-check/neutral-theme-new-syntax
- **ampersand chaining (A & B)** (12): nightly-check/6088-5-should-handle-diamond-shape-intersections, nightly-check/6088-6-should-handle-diamond-shape-intersections, nightly-check/6647-should-keep-node-order-when-using-layout-unless-it-would-add-crossings, nightly-check/7213-should-render-edges-with-right-angles-not-curves, nightly-check/handdrawn/handle-link-click-events-link-anchor-mailto-other-protocol-script, nightly-check/handdrawn/multiple-nodes-and-chaining-in-one-statement-2, nightly-check/handdrawn/multiple-nodes-and-chaining-in-one-statement, nightly-check/should-merge-edges-within-subgraphs-when-mergeedges-is-true-7659, visual-check/19-multiple-nodes-and-chaining-in-one-statement, visual-check/20-multiple-nodes-and-chaining-in-one-statement, visual-check/25-handle-link-click-events-link-anchor-mailto-other-protocol-script, visual-check/53-handle-nested-subgraphs-with-edges-in-and-out
- **accTitle or accDescr** (8): visual-check/dark-theme-legacy-graph, visual-check/dark-theme-new-syntax, visual-check/default-theme-legacy-graph, visual-check/default-theme-new-syntax, visual-check/forest-theme-legacy-graph, visual-check/forest-theme-new-syntax, visual-check/neutral-theme-legacy-graph, visual-check/neutral-theme-new-syntax
- **subgraph direction lines** (14): pr-check/circle-content-sizing, visual-check/2050-handling-of-different-rendering-direction-in-subgraphs, visual-check/3258-should-render-subgraphs-with-large-nodespacing-and-rankspacing, visual-check/3258-should-render-subgraphs-with-main-graph-nodespacing-and-rankspacing, visual-check/68-should-honor-subgraph-direction-when-inheritdir-is-false, visual-check/69-should-inherit-global-direction-when-inheritdir-is-true, visual-check/70-should-render-a-subgraph-with-direction-td, visual-check/issue-4648-nested-subgraph-with-external-connection, visual-check/issue-4648-sibling-subgraphs-with-different-directions-and-external-connections, visual-check/should-render-self-loops-in-non-tb-directions, visual-check/should-render-subgraphs-with-title-margins-and-edge-labels, visual-check/should-render-subgraphs-with-title-margins-set-lr-and-htmllabels-set-to-false, visual-check/should-render-subgraphs-with-title-margins-set-lr, visual-check/should-render-subgraphs-with-title-margins-set-td
- **text edge labels (-- text -->)** (22): nightly-check/6080-should-handle-diamond-shape-intersections, nightly-check/6088-1-should-handle-diamond-shape-intersections, nightly-check/6088-3-should-handle-diamond-shape-intersections, nightly-check/6088-4-should-handle-diamond-shape-intersections, nightly-check/8292-elk-edge-label-jog, nightly-check/handdrawn/multiple-nodes-and-chaining-in-one-statement-2, nightly-check/handdrawn/should-honor-minimum-edge-length-as-specified-by-the-user, nightly-check/sub-graphs, nightly-check/v2/03-should-render-raw-strings, nightly-check/v2/04-should-render-raw-strings-with-htmllabels-false, nightly-check/v2/05-should-wrap-edge-labels-markdownautowrap-true-htmllabels-true, nightly-check/v2/06-should-wrap-edge-labels-markdownautowrap-true-htmllabels-false, nightly-check/v2/07-should-wrap-edge-labels-markdownautowrap-false-htmllabels-true, visual-check/20-multiple-nodes-and-chaining-in-one-statement, visual-check/35-should-honor-minimum-edge-length-as-specified-by-the-user, visual-check/4b-labeled-open-edges-should-not-have-arrowheads, visual-check/5824-should-be-able-to-render-string-and-markdown-labels, visual-check/74-handle-labels-for-multiple-edges-from-and-to-the-same-couple-of-nodes, visual-check/sub-graphs-and-markdown-strings-2, visual-check/sub-graphs-and-markdown-strings, visual-check/with-formatting-in-a-node-2, visual-check/with-formatting-in-a-node
- **pipe edge labels (|text|)** (75): see `lists-full.txt`

## 6. Open questions

1. **Courier (70 files).** Options: (a) make ferrigura measure a courier face (which font, which license, which metrics); (b) rewrite `fontFamily` on both sides to a bundled face (changes the input, so the comparison no longer tests the fixture as written); (c) leave them out of M5. This decides whether the "about 30" first set exists.
2. **HTML labels (66 files set `htmlLabels: true`).** In scope for M5, or a later milestone? If later, they are excluded from the done criterion, and the done criterion has to say so.
3. **handDrawn (50 files).** Same question: rough.js is not in the port.
4. **Node and edge geometry contract.** Does ferrigura write `data-points`, `data-id`, the node `transform` and `label-container` classes, the way the reference does? Pairing by element id needs the same ids. This is a decision for the M5 SVG contract.
5. **Neo look.** Ferrigura must draw neo (per-shape padding, margin markers, cluster drawing). Is neo in M5 scope? Without it, stage 1 cannot be compared, because the default look is neo.
6. **Per-diagram defaults.** Does ferrigura apply `FlowchartDiagramConfig` defaults (`theme: redux-color`, `look: neo`)? The harness assumes it, and pie's harness does not exercise it (section 1.8, point 5).
7. **Other themes (71 files outside the default appearance).** Ferrigura ships only `redux-color` and `redux-dark-color`; other names fall back (`theme/mod.rs` `from_name`). Whether geometry changes with the theme is **unverified**; colours are not compared. Explicit `theme: default` (4 files) is the Mermaid default theme, not the flowchart default.
8. **`@{ shape }` names (102 files outside stage 1a).** Needs the full shape registry for geometry. Is this M5, or later?
9. **ELK algorithms.** Two fixtures need `elk.box` or `elk.rectpacking`. The reference runs the real algorithm, and the port runs layered. Exclude, or port them?
10. **Scope of the done criterion.** "Within tolerance over the flowchart fixtures" can mean all 352 files or the comparable subset. The partition above shows how much is outside the subset at each stage.
11. **Reference build.** Harness numbers depend on the reference bundle built from the M5 branch. The main checkout's dist (11:57) predates HEAD's commit (12:17); **unverified** that it matches HEAD.
12. **Duplicate basenames** (two in flowcharts) need relative-path ids (section 1.8, point 3).
13. **Terminal labels and `edgeTerminals`** (11 candidates, heuristic). Verify by eye before staging them.
