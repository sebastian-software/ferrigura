//! The person: a circular head over a rounded body, as the C4 model draws a person. A port of
//! `shapes/person.ts` in the plain elements that both looks draw (the rough version is for the
//! hand-drawn look only). See `docs-port/flowchart/render-notes/shapes-core.md` §2.1 and §2.4.

use ferrigura_jsmath::asin;

use super::util::{self, Part, attribute, label_helper, styles2string};
use super::{Intersect, ShapeInput};
use crate::svg::number;
use crate::unified::geom::{BBox, Point};

/// The narrowest and widest the head is, in pixels: the head's radius is 0.23 of the body's
/// width, clamped to this range.
const HEAD_RADIUS_MIN: f64 = 16.0;
const HEAD_RADIUS_MAX: f64 = 56.0;
/// The points of the head's arc and of each corner of the body in the intersect outline.
const HEAD_SEGMENTS: usize = 24;
const CORNER_SEGMENTS: usize = 12;

/// `person.ts`: the body is a rounded rectangle, the head a circle that overlaps its top by 27 per
/// cent of its radius. The label sits in the body. The width is at least 100.
pub(super) fn person(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let label = label_helper(input, false);
    let padding = node.padding;

    let width = (label.bbox.width + padding * 2.0)
        .max(node.width.unwrap_or(0.0))
        .max(100.0);
    let head_radius = (width * 0.23).clamp(HEAD_RADIUS_MIN, HEAD_RADIUS_MAX);
    let overlap = head_radius * 0.27;
    let body_height = (label.bbox.height + padding * 2.0).max(
        node.height
            .filter(|height| *height != 0.0)
            .map_or(0.0, |height| height - (2.0 * head_radius - overlap)),
    );
    let body_radius = (width * 0.177).min(body_height * 0.45);
    let total_height = body_height + 2.0 * head_radius - overlap;
    let top = -total_height / 2.0;
    let body_top = top + 2.0 * head_radius - overlap;
    let head_centre_y = top + head_radius;

    let style = attribute("style", &styles.node);
    let svg = format!(
        "<g class=\"basic label-container\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" ry=\"{}\"{style}/><circle cx=\"0\" cy=\"{}\" r=\"{}\"{style}/></g>",
        number(-width / 2.0),
        number(body_top),
        number(width),
        number(body_height),
        number(body_radius),
        number(body_radius),
        number(head_centre_y),
        number(head_radius),
    );
    let bbox = BBox::new(-width / 2.0, body_top, width, body_height).union(BBox::new(
        -head_radius,
        head_centre_y - head_radius,
        2.0 * head_radius,
        2.0 * head_radius,
    ));

    // The label is centred in the body.
    let body_centre_y = body_top + body_height / 2.0;
    let label_offset = Point::new(
        -(label.bbox.width / 2.0) - label.bbox.x,
        body_centre_y - label.bbox.height / 2.0 - label.bbox.y,
    );

    // The intersect outline: the head's arc that shows above the body, joined to the body's
    // rounded corners, so edges meet the silhouette and not the box.
    let phi_right_deg =
        asin(((body_top - head_centre_y) / head_radius).min(1.0)) * 180.0 / std::f64::consts::PI;
    let mut points = util::generate_circle_points(
        0.0,
        -head_centre_y,
        head_radius,
        HEAD_SEGMENTS,
        180.0 + phi_right_deg,
        -phi_right_deg,
    );
    points.extend(util::generate_circle_points(
        -(-width / 2.0 + body_radius),
        -(body_top + body_radius),
        body_radius,
        CORNER_SEGMENTS,
        90.0,
        0.0,
    ));
    points.extend(util::generate_circle_points(
        -(-width / 2.0 + body_radius),
        -(total_height / 2.0 - body_radius),
        body_radius,
        CORNER_SEGMENTS,
        360.0,
        270.0,
    ));
    points.extend(util::generate_circle_points(
        -(width / 2.0 - body_radius),
        -(total_height / 2.0 - body_radius),
        body_radius,
        CORNER_SEGMENTS,
        270.0,
        180.0,
    ));
    points.extend(util::generate_circle_points(
        -(width / 2.0 - body_radius),
        -(body_top + body_radius),
        body_radius,
        CORNER_SEGMENTS,
        180.0,
        90.0,
    ));

    Part {
        svg,
        after_label: String::new(),
        bbox,
        intersect: Intersect::Polygon { points },
        label_offset: Some(label_offset),
        label: Some(label),
    }
}
