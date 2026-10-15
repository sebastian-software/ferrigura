# Fork notes

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

**Project infrastructure.** Upstream's GitHub workflows, issue templates, Netlify, Docker, Renovate, funding and citation files, the Chinese README copy, and the Applitools/Argos visual-testing integration. One workflow, `.github/workflows/ci.yml`, runs lint, the schema/type sync check, the circular-dependency check, unit tests and the out-of-tree TypeScript check. Playwright compares against local screenshot baselines.

**Documentation website.** The VitePress site (theme, landing page, news, ecosystem and integration pages) and the generated JSON Schema reference. The Markdown reference docs stay in `packages/mermaid/src/docs` and are rendered to `docs/` for GitHub.

**Dead code** (about 4,000 lines). Upstream dropped the legacy `flowchart`, `class` and `state` diagram ids in 12.0, but their renderers stayed behind: `class/classRenderer.js`, `class/svgDraw.js`, the duplicate `class/classDiagram.ts`, `state/stateRenderer.js`, `state/shapes.js`, `state/stateDiagram.ts`, the StateDB v1 path, `er/erRenderer.js`, `er/erMarkers.js`, `flowchart/flowChartShapes.js` and the pre-unified `mindmap/svgDraw.ts`. Also gone: `interactionDb.ts` (never populated), the orphaned copies `block/blockUtils.ts` and `shapes/document.ts`, and helpers, `setConf` stubs and types nothing called.

## Candidates for the next round

These are deprecated, legacy or redundant, but removing them changes user-visible behaviour or needs a decision first:

- **`%%{init}%%` and `%%{wrap}%%` directives.** Deprecated in the docs since 10.5 in favour of front matter; about 140 e2e fixtures still use them.
- **Keyword aliases that upstream never deprecated.** `graph` (alias of `flowchart`), `classDiagram-v2`, `stateDiagram-v2`; `ishikawa-beta`, `treemap-beta` and `architecture-beta`, whose docs still use the beta keyword.
- **`securityLevel: 'antiscript'`.** The schema describes it as allowing HTML and clicks, but the code treats it exactly like `strict`.
- **Smaller internals.** The `arrowMarkerAbsolute` copies in each diagram section; the deprecated `createLabel.js` wrapper around `createText`; the `D3Element = any` type; the `"./*"` deep-import entry in `package.json` exports; layout test infrastructure (`ddlt`, `layout-utils`, the agentflow conformance runner) living in `src`; the size-capture dev hook in `createGraph.ts`; the single `uuid` use; 47 JavaScript files that could move to TypeScript; and duplicated helpers (three ways to select the SVG root, two viewport helpers, two curve maps).

## Upstream bugs fixed

- A `%%{init}%%` directive was ignored when an earlier directive search on the same page had failed: the shared global `directiveRegex` kept its `lastIndex` after a `JSON.parse` error. `mermaid.run()` triggered this for any diagram whose front matter mentioned a directive in prose, because it ran an extra, debug-only directive search first.
- A Gantt chart with the `topAxis` keyword failed to parse: the grammar called `yy.TopAxis()`, which the db never defined. It now calls `enableTopAxis()`.
- An architecture diagram starting with the graduated `architecture` keyword passed detection and then failed to parse: the grammar accepted only `architecture-beta`, and the token builder guarded a keyword that did not exist. The grammar now accepts both, as the beta policy requires.
- The requirement diagram detector also claimed text starting with `requirement`, which its grammar rejects. It now matches only `requirementDiagram`, so such text gets the "unknown diagram" error instead of a parse error.
- The C4 detector anchored only `C4Context` to the start of the text. `C4Container`, `C4Component`, `C4Dynamic` and `C4Deployment` matched anywhere, and since C4 is checked before most diagrams, a flowchart or sequence diagram that merely mentioned one of them was rendered as C4.
- The Wardley config section, keyed `wardley-beta`, never applied its `theme`, `look` and `layout`: `diagramConfigKeys.ts` did not map the `wardley` diagram type to it, and the schema defaults generator skipped it, so the section was missing from `defaultConfig` and front matter or directives dropped it as an unknown key.
- `defaultConfig.ts` copied the whole flowchart section into `c4.flowchart`, apparently to default `flowchart.inheritDir` to `false`. The schema already sets that default in the right place, so the copy only put a stray `flowchart` section under `c4`.

## Upstream bugs noticed along the way (not fixed)

- `mermaid.run()` calls `parseError` twice for a diagram that fails to render.
- `setLogLevel` lowercases a string level to check it, but looks up the original spelling.
- The radar diagram imports types from the parser's `dist/` output instead of the package.
- Railroad: like Wardley, `railroad` is missing from the schema defaults generator's key list in `.build/jsonSchema.ts`. Options only railroad has, such as `arcRadius`, `compactMode` and `verticalSeparation`, are therefore not in `configKeys`, and front matter or directives drop them.
