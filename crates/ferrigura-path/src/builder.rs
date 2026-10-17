//! Blink's `SVGPathBuilder` (`core/svg/svg_path_builder.cc`): each segment of the byte stream becomes the
//! `SkPath` calls of `EmitSegment` (line 110-186). It makes the relative points absolute, reflects the
//! control points of the smooth curves, and turns the horizontal and vertical lines into lines. An arc
//! is `SkPathBuilder::arcTo` (`skia.rs`): conics, not cubics.

use crate::segments::{Kind, Point, Segment};
use crate::skia;

/// One `SkPath` call that the bounds read.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Verb {
    Move(Point),
    Line(Point),
    Quad(Point, Point),
    Cubic(Point, Point, Point),
    /// A conic: its control point, its end and its weight.
    Conic(Point, Point, f32),
    Close,
}

/// The state of `SVGPathBuilder`, and the last point of the `SkPath` (`fPts.back()`).
#[derive(Default)]
struct Builder {
    current: Point,
    subpath: Point,
    last_control: Point,
    last_kind: Option<(Kind, bool)>,
    last_point: Point,
    verbs: Vec<Verb>,
}

/// `Vector2dF` addition of a point and a vector (`PointF + Vector2dF`): two float additions.
fn add(a: Point, b: Point) -> Point {
    Point {
        x: a.x + b.x,
        y: a.y + b.y,
    }
}

impl Builder {
    /// `EmitMoveTo`.
    fn move_to(&mut self, p: Point) {
        self.verbs.push(Verb::Move(p));
        self.subpath = p;
        self.current = p;
        self.last_point = p;
    }

    /// `EmitLineTo`.
    fn line_to(&mut self, p: Point) {
        self.verbs.push(Verb::Line(p));
        self.current = p;
        self.last_point = p;
    }

    /// `EmitQuadTo`: the quadratic's control point is the one the smooth quadratics reflect.
    fn quad_to(&mut self, c0: Point, p: Point) {
        self.verbs.push(Verb::Quad(c0, p));
        self.last_control = c0;
        self.current = p;
        self.last_point = p;
    }

    /// `EmitCubicTo`: the second control point is the one the smooth cubics reflect.
    fn cubic_to(&mut self, c0: Point, c1: Point, p: Point) {
        self.verbs.push(Verb::Cubic(c0, c1, p));
        self.last_control = c1;
        self.current = p;
        self.last_point = p;
    }

    /// `SmoothControl`: the reflection of the last control point through the current point, when the
    /// last segment was of the same family; the current point otherwise.
    fn smooth_control(&self, compatible: bool) -> Point {
        let mut control = self.current;
        if compatible {
            let delta = Point {
                x: self.current.x - self.last_control.x,
                y: self.current.y - self.last_control.y,
            };
            control = add(control, delta);
        }
        control
    }

    /// `EmitClose`: the path closes, and the current point is the start of the sub-path.
    fn close(&mut self) {
        self.verbs.push(Verb::Close);
        self.current = self.subpath;
        self.last_point = self.subpath;
    }

    /// `EmitArcTo`: `SkPathBuilder::arcTo`, from the last point of the path (its `fPts.back()`).
    fn arc_to(&mut self, p: Point, rx: f32, ry: f32, angle: f32, large: bool, sweep: bool) {
        let from = self.last_point;
        let conics = skia::arc_to(from, p, rx, ry, angle, large, sweep);
        self.verbs.extend(conics);
        self.current = p;
        self.last_point = p;
    }

    /// `EmitSegment` of `SVGPathBuilder`.
    fn emit(&mut self, segment: &Segment) {
        let Some(kind) = segment.kind else {
            return;
        };
        let rel = segment.rel;
        let current = self.current;
        // The absolute point of the segment: the target, plus the current point when it is relative.
        let absolute = |p: Point| if rel { add(current, p) } else { p };
        match kind {
            Kind::Close => self.close(),
            Kind::Move => self.move_to(absolute(segment.target)),
            Kind::Line => self.line_to(absolute(segment.target)),
            Kind::Horizontal => {
                // `current + Vector2dF(x, 0)` when relative; `(x, current.y)` when absolute.
                let p = if rel {
                    add(
                        current,
                        Point {
                            x: segment.target.x,
                            y: 0.0,
                        },
                    )
                } else {
                    Point {
                        x: segment.target.x,
                        y: current.y,
                    }
                };
                self.line_to(p);
            }
            Kind::Vertical => {
                let p = if rel {
                    add(
                        current,
                        Point {
                            x: 0.0,
                            y: segment.target.y,
                        },
                    )
                } else {
                    Point {
                        x: current.x,
                        y: segment.target.y,
                    }
                };
                self.line_to(p);
            }
            Kind::Quad => self.quad_to(absolute(segment.p1), absolute(segment.target)),
            Kind::SmoothQuad => {
                let last_was_quadratic =
                    matches!(self.last_kind, Some((Kind::Quad | Kind::SmoothQuad, _)));
                let control = self.smooth_control(last_was_quadratic);
                self.quad_to(control, absolute(segment.target));
            }
            Kind::Cubic => self.cubic_to(
                absolute(segment.p1),
                absolute(segment.p2),
                absolute(segment.target),
            ),
            Kind::SmoothCubic => {
                let last_was_cubic =
                    matches!(self.last_kind, Some((Kind::Cubic | Kind::SmoothCubic, _)));
                let control = self.smooth_control(last_was_cubic);
                self.cubic_to(control, absolute(segment.p2), absolute(segment.target));
            }
            Kind::Arc => self.arc_to(
                absolute(segment.target),
                segment.rx,
                segment.ry,
                segment.angle,
                segment.large,
                segment.sweep,
            ),
        }
        self.last_kind = Some((kind, rel));
    }
}

/// The `SkPath` calls of the segments, as `SVGPathBuilder` makes them.
pub(crate) fn build(segments: &[Segment]) -> Vec<Verb> {
    let mut builder = Builder::default();
    for segment in segments {
        builder.emit(segment);
    }
    builder.verbs
}
