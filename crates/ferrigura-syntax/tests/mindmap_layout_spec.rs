//! Port of `packages/mermaid/src/diagrams/mindmap/mindmapLayout.spec.ts`. Test names follow the
//! spec's titles in snake_case.
//!
//! The layout selection is `getData`'s (`config.layout` becomes `cose-bilkent` when the user set no
//! layout), which is the renderer stage: every test is ignored with that reason. The third test
//! repeats the second in the reference, and is ported as it is.

#[test]
#[ignore = "port: the layout selection is getData, the renderer stage (docs-port/mindmap/PLAN.md, section 3.5)"]
fn uses_cose_bilkent_when_no_layout_was_requested_despite_the_elk_default() {}

#[test]
#[ignore = "port: the layout selection is getData, the renderer stage (docs-port/mindmap/PLAN.md, section 3.5)"]
fn honours_a_layout_the_diagram_explicitly_asks_for() {}

#[test]
#[ignore = "port: the layout selection is getData, the renderer stage (docs-port/mindmap/PLAN.md, section 3.5)"]
fn honours_an_explicit_elk() {}
