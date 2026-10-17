# Comparisons

How ferrigura stands next to other renderers of Mermaid's syntax. Each project has a page of its own: what it is, how it differs from ferrigura, and what ferrigura can learn from it. This page holds the overview and the measurements. Facts are from each project's documentation and sources at the version named, as of 2026-10-17; measured values say how they were measured.

| Project                 | What it is                                                      |
| ----------------------- | --------------------------------------------------------------- |
| [Mermaid](./mermaid.md) | The original, and the reference ferrigura is compared against   |
| [Pele](./pele.md)       | An independent, fast renderer of Mermaid's syntax, for Obsidian |

## Overview

|                        | ferrigura                                                                                | Mermaid                                                     | Pele                                                         |
| ---------------------- | ---------------------------------------------------------------------------------------- | ----------------------------------------------------------- | ------------------------------------------------------------ |
| Version                | `main` after P2 (2026-10-17)                                                             | 12.2.0                                                      | 1.6.0                                                        |
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
| Render time, median    | 1.6 ms                                                                                   | 8.5 ms                                                      | 0.05 ms                                                      |

The sizes are not alike: ferrigura's is a native library for one platform, with its text engine and 0.5 MB of fonts, where the other two are JavaScript that the browser brings the rest to.

## Render time

Measured with [`tools/port/compare-bench.ts`](../../tools/port/compare-bench.ts) on the 215 `pr-check` fixtures of the 25 types all three render: each time is the median of 30 renders of one fixture (10 for Mermaid) after a warm-up. ferrigura ran through its Node addon (release build), Pele in Node (with its width table) and in Chromium (with the canvas), Mermaid in Chromium with its defaults. Every fixture rendered with all three. Machine: Apple M1 Ultra, macOS 27.0.1, Node 24.21.0, Chromium 151.0.7922.34.

| Per diagram, over all 215 | ferrigura | Pele (Node) | Pele (Chromium) | Mermaid (Chromium) |
| ------------------------- | --------: | ----------: | --------------: | -----------------: |
| Median                    |   1.59 ms |     0.05 ms |       < 0.1 ms¹ |            8.45 ms |
| 90th percentile           |   2.53 ms |     0.17 ms |          0.2 ms |            35.0 ms |
| Slowest                   |   35.0 ms |     0.70 ms |          0.5 ms |            97.5 ms |

¹ Chromium's timer resolution is 0.1 ms on a page that is not cross-origin isolated, which is below what Pele needs.

The median per type, in milliseconds:

| Type          | Fixtures | ferrigura | Pele (Node) | Mermaid |
| ------------- | -------: | --------: | ----------: | ------: |
| flowchart     |       11 |      1.86 |        0.15 |   23.35 |
| sequence      |       10 |      1.59 |        0.08 |   14.90 |
| class         |        9 |      2.03 |        0.08 |   34.15 |
| state         |       10 |      2.27 |        0.11 |   32.83 |
| state v2      |       10 |      1.84 |        0.10 |   23.02 |
| ER            |       10 |      1.67 |        0.07 |   17.70 |
| gantt         |       11 |      1.51 |        0.11 |    4.80 |
| timeline      |       10 |      1.95 |        0.04 |    5.63 |
| gitGraph      |       10 |      1.41 |        0.05 |    8.50 |
| journey       |       11 |      1.53 |        0.04 |    6.45 |
| pie           |       10 |      1.23 |        0.02 |    3.05 |
| info          |        2 |      1.13 |        0.00 |    1.85 |
| mindmap       |        4 |      1.88 |        0.04 |   23.65 |
| block         |       10 |      1.46 |        0.04 |   21.75 |
| packet        |        5 |      1.25 |        0.02 |    3.35 |
| xychart       |       10 |      2.12 |        0.05 |    7.72 |
| sankey        |        3 |      1.62 |        0.11 |    8.40 |
| quadrantChart |       10 |      1.41 |        0.04 |    5.75 |
| requirement   |        2 |      2.19 |        0.09 |   26.25 |
| C4            |       10 |      1.80 |        0.05 |   10.00 |
| kanban        |       10 |      1.54 |        0.02 |    6.10 |
| architecture  |       10 |      2.08 |        0.09 |   20.42 |
| treemap       |       10 |      1.61 |        0.05 |    4.57 |
| ishikawa      |       11 |      1.35 |        0.02 |    4.65 |
| eventmodeling |        6 |      1.31 |        0.04 |    4.25 |

The work is not the same either: ferrigura embeds a font subset and reproduces Mermaid's layout, Mermaid lays out and measures in the page, and Pele in Node estimates text widths and draws a simpler layout.

## Where the time goes

A CPU profile of each, on the same fixtures, shows that the two that render Mermaid's output spend most of their time on the configuration, not on drawing:

- **Mermaid** (Chromium's profiler, 475 renders of the `pr-check` fixtures of ten types): 57 percent of the time is in `assignWithDepth`, the deep merge that `getConfig()` runs to hand out a fresh copy of the whole configuration, theme variables included, every time a renderer, shape or label asks for it. Then ELK and the other layouts (17 percent), DOMPurify (10), style and layout work of the browser (6), and the DOM measurement (`getBBox()` and the like, with the layout it forces: 4). So measuring text with a canvas, as Pele does, would save Mermaid a few percent; handing out the configuration without a deep copy per call would save more than half.
- **ferrigura** (macOS `sample`, 4,000 renders of a small pie): 84 percent of the time is in resolving the configuration and the theme (`ferrigura_model::config` and `::theme`, building maps of the defaults on every call), 11 percent in drawing the pie and 7 in the font subset. A cache of the resolved defaults and themes should take most of the 1.2 ms off every render.
- **Pele** was not profiled here. It has no DOM, no theme to compute and no configuration of that size to merge (its page), which is where the other two spend their time.

## Adding a comparison

Add a page named after the project with the sections of the others (what it is, how it differs from ferrigura, what ferrigura can learn from it), a row in the table at the top and a column in the overview, and run the benchmark with it when it can render the same fixtures. Name the version and the date of every fact.
