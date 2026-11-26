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
| Version                | 0.8.2 (2026-11-21)                                                                       | 12.2.0                                                      | 1.9.0                                                        | 0.9.2 (2026-10-21), after Mermaid 12.1.0                           | 0.3.1 (2026-07-06)                                  |
| By                     | Sebastian Software                                                                       | the mermaid-js project                                      | kepano, for Obsidian                                         | Latias94                                                           | Jeremy Huang                                        |
| License                | MIT; ported parts under EPL-2.0 (ELK), Apache-2.0 and BSD-3-Clause (llvm-libc, V8, Skia) | MIT                                                         | MIT                                                          | MIT OR Apache-2.0; EPL-2.0 for its ELK port                        | MIT                                                 |
| Language               | Rust                                                                                     | TypeScript                                                  | TypeScript                                                   | Rust                                                               | Rust                                                |
| Runs in                | A library, a CLI and a native Node addon; no browser, no WebAssembly                     | A browser, or a headless one on a server                    | Any JavaScript runtime, browser or server                    | A library, a CLI, WebAssembly, Node, Python and more               | A library and a CLI                                 |
| Needs a DOM            | No                                                                                       | Yes                                                         | No                                                           | No                                                                 | No                                                  |
| API                    | Synchronous, returns the SVG                                                             | Asynchronous, draws into the page                           | Synchronous, returns the SVG                                 | Synchronous, returns the SVG or another format                     | Synchronous, returns the SVG                        |
| Goal                   | Mermaid's output                                                                         | (the reference)                                             | Mermaid's syntax, its own drawing                            | Mermaid's output, without pixel identity                           | Mermaid's syntax, its own drawing                   |
| Output against Mermaid | The same geometry within 0.05 px on every comparable fixture                             | (the reference)                                             | Its own layout and style                                     | Compared as SVG DOM with mermaid-cli's output                      | Its own layout and style                            |
| Diagram families       | 10 of 33, each one completely                                                            | 33                                                          | 33 (those of Mermaid 12.1.0)                                 | 37 by its count, the railroad dialects one by one                  | 23                                                  |
| Layout                 | Mermaid's engines, ported: ELK Layered, cose-bilkent, the tidy tree                      | ELK (the default since 12.0), cose-bilkent, fcose, and more | Its own: layered, compound, lanes                            | Mermaid's engines, ported: dagre, ELK Layered, cose-bilkent, fcose | Its own layered layout                              |
| Text measurement       | Local registered faces or bundled defaults; HarfRust and Chromium-aligned metrics        | The DOM: `getBBox()`, `getComputedTextLength()`             | The canvas's `measureText`; an Arial width table without one | Deterministic heuristics; font files as an option                  | The fonts installed on the machine                  |
| Fonts                  | Embedded: TrueType subsets; complete CFF/CFF2 faces                                      | The page's                                                  | The page's                                                   | The viewer's                                                       | The viewer's                                        |
| Labels                 | SVG text                                                                                 | HTML in `foreignObject` by default                          | SVG text                                                     | HTML in `foreignObject` by default; SVG text as an option          | SVG text                                            |
| Colors                 | CSS custom properties with attribute fallbacks; Mermaid's redux themes                   | Themes and `themeVariables`                                 | CSS custom properties                                        | Mermaid's themes and `themeVariables`                              | Its own `modern` theme by default; Mermaid's themes |
| Output formats         | SVG                                                                                      | SVG; PNG and PDF through mermaid-cli                        | SVG                                                          | SVG, PNG, JPEG, PDF, ASCII                                         | SVG, PNG                                            |
| Narrow screens         | Scales the diagram                                                                       | Scales the diagram                                          | Adapts the layout                                            | Not checked                                                        | Not checked                                         |
| Size, gzip (raw)       | 5.3 MB (10.5 MB), the native addon, fonts included                                       | 1.57 MB (5.5 MB), `mermaid.min.js`                          | 261 kB (702 kB), `pele.js`                                   | Not measured here                                                  | Not measured here                                   |
| Direct dependencies    | 14 crates                                                                                | 23                                                          | 0                                                            | Not counted here                                                   | 9 crates (12 with its default CLI and PNG)          |
| Render time, median    | 0.15 ms, through a reused renderer                                                       | 8.7 ms                                                      | 0.05 ms                                                      | Not measured here                                                  | Not measured here                                   |

The ferrigura size and timing are those of `main` on 2026-11-23, two days after 0.8.2, measured as the render time below says. Hosts supply local bytes or files and explicit fallback faces. Matching viewer layout requires glyph coverage; uncovered characters are measured with the primary missing glyph, but the viewer may substitute a system font. Additional script layout needs its own verification ([font inputs](../fonts.md)).

The sizes are not alike: ferrigura's is a native library for one platform, with its text engine and 1.4 MB of fonts, where Mermaid and Pele are JavaScript that the browser brings the rest to. merman and mermaid-rs-renderer are not measured here yet: their facts are from their documentation and sources, and the benchmark below runs on JavaScript and the Node addon.

## Render time

Measured with [`tools/port/compare-bench.ts`](../../tools/port/compare-bench.ts) on the 215 `pr-check` fixtures of the 25 types all three render: each time is the median of 30 renders of one fixture (10 for Mermaid) after a warm-up. ferrigura ran through its Node addon (release build) in two ways: `render`, which takes the configuration with each call, and a `Renderer` made once for the configuration and reused for every render. Pele ran in Node (with its width table) and in Chromium (with the canvas), Mermaid in Chromium with its defaults. Every fixture rendered with all three. Machine: Apple M1 Pro, macOS 27.2.1, Node 24.15.0, Chromium 159.0.8154.6. Run of 2026-11-23, with ferrigura's `main` of that day (after 0.8.2), Pele 1.9.0 and Mermaid 12.2.0.

| Per diagram, over all 215 | ferrigura | ferrigura (Renderer) | Pele (Node) | Pele (Chromium) | Mermaid (Chromium) |
| ------------------------- | --------: | -------------------: | ----------: | --------------: | -----------------: |
| Median                    |   0.15 ms |              0.15 ms |     0.05 ms |       < 0.1 ms¹ |            8.65 ms |
| 90th percentile           |   0.46 ms |              0.45 ms |     0.15 ms |         0.10 ms |            35.6 ms |
| Slowest                   |   5.95 ms |              6.01 ms |     0.68 ms |         0.40 ms |            97.7 ms |

The two ferrigura columns are the same: the shorthand's cache of the resolved configuration costs no more than a `Renderer` made once. Of the 215 fixtures, 103 carry a directive or a configuration block, which is merged on top of the resolved configuration, as the decision keeps: 0.18 ms in the median. The other 112 use the resolved configuration as it is: 0.10 ms. A renderer keeps the merged configuration of the last eight distinct directives, and the benchmark renders each fixture 30 times, so these times are those of a repeated render; the first render of a text with a directive merges it once and takes longer. On the same machine and day, `main` at commit `a3dddb1`, before the allocator of the addon, the trigonometry, the property lookups of the ELK layout, the shape plans, the directive cache and the SVG writer were reworked, took 0.43 ms in the median, 1.21 ms at the 90th percentile and 7.99 ms for the slowest.

¹ Chromium's timer resolution is 0.1 ms on a page that is not cross-origin isolated, which is below what Pele needs.

The median per type that ferrigura draws, in milliseconds (the overall figures above cover all 25 types of the run):

| Type          | Fixtures | ferrigura | ferrigura (Renderer) | Pele (Node) | Mermaid |
| ------------- | -------: | --------: | -------------------: | ----------: | ------: |
| flowchart     |       11 |      0.22 |                 0.20 |        0.13 |   22.55 |
| sequence      |       10 |      0.22 |                 0.21 |        0.07 |   15.17 |
| class         |        9 |      0.29 |                 0.30 |        0.07 |   35.55 |
| state         |       10 |      0.44 |                 0.42 |        0.11 |   34.18 |
| state v2      |       10 |      0.28 |                 0.26 |        0.07 |   23.77 |
| ER            |       10 |      0.20 |                 0.20 |        0.07 |   18.70 |
| gantt         |       11 |      0.17 |                 0.16 |        0.10 |    5.00 |
| pie           |       10 |      0.04 |                 0.04 |        0.02 |    2.90 |
| mindmap       |        4 |      0.30 |                 0.29 |        0.04 |   23.98 |
| xychart       |       10 |      0.32 |                 0.32 |        0.04 |    8.12 |
| quadrantChart |       10 |      0.08 |                 0.08 |        0.03 |    5.83 |

The work is not the same either: ferrigura embeds a font subset and reproduces Mermaid's layout, Mermaid lays out and measures in the page, and Pele in Node estimates text widths and draws a simpler layout.

## Where the time goes

A CPU profile of each shows where the time goes (profiles of 2026-10-17 and 2026-10-18, before the run above). In Mermaid, and in ferrigura before step 2 of [decision 0008](../decisions/0008-canonical-model-and-typed-views.md), much of the time went to the configuration, not to drawing:

- **Mermaid** (Chromium's profiler, 475 renders of the `pr-check` fixtures of ten types): 57 percent of the time is in `assignWithDepth`, the deep merge that `getConfig()` runs to hand out a fresh copy of the whole configuration, theme variables included, every time a renderer, shape or label asks for it. Then ELK and the other layouts (17 percent), DOMPurify (10), style and layout work of the browser (6), and the DOM measurement (`getBBox()` and the like, with the layout it forces: 4). So measuring text with a canvas, as Pele does, would save Mermaid a few percent; handing out the configuration without a deep copy per call would save more than half.
- **ferrigura** (macOS `sample`, 12 seconds of small pie renders through one `Renderer`, 0.13 ms each, with no directive): the configuration no longer shows in the profile. Before step 2, 63 percent of the time of such a render was in `prepare`, which merged the layers (`effective`) and read the typed view (`typed`) on every call. The render now takes the resolved configuration as it is, and about half of its samples are in the text crate, embedding the subset of the bundled faces that the labels use (the skera subsetter), which each SVG needs to be self-contained. A text with a directive or a configuration block merges it on top on its first render; the renderer keeps the result for the next.
- **Pele** was not profiled here. It has no DOM, no theme to compute and no configuration of that size to merge (its page), which is where the other two spend their time.

## Adding a comparison

Add a page named after the project with the sections of the others (what it is, how it differs from ferrigura, what ferrigura can learn from it), a row in the table at the top and a column in the overview, and run the benchmark with it when it can render the same fixtures. Name the version and the date of every fact.
