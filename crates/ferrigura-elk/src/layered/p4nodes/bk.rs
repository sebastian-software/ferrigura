//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/p4nodes/bk/BKNodePlacer.java
//!
//! Brandes & Koepf node placement, `BKNodePlacer`. It builds four candidate layouts, one for each
//! combination of vertical and horizontal direction, and applies the smallest feasible one, or the
//! balanced median of the four. The work is split as in Java: [`neighborhood`] (`NeighborhoodInformation`),
//! [`aligned_layout`] (`BKAlignedLayout`), [`aligner`] (`BKAligner`), [`compactor`] (`BKCompactor`) and
//! [`threshold`] (`ThresholdStrategy`). This file holds `BKNodePlacer` itself, with the conflict marking,
//! the balancing and the feasibility check, and the static helpers `getEdge`, `getBlocks` and `getClasses`.
//!
//! A node is referred to by its placement id (`LNode.id` in Java). The marked edges are a set of edge
//! indices. Java's `HashSet<LEdge>` iterates in identity-hash order, which depends on elkjs's run and not on
//! the graph, so the `Marked edges` log lists them in the order they were marked instead.
//!
//! `getLayoutProcessorConfiguration`, which adds `HIERARCHICAL_PORT_POSITION_PROCESSOR` before edge
//! routing for graphs with external ports, is in `phases::PhaseStep::configuration`.

pub mod aligned_layout;
pub mod aligner;
pub mod compactor;
pub mod neighborhood;
pub mod threshold;

mod access;

use indexmap::{IndexMap, IndexSet};

use crate::core::data::enums::{FixedAlignment, NodeType};
use crate::core::util::progress::{Monitor, ProgressMonitor};
use crate::error::LayoutError;
use crate::json::js_number_string;
use crate::layered::lgraph::{LEdge, LGraph};
use crate::layered::options;

use access::{connected_edges, incoming_edges, java_max, java_min, lnode, name_of};
pub use aligned_layout::{AlignedLayout, HDirection, VDirection};
use neighborhood::NeighborhoodInformation;
use threshold::ThresholdKind;

/// `BKNodePlacer.MIN_LAYERS_FOR_CONFLICTS`: conflicts need at least three layers.
const MIN_LAYERS_FOR_CONFLICTS: usize = 3;

/// `BKNodePlacer.process`: sets the y coordinate of every node.
pub fn place(
    graph: &mut LGraph,
    progress: &mut ProgressMonitor,
    monitor: Monitor,
) -> Result<(), LayoutError> {
    progress.begin(monitor, "Brandes & Koepf node placement", 1.0);

    let ni = NeighborhoodInformation::build_for(graph);
    let positions = node_positions(graph, &ni, progress, monitor)?;
    for (node, y) in positions {
        graph.nodes[node].position.y = y;
    }

    progress.done(monitor);
    Ok(())
}

/// The body of `process` that does not change the graph: builds the layouts, chooses one, and returns
/// the y coordinate of each node, by arena index.
fn node_positions(
    graph: &LGraph,
    ni: &NeighborhoodInformation,
    progress: &mut ProgressMonitor,
    monitor: Monitor,
) -> Result<Vec<(usize, f64)>, LayoutError> {
    let logging = progress.is_logging_enabled();

    // A balanced layout is wanted if no fixed alignment is set and straight edges are not favoured,
    // or if a balanced alignment is enforced.
    let align = graph
        .properties
        .enum_with_default::<FixedAlignment>(options::NODE_PLACEMENT_BK_FIXED_ALIGNMENT);
    let favor_straight_edges = graph
        .properties
        .bool_of(options::NODE_PLACEMENT_FAVOR_STRAIGHT_EDGES);
    let produce_balanced_layout = (align == FixedAlignment::NONE && !favor_straight_edges)
        || align == FixedAlignment::BALANCED;
    let straightening = ThresholdKind::for_graph(graph);

    // The type 1 conflicts, which are the same for all four layouts.
    let marked_edges = mark_conflicts(graph, ni)?;

    let mut layouts = create_layouts(align, ni.node_count);
    for layout in layouts.iter_mut() {
        aligner::vertical_alignment(graph, ni, layout, &marked_edges);
        aligner::inside_block_shift(graph, ni, layout)?;
    }
    for layout in layouts.iter_mut() {
        compactor::horizontal_compaction(graph, ni, layout, straightening)?;
    }

    if logging {
        for layout in &layouts {
            let size = layout.layout_size(graph);
            progress.log(
                monitor,
                format!("{} size is {}", layout.name(), js_number_string(size)),
            );
        }
    }

    // The balanced layout if one is wanted and it respects the order. Otherwise the feasible layout
    // with the smallest size, the first of equal ones. Without a feasible layout, the first one.
    let balanced = if produce_balanced_layout {
        let candidate = create_balanced_layout(graph, ni, &layouts);
        check_order_constraint(graph, &candidate, progress, monitor).then_some(candidate)
    } else {
        None
    };
    let chosen: &AlignedLayout = match &balanced {
        Some(layout) => layout,
        None => {
            let mut best: Option<usize> = None;
            for (index, layout) in layouts.iter().enumerate() {
                if check_order_constraint(graph, layout, progress, monitor) {
                    let replace = match best {
                        None => true,
                        Some(current) => {
                            layouts[current].layout_size(graph) > layout.layout_size(graph)
                        }
                    };
                    if replace {
                        best = Some(index);
                    }
                }
            }
            match best {
                Some(index) => &layouts[index],
                None => layouts.first().ok_or_else(|| {
                    LayoutError::Unsupported("node placement built no layout".to_string())
                })?,
            }
        }
    };

    if logging {
        progress.log(monitor, format!("Chosen node placement: {}", chosen.name()));
        let blocks = get_blocks(graph, chosen);
        progress.log(monitor, format!("Blocks: {}", map_text(graph, ni, &blocks)));
        let classes = get_classes(chosen, progress, monitor);
        progress.log(
            monitor,
            format!("Classes: {}", map_text(graph, ni, &classes)),
        );
        let marked: Vec<String> = marked_edges
            .iter()
            .map(|&edge| edge_text(graph, &graph.edges[edge]))
            .collect();
        progress.log(monitor, format!("Marked edges: [{}]", marked.join(", ")));
    }

    // The position is the y of the block plus the shift of the node inside it.
    let positions = graph
        .layers
        .iter()
        .flatten()
        .map(|&node| {
            let id = graph.nodes[node].placement_id;
            (node, chosen.y_of(id) + chosen.inner_shift[id])
        })
        .collect();
    Ok(positions)
}

/// The layouts `BKNodePlacer` builds: one for a fixed alignment, all four otherwise.
fn create_layouts(align: FixedAlignment, node_count: usize) -> Vec<AlignedLayout> {
    use HDirection::{Left, Right};
    use VDirection::{Down, Up};
    match align {
        FixedAlignment::LEFTDOWN => vec![AlignedLayout::new(node_count, Some(Down), Some(Left))],
        FixedAlignment::LEFTUP => vec![AlignedLayout::new(node_count, Some(Up), Some(Left))],
        FixedAlignment::RIGHTDOWN => vec![AlignedLayout::new(node_count, Some(Down), Some(Right))],
        FixedAlignment::RIGHTUP => vec![AlignedLayout::new(node_count, Some(Up), Some(Right))],
        _ => vec![
            AlignedLayout::new(node_count, Some(Down), Some(Right)),
            AlignedLayout::new(node_count, Some(Up), Some(Right)),
            AlignedLayout::new(node_count, Some(Down), Some(Left)),
            AlignedLayout::new(node_count, Some(Up), Some(Left)),
        ],
    }
}

/// `markConflicts`: marks the non-inner segments that cross an inner segment (type 1 conflicts), so that
/// the vertical alignment does not join across them. Returns the marked edges in the order they were
/// marked.
fn mark_conflicts(
    graph: &LGraph,
    ni: &NeighborhoodInformation,
) -> Result<IndexSet<usize>, LayoutError> {
    let mut marked_edges = IndexSet::new();
    let number_of_layers = graph.layers.len();
    if number_of_layers < MIN_LAYERS_FOR_CONFLICTS {
        return Ok(marked_edges);
    }

    let layer_size: Vec<usize> = graph.layers.iter().map(Vec::len).collect();

    // The names follow the paper, as in Java: `k` are positions in the layers, and `l` a node index.
    for i in 1..number_of_layers - 1 {
        let current_layer = &graph.layers[i + 1];
        let mut k_0: i64 = 0;
        let mut l: usize = 0;

        for l_1 in 0..layer_size[i + 1] {
            let v_l_i = graph.nodes[current_layer[l_1]].placement_id;

            if l_1 == layer_size[i + 1] - 1 || incident_to_inner_segment(graph, ni, v_l_i, i + 1, i)
            {
                let mut k_1 = layer_size[i] as i64 - 1;
                if incident_to_inner_segment(graph, ni, v_l_i, i + 1, i) {
                    let first = ni.left_neighbors[v_l_i].first().ok_or_else(|| {
                        LayoutError::Format(
                            "an inner segment has no upper neighbour, which elkjs fails on"
                                .to_string(),
                        )
                    })?;
                    k_1 = ni.node_index[first.node] as i64;
                }

                while l <= l_1 {
                    let v_l = graph.nodes[current_layer[l]].placement_id;
                    if !incident_to_inner_segment(graph, ni, v_l, i + 1, i) {
                        for upper in &ni.left_neighbors[v_l] {
                            let k = ni.node_index[upper.node] as i64;
                            if k < k_0 || k > k_1 {
                                marked_edges.insert(upper.edge);
                            }
                        }
                    }
                    l += 1;
                }

                k_0 = k_1;
            }
        }
    }
    Ok(marked_edges)
}

/// `incidentToInnerSegment(node, layer1, layer2)`: whether the node is a dummy of a long edge that
/// comes from a dummy in `layer2`, with the node in `layer1`.
fn incident_to_inner_segment(
    graph: &LGraph,
    ni: &NeighborhoodInformation,
    v: usize,
    layer1: usize,
    layer2: usize,
) -> bool {
    let node = lnode(graph, ni, v);
    if node.node_type != NodeType::LONG_EDGE {
        return false;
    }
    for edge in incoming_edges(graph, node) {
        let source = &graph.nodes[graph.ports[graph.edges[edge].source].node];
        if source.node_type == NodeType::LONG_EDGE
            && ni.layer_index[source.layer.expect("a placed node has a layer")] == layer2
            && ni.layer_index[node.layer.expect("a placed node has a layer")] == layer1
        {
            return true;
        }
    }
    false
}

/// `createBalancedLayout`: the median of the four layouts, each shifted onto the smallest one.
fn create_balanced_layout(
    graph: &LGraph,
    ni: &NeighborhoodInformation,
    layouts: &[AlignedLayout],
) -> AlignedLayout {
    let no_of_layouts = layouts.len();
    let mut balanced = AlignedLayout::new(ni.node_count, None, None);
    let mut width = vec![0.0; no_of_layouts];
    let mut min = vec![f64::from(i32::MAX); no_of_layouts];
    let mut max = vec![f64::from(i32::MIN); no_of_layouts];
    let mut min_width_layout = 0;

    for (i, bal) in layouts.iter().enumerate() {
        width[i] = bal.layout_size(graph);
        if width[min_width_layout] > width[i] {
            min_width_layout = i;
        }
        for layer in &graph.layers {
            for &node in layer {
                let id = graph.nodes[node].placement_id;
                // The inner shift is part of the position here.
                let node_pos_y = bal.y_of(id) + bal.inner_shift[id];
                min[i] = java_min(min[i], node_pos_y);
                max[i] = java_max(max[i], node_pos_y + graph.nodes[node].size.y);
            }
        }
    }

    // The shift of each layout onto the smallest one, from its top for DOWN and from its bottom for UP.
    let shift: Vec<f64> = layouts
        .iter()
        .enumerate()
        .map(|(i, layout)| {
            if layout.vdir == Some(VDirection::Down) {
                min[min_width_layout] - min[i]
            } else {
                max[min_width_layout] - max[i]
            }
        })
        .collect();

    let mut calculated_ys = vec![0.0; no_of_layouts];
    for layer in &graph.layers {
        for &node in layer {
            let id = graph.nodes[node].placement_id;
            for (i, layout) in layouts.iter().enumerate() {
                // It is important to include the inner shift here.
                calculated_ys[i] = layout.y_of(id) + layout.inner_shift[id] + shift[i];
            }
            // Arrays.sort on doubles: ascending, -0.0 before 0.0 and NaN last, which `total_cmp` gives too.
            calculated_ys.sort_by(f64::total_cmp);
            balanced.y[id] = Some((calculated_ys[1] + calculated_ys[2]) / 2.0);
            // The inner shift is in the balanced y now, so it is not used any more.
            balanced.inner_shift[id] = 0.0;
        }
    }
    balanced
}

/// `checkOrderConstraint`: whether the nodes of each layer are in order and do not overlap.
fn check_order_constraint(
    graph: &LGraph,
    bal: &AlignedLayout,
    progress: &mut ProgressMonitor,
    monitor: Monitor,
) -> bool {
    let mut feasible = true;
    'layers: for layer in &graph.layers {
        let mut pos = f64::NEG_INFINITY;
        let mut previous: Option<usize> = None;
        for &node in layer {
            let current = &graph.nodes[node];
            let id = current.placement_id;
            // Both the top border and the bottom border must lie beyond the current position.
            let top = bal.y_of(id) + bal.inner_shift[id] - current.margin.top;
            let bottom =
                bal.y_of(id) + bal.inner_shift[id] + current.size.y + current.margin.bottom;
            if top > pos && bottom > pos {
                previous = Some(node);
                pos = bal.y_of(id) + bal.inner_shift[id] + current.size.y + current.margin.bottom;
            } else {
                feasible = false;
                if progress.is_logging_enabled() {
                    let before =
                        previous.map_or_else(|| "null".to_string(), |p| graph.node_name(p));
                    progress.log(
                        monitor,
                        format!(
                            "bk node placement breaks on {} which should have been after {}",
                            graph.node_name(node),
                            before
                        ),
                    );
                }
                break 'layers;
            }
        }
    }

    if progress.is_logging_enabled() {
        progress.log(monitor, format!("{} is feasible: {}", bal.name(), feasible));
    }
    feasible
}

/// `getEdge(source, target)`: the first connected edge of `source` that joins it to `target`.
pub(super) fn get_edge(
    graph: &LGraph,
    ni: &NeighborhoodInformation,
    source: usize,
    target: usize,
) -> Option<usize> {
    let target_arena = ni.node_of_id[target];
    connected_edges(graph, lnode(graph, ni, source))
        .into_iter()
        .find(|&edge| {
            graph.ports[graph.edges[edge].target].node == target_arena
                || graph.ports[graph.edges[edge].source].node == target_arena
        })
}

/// `getBlocks(bal)`: the nodes of each block, by its root, in layer order. The key is `None` for the
/// balanced layout, which has no roots.
pub(super) fn get_blocks(
    graph: &LGraph,
    bal: &AlignedLayout,
) -> IndexMap<Option<usize>, Vec<usize>> {
    let mut blocks: IndexMap<Option<usize>, Vec<usize>> = IndexMap::new();
    for layer in &graph.layers {
        for &node in layer {
            let id = graph.nodes[node].placement_id;
            blocks.entry(bal.root[id]).or_default().push(id);
        }
    }
    blocks
}

/// `getClasses(bal, monitor)`: the block roots of each class, by sink. A balanced layout has no classes.
fn get_classes(
    bal: &AlignedLayout,
    progress: &mut ProgressMonitor,
    monitor: Monitor,
) -> IndexMap<Option<usize>, Vec<usize>> {
    // The block roots, in node id order, as `Arrays.asList(bal.root)` gives them.
    let roots: IndexSet<Option<usize>> = bal.root.iter().copied().collect();
    let mut classes: IndexMap<Option<usize>, Vec<usize>> = IndexMap::new();
    for root in roots {
        match root {
            None => {
                progress.log(monitor, "There are no classes in a balanced layout.");
                break;
            }
            Some(root) => {
                let sink = bal.sink[root];
                classes.entry(Some(sink)).or_default().push(root);
            }
        }
    }
    classes
}

/// `Map.toString()` of a block or class map: `{key=[value, ...], ...}`, with `null` for no key.
fn map_text(
    graph: &LGraph,
    ni: &NeighborhoodInformation,
    map: &IndexMap<Option<usize>, Vec<usize>>,
) -> String {
    let entries: Vec<String> = map
        .iter()
        .map(|(key, values)| {
            let key_text = match key {
                Some(id) => name_of(graph, ni, *id),
                None => "null".to_string(),
            };
            let values_text: Vec<String> =
                values.iter().map(|&id| name_of(graph, ni, id)).collect();
            format!("{}=[{}]", key_text, values_text.join(", "))
        })
        .collect();
    format!("{{{}}}", entries.join(", "))
}

/// `LEdge.toString()`: `e_<designation> <source port>[<source node>] -> <target port>[<target node>]`.
///
/// The designation is the first label's text, or else the edge's id with the ids of its containers
/// in front (`LGraphElement.getDesignation`, `ElkGraphTransformer.getIdentifier`): an edge of the
/// root level reads `root.<id>`. The level's prefix is `root_id`. The port designation is its index
/// in its node, which is what Java prints for a port with no label and no origin identifier. The importer
/// keeps neither for edges or ports, so the text can differ from Java where those are set.
fn edge_text(graph: &LGraph, edge: &LEdge) -> String {
    let mut text = String::from("e_");
    match edge.labels.first().map(|&id| &graph.labels[id]) {
        Some(label) if !label.text.is_empty() => text.push_str(&label.text),
        _ if !edge.id.is_empty() && !graph.root_id.is_empty() => {
            text.push_str(&format!("{}.{}", graph.root_id, edge.id));
        }
        _ if !edge.id.is_empty() => text.push_str(&edge.id),
        _ => {}
    }
    let source = &graph.ports[edge.source];
    let target = &graph.ports[edge.target];
    text.push_str(&format!(
        " {}[{}] -> {}[{}]",
        port_index(graph, edge.source),
        graph.node_name(source.node),
        port_index(graph, edge.target),
        graph.node_name(target.node),
    ));
    text
}

/// `LPort.getIndex()`: the position of the port in its node's port list.
fn port_index(graph: &LGraph, port: usize) -> usize {
    let owner = graph.ports[port].node;
    graph.nodes[owner]
        .ports
        .iter()
        .position(|&found| found == port)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::OptionValue;
    use crate::core::math::KVector;
    use crate::layered::graph_util::create_port;
    use crate::layered::lgraph::{LNode, PortType};

    /// Adds a node of the given height to layer `layer`, after the nodes already there.
    fn add_node(
        graph: &mut LGraph,
        designation: &str,
        height: f64,
        node_type: NodeType,
        layer: usize,
    ) -> usize {
        graph.nodes.push(LNode {
            designation: designation.to_string(),
            size: KVector { x: 10.0, y: height },
            node_type,
            ..LNode::default()
        });
        let index = graph.nodes.len() - 1;
        while graph.layers.len() <= layer {
            graph.layers.push(Vec::new());
        }
        graph.layers[layer].push(index);
        index
    }

    /// Adds an edge from `source` to `target`, with a port at the origin of each node.
    fn add_edge(graph: &mut LGraph, source: usize, target: usize) -> usize {
        let out = create_port(graph, source, PortType::OUTPUT);
        let inn = create_port(graph, target, PortType::INPUT);
        graph.edges.push(LEdge {
            source: out,
            target: inn,
            thickness: 1.0,
            ..LEdge::default()
        });
        let edge = graph.edges.len() - 1;
        graph.ports[out].outgoing.push(edge);
        graph.ports[inn].incoming.push(edge);
        edge
    }

    /// The placement id of an arena node, once the neighbourhood has assigned them.
    fn id_of(graph: &LGraph, arena: usize) -> usize {
        graph.nodes[arena].placement_id
    }

    /// Two layers of one node of height 10 each, joined by an edge whose source port sits 5 below the
    /// top of its node. The target port is at the top of its node.
    fn offset_chain() -> LGraph {
        let mut graph = LGraph::default();
        let a = add_node(&mut graph, "a", 10.0, NodeType::NORMAL, 0);
        let b = add_node(&mut graph, "b", 10.0, NodeType::NORMAL, 1);
        let edge = add_edge(&mut graph, a, b);
        let out = graph.edges[edge].source;
        graph.ports[out].position.y = 5.0;
        graph
    }

    #[test]
    fn a_normal_edge_crossing_a_long_edge_is_marked() {
        // Layer 1 holds the dummy d and the node b, and layer 2 the node c and the dummy e. The long
        // edge d -> e crosses b -> c, so b -> c is marked.
        let mut graph = LGraph::default();
        let a = add_node(&mut graph, "a", 10.0, NodeType::NORMAL, 0);
        let d = add_node(&mut graph, "d", 10.0, NodeType::LONG_EDGE, 1);
        let b = add_node(&mut graph, "b", 10.0, NodeType::NORMAL, 1);
        let c = add_node(&mut graph, "c", 10.0, NodeType::NORMAL, 2);
        let e = add_node(&mut graph, "e", 10.0, NodeType::LONG_EDGE, 2);
        add_edge(&mut graph, a, d);
        add_edge(&mut graph, d, e);
        let crossing = add_edge(&mut graph, b, c);
        let ni = NeighborhoodInformation::build_for(&mut graph);
        let marked = mark_conflicts(&graph, &ni).expect("marking");
        assert_eq!(marked.into_iter().collect::<Vec<_>>(), vec![crossing]);
    }

    #[test]
    fn a_node_joins_the_first_neighbour_going_down_and_the_last_going_up() {
        let mut graph = LGraph::default();
        let p0 = add_node(&mut graph, "p0", 10.0, NodeType::NORMAL, 0);
        let p1 = add_node(&mut graph, "p1", 10.0, NodeType::NORMAL, 0);
        let x = add_node(&mut graph, "x", 10.0, NodeType::NORMAL, 1);
        add_edge(&mut graph, p0, x);
        add_edge(&mut graph, p1, x);
        let ni = NeighborhoodInformation::build_for(&mut graph);
        let marked = IndexSet::new();

        let mut down = AlignedLayout::new(3, Some(VDirection::Down), Some(HDirection::Right));
        aligner::vertical_alignment(&graph, &ni, &mut down, &marked);
        assert_eq!(down.root_of(id_of(&graph, x)), id_of(&graph, p0));

        let mut up = AlignedLayout::new(3, Some(VDirection::Up), Some(HDirection::Right));
        aligner::vertical_alignment(&graph, &ni, &mut up, &marked);
        assert_eq!(up.root_of(id_of(&graph, x)), id_of(&graph, p1));
    }

    #[test]
    fn a_marked_edge_is_not_joined() {
        let mut graph = LGraph::default();
        let p0 = add_node(&mut graph, "p0", 10.0, NodeType::NORMAL, 0);
        let p1 = add_node(&mut graph, "p1", 10.0, NodeType::NORMAL, 0);
        let x = add_node(&mut graph, "x", 10.0, NodeType::NORMAL, 1);
        let first = add_edge(&mut graph, p0, x);
        add_edge(&mut graph, p1, x);
        let ni = NeighborhoodInformation::build_for(&mut graph);
        let marked: IndexSet<usize> = [first].into_iter().collect();
        let mut down = AlignedLayout::new(3, Some(VDirection::Down), Some(HDirection::Right));
        aligner::vertical_alignment(&graph, &ni, &mut down, &marked);
        // The edge from p0 is marked, so x joins p1.
        assert_eq!(down.root_of(id_of(&graph, x)), id_of(&graph, p1));
    }

    #[test]
    fn a_node_with_three_neighbours_joins_the_middle_one() {
        let mut graph = LGraph::default();
        let p0 = add_node(&mut graph, "p0", 10.0, NodeType::NORMAL, 0);
        let p1 = add_node(&mut graph, "p1", 10.0, NodeType::NORMAL, 0);
        let p2 = add_node(&mut graph, "p2", 10.0, NodeType::NORMAL, 0);
        let x = add_node(&mut graph, "x", 10.0, NodeType::NORMAL, 1);
        add_edge(&mut graph, p0, x);
        add_edge(&mut graph, p1, x);
        add_edge(&mut graph, p2, x);
        let ni = NeighborhoodInformation::build_for(&mut graph);
        let marked = IndexSet::new();
        for vdir in [VDirection::Down, VDirection::Up] {
            let mut layout = AlignedLayout::new(4, Some(vdir), Some(HDirection::Right));
            aligner::vertical_alignment(&graph, &ni, &mut layout, &marked);
            assert_eq!(
                layout.root_of(id_of(&graph, x)),
                id_of(&graph, p1),
                "{vdir:?}"
            );
        }
    }

    #[test]
    fn a_port_offset_moves_the_target_inside_its_block_and_balances_the_chain() {
        // Each of the four layouts is 15 high: the block of a and b spans 5 below the top of a
        // because b's port is level with a's port. The balanced y of b is therefore 5.
        let mut graph = offset_chain();
        let (mut progress, root) = ProgressMonitor::new(true);
        place(&mut graph, &mut progress, root).expect("place");
        assert_eq!(graph.nodes[0].position.y, 0.0);
        assert_eq!(graph.nodes[1].position.y, 5.0);
        assert_eq!(
            progress.trace(root).logs,
            vec![
                "RIGHTDOWN size is 15",
                "RIGHTUP size is 15",
                "LEFTDOWN size is 15",
                "LEFTUP size is 15",
                "BALANCED is feasible: true",
                "Chosen node placement: BALANCED",
                "Blocks: {null=[n_a, n_b]}",
                "There are no classes in a balanced layout.",
                "Classes: {}",
                "Marked edges: []",
            ]
        );
    }

    #[test]
    fn a_long_edge_chain_is_one_block_and_the_fixed_layout_is_applied() {
        let mut graph = LGraph::default();
        let a = add_node(&mut graph, "a", 10.0, NodeType::NORMAL, 0);
        let d = add_node(&mut graph, "d", 10.0, NodeType::LONG_EDGE, 1);
        let b = add_node(&mut graph, "b", 10.0, NodeType::NORMAL, 2);
        add_edge(&mut graph, a, d);
        add_edge(&mut graph, d, b);
        graph.properties.set(
            options::NODE_PLACEMENT_BK_FIXED_ALIGNMENT,
            Some(OptionValue::Enum {
                class: "FixedAlignment",
                name: "RIGHTDOWN".to_string(),
            }),
        );
        let (mut progress, root) = ProgressMonitor::new(true);
        place(&mut graph, &mut progress, root).expect("place");
        for node in &graph.nodes {
            assert_eq!(node.position.y, 0.0, "{}", node.designation);
        }
        assert_eq!(
            progress.trace(root).logs,
            vec![
                "RIGHTDOWN size is 10",
                "RIGHTDOWN is feasible: true",
                "Chosen node placement: RIGHTDOWN",
                "Blocks: {n_a=[n_a, n(long_edge)_d, n_b]}",
                "Classes: {n_a=[n_a]}",
                "Marked edges: []",
            ]
        );
    }

    #[test]
    fn nodes_of_a_layer_do_not_overlap_after_placement() {
        // Invariant only: the expected coordinates of this graph are not derived by hand. Every
        // layer must come out in order, with each node below the one before it.
        let mut graph = LGraph::default();
        let a = add_node(&mut graph, "a", 12.0, NodeType::NORMAL, 0);
        let b = add_node(&mut graph, "b", 30.0, NodeType::NORMAL, 0);
        let c = add_node(&mut graph, "c", 8.0, NodeType::NORMAL, 1);
        let d = add_node(&mut graph, "d", 20.0, NodeType::LONG_EDGE, 1);
        let e = add_node(&mut graph, "e", 16.0, NodeType::NORMAL, 2);
        let f = add_node(&mut graph, "f", 10.0, NodeType::NORMAL, 2);
        add_edge(&mut graph, a, d);
        add_edge(&mut graph, b, c);
        add_edge(&mut graph, d, f);
        add_edge(&mut graph, c, e);
        add_edge(&mut graph, a, c);
        let (mut progress, root) = ProgressMonitor::new(false);
        place(&mut graph, &mut progress, root).expect("place");
        for layer in &graph.layers {
            for pair in layer.windows(2) {
                let upper = &graph.nodes[pair[0]];
                let lower = &graph.nodes[pair[1]];
                assert!(
                    lower.position.y >= upper.position.y + upper.size.y,
                    "{} overlaps {}",
                    upper.designation,
                    lower.designation
                );
            }
        }
    }

    #[test]
    fn map_text_matches_java_map_to_string() {
        let mut graph = LGraph::default();
        let a = add_node(&mut graph, "a", 10.0, NodeType::NORMAL, 0);
        let b = add_node(&mut graph, "b", 10.0, NodeType::NORMAL, 0);
        let ni = NeighborhoodInformation::build_for(&mut graph);
        let mut blocks: IndexMap<Option<usize>, Vec<usize>> = IndexMap::new();
        blocks.insert(Some(id_of(&graph, a)), vec![id_of(&graph, a)]);
        blocks.insert(None, vec![id_of(&graph, a), id_of(&graph, b)]);
        assert_eq!(
            map_text(&graph, &ni, &blocks),
            "{n_a=[n_a], null=[n_a, n_b]}"
        );
    }
}
