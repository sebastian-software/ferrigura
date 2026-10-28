//! Port of `rendering-elements/lineJump.integration.spec.ts`: whether the routed swimlane layout of
//! the `7-car-sales-constr` fixture has an edge crossing.
//!
//! Ignored: the case runs the browser-side pipeline (the parser, then the swimlane layout), which
//! the flowchart port does not have, and it reads the `e2e` fixtures.

#[test]
#[ignore = "port: browser-only integration"]
fn the_routed_swimlane_layout_has_at_least_one_edge_crossing() {}
