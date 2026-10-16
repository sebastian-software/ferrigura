//! The outlines that rough.js draws at roughness 0, as deterministic paths with the same geometry.
//!
//! At roughness 0 rough.js draws every line through its control points on the segment, so a
//! polygon is its own outline and its box is the box of its points. A circle is not: rough.js
//! samples the ellipse at a quarter of its step and joins the samples with cubic curves whose
//! control points lie outside the ellipse, so the curve overshoots the diameter (14 becomes
//! 14.0133 wide). The box is what the layout uses, so the curve is reproduced here
//! (`renderer.js` `_computeEllipsePoints` with `coreOnly`, `_curve`, and `generateEllipseParams`).
//! The `handDrawnSeed` and the random offsets play no part at roughness 0. See
//! `docs-port/flowchart/render-notes/shapes-core.md` §2.3.

use std::f64::consts::PI;

use super::util::attribute;
use crate::svg::{escape, number};
use crate::unified::geom::{BBox, Point};

/// rough.js's default `curveStepCount`.
const CURVE_STEP_COUNT: f64 = 9.0;

/// An outline: its path data and its box, in the coordinates of the points.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Outline {
    /// The path data of the stroke.
    pub d: String,
    /// The path data of the solid fill, when it differs from the stroke's (see [`solid_polygon`]).
    pub fill: Option<String>,
    /// The box of the path's geometry.
    pub bbox: BBox,
}

/// The outline of a polygon: rough.js draws each side as a line, so the path is the points in
/// order, closed. The box is the box of the points.
#[must_use]
pub(super) fn polygon(points: &[Point]) -> Outline {
    Outline {
        d: super::util::create_path_from_points(points),
        fill: None,
        bbox: BBox::from_points(points.iter().copied()).unwrap_or_default(),
    }
}

/// The outline of a polygon that rough.js draws solid at roughness 0: the state diagram's forks,
/// joins, choices and notes (`rc.polygon` and `rc.rectangle` with `fillStyle: 'solid'`).
///
/// The fill is the points as lines, from the first to the last, without the side that closes them:
/// rough.js's solid fill is `M p0 L p1 … L pn` (`generator.js`, `solidFillPolygon`), so a rectangle's
/// fill is three of its sides. The stroke draws every side, the closing one too, twice: each side is
/// a `move` and a `bcurveTo` from `_doubleLine` (`renderer.js` 237-245), so the path has two passes
/// per side, in the order of the sides. At roughness 0 the control points of each pass lie on its
/// side (`_line`: the offsets are multiplied by the roughness), so a pass is the side whatever its
/// random divergence point, and lengths and points along the path are the sides'. The passes are
/// written as lines. A closed path, as the choice's, is filled with its closing side too: `closed`
/// gives that fill.
pub(super) fn solid_polygon(points: &[Point], closed: bool) -> Outline {
    let mut fill = String::new();
    let mut stroke = String::new();
    for (index, point) in points.iter().enumerate() {
        let command = if index == 0 { "M" } else { "L" };
        fill.push_str(&format!(
            "{command}{},{} ",
            number(point.x),
            number(point.y)
        ));
    }
    if closed {
        fill.push('Z');
    }
    for (index, start) in points.iter().enumerate() {
        let end = points[(index + 1) % points.len()];
        for _ in 0..2 {
            stroke.push_str(&format!(
                "M{},{} L{},{} ",
                number(start.x),
                number(start.y),
                number(end.x),
                number(end.y),
            ));
        }
    }
    Outline {
        d: stroke.trim_end().to_owned(),
        fill: Some(fill.trim_end().to_owned()),
        bbox: BBox::from_points(points.iter().copied()).unwrap_or_default(),
    }
}

/// The outline of a rectangle, as `rc.rectangle` draws it at roughness 0: its four corners.
#[must_use]
pub(super) fn rectangle(x: f64, y: f64, width: f64, height: f64) -> Outline {
    polygon(&[
        Point::new(x, y),
        Point::new(x + width, y),
        Point::new(x + width, y + height),
        Point::new(x, y + height),
    ])
}

/// The outline of a rectangle with rounded corners, from `createRoundedRectPathD`
/// (`shapes/roundedRectPath.ts`). Its arcs are quarter circles that end on the sides of the
/// rectangle, so its box is the rectangle's.
#[must_use]
pub(super) fn rounded_rectangle(x: f64, y: f64, width: f64, height: f64, radius: f64) -> Outline {
    let r = number(radius);
    let d = format!(
        "M{},{} H{} A{r},{r} 0 0 1 {},{} V{} A{r},{r} 0 0 1 {},{} H{} A{r},{r} 0 0 1 {},{} V{} A{r},{r} 0 0 1 {},{} Z",
        number(x + radius),
        number(y),
        number(x + width - radius),
        number(x + width),
        number(y + radius),
        number(y + height - radius),
        number(x + width - radius),
        number(y + height),
        number(x + radius),
        number(x),
        number(y + height - radius),
        number(y + radius),
        number(x + radius),
        number(y),
    );
    Outline {
        fill: None,
        d,
        bbox: BBox::new(x, y, width, height),
    }
}

/// The outline of an ellipse with the given diameters, centred on `(cx, cy)`, as
/// `rc.ellipse(cx, cy, width, height)` draws it at roughness 0.
#[must_use]
pub(super) fn ellipse(cx: f64, cy: f64, width: f64, height: f64) -> Outline {
    let rx = (width / 2.0).abs();
    let ry = (height / 2.0).abs();
    // `generateEllipseParams`: the number of steps grows with the size.
    let psq = (2.0 * PI * (((width / 2.0).powi(2) + (height / 2.0).powi(2)) / 2.0).sqrt()).sqrt();
    let steps = CURVE_STEP_COUNT
        .max(CURVE_STEP_COUNT / 200.0_f64.sqrt() * psq)
        .ceil();
    let increment = 2.0 * PI / steps;
    // `_computeEllipsePoints` at roughness 0 samples a quarter of the step, from one step before
    // the start to one step past the end.
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

    // `_curve` with tightness 0: a Catmull-Rom spline through the samples, from the second sample
    // to the one before the last.
    let mut d = format!("M{},{}", number(points[1].x), number(points[1].y));
    let mut bounds = vec![points[1]];
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
        d.push_str(&format!(
            " C{},{} {},{} {},{}",
            number(control_1.x),
            number(control_1.y),
            number(control_2.x),
            number(control_2.y),
            number(next.x),
            number(next.y),
        ));
        bounds.extend(cubic_extremes(here, control_1, control_2, next));
    }
    Outline {
        fill: None,
        d,
        bbox: BBox::from_points(bounds).unwrap_or_default(),
    }
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

/// The two paths that rough.js writes for one outline: the fill, then the stroke. The fill path
/// has no stroke; the stroke path has no fill. The colours come from the style sheet
/// (`docs-port/decisions/0007`), so neither path carries one. `style` is written on both paths,
/// as the reference does for the node's declarations. `stroke_visible` is false for an outline
/// with `stroke: none`.
#[must_use]
pub(super) fn paths(
    outline: &Outline,
    style: Option<&str>,
    stroke_width: &str,
    stroke_visible: bool,
) -> String {
    let style = style.map_or_else(String::new, |style| attribute("style", style));
    let stroke = if stroke_visible {
        ""
    } else {
        " stroke=\"none\""
    };
    let fill = outline.fill.as_deref().unwrap_or(&outline.d);
    format!(
        "<path d=\"{fill}\" stroke=\"none\" stroke-width=\"0\"{style}/><path d=\"{d}\" fill=\"none\"{stroke} stroke-width=\"{stroke_width}\" stroke-dasharray=\"0 0\"{style}/>",
        d = outline.d,
    )
}

/// The stroke alone, for an outline with `fill: none` (`curlyBraces`): rough.js writes no fill path
/// for it, so only the stroke path is drawn.
#[must_use]
pub(super) fn stroke(outline: &Outline, style: Option<&str>, stroke_width: &str) -> String {
    let style = style.map_or_else(String::new, |style| attribute("style", style));
    format!(
        "<path d=\"{}\" fill=\"none\" stroke-width=\"{stroke_width}\" stroke-dasharray=\"0 0\"{style}/>",
        outline.d,
    )
}

/// The fill of a polygon: its colour, and whether it carries `fill-rule="evenodd"`, which
/// `rc.polygon` gives and `rc.rectangle` does not.
#[derive(Debug, Clone, Copy)]
pub(super) struct Fill<'a> {
    /// The fill colour.
    pub colour: &'a str,
    /// Whether the fill's rule is `evenodd`.
    pub evenodd: bool,
}

/// A closed polygon as rough.js draws it at roughness 0 with `fillStyle: solid`, as `rc.rectangle`
/// and `rc.polygon` write it (the ER entity's `erBox.ts`, the `neo` look):
///
/// - the fill is the points as an open polyline, `M p0 L p1 ... L pn`, with no closing side;
/// - the stroke has two passes along every side, the closing side included (`renderer.js`
///   `_doubleLine`, `disableMultiStroke` false). Each pass is a cubic whose control points lie on
///   the side, so both passes trace it and the stroke's length is twice the perimeter. The second
///   pass's control points differ from the first's in the reference (`divergePoint` is random);
///   they stay on the side, so the geometry does not.
///
/// The fill path is written only when the options give a fill: roughjs writes nothing for an
/// `undefined` fill, as the rows of an ER entity draw without one in the redux themes.
/// `path_style` is the `style` of every path, when the node has style declarations.
#[must_use]
pub(super) fn double_stroked_polygon(
    points: &[Point],
    fill: Option<Fill<'_>>,
    stroke: &str,
    stroke_width: &str,
    path_style: Option<&str>,
) -> String {
    let style = path_style.map_or_else(String::new, |style| attribute("style", style));
    let mut out = String::new();
    if let Some(fill) = fill {
        let rule = if fill.evenodd {
            " fill-rule=\"evenodd\""
        } else {
            ""
        };
        let d = points
            .iter()
            .enumerate()
            .map(|(index, point)| {
                format!(
                    "{}{},{}",
                    if index == 0 { "M" } else { " L" },
                    number(point.x),
                    number(point.y)
                )
            })
            .collect::<String>();
        out.push_str(&format!(
            "<path d=\"{d}\" stroke=\"none\" stroke-width=\"0\" fill=\"{}\"{rule}{style}/>",
            escape(fill.colour),
        ));
    }
    let mut d = String::new();
    for (index, start) in points.iter().enumerate() {
        let end = points[(index + 1) % points.len()];
        let first = Point::new(
            start.x + (end.x - start.x) / 3.0,
            start.y + (end.y - start.y) / 3.0,
        );
        let second = Point::new(
            start.x + 2.0 * (end.x - start.x) / 3.0,
            start.y + 2.0 * (end.y - start.y) / 3.0,
        );
        for _ in 0..2 {
            d.push_str(&format!(
                "M{},{} C{},{} {},{} {},{} ",
                number(start.x),
                number(start.y),
                number(first.x),
                number(first.y),
                number(second.x),
                number(second.y),
                number(end.x),
                number(end.y),
            ));
        }
    }
    out.push_str(&format!(
        "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" stroke-dasharray=\"0 0\"{style}/>",
        d.trim_end(),
        escape(stroke),
        escape(stroke_width),
    ));
    out
}

/// One side of a rough.js line at roughness 0, drawn twice (`_doubleLine`): first as a line, then as
/// a bowed curve whose control points lie on the segment. The reference's random divergence point
/// moves no point that a sample can see (the curve is monotone along the segment, so its length and
/// box are the segment's), so a fixed fraction of the length stands for it.
pub(super) fn double_side(from: Point, to: Point) -> String {
    const DIVERGE: f64 = 0.3;
    let c1 = Point::new(
        from.x + (to.x - from.x) * DIVERGE,
        from.y + (to.y - from.y) * DIVERGE,
    );
    let c2 = Point::new(
        from.x + 2.0 * (to.x - from.x) * DIVERGE,
        from.y + 2.0 * (to.y - from.y) * DIVERGE,
    );
    format!(
        "M{} {} L{} {} M{} {} C{} {} {} {} {} {}",
        number(from.x),
        number(from.y),
        number(to.x),
        number(to.y),
        number(from.x),
        number(from.y),
        number(c1.x),
        number(c1.y),
        number(c2.x),
        number(c2.y),
        number(to.x),
        number(to.y),
    )
}

/// `rc.rectangle` at roughness 0 with a solid fill: the fill is the three sides from the first corner,
/// open (`solidFillPolygon`), and the stroke is each of the four sides drawn twice (`linearPath` with
/// `_doubleLine`), so its length is twice the perimeter. `style` is the style of both paths.
pub(super) fn rectangle_paths(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    style: Option<&str>,
    stroke_width: &str,
) -> String {
    let corners = [
        Point::new(x, y),
        Point::new(x + width, y),
        Point::new(x + width, y + height),
        Point::new(x, y + height),
    ];
    let fill: String = corners
        .iter()
        .enumerate()
        .map(|(index, corner)| {
            format!(
                "{}{} {}",
                if index == 0 { "M" } else { "L" },
                number(corner.x),
                number(corner.y)
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let stroke: String = (0..corners.len())
        .map(|index| double_side(corners[index], corners[(index + 1) % corners.len()]))
        .collect::<Vec<_>>()
        .join(" ");
    let style = style.map_or_else(String::new, |style| format!(" style=\"{}\"", escape(style)));
    format!(
        "<path d=\"{fill}\" stroke=\"none\" stroke-width=\"0\"{style}/><path d=\"{stroke}\" fill=\"none\" stroke-width=\"{}\" stroke-dasharray=\"0 0\"{style}/>",
        escape(stroke_width),
    )
}

/// `rc.line` at roughness 0: a stroke path of the segment, drawn twice, with no fill.
pub(super) fn line(from: Point, to: Point, style: Option<&str>, stroke_width: &str) -> String {
    let outline = Outline {
        d: double_side(from, to),
        bbox: BBox::new(from.x, from.y, to.x - from.x, to.y - from.y),
        fill: None,
    };
    stroke(&outline, style, stroke_width)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_double_stroked_rectangle_has_a_fill_polyline_and_two_passes_a_side() {
        let points = [
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 4.0),
            Point::new(0.0, 4.0),
        ];
        let svg = double_stroked_polygon(
            &points,
            Some(Fill {
                colour: "#ffffff",
                evenodd: false,
            }),
            "#000",
            "1.3",
            None,
        );
        // The fill is open: four points and no closing side.
        assert!(
            svg.starts_with("<path d=\"M0,0 L10,0 L10,4 L0,4\" stroke=\"none\""),
            "{svg}"
        );
        assert!(!svg.contains("fill-rule"), "{svg}");
        // Four sides, two passes each: eight `M` moves, and the closing side starts the last pass.
        let stroke = svg.split("stroke-width=\"0\"").nth(1).expect("the stroke");
        assert_eq!(stroke.matches('M').count(), 8, "{svg}");
        assert!(stroke.contains("M0,4 C0,"), "{svg}");
    }

    #[test]
    fn a_polygon_without_a_fill_writes_only_the_stroke() {
        let points = [
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(1.0, 1.0),
        ];
        let svg = double_stroked_polygon(&points, None, "#000", "1.3", Some("fill: red"));
        assert_eq!(svg.matches("<path ").count(), 1, "{svg}");
        assert!(svg.contains("style=\"fill: red\""), "{svg}");
    }

    #[test]
    fn a_circle_of_diameter_fourteen_overshoots_its_diameter_by_the_reference_amount() {
        // The rough circle of `filledCircle` is 14.0133 wide and exactly 14 high (shapes-core.md
        // R5, computed with the same cubic bounds).
        let outline = ellipse(0.0, 0.0, 14.0, 14.0);
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
        let outline = ellipse(0.0, 0.0, 5.0, 5.0);
        assert!(
            (outline.bbox.width - 5.0047).abs() < 5e-5,
            "{:?}",
            outline.bbox
        );
    }

    #[test]
    fn a_polygon_outline_has_the_box_of_its_points() {
        let outline = polygon(&[
            Point::new(0.0, 0.0),
            Point::new(70.0, 0.0),
            Point::new(70.0, -10.0),
        ]);
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
        let outline = solid_polygon(&points, false);
        let fill = outline.fill.expect("a solid fill");
        assert_eq!(fill.matches('L').count(), 3, "{fill}");
        assert!(!fill.contains('Z'), "{fill}");
        assert_eq!(outline.d.matches('M').count(), 8, "{}", outline.d);
        assert!(
            outline
                .d
                .starts_with("M-75,-46.6 L75,-46.6 M-75,-46.6 L75,-46.6")
        );
    }

    #[test]
    fn a_closed_solid_polygon_is_filled_with_its_closing_side() {
        let points = [
            Point::new(0.0, 14.0),
            Point::new(14.0, 0.0),
            Point::new(0.0, -14.0),
        ];
        let outline = solid_polygon(&points, true);
        assert!(outline.fill.expect("a solid fill").ends_with('Z'));
    }
}
