//! The SVG of an architecture diagram: the edges, the services and junctions, and the groups, as
//! `svgDraw.ts` writes them (`drawEdges` 25-191, `drawGroups` 193-274, `drawServices` 276-362,
//! `drawJunctions` 364-389) with the transforms of `positionNodes` (`architectureRenderer.ts` 82-94).
//!
//! Each drawn element also contributes its box in the root's space. The root's `getBBox` is the union
//! of those boxes, and it sets the viewBox. A box with no area is left out of a union, as `getBBox`
//! does. A text's box is the one `unified::label` gives, moved by the `dominant-baseline` the element
//! computes: `middle` for the service titles and the X and Y edge labels, and the alphabetic baseline
//! for the group labels (`start` is not a `dominant-baseline` value) and the XY edge labels.

use std::fmt::Write as _;

use ferrigura_model::architecture::Direction;
use ferrigura_model::flowchart::LabelType;
use ferrigura_syntax::sanitize::sanitize_text;
use ferrigura_text::{Face, FontFamily, FontStyle, FontWeight, face, x_height};

use super::geometry::{Affine, Box2};
use super::icons;
use super::scene::{Kind, Metrics, Scene};
use crate::RenderError;
use crate::svg::{escape, number};
use crate::unified::label::{self, Anchor, LabelLayout, LabelSpec};

/// The size of an arrow: a sixth of an icon (`svgDraw.ts` 34).
const ARROW_DIVISOR: f64 = 6.0;

/// The width of a service title: `iconSize * 1.5` (`svgDraw.ts` 294).
const TITLE_WIDTH_FACTOR: f64 = 1.5;

/// The extra distance that a `{group}` edge clears below a service label (`svgDraw.ts` 56).
const SERVICE_LABEL_HEIGHT: f64 = 18.0;

/// The prefix of the icon pack the architecture registers (`architectureIcons.prefix`).
const ICON_PREFIX: &str = "mermaid-architecture";

/// The drawing: the three groups of the root, the box of everything in them, and what the fonts
/// need.
#[derive(Debug, Clone)]
pub struct Drawing {
    /// `g.architecture-edges`, with its edges.
    pub edges: String,
    /// `g.architecture-services`: the services, then the junctions.
    pub services: String,
    /// `g.architecture-groups`.
    pub groups: String,
    /// The union of the drawn boxes, in the root's space (`getBBox` of the root).
    pub bounds: Box2,
    /// Every label laid out, for the fonts.
    pub labels: Vec<LabelLayout>,
}

/// The `dominant-baseline` an element computes for its text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Baseline {
    /// `middle`: the middle of the x-height sits on the line's `y`.
    Middle,
    /// The alphabetic baseline: `auto`, which `start` falls back to in Chromium.
    Alphabetic,
}

impl Baseline {
    fn keyword(self) -> &'static str {
        match self {
            Self::Middle => "middle",
            Self::Alphabetic => "auto",
        }
    }
}

/// A label as drawn.
struct Text {
    /// The layout, for the fonts.
    layout: LabelLayout,
    /// The markup of the label's `<g>` (the background and the `<text>`).
    markup: String,
    /// The box of the text in its own coordinates, with the baseline of the element.
    bounds: Box2,
}

/// The regular sans face of the labels.
fn sans() -> &'static Face {
    face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal)
}

/// Lays out a label as `createText` does with `useHtmlLabels: false`: a Markdown label, wrapped at
/// `width` and anchored on `x = 0`, and the box that its element's baseline gives it.
fn text(
    label_text: &str,
    width: f64,
    anchor: Anchor,
    font_size: f64,
    baseline: Baseline,
) -> Result<Text, RenderError> {
    let layout = label::layout(&LabelSpec {
        text: label_text,
        label_type: LabelType::Markdown,
        width,
        is_node: true,
        anchor,
        wrap_in_style: false,
        add_background: false,
        styles: &[],
        font_size,
        title: false,
    })?;
    let raw = layout.bbox;
    let shift = match baseline {
        Baseline::Middle => {
            let half = x_height(sans(), font_size)? / 2.0;
            (half * 64.0).round() / 64.0
        }
        Baseline::Alphabetic => 0.0,
    };
    let bounds = if raw.is_empty() {
        Box2::EMPTY
    } else {
        Box2::new(
            raw.x,
            raw.y + shift,
            raw.x + raw.width,
            raw.y + shift + raw.height,
        )
    };
    let markup = layout.svg_with("", "");
    Ok(Text {
        layout,
        markup,
        bounds,
    })
}

/// A `<g>` with its attributes, in the order given. An empty body is written as `<g></g>`.
fn group(attributes: &[(&str, String)], inner: &str) -> String {
    let mut out = String::from("<g");
    for (name, value) in attributes {
        let _ = write!(out, " {name}=\"{}\"", escape(value));
    }
    out.push('>');
    out.push_str(inner);
    out.push_str("</g>");
    out
}

/// The `<g>` a label is drawn in: the baseline and the anchor attributes that the reference sets on
/// it, the transform, and the label's own markup.
fn text_group(text: &Text, baseline: Baseline, anchor: &str, transform: Option<String>) -> String {
    let mut attributes = vec![
        ("dy", "1em".to_owned()),
        ("alignment-baseline", baseline.keyword().to_owned()),
        ("dominant-baseline", baseline.keyword().to_owned()),
        ("text-anchor", anchor.to_owned()),
    ];
    if let Some(transform) = transform {
        attributes.push(("transform", transform));
    }
    group(&attributes, &text.markup)
}

/// `(data.x || 0)` of `positionNodes`: NaN is written as 0.
fn or_zero(value: f64) -> f64 {
    if value.is_nan() { 0.0 } else { value }
}

/// `ArchitectureDirectionArrow`: the points of an arrow of `scale`, for the side it sits on.
fn arrow_points(dir: Direction, scale: f64) -> [(f64, f64); 3] {
    let half = scale / 2.0;
    match dir {
        Direction::L => [(scale, half), (0.0, scale), (0.0, 0.0)],
        Direction::R => [(0.0, half), (scale, 0.0), (scale, scale)],
        Direction::T => [(0.0, 0.0), (scale, 0.0), (half, scale)],
        Direction::B => [(half, 0.0), (scale, scale), (0.0, scale)],
    }
}

/// The `points` attribute of [`arrow_points`], in the reference's number format.
fn arrow_points_attribute(dir: Direction, scale: f64) -> String {
    arrow_points(dir, scale)
        .iter()
        .map(|(x, y)| format!("{},{}", number(*x), number(*y)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// `ArchitectureDirectionArrowShift`: where an arrow sits along its side, from the end's coordinate.
fn arrow_shift(dir: Direction, orig: f64, arrow: f64) -> f64 {
    match dir {
        Direction::L | Direction::T => orig - arrow + 2.0,
        Direction::R | Direction::B => orig - 2.0,
    }
}

/// The box of a set of points (`getBBox` of a path or a polygon).
fn points_box(points: &[(f64, f64)]) -> Box2 {
    let mut x1 = f64::INFINITY;
    let mut y1 = f64::INFINITY;
    let mut x2 = f64::NEG_INFINITY;
    let mut y2 = f64::NEG_INFINITY;
    for &(x, y) in points {
        if x.is_nan() || y.is_nan() {
            return Box2::EMPTY;
        }
        x1 = x1.min(x);
        y1 = y1.min(y);
        x2 = x2.max(x);
        y2 = y2.max(y);
    }
    Box2::new(x1, y1, x2, y2)
}

/// `getArchitectureDirectionXYFactors` of a pair of sides.
fn xy_factors(dirs: (Direction, Direction)) -> (f64, f64) {
    use Direction::{B, L, R, T};
    match dirs {
        (L, T) | (T, L) => (1.0, 1.0),
        (B, L) | (L, B) => (1.0, -1.0),
        (B, R) | (R, B) => (-1.0, -1.0),
        _ => (-1.0, 1.0),
    }
}

/// The transform of an XY edge's label (`svgDraw.ts` 161-186). The text is turned by the angle of its
/// pair, then moved so that its corner sits on the middle of the edge. `bounds` is the text's box
/// after the turn's base (the alphabetic box), and the size of the turned box is measured from it.
fn xy_label_transform(
    middle: (f64, f64),
    dirs: (Direction, Direction),
    bounds: Box2,
) -> (String, Affine) {
    let (x, y) = xy_factors(dirs);
    let angle = -x * y * 45.0;
    let height = bounds.h();
    let turned = Affine::rotate(angle);
    let corners = [
        turned.apply(bounds.x1, bounds.y1),
        turned.apply(bounds.x2, bounds.y1),
        turned.apply(bounds.x1, bounds.y2),
        turned.apply(bounds.x2, bounds.y2),
    ];
    let mut x1 = f64::INFINITY;
    let mut y1 = f64::INFINITY;
    let mut x2 = f64::NEG_INFINITY;
    let mut y2 = f64::NEG_INFINITY;
    for (px, py) in corners {
        x1 = x1.min(px);
        y1 = y1.min(py);
        x2 = x2.max(px);
        y2 = y2.max(py);
    }
    let width_new = x2 - x1;
    let height_new = y2 - y1;
    let half = height / 2.0;
    let attribute = format!(
        "translate({}, {}) translate({}, {}) rotate({}, 0, {})",
        number(middle.0),
        number(middle.1 - half),
        number(x * width_new / 2.0),
        number(y * height_new / 2.0),
        number(angle),
        number(half),
    );
    // The same transform as a matrix, applied to a point: the turn about (0, half) first, then the
    // two translations.
    let matrix = Affine::rotate_about(angle, 0.0, half)
        .then(&Affine::translate(
            x * width_new / 2.0,
            y * height_new / 2.0,
        ))
        .then(&Affine::translate(middle.0, middle.1 - half));
    (attribute, matrix)
}

/// The end of an edge at a `{group}` modifier (`svgDraw.ts` 54-71): the end moves out of its group by
/// `shift` on its own axis, and a bottom end also clears the service label.
fn group_end(dir: Direction, point: (f64, f64), shift: f64) -> (f64, f64) {
    let (x, y) = point;
    if dir.is_horizontal() {
        (x + if dir == Direction::L { -shift } else { shift }, y)
    } else {
        (
            x,
            y + if dir == Direction::T {
                -shift
            } else {
                shift + SERVICE_LABEL_HEIGHT
            },
        )
    }
}

/// The end of an edge at a junction without `{group}` (`svgDraw.ts` 73-87): half an icon towards the
/// junction's centre.
fn junction_end(dir: Direction, point: (f64, f64), half: f64) -> (f64, f64) {
    let (x, y) = point;
    if dir.is_horizontal() {
        (x + if dir == Direction::L { half } else { -half }, y)
    } else {
        (x, y + if dir == Direction::T { half } else { -half })
    }
}

/// The box of an icon at `size` pixels, in its own coordinates: its square. The unknown icon's question
/// mark lies inside the square in the metrics of its font (Arial: from 3.4 to 79.0 units vertically, and
/// from 21.2 to 58.8 horizontally), so the square bounds it.
fn icon_bounds(size: f64) -> Box2 {
    Box2::new(0.0, 0.0, size, size)
}

/// The `<svg>` of an icon at `size` pixels, as `getIconSVG` writes it (`svgDraw.ts` 228, 309-317). A
/// name with another prefix, or a name that is not in the pack, draws the unknown icon (`icons.ts`
/// 94-99).
fn icon_markup(name: &str, size: f64) -> String {
    let artwork = resolve_icon(name).unwrap_or(icons::UNKNOWN);
    let body = if artwork == icons::UNKNOWN || artwork == icons::BLANK {
        artwork.to_owned()
    } else {
        format!("{}{artwork}{}", icons::WRAP_OPEN, icons::WRAP_CLOSE)
    };
    let svg = format!(
        "<svg width=\"{}\" height=\"{}\" viewBox=\"0 0 80 80\">{body}</svg>",
        number(size),
        number(size)
    );
    svg
}

/// The artwork of a name of the architecture pack, with or without its prefix (`getRegisteredIconData`).
fn resolve_icon(name: &str) -> Option<&'static str> {
    let short = match name.split_once(':') {
        Some((prefix, rest)) if prefix == ICON_PREFIX => rest,
        Some(_) => return None,
        None => name,
    };
    match short {
        "database" => Some(icons::DATABASE),
        "server" => Some(icons::SERVER),
        "disk" => Some(icons::DISK),
        "internet" => Some(icons::INTERNET),
        "cloud" => Some(icons::CLOUD),
        "unknown" => Some(icons::UNKNOWN),
        "blank" => Some(icons::BLANK),
        _ => None,
    }
}

/// Draws the diagram. The positions of the nodes are the layout's, and the edges' geometry is the
/// scene's at the time of the call.
///
/// # Errors
///
/// Returns an error if a label cannot be laid out.
pub fn draw(
    scene: &Scene,
    diagram_id: &str,
    metrics: Metrics,
    text_size: f64,
) -> Result<Drawing, RenderError> {
    let icon_size = metrics.icon_size;
    let half_icon = icon_size / 2.0;
    let arrow_size = icon_size / ARROW_DIVISOR;
    let half_arrow = arrow_size / 2.0;
    let group_shift = metrics.padding + 4.0;
    let mut labels: Vec<LabelLayout> = Vec::new();

    // `drawEdges` (svgDraw.ts 25-191).
    let mut edges_markup = String::new();
    let mut edges_bounds = Box2::EMPTY;
    for index in 0..scene.edges.len() {
        let edge = &scene.edges[index];
        let geometry = scene.edge_geometry(index);
        let mut start = geometry.arrow_start;
        let mut end = geometry.arrow_end;
        let (mid_x, mid_y) = geometry.middle;
        if edge.source_group {
            start = group_end(edge.source_dir, start, group_shift);
        }
        if edge.target_group {
            end = group_end(edge.target_dir, end, group_shift);
        }
        if !edge.source_group && scene.nodes[edge.source].kind == Kind::Junction {
            start = junction_end(edge.source_dir, start, half_icon);
        }
        if !edge.target_group && scene.nodes[edge.target].kind == Kind::Junction {
            end = junction_end(edge.target_dir, end, half_icon);
        }
        let (start_x, start_y) = start;
        let (end_x, end_y) = end;

        let source = &scene.nodes[edge.source].id;
        let target = &scene.nodes[edge.target].id;
        let mut children = format!(
            "<path d=\"M {},{} L {},{} L{},{} \" class=\"edge\" id=\"{}\"/>",
            number(start_x),
            number(start_y),
            number(mid_x),
            number(mid_y),
            number(end_x),
            number(end_y),
            escape(&format!("{diagram_id}-L_{source}_{target}_0")),
        );
        edges_bounds.include_box(points_box(&[
            (start_x, start_y),
            (mid_x, mid_y),
            (end_x, end_y),
        ]));

        let ends = [
            (edge.source_arrow, edge.source_dir, start_x, start_y),
            (edge.target_arrow, edge.target_dir, end_x, end_y),
        ];
        for (arrow, dir, x, y) in ends {
            if !arrow {
                continue;
            }
            let x_shift = if dir.is_horizontal() {
                arrow_shift(dir, x, arrow_size)
            } else {
                x - half_arrow
            };
            let y_shift = if dir.is_horizontal() {
                y - half_arrow
            } else {
                arrow_shift(dir, y, arrow_size)
            };
            children.push_str(&format!(
                "<polygon points=\"{}\" transform=\"translate({},{})\" class=\"arrow\"/>",
                arrow_points_attribute(dir, arrow_size),
                number(x_shift),
                number(y_shift),
            ));
            let corners =
                arrow_points(dir, arrow_size).map(|(px, py)| (px + x_shift, py + y_shift));
            edges_bounds.include_box(points_box(&corners));
        }

        if let Some(title) = &edge.label {
            let xy = edge.segments;
            let (width, axis_x, axis_y) = if xy {
                ((start_x - end_x).abs() / 2.0, false, false)
            } else if edge.source_dir.is_horizontal() {
                ((start_x - end_x).abs(), true, false)
            } else {
                ((start_y - end_y).abs() / 1.5, false, true)
            };
            let baseline = if xy {
                Baseline::Alphabetic
            } else {
                Baseline::Middle
            };
            let label = text(title, width, Anchor::Middle, text_size, baseline)?;
            let (transform, matrix) = if axis_x {
                (
                    format!("translate({}, {})", number(mid_x), number(mid_y)),
                    Affine::translate(mid_x, mid_y),
                )
            } else if axis_y {
                (
                    format!(
                        "translate({}, {}) rotate(-90)",
                        number(mid_x),
                        number(mid_y)
                    ),
                    Affine::rotate(-90.0).then(&Affine::translate(mid_x, mid_y)),
                )
            } else {
                xy_label_transform(
                    (mid_x, mid_y),
                    (edge.source_dir, edge.target_dir),
                    label.bounds,
                )
            };
            if label.bounds.has_area() {
                edges_bounds.include_box(label.bounds.transformed(&matrix));
            }
            children.push_str(&text_group(&label, baseline, "middle", Some(transform)));
            labels.push(label.layout);
        }
        edges_markup.push_str(&group(&[], &children));
    }
    let mut root = Box2::EMPTY;
    root.include_box(edges_bounds);
    let edges = group(&[("class", "architecture-edges".to_owned())], &edges_markup);

    // `drawServices` (svgDraw.ts 276-362), then `drawJunctions` (364-389).
    let mut services_markup = String::new();
    let mut services_bounds = Box2::EMPTY;
    for index in 0..scene.nodes.len() {
        let node = &scene.nodes[index];
        if node.kind != Kind::Service {
            continue;
        }
        let (x, y) = (or_zero(node.position.0), or_zero(node.position.1));
        let mut children = String::new();
        let mut content = Box2::EMPTY;
        if let Some(title) = &node.label {
            let label = text(
                title,
                icon_size * TITLE_WIDTH_FACTOR,
                Anchor::Middle,
                text_size,
                Baseline::Middle,
            )?;
            let at = (half_icon, icon_size);
            content.include_box(label.bounds.translate(at.0, at.1));
            children.push_str(&text_group(
                &label,
                Baseline::Middle,
                "middle",
                Some(format!("translate({}, {})", number(at.0), number(at.1))),
            ));
            labels.push(label.layout);
        }
        let mut background = String::new();
        if let Some(name) = &node.icon {
            let markup = icon_markup(name, icon_size);
            background.push_str(&group(&[], &markup));
            content.include_box(icon_bounds(icon_size));
        } else if let Some(icon_text) = &node.icon_text {
            // `iconText` (svgDraw.ts 318-340): the blank icon, and the text in a foreignObject.
            let blank = icon_markup("blank", icon_size);
            let clamp = ((icon_size - 2.0) / text_size).floor();
            let text_markup = format!(
                "<foreignObject width=\"{}\" height=\"{}\"><div class=\"node-icon-text\" style=\"height: {}px;\"><div style=\"-webkit-line-clamp: {};\">{}</div></div></foreignObject>",
                number(icon_size),
                number(icon_size),
                number(icon_size),
                number(clamp),
                sanitize_text(icon_text),
            );
            // The blank icon is the bkg's own `g > svg`, and the text its own `g > foreignObject`.
            background.push_str(&group(&[], &blank));
            background.push_str(&group(&[], &text_markup));
            content.include_box(Box2::new(0.0, 0.0, icon_size, icon_size));
        } else {
            background.push_str(&format!(
                "<path class=\"node-bkg\" id=\"{}\" d=\"M0,{} V5 Q0,0 5,0 H{} Q{},0 {},5 V{} Z\"/>",
                escape(&format!("{diagram_id}-node-{}", node.id)),
                number(icon_size),
                number(icon_size - 5.0),
                number(icon_size),
                number(icon_size),
                number(icon_size),
            ));
            content.include_box(Box2::new(0.0, 0.0, icon_size, icon_size));
        }
        children.push_str(&group(&[], &background));
        services_markup.push_str(&group(
            &[
                ("id", format!("{diagram_id}-service-{}", node.id)),
                ("class", "architecture-service".to_owned()),
                (
                    "transform",
                    format!("translate({},{})", number(x), number(y)),
                ),
            ],
            &children,
        ));
        services_bounds.include_box(content.transformed(&Affine::translate(x, y)));
    }
    for index in 0..scene.nodes.len() {
        let node = &scene.nodes[index];
        if node.kind != Kind::Junction {
            continue;
        }
        let (x, y) = (or_zero(node.position.0), or_zero(node.position.1));
        let rect = format!(
            "<rect id=\"{}\" fill-opacity=\"0\" width=\"{}\" height=\"{}\"/>",
            escape(&format!("{diagram_id}-node-{}", node.id)),
            number(icon_size),
            number(icon_size),
        );
        services_markup.push_str(&group(
            &[
                ("class", "architecture-junction".to_owned()),
                (
                    "transform",
                    format!("translate({},{})", number(x), number(y)),
                ),
            ],
            &group(&[], &rect),
        ));
        services_bounds.include_box(Box2::new(0.0, 0.0, icon_size, icon_size).translate(x, y));
    }
    root.include_box(services_bounds);
    let services = group(
        &[("class", "architecture-services".to_owned())],
        &services_markup,
    );

    // `drawGroups` (svgDraw.ts 193-274): the rectangle of each group, then its icon and its label.
    let mut groups_markup = String::new();
    let mut groups_bounds = Box2::EMPTY;
    let icon_size_group = metrics.padding * 0.75;
    for index in 0..scene.nodes.len() {
        let node = &scene.nodes[index];
        if node.kind != Kind::Group {
            continue;
        }
        let bounds = scene.node_box(index);
        let (x1, y1) = (bounds.x1, bounds.y1);
        let (w, h) = (bounds.w(), bounds.h());
        groups_bounds.include_box(Box2::new(
            x1 + half_icon,
            y1 + half_icon,
            x1 + half_icon + w,
            y1 + half_icon + h,
        ));
        let rect = format!(
            "<rect id=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" class=\"node-bkg\"/>",
            escape(&format!("{diagram_id}-group-{}", node.id)),
            number(x1 + half_icon),
            number(y1 + half_icon),
            number(w),
            number(h),
        );
        let mut container = String::new();
        let mut shifted_x1 = x1;
        let mut shifted_y1 = y1;
        if let Some(name) = &node.icon {
            let markup = icon_markup(name, icon_size_group);
            let (tx, ty) = (shifted_x1 + half_icon + 1.0, shifted_y1 + half_icon + 1.0);
            container.push_str(&group(
                &[(
                    "transform",
                    format!("translate({}, {})", number(tx), number(ty)),
                )],
                &group(&[], &markup),
            ));
            groups_bounds.include_box(icon_bounds(icon_size_group).translate(tx, ty));
            shifted_x1 += icon_size_group;
            // `fontSize / 2 - 1 - 2` (svgDraw.ts 236).
            shifted_y1 += metrics.font_size / 2.0 - 1.0 - 2.0;
        }
        if let Some(title) = &node.label {
            let label = text(title, w, Anchor::Start, text_size, Baseline::Alphabetic)?;
            let (tx, ty) = (shifted_x1 + half_icon + 4.0, shifted_y1 + half_icon + 2.0);
            groups_bounds.include_box(label.bounds.translate(tx, ty));
            container.push_str(&text_group(
                &label,
                Baseline::Alphabetic,
                "start",
                Some(format!("translate({}, {})", number(tx), number(ty))),
            ));
            labels.push(label.layout);
        }
        groups_markup.push_str(&rect);
        groups_markup.push_str(&group(&[], &container));
    }
    root.include_box(groups_bounds);
    let groups = group(
        &[("class", "architecture-groups".to_owned())],
        &groups_markup,
    );

    Ok(Drawing {
        edges,
        services,
        groups,
        bounds: root,
        labels,
    })
}
