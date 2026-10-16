//! rough.js at roughness 0, as the reference's shapes draw with it.
//!
//! The shapes that the reference draws with `rc.*` set `roughness: 0` and `fillStyle: 'solid'`
//! (`shapes/*.ts`, `options.roughness = 0`), so the drawing is deterministic: this module ports the
//! generator (`generator.js`: `polygon`, `rectangle`, `line`, `ellipse`/`circle`, `path`) and the
//! renderer's ops that they reach (`renderer.js`: `linearPath`, `_doubleLine`, `_line`, `svgPath`,
//! `_bezierTo`, `solidFillPolygon`, `_computeEllipsePoints`, `_curve`). The `handDrawn` look is out of
//! scope.
//!
//! What roughness 0 keeps of the random parts (`random(o)`, `_offset`):
//! - every offset is multiplied by the roughness, so the points and the control points of a line
//!   lie on the line, and an ellipse is its sampled core;
//! - the divergence point of a double stroke is `0.2 + random * 0.2`. The control points lie on the
//!   segment whatever it is, so the geometry does not depend on it; a fixed fraction stands for it
//!   (`DIVERGE`).
//!
//! The drawing has a fill path, when the options give a fill, and a stroke path, when the stroke is
//! not `none`. The ops are written as rough.js writes them (`opsToPath`). See
//! `docs-port/flowchart/render-notes/shapes-core.md` §2.3.

use std::f64::consts::PI;

use super::util::attribute;
use crate::svg::{escape, number};
use crate::unified::geom::{BBox, Point};

/// rough.js's default `curveStepCount`.
const CURVE_STEP_COUNT: f64 = 9.0;

/// The divergence point of a double stroke, as a fraction of the segment (rough.js draws it at
/// `0.2 + random * 0.2`, and the second control point at twice that).
const DIVERGE: f64 = 0.3;

/// One op of a rough.js path (`move`, `lineTo` and `bcurveTo` in `renderer.js`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Op {
    /// `move`: the start of a pass.
    Move(Point),
    /// `lineTo`: a straight segment (`solidFillPolygon`).
    Line(Point),
    /// `bcurveTo`: a cubic with its two control points and its end point.
    Curve(Point, Point, Point),
}

/// A segment of an SVG path as `normalize` leaves it: moves, lines, cubic curves and closes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Segment {
    /// `M`.
    Move(Point),
    /// `L` (and `H` and `V`, which `normalize` writes as lines).
    Line(Point),
    /// `C` (and the arcs, which `normalize` writes as cubics).
    Curve(Point, Point, Point),
    /// `Z`.
    Close,
}

/// What a drawing paints: its fill (`fill` set and not `none`) and its stroke (`stroke` not `none`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Paint {
    /// Whether the drawing has a fill path.
    pub fill: bool,
    /// Whether the drawing has a stroke path.
    pub stroke: bool,
}

impl Paint {
    /// A fill and a stroke, the usual options of a node's outline.
    pub(super) const BOTH: Self = Self {
        fill: true,
        stroke: true,
    };
    /// A stroke alone, as `rc.line` and the `fill: none` outlines.
    pub(super) const STROKE: Self = Self {
        fill: false,
        stroke: true,
    };
    /// A fill alone, as the icons' outer boxes draw (`fill: 'transparent'` and `stroke: 'none'`):
    /// rough.js writes a fill path for any fill that is not undefined, and no stroke path for `none`.
    pub(super) const FILL: Self = Self {
        fill: true,
        stroke: false,
    };
}

/// A drawing: the ops of its fill path and of its stroke path, and the box of its geometry.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Outline {
    /// The ops of the fill path, when the drawing has one.
    pub fill: Option<Vec<Op>>,
    /// The ops of the stroke path, when the drawing has one.
    pub stroke: Option<Vec<Op>>,
    /// The box of the drawing's geometry, as `getBBox` gives it.
    pub bbox: BBox,
}

/// The attributes of a drawing's paths: the style they all carry, and the colours that the
/// reference writes as presentation attributes (`fill`, `fill-rule`, `stroke`). The flowchart's
/// outlines take their colours from the style sheet (`docs-port/decisions/0007`), so they set none.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Attrs<'a> {
    /// The `style` of both paths, when the node has style declarations.
    pub style: Option<&'a str>,
    /// The fill path's colour, and whether its rule is `evenodd` (`rc.polygon` gives it, `rc.rectangle`
    /// does not).
    pub fill: Option<Fill<'a>>,
    /// The stroke path's colour.
    pub stroke: Option<&'a str>,
    /// The stroke path's width.
    pub stroke_width: &'a str,
}

/// The fill of a path: its colour, and whether its rule is `evenodd`.
#[derive(Debug, Clone, Copy)]
pub(super) struct Fill<'a> {
    /// The fill colour.
    pub colour: &'a str,
    /// Whether the fill's rule is `evenodd`.
    pub evenodd: bool,
}

/// The box of a list of points.
fn points_box(points: impl IntoIterator<Item = Point>) -> BBox {
    BBox::from_points(points).unwrap_or_default()
}

/// `_doubleLine` (`renderer.js`): a line drawn as one pass or two. Each pass is a `move` to the start
/// and a `bcurveTo` whose control points lie on the segment, so both passes trace it.
fn double_line(from: Point, to: Point, passes: usize, ops: &mut Vec<Op>) {
    let first = Point::new(
        from.x + (to.x - from.x) * DIVERGE,
        from.y + (to.y - from.y) * DIVERGE,
    );
    let second = Point::new(
        from.x + 2.0 * (to.x - from.x) * DIVERGE,
        from.y + 2.0 * (to.y - from.y) * DIVERGE,
    );
    for _ in 0..passes {
        ops.push(Op::Move(from));
        ops.push(Op::Curve(first, second, to));
    }
}

/// `_bezierTo` (`renderer.js`): a cubic of an svg path, drawn as one pass or two from `current`.
fn bezier_to(current: Point, c1: Point, c2: Point, end: Point, passes: usize, ops: &mut Vec<Op>) {
    for _ in 0..passes {
        ops.push(Op::Move(current));
        ops.push(Op::Curve(c1, c2, end));
    }
}

/// `linearPath` (`renderer.js`): the sides of a polyline, each drawn as a double line. A closed
/// polyline has its closing side too. A polyline of two points is one line, closed or not.
fn linear_path_ops(points: &[Point], close: bool, passes: usize, ops: &mut Vec<Op>) {
    let len = points.len();
    if len > 2 {
        for pair in points.windows(2) {
            double_line(pair[0], pair[1], passes, ops);
        }
        if close {
            double_line(points[len - 1], points[0], passes, ops);
        }
    } else if len == 2 {
        double_line(points[0], points[1], passes, ops);
    }
}

/// `solidFillPolygon` (`generator.js` and `renderer.js`): each polygon of at least three points is
/// filled as `move` to its first point and `lineTo` to the others. The fill is open: its closing side
/// is not drawn.
fn solid_fill_polygon(polygons: &[Vec<Point>]) -> Vec<Op> {
    let mut ops = Vec::new();
    for points in polygons {
        if points.len() > 2 {
            ops.push(Op::Move(points[0]));
            for point in &points[1..] {
                ops.push(Op::Line(*point));
            }
        }
    }
    ops
}

/// `svgPath` (`renderer.js`): the ops of an svg path, with the passes of every segment. `M` writes no
/// op of its own; the segments that follow it start from its point, and `Z` closes to it.
fn svg_path_ops(segments: &[Segment], passes: usize) -> Vec<Op> {
    let mut ops = Vec::new();
    let mut current = Point::default();
    let mut first = Point::default();
    for segment in segments {
        match *segment {
            Segment::Move(point) => {
                current = point;
                first = point;
            }
            Segment::Line(point) => {
                double_line(current, point, passes, &mut ops);
                current = point;
            }
            Segment::Curve(c1, c2, end) => {
                bezier_to(current, c1, c2, end, passes, &mut ops);
                current = end;
            }
            Segment::Close => {
                double_line(current, first, passes, &mut ops);
                current = first;
            }
        }
    }
    ops
}

/// The fill of a path with one sub-path (`path` in `generator.js`): the path's single pass, with
/// every `move` after the first removed (`_mergedShape`), so the fill is one open or closed polyline
/// through the whole path.
fn merged_fill(segments: &[Segment]) -> Vec<Op> {
    svg_path_ops(segments, 1)
        .into_iter()
        .enumerate()
        .filter(|(index, op)| *index == 0 || !matches!(op, Op::Move(_)))
        .map(|(_, op)| op)
        .collect()
}

/// `createPathFromPoints` (`shapes/util.ts`): `M p0 L p1 ... Z`.
#[must_use]
pub(super) fn points_path(points: &[Point]) -> Vec<Segment> {
    let mut segments = Vec::with_capacity(points.len() + 1);
    for (index, point) in points.iter().enumerate() {
        segments.push(if index == 0 {
            Segment::Move(*point)
        } else {
            Segment::Line(*point)
        });
    }
    segments.push(Segment::Close);
    segments
}

/// `createRoundedRectPathD` (`shapes/roundedRectPath.ts`), as the segments that `normalize` gives it:
/// its arcs are cubics (`arcToCubicCurves`).
#[must_use]
pub(super) fn rounded_rectangle_path(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    radius: f64,
) -> Vec<Segment> {
    let mut segments = vec![Segment::Move(Point::new(x + radius, y))];
    let arc = |segments: &mut Vec<Segment>, from: Point, to: Point| {
        for [c1, c2, end] in arc_to_cubic(from, to, radius, radius, false, true) {
            segments.push(Segment::Curve(c1, c2, end));
        }
    };
    let corners = [
        (
            Point::new(x + width - radius, y),
            Point::new(x + width, y + radius),
        ),
        (
            Point::new(x + width, y + height - radius),
            Point::new(x + width - radius, y + height),
        ),
        (
            Point::new(x + radius, y + height),
            Point::new(x, y + height - radius),
        ),
        (Point::new(x, y + radius), Point::new(x + radius, y)),
    ];
    for (edge_start, edge_end) in corners {
        segments.push(Segment::Line(edge_start));
        arc(&mut segments, edge_start, edge_end);
    }
    segments.push(Segment::Close);
    segments
}

/// `arcToCubicCurves` (`path-data-parser`, `normalize.js`) for an arc without rotation: the cubic
/// curves `[control, control, end]` that approximate the arc from `from` to `to`. An arc longer than
/// 120 degrees is split in two, as the library does.
fn arc_to_cubic(
    from: Point,
    to: Point,
    rx: f64,
    ry: f64,
    large: bool,
    sweep: bool,
) -> Vec<[Point; 3]> {
    let flat = arc_to_flat(from, to, rx, ry, large, sweep, None);
    flat.as_chunks::<3>().0.to_vec()
}

/// The control points and end points of an arc, flat: `[c1, c2, end]` triples, in order.
/// `recursive` carries the angles and the centre of a split arc: `[f1, f2, cx, cy]`.
fn arc_to_flat(
    from: Point,
    to: Point,
    rx: f64,
    ry: f64,
    large: bool,
    sweep: bool,
    recursive: Option<[f64; 4]>,
) -> Vec<Point> {
    let (x1, y1, x2, y2) = (from.x, from.y, to.x, to.y);
    let (mut r1, mut r2) = (rx.abs(), ry.abs());
    let (mut f1, mut f2, cx, cy);
    let (mut x2, mut y2) = (x2, y2);
    if let Some([rf1, rf2, rcx, rcy]) = recursive {
        f1 = rf1;
        f2 = rf2;
        cx = rcx;
        cy = rcy;
    } else {
        let x = (x1 - x2) / 2.0;
        let y = (y1 - y2) / 2.0;
        let mut h = (x * x) / (r1 * r1) + (y * y) / (r2 * r2);
        if h > 1.0 {
            h = h.sqrt();
            r1 *= h;
            r2 *= h;
        }
        let sign = if large == sweep { -1.0 } else { 1.0 };
        let r1_pow = r1 * r1;
        let r2_pow = r2 * r2;
        let left = r1_pow * r2_pow - r1_pow * y * y - r2_pow * x * x;
        let right = r1_pow * y * y + r2_pow * x * x;
        let k = sign * (left / right).abs().sqrt();
        cx = k * r1 * y / r2 + (x1 + x2) / 2.0;
        cy = k * -r2 * x / r1 + (y1 + y2) / 2.0;
        f1 = round9((y1 - cy) / r2).asin();
        f2 = round9((y2 - cy) / r2).asin();
        if x1 < cx {
            f1 = PI - f1;
        }
        if x2 < cx {
            f2 = PI - f2;
        }
        if f1 < 0.0 {
            f1 += PI * 2.0;
        }
        if f2 < 0.0 {
            f2 += PI * 2.0;
        }
        if sweep && f1 > f2 {
            f1 -= PI * 2.0;
        }
        if !sweep && f2 > f1 {
            f2 -= PI * 2.0;
        }
    }

    let mut params = Vec::new();
    let mut df = f2 - f1;
    let limit = PI * 120.0 / 180.0;
    if df.abs() > limit {
        let f2old = f2;
        let (x2old, y2old) = (x2, y2);
        f2 = if sweep && f2 > f1 {
            f1 + limit
        } else {
            f1 - limit
        };
        x2 = cx + r1 * f2.cos();
        y2 = cy + r2 * f2.sin();
        params = arc_to_flat(
            Point::new(x2, y2),
            Point::new(x2old, y2old),
            r1,
            r2,
            false,
            sweep,
            Some([f2, f2old, cx, cy]),
        );
    }
    df = f2 - f1;
    let (c1, s1, c2, s2) = (f1.cos(), f1.sin(), f2.cos(), f2.sin());
    let t = (df / 4.0).tan();
    let hx = 4.0 / 3.0 * r1 * t;
    let hy = 4.0 / 3.0 * r2 * t;
    let m2 = Point::new(2.0 * x1 - (x1 + hx * s1), 2.0 * y1 - (y1 - hy * c1));
    let m3 = Point::new(x2 + hx * s2, y2 - hy * c2);
    let m4 = Point::new(x2, y2);
    let mut out = vec![m2, m3, m4];
    out.extend(params);
    out
}

/// `Math.parseFloat(x.toFixed(9))`, which `arcToCubicCurves` applies to its asin arguments.
fn round9(value: f64) -> f64 {
    format!("{value:.9}").parse().unwrap_or(value)
}

/// `ellipseWithParams` (`renderer.js`) at roughness 0: the ellipse's core, sampled a quarter of a step
/// from one step before the start to one step past the end, and joined by Catmull-Rom cubics
/// (`_computeEllipsePoints` with `coreOnly`, `_curve` with tightness 0). Returns the curve's ops and the
/// points that bound it.
fn ellipse_ops(cx: f64, cy: f64, width: f64, height: f64) -> (Vec<Op>, Vec<Point>) {
    // `generateEllipseParams`: the number of steps grows with the size.
    let rx = (width / 2.0).abs();
    let ry = (height / 2.0).abs();
    let psq = (PI * 2.0 * (((width / 2.0).powi(2) + (height / 2.0).powi(2)) / 2.0).sqrt()).sqrt();
    let steps = CURVE_STEP_COUNT
        .max(CURVE_STEP_COUNT / 200.0_f64.sqrt() * psq)
        .ceil();
    let increment = 2.0 * PI / steps;
    let step = increment / 4.0;
    let sample = |angle: f64| Point::new(cx + rx * angle.cos(), cy + ry * angle.sin());
    let mut points = vec![sample(-step)];
    let mut angle = 0.0;
    while angle <= 2.0 * PI {
        points.push(sample(angle));
        angle += step;
    }
    points.push(sample(0.0));
    points.push(sample(step));

    let mut ops = Vec::new();
    let mut bounds = Vec::new();
    ops.push(Op::Move(points[1]));
    bounds.push(points[1]);
    for i in 1..points.len().saturating_sub(2) {
        let (before, here, next, after) = (points[i - 1], points[i], points[i + 1], points[i + 2]);
        let control_1 = Point::new(
            here.x + (next.x - before.x) / 6.0,
            here.y + (next.y - before.y) / 6.0,
        );
        let control_2 = Point::new(
            next.x + (here.x - after.x) / 6.0,
            next.y + (here.y - after.y) / 6.0,
        );
        ops.push(Op::Curve(control_1, control_2, next));
        bounds.extend(cubic_extremes(here, control_1, control_2, next));
    }
    (ops, bounds)
}

/// The points where a cubic curve is furthest in x or y, with its end points: the points that
/// give its tight box, as `getBBox` computes it.
fn cubic_extremes(p0: Point, c1: Point, c2: Point, p3: Point) -> Vec<Point> {
    let mut points = vec![p0, p3];
    for axis in [Axis::X, Axis::Y] {
        // The derivative of the curve is a quadratic in t: a t^2 + b t + c.
        let (a0, a1, a2, a3) = axis.pick(p0, c1, c2, p3);
        let d0 = a1 - a0;
        let d1 = a2 - a1;
        let d2 = a3 - a2;
        let a = d0 - 2.0 * d1 + d2;
        let b = 2.0 * (d1 - d0);
        let c = d0;
        for t in quadratic_roots(a, b, c) {
            if t > 0.0 && t < 1.0 {
                points.push(evaluate(p0, c1, c2, p3, t));
            }
        }
    }
    points
}

/// The real roots of `a t^2 + b t + c`, a linear equation when `a` is zero.
fn quadratic_roots(a: f64, b: f64, c: f64) -> Vec<f64> {
    if a.abs() < 1e-12 {
        return if b.abs() < 1e-12 {
            Vec::new()
        } else {
            vec![-c / b]
        };
    }
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return Vec::new();
    }
    let root = discriminant.sqrt();
    vec![(-b + root) / (2.0 * a), (-b - root) / (2.0 * a)]
}

/// The point of a cubic curve at `t`.
fn evaluate(p0: Point, c1: Point, c2: Point, p3: Point, t: f64) -> Point {
    let u = 1.0 - t;
    let weight = |a: f64, b: f64, c: f64, d: f64| {
        u * u * u * a + 3.0 * u * u * t * b + 3.0 * u * t * t * c + t * t * t * d
    };
    Point::new(
        weight(p0.x, c1.x, c2.x, p3.x),
        weight(p0.y, c1.y, c2.y, p3.y),
    )
}

#[derive(Debug, Clone, Copy)]
enum Axis {
    X,
    Y,
}

impl Axis {
    fn pick(self, p0: Point, c1: Point, c2: Point, p3: Point) -> (f64, f64, f64, f64) {
        match self {
            Self::X => (p0.x, c1.x, c2.x, p3.x),
            Self::Y => (p0.y, c1.y, c2.y, p3.y),
        }
    }
}

/// The box of the geometry of a list of segments: its points, and the extremes of its curves.
fn segments_box(segments: &[Segment]) -> BBox {
    let mut points = Vec::new();
    let mut current = Point::default();
    let mut first = Point::default();
    for segment in segments {
        match *segment {
            Segment::Move(point) => {
                current = point;
                first = point;
                points.push(point);
            }
            Segment::Line(point) => {
                points.push(current);
                points.push(point);
                current = point;
            }
            Segment::Curve(c1, c2, end) => {
                points.extend(cubic_extremes(current, c1, c2, end));
                current = end;
            }
            Segment::Close => {
                points.push(current);
                points.push(first);
                current = first;
            }
        }
    }
    points_box(points)
}

/// `points-on-curve`'s `getPointsOnBezierCurveWithSplitting`: the points of a cubic at the given
/// flatness, subdividing it at its midpoint until it is flat enough.
fn points_on_cubic(p: [Point; 4], tolerance: f64, out: &mut Vec<Point>) {
    if flatness(p) < tolerance {
        let p0 = p[0];
        match out.last() {
            Some(last) if last.distance(p0) > 1.0 => out.push(p0),
            Some(_) => {}
            None => out.push(p0),
        }
        out.push(p[3]);
    } else {
        let lerp = |a: Point, b: Point| Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
        let q1 = lerp(p[0], p[1]);
        let q2 = lerp(p[1], p[2]);
        let q3 = lerp(p[2], p[3]);
        let r1 = lerp(q1, q2);
        let r2 = lerp(q2, q3);
        let red = lerp(r1, r2);
        points_on_cubic([p[0], q1, r1, red], tolerance, out);
        points_on_cubic([red, r2, q3, p[3]], tolerance, out);
    }
}

/// `flatness` of `points-on-curve`: the larger of the squared deviations of the control points.
fn flatness(p: [Point; 4]) -> f64 {
    let square = |v: f64| v * v;
    let mut ux = square(3.0 * p[1].x - 2.0 * p[0].x - p[3].x);
    let mut uy = square(3.0 * p[1].y - 2.0 * p[0].y - p[3].y);
    let vx = square(3.0 * p[2].x - 2.0 * p[3].x - p[0].x);
    let vy = square(3.0 * p[2].y - 2.0 * p[3].y - p[0].y);
    if ux < vx {
        ux = vx;
    }
    if uy < vy {
        uy = vy;
    }
    ux + uy
}

/// `simplifyPoints` of `points-on-curve`: Ramer-Douglas-Peucker with `epsilon`, from `start` to `end`.
fn simplify_points(points: &[Point], start: usize, end: usize, epsilon: f64, out: &mut Vec<Point>) {
    let s = points[start];
    let e = points[end - 1];
    let mut max_distance_sq = 0.0;
    let mut max_index = 1;
    for (i, point) in points.iter().enumerate().take(end - 1).skip(start + 1) {
        let distance_sq = distance_to_segment_sq(*point, s, e);
        if distance_sq > max_distance_sq {
            max_distance_sq = distance_sq;
            max_index = i;
        }
    }
    if max_distance_sq.sqrt() > epsilon {
        simplify_points(points, start, max_index + 1, epsilon, out);
        simplify_points(points, max_index, end, epsilon, out);
    } else {
        if out.is_empty() {
            out.push(s);
        }
        out.push(e);
    }
}

/// The squared distance from `p` to the segment `vw` (`points-on-curve`'s `distanceToSegmentSq`).
fn distance_to_segment_sq(p: Point, v: Point, w: Point) -> f64 {
    let l2 = (v.x - w.x).powi(2) + (v.y - w.y).powi(2);
    if l2 == 0.0 {
        return (p.x - v.x).powi(2) + (p.y - v.y).powi(2);
    }
    let t = (((p.x - v.x) * (w.x - v.x) + (p.y - v.y) * (w.y - v.y)) / l2).clamp(0.0, 1.0);
    let projected = Point::new(v.x + (w.x - v.x) * t, v.y + (w.y - v.y) * t);
    (p.x - projected.x).powi(2) + (p.y - projected.y).powi(2)
}

/// `pointsOnPath` (`points-on-path`): the points of each sub-path of a path, its curves subdivided at
/// tolerance 1 and every sub-path simplified at distance 0.5, as rough.js samples a path for a fill
/// with several sub-paths (`generator.js`, `path`). Empty sub-paths are dropped.
fn points_on_path(segments: &[Segment]) -> Vec<Vec<Point>> {
    const TOLERANCE: f64 = 1.0;
    const DISTANCE: f64 = 0.5;
    let mut sets: Vec<Vec<Point>> = Vec::new();
    let mut current: Vec<Point> = Vec::new();
    let mut start = Point::default();
    let mut pending: Vec<Point> = Vec::new();

    fn append_pending_curve(pending: &mut Vec<Point>, current: &mut Vec<Point>) {
        if pending.len() >= 4 {
            let mut out = Vec::new();
            let segments = (pending.len() - 1) / 3;
            for i in 0..segments {
                let offset = i * 3;
                points_on_cubic(
                    [
                        pending[offset],
                        pending[offset + 1],
                        pending[offset + 2],
                        pending[offset + 3],
                    ],
                    TOLERANCE,
                    &mut out,
                );
            }
            current.extend(out);
        }
        pending.clear();
    }

    for segment in segments {
        match *segment {
            Segment::Move(point) => {
                append_pending_curve(&mut pending, &mut current);
                if !current.is_empty() {
                    sets.push(std::mem::take(&mut current));
                }
                start = point;
                current.push(point);
            }
            Segment::Line(point) => {
                append_pending_curve(&mut pending, &mut current);
                current.push(point);
            }
            Segment::Curve(c1, c2, end) => {
                if pending.is_empty() {
                    pending.push(current.last().copied().unwrap_or(start));
                }
                pending.push(c1);
                pending.push(c2);
                pending.push(end);
            }
            Segment::Close => {
                append_pending_curve(&mut pending, &mut current);
                current.push(start);
            }
        }
    }
    append_pending_curve(&mut pending, &mut current);
    if !current.is_empty() {
        sets.push(current);
    }

    sets.into_iter()
        .filter_map(|set| {
            let mut out = Vec::new();
            simplify_points(&set, 0, set.len(), DISTANCE, &mut out);
            (!out.is_empty()).then_some(out)
        })
        .collect()
}

/// The outline of a polygon, as `rc.polygon` draws it: the fill is the points as an open polyline
/// (`solidFillPolygon`), and the stroke has a double line along every side, the closing one too
/// (`linearPath` with `close`). A polygon of two points is a single double line.
#[must_use]
pub(super) fn polygon(points: &[Point], paint: Paint) -> Outline {
    let fill = paint.fill.then(|| solid_fill_polygon(&[points.to_vec()]));
    let stroke = paint.stroke.then(|| {
        let mut ops = Vec::new();
        linear_path_ops(points, true, 2, &mut ops);
        ops
    });
    Outline {
        fill,
        stroke,
        bbox: points_box(points.iter().copied()),
    }
}

/// The outline of a rectangle, as `rc.rectangle` draws it: a polygon of its four corners.
#[must_use]
pub(super) fn rectangle(x: f64, y: f64, width: f64, height: f64, paint: Paint) -> Outline {
    polygon(
        &[
            Point::new(x, y),
            Point::new(x + width, y),
            Point::new(x + width, y + height),
            Point::new(x, y + height),
        ],
        paint,
    )
}

/// The outline of a line, as `rc.line` draws it: a double line, with no fill.
#[must_use]
pub(super) fn line(from: Point, to: Point, paint: Paint) -> Outline {
    let mut ops = Vec::new();
    double_line(from, to, 2, &mut ops);
    Outline {
        fill: None,
        stroke: paint.stroke.then_some(ops),
        bbox: points_box([from, to]),
    }
}

/// The outline of an ellipse, as `rc.ellipse` and `rc.circle` draw it: the same curve for the fill
/// and the stroke, with one pass (roughness 0 draws no second pass).
#[must_use]
pub(super) fn ellipse(cx: f64, cy: f64, width: f64, height: f64, paint: Paint) -> Outline {
    let (ops, bounds) = ellipse_ops(cx, cy, width, height);
    Outline {
        fill: paint.fill.then(|| ops.clone()),
        stroke: paint.stroke.then_some(ops),
        bbox: points_box(bounds),
    }
}

/// The outline of a path, as `rc.path` draws it (`generator.js`, `path`). The stroke is every segment
/// drawn in two passes. The fill is the path's single pass when it has one sub-path, which keeps the
/// closing side of a closed path; with several sub-paths it is their sampled points as open polylines
/// (`solidFillPolygon` of `pointsOnPath`), and a sub-path of two points or fewer draws nothing.
#[must_use]
pub(super) fn path(segments: &[Segment], paint: Paint) -> Outline {
    let fill = paint.fill.then(|| {
        let sets = points_on_path(segments);
        if sets.len() == 1 {
            merged_fill(segments)
        } else {
            solid_fill_polygon(&sets)
        }
    });
    Outline {
        fill,
        stroke: paint.stroke.then(|| svg_path_ops(segments, 2)),
        bbox: segments_box(segments),
    }
}

/// The outline of a polygon that rough.js draws as a path with a closed fill: the `rc.path` of
/// `createPathFromPoints`. Its fill is one closed pass.
#[must_use]
pub(super) fn closed_polygon(points: &[Point], paint: Paint) -> Outline {
    path(&points_path(points), paint)
}

/// The SVG of a drawing whose colours are in the style sheet: its fill path, then its stroke path, each
/// with the node's style (`toPaths` in `generator.js`).
#[must_use]
pub(super) fn paths(outline: &Outline, style: Option<&str>, stroke_width: &str) -> String {
    attributed(
        outline,
        &Attrs {
            style,
            stroke_width,
            ..Attrs::default()
        },
    )
}

/// The SVG of a drawing with its colours as presentation attributes, as the ER, class and state
/// outlines write them.
#[must_use]
pub(super) fn attributed(outline: &Outline, attrs: &Attrs<'_>) -> String {
    let style = attrs
        .style
        .map_or_else(String::new, |style| attribute("style", style));
    let mut out = String::new();
    if let Some(fill) = &outline.fill {
        let colour = attrs
            .fill
            .map(|fill| {
                let rule = if fill.evenodd {
                    " fill-rule=\"evenodd\""
                } else {
                    ""
                };
                format!(" fill=\"{}\"{rule}", escape(fill.colour))
            })
            .unwrap_or_default();
        out.push_str(&format!(
            "<path d=\"{}\" stroke=\"none\" stroke-width=\"0\"{colour}{style}/>",
            data(fill),
        ));
    }
    if let Some(stroke) = &outline.stroke {
        let colour = attrs
            .stroke
            .map(|stroke| format!(" stroke=\"{}\"", escape(stroke)))
            .unwrap_or_default();
        out.push_str(&format!(
            "<path d=\"{}\" fill=\"none\"{colour} stroke-width=\"{}\" stroke-dasharray=\"0 0\"{style}/>",
            data(stroke),
            escape(attrs.stroke_width),
        ));
    }
    out
}

/// The data of an op list, as `opsToPath` writes it.
#[must_use]
pub(super) fn data(ops: &[Op]) -> String {
    let mut out = String::new();
    for op in ops {
        if !out.is_empty() {
            out.push(' ');
        }
        match *op {
            Op::Move(point) => out.push_str(&format!("M{} {}", number(point.x), number(point.y))),
            Op::Line(point) => out.push_str(&format!("L{} {}", number(point.x), number(point.y))),
            Op::Curve(c1, c2, end) => out.push_str(&format!(
                "C{} {}, {} {}, {} {}",
                number(c1.x),
                number(c1.y),
                number(c2.x),
                number(c2.y),
                number(end.x),
                number(end.y),
            )),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> [Point; 4] {
        [
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 4.0),
            Point::new(0.0, 4.0),
        ]
    }

    #[test]
    fn a_stroked_polygon_draws_every_side_twice_the_closing_one_too() {
        let outline = polygon(&square(), Paint::STROKE);
        let stroke = outline.stroke.expect("a stroke");
        // Four sides, two passes each: eight moves, and the closing side starts the last pass.
        let moves = stroke.iter().filter(|op| matches!(op, Op::Move(_))).count();
        assert_eq!(moves, 8);
        assert_eq!(
            stroke.last(),
            Some(&Op::Curve(
                Point::new(0.0 + (0.0 - 0.0) * DIVERGE, 4.0 + (0.0 - 4.0) * DIVERGE),
                Point::new(0.0, 4.0 + 2.0 * (0.0 - 4.0) * DIVERGE),
                Point::new(0.0, 0.0),
            ))
        );
    }

    #[test]
    fn a_polygon_fill_is_open_three_sides_of_a_rectangle() {
        let outline = rectangle(0.0, 0.0, 10.0, 4.0, Paint::BOTH);
        let fill = outline.fill.expect("a fill");
        // `solidFillPolygon`: one move and three lines, the closing side is not filled.
        assert_eq!(fill.len(), 4);
        assert!(matches!(fill[0], Op::Move(_)));
        assert_eq!(
            fill.iter().filter(|op| matches!(op, Op::Line(_))).count(),
            3
        );
    }

    #[test]
    fn a_path_fill_with_one_sub_path_is_one_closed_pass() {
        let outline = closed_polygon(&square(), Paint::BOTH);
        let fill = outline.fill.expect("a fill");
        // The merged fill has one move, and a line to each point and back to the first.
        assert_eq!(
            fill.iter().filter(|op| matches!(op, Op::Move(_))).count(),
            1
        );
        assert_eq!(fill.len(), 5);
        assert_eq!(outline.stroke.expect("a stroke").len(), 16);
    }

    #[test]
    fn a_polygon_of_two_points_is_one_double_line() {
        let outline = polygon(&square()[..2], Paint::STROKE);
        assert_eq!(outline.stroke.expect("a stroke").len(), 4);
    }

    #[test]
    fn a_drawing_without_a_stroke_has_no_stroke_path() {
        let outline = ellipse(
            0.0,
            0.0,
            2.0,
            2.0,
            Paint {
                fill: true,
                stroke: false,
            },
        );
        assert!(outline.stroke.is_none());
        assert!(outline.fill.is_some());
    }

    #[test]
    fn a_circle_of_diameter_fourteen_overshoots_its_diameter_by_the_reference_amount() {
        // The rough circle of `filledCircle` is 14.0133 wide and exactly 14 high (shapes-core.md
        // R5, computed with the same cubic bounds).
        let outline = ellipse(0.0, 0.0, 14.0, 14.0, Paint::BOTH);
        assert!(
            (outline.bbox.width - 14.0133).abs() < 5e-5,
            "{:?}",
            outline.bbox
        );
        assert!(
            (outline.bbox.height - 14.0).abs() < 1e-9,
            "{:?}",
            outline.bbox
        );
    }

    #[test]
    fn a_circle_of_diameter_five_overshoots_by_the_reference_amount() {
        let outline = ellipse(0.0, 0.0, 5.0, 5.0, Paint::BOTH);
        assert!(
            (outline.bbox.width - 5.0047).abs() < 5e-5,
            "{:?}",
            outline.bbox
        );
    }

    #[test]
    fn a_polygon_outline_has_the_box_of_its_points() {
        let outline = polygon(
            &[
                Point::new(0.0, 0.0),
                Point::new(70.0, 0.0),
                Point::new(70.0, -10.0),
            ],
            Paint::BOTH,
        );
        assert_eq!(outline.bbox, BBox::new(0.0, -10.0, 70.0, 10.0));
    }

    #[test]
    fn a_solid_rectangle_fills_three_sides_and_strokes_each_side_twice() {
        // The note of `a-state-with-a-note`: its fill is `M L L L` (393.2 long, not 486.4), and each of
        // its four sides is drawn twice (`renderer.js` `_doubleLine`).
        let points = [
            Point::new(-75.0, -46.6),
            Point::new(75.0, -46.6),
            Point::new(75.0, 46.6),
            Point::new(-75.0, 46.6),
        ];
        let outline = polygon(&points, Paint::BOTH);
        let fill = outline.fill.expect("a solid fill");
        assert_eq!(fill.len(), 4, "{fill:?}");
        assert_eq!(outline.stroke.expect("a stroke").len(), 16);
    }

    #[test]
    fn a_rounded_rectangle_path_has_its_corners_as_quarter_circles() {
        let segments = rounded_rectangle_path(0.0, 0.0, 20.0, 10.0, 2.0);
        let outline = path(&segments, Paint::STROKE);
        assert!(
            (outline.bbox.width - 20.0).abs() < 1e-9,
            "{:?}",
            outline.bbox
        );
        assert!(
            (outline.bbox.height - 10.0).abs() < 1e-9,
            "{:?}",
            outline.bbox
        );
    }
}
