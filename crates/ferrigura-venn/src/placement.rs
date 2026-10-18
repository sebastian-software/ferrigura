//! The placement of `src/layout.js` as the bundle has it (build lines 623-1101): `venn`,
//! `addMissingAreas`, `distanceFromIntersectArea`, the greedy placement, the constrained
//! multidimensional scaling of its branch, and the loss.
//!
//! The random draws are those of `Math.random` in `constrainedMDSLayout`: `2n` numbers per
//! restart, in index order, before each conjugate-gradient run.

use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;

use crate::Area;
use crate::Circle;
use crate::Error;
use crate::fmin::{self, VENN_TOLERANCE, norm2};
use crate::geometry::{
    Point, SMALL, centre, circle_circle_intersection, circle_overlap, distance, intersection_area,
};
use crate::js::{JsObject, js_has, js_min, js_sort, js_str_cmp};

/// The number of restarts of the constrained layout (`params.restarts || 10`).
const RESTARTS: usize = 10;

/// The iteration limit of the conjugate gradient (`maxIterations`, set to 500 by `venn()`).
const GRADIENT_ITERATIONS: usize = 500;

/// The areas the loss sums over, with their sets as positions in a vector of circles.
#[derive(Clone, Debug)]
pub(crate) struct Resolved {
    sets: Vec<usize>,
    size: f64,
}

/// The loss of `lossFunction`: the sum of the squared differences between each overlap and its
/// size, over the areas with at least two sets. Single sets contribute nothing.
pub(crate) fn loss(values: &[Circle], areas: &[Resolved]) -> f64 {
    let mut output = 0.0;
    for area in areas {
        let overlap = if area.sets.len() == 2 {
            let left = values[area.sets[0]];
            let right = values[area.sets[1]];
            circle_overlap(
                left.radius,
                right.radius,
                distance(centre(&left), centre(&right)),
            )
        } else {
            let circles: Vec<Circle> = area.sets.iter().map(|&position| values[position]).collect();
            intersection_area(&circles).area
        };
        output += (overlap - area.size) * (overlap - area.size);
    }
    output
}

/// Resolves the areas with two or more sets against `position`, the place of each set's circle.
pub(crate) fn resolve<'a>(
    areas: impl IntoIterator<Item = &'a Area>,
    position: impl Fn(&str) -> Option<usize>,
) -> Result<Vec<Resolved>, Error> {
    let mut resolved = Vec::new();
    for area in areas {
        if area.sets.len() < 2 {
            continue;
        }
        let sets = area
            .sets
            .iter()
            .map(|set| position(set).ok_or_else(|| Error::UnknownSet(set.clone())))
            .collect::<Result<Vec<usize>, Error>>()?;
        resolved.push(Resolved {
            sets,
            size: area.size,
        });
    }
    Ok(resolved)
}

/// Refuses a set named `__proto__`, which the port does not reproduce.
pub(crate) fn check_names(areas: &[Area]) -> Result<(), Error> {
    if areas
        .iter()
        .any(|area| area.sets.iter().any(|set| set == "__proto__"))
    {
        return Err(Error::ReservedName);
    }
    Ok(())
}

/// Checks the input for the calls that read every area (`venn`): every area has a set, no set is
/// `__proto__`, and every set of every area has a single-set area of its own.
pub(crate) fn check(areas: &[Area]) -> Result<(), Error> {
    if areas.iter().any(|area| area.sets.is_empty()) {
        return Err(Error::EmptyArea);
    }
    check_names(areas)?;
    let singles: HashSet<&str> = areas
        .iter()
        .filter(|area| area.sets.len() == 1)
        .map(|area| area.sets[0].as_str())
        .collect();
    for area in areas {
        for set in &area.sets {
            if !singles.contains(set.as_str()) {
                return Err(Error::UnknownSet(set.clone()));
            }
        }
    }
    Ok(())
}

/// The key the library gives a set of ids: `join(';')`.
fn pair_key(a: &str, b: &str) -> String {
    format!("{a};{b}")
}

/// `addMissingAreas(areas)` with `distinct` off: every pair of single sets that has no two-set
/// area gets one of size 0, appended in the order of the sorted ids.
pub fn add_missing_areas(areas: &[Area]) -> Vec<Area> {
    let mut result: Vec<Area> = areas.to_vec();
    let mut ids: Vec<String> = Vec::new();
    let mut pairs: HashSet<String> = HashSet::new();
    for area in areas {
        if area.sets.len() == 1 {
            ids.push(area.sets[0].clone());
        } else if area.sets.len() == 2 {
            pairs.insert(pair_key(&area.sets[0], &area.sets[1]));
            pairs.insert(pair_key(&area.sets[1], &area.sets[0]));
        }
    }

    ids.sort_by(|a, b| js_str_cmp(a, b));

    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            if !pairs.contains(&pair_key(&ids[i], &ids[j])) {
                result.push(Area {
                    sets: vec![ids[i].clone(), ids[j].clone()],
                    size: 0.0,
                });
            }
        }
    }
    result
}

/// `distanceFromIntersectArea(r1, r2, overlap)`: the distance at which two circles overlap by
/// `overlap`.
pub(crate) fn distance_from_intersect_area(r1: f64, r2: f64, overlap: f64) -> Result<f64, Error> {
    let smaller = js_min(r1, r2);
    if smaller * smaller * PI <= overlap + SMALL {
        return Ok((r1 - r2).abs());
    }
    fmin::bisect(|d| circle_overlap(r1, r2, d) - overlap, 0.0, r1 + r2)
}

/// One overlap of a set with another: the other set, the overlap's size and its weight.
#[derive(Clone, Debug)]
struct Overlap {
    set: String,
    size: f64,
    weight: f64,
}

/// A circle with its size, as the greedy layout keeps it.
#[derive(Clone, Debug)]
struct Entry {
    circle: Circle,
    size: f64,
}

/// `greedyLayout(areas)`: places the sets one at a time, starting from the most overlapped, each
/// where the loss of the pairwise areas is least among a few candidate points.
pub(crate) fn greedy(areas: &[Area]) -> Result<JsObject<Circle>, Error> {
    let mut circles: JsObject<Entry> = JsObject::new();
    let mut set_overlaps: JsObject<Vec<Overlap>> = JsObject::new();
    for area in areas {
        if area.sets.len() == 1 {
            let set = &area.sets[0];
            circles.insert(
                set,
                Entry {
                    circle: Circle {
                        x: 1e10,
                        y: 1e10,
                        radius: (area.size / PI).sqrt(),
                    },
                    size: area.size,
                },
            );
            set_overlaps.insert(set, Vec::new());
        }
    }

    let pairs: Vec<&Area> = areas.iter().filter(|area| area.sets.len() == 2).collect();

    for current in &pairs {
        let mut weight = 1.0;
        let left = &current.sets[0];
        let right = &current.sets[1];

        let left_size = entry(&circles, left)?.size;
        let right_size = entry(&circles, right)?.size;
        if current.size + SMALL >= js_min(left_size, right_size) {
            weight = 0.0;
        }

        overlaps_of(&mut set_overlaps, left)?.push(Overlap {
            set: right.clone(),
            size: current.size,
            weight,
        });
        overlaps_of(&mut set_overlaps, right)?.push(Overlap {
            set: left.clone(),
            size: current.size,
            weight,
        });
    }

    let mut most_overlapped: Vec<(String, f64)> = Vec::new();
    for (set, overlaps) in set_overlaps.iter() {
        let mut size = 0.0;
        for overlap in overlaps {
            size += overlap.size * overlap.weight;
        }
        most_overlapped.push((set.to_owned(), size));
    }
    js_sort(&mut most_overlapped, |a, b| b.1 - a.1);

    let mut positioned: HashSet<String> = HashSet::new();
    let first = most_overlapped.first().ok_or(Error::NoAreas)?.0.clone();
    position_set(
        &mut circles,
        &mut positioned,
        &first,
        Point { x: 0.0, y: 0.0 },
    )?;

    let resolved_pairs = resolve(pairs.iter().copied(), |set| circles.position(set))?;

    for (set_index, _) in most_overlapped.iter().skip(1) {
        let mut overlap: Vec<Overlap> = overlaps_of_ref(&set_overlaps, set_index)?
            .iter()
            .filter(|candidate| js_has(&positioned, &candidate.set))
            .cloned()
            .collect();
        let set_radius = entry(&circles, set_index)?.circle.radius;

        js_sort(&mut overlap, |a, b| b.size - a.size);

        if overlap.is_empty() {
            return Err(Error::MissingPairwiseOverlap);
        }

        let mut points: Vec<Point> = Vec::new();
        for j in 0..overlap.len() {
            let p1 = entry(&circles, &overlap[j].set)?.circle;
            let d1 = distance_from_intersect_area(set_radius, p1.radius, overlap[j].size)?;

            points.push(Point {
                x: p1.x + d1,
                y: p1.y,
            });
            points.push(Point {
                x: p1.x - d1,
                y: p1.y,
            });
            points.push(Point {
                x: p1.x,
                y: p1.y + d1,
            });
            points.push(Point {
                x: p1.x,
                y: p1.y - d1,
            });

            for other in &overlap[j + 1..] {
                let p2 = entry(&circles, &other.set)?.circle;
                let d2 = distance_from_intersect_area(set_radius, p2.radius, other.size)?;
                points.extend(circle_circle_intersection(
                    Circle {
                        x: p1.x,
                        y: p1.y,
                        radius: d1,
                    },
                    Circle {
                        x: p2.x,
                        y: p2.y,
                        radius: d2,
                    },
                ));
            }
        }

        let mut best_loss = 1e50;
        let mut best_point = points[0];
        for point in &points {
            let entry_mut = circles
                .get_mut(set_index)
                .ok_or_else(|| Error::UnknownSet(set_index.clone()))?;
            entry_mut.circle.x = point.x;
            entry_mut.circle.y = point.y;
            let values: Vec<Circle> = circles.values().map(|entry| entry.circle).collect();
            let local_loss = loss(&values, &resolved_pairs);
            if local_loss < best_loss {
                best_loss = local_loss;
                best_point = *point;
            }
        }

        position_set(&mut circles, &mut positioned, set_index, best_point)?;
    }

    let mut result = JsObject::new();
    for (set, entry) in circles.iter() {
        result.insert(set, entry.circle);
    }
    Ok(result)
}

fn entry<'a>(circles: &'a JsObject<Entry>, set: &str) -> Result<&'a Entry, Error> {
    circles
        .get(set)
        .ok_or_else(|| Error::UnknownSet(set.to_owned()))
}

fn overlaps_of<'a>(
    set_overlaps: &'a mut JsObject<Vec<Overlap>>,
    set: &str,
) -> Result<&'a mut Vec<Overlap>, Error> {
    set_overlaps
        .get_mut(set)
        .ok_or_else(|| Error::UnknownSet(set.to_owned()))
}

fn overlaps_of_ref<'a>(
    set_overlaps: &'a JsObject<Vec<Overlap>>,
    set: &str,
) -> Result<&'a Vec<Overlap>, Error> {
    set_overlaps
        .get(set)
        .ok_or_else(|| Error::UnknownSet(set.to_owned()))
}

/// `positionSet(point, index)`: moves the circle and marks the set as positioned.
fn position_set(
    circles: &mut JsObject<Entry>,
    positioned: &mut HashSet<String>,
    set: &str,
    point: Point,
) -> Result<(), Error> {
    let entry_mut = circles
        .get_mut(set)
        .ok_or_else(|| Error::UnknownSet(set.to_owned()))?;
    entry_mut.circle.x = point.x;
    entry_mut.circle.y = point.y;
    positioned.insert(set.to_owned());
    Ok(())
}

/// The gradient of the constrained objective, written into `fxprime`, with its loss returned
/// (`constrainedMDSGradient`).
fn constrained_gradient(
    x: &[f64],
    fxprime: &mut [f64],
    distances: &[Vec<f64>],
    constraints: &[Vec<f64>],
) -> f64 {
    for value in fxprime.iter_mut() {
        *value = 0.0;
    }

    let mut loss = 0.0;
    let n = distances.len();
    for i in 0..n {
        let xi = x[2 * i];
        let yi = x[2 * i + 1];
        for j in (i + 1)..n {
            let xj = x[2 * j];
            let yj = x[2 * j + 1];
            let dij = distances[i][j];
            let constraint = constraints[i][j];

            let squared_distance = (xj - xi) * (xj - xi) + (yj - yi) * (yj - yi);
            let distance = squared_distance.sqrt();
            let delta = squared_distance - dij * dij;

            if (constraint > 0.0 && distance <= dij) || (constraint < 0.0 && distance >= dij) {
                continue;
            }

            loss += 2.0 * delta * delta;

            fxprime[2 * i] += 4.0 * delta * (xi - xj);
            fxprime[2 * i + 1] += 4.0 * delta * (yi - yj);

            fxprime[2 * j] += 4.0 * delta * (xj - xi);
            fxprime[2 * j + 1] += 4.0 * delta * (yj - yi);
        }
    }
    loss
}

/// A square matrix of the sets, by position.
type Matrix = Vec<Vec<f64>>;

/// `getDistanceMatrices(areas, sets, setids)`: the distance and the constraint (1 for a subset,
/// -1 for disjoint, 0 otherwise) of each pair of sets with a two-set area.
fn distance_matrices(
    areas: &[Area],
    sets: &[&Area],
    setids: &HashMap<&str, usize>,
) -> Result<(Matrix, Matrix), Error> {
    let n = sets.len();
    let mut distances = vec![vec![0.0; n]; n];
    let mut constraints = vec![vec![0.0; n]; n];

    for current in areas.iter().filter(|area| area.sets.len() == 2) {
        let left = *setids
            .get(current.sets[0].as_str())
            .ok_or_else(|| Error::UnknownSet(current.sets[0].clone()))?;
        let right = *setids
            .get(current.sets[1].as_str())
            .ok_or_else(|| Error::UnknownSet(current.sets[1].clone()))?;
        let r1 = (sets[left].size / PI).sqrt();
        let r2 = (sets[right].size / PI).sqrt();
        let distance = distance_from_intersect_area(r1, r2, current.size)?;

        distances[left][right] = distance;
        distances[right][left] = distance;

        let mut constraint = 0.0;
        if current.size + 1e-10 >= js_min(sets[left].size, sets[right].size) {
            constraint = 1.0;
        } else if current.size <= 1e-10 {
            constraint = -1.0;
        }
        constraints[left][right] = constraint;
        constraints[right][left] = constraint;
    }

    Ok((distances, constraints))
}

/// `constrainedMDSLayout(areas)`: the best of ten conjugate-gradient runs from random starts.
pub(crate) fn constrained_mds(
    areas: &[Area],
    random: &mut impl FnMut() -> f64,
) -> Result<JsObject<Circle>, Error> {
    let mut sets: Vec<&Area> = Vec::new();
    let mut setids: HashMap<&str, usize> = HashMap::new();
    for area in areas {
        if area.sets.len() == 1 {
            setids.insert(area.sets[0].as_str(), sets.len());
            sets.push(area);
        }
    }

    let (raw_distances, constraints) = distance_matrices(areas, &sets, &setids)?;
    let n = raw_distances.len();

    // Keep the distances bounded: divide by the norm of the row norms, over n.
    let row_norms: Vec<f64> = raw_distances.iter().map(|row| norm2(row)).collect();
    let norm = norm2(&row_norms) / n as f64;
    let distances: Vec<Vec<f64>> = raw_distances
        .iter()
        .map(|row| row.iter().map(|value| value / norm).collect())
        .collect();

    let mut best: Option<fmin::Gradient> = None;
    for _ in 0..RESTARTS {
        let initial: Vec<f64> = (0..2 * n).map(|_| random()).collect();
        let current = fmin::conjugate_gradient(
            |x, fxprime| constrained_gradient(x, fxprime, &distances, &constraints),
            &initial,
            GRADIENT_ITERATIONS,
        );
        let better = match &best {
            None => true,
            Some(previous) => current.fx < previous.fx,
        };
        if better {
            best = Some(current);
        }
    }

    let positions = best.map(|best| best.x).unwrap_or_default();
    let mut circles = JsObject::new();
    for (i, set) in sets.iter().enumerate() {
        circles.insert(
            &set.sets[0],
            Circle {
                x: positions[2 * i] * norm,
                y: positions[2 * i + 1] * norm,
                radius: (set.size / PI).sqrt(),
            },
        );
    }
    Ok(circles)
}

/// `bestInitialLayout(areas)`: the greedy layout, or the constrained one when there are eight or
/// more areas and it has the smaller loss by more than 1e-8.
pub(crate) fn best_initial_layout(
    areas: &[Area],
    random: &mut impl FnMut() -> f64,
) -> Result<JsObject<Circle>, Error> {
    let mut initial = greedy(areas)?;

    if areas.len() >= 8 {
        let constrained = constrained_mds(areas, random)?;
        let constrained_loss = loss_of(&constrained, areas)?;
        let greedy_loss = loss_of(&initial, areas)?;
        if constrained_loss + 1e-8 < greedy_loss {
            initial = constrained;
        }
    }
    Ok(initial)
}

/// The loss of a layout over the areas, with the circles looked up by set.
fn loss_of(circles: &JsObject<Circle>, areas: &[Area]) -> Result<f64, Error> {
    let values: Vec<Circle> = circles.values().copied().collect();
    let resolved = resolve(areas, |set| circles.position(set))?;
    Ok(loss(&values, &resolved))
}

/// `venn(areas)` with the library's defaults: the placement, then the minimisation of the loss by
/// the simplex, from the best initial layout. The random source supplies the starts of the
/// constrained layout, when it runs.
pub(crate) fn venn(
    areas: &[Area],
    random: &mut impl FnMut() -> f64,
) -> Result<JsObject<Circle>, Error> {
    check(areas)?;
    let areas = add_missing_areas(areas);
    let mut circles = best_initial_layout(&areas, random)?;

    let order: Vec<(String, Circle)> = circles
        .iter()
        .map(|(set, c)| (set.to_owned(), *c))
        .collect();
    let mut initial = Vec::with_capacity(2 * order.len());
    for (_, circle) in &order {
        initial.push(circle.x);
        initial.push(circle.y);
    }
    let radii: Vec<f64> = order.iter().map(|(_, circle)| circle.radius).collect();
    let position: HashMap<&str, usize> = order
        .iter()
        .enumerate()
        .map(|(i, (set, _))| (set.as_str(), i))
        .collect();
    let resolved = resolve(&areas, |set| position.get(set).copied())?;

    let solution = fmin::nelder_mead(
        |values| {
            let current: Vec<Circle> = radii
                .iter()
                .enumerate()
                .map(|(i, &radius)| Circle {
                    x: values[2 * i],
                    y: values[2 * i + 1],
                    radius,
                })
                .collect();
            loss(&current, &resolved)
        },
        &initial,
        VENN_TOLERANCE,
    );

    for (i, (set, circle)) in order.iter().enumerate() {
        circles.insert(
            set,
            Circle {
                x: solution.x[2 * i],
                y: solution.x[2 * i + 1],
                radius: circle.radius,
            },
        );
    }
    Ok(circles)
}
