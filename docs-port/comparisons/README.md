# Comparisons

How ferrigura stands next to other renderers of Mermaid's syntax. Each project has a page of its own: what it is, how it differs from ferrigura, and what ferrigura can learn from it. This page holds the overview and the measurements. Facts are from each project's documentation and sources at the version named, as of 2026-10-24; measured values say how they were measured.

| Project                          | What it is                                                                                     |
| -------------------------------- | ---------------------------------------------------------------------------------------------- |
| [Mermaid](./mermaid.md)          | The original, and the reference ferrigura is compared against                                  |
| [Pele](./pele.md)                | An independent, fast renderer of Mermaid's syntax, for Obsidian                                |
| [merman](./merman.md)            | A headless Rust implementation of Mermaid, aimed at its output, with many outputs and bindings |
| [mermaid-rs-renderer](./mmdr.md) | A fast Rust renderer of Mermaid's syntax with a layout of its own                              |

## Overview

|                        | ferrigura                                                                                | Mermaid                                                     | Pele                                                         | merman                                                             | mermaid-rs-renderer                                 |
| ---------------------- | ---------------------------------------------------------------------------------------- | ----------------------------------------------------------- | ------------------------------------------------------------ | ------------------------------------------------------------------ | --------------------------------------------------- |
| Version                | 0.2.0 (2026-10-22)                                                                       | 12.2.0                                                      | 1.9.0                                                        | 0.9.2 (2026-10-21), after Mermaid 12.1.0                           | 0.3.1 (2026-07-06)                                  |
| By                     | Sebastian Software                                                                       | the mermaid-js project                                      | kepano, for Obsidian                                         | Latias94                                                           | Jeremy Huang                                        |
| Licence                | MIT; ported parts under EPL-2.0 (ELK), Apache-2.0 and BSD-3-Clause (llvm-libc, V8, Skia) | MIT                                                         | MIT                                                          | MIT OR Apache-2.0; EPL-2.0 for its ELK port                        | MIT                                                 |
| Language               | Rust                                                                                     | TypeScript                                                  | TypeScript                                                   | Rust                                                               | Rust                                                |
| Runs in                | A library, a CLI and a native Node addon; no browser, no WebAssembly                     | A browser, or a headless one on a server                    | Any JavaScript runtime, browser or server                    | A library, a CLI, WebAssembly, Node, Python and more               | A library and a CLI                                 |
| Needs a DOM            | No                                                                                       | Yes                                                         | No                                                           | No                                                                 | No                                                  |
| API                    | Synchronous, returns the SVG                                                             | Asynchronous, draws into the page                           | Synchronous, returns the SVG                                 | Synchronous, returns the SVG or another format                     | Synchronous, returns the SVG                        |
| Goal                   | Mermaid's output                                                                         | (the reference)                                             | Mermaid's syntax, its own drawing                            | Mermaid's output, without pixel identity                           | Mermaid's syntax, its own drawing                   |
| Output against Mermaid | The same geometry within 0.05 px on every comparable fixture                             | (the reference)                                             | Its own layout and style                                     | Compared as SVG DOM with mermaid-cli's output                      | Its own layout and style                            |
| Diagram families       | 33 of 33                                                                                 | 33                                                          | 33 (those of Mermaid 12.1.0)                                 | 37 by its count, the railroad dialects one by one                  | 23                                                  |
| Layout                 | Mermaid's engines, ported: ELK Layered, cose-bilkent, fcose, the tidy tree               | ELK (the default since 12.0), cose-bilkent, fcose, and more | Its own: layered, compound, lanes                            | Mermaid's engines, ported: dagre, ELK Layered, cose-bilkent, fcose | Its own layered layout                              |
| Text measurement       | The bundled faces, shaped with HarfRust and rounded as Chromium rounds                   | The DOM: `getBBox()`, `getComputedTextLength()`             | The canvas's `measureText`; an Arial width table without one | Deterministic heuristics; font files as an option                  | The fonts installed on the machine                  |
| Fonts                  | Embedded in the SVG as a subset of the glyphs used                                       | The page's                                                  | The page's                                                   | The viewer's                                                       | The viewer's                                        |
| Labels                 | SVG text                                                                                 | HTML in `foreignObject` by default                          | SVG text                                                     | HTML in `foreignObject` by default; SVG text as an option          | SVG text                                            |
| Colours                | CSS custom properties with attribute fallbacks; Mermaid's redux themes                   | Themes and `themeVariables`                                 | CSS custom properties                                        | Mermaid's themes and `themeVariables`                              | Its own `modern` theme by default; Mermaid's themes |
| Output formats         | SVG                                                                                      | SVG; PNG and PDF through mermaid-cli                        | SVG                                                          | SVG, PNG, JPEG, PDF, ASCII                                         | SVG, PNG                                            |
| Narrow screens         | Scales the diagram                                                                       | Scales the diagram                                          | Adapts the layout                                            | Not checked                                                        | Not checked                                         |
| Size, gzip (raw)       | 7.2 MB (17.9 MB), the native addon, fonts included                                       | 1.57 MB (5.5 MB), `mermaid.min.js`                          | 261 kB (702 kB), `pele.js`                                   | Not measured here                                                  | Not measured here                                   |
| Direct dependencies    | 14 crates                                                                                | 23                                                          | 0                                                            | Not counted here                                                   | 9 crates (12 with its default CLI and PNG)          |
| Render time, median    | 0.70 ms, through a reused renderer                                                       | 7.8 ms                                                      | 0.05 ms                                                      | Not measured here                                                  | Not measured here                                   |

The sizes are not alike: ferrigura's is a native library for one platform, with its text engine and 1.4 MB of fonts, where Mermaid and Pele are JavaScript that the browser brings the rest to. merman and mermaid-rs-renderer are not measured here yet: their facts are from their documentation and sources, and the benchmark below runs on JavaScript and the Node addon.

## Render time

Measured with [`tools/port/compare-bench.ts`](../../tools/port/compare-bench.ts) on the 215 `pr-check` fixtures of the 25 types all three render: each time is the median of 30 renders of one fixture (10 for Mermaid) after a warm-up. ferrigura ran through its Node addon (release build) in two ways: `render`, which takes the configuration with each call, and a `Renderer` made once for the configuration and reused for every render. Pele ran in Node (with its width table) and in Chromium (with the canvas), Mermaid in Chromium with its defaults. Every fixture rendered with all three. Machine: Apple M1 Ultra, macOS 27.0.1, Node 24.22.0, Chromium 151.0.7922.34. Run of 2026-10-24, with ferrigura 0.2.0, Pele 1.9.0 and Mermaid 12.2.0; two runs earlier that night, under the load of a backup, gave ferrigura medians up to 0.84 ms and left Pele and Mermaid where they are.

| Per diagram, over all 215 | ferrigura | ferrigura (Renderer) | Pele (Node) | Pele (Chromium) | Mermaid (Chromium) |
| ------------------------- | --------: | -------------------: | ----------: | --------------: | -----------------: |
| Median                    |   0.70 ms |              0.70 ms |     0.05 ms |       < 0.1 ms¹ |            7.75 ms |
| 90th percentile           |   2.01 ms |              2.01 ms |     0.15 ms |         0.10 ms |            33.3 ms |
| Slowest                   |   34.2 ms |              36.6 ms |     0.61 ms |         0.60 ms |            93.3 ms |

The two ferrigura columns are the same now: the shorthand's cache of the resolved configuration costs no more than a `Renderer` made once (on 2026-10-18, `render` took 0.73 ms for a fixture without configuration, against 0.34 ms for the `Renderer`). Of the 215 fixtures, 103 carry a directive or a configuration block, which each render merges on top of the resolved configuration, as the decision keeps: 0.93 ms in the median. The other 112 use the resolved configuration as it is: 0.37 ms. Against the run of 2026-10-18, before the P3 types, the swimlanes and the emoji face came in, the median rose from 0.66 to 0.70 ms and the 90th percentile from 1.61 to 2.01 ms, the state diagrams the most (1.60 to 2.13 ms); what costs the difference is not measured.

¹ Chromium's timer resolution is 0.1 ms on a page that is not cross-origin isolated, which is below what Pele needs.

The median per type, in milliseconds:

| Type          | Fixtures | ferrigura | ferrigura (Renderer) | Pele (Node) | Mermaid |
| ------------- | -------: | --------: | -------------------: | ----------: | ------: |
| flowchart     |       11 |      1.00 |                 1.01 |        0.12 |   21.50 |
| sequence      |       10 |      0.79 |                 0.78 |        0.07 |   13.48 |
| class         |        9 |      1.22 |                 1.22 |        0.07 |   32.05 |
| state         |       10 |      2.15 |                 2.13 |        0.11 |   31.80 |
| state v2      |       10 |      1.57 |                 1.57 |        0.08 |   22.90 |
| ER            |       10 |      0.88 |                 0.88 |        0.07 |   16.80 |
| gantt         |       11 |      0.41 |                 0.41 |        0.10 |    4.55 |
| timeline      |       10 |      1.09 |                 1.10 |        0.03 |    5.30 |
| gitGraph      |       10 |      0.48 |                 0.48 |        0.04 |    7.92 |
| journey       |       11 |      0.74 |                 0.73 |        0.04 |    6.30 |
| pie           |       10 |      0.38 |                 0.38 |        0.02 |    2.60 |
| info          |        2 |      0.06 |                 0.06 |        0.01 |    1.73 |
| mindmap       |        4 |      1.06 |                 1.05 |        0.04 |   22.18 |
| block         |       10 |      0.36 |                 0.36 |        0.04 |   19.80 |
| packet        |        5 |      0.29 |                 0.29 |        0.02 |    2.55 |
| xychart       |       10 |      0.82 |                 0.82 |        0.05 |    7.18 |
| sankey        |        3 |      0.80 |                 0.80 |        0.11 |    7.75 |
| quadrantChart |       10 |      0.27 |                 0.27 |        0.03 |    5.05 |
| requirement   |        2 |      1.32 |                 1.30 |        0.10 |   22.57 |
| C4            |       10 |      0.66 |                 0.66 |        0.06 |    8.85 |
| kanban        |       10 |      0.36 |                 0.36 |        0.02 |    5.37 |
| architecture  |       10 |      0.95 |                 0.99 |        0.08 |   18.60 |
| treemap       |       10 |      0.49 |                 0.48 |        0.05 |    4.28 |
| ishikawa      |       11 |      0.38 |                 0.38 |        0.01 |    4.10 |
| eventmodeling |        6 |      0.24 |                 0.24 |        0.04 |    3.70 |

The work is not the same either: ferrigura embeds a font subset and reproduces Mermaid's layout, Mermaid lays out and measures in the page, and Pele in Node estimates text widths and draws a simpler layout.

## Where the time goes

A CPU profile of each shows where the time goes (profiles of 2026-10-17 and 2026-10-18). In Mermaid, and in ferrigura before step 2 of [decision 0009](../decisions/0009-configuration-resolved-once.md), much of the time went to the configuration, not to drawing:

- **Mermaid** (Chromium's profiler, 475 renders of the `pr-check` fixtures of ten types): 57 percent of the time is in `assignWithDepth`, the deep merge that `getConfig()` runs to hand out a fresh copy of the whole configuration, theme variables included, every time a renderer, shape or label asks for it. Then ELK and the other layouts (17 percent), DOMPurify (10), style and layout work of the browser (6), and the DOM measurement (`getBBox()` and the like, with the layout it forces: 4). So measuring text with a canvas, as Pele does, would save Mermaid a few percent; handing out the configuration without a deep copy per call would save more than half.
- **ferrigura** (macOS `sample`, 12 seconds of small pie renders through one `Renderer`, 0.13 ms each, with no directive): the configuration no longer shows in the profile. Before step 2, 63 percent of the time of such a render was in `prepare`, which merged the layers (`effective`) and read the typed view (`typed`) on every call. The render now takes the resolved configuration as it is, and about half of its samples are in the text crate, embedding the subset of the bundled faces that the labels use (the skera subsetter), which each SVG needs to be self-contained. A text with a directive or a configuration block still merges it on top in each render. In a profile of one such flowchart, `prepare` took 47 percent of the render, most of it in the typed view of the merged configuration (`typed`, the lenient reader and serde) and in the layers (`effective`). That is the next cost to take, with the typed sections of [decision 0009](../decisions/0009-configuration-resolved-once.md), step 3. Step 3 is done since 2026-10-19 (issue #4); its effect is not measured here.
- **Pele** was not profiled here. It has no DOM, no theme to compute and no configuration of that size to merge (its page), which is where the other two spend their time.

## Adding a comparison

Add a page named after the project with the sections of the others (what it is, how it differs from ferrigura, what ferrigura can learn from it), a row in the table at the top and a column in the overview, and run the benchmark with it when it can render the same fixtures. Name the version and the date of every fact.
