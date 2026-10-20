//! JavaScript's number text and rounding, which the reference's attributes, paths and labels are
//! written with: `Number#toString` ([`js_number`]), `Math.round` ([`js_round`]), the three-digit
//! coordinates of `d3-path` ([`d3_number`]), the `+x` coercion of a string ([`js_to_number`]) and
//! `interpolateNumber` of `d3-interpolate` ([`interpolate_number`]).

/// JavaScript's `Number.prototype.toString` for a double, as the model writes it.
pub use ferrigura_model::value::number_text as js_number;

/// `Math.round`: ties go towards +infinity (`Math.round(-2.5)` is `-2`).
#[must_use]
pub fn js_round(value: f64) -> f64 {
    let floor = value.floor();
    if value - floor >= 0.5 {
        floor + 1.0
    } else {
        floor
    }
}

/// A coordinate as `d3-path` writes it with three digits: `Math.round(v * 1000) / 1000`.
#[must_use]
pub fn d3_number(value: f64) -> String {
    js_number(js_round(value * 1000.0) / 1000.0)
}

/// `interpolateNumber(a, b)(t)` of `d3-interpolate` 3.0.1 (`number.js`): `a * (1 - t) + b * t`. The
/// order of the operations is the reference's, so the last digit is too.
#[must_use]
pub fn interpolate_number(a: f64, b: f64, t: f64) -> f64 {
    a * (1.0 - t) + b * t
}

/// The number a string reads as in JavaScript (`+text`): blanks around it are dropped, the empty
/// string is 0, and text that is not a number is NaN. Only the forms the reference writes are read
/// exactly: decimal numbers, with an exponent, `Infinity` and `-Infinity`.
#[must_use]
pub fn js_to_number(text: &str) -> f64 {
    let trimmed = text.trim_matches(crate::common::is_js_whitespace);
    if trimmed.is_empty() {
        return 0.0;
    }
    match trimmed {
        "Infinity" | "+Infinity" => f64::INFINITY,
        "-Infinity" => f64::NEG_INFINITY,
        _ if trimmed.bytes().all(|byte| {
            byte.is_ascii_digit() || matches!(byte, b'+' | b'-' | b'.' | b'e' | b'E')
        }) =>
        {
            trimmed.parse::<f64>().unwrap_or(f64::NAN)
        }
        _ => f64::NAN,
    }
}
