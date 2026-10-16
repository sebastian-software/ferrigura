# M6 renderers: how a renderer task works

This is the working guide for the renderer tasks of M6 (stages 2 and 3 of [M6-PLAN.md](./M6-PLAN.md)). Each task ports one diagram type's renderer in its own worktree and branch, cut from `sebastian/port-m6-p1-diagrams`. The coordinator reviews the report and merges the branch.

## What is already there

- **The facade** (`crates/ferrigura/src/lib.rs`) parses every P1 type and calls `ferrigura_render::<type>::render(&<Type>Input { diagram, config, themes, id })`. The configuration is the effective one in the type's scope (`DiagramType::config_key`), typed as `MermaidConfig`; the section of the type is `config.<type>` (for example `config.sequence`, `config.git_graph`). Gantt also gets `now`, the clock in milliseconds since the epoch. The facade does not need to change for a renderer task.
- **The stub** `crates/ferrigura-render/src/<type>/mod.rs`: the input struct and a `render` that returns `RenderError::Unsupported`. The task replaces the body and adds the modules it needs under that folder.
- **Themes** (`ferrigura_render::theme::Themes`): one set of theme variables or a light and a dark set; `themes.primary()` is the set that gives sizes and presentation attributes.
- **The unified renderer** (`crates/ferrigura-render/src/unified/`), used by flowchart, class, state and ER: `pipeline::measure` (edge render data, the measure step), `pipeline::lay_out` (elkjs graph, ELK Layered, post-processing), `pipeline::paint` (clusters, nodes, edges, labels in the order of `paintLayoutData`). A diagram builds its `LayoutData` (its `getData`), runs the three steps and writes the root `<svg>` with its own markers, style sheet and title. `crates/ferrigura-render/src/flowchart/mod.rs` is the worked example.
- **Text**: `ferrigura-text` measures as Chromium does (`measure`, `measure_line`, `text_box`, `font_metrics`, the wrapping of `wrap.rs`). Labels with `htmlLabels: false` are `unified::label` (`createText`). The custom renderers' shared helpers (`calculateTextDimensions`, `wrapLabel`, `svgDrawCommon.ts`, `setupGraphViewbox.js`) go into `crates/ferrigura-render/src/common/`; until that task lands, a custom renderer does not depend on them.
- **SVG helpers**: `svg::escape`, `svg::number` (how numbers are written), `svg::font_faces`, and `a11y.rs` (`accTitle`, `accDescr`, `aria-roledescription`).
- **Styles**: every colour is a CSS custom property declared on the diagram's own id, and every coloured element also carries its concrete colour as a presentation attribute; every selector starts with `#id` (decision 0007). `flowchart/style.rs` and `pie.rs` show how a reference `styles.js` becomes a style sheet. Only `redux-color` and `redux-dark-color` ship (M6-PLAN, themes).

## The reference and the research

- The renderer's sources are under `packages/mermaid/src/diagrams/<type>/` and `packages/mermaid/src/rendering-util/`. The research, with file and line for every rule, is in `docs-port/<type>/NOTES.md` (the renderer section) and the parser decisions in `docs-port/<type>/PLAN.md`.
- The decisions of [M6-PLAN.md](./M6-PLAN.md) apply: quirks are reproduced and listed; text is measured where and how the reference measures it, with Chromium's rules (run advances ceiled to 1/64 px, ascent and descent rounded with a tie going down after flooring the size to 1/100 px, float32 `getBBox` values); links stay as `<a>`, callbacks and click handlers are dropped.

## How it is checked

- `pnpm port:geometry <type>` renders every fixture of the type on both sides (the reference in Chromium, ferrigura through the `render` example), measures both SVGs in one page at scale 1, pairs the elements by structural key (`tag.class[n]` from the root; ids are recorded but not part of the key) and compares boxes, origins, path samples and text content within 0.05 px. Classes: within, outside, not rendered (ferrigura returned an error), excluded (handDrawn, a non-layered layout, CJK), reference failed. `--filter 'folder/*name*'` narrows the run; `--capture` writes the reference's SVG and geometry to `target/geometry/generic/<type>/<slug>/`; `DUMP=<slug>` writes both measurements. Build the example first: `cargo build -p ferrigura --example render`.
- The structure must match the reference: the same elements, in the same order, with the same classes. Write the elements the reference writes, even empty groups.
- For a type on the unified renderer, compare the layout data too: a `pnpm port:diff <type>-data` run like `flowchart-data` (`tools/port/diff-flowchart-data.ts`, `export-flowchart-data.ts`, the `flowchart_data` example of `ferrigura-render`) finds a `getData` difference before the geometry does.
- Specs: the reference's render specs of the type (the ignored ones in `crates/ferrigura-syntax/tests/<type>_*` that name rendering or `getData`) are ported where they test what the task builds, as tests in `crates/ferrigura-render/tests/<type>_*.rs`, and their `#[ignore]` goes.

## Gates before the report

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --no-fail-fast` (count with `--no-fail-fast`; without it a failure can hide the rest).
- For changes in `tools/port/`: `pnpm exec prettier --check tools/port` and `pnpm exec eslint tools/port`.
- `pnpm port:geometry <type>`: the counts per class and the problems that remain, grouped by cause.
- Flowchart must stay where it is: if the task touched `unified/`, `pnpm port:geometry-flowchart` still gives 298 of 298 within tolerance.

## Rules of the branch

- Work only in the worktree you were given. Commit there, on its branch, with the trailer `Co-Authored-By: Claude Haiku 5.5 <noreply@anthropic.com>`. Do not merge, rebase, push, or use `git stash`; the coordinator merges.
- Own files: your type's folder in `crates/ferrigura-render/src/`, its tests and examples, its harness files in `tools/port/`, and its docs under `docs-port/<type>/`. A change to a shared file (`unified/`, `tools/port/geometry/common.ts`, `tools/port/diff.ts`, the facade) is allowed when the task needs it; keep it small, add rather than rewrite, and name it in the report.
- Repository language is English. Comments say why, with the reference's file and line.
- If something blocks you (a permission, a missing tool), stop and say so in the report; do not work around it.

## The report

The counts of `pnpm port:geometry <type>` (within, outside, not rendered, excluded) and of the specs; the differentials you ran; the shared files you changed and why; the quirks you reproduced; what is left, with the cause of each remaining problem group.
