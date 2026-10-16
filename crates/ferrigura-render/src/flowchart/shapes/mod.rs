//! Node shapes: a port of `packages/mermaid/src/rendering-util/rendering-elements/nodes.ts`, the
//! shape handlers in `shapes/` and the intersect functions in `intersect/`, for the `neo` and
//! `classic` looks.
//!
//! A shape gets the node and its label laid out at its natural width, and returns its own SVG
//! elements in the node's coordinates (origin at the node's centre), where the label goes, the box
//! of the whole node group, and how edges meet its outline. The box is what the reference's
//! `getBBox` gives for the node's `<g>`, shape and label together, and it is the size the layout
//! uses. See `docs-port/flowchart/render-notes/shapes-core.md`.

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::value::Value;

use super::data::Node;
use super::geom::{BBox, CenterBox, Point};
use super::label::LabelLayout;
use crate::RenderError;

/// What a shape is drawn from.
#[derive(Debug, Clone, Copy)]
pub struct ShapeInput<'a> {
    /// The node.
    pub node: &'a Node,
    /// The node's label, laid out at the node's wrapping width and before any minimum width is
    /// applied; `None` when the node has no label.
    pub label: Option<&'a LabelLayout>,
    /// The effective configuration.
    pub config: &'a MermaidConfig,
    /// The theme variables (`radius`, `strokeWidth`, `nodeShadow`, ...).
    pub theme: &'a Value,
    /// The direction the node is laid out in: the nearest ancestor's `dir`, else the diagram's.
    pub dir: Option<&'a str>,
}

/// A drawn shape.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeOutput {
    /// The shape's own elements, in the node's coordinates. They are the first children of the
    /// node's `<g>`, under the label.
    pub svg: String,
    /// The translation of the label's `<g class="label">`, when the shape draws a label.
    pub label_offset: Option<Point>,
    /// The label as the shape draws it: the input label with the minimum width applied, if the
    /// shape applies one.
    pub label: Option<LabelLayout>,
    /// The box of the whole node group, in the node's coordinates.
    pub bbox: BBox,
    /// How edges meet the outline.
    pub intersect: Intersect,
}

/// How an edge meets a node's outline: the reference's `node.intersect`.
#[derive(Debug, Clone, PartialEq)]
pub enum Intersect {
    /// `intersect-rect.js` on the node's box.
    Rect,
    /// `intersect-circle.js` with this radius.
    Circle {
        /// The radius.
        radius: f64,
    },
    /// `intersect-ellipse.js` with these radii.
    Ellipse {
        /// The horizontal radius.
        rx: f64,
        /// The vertical radius.
        ry: f64,
    },
    /// `intersect-polygon.js` with these points, in the shape's own coordinates as the handler
    /// passes them.
    Polygon {
        /// The polygon's points.
        points: Vec<Point>,
    },
    /// The cylinder's own intersect (`cylinder.ts`): the box, then the ellipse at the cap.
    Cylinder {
        /// The cap's horizontal radius.
        rx: f64,
        /// The cap's vertical radius.
        ry: f64,
    },
}

impl Intersect {
    /// The point where the line from the node's centre towards `point` meets the outline, for a
    /// node at `node`, as the reference's intersect function computes it.
    #[must_use]
    pub fn intersect(&self, node: CenterBox, point: Point) -> Point {
        let _ = (node, point);
        todo!("port the intersect functions (task T3)")
    }
}

/// Lays out a node's label as `labelHelper` does before it draws the shape: the label text at
/// `width`, or else the node's wrapping width (`node.width || node.wrappingWidth ||
/// flowchart.wrappingWidth`). `None` when the shape draws no label.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read.
pub fn node_label(
    node: &Node,
    config: &MermaidConfig,
    theme: &Value,
    width: Option<f64>,
) -> Result<Option<LabelLayout>, RenderError> {
    let _ = (node, config, theme, width);
    todo!("port the label part of labelHelper (task T3)")
}

/// The node's `<g>` for the diagram `svg_id`, positioned at its centre, as `insertNode` and
/// `positionNode` leave it: the shape's elements, then the label; inside an `<a>` when the node
/// has a link.
#[must_use]
pub fn node_svg(node: &Node, shape: &ShapeOutput, svg_id: &str) -> String {
    let _ = (node, shape, svg_id);
    todo!("port insertNode and positionNode (task T3)")
}

/// Draws a node's shape, as `insertNode` does with the shape's handler.
///
/// # Errors
///
/// Returns an error for a shape name the registry does not know.
pub fn draw(input: &ShapeInput<'_>) -> Result<ShapeOutput, RenderError> {
    let _ = input;
    todo!("port the shape handlers (task T3)")
}
