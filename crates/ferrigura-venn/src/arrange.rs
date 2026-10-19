//! `normalizeSolution` and `scaleSolution` (src/layout.js 591-799): the disjoint clusters of a
//! layout are each oriented, the clusters are arranged in a grid, and the whole is scaled into the
//! box with its padding and centred.

use std::collections::HashMap;
use std::f64::consts::PI;

use ferrigura_jsmath::{atan2, cos, sin};

use crate::Circle;
use crate::Error;
use crate::geometry::{Point, distance};
use crate::js::{JsObject, js_max, js_min, js_sort};

/// A circle with its set id: the entries of the library's `fromObjectNotation`.
#[derive(Clone, Debug)]
struct Placed {
    x: f64,
    y: f64,
    radius: f64,
    set: String,
}

impl Placed {
    fn centre(&self) -> Point {
        Point {
            x: self.x,
            y: self.y,
        }
    }
}

/// The extent of a set of circles, along x and along y.
#[derive(Clone, Copy, Debug)]
struct Bounds {
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
}

/// `getBoundingBox(circles)`: the reduces of `Math.max` and `Math.min` over the circles' extents.
fn bounds(circles: &[Placed]) -> Bounds {
    let mut x_max = f64::NEG_INFINITY;
    let mut x_min = f64::INFINITY;
    let mut y_max = f64::NEG_INFINITY;
    let mut y_min = f64::INFINITY;
    for circle in circles {
        x_max = js_max(x_max, circle.x + circle.radius);
        x_min = js_min(x_min, circle.x - circle.radius);
        y_max = js_max(y_max, circle.y + circle.radius);
        y_min = js_min(y_min, circle.y - circle.radius);
    }
    Bounds {
        x_min,
        x_max,
        y_min,
        y_max,
    }
}

/// `disjointCluster(circles)`: the groups of circles that overlap, by union-find, in the order of
/// their first circle. Two circles are joined when they are closer than the sum of their radii,
/// by more than 1e-10.
fn disjoint_cluster(circles: &[Placed]) -> Vec<Vec<Placed>> {
    let n = circles.len();
    let mut parent: Vec<usize> = (0..n).collect();

    fn find(parent: &mut [usize], i: usize) -> usize {
        if parent[i] != i {
            let root = find(parent, parent[i]);
            parent[i] = root;
        }
        parent[i]
    }

    for (i, first) in circles.iter().enumerate() {
        for (j, second) in circles.iter().enumerate().skip(i + 1) {
            let max_distance = first.radius + second.radius;
            if distance(first.centre(), second.centre()) + 1e-10 < max_distance {
                let x_root = find(&mut parent, j);
                let y_root = find(&mut parent, i);
                parent[x_root] = y_root;
            }
        }
    }

    let mut groups: Vec<Vec<Placed>> = Vec::new();
    let mut group_of_root: HashMap<usize, usize> = HashMap::new();
    for (i, circle) in circles.iter().enumerate() {
        let root = find(&mut parent, i);
        let group = *group_of_root.entry(root).or_insert_with(|| {
            groups.push(Vec::new());
            groups.len() - 1
        });
        groups[group].push(circle.clone());
    }
    groups
}

/// `orientateCircles(circles, orientation)`: sorts by radius, puts the largest at the origin,
/// turns the second circle to `orientation`, and mirrors a third that lies above the line.
fn orientate(circles: &mut [Placed], orientation: f64) {
    js_sort(circles, |a, b| b.radius - a.radius);

    if !circles.is_empty() {
        let largest_x = circles[0].x;
        let largest_y = circles[0].y;
        for circle in circles.iter_mut() {
            circle.x -= largest_x;
            circle.y -= largest_y;
        }
    }

    if circles.len() == 2 {
        let dist = distance(circles[0].centre(), circles[1].centre());
        if dist < (circles[1].radius - circles[0].radius).abs() {
            circles[1].x = circles[0].x + circles[0].radius - circles[1].radius - 1e-10;
            circles[1].y = circles[0].y;
        }
    }

    if circles.len() > 1 {
        let rotation = atan2(circles[1].x, circles[1].y) - orientation;
        let c = cos(rotation);
        let s = sin(rotation);
        for circle in circles.iter_mut() {
            let x = circle.x;
            let y = circle.y;
            circle.x = c * x - s * y;
            circle.y = s * x + c * y;
        }
    }

    if circles.len() > 2 {
        let mut angle = atan2(circles[2].x, circles[2].y) - orientation;
        while angle < 0.0 {
            angle += 2.0 * PI;
        }
        while angle > 2.0 * PI {
            angle -= 2.0 * PI;
        }
        if angle > PI {
            let slope = circles[1].y / (1e-10 + circles[1].x);
            for circle in circles.iter_mut() {
                let d = (circle.x + slope * circle.y) / (1.0 + slope * slope);
                circle.x = 2.0 * d - circle.x;
                circle.y = 2.0 * d * slope - circle.y;
            }
        }
    }
}

/// A disjoint cluster, oriented, with its bounds and its area of bounds.
struct Cluster {
    circles: Vec<Placed>,
    bounds: Bounds,
    size: f64,
}

/// `addCluster(cluster, right, bottom)`: moves a cluster beside the placed circles, to the right
/// or below, and centres it on the first cluster's side when it is smaller.
fn add_cluster(
    placed: &mut Vec<Placed>,
    cluster: Option<&Cluster>,
    right: bool,
    bottom: bool,
    reference: Bounds,
    spacing: f64,
) {
    let Some(cluster) = cluster else {
        return;
    };
    let b = cluster.bounds;

    let x_offset = if right {
        reference.x_max - b.x_min + spacing
    } else {
        let mut offset = reference.x_max - b.x_max;
        let centreing = (b.x_max - b.x_min) / 2.0 - (reference.x_max - reference.x_min) / 2.0;
        if centreing < 0.0 {
            offset += centreing;
        }
        offset
    };

    let y_offset = if bottom {
        reference.y_max - b.y_min + spacing
    } else {
        let mut offset = reference.y_max - b.y_max;
        let centreing = (b.y_max - b.y_min) / 2.0 - (reference.y_max - reference.y_min) / 2.0;
        if centreing < 0.0 {
            offset += centreing;
        }
        offset
    };

    for circle in &cluster.circles {
        let mut moved = circle.clone();
        moved.x += x_offset;
        moved.y += y_offset;
        placed.push(moved);
    }
}

/// `normalizeSolution(solution, orientation)`: the solution with its clusters oriented and
/// arranged in a grid, the largest cluster first.
pub(crate) fn normalize(
    solution: &JsObject<Circle>,
    orientation: f64,
) -> Result<JsObject<Circle>, Error> {
    let circles: Vec<Placed> = solution
        .iter()
        .map(|(set, circle)| Placed {
            x: circle.x,
            y: circle.y,
            radius: circle.radius,
            set: set.to_owned(),
        })
        .collect();

    let mut clusters: Vec<Cluster> = disjoint_cluster(&circles)
        .into_iter()
        .map(|mut circles| {
            orientate(&mut circles, orientation);
            let bounds = bounds(&circles);
            let size = (bounds.x_max - bounds.x_min) * (bounds.y_max - bounds.y_min);
            Cluster {
                circles,
                bounds,
                size,
            }
        })
        .collect();
    js_sort(&mut clusters, |a, b| b.size - a.size);

    if clusters.is_empty() {
        return Err(Error::NoAreas);
    }
    let first = clusters.remove(0);
    let rest = clusters;
    let mut placed = first.circles;
    let mut reference = first.bounds;
    let spacing = (reference.x_max - reference.x_min) / 50.0;

    let mut index = 0;
    while index < rest.len() {
        add_cluster(
            &mut placed,
            rest.get(index),
            true,
            false,
            reference,
            spacing,
        );
        add_cluster(
            &mut placed,
            rest.get(index + 1),
            false,
            true,
            reference,
            spacing,
        );
        add_cluster(
            &mut placed,
            rest.get(index + 2),
            true,
            true,
            reference,
            spacing,
        );
        index += 3;
        reference = bounds(&placed);
    }

    let mut result = JsObject::new();
    for circle in &placed {
        result.insert(
            &circle.set,
            Circle {
                x: circle.x,
                y: circle.y,
                radius: circle.radius,
            },
        );
    }
    Ok(result)
}

/// `scaleSolution(solution, width, height, padding)` with `scaleToFit` off: the solution scaled
/// to the box inside its padding and centred. A solution with no extent is returned as it is.
pub(crate) fn scale(
    solution: &JsObject<Circle>,
    width: f64,
    height: f64,
    padding: f64,
) -> JsObject<Circle> {
    let circles: Vec<Placed> = solution
        .iter()
        .map(|(set, circle)| Placed {
            x: circle.x,
            y: circle.y,
            radius: circle.radius,
            set: set.to_owned(),
        })
        .collect();

    let width = width - 2.0 * padding;
    let height = height - 2.0 * padding;

    let extent = bounds(&circles);
    if extent.x_max == extent.x_min || extent.y_max == extent.y_min {
        return solution.clone();
    }

    let x_scaling = width / (extent.x_max - extent.x_min);
    let y_scaling = height / (extent.y_max - extent.y_min);

    let scaling = js_min(y_scaling, x_scaling);
    let x_offset = (width - (extent.x_max - extent.x_min) * scaling) / 2.0;
    let y_offset = (height - (extent.y_max - extent.y_min) * scaling) / 2.0;

    let mut result = JsObject::new();
    for circle in &circles {
        result.insert(
            &circle.set,
            Circle {
                x: padding + x_offset + (circle.x - extent.x_min) * scaling,
                y: padding + y_offset + (circle.y - extent.y_min) * scaling,
                radius: scaling * circle.radius,
            },
        );
    }
    result
}
