# Changelog

## 0.3.0 (2026-10-27)

The first public baseline. ferrigura renders the diagram text of all 33 diagram types of Mermaid 12.2.0 to SVG, in Rust and without a browser:

* the library `ferrigura` and the command-line tool `ferrigura-cli` on crates.io, and a Node.js addon with its npm packages in `node/`
* the layouts ported from their sources: ELK Layered, cose-bilkent, fcose, the tidy tree, swimlanes and Venn diagrams
* text measured with embedded fonts, so the output is the same on every machine
* parsers checked against Mermaid 12.2.0 model by model, and the geometry of the output checked against Mermaid 12.2.0 in Chromium
