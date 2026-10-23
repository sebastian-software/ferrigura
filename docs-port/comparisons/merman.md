# merman

[merman](https://github.com/Latias94/merman) is a headless Rust implementation of Mermaid, "parity-focused" in its own words: it parses, lays out and renders Mermaid's syntax without a browser or a JavaScript runtime. It is mostly the work of one author (Latias94), MIT OR Apache-2.0 for its own code, with a translated ELK engine under EPL-2.0 in its default builds, and published as the crate `merman` and its siblings (`merman-core`, `merman-render`, `merman-cli`, ...) since 2026-02-23; version 0.9.2 of 2026-10-21, as of 2026-10-24. The editor Zed draws Mermaid in Markdown with it (since 2026-05). Facts below are from its documentation and sources at that version and its development branch; nothing here was built or measured.

## What it is

merman reimplements Mermaid in Rust. Its parsers follow Mermaid's Jison and Langium grammars, with lalrpop or hand-written lexers, and its layouts are ports of the engines Mermaid uses: dagre (`dugong`), cose-bilkent and fcose (`manatee`) and ELK Layered (`merman-elk-layered`), the default of several graph types since its 0.8.0. The hand-drawn look runs on a port of Rough.js with the same seeded randomness, and math labels on RaTeX. The published 0.9.x follows Mermaid 12.1.0; its development branch targets 12.2.0.

It checks its output against Mermaid: about 3,700 reference SVGs that Mermaid draws through mermaid-cli, compared as SVG DOM per diagram type in several modes, semantic and layout snapshots, and a Chromium check that the drawn content stays inside the viewport. It says plainly that text measurement, the browser's font fallback, HTML labels and the hand-drawn paths can make its output differ from the browser's, and that pixel-identical output is not its aim.

It counts 37 built-in diagram types, the four railroad dialects one by one; ZenUML is among them, outside its parity checks. Text is measured by deterministic heuristics, with no bundled fonts: a host can pass a measurer of its own, or font files to measure with. The default SVG keeps Mermaid's HTML labels in `foreignObject`; a "resvg-safe" pipeline draws them as SVG text, and the PNG, JPEG and PDF output uses it.

Beyond SVG, merman writes PNG, JPEG, PDF, ASCII and Unicode drawings for a terminal, and the semantic and layout models as JSON. It ships a CLI with a mode that takes `mmdc`'s options, a language server, a tree-sitter grammar, WebAssembly and Node packages, Python wheels, and bindings for Android, Apple platforms, Flutter, Typst and C. Every request carries cancellation, a deadline and resource limits. It moves fast: about 850 commits in the month up to 2026-10-24.

## How it differs from ferrigura

| Topic            | merman                                                                          | ferrigura                                                                                                          |
| ---------------- | ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| Goal             | Mermaid's output, without pixel identity                                        | Mermaid's output: the same geometry within 0.05 px on every comparable fixture                                     |
| Mermaid version  | 12.1.0 (0.9.2); 12.2.0 on its development branch                                | 12.2.0, checked against upstream at a pinned commit ([0010](../decisions/0010-upstream-reference-at-a-pin.md))     |
| How it checks    | Reference SVGs from mermaid-cli, compared as SVG DOM per type; layout snapshots | The models against the running reference on every input, the geometry in Chromium, hashes of its own output        |
| Diagram types    | 37 by its count, the railroad dialects one by one, ZenUML included              | 33 of 33 families of Mermaid 12.2.0                                                                                |
| Layout           | Ports of dagre, ELK Layered, cose-bilkent and fcose                             | Ports of ELK Layered, cose-bilkent, fcose and the tidy tree; no dagre                                              |
| Text measurement | Deterministic heuristics; a host's measurer or font files as options            | The bundled faces, shaped with HarfRust and rounded as Chromium rounds; the same on every host                     |
| Fonts            | None bundled; the viewer's fonts                                                | Embedded in the SVG as a subset of the glyphs used                                                                 |
| Labels           | HTML in `foreignObject` by default; SVG text in its resvg-safe pipeline         | SVG text                                                                                                           |
| Hand-drawn, math | A port of Rough.js; RaTeX                                                       | Not yet                                                                                                            |
| Output           | SVG, PNG, JPEG, PDF, ASCII and Unicode, JSON models                             | SVG                                                                                                                |
| Runs in          | Rust, a CLI, WebAssembly, Node, Python, mobile platforms, Typst, C              | Rust: a library, a CLI and a native Node addon; no WebAssembly ([0001](../decisions/0001-rust-only-no-browser.md)) |
| Licence          | MIT OR Apache-2.0; EPL-2.0 for the ELK port                                     | MIT; ported parts under EPL-2.0 (ELK), Apache-2.0 and BSD-3-Clause                                                 |

## What ferrigura can learn from it

- Output beyond SVG: PNG and PDF through resvg, and an ASCII drawing for a terminal, are what users of a command-line tool ask for next. ferrigura's SVG, with SVG text and its fonts embedded, is a good input for resvg.
- Cancellation, a deadline and resource limits on every request are a guard for untrusted input on a server.
- The Chromium check that the drawn content stays inside the viewport is a cheap invariant that ferrigura's geometry comparison does not state on its own.
- Its many bindings show where the demand is. WebAssembly is the case decision [0001](../decisions/0001-rust-only-no-browser.md) left out.
