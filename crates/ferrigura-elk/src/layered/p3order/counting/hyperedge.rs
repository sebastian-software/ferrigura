//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/p3order/counting/HyperedgeCrossingsCounter.java
//!
//! Counts the crossings between two layers when some ports have several edges: the edges of one port
//! form a hyperedge, and the hyperedges are counted by the positions of their ends.
//!
//! The Java breaks ties between hyperedges (and between their corners) by `hashCode()`. elkjs is
//! compiled by GWT, whose identity hash codes come from a counter in the order they are first asked
//! for; a hyperedge is added to the set that asks for its hash code when it is created. So a tie goes
//! to the hyperedge created first, which is the order this port keeps. This is an assumption about
//! GWT's `hashCode`, not something the Java source states.

use std::cmp::Ordering;
use std::collections::HashMap;

use crate::core::data::enums::PortSide;
use crate::java::sort;
use crate::layered::p3order::{Layout, Net};

/// A hyperedge: the ports of its edges, and its ends on both layers. `upper` and `lower` are the
/// first and last positions of its ports, and `left` and `right` the two layers.
#[derive(Clone, Default)]
struct Hyperedge {
    ports: Vec<usize>,
    upper_left: usize,
    lower_left: usize,
    upper_right: usize,
    lower_right: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Upper,
    Lower,
}

/// `HyperedgeCorner`: one end of a hyperedge on one side. `he` is the hyperedge's creation index.
#[derive(Clone, Copy)]
struct Corner {
    position: usize,
    opposite: usize,
    he: usize,
    kind: Kind,
}

/// `HyperedgeCorner.compareTo`.
fn compare_corners(first: &Corner, second: &Corner) -> Ordering {
    if first.position != second.position {
        return first.position.cmp(&second.position);
    }
    if first.opposite != second.opposite {
        return first.opposite.cmp(&second.opposite);
    }
    if first.he != second.he {
        return first.he.cmp(&second.he);
    }
    match (first.kind, second.kind) {
        (Kind::Upper, Kind::Lower) => Ordering::Less,
        (Kind::Lower, Kind::Upper) => Ordering::Greater,
        _ => Ordering::Equal,
    }
}

/// `Hyperedge.compareTo`: by the upper ends on the left, then on the right, then by creation.
fn compare_hyperedges(first: (&Hyperedge, usize), second: (&Hyperedge, usize)) -> Ordering {
    let (a, a_index) = first;
    let (b, b_index) = second;
    a.upper_left
        .cmp(&b.upper_left)
        .then(a.upper_right.cmp(&b.upper_right))
        .then(a_index.cmp(&b_index))
}

/// `HyperedgeCrossingsCounter.countCrossings` between `left` and `right`, the nodes of two adjacent
/// layers in the layout's order. The port positions are written to `port_positions`, the array the
/// caller shares. An empty side has no hyperedges, and the Java would fail on it, so 0 is returned.
pub(super) fn count_crossings(
    net: &Net,
    layout: &Layout,
    port_positions: &mut [usize],
    left: &[usize],
    right: &[usize],
) -> i32 {
    let (Some(&left_node), Some(&right_node)) = (left.first(), right.first()) else {
        return 0;
    };

    // Index the ports of the left layer: the ones with edges to the right, north to south.
    let mut source_count = 0_usize;
    for &node in left {
        for &port in &layout.node_ports[node] {
            let port_edges = net
                .outgoing(port)
                .iter()
                .filter(|&&edge| net.layer(node) != net.layer_of_port(net.target(edge)))
                .count();
            if port_edges > 0 {
                port_positions[port] = source_count;
                source_count += 1;
            }
        }
    }

    // Index the ports of the right layer: north input ports first, then the others.
    let mut target_count = 0_usize;
    for &node in right {
        let ports = &layout.node_ports[node];
        let mut north_input_ports = 0_usize;
        for &port in ports {
            if net.side(port) != PortSide::NORTH {
                break;
            }
            if net
                .incoming(port)
                .iter()
                .any(|&edge| net.layer(node) != net.layer_of_port(net.source(edge)))
            {
                north_input_ports += 1;
            }
        }
        let mut other_input_ports = 0_usize;
        for &port in ports.iter().rev() {
            let port_edges = net
                .incoming(port)
                .iter()
                .filter(|&&edge| net.layer(node) != net.layer_of_port(net.source(edge)))
                .count();
            if port_edges > 0 {
                if net.side(port) == PortSide::NORTH {
                    port_positions[port] = target_count;
                    target_count += 1;
                } else {
                    port_positions[port] = target_count + north_input_ports + other_input_ports;
                    other_input_ports += 1;
                }
            }
        }
        target_count += other_input_ports;
    }

    // Gather the hyperedges: an edge joins the hyperedges of its two ports. `hyperedges` is in creation
    // order; a merged one stays in place, marked dead, as the Java set removes it.
    let mut hyperedges: Vec<Hyperedge> = Vec::new();
    let mut alive: Vec<bool> = Vec::new();
    let mut port_to_hyperedge: HashMap<usize, usize> = HashMap::new();
    for &node in left {
        for &source_port in &layout.node_ports[node] {
            for &edge in net.outgoing(source_port) {
                let target_port = net.target(edge);
                if net.layer(node) == net.layer_of_port(target_port) {
                    continue;
                }
                let source_he = port_to_hyperedge.get(&source_port).copied();
                let target_he = port_to_hyperedge.get(&target_port).copied();
                match (source_he, target_he) {
                    (None, None) => {
                        let index = hyperedges.len();
                        hyperedges.push(Hyperedge {
                            ports: vec![source_port, target_port],
                            ..Hyperedge::default()
                        });
                        alive.push(true);
                        port_to_hyperedge.insert(source_port, index);
                        port_to_hyperedge.insert(target_port, index);
                    }
                    (None, Some(target)) => {
                        hyperedges[target].ports.push(source_port);
                        port_to_hyperedge.insert(source_port, target);
                    }
                    (Some(source), None) => {
                        hyperedges[source].ports.push(target_port);
                        port_to_hyperedge.insert(target_port, source);
                    }
                    (Some(source), Some(target)) if source == target => {}
                    (Some(source), Some(target)) => {
                        let moved = std::mem::take(&mut hyperedges[target].ports);
                        for &port in &moved {
                            port_to_hyperedge.insert(port, source);
                        }
                        hyperedges[source].ports.extend(moved);
                        alive[target] = false;
                    }
                }
            }
        }
    }

    // The top and bottom position of each hyperedge on both layers.
    let left_layer = net.layer(left_node);
    let right_layer = net.layer(right_node);
    let mut live: Vec<usize> = (0..hyperedges.len()).filter(|&h| alive[h]).collect();
    for &h in &live {
        let mut he = Hyperedge {
            ports: Vec::new(),
            upper_left: source_count,
            lower_left: 0,
            upper_right: target_count,
            lower_right: 0,
        };
        for &port in &hyperedges[h].ports {
            let position = port_positions[port];
            let layer = net.layer_of_port(port);
            if layer == left_layer {
                he.upper_left = he.upper_left.min(position);
                he.lower_left = he.lower_left.max(position);
            } else if layer == right_layer {
                he.upper_right = he.upper_right.min(position);
                he.lower_right = he.lower_right.max(position);
            }
        }
        he.ports = std::mem::take(&mut hyperedges[h].ports);
        hyperedges[h] = he;
    }

    // Sort by the upper ends, then count the crossings of the right ends with a tree.
    sort::sort_by(&mut live, |&a, &b| {
        compare_hyperedges((&hyperedges[a], a), (&hyperedges[b], b))
    });
    let mut south_sequence: Vec<usize> = live.iter().map(|&h| hyperedges[h].upper_right).collect();
    let mut compress_deltas = vec![0_i32; target_count + 1];
    for &position in &south_sequence {
        compress_deltas[position] = 1;
    }
    let mut delta = 0_i32;
    for value in compress_deltas.iter_mut() {
        if *value == 1 {
            *value = delta;
        } else {
            delta -= 1;
        }
    }
    let mut q = 0_usize;
    for position in south_sequence.iter_mut() {
        let adjusted = (*position as i32 + compress_deltas[*position]) as usize;
        *position = adjusted;
        q = q.max(adjusted + 1);
    }
    let mut first_index = 1_usize;
    while first_index < q {
        first_index *= 2;
    }
    let tree_size = 2 * first_index - 1;
    first_index -= 1;
    let mut tree = vec![0_i32; tree_size];
    let mut crossings = 0_i32;
    for &position in &south_sequence {
        let mut index = position + first_index;
        tree[index] += 1;
        while index > 0 {
            if !index.is_multiple_of(2) {
                crossings += tree[index + 1];
            }
            index = (index - 1) / 2;
            tree[index] += 1;
        }
    }

    // Crossings where the hyperedges' areas overlap, on each side.
    crossings += overlap_crossings(&live, &hyperedges, Side::Left);
    crossings += overlap_crossings(&live, &hyperedges, Side::Right);
    crossings
}

#[derive(Clone, Copy)]
enum Side {
    Left,
    Right,
}

/// The corners of the hyperedges on one side, sorted; each lower end counts the hyperedges still open.
fn overlap_crossings(live: &[usize], hyperedges: &[Hyperedge], side: Side) -> i32 {
    let mut corners = Vec::with_capacity(live.len() * 2);
    for &h in live {
        let he = &hyperedges[h];
        let (upper, lower) = match side {
            Side::Left => (he.upper_left, he.lower_left),
            Side::Right => (he.upper_right, he.lower_right),
        };
        corners.push(Corner {
            position: upper,
            opposite: lower,
            he: h,
            kind: Kind::Upper,
        });
        corners.push(Corner {
            position: lower,
            opposite: upper,
            he: h,
            kind: Kind::Lower,
        });
    }
    sort::sort_by(&mut corners, compare_corners);
    let mut open = 0_i32;
    let mut crossings = 0_i32;
    for corner in &corners {
        match corner.kind {
            Kind::Upper => open += 1,
            Kind::Lower => {
                open -= 1;
                crossings += open;
            }
        }
    }
    crossings
}
