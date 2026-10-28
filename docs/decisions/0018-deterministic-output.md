# 0018 The same input gives the same output

**Scope:** what a render may depend on besides the text, the configuration and the render options. It does not cover the arithmetic that makes the numbers equal to the reference's ([0012](./0012-layout-engines-ported-exactly.md)), or the fonts ([0010](./0010-text-measurement-and-fonts.md)).

## Decision

The output of a render is a function of the text, the configuration and the render options, with one exception: the clock, where a gantt chart reads it and the caller does not set it. Where the reference reads the page it runs in, ferrigura takes a fixed value or an option:

- **Random numbers** come from seeded generators or fixed values: the generator and the seed the reference uses where it has one (fcose's mulberry32 from the configured seed), a fixed seed for the venn layout's draws, ids derived from the sequence number where gitGraph draws random ids, and a fixed fraction for the divergence of rough.js double strokes. A configuration that asks for the page's randomness, such as an architecture seed of 0, is an error.
- **The clock** is read only for the dates that depend on it (the gantt's today marker and dates without a day): from `RenderOptions::now` when it is set, from the system clock otherwise. Dates are in UTC.
- **The screen** is the harness page's: a renderer that reads the screen or its container (C4's `screen.availWidth`, the gantt's `offsetWidth`) gets 1200 px, unless the configuration sets a width.
- **No state is kept between renders.** Each render starts from the resolved configuration; counters, memos and ids live for one render.
- **The SVG id** is derived from the text and the configuration unless the caller sets one.

## Why

A renderer that runs on servers and in builds is useful only if it is reproducible: the same diagram gives the same bytes, so outputs can be cached, diffed and pinned by snapshots ([0005](./0005-quality-gates.md)). The reference is not: two renders of one text on one page can differ, and its module-level state leaks from one render into the next. The fixed values are the harness page's own, so the comparison with the reference stays exact ([0015](./0015-differential-testing.md)).

## Consequences

- A gantt chart that depends on today is reproducible only with `now` set.
- The harness page fixes the reference's clock, random numbers and screen to the same values, and `port:geometry <type> --self` renders the reference twice to find what it does not yet fix.
- Any change to a text, even a `%%` comment, changes its default id and so its snapshot.
- Hidden options pass the reference's seeds to ferrigura for the comparison; they are not part of the interface.

## Related

[0005](./0005-quality-gates.md), [0015](./0015-differential-testing.md), [0017](./0017-reproduce-the-reference.md); [LEARNINGS.md](../LEARNINGS.md#the-page-fixes-the-clock-the-random-numbers-and-the-environment), [LEARNINGS.md](../LEARNINGS.md#the-reference-keeps-state-between-renders-ferrigura-does-not).
