# SVG animation preview

Build the checkout's native Node.js addon from `node/`:

```sh
pnpm install --frozen-lockfile --ignore-scripts
pnpm build
```

Run from the repository root, then open the printed URL:

```sh
node tools/animation/preview.mjs
```

Pass a different port as the first argument. The server binds to `127.0.0.1`.
It renders through the checkout's `@ferrigura/core` facade and the native Rust
`animation` render option. JavaScript controls playback; it does not prepare
paths, schedule the drawing, or add animation to the SVG.

Choose a diagram, drawing order and speed; play, pause, replay, or scrub the
result. Download exports the native SVG unchanged, with its CSS, masks and font
subsets. The file contains no script. Reduced motion shows the complete diagram.
The preview uses native Web Animations controls to inspect the CSS animations.

The default is one line at a time, averaging 700 SVG user units per second. Speed
multiplies that velocity rather than setting a duration for the whole diagram.
Each connection, bar, and sector independently eases in and out with
`cubic-bezier(0.4, 0, 0.2, 1)` within its length-based duration.
Nodes fade in over 80 ms. Flow order draws branches in parallel and reveals a
node on its first incoming arrival. Return edges draw once. A component without
a root starts at its first declared node after preceding reachable components.
Subgraph frames and diagram titles remain visible. For Gantt, pie, and XY,
flow order starts independent chart elements together. See
[draw animation](../../docs/animation.md) for the complete option contract.

The ten examples cover three flowcharts, a release Gantt, a Git graph with a
merge, a branching mind map, pie and donut charts, and XY charts with lines,
vertical bars, and horizontal bars. Legend entries appear with their sectors or
series. Axes and other context remain visible while the elements draw.

With the harness dependencies installed, run the browser checks from the
repository root:

```sh
node tools/animation/check.mjs
```

The checks start a local server and use the harness's Playwright installation.
They verify source and flow order, playback, scrubbing, markers, per-section
easing, average velocity, synchronized legends, stable pie separators, and the speed multiplier. Final geometry, styles and pixels must match
static native output. Exports run with JavaScript disabled; reduced motion and
four viewport widths are checked as well. Every example is tested in both
orders and as a standalone export without JavaScript. The flowcharts include
neo and classic looks, a return edge, a subgraph, bidirectional markers, custom
line styles, and a linked node.
