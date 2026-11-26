//! The steps of the unified renderer after a diagram's `getData`: the edge render data, the measure
//! step of `createGraphWithElements`, the elkjs graph and ELK Layered, the post-processing of the
//! result, and the paint of `paintLayoutData` (`rendering-util/render.ts`,
//! `layout-algorithms/common/index.ts`, `createGraph.ts` and `elk/render.ts`).
//!
//! A diagram on the unified renderer (flowchart, class, state, ER) builds its layout data, runs
//! [`measure`], [`lay_out`] and [`paint`], and writes the root `<svg>` around the painted groups,
//! with its own markers, style sheet and title.

use std::borrow::Cow;
use std::collections::HashMap;

use ferrigura_elk::json::JsonNode;
use ferrigura_model::flowchart::LabelType;

use crate::RenderError;
use crate::unified::data::{Edge, LayoutData, Node};
use crate::unified::geom::BBox;
use crate::unified::label::{Anchor, LabelLayout};
use crate::unified::line_jumps::JumpStyle;
use crate::unified::shapes::{self, ShapeInput, ShapeOutput};
use crate::unified::{Config, clusters, edges, elk_apply, elk_build, swimlane_layout, terminal};

/// The layout that placed the nodes of a diagram. It decides how the paint clips the edges and which
/// line hops it draws: `insertEdge` takes the swimlane branch for the `swimlane` layout (`edges.js`
/// 654-700), and the hops are the layout's own (`elk.lineHops`, or `swimlane.lineHops` for the swimlane
/// engine).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// ELK Layered: the flowchart, the other unified diagrams, and a swimlane diagram that its
    /// configuration lays out with ELK.
    Elk,
    /// The swimlane engine, with the line hops of `swimlane.lineHops` (`None` when they are off).
    Swimlane(Option<JumpStyle>),
}

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
    /// The layout that places the nodes, which [`paint`] follows.
    pub layout: Layout,
}

/// What [`paint`] drew: the groups below the root, the box they cover, and every label, so that
/// the caller can subset the fonts once for the whole diagram.
#[derive(Debug)]
pub struct Painted<'a> {
    /// `<g class="root">` with the clusters, the edge paths, the edge labels and the nodes.
    pub root: String,
    /// The markers in an edge's own color, which the reference appends after the root group.
    pub coloured_markers: String,
    /// Every box the painted elements cover: what `getBBox` of the svg gives, before the title.
    pub bounds: Option<BBox>,
    /// The cluster titles that were drawn.
    pub cluster_labels: Vec<LabelLayout>,
    /// The node and edge labels that were drawn.
    pub labels: Vec<&'a LabelLayout>,
    /// Whether a node is wrapped in a link, so that the root declares the `xlink` prefix.
    pub links: bool,
}

#[derive(Debug)]
pub(crate) struct RootParts {
    clusters: String,
    edge_paths: String,
    edge_labels: String,
    nodes: String,
    animation: String,
}

impl RootParts {
    pub(crate) fn slices(&self) -> [&str; 11] {
        [
            "<g class=\"root\"><g class=\"clusters\">",
            &self.clusters,
            "</g><g class=\"edges edgePaths\">",
            &self.edge_paths,
            "</g><g class=\"edgeLabels\">",
            &self.edge_labels,
            "</g><g class=\"nodes\">",
            &self.nodes,
            "</g>",
            &self.animation,
            "</g>",
        ]
    }
}

pub(crate) struct PaintedParts<'a> {
    pub root: RootParts,
    pub coloured_markers: String,
    pub bounds: Option<BBox>,
    pub cluster_labels: Vec<LabelLayout>,
    pub labels: Vec<&'a LabelLayout>,
    pub links: bool,
}

impl PaintedParts<'_> {
    pub(crate) fn all_labels(&self) -> impl Iterator<Item = &LabelLayout> {
        self.cluster_labels
            .iter()
            .chain(self.labels.iter().copied())
    }
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
pub(crate) fn node_dir<'a>(
    data: &'a LayoutData,
    index: usize,
    by_id: &HashMap<&str, &'a Node>,
) -> Option<&'a str> {
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

/// For each node, the length its busiest side needs in the design mode
/// ([`crate::design::side_length`]) and whether that side runs along the node's height: it does in
/// a layout from left to right, where edges meet the left and the right side, and it runs along
/// the width in a layout from top to bottom. The edges into a node meet one side and the edges out
/// of it the other; a self-loop is left out.
fn busiest_sides(data: &LayoutData) -> Vec<(f64, bool)> {
    let by_id: HashMap<&str, &Node> = data
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();
    let mut degrees: HashMap<&str, (usize, usize)> = HashMap::new();
    // The connector of a state's note is no transition: the note stands beside its state, and its
    // connector meets a side that no transition does.
    let is_note = |id: &str| {
        by_id
            .get(id)
            .is_some_and(|node| node.css_classes.contains("statediagram-note"))
    };
    for edge in &data.edges {
        if edge.start == edge.end || is_note(&edge.start) || is_note(&edge.end) {
            continue;
        }
        degrees.entry(edge.start.as_str()).or_default().1 += 1;
        degrees.entry(edge.end.as_str()).or_default().0 += 1;
    }
    data.nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let (into, out) = degrees.get(node.id.as_str()).copied().unwrap_or_default();
            // `node_dir` gives a node without a parent no direction; the diagram's is meant here.
            let direction = node_dir(data, index, &by_id).unwrap_or(data.direction.as_str());
            let along_height = matches!(direction, "LR" | "RL");
            (crate::design::side_length(into.max(out)), along_height)
        })
        .collect()
}

/// The edge render data and the measure step of `createGraphWithElements`, for the layout data
/// of a diagram's `getData`, laid out by ELK. `config` gives the sections and the globals, and the
/// theme's tokens give the sizes.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read or a shape is unknown.
pub fn measure(data: LayoutData, config: &Config<'_>) -> Result<Measured, RenderError> {
    measure_for(data, config, Layout::Elk)
}

/// [`measure`], for the layout that will place the nodes. The swimlane engine's groups are measured
/// wrapped, as `createGraphWithElements` does without `unwrapGroupLabels` (`createGraph.ts` 42-59).
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read or a shape is unknown.
pub fn measure_for(
    mut data: LayoutData,
    config: &Config<'_>,
    layout: Layout,
) -> Result<Measured, RenderError> {
    edges::prepare(&mut data);

    // Direction is only consumed by fork/join shapes. Resolve those before mutating sizes,
    // building the parent index once instead of once per node.
    let directions = if data
        .nodes
        .iter()
        .any(|node| !node.is_group && shapes::uses_direction(node))
    {
        let by_id: HashMap<&str, &Node> = data
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), node))
            .collect();
        data.nodes
            .iter()
            .enumerate()
            .map(|(index, node)| {
                (!node.is_group && shapes::uses_direction(node))
                    .then(|| node_dir(&data, index, &by_id).map(str::to_owned))
                    .flatten()
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    // The design mode centers the label of a node in every diagram type it draws.
    let anchor = if config.section.design.is_some() {
        Anchor::Middle
    } else {
        node_anchor(&data.diagram_type)
    };
    let busiest = if config.section.design.is_some() {
        busiest_sides(&data)
    } else {
        Vec::new()
    };
    let mut shapes = HashMap::new();
    for index in 0..data.nodes.len() {
        let node = &data.nodes[index];
        if node.is_group {
            // ELK sizes a subgraph from its title, measured unwrapped unless it is Markdown. The
            // title's box is already a float32 value (`node_label`). The swimlane engine takes the
            // title wrapped, whatever its type.
            let unwrapped = layout == Layout::Elk && node.label_type != LabelType::Markdown;
            let width = unwrapped.then_some(f64::INFINITY);
            // `measureGroupLabel` gives a group without a label no size (`createGraph.ts` 56-58).
            let size = if node.label.is_empty() {
                (0.0, 0.0)
            } else {
                shapes::node_label(node, config, width, anchor)?
                    .map_or((0.0, 0.0), |label| (label.bbox.width, label.bbox.height))
            };
            data.nodes[index].label_bbox = Some(size);
            continue;
        }
        // A `labelRect` is the label of a swimlane edge, which the stylesheet does not center: its lines start
        // at the origin, and `labelRect.ts` moves the label by half its width (`shapes/rect.rs`).
        // A note of a state or a class diagram is a block of text, which the design mode keeps at
        // the left.
        let label_anchor = if node.shape == "labelRect"
            || (node.shape == "note" && config.section.roles != crate::design::Roles::Flowchart)
        {
            Anchor::Start
        } else {
            anchor
        };
        let label = shapes::node_label(node, config, None, label_anchor)?;
        let mut shape = shapes::draw(&ShapeInput {
            node,
            label: label.as_ref(),
            config,
            dir: directions.get(index).and_then(Option::as_deref),
        })?;
        // The design mode gives a node room for the edges of its busiest side: it draws the shape
        // again with that length as its size along the side, and keeps the larger drawing.
        let grows =
            !(config.section.roles == crate::design::Roles::State && shapes::is_choice(node));
        if let Some(grown) =
            busiest
                .get(index)
                .filter(|_| grows)
                .and_then(|&(length, along_height)| {
                    let current = if along_height {
                        shape.bbox.height
                    } else {
                        shape.bbox.width
                    };
                    (length > current).then(|| {
                        let mut grown = node.clone();
                        if along_height {
                            grown.height = Some(length);
                        } else {
                            grown.width = Some(length);
                        }
                        grown
                    })
                })
        {
            let label = shapes::node_label(&grown, config, None, label_anchor)?;
            let redrawn = shapes::draw(&ShapeInput {
                node: &grown,
                label: label.as_ref(),
                config,
                dir: directions.get(index).and_then(Option::as_deref),
            })?;
            if redrawn.bbox.width * redrawn.bbox.height > shape.bbox.width * shape.bbox.height {
                shape = redrawn;
            }
        }
        let id = node.id.clone();
        // `insertMeasuredNode` takes the node's size from the `getBBox` of its group (createGraph.ts
        // 61-71, line 67), so it is a float32 value.
        let bbox = shape.bbox.browser();
        data.nodes[index].width = Some(bbox.width);
        data.nodes[index].height = Some(bbox.height);
        shapes.insert(id, shape);
    }

    let mut edge_labels = HashMap::new();
    let font_size = config.theme.font_size.px().unwrap_or(DEFAULT_FONT_SIZE);
    for edge in &mut data.edges {
        if let Some(label) = edges::label(edge, font_size, &config.fonts)? {
            let (mut width, height) = edges::label_size(&label);
            // The design mode draws the label in a chip; the layout is given the chip's width and
            // the room around it.
            if config.section.design.is_some() {
                width += 2.0 * (crate::design::CHIP_PADDING + crate::design::CHIP_MARGIN);
            }
            edge.width = Some(width);
            edge.height = Some(height);
            edge_labels.insert(edge.id.clone(), label);
        }
        edge.terminals = terminal::measure(edge, &config.fonts)?;
    }

    Ok(Measured {
        data,
        shapes,
        edge_labels,
        layout,
    })
}

/// Where the lines of a node's label start on `x = 0`. The flowchart's stylesheet centers node labels
/// (`.node .label text`); the state and class diagrams' do not, so their lines start at the label's
/// origin (`state/styles.js` and `class/styles.js` have no text-anchor for them).
#[must_use]
pub fn node_anchor(diagram_type: &str) -> Anchor {
    // The mindmap's stylesheet does not center its labels either: its `centerLabel` offsets the label
    // by half its width instead (`labelHelper`, `util.ts` 149-151).
    if diagram_type == crate::unified::markers::USECASE {
        // The use case's stylesheet has no `text-anchor`, and its `labelHelper` offsets the label in y
        // only (`usecaseRenderer.ts` styles, `shapes/util.ts` 146).
        Anchor::Origin
    } else if diagram_type == crate::state::DIAGRAM_TYPE
        || diagram_type == crate::unified::markers::CLASS
        || diagram_type == crate::mindmap::data::DIAGRAM_TYPE
    {
        Anchor::Start
    } else {
        Anchor::Middle
    }
}

/// Lays the measured diagram out with ELK and applies the result.
///
/// # Errors
///
/// Returns an error if the layout fails.
pub fn lay_out(measured: &mut Measured, config: &Config<'_>) -> Result<ElkStages, RenderError> {
    let elk = elk_build::build(&measured.data, config);
    let layout = if own_placement(config) {
        ferrigura_elk::placement::layout_with(&elk.graph, false, crate::flowchart::placement::Axes)
    } else {
        ferrigura_elk::layout(&elk.graph, false)
    }
    .map_err(|error| RenderError::Layout(error.to_string()))?;
    apply_layout(measured, config, &layout.graph, &elk.state);
    Ok(ElkStages {
        input: elk.graph,
        output: layout.graph,
    })
}

/// Lays out a diagram without retaining diagnostic ELK stages.
pub(crate) fn lay_out_for_render(
    measured: &mut Measured,
    config: &Config<'_>,
) -> Result<(), RenderError> {
    if own_placement(config) {
        return lay_out(measured, config).map(|_| ());
    }
    let elk = elk_build::build(&measured.data, config);
    let layout = ferrigura_elk::layout_owned(elk.graph, false)
        .map_err(|error| RenderError::Layout(error.to_string()))?;
    apply_layout(measured, config, &layout.graph, &elk.state);
    Ok(())
}

/// Lays out a diagram as [`lay_out_for_render`] does, with a step of the caller's between ELK's
/// layout and its application: `adjust` gets ELK's result, the layout data and the side tables,
/// and may change all three. When it returns `false`, nothing is applied and the result is
/// `false`; the caller then lays the diagram out another way.
///
/// # Errors
///
/// Returns an error if the layout fails.
pub(crate) fn lay_out_adjusted(
    measured: &mut Measured,
    config: &Config<'_>,
    adjust: impl FnOnce(&mut JsonNode, &mut LayoutData, &mut elk_build::ElkState) -> bool,
) -> Result<bool, RenderError> {
    let elk = elk_build::build(&measured.data, config);
    let mut layout = if own_placement(config) {
        ferrigura_elk::placement::layout_with(&elk.graph, false, crate::flowchart::placement::Axes)
    } else {
        ferrigura_elk::layout_owned(elk.graph, false)
    }
    .map_err(|error| RenderError::Layout(error.to_string()))?;
    let mut state = elk.state;
    if !adjust(&mut layout.graph, &mut measured.data, &mut state) {
        return Ok(false);
    }
    apply_layout(measured, config, &layout.graph, &state);
    Ok(true)
}

/// Whether the layout runs with the design mode's own node placement, on the layers and the order
/// ELK decided (`crate::flowchart::placement`): in the design mode, unless the configuration
/// names a placement strategy, which is the user's choice.
fn own_placement(config: &Config<'_>) -> bool {
    config.section.design.is_some() && config.section.elk.node_placement_strategy.is_none()
}

fn apply_layout(
    measured: &mut Measured,
    config: &Config<'_>,
    graph: &JsonNode,
    state: &elk_build::ElkState,
) {
    let crossing = |id: &str, node, probe| {
        measured
            .shapes
            .get(id)
            .map(|shape| shape.intersect.intersect(node, probe))
    };
    elk_apply::apply_with_crossing(
        &mut measured.data,
        graph,
        state,
        &crossing,
        config.section.elk.straighten_edges,
    );
    // The design mode takes the branch of a decision or a choice to a node beside it out of the
    // side tip.
    if config.section.design.is_some() {
        crate::flowchart::turns::side_exits(&mut measured.data);
        // A class relation's cardinalities stand beside its ends.
        if measured.data.diagram_type == super::markers::CLASS {
            crate::class::tree::merge_generalizations(
                &mut measured.data.edges,
                &measured.data.nodes,
            );
            super::terminal::place_beside_ends(&mut measured.data.edges, &measured.data.nodes);
        }
    }
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
    config: &Config<'_>,
) -> Result<Painted<'a>, RenderError> {
    let parts = paint_with_animation(measured, id, config, None)?;
    Ok(Painted {
        root: parts.root.slices().concat(),
        coloured_markers: parts.coloured_markers,
        bounds: parts.bounds,
        cluster_labels: parts.cluster_labels,
        labels: parts.labels,
        links: parts.links,
    })
}

pub(crate) fn paint_with_animation<'a>(
    measured: &'a Measured,
    id: &str,
    config: &Config<'_>,
    animation: Option<crate::animation::Animation>,
) -> Result<PaintedParts<'a>, RenderError> {
    let data = &measured.data;
    let mut bounds: Option<BBox> = None;
    let mut labels: Vec<&LabelLayout> = Vec::new();

    let mut clusters = String::new();
    let mut painted_clusters = Vec::new();
    // The size of each lane once its shape has drawn it: the swimlane edges clip to it (`node.width`
    // and `node.height` after `swimlane.js`).
    let mut lanes: HashMap<String, (f64, f64)> = HashMap::new();
    for node in data.nodes.iter().filter(|node| node.is_group) {
        // `render` prefixes every node's DOM id with the diagram's id before the layout runs.
        let prefixed = Node {
            dom_id: format!("{id}-{}", node.dom_id),
            ..node.clone()
        };
        let cluster = clusters::paint_among(&prefixed, config, &data.edges)?;
        if matches!(measured.layout, Layout::Swimlane(_)) {
            lanes.insert(node.id.clone(), (cluster.bbox.width, cluster.bbox.height));
        }
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
    let mut node_parts = Vec::new();
    let mut links = false;
    for (index, node) in data
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| !node.is_group)
    {
        let Some(shape) = measured.shapes.get(&node.id) else {
            continue;
        };
        links |= node.link.is_some();
        let (x, y) = (node.x.unwrap_or(0.0), node.y.unwrap_or(0.0));
        cover(
            &mut bounds,
            shape.extent.unwrap_or(shape.bbox).translate(x, y),
        );
        // A state diagram's links are written after its render (`LinkForm::PostRender`).
        let link_form = if data.diagram_type == crate::state::DIAGRAM_TYPE {
            shapes::LinkForm::PostRender
        } else {
            shapes::LinkForm::Flowchart
        };
        let svg = shapes::node_svg(node, shape, id, link_form);
        if animation.is_some() {
            node_parts.push((index, svg));
        } else {
            nodes.push_str(&svg);
        }
        labels.extend(shape.label.as_ref());
        labels.extend(shape.texts.iter());
    }

    // The swimlane engine's edges are clipped to the outlines of their ends (`insertEdge`'s swimlane
    // branch) before they are drawn, and so are the line hops that follow from their points.
    let (path_edges, jumps): (Cow<'_, [Edge]>, Option<JumpStyle>) = match measured.layout {
        Layout::Elk => (
            Cow::Borrowed(data.edges.as_slice()),
            edges::jump_style(&config.section),
        ),
        Layout::Swimlane(jumps) => (
            Cow::Owned(swimlane_layout::clip_edges(
                &data.edges,
                &data.nodes,
                &measured.shapes,
                &lanes,
            )),
            jumps,
        ),
    };
    let mut edge_paths = String::new();
    let mut coloured_markers = String::new();
    let paths = edges::prepare_paths(&path_edges, id, &data.diagram_type, &config.section, jumps);
    let mut animator = animation
        .map(|options| {
            crate::animation::Animator::new(id, &data.nodes, &path_edges, &paths, options)
        })
        .transpose()?;
    for (index, mut path) in paths.into_iter().enumerate() {
        let animated = animator
            .as_mut()
            .map(|animator| animator.path(index, &mut path));
        let path = edges::finish_path(path, animated);
        cover(&mut bounds, path.bbox);
        edge_paths.push_str(&path.svg);
        coloured_markers.push_str(&path.markers);
    }
    let mut edge_labels = String::new();
    // The boxes of the edge labels in the design mode, where each interrupts its edge.
    let mut label_gaps: Vec<BBox> = Vec::new();
    for (index, edge) in data.edges.iter().enumerate() {
        if let Some(label) = measured.edge_labels.get(&edge.id) {
            // The design mode keeps a label's chip off the frame of a group.
            let moved = config.section.design.map(|_| {
                let frames: Vec<BBox> = painted_clusters
                    .iter()
                    .map(|cluster| cluster.bbox)
                    .collect();
                let place = edges::label_clear_of_frames(edge, label, &frames);
                Edge {
                    x: Some(place.x),
                    y: Some(place.y),
                    ..edge.clone()
                }
            });
            let edge = moved.as_ref().unwrap_or(edge);
            let painted = edges::paint_label(edge, label, &config.section);
            cover(&mut bounds, painted.bbox);
            if config.section.design.is_some() && label.background.is_some() {
                label_gaps.push(painted.bbox);
            }
            match &mut animator {
                Some(animator) => edge_labels.push_str(&animator.label(index, &painted.svg)),
                None => edge_labels.push_str(&painted.svg),
            }
            labels.push(label);
        }
        // `insertEdgeLabel` runs for an edge with a label or a terminal label (`hasEdgeLabel`,
        // `edges.js` 55-62): without text, the label has no position, and its background sits beside it.
        if edge.label.is_empty() && !edge.terminals.is_empty() {
            edge_labels.push_str(&format!(
                "<g class=\"edgeLabel\"><g class=\"label\" data-id=\"{}\" transform=\"translate(0, 0)\"><text y=\"-10.1\" text-anchor=\"middle\"><tspan class=\"text-outer-tspan row\" x=\"0\" y=\"-0.1em\" dy=\"1.1em\" text-anchor=\"middle\"></tspan></text></g></g><g><rect class=\"background\" style=\"stroke: none\"></rect></g>",
                crate::svg::escape(&edge.id),
            ));
        }
        // The terminal labels of an edge follow its center label in `edgeLabels` (`insertEdgeLabel`).
        for terminal_label in &edge.terminals {
            let (svg, bbox) = terminal::paint(edge, terminal_label);
            cover(&mut bounds, bbox);
            edge_labels.push_str(&svg);
            labels.push(&terminal_label.label);
        }
    }

    let cluster_labels = painted_clusters
        .into_iter()
        .filter_map(|cluster| cluster.label)
        .collect();

    if let Some(animator) = &mut animator {
        for (index, svg) in node_parts {
            nodes.push_str(&animator.node(index, &svg));
        }
    }
    let animation = animator
        .map(crate::animation::Animator::finish)
        .unwrap_or_default();

    // The design mode leaves a gap in an edge where its label sits: the edges are masked there, so
    // the label needs no background and reads on any ground.
    if let (false, Some(drawn)) = (label_gaps.is_empty(), bounds) {
        edge_paths = edges::with_label_gaps(&edge_paths, id, drawn, &label_gaps);
    }

    Ok(PaintedParts {
        root: RootParts {
            clusters,
            edge_paths,
            edge_labels,
            nodes,
            animation,
        },
        coloured_markers,
        bounds,
        cluster_labels,
        labels,
        links,
    })
}

/// Adds a painted box to the boxes the viewBox covers.
pub fn cover(bounds: &mut Option<BBox>, bbox: BBox) {
    *bounds = Some(bounds.map_or(bbox, |all| all.union(bbox)));
}

/// The font size of labels when the theme has none.
const DEFAULT_FONT_SIZE: f64 = 16.0;

#[cfg(test)]
mod direction_tests {
    use super::*;

    #[test]
    fn the_busiest_side_counts_the_edges_into_and_out_of_a_node() {
        let node = |id: &str, parent: Option<&str>, dir: Option<&str>| Node {
            id: id.into(),
            parent_id: parent.map(str::to_owned),
            dir: dir.map(str::to_owned),
            is_group: dir.is_some(),
            ..Node::default()
        };
        let edge = |start: &str, end: &str| Edge {
            start: start.into(),
            end: end.into(),
            ..Edge::default()
        };
        let data = LayoutData {
            direction: "LR".into(),
            nodes: vec![
                node("a", None, None),
                node("b", None, None),
                node("group", None, Some("TB")),
                node("c", Some("group"), None),
            ],
            edges: vec![
                edge("a", "b"),
                edge("a", "b"),
                edge("a", "c"),
                edge("c", "b"),
                edge("b", "b"),
            ],
            ..LayoutData::default()
        };
        let sides = busiest_sides(&data);
        // `a` has three edges out: 2 * 20 + 2 * 24 along its height, from left to right.
        assert_eq!(sides[0], (88.0, true));
        // `b` has three edges in; its self-loop does not count.
        assert_eq!(sides[1], (88.0, true));
        // `c` has one edge on each side and needs nothing; its group runs from top to bottom.
        assert_eq!(sides[3], (0.0, false));
    }

    #[test]
    fn node_direction_preserves_parent_lookup_and_fallbacks() {
        let node = |id: &str, parent: Option<&str>, dir: Option<&str>| Node {
            id: id.into(),
            parent_id: parent.map(str::to_owned),
            dir: dir.map(str::to_owned),
            ..Node::default()
        };
        let data = LayoutData {
            direction: "TB".into(),
            nodes: vec![
                node("own", None, Some("RL")),
                node("leaf", Some("group"), None),
                node("group", None, Some("LR")),
                node("parentless", None, None),
                node("missing", Some("absent"), None),
                node("cycle1", Some("cycle2"), None),
                node("cycle2", Some("cycle1"), None),
            ],
            ..LayoutData::default()
        };
        let by_id = data.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
        assert_eq!(node_dir(&data, 0, &by_id), Some("RL"));
        assert_eq!(node_dir(&data, 1, &by_id), Some("LR"));
        assert_eq!(node_dir(&data, 3, &by_id), None);
        assert_eq!(node_dir(&data, 4, &by_id), Some("TB"));
        assert_eq!(node_dir(&data, 5, &by_id), Some("TB"));
    }

    #[test]
    fn direction_consumers_include_all_fork_join_aliases() {
        for shape in ["fork", "join", "forkJoin"] {
            assert!(shapes::uses_direction(&Node {
                shape: shape.into(),
                ..Node::default()
            }));
        }
        assert!(!shapes::uses_direction(&Node {
            shape: "rect".into(),
            ..Node::default()
        }));
    }
}
