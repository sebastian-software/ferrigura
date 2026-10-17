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
mod block_arrow;
mod braces;
mod circle;
mod class_box;
mod collapsed_group;
mod composite;
mod containers;
mod cylinder;
mod decorated;
mod er_box;
mod icon;
mod intersect;
mod person;
mod polygon;
mod rect;
mod rect_with_title;
mod rects;
mod registry;
mod rough;
mod stadium;
mod state;
mod symbols;
#[cfg(test)]
mod test_support;
mod util;
mod wave;

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::value::Value;
use ferrigura_syntax::sanitize::sanitize_text;

use super::clusters;
use super::data::Node;
use super::geom::{BBox, CenterBox, Point};
use super::label::{self, Anchor, LabelLayout, LabelSpec};
use crate::RenderError;
use crate::common::decode_entities;
use crate::svg::{escape, number};
use registry::Kind;
pub(crate) use util::styles_from_array;

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
    /// Further labels the shape draws in its own elements (the cells of an ER entity, the rows of a
    /// class box, the title and the rows of a `rectWithTitle`), which the font subsets of the
    /// diagram cover as they cover `label`.
    pub texts: Vec<LabelLayout>,
    /// Attributes the shape writes on the node's `<g>`, after its `data-look`, each with its
    /// leading space (the colour slot of an ER entity). Empty for the other shapes.
    pub group_attributes: String,
    /// The box of the whole node group, in the node's coordinates.
    pub bbox: BBox,
    /// How edges meet the outline.
    pub intersect: Intersect,
    /// The box the shape's elements cover, when it is larger than `bbox` (a class box).
    pub extent: Option<BBox>,
    /// The box of the shape's own outline or elements, without its label: what `updateNodeBounds`
    /// reads from the element, which `intersect` uses as the node's size (a block's rectangle).
    pub element: BBox,
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
    /// The tilted cylinder's own intersect (`tiltedCylinder.ts`): the box, then the cap on its side.
    TiltedCylinder {
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
            Self::TiltedCylinder { rx, ry } => intersect::tilted_cylinder(node, *rx, *ry, point),
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
    anchor: Anchor,
) -> Result<Option<LabelLayout>, RenderError> {
    // A group's label is measured whatever its cluster kind: `measureGroupLabel` runs `labelHelper`
    // on every subgraph, and the cluster kinds that paint no title (`noteGroup`, `divider`) still
    // size their group by it (`createGraph.ts` 42-59, 169-171). Such a group has no shape of its own.
    let kind = match registry::resolve_node(node) {
        Some(kind) => {
            if !kind.draws_label() {
                return Ok(None);
            }
            Some(kind)
        }
        None if node.is_group && clusters::is_cluster_kind(&node.shape) => None,
        None => return Err(unknown_shape(&node.shape)),
    };
    // `node.wrappingWidth` (the state diagram's `state.wrappingWidth`) before the flowchart's.
    let default_width = node
        .wrapping_width
        .unwrap_or(config.flowchart.wrapping_width);
    // The icon and image handlers set the node's width before they lay out the label: the icon
    // size or the image's width, at least the wrapping width.
    let wrapping_width = match kind {
        Some(Kind::Icon | Kind::IconCircle | Kind::IconRounded | Kind::IconSquare) => {
            icon::label_width(node, default_width)
        }
        Some(Kind::ImageSquare) => icon::image_label_width(node, default_width)?,
        // The hexagon and the cylinder narrow the node's own width before they lay out the label, and
        // the label wraps at the narrowed width (`hexagon.ts` 40-41, `cylinder.ts` 65-71).
        Some(Kind::Hexagon) => polygon::hexagon_wrap_width(node)
            .or(width)
            .or_else(|| node.width.filter(|width| *width != 0.0))
            .unwrap_or(default_width),
        Some(Kind::Cylinder) => cylinder::narrowed_width(node)
            .or(width)
            .or_else(|| node.width.filter(|width| *width != 0.0))
            .unwrap_or(default_width),
        _ => width
            .or_else(|| node.width.filter(|width| *width != 0.0))
            .unwrap_or(default_width),
    };
    // The label's own style declarations: the wrap sees them, as `createText` does for nodes.
    let styles = util::styles2string(node);
    let declarations: Vec<String> = styles
        .label
        .split(';')
        .filter(|declaration| !declaration.is_empty())
        .map(str::to_owned)
        .collect();
    // `labelHelper` sanitises the decoded label again when it draws it (`shapes/util.ts`), so the
    // label of an `@{ label }` node, which the DB did not sanitise, is sanitised here.
    let text = sanitize_text(&decode_entities(&node.label));
    let spec = LabelSpec {
        text: if kind.is_some_and(Kind::empties_label) {
            ""
        } else {
            &text
        },
        label_type: node.label_type,
        width: wrapping_width,
        is_node: true,
        add_background: node.icon.is_some() || node.img.is_some(),
        // The diagram's stylesheet decides where the lines of a node label start: the flowchart's
        // centres them (`.node .label text`), the state diagram's does not.
        anchor,
        wrap_in_style: true,
        styles: &declarations,
        font_size: util::theme_number(theme, "fontSize").unwrap_or(config.font_size),
        title: false,
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
pub fn node_svg(node: &Node, shape: &ShapeOutput, svg_id: &str, links: LinkForm) -> String {
    let kind = registry::resolve_node(node);
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
        // `note.ts` gives its label group a class of its own.
        let label_class = if kind == Some(Kind::Note) {
            "label noteLabel"
        } else {
            "label"
        };
        label_svg = format!(
            "<g class=\"{label_class}\"{style} transform=\"translate({}, {})\"><rect></rect>{}</g>",
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
    let group_attributes = &shape.group_attributes;
    let title = node
        .tooltip
        .as_deref()
        .map_or_else(String::new, |tooltip| util::attribute("title", tooltip));

    let body = format!("{shape_svg}{label_svg}{after_label}");
    match node.link.as_deref() {
        // The state diagram wraps a clicked node after its render (`stateRenderer-v3-unified.ts`
        // 80-133): the anchor has no transform, and the tooltip is on the anchor and on the node.
        Some(link) if links == LinkForm::PostRender => {
            let tooltip = node
                .tooltip
                .as_deref()
                .map_or_else(String::new, |tooltip| util::attribute("title", tooltip));
            format!(
                "<a xlink:href=\"{}\"{tooltip}><g class=\"{}\" id=\"{}\"{data_look} transform=\"{transform}\"{tooltip}>{body}</g></a>",
                escape(link),
                escape(&class),
                escape(&id),
            )
        }
        // With a link the `<a>` takes the look and the position, and the `<g>` the class and id.
        Some(link) => {
            let target = node
                .link_target
                .as_deref()
                .map_or_else(String::new, |target| util::attribute("target", target));
            format!(
                "<a xlink:href=\"{}\"{target}{data_look} transform=\"{transform}\"><g class=\"{}\" id=\"{}\"{group_attributes}{title}>{body}</g></a>",
                escape(link),
                escape(&class),
                escape(&id),
            )
        }
        None => format!(
            "<g class=\"{}\" id=\"{}\"{data_look}{group_attributes} transform=\"{transform}\"{title}>{body}</g>",
            escape(&class),
            escape(&id),
        ),
    }
}

/// How a node with a link is written: the flowchart's anchor carries the node's transform, and the
/// state diagram's anchor wraps the node after the render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkForm {
    /// The flowchart's: `<a>` with the transform and the target.
    Flowchart,
    /// The state diagram's: `<a>` around the node, with the tooltip.
    PostRender,
}

/// Draws a node's shape, as `insertNode` does with the shape's handler.
///
/// # Errors
///
/// Returns an error for a shape name the registry does not know.
pub fn draw(input: &ShapeInput<'_>) -> Result<ShapeOutput, RenderError> {
    let kind =
        registry::resolve_node(input.node).ok_or_else(|| unknown_shape(&input.node.shape))?;
    // The box the elements cover, when it is larger than the node's, and the texts a class box
    // draws beside its label (its rows).
    let mut extent = None;
    let mut texts = Vec::new();
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
        Kind::RectWithTitle => {
            // The title and the rows are labels of the shape, so the font subsets cover them.
            let (part, title_and_rows) = rect_with_title::rect_with_title(input)?;
            texts = title_and_rows;
            part
        }
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
        Kind::Person => person::person(input),
        Kind::LightningBolt => polygon::lightning_bolt(input),
        Kind::LinedCylinder => cylinder::lined_cylinder(input),
        Kind::TiltedCylinder => cylinder::tilted_cylinder(input),
        Kind::LinedWaveEdgedRect => wave::lined_wave_edged_rect(input),
        Kind::WaveEdgedRectangle => wave::wave_edged_rectangle(input),
        Kind::MultiWaveEdgedRectangle => wave::multi_wave_edged_rectangle(input),
        Kind::TaggedWaveEdgedRectangle => wave::tagged_wave_edged_rectangle(input),
        Kind::WaveRectangle => wave::wave_rectangle(input),
        Kind::MultiRect => decorated::multi_rect(input),
        Kind::ShadedProcess => decorated::shaded_process(input),
        Kind::SlopedRect => decorated::sloped_rect(input),
        Kind::TaggedRect => decorated::tagged_rect(input),
        Kind::WindowPane => decorated::window_pane(input),
        Kind::TrapezoidalPentagon => polygon::trapezoidal_pentagon(input),
        Kind::Triangle => polygon::triangle(input),
        Kind::Icon => icon::icon(input),
        Kind::IconSquare => icon::icon_square(input),
        Kind::IconRounded => icon::icon_rounded(input),
        Kind::IconCircle => icon::icon_circle(input),
        Kind::ImageSquare => icon::image_square(input)?,
        // The entity draws its own labels and their group box, so it returns its output whole.
        Kind::ErBox => return er_box::er_box(input),
        Kind::Composite => composite::composite(input),
        Kind::BlockArrow => block_arrow::block_arrow(input),
        Kind::ClassBox => {
            let class_box::ClassBox {
                part,
                covered,
                texts: rows,
            } = class_box::class_box(input)?;
            extent = Some(covered);
            texts = rows;
            part
        }
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
        element: part.bbox,
        svg: part.svg,
        after_label: part.after_label,
        label_offset: part.label_offset,
        label: part.label,
        group_attributes: String::new(),
        bbox,
        intersect: part.intersect,
        extent,
        texts,
    })
}

/// The error for a shape name that the registry does not know.
fn unknown_shape(name: &str) -> RenderError {
    // A node without a shape reads `undefined` in the reference's message (`nodes.ts` 31-35).
    let name = if name.is_empty() { "undefined" } else { name };
    RenderError::Options(format!("No such shape: {name}. Please check your syntax."))
}

#[cfg(test)]
mod tests {
    use super::test_support::{draw_box, node, theme};
    use super::*;

    fn group_of(node: &Node, shape: &ShapeOutput) -> String {
        node_svg(node, shape, "svg-id", LinkForm::Flowchart)
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
                node_label(&node, &config, &theme, None, Anchor::Middle).expect("known"),
                None,
                "{shape}"
            );
        }
    }

    /// `shapes/util.ts` sanitises the decoded label when it draws it, as DOMPurify writes a `<` that
    /// starts no tag as `&lt;`, which the plain-text split then reads as `<`. A label that already
    /// holds `&lt;` (the DB's output) is still drawn as `<`, and so is a raw `<11` (an `@{ label }`
    /// node, which the DB leaves as it is): `nightly-check/v2/03-should-render-raw-strings`.
    #[test]
    fn a_node_label_is_sanitised_and_decoded_as_it_is_drawn() {
        let config = MermaidConfig::default();
        let theme = theme();
        for (text, shown_as) in [("<11", "<11"), ("&lt;11", "<11")] {
            let mut node = node("squareRect", "neo");
            node.label = text.to_owned();
            let label = node_label(&node, &config, &theme, None, Anchor::Middle)
                .expect("known")
                .expect("a label");
            let shown: String = label
                .lines
                .iter()
                .flat_map(|line| &line.runs)
                .map(|run| run.text.as_str())
                .collect();
            assert_eq!(shown, shown_as, "label {text:?}");
        }
    }

    #[test]
    fn an_unknown_shape_is_an_error_as_no_such_shape_is_in_the_reference() {
        let config = MermaidConfig::default();
        let theme = theme();
        let node = node("no-such-shape", "neo");
        let error = node_label(&node, &config, &theme, None, Anchor::Middle).expect_err("unknown");
        assert!(
            error.to_string().contains("No such shape: no-such-shape"),
            "{error}"
        );
    }
}
