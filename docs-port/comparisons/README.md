# Comparisons

How ferrigura stands next to other renderers of Mermaid's syntax. Each project has a page of its own: what it is, how it differs from ferrigura, and what ferrigura can learn from it. This page holds the overview and the measurements. Facts are from each project's documentation and sources at the version named, as of 2026-10-17; measured values say how they were measured.

| Project                 | What it is                                                      |
| ----------------------- | --------------------------------------------------------------- |
| [Mermaid](./mermaid.md) | The original, and the reference ferrigura is compared against   |
| [Pele](./pele.md)       | An independent, fast renderer of Mermaid's syntax, for Obsidian |

## Overview

|                        | ferrigura                                                                                | Mermaid                                                     | Pele                                                         |
| ---------------------- | ---------------------------------------------------------------------------------------- | ----------------------------------------------------------- | ------------------------------------------------------------ |
| Version                | 0.1.0 (2026-10-21); measured on `main` with the `Renderer` of 0009, step 2 (2026-10-18)  | 12.2.0                                                      | 1.6.0                                                        |
| By                     | Sebastian Software                                                                       | the mermaid-js project                                      | kepano, for Obsidian                                         |
| Licence                | MIT; ported parts under EPL-2.0 (ELK), Apache-2.0 and BSD-3-Clause (llvm-libc, V8, Skia) | MIT                                                         | MIT                                                          |
| Language               | Rust                                                                                     | TypeScript                                                  | TypeScript                                                   |
| Runs in                | A library, a CLI and a native Node addon; no browser, no WebAssembly                     | A browser, or a headless one on a server                    | Any JavaScript runtime, browser or server                    |
| Needs a DOM            | No                                                                                       | Yes                                                         | No                                                           |
| API                    | Synchronous, returns the SVG                                                             | Asynchronous, draws into the page                           | Synchronous, returns the SVG                                 |
| Goal                   | Mermaid's output                                                                         | (the reference)                                             | Mermaid's syntax, its own drawing                            |
| Output against Mermaid | The same geometry within 0.05 px on every comparable fixture                             | (the reference)                                             | Its own layout and style                                     |
| Diagram families       | 33 of 33                                                                                 | 33                                                          | 33 (those of Mermaid 12.1.0)                                 |
| Layout                 | Mermaid's engines, ported: ELK Layered, cose-bilkent, fcose, the tidy tree               | ELK (the default since 12.0), cose-bilkent, fcose, and more | Its own: layered, compound, lanes                            |
| Text measurement       | The bundled faces, shaped with HarfRust and rounded as Chromium rounds                   | The DOM: `getBBox()`, `getComputedTextLength()`             | The canvas's `measureText`; an Arial width table without one |
| Fonts                  | Embedded in the SVG as a subset of the glyphs used                                       | The page's                                                  | The page's                                                   |
| Labels                 | SVG text                                                                                 | HTML in `foreignObject` by default                          | SVG text                                                     |
| Colours                | CSS custom properties with attribute fallbacks; Mermaid's redux themes                   | Themes and `themeVariables`                                 | CSS custom properties                                        |
| Narrow screens         | Scales the diagram                                                                       | Scales the diagram                                          | Adapts the layout                                            |
| Size, gzip (raw)       | 5.0 MB (11.6 MB), the native addon, fonts included                                       | 1.57 MB (5.5 MB), `mermaid.min.js`                          | 251 kB (678 kB), `pele.js`                                   |
| Direct dependencies    | 14 crates                                                                                | 23                                                          | 0                                                            |
| Render time, median    | 0.66 ms, through a reused renderer                                                       | 9.3 ms                                                      | 0.05 ms                                                      |

The sizes are not alike: ferrigura's is a native library for one platform, with its text engine and 0.5 MB of fonts, where the other two are JavaScript that the browser brings the rest to.

## Render time

Measured with [`tools/port/compare-bench.ts`](../../tools/port/compare-bench.ts) on the 215 `pr-check` fixtures of the 25 types all three render: each time is the median of 30 renders of one fixture (10 for Mermaid) after a warm-up. ferrigura ran through its Node addon (release build) in two ways: `render`, which takes the configuration with each call, and a `Renderer` made once for the configuration and reused for every render. Pele ran in Node (with its width table) and in Chromium (with the canvas), Mermaid in Chromium with its defaults. Every fixture rendered with all three. Machine: Apple M1 Ultra, macOS 27.0.1, Node 24.21.0, Chromium 151.0.7922.34. The ferrigura figures are of the build with the `Renderer` ([decision 0009](../decisions/0009-configuration-resolved-once.md), step 2). On the same machine before this step, `render` had a median of 0.84 ms; the cache of step 1 had brought it from 1.64 ms.

| Per diagram, over all 215 | ferrigura | ferrigura (Renderer) | Pele (Node) | Pele (Chromium) | Mermaid (Chromium) |
| ------------------------- | --------: | -------------------: | ----------: | --------------: | -----------------: |
| Median                    |   0.68 ms |              0.66 ms |     0.05 ms |       < 0.1 ms¹ |            9.30 ms |
| 90th percentile           |   1.58 ms |              1.61 ms |     0.17 ms |         0.20 ms |            39.1 ms |
| Slowest                   |   34.7 ms |              34.7 ms |     0.63 ms |         0.50 ms |           107.3 ms |

The two ferrigura columns differ only by the shorthand's lookup in its cache (a comparison of the configuration and a lock) and by the noise of the run. Of the 215 fixtures, 103 carry a directive or a configuration block: their renders merge it on top of the resolved configuration, in each render, as the decision keeps, so their median fell only from 0.96 to 0.87 ms. The other 112 use the resolved configuration as it is: from 0.73 to 0.34 ms.

¹ Chromium's timer resolution is 0.1 ms on a page that is not cross-origin isolated, which is below what Pele needs.

The median per type, in milliseconds:

| Type          | Fixtures | ferrigura | ferrigura (Renderer) | Pele (Node) | Mermaid |
| ------------- | -------: | --------: | -------------------: | ----------: | ------: |
| flowchart     |       11 |      0.99 |                 0.93 |        0.15 |   24.60 |
| sequence      |       10 |      0.71 |                 0.70 |        0.08 |   15.72 |
| class         |        9 |      1.15 |                 1.09 |        0.08 |   38.10 |
| state         |       10 |      1.57 |                 1.60 |        0.11 |   36.55 |
| state v2      |       10 |      1.18 |                 1.19 |        0.09 |   25.45 |
| ER            |       10 |      0.82 |                 0.82 |        0.07 |   19.63 |
| gantt         |       11 |      0.39 |                 0.38 |        0.10 |    5.90 |
| timeline      |       10 |      1.05 |                 1.05 |        0.04 |    6.33 |
| gitGraph      |       10 |      0.33 |                 0.34 |        0.04 |    9.30 |
| journey       |       11 |      0.68 |                 0.67 |        0.04 |    7.20 |
| pie           |       10 |      0.29 |                 0.29 |        0.02 |    3.13 |
| info          |        2 |      0.06 |                 0.06 |        0.01 |    2.00 |
| mindmap       |        4 |      0.99 |                 0.99 |        0.04 |   25.43 |
| block         |       10 |      0.34 |                 0.35 |        0.04 |   23.43 |
| packet        |        5 |      0.25 |                 0.26 |        0.02 |    3.50 |
| xychart       |       10 |      0.83 |                 0.80 |        0.05 |    8.65 |
| sankey        |        3 |      0.77 |                 0.75 |        0.12 |    9.10 |
| quadrantChart |       10 |      0.29 |                 0.28 |        0.04 |    6.32 |
| requirement   |        2 |      1.21 |                 1.17 |        0.09 |   27.65 |
| C4            |       10 |      0.64 |                 0.65 |        0.06 |   11.75 |
| kanban        |       10 |      0.35 |                 0.36 |        0.02 |    7.23 |
| architecture  |       10 |      0.90 |                 0.90 |        0.09 |   23.07 |
| treemap       |       10 |      0.48 |                 0.47 |        0.05 |    4.83 |
| ishikawa      |       11 |      0.34 |                 0.34 |        0.02 |    4.85 |
| eventmodeling |        6 |      0.22 |                 0.22 |        0.05 |    4.38 |

The work is not the same either: ferrigura embeds a font subset and reproduces Mermaid's layout, Mermaid lays out and measures in the page, and Pele in Node estimates text widths and draws a simpler layout.

## Where the time goes

A CPU profile of each shows where the time goes. In Mermaid, and in ferrigura before this step, much of the time went to the configuration, not to drawing:

- **Mermaid** (Chromium's profiler, 475 renders of the `pr-check` fixtures of ten types): 57 percent of the time is in `assignWithDepth`, the deep merge that `getConfig()` runs to hand out a fresh copy of the whole configuration, theme variables included, every time a renderer, shape or label asks for it. Then ELK and the other layouts (17 percent), DOMPurify (10), style and layout work of the browser (6), and the DOM measurement (`getBBox()` and the like, with the layout it forces: 4). So measuring text with a canvas, as Pele does, would save Mermaid a few percent; handing out the configuration without a deep copy per call would save more than half.
- **ferrigura** (macOS `sample`, 12 seconds of small pie renders through one `Renderer`, 0.13 ms each, with no directive): the configuration no longer shows in the profile. Before this step, 63 percent of the time of such a render was in `prepare`, which merged the layers (`effective`) and read the typed view (`typed`) on every call. The render now takes the resolved configuration as it is, and about half of its samples are in the text crate, embedding the subset of the bundled faces that the labels use (the skera subsetter), which each SVG needs to be self-contained. A text with a directive or a configuration block still merges it on top in each render. In a profile of one such flowchart, `prepare` took 47 percent of the render, most of it in the typed view of the merged configuration (`typed`, the lenient reader and serde) and in the layers (`effective`). That is the next cost to take, with the typed sections of [decision 0009](../decisions/0009-configuration-resolved-once.md), step 3. Step 3 is done since 2026-10-19 (issue #4); its effect is not measured here.
- **Pele** was not profiled here. It has no DOM, no theme to compute and no configuration of that size to merge (its page), which is where the other two spend their time.

## Adding a comparison

Add a page named after the project with the sections of the others (what it is, how it differs from ferrigura, what ferrigura can learn from it), a row in the table at the top and a column in the overview, and run the benchmark with it when it can render the same fixtures. Name the version and the date of every fact.
