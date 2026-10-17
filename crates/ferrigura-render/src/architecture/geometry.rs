//! The boxes and the arithmetic of cytoscape that the architecture renderer reaches.
//!
//! cytoscape keeps a box as its two corners, and its `updateBounds` skips a box with no area and
//! leaves a side alone when the new value does not beat it. [`Box2`] keeps those rules, and the
//! JavaScript `Math.min` and `Math.max`, which are NaN as soon as one argument is (the reference
//! reads them on positions that a degenerate edge can make NaN).

/// `Infinity`: the start of a box that holds nothing yet (`makeBoundingBox`).
pub const INFINITY: f64 = f64::INFINITY;

/// A box by its two corners, as cytoscape's `BoundingBox` holds it (`x1`, `y1`, `x2`, `y2`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Box2 {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

impl Box2 {
    /// A box that holds nothing: the union with it changes nothing.
    pub const EMPTY: Self = Self {
        x1: INFINITY,
        y1: INFINITY,
        x2: -INFINITY,
        y2: -INFINITY,
    };

    /// The box of a corner pair, as it is.
    #[must_use]
    pub const fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        Self { x1, y1, x2, y2 }
    }

    /// The box of a centre and a size: `x ± w / 2`, `y ± h / 2`.
    #[must_use]
    pub fn around(x: f64, y: f64, w: f64, h: f64) -> Self {
        let half_w = w / 2.0;
        let half_h = h / 2.0;
        Self::new(x - half_w, y - half_h, x + half_w, y + half_h)
    }

    /// cytoscape's `updateBounds`: a box with no width or no height is skipped, and each side moves
    /// only when the new value is past the old one.
    pub fn include(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        if x2 - x1 == 0.0 || y2 - y1 == 0.0 {
            return;
        }
        self.x1 = if x1 < self.x1 { x1 } else { self.x1 };
        self.x2 = if x2 > self.x2 { x2 } else { self.x2 };
        self.y1 = if y1 < self.y1 { y1 } else { self.y1 };
        self.y2 = if y2 > self.y2 { y2 } else { self.y2 };
    }

    /// [`Box2::include`] with another box.
    pub fn include_box(&mut self, other: Box2) {
        self.include(other.x1, other.y1, other.x2, other.y2);
    }

    /// Whether the box has area: `getBBox` leaves a box with no width or no height out of a union.
    #[must_use]
    pub fn has_area(&self) -> bool {
        self.w() > 0.0 && self.h() > 0.0
    }

    /// The width, `x2 - x1`.
    #[must_use]
    pub fn w(&self) -> f64 {
        self.x2 - self.x1
    }

    /// The height, `y2 - y1`.
    #[must_use]
    pub fn h(&self) -> f64 {
        self.y2 - self.y1
    }

    /// `noninf`: an infinite side becomes 0, as `boundingBox` leaves an empty box at the origin.
    #[must_use]
    pub fn finite(self) -> Self {
        let fix = |v: f64| if v.is_infinite() { 0.0 } else { v };
        Self::new(fix(self.x1), fix(self.y1), fix(self.x2), fix(self.y2))
    }

    /// The box moved by `(dx, dy)`.
    #[must_use]
    pub fn translate(self, dx: f64, dy: f64) -> Self {
        Self::new(self.x1 + dx, self.y1 + dy, self.x2 + dx, self.y2 + dy)
    }

    /// `expandBoundingBox(bb, 1)`: each side moves out by one pixel.
    #[must_use]
    pub fn expand1(self) -> Self {
        Self::new(self.x1 - 1.0, self.y1 - 1.0, self.x2 + 1.0, self.y2 + 1.0)
    }

    /// The end of `boundingBoxImpl`: the box made finite, and expanded by one pixel when it has area.
    #[must_use]
    pub fn settled(self) -> Self {
        let finite = self.finite();
        if finite.w() > 0.0 && finite.h() > 0.0 {
            finite.expand1()
        } else {
            finite
        }
    }

    /// The box of the four corners of this one after `transform` (AABB of the corners), which is what
    /// `getBBox` of a transformed child contributes to its parent.
    #[must_use]
    pub fn transformed(self, transform: &Affine) -> Self {
        if self.x1 > self.x2 || self.y1 > self.y2 {
            return Self::EMPTY;
        }
        let corners = [
            transform.apply(self.x1, self.y1),
            transform.apply(self.x2, self.y1),
            transform.apply(self.x1, self.y2),
            transform.apply(self.x2, self.y2),
        ];
        let mut out = Self::EMPTY;
        let (mut x1, mut y1, mut x2, mut y2) = (INFINITY, INFINITY, -INFINITY, -INFINITY);
        for (x, y) in corners {
            x1 = x1.min(x);
            y1 = y1.min(y);
            x2 = x2.max(x);
            y2 = y2.max(y);
        }
        out.x1 = x1;
        out.y1 = y1;
        out.x2 = x2;
        out.y2 = y2;
        out
    }
}

/// `Math.min` of three numbers: NaN when one of them is NaN.
#[must_use]
pub fn js_min3(a: f64, b: f64, c: f64) -> f64 {
    if a.is_nan() || b.is_nan() || c.is_nan() {
        return f64::NAN;
    }
    a.min(b).min(c)
}

/// `Math.max` of three numbers: NaN when one of them is NaN.
#[must_use]
pub fn js_max3(a: f64, b: f64, c: f64) -> f64 {
    if a.is_nan() || b.is_nan() || c.is_nan() {
        return f64::NAN;
    }
    a.max(b).max(c)
}

/// An affine transform of the kinds the architecture writes: a translation, and a rotation about a
/// point, composed in SVG order (`transform="a b c"` applies `c` first to the points).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affine {
    /// The matrix `[a, b, c, d, e, f]` of `matrix(a b c d e f)`.
    pub m: [f64; 6],
}

impl Affine {
    /// `translate(x, y)`.
    #[must_use]
    pub fn translate(x: f64, y: f64) -> Self {
        Self {
            m: [1.0, 0.0, 0.0, 1.0, x, y],
        }
    }

    /// `rotate(degrees)` about the origin.
    #[must_use]
    pub fn rotate(degrees: f64) -> Self {
        Self::rotate_about(degrees, 0.0, 0.0)
    }

    /// `rotate(degrees, cx, cy)`: a rotation about `(cx, cy)`.
    #[must_use]
    pub fn rotate_about(degrees: f64, cx: f64, cy: f64) -> Self {
        let radians = degrees.to_radians();
        let (sin, cos) = (radians.sin(), radians.cos());
        let rotation = Self {
            m: [cos, sin, -sin, cos, 0.0, 0.0],
        };
        // `translate(cx, cy) rotate(degrees) translate(-cx, -cy)`: the point goes to the origin first.
        Self::translate(-cx, -cy)
            .then(&rotation)
            .then(&Self::translate(cx, cy))
    }

    /// The transform that applies `self` first, then `next`: the matrix `next * self`.
    #[must_use]
    pub fn then(&self, next: &Self) -> Self {
        let a = self.m;
        let b = next.m;
        Self {
            m: [
                b[0] * a[0] + b[2] * a[1],
                b[1] * a[0] + b[3] * a[1],
                b[0] * a[2] + b[2] * a[3],
                b[1] * a[2] + b[3] * a[3],
                b[0] * a[4] + b[2] * a[5] + b[4],
                b[1] * a[4] + b[3] * a[5] + b[5],
            ],
        }
    }

    /// The image of a point.
    #[must_use]
    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        let m = self.m;
        (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_box_leaves_a_union_alone() {
        let mut bounds = Box2::EMPTY;
        bounds.include(0.0, 0.0, 10.0, 10.0);
        assert_eq!(bounds, Box2::new(0.0, 0.0, 10.0, 10.0));
        bounds.include(5.0, 5.0, 5.0, 20.0);
        assert_eq!(bounds, Box2::new(0.0, 0.0, 10.0, 10.0));
    }

    #[test]
    fn a_settled_box_with_area_is_expanded_by_one() {
        let settled = Box2::new(0.0, 0.0, 10.0, 10.0).settled();
        assert_eq!(settled, Box2::new(-1.0, -1.0, 11.0, 11.0));
        assert_eq!(Box2::EMPTY.settled(), Box2::new(0.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn math_min_is_nan_for_a_nan_argument() {
        assert!(js_min3(1.0, f64::NAN, 2.0).is_nan());
        assert_eq!(js_min3(1.0, -2.0, 2.0), -2.0);
    }

    #[test]
    fn rotation_about_a_point_fixes_it() {
        let rotate = Affine::rotate_about(90.0, 3.0, 4.0);
        let (x, y) = rotate.apply(3.0, 4.0);
        assert!((x - 3.0).abs() < 1e-12 && (y - 4.0).abs() < 1e-12);
        let (x, y) = rotate.apply(4.0, 4.0);
        assert!((x - 3.0).abs() < 1e-12 && (y - 5.0).abs() < 1e-12);
    }
}
