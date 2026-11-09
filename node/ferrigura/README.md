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

`render(text, config?, options?)` renders one diagram. `new Renderer(config?)`
resolves configuration once; reuse `renderer.render(text, options?)` for many
blocks. Both return SVG synchronously and throw on engine errors. Give diagrams
unique IDs when combining them in one document. SVG is not automatically JSX.

Flowcharts center edge attachment points with label-aware spacing and draw edge-label backgrounds at full opacity by default.
For Mermaid-compatible routing, pass `{ flowchartMode: 'mermaid' }` as the
render options. Use `'adaptive'` to select the default explicitly. This option
affects only flowcharts and is separate from Mermaid configuration and
`elk.straightenEdges`.

Supported platforms: macOS arm64; Linux x64/arm64 with glibc or musl; Windows
x64/arm64. Node.js 22.13.0 or newer is required. The main package contains no
native addon. See `NOTICE.md` for layout and bundled-font license texts.

The published addon draws all 33 diagram types. To ship fewer, build the
[addon crate](https://github.com/sebastian-software/ferrigura/blob/main/crates/ferrigura-node/README.md)
with the features of the types you draw.
