//! The elements a gitGraph draws, as a tree that knows the box each element has to `getBBox`.
//!
//! The reference reads boxes from its drawing: the box of each label text (the label is placed by
//! it), the title's place and the viewBox, which is the box of the root. Chromium gives an element
//! its own box in its own user space, a container the union of its children's boxes, each mapped
//! through the child's transform, and a transformed box the box of its four corners. An empty box
//! (no width, or no height) is united too, so a vertical branch line reaches its end.

use crate::svg::{escape, number};
use crate::unified::geom::{BBox, Point};

/// An affine transform, as a `transform` attribute lists it: the text and its matrix `a b c d e f`.
#[derive(Debug, Clone, PartialEq)]
pub struct Transform {
    text: String,
    matrix: [f64; 6],
}

impl Transform {
    /// `translate(x, y)`.
    #[must_use]
    pub fn translate(x: f64, y: f64) -> Self {
        Self {
            text: format!("translate({}, {})", number(x), number(y)),
            matrix: [1.0, 0.0, 0.0, 1.0, x, y],
        }
    }

    /// `rotate(degrees, cx, cy)`: a rotation about the point `(cx, cy)`.
    #[must_use]
    pub fn rotate(degrees: f64, cx: f64, cy: f64) -> Self {
        let radians = degrees.to_radians();
        let (sin, cos) = radians.sin_cos();
        let rotation = [cos, sin, -sin, cos, 0.0, 0.0];
        let matrix = multiply(
            &multiply(&[1.0, 0.0, 0.0, 1.0, cx, cy], &rotation),
            &[1.0, 0.0, 0.0, 1.0, -cx, -cy],
        );
        Self {
            text: format!(
                "rotate({}, {}, {})",
                number(degrees),
                number(cx),
                number(cy)
            ),
            matrix,
        }
    }

    /// This transform followed by `next` in the attribute: `translate(…) rotate(…)`.
    #[must_use]
    pub fn then(self, next: Self) -> Self {
        Self {
            text: format!("{} {}", self.text, next.text),
            matrix: multiply(&self.matrix, &next.matrix),
        }
    }

    /// The box of the four corners of `bounds`, mapped by the transform.
    fn map_box(&self, bounds: BBox) -> BBox {
        let [a, b, c, d, e, f] = self.matrix;
        let corners = [
            (bounds.x, bounds.y),
            (bounds.right(), bounds.y),
            (bounds.x, bounds.bottom()),
            (bounds.right(), bounds.bottom()),
        ]
        .map(|(x, y)| (a * x + c * y + e, b * x + d * y + f));
        let left = corners
            .iter()
            .map(|corner| corner.0)
            .fold(f64::INFINITY, f64::min);
        let top = corners
            .iter()
            .map(|corner| corner.1)
            .fold(f64::INFINITY, f64::min);
        let right = corners
            .iter()
            .map(|corner| corner.0)
            .fold(f64::NEG_INFINITY, f64::max);
        let bottom = corners
            .iter()
            .map(|corner| corner.1)
            .fold(f64::NEG_INFINITY, f64::max);
        BBox::new(left, top, right - left, bottom - top)
    }
}

/// `m1 · m2` for affine matrices `a b c d e f`.
fn multiply(m1: &[f64; 6], m2: &[f64; 6]) -> [f64; 6] {
    [
        m1[0] * m2[0] + m1[2] * m2[1],
        m1[1] * m2[0] + m1[3] * m2[1],
        m1[0] * m2[2] + m1[2] * m2[3],
        m1[1] * m2[2] + m1[3] * m2[3],
        m1[0] * m2[4] + m1[2] * m2[5] + m1[4],
        m1[1] * m2[4] + m1[3] * m2[5] + m1[5],
    ]
}

/// What `getBBox` gives an element.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Bounds {
    /// Not drawn, so no box: `title`, `desc`, `defs` and the like.
    None,
    /// A drawn leaf, with its own box in its own user space.
    Own(BBox),
    /// A container: the union of its children.
    Children,
}

/// An SVG element of the diagram. Attributes are kept in the order they are written.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    tag: &'static str,
    attributes: Vec<(&'static str, String)>,
    transform: Option<Transform>,
    bounds: Bounds,
    content: Option<String>,
    children: Vec<Element>,
}

impl Element {
    /// A container with no attributes: `g`.
    #[must_use]
    pub fn group() -> Self {
        Self {
            tag: "g",
            attributes: Vec::new(),
            transform: None,
            bounds: Bounds::Children,
            content: None,
            children: Vec::new(),
        }
    }

    /// An element that is written but has no box: `defs`, `filter` and the like.
    #[must_use]
    pub fn hidden(tag: &'static str) -> Self {
        Self {
            tag,
            attributes: Vec::new(),
            transform: None,
            bounds: Bounds::None,
            content: None,
            children: Vec::new(),
        }
    }

    /// A rectangle with its top-left corner `(x, y)`.
    #[must_use]
    pub fn rect(x: f64, y: f64, width: f64, height: f64) -> Self {
        let mut element = Self::leaf("rect", BBox::new(x, y, width, height));
        element.attributes.extend(number_attributes(&[
            ("x", x),
            ("y", y),
            ("width", width),
            ("height", height),
        ]));
        element
    }

    /// A circle.
    #[must_use]
    pub fn circle(cx: f64, cy: f64, r: f64) -> Self {
        let mut element = Self::leaf("circle", BBox::new(cx - r, cy - r, 2.0 * r, 2.0 * r));
        element
            .attributes
            .extend(number_attributes(&[("cx", cx), ("cy", cy), ("r", r)]));
        element
    }

    /// A line from `(x1, y1)` to `(x2, y2)`.
    #[must_use]
    pub fn line(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        let bounds =
            BBox::from_points([Point::new(x1, y1), Point::new(x2, y2)]).unwrap_or_default();
        let mut element = Self::leaf("line", bounds);
        element.attributes.extend(number_attributes(&[
            ("x1", x1),
            ("y1", y1),
            ("x2", x2),
            ("y2", y2),
        ]));
        element
    }

    /// A polygon through the points, in the order given.
    #[must_use]
    pub fn polygon(points: &[Point]) -> Self {
        let bounds = BBox::from_points(points.iter().copied()).unwrap_or_default();
        let mut element = Self::leaf("polygon", bounds);
        element.attributes.push(("points", points_text(points)));
        element
    }

    /// A path with the given `d` and box.
    #[must_use]
    pub fn path(d: String, bounds: BBox) -> Self {
        let mut element = Self::leaf("path", bounds);
        element.attributes.push(("d", d));
        element
    }

    /// A `text` element with its content and its box, which the caller measured.
    #[must_use]
    pub fn text(content: &str, bounds: BBox) -> Self {
        let mut element = Self::leaf("text", bounds);
        element.content = Some(content.to_owned());
        element
    }

    /// A `text` element whose content is its `tspan` children, with the box of all of them.
    #[must_use]
    pub fn text_lines(bounds: BBox) -> Self {
        Self::leaf("text", bounds)
    }

    /// A `tspan` of a multi-line label: a row of text, placed at `x = 0` one line down.
    #[must_use]
    pub fn tspan(content: &str) -> Self {
        Self::hidden("tspan")
            .attr("xml:space", "preserve")
            .attr("dy", "1em")
            .attr("x", "0")
            .class("row")
            .with_content(content)
    }

    /// Sets the text content of the element.
    #[must_use]
    pub fn with_content(mut self, content: &str) -> Self {
        self.content = Some(content.to_owned());
        self
    }

    /// A container with a tag of its own: the root `svg`, which is the union of its children.
    #[must_use]
    pub fn container(tag: &'static str) -> Self {
        Self {
            tag,
            ..Self::group()
        }
    }

    /// Appends the text of every `text` and `tspan` below the element, one after another, to find
    /// the characters the diagram draws.
    pub fn collect_text(&self, out: &mut String) {
        if let (true, Some(content)) = (matches!(self.tag, "text" | "tspan"), &self.content) {
            out.push_str(content);
            out.push(' ');
        }
        for child in &self.children {
            child.collect_text(out);
        }
    }

    /// Writes the children of the element as markup, without its own tags.
    pub fn write_children(&self, out: &mut String) {
        for child in &self.children {
            child.write(out);
        }
    }

    fn leaf(tag: &'static str, bounds: BBox) -> Self {
        Self {
            tag,
            attributes: Vec::new(),
            transform: None,
            bounds: Bounds::Own(bounds),
            content: None,
            children: Vec::new(),
        }
    }

    /// Adds an attribute.
    #[must_use]
    pub fn attr(mut self, name: &'static str, value: impl Into<String>) -> Self {
        self.attributes.push((name, value.into()));
        self
    }

    /// Sets the class attribute.
    #[must_use]
    pub fn class(self, value: impl Into<String>) -> Self {
        self.attr("class", value)
    }

    /// Sets the transform, which both the attribute and the box use.
    #[must_use]
    pub fn transform(mut self, transform: Transform) -> Self {
        self.attributes.push(("transform", transform.text.clone()));
        self.transform = Some(transform);
        self
    }

    /// Adds a child.
    pub fn push(&mut self, child: Element) {
        self.children.push(child);
    }

    /// Adds a child, for chaining.
    #[must_use]
    pub fn with(mut self, child: Element) -> Self {
        self.children.push(child);
        self
    }

    /// The box of the element in its own user space, as `getBBox` gives it, or `None` when it
    /// has none.
    #[must_use]
    pub fn local_box(&self) -> Option<BBox> {
        match self.bounds {
            Bounds::None => None,
            Bounds::Own(bounds) => Some(bounds),
            Bounds::Children => union_all(self.children.iter().filter_map(Element::parent_box)),
        }
    }

    /// The box of the element in its parent's user space: its own box, mapped by its transform.
    #[must_use]
    pub fn parent_box(&self) -> Option<BBox> {
        let local = self.local_box()?;
        Some(match &self.transform {
            Some(transform) => transform.map_box(local),
            None => local,
        })
    }

    /// Writes the element as markup, with its children.
    pub fn write(&self, out: &mut String) {
        out.push('<');
        out.push_str(self.tag);
        for (name, value) in &self.attributes {
            out.push_str(&format!(" {name}=\"{}\"", escape(value)));
        }
        if self.content.is_none() && self.children.is_empty() {
            out.push_str("/>");
            return;
        }
        out.push('>');
        if let Some(content) = &self.content {
            out.push_str(&escape(content));
        }
        for child in &self.children {
            child.write(out);
        }
        out.push_str(&format!("</{}>", self.tag));
    }
}

/// The union of the boxes, empty ones included: a vertical branch line has no width and still
/// extends the drawing down to its end, as `UniteEvenIfEmpty` makes it. `None` when there are none.
fn union_all(boxes: impl Iterator<Item = BBox>) -> Option<BBox> {
    boxes.reduce(BBox::union)
}

/// The attributes of numbers, written as the reference writes them.
fn number_attributes(pairs: &[(&'static str, f64)]) -> Vec<(&'static str, String)> {
    pairs
        .iter()
        .map(|(name, value)| (*name, number(*value)))
        .collect()
}

/// The `points` attribute: each point as `x,y`, separated by spaces.
fn points_text(points: &[Point]) -> String {
    points
        .iter()
        .map(|point| format!("{},{}", number(point.x), number(point.y)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// A path under construction: its `d` text and the box of its points, arcs included.
#[derive(Debug, Clone)]
pub struct PathBuilder {
    d: String,
    current: Point,
    points: Vec<Point>,
}

impl PathBuilder {
    /// Starts a path at `start`, as `M`.
    #[must_use]
    pub fn new(start: Point) -> Self {
        Self {
            d: format!("M {} {}", number(start.x), number(start.y)),
            current: start,
            points: vec![start],
        }
    }

    /// A straight line to `to`, as `L`.
    #[must_use]
    pub fn line(mut self, to: Point) -> Self {
        self.d
            .push_str(&format!(" L {} {}", number(to.x), number(to.y)));
        self.points.push(to);
        self.current = to;
        self
    }

    /// A circular arc of radius `radius` to `to`, drawn clockwise when `sweep` is set and
    /// counter-clockwise otherwise, as `A r r, 0, 0, sweep,`.
    #[must_use]
    pub fn arc(mut self, radius: f64, sweep: bool, to: Point) -> Self {
        let flag = u8::from(sweep);
        self.d.push_str(&format!(
            " A {r} {r}, 0, 0, {flag}, {x} {y}",
            r = number(radius),
            x = number(to.x),
            y = number(to.y)
        ));
        self.points
            .extend(arc_extremes(self.current, to, radius, sweep));
        self.points.push(to);
        self.current = to;
        self
    }

    /// The `d` text and the box of the path.
    #[must_use]
    pub fn finish(self) -> (String, BBox) {
        let bounds = BBox::from_points(self.points).unwrap_or_default();
        (self.d, bounds)
    }
}

/// The points of a circular arc of radius `radius` from `from` to `to` that lie at the four
/// axis-aligned extremes of its circle, when the arc passes them. Together with the end points,
/// they hold the arc's tight box.
fn arc_extremes(from: Point, to: Point, radius: f64, sweep: bool) -> Vec<Point> {
    let (x1, y1, x2, y2) = (from.x, from.y, to.x, to.y);
    let half_x = (x1 - x2) / 2.0;
    let half_y = (y1 - y2) / 2.0;
    let mut r = radius.abs();
    let lambda = (half_x * half_x + half_y * half_y) / (r * r);
    if lambda > 1.0 {
        r *= lambda.sqrt();
    }
    let numerator = r * r * r * r - r * r * half_y * half_y - r * r * half_x * half_x;
    let denominator = r * r * half_y * half_y + r * r * half_x * half_x;
    // The large-arc flag is 0 in every arc the diagram draws. The sign of the centre's offset is
    // `+` when the sweep flag is 1 and `-` when it is 0 (SVG 1.1, F.6.5 with fA = 0).
    let root = if denominator > 0.0 {
        (numerator / denominator).max(0.0).sqrt()
    } else {
        0.0
    };
    let sign = if sweep { 1.0 } else { -1.0 };
    let (cxp, cyp) = (sign * root * r * half_y / r, sign * root * -r * half_x / r);
    let cx = cxp + (x1 + x2) / 2.0;
    let cy = cyp + (y1 + y2) / 2.0;
    let start = ((half_x - cxp) / r, (half_y - cyp) / r);
    let end = ((-half_x - cxp) / r, (-half_y - cyp) / r);
    let theta = start.1.atan2(start.0);
    let mut delta = end.1.atan2(end.0) - theta;
    let tau = std::f64::consts::TAU;
    if !sweep && delta > 0.0 {
        delta -= tau;
    }
    if sweep && delta < 0.0 {
        delta += tau;
    }
    (0..4)
        .map(|quarter| f64::from(quarter) * std::f64::consts::FRAC_PI_2)
        .filter(|angle| {
            let offset = if delta >= 0.0 {
                (angle - theta).rem_euclid(tau)
            } else {
                (theta - angle).rem_euclid(tau)
            };
            offset <= delta.abs()
        })
        .map(|angle| Point::new(cx + r * angle.cos(), cy + r * angle.sin()))
        .collect()
}
