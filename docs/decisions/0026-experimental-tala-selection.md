# 0026 Select experimental TALA explicitly

**Scope.** The engine selection used by Ferrigura and the distribution
boundary of the experimental TALA port.

## Decision

Request the engine on the render call in Rust with
`Renderer::render_with_engine`, in Node.js with `layoutEngine`, and in the
CLI with `--layout-engine`. Without this request, the diagram keeps the
engine it normally uses. The request takes precedence over the Mermaid
document's layout setting. An explicit `elk` is available for the diagram
types that use ELK. An explicit `tala` uses the experimental path for
supported flowcharts in Ferrigura's default look.

`compat` keeps the Mermaid-reference geometry and cannot be combined with
TALA. A request for an engine that cannot draw the graph returns an error.

The Cargo feature `experimental-tala` is off by default. It links the
separately licensed `ferrigura-tala` crate for native-only layout. The
experimental subset covers flat disconnected rectangles, isolated rectangular
self-loops, bounded straight paths and parallel edges, bounded container
packing, and individual routes within a container or across one boundary.
The ordinary seeded path also handles a single rectangular component with
cycles and labeled routes. More general compound graphs return an
unsupported-graph error. The [crate README](../../crates/ferrigura-tala/README.md)
records the current size and topology limits.

## Consequences

- The Rust option uses a separate method. Existing `RenderOptions` struct
  literals and entry points remain valid.
- The default CLI and npm binaries keep their existing dependency and
  notice sets. The TALA source package includes its MPL-2.0 notice.
  Anyone distributing a feature-enabled binary provides the matching
  covered Rust source and notice. Decision 0004 describes the release workflow
  for the new crate and its transition to Trusted Publishing.
- The Go reference runs in the development harness. Runtime dispatch,
  layout and SVG rendering all run in Rust.

[Release process](./0004-one-version-one-release.md) ·
[License decision](./0013-licenses-of-ported-code.md) ·
[Compatibility mode](./0019-mermaid-geometry-with-compat.md) ·
[Pilot](./0025-tala-pilot.md) ·
[TALA notice](../../crates/ferrigura-tala/NOTICE.md)
