//! Placeholders for `packages/mermaid/src/diagrams/state/stateRenderer-v3-unified.spec.js`. The spec
//! draws the diagram with `render` mocked and then dispatches mouse events on the DOM, which the
//! parser port does not cover, so the bodies are not ported. The spec's `it.each` over `node` and
//! `rough-node` gives one test per value, named with the substituted title.

#[test]
#[ignore = "port: renderer and DOM (jsdom, mouse events), not in the parser port"]
fn uses_mermaidtooltip_for_state_click_tooltips_on_node_elements() {
    // stateRenderer-v3-unified.spec.js: "uses mermaidTooltip for state click tooltips on node elements"
}

#[test]
#[ignore = "port: renderer and DOM (jsdom, mouse events), not in the parser port"]
fn uses_mermaidtooltip_for_state_click_tooltips_on_rough_node_elements() {
    // stateRenderer-v3-unified.spec.js: "uses mermaidTooltip for state click tooltips on rough-node elements"
}
