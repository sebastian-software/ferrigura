# 0020 Diagram types are plugins of the renderer

**Scope:** how the facade `ferrigura` reaches the parser and the renderer of a diagram type, and what a program that embeds the crate links. It does not change the crates, the parsers, the renderers or the layout engines themselves, nor the command-line tool and the npm packages, which draw every type.

## Decision

Each diagram type is a `diagrams::Plugin`: the kind its detector reports, with the parse-and-render step of that kind. A `Renderer` holds the plugins it draws with, and a text whose detected type has no plugin is refused with `Error::Unsupported`.

- `Renderer::new(config)` registers `diagrams::all()`, so it draws every type, as the shorthand `render` does.
- `Renderer::builder(config).with(plugin)…build()` registers only the plugins it is given. The plugins are made by the functions of `ferrigura::diagrams` (`flowchart()`, `sequence()`, …), one per type, and a kind registered twice counts once.
- Detection stays in `ferrigura-syntax` and runs for every renderer: it is string matching on the header and references no parser.
- The parse-and-render functions live in `crates/ferrigura/src/diagrams.rs`, one per type, and nothing in the facade refers to a type's parser or renderer except its own function. So a program that names only the types it draws links only those, with the layout engines they reach; the linker drops the rest.

## Why

The crates are one workspace and every type is a module in them (ARCHITECTURE), which keeps one build, one test suite and one release. The cost was that every embedder linked all 33 types: a `match` over the detected type in the facade referenced every parser and renderer, so a program that draws sequence diagrams carried ELK, the swimlane engine and the thirty-two other renderers.

Registration moves that choice to the embedder without cargo features and without crates per type. Cargo features would multiply the build matrix and the feature combinations to test; crates per type would multiply the releases ([0004](./0004-one-version-one-release.md)). A plugin is a function pointer behind a constructor, and the linker does the rest: a release build that registers only `sequence()` is about half the size of one that registers all (7.96 MB against 15.37 MB, stripped, macOS arm64, with the bundled fonts in both); one with `flowchart()` alone is 9.76 MB, one with flowchart, sequence, class, state and ER is 10.85 MB.

Weighed and not chosen: a trait object per type (the same effect, but a larger public surface than a constructor and an opaque struct); registering by `DiagramType` value (the enum match would reference every type again, so nothing would be dropped); and crates per type, which stay possible later on top of this model, since a plugin is already the unit a crate would export.

## Consequences

- The public API of the facade grows by `Renderer::builder`, `RendererBuilder`, `diagrams::Plugin`, `diagrams::all` and one constructor per type, and re-exports `DiagramType` for `Plugin::kind`. New types add a constructor and an entry in `all`.
- The typed configuration (`MermaidConfig`, [0008](./0008-canonical-model-and-typed-views.md)) is still read as one struct, so the generated configuration code of every type stays linked; reading it section by section is the next step for smaller builds.
- The command-line tool and the Node addon keep `Renderer::new`, so their binaries and their behavior do not change.
- The snapshots and the geometry runs are unchanged: the plugins call the same functions the `match` called, in the same order, and the output is byte for byte the same.

## Related

[0004](./0004-one-version-one-release.md), [0008](./0008-canonical-model-and-typed-views.md), [0012](./0012-layout-engines-ported-exactly.md), [ARCHITECTURE](../ARCHITECTURE.md).
