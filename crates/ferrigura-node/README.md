# ferrigura-node

`ferrigura-node` is the Node.js addon of [ferrigura](https://github.com/sebastian-software/ferrigura). It is a native addon built with napi-rs, so it renders Mermaid diagram text to SVG in the Node.js process, without a browser. It is for Node.js only: there is no WebAssembly build.

The addon is not published yet. There is no npm package: its name, its entry file and its prebuilt binaries per platform are still to be made. This README describes the addon as the crate builds it.

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

## render(text, config?, options?)

Returns the SVG document of a diagram as a string.

- `text` is the diagram text.
- `config` is the configuration that `mermaid.initialize` takes, as an object. Without it, the defaults apply. Front matter and `%%{init}%%` directives in `text` apply on top of it, as in Mermaid.
- `options` is an object with two optional fields:
  - `id`: the id of the SVG element, which scopes its styles. It must start with an ASCII letter and contain only ASCII letters, digits, `-` and `_`. By default it is derived from the text and the configuration, so the same input gives the same id.
  - `colorScheme`: `'light'`, `'dark'` or `'auto'`. `'auto'` writes both colour sets, and the dark one applies under `prefers-color-scheme: dark`. By default the configured theme decides the colours.

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

`render` throws an `Error` with the message of the render error. It throws for text that is not a diagram or does not parse, for a configuration value of the wrong type, and for a colour scheme or an id that is not valid.

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
