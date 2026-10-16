//! The steps of the unified renderer after a diagram's `getData`: the edge render data, the measure
//! step of `createGraphWithElements`, the elkjs graph and ELK Layered, the post-processing of the
//! result, and the paint of `paintLayoutData` (`rendering-util/render.ts`,
//! `layout-algorithms/common/index.ts`, `createGraph.ts` and `elk/render.ts`).
//!
//! A diagram on the unified renderer (flowchart, class, state, ER) builds its layout data, runs
//! [`measure`], [`lay_out`] and [`paint`], and writes the root `<svg>` around the painted groups,
//! with its own markers, style sheet and title.

use std::collections::HashMap;

use ferrigura_elk::json::JsonNode;
use ferrigura_model::config::MermaidConfig;
use ferrigura_model::flowchart::LabelType;
use ferrigura_model::value::Value;

use crate::RenderError;
use crate::unified::data::{LayoutData, Node};
use crate::unified::geom::BBox;
use crate::unified::label::LabelLayout;
use crate::unified::shapes::{self, ShapeInput, ShapeOutput};
use crate::unified::{clusters, edges, elk_apply, elk_build};
use crate::variables::px;

/// The graph handed to ELK and the graph it returned, for the differential comparison with the
/// reference's capture (`tools/port/geometry/flowchart-geometry.ts`).
#[derive(Debug, Clone)]
pub struct ElkStages {
    /// The input of the layout.
    pub input: JsonNode,
    /// The output of the layout.
    pub output: JsonNode,
}

/// The diagram after the measure step: the layout data with every size the layout needs, and
/// what was drawn to measure it.
#[derive(Debug, Clone)]
pub struct Measured {
    /// The layout data, with the sizes of the measure step and, after [`lay_out`], the positions.
    pub data: LayoutData,
    /// The shape of every leaf node, by id.
    pub shapes: HashMap<String, ShapeOutput>,
    /// The label of every edge that has one, by edge id.
    pub edge_labels: HashMap<String, LabelLayout>,
}

/// What [`paint`] drew: the groups below the root, the box they cover, and every label, so that
/// the caller can subset the fonts once for the whole diagram.
#[derive(Debug)]
pub struct Painted<'a> {
    /// `<g class="root">` with the clusters, the edge paths, the edge labels and the nodes.
    pub root: String,
    /// The markers in an edge's own colour, which the reference appends after the root group.
    pub coloured_markers: String,
    /// Every box the painted elements cover: what `getBBox` of the svg gives, before the title.
    pub bounds: Option<BBox>,
    /// The cluster titles that were drawn.
    pub cluster_labels: Vec<LabelLayout>,
    /// The node and edge labels that were drawn.
    pub labels: Vec<&'a LabelLayout>,
}

impl Painted<'_> {
    /// Every label that was drawn: the cluster titles, then the node and edge labels.
    pub fn all_labels(&self) -> impl Iterator<Item = &LabelLayout> {
        self.cluster_labels
            .iter()
            .chain(self.labels.iter().copied())
    }
}

/// The nearest `dir` of the node or one of its subgraphs, else the diagram's direction, as
/// `resolveNodeDir` in `createGraph.ts` finds it.
fn node_dir(data: &LayoutData, index: usize) -> Option<&str> {
    let by_id: HashMap<&str, &Node> = data
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();
    let mut current = Some(&data.nodes[index]);
    let mut seen = std::collections::HashSet::new();
    while let Some(node) = current {
        if let Some(dir) = node.dir.as_deref() {
            return Some(dir);
        }
        let parent = node.parent_id.as_deref()?;
        if !seen.insert(parent) {
            break;
        }
        current = by_id.get(parent).copied();
    }
    Some(data.direction.as_str()).filter(|dir| !dir.is_empty())
}

/// The edge render data and the measure step of `createGraphWithElements`, for the layout data
/// of a diagram's `getData`. `theme` is the set of theme variables that gives the sizes.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read or a shape is unknown.
pub fn measure(
    mut data: LayoutData,
    config: &MermaidConfig,
    theme: &Value,
) -> Result<Measured, RenderError> {
    edges::prepare(&mut data, config);

    let mut shapes = HashMap::new();
    for index in 0..data.nodes.len() {
        let node = &data.nodes[index];
        if node.is_group {
            // ELK sizes a subgraph from its title, measured unwrapped unless it is Markdown. The
            // title's box is already a float32 value (`node_label`).
            let width = (node.label_type != LabelType::Markdown).then_some(f64::INFINITY);
            let size = shapes::node_label(node, config, theme, width)?
                .map_or((0.0, 0.0), |label| (label.bbox.width, label.bbox.height));
            data.nodes[index].label_bbox = Some(size);
            continue;
        }
        let label = shapes::node_label(node, config, theme, None)?;
        let shape = shapes::draw(&ShapeInput {
            node,
            label: label.as_ref(),
            config,
            theme,
            dir: node_dir(&data, index),
        })?;
        let id = node.id.clone();
        // `insertMeasuredNode` takes the node's size from the `getBBox` of its group (createGraph.ts
        // 61-71, line 67), so it is a float32 value.
        let bbox = shape.bbox.browser();
        data.nodes[index].width = Some(bbox.width);
        data.nodes[index].height = Some(bbox.height);
        shapes.insert(id, shape);
    }

    let mut edge_labels = HashMap::new();
    let font_size = px(theme, "fontSize").unwrap_or(DEFAULT_FONT_SIZE);
    for edge in &mut data.edges {
        if let Some(label) = edges::label_with_font_size(edge, font_size)? {
            let (width, height) = edges::label_size(&label);
            edge.width = Some(width);
            edge.height = Some(height);
            edge_labels.insert(edge.id.clone(), label);
        }
    }

    Ok(Measured {
        data,
        shapes,
        edge_labels,
    })
}

/// Lays the measured diagram out with ELK and applies the result.
///
/// # Errors
///
/// Returns an error if the layout fails.
pub fn lay_out(measured: &mut Measured, config: &MermaidConfig) -> Result<ElkStages, RenderError> {
    let elk = elk_build::build(&measured.data, config);
    let layout = ferrigura_elk::layout(&elk.graph, false)
        .map_err(|error| RenderError::Layout(error.to_string()))?;
    let intersects = measured
        .shapes
        .iter()
        .map(|(id, shape)| (id.clone(), shape.intersect.clone()))
        .collect();
    elk_apply::apply(
        &mut measured.data,
        &layout.graph,
        &elk.state,
        &intersects,
        config,
    );
    Ok(ElkStages {
        input: elk.graph,
        output: layout.graph,
    })
}

/// Paints a laid-out diagram in the order of `paintLayoutData`: the clusters in the order
/// `orderNodesForElkPaint` left, then the nodes as the measure step inserted them, then the edges
/// and their labels. `id` is the diagram's id, which prefixes every DOM id.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read.
pub fn paint<'a>(
    measured: &'a Measured,
    id: &str,
    config: &MermaidConfig,
    theme: &Value,
) -> Result<Painted<'a>, RenderError> {
    let data = &measured.data;
    let mut bounds: Option<BBox> = None;
    let mut labels: Vec<&LabelLayout> = Vec::new();

    let mut clusters = String::new();
    let mut painted_clusters = Vec::new();
    for node in data.nodes.iter().filter(|node| node.is_group) {
        // `render` prefixes every node's DOM id with the diagram's id before the layout runs.
        let prefixed = Node {
            dom_id: format!("{id}-{}", node.dom_id),
            ..node.clone()
        };
        let cluster = clusters::paint(&prefixed, config, theme)?;
        // The frame of an empty subgraph has no height: an unrendered rect, which `getBBox` leaves
        // out of the drawing's box. Its title is text and counts.
        if !cluster.bbox.is_empty() {
            cover(&mut bounds, cluster.bbox);
        }
        if !cluster.title_box.is_empty() {
            cover(&mut bounds, cluster.title_box);
        }
        clusters.push_str(&cluster.svg);
        painted_clusters.push(cluster);
    }

    let mut nodes = String::new();
    for node in data.nodes.iter().filter(|node| !node.is_group) {
        let Some(shape) = measured.shapes.get(&node.id) else {
            continue;
        };
        let (x, y) = (node.x.unwrap_or(0.0), node.y.unwrap_or(0.0));
        cover(&mut bounds, shape.bbox.translate(x, y));
        nodes.push_str(&shapes::node_svg(node, shape, id));
        labels.extend(shape.label.as_ref());
    }

    let mut edge_paths = String::new();
    let mut coloured_markers = String::new();
    for path in edges::paint_paths(&data.edges, id, config) {
        cover(&mut bounds, path.bbox);
        edge_paths.push_str(&path.svg);
        coloured_markers.push_str(&path.markers);
    }
    let mut edge_labels = String::new();
    for edge in &data.edges {
        if let Some(label) = measured.edge_labels.get(&edge.id) {
            let painted = edges::paint_label(edge, label, config);
            cover(&mut bounds, painted.bbox);
            edge_labels.push_str(&painted.svg);
            labels.push(label);
        }
    }

    let cluster_labels = painted_clusters
        .into_iter()
        .filter_map(|cluster| cluster.label)
        .collect();

    Ok(Painted {
        root: format!(
            "<g class=\"root\"><g class=\"clusters\">{clusters}</g><g class=\"edges edgePaths\">{edge_paths}</g><g class=\"edgeLabels\">{edge_labels}</g><g class=\"nodes\">{nodes}</g></g>"
        ),
        coloured_markers,
        bounds,
        cluster_labels,
        labels,
    })
}

/// Adds a painted box to the boxes the viewBox covers.
pub fn cover(bounds: &mut Option<BBox>, bbox: BBox) {
    *bounds = Some(bounds.map_or(bbox, |all| all.union(bbox)));
}

/// The font size of labels when the theme has none.
const DEFAULT_FONT_SIZE: f64 = 16.0;
