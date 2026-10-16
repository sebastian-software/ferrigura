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

use ferrigura_model::er::ErAttribute;
use ferrigura_model::flowchart::LabelType;
use ferrigura_model::value::Value;
use serde::Serialize;

use super::geom::Point;

/// The fields that only an entity of an ER diagram carries (`EntityNode` in
/// `diagrams/er/erTypes.ts`): the alias and the attributes, which its `erBox` shape draws. They are
/// flattened into the node, as the reference's entity object has them next to the node's fields.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityFields {
    /// The alias, which replaces the name as the drawn label; empty if none.
    pub alias: String,
    /// The attributes, in the order the DB's entity holds them (the source order of the model).
    pub attributes: Vec<ErAttribute>,
}

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
    /// The padding the shapes add around the label.
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
    /// `flowchart.minNodeWidth`, set on vertices only (`state.minNodeWidth` on a state diagram's
    /// non-groups).
    pub min_width: Option<f64>,
    /// The colour slot of a subgraph.
    pub color_index: Option<u32>,
    /// The subgraph's metadata.
    pub metadata: BTreeMap<String, Value>,
    /// The alias and attributes of an ER entity; `None` for every other node.
    #[serde(flatten)]
    pub entity: Option<EntityFields>,
    /// The corner radius of a rounded shape. A `rect` with both radii is a rounded rectangle
    /// (`nodes.ts` 22-29). Only the state diagram sets them; a flowchart node has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rx: Option<f64>,
    /// See `rx`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ry: Option<f64>,
    /// The width at which the label wraps (`state.wrappingWidth` on a state diagram).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrapping_width: Option<f64>,
    /// The reference's `type` of the node: `group` for a composite state, `node` for a note.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub node_type: Option<String>,
    /// The reference's `centerLabel`, which the state diagram sets on its states.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub center_label: Option<bool>,
    /// The side of a note: `left of` or `right of`, as written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
    /// The descriptions of a state that has two or more of them, after the first one (which is
    /// the label). `rectWithTitle` draws them as its rows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Vec<String>>,

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
    /// The label's centre after layout.
    #[serde(skip)]
    pub x: Option<f64>,
    /// See `x`.
    #[serde(skip)]
    pub y: Option<f64>,
    /// The points of the path after layout, in absolute coordinates.
    #[serde(skip)]
    pub points: Vec<Point>,
}
