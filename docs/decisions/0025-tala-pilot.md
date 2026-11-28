# 0025 TALA in a separate layout pilot

**Scope.** The development harness that evaluates D2's TALA on measured Mermaid
flowcharts, its comparison with ELK Layered, and the boundary of its Rust slice.

## Decision

The pilot compares the upstream Go TALA layout and Ferrigura's ELK layout on
the same Mermaid input. Ferrigura supplies the measured nodes and labels and
paints both SVGs. The engine may change placement, dimensions, routes, and
label positions. The comparison keeps font, theme, SVG styling, and diagram
content constant. Use nested groups with labeled cross-group edges and cycles,
and compare a long directional flow as a control.

The pilot's Rust slice covers the terminal `Normalize` stage of the seed
engine for unpinned graphs. It uses the positions, routes, and edge-label
geometry supplied by the pinned Go engine. Compare that stage with Go on
the same input. The whole-diagram comparison independently measures whether
TALA is useful on these diagrams. Geometry differences between TALA and ELK
are expected in this comparison.

The Rust slice lives in an unpublished MPL-2.0 crate under
`tools/tala-pilot/rust/`. Ferrigura's measurements, adapters, and SVG paint
code continue under MIT in separate files. ELK retains its EPL-2.0 notices.
The public renderer's engine selection and the pinned Mermaid comparison
continue using the existing path.

## Why

TALA is designed for software architecture diagrams where nested boundaries,
cross-group routing, and visual balance matter. Measuring the same Mermaid
source once and painting both layouts in Ferrigura isolates differences that
a user would see when choosing an engine. A narrow, replayable Rust stage
tests the port and the graph boundary before translating the larger placement
and routing pipeline.

## Consequences

- Go is a development dependency for capturing the TALA reference. Compiled
  Ferrigura packages and ordinary rendering do not depend on it.
- Keep the pin, source-to-target map, and license notices with the pilot.
  Rust files translated from the upstream source carry MPL-2.0.
- A distributed executable that includes covered Rust source carries the
  notice and tells recipients where to obtain the exact covered Rust source,
  including modifications. The release tag or commit identifies Ferrigura's
  source while the D2 pin identifies the provenance.

## Related

[0012](./0012-layout-engines-ported-exactly.md),
[0013](./0013-licenses-of-ported-code.md),
[0015](./0015-differential-testing.md),
[0022](./0022-stable-public-api.md),
[TALA provenance](../../tools/tala-pilot/PROVENANCE.md).
