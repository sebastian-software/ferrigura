//! The elkjs graph of a flowchart: a port of `buildElkGraphFromLayoutData` and the functions it
//! calls in `packages/mermaid/src/rendering-util/layout-algorithms/elk/render.ts`, with the subgraph
//! tree of `elk_tree.rs` (`subgraphFeedbackEdges.ts`, `find-common-ancestor.ts`). See
//! `docs-port/flowchart/render-notes/elk-build.md`.
//!
//! The reference keeps its working data on the elkjs objects (`isGroup`, `labelData`,
//! `layoutReversed`); here the graph holds only what elkjs reads, and [`ElkState`] holds the rest.
//!
//! Layout algorithms other than `elk.layered` are not run by ferrigura (decision 0006). The root
//! graph always uses `elk.layered`, and a container that asks for another algorithm is laid out as
//! if it had asked for none, which is the reference's own fallback for an unknown name. Each such
//! choice is recorded in [`ElkState::warnings`].

use std::collections::{HashMap, HashSet};

use ferrigura_elk::json::{JsonEdge, JsonLabel, JsonNode, LayoutOptions, js_number_string};
use ferrigura_model::config::{MermaidConfig, MermaidConfigElk};
use ferrigura_model::value::Value;
use serde_json::{Number, Value as Json};

use super::data::{Edge, LayoutData, Node, Terminal};
use super::elk_tree::{find_common_ancestor, find_subgraph_feedback_edges};
use super::terminal;

/// The measured title of a subgraph, as the reference keeps it in `labelData`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LabelData {
    /// The title's width.
    pub width: f64,
    /// The title's height less 2, as `getMeasuredLabelData` keeps it.
    pub height: f64,
    /// The width the title was wrapped at.
    pub wrapping_width: f64,
}

/// What the post-processing needs besides the graph.
#[derive(Debug, Clone, Default)]
pub struct ElkState {
    /// The subgraph of each node (`parentLookupDb.parentById`).
    pub parent_by_id: HashMap<String, String>,
    /// The members of each subgraph in order (`parentLookupDb.childrenById`).
    pub children_by_id: HashMap<String, Vec<String>>,
    /// The title of each subgraph.
    pub label_data: HashMap<String, LabelData>,
    /// The edges handed to elkjs with source and target swapped (`layoutReversed`).
    pub reversed_edges: HashSet<String>,
    /// Choices the port made that the reference does not: a layout algorithm that ferrigura does not
    /// run, and the reference's own warnings about unknown algorithm names.
    pub warnings: Vec<String>,
}

/// The graph for elkjs and the state for the post-processing.
#[derive(Debug, Clone, Default)]
pub struct ElkInput {
    /// The graph, as the reference passes it to `elk.layout`.
    pub graph: JsonNode,
    /// The side tables.
    pub state: ElkState,
}

/// Builds the elkjs graph from the measured layout data.
///
/// The reference's `buildElkGraphFromLayoutData`, with the steps in the same order: the root
/// options, the subgraph tree, the vertices, the edges, the subgraph options, the cross-hierarchy
/// policy, the feedback edges (when `orientFeedbackEdges`) and the entry constraint (when
/// `keepEntryNodeOnTop`).
#[must_use]
pub fn build(data: &LayoutData, config: &MermaidConfig) -> ElkInput {
    let mut state = ElkState::default();
    let algorithm = root_algorithm(&config.layout, &mut state.warnings);
    let mut root_options = root_layout_options(config, algorithm);
    set(
        &mut root_options,
        "elk.direction",
        text(dir2_elk_direction(&data.direction)),
    );

    let (parent_by_id, children_by_id) = sub_graphs(data);
    state.parent_by_id = parent_by_id;
    state.children_by_id = children_by_id;

    let mut arena: Vec<Work> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    let root_children = add_vertices(data, None, config, &mut arena, &mut index);
    let mut edges = add_edges(data);
    configure_subgraph_nodes(
        data,
        config,
        algorithm,
        &mut arena,
        &index,
        &state.children_by_id,
        &mut state.warnings,
    );
    configure_cross_hierarchy_edges(&edges, &mut arena, &index, &state.parent_by_id);
    if config.elk.orient_feedback_edges {
        reverse_subgraph_feedback_edges(&mut edges, &state.parent_by_id, &mut state.reversed_edges);
    }
    if config.elk.keep_entry_node_on_top {
        apply_cyclic_entry_constraint(data, &mut arena, &index);
    }

    state.label_data = data
        .nodes
        .iter()
        .filter(|node| node.is_group)
        .map(|node| (node.id.clone(), label_data_of(node, config)))
        .collect();

    let graph = JsonNode {
        id: "root".to_owned(),
        layout_options: Some(root_options),
        children: root_children
            .iter()
            .map(|&position| to_json(&arena, position))
            .collect(),
        edges,
        ..JsonNode::default()
    };
    ElkInput { graph, state }
}

/// A node of the reference's `nodeDb`: the ELK node under construction, with the fields of the
/// layout data that the later steps read.
#[derive(Debug, Clone, Default)]
struct Work {
    id: String,
    parent_id: Option<String>,
    is_group: bool,
    label: String,
    padding: f64,
    shape: String,
    dir: Option<String>,
    /// `metadata.algorithm` as the layout data gives it.
    raw_algorithm: Option<Value>,
    /// The container algorithm the layout uses, decided in [`configure_subgraph_nodes`].
    container_algorithm: Option<Value>,
    label_data: LabelData,
    /// The measured width of a leaf.
    width: f64,
    /// The measured height of a leaf.
    height: f64,
    /// The arena positions of the children, in order.
    children: Vec<usize>,
    labels: Vec<JsonLabel>,
    layout_options: Option<LayoutOptions>,
}

/// A subgraph as `buildSubgraphLayoutOptions` reads it.
#[derive(Debug, Clone, Copy, Default)]
pub struct SubgraphNode<'a> {
    /// The direction of the subgraph, as written (`TB`, `LR`, ...).
    pub dir: Option<&'a str>,
    /// The shape of the subgraph; `rect` when absent.
    pub shape: Option<&'a str>,
    /// The padding the shapes add around the title.
    pub padding: f64,
    /// The measured title.
    pub label_data: LabelData,
    /// `metadata.algorithm`, as given.
    pub metadata_algorithm: Option<&'a Value>,
}

/// The ELK layout algorithms that `layout:` accepts besides `elk` (`elk/algorithms.ts`).
static ELK_ALGORITHMS: [&str; 6] = [
    "elk.stress",
    "elk.force",
    "elk.mrtree",
    "elk.sporeOverlap",
    "elk.box",
    "elk.rectpacking",
];

/// The algorithms a container may select (`CONTAINER_ALGORITHMS` in `render.ts`).
static CONTAINER_ALGORITHMS: [&str; 8] = [
    "elk.layered",
    "elk.box",
    "elk.rectpacking",
    "elk.stress",
    "elk.force",
    "elk.mrtree",
    "elk.radial",
    "elk.sporeOverlap",
];

/// The algorithm of the root graph and of every container that runs the diagram's own.
const DEFAULT_ALGORITHM: &str = "elk.layered";

/// Margin reserved at the ends of each side of a node (`PORTS_SURROUNDING` in `render.ts`).
const PORTS_SURROUNDING: &str = "[top=12,left=12,bottom=12,right=12]";
/// Padding between a subgraph frame and its children.
const SUBGRAPH_PADDING: &str = "[top=24,left=24,bottom=24,right=24]";
/// Default `spacing.baseValue` for a subgraph that has no algorithm of its own.
const DEFAULT_SUBGRAPH_SPACING_BASE_VALUE: f64 = 24.0;
/// Gap between two sibling nodes in a subgraph.
const DEFAULT_SUBGRAPH_NODE_SPACING: f64 = 50.0;
/// Inner padding reserved around a container that runs its own algorithm.
const CONTAINER_PADDING: f64 = 15.0;
/// The same, for `elk.rectpacking`, which packs tighter.
const RECTPACKING_CONTAINER_PADDING: f64 = 10.0;

/// The options that `elk.rectpacking` sets on a container (`RECTPACKING_OPTIONS` in `render.ts`).
fn rectpacking_options() -> Vec<(&'static str, Json)> {
    vec![
        ("spacing.baseValue", number(15.0)),
        ("spacing.nodeNode", number(15.0)),
        ("elk.aspectRatio", text("1.6")),
        ("elk.expandNodes", text("true")),
        ("elk.rectpacking.trybox", text("true")),
        (
            "elk.rectpacking.packing.compaction.rowHeightReevaluation",
            text("true"),
        ),
        (
            "elk.rectpacking.packing.compaction.iterations",
            number(10.0),
        ),
        (
            "elk.rectpacking.whiteSpaceElimination.strategy",
            text("EQUAL_BETWEEN_STRUCTURES"),
        ),
        (
            "elk.rectpacking.widthApproximation.strategy",
            text("SCANLINE"),
        ),
    ]
}

/// The options that a container's algorithm sets, and so that [`clear_container_algorithm_options`]
/// removes (`CONTAINER_ALGORITHM_OVERRIDES` in `render.ts`).
const CONTAINER_ALGORITHM_OVERRIDES: [&str; 14] = [
    "nodeSize.constraints",
    "nodeSize.minimum",
    "elk.algorithm",
    "elk.aspectRatio",
    "elk.contentAlignment",
    "elk.expandNodes",
    "elk.padding",
    "spacing.baseValue",
    "spacing.nodeNode",
    "elk.rectpacking.trybox",
    "elk.rectpacking.packing.compaction.rowHeightReevaluation",
    "elk.rectpacking.packing.compaction.iterations",
    "elk.rectpacking.whiteSpaceElimination.strategy",
    "elk.rectpacking.widthApproximation.strategy",
];

/// The named presets of `elk.preset` (`ELK_PRESETS` in `render.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElkPreset {
    /// The layering strategy.
    pub layering: &'static str,
    /// The node placement of the root graph.
    pub placement: &'static str,
    /// The node placement of the containers.
    pub container_placement: &'static str,
    /// The Brandes-Koepf alignment.
    pub alignment: &'static str,
    /// The cycle breaking strategy.
    pub cycle_breaking: &'static str,
}

const DEFAULT_PRESET: ElkPreset = ElkPreset {
    layering: "NETWORK_SIMPLEX",
    placement: "BRANDES_KOEPF",
    container_placement: "BRANDES_KOEPF",
    alignment: "BALANCED",
    cycle_breaking: "DEPTH_FIRST",
};

const LEGACY_PRESET: ElkPreset = ElkPreset {
    layering: "NETWORK_SIMPLEX",
    placement: "BRANDES_KOEPF",
    container_placement: "BRANDES_KOEPF",
    alignment: "NONE",
    cycle_breaking: "GREEDY",
};

const MODEL_ORDER_PRESET: ElkPreset = ElkPreset {
    layering: "NETWORK_SIMPLEX",
    placement: "NETWORK_SIMPLEX",
    container_placement: "BRANDES_KOEPF",
    alignment: "NONE",
    cycle_breaking: "GREEDY_MODEL_ORDER",
};

const DEPTH_FIRST_PRESET: ElkPreset = ElkPreset {
    layering: "NETWORK_SIMPLEX",
    placement: "NETWORK_SIMPLEX",
    container_placement: "BRANDES_KOEPF",
    alignment: "NONE",
    cycle_breaking: "DEPTH_FIRST",
};

/// Resolves a preset name, falling back to `default` for an unknown one (`resolveElkPreset`).
#[must_use]
pub fn resolve_elk_preset(name: Option<&str>) -> &'static ElkPreset {
    match name {
        Some("legacy") => &LEGACY_PRESET,
        Some("modelOrder") => &MODEL_ORDER_PRESET,
        Some("depthFirst") => &DEPTH_FIRST_PRESET,
        _ => &DEFAULT_PRESET,
    }
}

/// `dir2ElkDirection`: the ELK direction of a diagram direction; `DOWN` for an unknown one.
#[must_use]
pub fn dir2_elk_direction(dir: &str) -> &'static str {
    match dir {
        "LR" => "RIGHT",
        "RL" => "LEFT",
        "TB" | "TD" => "DOWN",
        "BT" => "UP",
        _ => "DOWN",
    }
}

/// Resolves a container's requested algorithm, or `None` when the request is absent, not a string,
/// or not a supported ELK algorithm (`resolveContainerAlgorithm`).
///
/// An unknown name is reported to `log`, when there is one, and the container falls back to the
/// diagram's algorithm.
pub fn resolve_container_algorithm(
    requested: Option<&Value>,
    log: Option<&mut Vec<String>>,
) -> Option<&'static str> {
    let name = requested?.as_str()?;
    if let Some(known) = CONTAINER_ALGORITHMS.iter().find(|known| **known == name) {
        return Some(*known);
    }
    if let Some(log) = log {
        log.push(format!(
            "Unknown container layout algorithm \"{name}\". Supported values: {}. Falling back to the diagram's layout algorithm.",
            CONTAINER_ALGORITHMS.join(", ")
        ));
    }
    None
}

/// Whether a cluster shape paints its label as a title strip (`clusterPaintsTitle`).
fn cluster_paints_title(shape: Option<&str>) -> bool {
    !matches!(shape.unwrap_or("rect"), "noteGroup" | "divider")
}

/// The options that reserve a painted title's width before routing (`groupTitleSizeOptions`).
/// Empty for a cluster shape that paints no title.
fn group_title_size_options(
    shape: Option<&str>,
    padding: f64,
    label_data: LabelData,
) -> Vec<(&'static str, Json)> {
    if !cluster_paints_title(shape) {
        return Vec::new();
    }
    vec![
        ("nodeSize.constraints", text("[MINIMUM_SIZE, NODE_LABELS]")),
        (
            "nodeSize.minimum",
            text(&format!("({}, 0)", js(label_data.width + padding))),
        ),
    ]
}

/// The layout options of a subgraph with members (`buildSubgraphLayoutOptions`).
///
/// `elk` is the diagram's `elk` configuration, or `None` when it is not known; the options that
/// read it are then absent or take the preset's value, as the reference's `elkConfig?.` reads do.
/// `log` receives the reference's warning about an unknown container algorithm.
pub fn build_subgraph_layout_options(
    node: &SubgraphNode<'_>,
    elk: Option<&MermaidConfigElk>,
    algorithm: &str,
    log: Option<&mut Vec<String>>,
) -> LayoutOptions {
    let label_w = node.label_data.width;
    let pad = node.padding;
    let min_width = label_w + 2.0 * pad;
    let label_h = node.label_data.height;
    let preset = resolve_elk_preset(elk.map(|config| config.preset.as_str()));

    let mut options = LayoutOptions::default();
    // Every group gets its painted title width as a floor. Containers that run their own algorithm
    // override that floor with a wider, label-plus-both-paddings minimum.
    for (key, value) in group_title_size_options(node.shape, pad, node.label_data) {
        set(&mut options, key, value);
    }
    set(
        &mut options,
        "spacing.baseValue",
        number(DEFAULT_SUBGRAPH_SPACING_BASE_VALUE),
    );
    set(
        &mut options,
        "elk.layered.spacing.edgeNodeBetweenLayers",
        number(30.0),
    );
    set(&mut options, "elk.spacing.edgeEdge", number(20.0));
    set(
        &mut options,
        "spacing.nodeNode",
        number(DEFAULT_SUBGRAPH_NODE_SPACING),
    );
    set(&mut options, "elk.padding", text(SUBGRAPH_PADDING));
    set(
        &mut options,
        "nodeLabels.placement",
        text("[H_CENTER V_TOP, INSIDE]"),
    );
    if let Some(elk) = elk {
        set(
            &mut options,
            "elk.layered.mergeEdges",
            Json::Bool(elk.merge_edges),
        );
    }
    set(
        &mut options,
        "elk.layered.nodePlacement.bk.fixedAlignment",
        text(
            elk.and_then(|config| config.node_placement_alignment.as_deref())
                .unwrap_or(preset.alignment),
        ),
    );
    set(
        &mut options,
        "elk.layered.nodePlacement.strategy",
        text(
            elk.and_then(|config| config.node_placement_strategy.as_deref())
                .unwrap_or(preset.container_placement),
        ),
    );
    set(
        &mut options,
        "elk.layered.cycleBreaking.strategy",
        text(
            elk.and_then(|config| config.cycle_breaking_strategy.as_deref())
                .unwrap_or(preset.cycle_breaking),
        ),
    );
    set(
        &mut options,
        "elk.layered.nodePlacement.networkSimplex.nodeFlexibility",
        text("PORT_POSITION"),
    );
    set(
        &mut options,
        "elk.spacing.portsSurrounding",
        text(PORTS_SURROUNDING),
    );

    if let Some(container) = resolve_container_algorithm(node.metadata_algorithm, log) {
        // These algorithms also need a minimum height for their title strip.
        let pad_top = label_h + CONTAINER_PADDING;
        set(
            &mut options,
            "nodeSize.constraints",
            text("[MINIMUM_SIZE, NODE_LABELS]"),
        );
        set(
            &mut options,
            "nodeSize.minimum",
            text(&format!(
                "({}, {})",
                js(min_width),
                js(pad_top + CONTAINER_PADDING)
            )),
        );
        set(&mut options, "elk.algorithm", text(container));
        set(
            &mut options,
            "elk.hierarchyHandling",
            text("SEPARATE_CHILDREN"),
        );
        set(&mut options, "elk.aspectRatio", text("2.0"));
        set(&mut options, "elk.contentAlignment", text("H_CENTER V_TOP"));
        set(&mut options, "elk.expandNodes", text("true"));
        set(
            &mut options,
            "elk.padding",
            text(&format!(
                "[top={},left={},bottom={},right={}]",
                js(pad_top),
                js(CONTAINER_PADDING),
                js(CONTAINER_PADDING),
                js(CONTAINER_PADDING)
            )),
        );

        // Tighter spacing for rectpacking.
        if container == "elk.rectpacking" {
            let rect_pad_top = label_h + RECTPACKING_CONTAINER_PADDING;
            for (key, value) in rectpacking_options() {
                set(&mut options, key, value);
            }
            set(
                &mut options,
                "elk.padding",
                text(&format!(
                    "[top={},left={},bottom={},right={}]",
                    js(rect_pad_top),
                    js(RECTPACKING_CONTAINER_PADDING),
                    js(RECTPACKING_CONTAINER_PADDING),
                    js(RECTPACKING_CONTAINER_PADDING)
                )),
            );
            set(
                &mut options,
                "nodeSize.minimum",
                text(&format!(
                    "({}, {})",
                    js(min_width),
                    js(rect_pad_top + RECTPACKING_CONTAINER_PADDING)
                )),
            );
        }
    } else if let Some(dir) = node.dir.filter(|dir| !dir.is_empty()) {
        // A directional subgraph without an explicit algorithm runs the parent's algorithm in its
        // own coordinate system.
        set(&mut options, "elk.algorithm", text(algorithm));
        set(&mut options, "elk.direction", text(dir2_elk_direction(dir)));
        set(
            &mut options,
            "elk.hierarchyHandling",
            text("SEPARATE_CHILDREN"),
        );
    }

    options
}

/// Removes the options that a container's algorithm set, and restores the values a plain subgraph
/// has (`clearContainerAlgorithmOptions`). Used when cross-boundary edges force the container back
/// onto the inherited algorithm.
pub fn clear_container_algorithm_options(options: &mut LayoutOptions) {
    for key in CONTAINER_ALGORITHM_OVERRIDES {
        options.shift_remove(key);
    }
    // `spacing.baseValue` and `spacing.nodeNode` are restored after their removal, so they move to
    // the end of the order, as the reference's delete-and-assign does.
    set(
        options,
        "spacing.baseValue",
        number(DEFAULT_SUBGRAPH_SPACING_BASE_VALUE),
    );
    set(
        options,
        "spacing.nodeNode",
        number(DEFAULT_SUBGRAPH_NODE_SPACING),
    );
}

/// The nodes that a source-less cycle of a container nominates as its entry
/// (`findCyclicEntryNodes`), in the order the reference finds them.
///
/// Edges are read only within a container, and self loops are ignored. A component without a node
/// of in-degree 0 contains a cycle; its entry is the first node, in declaration order, that is a
/// source once the back edges are removed greedily in declaration order.
#[must_use]
pub fn find_cyclic_entry_nodes(nodes: &[Node], edges: &[Edge]) -> Vec<String> {
    // The nodes of each container, in declaration order; containers in first-appearance order.
    let mut groups: Vec<(Option<&str>, Vec<&str>)> = Vec::new();
    for node in nodes {
        let parent = node.parent_id.as_deref();
        match groups.iter_mut().find(|(key, _)| *key == parent) {
            Some((_, ids)) => ids.push(node.id.as_str()),
            None => groups.push((parent, vec![node.id.as_str()])),
        }
    }

    let mut entries = Vec::new();
    for (_, ids) in &groups {
        let id_set: HashSet<&str> = ids.iter().copied().collect();
        let mut in_degree: HashMap<&str, usize> = ids.iter().map(|id| (*id, 0)).collect();
        // Undirected adjacency, used only to find weakly connected components.
        let mut neighbors: HashMap<&str, Vec<&str>> =
            ids.iter().map(|id| (*id, Vec::new())).collect();
        // Container-internal directed edges in declaration order.
        let mut internal: Vec<(&str, &str)> = Vec::new();

        for edge in edges {
            let (source, target) = (edge.start.as_str(), edge.end.as_str());
            if source.is_empty() || target.is_empty() || source == target {
                continue;
            }
            if !id_set.contains(source) || !id_set.contains(target) {
                continue;
            }
            *in_degree.entry(target).or_insert(0) += 1;
            neighbors.entry(source).or_default().push(target);
            neighbors.entry(target).or_default().push(source);
            internal.push((source, target));
        }

        // Label the weakly connected components.
        let mut component: HashMap<&str, usize> = HashMap::new();
        let mut component_count = 0;
        for id in ids {
            if component.contains_key(id) {
                continue;
            }
            let mut stack = vec![*id];
            component.insert(*id, component_count);
            while let Some(current) = stack.pop() {
                for &next in &neighbors[current] {
                    if !component.contains_key(next) {
                        component.insert(next, component_count);
                        stack.push(next);
                    }
                }
            }
            component_count += 1;
        }

        // A component with no in-degree-0 node necessarily contains a cycle.
        let mut has_source = vec![false; component_count];
        for id in ids {
            if in_degree[id] == 0 {
                has_source[component[id]] = true;
            }
        }
        if has_source.iter().all(|&source| source) {
            continue;
        }

        // Break the cycles greedily in declaration order: an edge that would close a directed
        // cycle is skipped.
        let mut forward: HashMap<&str, Vec<&str>> =
            ids.iter().map(|id| (*id, Vec::new())).collect();
        let mut residual_in_degree: HashMap<&str, usize> = ids.iter().map(|id| (*id, 0)).collect();
        for &(source, target) in &internal {
            if reaches(&forward, target, source) {
                continue;
            }
            forward.entry(source).or_default().push(target);
            *residual_in_degree.entry(target).or_insert(0) += 1;
        }

        let mut nominated = vec![false; component_count];
        for id in ids {
            let c = component[id];
            if !has_source[c] && !nominated[c] && residual_in_degree[id] == 0 {
                entries.push((*id).to_owned());
                nominated[c] = true;
            }
        }
    }
    entries
}

/// Whether `to` is reachable from `from` along the forward edges.
fn reaches(forward: &HashMap<&str, Vec<&str>>, from: &str, to: &str) -> bool {
    let mut seen: HashSet<&str> = HashSet::from([from]);
    let mut stack = vec![from];
    while let Some(current) = stack.pop() {
        if current == to {
            return true;
        }
        for &next in forward.get(current).map_or(&[][..], Vec::as_slice) {
            if seen.insert(next) {
                stack.push(next);
            }
        }
    }
    false
}

/// The algorithm of the root graph. ferrigura runs `elk.layered` only (decision 0006); any other
/// `layout:` name that ELK offers is laid out with it, and a warning is kept.
fn root_algorithm(layout: &str, warnings: &mut Vec<String>) -> &'static str {
    if ELK_ALGORITHMS.contains(&layout) {
        warnings.push(format!(
            "Layout \"{layout}\" is not run by ferrigura; the diagram is laid out with elk.layered (decision 0006)."
        ));
    }
    DEFAULT_ALGORITHM
}

/// The options of the root graph, in the reference's order (`createRootElkGraph`). The direction is
/// set by the caller, which replaces the placeholder `DOWN` in place.
fn root_layout_options(config: &MermaidConfig, algorithm: &str) -> LayoutOptions {
    let elk = &config.elk;
    let preset = resolve_elk_preset(Some(elk.preset.as_str()));
    let mut options = LayoutOptions::default();
    set(
        &mut options,
        "elk.hierarchyHandling",
        text("INCLUDE_CHILDREN"),
    );
    set(&mut options, "elk.algorithm", text(algorithm));
    set(
        &mut options,
        "elk.layered.nodePlacement.strategy",
        text(
            elk.node_placement_strategy
                .as_deref()
                .unwrap_or(preset.placement),
        ),
    );
    set(
        &mut options,
        "elk.layered.nodePlacement.bk.fixedAlignment",
        text(
            elk.node_placement_alignment
                .as_deref()
                .unwrap_or(preset.alignment),
        ),
    );
    set(
        &mut options,
        "elk.layered.mergeEdges",
        Json::Bool(elk.merge_edges),
    );
    set(&mut options, "elk.direction", text("DOWN"));
    set(&mut options, "spacing.baseValue", number(40.0));
    set(
        &mut options,
        "elk.layered.crossingMinimization.forceNodeModelOrder",
        Json::Bool(elk.force_node_model_order),
    );
    set(
        &mut options,
        "elk.layered.considerModelOrder.strategy",
        text(&elk.consider_model_order),
    );
    set(
        &mut options,
        "elk.layered.unnecessaryBendpoints",
        Json::Bool(true),
    );
    set(
        &mut options,
        "elk.layered.cycleBreaking.strategy",
        text(
            elk.cycle_breaking_strategy
                .as_deref()
                .unwrap_or(preset.cycle_breaking),
        ),
    );
    set(
        &mut options,
        "elk.layered.layering.strategy",
        text(elk.layering_strategy.as_deref().unwrap_or(preset.layering)),
    );
    // Only COFFMAN_GRAHAM reads this; the others ignore it.
    set(
        &mut options,
        "elk.layered.layering.coffmanGraham.layerBound",
        number(elk.layering_layer_bound),
    );
    set(
        &mut options,
        "elk.layered.wrapping.multiEdge.improveCuts",
        Json::Bool(true),
    );
    set(
        &mut options,
        "elk.layered.wrapping.multiEdge.improveWrappedEdges",
        Json::Bool(true),
    );
    set(
        &mut options,
        "elk.layered.edgeRouting.selfLoopDistribution",
        text("EQUALLY"),
    );
    set(
        &mut options,
        "elk.layered.mergeHierarchyEdges",
        Json::Bool(true),
    );
    set(
        &mut options,
        "elk.spacing.portsSurrounding",
        text(PORTS_SURROUNDING),
    );
    options
}

/// The parent lookup of the subgraphs (`addSubGraphs`): each node's subgraph, and each subgraph's
/// members in layout-data order. A subgraph without members has no entry in `childrenById`.
fn sub_graphs(data: &LayoutData) -> (HashMap<String, String>, HashMap<String, Vec<String>>) {
    let mut parent_by_id = HashMap::new();
    let mut children_by_id: HashMap<String, Vec<String>> = HashMap::new();
    for subgraph in data.nodes.iter().filter(|node| node.is_group) {
        for node in data
            .nodes
            .iter()
            .filter(|node| node.parent_id.as_deref() == Some(subgraph.id.as_str()))
        {
            parent_by_id.insert(node.id.clone(), subgraph.id.clone());
            children_by_id
                .entry(subgraph.id.clone())
                .or_default()
                .push(node.id.clone());
        }
    }
    (parent_by_id, children_by_id)
}

/// The measured title of a node (`getMeasuredLabelData`): the label box, with the height less 2.
fn label_data_of(node: &Node, config: &MermaidConfig) -> LabelData {
    match node.label_bbox {
        Some((width, height)) => LabelData {
            width,
            height: (height - 2.0).max(0.0),
            wrapping_width: config.flowchart.wrapping_width,
        },
        None => LabelData {
            width: 0.0,
            height: 0.0,
            wrapping_width: config.flowchart.wrapping_width,
        },
    }
}

/// Adds the children of `parent` and, recursively, of every subgraph among them (`addVertices`).
/// Returns the arena positions of the children in layout-data order.
fn add_vertices(
    data: &LayoutData,
    parent: Option<&str>,
    config: &MermaidConfig,
    arena: &mut Vec<Work>,
    index: &mut HashMap<String, usize>,
) -> Vec<usize> {
    let mut added = Vec::new();
    for node in data
        .nodes
        .iter()
        .filter(|node| node.parent_id.as_deref() == parent)
    {
        let position = arena.len();
        arena.push(Work {
            id: node.id.clone(),
            parent_id: node.parent_id.clone(),
            is_group: node.is_group,
            label: node.label.clone(),
            padding: node.padding,
            shape: node.shape.clone(),
            dir: node.dir.clone(),
            raw_algorithm: node.metadata.get("algorithm").cloned(),
            label_data: label_data_of(node, config),
            width: node.width.unwrap_or(0.0),
            height: node.height.unwrap_or(0.0),
            ..Work::default()
        });
        index.insert(node.id.clone(), position);
        added.push(position);
        if node.is_group {
            let children = add_vertices(data, Some(&node.id), config, arena, index);
            arena[position].children = children;
        }
    }
    added
}

/// The root edges of the graph, one per layout-data edge, with their label (`addEdgesToElkGraph`).
fn add_edges(data: &LayoutData) -> Vec<JsonEdge> {
    data.edges
        .iter()
        .map(|edge| JsonEdge {
            id: edge.id.clone(),
            sources: vec![edge.start.clone()],
            targets: vec![edge.end.clone()],
            labels: std::iter::once(JsonLabel {
                text: Some(edge.label.clone()),
                width: Some(edge.width.unwrap_or(0.0)),
                height: Some(edge.height.unwrap_or(0.0)),
                layout_options: Some(edge_label_options()),
                ..JsonLabel::default()
            })
            .chain(edge.terminals.iter().map(terminal_elk_label(edge)))
            .collect(),
            ..JsonEdge::default()
        })
        .collect()
}

/// The ELK label of a terminal label: padded past its end's marker, and placed at the start or the
/// end of the edge (`terminalElkLabels`, `render.ts` 1148-1170).
fn terminal_elk_label(edge: &Edge) -> impl Fn(&Terminal) -> JsonLabel + '_ {
    move |terminal| {
        let pad = terminal::elk_padding(edge, terminal.key);
        let mut options = LayoutOptions::default();
        let placement = if terminal.key.at_start() {
            "TAIL"
        } else {
            "HEAD"
        };
        set(&mut options, "edgeLabels.placement", text(placement));
        JsonLabel {
            text: Some(terminal.text.clone()),
            width: Some(terminal.width + 2.0 * pad),
            height: Some(terminal.height + 2.0 * pad),
            layout_options: Some(options),
            ..JsonLabel::default()
        }
    }
}

/// The options of an edge label: its position in the middle of the edge.
fn edge_label_options() -> LayoutOptions {
    let mut options = LayoutOptions::default();
    set(&mut options, "edgeLabels.inline", text("true"));
    set(&mut options, "edgeLabels.placement", text("CENTER"));
    options
}

/// Gives every subgraph with members its label and options (`configureSubgraphNodes`).
///
/// A subgraph without members is left as it is: it reaches ELK as an empty compound node.
fn configure_subgraph_nodes(
    data: &LayoutData,
    config: &MermaidConfig,
    algorithm: &str,
    arena: &mut [Work],
    index: &HashMap<String, usize>,
    children_by_id: &HashMap<String, Vec<String>>,
    warnings: &mut Vec<String>,
) {
    for node in &data.nodes {
        let Some(&position) = index.get(&node.id) else {
            continue;
        };
        if !children_by_id.contains_key(&node.id) {
            continue;
        }
        let container =
            decide_container_algorithm(arena[position].raw_algorithm.as_ref(), warnings);
        let work = &mut arena[position];
        work.container_algorithm = container;
        work.labels = vec![JsonLabel {
            text: Some(work.label.clone()),
            width: Some(work.label_data.width),
            height: Some(work.label_data.height),
            ..JsonLabel::default()
        }];
        let options = build_subgraph_layout_options(
            &SubgraphNode {
                dir: work.dir.as_deref(),
                shape: Some(work.shape.as_str()),
                padding: work.padding,
                label_data: work.label_data,
                metadata_algorithm: work.container_algorithm.as_ref(),
            },
            Some(&config.elk),
            algorithm,
            Some(&mut *warnings),
        );
        work.layout_options = Some(options);
    }
}

/// The container algorithm a subgraph lays out with: the requested one when ferrigura runs it, else
/// none, so that the subgraph falls back to the diagram's algorithm (decision 0006). An unknown
/// name is reported by [`resolve_container_algorithm`]; a supported one that ferrigura does not run
/// is reported here.
fn decide_container_algorithm(
    requested: Option<&Value>,
    warnings: &mut Vec<String>,
) -> Option<Value> {
    let name = resolve_container_algorithm(requested, Some(&mut *warnings))?;
    if name == DEFAULT_ALGORITHM {
        return Some(Value::String(name.to_owned()));
    }
    warnings.push(format!(
        "Container layout algorithm \"{name}\" is not run by ferrigura; the container is laid out with the diagram's algorithm, elk.layered (decision 0006)."
    ));
    None
}

/// Makes every cross-boundary edge's ends include their children, up to the common ancestor
/// (`configureCrossHierarchyEdges`).
fn configure_cross_hierarchy_edges(
    edges: &[JsonEdge],
    arena: &mut [Work],
    index: &HashMap<String, usize>,
    parent_by_id: &HashMap<String, String>,
) {
    for edge in edges {
        let source = first_end(&edge.sources);
        let target = first_end(&edge.targets);
        let (Some(&source_position), Some(&target_position)) =
            (index.get(source), index.get(target))
        else {
            continue;
        };
        if arena[source_position].parent_id != arena[target_position].parent_id {
            let ancestor = find_common_ancestor(source, target, parent_by_id);
            set_include_children_policy(arena, index, source, &ancestor);
            set_include_children_policy(arena, index, target, &ancestor);
        }
    }
}

/// The first id of an edge's ends, or the empty string.
fn first_end(ends: &[String]) -> &str {
    ends.first().map_or("", String::as_str)
}

/// Makes a node and its ancestors up to `ancestor_id` include their children
/// (`setIncludeChildrenPolicy`). A container with its own algorithm loses it, and gets the title
/// floor of a plain subgraph back.
fn set_include_children_policy(
    arena: &mut [Work],
    index: &HashMap<String, usize>,
    node_id: &str,
    ancestor_id: &str,
) {
    let mut current = node_id.to_owned();
    loop {
        let Some(&position) = index.get(&current) else {
            return;
        };
        let work = &mut arena[position];
        let title =
            group_title_size_options(Some(work.shape.as_str()), work.padding, work.label_data);
        let options = work
            .layout_options
            .get_or_insert_with(LayoutOptions::default);
        let separate = options.get("elk.hierarchyHandling").and_then(Json::as_str)
            == Some("SEPARATE_CHILDREN");
        if separate && work.container_algorithm.is_some() {
            clear_container_algorithm_options(options);
            for (key, value) in title {
                set(options, key, value);
            }
        }
        set(options, "elk.hierarchyHandling", text("INCLUDE_CHILDREN"));

        match work.parent_id.clone() {
            Some(parent) if current != ancestor_id && !parent.is_empty() => current = parent,
            _ => return,
        }
    }
}

/// Hands the edges that re-enter a subgraph the other way round to ELK, and records them
/// (`reverseSubgraphFeedbackEdges`).
fn reverse_subgraph_feedback_edges(
    edges: &mut [JsonEdge],
    parent_by_id: &HashMap<String, String>,
    reversed: &mut HashSet<String>,
) {
    let pairs: Vec<(String, String)> = edges
        .iter()
        .map(|edge| {
            (
                first_end(&edge.sources).to_owned(),
                first_end(&edge.targets).to_owned(),
            )
        })
        .collect();
    let ends: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(source, target)| (source.as_str(), target.as_str()))
        .collect();
    let flags = find_subgraph_feedback_edges(&ends, parent_by_id);
    for (edge, flag) in edges.iter_mut().zip(flags) {
        if flag {
            std::mem::swap(&mut edge.sources, &mut edge.targets);
            reversed.insert(edge.id.clone());
        }
    }
}

/// Pins the entry node of each cyclic container to the first layer (`applyCyclicEntryConstraint`).
fn apply_cyclic_entry_constraint(
    data: &LayoutData,
    arena: &mut [Work],
    index: &HashMap<String, usize>,
) {
    for id in find_cyclic_entry_nodes(&data.nodes, &data.edges) {
        if let Some(&position) = index.get(&id) {
            let options = arena[position]
                .layout_options
                .get_or_insert_with(LayoutOptions::default);
            set(
                options,
                "elk.layered.layering.layerConstraint",
                text("FIRST"),
            );
        }
    }
}

/// The elkjs node of an arena position, with its children (the `nodeDb` objects of the reference,
/// in the order elkjs reads them).
fn to_json(arena: &[Work], position: usize) -> JsonNode {
    let work = &arena[position];
    JsonNode {
        id: work.id.clone(),
        width: (!work.is_group).then_some(work.width),
        height: (!work.is_group).then_some(work.height),
        layout_options: work.layout_options.clone(),
        labels: work.labels.clone(),
        children: work
            .children
            .iter()
            .map(|&child| to_json(arena, child))
            .collect(),
        ..JsonNode::default()
    }
}

/// Sets an option, keeping its place when it is already set (`object[key] = value`).
fn set(options: &mut LayoutOptions, key: &str, value: Json) {
    options.insert(key.to_owned(), value);
}

/// A string-valued option.
fn text(value: &str) -> Json {
    Json::String(value.to_owned())
}

/// A number-valued option; a value that JSON cannot hold becomes `null`, as `JSON.stringify` does.
fn number(value: f64) -> Json {
    Number::from_f64(value).map_or(Json::Null, Json::Number)
}

/// The text of a number as the reference writes it into a template string.
fn js(value: f64) -> String {
    js_number_string(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// The configuration with the given `elk` settings.
    fn config(settings: impl FnOnce(&mut MermaidConfigElk)) -> MermaidConfig {
        let mut config = MermaidConfig::default();
        settings(&mut config.elk);
        config
    }

    /// An `elk` configuration with only `mergeEdges` set, as the reference's specs pass it.
    fn elk_merge(merge_edges: bool) -> MermaidConfigElk {
        MermaidConfigElk {
            merge_edges,
            ..MermaidConfigElk::default()
        }
    }

    /// A `metadata.algorithm` value.
    fn algorithm(name: &str) -> Value {
        Value::String(name.to_owned())
    }

    /// The options as text, if the option is a string.
    fn text_of(options: &LayoutOptions, key: &str) -> Option<String> {
        options.get(key).and_then(Json::as_str).map(str::to_owned)
    }

    /// The options as a number, if the option is a number.
    fn number_of(options: &LayoutOptions, key: &str) -> Option<f64> {
        options.get(key).and_then(Json::as_f64)
    }

    /// A node of the layout data with the given container.
    fn node(id: &str, parent: Option<&str>) -> Node {
        Node {
            id: id.to_owned(),
            label: id.to_owned(),
            parent_id: parent.map(str::to_owned),
            shape: "squareRect".to_owned(),
            ..Node::default()
        }
    }

    /// A subgraph of the layout data with the given container.
    fn subgraph(id: &str, parent: Option<&str>, label_bbox: Option<(f64, f64)>) -> Node {
        Node {
            is_group: true,
            padding: 8.0,
            shape: "rect".to_owned(),
            label_bbox,
            ..node(id, parent)
        }
    }

    /// An edge of the layout data.
    fn edge(id: &str, start: &str, end: &str) -> Edge {
        Edge {
            id: id.to_owned(),
            start: start.to_owned(),
            end: end.to_owned(),
            ..Edge::default()
        }
    }

    /// The node with the given id, anywhere in the graph.
    fn find<'a>(graph: &'a JsonNode, id: &str) -> Option<&'a JsonNode> {
        if graph.id == id {
            return Some(graph);
        }
        graph.children.iter().find_map(|child| find(child, id))
    }

    mod resolve_container_algorithm {
        use super::*;

        #[test]
        fn accepts_the_supported_elk_algorithms() {
            for name in ["elk.layered", "elk.box", "elk.rectpacking", "elk.mrtree"] {
                assert_eq!(
                    resolve_container_algorithm(Some(&algorithm(name)), None),
                    Some(name)
                );
            }
        }

        #[test]
        fn rejects_an_unknown_algorithm_and_warns_instead_of_handing_it_to_elk() {
            let mut warnings = Vec::new();
            assert_eq!(
                resolve_container_algorithm(Some(&algorithm("garbage")), Some(&mut warnings)),
                None
            );
            assert_eq!(warnings.len(), 1);
            assert!(warnings[0].contains("garbage"));
        }

        #[test]
        fn rejects_non_string_values_without_warning() {
            let mut warnings = Vec::new();
            let object = Value::Object(BTreeMap::from([(
                "elk".to_owned(),
                Value::String("box".to_owned()),
            )]));
            assert_eq!(resolve_container_algorithm(None, Some(&mut warnings)), None);
            assert_eq!(
                resolve_container_algorithm(Some(&object), Some(&mut warnings)),
                None
            );
            assert!(warnings.is_empty());
        }
    }

    mod build_subgraph_layout_options {
        use super::*;

        #[test]
        fn derives_a_label_based_minimum_size_for_containers_with_their_own_algorithm() {
            let box_algorithm = algorithm("elk.box");
            let options = build_subgraph_layout_options(
                &SubgraphNode {
                    padding: 8.0,
                    label_data: LabelData {
                        width: 44.0,
                        height: 14.0,
                        wrapping_width: 0.0,
                    },
                    metadata_algorithm: Some(&box_algorithm),
                    ..SubgraphNode::default()
                },
                Some(&elk_merge(true)),
                "elk.layered",
                None,
            );
            assert_eq!(
                text_of(&options, "nodeSize.constraints").as_deref(),
                Some("[MINIMUM_SIZE, NODE_LABELS]")
            );
            // Height clears the whole reserved strip: label (14) plus 15px above and below.
            assert_eq!(
                text_of(&options, "nodeSize.minimum").as_deref(),
                Some("(60, 44)")
            );
            assert_eq!(
                text_of(&options, "elk.padding").as_deref(),
                Some("[top=29,left=15,bottom=15,right=15]")
            );
        }

        #[test]
        fn reserves_title_padding_while_leaving_child_layout_and_height_to_elk() {
            let options = build_subgraph_layout_options(
                &SubgraphNode {
                    padding: 8.0,
                    label_data: LabelData {
                        width: 44.0,
                        height: 14.0,
                        wrapping_width: 0.0,
                    },
                    ..SubgraphNode::default()
                },
                Some(&elk_merge(true)),
                "elk.layered",
                None,
            );
            assert_eq!(
                text_of(&options, "nodeSize.constraints").as_deref(),
                Some("[MINIMUM_SIZE, NODE_LABELS]")
            );
            assert_eq!(
                text_of(&options, "nodeSize.minimum").as_deref(),
                Some("(52, 0)")
            );
        }

        #[test]
        fn reserves_no_title_width_for_cluster_shapes_that_paint_no_title() {
            let options = build_subgraph_layout_options(
                &SubgraphNode {
                    shape: Some("noteGroup"),
                    padding: 16.0,
                    label_data: LabelData {
                        width: 336.0,
                        height: 40.0,
                        wrapping_width: 0.0,
                    },
                    ..SubgraphNode::default()
                },
                None,
                "elk.layered",
                None,
            );
            assert!(!options.contains_key("nodeSize.constraints"));
            assert!(!options.contains_key("nodeSize.minimum"));
        }

        #[test]
        fn propagates_merge_edges_to_subgraphs_without_an_explicit_direction() {
            let options = build_subgraph_layout_options(
                &SubgraphNode::default(),
                Some(&elk_merge(true)),
                "elk.layered",
                None,
            );
            assert_eq!(
                options.get("elk.layered.mergeEdges"),
                Some(&Json::Bool(true))
            );
        }

        #[test]
        fn propagates_merge_edges_to_subgraphs_with_an_explicit_direction() {
            let options = build_subgraph_layout_options(
                &SubgraphNode {
                    dir: Some("LR"),
                    ..SubgraphNode::default()
                },
                Some(&elk_merge(true)),
                "elk.layered",
                None,
            );
            assert_eq!(
                options.get("elk.layered.mergeEdges"),
                Some(&Json::Bool(true))
            );
            assert_eq!(text_of(&options, "elk.direction").as_deref(), Some("RIGHT"));
            assert_eq!(
                text_of(&options, "elk.algorithm").as_deref(),
                Some("elk.layered")
            );
            assert_eq!(
                text_of(&options, "elk.hierarchyHandling").as_deref(),
                Some("SEPARATE_CHILDREN")
            );
        }

        #[test]
        fn ignores_an_unsupported_metadata_algorithm_and_falls_back_to_the_dir_branch() {
            let garbage = algorithm("elk.garbage");
            let mut warnings = Vec::new();
            let options = build_subgraph_layout_options(
                &SubgraphNode {
                    dir: Some("LR"),
                    padding: 0.0,
                    label_data: LabelData {
                        width: 30.0,
                        height: 14.0,
                        wrapping_width: 0.0,
                    },
                    metadata_algorithm: Some(&garbage),
                    ..SubgraphNode::default()
                },
                None,
                "elk.layered",
                Some(&mut warnings),
            );
            assert_eq!(
                text_of(&options, "elk.algorithm").as_deref(),
                Some("elk.layered")
            );
            assert_eq!(text_of(&options, "elk.direction").as_deref(), Some("RIGHT"));
            assert_eq!(
                text_of(&options, "nodeSize.minimum").as_deref(),
                Some("(30, 0)")
            );
            assert_eq!(warnings.len(), 1);
        }

        #[test]
        fn omits_direction_specific_options_when_node_has_no_dir() {
            let options = build_subgraph_layout_options(
                &SubgraphNode::default(),
                Some(&elk_merge(true)),
                "elk.layered",
                None,
            );
            assert!(!options.contains_key("elk.algorithm"));
            assert!(!options.contains_key("elk.direction"));
            assert!(!options.contains_key("elk.hierarchyHandling"));
        }

        #[test]
        fn passes_through_node_placement_strategy_from_config() {
            let options = build_subgraph_layout_options(
                &SubgraphNode::default(),
                Some(&MermaidConfigElk {
                    node_placement_strategy: Some("BRANDES_KOEPF".to_owned()),
                    ..MermaidConfigElk::default()
                }),
                "elk.layered",
                None,
            );
            assert_eq!(
                text_of(&options, "elk.layered.nodePlacement.strategy").as_deref(),
                Some("BRANDES_KOEPF")
            );
        }

        #[test]
        fn defaults_node_placement_alignment_to_balanced() {
            let options = build_subgraph_layout_options(
                &SubgraphNode::default(),
                Some(&elk_merge(true)),
                "elk.layered",
                None,
            );
            assert_eq!(
                text_of(&options, "elk.layered.nodePlacement.bk.fixedAlignment").as_deref(),
                Some("BALANCED")
            );
        }

        #[test]
        fn passes_through_node_placement_alignment_from_config() {
            let options = build_subgraph_layout_options(
                &SubgraphNode::default(),
                Some(&MermaidConfigElk {
                    node_placement_alignment: Some("BALANCED".to_owned()),
                    ..MermaidConfigElk::default()
                }),
                "elk.layered",
                None,
            );
            assert_eq!(
                text_of(&options, "elk.layered.nodePlacement.bk.fixedAlignment").as_deref(),
                Some("BALANCED")
            );
        }

        #[test]
        fn handles_undefined_elk_config_gracefully() {
            let options =
                build_subgraph_layout_options(&SubgraphNode::default(), None, "elk.layered", None);
            assert!(!options.contains_key("elk.layered.mergeEdges"));
            // With no config at all the `default` preset supplies the placement and alignment.
            assert_eq!(
                text_of(&options, "elk.layered.nodePlacement.strategy").as_deref(),
                Some("BRANDES_KOEPF")
            );
            assert_eq!(
                text_of(&options, "elk.layered.nodePlacement.bk.fixedAlignment").as_deref(),
                Some("BALANCED")
            );
        }

        #[test]
        fn lets_an_explicit_strategy_beat_the_preset() {
            let options = build_subgraph_layout_options(
                &SubgraphNode::default(),
                Some(&MermaidConfigElk {
                    preset: "legacy".to_owned(),
                    node_placement_strategy: Some("SIMPLE".to_owned()),
                    ..MermaidConfigElk::default()
                }),
                "elk.layered",
                None,
            );
            assert_eq!(
                text_of(&options, "elk.layered.nodePlacement.strategy").as_deref(),
                Some("SIMPLE")
            );
        }

        #[test]
        fn takes_the_cycle_breaking_strategy_from_the_named_preset() {
            let breaking = |preset: &str| {
                let options = build_subgraph_layout_options(
                    &SubgraphNode::default(),
                    Some(&MermaidConfigElk {
                        preset: preset.to_owned(),
                        ..MermaidConfigElk::default()
                    }),
                    "elk.layered",
                    None,
                );
                text_of(&options, "elk.layered.cycleBreaking.strategy")
            };
            assert_eq!(breaking("default").as_deref(), Some("DEPTH_FIRST"));
            assert_eq!(
                breaking("modelOrder").as_deref(),
                Some("GREEDY_MODEL_ORDER")
            );
            assert_eq!(breaking("legacy").as_deref(), Some("GREEDY"));
        }

        #[test]
        fn lets_an_explicit_cycle_breaking_strategy_beat_the_preset_in_a_container() {
            let options = build_subgraph_layout_options(
                &SubgraphNode::default(),
                Some(&MermaidConfigElk {
                    preset: "legacy".to_owned(),
                    cycle_breaking_strategy: Some("MODEL_ORDER".to_owned()),
                    ..MermaidConfigElk::default()
                }),
                "elk.layered",
                None,
            );
            assert_eq!(
                text_of(&options, "elk.layered.cycleBreaking.strategy").as_deref(),
                Some("MODEL_ORDER")
            );
        }

        #[test]
        fn resolves_container_cycle_breaking_to_the_same_value_as_the_root() {
            for preset in ["default", "legacy", "modelOrder", "depthFirst"] {
                let options = build_subgraph_layout_options(
                    &SubgraphNode::default(),
                    Some(&MermaidConfigElk {
                        preset: preset.to_owned(),
                        ..MermaidConfigElk::default()
                    }),
                    "elk.layered",
                    None,
                );
                assert_eq!(
                    text_of(&options, "elk.layered.cycleBreaking.strategy").as_deref(),
                    Some(resolve_elk_preset(Some(preset)).cycle_breaking)
                );
            }
        }

        #[test]
        fn takes_the_container_placement_strategy_from_the_named_preset() {
            let placement = |preset: &str| {
                let options = build_subgraph_layout_options(
                    &SubgraphNode::default(),
                    Some(&MermaidConfigElk {
                        preset: preset.to_owned(),
                        ..MermaidConfigElk::default()
                    }),
                    "elk.layered",
                    None,
                );
                text_of(&options, "elk.layered.nodePlacement.strategy")
            };
            assert_eq!(placement("legacy").as_deref(), Some("BRANDES_KOEPF"));
            assert_eq!(placement("depthFirst").as_deref(), Some("BRANDES_KOEPF"));
            assert_eq!(placement("default").as_deref(), Some("BRANDES_KOEPF"));
        }

        #[test]
        fn names_the_placement_option_once_fully_qualified() {
            let options = build_subgraph_layout_options(
                &SubgraphNode::default(),
                Some(&MermaidConfigElk {
                    node_placement_strategy: Some("SIMPLE".to_owned()),
                    ..MermaidConfigElk::default()
                }),
                "elk.layered",
                None,
            );
            assert!(!options.contains_key("nodePlacement.strategy"));
            assert_eq!(
                text_of(&options, "elk.layered.nodePlacement.strategy").as_deref(),
                Some("SIMPLE")
            );
        }

        #[test]
        fn applies_a_per_group_algorithm_from_metadata_with_separate_children() {
            let box_algorithm = algorithm("elk.box");
            let options = build_subgraph_layout_options(
                &SubgraphNode {
                    label_data: LabelData {
                        width: 30.0,
                        height: 14.0,
                        wrapping_width: 0.0,
                    },
                    metadata_algorithm: Some(&box_algorithm),
                    ..SubgraphNode::default()
                },
                None,
                "elk.layered",
                None,
            );
            assert_eq!(
                text_of(&options, "elk.algorithm").as_deref(),
                Some("elk.box")
            );
            assert_eq!(
                text_of(&options, "elk.hierarchyHandling").as_deref(),
                Some("SEPARATE_CHILDREN")
            );
            assert_eq!(
                text_of(&options, "elk.padding").as_deref(),
                Some("[top=29,left=15,bottom=15,right=15]")
            );
        }

        #[test]
        fn metadata_algorithm_takes_precedence_over_dir() {
            let box_algorithm = algorithm("elk.box");
            let options = build_subgraph_layout_options(
                &SubgraphNode {
                    dir: Some("LR"),
                    metadata_algorithm: Some(&box_algorithm),
                    ..SubgraphNode::default()
                },
                None,
                "elk.layered",
                None,
            );
            assert_eq!(
                text_of(&options, "elk.algorithm").as_deref(),
                Some("elk.box")
            );
            assert!(!options.contains_key("elk.direction"));
        }

        #[test]
        fn applies_tighter_rectpacking_options_for_elk_rectpacking_groups() {
            let rectpacking = algorithm("elk.rectpacking");
            let options = build_subgraph_layout_options(
                &SubgraphNode {
                    label_data: LabelData {
                        width: 30.0,
                        height: 14.0,
                        wrapping_width: 0.0,
                    },
                    metadata_algorithm: Some(&rectpacking),
                    ..SubgraphNode::default()
                },
                None,
                "elk.layered",
                None,
            );
            assert_eq!(
                text_of(&options, "elk.rectpacking.trybox").as_deref(),
                Some("true")
            );
            assert_eq!(
                text_of(&options, "elk.padding").as_deref(),
                Some("[top=24,left=10,bottom=10,right=10]")
            );
            // The minimum height clears the tighter rectpacking strip, not the default one.
            assert_eq!(
                text_of(&options, "nodeSize.minimum").as_deref(),
                Some("(30, 34)")
            );
            assert_eq!(number_of(&options, "spacing.baseValue"), Some(15.0));
        }
    }

    mod find_cyclic_entry_nodes {
        use super::*;

        #[test]
        fn returns_nothing_for_an_acyclic_flow_natural_source_exists() {
            let nodes = [node("a", None), node("b", None), node("c", None)];
            let edges = [edge("e1", "a", "b"), edge("e2", "b", "c")];
            assert!(find_cyclic_entry_nodes(&nodes, &edges).is_empty());
        }

        #[test]
        fn nominates_the_first_declared_node_of_a_source_less_cycle() {
            // a -> b -> c -> a : pure recursion, no in-degree-0 node
            let nodes = [node("a", None), node("b", None), node("c", None)];
            let edges = [
                edge("e1", "a", "b"),
                edge("e2", "b", "c"),
                edge("e3", "c", "a"),
            ];
            assert_eq!(
                find_cyclic_entry_nodes(&nodes, &edges),
                vec!["a".to_owned()]
            );
        }

        #[test]
        fn pins_the_entry_of_a_recursive_flow_reported_recursion_case() {
            // format_output is an orphan; format_output2 is the real sink.
            let nodes: Vec<Node> = [
                "brief",
                "web",
                "academic",
                "expert",
                "synthesize",
                "format_output",
                "decision",
                "format_output2",
            ]
            .iter()
            .map(|id| node(id, Some("research")))
            .collect();
            let edges = [
                edge("e1", "brief", "web"),
                edge("e2", "web", "academic"),
                edge("e3", "academic", "expert"),
                edge("e4", "expert", "synthesize"),
                edge("e5", "synthesize", "decision"),
                edge("e6", "decision", "format_output2"),
                edge("e7", "decision", "brief"),
            ];
            let entries = find_cyclic_entry_nodes(&nodes, &edges);
            assert_eq!(entries, vec!["brief".to_owned()]);
        }

        #[test]
        fn pins_the_true_entry_when_a_back_edge_feeds_it_and_it_is_not_declared_first() {
            // The chain starts at stockholm; end_decision -> stockholm gives the entry an in-degree
            // of 1 while san_francisco is declared first. Nomination follows edge order.
            let nodes: Vec<Node> = [
                "san_francisco",
                "stockholm",
                "new_york",
                "decide",
                "end_decision",
                "format_json",
            ]
            .iter()
            .map(|id| node(id, Some("world-clock")))
            .collect();
            let edges = [
                edge("e1", "stockholm", "new_york"),
                edge("e2", "new_york", "san_francisco"),
                edge("e3", "san_francisco", "decide"),
                edge("e4", "decide", "end_decision"),
                edge("e5", "end_decision", "format_json"),
                edge("e6", "end_decision", "stockholm"),
            ];
            assert_eq!(
                find_cyclic_entry_nodes(&nodes, &edges),
                vec!["stockholm".to_owned()]
            );
        }

        #[test]
        fn scopes_detection_per_container_parent_id() {
            // A self-contained cycle inside subgraph "sub"; an acyclic chain at the root.
            let nodes = [
                node("root_a", None),
                node("root_b", None),
                node("x", Some("sub")),
                node("y", Some("sub")),
            ];
            let edges = [
                edge("e1", "root_a", "root_b"),
                edge("e2", "x", "y"),
                edge("e3", "y", "x"),
            ];
            assert_eq!(
                find_cyclic_entry_nodes(&nodes, &edges),
                vec!["x".to_owned()]
            );
        }

        #[test]
        fn ignores_self_loops_when_deciding_whether_a_node_is_a_source() {
            let nodes = [node("a", None), node("b", None)];
            let edges = [edge("e1", "a", "a"), edge("e2", "a", "b")];
            // 'a' is still a source, so nothing is nominated.
            assert!(find_cyclic_entry_nodes(&nodes, &edges).is_empty());
        }
    }

    mod dir2_elk_direction {
        use super::*;

        #[test]
        fn maps_lr_to_right() {
            assert_eq!(dir2_elk_direction("LR"), "RIGHT");
        }

        #[test]
        fn maps_rl_to_left() {
            assert_eq!(dir2_elk_direction("RL"), "LEFT");
        }

        #[test]
        fn maps_tb_and_td_to_down() {
            assert_eq!(dir2_elk_direction("TB"), "DOWN");
            assert_eq!(dir2_elk_direction("TD"), "DOWN");
        }

        #[test]
        fn maps_bt_to_up() {
            assert_eq!(dir2_elk_direction("BT"), "UP");
        }

        #[test]
        fn defaults_to_down_for_unknown() {
            assert_eq!(dir2_elk_direction("xyz"), "DOWN");
        }
    }

    mod build_elk_graph_from_layout_data {
        use super::*;

        /// The layout data of a directional graph with a subgraph and one labelled edge.
        fn directional() -> LayoutData {
            LayoutData {
                direction: "LR".to_owned(),
                nodes: vec![
                    subgraph("group", None, Some((44.0, 16.0))),
                    Node {
                        width: Some(50.0),
                        height: Some(20.0),
                        ..node("A", Some("group"))
                    },
                    Node {
                        width: Some(60.0),
                        height: Some(24.0),
                        ..node("B", None)
                    },
                ],
                edges: vec![Edge {
                    width: Some(22.0),
                    height: Some(10.0),
                    label: "go".to_owned(),
                    ..edge("edge-A-B", "A", "B")
                }],
                ..LayoutData::default()
            }
        }

        #[test]
        fn builds_an_elk_graph_from_measured_layout_data_without_dom_handles() {
            let graph = build(&directional(), &config(|elk| elk.merge_edges = true)).graph;
            let root = graph.layout_options.as_ref().expect("root options");
            assert_eq!(text_of(root, "elk.direction").as_deref(), Some("RIGHT"));
            assert_eq!(
                text_of(root, "elk.layered.nodePlacement.bk.fixedAlignment").as_deref(),
                Some("BALANCED")
            );
            assert_eq!(graph.children.len(), 2);

            let group = find(&graph, "group").expect("the group");
            assert_eq!(group.children.len(), 1);
            let title = &group.labels[0];
            assert_eq!(title.text.as_deref(), Some("group"));
            assert_eq!(title.width, Some(44.0));
            assert_eq!(title.height, Some(14.0));
            assert_eq!(group.width, None);

            let edge = &graph.edges[0];
            assert_eq!(edge.labels[0].width, Some(22.0));
            assert_eq!(edge.labels[0].height, Some(10.0));
            assert_eq!(edge.labels[0].text.as_deref(), Some("go"));
        }

        #[test]
        fn passes_through_node_placement_alignment_to_the_root_graph() {
            let data = LayoutData {
                direction: "TB".to_owned(),
                ..LayoutData::default()
            };
            let graph = build(
                &data,
                &config(|elk| elk.node_placement_alignment = Some("BALANCED".to_owned())),
            )
            .graph;
            assert_eq!(
                text_of(
                    graph.layout_options.as_ref().expect("root options"),
                    "elk.layered.nodePlacement.bk.fixedAlignment"
                )
                .as_deref(),
                Some("BALANCED")
            );
        }

        /// Three nodes in a loop, with no entry: a recursive flow.
        fn recursive() -> LayoutData {
            LayoutData {
                direction: "TB".to_owned(),
                nodes: vec![
                    Node {
                        width: Some(50.0),
                        height: Some(20.0),
                        ..node("a", None)
                    },
                    Node {
                        width: Some(50.0),
                        height: Some(20.0),
                        ..node("b", None)
                    },
                    Node {
                        width: Some(50.0),
                        height: Some(20.0),
                        ..node("c", None)
                    },
                ],
                edges: vec![
                    edge("e1", "a", "b"),
                    edge("e2", "b", "c"),
                    edge("e3", "c", "a"),
                ],
                ..LayoutData::default()
            }
        }

        #[test]
        fn pins_the_cyclic_entry_node_to_the_first_layer_when_keep_entry_node_on_top_is_enabled() {
            let graph = build(
                &recursive(),
                &config(|elk| elk.keep_entry_node_on_top = true),
            )
            .graph;
            let constraint = "elk.layered.layering.layerConstraint";
            let option = |id: &str| {
                find(&graph, id)
                    .and_then(|node| node.layout_options.as_ref())
                    .and_then(|options| text_of(options, constraint))
            };
            assert_eq!(option("a").as_deref(), Some("FIRST"));
            assert_eq!(option("b"), None);
            assert_eq!(option("c"), None);
        }

        #[test]
        fn does_not_constrain_any_node_when_keep_entry_node_on_top_is_disabled_default() {
            let graph = build(&recursive(), &config(|_| {})).graph;
            for id in ["a", "b", "c"] {
                let options = find(&graph, id).and_then(|node| node.layout_options.as_ref());
                assert!(!options.is_some_and(|options| {
                    options.contains_key("elk.layered.layering.layerConstraint")
                }));
            }
        }
    }

    mod clear_container_algorithm_options {
        use super::*;

        #[test]
        fn removes_every_option_the_algorithm_branch_added() {
            let box_algorithm = algorithm("elk.box");
            let mut options = build_subgraph_layout_options(
                &SubgraphNode {
                    padding: 8.0,
                    label_data: LabelData {
                        width: 44.0,
                        height: 14.0,
                        wrapping_width: 0.0,
                    },
                    metadata_algorithm: Some(&box_algorithm),
                    ..SubgraphNode::default()
                },
                Some(&elk_merge(true)),
                "elk.layered",
                None,
            );
            clear_container_algorithm_options(&mut options);
            for key in [
                "elk.algorithm",
                "nodeSize.constraints",
                "nodeSize.minimum",
                "elk.aspectRatio",
                "elk.contentAlignment",
                "elk.expandNodes",
                "elk.padding",
            ] {
                assert!(!options.contains_key(key), "{key} is still set");
            }
        }

        #[test]
        fn removes_the_rectpacking_overrides_and_restores_the_default_base_spacing() {
            let rectpacking = algorithm("elk.rectpacking");
            let subgraph_node = SubgraphNode {
                padding: 8.0,
                label_data: LabelData {
                    width: 44.0,
                    height: 14.0,
                    wrapping_width: 0.0,
                },
                metadata_algorithm: Some(&rectpacking),
                ..SubgraphNode::default()
            };
            let mut options = build_subgraph_layout_options(
                &subgraph_node,
                Some(&elk_merge(true)),
                "elk.layered",
                None,
            );
            assert_eq!(number_of(&options, "spacing.baseValue"), Some(15.0));

            clear_container_algorithm_options(&mut options);
            assert_eq!(number_of(&options, "spacing.baseValue"), Some(24.0));
            // Restored too, rather than left deleted.
            assert_eq!(number_of(&options, "spacing.nodeNode"), Some(50.0));
            assert!(!options.contains_key("elk.rectpacking.trybox"));
        }

        #[test]
        fn leaves_the_base_options_alone() {
            let box_algorithm = algorithm("elk.box");
            let mut options = build_subgraph_layout_options(
                &SubgraphNode {
                    padding: 8.0,
                    label_data: LabelData {
                        width: 44.0,
                        height: 14.0,
                        wrapping_width: 0.0,
                    },
                    metadata_algorithm: Some(&box_algorithm),
                    ..SubgraphNode::default()
                },
                Some(&MermaidConfigElk {
                    merge_edges: true,
                    node_placement_strategy: Some("BRANDES_KOEPF".to_owned()),
                    ..MermaidConfigElk::default()
                }),
                "elk.layered",
                None,
            );
            clear_container_algorithm_options(&mut options);
            assert_eq!(
                options.get("elk.layered.mergeEdges"),
                Some(&Json::Bool(true))
            );
            assert_eq!(
                text_of(&options, "elk.layered.nodePlacement.strategy").as_deref(),
                Some("BRANDES_KOEPF")
            );
            assert_eq!(
                text_of(&options, "nodeLabels.placement").as_deref(),
                Some("[H_CENTER V_TOP, INSIDE]")
            );
        }
    }

    mod resolve_elk_preset {
        use super::*;

        #[test]
        fn breaks_cycles_depth_first_by_default() {
            assert_eq!(resolve_elk_preset(None).cycle_breaking, "DEPTH_FIRST");
            assert_eq!(
                resolve_elk_preset(Some("default")).cycle_breaking,
                "DEPTH_FIRST"
            );
            assert_eq!(
                resolve_elk_preset(Some("modelOrder")).cycle_breaking,
                "GREEDY_MODEL_ORDER"
            );
            assert_eq!(resolve_elk_preset(Some("legacy")).cycle_breaking, "GREEDY");
        }

        #[test]
        fn preserves_depth_first_and_model_order_placement_independently_of_default() {
            for name in ["depthFirst", "modelOrder"] {
                let preset = resolve_elk_preset(Some(name));
                assert_eq!(preset.layering, "NETWORK_SIMPLEX");
                assert_eq!(preset.placement, "NETWORK_SIMPLEX");
                assert_eq!(preset.container_placement, "BRANDES_KOEPF");
                assert_eq!(preset.alignment, "NONE");
            }
        }

        #[test]
        fn falls_back_to_default_for_an_unknown_name_including_proto() {
            assert_eq!(
                resolve_elk_preset(Some("nope")),
                resolve_elk_preset(Some("default"))
            );
            assert_eq!(
                resolve_elk_preset(Some("__proto__")),
                resolve_elk_preset(Some("default"))
            );
        }
    }

    /// The `ELK preset configuration through initialize` cases of `defaultPreset.spec.ts`: the
    /// graph built for a group and a child, under each preset.
    mod elk_preset_configuration_through_initialize {
        use super::*;

        /// The graph of a group with one member, under the given settings.
        fn graph_for(settings: impl FnOnce(&mut MermaidConfigElk)) -> JsonNode {
            let data = LayoutData {
                direction: "TB".to_owned(),
                nodes: vec![
                    subgraph("group", None, None),
                    Node {
                        width: Some(100.0),
                        height: Some(40.0),
                        ..node("child", Some("group"))
                    },
                ],
                edges: Vec::new(),
                ..LayoutData::default()
            };
            build(&data, &config(settings)).graph
        }

        #[test]
        fn centers_omitted_default_preset() {
            for preset in [None, Some("default")] {
                let graph = graph_for(|elk| {
                    if let Some(preset) = preset {
                        elk.preset = preset.to_owned();
                    }
                });
                let group = find(&graph, "group").expect("the group");
                for options in [graph.layout_options.as_ref(), group.layout_options.as_ref()] {
                    let options = options.expect("options");
                    assert_eq!(
                        text_of(options, "elk.layered.nodePlacement.strategy").as_deref(),
                        Some("BRANDES_KOEPF")
                    );
                    assert_eq!(
                        text_of(options, "elk.layered.nodePlacement.bk.fixedAlignment").as_deref(),
                        Some("BALANCED")
                    );
                }
            }
        }

        #[test]
        fn preserves_the_legacy_preset() {
            assert_preset("legacy", "BRANDES_KOEPF", "GREEDY");
        }

        #[test]
        fn preserves_the_model_order_preset() {
            assert_preset("modelOrder", "NETWORK_SIMPLEX", "GREEDY_MODEL_ORDER");
        }

        #[test]
        fn preserves_the_depth_first_preset() {
            assert_preset("depthFirst", "NETWORK_SIMPLEX", "DEPTH_FIRST");
        }

        /// The root takes the preset's placement, the group keeps the container placement, and both
        /// use the preset's alignment and cycle breaking.
        fn assert_preset(preset: &str, placement: &str, cycle_breaking: &str) {
            let graph = graph_for(|elk| elk.preset = preset.to_owned());
            let group = find(&graph, "group").expect("the group");
            let root = graph.layout_options.as_ref().expect("root options");
            assert_eq!(
                text_of(root, "elk.layered.nodePlacement.strategy").as_deref(),
                Some(placement)
            );
            assert_eq!(
                text_of(
                    group.layout_options.as_ref().expect("group options"),
                    "elk.layered.nodePlacement.strategy"
                )
                .as_deref(),
                Some("BRANDES_KOEPF")
            );
            assert_eq!(
                text_of(root, "elk.layered.layering.strategy").as_deref(),
                Some("NETWORK_SIMPLEX")
            );
            for options in [root, group.layout_options.as_ref().expect("group options")] {
                assert_eq!(
                    text_of(options, "elk.layered.nodePlacement.bk.fixedAlignment").as_deref(),
                    Some("NONE")
                );
                assert_eq!(
                    text_of(options, "elk.layered.cycleBreaking.strategy").as_deref(),
                    Some(cycle_breaking)
                );
            }
        }

        #[test]
        fn honors_explicit_placement_and_none_alignment_with_each_preset() {
            for preset in ["default", "legacy", "modelOrder", "depthFirst"] {
                let graph = graph_for(|elk| {
                    elk.preset = preset.to_owned();
                    elk.node_placement_strategy = Some("SIMPLE".to_owned());
                    elk.node_placement_alignment = Some("NONE".to_owned());
                });
                let group = find(&graph, "group").expect("the group");
                for options in [
                    graph.layout_options.as_ref().expect("root options"),
                    group.layout_options.as_ref().expect("group options"),
                ] {
                    assert_eq!(
                        text_of(options, "elk.layered.nodePlacement.strategy").as_deref(),
                        Some("SIMPLE")
                    );
                    assert_eq!(
                        text_of(options, "elk.layered.nodePlacement.bk.fixedAlignment").as_deref(),
                        Some("NONE")
                    );
                }
            }
        }

        #[test]
        fn allows_alignment_to_be_overridden_without_replacing_default_placement() {
            let graph = graph_for(|elk| elk.node_placement_alignment = Some("LEFTUP".to_owned()));
            let group = find(&graph, "group").expect("the group");
            for options in [
                graph.layout_options.as_ref().expect("root options"),
                group.layout_options.as_ref().expect("group options"),
            ] {
                assert_eq!(
                    text_of(options, "elk.layered.nodePlacement.strategy").as_deref(),
                    Some("BRANDES_KOEPF")
                );
                assert_eq!(
                    text_of(options, "elk.layered.nodePlacement.bk.fixedAlignment").as_deref(),
                    Some("LEFTUP")
                );
            }
        }

        #[test]
        #[ignore = "port: needs runElkLayoutCore, the ELK run and the post-layout passes (T5)"]
        fn centers_the_initial_transition_to_tv_with_defaults_and_keeps_the_entry_straight() {}
    }
}
