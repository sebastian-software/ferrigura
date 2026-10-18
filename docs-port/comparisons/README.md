# Comparisons

How ferrigura stands next to other renderers of Mermaid's syntax. Each project has a page of its own: what it is, how it differs from ferrigura, and what ferrigura can learn from it. This page holds the overview and the measurements. Facts are from each project's documentation and sources at the version named, as of 2026-10-17; measured values say how they were measured.

| Project                 | What it is                                                      |
| ----------------------- | --------------------------------------------------------------- |
| [Mermaid](./mermaid.md) | The original, and the reference ferrigura is compared against   |
| [Pele](./pele.md)       | An independent, fast renderer of Mermaid's syntax, for Obsidian |

## Overview

|                        | ferrigura                                                                                | Mermaid                                                     | Pele                                                         |
| ---------------------- | ---------------------------------------------------------------------------------------- | ----------------------------------------------------------- | ------------------------------------------------------------ |
| Version                | `main` after P2 (2026-10-17) and the cache of decision 0009                              | 12.2.0                                                      | 1.6.0                                                        |
| By                     | Sebastian Software                                                                       | the mermaid-js project                                      | kepano, for Obsidian                                         |
| Licence                | MIT; ported parts under EPL-2.0 (ELK), Apache-2.0 and BSD-3-Clause (llvm-libc, V8, Skia) | MIT                                                         | MIT                                                          |
| Language               | Rust                                                                                     | TypeScript                                                  | TypeScript                                                   |
| Runs in                | A library, a CLI and a native Node addon; no browser, no WebAssembly                     | A browser, or a headless one on a server                    | Any JavaScript runtime, browser or server                    |
| Needs a DOM            | No                                                                                       | Yes                                                         | No                                                           |
| API                    | Synchronous, returns the SVG                                                             | Asynchronous, draws into the page                           | Synchronous, returns the SVG                                 |
| Goal                   | Mermaid's output                                                                         | (the reference)                                             | Mermaid's syntax, its own drawing                            |
| Output against Mermaid | The same geometry within 0.05 px on every comparable fixture                             | (the reference)                                             | Its own layout and style                                     |
| Diagram families       | 24 of 33                                                                                 | 33                                                          | 33 (those of Mermaid 12.1.0)                                 |
| Layout                 | Mermaid's engines, ported: ELK Layered, cose-bilkent, fcose, the tidy tree               | ELK (the default since 12.0), cose-bilkent, fcose, and more | Its own: layered, compound, lanes                            |
| Text measurement       | The bundled faces, shaped with HarfRust and rounded as Chromium rounds                   | The DOM: `getBBox()`, `getComputedTextLength()`             | The canvas's `measureText`; an Arial width table without one |
| Fonts                  | Embedded in the SVG as a subset of the glyphs used                                       | The page's                                                  | The page's                                                   |
| Labels                 | SVG text                                                                                 | HTML in `foreignObject` by default                          | SVG text                                                     |
| Colours                | CSS custom properties with attribute fallbacks; Mermaid's redux themes                   | Themes and `themeVariables`                                 | CSS custom properties                                        |
| Narrow screens         | Scales the diagram                                                                       | Scales the diagram                                          | Adapts the layout                                            |
| Size, gzip (raw)       | 5.0 MB (11.6 MB), the native addon, fonts included                                       | 1.57 MB (5.5 MB), `mermaid.min.js`                          | 251 kB (678 kB), `pele.js`                                   |
| Direct dependencies    | 14 crates                                                                                | 23                                                          | 0                                                            |
| Render time, median    | 0.87 ms                                                                                  | 8.8 ms                                                      | 0.05 ms                                                      |

The sizes are not alike: ferrigura's is a native library for one platform, with its text engine and 0.5 MB of fonts, where the other two are JavaScript that the browser brings the rest to.

## Render time

Measured with [`tools/port/compare-bench.ts`](../../tools/port/compare-bench.ts) on the 215 `pr-check` fixtures of the 25 types all three render: each time is the median of 30 renders of one fixture (10 for Mermaid) after a warm-up. ferrigura ran through its Node addon (release build), Pele in Node (with its width table) and in Chromium (with the canvas), Mermaid in Chromium with its defaults. Every fixture rendered with all three. Machine: Apple M1 Ultra, macOS 27.0.1, Node 24.21.0, Chromium 151.0.7922.34. The ferrigura figures are of the build after the cache of the configuration ([decision 0009](../decisions/0009-configuration-resolved-once.md), step 1); the same fixtures had a median of 1.64 ms for ferrigura before it.

| Per diagram, over all 215 | ferrigura | Pele (Node) | Pele (Chromium) | Mermaid (Chromium) |
| ------------------------- | --------: | ----------: | --------------: | -----------------: |
| Median                    |   0.87 ms |     0.05 ms |       < 0.1 ms¹ |            8.80 ms |
| 90th percentile           |   1.87 ms |     0.16 ms |         0.20 ms |            36.8 ms |
| Slowest                   |   35.0 ms |     0.73 ms |         0.50 ms |           100.9 ms |

¹ Chromium's timer resolution is 0.1 ms on a page that is not cross-origin isolated, which is below what Pele needs.

The median per type, in milliseconds:

| Type          | Fixtures | ferrigura | Pele (Node) | Mermaid |
| ------------- | -------: | --------: | ----------: | ------: |
| flowchart     |       11 |      1.06 |        0.15 |   22.75 |
| sequence      |       10 |      0.89 |        0.08 |   14.85 |
| class         |        9 |      1.24 |        0.07 |   36.05 |
| state         |       10 |      1.72 |        0.12 |   34.15 |
| state v2      |       10 |      1.33 |        0.10 |   25.00 |
| ER            |       10 |      0.92 |        0.07 |   19.07 |
| gantt         |       11 |      0.74 |        0.10 |    5.45 |
| timeline      |       10 |      1.24 |        0.04 |    5.93 |
| gitGraph      |       10 |      0.66 |        0.04 |    8.90 |
| journey       |       11 |      0.79 |        0.05 |    6.70 |
| pie           |       10 |      0.51 |        0.02 |    2.98 |
| info          |        2 |      0.42 |        0.01 |    2.03 |
| mindmap       |        4 |      1.27 |        0.04 |   24.83 |
| block         |       10 |      0.71 |        0.04 |   22.53 |
| packet        |        5 |      0.49 |        0.02 |    3.40 |
| xychart       |       10 |      1.10 |        0.05 |    8.30 |
| sankey        |        3 |      0.86 |        0.11 |    9.10 |
| quadrantChart |       10 |      0.63 |        0.04 |    5.85 |
| requirement   |        2 |      1.48 |        0.09 |   25.93 |
| C4            |       10 |      1.04 |        0.05 |   10.30 |
| kanban        |       10 |      0.75 |        0.02 |    6.43 |
| architecture  |       10 |      1.27 |        0.09 |   21.40 |
| treemap       |       10 |      0.85 |        0.05 |    4.75 |
| ishikawa      |       11 |      0.64 |        0.02 |    4.95 |
| eventmodeling |        6 |      0.62 |        0.04 |    4.20 |

The work is not the same either: ferrigura embeds a font subset and reproduces Mermaid's layout, Mermaid lays out and measures in the page, and Pele in Node estimates text widths and draws a simpler layout.

## Where the time goes

A CPU profile of each, on the same fixtures, shows that the two that render Mermaid's output spend most of their time on the configuration, not on drawing:

- **Mermaid** (Chromium's profiler, 475 renders of the `pr-check` fixtures of ten types): 57 percent of the time is in `assignWithDepth`, the deep merge that `getConfig()` runs to hand out a fresh copy of the whole configuration, theme variables included, every time a renderer, shape or label asks for it. Then ELK and the other layouts (17 percent), DOMPurify (10), style and layout work of the browser (6), and the DOM measurement (`getBBox()` and the like, with the layout it forces: 4). So measuring text with a canvas, as Pele does, would save Mermaid a few percent; handing out the configuration without a deep copy per call would save more than half.
- **ferrigura** (macOS `sample`, 8 seconds of small pie renders after the cache, 4,867 samples in the render): 63 percent of the time is in `prepare`, which resolves the configuration of the render: 31 percent in merging the layers (`effective`), 28 in reading the typed view (`typed`, which copies the configuration and reads it into the types), and the rest in the front matter, the directives and the theme's variables. The renderer takes 26 percent, 20 of them in the text crate (shaping the labels and subsetting the font). Before the cache, 84 percent of the renders of a small pie went into rebuilding the defaults, the themes and the lenient reader's tables on every call (`ferrigura_model::config` and `::theme`). The cache took the pie's median from 1.27 ms to 0.51 ms; what remains is the copying and merging of the layers, which decision 0009 keeps for now.
- **Pele** was not profiled here. It has no DOM, no theme to compute and no configuration of that size to merge (its page), which is where the other two spend their time.

## Adding a comparison

Add a page named after the project with the sections of the others (what it is, how it differs from ferrigura, what ferrigura can learn from it), a row in the table at the top and a column in the overview, and run the benchmark with it when it can render the same fixtures. Name the version and the date of every fact.
