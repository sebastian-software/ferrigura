//! The flowchart: a port of the reference's unified flowchart renderer on ELK
//! (`diagrams/flowchart/flowRenderer-v3-unified.ts`, `rendering-util/render.ts`,
//! `layout-algorithms/common/index.ts` and `createGraph.ts`).
//!
//! The pipeline follows the reference step by step: `getData`, the edge render data, the measure
//! step (labels and shapes), the elkjs graph, ELK Layered, the post-processing of the result, and
//! the paint. Each step is a module, so that its output can be compared with the reference's. See
//! `docs-port/flowchart/RENDER-PLAN.md`.

pub mod clusters;
pub mod data;
pub mod edges;
pub mod elk_apply;
pub mod elk_build;
pub mod elk_geometry;
pub mod elk_straighten;
pub mod elk_tree;
pub mod geom;
pub mod label;
pub mod line_jumps;
pub mod markdown;
pub mod markers;
pub mod path;
pub mod shapes;
pub mod style;

use std::collections::HashMap;

use ferrigura_elk::json::JsonNode;
use ferrigura_model::config::MermaidConfig;
use ferrigura_model::flowchart::{FlowchartDiagram, LabelType};
use ferrigura_model::value::Value;
use ferrigura_text::{
    FontFamily, FontStyle, FontWeight, Run, embed_runs, face, measure as measure_text, text_box,
};

use self::data::LayoutData;
use self::geom::BBox;
use self::label::LabelLayout;
use self::shapes::{ShapeInput, ShapeOutput};
use crate::RenderError;
use crate::a11y;
use crate::pie::Themes;
use crate::svg::{escape, number};
use crate::variables::px;

/// What the flowchart renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct FlowchartInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a FlowchartDiagram,
    /// The effective configuration, with the flowchart's `theme`, `look` and `layout` resolved.
    pub config: &'a MermaidConfig,
    /// The colours and sizes of the theme.
    pub themes: Themes<'a>,
    /// The id of the SVG element. It scopes every selector and prefixes every element id, so it
    /// must be a CSS identifier.
    pub id: &'a str,
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
struct Measured {
    data: LayoutData,
    /// The shape of every leaf node, by id.
    shapes: HashMap<String, ShapeOutput>,
    /// The label of every edge that has one, by edge id.
    edge_labels: HashMap<String, LabelLayout>,
}

/// The theme variables that give the sizes and the presentation attributes.
fn primary<'a>(themes: &Themes<'a>) -> &'a Value {
    match *themes {
        Themes::Single(variables) => variables,
        Themes::Schemes { light, .. } => light,
    }
}

/// The nearest `dir` of the node or one of its subgraphs, else the diagram's direction, as
/// `resolveNodeDir` in `createGraph.ts` finds it.
fn node_dir(data: &LayoutData, index: usize) -> Option<&str> {
    let by_id: HashMap<&str, &data::Node> = data
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

/// `getData`, the edge render data, and the measure step of `createGraphWithElements`.
fn measure(input: &FlowchartInput<'_>) -> Result<Measured, RenderError> {
    let theme = primary(&input.themes);
    let mut data = data::get_data(input.diagram, input.config);
    edges::prepare(&mut data, input.config);

    let mut shapes = HashMap::new();
    for index in 0..data.nodes.len() {
        let node = &data.nodes[index];
        if node.is_group {
            // ELK sizes a subgraph from its title, measured unwrapped unless it is Markdown.
            let width = (node.label_type != LabelType::Markdown).then_some(f64::INFINITY);
            let size = shapes::node_label(node, input.config, theme, width)?
                .map_or((0.0, 0.0), |label| (label.bbox.width, label.bbox.height));
            data.nodes[index].label_bbox = Some(size);
            continue;
        }
        let label = shapes::node_label(node, input.config, theme, None)?;
        let shape = shapes::draw(&ShapeInput {
            node,
            label: label.as_ref(),
            config: input.config,
            theme,
            dir: node_dir(&data, index),
        })?;
        let id = node.id.clone();
        data.nodes[index].width = Some(shape.bbox.width);
        data.nodes[index].height = Some(shape.bbox.height);
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
fn lay_out(measured: &mut Measured, config: &MermaidConfig) -> Result<ElkStages, RenderError> {
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

/// The graphs of the layout step, for the differential comparison.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read or the layout fails.
pub fn elk_stages(input: &FlowchartInput<'_>) -> Result<ElkStages, RenderError> {
    let mut measured = measure(input)?;
    lay_out(&mut measured, input.config)
}

/// Renders a flowchart as an SVG document.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read, a shape is unknown, or the layout fails.
pub fn render(input: &FlowchartInput<'_>) -> Result<String, RenderError> {
    let mut measured = measure(input)?;
    lay_out(&mut measured, input.config)?;
    let config = input.config;
    let theme = primary(&input.themes);
    let id = input.id;
    let data = &measured.data;

    // Every box the painted elements cover, for the viewBox: what `getBBox` of the svg gives.
    let mut bounds: Option<BBox> = None;
    // Every text the diagram draws, to subset the fonts once for the whole diagram.
    let mut labels: Vec<&LabelLayout> = Vec::new();

    // The paint order of `paintLayoutData`: the clusters in the order `orderNodesForElkPaint` left,
    // then the nodes as the measure step inserted them, then the edges and their labels.
    let mut clusters = String::new();
    let mut painted_clusters = Vec::new();
    for node in data.nodes.iter().filter(|node| node.is_group) {
        // `render` prefixes every node's DOM id with the diagram's id before the layout runs.
        let prefixed = data::Node {
            dom_id: format!("{id}-{}", node.dom_id),
            ..node.clone()
        };
        let cluster = clusters::paint(&prefixed, config, theme)?;
        // The frame of an empty subgraph has no height: an unrendered rect, which `getBBox` leaves out
        // of the drawing's box. Its title is text and counts.
        if !cluster.bbox.is_empty() {
            cover(&mut bounds, cluster.bbox);
        }
        if !cluster.title_box.is_empty() {
            cover(&mut bounds, cluster.title_box);
        }
        clusters.push_str(&cluster.svg);
        painted_clusters.push(cluster);
    }
    labels.extend(
        painted_clusters
            .iter()
            .filter_map(|cluster| cluster.label.as_ref()),
    );

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
    // The markers in an edge's own colour, which the reference appends after the root group.
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

    // `insertTitle`: centred over everything drawn so far, `titleTopMargin` above the origin.
    let sans = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
    let title = &input.diagram.title;
    let mut title_svg = String::new();
    if !title.is_empty() {
        let drawn = bounds.unwrap_or_default();
        let width = measure_text(sans, title, TITLE_FONT_SIZE)?;
        let text_box = text_box(sans, TITLE_FONT_SIZE)?;
        let baseline = -(config.flowchart.title_top_margin as f64);
        let x = drawn.x + drawn.width / 2.0 - width / 2.0;
        cover(
            &mut bounds,
            BBox::new(x, baseline - text_box.top, width, text_box.height),
        );
        title_svg = format!(
            "<text text-anchor=\"start\" x=\"{}\" y=\"{}\" class=\"flowchartTitleText\">{}</text>",
            number(x),
            number(baseline),
            escape(title)
        );
    }

    let mut runs: Vec<Run<'_>> = labels.iter().flat_map(|label| label.text_runs()).collect();
    if !title.is_empty() {
        runs.push(Run {
            text: title,
            face: sans,
        });
    }
    let families = embed_runs(&runs)?;

    // `setupViewPortForSVG`: the drawn box with `diagramPadding` around it.
    let padding = config.flowchart.diagram_padding as f64;
    let drawn = bounds.unwrap_or_default();
    let (view_width, view_height) = (drawn.width + padding * 2.0, drawn.height + padding * 2.0);
    let size_attributes = if config.flowchart.use_max_width {
        format!(
            "width=\"100%\" style=\"max-width: {}px;\"",
            number(view_width)
        )
    } else {
        format!(
            "height=\"{}\" width=\"{}\"",
            number(view_height),
            number(view_width)
        )
    };
    let diagram = input.diagram;
    let accessibility = a11y::children(id, &diagram.acc_title, &diagram.acc_descr);
    let accessible_attributes =
        a11y::root_attributes(id, "flowchart-v2", &diagram.acc_title, &diagram.acc_descr);
    let style =
        style::style_sheet_with_fonts(id, input.themes, config, &diagram.classes, &families);
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" id=\"{id}\" {size_attributes} class=\"flowchart\" viewBox=\"{x} {y} {w} {h}\"{accessible_attributes}>{accessibility}<style>{style}</style><g>{markers}<g class=\"root\"><g class=\"clusters\">{clusters}</g><g class=\"edges edgePaths\">{edge_paths}</g><g class=\"edgeLabels\">{edge_labels}</g><g class=\"nodes\">{nodes}</g></g>{coloured_markers}</g>{title_svg}{defs}</svg>",
        id = escape(id),
        x = number(drawn.x - padding),
        y = number(drawn.y - padding),
        w = number(view_width),
        h = number(view_height),
        markers = edges::markers(id, config),
        defs = style::look_defs(id, input.themes, config),
    ))
}

/// Adds a painted box to the boxes the viewBox covers.
fn cover(bounds: &mut Option<BBox>, bbox: BBox) {
    *bounds = Some(bounds.map_or(bbox, |all| all.union(bbox)));
}

/// The font size of labels when the theme has none.
const DEFAULT_FONT_SIZE: f64 = 16.0;

/// The font size of the title: `.flowchartTitleText` in the flowchart styles.
const TITLE_FONT_SIZE: f64 = 18.0;
