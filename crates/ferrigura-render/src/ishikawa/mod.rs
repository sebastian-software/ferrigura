//! The ishikawa (fishbone) diagram: a port of `packages/mermaid/src/diagrams/ishikawa/
//! ishikawaRenderer.ts` (`draw`) and `ishikawaStyles.ts`, with the research in
//! `docs-port/ishikawa/NOTES.md` (section 6) and the decisions of `docs-port/ishikawa/PLAN.md`.
//!
//! The layout is arithmetic on the angles of the spine and on the boxes of the labels. The four
//! boxes the reference reads back with `getBBox` are computed from the fonts (`text`): the box of
//! each label, the tight bounds of the head's curve, and the root's box as the union of everything
//! drawn (`Drawing::bounds`). The geometry of the causes and their bones is in [`bones`]; the style
//! sheet is in [`style`].
//!
//! The `look: handDrawn` of the reference is drawn by roughjs, which is not ported: the classic
//! geometry is drawn for it, as the other ported types do (`docs-port/ishikawa/PLAN.md`, section 10).

mod bones;
mod style;
mod text;

use std::f64::consts::PI;

use ferrigura_jsmath::{cos, sin};
use ferrigura_model::config::MermaidConfig;
use ferrigura_model::ishikawa::{IshikawaDiagram, IshikawaNode};
use ferrigura_text::{EmbeddedFamily, Face, FontFamily, FontStyle, FontWeight, embed_runs, face};

use crate::RenderError;
use crate::a11y;
use crate::common::Element;
use crate::common::viewbox::{BBox as ViewBox, setup_graph_viewbox};
use crate::svg::number;
use crate::theme::Themes;
use crate::unified::geom::BBox;
use crate::variables::{px, text_of};
use text::{Anchor, Baseline, Label, Style, as_runs, wrap_text};

/// The XML namespace of the root element.
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// `FONT_SIZE_DEFAULT` of the reference (`ishikawaRenderer.ts` 19): the size a label falls back to
/// when the theme gives none.
const FONT_SIZE_DEFAULT: f64 = 14.0;

/// The length of the spine on each side before the causes lengthen it (`SPINE_BASE_LENGTH`, 20).
const SPINE_BASE_LENGTH: f64 = 250.0;

/// The length of a bone without children (`BONE_STUB`, 21).
const BONE_STUB: f64 = 30.0;

/// The base length of a bone with children, and its growth per child (`BONE_BASE`, `BONE_PER_CHILD`,
/// 22-23).
const BONE_BASE: f64 = 60.0;
const BONE_PER_CHILD: f64 = 5.0;

/// The angle of the causes from the spine, in radians: 82 degrees (`ANGLE`, 24).
const ANGLE: f64 = (82.0 * PI) / 180.0;

/// The distance of a cause's label below or above its line's end (`ishikawaRenderer.ts` 279).
const CAUSE_OFFSET: f64 = 11.0;

/// The size of the head's label, written by its own stylesheet rule (`ishikawaStyles.ts` 42).
const HEAD_LABEL_SIZE: f64 = 14.0;

/// The line step of a label, as a share of the configured font size (`ishikawaRenderer.ts` 467).
const LINE_STEP: f64 = 1.05;

/// The diagram padding the reference uses when the configuration gives none
/// (`ishikawaRenderer.ts` 49).
const PADDING_DEFAULT: f64 = 20.0;

/// What the ishikawa renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct IshikawaInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a IshikawaDiagram,
    /// The effective configuration, in the ishikawa diagram's scope.
    pub config: &'a MermaidConfig,
    /// The colours and sizes of the theme.
    pub themes: Themes<'a>,
    /// The id of the SVG element. It scopes every selector and prefixes every element id, so it
    /// must be a CSS identifier.
    pub id: &'a str,
}

/// What the layout and the drawing read: the trigonometry of the causes, the sizes of the text and
/// the colours.
pub(super) struct Context {
    /// `cos(ANGLE)` and `sin(ANGLE)`, from `ferrigura_jsmath`, which matches V8's values.
    pub cos_a: f64,
    pub sin_a: f64,
    /// The font size of the layout: the configured `fontSize`, which spaces the lines.
    pub font_size: f64,
    /// The size the labels are drawn at: the theme's `fontSize`, from the stylesheet.
    pub label_size: f64,
    /// The faces: the regular one for the labels and the bold one for the head (weight 600).
    pub regular: &'static Face,
    pub bold: &'static Face,
    /// The colours of the presentation attributes, the light scheme's.
    pub line: String,
    pub main: String,
    pub text: String,
    /// The id of the arrow marker, which the branches refer to.
    pub marker_id: String,
}

impl Context {
    /// The distance between two lines of a label.
    pub fn line_height(&self) -> f64 {
        self.font_size * LINE_STEP
    }
}

/// What the drawing collects as it is written: the root's box, which the viewBox takes, and the text
/// of every label with its face, which the subset of the font is cut from.
#[derive(Default)]
pub(super) struct Drawing {
    /// The union of everything drawn, in the user space of the diagram (`getBBox` of the root).
    bounds: Option<BBox>,
    /// The text of every label and the face it is set in.
    runs: Vec<(String, &'static Face)>,
}

impl Drawing {
    /// Adds a box to the box of the root. An empty box still counts: an SVG's `getBBox` takes the
    /// zero-area boxes of lines into the union.
    pub fn include(&mut self, bounds: BBox) {
        self.bounds = Some(self.bounds.map_or(bounds, |so_far| so_far.union(bounds)));
    }

    /// Draws a label: its box joins the root's, and its lines the font subset.
    pub fn label(&mut self, label: &Label, fill: &str) -> Element {
        self.include(label.bbox);
        self.runs.extend(label.runs());
        label.element(fill)
    }
}

/// Linear interpolation, as `lerp` of the reference (`a + (b - a) * t`).
pub(super) fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Renders the diagram to an SVG document. Every look draws the classic geometry: the handDrawn
/// look is out of scope (see the module's notes).
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read or subset.
pub fn render(input: &IshikawaInput<'_>) -> Result<String, RenderError> {
    let config = input.config;
    let themes = input.themes;
    let variables = themes.primary();
    let id = input.id;

    let font_size = if config.font_size.is_finite() {
        config.font_size
    } else {
        FONT_SIZE_DEFAULT
    };
    let label_size = px(variables, "fontSize").unwrap_or(FONT_SIZE_DEFAULT);
    let regular = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
    let bold = face(FontFamily::Sans, FontWeight::Bold, FontStyle::Normal);
    let context = Context {
        cos_a: cos(ANGLE),
        sin_a: sin(ANGLE),
        font_size,
        label_size,
        regular,
        bold,
        line: text_of(variables, "lineColor").unwrap_or_else(|| "#333333".to_owned()),
        main: text_of(variables, "mainBkg").unwrap_or_else(|| "#ffffff".to_owned()),
        text: text_of(variables, "textColor").unwrap_or_else(|| "#333333".to_owned()),
        marker_id: format!("ishikawa-arrow-{id}"),
    };

    let padding = config.ishikawa.diagram_padding as f64;
    let padding = if padding.is_finite() {
        padding
    } else {
        PADDING_DEFAULT
    };
    let mut drawing = Drawing::default();
    let body = match input.diagram.root.as_ref() {
        Some(root) => Some(draw_root(&context, &mut drawing, root)?),
        None => None,
    };

    // The fonts: every line of every label, in the face each one is set in.
    let families: Vec<EmbeddedFamily> = embed_runs(&as_runs(&drawing.runs))?;
    let family = families.first().map_or_else(
        || text_of(variables, "fontFamily").unwrap_or_else(|| "sans-serif".to_owned()),
        |family| format!("\"{}\"", family.name),
    );
    let label_size_css = text_of(variables, "fontSize").unwrap_or_else(|| "14px".to_owned());
    let sheet = style::sheet(id, themes, &family, &label_size_css, &families);

    let mut root = Element::new("svg")
        .with("id", id)
        .with("xmlns", SVG_NAMESPACE);
    if body.is_some() {
        // `setupGraphViewbox` with the box of the drawing and `ishikawa.diagramPadding` and
        // `ishikawa.useMaxWidth` (`ishikawaRenderer.ts` 28-34). Without a head the reference
        // returns before it, so the root has no viewBox.
        let bounds = drawing.bounds.unwrap_or_default();
        setup_graph_viewbox(
            &mut root,
            ViewBox {
                x: bounds.x,
                y: bounds.y,
                width: bounds.width,
                height: bounds.height,
            },
            padding,
            config.ishikawa.use_max_width,
        );
    }
    root.set("role", a11y::SVG_ROLE);
    root.set("aria-roledescription", "ishikawa");
    let mut root = root
        .child(Element::new("style").raw_text(sheet))
        // The empty group `mermaidAPI` appends before the renderer draws.
        .child(Element::new("g"));
    if let Some(body) = body {
        root.push_child(body);
    }
    Ok(root.render())
}

/// The `g.ishikawa` of a diagram with a head: the marker, the spine, the head and the pairs of causes
/// (`ishikawaRenderer.ts` 52-150).
fn draw_root(
    context: &Context,
    drawing: &mut Drawing,
    root: &IshikawaNode,
) -> Result<Element, RenderError> {
    let causes = &root.children;
    // The spine starts at the head: `spineX` is 0 there, and 20 further left once a cause is drawn
    // (`ishikawaRenderer.ts` 97).
    let mut spine_x = 0.0;
    let mut spine_y = SPINE_BASE_LENGTH;
    let mut pairs = Vec::new();
    if !causes.is_empty() {
        spine_x -= 20.0;
        let (upper_len, lower_len) = side_lengths(context, causes);
        spine_y = upper_len.max(SPINE_BASE_LENGTH);
        for pair in 0..causes.len().div_ceil(2) {
            let mut elements = Vec::new();
            let mut left = f64::INFINITY;
            // The upper cause is the even one, and the lower the odd one (`:128-131`).
            for (cause, direction, length) in [
                (causes.get(pair * 2), -1.0, upper_len),
                (causes.get(pair * 2 + 1), 1.0, lower_len),
            ] {
                if let Some(cause) = cause {
                    let (drawn, text_left) = bones::draw_branch(
                        context,
                        drawing,
                        cause,
                        (spine_x, spine_y),
                        direction,
                        length,
                    )?;
                    elements.extend(drawn);
                    left = left.min(text_left);
                }
            }
            let mut pair_group = Element::new("g").with("class", "ishikawa-pair");
            for element in elements {
                pair_group.push_child(element);
            }
            pairs.push(pair_group);
            // The next pair starts at the leftmost text of this one (`ishikawaRenderer.ts` 136-139).
            spine_x = left;
        }
    }

    let mut group = Element::new("g").with("class", "ishikawa");
    group.push_child(marker(context));
    group.push_child(spine_line(context, drawing, spine_x, spine_y));
    group.push_child(head(context, drawing, root, spine_y)?);
    for pair in pairs {
        group.push_child(pair);
    }
    Ok(group)
}

/// The arrow marker, which the branches refer to (`ishikawaRenderer.ts` 64-77).
fn marker(context: &Context) -> Element {
    Element::new("defs").child(
        Element::new("marker")
            .with("id", context.marker_id.as_str())
            .with("viewBox", "0 0 10 10")
            .with("refX", "0")
            .with("refY", "5")
            .with("markerWidth", "6")
            .with("markerHeight", "6")
            .with("orient", "auto")
            .child(
                Element::new("path")
                    .with("d", "M 10 0 L 0 5 L 10 10 Z")
                    .with("class", "ishikawa-arrow")
                    .with("fill", context.line.as_str()),
            ),
    )
}

/// The lengths of the upper and the lower side: a share of 500 by each side's descendants, at
/// least 75, and at least the most descendants under one cause times the line spacing
/// (`ishikawaRenderer.ts` 96-119, 152-165). The upper side's length is the spine's, the lower
/// side's is returned with it.
fn side_lengths(context: &Context, causes: &[IshikawaNode]) -> (f64, f64) {
    let upper: Vec<&IshikawaNode> = causes.iter().step_by(2).collect();
    let lower: Vec<&IshikawaNode> = causes.iter().skip(1).step_by(2).collect();
    let (upper_total, upper_max) = side_stats(&upper);
    let (lower_total, lower_max) = side_stats(&lower);
    let total = upper_total + lower_total;

    let mut upper_len = SPINE_BASE_LENGTH;
    let mut lower_len = SPINE_BASE_LENGTH;
    if total > 0 {
        let pool = SPINE_BASE_LENGTH * 2.0;
        let minimum = SPINE_BASE_LENGTH * 0.3;
        upper_len = minimum.max(pool * (upper_total as f64 / total as f64));
        lower_len = minimum.max(pool * (lower_total as f64 / total as f64));
    }
    let spacing = context.font_size * 2.0;
    upper_len = upper_len.max(upper_max as f64 * spacing);
    lower_len = lower_len.max(lower_max as f64 * spacing);
    (upper_len, lower_len)
}

/// The number of descendants of a side, in total and under its largest cause (`sideStats`).
fn side_stats(causes: &[&IshikawaNode]) -> (usize, usize) {
    causes
        .iter()
        .map(|cause| descendants(cause))
        .fold((0, 0), |(total, max), count| {
            (total + count, max.max(count))
        })
}

/// The number of descendants of a node, at any depth.
fn descendants(node: &IshikawaNode) -> usize {
    node.children
        .iter()
        .map(|child| 1 + descendants(child))
        .sum()
}

/// The spine: from `(spine_x, spine_y)` to `(0, spine_y)` (`ishikawaRenderer.ts` 80-86, 145).
fn spine_line(context: &Context, drawing: &mut Drawing, spine_x: f64, spine_y: f64) -> Element {
    drawing.include(BBox::new(spine_x.min(0.0), spine_y, spine_x.abs(), 0.0));
    Element::new("line")
        .with("class", "ishikawa-spine")
        .with("x1", number(spine_x))
        .with("y1", number(spine_y))
        .with("x2", number(0.0))
        .with("y2", number(spine_y))
        .with("stroke", context.line.as_str())
}

/// The head: a shape of the width and height of its label, with the label centred in it, at
/// `(0, spine_y)` (`ishikawaRenderer.ts` 176-210).
fn head(
    context: &Context,
    drawing: &mut Drawing,
    root: &IshikawaNode,
    spine_y: f64,
) -> Result<Element, RenderError> {
    // The label wraps at `max(6, floor(110 / (fontSize * 0.6)))` characters (`:173`).
    let max_chars = (110.0 / (context.font_size * 0.6)).floor().max(6.0);
    let head_text = wrap_text(&root.text, max_chars);
    let head_style = Style {
        class: "ishikawa-head-label",
        anchor_attribute: "start",
        anchor: Anchor::Middle,
        baseline: Baseline::Middle,
        face: context.bold,
        size: HEAD_LABEL_SIZE,
        line_height: context.line_height(),
    };
    let head = Label::new(head_style, &head_text, 0.0, 0.0)?;
    let text = head.bbox;
    let width = (text.width + 6.0).max(60.0);
    let height = (text.height * 2.0 + 40.0).max(40.0);

    // The shape is a straight edge and a quadratic curve whose control point is 2.4 times the width
    // out. Its tight bounds reach 1.2 times the width, the curve's extreme, not the control point
    // (`NOTES.md` 6.3).
    let path = format!(
        "M 0 {} L 0 {} Q {} 0 0 {} Z",
        number(-height / 2.0),
        number(height / 2.0),
        number(width * 2.4),
        number(-height / 2.0)
    );
    let group_y = spine_y;
    drawing.include(BBox::new(0.0, group_y - height / 2.0, width * 1.2, height));

    // The label is centred in the shape, `+ 3` to the right (`:190`), and its box is then centred
    // on the origin of the group.
    let translate_x = (width - text.width) / 2.0 - text.x + 3.0;
    let translate_y = -text.y - text.height / 2.0;
    let centred = BBox::new(
        text.x + translate_x,
        text.y + translate_y,
        text.width,
        text.height,
    );
    drawing.include(centred.translate(0.0, group_y));
    drawing.runs.extend(head.runs());

    let mut group = Element::new("g")
        .with("class", "ishikawa-head-group")
        .with("transform", format!("translate(0,{})", number(group_y)));
    group.push_child(
        Element::new("path")
            .with("class", "ishikawa-head")
            .with("d", path)
            .with("fill", context.main.as_str())
            .with("stroke", context.line.as_str()),
    );
    group.push_child(head.element(&context.text).with(
        "transform",
        format!("translate({},{})", number(translate_x), number(translate_y)),
    ));
    Ok(group)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_side_lengths_follow_the_descendants_of_each_side() {
        // Each side's share of 500 is at least 75; the spine is at least 250 (`ishikawaRenderer.ts`
        // 106-119). With one descendant on each side the lengths are 250.
        let context = test_context();
        let causes = vec![
            node("Process", &["Out of focus"]),
            node("User", &["Shaky hands"]),
        ];
        assert_eq!(side_lengths(&context, &causes), (250.0, 250.0));
        let heavy = vec![
            node("A", &["1", "2", "3", "4", "5", "6"]),
            node("B", &["7"]),
        ];
        // Six of the seven descendants are upper: 500 * 6 / 7, and the lower side's 500 / 7 is raised
        // to the minimum of 75.
        let (upper, lower) = side_lengths(&context, &heavy);
        assert!((upper - 500.0 * 6.0 / 7.0).abs() < 1e-9, "{upper}");
        assert_eq!(lower, 75.0);
    }

    #[test]
    // The literals are the values V8 prints, which carry 17 significant digits on purpose.
    #[allow(clippy::excessive_precision)]
    fn the_angle_is_eighty_two_degrees_as_v8_computes_it() {
        assert_eq!(cos(ANGLE), 0.13917310096006569);
        assert_eq!(sin(ANGLE), 0.99026806874157025);
    }

    #[test]
    fn descendants_count_every_depth() {
        let tree = node("a", &["b", "c"]);
        assert_eq!(descendants(&tree), 2);
        let deep = IshikawaNode {
            text: "x".to_owned(),
            children: vec![node("y", &["z"])],
        };
        assert_eq!(descendants(&deep), 2);
    }

    fn node(text: &str, children: &[&str]) -> IshikawaNode {
        IshikawaNode {
            text: text.to_owned(),
            children: children
                .iter()
                .map(|child| IshikawaNode {
                    text: (*child).to_owned(),
                    children: Vec::new(),
                })
                .collect(),
        }
    }

    fn test_context() -> Context {
        let regular = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
        Context {
            cos_a: cos(ANGLE),
            sin_a: sin(ANGLE),
            font_size: 16.0,
            label_size: 14.0,
            regular,
            bold: regular,
            line: "#333".to_owned(),
            main: "#fff".to_owned(),
            text: "#333".to_owned(),
            marker_id: "ishikawa-arrow-test".to_owned(),
        }
    }
}
