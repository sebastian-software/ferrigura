# Fork notes

The record of the fork's cleanup of Mermaid. The cleanup ended when the Rust port, ferrigura, took the fork as its reference: `packages/` does not change any more (see [README.md](./README.md)).

This fork starts from Mermaid 12.2.0 (upstream `develop` at `e28fbd9de`) and keeps the full upstream history. Each cleanup step is its own commit, so any of them can be reverted on its own:

```bash
git log --oneline e28fbd9de..HEAD
```

The guiding rule: keep the library, drop everything around it that the library doesn't need, and drop anything inside it that upstream deprecated or that no code path reaches.

## Migration guide

### Public API

| Removed                                                       | Use instead                                                                           |
| ------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| `mermaid.init(config, nodes, callback)`                       | `mermaid.initialize(config)`, then `await mermaid.run({ nodes, postRenderCallback })` |
| `mermaid.mermaidAPI.*`                                        | `mermaid.parse()`, `mermaid.render()`, `mermaid.initialize()`                         |
| `mermaid.startOnLoad` property                                | the `startOnLoad` config option                                                       |
| `mermaid.setParseErrorHandler(fn)`                            | `mermaid.parseError = fn`                                                             |
| 7th `injectUtils` argument (`parseDirective`)                 | nothing; it was a no-op since v10                                                     |
| C4 db `getBoundarys()`                                        | `getBoundaries()`                                                                     |
| `@mermaid-js/tiny`, `mermaid.tiny.min.js`                     | the ESM build, which loads large features as lazy chunks                              |
| `@mermaid-js/layout-elk`, `registerLayoutLoaders(elkLayouts)` | nothing; ELK is built in. `registerLayoutLoaders` stays for custom layouts            |

### Configuration

| Removed                                                                                                                                                                                           | Use instead                                                            |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| `flowchart.htmlLabels`                                                                                                                                                                            | root `htmlLabels`, which front matter and directives can now set too   |
| `class.htmlLabels`, `altFontFamily`                                                                                                                                                               | nothing; no code read them                                             |
| `legacyMathML`                                                                                                                                                                                    | nothing; every supported browser has MathML. `forceLegacyMathML` stays |
| `state.sizeUnit`, `titleShift`, `forkWidth`, `forkHeight`, `miniPadding`, `fontSizeFactor`, `edgeLengthFactor`, `compositTitleSize`, `dividerMargin`; `class.dividerMargin`; `er.minEntityHeight` | nothing; only the removed legacy renderers read them                   |
| warning for `lazyLoadedDiagrams` / `loadExternalDiagramsAtStartup` (options gone since v10)                                                                                                       | `registerExternalDiagrams()`                                           |
| theme variables `attributeBackgroundColorOdd` / `attributeBackgroundColorEven`                                                                                                                    | nothing; no stylesheet read them                                       |

### Diagram keywords

| Removed                                                    | Use instead                                                         |
| ---------------------------------------------------------- | ------------------------------------------------------------------- |
| `flowchart-elk`                                            | `flowchart`; ELK is the default layout, set `layout: elk` to pin it |
| `sankey-beta`, `packet-beta`, `xychart-beta`, `block-beta` | `sankey`, `packet`, `xychart`, `block`                              |

Diagrams that still use a removed keyword fail with an "unknown diagram" error.

### Layouts

| Removed         | Use instead                                                                                                         |
| --------------- | ------------------------------------------------------------------------------------------------------------------- |
| `layout: dagre` | nothing to set; ELK is the default. A diagram that still names `dagre` is laid out with ELK and a warning is logged |

ELK has been the default since 12.0. The dagre layout was the fallback for unregistered layout names and the documented way to get the pre-12 layout back; ELK now closes the fallback chain, and the "previous appearance" examples in the docs restore only `theme` and `look`. `graphlib` from `dagre-d3-es` stays, as the graph data structure behind the shared rendering pipeline and the block diagram.

### Behaviour changes

- The triangle shape and swimlane lane labels now follow the root `htmlLabels` option (default on) like every other shape. Before, they read `flowchart.htmlLabels`, which defaulted to off.
- Replacing `flowchart.htmlLabels: false` with root `htmlLabels: false` is not a pure rename. Upstream applied the nested option only where code went through its compatibility shim (edge and cluster labels, most flowchart shapes); class boxes, ER entities and a few other shapes read the root option and kept HTML labels. With the root option, those switch to SVG text too.

## What else was removed

**Packages and builds.** `@mermaid-js/mermaid-zenuml`, `@mermaid-js/examples`, `@mermaid-js/mermaid-example-diagram`, `@mermaid-js/mermaid-local-editor`, `@mermaid-js/layout-tidy-tree`, `@mermaid-js/tiny`, `@mermaid-js/layout-elk`, and the webpack and release-verification test harnesses. The `injected.includeLargeFeatures` build switch is gone with the tiny build.

**Project infrastructure.** Upstream's GitHub workflows, issue templates, Netlify, Docker, Renovate, funding and citation files, the Chinese README copy, and the Applitools/Argos visual-testing integration. The repository's workflows are `.github/workflows/ci.yml` (lint, the tests of the port's tools, the Rust job and the port's differential and geometry checks) and `publish.yml` (the releases of the crates). Since 2026-10-21 they no longer run Mermaid's unit tests, its Playwright suites, the schema/type sync check, the circular-dependency check or the out-of-tree TypeScript check, and those suites, scripts and their dependencies are gone: `packages/` stays only as the reference the port is checked against, and it does not change.

**Demo pages.** The 38 hand-written pages under `demos/` that the dev server used to serve at `/`. The dev explorer that replaced them at `/dev/` went on 2026-10-21 too; the dev server now serves the bundles and `e2e/platform`, which the port's corpus capture tools load.

**Documentation website.** The VitePress site (theme, landing page, news, ecosystem and integration pages) and the generated JSON Schema reference. The Markdown reference docs stay in `packages/mermaid/src/docs`, where the port reads their examples; their generated copy in `docs/` went on 2026-10-21.

**Dead code** (about 4,000 lines). Upstream dropped the legacy `flowchart`, `class` and `state` diagram ids in 12.0, but their renderers stayed behind: `class/classRenderer.js`, `class/svgDraw.js`, the duplicate `class/classDiagram.ts`, `state/stateRenderer.js`, `state/shapes.js`, `state/stateDiagram.ts`, the StateDB v1 path, `er/erRenderer.js`, `er/erMarkers.js`, `flowchart/flowChartShapes.js` and the pre-unified `mindmap/svgDraw.ts`. Also gone: `interactionDb.ts` (never populated), the orphaned copies `block/blockUtils.ts` and `shapes/document.ts`, and helpers, `setConf` stubs and types nothing called.

## Not removed

These are deprecated, legacy or redundant, but removing them would have changed user-visible behaviour or needed a decision first. With `packages/` frozen as the reference, they stay:

- **`%%{init}%%` and `%%{wrap}%%` directives.** Deprecated in the docs since 10.5 in favour of front matter; about 140 e2e fixtures still use them.
- **Keyword aliases that upstream never deprecated.** `graph` (alias of `flowchart`), `classDiagram-v2`, `stateDiagram-v2`; `ishikawa-beta`, `treemap-beta` and `architecture-beta`, whose docs still use the beta keyword.
- **`securityLevel: 'antiscript'`.** The schema describes it as allowing HTML and clicks, but the code treats it exactly like `strict`.
- **Smaller internals.** The `arrowMarkerAbsolute` copies in each diagram section; the deprecated `createLabel.js` wrapper around `createText`; the `D3Element = any` type; the `"./*"` deep-import entry in `package.json` exports; layout test infrastructure (`ddlt`, `layout-utils`, the agentflow conformance runner) living in `src`; the size-capture dev hook in `createGraph.ts`; the single `uuid` use; 47 JavaScript files that could move to TypeScript; and duplicated helpers (three ways to select the SVG root, two viewport helpers, two curve maps).

## Size compared with Mermaid 12.2.0

Measured on 15 October 2026 against upstream commit `e28fbd9de`, with both checkouts built by `pnpm build`. Compressed sizes use gzip level 9 and Brotli quality 11; 1 kB is 1,000 bytes.

|                                              | Mermaid 12.2.0 |  ferraid | Change |
| -------------------------------------------- | -------------: | -------: | -----: |
| `mermaid.min.js` (IIFE), minified            |        5.49 MB |  5.42 MB |  −1.3% |
| `mermaid.min.js` (IIFE), gzip                |       1,567 kB | 1,542 kB |  −1.6% |
| `mermaid.min.js` (IIFE), Brotli              |       1,177 kB | 1,157 kB |  −1.7% |
| ESM entry `mermaid.esm.min.mjs`, gzip        |        11.6 kB |  11.3 kB |  −2.4% |
| ESM entry and all lazy chunks, gzip          |       1,579 kB | 1,556 kB |  −1.5% |
| npm tarball of `mermaid`, packed             |        26.0 MB |  25.5 MB |  −2.0% |
| Library source lines, without tests and docs |        114,970 |  109,747 |  −4.5% |
| Workspace packages                           |              9 |        2 |     −7 |
| Packages resolved in `pnpm-lock.yaml`        |          2,409 |    1,280 |   −47% |
| Files tracked in the repository              |          4,272 |    3,762 |   −12% |
| Runtime dependencies of `mermaid`            |             23 |       23 |      0 |

The cleanup mostly removed what surrounds the library: seven packages, the website and the tooling they needed, which halves what `pnpm install` resolves. The bundles are only about 2% smaller, and most of that comes from removing the dagre layout. Most of the dead code removed from the library was never bundled in the first place, because nothing imported it. ("ferraid" was the fork's working name.)

## Upstream bugs fixed

- A `%%{init}%%` directive was ignored when an earlier directive search on the same page had failed: the shared global `directiveRegex` kept its `lastIndex` after a `JSON.parse` error. `mermaid.run()` triggered this for any diagram whose front matter mentioned a directive in prose, because it ran an extra, debug-only directive search first.
- A Gantt chart with the `topAxis` keyword failed to parse: the grammar called `yy.TopAxis()`, which the db never defined. It now calls `enableTopAxis()`.
- An architecture diagram starting with the graduated `architecture` keyword passed detection and then failed to parse: the grammar accepted only `architecture-beta`, and the token builder guarded a keyword that did not exist. The grammar now accepts both, as the beta policy requires.
- The requirement diagram detector also claimed text starting with `requirement`, which its grammar rejects. It now matches only `requirementDiagram`, so such text gets the "unknown diagram" error instead of a parse error.
- The C4 detector anchored only `C4Context` to the start of the text. `C4Container`, `C4Component`, `C4Dynamic` and `C4Deployment` matched anywhere, and since C4 is checked before most diagrams, a flowchart or sequence diagram that merely mentioned one of them was rendered as C4.
- The Wardley config section, keyed `wardley-beta`, never applied its `theme`, `look` and `layout`: `diagramConfigKeys.ts` did not map the `wardley` diagram type to it, and the schema defaults generator skipped it, so the section was missing from `defaultConfig` and front matter or directives dropped it as an unknown key.
- `defaultConfig.ts` copied the whole flowchart section into `c4.flowchart`, apparently to default `flowchart.inheritDir` to `false`. The schema already sets that default in the right place, so the copy only put a stray `flowchart` section under `c4`.
- `mermaid.run()` reported a diagram that failed to render to `parseError` three times: from the public `render()` it went through, from its per-diagram error handler, and once more when it rethrew the first failure. It now reports each failing diagram once. Errors outside any diagram, such as passing neither `nodes` nor `querySelector`, still reach `parseError` once.
- `setLogLevel('DEBUG')` silenced every level, fatal included: it lowercased the name to check it, then looked up the original spelling and got `undefined`. Names inherited from `Object.prototype`, such as `constructor`, passed the check the same way. Level names are now case-insensitive, and anything else falls back to `fatal`.
- The radar diagram imported its AST types through a relative path into the parser's `dist/` output. That worked inside the monorepo but ended up in mermaid's published `diagrams/radar/types.d.ts`, where nothing exists at that path. The parser now exports `Axis`, `Curve`, `Entry` and `Option`, and the radar diagram imports them from `@mermaid-js/parser`.
- Front matter and directives dropped the options only railroad has, such as `arcRadius`, `compactMode` and `verticalSeparation`: the schema defaults generator skipped the `railroad` section, so its keys never reached `defaultConfig` or `configKeys`. Its colours and typography still default to `undefined`, so the theme keeps supplying them.
- A flowchart node with shape data `@{ shape: constructor }` was accepted: `isValidShape` checked the name with `in`, so names inherited from `Object.prototype` counted as shapes. It now checks own keys only and reports `No such shape: constructor.`
- A node called `__proto__` listed twice in a subgraph stayed twice: the de-duplication marked ids in a plain object, and setting `__proto__` on it changed the prototype instead of adding a key. It now uses a `Set`.
