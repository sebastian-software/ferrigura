# Draw animation

Draw animation is an optional, one-time reveal in the SVG itself. It is available
in Rust, Node.js, and the CLI, with no browser or animation library needed to
render the file. Omit the option to preserve the static SVG byte for byte.

## Supported diagram types

| Diagram type         | Drawing behavior                                                                                                                                             | Context that stays visible                             |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------ |
| Flowchart            | Connections follow their final routed paths; nodes and arrowheads appear on arrival.                                                                         | Subgraph frames and title                              |
| Gantt                | Task bars grow along the timeline; milestones and vertical task markers fade in.                                                                             | Axes, sections, excluded days, today marker, and title |
| Git graph            | Connections trace commit history; commit markers, labels, and tags appear on arrival.                                                                        | Branch lines, branch labels, and title                 |
| Mind map             | Connections draw between nodes; nodes appear on arrival.                                                                                                     | Title                                                  |
| Pie, including donut | Each sector reveals clockwise with its own easing, paced by its perimeter arc length. Its legend entry appears with it.                                      | Outer circle and title                                 |
| XY chart             | Bars grow along the value axis; lines trace their points. Each series brings in its legend entry when it starts. Horizontal charts follow their orientation. | Axes, background, and title                            |

Other diagram types return an option error when draw animation is requested.
Mermaid syntax and configuration do not change. Existing per-edge moving-dash
animations are separate from this effect.

## Options

- `speed` defaults to `1`, an average drawing velocity of 700 SVG user units per second.
  A value of `2` doubles that velocity. Longer lines take proportionally longer;
  adding elements does not accelerate an existing line. Pie timing uses arc
  length at the outer perimeter. Speed must be finite and greater than zero;
  a value that overflows a diagram's timing returns an option error.
- `order` defaults to `source`: **One line at a time**. The renderer draws
  connections, bars, or sectors sequentially in its drawing order. Graph nodes
  appear when first reached. Gantt uses its task drawing order, sorted by start.
- `flow` order follows graph connectivity: roots and outgoing branches draw in
  parallel, and first arrival reveals a node. Every connection draws once,
  including return connections. Rootless and disconnected components start
  deterministically. For Gantt, pie, and XY charts, independent elements start
  together at the selected speed.

Graph nodes fade in over 80 ms; sequential elements have a 15 ms gap. These brief
transitions do not scale with diagram size. The effect runs once and leaves the
complete diagram visible. It does not delay rendering or interaction.

Each connection, bar, and sector has its own soft acceleration and deceleration
(`cubic-bezier(0.4, 0, 0.2, 1)`). Speed determines the section's duration from its
length; easing distributes movement within that duration. It restarts for each
section, so successive pie sectors have distinct starts and finishes. The curve
settles without overshooting or briefly hiding completed parts of a reveal mask.

Pie and XY legend markers and labels fade in over 80 ms when their sector or
series starts drawing. The legend keeps its final position and space throughout.
A pie entry whose sector is omitted because it is below one percent appears
after the visible sectors, so the complete data remains available.

## Usage

Rust:

```rust
use ferrigura::{Animation, RenderOptions, Renderer, Value};

let renderer = Renderer::new(&Value::empty_object());
let svg = renderer.render_with(
    "flowchart LR\n A[Start] --> B[Done]\n",
    &RenderOptions {
        animation: Some(Animation::new(1.5)?),
        ..RenderOptions::default()
    },
)?;
```

`Animation::default()` selects speed `1` and `AnimationOrder::Source`. Set its
public `order` field to `AnimationOrder::Flow` for parallel drawing.

Node.js:

```js
import { Renderer } from '@ferrigura/core';

const renderer = new Renderer();
const svg = renderer.render('pie\n "Build" : 60\n "Test" : 40\n', {
  animation: { speed: 1.5 },
});
```

An empty `animation: {}` enables the defaults. Use `order: 'flow'` for parallel
drawing. The shorthand accepts render options as its third argument:
`render(text, config, { animation: {} })`.

CLI:

```sh
ferrigura --animate diagram.mmd > diagram.svg
ferrigura --animation-speed 1.5 --animation-order flow diagram.mmd > diagram.svg
```

A speed or order flag enables animation without a separate `--animate` flag.

## Playback and accessibility

The exported SVG contains CSS keyframes and reveal masks, with no script. It
plays in a viewer that supports SVG CSS animations. The default SVG state is the
complete diagram; motion applies only when the viewer reports
`prefers-reduced-motion: no-preference`. Reduced motion shows the full drawing
immediately. Completed output retains the static geometry, styles, fonts,
links, and viewBox. Masks preserve existing dash patterns and colors, and
release when finished.

The [local preview](../tools/animation/README.md) includes ten examples across
the six supported types, with speed, order, pause, replay, scrubbing, and SVG
download. Playback controls use native Web Animations APIs; all drawing timing
and SVG animation come from Rust.

The contract is recorded in [decision 0021](./decisions/0021-optional-svg-draw-animation.md).
