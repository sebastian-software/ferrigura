//! The text centres of `src/diagram.js` (445-610): for each area, the point that lies furthest
//! inside the area's own circles and outside the others, found from the best of a few sampled
//! points by the simplex.

use std::collections::{HashMap, HashSet};

use crate::Area;
use crate::Centre;
use crate::Circle;
use crate::Error;
use crate::fmin::{self, TEXT_TOLERANCE};
use crate::geometry::{Point, centre, distance, get_center, intersection_area};
use crate::js::{JsObject, js_has};

/// `circleMargin(current, interior, exterior)`: the smallest margin of the point inside every
/// interior circle and outside every exterior one. Ties take the later circle.
fn circle_margin(current: Point, interior: &[Circle], exterior: &[Circle]) -> f64 {
    let mut margin = interior[0].radius - distance(centre(&interior[0]), current);

    for circle in &interior[1..] {
        let m = circle.radius - distance(centre(circle), current);
        if m <= margin {
            margin = m;
        }
    }

    for circle in exterior {
        let m = distance(centre(circle), current) - circle.radius;
        if m <= margin {
            margin = m;
        }
    }
    margin
}

/// `computeTextCentre(interior, exterior)`. `interior` must not be empty.
pub(crate) fn compute_text_centre(interior: &[Circle], exterior: &[Circle]) -> Centre {
    let mut points: Vec<Point> = Vec::new();
    for c in interior {
        points.push(Point { x: c.x, y: c.y });
        points.push(Point {
            x: c.x + c.radius / 2.0,
            y: c.y,
        });
        points.push(Point {
            x: c.x - c.radius / 2.0,
            y: c.y,
        });
        points.push(Point {
            x: c.x,
            y: c.y + c.radius / 2.0,
        });
        points.push(Point {
            x: c.x,
            y: c.y - c.radius / 2.0,
        });
    }

    let mut initial = points[0];
    let mut margin = circle_margin(points[0], interior, exterior);
    for point in &points[1..] {
        let m = circle_margin(*point, interior, exterior);
        if m >= margin {
            initial = *point;
            margin = m;
        }
    }

    let solution = fmin::nelder_mead(
        |p| -circle_margin(Point { x: p[0], y: p[1] }, interior, exterior),
        &[initial.x, initial.y],
        TEXT_TOLERANCE,
    );
    let ret = Point {
        x: solution.x[0],
        y: solution.x[1],
    };

    // The library tests with `>` and `<`, so a NaN distance counts as inside.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    let valid = interior
        .iter()
        .all(|i| !(distance(ret, centre(i)) > i.radius))
        && exterior
            .iter()
            .all(|e| !(distance(ret, centre(e)) < e.radius));
    if valid {
        return Centre {
            x: ret.x,
            y: ret.y,
            disjoint: false,
        };
    }

    if interior.len() == 1 {
        return Centre {
            x: interior[0].x,
            y: interior[0].y,
            disjoint: false,
        };
    }

    let stats = intersection_area(interior);
    if stats.arcs.is_empty() {
        return Centre {
            x: 0.0,
            y: -1000.0,
            disjoint: true,
        };
    }
    if stats.arcs.len() == 1 {
        return Centre {
            x: stats.arcs[0].circle.x,
            y: stats.arcs[0].circle.y,
            disjoint: false,
        };
    }
    if !exterior.is_empty() {
        return compute_text_centre(interior, &[]);
    }
    let points: Vec<Point> = stats.arcs.iter().map(|arc| arc.p1).collect();
    let average = get_center(&points);
    Centre {
        x: average.x,
        y: average.y,
        disjoint: false,
    }
}

/// `getOverlappingCircles(circles)`: for each circle (by position in `Object.keys` order), the
/// positions of the circles that contain it.
fn overlapping_circles(order: &[(&str, Circle)]) -> Vec<Vec<usize>> {
    let mut containers: Vec<Vec<usize>> = vec![Vec::new(); order.len()];
    for i in 0..order.len() {
        for j in (i + 1)..order.len() {
            let a = order[i].1;
            let b = order[j].1;
            let d = distance(centre(&a), centre(&b));
            if d + b.radius <= a.radius + 1e-10 {
                containers[j].push(i);
            } else if d + a.radius <= b.radius + 1e-10 {
                containers[i].push(j);
            }
        }
    }
    containers
}

/// `computeTextCentres(circles, areas)`: the text centre of each area, keyed by its sets joined
/// with commas, in the order the library's object gives its keys.
pub(crate) fn compute_text_centres(
    circles: &JsObject<Circle>,
    areas: &[Area],
) -> Result<JsObject<Centre>, Error> {
    let order: Vec<(&str, Circle)> = circles.iter().map(|(set, circle)| (set, *circle)).collect();
    let position: HashMap<&str, usize> = order
        .iter()
        .enumerate()
        .map(|(i, (set, _))| (*set, i))
        .collect();
    let containers = overlapping_circles(&order);

    let mut result = JsObject::new();
    for area in areas {
        let mut area_ids: HashSet<String> = HashSet::new();
        let mut exclude: HashSet<String> = HashSet::new();
        for set in &area.sets {
            let at = *position
                .get(set.as_str())
                .ok_or_else(|| Error::UnknownSet(set.clone()))?;
            area_ids.insert(set.clone());
            for &container in &containers[at] {
                exclude.insert(order[container].0.to_owned());
            }
        }

        let mut interior: Vec<Circle> = Vec::new();
        let mut exterior: Vec<Circle> = Vec::new();
        for (set, circle) in &order {
            if js_has(&area_ids, set) {
                interior.push(*circle);
            } else if !js_has(&exclude, set) {
                exterior.push(*circle);
            }
        }

        let centre = compute_text_centre(&interior, &exterior);
        result.insert(&area.sets.join(","), centre);
    }
    Ok(result)
}
