# Port ELK Layered from Java, as its own EPL-2.0 crate

The P1 diagrams that use the shared layout (flowchart, class, state, ER, requirement) only need ELK Layered. elkjs is ELK's Java code compiled to JavaScript with GWT, so we port Layered from the original Java sources, mechanically at first, and check it against elkjs by feeding both the same ELK JSON graph. Mermaid's own ELK integration (`rendering-util/layout-algorithms/elk/render.ts`) is ported along with it. ELK is licensed under the Eclipse Public License 2.0, and a port is a derivative work, so it lives in its own crate, `ferrigura-elk`, under EPL-2.0 while the rest stays MIT; the crate is also useful on its own.

## Consequences

- The other ELK algorithms (`elk.stress`, `elk.force`, `elk.mrtree`, `elk.radial`, `elk.box`, `elk.rectpacking`, `elk.sporeOverlap`) are not ported and fall back to Layered with a warning.
- Mindmaps do not use ELK by default; their layouts are in [0008](./0008-force-directed-layouts.md).
