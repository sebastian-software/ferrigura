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

- **dagre layout.** ELK is the default and always available now. dagre is still the fallback layout, is hard-wired for use case diagrams, and is the documented way to keep the pre-12 look. `graphlib` from `dagre-d3-es` is used as a data structure regardless.
- **`%%{init}%%` and `%%{wrap}%%` directives.** Deprecated in the docs since 10.5 in favour of front matter; about 140 e2e fixtures still use them.
- **Keyword aliases that upstream never deprecated.** `graph` (alias of `flowchart`), `classDiagram-v2`, `stateDiagram-v2`; `ishikawa-beta` and `treemap-beta`, whose docs still use the beta keyword; `architecture-beta`, whose grammar accepts nothing else.
- **`securityLevel: 'antiscript'`.** The schema describes it as allowing HTML and clicks, but the code treats it exactly like `strict`.
- **Smaller internals.** The `arrowMarkerAbsolute` copies in each diagram section; the deprecated `createLabel.js` wrapper around `createText`; the `D3Element = any` type; the `"./*"` deep-import entry in `package.json` exports; layout test infrastructure (`ddlt`, `layout-utils`, the agentflow conformance runner) living in `src`; the size-capture dev hook in `createGraph.ts`; the single `uuid` use; 47 JavaScript files that could move to TypeScript; and duplicated helpers (three ways to select the SVG root, two viewport helpers, two curve maps).

## Upstream bugs fixed

- A `%%{init}%%` directive was ignored when an earlier directive search on the same page had failed: the shared global `directiveRegex` kept its `lastIndex` after a `JSON.parse` error. `mermaid.run()` triggered this for any diagram whose front matter mentioned a directive in prose, because it ran an extra, debug-only directive search first.

## Upstream bugs noticed along the way (not fixed)

- Gantt: the grammar calls `yy.TopAxis()` for the `topAxis` keyword, but the db only defines `enableTopAxis`.
- Architecture: the detector accepts `architecture`, but the grammar only accepts `architecture-beta`.
- Requirement: the detector accepts `requirement`, but the grammar only accepts `requirementDiagram`.
- C4: the detector anchors only `C4Context` to the start of the text; the other four keywords match anywhere.
- Wardley: its config section is keyed `wardley-beta` and is missing from `diagramConfigKeys.ts`, so per-diagram `theme`, `look` and `layout` are not applied.
- `flowchart.inheritDir` gets its default from a `c4.flowchart` block in `defaultConfig.ts`.
- `mermaid.run()` calls `parseError` twice for a diagram that fails to render.
- `setLogLevel` lowercases a string level to check it, but looks up the original spelling.
- The radar diagram imports types from the parser's `dist/` output instead of the package.
