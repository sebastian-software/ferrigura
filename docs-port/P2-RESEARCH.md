# P2 research: how a research task works

Each research task reads the TypeScript reference for one diagram type or one layout engine and writes `docs-port/<topic>/NOTES.md`, the basis for the parser, engine and renderer tasks that follow. The P1 notes are the model: `docs-port/journey/NOTES.md`, `docs-port/gantt/NOTES.md` and `docs-port/class/NOTES.md`. The decisions that apply are in [P2-PLAN.md](./P2-PLAN.md) and [M6-PLAN.md](./M6-PLAN.md).

## Rules

- Read only. Write nothing but your own `NOTES.md` (and probe scripts outside the repository, in your scratch directory). Do not commit; the coordinator commits.
- Every rule cites the reference's file and line (`packages/mermaid/src/diagrams/<type>/<file>:<line>`). Mark what you only inferred as **unverified**, and what you checked by running the reference as **probed**, with how.
- Probes run the reference's own code: the built bundle `packages/mermaid/dist` in Chromium (see `tools/port/geometry/common.ts` for how the harness loads it), the source with `tsx` for parser and DB code, or the libraries in `node_modules` directly. Do not reimplement the reference to learn what it does.
- Write in English, in plain sentences; tables where they help.

## Sections of a type's notes

0. **Summary**: what the type is, how big the port is, the riskiest parts.
1. **Sources**: the files that matter (grammar, DB, renderer, styles, detector, types, specs), with their line ranges and what each holds.
2. **Parser**: Jison or Langium; the lexer states and tokens, the grammar rules, what the DB receives from each, the preprocessing (`preprocessDiagram`, `encodeEntities`, comment and directive removal), the errors. Every quirk a port must reproduce, with a test input.
3. **DB**: each DB call in order, what it stores, the sanitising (which texts go through `sanitizeText`), the defaults, what `clear` resets and what it does not.
4. **Canonical model proposal**: the fields of the model the differential compares (camelCase, as the P1 models), what is left out and why.
5. **Specs**: the spec files with their test counts, and which test the parser, the DB, the layout data or the renderer.
6. **Renderer**: the pipeline from the DB to the SVG, element by element in DOM order (tags, classes, attributes), every measurement of text (`getBBox`, `calculateTextDimensions`, `getComputedTextLength`) and what it feeds, the layout (the unified renderer and its layout algorithm, d3 modules, or the type's own arithmetic), the sizing of the root `svg`, the style sheet (`styles.*`) and the theme variables it reads, the configuration section and its defaults, every use of randomness or the clock, every DOM dependency that a server renderer has to replace.
7. **Fixtures**: the count under `e2e/diagrams/<folder>/`, the features they exercise, what the generic geometry harness will have to exclude or handle (handDrawn, scripts the bundled faces do not cover, randomness).
8. **Risks, effort, open questions**: what is hard, what needs a decision from the coordinator, and a rough size.

## Sections of a layout engine's notes

0. **Summary**, 1. **Sources** (the library files with their line ranges and versions), 2. **Input**: what Mermaid builds and hands the engine (`rendering-util/layout-algorithms/<engine>/`), 3. **Algorithm**: the phases in order, with their constants and options, 4. **Randomness**: every draw, from which generator, in which order, and what state survives between layouts on one page, 5. **Output**: what Mermaid reads back and how it positions nodes and edges, 6. **Determinism and floating point**: what a port must do to give the same numbers (operation order, `Math` functions, iteration limits), 7. **How to test a port**: how to capture inputs and outputs from the reference for a differential, 8. **Risks, effort, open questions**.
