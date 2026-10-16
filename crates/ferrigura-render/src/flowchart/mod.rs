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
pub mod geom;
pub mod label;
pub mod line_jumps;
pub mod markers;
pub mod path;
pub mod shapes;
pub mod style;

use std::collections::HashMap;

use ferrigura_elk::json::JsonNode;
use ferrigura_model::config::MermaidConfig;
use ferrigura_model::flowchart::{FlowchartDiagram, LabelType};
use ferrigura_model::value::Value;

use self::data::LayoutData;
use self::label::LabelLayout;
use self::shapes::{ShapeInput, ShapeOutput};
use crate::RenderError;
use crate::pie::Themes;

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
    for edge in &mut data.edges {
        if let Some(label) = edges::label(edge, input.config)? {
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
    let _ = &measured.edge_labels;
    todo!("paint and assemble the SVG (coordinator, after T1 to T7)")
}
