//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/SelfLoopPreProcessor.java,
//!   SelfLoopPortRestorer.java, SelfLoopRouter.java and SelfLoopPostProcessor.java, and the
//!   intermediate/loops package they use.
//!
//! Self loops: edges whose source and target are on the same node. The four processors run in the
//! order the orthogonal edge router adds them: the preprocessor before phase 1, the port restorer and
//! the router before phase 4, and the postprocessor after phase 5. Between the preprocessor and the
//! postprocessor the loops' edges are detached from their ports, so the phases in between do not see
//! them.
//!
//! Only simple self loops are ported: each edge has its own ports, as the importer makes them, so each
//! hyper loop holds one edge. Hyper loops of several edges are reported as unsupported, and so are
//! the polyline and spline routers. The hierarchical cases (nested graphs, hierarchical ports) are not
//! ported.

mod labels;
mod model;
mod ordering;
mod router;
mod routing;
mod slots;

pub use model::SelfLoopHolder;

use crate::core::data::enums::{EdgeRouting, NodeType};
use crate::layered::LayoutError;
use crate::layered::lgraph::LGraph;
use crate::layered::options;

/// `SelfLoopPreProcessor.process`: installs a holder on each node with self loops, detaches the loop
/// edges, and hides the ports that have only loops.
pub fn preprocess(graph: &mut LGraph) -> Result<(), LayoutError> {
    for node in graph.layerless_nodes.clone() {
        if !model::needs_self_loop_processing(graph, node) {
            continue;
        }
        let mut holder = SelfLoopHolder::initialize(graph, node)?;
        model::hide(graph, &mut holder)?;
        graph.nodes[node].self_loops = Some(holder);
    }
    Ok(())
}

/// `SelfLoopPortRestorer.process`: "Self-Loop ordering".
pub fn restore_ports(graph: &mut LGraph) -> Result<(), LayoutError> {
    ordering::restore_hidden_ports(graph)
}

/// `SelfLoopRouter.process`: "Self-Loop routing", with the orthogonal router.
pub fn route(graph: &mut LGraph) -> Result<(), LayoutError> {
    let router = graph
        .properties
        .enum_with_default::<EdgeRouting>(options::EDGE_ROUTING);
    if !matches!(router, EdgeRouting::ORTHOGONAL | EdgeRouting::UNDEFINED) {
        return Err(LayoutError::Unsupported(
            "self loops with a polyline or spline router".to_string(),
        ));
    }
    for node in regular_nodes_with_loops(graph) {
        let Some(mut holder) = graph.nodes[node].self_loops.take() else {
            continue;
        };
        let result = route_node(graph, &mut holder);
        graph.nodes[node].self_loops = Some(holder);
        result?;
    }
    Ok(())
}

/// The routing steps for one node: its loop routes, their labels, their routing slots, and the bend
/// points of their edges.
fn route_node(graph: &mut LGraph, holder: &mut SelfLoopHolder) -> Result<(), LayoutError> {
    routing::determine_loop_routes(graph, holder)?;
    routing::place_labels(graph, holder)?;
    slots::assign_routing_slots(graph, holder)?;
    router::route_self_loops(graph, holder)
}

/// `SelfLoopPostProcessor.process`: attaches the loop edges again, moves their bend points and labels
/// to the node's position.
pub fn postprocess(graph: &mut LGraph) -> Result<(), LayoutError> {
    for node in regular_nodes_with_loops(graph) {
        let Some(holder) = graph.nodes[node].self_loops.take() else {
            continue;
        };
        restore_edges(graph, node, &holder);
        graph.nodes[node].self_loops = Some(holder);
    }
    Ok(())
}

/// `SelfLoopPostProcessor.processNode`.
fn restore_edges(graph: &mut LGraph, node: usize, holder: &SelfLoopHolder) {
    let position = graph.nodes[node].position;
    for hyper in &holder.hyper_loops {
        for &edge in &hyper.sl_edges {
            let ledge = holder.sl_edges[edge].ledge;
            graph.attach_edge(ledge);
            for point in &mut graph.edges[ledge].bend_points {
                point.x += position.x;
                point.y += position.y;
            }
        }
    }
    for hyper in &holder.hyper_loops {
        if let Some(labels) = &hyper.labels {
            labels.apply_placement(graph, position);
        }
    }
}

/// The regular nodes of the layers that have a self loop holder, in layer order.
fn regular_nodes_with_loops(graph: &LGraph) -> Vec<usize> {
    graph
        .layers
        .iter()
        .flatten()
        .copied()
        .filter(|&node| {
            graph.nodes[node].node_type == NodeType::NORMAL
                && graph.nodes[node].self_loops.is_some()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::enums::PortSide;
    use crate::layered::lgraph::{LEdge, LNode, LPort};

    /// A node with `count` ports on the east side, in its port list. Returns their indices.
    fn node_with_ports(graph: &mut LGraph, count: usize) -> Vec<usize> {
        graph.nodes.push(LNode::default());
        (0..count)
            .map(|_| {
                graph.ports.push(LPort::new(0, PortSide::EAST));
                let port = graph.ports.len() - 1;
                graph.nodes[0].ports.push(port);
                port
            })
            .collect()
    }

    /// An edge from `source` to `target`, attached to both ports.
    fn connect(graph: &mut LGraph, source: usize, target: usize) -> usize {
        graph.edges.push(LEdge {
            source,
            target,
            ..LEdge::default()
        });
        let edge = graph.edges.len() - 1;
        graph.ports[source].outgoing.push(edge);
        graph.ports[target].incoming.push(edge);
        edge
    }

    #[test]
    fn a_single_self_loop_is_one_hyper_loop_of_two_ports() {
        let mut graph = LGraph::default();
        let ports = node_with_ports(&mut graph, 2);
        connect(&mut graph, ports[0], ports[1]);

        let holder = SelfLoopHolder::initialize(&graph, 0).unwrap();

        assert_eq!(holder.hyper_loops.len(), 1);
        assert_eq!(holder.hyper_loops[0].sl_ports.len(), 2);
        assert_eq!(holder.hyper_loops[0].sl_edges, vec![0]);
        assert!(holder.sl_ports.iter().all(|port| port.had_only_self_loops));
    }

    #[test]
    fn edges_sharing_a_port_form_a_hyper_loop_that_is_not_ported() {
        // Two self loops through the middle port: one hyper loop of two edges, which is not ported.
        let mut graph = LGraph::default();
        let ports = node_with_ports(&mut graph, 3);
        connect(&mut graph, ports[0], ports[1]);
        connect(&mut graph, ports[1], ports[2]);

        assert!(matches!(
            SelfLoopHolder::initialize(&graph, 0),
            Err(LayoutError::Unsupported(_))
        ));
    }

    #[test]
    fn hiding_detaches_the_loop_and_removes_its_only_self_loop_ports() {
        let mut graph = LGraph::default();
        let ports = node_with_ports(&mut graph, 2);
        let edge = connect(&mut graph, ports[0], ports[1]);
        let mut holder = SelfLoopHolder::initialize(&graph, 0).unwrap();

        model::hide(&mut graph, &mut holder).unwrap();

        assert!(!graph.is_attached(edge));
        assert!(graph.nodes[0].ports.is_empty());
        assert!(holder.ports_hidden);
        assert!(holder.sl_ports.iter().all(|port| port.hidden));
    }

    #[test]
    fn a_self_loop_is_not_a_self_loop_once_detached() {
        let mut graph = LGraph::default();
        let ports = node_with_ports(&mut graph, 2);
        let edge = connect(&mut graph, ports[0], ports[1]);
        assert!(model::is_self_loop(&graph, edge));

        graph.detach_edge(edge);
        assert!(!model::is_self_loop(&graph, edge));

        graph.attach_edge(edge);
        assert!(model::is_self_loop(&graph, edge));
    }

    #[test]
    fn a_non_regular_node_with_a_loop_is_not_processed_as_one() {
        let mut graph = LGraph::default();
        let ports = node_with_ports(&mut graph, 2);
        connect(&mut graph, ports[0], ports[1]);
        graph.nodes[0].node_type = NodeType::LABEL;

        assert!(!model::needs_self_loop_processing(&graph, 0));
    }
}
