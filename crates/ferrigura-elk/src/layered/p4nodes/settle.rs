//! An experiment, not ported from ELK: a node placement for a calm drawing, run after the network
//! simplex placement on the layers and the node order ELK decided.
//!
//! A drawing is calm when its nodes stand on few common lines and an edge is either straight or
//! turns clearly. The lengths of the steps of the edges do not say that: a step of ten costs
//! little and looks worst. So the placement works in four steps:
//!
//! 1. **Axes.** An edge is an axis edge when it is in the middle of the edges of its side at both
//!    of its nodes: the edge of a chain, the middle edge of a fan. Where a side has an even number
//!    of edges, the one of the middle two that leads to a node, and not to the bend of a long edge,
//!    is in the middle.
//! 2. **Bundles.** A side with an even number of edges has no port in its middle. Where it has an
//!    axis edge, its bundle moves by half the distance between two ports, so that the axis edge
//!    leaves from the middle of the node.
//! 3. **Places.** The network simplex method places the nodes for short steps, as ELK's placement
//!    does, with other weights: an axis edge counts many times more than any other edge, an edge
//!    between two nodes more than the end of a long edge, and an edge against the flow less.
//! 4. **Middles.** Where the lengths leave a choice, the method takes one end of it. A group of
//!    nodes on one axis then moves to where the squares of the steps are least: into the middle
//!    of an even fan.

use std::collections::BTreeSet;

use crate::common::networksimplex::{NEdge, NGraph, NNode, NetworkSimplex};
use crate::core::data::enums::{NodeType, PortSide};
use crate::layered::lgraph::LGraph;
use crate::layered::options;
use crate::layered::p4nodes::bk::access::{is_in_layer_edge, is_self_loop};
use crate::layered::spacings;

/// How many times more an axis edge counts than it would otherwise.
const AXIS: f64 = 64.0;

/// The share of the squared step of an edge in the cost of a place, beside its length.
const SQUARE_SHARE: f64 = 1e-5;

/// The passes of the placement at most: after the first, each one puts a port in the middle that
/// the pass before left beside it.
const PASSES: usize = 3;

/// The rounds in which the groups move to their middles, at most.
const ROUNDS: usize = 8;

/// A term of a cost in one variable: `abs * |z - at| + square * (z - at)^2`.
#[derive(Debug, Clone, Copy)]
struct Term {
    at: f64,
    abs: f64,
    square: f64,
}

/// An edge between two layers, with what the placement needs of it.
#[derive(Debug, Clone, Copy)]
struct Step {
    edge: usize,
    from: usize,
    to: usize,
    weight: f64,
    axis: bool,
}

/// Places the nodes of the graph anew along their layers. A graph with an edge inside a layer or
/// with a north or south port dummy keeps its placement.
pub fn settle(graph: &mut LGraph) {
    if graph.layers.iter().flatten().any(|&node| {
        graph.nodes[node].node_type == NodeType::NORTH_SOUTH_PORT
            || graph.nodes[node].node_type == NodeType::BREAKING_POINT
    }) {
        return;
    }
    let mut handled = Vec::new();
    for edge in 0..graph.edges.len() {
        if is_self_loop(graph, edge) {
            continue;
        }
        let (from, to) = ends(graph, edge);
        if graph.nodes[from].layer.is_none() || graph.nodes[to].layer.is_none() {
            continue;
        }
        if is_in_layer_edge(graph, edge) {
            return;
        }
        handled.push(edge);
    }

    let handled_set: BTreeSet<usize> = handled.iter().copied().collect();
    let gap = gaps(graph);
    // The ports a later pass puts in the middle of an even side, and the sides whose bundle moved.
    let mut chosen = BTreeSet::new();
    let mut moved = BTreeSet::new();
    for _ in 0..PASSES {
        let axis = axis_edges(graph, &handled, &handled_set, &chosen, &moved);
        center_axis_ports(graph, &handled_set, &axis, &mut moved);
        let steps: Vec<Step> = handled
            .iter()
            .map(|&edge| {
                let (from, to) = ends(graph, edge);
                let axis = axis.contains(&edge);
                Step {
                    edge,
                    from,
                    to,
                    weight: weight(graph, edge) * if axis { AXIS } else { 1.0 },
                    axis,
                }
            })
            .collect();
        place(graph, &steps, &gap);
        move_to_middles(graph, &steps, &gap);
        if !choose_straight_middles(graph, &handled_set, &mut chosen) {
            break;
        }
    }
}

/// On an even side whose middle two ports are equals, and of which the placement made one edge
/// straight and not the other, chooses the port of the straight edge as the middle: the next pass
/// puts it on the axis, so that the two nodes stand on one line and not half a step apart.
/// Returns whether it chose a port.
fn choose_straight_middles(
    graph: &LGraph,
    handled: &BTreeSet<usize>,
    chosen: &mut BTreeSet<usize>,
) -> bool {
    let edge_of = |port: usize| {
        graph.ports[port]
            .incoming
            .iter()
            .chain(&graph.ports[port].outgoing)
            .copied()
            .find(|edge| handled.contains(edge))
    };
    let mut pairs = Vec::new();
    for node in graph.layers.iter().flatten().copied() {
        for side in [PortSide::WEST, PortSide::EAST] {
            let ports = side_ports(graph, node, side, handled);
            if ports.is_empty() || ports.len() % 2 == 1 {
                continue;
            }
            if !middle_ports(graph, &ports, handled, chosen, false).is_empty() {
                continue;
            }
            pairs.push((ports[ports.len() / 2 - 1], ports[ports.len() / 2]));
        }
    }
    let mut any = false;
    for &(a, b) in &pairs {
        let straight =
            |port: usize| edge_of(port).is_some_and(|edge| step(graph, edge).abs() < 1e-9);
        let port = match (straight(a), straight(b)) {
            (true, false) => a,
            (false, true) => b,
            _ => continue,
        };
        chosen.insert(port);
        any = true;
        // The other end of the edge, where it is one of two equals as well.
        if let Some(edge) = edge_of(port) {
            let (source, target) = (graph.edges[edge].source, graph.edges[edge].target);
            let other = if source == port { target } else { source };
            if pairs.iter().any(|&(a, b)| a == other || b == other) {
                chosen.insert(other);
            }
        }
    }
    any
}

/// The nodes at the two ends of an edge.
fn ends(graph: &LGraph, edge: usize) -> (usize, usize) {
    (
        graph.ports[graph.edges[edge].source].node,
        graph.ports[graph.edges[edge].target].node,
    )
}

/// Where the edges of a port meet it along the layer, from the top of its node.
fn port_at(graph: &LGraph, port: usize) -> f64 {
    graph.ports[port].position.y + graph.ports[port].anchor.y
}

/// How far the source end of an edge is below its target end along the layer.
fn step(graph: &LGraph, edge: usize) -> f64 {
    let (source, target) = (graph.edges[edge].source, graph.edges[edge].target);
    (graph.nodes[graph.ports[source].node].position.y + port_at(graph, source))
        - (graph.nodes[graph.ports[target].node].position.y + port_at(graph, target))
}

/// What an edge counts for: the parts of a long edge stay on one line first, then an edge between
/// two nodes counts for more than the end of a long edge, and an edge that runs against the flow
/// for less. So the nodes of the main flow line up, and what goes around them bends.
fn weight(graph: &LGraph, edge: usize) -> f64 {
    let (from, to) = ends(graph, edge);
    let plain = |node: usize| graph.nodes[node].node_type == NodeType::NORMAL;
    let weight = match (plain(from), plain(to)) {
        (true, true) => 4.0,
        (false, false) => 16.0,
        _ => 2.0,
    };
    if graph.edges[edge].reversed && (plain(from) || plain(to)) {
        weight / 2.0
    } else {
        weight
    }
}

/// The least distance of each node from the node before it in its layer, by node.
fn gaps(graph: &LGraph) -> Vec<f64> {
    let mut gap = vec![0.0; graph.nodes.len()];
    for members in &graph.layers {
        for pair in members.windows(2) {
            let (before, this) = (&graph.nodes[pair[0]], &graph.nodes[pair[1]]);
            gap[pair[1]] = (before.size.y
                + before.margin.bottom
                + spacings::vertical(graph, before, this)
                + this.margin.top)
                .ceil();
        }
    }
    gap
}

/// The ports of a node's side that have a handled edge, in their order along the side.
fn side_ports(
    graph: &LGraph,
    node: usize,
    side: PortSide,
    handled: &BTreeSet<usize>,
) -> Vec<usize> {
    let mut ports: Vec<usize> = graph.nodes[node]
        .ports
        .iter()
        .copied()
        .filter(|&port| {
            let port = &graph.ports[port];
            port.side == side
                && port
                    .incoming
                    .iter()
                    .chain(&port.outgoing)
                    .any(|edge| handled.contains(edge))
        })
        .collect();
    ports.sort_by(|&a, &b| port_at(graph, a).total_cmp(&port_at(graph, b)));
    ports
}

/// The room a bundle keeps to the ends of its side when it moves, so that no edge meets the node
/// where its corner turns.
const END_ROOM: f64 = 12.0;

/// Whether the bundle of a side can move by `shift` along the side: by whole units, and with
/// [`END_ROOM`] left at both ends.
fn fits(graph: &LGraph, ports: &[usize], shift: f64) -> bool {
    let size = graph.nodes[graph.ports[ports[0]].node].size.y;
    shift.fract() == 0.0
        && port_at(graph, ports[0]) + shift >= END_ROOM
        && port_at(graph, ports[ports.len() - 1]) + shift <= size - END_ROOM
}

/// The middle of the bundle of a side, along the side.
fn bundle_middle(graph: &LGraph, ports: &[usize]) -> f64 {
    (port_at(graph, ports[0]) + port_at(graph, ports[ports.len() - 1])) / 2.0
}

/// The node the edge of a port leads to, through the bends of a long edge and its label, and how
/// much of a main edge that edge is: an edge with the flow before one against it, and of two of
/// the same direction the one that passes fewer layers.
fn lead(graph: &LGraph, port: usize, handled: &BTreeSet<usize>) -> (Option<usize>, i32) {
    let edge_of = |port: usize| {
        graph.ports[port]
            .incoming
            .iter()
            .chain(&graph.ports[port].outgoing)
            .copied()
            .find(|edge| handled.contains(edge))
    };
    let Some(first) = edge_of(port) else {
        return (None, 0);
    };
    let (mut edge, mut from, mut passed) = (first, port, 0);
    let other = loop {
        let (source, target) = (graph.edges[edge].source, graph.edges[edge].target);
        let arrival = if source == from { target } else { source };
        let node = graph.ports[arrival].node;
        let dummy = matches!(
            graph.nodes[node].node_type,
            NodeType::LONG_EDGE | NodeType::LABEL
        );
        // The way on: the other port of the dummy.
        let onward = graph.nodes[node]
            .ports
            .iter()
            .copied()
            .find(|&next| next != arrival && edge_of(next).is_some());
        match onward.filter(|_| dummy && passed < graph.nodes.len()) {
            Some(next) => {
                edge = edge_of(next).expect("a port with an edge");
                from = next;
                passed += 1;
            }
            None => break node,
        }
    };
    let direction = if graph.edges[first].reversed {
        0
    } else {
        1_000_000
    };
    (Some(other), direction - passed as i32)
}

/// The ports of a side whose edges may be axis edges: the port the side's bundle is put around.
///
/// Of an odd number of ports that is the middle one, and of an even number the one of the middle
/// two whose edge is more of a main edge: one that leads to a node, and not to the bend of a long
/// edge, and that runs with the flow. A port beside the middle of an odd number comes before the
/// middle one where its edge is more of a main edge and the bundle can move by one port. Two
/// middle ports whose edges lead to the same node alike are both such ports. A port of `chosen` comes before
/// its equal. With `moved`, the side's bundle was moved in an earlier pass and stays.
fn middle_ports(
    graph: &LGraph,
    ports: &[usize],
    handled: &BTreeSet<usize>,
    chosen: &BTreeSet<usize>,
    moved: bool,
) -> Vec<usize> {
    if ports.is_empty() {
        return Vec::new();
    }
    let lead = |port: usize| lead(graph, port, handled);
    let plain = graph.nodes[graph.ports[ports[0]].node].node_type == NodeType::NORMAL;
    let middle = bundle_middle(graph, ports);
    let reachable =
        |port: usize| moved || !plain || fits(graph, ports, middle - port_at(graph, port));
    if ports.len() % 2 == 1 {
        let at = ports.len() / 2;
        let own = lead(ports[at]).1;
        let better: Vec<usize> = [at.checked_sub(1), Some(at + 1)]
            .into_iter()
            .flatten()
            .filter_map(|index| ports.get(index).copied())
            .filter(|&port| lead(port).1 > own && reachable(port))
            .collect();
        return match better.as_slice() {
            [] => vec![ports[at]],
            [one] => vec![*one],
            [one, other] if lead(*one).1 > lead(*other).1 => vec![*one],
            [one, other] if lead(*other).1 > lead(*one).1 => vec![*other],
            _ => Vec::new(),
        };
    }
    let (a, b) = (ports[ports.len() / 2 - 1], ports[ports.len() / 2]);
    match (chosen.contains(&a), chosen.contains(&b)) {
        (true, false) => return vec![a],
        (false, true) => return vec![b],
        _ => {}
    }
    let (lead_a, lead_b) = (lead(a), lead(b));
    if lead_a.0.is_some() && lead_a == lead_b {
        vec![a, b]
    } else if lead_a.1 > lead_b.1 && reachable(a) {
        vec![a]
    } else if lead_b.1 > lead_a.1 && reachable(b) {
        vec![b]
    } else {
        Vec::new()
    }
}

/// The edges that are in the middle of their side at both of their nodes.
fn axis_edges(
    graph: &LGraph,
    handled: &[usize],
    handled_set: &BTreeSet<usize>,
    chosen: &BTreeSet<usize>,
    moved: &BTreeSet<(usize, bool)>,
) -> BTreeSet<usize> {
    let mut middle = vec![false; graph.ports.len()];
    for node in graph.layers.iter().flatten().copied() {
        for side in [PortSide::WEST, PortSide::EAST] {
            let ports = side_ports(graph, node, side, handled_set);
            let moved = moved.contains(&(node, side == PortSide::WEST));
            for port in middle_ports(graph, &ports, handled_set, chosen, moved) {
                middle[port] = true;
            }
        }
    }
    handled
        .iter()
        .copied()
        .filter(|&edge| middle[graph.edges[edge].source] && middle[graph.edges[edge].target])
        .collect()
}

/// Moves the bundle of a side so that the port of its axis edge is in the middle of the side: by
/// half the distance between two ports where their number is even, and by a whole one where the
/// axis edge is beside the middle of an odd number.
fn center_axis_ports(
    graph: &mut LGraph,
    handled: &BTreeSet<usize>,
    axis: &BTreeSet<usize>,
    moved: &mut BTreeSet<(usize, bool)>,
) {
    for node in graph.layers.iter().flatten().copied().collect::<Vec<_>>() {
        if graph.nodes[node].node_type != NodeType::NORMAL {
            continue;
        }
        for side in [PortSide::WEST, PortSide::EAST] {
            let ports = side_ports(graph, node, side, handled);
            if ports.is_empty() || moved.contains(&(node, side == PortSide::WEST)) {
                continue;
            }
            let on_axis: Vec<usize> = ports
                .iter()
                .copied()
                .filter(|&port| {
                    graph.ports[port]
                        .incoming
                        .iter()
                        .chain(&graph.ports[port].outgoing)
                        .any(|edge| axis.contains(edge))
                })
                .collect();
            let [port] = on_axis.as_slice() else {
                continue;
            };
            let shift = bundle_middle(graph, &ports) - port_at(graph, *port);
            if shift == 0.0 || !fits(graph, &ports, shift) {
                continue;
            }
            for port in ports {
                graph.ports[port].position.y += shift;
            }
            moved.insert((node, side == PortSide::WEST));
        }
    }
}

/// Places the nodes with the network simplex method: a node of its graph for each node, an edge
/// of the least distance between two neighbors of a layer, and for each edge a node with an edge
/// to each of its ends, as ELK's placement builds it.
fn place(graph: &mut LGraph, steps: &[Step], gap: &[f64]) {
    let mut network = NGraph::new();
    let mut rep = vec![usize::MAX; graph.nodes.len()];
    for members in &graph.layers {
        for (index, &node) in members.iter().enumerate() {
            rep[node] = network.add_node(NNode::new());
            if index > 0 {
                let mut apart = NEdge::new(rep[members[index - 1]], rep[node]);
                apart.weight = 0.0;
                apart.delta = gap[node] as i32;
                network.add_edge(apart);
            }
        }
    }
    for step in steps {
        let source = port_at(graph, graph.edges[step.edge].source);
        let target = port_at(graph, graph.edges[step.edge].target);
        let between = network.add_node(NNode::new());
        for (end, delta) in [
            (step.from, (target - source).max(0.0)),
            (step.to, (source - target).max(0.0)),
        ] {
            let mut pull = NEdge::new(between, rep[end]);
            pull.weight = step.weight;
            pull.delta = delta as i32;
            network.add_edge(pull);
        }
    }
    network.make_connected();
    let thoroughness = graph.properties.int_with_default(options::THOROUGHNESS);
    let limit = thoroughness.wrapping_mul(network.nodes().len() as i32);
    NetworkSimplex::for_graph(&mut network)
        .with_iteration_limit(limit)
        .with_balancing(false)
        .execute();
    for node in graph.layers.iter().flatten().copied().collect::<Vec<_>>() {
        graph.nodes[node].position.y = f64::from(network.node(rep[node]).layer);
    }
}

/// The root of the group a node is in.
fn find(group: &mut [usize], node: usize) -> usize {
    let mut root = node;
    while group[root] != root {
        root = group[root];
    }
    let mut at = node;
    while group[at] != root {
        let next = group[at];
        group[at] = root;
        at = next;
    }
    root
}

/// Moves each group of nodes that stand on one axis to where its cost is least, with the squares
/// of the steps beside their lengths: where the lengths leave a stretch open, into its middle.
fn move_to_middles(graph: &mut LGraph, steps: &[Step], gap: &[f64]) {
    let count = graph.nodes.len();
    // The groups: the nodes joined by axis edges and by the parts of long edges that are straight.
    let mut group: Vec<usize> = (0..count).collect();
    for part in steps {
        let rigid = part.axis || part.weight >= 16.0;
        if rigid && step(graph, part.edge).abs() < 1e-9 {
            let (a, b) = (find(&mut group, part.from), find(&mut group, part.to));
            group[a] = b;
        }
    }
    let mut members: Vec<Vec<usize>> = vec![Vec::new(); count];
    for node in graph.layers.iter().flatten().copied() {
        let root = find(&mut group, node);
        members[root].push(node);
    }
    let mut before = vec![None; count];
    let mut after = vec![None; count];
    for layer in &graph.layers {
        for pair in layer.windows(2) {
            before[pair[1]] = Some(pair[0]);
            after[pair[0]] = Some(pair[1]);
        }
    }
    let mut touching: Vec<Vec<usize>> = vec![Vec::new(); count];
    for (index, part) in steps.iter().enumerate() {
        touching[part.from].push(index);
        touching[part.to].push(index);
    }

    for _ in 0..ROUNDS {
        let mut moved = false;
        for (root, nodes) in members.iter().enumerate() {
            if nodes.is_empty() {
                continue;
            }
            let (mut low, mut high) = (f64::NEG_INFINITY, f64::INFINITY);
            let mut terms = Vec::new();
            for &node in nodes {
                let place = graph.nodes[node].position.y;
                if let Some(other) = before[node].filter(|&other| find(&mut group, other) != root) {
                    low = low.max(graph.nodes[other].position.y + gap[node] - place);
                }
                if let Some(other) = after[node].filter(|&other| find(&mut group, other) != root) {
                    high = high.min(graph.nodes[other].position.y - gap[other] - place);
                }
                for &index in &touching[node] {
                    let part = steps[index];
                    let other = if part.from == node {
                        part.to
                    } else {
                        part.from
                    };
                    if find(&mut group, other) == root {
                        continue;
                    }
                    // The shift of the group at which this edge is straight.
                    let straight = if part.from == node {
                        -step(graph, part.edge)
                    } else {
                        step(graph, part.edge)
                    };
                    terms.push(Term {
                        at: straight,
                        abs: part.weight,
                        square: SQUARE_SHARE * part.weight,
                    });
                }
            }
            if terms.is_empty() || low > high {
                continue;
            }
            let shift = minimum(&mut terms).clamp(low, high).round();
            let shift = if shift < low {
                low.ceil()
            } else if shift > high {
                high.floor()
            } else {
                shift
            };
            if shift.abs() >= 1.0 && shift >= low && shift <= high {
                moved = true;
                for &node in nodes {
                    graph.nodes[node].position.y += shift;
                }
            }
        }
        if !moved {
            break;
        }
    }
}

/// Where the sum of the terms is least. Sorts the terms.
fn minimum(terms: &mut [Term]) -> f64 {
    terms.sort_by(|a, b| a.at.total_cmp(&b.at));
    let square: f64 = terms.iter().map(|term| term.square).sum();
    let moment: f64 = terms.iter().map(|term| term.square * term.at).sum();
    let mut right: f64 = terms.iter().map(|term| term.abs).sum();
    let mut left = 0.0;
    // The slope of the cost at `z`, between two terms: `left - right + 2 * (square * z - moment)`.
    let level = |left: f64, right: f64| (moment - (left - right) / 2.0) / square;
    let mut index = 0;
    while index < terms.len() {
        let at = terms[index].at;
        let flat = level(left, right);
        if flat <= at {
            return flat;
        }
        let mut here = 0.0;
        while index < terms.len() && terms[index].at == at {
            here += terms[index].abs;
            index += 1;
        }
        left += here;
        right -= here;
        if level(left, right) <= at {
            return at;
        }
    }
    level(left, right)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn term(at: f64, abs: f64) -> Term {
        Term {
            at,
            abs,
            square: SQUARE_SHARE * abs,
        }
    }

    #[test]
    fn the_least_cost_is_at_the_median_and_in_the_middle_of_an_even_fan() {
        // An odd number of equal pulls: the middle one.
        assert_eq!(
            minimum(&mut [term(0.0, 1.0), term(10.0, 1.0), term(50.0, 1.0)]),
            10.0
        );
        // An even number: the lengths leave the stretch between the middle two open, and the
        // squares choose its middle.
        let middle = minimum(&mut [term(0.0, 1.0), term(40.0, 1.0)]);
        assert!((middle - 20.0).abs() < 1e-9, "{middle}");
        // A heavier pull wins.
        assert_eq!(minimum(&mut [term(0.0, 8.0), term(40.0, 1.0)]), 0.0);
    }
}
