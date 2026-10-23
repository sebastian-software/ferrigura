# mermaid-rs-renderer (mmdr)

[mermaid-rs-renderer](https://github.com/1jehuang/mermaid-rs-renderer), with its command `mmdr`, is a Rust renderer of Mermaid's syntax by Jeremy Huang, MIT-licensed and published as the crate `mermaid-rs-renderer` since 2026-01-23; version 0.3.1 of 2026-07-06, as of 2026-10-24. Its main branch is ahead of that release, with its last commit on 2026-09-07. It is widely used: about 200,000 downloads and 40 crates that depend on it, such as `mdbook-mermaid-mmdr`. Facts below are from its documentation and sources; nothing here was built or measured.

## What it is

mmdr is an implementation of its own, not a port: its parser, its layered layout and its SVG writer are written for it, with Dagre-style engines as an option. It names no Mermaid version it follows (its benchmarks use mermaid-cli 11.4.2), and it does not claim Mermaid's output: its README says the output may not match mermaid-cli in all cases yet. Its CI checks the drawing on its own terms, with no overlaps, no edge through a node and no edge end off its box; a comparison with mermaid-cli is a script for development.

It reads 23 diagram types, ZenUML among them; ishikawa, venn, wardley and the other newer types of Mermaid 12 are not. Labels are SVG text. Text is measured with the fonts installed on the machine (`fontdb` and `ttf-parser`), so the drawing depends on the machine, and a fixed width per character stands in when there is no font. Its default theme is its own, `modern`; Mermaid's themes and `themeVariables` are there too. It writes SVG and, through resvg, PNG.

Its headline is speed against mermaid-cli: 100 to 1,400 times faster by its own figures, for example 4.5 ms against 1,971 ms for a flowchart. These are whole processes, measured end to end, so most of the difference is the start of Chromium; for a flowchart of 200 nodes it gives 47 ms against 2,276 ms.

## How it differs from ferrigura

| Topic            | mmdr                                                                          | ferrigura                                                                                               |
| ---------------- | ----------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| Goal             | Mermaid's syntax, its own drawing; Mermaid's output not claimed               | Mermaid's output: the same geometry within 0.05 px on every comparable fixture                          |
| Mermaid version  | None named; Mermaid 11 in its tools                                           | 12.2.0                                                                                                  |
| How it checks    | Invariants of the drawing in CI; mermaid-cli compared by a development script | The models against the running reference on every input, the geometry in Chromium, hashes of its output |
| Diagram types    | 23                                                                            | 33 of 33 families of Mermaid 12.2.0                                                                     |
| Layout           | Its own layered layout; Dagre-style engines as an option                      | Mermaid's engines, ported: ELK Layered, cose-bilkent, fcose, the tidy tree                              |
| Text measurement | The fonts installed on the machine; the output differs between machines       | The bundled faces, shaped with HarfRust; the same on every host                                         |
| Fonts            | The viewer's                                                                  | Embedded in the SVG as a subset of the glyphs used                                                      |
| Themes           | Its own `modern` by default; Mermaid's themes and `themeVariables`            | `redux-color` and `redux-dark-color` as CSS custom properties                                           |
| Output           | SVG, PNG                                                                      | SVG                                                                                                     |
| Runs in          | Rust: a library and a CLI                                                     | Rust: a library, a CLI and a native Node addon                                                          |
| Licence          | MIT                                                                           | MIT; ported parts under EPL-2.0 (ELK), Apache-2.0 and BSD-3-Clause                                      |

## What ferrigura can learn from it

- Invariants of the drawing as a CI gate (no overlaps, no edge through a node) check what a comparison with Mermaid cannot, where Mermaid's own drawing is wrong.
- `--timing`, a breakdown of the stages of a render, makes performance work easier to start.
- A library build without the command line and PNG output (`default-features = false`) keeps the dependencies of an embedding small.
