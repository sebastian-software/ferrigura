//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/NorthSouthPortPreprocessor.java
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/NorthSouthPortPostprocessor.java
//!
//! The northern and southern ports of side-fixed nodes, "Odd port side processing".
//!
//! Before crossing minimization, each port on the north or south side of a normal node with fixed
//! port sides gets a dummy node in the node's layer, above the node for a northern port and below it
//! for a southern one, and the port's edges move to the dummy (`preprocess`). The dummies and the
//! node form a layout unit; each northern dummy must stay above the node and each southern one below
//! it (in-layer successor constraints), and the node's barycenter counts its dummies'. This is the
//! Java's "new approach" (`USE_NEW_APPROACH`): one dummy per port, none shared.
//!
//! After edge routing, the edges go back to their ports with a bend point where the dummy was, and
//! the dummies leave the layers (`postprocess`).
//!
//! Not ported, and reported as `Unsupported`: self-loops at northern or southern ports, which the
//! Java makes dummies of its own for (`createSameSideSelfLoopDummyNode`,
//! `createNorthSouthSelfLoopDummyNodes`, `processSelfLoop`); the self-loop preprocessor hides every
//! self-loop before this runs, so they do not reach it. Ports that may switch sides
//! (`ALLOW_NON_FLOW_PORTS_TO_SWITCH_SIDES`) are refused by the importer, so every dummy gets its
//! successor constraint. Spline routing (`processSplineInputPort`) is refused with the router.
//! The dummies' `ORIGIN` and `CROSSING_HINT` are not kept: nothing reads them once self-loop dummies
//! are out.

use crate::core::data::OptionValue;
use crate::core::data::enums::{EdgeRouting, NodeType, PortConstraints, PortSide};
use crate::core::math::KVector;
use crate::java::sort;
use crate::layered::LayoutError;
use crate::layered::lgraph::{LGraph, LNode, LPort};
use crate::layered::options;
use crate::layered::ports::{is_order_fixed, is_side_fixed};

/// `NorthSouthPortPreprocessor.process`.
pub fn preprocess(graph: &mut LGraph) -> Result<(), LayoutError> {
    for layer in 0..graph.layers.len() {
        // The index of the current node while its northern ports are processed, and of the most
        // recently inserted dummy while its southern ports are.
        let mut pointer: usize = 0;
        let node_array = graph.layers[layer].clone();
        for (visited, node) in node_array.into_iter().enumerate() {
            // `pointer++`, from -1.
            if visited > 0 {
                pointer += 1;
            }
            let constraints = graph.nodes[node]
                .properties
                .enum_with_default::<PortConstraints>(options::PORT_CONSTRAINTS);
            if graph.nodes[node].node_type != NodeType::NORMAL || !is_side_fixed(constraints) {
                continue;
            }
            if !is_order_fixed(constraints) {
                sort_port_list(graph, node);
            }
            graph.nodes[node].in_layer_layout_unit = Some(node);
            let mut barycenter_associates = Vec::new();

            // The northern ports, left to right.
            let north_ports = side_ports(graph, node, PortSide::NORTH);
            let mut north_dummies = Vec::new();
            create_dummy_nodes(
                graph,
                node,
                &north_ports,
                &mut north_dummies,
                &mut barycenter_associates,
            )?;
            // Every northern dummy goes in at the node's index, so each one lands above the one
            // before it, and the node moves down by one.
            let insert_point = pointer;
            for &dummy in &north_dummies {
                set_layer(graph, dummy, layer, insert_point);
                pointer += 1;
                graph.nodes[dummy].in_layer_layout_unit = Some(node);
                graph.nodes[dummy].in_layer_successor_constraints.push(node);
            }

            // The southern ports, which the port list holds right to left.
            let mut south_ports = side_ports(graph, node, PortSide::SOUTH);
            south_ports.reverse();
            let mut south_dummies = Vec::new();
            create_dummy_nodes(
                graph,
                node,
                &south_ports,
                &mut south_dummies,
                &mut barycenter_associates,
            )?;
            for &dummy in &south_dummies {
                pointer += 1;
                set_layer(graph, dummy, layer, pointer);
                graph.nodes[dummy].in_layer_layout_unit = Some(node);
                graph.nodes[node].in_layer_successor_constraints.push(dummy);
            }

            if !barycenter_associates.is_empty() {
                graph.nodes[node].barycenter_associates = Some(barycenter_associates);
            }
        }
    }
    Ok(())
}

/// `LNode.setLayer(index, layer)`: the node joins the layer at `index`.
fn set_layer(graph: &mut LGraph, node: usize, layer: usize, index: usize) {
    graph.layers[layer].insert(index, node);
    graph.nodes[node].layer = Some(layer);
}

/// `LNode.getPorts(side)`: the node's ports on `side`, in list order.
fn side_ports(graph: &LGraph, node: usize, side: PortSide) -> Vec<usize> {
    graph.nodes[node]
        .ports
        .iter()
        .copied()
        .filter(|&port| graph.ports[port].side == side)
        .collect()
}

/// `sortPortList`: the northern and southern ports with only incoming edges come first, then those
/// with both, then those with only outgoing edges; unconnected ones count as incoming. The south
/// side runs the other way, as the list holds it clockwise. The eastern and western ports keep
/// their order.
fn sort_port_list(graph: &mut LGraph, node: usize) {
    let count = graph.nodes[node].ports.len() as i64;
    // The next ids for each kind of port; their ranges do not overlap.
    let mut in_ports = 0_i64;
    let mut in_out_ports = count;
    let mut out_ports = 2 * count;
    // `port.id`, kept here: the id of each port of the node, by its index in `LGraph::ports`.
    let mut ids: Vec<(usize, i64)> = Vec::with_capacity(graph.nodes[node].ports.len());
    for &port in &graph.nodes[node].ports {
        let data = &graph.ports[port];
        let id = match data.side {
            PortSide::EAST | PortSide::WEST => -1,
            PortSide::NORTH | PortSide::SOUTH => {
                let incoming = data.incoming.len();
                let outgoing = data.outgoing.len();
                let next = if incoming > 0 && outgoing > 0 {
                    &mut in_out_ports
                } else if outgoing > 0 && incoming == 0 {
                    &mut out_ports
                } else {
                    // Incoming only, or unconnected: the unconnected ports go among the inputs.
                    &mut in_ports
                };
                *next += 1;
                *next - 1
            }
            // The Java switch has no case for it, so the port keeps the id it had; the importer
            // gives every port a side, and the port side processor gives the rest theirs.
            PortSide::UNDEFINED => 0,
        };
        ids.push((port, id));
    }
    let id_of = |port: usize| {
        ids.iter()
            .find(|&&(candidate, _)| candidate == port)
            .map_or(0, |&(_, id)| id)
    };
    let mut ports = graph.nodes[node].ports.clone();
    {
        let view: &LGraph = graph;
        sort::sort_by(&mut ports, |&first, &second| {
            let (side1, side2) = (view.ports[first].side, view.ports[second].side);
            if side1 != side2 {
                return (side1 as usize).cmp(&(side2 as usize));
            }
            let (id1, id2) = (id_of(first), id_of(second));
            if id1 == id2 {
                std::cmp::Ordering::Equal
            } else if side1 == PortSide::NORTH {
                id1.cmp(&id2)
            } else {
                id2.cmp(&id1)
            }
        });
    }
    graph.nodes[node].ports = ports;
}

/// `createDummyNodes`: a dummy for each port of `ports` with edges, the ports with only incoming
/// edges first, then those with only outgoing ones, then those with both. `ports` runs left to right.
fn create_dummy_nodes(
    graph: &mut LGraph,
    node: usize,
    ports: &[usize],
    dummy_nodes: &mut Vec<usize>,
    barycenter_associates: &mut Vec<usize>,
) -> Result<(), LayoutError> {
    for &port in ports {
        // `PORT_DUMMY` of the compound graph preprocessor would be overwritten here, and read as a
        // north/south dummy where it is not; such ports are refused before they get here.
        if graph.ports[port].port_dummy.is_some() {
            return Err(LayoutError::Unsupported(format!(
                "a northern or southern hierarchical port on {} (NorthSouthPortPreprocessor with \
                 the PORT_DUMMY of CompoundGraphPreprocessor)",
                graph.node_name(node)
            )));
        }
        for &edge in &graph.ports[port].outgoing {
            let target = graph.edges[edge].target;
            if graph.ports[target].node == node {
                return Err(LayoutError::Unsupported(format!(
                    "a self-loop at a northern or southern port of {} \
                     (NorthSouthPortPreprocessor.createSameSideSelfLoopDummyNode and \
                     createNorthSouthSelfLoopDummyNodes)",
                    graph.node_name(node)
                )));
            }
        }
    }
    let mut in_ports = Vec::with_capacity(ports.len());
    let mut out_ports = Vec::with_capacity(ports.len());
    let mut in_out_ports = Vec::with_capacity(ports.len());
    for &port in ports {
        let incoming = !graph.ports[port].incoming.is_empty();
        let outgoing = !graph.ports[port].outgoing.is_empty();
        if incoming && outgoing {
            in_out_ports.push(port);
        } else if incoming {
            in_ports.push(port);
        } else if outgoing {
            out_ports.push(port);
        }
    }
    for port in in_ports {
        barycenter_associates.push(create_dummy_node(graph, Some(port), None, dummy_nodes));
    }
    for port in out_ports {
        barycenter_associates.push(create_dummy_node(graph, None, Some(port), dummy_nodes));
    }
    for port in in_out_ports {
        barycenter_associates.push(create_dummy_node(
            graph,
            Some(port),
            Some(port),
            dummy_nodes,
        ));
    }
    Ok(())
}

/// `createDummyNode`: a dummy with a western port for the edges entering `in_port` and an eastern
/// one for the edges leaving `out_port`. Both ports may be the same; each port's `PORT_DUMMY` is
/// the dummy, and each dummy port's `ORIGIN` the port it holds the edges of.
fn create_dummy_node(
    graph: &mut LGraph,
    in_port: Option<usize>,
    out_port: Option<usize>,
    dummy_nodes: &mut Vec<usize>,
) -> usize {
    let mut node = LNode {
        node_type: NodeType::NORTH_SOUTH_PORT,
        ..LNode::default()
    };
    node.properties.set(
        options::PORT_CONSTRAINTS,
        Some(OptionValue::Enum {
            class: "PortConstraints",
            name: "FIXED_POS".to_string(),
        }),
    );
    graph.nodes.push(node);
    let dummy = graph.nodes.len() - 1;

    if let Some(in_port) = in_port {
        let input = add_port(graph, dummy, PortSide::WEST, in_port);
        for edge in graph.ports[in_port].incoming.clone() {
            graph.set_edge_target(edge, input);
        }
        graph.ports[in_port].north_south_dummy = Some(dummy);
    }
    if let Some(out_port) = out_port {
        let output = add_port(graph, dummy, PortSide::EAST, out_port);
        for edge in graph.ports[out_port].outgoing.clone() {
            graph.set_edge_source(edge, output);
        }
        graph.ports[out_port].north_south_dummy = Some(dummy);
    }
    dummy_nodes.push(dummy);
    dummy
}

/// `new LPort()` on `side` of the dummy, with `ORIGIN` set to the port it stands for.
fn add_port(graph: &mut LGraph, dummy: usize, side: PortSide, origin: usize) -> usize {
    let mut port = LPort::new(dummy, side);
    port.origin_port = Some(origin);
    graph.ports.push(port);
    let index = graph.ports.len() - 1;
    graph.nodes[dummy].ports.push(index);
    index
}

/// `NorthSouthPortPostprocessor.process`: the edges of each north/south port dummy go back to the
/// ports the dummy was made for, with a bend point where the dummy was, and the dummy leaves its
/// layer. A dummy whose ports all stand for one port also gets junction points there.
pub fn postprocess(graph: &mut LGraph) -> Result<(), LayoutError> {
    if graph
        .properties
        .enum_with_default::<EdgeRouting>(options::EDGE_ROUTING)
        == EdgeRouting::SPLINES
    {
        return Err(LayoutError::Unsupported(
            "north/south port dummies with spline routing \
             (NorthSouthPortPostprocessor.processSplineInputPort)"
                .to_string(),
        ));
    }
    for layer in 0..graph.layers.len() {
        for node in graph.layers[layer].clone() {
            if graph.nodes[node].node_type != NodeType::NORTH_SOUTH_PORT {
                continue;
            }
            let ports = graph.nodes[node].ports.clone();
            // Whether all the dummy's ports were made for the same port.
            let same_origin_port = ports.len() >= 2
                && ports.windows(2).all(|pair| {
                    graph.ports[pair[0]].origin_port == graph.ports[pair[1]].origin_port
                });
            for port in ports {
                if !graph.ports[port].incoming.is_empty() {
                    process_input_port(graph, port, same_origin_port)?;
                }
                if !graph.ports[port].outgoing.is_empty() {
                    process_output_port(graph, port, same_origin_port)?;
                }
            }
            // `node.setLayer(null)`.
            if let Some(index) = graph.layers[layer].iter().position(|&found| found == node) {
                graph.layers[layer].remove(index);
            }
            graph.nodes[node].layer = None;
        }
    }
    Ok(())
}

/// `processInputPort`: the edges entering the dummy's port enter the port it was made for, after a
/// last bend point at the port's x and the dummy's y.
fn process_input_port(
    graph: &mut LGraph,
    input_port: usize,
    add_junction_points: bool,
) -> Result<(), LayoutError> {
    let origin = origin_of(graph, input_port)?;
    let bend_point = KVector {
        x: absolute_anchor(graph, origin).x,
        y: graph.nodes[graph.ports[input_port].node].position.y,
    };
    for edge in graph.ports[input_port].incoming.clone() {
        graph.set_edge_target(edge, origin);
        graph.edges[edge].bend_points.push(bend_point);
        if add_junction_points {
            graph
                .junction_points
                .entry(edge)
                .or_default()
                .push(bend_point);
        }
    }
    Ok(())
}

/// `processOutputPort`: the edges leaving the dummy's port leave the port it was made for, with a
/// first bend point at the port's x and the dummy's y.
fn process_output_port(
    graph: &mut LGraph,
    output_port: usize,
    add_junction_points: bool,
) -> Result<(), LayoutError> {
    let origin = origin_of(graph, output_port)?;
    let bend_point = KVector {
        x: absolute_anchor(graph, origin).x,
        y: graph.nodes[graph.ports[output_port].node].position.y,
    };
    for edge in graph.ports[output_port].outgoing.clone() {
        graph.set_edge_source(edge, origin);
        graph.edges[edge].bend_points.insert(0, bend_point);
        if add_junction_points {
            graph
                .junction_points
                .entry(edge)
                .or_default()
                .push(bend_point);
        }
    }
    Ok(())
}

/// `InternalProperties.ORIGIN` of a dummy's port: the port it was made for.
fn origin_of(graph: &LGraph, port: usize) -> Result<usize, LayoutError> {
    graph.ports[port].origin_port.ok_or_else(|| {
        LayoutError::Format("a north/south port dummy's port without its origin".to_string())
    })
}

/// `LPort.getAbsoluteAnchor`: the node's position, plus the port's position and anchor.
fn absolute_anchor(graph: &LGraph, port: usize) -> KVector {
    let data = &graph.ports[port];
    let owner = graph.nodes[data.node].position;
    KVector {
        x: owner.x + data.position.x + data.anchor.x,
        y: owner.y + data.position.y + data.anchor.y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layered::lgraph::LEdge;

    /// Adds a port on `side` to `node`.
    fn port(graph: &mut LGraph, node: usize, side: PortSide) -> usize {
        graph.ports.push(LPort::new(node, side));
        let port = graph.ports.len() - 1;
        graph.nodes[node].ports.push(port);
        port
    }

    /// Adds an edge from `source` to `target`.
    fn edge(graph: &mut LGraph, source: usize, target: usize) -> usize {
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

    /// A side-fixed node `n` in the second layer with a northern input and a southern output, and
    /// the nodes `a` and `b` they connect to in the first and the third layer.
    fn graph_with_odd_ports() -> (LGraph, usize, usize) {
        let mut graph = LGraph {
            nodes: vec![LNode::default(); 3],
            layers: vec![vec![0], vec![1], vec![2]],
            ..LGraph::default()
        };
        for (index, node) in graph.nodes.iter_mut().enumerate() {
            node.layer = Some(index);
        }
        graph.nodes[1].properties.set(
            options::PORT_CONSTRAINTS,
            Some(OptionValue::Enum {
                class: "PortConstraints",
                name: "FIXED_SIDE".to_string(),
            }),
        );
        let a = port(&mut graph, 0, PortSide::EAST);
        let north = port(&mut graph, 1, PortSide::NORTH);
        let south = port(&mut graph, 1, PortSide::SOUTH);
        let b = port(&mut graph, 2, PortSide::WEST);
        edge(&mut graph, a, north);
        edge(&mut graph, south, b);
        (graph, north, south)
    }

    #[test]
    fn dummies_go_above_and_below_their_node_with_its_edges() {
        let (mut graph, north, south) = graph_with_odd_ports();
        preprocess(&mut graph).unwrap();
        let layer = &graph.layers[1];
        assert_eq!(layer.len(), 3);
        let (above, below) = (layer[0], layer[2]);
        assert_eq!(layer[1], 1);
        assert_eq!(graph.nodes[above].node_type, NodeType::NORTH_SOUTH_PORT);
        assert_eq!(graph.nodes[above].in_layer_successor_constraints, vec![1]);
        assert_eq!(graph.nodes[1].in_layer_successor_constraints, vec![below]);
        assert_eq!(
            graph.nodes[1].barycenter_associates,
            Some(vec![above, below])
        );
        assert!(graph.ports[north].incoming.is_empty());
        assert!(graph.ports[south].outgoing.is_empty());
        assert_eq!(graph.ports[north].north_south_dummy, Some(above));
        let input = graph.nodes[above].ports[0];
        assert_eq!(graph.ports[input].side, PortSide::WEST);
        assert_eq!(graph.ports[input].origin_port, Some(north));
        assert_eq!(graph.ports[input].incoming, vec![0]);
    }

    #[test]
    fn the_postprocessor_brings_the_edges_back_with_a_bend_point() {
        let (mut graph, north, south) = graph_with_odd_ports();
        preprocess(&mut graph).unwrap();
        let (above, below) = (graph.layers[1][0], graph.layers[1][2]);
        graph.nodes[1].position = KVector { x: 100.0, y: 50.0 };
        graph.ports[north].position = KVector { x: 4.0, y: 0.0 };
        graph.nodes[above].position.y = 30.0;
        graph.nodes[below].position.y = 90.0;
        postprocess(&mut graph).unwrap();
        assert_eq!(graph.layers[1], vec![1]);
        assert_eq!(graph.edges[0].target, north);
        assert_eq!(
            graph.edges[0].bend_points,
            vec![KVector { x: 104.0, y: 30.0 }]
        );
        assert_eq!(graph.edges[1].source, south);
        assert_eq!(
            graph.edges[1].bend_points,
            vec![KVector { x: 100.0, y: 90.0 }]
        );
        assert!(graph.junction_points.is_empty());
    }

    #[test]
    fn the_port_list_puts_inputs_before_outputs_on_the_north_side() {
        let mut graph = LGraph {
            nodes: vec![LNode::default(); 2],
            ..LGraph::default()
        };
        let out = port(&mut graph, 0, PortSide::NORTH);
        let input = port(&mut graph, 0, PortSide::NORTH);
        let other = port(&mut graph, 1, PortSide::EAST);
        edge(&mut graph, out, other);
        edge(&mut graph, other, input);
        sort_port_list(&mut graph, 0);
        assert_eq!(graph.nodes[0].ports, vec![input, out]);
    }
}
