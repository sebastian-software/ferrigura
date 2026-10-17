//! Geometry of layout-base 1.0.2: `util/RectangleD.js`, `util/Transform.js`, `util/PointD.js`,
//! `util/IGeometry.js` (the functions the tree path calls) and `util/IMath.js`.
//!
//! Every expression keeps the operand order of the JavaScript, so the results are the same bits.
//! `Math.min` and `Math.max` are [`js_min`] and [`js_max`], which order the two zeros as JavaScript
//! does.

use std::f64::consts::PI;

/// `IGeometry.TWO_PI` (`IGeometry.js`, the last lines).
pub(crate) const TWO_PI: f64 = 2.0 * PI;

/// `Math.min` of two numbers: NaN wins, and -0 is smaller than +0.
pub(crate) fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == b {
        if a.is_sign_negative() { a } else { b }
    } else if a < b {
        a
    } else {
        b
    }
}

/// `Math.max` of two numbers: NaN wins, and +0 is larger than -0.
pub(crate) fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == b {
        if a.is_sign_positive() { a } else { b }
    } else if a > b {
        a
    } else {
        b
    }
}

/// `IMath.sign`: 1, -1 or 0 (also for -0 and NaN).
pub(crate) fn sign(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// `PointD`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct PointD {
    pub x: f64,
    pub y: f64,
}

impl PointD {
    pub(crate) fn new(x: f64, y: f64) -> PointD {
        PointD { x, y }
    }
}

/// `RectangleD`: the rectangle of a node, its upper left corner and its size.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct RectD {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl RectD {
    pub(crate) fn new(x: f64, y: f64, width: f64, height: f64) -> RectD {
        RectD {
            x,
            y,
            width,
            height,
        }
    }

    /// `getRight`: `x + width`.
    pub(crate) fn right(&self) -> f64 {
        self.x + self.width
    }

    /// `getBottom`: `y + height`.
    pub(crate) fn bottom(&self) -> f64 {
        self.y + self.height
    }

    /// `getCenterX`: `x + width / 2`.
    pub(crate) fn center_x(&self) -> f64 {
        self.x + self.width / 2.0
    }

    /// `getCenterY`: `y + height / 2`.
    pub(crate) fn center_y(&self) -> f64 {
        self.y + self.height / 2.0
    }

    /// `intersects`: the two rectangles share a point, edges included.
    pub(crate) fn intersects(&self, other: &RectD) -> bool {
        !(self.right() < other.x
            || self.bottom() < other.y
            || other.right() < self.x
            || other.bottom() < self.y)
    }
}

/// `Transform` with the extents of layout-base's defaults (both 1) as the tree path uses them.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Transform {
    world_org: PointD,
    device_org: PointD,
    world_ext: PointD,
    device_ext: PointD,
}

impl Transform {
    /// The transform that maps `device_org` to `world_org`, with unit extents.
    pub(crate) fn new(world_org: PointD, device_org: PointD) -> Transform {
        Transform {
            world_org,
            device_org,
            world_ext: PointD::new(1.0, 1.0),
            device_ext: PointD::new(1.0, 1.0),
        }
    }

    /// `inverseTransformX` (`Transform.js`): the world x of a device x.
    fn inverse_x(&self, x: f64) -> f64 {
        if self.device_ext.x != 0.0 {
            self.world_org.x + ((x - self.device_org.x) * self.world_ext.x / self.device_ext.x)
        } else {
            0.0
        }
    }

    /// `inverseTransformY`.
    fn inverse_y(&self, y: f64) -> f64 {
        if self.device_ext.y != 0.0 {
            self.world_org.y + ((y - self.device_org.y) * self.world_ext.y / self.device_ext.y)
        } else {
            0.0
        }
    }

    /// `inverseTransformPoint`.
    pub(crate) fn inverse_transform_point(&self, point: PointD) -> PointD {
        PointD::new(self.inverse_x(point.x), self.inverse_y(point.y))
    }
}

/// `IGeometry.calcSeparationAmount` (`IGeometry.js` 21-111): the displacement that separates two
/// overlapping rectangles, each axis with `separation_buffer` added. The rectangles must intersect.
pub(crate) fn calc_separation_amount(
    rect_a: &RectD,
    rect_b: &RectD,
    separation_buffer: f64,
) -> [f64; 2] {
    debug_assert!(
        rect_a.intersects(rect_b),
        "assert failed: the rectangles must intersect"
    );
    let (direction_x, direction_y) = decide_directions(rect_a, rect_b);
    let mut overlap = [0.0_f64; 2];
    overlap[0] = js_min(rect_a.right(), rect_b.right()) - js_max(rect_a.x, rect_b.x);
    overlap[1] = js_min(rect_a.bottom(), rect_b.bottom()) - js_max(rect_a.y, rect_b.y);
    if rect_a.x <= rect_b.x && rect_a.right() >= rect_b.right() {
        overlap[0] += js_min(rect_b.x - rect_a.x, rect_a.right() - rect_b.right());
    } else if rect_b.x <= rect_a.x && rect_b.right() >= rect_a.right() {
        overlap[0] += js_min(rect_a.x - rect_b.x, rect_b.right() - rect_a.right());
    }
    if rect_a.y <= rect_b.y && rect_a.bottom() >= rect_b.bottom() {
        overlap[1] += js_min(rect_b.y - rect_a.y, rect_a.bottom() - rect_b.bottom());
    } else if rect_b.y <= rect_a.y && rect_b.bottom() >= rect_a.bottom() {
        overlap[1] += js_min(rect_a.y - rect_b.y, rect_b.bottom() - rect_a.bottom());
    }
    let mut slope =
        ((rect_b.center_y() - rect_a.center_y()) / (rect_b.center_x() - rect_a.center_x())).abs();
    if rect_b.center_y() == rect_a.center_y() && rect_b.center_x() == rect_a.center_x() {
        slope = 1.0;
    }
    let mut move_by_y = slope * overlap[0];
    let mut move_by_x = overlap[1] / slope;
    if overlap[0] < move_by_x {
        move_by_x = overlap[0];
    } else {
        move_by_y = overlap[1];
    }
    overlap[0] = -direction_x * ((move_by_x / 2.0) + separation_buffer);
    overlap[1] = -direction_y * ((move_by_y / 2.0) + separation_buffer);
    overlap
}

/// `IGeometry.decideDirectionsForOverlappingNodes` (`IGeometry.js` 58-70).
fn decide_directions(rect_a: &RectD, rect_b: &RectD) -> (f64, f64) {
    let x = if rect_a.center_x() < rect_b.center_x() {
        -1.0
    } else {
        1.0
    };
    let y = if rect_a.center_y() < rect_b.center_y() {
        -1.0
    } else {
        1.0
    };
    (x, y)
}

/// `IGeometry.getCardinalDirection` (`IGeometry.js` 428-437).
fn cardinal_direction(slope: f64, slope_prime: f64, line: i32) -> i32 {
    if slope > slope_prime {
        line
    } else {
        1 + line % 4
    }
}

/// `IGeometry.getIntersection2` (`IGeometry.js` 113-431) with the rectangles as arguments: the two
/// clip points of the segment between the centres, on the boundary of each rectangle. Returns
/// whether the rectangles overlap, in which case the clip points are the centres. The result is
/// `[ax, ay, bx, by]`.
pub(crate) fn intersection_clip(rect_a: &RectD, rect_b: &RectD) -> (bool, [f64; 4]) {
    let p1x = rect_a.center_x();
    let p1y = rect_a.center_y();
    let p2x = rect_b.center_x();
    let p2y = rect_b.center_y();
    if rect_a.intersects(rect_b) {
        return (true, [p1x, p1y, p2x, p2y]);
    }
    let mut result = [0.0_f64; 4];
    let top_left_ax = rect_a.x;
    let top_left_ay = rect_a.y;
    let top_right_ax = rect_a.right();
    let bottom_left_ax = rect_a.x;
    let bottom_left_ay = rect_a.bottom();
    let bottom_right_ax = rect_a.right();
    let half_width_a = rect_a.width / 2.0;
    let half_height_a = rect_a.height / 2.0;
    let top_left_bx = rect_b.x;
    let top_left_by = rect_b.y;
    let top_right_bx = rect_b.right();
    let bottom_left_bx = rect_b.x;
    let bottom_left_by = rect_b.bottom();
    let bottom_right_bx = rect_b.right();
    let half_width_b = rect_b.width / 2.0;
    let half_height_b = rect_b.height / 2.0;

    if p1x == p2x {
        if p1y > p2y {
            result = [p1x, top_left_ay, p2x, bottom_left_by];
            return (false, result);
        } else if p1y < p2y {
            result = [p1x, bottom_left_ay, p2x, top_left_by];
            return (false, result);
        }
    } else if p1y == p2y {
        if p1x > p2x {
            result = [top_left_ax, p1y, top_right_bx, p2y];
            return (false, result);
        } else if p1x < p2x {
            result = [top_right_ax, p1y, top_left_bx, p2y];
            return (false, result);
        }
    } else {
        let slope_a = rect_a.height / rect_a.width;
        let slope_b = rect_b.height / rect_b.width;
        let slope_prime = (p2y - p1y) / (p2x - p1x);
        let mut clip_a_found = false;
        let mut clip_b_found = false;

        if -slope_a == slope_prime {
            if p1x > p2x {
                result[0] = bottom_left_ax;
                result[1] = bottom_left_ay;
            } else {
                result[0] = top_right_ax;
                result[1] = top_left_ay;
            }
            clip_a_found = true;
        } else if slope_a == slope_prime {
            if p1x > p2x {
                result[0] = top_left_ax;
                result[1] = top_left_ay;
            } else {
                result[0] = bottom_right_ax;
                result[1] = bottom_left_ay;
            }
            clip_a_found = true;
        }
        if -slope_b == slope_prime {
            if p2x > p1x {
                result[2] = bottom_left_bx;
                result[3] = bottom_left_by;
            } else {
                result[2] = top_right_bx;
                result[3] = top_left_by;
            }
            clip_b_found = true;
        } else if slope_b == slope_prime {
            if p2x > p1x {
                result[2] = top_left_bx;
                result[3] = top_left_by;
            } else {
                result[2] = bottom_right_bx;
                result[3] = bottom_left_by;
            }
            clip_b_found = true;
        }
        if clip_a_found && clip_b_found {
            return (false, result);
        }

        let (cardinal_a, cardinal_b) = if p1x > p2x {
            if p1y > p2y {
                (
                    cardinal_direction(slope_a, slope_prime, 4),
                    cardinal_direction(slope_b, slope_prime, 2),
                )
            } else {
                (
                    cardinal_direction(-slope_a, slope_prime, 3),
                    cardinal_direction(-slope_b, slope_prime, 1),
                )
            }
        } else if p1y > p2y {
            (
                cardinal_direction(-slope_a, slope_prime, 1),
                cardinal_direction(-slope_b, slope_prime, 3),
            )
        } else {
            (
                cardinal_direction(slope_a, slope_prime, 2),
                cardinal_direction(slope_b, slope_prime, 4),
            )
        };

        if !clip_a_found {
            match cardinal_a {
                1 => {
                    result[1] = top_left_ay;
                    result[0] = p1x + (-half_height_a) / slope_prime;
                }
                2 => {
                    result[0] = bottom_right_ax;
                    result[1] = p1y + half_width_a * slope_prime;
                }
                3 => {
                    result[1] = bottom_left_ay;
                    result[0] = p1x + half_height_a / slope_prime;
                }
                4 => {
                    result[0] = bottom_left_ax;
                    result[1] = p1y + (-half_width_a) * slope_prime;
                }
                _ => {}
            }
        }
        if !clip_b_found {
            match cardinal_b {
                1 => {
                    result[3] = top_left_by;
                    result[2] = p2x + (-half_height_b) / slope_prime;
                }
                2 => {
                    result[2] = bottom_right_bx;
                    result[3] = p2y + half_width_b * slope_prime;
                }
                3 => {
                    result[3] = bottom_left_by;
                    result[2] = p2x + half_height_b / slope_prime;
                }
                4 => {
                    result[2] = bottom_left_bx;
                    result[3] = p2y + (-half_width_b) * slope_prime;
                }
                _ => {}
            }
        }
    }
    (false, result)
}
