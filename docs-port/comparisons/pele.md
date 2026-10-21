# Pele

[Pele](https://pele.run/) is an independent renderer of Mermaid syntax, written by kepano for [Obsidian](https://obsidian.md/) to render diagrams quickly and in the app's theme. It is TypeScript with no dependencies, MIT-licensed, and published as the npm package `pele` (repository [`obsidianmd/pele`](https://github.com/obsidianmd/pele), created on 2026-10-05; version 1.6.0 as of 2026-10-17). Facts below are from its documentation and sources at that version, unless they are marked as measured here.

## What it is

Pele reads the Mermaid syntax of every diagram type built into Mermaid 12.1.0 (33 families) and draws them in its own layout and its own visual style. It says so plainly: node positions, spacing and edge paths differ from Mermaid, Mermaid's themes and `themeVariables` do not apply, and labels are SVG text with line breaks, bold and italic, with no HTML. Its goal is a fast renderer with Mermaid's input language, not Mermaid's output.

The API is synchronous. `render(source, options)` returns the SVG as a string with its natural size; `parse()` gives a model that can be rendered at several widths; `mount(element, source)` renders into an element and redraws when the container resizes or web fonts arrive. Diagram types can be imported one by one (`pele/diagrams/<type>`) or loaded on first use (`pele/lazy`).

## Why it is fast

Pele reports 0.05 to 0.3 ms per diagram, against 6 to 82 ms for Mermaid.js, and 230 kB against 1.6 MB gzipped (its own figures). Measured here (the [overview](./README.md) has the method): 0.05 ms in the median over 215 fixtures in Node, 0.17 ms at the 90th percentile, against 8.5 ms for Mermaid in Chromium and 1.6 ms for ferrigura's Node addon (the first run, of 2026-10-17; the overview's table is a later run, of 2026-10-18, with ferrigura's `Renderer`); `pele.js` with every diagram type is 678 kB, 251 kB gzipped. The sources show where the time goes, or rather does not:

- **No DOM while rendering.** The SVG is built as a string (`src/svg/builder.ts`). Mermaid inserts every label into the page and measures it with `getBBox()` or `getComputedTextLength()`, which makes the browser lay out the page each time; that is the largest cost Pele avoids.
- **Cheap text measurement.** A browser measures with the 2D canvas's `measureText`, which needs no layout. Elsewhere, a built-in table of Arial advances for the 95 printable ASCII characters estimates the width, with a fixed width for wide (CJK) and other characters (`src/text/measurer.ts`). An app can pass its own measurer.
- **Its own, light layouts.** Three modules carry the diagrams: a layered, Sugiyama-style layout of a flat graph (`src/layout/layered.ts`), compound graphs (`compound.ts`) and lanes (`lanes.ts`). There is no dagre, ELK or cytoscape, and no iterative force-directed layout such as the cose-bilkent and fcose runs Mermaid makes for mindmaps and architecture diagrams.
- **No theme pipeline.** Colours are CSS custom properties (`--pele-bg`, `--pele-fg`, `--pele-series-1` to `-8`, ...) set on the SVG or its container, so a colour change needs no new render. Mermaid computes the theme's variables, merges configurations and runs DOMPurify for each render.

## How it differs from ferrigura

| Topic            | Pele                                                                                  | ferrigura                                                                                                         |
| ---------------- | ------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Goal             | Mermaid's syntax, its own drawing                                                     | Mermaid's drawing: the same geometry within 0.05 px on every comparable fixture                                   |
| Layout           | Its own layered, compound and lane layouts                                            | Mermaid's engines, ported: ELK Layered, cose-bilkent, fcose, the tidy tree, and each type's own renderer          |
| Text measurement | Canvas in a browser, an Arial width table elsewhere; depends on the fonts of the host | The bundled faces, shaped with HarfRust and rounded as Chromium rounds; the same on every host                    |
| Fonts            | The page's fonts (`--pele-font`); `mount()` measures again when they change           | Lexica Ultralegible and Atkinson Hyperlegible Mono, embedded as a subset, so every viewer draws what was measured |
| Colours          | CSS custom properties, eight series colours                                           | CSS custom properties with presentation attributes as a fallback (decision 0007); Mermaid's redux themes          |
| Small screens    | Adapts the layout: `maxWidth`, and a vertical layout below 640 px (`autoDirection`)   | Scales the diagram, as Mermaid does                                                                               |
| Runs in          | Any JavaScript runtime, browser or server, no DOM                                     | Rust: a library, a CLI and a native Node addon; no browser and no WebAssembly (decision 0001)                     |
| Diagram types    | All 33 of Mermaid 12.1.0                                                              | 33 of 33 (P1, P2, P3 and swimlanes), those of Mermaid 12.2.0                                                      |
| Security         | Escapes text, filters links, images and styles, limits source, output and edges       | DOMPurify's rules on the texts, as the reference runs them; no scripts or callbacks                               |

## What ferrigura can learn from it

- The responsive layout: rendering a model at the container's width, and switching a wide `LR` flowchart to vertical on a narrow screen, is a feature Mermaid lacks. ferrigura could offer it as an option outside the parity mode.
- The ordered list of aims for edge routing at the top of `layered.ts` (lines apart from each other, few crossings, short and simple paths, ends at the middle of a side, balance, square ends, source order, compactness) is a clear statement of what a good layered drawing is.
- Limits on source length, output size and edge count, with a dedicated error code, are a simple guard for untrusted input.
