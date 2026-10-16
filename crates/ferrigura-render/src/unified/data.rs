//! The layout data: a port of the `LayoutData`, `Node` and `Edge` types in
//! `packages/mermaid/src/rendering-util/types.ts`, which every diagram on the unified renderer
//! (flowchart, class, state, ER) fills with its own `getData`.
//!
//! The structs mirror the reference's objects field by field, so that each step of the pipeline can
//! be ported as the reference writes it. Fields that a later step sets (sizes, positions, points)
//! are `Option`s or empty until that step has run.
//!
//! The serialised form is the output of `getData` with the reference's names: an `undefined` field
//! of the reference is `null` (or `{}` and `[]` for the metadata and the lists), and the fields that
//! only a later step sets are not serialised.

use std::collections::BTreeMap;

use ferrigura_model::class::ClassMember;
use ferrigura_model::flowchart::LabelType;
use ferrigura_model::value::Value;
use serde::Serialize;

use super::geom::Point;

/// The nodes and edges of a diagram, in the order its `getData` gives them.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutData {
    /// Subgraphs first, in reverse closing order, then the vertices in creation order.
    pub nodes: Vec<Node>,
    /// The edges in creation order.
    pub edges: Vec<Edge>,
    /// The diagram's direction as the DB gives it (`TB`, `TD`, `LR`, `RL`, `BT`).
    pub direction: String,
    /// The `linkStyle default` styles, which the reference keeps on the edge array.
    pub default_style: Option<Vec<String>>,
    /// The `linkStyle default interpolate` value, kept on the edge array as well.
    pub default_interpolate: Option<String>,
    /// Whether the diagram's stylesheet centres node labels (`.node .label text { text-anchor:
    /// middle }`, `flowchart/styles.ts`). The flowchart does; the class diagram's sheet does not, so
    /// its node labels (notes) start at the node's origin. Not part of `getData`.
    #[serde(skip)]
    pub centre_node_labels: bool,
    /// The diagram type the renderer sets on the layout data (`flowchart-v2`, `classDiagram`,
    /// `stateDiagram`, `erDiagram`): it names the markers and their class. Not part of `getData`,
    /// so it is not written to the layout data's JSON.
    #[serde(skip)]
    pub diagram_type: String,
}

/// A node of the layout: a vertex, or a subgraph (`is_group`), or a collapsed subgraph.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    /// The vertex or subgraph id.
    pub id: String,
    /// The id of the SVG element without the diagram's prefix: `flowchart-<id>-<n>` for a vertex,
    /// the id for a subgraph.
    pub dom_id: String,
    /// The label text.
    pub label: String,
    /// How the label is written.
    pub label_type: LabelType,
    /// The label's own style; empty for flowcharts.
    pub label_style: String,
    /// The innermost subgraph that contains the node.
    pub parent_id: Option<String>,
    /// The padding the shapes add around the label. Zero when the node has none: the class
    /// vertices of `classDb.ts` `getData` carry no padding, so the field is left out for them.
    #[serde(skip_serializing_if = "is_zero")]
    pub padding: f64,
    /// The node's `style` declarations, in statement order.
    pub css_styles: Vec<String>,
    /// The declarations of its classes (`getCompiledStyles`).
    pub css_compiled_styles: Vec<String>,
    /// The class attribute: `default ` and the classes for a vertex.
    pub css_classes: String,
    /// The direction of a subgraph, as written.
    pub dir: Option<String>,
    /// The look: `neo`, `classic` or `handDrawn`.
    pub look: String,
    /// The link of a `click` statement.
    pub link: Option<String>,
    /// The link's target.
    pub link_target: Option<String>,
    /// The tooltip.
    pub tooltip: Option<String>,
    /// The icon of an icon shape.
    pub icon: Option<String>,
    /// The label position of an icon or image shape (`t` or `b`).
    pub pos: Option<String>,
    /// The image of an image shape.
    pub img: Option<String>,
    /// The asset width of an icon or image shape.
    pub asset_width: Option<f64>,
    /// The asset height of an icon or image shape.
    pub asset_height: Option<f64>,
    /// The aspect constraint of an image shape.
    pub constraint: Option<String>,
    /// Whether this is an expanded subgraph.
    pub is_group: bool,
    /// The shape name, as the registry knows it.
    pub shape: String,
    /// `flowchart.minNodeWidth`, set on vertices only.
    pub min_width: Option<f64>,
    /// The colour slot of a subgraph, or of a class.
    pub color_index: Option<u32>,
    /// The subgraph's metadata.
    pub metadata: BTreeMap<String, Value>,
    /// The class data of a class node: what its `classBox` shape draws (see [`ClassNodeData`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<ClassNodeData>,

    /// The size the layout uses: the measured box of a leaf, ELK's size afterwards.
    #[serde(skip)]
    pub width: Option<f64>,
    /// See `width`.
    #[serde(skip)]
    pub height: Option<f64>,
    /// The centre after layout.
    #[serde(skip)]
    pub x: Option<f64>,
    /// See `x`.
    #[serde(skip)]
    pub y: Option<f64>,
    /// The measured size of a subgraph's title.
    #[serde(skip)]
    pub label_bbox: Option<(f64, f64)>,
}

impl Node {
    /// Whether the node is drawn in the `neo` look.
    #[must_use]
    pub fn is_neo(&self) -> bool {
        self.look == "neo"
    }
}

/// What a class node carries beyond the common node fields (`classDb.ts` `getData`, which spreads
/// the DB's `ClassNode` into the node). The names are the reference's.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassNodeData {
    /// The row that names the class: the name, with the generic type written as `&lt;T&gt;`.
    pub text: String,
    /// The annotations; the box draws the first one.
    pub annotations: Vec<String>,
    /// The attributes, in source order.
    pub members: Vec<ClassMember>,
    /// The methods, in source order.
    pub methods: Vec<ClassMember>,
    /// The `style` and `cssClass` declarations of the class, in order.
    pub styles: Vec<String>,
    /// The class's parent namespace as the DB holds it (before the compact-mode redirection).
    pub parent: Option<String>,
    /// Whether a `callback` was set on the class.
    pub have_callback: Option<bool>,
}

/// A serde predicate: the field is left out when it is zero.
fn is_zero(value: &f64) -> bool {
    *value == 0.0
}

/// An edge of the layout.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    /// The edge id: `L_<start>_<end>_<n>`, or the user's id.
    pub id: String,
    /// Whether the user gave the id.
    pub is_user_defined_id: bool,
    /// The start node, after redirection to a collapsed subgraph.
    pub start: String,
    /// The end node, after redirection to a collapsed subgraph.
    pub end: String,
    /// The arrow type of the link (`arrow_point`, `double_arrow_cross`, ...), `type` in the reference.
    #[serde(rename = "type")]
    pub edge_type: String,
    /// The label text; empty when there is none.
    pub label: String,
    /// How the label is written.
    pub label_type: LabelType,
    /// The label position; `c`.
    pub labelpos: String,
    /// The stroke: `normal`, `thick`, `dotted` or `invisible`.
    pub thickness: String,
    /// The stroke again, as the reference's `pattern`.
    pub pattern: String,
    /// The minimum length.
    pub minlen: Option<u32>,
    /// The class attribute of the path.
    pub classes: String,
    /// The marker at the start: `none`, `arrow_point`, `arrow_circle`, `arrow_cross`, ...
    pub arrow_type_start: String,
    /// The marker at the end.
    pub arrow_type_end: String,
    /// The arrowhead style.
    pub arrowhead_style: String,
    /// The arrowhead (`normal` or `none`), set in prepare.
    #[serde(skip)]
    pub arrowhead: Option<String>,
    /// The declarations of the edge's classes.
    pub css_compiled_styles: Vec<String>,
    /// The label's style declarations.
    pub label_style: Vec<String>,
    /// The path's style declarations: `linkStyle default` first, then the edge's own.
    pub style: Vec<String>,
    /// The look.
    pub look: String,
    /// Whether the edge is animated.
    pub animate: Option<bool>,
    /// The animation speed.
    pub animation: Option<String>,
    /// The curve: the edge's interpolation or the default after `getData`; `rounded` or `linear`
    /// after layout.
    pub curve: Option<String>,

    /// The label's measured width.
    #[serde(skip)]
    pub width: Option<f64>,
    /// The label's measured height.
    #[serde(skip)]
    pub height: Option<f64>,
    /// The start cardinality of a class relation (`startLabelRight`): empty when there is none.
    /// Left out of the other edges.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_label_right: Option<String>,
    /// The end cardinality of a class relation (`endLabelLeft`). Left out of the other edges.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_label_left: Option<String>,

    /// The label's centre after layout.
    #[serde(skip)]
    pub x: Option<f64>,
    /// See `x`.
    #[serde(skip)]
    pub y: Option<f64>,
    /// The points of the path after layout, in absolute coordinates.
    #[serde(skip)]
    pub points: Vec<Point>,
    /// The terminal labels (cardinalities) of the edge, in the order `edges.js` records them
    /// (`terminalLabelSizes`). Only class relations have them.
    #[serde(skip)]
    pub terminals: Vec<Terminal>,
    /// The ports of the edge after layout: the first and last point ELK routed, before the edge is
    /// clipped at the outlines (`elkPorts`). Set when a terminal label has a centre.
    #[serde(skip)]
    pub ports: Option<(Point, Point)>,
}

/// Which end of an edge a terminal label belongs to, and on which side of the line it sits
/// (`TerminalLabelKey`, `rendering-util/types.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalKey {
    /// The start, on the left of the direction of travel.
    StartLeft,
    /// The start, on the right of the direction of travel.
    StartRight,
    /// The end, on the left of the direction of travel.
    EndLeft,
    /// The end, on the right of the direction of travel.
    EndRight,
}

impl TerminalKey {
    /// Whether the label is at the start of the edge.
    #[must_use]
    pub fn at_start(self) -> bool {
        matches!(self, Self::StartLeft | Self::StartRight)
    }

    /// Whether the label is on the right of the direction of travel.
    #[must_use]
    pub fn on_right(self) -> bool {
        matches!(self, Self::StartRight | Self::EndRight)
    }
}

/// A terminal label: its text, its laid-out size (the text's box, unpadded), and its centre once
/// the layout has placed it.
#[derive(Debug, Clone, PartialEq)]
pub struct Terminal {
    /// The end and side.
    pub key: TerminalKey,
    /// The text.
    pub text: String,
    /// The laid-out label (its text box).
    pub label: crate::unified::label::LabelLayout,
    /// The width of the text's box.
    pub width: f64,
    /// The height of the text's box.
    pub height: f64,
    /// The centre after layout, when ELK placed the label (`terminalLabelCenters`).
    pub centre: Option<Point>,
}
