# Getting started

Ferrigura renders Mermaid diagram text to standalone SVG in Rust. Start with the
command-line tool to render a file, or embed the library in your application.
The rendering process needs no browser.

## Render a file

Install the published command-line tool:

```sh
cargo install --locked ferrigura-cli
```

Save a diagram as `review.mmd`:

```text
flowchart TD
  draft[Draft] --> review{Review}
  review -->|Approved| publish[Publish]
  review -->|Changes| draft
```

Render it to SVG:

```sh
ferrigura --id review --scheme light -o review.svg review.mmd
```

The SVG contains its labels as text and embeds subsets of the fonts it uses.
Open the file in an SVG viewer, include it in a page, or keep it as a build
artifact. The [CLI reference](../../crates/ferrigura-cli/README.md) covers stdin,
batch output, configuration and exit codes.

## Use the Rust library

```sh
cargo add ferrigura
```

```rust
use ferrigura::Value;

let svg = ferrigura::render(
    "flowchart LR\n  A[Draft] --> B[Published]\n",
    &Value::empty_object(),
)?;
```

For many diagrams, reuse a `Renderer` with one resolved configuration. Give
inline diagrams unique IDs when combining them on one page. The
[Rust reference](../../crates/ferrigura/README.md) includes complete examples,
configuration, themes and render options.

## Node.js integration

Install the native Node.js package with Node.js 22.13.0 or newer:

```sh
npm install @ferrigura/core
```

```js
import { Renderer } from '@ferrigura/core';

const diagrams = new Renderer();
const svg = diagrams.render('flowchart LR\n  A[Draft] --> B[Published]\n', {
  id: 'review',
  colorScheme: 'light',
});
```

npm installs the native binary for your platform; keep optional dependencies
enabled. The [package reference](../../node/ferrigura/README.md) covers supported
platforms, configuration and render options. Use the
[addon instructions](../../crates/ferrigura-node/README.md) for binding development.

The optional Ferromark adapter renders Mermaid fences in custom builds. It is
not included in Ferromark’s standard npm addon. See the
[Ferromark integration guide](https://github.com/sebastian-software/ferromark/blob/6ee1647d8ffcb1b7469ae26120daa1d829af5675/docs/ferrigura.md)
for that build path.

## Know the supported scope

Ferrigura ports Mermaid 12.2.0. Its
[diagram reference](../../docs/diagrams/README.md) documents coverage and limits
for each type. SVG is the output format; browser interactions, math labels,
classic themes and some font coverage are outside the supported scope.

Read [how output differs from Mermaid](../../crates/ferrigura/README.md#how-the-output-differs-from-mermaid)
before replacing an existing renderer. For reproducible gantt output, supply
`RenderOptions::now` when the diagram depends on the current date.
