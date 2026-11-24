//! The node placement of the design mode: where the nodes stand along the layers that ELK decided
//! (`docs/decisions/0023-design-mode.md`). It is ferrigura's own, run through the hook of
//! `ferrigura_elk::placement`.
//!
//! A drawing is calm when its nodes stand on few common lines and an edge is either straight or
//! turns clearly. The lengths of the steps of the edges do not say that: a step of ten costs
//! little and looks worst. So the placement works in four steps:
//!
//! 1. **Axes.** An edge is an axis edge when it is in the middle of the edges of its side at both
//!    of its nodes: the edge of a chain, the middle edge of a fan. Where a side has an even number
//!    of edges, the one of the middle two that is more of a main edge is in the middle.
//! 2. **Bundles.** A side with an even number of edges has no port in its middle. Where it has an
//!    axis edge, its bundle moves by half the distance between two ports, so that the axis edge
//!    leaves from the middle of the node.
//! 3. **Places.** The network simplex method places the nodes for short steps, as ELK's placement
//!    does, with other weights: an axis edge counts many times more than any other edge, an edge
//!    between two nodes more than the end of a long edge, and an edge against the flow less.
//! 4. **Middles.** Where the lengths leave a choice, the method takes one end of it. A group of
//!    nodes on one axis then moves to where the squares of the steps are least: into the middle
//!    of an even fan.
//!
//! Before the steps, every side's bundle moves into the middle of its node: ELK leaves the ports
//! of a group at the start of its side, and a leaf's are already in the middle.

use std::collections::BTreeSet;

use ferrigura_elk::placement::{Kind, Layers, Placement, Side};

/// How many times more an axis edge counts than it would otherwise.
const AXIS: f64 = 64.0;

/// The share of the squared step of an edge in the cost of a place, beside its length.
const SQUARE_SHARE: f64 = 1e-5;

/// The passes of the placement at most: after the first, each one puts a port in the middle that
/// the pass before left beside it.
const PASSES: usize = 3;

/// The rounds in which the groups move to their middles, at most.
const ROUNDS: usize = 8;

/// The room a bundle keeps to the ends of its side when it moves, so that no edge meets the node
/// where its corner turns.
const END_ROOM: f64 = 12.0;

/// The weight from which an edge holds its two nodes on one line like an axis edge: the parts of
/// a long edge.
const RIGID: f64 = 16.0;

/// The placement of the design mode.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Axes;

impl Placement for Axes {
    fn place(&self, layers: &mut Layers) {
        // The ports a later pass puts in the middle of an even side, and the sides whose bundle
        // moved.
        center_bundles(layers);
        let mut chosen = BTreeSet::new();
        let mut moved = BTreeSet::new();
        for _ in 0..PASSES {
            let axis = axis_edges(layers, &chosen, &moved);
            center_axis_ports(layers, &axis, &mut moved);
            let weights: Vec<f64> = (0..layers.edges.len())
                .map(|edge| weight(layers, edge) * if axis[edge] { AXIS } else { 1.0 })
                .collect();
            for (node, place) in layers.solve(&weights).into_iter().enumerate() {
                layers.nodes[node].place = place;
            }
            move_to_middles(layers, &weights, &axis);
            if !choose_straight_middles(layers, &mut chosen) {
                break;
            }
        }
    }
}

/// The nodes at the two ends of an edge.
fn ends(layers: &Layers, edge: usize) -> (usize, usize) {
    (
        layers.ports[layers.edges[edge].source].node,
        layers.ports[layers.edges[edge].target].node,
    )
}

/// What an edge counts for: the parts of a long edge stay on one line first, then an edge between
/// two nodes counts for more than the end of a long edge, and an edge that runs against the flow
/// for less. So the nodes of the main flow line up, and what goes around them bends.
fn weight(layers: &Layers, edge: usize) -> f64 {
    let (from, to) = ends(layers, edge);
    let plain = |node: usize| layers.nodes[node].kind == Kind::Node;
    let weight = match (plain(from), plain(to)) {
        (true, true) => 4.0,
        (false, false) => RIGID,
        _ => 2.0,
    };
    if layers.edges[edge].reversed && (plain(from) || plain(to)) {
        weight / 2.0
    } else {
        weight
    }
}

/// The ports of a node's side, in their order along the side.
fn side_ports(layers: &Layers, node: usize, side: Side) -> Vec<usize> {
    let mut ports: Vec<usize> = layers.nodes[node]
        .ports
        .iter()
        .copied()
        .filter(|&port| layers.ports[port].side == side)
        .collect();
    ports.sort_by(|&a, &b| layers.ports[a].at.total_cmp(&layers.ports[b].at));
    ports
}

/// Whether the bundle of a side can move by `shift` along the side: by whole units, and with
/// [`END_ROOM`] left at both ends.
fn fits(layers: &Layers, ports: &[usize], shift: f64) -> bool {
    let size = layers.nodes[layers.ports[ports[0]].node].size;
    shift.fract() == 0.0
        && layers.ports[ports[0]].at + shift >= END_ROOM
        && layers.ports[ports[ports.len() - 1]].at + shift <= size - END_ROOM
}

/// The middle of the bundle of a side, along the side.
fn bundle_middle(layers: &Layers, ports: &[usize]) -> f64 {
    (layers.ports[ports[0]].at + layers.ports[ports[ports.len() - 1]].at) / 2.0
}

/// The node the edge of a port leads to, through the bends of a long edge and its label, and how
/// much of a main edge that edge is: an edge with the flow before one against it, and of two of
/// the same direction the one that passes fewer layers.
fn lead(layers: &Layers, port: usize) -> (Option<usize>, i32) {
    let Some(&first) = layers.ports[port].edges.first() else {
        return (None, 0);
    };
    let (mut edge, mut from, mut passed) = (first, port, 0);
    let other = loop {
        let (source, target) = (layers.edges[edge].source, layers.edges[edge].target);
        let arrival = if source == from { target } else { source };
        let node = layers.ports[arrival].node;
        let passes = matches!(layers.nodes[node].kind, Kind::Bend | Kind::Label);
        // The way on: the other port of the bend.
        let onward = layers.nodes[node]
            .ports
            .iter()
            .copied()
            .find(|&next| next != arrival && !layers.ports[next].edges.is_empty());
        match onward.filter(|_| passes && passed < layers.nodes.len()) {
            Some(next) => {
                edge = layers.ports[next].edges[0];
                from = next;
                passed += 1;
            }
            None => break node,
        }
    };
    let direction = if layers.edges[first].reversed {
        0
    } else {
        1_000_000
    };
    (Some(other), direction - passed as i32)
}

/// The ports of a side whose edges may be axis edges: the port the side's bundle is put around.
///
/// Of an odd number of ports that is the middle one, and of an even number the one of the middle
/// two whose edge is more of a main edge: one that runs with the flow and passes fewer layers. A
/// port beside the middle of an odd number comes before the middle one where its edge is more of
/// a main edge and the bundle can move by one port. Two middle ports whose edges lead to the same
/// node alike are both such ports. A port of `chosen` comes before its equal. With `moved`, the
/// side's bundle was moved in an earlier pass and stays.
fn middle_ports(
    layers: &Layers,
    ports: &[usize],
    chosen: &BTreeSet<usize>,
    moved: bool,
) -> Vec<usize> {
    if ports.is_empty() {
        return Vec::new();
    }
    let lead = |port: usize| lead(layers, port);
    let plain = layers.nodes[layers.ports[ports[0]].node].kind == Kind::Node;
    let middle = bundle_middle(layers, ports);
    let reachable =
        |port: usize| moved || !plain || fits(layers, ports, middle - layers.ports[port].at);
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

/// Whether each edge is in the middle of its side at both of its nodes.
fn axis_edges(
    layers: &Layers,
    chosen: &BTreeSet<usize>,
    moved: &BTreeSet<(usize, Side)>,
) -> Vec<bool> {
    let mut middle = vec![false; layers.ports.len()];
    for node in 0..layers.nodes.len() {
        for side in [Side::Before, Side::After] {
            let ports = side_ports(layers, node, side);
            let moved = moved.contains(&(node, side));
            for port in middle_ports(layers, &ports, chosen, moved) {
                middle[port] = true;
            }
        }
    }
    layers
        .edges
        .iter()
        .map(|edge| middle[edge.source] && middle[edge.target])
        .collect()
}

/// Moves the bundle of every side whose middle is not the middle of its node there: the ports of
/// a group, which ELK keeps at the start of the side. ELK centers the bundle of a leaf on whole
/// units, so a bundle less than one unit off the middle is centered already and stays. A side
/// with a port that an edge crosses into the group stays too: the group's inner layout placed the
/// way on.
fn center_bundles(layers: &mut Layers) {
    for node in 0..layers.nodes.len() {
        if layers.nodes[node].kind != Kind::Node {
            continue;
        }
        for side in [Side::Before, Side::After] {
            let ports = side_ports(layers, node, side);
            if ports.is_empty() || ports.iter().any(|&port| layers.ports[port].inside) {
                continue;
            }
            let off = layers.nodes[node].size / 2.0 - bundle_middle(layers, &ports);
            let shift = off.round();
            if off.abs() < 1.0 || !fits(layers, &ports, shift) {
                continue;
            }
            for port in ports {
                layers.ports[port].at += shift;
            }
        }
    }
}

/// Moves the bundle of a side so that the port of its axis edge is in the middle of the side: by
/// half the distance between two ports where their number is even, and by a whole one where the
/// axis edge is beside the middle of an odd number.
fn center_axis_ports(layers: &mut Layers, axis: &[bool], moved: &mut BTreeSet<(usize, Side)>) {
    for node in 0..layers.nodes.len() {
        if layers.nodes[node].kind != Kind::Node {
            continue;
        }
        for side in [Side::Before, Side::After] {
            let ports = side_ports(layers, node, side);
            if ports.is_empty() || moved.contains(&(node, side)) {
                continue;
            }
            let on_axis: Vec<usize> = ports
                .iter()
                .copied()
                .filter(|&port| layers.ports[port].edges.iter().any(|&edge| axis[edge]))
                .collect();
            let [port] = on_axis.as_slice() else {
                continue;
            };
            let shift = bundle_middle(layers, &ports) - layers.ports[*port].at;
            if shift == 0.0 || !fits(layers, &ports, shift) {
                continue;
            }
            for port in ports {
                layers.ports[port].at += shift;
            }
            moved.insert((node, side));
        }
    }
}

/// On an even side whose middle two ports are equals, and of which the placement made one edge
/// straight and not the other, chooses the port of the straight edge as the middle: the next pass
/// puts it on the axis, so that the two nodes stand on one line and not half a step apart.
/// Returns whether it chose a port.
fn choose_straight_middles(layers: &Layers, chosen: &mut BTreeSet<usize>) -> bool {
    let mut pairs = Vec::new();
    for node in 0..layers.nodes.len() {
        for side in [Side::Before, Side::After] {
            let ports = side_ports(layers, node, side);
            if ports.is_empty() || ports.len() % 2 == 1 {
                continue;
            }
            if !middle_ports(layers, &ports, chosen, false).is_empty() {
                continue;
            }
            pairs.push((ports[ports.len() / 2 - 1], ports[ports.len() / 2]));
        }
    }
    let mut any = false;
    for &(a, b) in &pairs {
        let straight = |port: usize| {
            layers.ports[port]
                .edges
                .first()
                .is_some_and(|&edge| layers.step(edge).abs() < 1e-9)
        };
        let port = match (straight(a), straight(b)) {
            (true, false) => a,
            (false, true) => b,
            _ => continue,
        };
        chosen.insert(port);
        any = true;
        // The other end of the edge, where it is one of two equals as well.
        if let Some(&edge) = layers.ports[port].edges.first() {
            let (source, target) = (layers.edges[edge].source, layers.edges[edge].target);
            let other = if source == port { target } else { source };
            if pairs.iter().any(|&(a, b)| a == other || b == other) {
                chosen.insert(other);
            }
        }
    }
    any
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

/// A term of a cost in one variable: `abs * |z - at| + square * (z - at)^2`.
#[derive(Debug, Clone, Copy)]
struct Term {
    at: f64,
    abs: f64,
    square: f64,
}

/// Moves each group of nodes that stand on one axis to where its cost is least, with the squares
/// of the steps beside their lengths: where the lengths leave a stretch open, into its middle.
fn move_to_middles(layers: &mut Layers, weights: &[f64], axis: &[bool]) {
    let count = layers.nodes.len();
    // The groups: the nodes joined by axis edges and by the parts of long edges that are straight.
    let mut group: Vec<usize> = (0..count).collect();
    for edge in 0..layers.edges.len() {
        let rigid = axis[edge] || weights[edge] >= RIGID;
        if rigid && layers.step(edge).abs() < 1e-9 {
            let (from, to) = ends(layers, edge);
            let (a, b) = (find(&mut group, from), find(&mut group, to));
            group[a] = b;
        }
    }
    let mut members: Vec<Vec<usize>> = vec![Vec::new(); count];
    for node in layers.layers.iter().flatten().copied() {
        let root = find(&mut group, node);
        members[root].push(node);
    }
    let mut before = vec![None; count];
    let mut after = vec![None; count];
    for layer in &layers.layers {
        for pair in layer.windows(2) {
            before[pair[1]] = Some(pair[0]);
            after[pair[0]] = Some(pair[1]);
        }
    }
    let mut touching: Vec<Vec<usize>> = vec![Vec::new(); count];
    for edge in 0..layers.edges.len() {
        let (from, to) = ends(layers, edge);
        touching[from].push(edge);
        touching[to].push(edge);
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
                let place = layers.nodes[node].place;
                if let Some(other) = before[node].filter(|&other| find(&mut group, other) != root) {
                    low = low.max(layers.nodes[other].place + layers.nodes[node].gap - place);
                }
                if let Some(other) = after[node].filter(|&other| find(&mut group, other) != root) {
                    high = high.min(layers.nodes[other].place - layers.nodes[other].gap - place);
                }
                for &edge in &touching[node] {
                    let (from, to) = ends(layers, edge);
                    let other = if from == node { to } else { from };
                    if find(&mut group, other) == root {
                        continue;
                    }
                    // The shift of the group at which this edge is straight.
                    let straight = if from == node {
                        -layers.step(edge)
                    } else {
                        layers.step(edge)
                    };
                    terms.push(Term {
                        at: straight,
                        abs: weights[edge],
                        square: SQUARE_SHARE * weights[edge],
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
                    layers.nodes[node].place += shift;
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
