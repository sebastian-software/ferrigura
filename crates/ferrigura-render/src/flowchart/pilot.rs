//! Measured flowchart interchange for the development-only TALA pilot.

use std::collections::HashMap;

use ferrigura_model::flowchart::FlowchartDiagram;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::RenderError;
use crate::context::Context;
use crate::unified::geom::Point;
use crate::unified::pipeline::{self, Measured};
use crate::unified::shapes::{self, ShapeInput};
use crate::unified::{Config, Section};

type Flowchart<'a, 'b> = Context<'a, FlowchartDiagram, Section<'b>>;

/// Exports the sizes and topology the two engines receive.
///
/// # Errors
/// Returns the flowchart's normal measurement errors.
pub fn measured_graph(input: &Flowchart<'_, '_>) -> Result<Value, RenderError> {
    let data = super::measure(input)?.data;
    let nodes = data.nodes.iter().map(|node| {
        let (label_width, label_height) = node.label_bbox.unwrap_or_default();
        json!({
            "id": node.id,
            "parentId": node.parent_id,
            "direction": node.dir,
            "isGroup": node.is_group,
            "shape": node.shape,
            "width": node.width.unwrap_or_default(),
            "height": node.height.unwrap_or_default(),
            "label": node.label,
            "labelWidth": label_width,
            "labelHeight": label_height,
        })
    });
    let edges = data.edges.iter().map(|edge| {
        json!({
            "id": edge.id,
            "source": edge.start,
            "target": edge.end,
            "sourceArrow": !matches!(edge.arrow_type_start.as_str(), "" | "none"),
            "targetArrow": !matches!(edge.arrow_type_end.as_str(), "" | "none"),
            "label": edge.label,
            "labelWidth": edge.width.unwrap_or_default(),
            "labelHeight": edge.height.unwrap_or_default(),
        })
    });
    Ok(json!({
        "direction": data.direction,
        "nodes": nodes.collect::<Vec<_>>(),
        "edges": edges.collect::<Vec<_>>(),
    }))
}

/// Exports geometry after Ferrigura's normal ELK layout and post-processing.
///
/// # Errors
/// Returns the flowchart's normal measurement or layout errors.
pub fn elk_geometry(input: &Flowchart<'_, '_>) -> Result<Value, RenderError> {
    let mut measured = super::measure(input)?;
    pipeline::lay_out_for_render(&mut measured, &Config::of(input))?;
    let nodes = measured.data.nodes.iter().map(|node| {
        let width = node.width.unwrap_or_default();
        let height = node.height.unwrap_or_default();
        json!({
            "id": node.id,
            "x": node.x.map(|x| x - width / 2.0),
            "y": node.y.map(|y| y - height / 2.0),
            "width": width,
            "height": height,
        })
    });
    let edges = measured.data.edges.iter().map(|edge| {
        let points = edge
            .points
            .iter()
            .map(|point| json!({"x": point.x, "y": point.y}));
        let label = edge.x.zip(edge.y).map(|(x, y)| json!({"x": x, "y": y}));
        json!({"id": edge.id, "points": points.collect::<Vec<_>>(), "label": label})
    });
    Ok(json!({"nodes": nodes.collect::<Vec<_>>(), "edges": edges.collect::<Vec<_>>()}))
}

/// Paints captured node boxes, routes and label centers through the normal SVG painter.
///
/// # Errors
/// Returns an error if the geometry does not cover the measured graph, or painting fails.
pub fn paint_captured(input: &Flowchart<'_, '_>, capture: Value) -> Result<String, RenderError> {
    let capture: Geometry = serde_json::from_value(capture)
        .map_err(|error| RenderError::Layout(format!("invalid pilot geometry: {error}")))?;
    let mut measured = super::measure(input)?;
    let config = Config::of(input);
    apply(&mut measured, capture, &config)?;
    super::paint_measured(input, &measured, &config, None)
}

#[derive(Deserialize)]
struct Geometry {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

#[derive(Deserialize)]
struct Node {
    id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Deserialize)]
struct Edge {
    id: String,
    points: Vec<Position>,
    label: Option<Position>,
}

#[derive(Deserialize)]
struct Position {
    x: f64,
    y: f64,
}

impl From<Position> for Point {
    fn from(position: Position) -> Self {
        Self::new(position.x, position.y)
    }
}

fn apply(
    measured: &mut Measured,
    capture: Geometry,
    config: &Config<'_>,
) -> Result<(), RenderError> {
    let node_count = capture.nodes.len();
    let edge_count = capture.edges.len();
    let mut nodes: HashMap<_, _> = capture
        .nodes
        .into_iter()
        .map(|node| (node.id.clone(), node))
        .collect();
    let mut edges: HashMap<_, _> = capture
        .edges
        .into_iter()
        .map(|edge| (edge.id.clone(), edge))
        .collect();
    if nodes.len() != node_count
        || edges.len() != edge_count
        || node_count != measured.data.nodes.len()
        || edge_count != measured.data.edges.len()
    {
        return Err(RenderError::Layout(
            "pilot geometry has the wrong node or edge IDs".to_owned(),
        ));
    }
    for node in &mut measured.data.nodes {
        let geometry = nodes
            .remove(&node.id)
            .ok_or_else(|| missing("node", &node.id))?;
        if geometry.width < 0.0 || geometry.height < 0.0 {
            return Err(RenderError::Layout(format!(
                "negative dimensions for pilot node {:?}",
                node.id
            )));
        }
        let resized = node.width != Some(geometry.width) || node.height != Some(geometry.height);
        node.x = Some(geometry.x + geometry.width / 2.0);
        node.y = Some(geometry.y + geometry.height / 2.0);
        node.width = Some(geometry.width);
        node.height = Some(geometry.height);
        if resized && !node.is_group {
            let shape = measured
                .shapes
                .get_mut(&node.id)
                .ok_or_else(|| missing("shape", &node.id))?;
            if !matches!(
                node.shape.as_str(),
                "squareRect" | "rect" | "rectangle" | "roundedRect"
            ) {
                return Err(RenderError::Layout(format!(
                    "pilot cannot resize node {:?} of shape {:?}",
                    node.id, node.shape
                )));
            }
            let resized = shapes::draw(&ShapeInput {
                node,
                label: shape.label.as_ref(),
                config,
                dir: None,
            })?;
            let bounds = resized.bbox.browser();
            if bounds.width != geometry.width || bounds.height != geometry.height {
                return Err(RenderError::Layout(format!(
                    "pilot layout size does not fit the measured label of node {:?}",
                    node.id
                )));
            }
            *shape = resized;
        }
    }
    for edge in &mut measured.data.edges {
        let geometry = edges
            .remove(&edge.id)
            .ok_or_else(|| missing("edge", &edge.id))?;
        if geometry.points.len() < 2 {
            return Err(RenderError::Layout(format!(
                "missing route for pilot edge {:?}",
                edge.id
            )));
        }
        edge.points = geometry.points.into_iter().map(Point::from).collect();
        let curve = if edge.start == edge.end {
            "linear"
        } else {
            "rounded"
        };
        edge.curve = Some(curve.to_owned());
        if let Some(label) = geometry.label {
            edge.x = Some(label.x);
            edge.y = Some(label.y);
        } else if measured.edge_labels.contains_key(&edge.id) {
            return Err(missing("edge label", &edge.id));
        }
    }
    Ok(())
}

fn missing(kind: &str, id: &str) -> RenderError {
    RenderError::Layout(format!("pilot geometry is missing {kind} {id:?}"))
}
