//! The minimisers of `fmin`, as the bundle of venn.js 2.0.0 inlines them (build lines 262-622):
//! bisection, the downhill simplex (Nelder-Mead), the Wolfe line search and conjugate gradient
//! (Polak-Ribière). The options the library passes are fixed, so they are constants here. Each
//! expression keeps the operand order of the source.

use crate::Error;
use crate::js::{js_max, js_sort};

/// The iteration limit of the simplex (`maxIterations`, which `venn()` and `computeTextCentre` set
/// to 500).
const SIMPLEX_ITERATIONS: usize = 500;

/// The stopping tolerances of the simplex. `venn()` passes none, so the defaults apply
/// (`minErrorDelta` 1e-6, and 1e-5 for the tolerance, as the source reads `minErrorDelta` for both
/// of them); `computeTextCentre` passes `minErrorDelta` 1e-10.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Tolerance {
    pub min_error_delta: f64,
    pub min_tolerance: f64,
}

pub(crate) const VENN_TOLERANCE: Tolerance = Tolerance {
    min_error_delta: 1e-6,
    min_tolerance: 1e-5,
};

pub(crate) const TEXT_TOLERANCE: Tolerance = Tolerance {
    min_error_delta: 1e-10,
    min_tolerance: 1e-10,
};

/// `bisect(f, a, b)` with the defaults: 100 iterations, tolerance 1e-10.
pub(crate) fn bisect(f: impl Fn(f64) -> f64, mut a: f64, b: f64) -> Result<f64, Error> {
    const ITERATIONS: usize = 100;
    const TOLERANCE: f64 = 1e-10;
    let f_a = f(a);
    let f_b = f(b);
    let mut delta = b - a;

    if f_a * f_b > 0.0 {
        return Err(Error::NoBracket);
    }
    if f_a == 0.0 {
        return Ok(a);
    }
    if f_b == 0.0 {
        return Ok(b);
    }

    for _ in 0..ITERATIONS {
        delta /= 2.0;
        let mid = a + delta;
        let f_mid = f(mid);

        if f_mid * f_a >= 0.0 {
            a = mid;
        }

        if delta.abs() < TOLERANCE || f_mid == 0.0 {
            return Ok(mid);
        }
    }
    Ok(a + delta)
}

/// A vertex of the simplex: a point and its loss.
#[derive(Clone, Debug)]
pub(crate) struct Vertex {
    pub x: Vec<f64>,
    pub fx: f64,
}

/// `ret[j] = w1 * v1[j] + w2 * v2[j]`.
fn weighted_sum(ret: &mut [f64], w1: f64, v1: &[f64], w2: f64, v2: &[f64]) {
    for ((r, a), b) in ret.iter_mut().zip(v1).zip(v2) {
        *r = w1 * a + w2 * b;
    }
}

/// The sum of the products, from the first element on.
pub(crate) fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).fold(0.0, |sum, (x, y)| sum + x * y)
}

pub(crate) fn norm2(a: &[f64]) -> f64 {
    dot(a, a).sqrt()
}

/// Minimises `f` from the point `x0` with the downhill simplex. Returns the best vertex.
pub(crate) fn nelder_mead(
    mut f: impl FnMut(&[f64]) -> f64,
    x0: &[f64],
    tolerance: Tolerance,
) -> Vertex {
    const NON_ZERO_DELTA: f64 = 1.05;
    const ZERO_DELTA: f64 = 0.001;
    const RHO: f64 = 1.0;
    const CHI: f64 = 2.0;
    const PSI: f64 = -0.5;
    const SIGMA: f64 = 0.5;

    let n = x0.len();
    let mut simplex: Vec<Vertex> = Vec::with_capacity(n + 1);
    let fx0 = f(x0);
    simplex.push(Vertex {
        x: x0.to_vec(),
        fx: fx0,
    });
    for i in 0..n {
        let mut point = x0.to_vec();
        point[i] = if point[i] != 0.0 && !point[i].is_nan() {
            point[i] * NON_ZERO_DELTA
        } else {
            ZERO_DELTA
        };
        let fx = f(&point);
        simplex.push(Vertex { x: point, fx });
    }

    let mut centroid = x0.to_vec();
    let mut reflected = Vertex {
        x: x0.to_vec(),
        fx: 0.0,
    };
    let mut expanded = reflected.clone();
    let mut contracted = reflected.clone();

    for _ in 0..SIMPLEX_ITERATIONS {
        js_sort(&mut simplex, |a, b| a.fx - b.fx);

        let mut max_diff = 0.0;
        for i in 0..n {
            max_diff = js_max(max_diff, (simplex[0].x[i] - simplex[1].x[i]).abs());
        }

        if (simplex[0].fx - simplex[n].fx).abs() < tolerance.min_error_delta
            && max_diff < tolerance.min_tolerance
        {
            break;
        }

        for (i, coordinate) in centroid.iter_mut().enumerate() {
            let mut sum = 0.0;
            for vertex in simplex.iter().take(n) {
                sum += vertex.x[i];
            }
            *coordinate = sum / n as f64;
        }

        let worst = simplex[n].clone();

        weighted_sum(&mut reflected.x, 1.0 + RHO, &centroid, -RHO, &worst.x);
        reflected.fx = f(&reflected.x);

        if reflected.fx < simplex[0].fx {
            weighted_sum(&mut expanded.x, 1.0 + CHI, &centroid, -CHI, &worst.x);
            expanded.fx = f(&expanded.x);
            if expanded.fx < reflected.fx {
                simplex[n] = expanded.clone();
            } else {
                simplex[n] = reflected.clone();
            }
        } else if reflected.fx >= simplex[n - 1].fx {
            let mut should_reduce = false;

            if reflected.fx > worst.fx {
                weighted_sum(&mut contracted.x, 1.0 + PSI, &centroid, -PSI, &worst.x);
                contracted.fx = f(&contracted.x);
                if contracted.fx < worst.fx {
                    simplex[n] = contracted.clone();
                } else {
                    should_reduce = true;
                }
            } else {
                weighted_sum(
                    &mut contracted.x,
                    1.0 - PSI * RHO,
                    &centroid,
                    PSI * RHO,
                    &worst.x,
                );
                contracted.fx = f(&contracted.x);
                if contracted.fx < reflected.fx {
                    simplex[n] = contracted.clone();
                } else {
                    should_reduce = true;
                }
            }

            if should_reduce {
                let best = simplex[0].x.clone();
                for vertex in simplex.iter_mut().skip(1) {
                    let current = vertex.x.clone();
                    weighted_sum(&mut vertex.x, 1.0 - SIGMA, &best, SIGMA, &current);
                    vertex.fx = f(&vertex.x);
                }
            }
        } else {
            simplex[n] = reflected.clone();
        }
    }

    js_sort(&mut simplex, |a, b| a.fx - b.fx);
    simplex.swap_remove(0)
}

/// A point of the conjugate gradient, with its loss and gradient.
#[derive(Clone, Debug)]
pub(crate) struct Gradient {
    pub x: Vec<f64>,
    pub fx: f64,
    pub fxprime: Vec<f64>,
}

const WOLFE_C1: f64 = 1e-6;
const WOLFE_C2: f64 = 0.1;

/// `f(x, fxprime)`: the loss at `x`, with the gradient written to `fxprime`.
type Objective<'a> = dyn FnMut(&[f64], &mut [f64]) -> f64 + 'a;

/// Evaluates the objective at `current.x + a * pk` into `next`, and returns the loss.
fn evaluate_at(
    f: &mut Objective<'_>,
    pk: &[f64],
    current: &Gradient,
    next: &mut Gradient,
    a: f64,
) -> f64 {
    weighted_sum(&mut next.x, 1.0, &current.x, a, pk);
    next.fx = f(&next.x, &mut next.fxprime);
    next.fx
}

/// The inputs of a line search: the direction and the starting point.
struct Line<'a> {
    pk: &'a [f64],
    current: &'a Gradient,
    phi0: f64,
    phi_prime0: f64,
}

/// `zoom`: bisects between `a_lo` and `a_high` for a step that satisfies the Wolfe conditions.
fn zoom(
    f: &mut Objective<'_>,
    line: &Line<'_>,
    next: &mut Gradient,
    (mut a_lo, mut a_high, mut phi_lo): (f64, f64, f64),
) -> f64 {
    for _ in 0..16 {
        let a = (a_lo + a_high) / 2.0;
        let phi = evaluate_at(f, line.pk, line.current, next, a);
        let phi_prime = dot(&next.fxprime, line.pk);

        if phi > line.phi0 + WOLFE_C1 * a * line.phi_prime0 || phi >= phi_lo {
            a_high = a;
        } else {
            if phi_prime.abs() <= -WOLFE_C2 * line.phi_prime0 {
                return a;
            }

            if phi_prime * (a_high - a_lo) >= 0.0 {
                a_high = a_lo;
            }

            a_lo = a;
            phi_lo = phi;
        }
    }
    0.0
}

/// Searches along `pk` for a step that satisfies the Wolfe conditions (Nocedal and Wright,
/// 59-60). Returns the step, or 0 when none was found. A step of 0 or NaN starts from 1.
fn wolfe_line_search(
    f: &mut Objective<'_>,
    pk: &[f64],
    current: &Gradient,
    next: &mut Gradient,
    a: f64,
) -> f64 {
    let phi0 = current.fx;
    let phi_prime0 = dot(&current.fxprime, pk);
    let line = Line {
        pk,
        current,
        phi0,
        phi_prime0,
    };
    let mut phi_old = phi0;
    let mut a0 = 0.0;
    let mut a = if a == 0.0 || a.is_nan() { 1.0 } else { a };

    for iteration in 0..10 {
        let phi = evaluate_at(f, pk, current, next, a);
        let phi_prime = dot(&next.fxprime, pk);

        if phi > phi0 + WOLFE_C1 * a * phi_prime0 || (iteration != 0 && phi >= phi_old) {
            return zoom(f, &line, next, (a0, a, phi_old));
        }

        if phi_prime.abs() <= -WOLFE_C2 * phi_prime0 {
            return a;
        }

        if phi_prime >= 0.0 {
            return zoom(f, &line, next, (a, a0, phi));
        }

        phi_old = phi;
        a0 = a;
        a *= 2.0;
    }

    a
}

/// Minimises `f` from `initial` by Polak-Ribière conjugate gradient, for at most `iterations`
/// steps. Returns the last current point.
pub(crate) fn conjugate_gradient(
    mut f: impl FnMut(&[f64], &mut [f64]) -> f64,
    initial: &[f64],
    iterations: usize,
) -> Gradient {
    let mut current = Gradient {
        x: initial.to_vec(),
        fx: 0.0,
        fxprime: initial.to_vec(),
    };
    let mut next = current.clone();
    let mut yk = initial.to_vec();
    let mut a = 1.0;

    current.fx = f(&current.x, &mut current.fxprime);
    let mut pk: Vec<f64> = current.fxprime.iter().map(|g| g * -1.0).collect();

    for _ in 0..iterations {
        a = wolfe_line_search(&mut f, &pk, &current, &mut next, a);

        if a == 0.0 || a.is_nan() {
            // The search failed: reset the direction to the steepest descent.
            pk = current.fxprime.iter().map(|g| g * -1.0).collect();
        } else {
            weighted_sum(&mut yk, 1.0, &next.fxprime, -1.0, &current.fxprime);

            let delta_k = dot(&current.fxprime, &current.fxprime);
            let beta_k = js_max(0.0, dot(&yk, &next.fxprime) / delta_k);

            let previous = pk.clone();
            weighted_sum(&mut pk, beta_k, &previous, -1.0, &next.fxprime);

            std::mem::swap(&mut current, &mut next);
        }

        if norm2(&current.fxprime) <= 1e-5 {
            break;
        }
    }

    current
}
