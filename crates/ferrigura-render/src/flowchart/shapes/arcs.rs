//! The shapes whose outline is made of arcs: `bang` and `cloud` (plain paths, drawn as the
//! reference writes them), and `bowTieRect`, `curvedTrapezoid` and `halfRoundedRectangle` (rough.js
//! outlines at roughness 0, through their sampled points). Ports of `shapes/bang.ts`, `cloud.ts`,
//! `bowTieRect.ts`, `curvedTrapezoid.ts` and `halfRoundedRectangle.ts`. See
//! `docs-port/flowchart/render-notes/shapes-core.md` §2.3 (rough.js) and §2.4 (outlines).

use std::f64::consts::PI;

use super::rough;
use super::util::{
    self, Part, attribute, generate_circle_points, label_helper, path_style, stroke_width,
    styles2string,
};
use super::{Intersect, ShapeInput};
use crate::flowchart::geom::{BBox, Point};
use crate::svg::number;

/// The number of points an elliptical arc is sampled at for its box. The box is then within a
/// thousandth of a pixel of the exact one for every arc these shapes draw.
const ARC_SAMPLES: usize = 512;

/// One arc of a path written with relative coordinates: `a rx,ry rotation 0,sweep dx,dy`. Every
/// arc of `bang` and `cloud` is written with a large-arc flag of 0.
#[derive(Debug, Clone, Copy)]
struct RelativeArc {
    rx: f64,
    ry: f64,
    rotation: f64,
    sweep: bool,
    dx: f64,
    dy: f64,
}

/// The path of relative arcs that `bang` and `cloud` write: `M0 0`, the arcs, then `H0 V0 Z`.
/// Returns the path data and the path's own box, as `getBBox` gives it, in the path's coordinates.
fn relative_arc_path(arcs: &[RelativeArc]) -> (String, BBox) {
    let mut d = String::from("M0 0");
    let mut current = Point::default();
    let mut points = vec![current];
    for arc in arcs {
        d.push_str(&format!(
            " a{},{} {} 0,{} {},{}",
            number(arc.rx),
            number(arc.ry),
            number(arc.rotation),
            u8::from(arc.sweep),
            number(arc.dx),
            number(arc.dy),
        ));
        let target = Point::new(current.x + arc.dx, current.y + arc.dy);
        points.extend(arc_points(
            current,
            target,
            arc.rx,
            arc.ry,
            arc.rotation,
            false,
            arc.sweep,
        ));
        current = target;
    }
    d.push_str(" H0 V0 Z");
    (d, BBox::from_points(points).unwrap_or_default())
}

/// `bang.ts`: a burst of small arcs around the label. The burst is the label's box padded by half
/// the padding on each side, and at least 20 larger than the label. The label's centre is moved to
/// the centre of the burst's own box, which is not the node's centre, as the reference does.
pub(super) fn bang(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let label = label_helper(input, false);
    let half_padding = node.padding / 2.0;
    let w = label.bbox.width + 10.0 * half_padding;
    let h = label.bbox.height + 8.0 * half_padding;
    let r = 0.15 * w;
    let width = w.max(label.bbox.width + 20.0);
    let height = h.max(label.bbox.height + 20.0);

    let arc = |rx: f64, ry: f64, sweep: bool, dx: f64, dy: f64| RelativeArc {
        rx,
        ry,
        rotation: 1.0,
        sweep,
        dx,
        dy,
    };
    let arcs = [
        arc(r, r, false, width * 0.25, -height * 0.1),
        arc(r, r, false, width * 0.25, 0.0),
        arc(r, r, false, width * 0.25, 0.0),
        arc(r, r, false, width * 0.25, height * 0.1),
        arc(r, r, false, width * 0.15, height * 0.33),
        arc(r * 0.8, r * 0.8, false, 0.0, height * 0.34),
        arc(r, r, false, -width * 0.15, height * 0.33),
        arc(r, r, false, -width * 0.25, height * 0.15),
        arc(r, r, false, -width * 0.25, 0.0),
        arc(r, r, false, -width * 0.25, 0.0),
        arc(r, r, false, -width * 0.25, -height * 0.15),
        arc(r, r, false, -width * 0.1, -height * 0.33),
        arc(r * 0.8, r * 0.8, false, 0.0, -height * 0.34),
        arc(r, r, false, width * 0.1, -height * 0.33),
    ];
    let (d, path_box) = relative_arc_path(&arcs);
    let (translate_x, translate_y) = (-width / 2.0, -height / 2.0);
    let svg = format!(
        "<path class=\"basic label-container\"{} d=\"{d}\" transform=\"translate({}, {})\"/>",
        attribute("style", &styles.node),
        number(translate_x),
        number(translate_y),
    );
    // The label is moved to `translate(-W/2, -H/2)` (`bang.ts`), not to the centred offset.
    let label_offset = Point::new(-label.bbox.width / 2.0, -label.bbox.height / 2.0);
    Part {
        svg,
        after_label: String::new(),
        bbox: path_box.translate(translate_x, translate_y),
        intersect: Intersect::Rect,
        label_offset: Some(label_offset),
        label: Some(label),
    }
}

/// `cloud.ts`: a cloud of arcs around the label, the label's box padded by the padding.
pub(super) fn cloud(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let label = label_helper(input, false);
    let half_padding = node.padding / 2.0;
    let w = label.bbox.width + 2.0 * half_padding;
    let h = label.bbox.height + 2.0 * half_padding;
    let r1 = 0.15 * w;
    let r2 = 0.25 * w;
    let r3 = 0.35 * w;
    let r4 = 0.2 * w;

    let arc = |rx: f64, ry: f64, rotation: f64, dx: f64, dy: f64| RelativeArc {
        rx,
        ry,
        rotation,
        sweep: true,
        dx,
        dy,
    };
    let arcs = [
        arc(r1, r1, 0.0, w * 0.25, -w * 0.1),
        arc(r3, r3, 1.0, w * 0.4, -w * 0.1),
        arc(r2, r2, 1.0, w * 0.35, w * 0.2),
        arc(r1, r1, 1.0, w * 0.15, h * 0.35),
        arc(r4, r4, 1.0, -w * 0.15, h * 0.65),
        arc(r2, r1, 1.0, -w * 0.25, w * 0.15),
        arc(r3, r3, 1.0, -w * 0.5, 0.0),
        arc(r1, r1, 1.0, -w * 0.25, -w * 0.15),
        arc(r1, r1, 1.0, -w * 0.1, -h * 0.35),
        arc(r4, r4, 1.0, w * 0.1, -h * 0.65),
    ];
    let (d, path_box) = relative_arc_path(&arcs);
    let (translate_x, translate_y) = (-w / 2.0, -h / 2.0);
    let svg = format!(
        "<path class=\"basic label-container\"{} d=\"{d}\" transform=\"translate({}, {})\"/>",
        attribute("style", &styles.node),
        number(translate_x),
        number(translate_y),
    );
    // The label is moved to `translate(-W/2, -H/2)` (`cloud.ts`), not to the centred offset.
    let label_offset = Point::new(-label.bbox.width / 2.0, -label.bbox.height / 2.0);
    Part {
        svg,
        after_label: String::new(),
        bbox: path_box.translate(translate_x, translate_y),
        intersect: Intersect::Rect,
        label_offset: Some(label_offset),
        label: Some(label),
    }
}

/// `bowTieRect.ts`: a rectangle whose two short sides are elliptical arcs, drawn as a polygon of
/// their sampled points. The arcs' sagitta is added to the width, and the outline is moved half a
/// radius to the right.
pub(super) fn bow_tie_rect(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let (padding_x, padding_y) = if node.is_neo() {
        (16.0, 12.0)
    } else {
        (node.padding, node.padding)
    };
    let label = label_helper(input, false);
    let height = label.bbox.height + padding_y;
    let ry = height / 2.0;
    let rx = ry / (2.5 + height / 50.0);
    let sagitta = arc_sagitta(height, rx, ry);
    let total_width = label.bbox.width + padding_x * 2.0 + sagitta;
    let w = total_width - sagitta;
    let h = height;

    let mut points = vec![
        Point::new(w / 2.0, -h / 2.0),
        Point::new(-w / 2.0, -h / 2.0),
    ];
    points.extend(bow_tie_arc(
        -w / 2.0,
        -h / 2.0,
        -w / 2.0,
        h / 2.0,
        rx,
        ry,
        false,
    ));
    points.push(Point::new(w / 2.0, h / 2.0));
    points.extend(bow_tie_arc(
        w / 2.0,
        h / 2.0,
        w / 2.0,
        -h / 2.0,
        rx,
        ry,
        true,
    ));

    let outline = rough::polygon(&points);
    let style = path_style(node, &styles);
    let shift = rx / 2.0;
    let svg = format!(
        "<g class=\"basic label-container outer-path\" transform=\"translate({}, 0)\">{}</g>",
        number(shift),
        rough::paths(&outline, Some(style.as_str()), &stroke_width(&styles), true),
    );
    Part {
        svg,
        after_label: String::new(),
        bbox: outline.bbox.translate(shift, 0.0),
        intersect: Intersect::Polygon { points },
        label_offset: Some(util::centred_label_offset(&label)),
        label: Some(label),
    }
}

/// `curvedTrapezoid.ts`: a trapezoid whose narrow end is a half circle, the display shape. The neo
/// look pads the label by 16 and 12, the classic look by the padding. The label is wide enough to
/// clear the chevron and the cap at its corners.
pub(super) fn curved_trapezoid(input: &ShapeInput<'_>) -> Part {
    const MIN_WIDTH: f64 = 20.0;
    const MIN_HEIGHT: f64 = 5.0;
    let node = input.node;
    let styles = styles2string(node);
    let (padding_x, padding_y) = if node.is_neo() {
        (16.0, 12.0)
    } else {
        (node.padding, node.padding)
    };
    let label = label_helper(input, false);
    let height = MIN_HEIGHT
        .max(label.bbox.height + padding_y * 2.0)
        .max(node.height.unwrap_or(0.0));
    let radius = height / 2.0;
    let cap_clearance = radius - cap_inset(radius, label.bbox.height);
    let side_clearance = (height / 4.0).max(cap_clearance);
    let width = MIN_WIDTH
        .max(label.bbox.width + padding_x * 2.0 + side_clearance * 2.0)
        .max((label.bbox.width + padding_x * 2.0) * 1.25)
        .max(node.width.unwrap_or(0.0));

    let rw = width - radius;
    let tw = height / 4.0;
    let mut points = vec![
        Point::new(rw, 0.0),
        Point::new(tw, 0.0),
        Point::new(0.0, height / 2.0),
        Point::new(tw, height),
        Point::new(rw, height),
    ];
    points.extend(generate_circle_points(
        -rw,
        -height / 2.0,
        radius,
        50,
        270.0,
        90.0,
    ));

    let outline = rough::polygon(&points);
    let style = path_style(node, &styles);
    let svg = format!(
        "<g class=\"basic label-container outer-path\" transform=\"translate({}, {})\">{}</g>",
        number(-width / 2.0),
        number(-height / 2.0),
        rough::paths(&outline, Some(style.as_str()), &stroke_width(&styles), true),
    );
    Part {
        svg,
        after_label: String::new(),
        bbox: outline.bbox.translate(-width / 2.0, -height / 2.0),
        intersect: Intersect::Polygon { points },
        label_offset: Some(util::centred_label_offset(&label)),
        label: Some(label),
    }
}

/// `halfRoundedRectangle.ts`: a rectangle whose right side is a half circle, the delay shape. The
/// neo look pads the label by 16 and 12, the classic look by the padding.
pub(super) fn half_rounded_rectangle(input: &ShapeInput<'_>) -> Part {
    const MIN_WIDTH: f64 = 15.0;
    const MIN_HEIGHT: f64 = 10.0;
    let node = input.node;
    let styles = styles2string(node);
    let (padding_x, padding_y) = if node.is_neo() {
        (16.0, 12.0)
    } else {
        (node.padding, node.padding)
    };
    let label = label_helper(input, false);
    let height = MIN_HEIGHT.max(label.bbox.height) + padding_y * 2.0;
    let radius = height / 2.0;
    let cap_clearance = radius - cap_inset(radius, label.bbox.height);
    let width = MIN_WIDTH.max(label.bbox.width) + cap_clearance * 2.0 + padding_x * 2.0;

    let mut points = vec![
        Point::new(-width / 2.0, -height / 2.0),
        Point::new(width / 2.0 - radius, -height / 2.0),
    ];
    points.extend(generate_circle_points(
        -width / 2.0 + radius,
        0.0,
        radius,
        50,
        90.0,
        270.0,
    ));
    points.push(Point::new(width / 2.0 - radius, height / 2.0));
    points.push(Point::new(-width / 2.0, height / 2.0));

    let outline = rough::polygon(&points);
    let style = path_style(node, &styles);
    let svg = format!(
        "<g class=\"basic label-container outer-path\">{}</g>",
        rough::paths(&outline, Some(style.as_str()), &stroke_width(&styles), true),
    );
    Part {
        svg,
        after_label: String::new(),
        bbox: outline.bbox,
        intersect: Intersect::Polygon { points },
        label_offset: Some(util::centred_label_offset(&label)),
        label: Some(label),
    }
}

/// How far a cap of radius `radius` reaches in from its outer edge at the height of a label's top
/// corner, `sqrt(max(0, r^2 - (h/2)^2))` (`halfRoundedRectangle.ts`, `curvedTrapezoid.ts`).
fn cap_inset(radius: f64, label_height: f64) -> f64 {
    (radius * radius - (label_height / 2.0).powi(2))
        .max(0.0)
        .sqrt()
}

/// `calculateArcSagitta` (`bowTieRect.ts`): the bulge of an elliptical arc over a chord, with the
/// longer radius as the major axis.
fn arc_sagitta(chord: f64, radius_x: f64, radius_y: f64) -> f64 {
    let (major, minor) = if radius_x >= radius_y {
        (radius_x, radius_y)
    } else {
        (radius_y, radius_x)
    };
    minor * (1.0 - (1.0 - (chord / major / 2.0).powi(2)).sqrt())
}

/// `generateArcPoints` (`bowTieRect.ts`): 20 points of the elliptical arc from `(x1, y1)` to
/// `(x2, y2)`, on the clockwise or the counter-clockwise side of the chord.
fn bow_tie_arc(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    rx: f64,
    ry: f64,
    clockwise: bool,
) -> Vec<Point> {
    const POINTS: usize = 20;
    let mid_x = (x1 + x2) / 2.0;
    let mid_y = (y1 + y2) / 2.0;
    let chord_angle = (y2 - y1).atan2(x2 - x1);
    let dx = (x2 - x1) / 2.0;
    let dy = (y2 - y1) / 2.0;
    let distance = ((dx / rx).powi(2) + (dy / ry).powi(2)).sqrt();
    let scaled = (1.0 - distance * distance).max(0.0).sqrt();
    let side = if clockwise { -1.0 } else { 1.0 };
    let centre_x = mid_x + scaled * ry * chord_angle.sin() * side;
    let centre_y = mid_y - scaled * rx * chord_angle.cos() * side;
    let start = ((y1 - centre_y) / ry).atan2((x1 - centre_x) / rx);
    let end = ((y2 - centre_y) / ry).atan2((x2 - centre_x) / rx);
    let mut range = end - start;
    if clockwise && range < 0.0 {
        range += 2.0 * PI;
    }
    if !clockwise && range > 0.0 {
        range -= 2.0 * PI;
    }
    (0..POINTS)
        .map(|index| {
            let t = index as f64 / (POINTS - 1) as f64;
            let theta = start + t * range;
            Point::new(centre_x + rx * theta.cos(), centre_y + ry * theta.sin())
        })
        .collect()
}

/// The points of the elliptical arc from `from` to `to`, sampled from start to end (SVG's endpoint
/// parameterisation, appendix F.6.5). The radii grow when they are too small for the chord, and
/// the ellipse is rotated by `rotation` degrees about its centre.
#[must_use]
pub(super) fn arc_points(
    from: Point,
    to: Point,
    rx: f64,
    ry: f64,
    rotation: f64,
    large: bool,
    sweep: bool,
) -> Vec<Point> {
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if rx == 0.0 || ry == 0.0 {
        return vec![from, to];
    }
    let (sin, cos) = rotation.to_radians().sin_cos();
    let dx2 = (from.x - to.x) / 2.0;
    let dy2 = (from.y - to.y) / 2.0;
    let x1 = cos * dx2 + sin * dy2;
    let y1 = -sin * dx2 + cos * dy2;
    let lambda = x1 * x1 / (rx * rx) + y1 * y1 / (ry * ry);
    if lambda > 1.0 {
        rx *= lambda.sqrt();
        ry *= lambda.sqrt();
    }
    let numerator = rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1;
    let denominator = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let sign = if large != sweep { 1.0 } else { -1.0 };
    let factor = sign * (numerator / denominator).max(0.0).sqrt();
    let cxp = factor * rx * y1 / ry;
    let cyp = factor * -ry * x1 / rx;
    let centre_x = cos * cxp - sin * cyp + (from.x + to.x) / 2.0;
    let centre_y = sin * cxp + cos * cyp + (from.y + to.y) / 2.0;
    let angle = |ux: f64, uy: f64, vx: f64, vy: f64| {
        let sign = if ux * vy - uy * vx < 0.0 { -1.0 } else { 1.0 };
        let cosine = ((ux * vx + uy * vy)
            / ((ux * ux + uy * uy).sqrt() * (vx * vx + vy * vy).sqrt()))
        .clamp(-1.0, 1.0);
        sign * cosine.acos()
    };
    let theta1 = angle(1.0, 0.0, (x1 - cxp) / rx, (y1 - cyp) / ry);
    let mut delta = angle(
        (x1 - cxp) / rx,
        (y1 - cyp) / ry,
        (-x1 - cxp) / rx,
        (-y1 - cyp) / ry,
    );
    if !sweep && delta > 0.0 {
        delta -= 2.0 * PI;
    } else if sweep && delta < 0.0 {
        delta += 2.0 * PI;
    }
    (0..=ARC_SAMPLES)
        .map(|step| {
            let theta = theta1 + delta * step as f64 / ARC_SAMPLES as f64;
            Point::new(
                centre_x + rx * cos * theta.cos() - ry * sin * theta.sin(),
                centre_y + rx * sin * theta.cos() + ry * cos * theta.sin(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_half_circle_is_sampled_on_its_circle_between_its_ends() {
        // From (-5, 0) to (5, 0) with radius 5, sweeping the positive angle: through (0, -5).
        let points = arc_points(
            Point::new(-5.0, 0.0),
            Point::new(5.0, 0.0),
            5.0,
            5.0,
            0.0,
            false,
            true,
        );
        let first = points.first().copied().expect("a sample");
        let last = points.last().copied().expect("a sample");
        assert!(
            (first.x + 5.0).abs() < 1e-9 && first.y.abs() < 1e-9,
            "{first:?}"
        );
        assert!(
            (last.x - 5.0).abs() < 1e-9 && last.y.abs() < 1e-9,
            "{last:?}"
        );
        for point in &points {
            assert!((point.x.hypot(point.y) - 5.0).abs() < 1e-9, "{point:?}");
        }
        // A positive sweep turns through (0, -5) in SVG's y-down coordinates; the sampled extreme is
        // within the sampling error of 512 steps.
        let lowest = points
            .iter()
            .map(|point| point.y)
            .fold(f64::INFINITY, f64::min);
        assert!((lowest + 5.0).abs() < 1e-4, "{lowest}");
    }

    #[test]
    fn an_arc_too_small_for_its_chord_grows_to_a_half_circle() {
        // The radius 1 is too small for the chord of 10: the arc becomes the half circle of 5.
        let points = arc_points(
            Point::new(-5.0, 0.0),
            Point::new(5.0, 0.0),
            1.0,
            1.0,
            0.0,
            false,
            true,
        );
        let furthest = points.iter().map(|point| point.y.abs()).fold(0.0, f64::max);
        assert!((furthest - 5.0).abs() < 1e-6, "{furthest}");
    }

    #[test]
    fn a_rotated_ellipse_reaches_its_rotated_extent() {
        // A quarter of an ellipse with radii 10 and 2, turned by 90 degrees, is the quarter of the
        // ellipse with radii 2 and 10: its widest point is 2 from the centre, not 10.
        let points = arc_points(
            Point::new(0.0, -10.0),
            Point::new(2.0, 0.0),
            10.0,
            2.0,
            90.0,
            false,
            true,
        );
        let widest = points.iter().map(|point| point.x.abs()).fold(0.0, f64::max);
        assert!(widest <= 2.0 + 1e-6, "{widest}");
    }
}
