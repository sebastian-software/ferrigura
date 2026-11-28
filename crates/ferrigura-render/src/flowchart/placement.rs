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
//!    of edges, the one of the middle two that is more of a main edge is in the middle. The port
//!    of a group that an edge crosses counts as the middle of its side: the group's own layout,
//!    done before, has put the way on inside, so a chain that crosses the frame stays one axis.
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
//! Where one edge of two equals in the middle of an even side came out straight, the next pass
//! makes it the axis, so that two nodes stand on one line and not half a step apart. An **even
//! fan of equals** is the exception: two or four edges of a side, none more of a main edge than
//! the others, each to a node that meets no other edge on its side towards the fan's node. The
//! lengths would put the node over one of them, the one that has nothing else to hold it. After
//! the passes, the node's group moves into the middle of the fan, or the groups at the other ends
//! all move the other way, where that costs nothing on the edges outside the fans and each outer
//! edge then turns by at least [`TURN`]; a fan that cannot stand symmetric is placed as any other
//! side. A node on a chain whose two edges on the other side lead one on and one to an end is a
//! chain with a branch beside it, not a fan: the edge that leads on is the side's middle, and the
//! chain runs straight.
//!
//! Before the steps, every side's bundle moves into the middle of its node: ELK leaves the ports
//! of a group at the start of its side, and a leaf's are already in the middle. Where ELK spread
//! the ports of a group unevenly over a side whose edges are a fan of equals, they become a bundle
//! [`PORT_GAP`] apart in the middle; after the steps, they move to where their edges meet the next
//! layer where the side has room for all of them to run straight.
//!
//! After the steps, **twins** are set apart: two edges between the same two nodes, through the
//! same layers, that are the only edges of their sides. The axis makes one of them straight and
//! sends the other around the label beside it. Where both carry a label, they are equals: they
//! stand on both sides of the nodes' line instead. Where the sides have room for it, their ports
//! spread as far as the labels stand apart, and both edges run straight; where not, the ports stay
//! a bundle in the middle, and both edges turn out alike. Where one of them has no label, it
//! stays the axis.
//!
//! Two such edges with labels beside other edges of their sides are **one unit** when the axes are
//! chosen: where the axis of a side would go to an edge beside the pair, it goes to the pair, and
//! the bundle moves so that the pair's middle is the middle of the node. The other node then
//! stands below the pair, and the other edges beside it. Twins beside other edges are set apart
//! as above only where neither runs straight, and their ports stay a bundle.

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

/// The distance between two ports of a side, as the layout is given it.
const PORT_GAP: f64 = 20.0;

/// The least step of the edges of a fan that stands symmetric: a shorter one would show as a jog
/// and not as a turn.
const TURN: f64 = 20.0;

/// The weight from which an edge holds its two nodes on one line like an axis edge: the parts of
/// a long edge.
const RIGID: f64 = 16.0;

/// The placement of the design mode.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Axes;

impl Placement for Axes {
    fn place(&self, layers: &mut Layers) {
        let bundled = center_bundles(layers);
        // The ports a later pass puts in the middle of an even side, the sides whose bundle
        // moved, and the even fans that cannot stand symmetric and are placed as other sides.
        let mut chosen = BTreeSet::new();
        let mut moved = BTreeSet::new();
        let mut plain = BTreeSet::new();
        let mut fans = Vec::new();
        let mut last = (Vec::new(), Vec::new());
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
            let chains;
            (fans, chains) = even_fans(layers, &axis);
            fans.retain(|fan| !plain.contains(&fan.ports));
            plain.extend(center_fans(&mut layers.clone(), &weights, &axis, &fans));
            fans.retain(|fan| !plain.contains(&fan.ports));
            let mut more = choose_straight_middles(layers, &mut chosen, &fans, &chains);
            for port in chains {
                more |= chosen.insert(port);
            }
            last = (weights, axis);
            if !more {
                break;
            }
        }
        center_fans(layers, &last.0, &last.1, &fans);
        straighten_bundles(layers, &bundled);
        spread_twins(layers);
    }
}

/// The way of an edge from a port of a node through the bends and labels of the layers after it:
/// the dummies it passes and the port at which it arrives at the next node. `None` where a dummy
/// does not lead on.
fn way(layers: &Layers, port: usize) -> Option<(Vec<usize>, usize)> {
    let mut dummies = Vec::new();
    let mut from = port;
    loop {
        let [edge] = layers.ports[from].edges.as_slice() else {
            return None;
        };
        let arrival = layers.edges[*edge].target;
        let node = layers.ports[arrival].node;
        if layers.nodes[node].kind == Kind::Node {
            return Some((dummies, arrival));
        }
        if !matches!(layers.nodes[node].kind, Kind::Bend | Kind::Label) {
            return None;
        }
        dummies.push(node);
        from = layers.nodes[node]
            .ports
            .iter()
            .copied()
            .find(|&next| layers.ports[next].side == Side::After)?;
    }
}

/// Sets twins apart: two edges that leave a node from two ports next to each other and arrive at
/// one other node at two ports next to each other, through dummies that stand next to each other
/// in each layer, where both nodes or both pairs of ports have their middles on one line. Twins
/// beside other edges of their sides are set apart only where neither runs straight and none of
/// their ports is one an edge crosses into a group, and their ports stay a bundle. Their
/// dummies move to both sides of that line, as close as their layer allows, where it has room for
/// that. Where both sides have room for it with [`END_ROOM`] left at their ends, the ports spread
/// around the line as far as the dummies are apart, and both edges run straight; where one has
/// not, the ports stay a bundle around the line, and both edges turn out alike: ports that spread
/// without making their edges straight would only move the turns to the corners of the nodes.
/// Only twins that both carry a label are set apart.
fn spread_twins(layers: &mut Layers) {
    let mut position = vec![(0, 0); layers.nodes.len()];
    for (layer, members) in layers.layers.iter().enumerate() {
        for (index, &node) in members.iter().enumerate() {
            position[node] = (layer, index);
        }
    }
    for node in 0..layers.nodes.len() {
        if layers.nodes[node].kind != Kind::Node {
            continue;
        }
        let ports = side_ports(layers, node, Side::After);
        let mut index = 0;
        while index + 1 < ports.len() {
            let (first, second) = (&ports[index], &ports[index + 1]);
            index += 1;
            let (Some((left, arrive_left)), Some((right, arrive_right))) =
                (way(layers, *first), way(layers, *second))
            else {
                continue;
            };
            let other = layers.ports[arrive_left].node;
            let arrivals = side_ports(layers, other, Side::Before);
            let next_to_each_other = arrivals
                .windows(2)
                .any(|pair| pair == [arrive_left, arrive_right]);
            // Whether the twins are the only edges of their sides. Twins beside other edges keep
            // away from the ports of a group that an edge crosses into it.
            let sole = ports.len() == 2 && arrivals.len() == 2;
            let inside = [*first, *second, arrive_left, arrive_right]
                .iter()
                .any(|&port| layers.ports[port].inside);
            if left.is_empty()
                || left.len() != right.len()
                || other != layers.ports[arrive_right].node
                || !next_to_each_other
                || (inside && !sole)
            {
                continue;
            }
            // Twins beside other edges keep one of them where it runs straight.
            let absolute =
                |port: usize| layers.nodes[layers.ports[port].node].place + layers.ports[port].at;
            let straight_way = |from: usize, dummies: &[usize], to: usize| {
                let line = absolute(from);
                (absolute(to) - line).abs() < 1e-6
                    && dummies.iter().all(|&dummy| {
                        layers.nodes[dummy]
                            .ports
                            .iter()
                            .all(|&port| (absolute(port) - line).abs() < 1e-6)
                    })
            };
            if !sole
                && (straight_way(*first, &left, arrive_left)
                    || straight_way(*second, &right, arrive_right))
            {
                continue;
            }
            // The line of the twins: through the middles of both nodes where they stand on one and
            // the twins are alone, else through the middles of both pairs of ports.
            let middle = |node: usize| layers.nodes[node].place + layers.nodes[node].size / 2.0;
            let bundles = (
                (absolute(*first) + absolute(*second)) / 2.0,
                (absolute(arrive_left) + absolute(arrive_right)) / 2.0,
            );
            let line = if sole && (middle(node) - middle(other)).abs() <= 0.5 {
                middle(node)
            } else if (bundles.0 - bundles.1).abs() <= 0.5 {
                bundles.0
            } else {
                continue;
            };
            // The pairs of dummies stand next to each other, the left one first.
            let next_to = left.iter().zip(&right).all(|(&a, &b)| {
                let ((layer_a, index_a), (layer_b, index_b)) = (position[a], position[b]);
                layer_a == layer_b && index_b == index_a + 1
            });
            if !next_to {
                continue;
            }
            // The least distance of the middles of two dummies of a layer, the widest over the layers.
            let apart = left
                .iter()
                .zip(&right)
                .map(|(&a, &b)| {
                    layers.nodes[b].gap + layers.nodes[b].size / 2.0 - layers.nodes[a].size / 2.0
                })
                .fold(0.0_f64, f64::max)
                .ceil();
            let half = apart / 2.0;
            let places: Vec<(usize, f64)> = left
                .iter()
                .map(|&a| (a, line - half - layers.nodes[a].size / 2.0))
                .chain(
                    right
                        .iter()
                        .map(|&b| (b, line + half - layers.nodes[b].size / 2.0)),
                )
                .collect();
            let room = places.iter().all(|&(dummy, place)| {
                let (layer, index) = position[dummy];
                let members = &layers.layers[layer];
                let before = index
                    .checked_sub(1)
                    .map(|at| members[at])
                    .filter(|other| !left.contains(other));
                let after = members
                    .get(index + 1)
                    .copied()
                    .filter(|other| !right.contains(other));
                before.is_none_or(|other| {
                    place - layers.nodes[other].place >= layers.nodes[dummy].gap
                }) && after.is_none_or(|other| {
                    layers.nodes[other].place - place >= layers.nodes[other].gap
                })
            });
            // An edge without a label stays the axis: twins are equals only when both carry a label.
            let labeled = |dummies: &[usize]| {
                dummies
                    .iter()
                    .any(|&dummy| layers.nodes[dummy].kind == Kind::Label)
            };
            if !room || !labeled(&left) || !labeled(&right) {
                continue;
            }
            for &(dummy, place) in &places {
                layers.nodes[dummy].place = place;
            }
            // The middle of each end's side, from its node's place, and the room from there to
            // [`END_ROOM`] before the nearer end of the side.
            let ends = [(*first, *second), (arrive_left, arrive_right)].map(|(a, b)| {
                let owner = &layers.nodes[layers.ports[a].node];
                let middle = line - owner.place;
                let reach = (middle - END_ROOM).min(owner.size - END_ROOM - middle);
                (a, b, middle, reach)
            });
            let straight = sole && ends.iter().all(|&(_, _, _, reach)| half <= reach);
            for (a, b, middle, _) in ends {
                let bundle = (layers.ports[b].at - layers.ports[a].at) / 2.0;
                let spread = if straight { half.max(bundle) } else { bundle };
                layers.ports[a].at = middle - spread;
                layers.ports[b].at = middle + spread;
            }
            index += 1;
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
/// side's bundle was moved in an earlier pass and stays. Where these would put the axis on an edge
/// beside the pair of twins in the middle of the side, the pair's two ports are the middle
/// instead.
fn middle_ports(
    layers: &Layers,
    ports: &[usize],
    chosen: &BTreeSet<usize>,
    moved: bool,
) -> Vec<usize> {
    let single = single_middle_ports(layers, ports, chosen, moved);
    match middle_twins(layers, ports) {
        Some(pair) if !single.is_empty() && !single.iter().any(|port| pair.contains(port)) => pair,
        _ => single,
    }
}

/// The ports of [`middle_ports`] but for twins.
fn single_middle_ports(
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

/// Where the edges of a port lead, through the bends of a long edge and its label: the node, the
/// layers passed, and whether a label is among them.
fn trail(layers: &Layers, port: usize) -> Option<(usize, usize, bool)> {
    let &first = layers.ports[port].edges.first()?;
    let (mut edge, mut from, mut passed, mut labeled) = (first, port, 0, false);
    loop {
        let (source, target) = (layers.edges[edge].source, layers.edges[edge].target);
        let arrival = if source == from { target } else { source };
        let node = layers.ports[arrival].node;
        match layers.nodes[node].kind {
            Kind::Node => return Some((node, passed, labeled)),
            Kind::Bend | Kind::Label => {}
            Kind::Border => return None,
        }
        labeled |= layers.nodes[node].kind == Kind::Label;
        let next = layers.nodes[node]
            .ports
            .iter()
            .copied()
            .find(|&next| next != arrival && !layers.ports[next].edges.is_empty())?;
        if passed >= layers.nodes.len() {
            return None;
        }
        edge = layers.ports[next].edges[0];
        from = next;
        passed += 1;
    }
}

/// The two ports of a pair of twins that is the middle of its side, where the side has one: two
/// next to each other whose edges both carry a label and lead to the same node through the same
/// layers, whatever their direction. Such a pair counts as one, and its two ports are the middle
/// where the pair is the middle one of the side's units, or one of the middle two beside a
/// single port: it weighs as much as two edges.
fn middle_twins(layers: &Layers, ports: &[usize]) -> Option<Vec<usize>> {
    // The ports of a side that an edge crosses into its group stay where the group's layout put
    // them.
    if ports.iter().any(|&port| layers.ports[port].inside) {
        return None;
    }
    let trails: Vec<_> = ports.iter().map(|&port| trail(layers, port)).collect();
    let twins = |index: usize| match (trails[index], trails.get(index + 1).copied().flatten()) {
        (Some((a, passed_a, true)), Some((b, passed_b, true))) => a == b && passed_a == passed_b,
        _ => false,
    };
    let mut units: Vec<Vec<usize>> = Vec::new();
    let mut index = 0;
    while index < ports.len() {
        if twins(index) {
            units.push(vec![ports[index], ports[index + 1]]);
            index += 2;
        } else {
            units.push(vec![ports[index]]);
            index += 1;
        }
    }
    if units.len() == ports.len() {
        return None;
    }
    let half = units.len() / 2;
    if units.len() % 2 == 1 {
        return Some(units[half].clone()).filter(|unit| unit.len() == 2);
    }
    match (units[half - 1].len(), units[half].len()) {
        (2, 1) => Some(units[half - 1].clone()),
        (1, 2) => Some(units[half].clone()),
        _ => None,
    }
}

/// Whether each edge is in the middle of its side at both of its nodes. A port that an edge
/// crosses into its group is in the middle: it stands for the side of the node inside that the
/// group's layout placed the way on to, so a chain that crosses the frame stays one axis.
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
    for (index, port) in layers.ports.iter().enumerate() {
        if port.inside {
            middle[index] = true;
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
/// way on. Where ELK spread the ports of a group over its side unevenly, they become a bundle
/// [`PORT_GAP`] apart in the middle of the side, where it has room for that with [`END_ROOM`]
/// left at its ends. Returns those sides.
fn center_bundles(layers: &mut Layers) -> BTreeSet<(usize, Side)> {
    let mut bundled = BTreeSet::new();
    for node in 0..layers.nodes.len() {
        if layers.nodes[node].kind != Kind::Node {
            continue;
        }
        for side in [Side::Before, Side::After] {
            let ports = side_ports(layers, node, side);
            if ports.is_empty() || ports.iter().any(|&port| layers.ports[port].inside) {
                continue;
            }
            let gaps: Vec<f64> = ports
                .windows(2)
                .map(|pair| layers.ports[pair[1]].at - layers.ports[pair[0]].at)
                .collect();
            let uneven =
                gaps.iter().any(|gap| (gap - gaps[0]).abs() > 0.5) && fan_of_equals(layers, &ports);
            let size = layers.nodes[node].size;
            let start = ((size - PORT_GAP * gaps.len() as f64) / 2.0).round();
            if uneven
                && start >= END_ROOM
                && size - start - PORT_GAP * gaps.len() as f64 >= END_ROOM
            {
                for (index, &port) in ports.iter().enumerate() {
                    layers.ports[port].at = start + PORT_GAP * index as f64;
                }
                bundled.insert((node, side));
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
    bundled
}

/// Lets the edges of the sides that [`center_bundles`] made a bundle run straight where the side
/// has room for it: each port moves to where its edge meets the next layer, where all of them
/// then keep [`PORT_GAP`] between them and [`END_ROOM`] to the ends of the side. Where one does
/// not, the bundle stays, and its outer edges turn.
fn straighten_bundles(layers: &mut Layers, bundled: &BTreeSet<(usize, Side)>) {
    for &(node, side) in bundled {
        let ports = side_ports(layers, node, side);
        let place = layers.nodes[node].place;
        let size = layers.nodes[node].size;
        let wanted: Option<Vec<f64>> = ports
            .iter()
            .map(|&port| {
                let [edge] = layers.ports[port].edges.as_slice() else {
                    return None;
                };
                let other = far_end(layers, *edge, port);
                Some(layers.nodes[layers.ports[other].node].place + layers.ports[other].at - place)
            })
            .collect();
        let Some(wanted) = wanted else {
            continue;
        };
        let fits = wanted.first().is_some_and(|&at| at >= END_ROOM)
            && wanted.last().is_some_and(|&at| at <= size - END_ROOM)
            && wanted.windows(2).all(|pair| pair[1] - pair[0] >= PORT_GAP);
        if fits {
            for (&port, at) in ports.iter().zip(wanted) {
                layers.ports[port].at = at;
            }
        }
    }
}

/// Moves the bundle of a side so that the port of its axis edge is in the middle of the side: by
/// half the distance between two ports where their number is even, and by a whole one where the
/// axis edge is beside the middle of an odd number. A side with a port that an edge crosses into
/// its group stays, as the group's layout placed the way on.
fn center_axis_ports(layers: &mut Layers, axis: &[bool], moved: &mut BTreeSet<(usize, Side)>) {
    for node in 0..layers.nodes.len() {
        if layers.nodes[node].kind != Kind::Node {
            continue;
        }
        for side in [Side::Before, Side::After] {
            let ports = side_ports(layers, node, side);
            if ports.is_empty()
                || moved.contains(&(node, side))
                || ports.iter().any(|&port| layers.ports[port].inside)
            {
                continue;
            }
            let on_axis: Vec<usize> = ports
                .iter()
                .copied()
                .filter(|&port| layers.ports[port].edges.iter().any(|&edge| axis[edge]))
                .collect();
            // One port, or the two of the pair of twins in the middle of the side.
            let together = match on_axis.as_slice() {
                [_] => true,
                [a, b] => middle_twins(layers, &ports).is_some_and(|pair| pair == [*a, *b]),
                _ => false,
            };
            if !together {
                continue;
            }
            let shift = bundle_middle(layers, &ports) - bundle_middle(layers, &on_axis);
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
/// puts it on the axis, so that the two nodes stand on one line and not half a step apart. The
/// sides of `fans` stand symmetric instead, and those with a port of `chains` take that port.
/// Returns whether it chose a port.
fn choose_straight_middles(
    layers: &Layers,
    chosen: &mut BTreeSet<usize>,
    fans: &[Fan],
    chains: &[usize],
) -> bool {
    let mut pairs = Vec::new();
    for node in 0..layers.nodes.len() {
        for side in [Side::Before, Side::After] {
            let ports = side_ports(layers, node, side);
            if ports.is_empty()
                || ports.len() % 2 == 1
                || fans.iter().any(|fan| fan.ports == ports)
                || ports.iter().any(|port| chains.contains(port))
            {
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

/// The groups of nodes that move together: the nodes joined by axis edges and by the parts of long
/// edges that are straight. Each node's entry leads to its group's root through [`find`].
fn groups(layers: &Layers, weights: &[f64], axis: &[bool]) -> Vec<usize> {
    let mut group: Vec<usize> = (0..layers.nodes.len()).collect();
    for edge in 0..layers.edges.len() {
        let rigid = axis[edge] || weights[edge] >= RIGID;
        if rigid && layers.step(edge).abs() < 1e-9 {
            let (from, to) = ends(layers, edge);
            let (a, b) = (find(&mut group, from), find(&mut group, to));
            group[a] = b;
        }
    }
    group
}

/// An even fan of equals: the two or four edges of a node's side, none of which is more of a
/// main edge than the others, to as many other nodes that meet no other edge on their side
/// towards it. Its node stands in the middle of the fan.
#[derive(Debug, Clone, PartialEq)]
struct Fan {
    node: usize,
    /// The ports of the side, in their order along it.
    ports: Vec<usize>,
}

/// The other side.
fn opposite(side: Side) -> Side {
    match side {
        Side::Before => Side::After,
        Side::After => Side::Before,
    }
}

/// Whether the edges of the ports of a side are a fan of equals: none more of a main edge than
/// the others, each to a node of its own, one that meets no other edge on its side towards them.
fn fan_of_equals(layers: &Layers, ports: &[usize]) -> bool {
    let leads: Vec<(Option<usize>, i32)> = ports.iter().map(|&port| lead(layers, port)).collect();
    let targets: BTreeSet<usize> = leads.iter().filter_map(|lead| lead.0).collect();
    let equal = leads.iter().all(|lead| lead.1 == leads[0].1);
    let facing = opposite(layers.ports[ports[0]].side);
    let own = targets.iter().all(|&other| {
        let facing = side_ports(layers, other, facing);
        facing.len() == 1 && layers.ports[facing[0]].edges.len() == 1
    });
    targets.len() == ports.len() && equal && own
}

/// The even fans of equals, and the ports of the chains that go on beside a branch. A side of two
/// edges whose node continues a chain, where one edge leads on from its other end and the other
/// ends there, is a chain with a branch beside it and not a fan: the chain runs straight on, so
/// the port of the edge that leads on is the side's middle.
fn even_fans(layers: &Layers, axis: &[bool]) -> (Vec<Fan>, Vec<usize>) {
    let mut fans = Vec::new();
    let mut chains = Vec::new();
    for node in 0..layers.nodes.len() {
        if layers.nodes[node].kind != Kind::Node {
            continue;
        }
        for side in [Side::Before, Side::After] {
            let ports = side_ports(layers, node, side);
            if ports.len() != 2 && ports.len() != 4 {
                continue;
            }
            if !fan_of_equals(layers, &ports) {
                continue;
            }
            let leads: Vec<(Option<usize>, i32)> =
                ports.iter().map(|&port| lead(layers, port)).collect();
            let back = opposite(side);
            if ports.len() == 2 {
                let near = side_ports(layers, node, back);
                let chain = near.len() % 2 == 1
                    && layers.ports[near[near.len() / 2]]
                        .edges
                        .iter()
                        .any(|&edge| axis[edge]);
                let onward: Vec<bool> = leads
                    .iter()
                    .filter_map(|lead| lead.0)
                    .map(|other| !side_ports(layers, other, side).is_empty())
                    .collect();
                if chain && onward[0] != onward[1] {
                    chains.push(if onward[0] { ports[0] } else { ports[1] });
                    continue;
                }
            }
            fans.push(Fan { node, ports });
        }
    }
    (fans, chains)
}

/// The edges of a fan, and how far the middle of the ends of its outer two edges is past the
/// middle of their ports. `None` where the fan's edges are not one to a port, where its inner two
/// edges, of four, would not stand symmetric with the outer two, or where its outer edges would
/// then take a step shorter than [`TURN`].
fn fan_off(layers: &Layers, fan: &Fan) -> Option<(Vec<usize>, f64)> {
    let edges: Vec<usize> = fan
        .ports
        .iter()
        .filter_map(|&port| layers.ports[port].edges.first().copied())
        .collect();
    if edges.len() != fan.ports.len() {
        return None;
    }
    let absolute =
        |port: usize| layers.nodes[layers.ports[port].node].place + layers.ports[port].at;
    let off_of = |a: usize, b: usize| {
        let (port_a, port_b) = (fan.ports[a], fan.ports[b]);
        (absolute(far_end(layers, edges[a], port_a)) + absolute(far_end(layers, edges[b], port_b)))
            / 2.0
            - (absolute(port_a) + absolute(port_b)) / 2.0
    };
    let last = edges.len() - 1;
    let off = off_of(0, last);
    if edges.len() == 4 && (off_of(1, 2) - off).abs() >= 1.0 {
        return None;
    }
    // The step each outer edge takes once the fan stands symmetric.
    let step = ((absolute(far_end(layers, edges[last], fan.ports[last]))
        - absolute(far_end(layers, edges[0], fan.ports[0])))
        - (absolute(fan.ports[last]) - absolute(fan.ports[0])))
        / 2.0;
    if off.abs() >= 1.0 && step.abs() < TURN {
        return None;
    }
    Some((edges, off))
}

/// The port at the other end of an edge from one of its ports.
fn far_end(layers: &Layers, edge: usize, port: usize) -> usize {
    let (source, target) = (layers.edges[edge].source, layers.edges[edge].target);
    if source == port { target } else { source }
}

/// Puts each even fan of equals symmetric around the middle of its node: the node's group moves
/// into the middle of the ends of the outer two edges, or the groups at the ends of all edges move
/// together the other way, whichever costs less on the edges outside the fans, and the node where
/// both cost the same. Neither where both cost more there or break a distance in a layer. Moving
/// one end alone to the mirror of the other would spread the fan wide. A fan of four is centered
/// only where its inner two edges then stand symmetric as well. Returns the ports of the fans that
/// do not stand symmetric.
fn center_fans(
    layers: &mut Layers,
    weights: &[f64],
    axis: &[bool],
    fans: &[Fan],
) -> Vec<Vec<usize>> {
    if fans.is_empty() || weights.len() != layers.edges.len() {
        return Vec::new();
    }
    // The edges of all fans: what the move of one fan's group costs on another fan's edge, the
    // other fan's move takes back.
    let mut fanned = vec![false; layers.edges.len()];
    for fan in fans {
        for &port in &fan.ports {
            for &edge in &layers.ports[port].edges {
                fanned[edge] = true;
            }
        }
    }
    let count = layers.nodes.len();
    let mut neighbors = vec![(None, None); count];
    for members in &layers.layers {
        for pair in members.windows(2) {
            neighbors[pair[1]].0 = Some(pair[0]);
            neighbors[pair[0]].1 = Some(pair[1]);
        }
    }
    let mut touching: Vec<Vec<usize>> = vec![Vec::new(); count];
    for edge in 0..layers.edges.len() {
        let (from, to) = ends(layers, edge);
        touching[from].push(edge);
        touching[to].push(edge);
    }
    for _ in 0..ROUNDS {
        let mut changed = false;
        let mut group = groups(layers, weights, axis);
        let mut members: Vec<Vec<usize>> = vec![Vec::new(); count];
        for node in 0..count {
            let root = find(&mut group, node);
            members[root].push(node);
        }
        for fan in fans {
            let Some((edges, off)) = fan_off(layers, fan) else {
                continue;
            };
            if off.abs() < 1.0 {
                continue;
            }
            let own = group[fan.node];
            let children: Vec<usize> = edges
                .iter()
                .zip(&fan.ports)
                .map(|(&edge, &port)| group[layers.ports[far_end(layers, edge, port)].node])
                .collect();
            let distinct: BTreeSet<usize> = children.iter().copied().collect();
            let mut candidates: Vec<(Vec<usize>, f64)> = Vec::new();
            if !distinct.contains(&own) {
                candidates.push((vec![own], off));
                if distinct.len() == children.len() {
                    candidates.push((children, -off));
                }
            }
            let mut best: Option<(f64, usize)> = None;
            for (index, (moving, shift)) in candidates.iter().enumerate() {
                let inside = |node: usize| moving.contains(&group[node]);
                let nodes = moving.iter().flat_map(|&root| &members[root]).copied();
                let mut more = 0.0;
                let mut fits = true;
                for node in nodes {
                    let place = layers.nodes[node].place + shift;
                    let (before, after) = neighbors[node];
                    fits &= before.filter(|&other| !inside(other)).is_none_or(|other| {
                        place - layers.nodes[other].place >= layers.nodes[node].gap - 1e-6
                    });
                    fits &= after.filter(|&other| !inside(other)).is_none_or(|other| {
                        layers.nodes[other].place - place >= layers.nodes[other].gap - 1e-6
                    });
                    for &edge in &touching[node] {
                        let (from, to) = ends(layers, edge);
                        if fanned[edge] || (inside(from) && inside(to)) {
                            continue;
                        }
                        let step = layers.step(edge);
                        let moved = if from == node {
                            step + shift
                        } else {
                            step - shift
                        };
                        more += weights[edge] * (moved.abs() - step.abs());
                    }
                }
                if !fits || more > 1e-6 {
                    continue;
                }
                if best.is_none_or(|(least, _)| more < least - 1e-6) {
                    best = Some((more, index));
                }
            }
            let Some((_, index)) = best else {
                continue;
            };
            let (moving, shift) = &candidates[index];
            for &root in moving {
                for &node in &members[root] {
                    layers.nodes[node].place += shift;
                }
            }
            changed = true;
        }
        if !changed {
            break;
        }
    }
    fans.iter()
        .filter(|fan| fan_off(layers, fan).is_none_or(|(_, off)| off.abs() >= 1.0))
        .map(|fan| fan.ports.clone())
        .collect()
}

/// Moves each group of nodes that stand on one axis to where its cost is least, with the squares
/// of the steps beside their lengths: where the lengths leave a stretch open, into its middle.
fn move_to_middles(layers: &mut Layers, weights: &[f64], axis: &[bool]) {
    let count = layers.nodes.len();
    let mut group = groups(layers, weights, axis);
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

    /// Two edges from a node 136 long to another below it, each through a dummy 100 long of the
    /// given kinds, as the axis leaves them: the second edge straight down the middle, the first
    /// beside it.
    fn twins(kinds: [Kind; 2]) -> Layers {
        use ferrigura_elk::placement::{Edge, Node, Port};
        let node = |kind, size, place, gap| Node {
            kind,
            size,
            place,
            gap,
            ports: Vec::new(),
        };
        let mut nodes = vec![
            node(Kind::Node, 136.0, 0.0, 0.0),
            node(kinds[0], 100.0, -102.0, 0.0),
            node(kinds[1], 100.0, 18.0, 120.0),
            node(Kind::Node, 136.0, 0.0, 0.0),
        ];
        let mut ports = Vec::new();
        let mut port = |nodes: &mut Vec<Node>, node: usize, side, at| {
            nodes[node].ports.push(ports.len());
            ports.push(Port {
                node,
                side,
                at,
                edges: Vec::new(),
                inside: false,
            });
            ports.len() - 1
        };
        let ends = [
            (
                port(&mut nodes, 0, Side::After, 58.0),
                port(&mut nodes, 1, Side::Before, 50.0),
            ),
            (
                port(&mut nodes, 1, Side::After, 50.0),
                port(&mut nodes, 3, Side::Before, 58.0),
            ),
            (
                port(&mut nodes, 0, Side::After, 78.0),
                port(&mut nodes, 2, Side::Before, 50.0),
            ),
            (
                port(&mut nodes, 2, Side::After, 50.0),
                port(&mut nodes, 3, Side::Before, 78.0),
            ),
        ];
        let edges: Vec<Edge> = ends
            .iter()
            .map(|&(source, target)| Edge {
                source,
                target,
                reversed: false,
            })
            .collect();
        for (index, &(source, target)) in ends.iter().enumerate() {
            ports[source].edges.push(index);
            ports[target].edges.push(index);
        }
        Layers::new(vec![vec![0], vec![1, 2], vec![3]], nodes, ports, edges)
    }

    /// A node `P` 100 long over two others, `A` straight below it and `B` beside it with `C`
    /// straight below `B`, all 100 long, each edge from a port in the middle of its side.
    fn fan() -> Layers {
        use ferrigura_elk::placement::{Edge, Node, Port};
        let node = |place, gap| Node {
            kind: Kind::Node,
            size: 100.0,
            place,
            gap,
            ports: Vec::new(),
        };
        let mut nodes = vec![
            node(0.0, 0.0),
            node(-10.0, 0.0),
            node(130.0, 120.0),
            node(130.0, 0.0),
        ];
        let mut ports = Vec::new();
        let mut port = |nodes: &mut Vec<Node>, node: usize, side, at| {
            nodes[node].ports.push(ports.len());
            ports.push(Port {
                node,
                side,
                at,
                edges: Vec::new(),
                inside: false,
            });
            ports.len() - 1
        };
        let ends = [
            (
                port(&mut nodes, 0, Side::After, 40.0),
                port(&mut nodes, 1, Side::Before, 50.0),
            ),
            (
                port(&mut nodes, 0, Side::After, 60.0),
                port(&mut nodes, 2, Side::Before, 50.0),
            ),
            (
                port(&mut nodes, 2, Side::After, 50.0),
                port(&mut nodes, 3, Side::Before, 50.0),
            ),
        ];
        let edges: Vec<Edge> = ends
            .iter()
            .map(|&(source, target)| Edge {
                source,
                target,
                reversed: false,
            })
            .collect();
        for (index, &(source, target)) in ends.iter().enumerate() {
            ports[source].edges.push(index);
            ports[target].edges.push(index);
        }
        Layers::new(vec![vec![0], vec![1, 2], vec![3]], nodes, ports, edges)
    }

    #[test]
    fn a_node_moves_into_the_middle_of_its_even_fan() {
        let mut layers = fan();
        let axis = [false, false, true];
        let weights = [4.0, 4.0, 4.0 * AXIS];
        let (fans, chains) = even_fans(&layers, &axis);
        assert_eq!(
            fans,
            [Fan {
                node: 0,
                ports: vec![0, 2]
            }]
        );
        assert!(chains.is_empty());
        assert!(center_fans(&mut layers, &weights, &axis, &fans).is_empty());
        // The ends at 40 and 180: the ports' middle moves from 50 to 110.
        let places: Vec<f64> = layers.nodes.iter().map(|node| node.place).collect();
        assert_eq!(places, [60.0, -10.0, 130.0, 130.0]);
    }

    #[test]
    fn a_fan_under_a_chain_whose_one_edge_leads_on_is_a_chain() {
        let mut layers = fan();
        // An edge into `P` from a node above it, on the axis.
        use ferrigura_elk::placement::{Edge, Node, Port};
        layers.nodes.push(Node {
            kind: Kind::Node,
            size: 100.0,
            place: 0.0,
            gap: 0.0,
            ports: vec![6],
        });
        layers.nodes[0].ports.push(7);
        for (node, side) in [(4, Side::After), (0, Side::Before)] {
            layers.ports.push(Port {
                node,
                side,
                at: 50.0,
                edges: vec![3],
                inside: false,
            });
        }
        layers.edges.push(Edge {
            source: 6,
            target: 7,
            reversed: false,
        });
        layers.layers.insert(0, vec![4]);
        let axis = [false, false, true, true];
        let (fans, chains) = even_fans(&layers, &axis);
        assert!(fans.is_empty());
        // The chain goes on through `B`.
        assert_eq!(chains, [2]);
        // Without the edge to `C`, both edges of `P` end below it: a fan again.
        let axis = [false, false, false, true];
        layers.ports[4].edges.clear();
        layers.ports[5].edges.clear();
        layers.nodes[2].ports.retain(|&port| port != 4);
        layers.nodes[3].ports.clear();
        let (fans, chains) = even_fans(&layers, &axis);
        assert_eq!(fans.len(), 1);
        assert!(chains.is_empty());
    }

    #[test]
    fn labeled_twins_stand_on_both_sides_of_their_nodes() {
        let mut layers = twins([Kind::Label, Kind::Label]);
        spread_twins(&mut layers);
        // The middles of the labels 120 apart, 60 on each side of the nodes' middle at 68.
        assert_eq!(layers.nodes[1].place + 50.0, 8.0);
        assert_eq!(layers.nodes[2].place + 50.0, 128.0);
        // The sides have no room for ports 120 apart, so the bundles stay, around the middle.
        let at: Vec<f64> = layers.ports.iter().map(|port| port.at).collect();
        assert_eq!(at, [58.0, 50.0, 50.0, 58.0, 78.0, 50.0, 50.0, 78.0]);
    }

    #[test]
    fn labeled_twins_run_straight_where_the_sides_have_room() {
        let mut layers = twins([Kind::Label, Kind::Label]);
        // Labels 40 long, 60 apart at least: 30 on each side of the middle of sides 136 long.
        for (dummy, place) in [(1, -22.0), (2, 38.0)] {
            layers.nodes[dummy].size = 40.0;
            layers.nodes[dummy].place = place;
            for port in layers.nodes[dummy].ports.clone() {
                layers.ports[port].at = 20.0;
            }
        }
        layers.nodes[2].gap = 60.0;
        spread_twins(&mut layers);
        assert_eq!(layers.nodes[1].place + 20.0, 38.0);
        assert_eq!(layers.nodes[2].place + 20.0, 98.0);
        let at: Vec<f64> = layers.ports.iter().map(|port| port.at).collect();
        assert_eq!(at, [38.0, 20.0, 20.0, 38.0, 98.0, 20.0, 20.0, 98.0]);
        assert!(
            layers
                .edges
                .iter()
                .enumerate()
                .all(|(edge, _)| layers.step(edge) == 0.0)
        );
    }

    #[test]
    fn twins_of_which_one_has_no_label_keep_their_axis() {
        let mut layers = twins([Kind::Label, Kind::Bend]);
        let before = layers.clone();
        spread_twins(&mut layers);
        for (node, kept) in layers.nodes.iter().zip(&before.nodes) {
            assert_eq!(node.place, kept.place);
        }
        for (port, kept) in layers.ports.iter().zip(&before.ports) {
            assert_eq!(port.at, kept.at);
        }
    }

    /// A group of the given size and place over three nodes 152 long at 0, 200 and 400, with the
    /// ports of its edges to them where ELK leaves them: at 12, 184 and 204.
    fn group_over_three(size: f64, place: f64) -> Layers {
        use ferrigura_elk::placement::{Edge, Node, Port};
        let node = |size, place, gap| Node {
            kind: Kind::Node,
            size,
            place,
            gap,
            ports: Vec::new(),
        };
        let mut nodes = vec![
            node(size, place, 0.0),
            node(152.0, 0.0, 0.0),
            node(152.0, 200.0, 200.0),
            node(152.0, 400.0, 200.0),
        ];
        let mut ports = Vec::new();
        let mut edges = Vec::new();
        for (index, at) in [12.0, 184.0, 204.0].into_iter().enumerate() {
            for (owner, side, at) in [(0, Side::After, at), (index + 1, Side::Before, 76.0)] {
                nodes[owner].ports.push(ports.len());
                ports.push(Port {
                    node: owner,
                    side,
                    at,
                    edges: vec![index],
                    inside: false,
                });
            }
            edges.push(Edge {
                source: 2 * index,
                target: 2 * index + 1,
                reversed: false,
            });
        }
        Layers::new(vec![vec![0], vec![1, 2, 3]], nodes, ports, edges)
    }

    #[test]
    fn the_uneven_ports_of_a_group_over_a_fan_become_a_bundle() {
        let mut layers = group_over_three(216.0, 92.0);
        let bundled = center_bundles(&mut layers);
        assert_eq!(bundled, BTreeSet::from([(0, Side::After)]));
        let at = |layers: &Layers| [0, 2, 4].map(|port| layers.ports[port].at);
        assert_eq!(at(&layers), [88.0, 108.0, 128.0]);
        // The outer nodes stand beyond the group's side: the bundle stays, and both outer edges
        // turn.
        straighten_bundles(&mut layers, &bundled);
        assert_eq!(at(&layers), [88.0, 108.0, 128.0]);
        // A group wide enough: each edge runs straight.
        let mut layers = group_over_three(600.0, -24.0);
        let bundled = center_bundles(&mut layers);
        straighten_bundles(&mut layers, &bundled);
        assert_eq!(at(&layers), [100.0, 300.0, 500.0]);
        assert!((0..3).all(|edge| layers.step(edge) == 0.0));
    }

    /// The twins of [`twins`], with a third edge from the upper node to a node beside the lower
    /// one, from a port after theirs.
    fn twins_beside(kinds: [Kind; 2]) -> Layers {
        use ferrigura_elk::placement::{Edge, Node, Port};
        let mut layers = twins(kinds);
        layers.nodes.push(Node {
            kind: Kind::Node,
            size: 100.0,
            place: 300.0,
            gap: 200.0,
            ports: vec![9],
        });
        layers.nodes[0].ports.push(8);
        for (node, side, at) in [(0, Side::After, 98.0), (4, Side::Before, 50.0)] {
            layers.ports.push(Port {
                node,
                side,
                at,
                edges: vec![4],
                inside: false,
            });
        }
        layers.edges.push(Edge {
            source: 8,
            target: 9,
            reversed: false,
        });
        layers.layers[1].push(4);
        layers
    }

    #[test]
    fn labeled_twins_beside_another_edge_are_the_middle_of_their_side() {
        let layers = twins_beside([Kind::Label, Kind::Label]);
        // Of the units of the side, the twins and the edge beside them, the twins weigh more.
        let side = side_ports(&layers, 0, Side::After);
        assert_eq!(side, [0, 4, 8]);
        assert_eq!(middle_twins(&layers, &side), Some(vec![0, 4]));
        assert_eq!(
            middle_ports(&layers, &side, &BTreeSet::new(), false),
            [0, 4]
        );
        // Without a label on both, the side has no twins.
        let layers = twins_beside([Kind::Label, Kind::Bend]);
        assert_eq!(middle_twins(&layers, &side), None);
    }

    #[test]
    fn labeled_twins_beside_another_edge_stand_on_both_sides_of_their_line() {
        let mut layers = twins_beside([Kind::Label, Kind::Label]);
        // The pairs of ports at 58 and 78 of both nodes: their middles on one line at 68.
        spread_twins(&mut layers);
        assert_eq!(layers.nodes[1].place + 50.0, 8.0);
        assert_eq!(layers.nodes[2].place + 50.0, 128.0);
        // Beside another edge, the ports stay a bundle.
        let at: Vec<f64> = layers.ports.iter().map(|port| port.at).collect();
        assert_eq!(
            at,
            [58.0, 50.0, 50.0, 58.0, 78.0, 50.0, 50.0, 78.0, 98.0, 50.0]
        );
    }
}
