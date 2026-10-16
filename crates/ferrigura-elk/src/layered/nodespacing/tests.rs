//! Tests of the node spacing reduced path. The expected values are worked out by hand from the
//! Java code, as the comments say; each comment gives the step that produces the value.

use crate::core::data::enums::{NodeType, PortAlignment, PortSide};
use crate::core::data::{OptionValue, option_by_id};
use crate::core::math::{KVector, Spacing};
use crate::layered::LayoutError;
use crate::layered::lgraph::{LGraph, LNode, LPort};
use crate::layered::options;

use super::context::{
    NODE_LABELS_PADDING, NODE_LABELS_PLACEMENT, PORT_ALIGNMENT_EAST, PORT_ALIGNMENT_NORTH,
    PORT_ALIGNMENT_SOUTH, PORT_ALIGNMENT_WEST, PORT_LABELS_PLACEMENT, SPACING_INDIVIDUAL,
    SPACING_PORTS_SURROUNDING,
};
use super::sizing::spread_ports;
use super::{Rect, label_and_node_size, node_margins};
use crate::layered::spacings::{SPACING_LABEL_PORT_VERTICAL, SPACING_PORT_PORT};

const TOLERANCE: f64 = 1e-9;

fn assert_near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < TOLERANCE,
        "{actual} is not {expected}"
    );
}

/// A graph with one node of the given size, in the one layer.
fn node_graph(width: f64, height: f64) -> LGraph {
    let mut graph = LGraph::default();
    graph.nodes.push(LNode {
        size: KVector {
            x: width,
            y: height,
        },
        ..LNode::default()
    });
    graph.layers.push(vec![0]);
    graph
}

/// Adds a port to `node` at `position` (relative to the node) with `size`, and returns its index.
fn add_port(
    graph: &mut LGraph,
    node: usize,
    side: PortSide,
    position: (f64, f64),
    size: (f64, f64),
) -> usize {
    let mut port = LPort::new(node, side);
    port.position = KVector {
        x: position.0,
        y: position.1,
    };
    port.size = KVector {
        x: size.0,
        y: size.1,
    };
    graph.ports.push(port);
    let index = graph.ports.len() - 1;
    graph.nodes[node].ports.push(index);
    index
}

fn set_alignment(graph: &mut LGraph, node: usize, id: &'static str, name: &str) {
    graph.nodes[node].properties.set(
        id,
        Some(OptionValue::Enum {
            class: "PortAlignment",
            name: name.to_string(),
        }),
    );
}

fn set_enum_set(
    graph: &mut LGraph,
    node: usize,
    id: &'static str,
    class: &'static str,
    names: &[&str],
) {
    graph.nodes[node].properties.set(
        id,
        Some(OptionValue::EnumSet {
            class,
            names: names.iter().map(|name| name.to_string()).collect(),
        }),
    );
}

fn set_enum(graph: &mut LGraph, node: usize, id: &'static str, class: &'static str, name: &str) {
    graph.nodes[node].properties.set(
        id,
        Some(OptionValue::Enum {
            class,
            name: name.to_string(),
        }),
    );
}

fn set_surrounding(graph: &mut LGraph, top: f64, left: f64, bottom: f64, right: f64) {
    graph.properties.set(
        SPACING_PORTS_SURROUNDING,
        Some(OptionValue::Margin(Spacing {
            top,
            left,
            bottom,
            right,
        })),
    );
}

fn position_of(graph: &LGraph, port: usize) -> KVector {
    graph.ports[port].position
}

#[test]
fn margins_wrap_the_node_and_its_ports_on_all_four_sides() {
    let mut graph = node_graph(100.0, 50.0);
    // North: the box starts 4 above the node. West: 7 to the left. South: ends 6 below.
    // East: ends at x = 105, 5 past the node.
    add_port(&mut graph, 0, PortSide::NORTH, (-3.0, -4.0), (6.0, 4.0));
    add_port(&mut graph, 0, PortSide::EAST, (100.0, 10.0), (5.0, 5.0));
    add_port(&mut graph, 0, PortSide::SOUTH, (20.0, 50.0), (4.0, 6.0));
    add_port(&mut graph, 0, PortSide::WEST, (-7.0, 20.0), (7.0, 2.0));
    node_margins(&mut graph).unwrap();
    assert_eq!(
        graph.nodes[0].margin,
        Spacing {
            top: 4.0,
            left: 7.0,
            bottom: 6.0,
            right: 5.0
        }
    );
    assert_eq!(graph.nodes[0].size, KVector { x: 100.0, y: 50.0 });
}

#[test]
fn margins_are_relative_to_the_node_position() {
    // The port's box is at (110, 30) with size 5: it reaches x = 115, five past the node's right
    // side at x = 110. Nothing reaches above y = 20 or below y = 70.
    let mut graph = node_graph(100.0, 50.0);
    graph.nodes[0].position = KVector { x: 10.0, y: 20.0 };
    add_port(&mut graph, 0, PortSide::EAST, (100.0, 10.0), (5.0, 5.0));
    node_margins(&mut graph).unwrap();
    assert_eq!(
        graph.nodes[0].margin,
        Spacing {
            top: 0.0,
            left: 0.0,
            bottom: 0.0,
            right: 5.0
        }
    );
}

#[test]
fn a_node_without_ports_gets_zero_margins() {
    // The bounding box is the node's own box, so every margin is zero, whatever it was before.
    let mut graph = node_graph(40.0, 30.0);
    graph.nodes[0].margin = Spacing {
        top: 3.0,
        left: 3.0,
        bottom: 3.0,
        right: 3.0,
    };
    node_margins(&mut graph).unwrap();
    assert_eq!(graph.nodes[0].margin, Spacing::default());
    assert_eq!(graph.nodes[0].size, KVector { x: 40.0, y: 30.0 });
}

#[test]
fn margins_are_set_for_every_node_of_a_layer_including_dummies() {
    let mut graph = node_graph(100.0, 50.0);
    graph.nodes[0].node_type = NodeType::LONG_EDGE;
    add_port(&mut graph, 0, PortSide::EAST, (100.0, 0.0), (5.0, 5.0));
    node_margins(&mut graph).unwrap();
    assert_near(graph.nodes[0].margin.right, 5.0);
}

#[test]
fn a_node_without_ports_keeps_its_size_and_gets_no_positions() {
    let mut graph = node_graph(40.0, 30.0);
    label_and_node_size(&mut graph).unwrap();
    assert_eq!(graph.nodes[0].size, KVector { x: 40.0, y: 30.0 });
    assert!(graph.nodes[0].ports.is_empty());
}

#[test]
fn free_ports_on_one_side_are_distributed_by_default() {
    // The west cell needs 5 + 10 + 5 = 20, and distributed alignment adds 2 * 10: 40. The node is
    // 50 high, so the spacing is 10 + (50 - 40) / 3 = 40 / 3 around the ports. The port with the
    // larger volatile id comes first on the west side.
    let mut graph = node_graph(100.0, 50.0);
    let first = add_port(&mut graph, 0, PortSide::WEST, (0.0, 0.0), (5.0, 5.0));
    let second = add_port(&mut graph, 0, PortSide::WEST, (0.0, 0.0), (5.0, 5.0));
    label_and_node_size(&mut graph).unwrap();
    let upper = position_of(&graph, second);
    let lower = position_of(&graph, first);
    assert_near(upper.x, -5.0);
    assert_near(upper.y, 40.0 / 3.0);
    assert_near(lower.x, -5.0);
    assert_near(lower.y, 40.0 / 3.0 + 5.0 + 40.0 / 3.0);
    assert_eq!(graph.nodes[0].size, KVector { x: 100.0, y: 50.0 });
}

#[test]
fn centered_west_ports_start_at_the_middle_of_the_free_space() {
    // Content 5 + 10 + 5 = 20 in 50: the first port starts at (50 - 20) / 2 = 15.
    let mut graph = node_graph(100.0, 50.0);
    set_alignment(&mut graph, 0, PORT_ALIGNMENT_WEST, "CENTER");
    let first = add_port(&mut graph, 0, PortSide::WEST, (0.0, 0.0), (5.0, 5.0));
    let second = add_port(&mut graph, 0, PortSide::WEST, (0.0, 0.0), (5.0, 5.0));
    label_and_node_size(&mut graph).unwrap();
    assert_near(position_of(&graph, second).y, 15.0);
    assert_near(position_of(&graph, first).y, 30.0);
}

#[test]
fn north_and_south_ports_are_placed_along_the_width() {
    // North: the content is 10 + 10 + 10 = 30 in the 100 wide cell, centered at 35. The ports are
    // 10 apart. South: the larger volatile id comes first, and the ports sit on the bottom edge.
    let mut graph = node_graph(100.0, 50.0);
    set_alignment(&mut graph, 0, PORT_ALIGNMENT_NORTH, "CENTER");
    set_alignment(&mut graph, 0, PORT_ALIGNMENT_SOUTH, "CENTER");
    let north_a = add_port(&mut graph, 0, PortSide::NORTH, (0.0, 0.0), (10.0, 4.0));
    let north_b = add_port(&mut graph, 0, PortSide::NORTH, (0.0, 0.0), (10.0, 4.0));
    let south_c = add_port(&mut graph, 0, PortSide::SOUTH, (0.0, 0.0), (10.0, 4.0));
    let south_d = add_port(&mut graph, 0, PortSide::SOUTH, (0.0, 0.0), (10.0, 4.0));
    label_and_node_size(&mut graph).unwrap();
    assert_near(position_of(&graph, north_a).x, 35.0);
    assert_near(position_of(&graph, north_b).x, 55.0);
    assert_near(position_of(&graph, north_a).y, -4.0);
    assert_near(position_of(&graph, south_d).x, 35.0);
    assert_near(position_of(&graph, south_c).x, 55.0);
    assert_near(position_of(&graph, south_c).y, 50.0);
    assert_near(position_of(&graph, south_d).y, 50.0);
}

#[test]
fn a_single_east_port_is_centered_on_its_side() {
    // The eastern content is 5 + 2 * 10 = 25 under the default alignment. A single port makes that
    // centered: 25 - 20 = 5, and (50 - 5) / 2 = 22.5 from the top.
    let mut graph = node_graph(100.0, 50.0);
    let port = add_port(&mut graph, 0, PortSide::EAST, (0.0, 0.0), (5.0, 5.0));
    label_and_node_size(&mut graph).unwrap();
    assert_near(position_of(&graph, port).x, 100.0);
    assert_near(position_of(&graph, port).y, 22.5);
}

#[test]
fn surrounding_margins_and_the_top_border_offset_move_the_ports() {
    // Surrounding margins top 4, left 6, bottom 8, right 2.
    // North: the cell has padding 6 and 2, so 100 - 8 = 92 is free and centering gives 31 more:
    // the first port starts at 6 + 31 = 37.
    // West and east: the top padding loses the north cell's height, 1 (its vertical label spacing),
    // so 4 - 1 = 3; the bottom padding loses the south cell's 1, so 8 - 1 = 7. In the 50 that
    // remain, 40 - 20 = 20 centers the ports at 3 + 10 = 13.
    let mut graph = node_graph(100.0, 50.0);
    set_surrounding(&mut graph, 4.0, 6.0, 8.0, 2.0);
    set_alignment(&mut graph, 0, PORT_ALIGNMENT_NORTH, "CENTER");
    set_alignment(&mut graph, 0, PORT_ALIGNMENT_WEST, "CENTER");
    let north_a = add_port(&mut graph, 0, PortSide::NORTH, (0.0, 0.0), (10.0, 4.0));
    let north_b = add_port(&mut graph, 0, PortSide::NORTH, (0.0, 0.0), (10.0, 4.0));
    let west_a = add_port(&mut graph, 0, PortSide::WEST, (0.0, 0.0), (5.0, 5.0));
    let west_b = add_port(&mut graph, 0, PortSide::WEST, (0.0, 0.0), (5.0, 5.0));
    label_and_node_size(&mut graph).unwrap();
    assert_near(position_of(&graph, north_a).x, 37.0);
    assert_near(position_of(&graph, north_b).x, 57.0);
    assert_near(position_of(&graph, west_b).y, 13.0);
    assert_near(position_of(&graph, west_a).y, 28.0);
}

#[test]
fn free_fixed_side_and_fixed_order_place_the_ports_alike() {
    // FIXED_SIDE is what the port side processor leaves behind. FIXED_ORDER takes the free branch
    // of every Java switch on the constraints. The label and size step treats all three like FREE.
    let place = |constraints: &str| {
        let mut graph = node_graph(100.0, 50.0);
        set_enum(
            &mut graph,
            0,
            options::PORT_CONSTRAINTS,
            "PortConstraints",
            constraints,
        );
        let west = add_port(&mut graph, 0, PortSide::WEST, (0.0, 0.0), (5.0, 5.0));
        let east = add_port(&mut graph, 0, PortSide::EAST, (0.0, 0.0), (5.0, 5.0));
        label_and_node_size(&mut graph).unwrap();
        (position_of(&graph, west), position_of(&graph, east))
    };
    assert_eq!(place("FREE"), place("FIXED_SIDE"));
    assert_eq!(place("FREE"), place("FIXED_ORDER"));
}

#[test]
fn every_side_is_placed_against_the_node_border() {
    let mut graph = node_graph(100.0, 50.0);
    let north = add_port(&mut graph, 0, PortSide::NORTH, (0.0, 0.0), (6.0, 4.0));
    let east = add_port(&mut graph, 0, PortSide::EAST, (0.0, 0.0), (5.0, 5.0));
    let south = add_port(&mut graph, 0, PortSide::SOUTH, (0.0, 0.0), (6.0, 4.0));
    let west = add_port(&mut graph, 0, PortSide::WEST, (0.0, 0.0), (5.0, 5.0));
    label_and_node_size(&mut graph).unwrap();
    assert_near(position_of(&graph, north).y, -4.0);
    assert_near(position_of(&graph, south).y, 50.0);
    assert_near(position_of(&graph, east).x, 100.0);
    assert_near(position_of(&graph, west).x, -5.0);
    assert_eq!(graph.nodes[0].size, KVector { x: 100.0, y: 50.0 });
}

#[test]
fn only_normal_nodes_of_the_layers_are_placed() {
    let mut graph = node_graph(100.0, 50.0);
    graph.nodes[0].node_type = NodeType::LONG_EDGE;
    let port = add_port(&mut graph, 0, PortSide::WEST, (0.0, 0.0), (5.0, 5.0));
    label_and_node_size(&mut graph).unwrap();
    assert_eq!(position_of(&graph, port), KVector::default());
    assert_eq!(graph.nodes[0].size, KVector { x: 100.0, y: 50.0 });
}

#[test]
fn spreading_squeezes_the_ports_when_they_do_not_fit_and_may_not_overhang() {
    // Two ports need 30 in 20. Centered, the spacing goes to 10 + (20 - 30) / 1 = 0, so the ports
    // touch and the first one starts at the start of the free space.
    assert_eq!(
        spread_ports(20.0, 30.0, 2, PortAlignment::CENTER, 10.0, false),
        (0.0, 0.0)
    );
    // Distributed: spacing 10 + (20 - 30) / 3 = 20 / 3, and the first port moves by that much.
    let (offset, spacing) = spread_ports(20.0, 30.0, 2, PortAlignment::DISTRIBUTED, 10.0, false);
    assert_near(spacing, 20.0 / 3.0);
    assert_near(offset, 20.0 / 3.0);
}

#[test]
fn spreading_centers_when_the_ports_overhang() {
    // Distributed content 30 with the ports allowed to overhang: 30 - 2 * 10 = 10 is the content
    // centered, so the first port starts at (20 - 10) / 2 = 5, and the spacing stays 10.
    assert_eq!(
        spread_ports(20.0, 30.0, 2, PortAlignment::DISTRIBUTED, 10.0, true),
        (5.0, 10.0)
    );
}

#[test]
fn a_single_justified_port_is_centered() {
    // One port of 5 in 50, justified. Justified content has no extra spacing (only distributed
    // adds it), so the placement is 5, the alignment becomes centered, and the offset is
    // (50 - 5) / 2. The spacing is the port-port spacing, which nothing changes.
    assert_eq!(
        spread_ports(50.0, 5.0, 1, PortAlignment::JUSTIFIED, 10.0, false),
        (22.5, 10.0)
    );
}

#[test]
fn fixed_ratio_is_not_ported() {
    let mut graph = node_graph(100.0, 50.0);
    set_enum(
        &mut graph,
        0,
        options::PORT_CONSTRAINTS,
        "PortConstraints",
        "FIXED_RATIO",
    );
    add_port(&mut graph, 0, PortSide::WEST, (0.0, 0.0), (5.0, 5.0));
    assert_unsupported(&mut graph, "FIXED_RATIO");
}

#[test]
fn fixed_pos_north_and_south_ports_keep_their_x() {
    // The x of a fixed port is its own, and the alignment does not move it. North: the port's y
    // is minus its height, -4. South: the y starts on the border, 0, and the node's height, 50,
    // is added to it. The node keeps its size.
    let mut graph = node_graph(100.0, 50.0);
    set_enum(
        &mut graph,
        0,
        options::PORT_CONSTRAINTS,
        "PortConstraints",
        "FIXED_POS",
    );
    set_alignment(&mut graph, 0, PORT_ALIGNMENT_NORTH, "CENTER");
    let north = add_port(&mut graph, 0, PortSide::NORTH, (20.0, 99.0), (10.0, 4.0));
    let south = add_port(&mut graph, 0, PortSide::SOUTH, (60.0, 7.0), (10.0, 4.0));
    label_and_node_size(&mut graph).unwrap();
    assert_near(position_of(&graph, north).x, 20.0);
    assert_near(position_of(&graph, north).y, -4.0);
    assert_near(position_of(&graph, south).x, 60.0);
    assert_near(position_of(&graph, south).y, 50.0);
    assert_eq!(graph.nodes[0].size, KVector { x: 100.0, y: 50.0 });
}

#[test]
fn fixed_pos_east_and_west_ports_keep_their_y_whatever_the_margins() {
    // The y of a fixed port is its own, so neither the alignment nor the surrounding margins move
    // it. West: x is minus the width, -5. East: x is the node's width, 100.
    let mut graph = node_graph(100.0, 50.0);
    set_enum(
        &mut graph,
        0,
        options::PORT_CONSTRAINTS,
        "PortConstraints",
        "FIXED_POS",
    );
    set_surrounding(&mut graph, 4.0, 6.0, 8.0, 2.0);
    set_alignment(&mut graph, 0, PORT_ALIGNMENT_WEST, "CENTER");
    let west = add_port(&mut graph, 0, PortSide::WEST, (0.0, 10.0), (5.0, 5.0));
    let east = add_port(&mut graph, 0, PortSide::EAST, (0.0, 40.0), (5.0, 5.0));
    label_and_node_size(&mut graph).unwrap();
    assert_near(position_of(&graph, west).x, -5.0);
    assert_near(position_of(&graph, west).y, 10.0);
    assert_near(position_of(&graph, east).x, 100.0);
    assert_near(position_of(&graph, east).y, 40.0);
}

#[test]
fn size_constraints_are_not_ported() {
    let mut graph = node_graph(100.0, 50.0);
    set_enum_set(
        &mut graph,
        0,
        options::NODE_SIZE_CONSTRAINTS,
        "SizeConstraint",
        &["PORTS"],
    );
    add_port(&mut graph, 0, PortSide::WEST, (0.0, 0.0), (5.0, 5.0));
    assert_unsupported(&mut graph, "size constraints");
}

#[test]
fn fixed_port_label_placement_is_not_ported() {
    let mut graph = node_graph(100.0, 50.0);
    set_enum_set(
        &mut graph,
        0,
        PORT_LABELS_PLACEMENT,
        "PortLabelPlacement",
        &[],
    );
    add_port(&mut graph, 0, PortSide::WEST, (0.0, 0.0), (5.0, 5.0));
    assert_unsupported(&mut graph, "fixed port labels");
}

#[test]
fn compute_padding_is_not_ported() {
    let mut graph = node_graph(100.0, 50.0);
    set_enum_set(
        &mut graph,
        0,
        options::NODE_SIZE_OPTIONS,
        "SizeOptions",
        &["COMPUTE_PADDING"],
    );
    add_port(&mut graph, 0, PortSide::WEST, (0.0, 0.0), (5.0, 5.0));
    assert_unsupported(&mut graph, "COMPUTE_PADDING");
}

#[test]
fn individual_spacings_are_not_ported() {
    let mut graph = node_graph(100.0, 50.0);
    graph.nodes[0].properties.set(
        SPACING_INDIVIDUAL,
        Some(OptionValue::String("individual".to_string())),
    );
    add_port(&mut graph, 0, PortSide::WEST, (0.0, 0.0), (5.0, 5.0));
    assert_unsupported(&mut graph, "individual spacings");
}

#[test]
fn a_port_without_a_side_is_an_error() {
    let mut graph = node_graph(100.0, 50.0);
    add_port(&mut graph, 0, PortSide::UNDEFINED, (0.0, 0.0), (5.0, 5.0));
    let before = graph.clone();
    let error = label_and_node_size(&mut graph).unwrap_err();
    assert!(matches!(error, LayoutError::Format(_)), "{error}");
    assert_untouched(&graph, &before);
}

#[test]
fn an_invalid_port_label_placement_is_an_error() {
    let mut graph = node_graph(100.0, 50.0);
    set_enum_set(
        &mut graph,
        0,
        PORT_LABELS_PLACEMENT,
        "PortLabelPlacement",
        &["INSIDE", "OUTSIDE"],
    );
    add_port(&mut graph, 0, PortSide::WEST, (0.0, 0.0), (5.0, 5.0));
    let before = graph.clone();
    let error = label_and_node_size(&mut graph).unwrap_err();
    assert!(matches!(error, LayoutError::Format(_)), "{error}");
    assert_untouched(&graph, &before);
}

#[test]
fn an_invalid_node_label_placement_is_an_error() {
    let mut graph = node_graph(100.0, 50.0);
    set_enum_set(
        &mut graph,
        0,
        NODE_LABELS_PLACEMENT,
        "NodeLabelPlacement",
        &["H_LEFT", "H_RIGHT"],
    );
    add_port(&mut graph, 0, PortSide::WEST, (0.0, 0.0), (5.0, 5.0));
    let before = graph.clone();
    let error = label_and_node_size(&mut graph).unwrap_err();
    assert!(matches!(error, LayoutError::Format(_)), "{error}");
    assert_untouched(&graph, &before);
}

#[test]
fn an_error_on_a_later_node_leaves_the_first_node_unchanged() {
    // The first node is fine and would be placed; the second one fails, so nothing is written.
    let mut graph = node_graph(100.0, 50.0);
    add_port(&mut graph, 0, PortSide::WEST, (0.0, 0.0), (5.0, 5.0));
    graph.nodes.push(LNode {
        size: KVector { x: 10.0, y: 10.0 },
        ..LNode::default()
    });
    graph.layers[0].push(1);
    set_enum(
        &mut graph,
        1,
        options::PORT_CONSTRAINTS,
        "PortConstraints",
        "FIXED_RATIO",
    );
    add_port(&mut graph, 1, PortSide::WEST, (0.0, 0.0), (5.0, 5.0));
    let before = graph.clone();
    assert!(label_and_node_size(&mut graph).is_err());
    assert_untouched(&graph, &before);
}

#[test]
fn the_option_ids_are_in_the_registry() {
    for id in [
        PORT_LABELS_PLACEMENT,
        NODE_LABELS_PLACEMENT,
        NODE_LABELS_PADDING,
        SPACING_PORT_PORT,
        SPACING_LABEL_PORT_VERTICAL,
        SPACING_PORTS_SURROUNDING,
        SPACING_INDIVIDUAL,
        PORT_ALIGNMENT_NORTH,
        PORT_ALIGNMENT_EAST,
        PORT_ALIGNMENT_SOUTH,
        PORT_ALIGNMENT_WEST,
        options::NODE_SIZE_CONSTRAINTS,
        options::NODE_SIZE_OPTIONS,
        options::NODE_SIZE_FIXED_GRAPH_SIZE,
        options::PORT_CONSTRAINTS,
        options::SPACING_LABEL_LABEL,
    ] {
        assert!(option_by_id(id).is_some(), "{id} is not registered");
    }
}

#[test]
fn union_spans_both_rectangles() {
    let mut rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    rect.union(&Rect::new(5.0, -5.0, 10.0, 10.0));
    assert_eq!(rect, Rect::new(0.0, -5.0, 15.0, 15.0));
}

/// Checks that the graph's nodes and ports are the same as in `before`, after a failed step.
fn assert_untouched(graph: &LGraph, before: &LGraph) {
    for (now, then) in graph.nodes.iter().zip(&before.nodes) {
        assert_eq!(now.size, then.size);
    }
    for (now, then) in graph.ports.iter().zip(&before.ports) {
        assert_eq!(now.position, then.position);
    }
}

/// Runs the label and size step on a graph that should be refused with `Unsupported`.
fn assert_unsupported(graph: &mut LGraph, what: &str) {
    let before = graph.clone();
    let error = label_and_node_size(graph).unwrap_err();
    assert!(
        matches!(error, LayoutError::Unsupported(_)),
        "{what}: {error}"
    );
    assert_untouched(graph, &before);
}
