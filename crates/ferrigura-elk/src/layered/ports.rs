//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/PortSideProcessor.java
//!   and PortListSorter.java
//!
//! The sides and the order of the ports. A port takes the side its edges need: the net
//! flow decides between east (outgoing edges) and west (incoming edges), and the sorter
//! orders the ports of a node by side.

use crate::core::data::OptionValue;
use crate::core::data::enums::{PortConstraints, PortSide};
use crate::java::sort;
use crate::layered::LayoutError;
use crate::layered::lgraph::LGraph;
use crate::layered::options;

/// `PortConstraints.isSideFixed()`: the sides of the ports are given, or the order is.
pub fn is_side_fixed(constraints: PortConstraints) -> bool {
    !matches!(
        constraints,
        PortConstraints::FREE | PortConstraints::UNDEFINED
    )
}

/// `PortConstraints.isOrderFixed()`.
pub fn is_order_fixed(constraints: PortConstraints) -> bool {
    matches!(
        constraints,
        PortConstraints::FIXED_ORDER | PortConstraints::FIXED_RATIO | PortConstraints::FIXED_POS
    )
}

fn constraints_of(graph: &LGraph, node: usize) -> PortConstraints {
    graph.nodes[node]
        .properties
        .enum_of::<PortConstraints>(options::PORT_CONSTRAINTS)
        .unwrap_or(PortConstraints::UNDEFINED)
}

/// `PortSideProcessor.process`: the nodes whose sides are free get every port's side from its
/// net flow, and become side-fixed; the others keep the sides they have.
pub fn process_port_sides(graph: &mut LGraph) {
    for node in graph.layerless_nodes.clone() {
        process_node_sides(graph, node);
    }
    for node in graph.layers.iter().flatten().copied().collect::<Vec<_>>() {
        process_node_sides(graph, node);
    }
}

fn process_node_sides(graph: &mut LGraph, node: usize) {
    let ports = graph.nodes[node].ports.clone();
    if is_side_fixed(constraints_of(graph, node)) {
        for port in ports {
            if graph.ports[port].side == PortSide::UNDEFINED {
                set_port_side(graph, port);
            }
        }
    } else {
        for port in ports {
            set_port_side(graph, port);
        }
        graph.nodes[node].properties.set(
            options::PORT_CONSTRAINTS,
            Some(OptionValue::Enum {
                class: "PortConstraints",
                name: "FIXED_SIDE".to_string(),
            }),
        );
    }
}

/// `PortSideProcessor.setPortSide`: a port with more outgoing than incoming edges goes east,
/// the others west. Ports on external port dummies do not exist in this port yet.
fn set_port_side(graph: &mut LGraph, port: usize) {
    let net_flow =
        graph.ports[port].incoming.len() as i64 - graph.ports[port].outgoing.len() as i64;
    graph.ports[port].side = if net_flow < 0 {
        PortSide::EAST
    } else {
        PortSide::WEST
    };
}

/// `PortListSorter.process`: the ports of each side-fixed node are sorted by side, and the
/// west and south ports are reversed, so that the order runs around the node. Fixed orders
/// and port-degree sorting are not ported yet.
pub fn sort_ports(graph: &mut LGraph) -> Result<(), LayoutError> {
    for node in graph.layers.iter().flatten().copied().collect::<Vec<_>>() {
        let constraints = constraints_of(graph, node);
        if is_order_fixed(constraints) {
            // `CMP_COMBINED`: by side, then by position along the side. The port index that
            // FIXED_ORDER also sorts by is not modelled, since ports have none here.
            let mut ports = graph.nodes[node].ports.clone();
            sort::sort_by(&mut ports, |&a, &b| combined_order(graph, a, b));
            graph.nodes[node].ports = ports;
        } else if is_side_fixed(constraints) {
            let mut ports = graph.nodes[node].ports.clone();
            sort::sort_by(&mut ports, |&a, &b| {
                side_ordinal(graph.ports[a].side).cmp(&side_ordinal(graph.ports[b].side))
            });
            reverse_west_and_south(graph, &mut ports);
            graph.nodes[node].ports = ports;
        }
    }
    Ok(())
}

/// `CMP_PORT_SIDE.thenComparing(CMP_FIXED_ORDER_AND_FIXED_POS)`: the side ordinal, then along
/// the side: x for north and reversed x for south, y for east and reversed y for west. Ports
/// compare as equal when the comparison is between different sides, as the Java does.
fn combined_order(graph: &LGraph, a: usize, b: usize) -> std::cmp::Ordering {
    let (first, second) = (&graph.ports[a], &graph.ports[b]);
    side_ordinal(first.side)
        .cmp(&side_ordinal(second.side))
        .then_with(|| match first.side {
            PortSide::NORTH => first.position.x.total_cmp(&second.position.x),
            PortSide::EAST => first.position.y.total_cmp(&second.position.y),
            PortSide::SOUTH => second.position.x.total_cmp(&first.position.x),
            PortSide::WEST => second.position.y.total_cmp(&first.position.y),
            PortSide::UNDEFINED => std::cmp::Ordering::Equal,
        })
}

/// `PortSide.ordinal()`: the declaration order of the sides, which the generated enum keeps.
fn side_ordinal(side: PortSide) -> usize {
    side as usize
}

/// `PortListSorter.reverseWestAndSouthSide`.
fn reverse_west_and_south(graph: &LGraph, ports: &mut [usize]) {
    if ports.len() <= 1 {
        return;
    }
    let (low, high) = find_side_range(graph, ports, PortSide::SOUTH);
    reverse(ports, low, high);
    let (low, high) = find_side_range(graph, ports, PortSide::WEST);
    reverse(ports, low, high);
}

/// `PortListSorter.findPortSideRange`, including its reading of the current side at `lowIdx`
/// in the second loop, as the Java does.
fn find_side_range(graph: &LGraph, ports: &[usize], side: PortSide) -> (usize, usize) {
    if ports.is_empty() {
        return (0, 0);
    }
    let lb = side_ordinal(side);
    let hb = lb + 1;
    let mut low = 0;
    let mut current = side_ordinal(graph.ports[ports[0]].side);
    while low < ports.len() - 1 && current < lb {
        low += 1;
        current = side_ordinal(graph.ports[ports[low]].side);
    }
    let mut high = low;
    while high < ports.len() - 1 && current < hb {
        high += 1;
        current = side_ordinal(graph.ports[ports[low]].side);
    }
    (low, high)
}

/// `PortListSorter.reverse`: reverses `ports[low..high]`, when it has more than two ports.
fn reverse(ports: &mut [usize], low: usize, high: usize) {
    if high <= low + 2 {
        return;
    }
    let n = (high - low) / 2;
    for i in 0..n {
        ports.swap(low + i, high - i - 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layered::graph_util::create_port;
    use crate::layered::lgraph::{LEdge, LNode, LPort, PortType};

    /// Two nodes joined by one edge, with the ports the importer makes for it.
    fn one_edge_graph() -> LGraph {
        let mut graph = LGraph {
            nodes: vec![LNode::default(), LNode::default()],
            layerless_nodes: vec![0, 1],
            ..LGraph::default()
        };
        let source = create_port(&mut graph, 0, PortType::OUTPUT);
        let target = create_port(&mut graph, 1, PortType::INPUT);
        graph.ports[source].outgoing.push(0);
        graph.ports[target].incoming.push(0);
        graph.edges.push(LEdge {
            source,
            target,
            ..LEdge::default()
        });
        graph
    }

    #[test]
    fn outgoing_ports_go_east_and_incoming_ports_west() {
        let mut graph = one_edge_graph();
        process_port_sides(&mut graph);
        assert_eq!(graph.ports[graph.nodes[0].ports[0]].side, PortSide::EAST);
        assert_eq!(graph.ports[graph.nodes[1].ports[0]].side, PortSide::WEST);
        assert_eq!(
            graph.nodes[0]
                .properties
                .enum_of::<PortConstraints>(options::PORT_CONSTRAINTS),
            Some(PortConstraints::FIXED_SIDE)
        );
    }

    #[test]
    fn a_run_of_two_or_fewer_ports_is_left_as_it_is() {
        // A south or west run of at most two ports is not reversed.
        let mut graph = LGraph::default();
        let sides = [PortSide::WEST, PortSide::SOUTH, PortSide::EAST];
        for side in sides {
            graph.ports.push(LPort::new(0, side));
        }
        let mut ports = vec![0, 1, 2];
        reverse_west_and_south(&graph, &mut ports);
        assert_eq!(ports, vec![0, 1, 2]);
    }
}
