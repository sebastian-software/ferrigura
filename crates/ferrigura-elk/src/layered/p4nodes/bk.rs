//! Brandes & Koepf node placement, `org.eclipse.elk.alg.layered.p4nodes.bk` in the Java sources.
//!
//! Ported from `BKNodePlacer`, `BKAlignedLayout`, `BKAligner`, `BKCompactor`,
//! `NeighborhoodInformation` and `ThresholdStrategy` (the null and simple
//! strategies). The Java arrays indexed by `LNode.id` are vectors indexed by the
//! placement id, which `NeighborhoodInformation` gives each node in layer order.
//!
//! Edges are not imported yet, so every neighbour list is empty and the
//! edge-dependent branches are not reached. Where Java would reach them, this
//! port reports `Unsupported` rather than skipping them.

use std::collections::VecDeque;
use std::fmt::Write as _;

use indexmap::IndexMap;

use crate::core::data::JavaEnum;
use crate::core::data::enums::{EdgeStraighteningStrategy, FixedAlignment, NodeType};
use crate::core::data::property::PropertyHolder;
use crate::core::util::progress::{Monitor, ProgressMonitor};
use crate::error::LayoutError;
use crate::json::js_number_string;
use crate::layered::lgraph::{LGraph, LNode};
use crate::layered::options;
use crate::layered::spacings;

/// `VDirection`, with `None` for the balanced layout (`vdir == null`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VDirection {
    Down,
    Up,
}

/// `HDirection`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HDirection {
    Right,
    Left,
}

/// `NeighborhoodInformation`: layer and node indices, and the neighbour lists.
#[derive(Debug, Clone, Default)]
pub struct NeighborhoodInformation {
    pub node_count: usize,
    /// Index of each layer, by layer id.
    pub layer_index: Vec<usize>,
    /// Index of each node within its layer, by placement id.
    pub node_index: Vec<usize>,
    /// Arena index of each node, by placement id.
    pub node_of_id: Vec<usize>,
    /// Neighbour placement ids. Empty until edges are imported.
    pub left_neighbors: Vec<Vec<usize>>,
    pub right_neighbors: Vec<Vec<usize>>,
}

impl NeighborhoodInformation {
    /// `NeighborhoodInformation.buildFor`: assigns the placement ids in layer order.
    pub fn build_for(graph: &mut LGraph) -> Self {
        let mut ni = NeighborhoodInformation {
            node_count: graph.layers.iter().map(Vec::len).sum(),
            layer_index: vec![0; graph.layers.len()],
            node_index: vec![0; graph.layers.iter().map(Vec::len).sum()],
            node_of_id: vec![0; graph.layers.iter().map(Vec::len).sum()],
            left_neighbors: Vec::new(),
            right_neighbors: Vec::new(),
        };
        let mut next_id = 0;
        for (layer_id, layer) in graph.layers.iter().enumerate() {
            ni.layer_index[layer_id] = layer_id;
            for (index_in_layer, &node) in layer.iter().enumerate() {
                graph.nodes[node].placement_id = next_id;
                ni.node_index[next_id] = index_in_layer;
                ni.node_of_id[next_id] = node;
                graph.nodes[node].layer = Some(layer_id);
                next_id += 1;
            }
        }
        // Without edges every node has no neighbours on either side.
        ni.left_neighbors = vec![Vec::new(); ni.node_count];
        ni.right_neighbors = vec![Vec::new(); ni.node_count];
        ni
    }
}

/// `BKAlignedLayout`: one candidate placement of the nodes.
#[derive(Debug, Clone)]
pub struct AlignedLayout {
    /// Root of each node's block; `None` in the balanced layout, as in Java.
    pub root: Vec<Option<usize>>,
    pub block_size: Vec<f64>,
    pub align: Vec<usize>,
    pub inner_shift: Vec<f64>,
    pub sink: Vec<usize>,
    pub shift: Vec<f64>,
    pub y: Vec<Option<f64>>,
    pub vdir: Option<VDirection>,
    pub hdir: Option<HDirection>,
    pub su: Vec<bool>,
    pub od: Vec<bool>,
}

impl AlignedLayout {
    fn new(node_count: usize, vdir: Option<VDirection>, hdir: Option<HDirection>) -> Self {
        AlignedLayout {
            root: vec![None; node_count],
            block_size: vec![0.0; node_count],
            align: vec![0; node_count],
            inner_shift: vec![0.0; node_count],
            sink: vec![0; node_count],
            shift: vec![0.0; node_count],
            y: vec![None; node_count],
            vdir,
            hdir,
            su: vec![false; node_count],
            od: vec![true; node_count],
        }
    }

    /// The root of a node's block. Only the balanced layout leaves roots unset, and
    /// nothing reads them from there, so the node itself is the fallback.
    fn root_of(&self, id: usize) -> usize {
        self.root[id].unwrap_or(id)
    }

    /// `y` of a node, which is always set once the layout is built.
    fn y_of(&self, id: usize) -> f64 {
        self.y[id].unwrap_or(0.0)
    }

    /// `layoutSize()`: the height the layout takes up.
    fn layout_size(&self, graph: &LGraph, ni: &NeighborhoodInformation) -> f64 {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for layer in &graph.layers {
            for &node in layer {
                let id = graph.nodes[node].placement_id;
                let y_min = self.y_of(id);
                let y_max = y_min + self.block_size[self.root_of(id)];
                min = min.min(y_min);
                max = max.max(y_max);
            }
        }
        let _ = ni;
        max - min
    }

    /// `toString()`: the direction names, or `BALANCED`.
    fn name(&self) -> String {
        let mut result = String::new();
        match self.hdir {
            Some(HDirection::Right) => result.push_str("RIGHT"),
            Some(HDirection::Left) => result.push_str("LEFT"),
            None => {}
        }
        match self.vdir {
            Some(VDirection::Down) => result.push_str("DOWN"),
            Some(VDirection::Up) => result.push_str("UP"),
            None => result.push_str("BALANCED"),
        }
        result
    }
}

/// `LNode.toString()`: `n_<designation>`, with the type in brackets for non-normal nodes.
fn node_text(node: &LNode) -> String {
    let mut text = String::from("n");
    if node.node_type != NodeType::NORMAL {
        let _ = write!(text, "({})", node.node_type.java_name().to_lowercase());
    }
    let _ = write!(text, "_{}", node.designation);
    text
}

/// `Map.toString()` of an insertion-ordered map of blocks or classes.
fn map_text(entries: &[(Option<String>, Vec<String>)]) -> String {
    let parts: Vec<String> = entries
        .iter()
        .map(|(key, values)| {
            format!(
                "{}=[{}]",
                key.clone().unwrap_or_else(|| "null".to_string()),
                values.join(", ")
            )
        })
        .collect();
    format!("{{{}}}", parts.join(", "))
}

/// `BKNodePlacer.process`: places the nodes of each layer.
pub fn place(
    graph: &mut LGraph,
    progress: &mut ProgressMonitor,
    monitor: Monitor,
) -> Result<(), LayoutError> {
    progress.begin(monitor, "Brandes & Koepf node placement", 1.0);
    let logging = progress.is_logging_enabled();
    let mut ni = NeighborhoodInformation::build_for(graph);
    let align = graph
        .properties
        .enum_of::<FixedAlignment>(options::NODE_PLACEMENT_BK_FIXED_ALIGNMENT)
        .unwrap_or(FixedAlignment::NONE);
    let favor_straight_edges = graph
        .properties
        .bool_of(options::NODE_PLACEMENT_FAVOR_STRAIGHT_EDGES);
    let produce_balanced_layout = (align == FixedAlignment::NONE && !favor_straight_edges)
        || align == FixedAlignment::BALANCED;
    mark_conflicts(graph, &ni)?;

    let layouts = create_layouts(align, ni.node_count);
    let mut layouts = layouts;

    let aligner_state = AlignerState::new();
    for layout in layouts.iter_mut() {
        vertical_alignment(graph, &ni, layout);
        inside_block_shift(graph, &ni, layout)?;
    }
    let threshold = Threshold::for_graph(graph);
    for layout in layouts.iter_mut() {
        horizontal_compaction(graph, &ni, layout, threshold)?;
    }
    let _ = aligner_state;

    if logging {
        for layout in &layouts {
            let size = layout.layout_size(graph, &ni);
            progress.log(
                monitor,
                format!("{} size is {}", layout.name(), js_number_string(size)),
            );
        }
    }

    let mut chosen: Option<AlignedLayout> = None;
    if produce_balanced_layout {
        let balanced = create_balanced_layout(graph, &ni, &layouts);
        if check_order_constraint(graph, &balanced, progress, monitor) {
            chosen = Some(balanced);
        }
    }
    if chosen.is_none() {
        for layout in &layouts {
            if check_order_constraint(graph, layout, progress, monitor) {
                let better = match &chosen {
                    None => true,
                    Some(current) => {
                        current.layout_size(graph, &ni) > layout.layout_size(graph, &ni)
                    }
                };
                if better {
                    chosen = Some(layout.clone());
                }
            }
        }
    }
    let chosen = match chosen {
        Some(layout) => layout,
        None => layouts
            .first()
            .cloned()
            .ok_or_else(|| LayoutError::Unsupported("no candidate placement".to_string()))?,
    };

    for layer in &graph.layers {
        for &node in layer {
            let id = graph.nodes[node].placement_id;
            graph.nodes[node].position.y = chosen.y_of(id) + chosen.inner_shift[id];
        }
    }

    if logging {
        progress.log(monitor, format!("Chosen node placement: {}", chosen.name()));
        let blocks = get_blocks(graph, &chosen);
        progress.log(
            monitor,
            format!("Blocks: {}", map_text(&blocks_text(graph, &blocks))),
        );
        let classes = get_classes(graph, &chosen, progress, monitor);
        let classes_text = classes_text(graph, &classes);
        progress.log(monitor, format!("Classes: {}", map_text(&classes_text)));
        progress.log(monitor, "Marked edges: []");
    }
    ni.node_count = 0;
    progress.done(monitor);
    Ok(())
}

/// The layouts `BKNodePlacer` builds for the fixed alignment: all four for the default.
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

/// `markConflicts`: type 1 conflicts need long edges, which are not imported yet.
fn mark_conflicts(graph: &LGraph, _ni: &NeighborhoodInformation) -> Result<(), LayoutError> {
    let has_edges = false;
    if graph.layers.len() >= 3 && has_edges {
        return Err(LayoutError::Unsupported(
            "conflict marking between layers".to_string(),
        ));
    }
    Ok(())
}

/// The state `BKAligner` and `BKCompactor` share, kept for the list of pending items.
#[derive(Debug, Default)]
struct AlignerState;

impl AlignerState {
    fn new() -> Self {
        AlignerState
    }
}

/// `BKAligner.verticalAlignment`: each node starts as its own block, and blocks
/// join along the neighbour lists (empty without edges).
fn vertical_alignment(graph: &LGraph, ni: &NeighborhoodInformation, bal: &mut AlignedLayout) {
    for layer in &graph.layers {
        for &node in layer {
            let id = graph.nodes[node].placement_id;
            bal.root[id] = Some(id);
            bal.align[id] = id;
            bal.inner_shift[id] = 0.0;
        }
    }
    // No neighbours without edges, so no blocks join and nothing else changes.
    let _ = (ni, bal.hdir);
}

/// `BKAligner.insideBlockShift`: positions the nodes of each block inside it.
///
/// A block of one node is the only kind without edges. For it, Java's loop over
/// the block runs once and the node shifts by its top margin.
fn inside_block_shift(
    graph: &LGraph,
    _ni: &NeighborhoodInformation,
    bal: &mut AlignedLayout,
) -> Result<(), LayoutError> {
    for root in block_roots(graph, bal) {
        let root_node = &graph.nodes[node_of(graph, root)];
        let space_above = root_node.margin.top;
        let space_below = root_node.size.y + root_node.margin.bottom;
        bal.inner_shift[root] = 0.0;
        if bal.align[root] != root {
            // Blocks of more than one node are joined along edges, which are not imported yet.
            return Err(LayoutError::Unsupported(
                "blocks of more than one node need edges".to_string(),
            ));
        }
        bal.inner_shift[root] += space_above;
        bal.block_size[root] = space_above + space_below;
    }
    Ok(())
}

/// The placement ids of the block roots, in layer order (the keys of `getBlocks`).
fn block_roots(graph: &LGraph, bal: &AlignedLayout) -> Vec<usize> {
    let mut roots = Vec::new();
    for layer in &graph.layers {
        for &node in layer {
            let root = bal.root_of(graph.nodes[node].placement_id);
            if !roots.contains(&root) {
                roots.push(root);
            }
        }
    }
    roots
}

/// Arena index of a placement id, found through the layers.
fn node_of(graph: &LGraph, id: usize) -> usize {
    graph
        .layers
        .iter()
        .flatten()
        .copied()
        .find(|&node| graph.nodes[node].placement_id == id)
        .unwrap_or(0)
}

/// `getBlocks`: the nodes of each block, keyed by root, in layer order.
fn get_blocks(graph: &LGraph, bal: &AlignedLayout) -> Vec<(Option<usize>, Vec<usize>)> {
    let mut blocks: IndexMap<Option<usize>, Vec<usize>> = IndexMap::new();
    for layer in &graph.layers {
        for &node in layer {
            let id = graph.nodes[node].placement_id;
            let root = bal.root[id];
            blocks.entry(root).or_default().push(node);
        }
    }
    blocks.into_iter().collect()
}

fn blocks_text(
    graph: &LGraph,
    blocks: &[(Option<usize>, Vec<usize>)],
) -> Vec<(Option<String>, Vec<String>)> {
    blocks
        .iter()
        .map(|(root, nodes)| {
            (
                root.map(|id| node_text(&graph.nodes[node_of(graph, id)])),
                nodes
                    .iter()
                    .map(|&node| node_text(&graph.nodes[node]))
                    .collect(),
            )
        })
        .collect()
}

/// `getClasses`: the roots grouped by their sink, in root order. A balanced layout
/// has no roots, which gives a `None` entry and no classes.
fn get_classes(
    graph: &LGraph,
    bal: &AlignedLayout,
    progress: &mut ProgressMonitor,
    monitor: Monitor,
) -> Vec<(Option<usize>, Vec<usize>)> {
    let mut roots: Vec<Option<usize>> = Vec::new();
    for layer in &graph.layers {
        for &node in layer {
            let root = bal.root[graph.nodes[node].placement_id];
            if !roots.contains(&root) {
                roots.push(root);
            }
        }
    }
    let mut classes: IndexMap<Option<usize>, Vec<usize>> = IndexMap::new();
    for root in roots {
        match root {
            None => {
                progress.log(monitor, "There are no classes in a balanced layout.");
                break;
            }
            Some(id) => {
                let sink = bal.sink[id];
                classes.entry(Some(sink)).or_default().push(id);
            }
        }
    }
    classes.into_iter().collect()
}

fn classes_text(
    graph: &LGraph,
    classes: &[(Option<usize>, Vec<usize>)],
) -> Vec<(Option<String>, Vec<String>)> {
    classes
        .iter()
        .map(|(sink, roots)| {
            (
                sink.map(|id| node_text(&graph.nodes[node_of(graph, id)])),
                roots
                    .iter()
                    .map(|&id| node_text(&graph.nodes[node_of(graph, id)]))
                    .collect(),
            )
        })
        .collect()
}

/// `createBalancedLayout`: averages the four candidate layouts, shifted onto the
/// narrowest one.
fn create_balanced_layout(
    graph: &LGraph,
    ni: &NeighborhoodInformation,
    layouts: &[AlignedLayout],
) -> AlignedLayout {
    let count = layouts.len();
    let mut balanced = AlignedLayout::new(ni.node_count, None, None);
    let mut min = vec![f64::from(i32::MAX); count];
    let mut max = vec![f64::from(i32::MIN); count];
    let mut min_width_layout = 0;
    let widths: Vec<f64> = layouts
        .iter()
        .map(|layout| layout.layout_size(graph, ni))
        .collect();
    for index in 0..count {
        if widths[min_width_layout] > widths[index] {
            min_width_layout = index;
        }
        let bal = &layouts[index];
        for layer in &graph.layers {
            for &node in layer {
                let id = graph.nodes[node].placement_id;
                let node_pos_y = bal.y_of(id) + bal.inner_shift[id];
                min[index] = min[index].min(node_pos_y);
                max[index] = max[index].max(node_pos_y + graph.nodes[node].size.y);
            }
        }
    }
    let mut shift = vec![0.0; count];
    for index in 0..count {
        shift[index] = if layouts[index].vdir == Some(VDirection::Down) {
            min[min_width_layout] - min[index]
        } else {
            max[min_width_layout] - max[index]
        };
    }
    let mut calculated = vec![0.0; count];
    for layer in &graph.layers {
        for &node in layer {
            let id = graph.nodes[node].placement_id;
            for index in 0..count {
                calculated[index] =
                    layouts[index].y_of(id) + layouts[index].inner_shift[id] + shift[index];
            }
            // Java's Arrays.sort on doubles: ascending, with NaN last; there are no NaNs here.
            calculated.sort_by(|a, b| a.total_cmp(b));
            balanced.y[id] = Some((calculated[1] + calculated[2]) / 2.0);
            balanced.inner_shift[id] = 0.0;
        }
    }
    balanced
}

/// `checkOrderConstraint`: whether the nodes of each layer fit one after another.
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
            let top = bal.y_of(id) + bal.inner_shift[id] - current.margin.top;
            let bottom =
                bal.y_of(id) + bal.inner_shift[id] + current.size.y + current.margin.bottom;
            if top > pos && bottom > pos {
                previous = Some(node);
                pos = bal.y_of(id) + bal.inner_shift[id] + current.size.y + current.margin.bottom;
            } else {
                feasible = false;
                if progress.is_logging_enabled() {
                    let before = previous
                        .map(|p| node_text(&graph.nodes[p]))
                        .unwrap_or_default();
                    progress.log(
                        monitor,
                        format!(
                            "bk node placement breaks on {} which should have been after {}",
                            node_text(current),
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

/// `BKCompactor.horizontalCompaction`: assigns the y positions of the blocks.
fn horizontal_compaction(
    graph: &LGraph,
    ni: &NeighborhoodInformation,
    bal: &mut AlignedLayout,
    threshold: Threshold,
) -> Result<(), LayoutError> {
    for layer in &graph.layers {
        for &node in layer {
            let id = graph.nodes[node].placement_id;
            bal.sink[id] = id;
            bal.shift[id] = if bal.vdir == Some(VDirection::Up) {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            };
        }
    }
    let mut classes = ClassGraph::default();
    let mut layers: Vec<&Vec<usize>> = graph.layers.iter().collect();
    if bal.hdir == Some(HDirection::Left) {
        layers.reverse();
    }
    for y in bal.y.iter_mut() {
        *y = None;
    }
    for layer in &layers {
        let mut nodes: Vec<usize> = (*layer).clone();
        if bal.vdir == Some(VDirection::Up) {
            nodes.reverse();
        }
        for node in nodes {
            let id = graph.nodes[node].placement_id;
            if bal.root_of(id) == id {
                place_block(graph, ni, bal, threshold, &mut classes, id)?;
            }
        }
    }
    place_classes(bal, &mut classes);
    for layer in &layers {
        for &node in layer.iter() {
            let id = graph.nodes[node].placement_id;
            let root = bal.root_of(id);
            bal.y[id] = bal.y[root];
            if id == root {
                let sink_shift = bal.shift[bal.sink[id]];
                let shifts = (bal.vdir == Some(VDirection::Up) && sink_shift > f64::NEG_INFINITY)
                    || (bal.vdir == Some(VDirection::Down) && sink_shift < f64::INFINITY);
                if shifts {
                    bal.y[id] = bal.y[id].map(|value| value + sink_shift);
                }
            }
        }
    }
    threshold.post_process()?;
    Ok(())
}

/// `placeBlock`: places the block with the given root, after the blocks it depends on.
fn place_block(
    graph: &LGraph,
    ni: &NeighborhoodInformation,
    bal: &mut AlignedLayout,
    threshold: Threshold,
    classes: &mut ClassGraph,
    root: usize,
) -> Result<(), LayoutError> {
    if bal.y[root].is_some() {
        return Ok(());
    }
    let mut is_initial_assignment = true;
    bal.y[root] = Some(0.0);
    let mut current = root;
    let mut thresh = if bal.vdir == Some(VDirection::Down) {
        f64::NEG_INFINITY
    } else {
        f64::INFINITY
    };
    loop {
        let current_index = ni.node_index[current];
        let current_node = &graph.nodes[ni.node_of_id[current]];
        let layer_len = graph.layers[ni.layer_index[current_node.layer.unwrap_or(0)]].len();
        let has_neighbor = (bal.vdir == Some(VDirection::Down) && current_index > 0)
            || (bal.vdir == Some(VDirection::Up) && current_index + 1 < layer_len);
        if has_neighbor {
            let layer = &graph.layers[ni.layer_index[current_node.layer.unwrap_or(0)]];
            let neighbor_node = if bal.vdir == Some(VDirection::Up) {
                layer[current_index + 1]
            } else {
                layer[current_index - 1]
            };
            let neighbor = graph.nodes[neighbor_node].placement_id;
            let neighbor_root = bal.root_of(neighbor);
            place_block(graph, ni, bal, threshold, classes, neighbor_root)?;
            thresh = threshold.calculate(bal, thresh, root, current);
            if bal.sink[root] == root {
                bal.sink[root] = bal.sink[neighbor_root];
            }
            if bal.sink[root] == bal.sink[neighbor_root] {
                let spacing = spacings::vertical(graph, current_node, &graph.nodes[neighbor_node]);
                let neighbor_node_ref = &graph.nodes[neighbor_node];
                if bal.vdir == Some(VDirection::Up) {
                    let current_block_position = bal.y_of(root);
                    let new_position = bal.y_of(neighbor_root) + bal.inner_shift[neighbor]
                        - neighbor_node_ref.margin.top
                        - spacing
                        - current_node.margin.bottom
                        - current_node.size.y
                        - bal.inner_shift[current];
                    bal.y[root] = Some(if is_initial_assignment {
                        is_initial_assignment = false;
                        new_position.min(thresh)
                    } else {
                        current_block_position.min(new_position.min(thresh))
                    });
                } else {
                    let current_block_position = bal.y_of(root);
                    let new_position = bal.y_of(neighbor_root)
                        + bal.inner_shift[neighbor]
                        + neighbor_node_ref.size.y
                        + neighbor_node_ref.margin.bottom
                        + spacing
                        + current_node.margin.top
                        - bal.inner_shift[current];
                    bal.y[root] = Some(if is_initial_assignment {
                        is_initial_assignment = false;
                        new_position.max(thresh)
                    } else {
                        current_block_position.max(new_position.max(thresh))
                    });
                }
            } else {
                // Two different sinks: the classes are separated by the node spacing.
                let spacing = graph
                    .properties
                    .double_of(spacings::SPACING_NODE_NODE)
                    .unwrap_or(0.0);
                let sink_node = bal.sink[root];
                let neighbor_sink = bal.sink[neighbor_root];
                classes.add_edge(
                    sink_node,
                    neighbor_sink,
                    spacing,
                    bal,
                    graph,
                    ni,
                    current,
                    neighbor_node,
                    current_node,
                );
            }
        } else {
            thresh = threshold.calculate(bal, thresh, root, current);
        }
        current = bal.align[current];
        if current == root {
            break;
        }
    }
    threshold.finish_block(root);
    Ok(())
}

/// `ClassNode`s and their `ClassEdge`s, keyed by sink in creation order.
#[derive(Debug, Default)]
struct ClassGraph {
    nodes: IndexMap<usize, ClassNode>,
}

#[derive(Debug, Default, Clone)]
struct ClassNode {
    class_shift: Option<f64>,
    outgoing: Vec<(usize, f64)>,
    indegree: usize,
}

impl ClassGraph {
    /// `ClassNode.addEdge` for the separation a pair of blocks needs.
    #[allow(clippy::too_many_arguments)]
    fn add_edge(
        &mut self,
        sink: usize,
        neighbor_sink: usize,
        spacing: f64,
        bal: &AlignedLayout,
        graph: &LGraph,
        ni: &NeighborhoodInformation,
        current: usize,
        neighbor_node: usize,
        current_node: &LNode,
    ) {
        let neighbor = &graph.nodes[neighbor_node];
        let neighbor_id = neighbor.placement_id;
        let separation = if bal.vdir == Some(VDirection::Up) {
            bal.y_of(bal.root_of(current))
                + bal.inner_shift[current]
                + current_node.size.y
                + current_node.margin.bottom
                + spacing
                - (bal.y_of(bal.root_of(neighbor_id)) + bal.inner_shift[neighbor_id]
                    - neighbor.margin.top)
        } else {
            bal.y_of(bal.root_of(current)) + bal.inner_shift[current]
                - current_node.margin.top
                - bal.y_of(bal.root_of(neighbor_id))
                - bal.inner_shift[neighbor_id]
                - neighbor.size.y
                - neighbor.margin.bottom
                - spacing
        };
        let _ = ni;
        self.nodes.entry(sink).or_default();
        self.nodes.entry(neighbor_sink).or_default();
        if let Some(source) = self.nodes.get_mut(&sink) {
            source.outgoing.push((neighbor_sink, separation));
        }
        if let Some(target) = self.nodes.get_mut(&neighbor_sink) {
            target.indegree += 1;
        }
    }
}

/// `placeClasses`: shifts the blocks of each class, from the classes with no incoming edge.
fn place_classes(bal: &mut AlignedLayout, classes: &mut ClassGraph) {
    let mut queue: VecDeque<usize> = VecDeque::new();
    for (&sink, node) in classes.nodes.iter() {
        if node.indegree == 0 {
            queue.push_back(sink);
        }
    }
    while let Some(sink) = queue.pop_front() {
        let shift = classes
            .nodes
            .get(&sink)
            .and_then(|node| node.class_shift)
            .unwrap_or(0.0);
        if let Some(node) = classes.nodes.get_mut(&sink) {
            node.class_shift = Some(shift);
        }
        let outgoing = classes
            .nodes
            .get(&sink)
            .map(|node| node.outgoing.clone())
            .unwrap_or_default();
        for (target_sink, separation) in outgoing {
            let vdir = bal.vdir;
            if let Some(target) = classes.nodes.get_mut(&target_sink) {
                target.class_shift = Some(match target.class_shift {
                    None => shift + separation,
                    Some(current) if vdir == Some(VDirection::Down) => {
                        current.min(shift + separation)
                    }
                    Some(current) => current.max(shift + separation),
                });
                target.indegree -= 1;
                if target.indegree == 0 {
                    queue.push_back(target_sink);
                }
            }
        }
    }
    for (&sink, node) in classes.nodes.iter() {
        bal.shift[sink] = node.class_shift.unwrap_or(0.0);
    }
}

/// `ThresholdStrategy`: the null strategy, or the simple one for straightened edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Threshold {
    Null,
    Simple,
}

impl Threshold {
    fn for_graph(graph: &LGraph) -> Threshold {
        match graph
            .properties
            .enum_of::<EdgeStraighteningStrategy>(options::NODE_PLACEMENT_BK_EDGE_STRAIGHTENING)
        {
            Some(EdgeStraighteningStrategy::IMPROVE_STRAIGHTNESS) => Threshold::Simple,
            _ => Threshold::Null,
        }
    }

    /// `calculateThreshold`. Without edges the simple strategy finds no bound, which is
    /// the same infinity as the null strategy.
    fn calculate(self, bal: &AlignedLayout, _old: f64, _root: usize, _current: usize) -> f64 {
        match (self, bal.vdir) {
            (_, Some(VDirection::Up)) => f64::INFINITY,
            _ => f64::NEG_INFINITY,
        }
    }

    fn finish_block(self, _root: usize) {}

    fn post_process(self) -> Result<(), LayoutError> {
        Ok(())
    }
}

/// `PropertyHolder` import kept for the option reads above.
#[allow(dead_code)]
fn _property_holder_type(_: &PropertyHolder) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_text_matches_java_map_to_string() {
        let text = map_text(&[(Some("n_root.a".to_string()), vec!["n_root.a".to_string()])]);
        assert_eq!(text, "{n_root.a=[n_root.a]}");
        let balanced = map_text(&[(None, vec!["n_root.a".to_string(), "n_root.b".to_string()])]);
        assert_eq!(balanced, "{null=[n_root.a, n_root.b]}");
    }
}
