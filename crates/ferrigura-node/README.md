# ferrigura-node

`ferrigura-node` is the Node.js addon of [ferrigura](https://github.com/sebastian-software/ferrigura). It is a native addon built with napi-rs, so it renders Mermaid diagram text to SVG in the Node.js process, without a browser. It is for Node.js only: there is no WebAssembly build.

Install the npm facade with `npm install @ferrigura/core`; npm selects the native
package for your platform. See [the npm package reference](../../node/ferrigura/README.md)
for supported platforms and the JavaScript API. The raw addon workflow below is
for binding development.

## Building and loading

From a checkout of the repository:

```sh
cargo build -p ferrigura-node
```

The build writes the library to `target/debug/`, as `libferrigura_node.dylib` on macOS, `libferrigura_node.so` on Linux and `ferrigura_node.dll` on Windows. Node loads a `.node` file with `process.dlopen`, which the addon's tests do too:

```js
const addon = { exports: {} };
process.dlopen(addon, 'target/debug/libferrigura_node.dylib');
const { render, Renderer } = addon.exports;
```

Run the tests from the repository root with `node --test crates/ferrigura-node/test/render.test.mjs`.

## Choosing the diagram types

The published addon draws every diagram type. The crate has one cargo feature per type, named as its constructor in `ferrigura::diagrams` (`flowchart`, `sequence`, `class`, `state`, `er`, `gantt`, …), and `all`, the default. A program that ships its own addon builds it with the types it draws; the parsers, renderers and layout engines of the other types are then not linked into it:

```sh
cargo build --profile release-node -p ferrigura-node --no-default-features --features flowchart,sequence
```

Node loads the library with `process.dlopen` as above. A text whose type the build leaves out throws an `Error` that names the type and says it is not registered. Under the release profile on macOS arm64, the addon with `sequence` alone is 6.0 MB (2.9 MB gzipped), with `flowchart`, `sequence`, `class`, `state` and `er` 8.1 MB (3.9 MB), with `all` 11.8 MB (5.7 MB).

## render(text, config?, options?)

Returns the SVG document of a diagram as a string.

- `text` is the diagram text.
- `config` is the configuration that `mermaid.initialize` takes, as an object. Without it, the defaults apply. Front matter and `%%{init}%%` directives in `text` apply on top of it, as in Mermaid.
- `options` is an object with these optional fields:
  - `id`: the id of the SVG element, which scopes its styles. It must start with an ASCII letter and contain only ASCII letters, digits, `-` and `_`. By default it is derived from the text and the configuration, so the same input gives the same id.
  - `compat`: `false` (default) uses optimized flowchart routing and compact sequence participant boxes. `true` preserves Mermaid-compatible geometry. Font, fill and readability fixes apply in both modes.
  - `colorScheme`: `'light'`, `'dark'` or `'auto'`. `'auto'` writes each color that differs as `light-dark(light, dark)`: inline in a page the diagram follows the page's `color-scheme`, and as a standalone file it follows the viewer's. By default the configured theme decides the colors.

```js
const svg = render(
  'pie\n  "Dogs" : 386\n  "Cats" : 85\n',
  { themeVariables: { pie1: '#00ff00' } },
  {
    id: 'pets',
    colorScheme: 'auto',
  }
);
```

`render` throws an `Error` with the message of the render error. It throws for text that is not a diagram or does not parse, for a configuration value of the wrong type, and for a color scheme or an id that is not valid.

## new Renderer(config?)

A `Renderer` resolves the configuration once, when it is made, for many renders:

```js
const renderer = new Renderer({ themeVariables: { pie1: '#00ff00' } });
const first = renderer.render('pie\n  "Dogs" : 386\n', { id: 'pets' });
const second = renderer.render('flowchart LR\n  A --> B\n');
```

`renderer.render(text, options?)` takes the same `text` and `options` as `render` and gives the same SVG that `render` gives with the same configuration, byte for byte. It throws in the same cases.

## What the addon does not take

The addon has no clock option, so dates that depend on the current time (the today marker of a gantt chart and the dates without a day) use the system clock.

For the rest of what ferrigura does with a diagram, including its differences from Mermaid and what it does not support yet, see the [ferrigura README](https://github.com/sebastian-software/ferrigura/blob/main/crates/ferrigura/README.md).
