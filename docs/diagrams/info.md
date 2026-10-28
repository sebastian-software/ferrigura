# Info diagram

An info diagram draws the Mermaid version, as `v12.2.0`. It starts with the keyword `info`, optionally followed by `showInfo`, and accepts the header lines `title`, `accTitle` and `accDescr`. ferrigura parses it with a hand-written parser on the shared lexer (`crates/ferrigura-syntax/src/info.rs`) that follows the Langium grammar `info.langium`, and draws it with a renderer of its own (`crates/ferrigura-render/src/info/mod.rs`). There is no layout engine.

## Model

`InfoDiagram` (`crates/ferrigura-model/src/info.rs`), version 1, with `title`, `accTitle` and `accDescr`, each `""` when absent. The texts are what the grammar gives, as in the pie ([canonical-model.md](../canonical-model.md#info)).

The model leaves out two things:

- **The version.** The reference's info DB holds only the Mermaid version, which is a constant of the build and not part of the input. In the model it would make the differential run depend on the build.
- **`showInfo`.** The keyword is not in the grammar's syntax tree, so no model can hold it.

The info DB has no title setter, so the front matter title does not reach the model: the title is the grammar's alone.

## Parsing

The parser skips blank lines around the keyword, so `showInfo` may follow `info` on the next line. After it, only header lines are allowed; anything else is an error. The header lines are read by the shared rules of `common.rs` ([0007](../decisions/0007-hand-written-parsers.md)).

## Layout and rendering

The renderer ports `infoRenderer.ts`. It draws the text `v<version>` at (100, 40), anchored in the middle, at 32 px, in a root that is sized by `configure_svg_size` to 100 by 400 with `useMaxWidth`: `width="100%"`, a `max-width` of 400 px and no viewBox. The text is SVG text in the bundled face, with the glyphs embedded as a subset, and the style sheet is the common base sheet scoped to the diagram's id ([0009](../decisions/0009-svg-text-labels.md), [0011](../decisions/0011-themes-as-css-variables.md)). An empty `<g>` precedes the drawing, as the reference writes one before every diagram.

The version is the constant `MERMAID_VERSION` in `info/mod.rs`, the version of the reference's `packages/mermaid/package.json` at the pin. It is the same text for every diagram. When the pin moves to a newer release, the constant moves with it ([0014](../decisions/0014-upstream-reference-at-a-pin.md)).

The diagram has no configuration section, so its renderer takes the unit type as its section. The title and the accessibility texts are not drawn: the root has `role="graphics-document document"` and `aria-roledescription="info"`, but no `<title>`, `<desc>` or `aria-labelledby`.

## Differences from Mermaid

- The text is SVG text in the bundled face, not the reference's font.
- The style sheet is scoped to the diagram's id and uses custom properties for its colors.

## Checking it

- `pnpm -C tools/port port:diff info` compares the canonical models on every input. The exporter reads the header from the reference's parser after `preprocessDiagram` and takes no title from the front matter.
- `pnpm -C tools/port port:geometry info` compares the drawing in Chromium; it fails when `MERMAID_VERSION` differs from the version the reference draws.
- The Rust specs are `crates/ferrigura-syntax/tests/info_spec.rs` (`info.spec.ts`) and `info_grammar.rs` (`packages/parser/tests/info.test.ts`). `crates/ferrigura/tests/info.rs` checks the SVG.
- The info diagram has no configuration section, so `pnpm -C tools/port port:diff config` leaves it out.

## Learnings

- **The reference's SVG always starts with an empty group.** `mermaidAPI` appends it before any renderer draws (`appendDivSvgG`), so every renderer writes `<g></g>` first, and the geometry key counts it.
- **The root's attributes come in the order the reference sets them:** `id`, `width`, `xmlns`, then the size from `setupGraphViewbox`, then the roles. The info diagram has nothing else, which makes it the plainest example of that order.
