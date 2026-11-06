# 0020 Diagram types are plugins of the renderer

**Scope:** how the facade `ferrigura` reaches the parser and the renderer of a diagram type, and what a program that embeds the crate links. It does not change the crates, the parsers, the renderers or the layout engines themselves, nor the command-line tool and the published npm packages, which draw every type.

## Decision

Each diagram type is a `diagrams::Plugin`: the kind its detector reports, with the reader of the configuration sections its renderer takes and the parse-and-render step of that kind. A `Renderer` holds the plugins it draws with, and a text whose detected type has no plugin is refused with `Error::Unsupported`.

- `Renderer::new(config)` registers `diagrams::all()`, so it draws every type, as the shorthand `render` does.
- `Renderer::builder(config).with(plugin)…build()` registers only the plugins it is given. The plugins are made by the functions of `ferrigura::diagrams` (`flowchart()`, `sequence()`, …), one per type, and a kind registered twice counts once.
- Detection stays in `ferrigura-syntax` and runs for every renderer: it is string matching on the header and references no parser.
- The parse-and-render functions live in `crates/ferrigura/src/diagrams.rs`, one per type, and nothing in the facade refers to a type's parser or renderer except its own function. So a program that names only the types it draws links only those, with the layout engines they reach; the linker drops the rest.
- `RendererBuilder::shared` keeps a renderer per configuration and plugin set, as the shorthand `render` does (it uses it with every plugin), so a program that renders one diagram at a time does not resolve its configuration again.
- The addon crate `ferrigura-node` has one cargo feature per type, named as the constructor, and `all` as its default. The published addon is built with `all`; a program that builds its own addon enables the types it draws. A cdylib has no caller that could call the builder, so for it the features are the registration.

## Why

The crates are one workspace and every type is a module in them (ARCHITECTURE), which keeps one build, one test suite and one release. The cost was that every embedder linked all 33 types: a `match` over the detected type in the facade referenced every parser and renderer, so a program that draws sequence diagrams carried ELK, the swimlane engine and the thirty-two other renderers.

Registration moves that choice to the embedder without cargo features on the library and without crates per type. Cargo features on the library would multiply the build matrix and the feature combinations to test, where a program can call the builder instead; crates per type would multiply the releases ([0004](./0004-one-version-one-release.md)). The addon crate is the exception: nothing calls its builder but itself, so its features are the only way to pick, and CI builds it with one type so that path keeps compiling. A plugin is a function pointer behind a constructor, and the linker does the rest: a release build that registers only `sequence()` is about half the size of one that registers all (7.96 MB against 15.37 MB, stripped, macOS arm64, with the bundled fonts in both); one with `flowchart()` alone is 9.76 MB, one with flowchart, sequence, class, state and ER is 10.85 MB.

Weighed and not chosen: a trait object per type (the same effect, but a larger public surface than a constructor and an opaque struct); registering by `DiagramType` value (the enum match would reference every type again, so nothing would be dropped); and crates per type, which stay possible later on top of this model, since a plugin is already the unit a crate would export.

## Consequences

- The public API of the facade grows by `Renderer::builder`, `RendererBuilder` with `with`, `with_all`, `build` and `shared`, `diagrams::Plugin`, `diagrams::all` and one constructor per type, and re-exports `DiagramType` for `Plugin::kind`. New types add a constructor, an entry in `all`, and a feature of the addon crate.
- The configuration is typed section by section ([0008](./0008-canonical-model-and-typed-views.md)): each plugin reads the sections its renderer takes, so the generated reading code of a section is linked only with a plugin that reads it. What every build keeps is the top level and the `MermaidConfig` struct with the defaults of every section, which a view starts from. For an addon with `sequence` alone, the section readers took the size from 6.52 MB to 5.98 MB (stripped, macOS arm64, release profile).
- The command-line tool keeps `Renderer::new`. The addon registers the plugins of its features, all by default, so the published binaries do not change; its `render` keeps a renderer per configuration through `RendererBuilder::shared`, as the facade's shorthand does.
- The snapshots and the geometry runs are unchanged: the plugins call the same functions the `match` called, in the same order, and the output is byte for byte the same.

## Related

[0004](./0004-one-version-one-release.md), [0008](./0008-canonical-model-and-typed-views.md), [0012](./0012-layout-engines-ported-exactly.md), [ARCHITECTURE](../ARCHITECTURE.md).
