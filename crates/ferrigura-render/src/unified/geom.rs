//! Points and boxes shared by the flowchart modules. Coordinates are SVG user units, y down.

use ferrigura_jsmath::hypot;

/// A point.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    /// The horizontal coordinate.
    pub x: f64,
    /// The vertical coordinate, growing downwards.
    pub y: f64,
}

impl Point {
    /// A point at `x`, `y`.
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// The distance to another point.
    #[must_use]
    pub fn distance(self, other: Self) -> f64 {
        hypot(self.x - other.x, self.y - other.y)
    }
}

/// A measured size as the browser gives it. `getBBox` returns an `SVGRect`, whose members are
/// single-precision floats, so every size the reference takes from it is a float32 value (for
/// example the node box height 56.400001525878906 in `shape-themes/07`). Its later arithmetic is
/// double precision on those values, which is exact where the layout tests a point against a box
/// edge (`outsideNode`, `outlineAttachPoint`); a double size only comes close to the edge, and the
/// test then decides the other way.
#[must_use]
pub fn browser_f32(value: f64) -> f64 {
    f64::from(value as f32)
}

/// A coordinate of a path as the browser reads it from the `d` string: the number the reference
/// writes (the shortest decimal that reads back as the same double), parsed the way Chromium's SVG
/// parser parses it. See `svg_number`.
#[must_use]
pub fn path_coordinate_f32(value: f64) -> f64 {
    f64::from(svg_number(&value.to_string()))
}

/// The height of a sloped rectangle's outline as the browser's `getBBox` gives it, from the
/// outline's top and bottom as the browser parses them ([`path_coordinate_f32`]).
///
/// The outline runs from `-2 * bottom` to `bottom`, so its exact span is `3 * bottom`. When that
/// span is a float32 the browser returns it. When it is not, the browser returns the float32 one
/// above the nearest float32 to it. Measured in Chromium on 433 outlines of this shape (half and
/// quarter ulps of the span among them) with no exception, and on 163 exact spans unchanged. The
/// rule is measured for this shape only: for spans between unrelated coordinates the same
/// nearest-plus-one answer misses on 11 of 178 and 87 of 484 inexact spans, so other paths keep
/// the nearest float32 of their span.
#[must_use]
pub fn sloped_outline_height_f32(top: f64, bottom: f64) -> f64 {
    let exact = bottom - top;
    let nearest = exact as f32;
    if f64::from(nearest) == exact {
        return exact;
    }
    f64::from(f32::from_bits(nearest.to_bits() + 1))
}

/// Parses a decimal the way Chromium's SVG number parser does, for a decimal without an exponent.
///
/// The integer digits accumulate in single precision. The fraction digits accumulate in single
/// precision too, each digit scaled by a single-precision power of ten that is itself multiplied by
/// 0.1 at each digit, and the two parts are added in single precision. The result differs from the
/// correctly rounded float32 of the decimal, by a fraction of a float32 ulp, on values near a tie
/// between two float32 values, where the reference's border and tie tests are decided. The values
/// it is checked against are in `svg_number_matches_chromium`.
fn svg_number(text: &str) -> f32 {
    let (negative, body) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (integer, fraction) = body.split_once('.').unwrap_or((body, ""));
    let mut whole = 0.0_f32;
    for digit in integer.bytes() {
        whole = ((f64::from(whole) * 10.0) as f32) + f32::from(digit - b'0');
    }
    let mut parts = 0.0_f32;
    let mut scale = 1.0_f32;
    for digit in fraction.bytes() {
        scale = (f64::from(scale) * 0.1) as f32;
        let term = (f64::from(digit - b'0') * f64::from(scale)) as f32;
        parts = (f64::from(parts) + f64::from(term)) as f32;
    }
    let value = if fraction.is_empty() {
        whole
    } else {
        (f64::from(whole) + f64::from(parts)) as f32
    };
    if negative { -value } else { value }
}

/// A box by its top-left corner and its size, as `getBBox` returns it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BBox {
    /// The left edge.
    pub x: f64,
    /// The top edge.
    pub y: f64,
    /// The width.
    pub width: f64,
    /// The height.
    pub height: f64,
}

impl BBox {
    /// A box from its top-left corner and size.
    #[must_use]
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// The right edge.
    #[must_use]
    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    /// The bottom edge.
    #[must_use]
    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    /// The centre.
    #[must_use]
    pub fn center(&self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Whether the box has no width or no height. `getBBox` leaves such a box out of a union, so
    /// an empty box does not move the box of a drawing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }

    /// The box as the browser's `getBBox` returns it, each value rounded to the nearest float32.
    /// See [`browser_f32`].
    #[must_use]
    pub fn browser(self) -> Self {
        Self::new(
            browser_f32(self.x),
            browser_f32(self.y),
            browser_f32(self.width),
            browser_f32(self.height),
        )
    }

    /// The smallest box that holds both boxes.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        Self::new(
            x,
            y,
            self.right().max(other.right()) - x,
            self.bottom().max(other.bottom()) - y,
        )
    }

    /// The box moved by `dx`, `dy`.
    #[must_use]
    pub fn translate(self, dx: f64, dy: f64) -> Self {
        Self::new(self.x + dx, self.y + dy, self.width, self.height)
    }

    /// The smallest box that holds the points, or `None` for no points.
    ///
    /// The points are the coordinates the reference writes into a path's `d`, so each is taken as
    /// the browser parses it ([`path_coordinate_f32`]) before the box is measured.
    pub fn from_points(points: impl IntoIterator<Item = Point>) -> Option<Self> {
        let mut points = points
            .into_iter()
            .map(|point| Point::new(path_coordinate_f32(point.x), path_coordinate_f32(point.y)));
        let first = points.next()?;
        let (mut left, mut top, mut right, mut bottom) = (first.x, first.y, first.x, first.y);
        for point in points {
            left = left.min(point.x);
            top = top.min(point.y);
            right = right.max(point.x);
            bottom = bottom.max(point.y);
        }
        Some(Self::new(left, top, right - left, bottom - top))
    }
}

/// A node's box as the reference's intersect functions and `boundsFor` read it: the centre and
/// the size.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CenterBox {
    /// The horizontal centre.
    pub x: f64,
    /// The vertical centre.
    pub y: f64,
    /// The width.
    pub width: f64,
    /// The height.
    pub height: f64,
}

impl CenterBox {
    /// A box from its centre and size.
    #[must_use]
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// The centre.
    #[must_use]
    pub fn center(&self) -> Point {
        Point::new(self.x, self.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_union_holds_both_boxes() {
        let union = BBox::new(0.0, 0.0, 10.0, 10.0).union(BBox::new(-5.0, 5.0, 10.0, 20.0));
        assert_eq!(union, BBox::new(-5.0, 0.0, 15.0, 25.0));
    }

    /// `svg_number` against Chromium: the `getBBox` height of `M0 0 L0 <text>`, measured in Chromium
    /// for each text, so that the float32 each text becomes is the browser's.
    #[test]
    fn svg_number_matches_chromium() {
        let measured: &[(&str, f32)] = &[
            ("32.3999996185304", 32.4_f32),
            ("81.80000000000035", 81.8_f32),
            ("17.949999809265236", 17.95_f32),
            ("36.50000000000039", 36.5_f32),
            ("71.79999923706059", 71.799995_f32),
            ("14.00000000000003", 14.0_f32),
            ("107.7000045776366", 107.700005_f32),
            ("71.79999923706045", 71.799995_f32),
            ("35.899999618530025", 35.899998_f32),
            ("88.19999999999966", 88.2_f32),
            ("32.39999961853032", 32.4_f32),
            ("29.399999618530302", 29.4_f32),
            ("71.79999923706049", 71.799995_f32),
            ("81.80000000000025", 81.8_f32),
            ("88.19999999999962", 88.2_f32),
            ("71.79999923706066", 71.799995_f32),
            ("44.79999923706067", 44.8_f32),
            ("182.40000915527307", 182.40001_f32),
            ("32.39999961853027", 32.4_f32),
            ("182.40000915527312", 182.40001_f32),
            ("81.79999999999973", 81.8_f32),
            ("44.79999923706052", 44.8_f32),
            ("107.70000457763668", 107.700005_f32),
            ("139.3637778884582", 139.36378_f32),
            ("17.949999809264735", 17.95_f32),
            ("35.89999961853027", 35.899998_f32),
            ("44.79999923706055", 44.8_f32),
            ("47.79999923706055", 47.8_f32),
            ("71.79999923706055", 71.799995_f32),
        ];
        for &(text, expected) in measured {
            assert_eq!(svg_number(text), expected, "{text}");
        }
    }

    /// `sloped_outline_height_f32` against Chromium: the `getBBox` height of a path `M0 -2b L0 b`
    /// (and of the sloped outline of `shape-themes/01`, whose parsed top and bottom are
    /// -71.79999542236328 and 35.89999771118164). The exact span is 3b; the reference's value
    /// is the float32 above the nearest one when 3b is not a float32.
    #[test]
    fn sloped_outline_height_matches_chromium() {
        let measured: &[(f64, f64)] = &[
            // The node of shape-themes/01: 3 * 35.89999771118164 is a half ulp off a float32.
            (35.89999771118164, 107.70000457763672),
            (20.184999465942383, 60.55500411987305),
            (22.219999313354492, 66.66000366210938),
            (23.145000457763672, 69.43500518798828),
        ];
        for &(bottom, expected) in measured {
            assert_eq!(
                sloped_outline_height_f32(-2.0 * bottom, bottom),
                expected,
                "bottom {bottom}"
            );
        }
        // An exact span is the browser's, unchanged: 3 * 20 = 60.
        assert_eq!(sloped_outline_height_f32(-40.0, 20.0), 60.0);
    }

    #[test]
    fn a_box_from_points_is_tight() {
        let points = [
            Point::new(3.0, -1.0),
            Point::new(-2.0, 4.0),
            Point::new(1.0, 1.0),
        ];
        assert_eq!(
            BBox::from_points(points),
            Some(BBox::new(-2.0, -1.0, 5.0, 5.0))
        );
        assert_eq!(BBox::from_points([]), None);
    }
}
