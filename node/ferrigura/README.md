# @ferrigura/core

[![Node.js 22 (22.13.0 or later), 24, and 26](https://img.shields.io/badge/Node.js-22%20%7C%2024%20%7C%2026-339933?logo=nodedotjs&logoColor=white)](https://github.com/sebastian-software/ferrigura/blob/main/node/ferrigura/README.md#platforms)

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

The default `compat: false` enables optimized flowchart routing and compact sequence participant boxes. Pass `{ compat: true }` as render options to preserve Mermaid-compatible geometry across diagram types. Font, fill and readability fixes apply in both modes. Pass `{ design: true }` to draw flowcharts, state, class and ER diagrams, mindmaps, and pie, XY, quadrant and gantt charts in the design mode: ferrigura's own look, with roomier nodes and spacing for flowcharts, state, class and ER diagrams. It cannot be combined with `compat`. Options belong to each render and cannot be set in Mermaid front matter or init directives.

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

## Resolve catalog fonts

The host can resolve a named catalog family before rendering. The default catalog is pinned to
an immutable manifest containing Lexica Ultralegible, Atkinson Hyperlegible Mono, Inter, Noto Sans
Math and Noto Sans Symbols 2. This helper fetches only when called explicitly.

```js
import { Renderer } from '@ferrigura/core';
import { resolveCatalogFonts } from '@ferrigura/core/font-catalog';

const fonts = await resolveCatalogFonts({
  families: ['Lexica Ultralegible'],
  cacheDir: './.cache/ferrigura-fonts',
});
const diagrams = new Renderer({}, fonts);
```

A selection can be `{ family: 'Catalog Name', alias: 'monospace' }` to register a local alias.
Additional selected families do not automatically become fallbacks: set `fallbacks` to an ordered
list of selected aliases when needed. Every face in each selected family is registered. Set
`offline: true` to require an already populated cache. Cached files are verified on every read;
missing or corrupt entries fail rather than silently using another face.

To use a different catalog, pass `catalog: { manifestUrl, manifestSha256, assetBase }` with a
reviewed SHA-256 of the raw manifest. Font paths resolve against `assetBase`, which must use the
same HTTPS origin as `manifestUrl`; credentials, query strings, fragments and redirects are rejected. The catalog and each font are bounded
and checked by SHA-256, with font length checked too. The cache stores both kinds of object as
`sha256/<digest>`. The renderer never fetches fonts or interprets URLs from diagram text.

For protected Ferrigura downloads, set `FERRIGURA_DOWNLOAD_TOKEN` in the host environment and pass
`protectedDownloads: true`. On a cache miss the helper requests a signed download from Ferrigura's
issuer and verifies its path, origin, length and SHA-256. It never sends the bearer token to the
asset server and does not fall back to the public catalog if authorization fails. Warm-cache and
`offline: true` resolution need no token. Protected downloads apply only to the built-in catalog;
custom catalogs never receive this token.

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
unchanged. Flowchart, Gantt, mind map, pie (including donut), and XY chart
support this option; other diagram types return an error. For charts,
`'flow'` starts independent elements together. This is a render option, separate
from Mermaid configuration. See [draw animation](../../docs/animation.md) for
the behavior of each type.

## Platforms

Supported platforms: macOS arm64; Linux x64/arm64 with glibc or musl; Windows
x64/arm64. Node.js 22.13.0 or newer is required. The main package contains no
native addon. See `NOTICE.md` for layout and bundled-font license texts.

The published addon draws all ten diagram types that ferrigura draws; a text of
any other Mermaid type throws an error that names it. To ship fewer, build the
[addon crate](https://github.com/sebastian-software/ferrigura/blob/main/crates/ferrigura-node/README.md)
with the features of the types you draw.

For reproducible Gantt output, pass `{ now: 1704067200000 }` as the render
options to `render` or `Renderer.render`. `now` is an integer number of UTC
milliseconds since the Unix epoch within ±8640000000000000. When omitted,
the system clock is used.
