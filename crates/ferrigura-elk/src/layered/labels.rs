//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/LabelDummySwitcher.java,
//!   LabelSideSelector.java and LabelDummyRemover.java; LGraphUtil.findMaxNonDummyNodeWidth
//!   (graph/LGraphUtil.java) and EndLabelPreprocessor.gatherLabels(LPort), which LabelSideSelector calls
//!   (the end label preprocessor itself is not ported).
//!
//! The label dummies of the layered algorithm, once layering has run. `label_dummy_switching`
//! moves each one into the layer its placement strategy picks, `label_side_selection` decides the
//! side its labels go on, and `label_dummy_removal` puts the labels back on their edges and removes
//! the dummies.
//!
//! A layer's id is its position in `LGraph::layers` (`Layer.id` after `assignIdsToLayers`), and
//! `LNode::layer` names that position. The Java `EnumMap`s are arrays in ordinal order here. No Java
//! `HashMap` or `HashSet` appears in these sources, so no `IndexMap` is needed. Java `float` does not
//! occur in these sources; every value is a `double`, kept as `f64`.

use std::cmp::Ordering;
use std::collections::VecDeque;

use crate::core::data::JavaEnum;
use crate::core::data::OptionValue;
use crate::core::data::enums::{
    Alignment, CenterEdgeLabelPlacementStrategy, Direction, EdgeLabelPlacement,
    EdgeLabelSideSelection, EdgeRouting, NodeType, PortSide,
};
use crate::core::math::KVector;
use crate::java::sort;
use crate::layered::LayoutError;
use crate::layered::dummies;
use crate::layered::lgraph::{LGraph, LLabel, PortType};
use crate::layered::options;

/// `org.eclipse.elk.core.options.LabelSide`. Written here because the generated enums do not
/// include it. The Java default is `UNKNOWN`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabelSide {
    #[default]
    UNKNOWN,
    ABOVE,
    BELOW,
    INLINE,
}

impl LabelSide {
    /// `LabelSide.opposite()`: above and below trade places, while inline and unknown stay.
    pub fn opposite(self) -> LabelSide {
        match self {
            LabelSide::ABOVE => LabelSide::BELOW,
            LabelSide::BELOW => LabelSide::ABOVE,
            LabelSide::INLINE => LabelSide::INLINE,
            LabelSide::UNKNOWN => LabelSide::UNKNOWN,
        }
    }
}

impl JavaEnum for LabelSide {
    fn from_java_text(text: &str) -> Option<Self> {
        match text {
            "UNKNOWN" => Some(LabelSide::UNKNOWN),
            "ABOVE" => Some(LabelSide::ABOVE),
            "BELOW" => Some(LabelSide::BELOW),
            "INLINE" => Some(LabelSide::INLINE),
            _ => None,
        }
    }

    fn java_name(self) -> &'static str {
        match self {
            LabelSide::UNKNOWN => "UNKNOWN",
            LabelSide::ABOVE => "ABOVE",
            LabelSide::BELOW => "BELOW",
            LabelSide::INLINE => "INLINE",
        }
    }
}

/// The id under which an edge label holds `InternalProperties.LABEL_SIDE`. `LLabel` has no field
/// for it: its literals elsewhere do not take one, so it lives in the label's properties, as it
/// does in Java's property map.
const LABEL_SIDE: &str = "labelSide";

/// The id under which an edge label holds `LabelDummySwitcher.INCLUDE_LABEL`, in the same way.
const INCLUDE_LABEL: &str = "edgelabelcenterednessanalysis.includelabel";

/// `label.setProperty(InternalProperties.LABEL_SIDE, side)` for an edge label.
fn set_label_side(label: &mut LLabel, side: LabelSide) {
    label.properties.set(
        LABEL_SIDE,
        Some(OptionValue::Enum {
            class: "LabelSide",
            name: side.java_name().to_string(),
        }),
    );
}

type Placement = CenterEdgeLabelPlacementStrategy;

/// `CenterEdgeLabelPlacementStrategy.values()` in ordinal order, which is also the order of the
/// Java `EnumMap` the switcher iterates.
const PLACEMENTS: [Placement; 6] = [
    Placement::MEDIAN_LAYER,
    Placement::TAIL_LAYER,
    Placement::HEAD_LAYER,
    Placement::SPACE_EFFICIENT_LAYER,
    Placement::WIDEST_LAYER,
    Placement::CENTER_LAYER,
];

/// The `NO_INCIDENT_EDGE_THICKNESS` sentinel of `EndLabelPreprocessor`.
const NO_INCIDENT_EDGE_THICKNESS: f64 = -1.0;

/// `CenterEdgeLabelPlacementStrategy.usesLabelSizeInformation()`.
fn uses_label_size_information(placement: Placement) -> bool {
    matches!(
        placement,
        Placement::WIDEST_LAYER | Placement::CENTER_LAYER | Placement::SPACE_EFFICIENT_LAYER
    )
}

/// `Math.max(double, double)`: NaN wins, and `0.0` is greater than `-0.0`.
fn java_max(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        return a;
    }
    if a == 0.0 && b == 0.0 && a.is_sign_negative() {
        return b;
    }
    if a >= b { a } else { b }
}

/// `Double.compare(double, double)`: `-0.0` is below `0.0`, and NaN equals NaN and is above
/// every other value.
fn java_double_compare(a: f64, b: f64) -> Ordering {
    if a < b {
        Ordering::Less
    } else if a > b {
        Ordering::Greater
    } else {
        canonical_bits(a).cmp(&canonical_bits(b))
    }
}

/// `Double.doubleToLongBits`: every NaN has the same bits.
fn canonical_bits(value: f64) -> i64 {
    if value.is_nan() {
        f64::NAN.to_bits() as i64
    } else {
        value.to_bits() as i64
    }
}

/// A value that the Java code reads without a null check. A missing one is a malformed graph.
fn required<T>(value: Option<T>, what: &str) -> Result<T, LayoutError> {
    value.ok_or_else(|| LayoutError::Format(format!("{what} is missing")))
}

fn graph_double(graph: &LGraph, id: &str) -> Result<f64, LayoutError> {
    required(graph.properties.double_of(id), id)
}

fn layer_id(graph: &LGraph, node: usize) -> Result<usize, LayoutError> {
    required(graph.nodes[node].layer, "the layer of a node")
}

/// `LNode.getIncomingEdges().iterator().next()`: the first incoming edge, port by port.
fn first_incoming(graph: &LGraph, node: usize) -> Result<usize, LayoutError> {
    let edge = graph.nodes[node]
        .ports
        .iter()
        .find_map(|&port| graph.ports[port].incoming.first().copied());
    required(edge, "an incoming edge of a node")
}

/// `LNode.getOutgoingEdges().iterator().next()`: the first outgoing edge, port by port.
fn first_outgoing(graph: &LGraph, node: usize) -> Result<usize, LayoutError> {
    let edge = graph.nodes[node]
        .ports
        .iter()
        .find_map(|&port| graph.ports[port].outgoing.first().copied());
    required(edge, "an outgoing edge of a node")
}

/// `LNode.getOutgoingEdges()`, port by port.
fn outgoing_edges(graph: &LGraph, node: usize) -> Vec<usize> {
    graph.nodes[node]
        .ports
        .iter()
        .flat_map(|&port| graph.ports[port].outgoing.iter().copied())
        .collect()
}

/// Every node of every layer, layer by layer.
fn layer_nodes(graph: &LGraph) -> Vec<usize> {
    graph.layers.iter().flatten().copied().collect()
}

/// `LNode.getPorts(PortType)`: the first port with incoming edges (for `INPUT`) or outgoing edges
/// (for `OUTPUT`). Java's `PortType` predicates test the edges, not the port's side.
fn port_with_edges(graph: &LGraph, node: usize, port_type: PortType) -> Result<usize, LayoutError> {
    let port = graph.nodes[node]
        .ports
        .iter()
        .copied()
        .find(|&port| match port_type {
            PortType::INPUT => !graph.ports[port].incoming.is_empty(),
            PortType::OUTPUT => !graph.ports[port].outgoing.is_empty(),
        });
    required(port, "a port with edges of a node")
}

/// `LNode.isInlineEdgeLabel()`: a label dummy whose represented labels are all inline. An empty
/// list counts as inline, as `allMatch` does.
fn is_inline_edge_label(graph: &LGraph, node: usize) -> bool {
    graph.nodes[node].node_type == NodeType::LABEL
        && graph.nodes[node]
            .represented_labels
            .iter()
            .all(|label| label.properties.bool_of(options::EDGE_LABELS_INLINE))
}

/// `LEdge.setSource(port)`: the edge leaves its old source's list and joins the end of the new one's.
fn set_edge_source(graph: &mut LGraph, edge: usize, port: usize) {
    let old = graph.edges[edge].source;
    if let Some(position) = graph.ports[old]
        .outgoing
        .iter()
        .position(|&found| found == edge)
    {
        graph.ports[old].outgoing.remove(position);
    }
    graph.edges[edge].source = port;
    graph.ports[port].outgoing.push(edge);
}

/// `LNode.setLayer(index, layer)`: the node leaves its layer and joins `layer` at `index`.
fn set_layer(
    graph: &mut LGraph,
    node: usize,
    index: usize,
    layer: usize,
) -> Result<(), LayoutError> {
    if index > graph.layers[layer].len() {
        return Err(LayoutError::Format(
            "index must be between 0 and the layer's node count".to_string(),
        ));
    }
    if let Some(old) = graph.nodes[node].layer
        && let Some(position) = graph.layers[old].iter().position(|&member| member == node)
    {
        graph.layers[old].remove(position);
    }
    graph.nodes[node].layer = Some(layer);
    graph.layers[layer].insert(index, node);
    Ok(())
}

/// `assignIdsToLayers`: each layer's id is its position, and its nodes refer to that position.
fn assign_ids_to_layers(graph: &mut LGraph) {
    let members: Vec<(usize, usize)> = graph
        .layers
        .iter()
        .enumerate()
        .flat_map(|(id, layer)| layer.iter().map(move |&node| (node, id)))
        .collect();
    for (node, id) in members {
        graph.nodes[node].layer = Some(id);
    }
}

/// A label held by an edge: `LGraph::edges[edge].labels[index]`, the element of a Java list that
/// refers to an `LLabel` the edge owns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LabelRef {
    edge: usize,
    index: usize,
}

/// `LabelDummySwitcher.process`. Each label dummy is moved into the layer its placement
/// strategy picks. Strategies that need no layer widths run first, since they may change the
/// widths that the others read.
pub fn label_dummy_switching(graph: &mut LGraph) -> Result<(), LayoutError> {
    let default_placement = required(
        graph
            .properties
            .enum_of::<Placement>(options::EDGE_LABELS_CENTER_LABEL_PLACEMENT_STRATEGY),
        "the center edge label placement strategy",
    )?;
    assign_ids_to_layers(graph);
    let mut infos = gather_label_dummy_infos(graph, default_placement)?;
    let mut switcher = LabelDummySwitcher {
        layer_widths: vec![0.0; graph.layers.len()],
    };
    let any_width_based = PLACEMENTS.iter().any(|&placement| {
        uses_label_size_information(placement) && !infos[placement as usize].is_empty()
    });
    if any_width_based {
        switcher.calculate_layer_widths(graph);
    }
    for placement in PLACEMENTS
        .into_iter()
        .filter(|&placement| !uses_label_size_information(placement))
    {
        let list = std::mem::take(&mut infos[placement as usize]);
        switcher.process_strategy(graph, list)?;
    }
    for placement in PLACEMENTS
        .into_iter()
        .filter(|&placement| uses_label_size_information(placement))
    {
        let list = std::mem::take(&mut infos[placement as usize]);
        switcher.process_strategy(graph, list)?;
    }
    Ok(())
}

/// `LabelDummySwitcher.gatherLabelDummyInfos`: every label dummy, in layer order, by strategy.
fn gather_label_dummy_infos(
    graph: &LGraph,
    default_placement: Placement,
) -> Result<[Vec<LabelDummyInfo>; 6], LayoutError> {
    let mut infos: [Vec<LabelDummyInfo>; 6] = Default::default();
    for node in layer_nodes(graph) {
        if graph.nodes[node].node_type == NodeType::LABEL {
            let info = LabelDummyInfo::new(graph, node, default_placement)?;
            infos[info.placement_strategy as usize].push(info);
        }
    }
    Ok(infos)
}

/// `LabelDummySwitcher.LabelDummyInfo`: a label dummy with the long edge dummies to its left and
/// right. The lists and the end layers are read once, when the info is made.
#[derive(Debug, Clone)]
struct LabelDummyInfo {
    /// `labelDummy`: the label dummy node.
    label_dummy: usize,
    /// `placementStrategy`: the strategy of the first label that overrides the graph's default.
    placement_strategy: Placement,
    /// `leftLongEdgeDummies`: the long edge dummies left of the label dummy, in layer order.
    left_long_edge_dummies: Vec<usize>,
    /// `rightLongEdgeDummies`: the long edge dummies right of the label dummy, in layer order.
    right_long_edge_dummies: Vec<usize>,
    /// `leftmostLayerId`: the layer of the leftmost dummy of the edge.
    leftmost_layer_id: usize,
    /// `rightmostLayerId`: the layer of the rightmost dummy of the edge.
    rightmost_layer_id: usize,
}

impl LabelDummyInfo {
    fn new(
        graph: &LGraph,
        label_dummy: usize,
        default_placement: Placement,
    ) -> Result<Self, LayoutError> {
        let left = gather_left_long_edge_dummies(graph, label_dummy)?;
        let right = gather_right_long_edge_dummies(graph, label_dummy)?;
        let leftmost_layer_id = match left.first() {
            Some(&first) => layer_id(graph, first)?,
            None => layer_id(graph, label_dummy)?,
        };
        let rightmost_layer_id = match right.last() {
            Some(&last) => layer_id(graph, last)?,
            None => layer_id(graph, label_dummy)?,
        };
        let mut placement_strategy = default_placement;
        for label in &graph.nodes[label_dummy].represented_labels {
            if label
                .properties
                .has(options::EDGE_LABELS_CENTER_LABEL_PLACEMENT_STRATEGY)
            {
                if let Some(placement) = label
                    .properties
                    .enum_of::<Placement>(options::EDGE_LABELS_CENTER_LABEL_PLACEMENT_STRATEGY)
                {
                    placement_strategy = placement;
                }
                break;
            }
        }
        Ok(LabelDummyInfo {
            label_dummy,
            placement_strategy,
            left_long_edge_dummies: left,
            right_long_edge_dummies: right,
            leftmost_layer_id,
            rightmost_layer_id,
        })
    }

    /// `totalDummyCount()`: the long edge and label dummies along the edge.
    fn total_dummy_count(&self) -> usize {
        self.rightmost_layer_id + 1 - self.leftmost_layer_id
    }

    /// `ithDummyNode(i)`: the i-th dummy of the edge, counting from its leftmost one.
    fn ith_dummy_node(&self, index: usize) -> Result<usize, LayoutError> {
        let left = self.left_long_edge_dummies.len();
        if index < left {
            Ok(self.left_long_edge_dummies[index])
        } else if index == left {
            Ok(self.label_dummy)
        } else {
            required(
                self.right_long_edge_dummies.get(index - left - 1).copied(),
                "a dummy of the label's edge",
            )
        }
    }
}

/// `gatherLeftLongEdgeDummies`: the long edge dummies before the label dummy in its chain, in
/// layer order.
fn gather_left_long_edge_dummies(
    graph: &LGraph,
    label_dummy: usize,
) -> Result<Vec<usize>, LayoutError> {
    let mut dummies = Vec::new();
    let mut source = label_dummy;
    loop {
        let edge = first_incoming(graph, source)?;
        source = graph.ports[graph.edges[edge].source].node;
        if graph.nodes[source].node_type != NodeType::LONG_EDGE {
            break;
        }
        dummies.push(source);
    }
    dummies.reverse();
    Ok(dummies)
}

/// `gatherRightLongEdgeDummies`: the long edge dummies after the label dummy in its chain, in
/// layer order.
fn gather_right_long_edge_dummies(
    graph: &LGraph,
    label_dummy: usize,
) -> Result<Vec<usize>, LayoutError> {
    let mut dummies = Vec::new();
    let mut target = label_dummy;
    loop {
        let edge = first_outgoing(graph, target)?;
        target = graph.ports[graph.edges[edge].target].node;
        if graph.nodes[target].node_type != NodeType::LONG_EDGE {
            break;
        }
        dummies.push(target);
    }
    Ok(dummies)
}

/// `LabelDummySwitcher`: the processor and its `layerWidths`, which the strategies read and change.
struct LabelDummySwitcher {
    /// `layerWidths`: the width of each layer by layer id, as the widest normal node in it.
    layer_widths: Vec<f64>,
}

impl LabelDummySwitcher {
    /// `calculateLayerWidths`.
    fn calculate_layer_widths(&mut self, graph: &LGraph) {
        for (layer, width) in self.layer_widths.iter_mut().enumerate() {
            *width = find_max_non_dummy_node_width(graph, layer);
        }
    }

    /// `processStrategy`. Space efficient placement decides all of its label dummies at once; the
    /// other strategies decide one label dummy at a time.
    fn process_strategy(
        &mut self,
        graph: &mut LGraph,
        infos: Vec<LabelDummyInfo>,
    ) -> Result<(), LayoutError> {
        let Some(first) = infos.first() else {
            return Ok(());
        };
        if first.placement_strategy == Placement::SPACE_EFFICIENT_LAYER {
            return self.compute_space_efficient_assignment(graph, infos);
        }
        for info in &infos {
            match info.placement_strategy {
                Placement::CENTER_LAYER => {
                    let target = self.find_center_layer_target_id(graph, info)?;
                    self.assign_layer(graph, info, target)?;
                }
                Placement::MEDIAN_LAYER => {
                    let target = self.find_median_layer_target_id(info);
                    self.assign_layer(graph, info, target)?;
                }
                Placement::WIDEST_LAYER => {
                    let target = self.find_widest_layer_target_id(info);
                    self.assign_layer(graph, info, target)?;
                }
                Placement::HEAD_LAYER => {
                    set_end_layer_node_alignment(graph, info)?;
                    let target = find_end_layer_target_id(graph, info, true)?;
                    self.assign_layer(graph, info, target)?;
                }
                Placement::TAIL_LAYER => {
                    set_end_layer_node_alignment(graph, info)?;
                    let target = find_end_layer_target_id(graph, info, false)?;
                    self.assign_layer(graph, info, target)?;
                }
                // The Java switch has no case for this strategy. The branch above takes it.
                Placement::SPACE_EFFICIENT_LAYER => {}
            }
            update_long_edge_source_label_dummy_info(graph, info)?;
        }
        Ok(())
    }

    /// `findWidestLayerTargetId`: the widest of the layers the edge spans, the leftmost on ties.
    fn find_widest_layer_target_id(&self, info: &LabelDummyInfo) -> usize {
        let mut widest = info.leftmost_layer_id;
        for index in (info.leftmost_layer_id + 1)..=info.rightmost_layer_id {
            if self.layer_widths[index] > self.layer_widths[widest] {
                widest = index;
            }
        }
        widest
    }

    /// `findMedianLayerTargetId`: the lower median of the layers the edge spans.
    fn find_median_layer_target_id(&self, info: &LabelDummyInfo) -> usize {
        let layers = info.total_dummy_count();
        // Java's `(layers - 1) / 2` is zero for an empty span as well.
        let lower_median = layers.saturating_sub(1) / 2;
        info.leftmost_layer_id + lower_median
    }

    /// `findCenterLayerTargetId`: the first layer whose accumulated width reaches half the edge.
    fn find_center_layer_target_id(
        &self,
        graph: &LGraph,
        info: &LabelDummyInfo,
    ) -> Result<usize, LayoutError> {
        let sums = self.compute_layer_width_sums(graph, info)?;
        let Some(&total) = sums.last() else {
            return Err(LayoutError::Format(
                "the layer width sums of a label dummy".to_string(),
            ));
        };
        let threshold = total / 2.0;
        for (index, &sum) in sums.iter().enumerate() {
            if sum >= threshold {
                return Ok(info.leftmost_layer_id + index);
            }
        }
        // Not reached in Java either (`assert false`): the leftmost long edge dummy's layer.
        Ok(info.leftmost_layer_id + info.left_long_edge_dummies.len())
    }

    /// `computeLayerWidthSums`: entry i is the combined width of the layers up to the i-th dummy
    /// of the edge, estimated from the layer widths and a guess at the spacing between layers.
    fn compute_layer_width_sums(
        &self,
        graph: &LGraph,
        info: &LabelDummyInfo,
    ) -> Result<Vec<f64>, LayoutError> {
        let edge_node_spacing =
            graph_double(graph, options::SPACING_EDGE_NODE_BETWEEN_LAYERS)? * 2.0;
        let node_node_spacing = graph_double(graph, options::SPACING_NODE_NODE_BETWEEN_LAYERS)?;
        let min_space_between_layers = java_max(edge_node_spacing, node_node_spacing);

        let mut sums = vec![0.0; info.total_dummy_count()];
        let mut current_sum = -min_space_between_layers;
        let chain = info
            .left_long_edge_dummies
            .iter()
            .chain(std::iter::once(&info.label_dummy))
            .chain(&info.right_long_edge_dummies);
        for (index, &dummy) in chain.enumerate() {
            current_sum += self.layer_widths[layer_id(graph, dummy)?] + min_space_between_layers;
            *sums.get_mut(index).ok_or_else(|| {
                LayoutError::Format("the layer width sums of a label dummy".to_string())
            })? = current_sum;
        }
        Ok(sums)
    }

    /// `assignLayer`: the label dummy takes the layer `target`, by swapping with the long edge
    /// dummy there. The layer's width grows to fit the label, and its labels are marked included.
    fn assign_layer(
        &mut self,
        graph: &mut LGraph,
        info: &LabelDummyInfo,
        target: usize,
    ) -> Result<(), LayoutError> {
        if target != info.leftmost_layer_id + info.left_long_edge_dummies.len() {
            let offset = target.checked_sub(info.leftmost_layer_id).ok_or_else(|| {
                LayoutError::Format("a target layer left of the label's edge".to_string())
            })?;
            let other = info.ith_dummy_node(offset)?;
            swap_nodes(graph, info.label_dummy, other)?;
        }

        let new_layer = layer_id(graph, info.label_dummy)?;
        self.layer_widths[new_layer] = java_max(
            self.layer_widths[new_layer],
            graph.nodes[info.label_dummy].size.x,
        );
        for label in &mut graph.nodes[info.label_dummy].represented_labels {
            label
                .properties
                .set(INCLUDE_LABEL, Some(OptionValue::Boolean(true)));
        }
        Ok(())
    }

    /// `computeSpaceEfficientAssignment`: the labels that fit a layer go there at once, and the
    /// rest, widest first, go to the layer that is widest once they are counted.
    fn compute_space_efficient_assignment(
        &mut self,
        graph: &mut LGraph,
        infos: Vec<LabelDummyInfo>,
    ) -> Result<(), LayoutError> {
        let mut non_trivial = self.perform_trivial_assignments(graph, infos)?;
        if non_trivial.is_empty() {
            return Ok(());
        }
        // Descending by label width. `Collections.sort` is the GWT merge sort, so the order of
        // equal labels is the one elkjs gives.
        {
            let graph: &LGraph = graph;
            sort::sort_by(&mut non_trivial, |a, b| {
                java_double_compare(
                    graph.nodes[b.label_dummy].size.x,
                    graph.nodes[a.label_dummy].size.x,
                )
            });
        }
        for (index, info) in non_trivial.iter().enumerate() {
            let target = self.find_potentially_widest_layer(graph, &non_trivial, index)?;
            self.assign_layer(graph, info, target)?;
        }
        Ok(())
    }

    /// `performTrivialAssignments`: the label dummies with one layer to choose from, or with a layer
    /// that is wide enough, are placed. The others are returned.
    fn perform_trivial_assignments(
        &mut self,
        graph: &mut LGraph,
        infos: Vec<LabelDummyInfo>,
    ) -> Result<Vec<LabelDummyInfo>, LayoutError> {
        let mut remaining = Vec::with_capacity(infos.len());
        for info in infos {
            if info.leftmost_layer_id == info.rightmost_layer_id {
                self.assign_layer(graph, &info, info.leftmost_layer_id)?;
            } else if !self.assign_to_wider_layer(graph, &info)? {
                remaining.push(info);
            }
        }
        Ok(remaining)
    }

    /// `assignToWiderLayer`: the first layer of the edge whose size (`Layer.getSize()`, not the
    /// width used by the strategies) fits the label. Returns whether one was found.
    fn assign_to_wider_layer(
        &mut self,
        graph: &mut LGraph,
        info: &LabelDummyInfo,
    ) -> Result<bool, LayoutError> {
        let dummy_width = graph.nodes[info.label_dummy].size.x;
        for layer in info.leftmost_layer_id..=info.rightmost_layer_id {
            if layer_size_x(graph, layer) >= dummy_width {
                self.assign_layer(graph, info, layer)?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// `findPotentiallyWidestLayer`: the layer of the label's edge with the largest potential width.
    /// `infos` is sorted by label width, descending.
    fn find_potentially_widest_layer(
        &self,
        graph: &LGraph,
        infos: &[LabelDummyInfo],
        label_index: usize,
    ) -> Result<usize, LayoutError> {
        let info = &infos[label_index];
        let label_width = graph.nodes[info.label_dummy].size.x;
        let mut widest_layer = info.leftmost_layer_id;
        let mut widest_width = 0.0;
        for layer in info.leftmost_layer_id..=info.rightmost_layer_id {
            // A layer that is wide enough already is the answer.
            if label_width <= self.layer_widths[layer] {
                return Ok(layer);
            }
            let mut potential_width = self.layer_widths[layer];
            // Java keeps the last unassigned label that covers this layer. Because `infos` is
            // sorted by descending width, that is the narrowest of them, not the widest; this is
            // ported as it is.
            let mut last_unassigned: Option<&LabelDummyInfo> = None;
            for other in &infos[label_index + 1..] {
                if other.leftmost_layer_id <= layer && other.rightmost_layer_id >= layer {
                    last_unassigned = Some(other);
                }
            }
            if let Some(other) = last_unassigned {
                potential_width = java_max(potential_width, graph.nodes[other.label_dummy].size.x);
            }
            if potential_width > widest_width {
                widest_layer = layer;
                widest_width = potential_width;
            }
        }
        Ok(widest_layer)
    }
}

/// `LGraphUtil.findMaxNonDummyNodeWidth(layer, false)`: zero in a vertical layout, else the widest
/// normal node of the layer.
fn find_max_non_dummy_node_width(graph: &LGraph, layer: usize) -> f64 {
    let direction = graph.properties.enum_of::<Direction>(options::DIRECTION);
    if matches!(direction, Some(Direction::UP | Direction::DOWN)) {
        return 0.0;
    }
    graph.layers[layer]
        .iter()
        .filter(|&&node| graph.nodes[node].node_type == NodeType::NORMAL)
        .fold(0.0, |max_width, &node| {
            java_max(max_width, graph.nodes[node].size.x)
        })
}

/// `Layer.getSize().x`. Layers without a computed size have none (zero).
fn layer_size_x(graph: &LGraph, layer: usize) -> f64 {
    graph.layer_sizes.get(layer).map_or(0.0, |size| size.x)
}

/// `findEndLayerTargetId`: the rightmost layer of the edge for the head, the leftmost for the tail,
/// swapped when the edge is reversed.
fn find_end_layer_target_id(
    graph: &LGraph,
    info: &LabelDummyInfo,
    head_layer: bool,
) -> Result<usize, LayoutError> {
    let reversed = is_part_of_reversed_edge(graph, info.label_dummy)?;
    if (head_layer && !reversed) || (!head_layer && reversed) {
        Ok(info.rightmost_layer_id)
    } else {
        Ok(info.leftmost_layer_id)
    }
}

/// `setEndLayerNodeAlignment`: the head or tail label dummy is aligned to the side of its node
/// that faces the label.
fn set_end_layer_node_alignment(
    graph: &mut LGraph,
    info: &LabelDummyInfo,
) -> Result<(), LayoutError> {
    let is_head_label = info.placement_strategy == Placement::HEAD_LAYER;
    let reversed = is_part_of_reversed_edge(graph, info.label_dummy)?;
    let alignment = if (is_head_label && !reversed) || (!is_head_label && reversed) {
        Alignment::RIGHT
    } else {
        Alignment::LEFT
    };
    graph.nodes[info.label_dummy].properties.set(
        options::ALIGNMENT,
        Some(OptionValue::Enum {
            class: "Alignment",
            name: alignment.java_name().to_string(),
        }),
    );
    Ok(())
}

/// `isPartOfReversedEdge`: whether the label dummy's incoming or outgoing edge is reversed.
fn is_part_of_reversed_edge(graph: &LGraph, label_dummy: usize) -> Result<bool, LayoutError> {
    let incoming = first_incoming(graph, label_dummy)?;
    let outgoing = first_outgoing(graph, label_dummy)?;
    Ok(graph.edges[incoming].reversed || graph.edges[outgoing].reversed)
}

/// `swapNodes`: the label dummy and the long edge dummy trade layers and positions in them, and
/// the edges that enter and leave each one are rerouted to the other.
fn swap_nodes(
    graph: &mut LGraph,
    label_dummy: usize,
    long_edge_dummy: usize,
) -> Result<(), LayoutError> {
    let layer1 = layer_id(graph, label_dummy)?;
    let layer2 = layer_id(graph, long_edge_dummy)?;
    let position1 = required(
        graph.layers[layer1]
            .iter()
            .position(|&member| member == label_dummy),
        "the label dummy in its layer",
    )?;
    let position2 = required(
        graph.layers[layer2]
            .iter()
            .position(|&member| member == long_edge_dummy),
        "the long edge dummy in its layer",
    )?;

    let input1 = port_with_edges(graph, label_dummy, PortType::INPUT)?;
    let output1 = port_with_edges(graph, label_dummy, PortType::OUTPUT)?;
    let input2 = port_with_edges(graph, long_edge_dummy, PortType::INPUT)?;
    let output2 = port_with_edges(graph, long_edge_dummy, PortType::OUTPUT)?;

    let incoming1 = graph.ports[input1].incoming.clone();
    let outgoing1 = graph.ports[output1].outgoing.clone();
    let incoming2 = graph.ports[input2].incoming.clone();
    let outgoing2 = graph.ports[output2].outgoing.clone();

    // The label dummy goes to the long edge dummy's place, and takes that dummy's edges.
    set_layer(graph, label_dummy, position2, layer2)?;
    for edge in incoming2 {
        graph.set_edge_target(edge, input1);
    }
    for edge in outgoing2 {
        set_edge_source(graph, edge, output1);
    }

    // And the long edge dummy goes to the label dummy's place, with the label dummy's edges.
    set_layer(graph, long_edge_dummy, position1, layer1)?;
    for edge in incoming1 {
        graph.set_edge_target(edge, input2);
    }
    for edge in outgoing1 {
        set_edge_source(graph, edge, output2);
    }
    Ok(())
}

/// `updateLongEdgeSourceLabelDummyInfo`: the long edge dummies before the label dummy are marked.
fn update_long_edge_source_label_dummy_info(
    graph: &mut LGraph,
    info: &LabelDummyInfo,
) -> Result<(), LayoutError> {
    let edge = first_incoming(graph, info.label_dummy)?;
    let mut current = graph.ports[graph.edges[edge].source].node;
    while graph.nodes[current].node_type == NodeType::LONG_EDGE {
        graph.nodes[current].long_edge_before_label_dummy = true;
        let edge = first_incoming(graph, current)?;
        current = graph.ports[graph.edges[edge].source].node;
    }
    Ok(())
}

/// `LabelSideSelector.process`: decides for each label on which side of its edge it goes.
pub fn label_side_selection(graph: &mut LGraph) -> Result<(), LayoutError> {
    let mode = required(
        graph
            .properties
            .enum_of::<EdgeLabelSideSelection>(options::EDGE_LABELS_SIDE_SELECTION),
        "the edge label side selection",
    )?;
    match mode {
        EdgeLabelSideSelection::ALWAYS_UP => same_side(graph, LabelSide::ABOVE),
        EdgeLabelSideSelection::ALWAYS_DOWN => same_side(graph, LabelSide::BELOW),
        EdgeLabelSideSelection::DIRECTION_UP => based_on_direction(graph, LabelSide::ABOVE),
        EdgeLabelSideSelection::DIRECTION_DOWN => based_on_direction(graph, LabelSide::BELOW),
        EdgeLabelSideSelection::SMART_UP => smart(graph, LabelSide::ABOVE),
        EdgeLabelSideSelection::SMART_DOWN => smart(graph, LabelSide::BELOW),
    }
}

/// `sameSide`: every label goes on `side`.
fn same_side(graph: &mut LGraph, side: LabelSide) -> Result<(), LayoutError> {
    for node in layer_nodes(graph) {
        if graph.nodes[node].node_type == NodeType::LABEL {
            apply_label_side_to_node(graph, node, side)?;
        }
        for edge in outgoing_edges(graph, node) {
            apply_label_side_to_edge(graph, edge, side);
        }
    }
    Ok(())
}

/// `basedOnDirection`: labels of rightward edges go on `side_for_rightward_edges`, others on the
/// opposite side.
fn based_on_direction(
    graph: &mut LGraph,
    side_for_rightward_edges: LabelSide,
) -> Result<(), LayoutError> {
    for node in layer_nodes(graph) {
        if graph.nodes[node].node_type == NodeType::LABEL {
            let side = if does_label_dummy_point_right(graph, node)? {
                side_for_rightward_edges
            } else {
                side_for_rightward_edges.opposite()
            };
            apply_label_side_to_node(graph, node, side)?;
        }
        for edge in outgoing_edges(graph, node) {
            let side = if does_edge_point_right(graph, edge) {
                side_for_rightward_edges
            } else {
                side_for_rightward_edges.opposite()
            };
            apply_label_side_to_edge(graph, edge, side);
        }
    }
    Ok(())
}

/// `doesEdgePointRight(LEdge)`: an edge points right unless it is reversed.
fn does_edge_point_right(graph: &LGraph, edge: usize) -> bool {
    !graph.edges[edge].reversed
}

/// `doesEdgePointRight(LNode)`: a label dummy's edge points right if either of its first incoming
/// or outgoing edges does.
fn does_label_dummy_point_right(graph: &LGraph, label_dummy: usize) -> Result<bool, LayoutError> {
    let incoming = first_incoming(graph, label_dummy)?;
    let outgoing = first_outgoing(graph, label_dummy)?;
    Ok(does_edge_point_right(graph, incoming) || does_edge_point_right(graph, outgoing))
}

/// `smart`: label sides follow the pattern of the dummies in each layer, and `default_side` is the
/// fallback.
fn smart(graph: &mut LGraph, default_side: LabelSide) -> Result<(), LayoutError> {
    // The dummies of consecutive runs collect here, across the layers, as in Java.
    let mut dummy_queue: VecDeque<usize> = VecDeque::new();
    for layer in graph.layers.clone() {
        // The run of dummies at the start of the layer is the topmost group, until a node that is
        // not a dummy ends it.
        let mut top_group = true;
        let mut label_dummies_in_queue = 0;
        for node in layer {
            match graph.nodes[node].node_type {
                NodeType::LABEL => {
                    label_dummies_in_queue += 1;
                    dummy_queue.push_back(node);
                }
                NodeType::LONG_EDGE => dummy_queue.push_back(node),
                node_type => {
                    if node_type == NodeType::NORMAL {
                        smart_for_regular_node(graph, node, default_side)?;
                    }
                    if !dummy_queue.is_empty() {
                        smart_for_consecutive_dummy_node_run(
                            graph,
                            &mut dummy_queue,
                            label_dummies_in_queue,
                            top_group,
                            false,
                            default_side,
                        )?;
                    }
                    top_group = false;
                    label_dummies_in_queue = 0;
                }
            }
        }
        if !dummy_queue.is_empty() {
            smart_for_consecutive_dummy_node_run(
                graph,
                &mut dummy_queue,
                label_dummies_in_queue,
                top_group,
                true,
                default_side,
            )?;
        }
    }
    Ok(())
}

/// `smartForConsecutiveDummyNodeRun`: labels for one run of dummies in a layer. The queue is
/// emptied afterwards.
fn smart_for_consecutive_dummy_node_run(
    graph: &mut LGraph,
    dummy_nodes: &mut VecDeque<usize>,
    label_dummy_count: usize,
    top_group: bool,
    bottom_group: bool,
    default_side: LabelSide,
) -> Result<(), LayoutError> {
    let len = dummy_nodes.len();
    let front = dummy_nodes.front().copied();
    let back = dummy_nodes.back().copied();
    let front_is_label = front.is_some_and(|node| graph.nodes[node].node_type == NodeType::LABEL);
    let back_is_label = back.is_some_and(|node| graph.nodes[node].node_type == NodeType::LABEL);

    if top_group && (!bottom_group || len > 1) && label_dummy_count == 1 && front_is_label {
        // The run is at the top of the layer and holds one label dummy, at its top: it goes above,
        // so its edge does not get too long.
        if let Some(node) = front {
            apply_label_side_to_node(graph, node, LabelSide::ABOVE)?;
        }
    } else if bottom_group && (!top_group || len > 1) && label_dummy_count == 1 && back_is_label {
        if let Some(node) = back {
            apply_label_side_to_node(graph, node, LabelSide::BELOW)?;
        }
    } else if len == 2 {
        // Two dummies: the first label goes above its edge and the second below.
        if let Some(first) = dummy_nodes.pop_front() {
            apply_label_side_to_node(graph, first, LabelSide::ABOVE)?;
        }
        if let Some(second) = dummy_nodes.pop_front() {
            apply_label_side_to_node(graph, second, LabelSide::BELOW)?;
        }
    } else {
        apply_for_dummy_node_run_with_simple_loops(graph, dummy_nodes, default_side)?;
    }
    dummy_nodes.clear();
    Ok(())
}

/// `applyForDummyNodeRunWithSimpleLoops`: the run is split where the long edge ends change, and
/// each piece gets `default_side`, except for pairs of label dummies on the same two nodes.
fn apply_for_dummy_node_run_with_simple_loops(
    graph: &mut LGraph,
    dummy_nodes: &VecDeque<usize>,
    default_side: LabelSide,
) -> Result<(), LayoutError> {
    let mut label_dummy_run: Vec<usize> = Vec::with_capacity(dummy_nodes.len());
    let mut prev_source: Option<usize> = None;
    let mut prev_target: Option<usize> = None;
    for &current in dummy_nodes {
        let curr_source = long_edge_end_node(graph, current, true);
        let curr_target = long_edge_end_node(graph, current, false);
        if prev_source != curr_source || prev_target != curr_target {
            apply_label_sides_to_label_dummy_run(graph, &mut label_dummy_run, default_side)?;
            prev_source = curr_source;
            prev_target = curr_target;
        }
        label_dummy_run.push(current);
    }
    apply_label_sides_to_label_dummy_run(graph, &mut label_dummy_run, default_side)
}

/// `getLongEdgeEndNode`: the node of the long edge's source or target port, if there is one.
fn long_edge_end_node(graph: &LGraph, node: usize, source: bool) -> Option<usize> {
    let port = if source {
        graph.nodes[node].long_edge_source
    } else {
        graph.nodes[node].long_edge_target
    };
    port.map(|port| graph.ports[port].node)
}

/// `applyLabelSidesToLabelDummyRun`: two dummies get above and below, others `default_side`. The
/// run is emptied afterwards.
fn apply_label_sides_to_label_dummy_run(
    graph: &mut LGraph,
    label_dummy_run: &mut Vec<usize>,
    default_side: LabelSide,
) -> Result<(), LayoutError> {
    if !label_dummy_run.is_empty() {
        if label_dummy_run.len() == 2 {
            apply_label_side_to_node(graph, label_dummy_run[0], LabelSide::ABOVE)?;
            apply_label_side_to_node(graph, label_dummy_run[1], LabelSide::BELOW)?;
        } else {
            for &dummy in label_dummy_run.iter() {
                apply_label_side_to_node(graph, dummy, default_side)?;
            }
        }
        label_dummy_run.clear();
    }
    Ok(())
}

/// `smartForRegularNode`: the end labels of a regular node's ports are decided per side. The queue
/// holds the end labels of the ports on one side that have edges, and a side with two such ports
/// gets opposite sides for them.
fn smart_for_regular_node(
    graph: &mut LGraph,
    node: usize,
    default_side: LabelSide,
) -> Result<(), LayoutError> {
    let mut end_label_queue: VecDeque<Vec<LabelRef>> =
        VecDeque::with_capacity(graph.nodes[node].ports.len());
    let mut current_port_side: Option<PortSide> = None;
    for port in graph.nodes[node].ports.clone() {
        let side = graph.ports[port].side;
        if Some(side) != current_port_side {
            if !end_label_queue.is_empty() {
                smart_for_regular_node_port_end_labels(
                    graph,
                    &mut end_label_queue,
                    current_port_side,
                    default_side,
                )?;
            }
            end_label_queue.clear();
            current_port_side = Some(side);
        }
        if let Some(port_end_labels) = gather_end_labels(graph, port) {
            end_label_queue.push_back(port_end_labels);
        }
    }
    if !end_label_queue.is_empty() {
        smart_for_regular_node_port_end_labels(
            graph,
            &mut end_label_queue,
            current_port_side,
            default_side,
        )?;
    }
    Ok(())
}

/// `smartForRegularNodePortEndLabels`: two ports on one side get opposite sides, otherwise the
/// default.
fn smart_for_regular_node_port_end_labels(
    graph: &mut LGraph,
    end_label_queue: &mut VecDeque<Vec<LabelRef>>,
    port_side: Option<PortSide>,
    default_side: LabelSide,
) -> Result<(), LayoutError> {
    let port_side = required(port_side, "the side of a port with end labels")?;
    if end_label_queue.len() == 2 {
        let (first_side, second_side) = if matches!(port_side, PortSide::NORTH | PortSide::EAST) {
            (LabelSide::ABOVE, LabelSide::BELOW)
        } else {
            (LabelSide::BELOW, LabelSide::ABOVE)
        };
        let first = end_label_queue.pop_front().unwrap_or_default();
        let second = end_label_queue.pop_front().unwrap_or_default();
        apply_label_side_to_refs(graph, &first, first_side);
        apply_label_side_to_refs(graph, &second, second_side);
    } else {
        for labels in end_label_queue.iter() {
            apply_label_side_to_refs(graph, labels, default_side);
        }
    }
    Ok(())
}

/// `EndLabelPreprocessor.gatherLabels(LPort)`: the end labels of a port, or `None` when no edge
/// meets the port. Also records the port's maximum edge thickness and each label's edge.
///
/// The north and south port dummies (`PORT_DUMMY`) are not part of this port: graphs with
/// north-south ports are not laid out yet, so no port has a dummy to gather from.
fn gather_end_labels(graph: &mut LGraph, port: usize) -> Option<Vec<LabelRef>> {
    let mut labels = Vec::new();
    let max_thickness = gather_port_end_labels(graph, port, &mut labels);
    if !labels.is_empty() {
        graph.ports[port].max_edge_thickness = Some(max_thickness);
    }
    if max_thickness != NO_INCIDENT_EDGE_THICKNESS {
        Some(labels)
    } else {
        None
    }
}

/// The private `gatherLabels(LPort, List)`: the tail labels of the port's outgoing edges and the
/// head labels of its incoming edges. Returns the largest incident edge thickness, or the sentinel.
fn gather_port_end_labels(graph: &mut LGraph, port: usize, target: &mut Vec<LabelRef>) -> f64 {
    let mut max_thickness = NO_INCIDENT_EDGE_THICKNESS;
    // `getConnectedEdges()`: the incoming edges, then the outgoing ones.
    let connected: Vec<usize> = graph.ports[port]
        .incoming
        .iter()
        .chain(graph.ports[port].outgoing.iter())
        .copied()
        .collect();
    for edge in connected {
        max_thickness = java_max(max_thickness, graph.edges[edge].thickness);
        // An outgoing edge's tail labels belong to this port; an incoming edge's head labels do.
        let wanted = if graph.edges[edge].source == port {
            EdgeLabelPlacement::TAIL
        } else {
            EdgeLabelPlacement::HEAD
        };
        let found: Vec<usize> = graph.edges[edge]
            .labels
            .iter()
            .enumerate()
            .filter(|(_, label)| {
                label
                    .properties
                    .enum_of::<EdgeLabelPlacement>(options::EDGE_LABELS_PLACEMENT)
                    == Some(wanted)
            })
            .map(|(index, _)| index)
            .collect();
        for &index in &found {
            let label = &mut graph.edges[edge].labels[index];
            if label.end_label_edge.is_none() {
                label.end_label_edge = Some(edge);
            }
            target.push(LabelRef { edge, index });
        }
    }
    max_thickness
}

/// `applyLabelSide(LNode, side)`: a label dummy gets `side`, or inline if all its labels are inline.
/// Its ports move to leave room for the label on that side.
fn apply_label_side_to_node(
    graph: &mut LGraph,
    node: usize,
    side: LabelSide,
) -> Result<(), LayoutError> {
    // Only label dummies have a side of their own.
    if graph.nodes[node].node_type != NodeType::LABEL {
        return Ok(());
    }
    let effective = if is_inline_edge_label(graph, node) {
        LabelSide::INLINE
    } else {
        side
    };
    graph.nodes[node].label_side = effective;

    // If the label is not below the edge, the ports need to move.
    if effective != LabelSide::BELOW {
        let origin = required(
            graph.nodes[node].origin_edge,
            "the origin edge of a label dummy",
        )?;
        let thickness = graph.edges[origin].thickness;
        let mut port_position = 0.0;
        if effective == LabelSide::ABOVE {
            port_position = graph.nodes[node].size.y - (thickness / 2.0).ceil();
        } else if effective == LabelSide::INLINE {
            // The label dummy has a superfluous label-edge spacing.
            let spacing = graph_double(graph, options::SPACING_EDGE_LABEL)?;
            port_position = (graph.nodes[node].size.y - spacing - thickness).ceil() / 2.0;
            graph.nodes[node].size.y -= spacing;
            graph.nodes[node].size.y -= thickness;
        }
        for port in graph.nodes[node].ports.clone() {
            graph.ports[port].position.y = port_position;
        }
    }
    Ok(())
}

/// `applyLabelSide(LEdge, side)`: every label of the edge gets `side`.
fn apply_label_side_to_edge(graph: &mut LGraph, edge: usize, side: LabelSide) {
    for label in &mut graph.edges[edge].labels {
        set_label_side(label, side);
    }
}

/// `applyLabelSide(List<LLabel>, side)`: every label in the list gets `side`.
fn apply_label_side_to_refs(graph: &mut LGraph, labels: &[LabelRef], side: LabelSide) {
    for label in labels {
        set_label_side(&mut graph.edges[label.edge].labels[label.index], side);
    }
}

/// `LabelDummyRemover.process`: each label dummy hands its labels back to its edge and leaves its
/// layer, and its edge is joined again.
pub fn label_dummy_removal(graph: &mut LGraph) -> Result<(), LayoutError> {
    let edge_label_spacing = graph_double(graph, options::SPACING_EDGE_LABEL)?;
    let label_label_spacing = graph_double(graph, options::SPACING_LABEL_LABEL)?;
    let direction = required(
        graph.properties.enum_of::<Direction>(options::DIRECTION),
        "the direction",
    )?;
    let polyline = graph
        .properties
        .enum_of::<EdgeRouting>(options::EDGE_ROUTING)
        == Some(EdgeRouting::POLYLINE);

    for layer in 0..graph.layers.len() {
        // The dummies leave the layer while it is walked, so the index is not advanced then.
        let mut index = 0;
        while index < graph.layers[layer].len() {
            let node = graph.layers[layer][index];
            if graph.nodes[node].node_type != NodeType::LABEL {
                index += 1;
                continue;
            }
            place_represented_labels(
                graph,
                node,
                edge_label_spacing,
                label_label_spacing,
                direction,
            )?;
            // Whether the edge keeps bend points at the dummy depends on the router: polylines need them.
            dummies::join_at(graph, node, polyline);
            graph.layers[layer].remove(index);
        }
    }
    Ok(())
}

/// Places the labels of a label dummy where the dummy was, and moves them onto its origin edge.
fn place_represented_labels(
    graph: &mut LGraph,
    node: usize,
    edge_label_spacing: f64,
    label_label_spacing: f64,
    direction: Direction,
) -> Result<(), LayoutError> {
    let origin = required(
        graph.nodes[node].origin_edge,
        "the origin edge of a label dummy",
    )?;
    let thickness = graph.edges[origin].thickness;
    let labels_below_edge = graph.nodes[node].label_side == LabelSide::BELOW;

    // Below the edge, the first label moves down past the thickness and the spacing.
    let mut label_pos = graph.nodes[node].position;
    if labels_below_edge {
        label_pos.y += thickness + edge_label_spacing;
    }

    let size = graph.nodes[node].size;
    let inline = is_inline_edge_label(graph, node);
    let label_space = KVector {
        x: size.x,
        y: size.y
            + if inline {
                0.0
            } else {
                -thickness - edge_label_spacing
            },
    };

    let mut labels = std::mem::take(&mut graph.nodes[node].represented_labels);
    if matches!(direction, Direction::UP | Direction::DOWN) {
        place_labels_for_vertical_layout(
            &mut labels,
            &mut label_pos,
            label_label_spacing,
            label_space,
            labels_below_edge,
            direction,
        );
    } else {
        place_labels_for_horizontal_layout(
            &mut labels,
            &mut label_pos,
            label_label_spacing,
            label_space,
        );
    }

    // The labels return to the edge they came from.
    graph.edges[origin].labels.extend(labels);
    Ok(())
}

/// `placeLabelsForHorizontalLayout`: the labels stack downwards, centred in the label space.
fn place_labels_for_horizontal_layout(
    labels: &mut [LLabel],
    label_pos: &mut KVector,
    label_spacing: f64,
    label_space: KVector,
) {
    for label in labels.iter_mut() {
        label.position.x = label_pos.x + (label_space.x - label.size.x) / 2.0;
        label.position.y = label_pos.y;
        label_pos.y += label.size.y + label_spacing;
    }
}

/// `placeLabelsForVerticalLayout`: the labels run along the edge. For `UP` they are placed in
/// reverse, so that the order in the final drawing is the same.
fn place_labels_for_vertical_layout(
    labels: &mut [LLabel],
    label_pos: &mut KVector,
    label_spacing: f64,
    label_space: KVector,
    left_aligned: bool,
    direction: Direction,
) {
    // The alignment is overridden when all the labels here are inline.
    let inline = labels
        .iter()
        .all(|label| label.properties.bool_of(options::EDGE_LABELS_INLINE));
    let order: Vec<usize> = if direction == Direction::UP {
        (0..labels.len()).rev().collect()
    } else {
        (0..labels.len()).collect()
    };
    for index in order {
        let label = &mut labels[index];
        label.position.x = label_pos.x;
        label.position.y = if inline {
            label_pos.y + (label_space.y - label.size.y) / 2.0
        } else if left_aligned {
            label_pos.y
        } else {
            label_pos.y + label_space.y - label.size.y
        };
        label_pos.x += label.size.x + label_spacing;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layered::lgraph::{LEdge, LNode, LPort};

    /// A graph with the given direction name, as `org.eclipse.elk.direction`.
    fn graph_in(direction: &str) -> LGraph {
        let mut graph = LGraph::default();
        graph.properties.set(
            options::DIRECTION,
            Some(OptionValue::Enum {
                class: "Direction",
                name: direction.to_string(),
            }),
        );
        graph
    }

    /// `LABEL_SIDE` of an edge label, `UNKNOWN` when it was never set.
    fn label_side_of(label: &LLabel) -> LabelSide {
        match label.properties.get(LABEL_SIDE) {
            Some(OptionValue::Enum { name, .. }) => {
                LabelSide::from_java_text(name).unwrap_or_default()
            }
            _ => LabelSide::UNKNOWN,
        }
    }

    fn is_included(label: &LLabel) -> bool {
        matches!(
            label.properties.get(INCLUDE_LABEL),
            Some(OptionValue::Boolean(true))
        )
    }

    fn enum_value(class: &'static str, name: &str) -> Option<OptionValue> {
        Some(OptionValue::Enum {
            class,
            name: name.to_string(),
        })
    }

    /// A node of `node_type` in `layer`, with an input port (`ports[0]`, west) and an output port
    /// (`ports[1]`, east), as the splitter gives dummy nodes.
    fn add_node(graph: &mut LGraph, node_type: NodeType, layer: usize) -> usize {
        let node = graph.nodes.len();
        graph.nodes.push(LNode {
            node_type,
            layer: Some(layer),
            ..LNode::default()
        });
        while graph.layers.len() <= layer {
            graph.layers.push(Vec::new());
        }
        graph.layers[layer].push(node);
        for side in [PortSide::WEST, PortSide::EAST] {
            let port = graph.ports.len();
            graph.ports.push(LPort::new(node, side));
            graph.nodes[node].ports.push(port);
        }
        node
    }

    /// An edge of thickness 1 from the output port of `from` to the input port of `to`.
    fn connect(graph: &mut LGraph, from: usize, to: usize) -> usize {
        let edge = graph.edges.len();
        let source = graph.nodes[from].ports[1];
        let target = graph.nodes[to].ports[0];
        graph.edges.push(LEdge {
            source,
            target,
            thickness: 1.0,
            ..LEdge::default()
        });
        graph.ports[source].outgoing.push(edge);
        graph.ports[target].incoming.push(edge);
        edge
    }

    /// A label dummy of `size` in `layer`, holding one label that is not inline. Its origin is
    /// set by the caller.
    fn add_label_dummy(graph: &mut LGraph, layer: usize, size: KVector) -> usize {
        let node = add_node(graph, NodeType::LABEL, layer);
        graph.nodes[node].size = size;
        graph.nodes[node].represented_labels.push(LLabel {
            size: KVector { x: 40.0, y: 21.0 },
            ..LLabel::default()
        });
        node
    }

    fn source_node(graph: &LGraph, edge: usize) -> usize {
        graph.ports[graph.edges[edge].source].node
    }

    fn target_node(graph: &LGraph, edge: usize) -> usize {
        graph.ports[graph.edges[edge].target].node
    }

    /// N0 (layer 0) -> label (layer 1) -> N1 (layer 2), with the label dummy's origin the first edge.
    fn chain_of_three(graph: &mut LGraph) -> (usize, usize, usize) {
        let n0 = add_node(graph, NodeType::NORMAL, 0);
        let label = add_label_dummy(graph, 1, KVector { x: 83.0, y: 24.0 });
        let n1 = add_node(graph, NodeType::NORMAL, 2);
        let first = connect(graph, n0, label);
        connect(graph, label, n1);
        graph.nodes[label].origin_edge = Some(first);
        (n0, label, n1)
    }

    fn set_side_selection(graph: &mut LGraph, name: &str) {
        graph.properties.set(
            options::EDGE_LABELS_SIDE_SELECTION,
            enum_value("EdgeLabelSideSelection", name),
        );
    }

    /// N0 (layer 0) -> label (layer 1) -> long edge dummy (layer 2) -> long edge dummy (layer 3)
    /// -> N1 (layer 4). The label dummy's origin is the first edge. Returns the nodes by name.
    fn long_chain_with_label_in_layer_one(
        graph: &mut LGraph,
    ) -> (usize, usize, usize, usize, usize) {
        let n0 = add_node(graph, NodeType::NORMAL, 0);
        let label = add_label_dummy(graph, 1, KVector { x: 83.0, y: 24.0 });
        let long2 = add_node(graph, NodeType::LONG_EDGE, 2);
        let long3 = add_node(graph, NodeType::LONG_EDGE, 3);
        let n1 = add_node(graph, NodeType::NORMAL, 4);
        let first = connect(graph, n0, label);
        connect(graph, label, long2);
        connect(graph, long2, long3);
        connect(graph, long3, n1);
        graph.nodes[label].origin_edge = Some(first);
        (n0, label, long2, long3, n1)
    }

    #[test]
    fn strategy_order_is_the_ordinal_order() {
        for (index, placement) in PLACEMENTS.iter().enumerate() {
            assert_eq!(*placement as usize, index);
        }
    }

    #[test]
    fn median_layer_moves_the_label_to_the_middle_dummy_of_its_edge() {
        let mut graph = graph_in("RIGHT");
        let (_, label, long2, long3, n1) = long_chain_with_label_in_layer_one(&mut graph);
        label_dummy_switching(&mut graph).unwrap();

        // The edge spans layers 1 to 3, so its median is layer 2, where the label swaps with
        // the first long edge dummy.
        assert_eq!(graph.nodes[label].layer, Some(2));
        assert_eq!(graph.nodes[long2].layer, Some(1));
        assert_eq!(graph.layers[1], vec![long2]);
        assert_eq!(graph.layers[2], vec![label]);
        assert_eq!(graph.layers[3], vec![long3]);
        // The edges now run N0 -> long2 -> label -> long3 -> N1.
        assert_eq!(target_node(&graph, 0), long2);
        assert_eq!(source_node(&graph, 1), long2);
        assert_eq!(target_node(&graph, 1), label);
        assert_eq!(source_node(&graph, 2), label);
        assert_eq!(target_node(&graph, 2), long3);
        assert_eq!(target_node(&graph, 3), n1);
    }

    #[test]
    fn end_layer_strategies_pick_the_end_of_the_edge_and_align_the_label() {
        fn head_or_tail(name: &str) -> (usize, Alignment) {
            let mut graph = graph_in("RIGHT");
            let n0 = add_node(&mut graph, NodeType::NORMAL, 0);
            let label = add_label_dummy(&mut graph, 1, KVector { x: 83.0, y: 24.0 });
            let long2 = add_node(&mut graph, NodeType::LONG_EDGE, 2);
            let n1 = add_node(&mut graph, NodeType::NORMAL, 3);
            let first = connect(&mut graph, n0, label);
            connect(&mut graph, label, long2);
            connect(&mut graph, long2, n1);
            graph.nodes[label].origin_edge = Some(first);
            graph.nodes[label].represented_labels[0].properties.set(
                options::EDGE_LABELS_CENTER_LABEL_PLACEMENT_STRATEGY,
                enum_value("CenterEdgeLabelPlacementStrategy", name),
            );
            label_dummy_switching(&mut graph).unwrap();
            let alignment = graph.nodes[label]
                .properties
                .enum_of::<Alignment>(options::ALIGNMENT)
                .unwrap();
            (graph.nodes[label].layer.unwrap(), alignment)
        }

        // The head is the rightmost layer of the edge, and the label dummy faces right.
        assert_eq!(head_or_tail("HEAD_LAYER"), (2, Alignment::RIGHT));
        // The tail is the leftmost layer, where the label dummy already is.
        assert_eq!(head_or_tail("TAIL_LAYER"), (1, Alignment::LEFT));
    }

    #[test]
    fn widest_and_center_layers_use_the_widest_normal_node_of_each_layer() {
        fn label_layer(name: &str) -> usize {
            let mut graph = graph_in("RIGHT");
            let (_, label, ..) = long_chain_with_label_in_layer_one(&mut graph);
            // A normal node of width 60 in layer 2 makes that layer the widest of the span.
            let wide = add_node(&mut graph, NodeType::NORMAL, 2);
            graph.nodes[wide].size.x = 60.0;
            graph.properties.set(
                options::EDGE_LABELS_CENTER_LABEL_PLACEMENT_STRATEGY,
                enum_value("CenterEdgeLabelPlacementStrategy", name),
            );
            label_dummy_switching(&mut graph).unwrap();
            graph.nodes[label].layer.unwrap()
        }

        // Widest: layer 1 has width 0 and layer 2 has 60.
        assert_eq!(label_layer("WIDEST_LAYER"), 2);
        // Center: the sums are 0, 80 and 100 over the label and the two long edge dummies to its
        // right, so the threshold is 50 and the second layer reaches it.
        assert_eq!(label_layer("CENTER_LAYER"), 2);
    }

    #[test]
    fn space_efficient_places_a_label_in_the_first_layer_wide_enough() {
        let mut graph = graph_in("RIGHT");
        let (_, label, ..) = long_chain_with_label_in_layer_one(&mut graph);
        graph.properties.set(
            options::EDGE_LABELS_CENTER_LABEL_PLACEMENT_STRATEGY,
            enum_value("CenterEdgeLabelPlacementStrategy", "SPACE_EFFICIENT_LAYER"),
        );
        // Layer 1 is 0 wide and layer 2 is 100 wide; the label is 83 wide.
        graph.layer_sizes = vec![
            KVector::default(),
            KVector::default(),
            KVector { x: 100.0, y: 0.0 },
            KVector::default(),
            KVector::default(),
        ];
        label_dummy_switching(&mut graph).unwrap();
        assert_eq!(graph.nodes[label].layer, Some(2));
        assert!(is_included(&graph.nodes[label].represented_labels[0]));
    }

    #[test]
    fn space_efficient_places_a_label_no_layer_fits_in_the_widest_layer() {
        let mut graph = graph_in("RIGHT");
        let (_, label, ..) = long_chain_with_label_in_layer_one(&mut graph);
        graph.properties.set(
            options::EDGE_LABELS_CENTER_LABEL_PLACEMENT_STRATEGY,
            enum_value("CenterEdgeLabelPlacementStrategy", "SPACE_EFFICIENT_LAYER"),
        );
        graph.nodes[label].size.x = 150.0;
        // A normal node of width 100 in layer 2. No layer fits the 150 wide label.
        let wide = add_node(&mut graph, NodeType::NORMAL, 2);
        graph.nodes[wide].size.x = 100.0;
        graph.layer_sizes = vec![
            KVector::default(),
            KVector::default(),
            KVector { x: 100.0, y: 0.0 },
            KVector::default(),
            KVector::default(),
        ];
        label_dummy_switching(&mut graph).unwrap();
        assert_eq!(graph.nodes[label].layer, Some(2));
    }

    #[test]
    fn always_up_puts_the_label_above_and_moves_the_ports_to_make_room() {
        let mut graph = graph_in("RIGHT");
        let (_, label, _) = chain_of_three(&mut graph);
        graph.edges[0].labels.push(LLabel::default());
        set_side_selection(&mut graph, "ALWAYS_UP");
        label_side_selection(&mut graph).unwrap();

        assert_eq!(graph.nodes[label].label_side, LabelSide::ABOVE);
        // The port sits at the height less half the thickness, rounded up: 24 - ceil(0.5) = 23.
        for &port in &graph.nodes[label].ports {
            assert_eq!(graph.ports[port].position.y, 23.0);
        }
        assert_eq!(label_side_of(&graph.edges[0].labels[0]), LabelSide::ABOVE);
    }

    #[test]
    fn direction_up_puts_the_label_below_when_both_its_edges_point_left() {
        fn side_with_reversed(first: bool, second: bool) -> LabelSide {
            let mut graph = graph_in("RIGHT");
            let (_, label, _) = chain_of_three(&mut graph);
            graph.edges[0].reversed = first;
            graph.edges[1].reversed = second;
            set_side_selection(&mut graph, "DIRECTION_UP");
            label_side_selection(&mut graph).unwrap();
            graph.nodes[label].label_side
        }

        // One rightward edge is enough for the label to be above.
        assert_eq!(side_with_reversed(true, false), LabelSide::ABOVE);
        assert_eq!(side_with_reversed(true, true), LabelSide::BELOW);
    }

    #[test]
    fn smart_splits_two_label_dummies_of_one_layer_above_and_below() {
        let mut graph = graph_in("RIGHT");
        let n0 = add_node(&mut graph, NodeType::NORMAL, 0);
        let first_label = add_label_dummy(&mut graph, 1, KVector { x: 83.0, y: 24.0 });
        let second_label = add_label_dummy(&mut graph, 1, KVector { x: 83.0, y: 24.0 });
        let first_edge = connect(&mut graph, n0, first_label);
        let second_edge = connect(&mut graph, n0, second_label);
        graph.nodes[first_label].origin_edge = Some(first_edge);
        graph.nodes[second_label].origin_edge = Some(second_edge);
        set_side_selection(&mut graph, "SMART_UP");
        label_side_selection(&mut graph).unwrap();

        assert_eq!(graph.nodes[first_label].label_side, LabelSide::ABOVE);
        assert_eq!(graph.nodes[second_label].label_side, LabelSide::BELOW);
    }

    #[test]
    fn removal_puts_the_labels_on_the_edge_centred_on_the_dummy() {
        let mut graph = graph_in("RIGHT");
        graph
            .properties
            .set(options::SPACING_EDGE_LABEL, Some(OptionValue::Double(2.0)));
        graph
            .properties
            .set(options::SPACING_LABEL_LABEL, Some(OptionValue::Double(1.0)));
        let (_, label, _) = chain_of_three(&mut graph);
        graph.nodes[label].position = KVector { x: 100.0, y: 50.0 };
        graph.nodes[label].represented_labels[0].size = KVector { x: 40.0, y: 21.0 };

        label_dummy_removal(&mut graph).unwrap();

        assert!(graph.layers[1].is_empty());
        // The label space is the dummy's width, and its height less the spacing and the thickness:
        // 24 - (1 + 2) = 21. The label is centred in it: 100 + (83 - 40) / 2.
        assert_eq!(graph.edges[0].labels.len(), 1);
        assert_eq!(
            graph.edges[0].labels[0].position,
            KVector { x: 121.5, y: 50.0 }
        );
        // The edge runs on to the dummy's old target.
        assert_eq!(target_node(&graph, 0), 2);
    }

    #[test]
    fn removal_in_an_upward_layout_places_the_labels_in_reverse() {
        let mut graph = graph_in("UP");
        graph
            .properties
            .set(options::SPACING_EDGE_LABEL, Some(OptionValue::Double(2.0)));
        graph
            .properties
            .set(options::SPACING_LABEL_LABEL, Some(OptionValue::Double(1.0)));
        let (_, label, _) = chain_of_three(&mut graph);
        graph.nodes[label].size = KVector { x: 30.0, y: 20.0 };
        graph.nodes[label].represented_labels = vec![
            LLabel {
                size: KVector { x: 10.0, y: 5.0 },
                ..LLabel::default()
            },
            LLabel {
                size: KVector { x: 20.0, y: 5.0 },
                ..LLabel::default()
            },
        ];

        label_dummy_removal(&mut graph).unwrap();

        // The label space is 30 by 20 - (1 + 2) = 17. For UP the labels are placed last first, so
        // the 20 wide label takes the left place, and the 10 wide one follows it at x = 21.
        let labels = &graph.edges[0].labels;
        assert_eq!(labels.len(), 2);
        assert_eq!(labels[0].position, KVector { x: 21.0, y: 12.0 });
        assert_eq!(labels[1].position, KVector { x: 0.0, y: 12.0 });
    }

    #[test]
    fn inline_labels_take_the_spacing_off_the_dummy_and_centre_its_ports() {
        let mut graph = graph_in("RIGHT");
        let (_, label, _) = chain_of_three(&mut graph);
        graph.nodes[label].represented_labels[0].properties.set(
            options::EDGE_LABELS_INLINE,
            Some(OptionValue::Boolean(true)),
        );
        set_side_selection(&mut graph, "ALWAYS_UP");
        label_side_selection(&mut graph).unwrap();

        // Inline: the dummy loses the edge label spacing (2) and the thickness (1), so 24 - 3 = 21,
        // and its ports sit at ceil(21) / 2 = 10.5.
        assert_eq!(graph.nodes[label].label_side, LabelSide::INLINE);
        assert_eq!(graph.nodes[label].size.y, 21.0);
        for &port in &graph.nodes[label].ports {
            assert_eq!(graph.ports[port].position.y, 10.5);
        }
    }

    /// A label dummy in `layer` on a long edge from the output of `source` to the input of `target`.
    fn label_on_long_edge(graph: &mut LGraph, layer: usize, source: usize, target: usize) -> usize {
        let label = add_label_dummy(graph, layer, KVector { x: 83.0, y: 24.0 });
        let origin = connect(graph, source, label);
        graph.nodes[label].origin_edge = Some(origin);
        graph.nodes[label].long_edge_source = Some(graph.nodes[source].ports[1]);
        graph.nodes[label].long_edge_target = Some(graph.nodes[target].ports[0]);
        label
    }

    #[test]
    fn runs_of_label_dummies_take_the_default_side_and_pairs_split() {
        // Three labels on one pair of nodes form one run, which takes the default side.
        let mut graph = graph_in("RIGHT");
        let source = add_node(&mut graph, NodeType::NORMAL, 0);
        let target = add_node(&mut graph, NodeType::NORMAL, 2);
        let labels: Vec<usize> = (0..3)
            .map(|_| label_on_long_edge(&mut graph, 1, source, target))
            .collect();
        set_side_selection(&mut graph, "SMART_DOWN");
        label_side_selection(&mut graph).unwrap();
        for label in labels {
            assert_eq!(graph.nodes[label].label_side, LabelSide::BELOW);
        }

        // Two labels on one pair of nodes split above and below; the label on another pair takes
        // the default side.
        let mut graph = graph_in("RIGHT");
        let source = add_node(&mut graph, NodeType::NORMAL, 0);
        let near = add_node(&mut graph, NodeType::NORMAL, 2);
        let far = add_node(&mut graph, NodeType::NORMAL, 2);
        let pair_first = label_on_long_edge(&mut graph, 1, source, near);
        let pair_second = label_on_long_edge(&mut graph, 1, source, near);
        let single = label_on_long_edge(&mut graph, 1, source, far);
        set_side_selection(&mut graph, "SMART_UP");
        label_side_selection(&mut graph).unwrap();
        assert_eq!(graph.nodes[pair_first].label_side, LabelSide::ABOVE);
        assert_eq!(graph.nodes[pair_second].label_side, LabelSide::BELOW);
        assert_eq!(graph.nodes[single].label_side, LabelSide::ABOVE);
    }

    /// An edge of thickness 1 from the output port `source` to the input port `target`.
    fn connect_ports(graph: &mut LGraph, source: usize, target: usize) -> usize {
        let edge = graph.edges.len();
        graph.edges.push(LEdge {
            source,
            target,
            thickness: 1.0,
            ..LEdge::default()
        });
        graph.ports[source].outgoing.push(edge);
        graph.ports[target].incoming.push(edge);
        edge
    }

    #[test]
    fn end_labels_of_two_ports_on_one_side_take_opposite_sides() {
        let mut graph = graph_in("RIGHT");
        let source = add_node(&mut graph, NodeType::NORMAL, 0);
        // A second east port, after the first, so that both ports are on one side in order.
        let second_port = graph.ports.len();
        graph.ports.push(LPort::new(source, PortSide::EAST));
        graph.nodes[source].ports.push(second_port);
        let first_target = add_node(&mut graph, NodeType::NORMAL, 1);
        let second_target = add_node(&mut graph, NodeType::NORMAL, 1);
        let first_out = graph.nodes[source].ports[1];
        let first_in = graph.nodes[first_target].ports[0];
        let second_in = graph.nodes[second_target].ports[0];
        let first_edge = connect_ports(&mut graph, first_out, first_in);
        let second_edge = connect_ports(&mut graph, second_port, second_in);
        // Each edge has a tail label, which belongs to the port the edge leaves by.
        for edge in [first_edge, second_edge] {
            let mut label = LLabel::default();
            label.properties.set(
                options::EDGE_LABELS_PLACEMENT,
                enum_value("EdgeLabelPlacement", "TAIL"),
            );
            graph.edges[edge].labels.push(label);
        }
        set_side_selection(&mut graph, "SMART_UP");
        label_side_selection(&mut graph).unwrap();

        // On the east side, the first port's label goes above and the second port's below.
        assert_eq!(
            label_side_of(&graph.edges[first_edge].labels[0]),
            LabelSide::ABOVE
        );
        assert_eq!(
            label_side_of(&graph.edges[second_edge].labels[0]),
            LabelSide::BELOW
        );
        // The end label remembers the edge it came from.
        assert_eq!(
            graph.edges[first_edge].labels[0].end_label_edge,
            Some(first_edge)
        );
    }
}
