# Choose a layout engine

A layout engine places a flowchart’s nodes and routes its connections. Ferrigura’s standard builds use ELK Layered for flowcharts, class diagrams, state diagrams and entity relationship diagrams. The other types keep their own layouts.

Start with ELK for directed workflows, graphs outside the experimental subset, and diagrams that need Mermaid’s geometry in `compat` mode. The [ELK and TALA comparison](https://ferrigura.dev/comparisons/elk-and-tala/) shows the same text with both geometries. In the grouped reference case TALA takes fewer turns and uses less width, while ELK makes a shallower drawing. In the directional case ELK uses shorter routes.

## Experimental TALA

A custom build with the `experimental-tala` Cargo feature can draw up to twelve disconnected rectangular nodes, one rectangular node with a single labeled self-loop, or one rectangular node with up to thirty-two unlabeled self-loops. The standard npm distribution and default source builds omit that feature. The larger diagrams on the comparison page are labeled as Go-reference evidence where the Rust path does not draw them.

Engine selection belongs to the render call and takes precedence over the Mermaid layout setting in the document. Ferrigura’s default look is available with TALA. Mermaid compatibility mode and unsupported graph shapes or connections return an error when TALA is selected. Ferrigura leaves the engine choice to the caller when a render fails.

## Rust

Use a checkout that contains the experimental crate and enable the Cargo feature:

```toml
[dependencies]
ferrigura = { path = "../ferrigura", features = ["experimental-tala"] }
```

```rust
use ferrigura::{LayoutEngine, RenderOptions, Renderer, Value};

let renderer = Renderer::new(&Value::empty_object());
let svg = renderer.render_with_engine(
    "flowchart TB\n  Queue[Retry queue] -->|retry job| Queue",
    &RenderOptions::default(),
    LayoutEngine::Tala,
)?;
# Ok::<(), ferrigura::Error>(())
```

## Command line

From the repository root, build the feature and select the engine when rendering:

```sh
printf 'flowchart TB\n  Queue[Retry queue] -->|retry job| Queue\n' |
  cargo run -q -p ferrigura-cli --features experimental-tala -- --layout-engine tala
```

## Node.js

Build a custom [Node.js addon](../crates/ferrigura-node/README.md) with `cargo build -p ferrigura-node --features experimental-tala`. The example loads the macOS artifact directly. Use the `.so` on Linux or the `.dll` on Windows.

```js
const addon = { exports: {} };
process.dlopen(addon, 'target/debug/libferrigura_node.dylib');
const svg = addon.exports.render(
  'flowchart TB\n  Queue[Retry queue] -->|retry job| Queue',
  undefined,
  { layoutEngine: 'tala' }
);
console.log(svg);
```

## Licenses

Ferrigura’s core is MIT-licensed. Its ELK Layered port uses Eclipse Public License 2.0. Its experimental Rust port of D2’s TALA from Terrastruct uses Mozilla Public License 2.0. Distributions that include the TALA port need to identify the covered Rust source at the corresponding revision and include its licensing notice. [ELK source notice](../crates/ferrigura-elk/NOTICE.md) · [TALA source notice and upstream pin](../crates/ferrigura-tala/NOTICE.md)
