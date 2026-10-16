//! Node shapes: a port of `packages/mermaid/src/rendering-util/rendering-elements/nodes.ts`, the
//! shape handlers in `shapes/` and the intersect functions in `intersect/`, for the `neo` and
//! `classic` looks.
//!
//! A shape gets the node and its label laid out at its natural width, and returns its own SVG
//! elements in the node's coordinates (origin at the node's centre), where the label goes, the box
//! of the whole node group, and how edges meet its outline. The box is what the reference's
//! `getBBox` gives for the node's `<g>`, shape and label together, and it is the size the layout
//! uses. See `docs-port/flowchart/render-notes/shapes-core.md`.
//!
//! Shapes that the reference draws with rough.js at roughness 0 are written as deterministic paths
//! with the same geometry (`rough`). The `handDrawn` look is out of scope; it takes the `classic`
//! geometry with plain elements.

mod arcs;
mod braces;
mod circle;
mod collapsed_group;
mod containers;
mod cylinder;
mod intersect;
mod polygon;
mod rect;
mod rects;
mod registry;
mod rough;
mod stadium;
mod state;
mod symbols;
#[cfg(test)]
mod test_support;
mod util;

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::value::Value;

use super::data::Node;
use super::geom::{BBox, CenterBox, Point};
use super::label::{self, Anchor, LabelLayout, LabelSpec};
use crate::RenderError;
use crate::svg::{escape, number};
use registry::Kind;

/// Stands for the diagram's id in a shape's markup, for the inline filter of a small shape. The
/// id is not known when a shape is drawn, so `node_svg` replaces it. It is not a valid identifier
/// character, so it cannot clash with an id.
const SVG_ID_TOKEN: &str = "{svg-id}";

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
    /// Elements drawn over the label, after it: the separator and indicators of a collapsed group.
    /// They are the last children of the node's `<g>`.
    pub after_label: String,
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
        match self {
            Self::Rect => intersect::rect(node, point),
            Self::Circle { radius } => intersect::circle(node, *radius, point),
            Self::Ellipse { rx, ry } => intersect::ellipse(node, *rx, *ry, point),
            Self::Polygon { points } => intersect::polygon(node, points, point),
            Self::Cylinder { rx, ry } => intersect::cylinder(node, *rx, *ry, point),
        }
    }
}

/// Lays out a node's label as `labelHelper` does before it draws the shape: the label text at
/// `width`, or else the node's wrapping width (`node.width || node.wrappingWidth ||
/// flowchart.wrappingWidth`). `None` when the shape draws no label.
///
/// # Errors
///
/// Returns an error for a shape name the registry does not know, or if a bundled font cannot be
/// read.
pub fn node_label(
    node: &Node,
    config: &MermaidConfig,
    theme: &Value,
    width: Option<f64>,
) -> Result<Option<LabelLayout>, RenderError> {
    let kind = registry::resolve(&node.shape).ok_or_else(|| unknown_shape(&node.shape))?;
    if !kind.draws_label() {
        return Ok(None);
    }
    let wrapping_width = width
        .or_else(|| node.width.filter(|width| *width != 0.0))
        .unwrap_or(config.flowchart.wrapping_width);
    // The label's own style declarations: the wrap sees them, as `createText` does for nodes.
    let styles = util::styles2string(node);
    let declarations: Vec<String> = styles
        .label
        .split(';')
        .filter(|declaration| !declaration.is_empty())
        .map(str::to_owned)
        .collect();
    let spec = LabelSpec {
        text: if kind.empties_label() {
            ""
        } else {
            &node.label
        },
        label_type: node.label_type,
        width: wrapping_width,
        is_node: true,
        add_background: node.icon.is_some() || node.img.is_some(),
        // The stylesheet centres node labels (`.node .label text`).
        anchor: Anchor::Middle,
        styles: &declarations,
        font_size: util::theme_number(theme, "fontSize").unwrap_or(config.font_size),
    };
    // The reference's `labelHelper` takes the label's box from `text.getBBox()` (shapes/util.ts
    // 135-140), before `withMinWidth` widens it. That box is a float32 value. Group titles are
    // measured through the same `labelHelper` (createGraph.ts `measureGroupLabel`).
    label::layout(&spec).map(|mut label| {
        label.bbox = label.bbox.browser();
        Some(label)
    })
}

/// The node's `<g>` for the diagram `svg_id`, positioned at its centre, as `insertNode` and
/// `positionNode` leave it: the shape's elements, then the label; inside an `<a>` when the node
/// has a link.
#[must_use]
pub fn node_svg(node: &Node, shape: &ShapeOutput, svg_id: &str) -> String {
    let kind = registry::resolve(&node.shape);
    let styles = util::styles2string(node);
    let shape_svg = shape.svg.replace(SVG_ID_TOKEN, svg_id);
    let after_label = shape.after_label.replace(SVG_ID_TOKEN, svg_id);

    let mut label_svg = String::new();
    if let (Some(label), Some(offset)) = (&shape.label, shape.label_offset) {
        let style = if kind.is_some_and(Kind::sets_label_style) {
            util::attribute("style", &styles.label)
        } else {
            String::new()
        };
        label_svg = format!(
            "<g class=\"label\"{style} transform=\"translate({}, {})\"><rect></rect>{}</g>",
            number(offset.x),
            number(offset.y),
            label.svg(&util::node_text_style(&styles.label)),
        );
    }

    let class = kind.map_or_else(|| util::node_classes(node), |kind| kind.group_class(node));
    let id = format!("{svg_id}-{}", node.dom_id);
    let x = node.x.unwrap_or(0.0);
    let y = node.y.unwrap_or(0.0);
    let transform = format!("translate({}, {})", number(x), number(y));
    let data_look = util::attribute("data-look", &node.look);
    let title = node
        .tooltip
        .as_deref()
        .map_or_else(String::new, |tooltip| util::attribute("title", tooltip));

    let body = format!("{shape_svg}{label_svg}{after_label}");
    match node.link.as_deref() {
        // With a link the `<a>` takes the look and the position, and the `<g>` the class and id.
        Some(link) => {
            let target = node
                .link_target
                .as_deref()
                .map_or_else(String::new, |target| util::attribute("target", target));
            format!(
                "<a xlink:href=\"{}\"{target}{data_look} transform=\"{transform}\"><g class=\"{}\" id=\"{}\"{title}>{body}</g></a>",
                escape(link),
                escape(&class),
                escape(&id),
            )
        }
        None => format!(
            "<g class=\"{}\" id=\"{}\"{data_look} transform=\"{transform}\"{title}>{body}</g>",
            escape(&class),
            escape(&id),
        ),
    }
}

/// Draws a node's shape, as `insertNode` does with the shape's handler.
///
/// # Errors
///
/// Returns an error for a shape name the registry does not know.
pub fn draw(input: &ShapeInput<'_>) -> Result<ShapeOutput, RenderError> {
    let kind =
        registry::resolve(&input.node.shape).ok_or_else(|| unknown_shape(&input.node.shape))?;
    let part = match kind {
        Kind::SquareRect => rect::square_rect(input),
        Kind::RoundedRect => rect::rounded_rect(input),
        Kind::Text => rect::text(input),
        Kind::LabelRect => rect::label_rect(input),
        Kind::Stadium => stadium::stadium(input),
        Kind::Circle => circle::circle(input),
        Kind::DoubleCircle => circle::double_circle(input),
        Kind::Question => polygon::question(input),
        Kind::Hexagon => polygon::hexagon(input),
        Kind::LeanRight => polygon::lean_right(input),
        Kind::LeanLeft => polygon::lean_left(input),
        Kind::Trapezoid => polygon::trapezoid(input),
        Kind::InvTrapezoid => polygon::inv_trapezoid(input),
        Kind::Subroutine => polygon::subroutine(input),
        Kind::RectLeftInvArrow => polygon::rect_left_inv_arrow(input),
        Kind::Cylinder => cylinder::cylinder(input),
        Kind::FilledCircle => state::filled_circle(input),
        Kind::ForkJoin => state::fork_join(input),
        Kind::StateStart => state::state_start(input),
        Kind::StateEnd => state::state_end(input),
        Kind::Anchor => state::anchor(input),
        Kind::CollapsedGroup => collapsed_group::collapsed_group(input),
        Kind::Bang => arcs::bang(input),
        Kind::Cloud => arcs::cloud(input),
        Kind::BowTieRect => arcs::bow_tie_rect(input),
        Kind::CurvedTrapezoid => arcs::curved_trapezoid(input),
        Kind::HalfRoundedRectangle => arcs::half_rounded_rectangle(input),
        Kind::CurlyBraceLeft => braces::curly_brace_left(input),
        Kind::CurlyBraceRight => braces::curly_brace_right(input),
        Kind::CurlyBraces => braces::curly_braces(input),
        Kind::Folder => containers::folder(input),
        Kind::Bucket => containers::bucket(input),
        Kind::Browser => containers::browser(input),
        Kind::Console => containers::console(input),
        Kind::Datastore => rects::datastore(input),
        Kind::Note => rects::note(input),
        Kind::DividedRectangle => rects::divided_rectangle(input),
        Kind::Card => rects::card(input),
        Kind::Choice => symbols::choice(input),
        Kind::Hourglass => symbols::hourglass(input),
        Kind::FlippedTriangle => symbols::flipped_triangle(input),
        Kind::CrossedCircle => symbols::crossed_circle(input),
    };
    // The node group's box is the shape's and the label's together. The label group's box is the
    // text's own box moved by the label's offset: the box widened to the minimum width (which
    // sizes the label) is not in the DOM, so the input's box is the one that counts.
    let mut bbox = part.bbox;
    if let Some(offset) = part.label_offset {
        let text = input
            .label
            .or(part.label.as_ref())
            .map_or_else(BBox::default, |label| label.bbox);
        bbox = bbox.union(text.translate(offset.x, offset.y));
    }
    Ok(ShapeOutput {
        svg: part.svg,
        after_label: part.after_label,
        label_offset: part.label_offset,
        label: part.label,
        bbox,
        intersect: part.intersect,
    })
}

/// The error for a shape name that the registry does not know.
fn unknown_shape(name: &str) -> RenderError {
    RenderError::Options(format!("No such shape: {name}. Please check your syntax."))
}

#[cfg(test)]
mod tests {
    use super::test_support::{draw_box, node, theme};
    use super::*;

    fn group_of(node: &Node, shape: &ShapeOutput) -> String {
        node_svg(node, shape, "svg-id")
    }

    #[test]
    fn a_node_group_takes_the_reference_class_id_look_and_position() {
        let mut node = node("f-circ", "neo");
        node.dom_id = "flowchart-A-0".to_owned();
        node.css_classes = "default".to_owned();
        node.x = Some(10.0);
        node.y = Some(-4.5);
        let shape = draw_box(&node, &LabelLayout::of_box(BBox::default()));
        let svg = group_of(&node, &shape);
        assert!(
            svg.starts_with(
                "<g class=\"node default\" id=\"svg-id-flowchart-A-0\" data-look=\"neo\" transform=\"translate(10, -4.5)\">"
            ),
            "{svg}"
        );
        assert!(svg.ends_with("</g>"), "{svg}");
    }

    #[test]
    #[ignore = "port: the label's SVG is task T2's (LabelLayout::svg); this checks the label group around it"]
    fn a_labelled_node_writes_its_label_group_after_the_shape() {
        let mut node = node("squareRect", "neo");
        node.dom_id = "flowchart-A-0".to_owned();
        node.label = "A".to_owned();
        let shape = draw_box(
            &node,
            &LabelLayout::of_box(BBox::new(-60.0, 0.0, 120.0, 17.0)),
        );
        let svg = group_of(&node, &shape);
        assert!(svg.contains("<rect class=\"basic label-container\" style=\"\" x=\"-76\" y=\"-20.5\" width=\"152\" height=\"41\"/><g class=\"label\" style=\"\" transform=\"translate(0, -8.5)\"><rect></rect>"), "{svg}");
    }

    #[test]
    fn a_linked_node_wraps_its_group_in_an_anchor_that_takes_the_position() {
        let mut node = node("f-circ", "neo");
        node.dom_id = "flowchart-A-0".to_owned();
        node.link = Some("https://example.com/a?x=1&y=2".to_owned());
        node.link_target = Some("_blank".to_owned());
        node.tooltip = Some("Open".to_owned());
        node.x = Some(0.0);
        node.y = Some(0.0);
        let shape = draw_box(&node, &LabelLayout::of_box(BBox::default()));
        let svg = group_of(&node, &shape);
        assert!(
            svg.starts_with(
                "<a xlink:href=\"https://example.com/a?x=1&amp;y=2\" target=\"_blank\" data-look=\"neo\" transform=\"translate(0, 0)\"><g class=\"node default\" id=\"svg-id-flowchart-A-0\" title=\"Open\">"
            ),
            "{svg}"
        );
        assert!(svg.ends_with("</g></a>"), "{svg}");
    }

    #[test]
    fn a_shape_without_a_label_has_no_label_group() {
        let node = node("stateStart", "neo");
        let shape = draw_box(&node, &LabelLayout::of_box(BBox::default()));
        assert_eq!(shape.label, None);
        assert!(!group_of(&node, &shape).contains("class=\"label\""));
    }

    #[test]
    fn the_label_of_a_shape_without_one_is_none_without_laying_it_out() {
        // `node_label` returns before the label layout for these shapes, so it runs without the
        // layout of `label` (task T2).
        let config = MermaidConfig::default();
        let theme = theme();
        for shape in ["f-circ", "fork", "sm-circ", "fr-circ", "anchor"] {
            let node = node(shape, "neo");
            assert_eq!(
                node_label(&node, &config, &theme, None).expect("known"),
                None,
                "{shape}"
            );
        }
    }

    #[test]
    fn an_unknown_shape_is_an_error_as_no_such_shape_is_in_the_reference() {
        let config = MermaidConfig::default();
        let theme = theme();
        let node = node("no-such-shape", "neo");
        let error = node_label(&node, &config, &theme, None).expect_err("unknown");
        assert!(
            error.to_string().contains("No such shape: no-such-shape"),
            "{error}"
        );
    }
}
