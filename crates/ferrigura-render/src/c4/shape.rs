//! The shapes of the C4 elements, as the reference draws them through its unified shapes:
//! `person.ts`, `drawRect.ts` (the rounded rectangle), `cylinder.ts` (the database) and
//! `tiltedCylinder.ts` (the queue), with the stacked label of `c4LabelHelper.ts`. `c4ShapeAdapter.ts`
//! chooses the shape, the colours and the stereotype. The sizes are the reference's formulas. The label
//! is centred on its width and height: its sections are centred on the widest one and stack from zero,
//! so `getBBox` of the label group is its origin and its size, and the formulas take that box.

use std::f64::consts::PI;

use ferrigura_jsmath::{asin, cos, sin};
use ferrigura_model::c4::{C4Shape, C4Text};
use ferrigura_text::FontWeight;

use super::config::Conf;
use super::label::{self, SectionFont, Stacked};
use crate::RenderError;
use crate::svg::number;
use crate::unified::geom::Point;
use crate::unified::shapes::Intersect;

/// The corner radius that the rounded shapes write in their style, `rx:12px` (`c4ShapeAdapter.ts:180`).
const ROUNDED_CSS_RADIUS: &str = "12px";

/// The minimum width of a database and of a queue, once the label padding is taken off
/// (`cylinder.ts:54`, `tiltedCylinder.ts:55`).
const CYLINDER_MIN_WIDTH: f64 = 8.0;
const QUEUE_MIN_WIDTH: f64 = 10.0;

/// The narrowest wrapping width of a label (`c4LabelHelper.ts:10`).
const MIN_WRAP_WIDTH: f64 = 32.0;

/// The shape an element is drawn with: `resolveNodeShape` (`c4ShapeAdapter.ts:69-93`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A rounded rectangle (`rounded`).
    Rounded,
    /// A person: a head on a body (`person`).
    Person,
    /// A database (`cylinder`).
    Cylinder,
    /// A queue (`h-cyl`).
    TiltedCylinder,
    /// A framed rectangle (`fr-rect`, the `component` keyword).
    FramedRectangle,
}

/// The shape of a keyword, as `SHAPE_KEYWORDS` maps it (`c4ShapeAdapter.ts:50-60`).
fn keyword(value: &str) -> Option<Kind> {
    match value.to_lowercase().as_str() {
        "person" => Some(Kind::Person),
        "box" | "rounded" => Some(Kind::Rounded),
        "cylinder" | "database" | "db" => Some(Kind::Cylinder),
        "queue" | "pipe" => Some(Kind::TiltedCylinder),
        "component" => Some(Kind::FramedRectangle),
        _ => None,
    }
}

/// The shape of an element (`resolveNodeShape`): `$shape`, else `$sprite`, then the comma-separated
/// `$tags` (the first keyword wins), then the element's type.
fn resolve(shape: &C4Shape) -> Kind {
    let explicit = shape
        .shape
        .as_deref()
        .and_then(keyword)
        .or_else(|| shape.sprite.as_deref().and_then(keyword));
    if let Some(kind) = explicit {
        return kind;
    }
    if let Some(tags) = &shape.tags {
        for tag in tags.split(',') {
            if let Some(kind) = keyword(tag.trim()) {
                return kind;
            }
        }
    }
    match shape.type_c4_shape.as_str() {
        "person" | "external_person" => Kind::Person,
        "system_db"
        | "external_system_db"
        | "container_db"
        | "external_container_db"
        | "component_db"
        | "external_component_db" => Kind::Cylinder,
        "system_queue"
        | "external_system_queue"
        | "container_queue"
        | "external_container_queue"
        | "component_queue"
        | "external_component_queue" => Kind::TiltedCylinder,
        _ => Kind::Rounded,
    }
}

/// The text of a boundary's or a relation's label: the DB's text, or nothing. A named argument in the
/// label slot is an object in the reference, whose text the boundary and the relation measure with
/// `split`: the reference fails to render it (probed), so it is an error here too.
///
/// # Errors
///
/// Returns an error for a named label.
pub fn plain(text: &Option<C4Text>) -> Result<String, RenderError> {
    match text {
        None => Ok(String::new()),
        Some(C4Text::Plain(text)) => Ok(text.clone()),
        Some(C4Text::Named(_)) => Err(RenderError::Unsupported(
            "the reference fails to render a named label of a boundary or a relation".to_owned(),
        )),
    }
}

/// The body of a built element, in its own coordinates (the origin is the centre of the element).
#[derive(Debug, Clone, PartialEq)]
pub enum Body {
    /// The rounded rectangle, from its top-left corner.
    Rounded {
        /// The corner radius of the rectangle (the theme's `radius`).
        radius: f64,
        /// The left edge.
        x: f64,
        /// The top edge.
        y: f64,
        /// The width.
        width: f64,
        /// The height.
        height: f64,
    },
    /// The person: a body rectangle, and the head circle above it.
    Person {
        /// The width of the body.
        width: f64,
        /// The top of the body.
        body_top: f64,
        /// The height of the body.
        body_height: f64,
        /// The corner radius of the body.
        body_radius: f64,
        /// The centre of the head, from the origin.
        head_centre_y: f64,
        /// The radius of the head.
        head_radius: f64,
    },
    /// The framed rectangle: its outline as a polygon, and the `transform` that places it.
    Polygon {
        /// The points, as the `points` attribute writes them.
        points: String,
        /// The `transform` of the polygon.
        transform: String,
    },
    /// A database or a queue: its path and the `transform` that places it.
    Path {
        /// The path's `d`.
        data: String,
        /// The `transform` of the path, as the reference writes it.
        transform: String,
    },
}

/// An element after its measurement: its box, its label and its body. `width` and `height` are the
/// node's size after the shape has drawn it, which the grid places (`node.width`, `node.height`).
#[derive(Debug, Clone, PartialEq)]
pub struct Built {
    /// The alias of the element.
    pub alias: String,
    /// The classes of the element's group: `c4-shape c4-<type>`, and `c4-external` for an external one.
    pub classes: String,
    /// The node's width.
    pub width: f64,
    /// The node's height.
    pub height: f64,
    /// The label.
    pub label: Stacked,
    /// The translation of the label group.
    pub label_translate: (f64, f64),
    /// The colour of the text (`fontColor`, white by default).
    pub text_fill: String,
    /// The style of the body: its fill and stroke as `!important` declarations.
    pub body_style: String,
    /// The body.
    pub body: Body,
    /// How relations meet the element.
    pub intersect: Intersect,
}

/// The `!important` declarations of a body (`styles2String`, `handDrawnShapeStyles.ts:75-87`): the fill
/// and the stroke, then the corner radius for the rounded shapes.
fn body_style(fill: Option<&str>, stroke: Option<&str>, rounded: bool) -> String {
    let mut parts = Vec::new();
    if let Some(fill) = fill {
        parts.push(format!("fill:{fill} !important"));
    }
    if let Some(stroke) = stroke {
        parts.push(format!("stroke:{stroke} !important"));
    }
    if rounded {
        parts.push(format!("rx:{ROUNDED_CSS_RADIUS} !important"));
        parts.push(format!("ry:{ROUNDED_CSS_RADIUS} !important"));
    }
    parts.join(";")
}

/// The wrapping width of an element's label: the node's width less its padding on both sides, and no
/// less than 32 px (`c4LabelHelper.ts:46-48`). `c4.wrap` off gives no wrap (line 53).
fn wrap_width(conf: &Conf, node_width: f64) -> f64 {
    if !conf.wrap {
        return f64::INFINITY;
    }
    (node_width - 2.0 * conf.shape_padding).max(MIN_WRAP_WIDTH)
}

/// The inherited weight of a section: bold above 500, as the measurement reads a CSS weight.
fn weight_of(weight: f64) -> FontWeight {
    if weight > 500.0 {
        FontWeight::Bold
    } else {
        FontWeight::Normal
    }
}

/// Measures an element of the diagram: its label and its shape, and the size the grid places. The
/// first pass of `drawC4ShapeArray` does this (`c4Renderer.ts:311-321`).
///
/// `radius` is the theme's corner radius for the rounded rectangle (`roundedRect.ts:10`).
///
/// # Errors
///
/// Returns an error for a shape the renderer does not draw yet, a named label, or a font that cannot
/// be read.
pub fn build(conf: &Conf, radius: f64, shape: &C4Shape) -> Result<Built, RenderError> {
    let type_name = shape.type_c4_shape.as_str();
    let kind = resolve(shape);
    let palette = conf.element(type_name).ok_or_else(|| {
        RenderError::Unsupported(format!("the C4 element type {type_name} has no palette"))
    })?;
    let size = palette.font.size;
    let weight = weight_of(palette.font.weight_number());

    // The colours: an explicit one (UpdateElementStyle), else the palette of the type
    // (`elementCssStyles`, `c4ShapeAdapter.ts:142-156`). An empty colour is no colour.
    let fill = Some(shape.bg_color.clone().unwrap_or(palette.bg)).filter(|value| !value.is_empty());
    let stroke = Some(shape.border_color.clone().unwrap_or(palette.border))
        .filter(|value| !value.is_empty());
    let text_fill = shape
        .font_color
        .clone()
        .unwrap_or_else(|| "#FFFFFF".to_owned());

    // The stacked label: the name, the stereotype and the description (`c4LabelHelper.ts`). The name
    // is bold, the stereotype and the description are smaller (`styles.js`).
    let mut sections = Vec::new();
    // A named argument in the label slot is an object in the reference, and `c4LabelHelper` takes the
    // name from its first element, which the object does not have: the name section is left out
    // (`c4LabelHelper.ts:39`, probed).
    let name = match &shape.label {
        Some(C4Text::Plain(text)) => text.clone(),
        _ => String::new(),
    };
    if !name.is_empty() {
        sections.push((
            "c4-name",
            name,
            SectionFont {
                size,
                weight: FontWeight::Bold,
            },
        ));
    }
    let techn = shape.techn.as_deref().unwrap_or("");
    sections.push((
        "c4-type",
        stereotype(type_name, techn),
        SectionFont {
            size: size * 0.75,
            weight,
        },
    ));
    if let Some(descr) = shape.descr.as_deref().filter(|descr| !descr.is_empty()) {
        sections.push((
            "c4-descr",
            descr.to_owned(),
            SectionFont {
                size: size * 0.82,
                weight,
            },
        ));
    }

    // The node width the label is wrapped at: the cylinder and the queue take their padding off the
    // element width before the label is laid out (`cylinder.ts:66-71`, `tiltedCylinder.ts:73-81`).
    let padding = conf.shape_padding;
    let node_width = match kind {
        Kind::Cylinder => (conf.width - padding).max(CYLINDER_MIN_WIDTH),
        Kind::TiltedCylinder => (conf.width - padding / 2.0).max(QUEUE_MIN_WIDTH),
        _ => conf.width,
    };
    let stacked = label::layout(&sections, wrap_width(conf, node_width))?;
    let (text_width, text_height) = (stacked.width, stacked.height);

    let (width, height, label_translate, body, intersect) = match kind {
        Kind::Rounded => rounded(text_width, text_height, conf, radius),
        Kind::Person => person(text_width, text_height, conf),
        Kind::Cylinder => cylinder(text_width, text_height, conf),
        Kind::TiltedCylinder => tilted(text_width, text_height, conf),
        Kind::FramedRectangle => framed(text_width, text_height, conf),
    };

    let mut classes = format!("c4-shape c4-{type_name}");
    if type_name.starts_with("external_") {
        classes.push_str(" c4-external");
    }
    Ok(Built {
        alias: shape.alias.clone(),
        classes,
        width,
        height,
        label: stacked,
        label_translate,
        text_fill,
        body_style: body_style(fill.as_deref(), stroke.as_deref(), kind == Kind::Rounded),
        body,
        intersect,
    })
}

/// The stereotype line: `[Person]`, `[Software System]`, `[Container: Tech]` (`stereotypeLabel` and
/// `stereotypeText`, `c4ShapeAdapter.ts:103-114`). The name drops the `external_` and `_db`/`_queue`
/// suffixes; the technology is written only when it is not empty.
fn stereotype(type_name: &str, techn: &str) -> String {
    let base = type_name.strip_prefix("external_").unwrap_or(type_name);
    let base = base
        .strip_suffix("_db")
        .or_else(|| base.strip_suffix("_queue"))
        .unwrap_or(base);
    let name = match base {
        "person" => "Person".to_owned(),
        "system" => "Software System".to_owned(),
        "container" => "Container".to_owned(),
        "component" => "Component".to_owned(),
        other => other.replace('_', " "),
    };
    if techn.is_empty() {
        format!("[{name}]")
    } else {
        format!("[{name}: {techn}]")
    }
}

/// The rounded rectangle (`drawRect.ts:21-22`, `roundedRect.ts`): at least the element width, and the
/// label's box plus the padding each side. The label is centred on the origin by the label helper.
fn rounded(
    text_width: f64,
    text_height: f64,
    conf: &Conf,
    radius: f64,
) -> (f64, f64, (f64, f64), Body, Intersect) {
    let padding = conf.shape_padding;
    let width = (text_width + 2.0 * padding).max(conf.width);
    let height = text_height + 2.0 * padding;
    let body = Body::Rounded {
        radius,
        x: -width / 2.0,
        y: -height / 2.0,
        width,
        height,
    };
    (
        width,
        height,
        (-text_width / 2.0, -text_height / 2.0),
        body,
        Intersect::Rect,
    )
}

/// The person (`person.ts:13-132`): the body is at least 100 px wide, the head's radius is a fraction of
/// the width clamped to 16 to 56 px, and the label is centred in the body. The outline of the head and
/// the body is what the edges meet.
fn person(
    text_width: f64,
    text_height: f64,
    conf: &Conf,
) -> (f64, f64, (f64, f64), Body, Intersect) {
    let padding = conf.shape_padding;
    let width = (text_width + padding * 2.0).max(conf.width).max(100.0);
    let head_radius = (width * 0.23).clamp(16.0, 56.0);
    let overlap = head_radius * 0.27;
    let body_height = text_height + padding * 2.0;
    let body_radius = (width * 0.177).min(body_height * 0.45);
    let total_height = body_height + 2.0 * head_radius - overlap;
    let top = -total_height / 2.0;
    let body_top = top + 2.0 * head_radius - overlap;
    let body = Body::Person {
        width,
        body_top,
        body_height,
        body_radius,
        head_centre_y: top + head_radius,
        head_radius,
    };

    // The outline the edges meet: the head's exposed arc, then the body's rounded corners (`person.ts:83-125`).
    let head_centre_y = top + head_radius;
    let phi_right = (asin(((body_top - head_centre_y) / head_radius).min(1.0)) * 180.0) / PI;
    let mut outline = circle_points(
        0.0,
        -head_centre_y,
        head_radius,
        24,
        180.0 + phi_right,
        -phi_right,
    );
    outline.extend(circle_points(
        -(-width / 2.0 + body_radius),
        -(body_top + body_radius),
        body_radius,
        12,
        90.0,
        0.0,
    ));
    outline.extend(circle_points(
        -(-width / 2.0 + body_radius),
        -(total_height / 2.0 - body_radius),
        body_radius,
        12,
        360.0,
        270.0,
    ));
    outline.extend(circle_points(
        -(width / 2.0 - body_radius),
        -(total_height / 2.0 - body_radius),
        body_radius,
        12,
        270.0,
        180.0,
    ));
    outline.extend(circle_points(
        -(width / 2.0 - body_radius),
        -(body_top + body_radius),
        body_radius,
        12,
        180.0,
        90.0,
    ));

    let body_centre_y = body_top + body_height / 2.0;
    (
        width,
        total_height,
        (-text_width / 2.0, body_centre_y - text_height / 2.0),
        body,
        Intersect::Polygon { points: outline },
    )
}

/// The width of the frame on each side of a framed rectangle (`subroutine.ts:11`).
const FRAME_WIDTH: f64 = 8.0;

/// The framed rectangle (`subroutine.ts:13-72`): the label's box with the padding, and a frame of
/// [`FRAME_WIDTH`] on each side, drawn as one ten-point polygon whose centre is the element's.
fn framed(
    text_width: f64,
    text_height: f64,
    conf: &Conf,
) -> (f64, f64, (f64, f64), Body, Intersect) {
    let padding = conf.shape_padding;
    let total_width = (text_width + 2.0 * FRAME_WIDTH + padding).max(conf.width);
    let total_height = text_height + padding;
    let w = total_width - 2.0 * FRAME_WIDTH;
    let h = total_height;
    let points = vec![
        Point::new(0.0, 0.0),
        Point::new(w, 0.0),
        Point::new(w, -h),
        Point::new(0.0, -h),
        Point::new(0.0, 0.0),
        Point::new(-8.0, 0.0),
        Point::new(w + 8.0, 0.0),
        Point::new(w + 8.0, -h),
        Point::new(-8.0, -h),
        Point::new(-8.0, 0.0),
    ];
    let attribute = points
        .iter()
        .map(|point| format!("{},{}", number(point.x), number(point.y)))
        .collect::<Vec<_>>()
        .join(" ");
    let body = Body::Polygon {
        points: attribute,
        transform: format!("translate({},{})", number(-w / 2.0), number(h / 2.0)),
    };
    (
        total_width,
        h,
        (-text_width / 2.0, -text_height / 2.0),
        body,
        Intersect::Polygon { points },
    )
}

/// The database (`cylinder.ts:57-154`): the label is raised by the padding over 1.5, and the node is the
/// path's box, which is `h + 2 ry` high.
fn cylinder(
    text_width: f64,
    text_height: f64,
    conf: &Conf,
) -> (f64, f64, (f64, f64), Body, Intersect) {
    let padding = conf.shape_padding;
    let node_width = (conf.width - padding).max(CYLINDER_MIN_WIDTH);
    let width = node_width.max(text_width) + padding;
    let rx = width / 2.0;
    let ry = rx / (2.5 + width / 50.0);
    let height = text_height + padding + ry;
    let body = Body::Path {
        data: cylinder_path(0.0, 0.0, width, height, rx, ry),
        transform: format!(
            "translate({}, {})",
            number(-width / 2.0),
            number(-(height / 2.0 + ry))
        ),
    };
    (
        width,
        height + 2.0 * ry,
        (-text_width / 2.0, -text_height / 2.0 + padding / 1.5),
        body,
        Intersect::Cylinder { rx, ry },
    )
}

/// The queue (`tiltedCylinder.ts:57-160`): the cap's arcs reach `rx` past both sides of the body, so the
/// node is `w + 2 rx` wide.
fn tilted(
    text_width: f64,
    text_height: f64,
    conf: &Conf,
) -> (f64, f64, (f64, f64), Body, Intersect) {
    let label_padding = conf.shape_padding / 2.0;
    let node_width = (conf.width - label_padding).max(QUEUE_MIN_WIDTH);
    let height = text_height + label_padding;
    let ry = height / 2.0;
    let rx = ry / (2.5 + height / 50.0);
    let width = node_width.max(text_width) + rx + label_padding;
    let body = Body::Path {
        data: tilted_path(0.0, 0.0, width, height, rx, ry),
        transform: format!(
            "translate({}, {} )",
            number(-width / 2.0),
            number(height / 2.0)
        ),
    };
    (
        width + 2.0 * rx,
        height,
        (-text_width / 2.0 - rx, -text_height / 2.0),
        body,
        Intersect::TiltedCylinder { rx, ry },
    )
}

/// `createCylinderPathD` (`cylinder.ts:9-25`): the database's path, one line.
fn cylinder_path(x: f64, y: f64, width: f64, height: f64, rx: f64, ry: f64) -> String {
    let (arc_x, arc_y) = (number(rx), number(ry));
    let (w, negative_w, h, negative_h) = (
        number(width),
        number(-width),
        number(height),
        number(-height),
    );
    [
        format!("M{},{}", number(x), number(y + ry)),
        format!("a{arc_x},{arc_y} 0,0,0 {w},0"),
        format!("a{arc_x},{arc_y} 0,0,0 {negative_w},0"),
        format!("l0,{h}"),
        format!("a{arc_x},{arc_y} 0,0,0 {w},0"),
        format!("l0,{negative_h}"),
    ]
    .join(" ")
}

/// `createCylinderPathD` of `tiltedCylinder.ts:9-24`: the queue's path, over several lines.
fn tilted_path(x: f64, y: f64, width: f64, height: f64, rx: f64, ry: f64) -> String {
    let (rx, ry) = (number(rx), number(ry));
    format!(
        "M{x},{y}\n    a{rx},{ry} 0,0,1 0,{neg_h}\n    l{w},0\n    a{rx},{ry} 0,0,1 0,{h}\n    M{w},{neg_h}\n    a{rx},{ry} 0,0,0 0,{h}\n    l{neg_w},0",
        x = number(x),
        y = number(y),
        w = number(width),
        neg_w = number(-width),
        h = number(height),
        neg_h = number(-height),
    )
}

/// `generateCirclePoints` (`shapes/util.ts:295-323`): `count` points on the arc from `start` to `end`
/// degrees, with both coordinates negated as the reference negates them.
fn circle_points(
    centre_x: f64,
    centre_y: f64,
    radius: f64,
    count: usize,
    start_degrees: f64,
    end_degrees: f64,
) -> Vec<Point> {
    let start = (start_degrees * PI) / 180.0;
    let end = (end_degrees * PI) / 180.0;
    let step = (end - start) / (count as f64 - 1.0);
    (0..count)
        .map(|index| {
            let angle = start + index as f64 * step;
            let x = centre_x + radius * cos(angle);
            let y = centre_y + radius * sin(angle);
            Point::new(-x, -y)
        })
        .collect()
}
