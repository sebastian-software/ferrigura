//! The flowchart's style sheet and look definitions: a port of
//! `packages/mermaid/src/diagrams/flowchart/styles.ts`, the rules of `packages/mermaid/src/styles.ts`
//! that a flowchart gets, the `classDef` rules, and `insertLookDefs.ts`. Colours follow
//! `docs-port/decisions/0007`: custom properties on the diagram's id, presentation attributes as
//! the fallback. See `docs-port/flowchart/render-notes/styles.md`.

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::flowchart::FlowClass;

use crate::pie::Themes;

/// The `<style>` element's content for a flowchart with the id `svg_id`.
#[must_use]
pub fn style_sheet(
    svg_id: &str,
    themes: Themes<'_>,
    config: &MermaidConfig,
    classes: &[FlowClass],
) -> String {
    let _ = (svg_id, themes, config, classes);
    todo!("port the flowchart styles (task T7)")
}

/// The `<defs>` that the look's rules refer to: the two drop shadows, and the gradient when the
/// theme uses one.
#[must_use]
pub fn look_defs(svg_id: &str, themes: Themes<'_>, config: &MermaidConfig) -> String {
    let _ = (svg_id, themes, config);
    todo!("port insertLookDefs (task T7)")
}
