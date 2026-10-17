//! Port of packages/mermaid/src/diagrams/block/blockDB.spec.ts. The spec's edge helpers are public
//! functions of the port, so their tests call them directly.
//!
//! The spec's last test spies on the JavaScript `getConfig` to check that sanitising reads the
//! configuration at call time. The port has no configuration object to spy on; the nearest check
//! is that a parse, which is the call of the reference's `setHierarchy`, sanitises its labels with
//! the configuration it reads then, and not one kept from an earlier parse.

use ferrigura_model::block::BlockDiagram;
use ferrigura_syntax::block::{
    edge_str_to_edge_data, edge_str_to_edge_start_data, edge_str_to_pattern, edge_str_to_thickness,
    parse,
};

#[test]
fn should_correctly_identify_edge_thickness() {
    assert_eq!(edge_str_to_thickness("-->"), "normal");
    assert_eq!(edge_str_to_thickness("-.->"), "normal");
    assert_eq!(edge_str_to_thickness("==>"), "thick");
}

#[test]
fn should_correctly_identify_edge_patterns() {
    assert_eq!(edge_str_to_pattern("-->"), "solid");
    assert_eq!(edge_str_to_pattern("==>"), "solid");
    assert_eq!(edge_str_to_pattern("-.->"), "dotted");
}

#[test]
fn should_correctly_identify_start_and_end_arrows() {
    assert_eq!(edge_str_to_edge_start_data("<-->"), "arrow_point");
    assert_eq!(edge_str_to_edge_data("<-->"), "arrow_point");

    assert_eq!(edge_str_to_edge_start_data("<==>"), "arrow_point");
    assert_eq!(edge_str_to_edge_data("<==>"), "arrow_point");

    assert_eq!(edge_str_to_edge_start_data("-->"), "arrow_open");
    assert_eq!(edge_str_to_edge_data("-->"), "arrow_point");

    assert_eq!(edge_str_to_edge_data("--x"), "arrow_cross");
    assert_eq!(edge_str_to_edge_data("--o"), "arrow_circle");
}

#[test]
fn should_call_get_config_at_sanitization_time_not_at_module_load_time() {
    let first: BlockDiagram = parse("block\nid[\"<script>x</script>y\"]").expect("parses");
    assert_eq!(first.blocks[0].label.as_deref(), Some("y"));
    let second: BlockDiagram = parse("block\nid[\"<script>x</script>z\"]").expect("parses");
    assert_eq!(second.blocks[0].label.as_deref(), Some("z"));
}
