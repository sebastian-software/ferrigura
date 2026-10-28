# Mermaid

[Mermaid](https://mermaid.js.org/) is the original: the TypeScript library that defines the syntax and draws it in a browser, MIT-licensed, by the mermaid-js project (npm package `mermaid`, 12.2.0 as of 2026-10-24, 23 direct dependencies). ferrigura is ported from Mermaid 12.2.0 and checked against upstream Mermaid at a pinned commit, with a few patches for Mermaid's bugs ([0014](../decisions/0014-upstream-reference-at-a-pin.md)).

## What ferrigura keeps

ferrigura aims at Mermaid's output, not a look of its own:

- **The syntax and the models.** Each type's parser and DB are ported with their quirks, and `pnpm -C tools/port port:diff <type>` compares the model with the reference's on every collected input; every ported type has no difference.
- **The geometry.** `pnpm -C tools/port port:geometry <type>` renders each fixture with both and pairs every element: boxes, origins, path samples and text agree within 0.05 px on every comparable fixture. The comparison forces SVG labels, the bundled font and the strict security level on both sides, and a theme ferrigura ships.
- **The layouts.** Mermaid's engines are ported rather than replaced: ELK Layered (the default layout since Mermaid 12.0) in `ferrigura-elk`, cose-bilkent and fcose in `ferrigura-cose`, the tidy tree, and each type's own renderer with its d3 pieces. They match the JavaScript engines exactly on captured inputs.
- **The browser's arithmetic where it shows.** Chromium's text rounding, its `Math` functions (`ferrigura-jsmath`) and its path bounds (`ferrigura-path`) are reproduced, because a force layout turns a one-unit difference into tens of pixels.
- **Even the bugs the output depends on**, such as an attribute the reference writes as `undefined;;;undefined`. Where the reference crashes, ferrigura names an exception and draws (a mindmap with `layout: elk`).

## Where ferrigura differs, by decision

| Topic         | Mermaid                                                                         | ferrigura                                                                                                               | Decision                                                |
| ------------- | ------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------- |
| Runtime       | A browser with a DOM; on a server through a headless browser (mermaid-cli)      | Rust: a library, a CLI and a native Node addon; no browser, no WebAssembly                                              | [0006](../decisions/0006-rust-only-no-browser.md)       |
| Parsers       | Jison and Langium grammars                                                      | Hand-written parsers, compared with the reference's on every input                                                      | [0007](../decisions/0007-hand-written-parsers.md)       |
| Labels        | HTML in `foreignObject` by default                                              | SVG text, Markdown as styled runs; the HTML line breaking emulated where the reference measures it                      | [0009](../decisions/0009-svg-text-labels.md)            |
| Fonts         | The page's fonts (Trebuchet MS, Verdana, Arial by default), measured by the DOM | Lexica Ultralegible and Atkinson Hyperlegible Mono, embedded as a subset of the glyphs used                             | [0010](../decisions/0010-text-measurement-and-fonts.md) |
| Themes        | About a dozen themes and `themeVariables`, computed per render                  | `redux-color` and `redux-dark-color` as CSS custom properties with attribute fallbacks; `colorScheme: auto` writes both | [0011](../decisions/0011-themes-as-css-variables.md)    |
| Interactivity | Click callbacks, tooltips, the `loose` and `sandbox` security levels            | Links stay `<a>`, tooltips become `<title>`; no scripts                                                                 | [0006](../decisions/0006-rust-only-no-browser.md)       |
| Ids           | The caller's id, and random ids inside some diagrams (mindmap, gitGraph)        | Derived from the input, so the same text gives the same SVG                                                             |                                                         |

## Speed and size

Measured here (the [overview](./README.md) has the method, in the run of 2026-10-24): Mermaid takes 7.75 ms per diagram in the median over 215 fixtures and 33.3 ms at the 90th percentile, against 0.70 ms and 2.01 ms for ferrigura's Node addon. `mermaid.min.js` is 5.5 MB, 1.57 MB gzipped.

Chromium's profiler puts 57 percent of Mermaid's render time into `assignWithDepth`: `getConfig()` returns a deep copy of the whole configuration, theme variables included, and the renderers, shapes and labels call it again and again. The layouts take 17 percent, DOMPurify 10, and the DOM measurement with the layout it forces only 4. A change to Mermaid that hands out the configuration once per render, or as a frozen object, would be worth more than measuring text another way. ferrigura had the same weakness in a smaller form: on 2026-10-17, 84 percent of a small render went into resolving its configuration and theme. Since decision 0008, step 2, the configuration no longer shows in the profile of a render without a directive (the overview has the profile).

## What is not there yet

- The `handDrawn` look (rough.js sketching), math in labels (KaTeX), images and icon packs in flowcharts, and fallback fonts for scripts the bundled faces do not cover (CJK, kana and Arabic; the emoji and the symbols of U+2600-27BF are bundled since 2026-10-20).
- The known gaps of each diagram type are listed on its page, and the coverage of all of them in the [index of the diagram types](../diagrams/README.md).
