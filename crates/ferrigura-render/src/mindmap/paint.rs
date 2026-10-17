//! The paint of a laid-out mindmap: the node groups, the edge paths with their labels, and the markers
//! (`insertMarkers` in `cose-bilkent/render.ts` 41-49, `insertNode` 70-73, `insertEdge` 159-167 and
//! `insertEdgeLabel` 143). The DOM order is the reference's: markers, the empty subgraph group, the
//! edge paths, the edge labels, and then the nodes, which paint over the edges.
//!
//! An edge path is `basis` through its points, after `fixCorners` (`edges.js` 729-738), with no
//! clipping: the mindmap's nodes have no `intersect` on the copies `insertEdge` receives
//! (`mindmap/NOTES.md` 6.5). The reference writes the style attribute as `undefined;;;undefined`
//! because the edge has no `style` field, and `data-points` is the base64 of the points as JSON.

use base64::Engine as _;

use crate::svg::{escape, number};
use crate::unified::data::{Edge, LayoutData};
use crate::unified::geom::{BBox, Point};
use crate::unified::markers;
use crate::unified::path::{Curve, curve_path, fix_corners, path_bounds};
use crate::unified::pipeline::{Measured, cover};
use crate::unified::shapes::{LinkForm, node_svg};

/// What the mindmap paints: the root group's content, and the box the drawing covers.
#[derive(Debug)]
pub struct Painted {
    /// The markers, then the groups `g.subgraphs`, `g.edgePaths`, `g.edgeLabels` and `g.nodes`, as the
    /// root group's children.
    pub root: String,
    /// The box the painted elements cover: what `getBBox` of the svg gives, before the padding.
    pub bounds: Option<BBox>,
}

/// Paints the measured and laid-out diagram. `id` is the diagram's id, which prefixes every DOM id.
#[must_use]
pub fn paint(measured: &Measured, id: &str) -> Painted {
    let data: &LayoutData = &measured.data;
    let mut bounds: Option<BBox> = None;

    let mut nodes = String::new();
    for node in &data.nodes {
        let Some(shape) = measured.shapes.get(&node.id) else {
            continue;
        };
        let (x, y) = (node.x.unwrap_or(0.0), node.y.unwrap_or(0.0));
        cover(
            &mut bounds,
            shape.extent.unwrap_or(shape.bbox).translate(x, y),
        );
        nodes.push_str(&node_svg(node, shape, id, LinkForm::Flowchart));
    }

    let mut edge_paths = String::new();
    let mut edge_labels = String::new();
    for edge in &data.edges {
        if edge.points.is_empty() {
            continue;
        }
        // `fixCorners` pre-rounds the right-angle corners, then `curveBasis` runs through the points.
        let d = curve_path(Curve::Basis, &fix_corners(&edge.points)).unwrap_or_default();
        if let Some(bbox) = path_bounds(&d) {
            cover(&mut bounds, bbox);
        }
        edge_paths.push_str(&edge_path(edge, &d, id));
        edge_labels.push_str(&empty_edge_label(edge));
    }

    let root = format!(
        "{markers}<g class=\"subgraphs\"></g><g class=\"edgePaths\">{edge_paths}</g><g class=\"edgeLabels\">{edge_labels}</g><g class=\"nodes\">{nodes}</g>",
        markers = markers::point_markers(id, super::data::DIAGRAM_TYPE),
    );
    Painted { root, bounds }
}

/// The `path` of an edge (`insertEdge`, `edges.js` 857-901): its id is the diagram's id and the edge's,
/// its classes are the thickness and the pattern before the edge's own, and `style` is the reference's
/// `undefined;;;undefined`, since an edge of the mindmap has no `style`.
fn edge_path(edge: &Edge, d: &str, id: &str) -> String {
    let points = points_json(&edge.points);
    let encoded = base64::engine::general_purpose::STANDARD.encode(points.as_bytes());
    format!(
        "<path d=\"{d}\" id=\"{id}-{edge_id}\" class=\" edge-thickness-{thickness} edge-pattern-solid {classes}\" style=\"undefined;;;undefined\" data-edge=\"true\" data-et=\"edge\" data-id=\"{edge_id}\" data-look=\"{look}\" data-points=\"{encoded}\"></path>",
        id = escape(id),
        edge_id = escape(&edge.id),
        thickness = escape(&edge.thickness),
        classes = escape(&edge.classes),
        look = escape(&edge.look),
    )
}

/// The label of an edge with no text (`insertEdgeLabel`, `edges.js` 84-101 and 119-126): its group,
/// with an empty `text` and `tspan`, and the background rectangle the reference leaves beside it.
fn empty_edge_label(edge: &Edge) -> String {
    format!(
        "<g class=\"edgeLabel\"><g class=\"label\" data-id=\"{}\" transform=\"translate(0, 0)\"><text y=\"-10.1\" text-anchor=\"middle\"><tspan class=\"text-outer-tspan row\" x=\"0\" y=\"-0.1em\" dy=\"1.1em\" text-anchor=\"middle\"></tspan></text></g></g><g><rect class=\"background\" style=\"stroke: none\"></rect></g>",
        escape(&edge.id),
    )
}

/// `JSON.stringify` of the points, as `[{"x":…,"y":…}, …]`: a number that JSON cannot hold is `null`.
fn points_json(points: &[Point]) -> String {
    let items: Vec<String> = points
        .iter()
        .map(|point| {
            format!(
                "{{\"x\":{},\"y\":{}}}",
                json_number(point.x),
                json_number(point.y)
            )
        })
        .collect();
    format!("[{}]", items.join(","))
}

fn json_number(value: f64) -> String {
    if value.is_finite() {
        number(value)
    } else {
        "null".to_owned()
    }
}
