//! The geometry of edge paths: the `d` strings that `insertEdge` writes and the boxes that
//! `getBBox` gives for them.
//!
//! Three writers meet here, each with the number text of its reference:
//!
//! - the d3 curves (`linear`, `basis`, `step`, ...) write through `d3-path` with three decimals:
//!   `Math.round(v * 1000) / 1000`, then JavaScript's number text (`curve`, `d3_round`);
//! - `generateRoundedPath` writes full-precision JavaScript number text (`generate_rounded_path`);
//! - the line hops write three decimals with spaces between the commands (`line_jumps`).
//!
//! The curves are ports of the state machines in `d3-shape` 3.2.0 (`src/curve/*.js`), kept
//! statement by statement, because their state carries over from one point to the next. See
//! `docs-port/flowchart/render-notes/edges.md` §3 and §4.

use std::f64::consts::PI;

use super::geom::{BBox, Point};

pub use crate::common::d3::number::{d3_number, js_number, js_round};

/// The radius of the rounded corners: `generateRoundedPath(…, 5)` in `insertEdge`.
pub const ROUNDED_RADIUS: f64 = 5.0;

/// `markerOffsets` in `utils/lineWithOffset.ts`: how far a path end is pulled back for its marker.
/// A marker without an entry (`arrow_cross`, `arrow_circle`, `none`) does not shorten the path.
#[must_use]
pub fn marker_offset(arrow: &str) -> Option<f64> {
    match arrow {
        "aggregation" | "extension" | "composition" => Some(17.25),
        "dependency" => Some(6.0),
        "lollipop" => Some(13.5),
        "arrow_point" => Some(4.0),
        "arrow_barb" => Some(0.0),
        "arrow_barb_neo" => Some(5.5),
        _ => None,
    }
}

/// `markerOffsets2` in `utils/lineWithOffset.ts`: the stroke hidden under each marker by the neo
/// dash array.
#[must_use]
pub fn marker_offset2(arrow: &str) -> Option<f64> {
    match arrow {
        "arrow_point" => Some(4.0),
        "arrow_cross" | "arrow_circle" => Some(12.5),
        _ => None,
    }
}

/// `Math.min` over three numbers: a NaN in any of them gives NaN.
fn js_min3(a: f64, b: f64, c: f64) -> f64 {
    if a.is_nan() || b.is_nan() || c.is_nan() {
        f64::NAN
    } else {
        a.min(b).min(c)
    }
}

/// The path writer of `d3-path` with three digits (`new Path(3)`).
#[derive(Debug, Default)]
struct Pen {
    out: String,
}

impl Pen {
    fn move_to(&mut self, x: f64, y: f64) {
        self.out
            .push_str(&format!("M{},{}", d3_number(x), d3_number(y)));
    }

    fn line_to(&mut self, x: f64, y: f64) {
        self.out
            .push_str(&format!("L{},{}", d3_number(x), d3_number(y)));
    }

    fn bezier_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64) {
        self.out.push_str(&format!(
            "C{},{},{},{},{},{}",
            d3_number(x1),
            d3_number(y1),
            d3_number(x2),
            d3_number(y2),
            d3_number(x),
            d3_number(y)
        ));
    }

    fn close(&mut self) {
        self.out.push('Z');
    }
}

/// The d3 curves that edges can use: `curveFactory` in `edges.js` `insertEdge` (lines 732-775).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Curve {
    /// `curveLinear`.
    Linear,
    /// `curveBasis`.
    Basis,
    /// `curveCardinal` with tension 0.
    Cardinal,
    /// `curveCatmullRom` with alpha 0.5.
    CatmullRom,
    /// `curveBumpX` (`x: true`) or `curveBumpY`.
    Bump {
        /// Whether the curve is `bumpX`.
        x: bool,
    },
    /// `curveMonotoneX` or `curveMonotoneY`.
    Monotone {
        /// Whether the curve is `monotoneY`.
        y: bool,
    },
    /// `curveNatural`.
    Natural,
    /// `curveStep` (`t = 0.5`), `curveStepBefore` (`0`) or `curveStepAfter` (`1`).
    Step {
        /// Where the step falls between two points: `0.5` in the middle, `0` before, `1` after.
        t: f64,
    },
}

impl Curve {
    /// The curve of a name, as `insertEdge`'s switch maps it. `rounded` is not a d3 curve and
    /// `None` is returned for it, as well as for any name the switch does not know: those fall
    /// back to `curveBasis` at the call site.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "linear" => Self::Linear,
            "basis" => Self::Basis,
            "cardinal" => Self::Cardinal,
            "bumpX" => Self::Bump { x: true },
            "bumpY" => Self::Bump { x: false },
            "catmullRom" => Self::CatmullRom,
            "monotoneX" => Self::Monotone { y: false },
            "monotoneY" => Self::Monotone { y: true },
            "natural" => Self::Natural,
            "step" => Self::Step { t: 0.5 },
            "stepAfter" => Self::Step { t: 1.0 },
            "stepBefore" => Self::Step { t: 0.0 },
            _ => return None,
        })
    }
}

/// The d3 curve state machine. Each field is the state of the reference's curve object; the
/// unused ones of a curve keep their NaN initial value.
struct Renderer {
    curve: Curve,
    pen: Pen,
    /// `_point`.
    point: u8,
    x0: f64,
    x1: f64,
    x2: f64,
    y0: f64,
    y1: f64,
    y2: f64,
    /// `_t0` of the monotone curves.
    t0: f64,
    /// `_l01_a`, `_l12_a`, `_l23_a` and their `_2a` twins of the Catmull-Rom curve.
    l01_a: f64,
    l12_a: f64,
    l23_a: f64,
    l01_2a: f64,
    l12_2a: f64,
    l23_2a: f64,
    /// `_x`, `_y` of the step curve.
    sx: f64,
    sy: f64,
    /// The points of the natural curve, which needs all of them before it draws.
    xs: Vec<f64>,
    ys: Vec<f64>,
}

impl Renderer {
    fn new(curve: Curve) -> Self {
        let nan = f64::NAN;
        Self {
            curve,
            pen: Pen::default(),
            point: 0,
            x0: nan,
            x1: nan,
            x2: nan,
            y0: nan,
            y1: nan,
            y2: nan,
            t0: nan,
            l01_a: 0.0,
            l12_a: 0.0,
            l23_a: 0.0,
            l01_2a: 0.0,
            l12_2a: 0.0,
            l23_2a: 0.0,
            sx: nan,
            sy: nan,
            xs: Vec::new(),
            ys: Vec::new(),
        }
    }

    /// `lineStart` of every curve. `d3-shape`'s `line` calls it once for a line with every point
    /// defined, so `_line` stays `undefined` and only `_point` decides what `lineEnd` closes.
    fn line_start(&mut self) {
        self.x0 = f64::NAN;
        self.x1 = f64::NAN;
        self.x2 = f64::NAN;
        self.y0 = f64::NAN;
        self.y1 = f64::NAN;
        self.y2 = f64::NAN;
        self.point = 0;
        self.sx = f64::NAN;
        self.sy = f64::NAN;
        self.t0 = f64::NAN;
        self.l01_a = 0.0;
        self.l12_a = 0.0;
        self.l23_a = 0.0;
        self.l01_2a = 0.0;
        self.l12_2a = 0.0;
        self.l23_2a = 0.0;
        if matches!(self.curve, Curve::Natural) {
            self.xs.clear();
            self.ys.clear();
        }
    }

    /// `lineEnd`. A single point closes (`_point === 1`), as in d3.
    fn line_end(&mut self) {
        match self.curve {
            Curve::Linear => {
                if self.point == 1 {
                    self.pen.close();
                }
            }
            Curve::Basis => {
                match self.point {
                    3 => {
                        self.basis_bezier(self.x1, self.y1);
                        self.pen.line_to(self.x1, self.y1);
                    }
                    2 => self.pen.line_to(self.x1, self.y1),
                    _ => {}
                }
                if self.point == 1 {
                    self.pen.close();
                }
            }
            Curve::Cardinal => {
                match self.point {
                    2 => self.pen.line_to(self.x2, self.y2),
                    3 => self.cardinal_bezier(self.x1, self.y1),
                    _ => {}
                }
                if self.point == 1 {
                    self.pen.close();
                }
            }
            Curve::CatmullRom => {
                match self.point {
                    2 => self.pen.line_to(self.x2, self.y2),
                    3 => self.catmull_point(self.x2, self.y2),
                    _ => {}
                }
                if self.point == 1 {
                    self.pen.close();
                }
            }
            Curve::Bump { .. } => {
                if self.point == 1 {
                    self.pen.close();
                }
            }
            Curve::Monotone { y: reflect } => {
                match self.point {
                    2 => self.emit_line(self.x1, self.y1, reflect),
                    3 => {
                        let t0 = self.t0;
                        let t1 = self.slope2(t0);
                        self.monotone_bezier_out(t0, t1, reflect);
                    }
                    _ => {}
                }
                if self.point == 1 {
                    self.pen.close();
                }
            }
            Curve::Natural => {
                self.natural_end();
            }
            Curve::Step { t } => {
                if 0.0 < t && t < 1.0 && self.point == 2 {
                    self.pen.line_to(self.sx, self.sy);
                }
                if self.point == 1 {
                    self.pen.close();
                }
            }
        }
    }

    /// `point(x, y)` of every curve.
    fn point(&mut self, x: f64, y: f64) {
        match self.curve {
            Curve::Linear => {
                if self.point == 0 {
                    self.point = 1;
                    self.pen.move_to(x, y);
                } else {
                    self.point = 2;
                    self.pen.line_to(x, y);
                }
            }
            Curve::Basis => {
                match self.point {
                    0 => {
                        self.point = 1;
                        self.pen.move_to(x, y);
                    }
                    1 => self.point = 2,
                    2 => {
                        self.point = 3;
                        self.pen.line_to(
                            (5.0 * self.x0 + self.x1) / 6.0,
                            (5.0 * self.y0 + self.y1) / 6.0,
                        );
                        self.basis_bezier(x, y);
                    }
                    _ => self.basis_bezier(x, y),
                }
                self.x0 = self.x1;
                self.x1 = x;
                self.y0 = self.y1;
                self.y1 = y;
            }
            Curve::Cardinal => {
                match self.point {
                    0 => {
                        self.point = 1;
                        self.pen.move_to(x, y);
                    }
                    1 => {
                        self.point = 2;
                        self.x1 = x;
                        self.y1 = y;
                    }
                    2 => {
                        self.point = 3;
                        self.cardinal_bezier(x, y);
                    }
                    _ => self.cardinal_bezier(x, y),
                }
                self.x0 = self.x1;
                self.x1 = self.x2;
                self.x2 = x;
                self.y0 = self.y1;
                self.y1 = self.y2;
                self.y2 = y;
            }
            Curve::CatmullRom => {
                self.catmull_point(x, y);
            }
            Curve::Bump { x: horizontal } => {
                match self.point {
                    0 => {
                        self.point = 1;
                        self.pen.move_to(x, y);
                    }
                    1 => {
                        self.point = 2;
                        self.bump_bezier(horizontal, x, y);
                    }
                    _ => self.bump_bezier(horizontal, x, y),
                }
                self.x0 = x;
                self.y0 = y;
            }
            Curve::Monotone { y: reflect } => {
                // `MonotoneY` feeds `MonotoneX` the swapped coordinates, and its context swaps them
                // back on the way out (`lineWithOffset`-free; see `d3-shape` `monotone.js`).
                let (x, y) = if reflect { (y, x) } else { (x, y) };
                self.monotone_point(x, y, reflect);
            }
            Curve::Natural => {
                self.xs.push(x);
                self.ys.push(y);
            }
            Curve::Step { t } => {
                match self.point {
                    0 => {
                        self.point = 1;
                        self.pen.move_to(x, y);
                    }
                    _ => {
                        if self.point == 1 {
                            self.point = 2;
                        }
                        if t <= 0.0 {
                            self.pen.line_to(self.sx, y);
                            self.pen.line_to(x, y);
                        } else {
                            let x1 = self.sx * (1.0 - t) + x * t;
                            self.pen.line_to(x1, self.sy);
                            self.pen.line_to(x1, y);
                        }
                    }
                }
                self.sx = x;
                self.sy = y;
            }
        }
    }

    /// `point(that, x, y)` of `basis.js`.
    fn basis_bezier(&mut self, x: f64, y: f64) {
        self.pen.bezier_to(
            (2.0 * self.x0 + self.x1) / 3.0,
            (2.0 * self.y0 + self.y1) / 3.0,
            (self.x0 + 2.0 * self.x1) / 3.0,
            (self.y0 + 2.0 * self.y1) / 3.0,
            (self.x0 + 4.0 * self.x1 + x) / 6.0,
            (self.y0 + 4.0 * self.y1 + y) / 6.0,
        );
    }

    /// `point(that, x, y)` of `cardinal.js` with `k = (1 - tension) / 6` and tension 0.
    fn cardinal_bezier(&mut self, x: f64, y: f64) {
        let k = 1.0 / 6.0;
        self.pen.bezier_to(
            self.x1 + k * (self.x2 - self.x0),
            self.y1 + k * (self.y2 - self.y0),
            self.x2 + k * (self.x1 - x),
            self.y2 + k * (self.y1 - y),
            self.x2,
            self.y2,
        );
    }

    /// `point(x, y)` of `catmullRom.js`, including the `_l23` update the method does first.
    fn catmull_point(&mut self, x: f64, y: f64) {
        if self.point != 0 {
            let x23 = self.x2 - x;
            let y23 = self.y2 - y;
            self.l23_2a = (x23 * x23 + y23 * y23).powf(0.5);
            self.l23_a = self.l23_2a.sqrt();
        }
        match self.point {
            0 => {
                self.point = 1;
                self.pen.move_to(x, y);
            }
            1 => self.point = 2,
            2 => {
                self.point = 3;
                self.catmull_bezier(x, y);
            }
            _ => self.catmull_bezier(x, y),
        }
        self.l01_a = self.l12_a;
        self.l12_a = self.l23_a;
        self.l01_2a = self.l12_2a;
        self.l12_2a = self.l23_2a;
        self.x0 = self.x1;
        self.x1 = self.x2;
        self.x2 = x;
        self.y0 = self.y1;
        self.y1 = self.y2;
        self.y2 = y;
    }

    /// The module-level `point(that, x, y)` of `catmullRom.js`.
    fn catmull_bezier(&mut self, x: f64, y: f64) {
        const EPSILON: f64 = 1e-12;
        let mut x1 = self.x1;
        let mut y1 = self.y1;
        let mut x2 = self.x2;
        let mut y2 = self.y2;
        if self.l01_a > EPSILON {
            let a = 2.0 * self.l01_2a + 3.0 * self.l01_a * self.l12_a + self.l12_2a;
            let n = 3.0 * self.l01_a * (self.l01_a + self.l12_a);
            x1 = (x1 * a - self.x0 * self.l12_2a + self.x2 * self.l01_2a) / n;
            y1 = (y1 * a - self.y0 * self.l12_2a + self.y2 * self.l01_2a) / n;
        }
        if self.l23_a > EPSILON {
            let b = 2.0 * self.l23_2a + 3.0 * self.l23_a * self.l12_a + self.l12_2a;
            let m = 3.0 * self.l23_a * (self.l23_a + self.l12_a);
            x2 = (x2 * b + self.x1 * self.l23_2a - x * self.l12_2a) / m;
            y2 = (y2 * b + self.y1 * self.l23_2a - y * self.l12_2a) / m;
        }
        self.pen.bezier_to(x1, y1, x2, y2, self.x2, self.y2);
    }

    /// `point(x, y)` of `bump.js`: the bezier of `bumpX` or `bumpY`.
    fn bump_bezier(&mut self, horizontal: bool, x: f64, y: f64) {
        if horizontal {
            self.x0 = (self.x0 + x) / 2.0;
            self.pen.bezier_to(self.x0, self.y0, self.x0, y, x, y);
        } else {
            self.y0 = (self.y0 + y) / 2.0;
            self.pen.bezier_to(self.x0, self.y0, x, self.y0, x, y);
        }
    }

    /// `slope2(that, t)` of `monotone.js`.
    fn slope2(&self, t: f64) -> f64 {
        let h = self.x1 - self.x0;
        if h != 0.0 && !h.is_nan() {
            (3.0 * (self.y1 - self.y0) / h - t) / 2.0
        } else {
            t
        }
    }

    /// `slope3(that, x2, y2)` of `monotone.js`, with its `||` fallbacks for a zero denominator.
    fn slope3(&self, x2: f64, y2: f64) -> f64 {
        let h0 = self.x1 - self.x0;
        let h1 = x2 - self.x1;
        let denominator0 = if h0 != 0.0 && !h0.is_nan() {
            h0
        } else if h1 < 0.0 {
            -0.0
        } else {
            0.0
        };
        let denominator1 = if h1 != 0.0 && !h1.is_nan() {
            h1
        } else if h0 < 0.0 {
            -0.0
        } else {
            0.0
        };
        let s0 = (self.y1 - self.y0) / denominator0;
        let s1 = (y2 - self.y1) / denominator1;
        let p = (s0 * h1 + s1 * h0) / (h0 + h1);
        let value = (sign(s0) + sign(s1)) * js_min3(s0.abs(), s1.abs(), 0.5 * p.abs());
        if value.is_nan() || value == 0.0 {
            0.0
        } else {
            value
        }
    }

    /// `point(x, y)` of `MonotoneX` on the (possibly swapped) coordinates. When `reflect` is set,
    /// every pair written goes out swapped, as `ReflectContext` does.
    fn monotone_point(&mut self, x: f64, y: f64, reflect: bool) {
        if x == self.x1 && y == self.y1 {
            return;
        }
        let mut t1 = f64::NAN;
        match self.point {
            0 => {
                self.point = 1;
                self.emit_move(x, y, reflect);
            }
            1 => self.point = 2,
            2 => {
                self.point = 3;
                t1 = self.slope3(x, y);
                let t0 = self.slope2(t1);
                self.monotone_bezier_out(t0, t1, reflect);
            }
            _ => {
                t1 = self.slope3(x, y);
                let t0 = self.t0;
                self.monotone_bezier_out(t0, t1, reflect);
            }
        }
        self.x0 = self.x1;
        self.x1 = x;
        self.y0 = self.y1;
        self.y1 = y;
        self.t0 = t1;
    }

    fn emit_move(&mut self, x: f64, y: f64, reflect: bool) {
        if reflect {
            self.pen.move_to(y, x);
        } else {
            self.pen.move_to(x, y);
        }
    }

    fn emit_line(&mut self, x: f64, y: f64, reflect: bool) {
        if reflect {
            self.pen.line_to(y, x);
        } else {
            self.pen.line_to(x, y);
        }
    }

    fn monotone_bezier_out(&mut self, t0: f64, t1: f64, reflect: bool) {
        let dx = (self.x1 - self.x0) / 3.0;
        let c1 = (self.x0 + dx, self.y0 + dx * t0);
        let c2 = (self.x1 - dx, self.y1 - dx * t1);
        let end = (self.x1, self.y1);
        if reflect {
            self.pen.bezier_to(c1.1, c1.0, c2.1, c2.0, end.1, end.0);
        } else {
            self.pen.bezier_to(c1.0, c1.1, c2.0, c2.1, end.0, end.1);
        }
    }

    /// `lineEnd` of `natural.js`: the control points of the whole line.
    fn natural_end(&mut self) {
        let n = self.xs.len();
        if n > 0 {
            self.pen.move_to(self.xs[0], self.ys[0]);
            if n == 2 {
                self.pen.line_to(self.xs[1], self.ys[1]);
            } else {
                let (px, py) = (control_points(&self.xs), control_points(&self.ys));
                for i0 in 0..n - 1 {
                    let i1 = i0 + 1;
                    self.pen.bezier_to(
                        px.0[i0],
                        py.0[i0],
                        px.1[i0],
                        py.1[i0],
                        self.xs[i1],
                        self.ys[i1],
                    );
                }
            }
        }
        if n == 1 {
            self.pen.close();
        }
    }
}

fn sign(value: f64) -> f64 {
    if value < 0.0 { -1.0 } else { 1.0 }
}

/// `controlPoints` of `natural.js`: the two control rows for one coordinate.
fn control_points(x: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let n = x.len() - 1;
    let mut a = vec![0.0; n];
    let mut b = vec![0.0; n];
    let mut r = vec![0.0; n];
    a[0] = 0.0;
    b[0] = 2.0;
    r[0] = x[0] + 2.0 * x[1];
    for i in 1..n.saturating_sub(1) {
        a[i] = 1.0;
        b[i] = 4.0;
        r[i] = 4.0 * x[i] + 2.0 * x[i + 1];
    }
    if n >= 2 {
        a[n - 1] = 2.0;
        b[n - 1] = 7.0;
        r[n - 1] = 8.0 * x[n - 1] + x[n];
    }
    for i in 1..n {
        let m = a[i] / b[i - 1];
        b[i] -= m;
        r[i] -= m * r[i - 1];
    }
    a[n - 1] = r[n - 1] / b[n - 1];
    for i in (0..n.saturating_sub(1)).rev() {
        a[i] = (r[i] - a[i + 1]) / b[i];
    }
    b[n - 1] = (x[n] + a[n - 1]) / 2.0;
    for i in 0..n.saturating_sub(1) {
        b[i] = 2.0 * x[i + 1] - a[i + 1];
    }
    (a, b)
}

/// The `d` that a d3 curve writes through `line()` for the points, as `insertEdge` calls it.
/// `None` when the line has no points, which `d3-shape` returns as `null`.
#[must_use]
pub fn curve_path(curve: Curve, points: &[Point]) -> Option<String> {
    if points.is_empty() {
        return None;
    }
    let mut renderer = Renderer::new(curve);
    renderer.line_start();
    for point in points {
        renderer.point(point.x, point.y);
    }
    renderer.line_end();
    let out = renderer.pen.out;
    (!out.is_empty()).then_some(out)
}

/// `fixCorners` in `edges.js`: pre-rounds the right-angle corners of a non-rounded curve, five
/// pixels back from each corner (lines 514-599).
#[must_use]
pub fn fix_corners(points: &[Point]) -> Vec<Point> {
    let corners = corner_positions(points);
    let mut out = Vec::with_capacity(points.len() + 2 * corners.len());
    for (i, &current) in points.iter().enumerate() {
        if !corners.contains(&i) {
            out.push(current);
            continue;
        }
        let prev = points[i - 1];
        let next = points[i + 1];
        let corner = current;
        let new_prev = find_adjacent_point(prev, corner, 5.0);
        let new_next = find_adjacent_point(next, corner, 5.0);
        let x_diff = new_next.x - new_prev.x;
        let y_diff = new_next.y - new_prev.y;
        out.push(new_prev);
        let a = 2f64.sqrt() * 2.0;
        let mut new_corner = Point::new(corner.x, corner.y);
        if (next.x - prev.x).abs() > 10.0 && (next.y - prev.y).abs() >= 10.0 {
            let r = 5.0;
            new_corner = if corner.x == new_prev.x {
                Point::new(
                    if x_diff < 0.0 {
                        new_prev.x - r + a
                    } else {
                        new_prev.x + r - a
                    },
                    if y_diff < 0.0 {
                        new_prev.y - a
                    } else {
                        new_prev.y + a
                    },
                )
            } else {
                Point::new(
                    if x_diff < 0.0 {
                        new_prev.x - a
                    } else {
                        new_prev.x + a
                    },
                    if y_diff < 0.0 {
                        new_prev.y - r + a
                    } else {
                        new_prev.y + r - a
                    },
                )
            };
        }
        out.push(new_corner);
        out.push(new_next);
    }
    out
}

/// `extractCornerPoints` in `edges.js`: the indices of the right-angle corners.
fn corner_positions(points: &[Point]) -> Vec<usize> {
    let mut positions = Vec::new();
    if points.len() < 3 {
        return positions;
    }
    for i in 1..points.len() - 1 {
        let (prev, curr, next) = (points[i - 1], points[i], points[i + 1]);
        let vertical_then_horizontal = prev.x == curr.x
            && curr.y == next.y
            && (curr.x - next.x).abs() > 5.0
            && (curr.y - prev.y).abs() > 5.0;
        let horizontal_then_vertical = prev.y == curr.y
            && curr.x == next.x
            && (curr.x - prev.x).abs() > 5.0
            && (curr.y - next.y).abs() > 5.0;
        if vertical_then_horizontal || horizontal_then_vertical {
            positions.push(i);
        }
    }
    positions
}

/// `findAdjacentPoint` in `edges.js`: the point `distance` before `b` on the segment from `a`.
fn find_adjacent_point(a: Point, b: Point, distance: f64) -> Point {
    let x_diff = b.x - a.x;
    let y_diff = b.y - a.y;
    let length = (x_diff * x_diff + y_diff * y_diff).sqrt();
    let ratio = distance / length;
    Point::new(b.x - ratio * x_diff, b.y - ratio * y_diff)
}

/// `applyMarkerOffsetsToPoints` in `edges.js`: pulls the first and last point back along their
/// segments by the marker offset of each end (`markerOffsets`, truthy values only).
#[must_use]
pub fn apply_marker_offsets(points: &[Point], start: &str, end: &str) -> Vec<Point> {
    let mut out = points.to_vec();
    let n = points.len();
    if n >= 2 {
        if let Some(offset) = marker_offset(start).filter(|value| *value != 0.0) {
            let (p0, p1) = (points[0], points[1]);
            let angle = (p1.y - p0.y).atan2(p1.x - p0.x);
            out[0] = Point::new(p0.x + offset * angle.cos(), p0.y + offset * angle.sin());
        }
        if let Some(offset) = marker_offset(end).filter(|value| *value != 0.0) {
            let (p2, p1) = (points[n - 2], points[n - 1]);
            let angle = (p1.y - p2.y).atan2(p1.x - p2.x);
            out[n - 1] = Point::new(p1.x - offset * angle.cos(), p1.y - offset * angle.sin());
        }
    }
    out
}

/// `generateRoundedPath` in `edges.js` with the given radius: quadratic corners, written with
/// full-precision number text. Fewer than two points give an empty string.
#[must_use]
pub fn generate_rounded_path(points: &[Point], radius: f64) -> String {
    if points.len() < 2 {
        return String::new();
    }
    let size = points.len();
    let mut path = String::new();
    for i in 0..size {
        let curr = points[i];
        if i == 0 {
            path.push_str(&format!("M{},{}", js_number(curr.x), js_number(curr.y)));
        } else if i == size - 1 {
            path.push_str(&format!("L{},{}", js_number(curr.x), js_number(curr.y)));
        } else {
            match rounded_corner(points[i - 1], curr, points[i + 1], radius) {
                Some(corner) => {
                    path.push_str(&format!(
                        "L{},{}",
                        js_number(corner.start.x),
                        js_number(corner.start.y)
                    ));
                    path.push_str(&format!(
                        "Q{},{} {},{}",
                        js_number(curr.x),
                        js_number(curr.y),
                        js_number(corner.end.x),
                        js_number(corner.end.y)
                    ));
                }
                None => {
                    path.push_str(&format!("L{},{}", js_number(curr.x), js_number(curr.y)));
                }
            }
        }
    }
    path
}

/// The rounded corner of a bend, as `generateRoundedPath` and the line hops compute it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoundedCorner {
    /// Where the straight part ends and the quadratic starts, on the incoming segment.
    pub start: Point,
    /// Where the quadratic ends, on the outgoing segment. The control point is the bend itself.
    pub end: Point,
    /// How far the corner reaches back along the incoming segment.
    pub cut_len: f64,
}

/// The corner of a bend between `prev -> curr` and `curr -> next`, or `None` when the bend is
/// degenerate (a zero-length segment, or an angle of almost zero or almost 180 degrees), in which
/// case the path has a plain `L` through it.
#[must_use]
pub fn rounded_corner(prev: Point, curr: Point, next: Point, radius: f64) -> Option<RoundedCorner> {
    const EPSILON: f64 = 1e-5;
    let dx1 = curr.x - prev.x;
    let dy1 = curr.y - prev.y;
    let dx2 = next.x - curr.x;
    let dy2 = next.y - curr.y;
    let len1 = dx1.hypot(dy1);
    let len2 = dx2.hypot(dy2);
    if len1 < EPSILON || len2 < EPSILON {
        return None;
    }
    let nx1 = dx1 / len1;
    let ny1 = dy1 / len1;
    let nx2 = dx2 / len2;
    let ny2 = dy2 / len2;
    let dot = nx1 * nx2 + ny1 * ny2;
    let angle = dot.clamp(-1.0, 1.0).acos();
    if angle < EPSILON || (PI - angle).abs() < EPSILON {
        return None;
    }
    let cut_len = js_min3(radius / (angle / 2.0).sin(), len1 / 2.0, len2 / 2.0);
    Some(RoundedCorner {
        start: Point::new(curr.x - nx1 * cut_len, curr.y - ny1 * cut_len),
        end: Point::new(curr.x + nx2 * cut_len, curr.y + ny2 * cut_len),
        cut_len,
    })
}

/// The offsets of the linear accessors of `getLineFunctionsWithOffset` (`utils/lineWithOffset.ts`),
/// applied to each point of a line drawn with the d3 curves. The start and end markers are the
/// arrow types; an arrow without an offset leaves its end and the proximity push out.
#[must_use]
pub fn line_with_offset(points: &[Point], start: &str, end: &str) -> Vec<Point> {
    let n = points.len();
    if n == 0 {
        return Vec::new();
    }
    let start_height = marker_offset(start);
    let end_height = marker_offset(end);
    let first = points[0];
    let last = points[n - 1];
    // `!(a < b)` as the reference writes it: NaN compares as not less, so it is not `a >= b`.
    let direction_x_right = first.x.partial_cmp(&last.x) != Some(std::cmp::Ordering::Less);
    let direction_y_up = first.y.partial_cmp(&last.y) != Some(std::cmp::Ordering::Less);
    points
        .iter()
        .enumerate()
        .map(|(i, &d)| {
            let mut offset_x = 0.0;
            let mut offset_y = 0.0;
            if i == 0 && start_height.is_some() {
                let m = start_height.unwrap_or(0.0);
                let (angle, dx, dy) = line_delta(Some(points[0]), points.get(1).copied());
                offset_x = m * angle.cos() * sign_of(dx);
                offset_y = m * angle.sin().abs() * sign_of(dy);
            } else if i == n - 1 && end_height.is_some() {
                let m = end_height.unwrap_or(0.0);
                let (angle, dx, dy) =
                    line_delta(Some(points[n - 1]), n.checked_sub(2).map(|k| points[k]));
                offset_x = m * angle.cos() * sign_of(dx);
                offset_y = m * angle.sin().abs() * sign_of(dy);
            }
            let mut x_offset = offset_x;
            let mut y_offset = offset_y;
            let extra_room = 1.0;

            if let Some(h) = end_height {
                let diff_end = (d.x - last.x).abs();
                let diff_y_end = (d.y - last.y).abs();
                if diff_end < h && diff_end > 0.0 && diff_y_end < h {
                    let adjustment =
                        (h + extra_room - diff_end) * if direction_x_right { -1.0 } else { 1.0 };
                    x_offset -= adjustment;
                }
                let diff_end_y = (d.y - last.y).abs();
                let diff_x_end = (d.x - last.x).abs();
                if diff_end_y < h && diff_end_y > 0.0 && diff_x_end < h {
                    let adjustment =
                        (h + extra_room - diff_end_y) * if direction_y_up { -1.0 } else { 1.0 };
                    y_offset -= adjustment;
                }
            }
            if let Some(h) = start_height {
                let diff_start = (d.x - first.x).abs();
                let diff_y_start = (d.y - first.y).abs();
                if diff_start < h && diff_start > 0.0 && diff_y_start < h {
                    let adjustment =
                        (h + extra_room - diff_start) * if direction_x_right { -1.0 } else { 1.0 };
                    x_offset += adjustment;
                }
                let diff_start_y = (d.y - first.y).abs();
                let diff_x_start = (d.x - first.x).abs();
                if diff_start_y < h && diff_start_y > 0.0 && diff_x_start < h {
                    let adjustment =
                        (h + extra_room - diff_start_y) * if direction_y_up { -1.0 } else { 1.0 };
                    y_offset += adjustment;
                }
            }
            Point::new(d.x + x_offset, d.y + y_offset)
        })
        .collect()
}

/// `calculateDeltaAndAngle` of `lineWithOffset.ts`: the angle `atan(dy / dx)` and the deltas from
/// `point1` to `point2`, zeros when either point is missing.
fn line_delta(point1: Option<Point>, point2: Option<Point>) -> (f64, f64, f64) {
    match (point1, point2) {
        (Some(a), Some(b)) => {
            let dx = b.x - a.x;
            let dy = b.y - a.y;
            ((dy / dx).atan(), dx, dy)
        }
        _ => (0.0, 0.0, 0.0),
    }
}

/// `deltaX >= 0 ? 1 : -1` of the accessors.
fn sign_of(delta: f64) -> f64 {
    if delta >= 0.0 { 1.0 } else { -1.0 }
}

/// A segment of a path as the geometry of `getBBox` and `getTotalLength` reads it.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Segment {
    Move(Point),
    Line(Point, Point),
    Quadratic(Point, Point, Point),
    Cubic(Point, Point, Point, Point),
    /// An elliptical arc from the first point to the last, with radii, rotation, and flags.
    Arc {
        from: Point,
        to: Point,
        rx: f64,
        ry: f64,
        rotation: f64,
        large: bool,
        sweep: bool,
    },
}

/// Parses the path commands this crate writes: `M`, `L`, `Q`, `C`, `A` and `Z`, absolute, with
/// numbers separated by commas or spaces.
fn parse_segments(d: &str) -> Vec<Segment> {
    let mut numbers: Vec<f64> = Vec::new();
    let mut command = ' ';
    let mut current = Point::default();
    let mut subpath_start = Point::default();
    let mut segments = Vec::new();
    let mut flush = |command: char, numbers: &mut Vec<f64>| {
        let mut values = numbers.drain(..);
        let mut next = || values.next().unwrap_or(0.0);
        match command {
            'M' => {
                let p = Point::new(next(), next());
                current = p;
                subpath_start = p;
                segments.push(Segment::Move(p));
            }
            'L' => {
                let p = Point::new(next(), next());
                segments.push(Segment::Line(current, p));
                current = p;
            }
            'Q' => {
                let c = Point::new(next(), next());
                let p = Point::new(next(), next());
                segments.push(Segment::Quadratic(current, c, p));
                current = p;
            }
            'C' => {
                let c1 = Point::new(next(), next());
                let c2 = Point::new(next(), next());
                let p = Point::new(next(), next());
                segments.push(Segment::Cubic(current, c1, c2, p));
                current = p;
            }
            'A' => {
                let rx = next();
                let ry = next();
                let rotation = next();
                let large = next() != 0.0;
                let sweep = next() != 0.0;
                let p = Point::new(next(), next());
                segments.push(Segment::Arc {
                    from: current,
                    to: p,
                    rx,
                    ry,
                    rotation,
                    large,
                    sweep,
                });
                current = p;
            }
            'Z' => {
                segments.push(Segment::Line(current, subpath_start));
                current = subpath_start;
            }
            _ => {}
        }
    };
    let bytes: Vec<char> = d.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_alphabetic() {
            if command != ' ' && (command != 'Z' || !numbers.is_empty()) {
                flush(command, &mut numbers);
            }
            command = c;
            if c == 'Z' {
                flush('Z', &mut numbers);
                command = ' ';
            }
            i += 1;
        } else if c == '-' || c == '+' || c == '.' || c.is_ascii_digit() {
            let start = i;
            i += 1;
            while i < bytes.len() {
                let d = bytes[i];
                let exponent_sign = (d == '-' || d == '+') && matches!(bytes[i - 1], 'e' | 'E');
                if d.is_ascii_digit() || d == '.' || d == 'e' || d == 'E' || exponent_sign {
                    i += 1;
                } else {
                    break;
                }
            }
            let text: String = bytes[start..i].iter().collect();
            numbers.push(text.parse().unwrap_or(0.0));
        } else {
            i += 1;
        }
    }
    if command != ' ' && command != 'Z' {
        flush(command, &mut numbers);
    }
    segments
}

/// The tight box of a path, as `getBBox` gives it: the end points and the extremes of every
/// curve. `None` for a path without segments.
#[must_use]
pub fn path_bounds(d: &str) -> Option<BBox> {
    let mut points: Vec<Point> = Vec::new();
    for segment in parse_segments(d) {
        match segment {
            Segment::Move(p) => points.push(p),
            Segment::Line(a, b) => {
                points.push(a);
                points.push(b);
            }
            Segment::Quadratic(p0, c, p1) => {
                points.push(p0);
                points.push(p1);
                for t in quadratic_extremes(p0, c, p1) {
                    points.push(quadratic_point(p0, c, p1, t));
                }
            }
            Segment::Cubic(p0, c1, c2, p1) => {
                points.push(p0);
                points.push(p1);
                for t in cubic_extremes(p0, c1, c2, p1) {
                    points.push(cubic_point(p0, c1, c2, p1, t));
                }
            }
            Segment::Arc {
                from,
                to,
                rx,
                ry,
                rotation,
                large,
                sweep,
            } => {
                points.push(from);
                points.push(to);
                points.extend(arc_extremes(from, to, rx, ry, rotation, large, sweep));
            }
        }
    }
    BBox::from_points(points)
}

/// The length of a path, as `getTotalLength` gives it, computed by composite Simpson rules over
/// each curve. Browsers flatten curves with their own tolerance, so the digits differ.
#[must_use]
pub fn path_length(d: &str) -> f64 {
    let mut total = 0.0;
    for segment in parse_segments(d) {
        total += match segment {
            Segment::Move(_) => 0.0,
            Segment::Line(a, b) => a.distance(b),
            Segment::Quadratic(p0, c, p1) => simpson(2000, |t| {
                let d1 = Point::new(2.0 * (c.x - p0.x), 2.0 * (c.y - p0.y));
                let d2 = Point::new(2.0 * (p1.x - c.x), 2.0 * (p1.y - c.y));
                let dx = (1.0 - t) * d1.x + t * d2.x;
                let dy = (1.0 - t) * d1.y + t * d2.y;
                dx.hypot(dy)
            }),
            Segment::Cubic(p0, c1, c2, p1) => simpson(2000, |t| {
                let s = 1.0 - t;
                let dx = 3.0
                    * (s * s * (c1.x - p0.x) + 2.0 * s * t * (c2.x - c1.x) + t * t * (p1.x - c2.x));
                let dy = 3.0
                    * (s * s * (c1.y - p0.y) + 2.0 * s * t * (c2.y - c1.y) + t * t * (p1.y - c2.y));
                dx.hypot(dy)
            }),
            Segment::Arc {
                from,
                to,
                rx,
                ry,
                rotation,
                large,
                sweep,
            } => arc_length(from, to, rx, ry, rotation, large, sweep),
        };
    }
    total
}

/// The composite Simpson rule over `[0, 1]` with `steps` intervals.
fn simpson(steps: usize, f: impl Fn(f64) -> f64) -> f64 {
    let h = 1.0 / steps as f64;
    let mut sum = f(0.0) + f(1.0);
    for i in 1..steps {
        let weight = if i % 2 == 1 { 4.0 } else { 2.0 };
        sum += weight * f(i as f64 * h);
    }
    sum * h / 3.0
}

fn quadratic_point(p0: Point, c: Point, p1: Point, t: f64) -> Point {
    let s = 1.0 - t;
    Point::new(
        s * s * p0.x + 2.0 * s * t * c.x + t * t * p1.x,
        s * s * p0.y + 2.0 * s * t * c.y + t * t * p1.y,
    )
}

fn quadratic_extremes(p0: Point, c: Point, p1: Point) -> Vec<f64> {
    let mut out = Vec::new();
    for (a, b, d) in [(p0.x, c.x, p1.x), (p0.y, c.y, p1.y)] {
        let denominator = a - 2.0 * b + d;
        if denominator != 0.0 {
            let t = (a - b) / denominator;
            if t > 0.0 && t < 1.0 {
                out.push(t);
            }
        }
    }
    out
}

fn cubic_point(p0: Point, c1: Point, c2: Point, p1: Point, t: f64) -> Point {
    let s = 1.0 - t;
    let a = s * s * s;
    let b = 3.0 * s * s * t;
    let c = 3.0 * s * t * t;
    let d = t * t * t;
    Point::new(
        a * p0.x + b * c1.x + c * c2.x + d * p1.x,
        a * p0.y + b * c1.y + c * c2.y + d * p1.y,
    )
}

/// The roots in `(0, 1)` of the derivative of a cubic, per coordinate.
fn cubic_extremes(p0: Point, c1: Point, c2: Point, p1: Point) -> Vec<f64> {
    let mut out = Vec::new();
    for (a0, a1, a2, a3) in [(p0.x, c1.x, c2.x, p1.x), (p0.y, c1.y, c2.y, p1.y)] {
        let v0 = a1 - a0;
        let v1 = a2 - a1;
        let v2 = a3 - a2;
        let a = v0 - 2.0 * v1 + v2;
        let b = 2.0 * (v1 - v0);
        let c = v0;
        let roots = if a.abs() < 1e-12 {
            if b.abs() < 1e-12 {
                Vec::new()
            } else {
                vec![-c / b]
            }
        } else {
            let disc = b * b - 4.0 * a * c;
            if disc < 0.0 {
                Vec::new()
            } else {
                let root = disc.sqrt();
                vec![(-b + root) / (2.0 * a), (-b - root) / (2.0 * a)]
            }
        };
        out.extend(roots.into_iter().filter(|t| *t > 0.0 && *t < 1.0));
    }
    out
}

/// The centre parametrisation of an elliptical arc (SVG implementation notes F.6.5), for an axis
/// aligned ellipse. Returns the centre, the start angle, the signed sweep, and the radii after
/// scaling up. `None` for a degenerate arc (zero radius or coincident end points).
fn arc_parameters(
    from: Point,
    to: Point,
    rx: f64,
    ry: f64,
    large: bool,
    sweep: bool,
) -> Option<(Point, f64, f64, f64, f64)> {
    let mut rx = rx.abs();
    let mut ry = ry.abs();
    if rx == 0.0 || ry == 0.0 || (from.x == to.x && from.y == to.y) {
        return None;
    }
    let x1p = (from.x - to.x) / 2.0;
    let y1p = (from.y - to.y) / 2.0;
    let lambda = x1p * x1p / (rx * rx) + y1p * y1p / (ry * ry);
    if lambda > 1.0 {
        let scale = lambda.sqrt();
        rx *= scale;
        ry *= scale;
    }
    let num = rx * rx * ry * ry - rx * rx * y1p * y1p - ry * ry * x1p * x1p;
    let den = rx * rx * y1p * y1p + ry * ry * x1p * x1p;
    let sign = if large == sweep { -1.0 } else { 1.0 };
    let coefficient = sign * (num / den).max(0.0).sqrt();
    let cxp = coefficient * rx * y1p / ry;
    let cyp = -coefficient * ry * x1p / rx;
    let centre = Point::new(cxp + (from.x + to.x) / 2.0, cyp + (from.y + to.y) / 2.0);
    let theta1 = ((y1p - cyp) / ry).atan2((x1p - cxp) / rx);
    let theta2 = ((-y1p - cyp) / ry).atan2((-x1p - cxp) / rx);
    let mut delta = theta2 - theta1;
    if sweep && delta < 0.0 {
        delta += 2.0 * PI;
    } else if !sweep && delta > 0.0 {
        delta -= 2.0 * PI;
    }
    Some((centre, theta1, delta, rx, ry))
}

/// The points of an arc at its extremes (the axis-aligned extremes that lie on the arc).
fn arc_extremes(
    from: Point,
    to: Point,
    rx: f64,
    ry: f64,
    rotation: f64,
    large: bool,
    sweep: bool,
) -> Vec<Point> {
    let Some((centre, theta1, delta, rx, ry)) =
        arc_parameters(from, to, rx, ry, large, sweep).filter(|_| rotation == 0.0)
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for angle in [0.0, PI / 2.0, PI, 3.0 * PI / 2.0] {
        let offset = (angle - theta1).rem_euclid(2.0 * PI);
        let on_arc = if delta >= 0.0 {
            offset <= delta
        } else {
            (2.0 * PI - offset) % (2.0 * PI) <= -delta
        };
        if on_arc {
            out.push(Point::new(
                centre.x + rx * angle.cos(),
                centre.y + ry * angle.sin(),
            ));
        }
    }
    out
}

fn arc_length(
    from: Point,
    to: Point,
    rx: f64,
    ry: f64,
    rotation: f64,
    large: bool,
    sweep: bool,
) -> f64 {
    let Some((_, theta1, delta, rx, ry)) = arc_parameters(from, to, rx, ry, large, sweep) else {
        return from.distance(to);
    };
    if rotation != 0.0 {
        return from.distance(to);
    }
    if (rx - ry).abs() < 1e-12 {
        return rx * delta.abs();
    }
    simpson(4000, |t| {
        let angle = theta1 + delta * t;
        (rx * angle.sin()).hypot(ry * angle.cos()) * delta.abs()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn numbers_read_as_javascript_writes_them() {
        assert_eq!(js_number(0.0), "0");
        assert_eq!(js_number(-0.0), "0");
        assert_eq!(js_number(7.0710678118654755), "7.0710678118654755");
        assert_eq!(js_number(1e-7), "1e-7");
        assert_eq!(js_number(0.000001), "0.000001");
        assert_eq!(js_number(1e21), "1e+21");
        assert_eq!(js_number(123456789012345680000.0), "123456789012345680000");
        assert_eq!(js_number(-2.5), "-2.5");
        assert_eq!(js_number(1.5e-10), "1.5e-10");
    }

    #[test]
    fn three_digit_numbers_round_ties_upwards() {
        assert_eq!(d3_number(10.12345), "10.123");
        assert_eq!(d3_number(20.5555), "20.556");
        assert_eq!(d3_number(-0.0004), "0");
        assert_eq!(js_round(-2.5), -2.0);
        assert_eq!(js_round(2.5), 3.0);
    }

    #[test]
    fn the_rounded_path_of_a_right_angle_matches_the_reference() {
        let path = generate_rounded_path(&[p(0.0, 0.0), p(0.0, 40.0), p(60.0, 40.0)], 5.0);
        assert_eq!(
            path,
            "M0,0L0,32.928932188134524Q0,40 7.0710678118654755,40L60,40"
        );
    }

    #[test]
    fn a_short_segment_between_collinear_points_stays_a_line() {
        let path = generate_rounded_path(&[p(0.0, 0.0), p(0.0, 0.1), p(0.0, 30.0)], 5.0);
        assert_eq!(path, "M0,0L0,0.1L0,30");
    }

    #[test]
    fn the_linear_curve_writes_moves_and_lines() {
        let path = curve_path(
            Curve::Linear,
            &[
                p(0.0, 0.0),
                p(10.12345, 0.0),
                p(20.0, 20.5555),
                p(30.0, -0.0004),
            ],
        );
        assert_eq!(path.as_deref(), Some("M0,0L10.123,0L20,20.556L30,0"));
    }

    #[test]
    fn the_basis_curve_follows_d3_shape() {
        let path = curve_path(
            Curve::Basis,
            &[
                p(0.0, 0.0),
                p(10.12345, 0.0),
                p(20.0, 20.5555),
                p(30.0, -0.0004),
            ],
        );
        assert_eq!(
            path.as_deref(),
            Some(
                "M0,0L1.687,0C3.374,0,6.749,0,10.082,3.426C13.416,6.852,16.708,13.704,20.021,13.704C23.333,13.704,26.667,6.852,28.333,3.426L30,0"
            )
        );
        assert_eq!(
            curve_path(Curve::Basis, &[p(0.0, 0.0), p(10.12345, 0.0)]).as_deref(),
            Some("M0,0L10.123,0")
        );
    }

    #[test]
    fn the_step_curves_follow_d3_shape() {
        let points = [p(0.0, 0.0), p(10.0, 5.0), p(20.0, 20.0)];
        assert_eq!(
            curve_path(Curve::Step { t: 0.0 }, &points).as_deref(),
            Some("M0,0L0,5L10,5L10,20L20,20")
        );
        assert_eq!(
            curve_path(Curve::Step { t: 1.0 }, &points).as_deref(),
            Some("M0,0L10,0L10,5L20,5L20,20")
        );
    }

    #[test]
    fn a_single_point_closes_as_d3_does() {
        assert_eq!(
            curve_path(Curve::Linear, &[p(1.0, 2.0)]).as_deref(),
            Some("M1,2Z")
        );
        assert_eq!(curve_path(Curve::Linear, &[]), None);
    }

    #[test]
    fn fix_corners_pre_rounds_a_right_angle() {
        let fixed = fix_corners(&[p(0.0, 0.0), p(0.0, 40.0), p(60.0, 40.0)]);
        assert_eq!(fixed.len(), 5);
        assert_eq!(fixed[0], p(0.0, 0.0));
        assert_eq!(fixed[1], p(0.0, 35.0));
        assert!((fixed[2].x - 2.1715728752538097).abs() < 1e-12);
        assert!((fixed[2].y - 37.82842712474619).abs() < 1e-12);
        assert_eq!(fixed[3], p(5.0, 40.0));
    }

    #[test]
    fn the_arrow_pulls_the_end_back_by_its_offset() {
        let moved = apply_marker_offsets(
            &[p(0.0, 0.0), p(50.0, 0.0), p(50.0, 60.0)],
            "none",
            "arrow_point",
        );
        assert!((moved[2].x - 50.0).abs() < 1e-9);
        assert!((moved[2].y - 56.0).abs() < 1e-9);
        let both =
            apply_marker_offsets(&[p(0.0, 0.0), p(0.0, 100.0)], "arrow_point", "arrow_point");
        assert!((both[0].y - 4.0).abs() < 1e-9);
        assert!((both[1].y - 96.0).abs() < 1e-9);
    }

    #[test]
    fn the_linear_accessors_pull_the_end_back_too() {
        let moved = line_with_offset(
            &[p(0.0, 0.0), p(50.0, 0.0), p(50.0, 60.0)],
            "none",
            "arrow_point",
        );
        assert!((moved[2].x - 50.0).abs() < 1e-9);
        assert!((moved[2].y - 56.0).abs() < 1e-9);
    }

    #[test]
    fn the_bounds_of_a_line_are_its_points() {
        let bounds = path_bounds("M0,0L10,0L10,20").expect("bounds");
        assert_eq!(bounds, BBox::new(0.0, 0.0, 10.0, 20.0));
        assert!((path_length("M0,0L10,0L10,20") - 30.0).abs() < 1e-12);
    }

    #[test]
    fn the_bounds_of_a_quadratic_are_its_tight_box() {
        // Quadratic from (0,0) with control (10,20) to (20,0): its top is at y = 10.
        let bounds = path_bounds("M0,0Q10,20 20,0").expect("bounds");
        assert!((bounds.y - 0.0).abs() < 1e-12);
        assert!((bounds.height - 10.0).abs() < 1e-12);
        assert!((bounds.width - 20.0).abs() < 1e-12);
    }

    #[test]
    fn the_length_of_a_half_circle_arc_is_pi_r() {
        let length = path_length("M0,0A5,5 0 0 1 10,0");
        assert!((length - 5.0 * PI).abs() < 1e-9, "{length}");
        let bounds = path_bounds("M0,0A5,5 0 0 1 10,0").expect("bounds");
        assert!((bounds.height - 5.0).abs() < 1e-9, "{bounds:?}");
    }
}
