# 0021 Optional draw animation belongs to the SVG output

**Scope:** optional, one-time draw animation for flowchart, Gantt, Git graph,
mind map, pie, and XY chart in the Rust library,
Node.js addon and CLI. It does not change Mermaid syntax, configuration, layout,
or the existing per-edge moving-dash animations.

## Decision

`RenderOptions::animation` enables draw animation. It is absent by default, and
an absent option preserves the static SVG byte for byte. Rust takes an
`Animation`, Node.js takes `animation: { speed?, order? }`, and the CLI takes
`--animate`, `--animation-speed` and `--animation-order`. Unsupported diagram types
return an option error when animation is requested.

The default draws one line at a time in drawing order. For graphs, `flow` order starts roots
and outgoing branches together; a node appears on its first incoming arrival.
Every edge draws once, including a return edge. A rootless component starts at
its first declared node after the preceding reachable component finishes.
Subgraph frames and diagram titles remain visible as context. Gantt task bars
and XY bars grow along their value axes; XY lines trace their points. Pie and
donut sectors reveal clockwise, timed by outer perimeter arc length. In these
charts, `flow` starts independent elements together. Pie and XY legend entries
appear with their corresponding sector or series, while keeping their final
layout space. Entries for omitted pie sectors appear after the visible sectors.
Axes, sections, and other context remain visible. [Draw animation](../animation.md) defines the
behavior of each supported type.

Speed is a positive, finite multiplier of average drawing velocity in SVG user units.
It is not a target duration for the whole diagram: adding nodes or edges does
not accelerate an existing path. Rust computes timing from the final routed
paths, including line hops and marker offsets, or from bar and arc lengths.
Node fades remain brief.

Every connection, bar, and sector independently eases in and out with
`cubic-bezier(0.4, 0, 0.2, 1)`. Length and speed determine its duration; easing
distributes movement within it. The curve settles without overshoot. A spring
that overshoots a reveal mask's dash offset could hide a completed part again.

The output carries its own CSS keyframes and masks and contains no script.
Motion applies only within `prefers-reduced-motion: no-preference`; the default
SVG state is the complete diagram. Each render owns its timing and identifiers.
No browser, JavaScript engine, or animation library is needed to produce it.

## Why

The effect is a short visual enhancement. Length-based timing gives long and
short diagrams the same pace along a line. A fixed total duration would make
large diagrams draw faster. Source order makes the default predictable;
parallel branches are an explicit choice.

CSS in the exported SVG keeps the effect independent of a website's JavaScript
runtime. Motion or a JavaScript playback helper would require a browser
integration and would not make the exported file self-contained. Native Web
Animations controls can still inspect or replay the CSS animations, as the
local preview does.

Reveal masks preserve existing dotted lines, neo arrowhead gaps, line hops and
colors. Animating the original dash array would replace those styles. Masks
and opacity effects release when finished so their compositing does not change
the final rasterization. Marker-only carriers retain the existing definitions
and edge styles.

## Consequences

- Animation is a ferrigura render option, separate from Mermaid configuration.
  It is an explicit addition to [0017](./0017-reproduce-the-reference.md).
- The renderer produces one complete layout and SVG synchronously. The viewer
  plays the effect. There is no intermediate rendering or state between calls.
- Existing corpus snapshots and the supported types' geometry runs check static
  output. Separate snapshots pin the animated output; native browser checks
  compare its final pixels with static output and run exports with JavaScript
  disabled, alongside reduced motion, per-section easing, and average-speed checks.
- A speed that overflows a diagram's timing is an option error. An empty
  Node.js animation object enables the defaults.

## Related

[0006](./0006-rust-only-no-browser.md), [0014](./0014-upstream-reference-at-a-pin.md),
[0018](./0018-deterministic-output.md), [0019](./0019-adaptive-flowchart-mode.md),
[draw animation](../animation.md),
[animation preview](../../tools/animation/README.md).
