//! JavaScript's number text and rounding, which the reference's attributes, paths and labels are
//! written with: `Number#toString` ([`js_number`]), `Math.round` ([`js_round`]), the three-digit
//! coordinates of `d3-path` ([`d3_number`]), the `+x` coercion of a string ([`js_to_number`]) and
//! `interpolateNumber` of `d3-interpolate` ([`interpolate_number`]).

/// JavaScript's `Number.prototype.toString` for a double: the shortest digits that read back to the
/// same value, in plain notation between `1e-6` and `1e21`, otherwise in exponent notation.
/// `-0` prints as `0`.
#[must_use]
pub fn js_number(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    if value == 0.0 {
        return "0".to_owned();
    }
    let sign = if value < 0.0 { "-" } else { "" };
    // `{:e}` gives the shortest round-trip digits, as JavaScript does.
    let scientific = format!("{:e}", value.abs());
    let (mantissa, exponent) = scientific
        .split_once('e')
        .unwrap_or((scientific.as_str(), "0"));
    let exponent: i64 = exponent.parse().unwrap_or(0);
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let k = digits.len() as i64;
    let n = exponent + 1;
    let body = if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        let (head, tail) = digits.split_at(n as usize);
        format!("{head}.{tail}")
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let e = n - 1;
        let exp = if e >= 0 {
            format!("+{e}")
        } else {
            format!("{e}")
        };
        if k == 1 {
            format!("{digits}e{exp}")
        } else {
            let (head, tail) = digits.split_at(1);
            format!("{head}.{tail}e{exp}")
        }
    };
    format!("{sign}{body}")
}

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
