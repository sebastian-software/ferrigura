# @ferrigura/core

Render Mermaid diagram text to SVG in Node.js without a browser. The engine runs
in Rust. npm installs the matching optional platform package; keep optional
dependencies enabled. No browser or WASM runtime is required.

```sh
npm install @ferrigura/core
```

```js
import { Renderer } from '@ferrigura/core';
const diagrams = new Renderer();
const svg = diagrams.render('flowchart LR\nA[Hello] --> B[World]', {
  id: 'article-flow',
  colorScheme: 'auto',
});
console.log(svg);
```

`render(text, config?, options?, fonts?)` renders one diagram. `new Renderer(config?, fonts?)`
resolves configuration once; reuse `renderer.render(text, options?)` for many
blocks. Both return SVG synchronously and throw on engine errors. Give diagrams
unique IDs when combining them in one document. SVG is not automatically JSX.

The default `compat: false` enables optimized flowchart routing and compact sequence participant boxes. Pass `{ compat: true }` as render options to preserve Mermaid-compatible geometry across diagram types. Font, fill and readability fixes apply in both modes. Options belong to each render and cannot be set in Mermaid front matter or init directives.

## Custom fonts

Register a local file with the renderer, separate from Mermaid configuration:

```js
const diagrams = new Renderer(
  {},
  { fonts: [{ family: 'Project Sans', path: './fonts/regular.ttf' }] }
);
const svg = diagrams.render('flowchart LR\n A[Hello]');
```

Each face takes `family` and exactly one of `path` or `data` (`Uint8Array`,
including Node.js `Buffer`). The JavaScript facade reads paths synchronously,
relative to the process directory, and passes bytes to the native addon. The
renderer owns a copy, so later file or buffer changes do not affect it.
`weight` defaults to 400 and accepts integers from 1 through 1000; `style` is
`'normal'` (default) or `'italic'`.

The first registered family is the default unless configuration explicitly sets
`fontFamily`. Register additional faces and set `fallbacks: ['Symbols']` to name
an ordered fallback chain. The primary CSS family list must resolve to a
registered face. Unknown names in that list or fallback chain are skipped; they
never trigger system-font discovery or network loading.

The same font object is the fourth argument of `render(text, config, options,
fonts)`. Fonts are renderer initialization inputs; the existing third argument
and `renderer.render` take only per-render SVG options. Missing files, invalid
font bytes, unsupported metadata, and ambiguous sources throw before rendering.
Front matter and init directives can select registered names, but cannot supply
font files or URLs. TrueType faces embed subsets of used glyphs; CFF/CFF2
currently embed the complete face and can produce larger SVGs. Omitting font
options preserves the bundled defaults.

## Draw animation

```js
const svg = diagrams.render('flowchart LR\nA[Start] --> B[Done]', {
  animation: { speed: 1.5 },
});
```

`animation: {}` enables a one-time draw effect with speed `1` and order
`'source'`: one line at a time. Speed is a positive, finite multiplier of an average of 700 SVG
user units per second along the routed path. Larger diagrams take longer at the
same speed. Each connection, bar, or sector eases in and out independently.
Set `order: 'flow'` for parallel branches and first-arrival node
reveals. Nodes fade in over 80 ms; subgraph frames and the title stay visible.

The SVG embeds CSS and masks without JavaScript and shows the complete drawing
when the viewer requests reduced motion. Omit `animation` to keep static output
unchanged. Flowchart, Gantt, Git graph, mind map, pie (including donut), and XY
chart support this option; other diagram types return an error. For charts,
`'flow'` starts independent elements together. This is a render option, separate
from Mermaid configuration. See [draw animation](../../docs/animation.md) for
the behavior of each type.

## Platforms

Supported platforms: macOS arm64; Linux x64/arm64 with glibc or musl; Windows
x64/arm64. Node.js 22.13.0 or newer is required. The main package contains no
native addon. See `NOTICE.md` for layout and bundled-font license texts.

The published addon draws all 33 diagram types. To ship fewer, build the
[addon crate](https://github.com/sebastian-software/ferrigura/blob/main/crates/ferrigura-node/README.md)
with the features of the types you draw.
