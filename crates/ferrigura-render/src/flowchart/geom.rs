//! Points and boxes shared by the flowchart modules. Coordinates are SVG user units, y down.

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
        (self.x - other.x).hypot(self.y - other.y)
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
    pub fn from_points(points: impl IntoIterator<Item = Point>) -> Option<Self> {
        let mut points = points.into_iter();
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
