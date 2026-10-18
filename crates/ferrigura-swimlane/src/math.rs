//! The `Math` functions and comparator results of the swimlane phases, with JavaScript's rules.
//!
//! Rust's `f64::min` and `f64::max` return the other operand when one is NaN, and they do not order
//! `-0` before `+0`. `Math.min` and `Math.max` return NaN when an operand is NaN, and they take `-0`
//! as smaller than `+0`. The layout uses both on values that can be NaN when a diagram has a NaN
//! size, so the functions here are the ones the port calls.

use std::cmp::Ordering;

/// `Math.max(a, b)`.
pub(crate) fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        return f64::NAN;
    }
    if a == b {
        // Equal values differ only in the sign of zero: `Math.max(-0, +0)` is `+0`.
        return if a.is_sign_negative() { b } else { a };
    }
    if a > b { a } else { b }
}

/// `Math.min(a, b)`.
pub(crate) fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        return f64::NAN;
    }
    if a == b {
        // Equal values differ only in the sign of zero: `Math.min(+0, -0)` is `-0`.
        return if a.is_sign_negative() { a } else { b };
    }
    if a < b { a } else { b }
}

/// The ordering that a JavaScript comparator returning `diff` gives to its elements.
///
/// `Array.prototype.sort` treats a NaN result as `+0`, which is "equal", and it sorts on the sign
/// otherwise. A comparator `(a, b) => a.x - b.x` therefore calls this with `a.x - b.x`.
pub(crate) fn js_sign_order(diff: f64) -> Ordering {
    if diff < 0.0 {
        Ordering::Less
    } else if diff > 0.0 {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}
