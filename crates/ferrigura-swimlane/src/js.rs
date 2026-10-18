//! The JavaScript semantics the swimlane layout depends on, as V8 gives them, bit for bit:
//!
//! - [`to_fixed`]: `Number.prototype.toFixed`, by the specification (ECMA-262), with the decision taken on
//!   the exact binary value and ties rounded up in magnitude.
//! - [`js_round`]: `Math.round`, half toward +infinity, with V8's sign of zero.
//! - [`locale_compare`] and [`try_locale_compare`]: `a.localeCompare(b)` in V8 with the default locale (en-US),
//!   for printable ASCII. It is ICU's root collation at tertiary strength, written as a table of the 95
//!   characters in ICU order.
//! - Sorting: the layout sorts with [`slice::sort_by`] and passes the comparator unchanged, with no wrapper.
//!   Rust's `sort_by` is stable, and so is `Array.prototype.sort` since ES2019: equal elements keep their
//!   input order in both, which the layout relies on.
//!
//! `Math.hypot` is not here: the port calls `ferrigura_jsmath::hypot`.
//!
//! The data files in `tests/data` hold V8's results for these functions (Node v24.21.0, ICU 78.3), written by
//! `tools/js-semantics.ts`; `tests/js_semantics.rs` checks them.

use std::cmp::Ordering;

use crate::Error;

/// The most fraction digits `toFixed` takes. JavaScript throws a `RangeError` above it.
const MAX_DIGITS: u32 = 100;

/// The number of fraction digits `format!` prints for a double without rounding. A double is m * 2^-k with
/// k at most 1074, so its decimal expansion has at most 1074 fraction digits; 1100 leaves room.
const EXACT_DIGITS: usize = 1100;

/// `2^52`: every double at or above it is an integer, so `Math.round` returns it unchanged.
const TWO_POW_52: f64 = 4_503_599_627_370_496.0;

/// `Number.prototype.toFixed(digits)`: the decimal string of `x` with `digits` fraction digits.
///
/// - `NaN` gives `"NaN"`, and the infinities give `"Infinity"` and `"-Infinity"`.
/// - For `|x| >= 1e21` the result is `Number::toString` of `x` in exponent form, as the specification says
///   (`"1e+21"`, `"-1.2345e+25"`). Its digits are the shortest that round-trip, which is what Rust's `{:e}`
///   writes, so there is no error case.
/// - Otherwise the sign is `-` when `x < 0`, so `-0` gives `"0.0"` but `-0.04` gives `"-0.0"`, and the number
///   is `n / 10^digits` for the integer `n` nearest to `|x|`. A tie goes to the larger `n`, so the rounding is
///   half up in magnitude: `(0.25).toFixed(1)` is `"0.3"` and `(-2.5).toFixed(0)` is `"-3"`. The decision uses
///   the exact binary value: `format!("{:.1100}")` writes the exact expansion of the double, and the first digit
///   after the last one kept decides (5 or more rounds up). The rounding of `format!` itself (ties to even) is
///   not used for the decision.
///
/// # Panics
///
/// Panics if `digits > 100`, which JavaScript rejects with a `RangeError`.
#[must_use]
pub fn to_fixed(x: f64, digits: u32) -> String {
    assert!(
        digits <= MAX_DIGITS,
        "toFixed takes at most {MAX_DIGITS} fraction digits, not {digits}"
    );
    if x.is_nan() {
        return "NaN".to_string();
    }
    if x.is_infinite() {
        return if x < 0.0 { "-Infinity" } else { "Infinity" }.to_string();
    }
    let sign = if x < 0.0 { "-" } else { "" };
    let magnitude = x.abs();
    if magnitude >= 1e21 {
        return format!("{sign}{}", exponent_form(magnitude));
    }

    let digits = digits as usize;
    let exact = format!("{:.*}", EXACT_DIGITS, magnitude);
    let (integer, fraction) = exact
        .split_once('.')
        .expect("a fixed-point string has a fraction");
    // The integer digits, then the fraction digits that are kept; the next digit decides the rounding.
    let mut kept: Vec<u8> = integer
        .bytes()
        .chain(fraction.bytes().take(digits))
        .collect();
    if fraction.as_bytes()[digits] >= b'5' {
        increment_decimal(&mut kept);
    }

    let split = kept.len() - digits;
    let mut out = String::with_capacity(sign.len() + kept.len() + 1);
    out.push_str(sign);
    out.push_str(ascii(&kept[..split]));
    if digits > 0 {
        out.push('.');
        out.push_str(ascii(&kept[split..]));
    }
    out
}

/// `Number::toString` of a finite positive double of at least 1e21: the shortest round-trip digits, in the
/// form `d.ddde+N`. A digit string of one digit has no point.
fn exponent_form(magnitude: f64) -> String {
    let scientific = format!("{magnitude:e}");
    let (mantissa, exponent) = scientific
        .split_once('e')
        .expect("an exponent form has an e");
    format!("{mantissa}e+{exponent}")
}

/// Adds one to a string of decimal digits, carrying; a carry out of the first digit adds a leading `1`.
fn increment_decimal(digits: &mut Vec<u8>) {
    for digit in digits.iter_mut().rev() {
        if *digit == b'9' {
            *digit = b'0';
        } else {
            *digit += 1;
            return;
        }
    }
    digits.insert(0, b'1');
}

/// The bytes of a decimal string as text; they are ASCII digits, so this cannot fail.
fn ascii(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).expect("decimal digits are ASCII")
}

/// `Math.round(x)`: the integer nearest to `x`, with a tie going toward +infinity (`Math.round(-2.5)` is
/// `-2`, `Math.round(2.5)` is `3`).
///
/// `NaN`, the infinities, the zeros, and every value with `|x| >= 2^52` (already an integer) come back as they
/// are. A result of zero keeps the sign V8 gives it: a negative `x` that rounds to zero gives `-0`
/// (`Math.round(-0.4)` is `-0`), and a positive one gives `+0`.
#[must_use]
pub fn js_round(x: f64) -> f64 {
    if !x.is_finite() || x == 0.0 || x.abs() >= TWO_POW_52 {
        return x;
    }
    let floor = x.floor();
    // The difference `x - floor` decides the tie, and it is exact in the cases that matter. For |x| >= 1 it is
    // a multiple of the ulp of `x` below 1, so representable. For -1 <= x <= -0.5 it is `x + 1`, exact by
    // Sterbenz's lemma. For -0.5 < x < 0 the exact difference is above 0.5, and rounding cannot take it below
    // 0.5, so the result is zero, as it is for the exact value.
    let rounded = if x - floor >= 0.5 { floor + 1.0 } else { floor };
    if rounded == 0.0 {
        if x < 0.0 { -0.0 } else { 0.0 }
    } else {
        rounded
    }
}

/// The printable ASCII characters, 0x20 to 0x7E, in the order ICU's root collation sorts them, as V8's
/// `localeCompare` does with the default locale. The order was read from node (v24.21.0, ICU 78.3) by
/// sorting the 95 characters with `localeCompare`. Letters appear as lowercase then uppercase pairs: the two
/// cases share a primary weight, and the lowercase form sorts first at the tertiary level.
const ICU_ORDER: &[u8] = b" _-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$0123456789aAbBcCdDeEfFgGhHiIjJkKlLmMnNoOpPqQrRsStTuUvVwWxXyYzZ";

/// The primary weight of each byte: its position in [`ICU_ORDER`], with an uppercase letter sharing the
/// weight of its lowercase form. The entries for bytes outside printable ASCII are never read.
const PRIMARY: [u8; 128] = primary_table();

const fn primary_table() -> [u8; 128] {
    let mut table = [0u8; 128];
    let mut next = 0u8;
    let mut i = 0;
    while i < ICU_ORDER.len() {
        let byte = ICU_ORDER[i];
        if byte.is_ascii_uppercase() {
            table[byte as usize] = table[byte.to_ascii_lowercase() as usize];
        } else {
            table[byte as usize] = next;
            next += 1;
        }
        i += 1;
    }
    table
}

/// Whether a byte is printable ASCII, 0x20 to 0x7E.
const fn is_printable_ascii(byte: u8) -> bool {
    matches!(byte, 0x20..=0x7e)
}

/// `a.localeCompare(b)` in V8 with the default locale (en-US), for printable ASCII strings.
///
/// The order is ICU's root collation at tertiary strength with non-ignorable punctuation: the primary weights
/// are compared over the whole strings first (a string that is a prefix of the other comes first), then, when
/// the primary weights are equal, the case of the letters, lowercase first at the first difference. Two
/// different printable ASCII strings never compare `Equal`. The order is checked against V8 in
/// `tests/js_semantics.rs`, exhaustively for strings of up to two characters.
///
/// # Panics
///
/// Panics if either string has a byte outside printable ASCII (a control character, or any non-ASCII text).
/// [`try_locale_compare`] returns an error instead.
#[must_use]
pub fn locale_compare(a: &str, b: &str) -> Ordering {
    match try_locale_compare(a, b) {
        Ok(ordering) => ordering,
        Err(error) => panic!("localeCompare: {error}"),
    }
}

/// [`locale_compare`], for input that is not known to be printable ASCII.
///
/// # Errors
///
/// Returns [`Error::NotPrintableAscii`] with the string that has a byte outside printable ASCII (0x20 to
/// 0x7E). The collation is not ported for those.
pub fn try_locale_compare(a: &str, b: &str) -> Result<Ordering, Error> {
    for text in [a, b] {
        if !text.bytes().all(is_printable_ascii) {
            return Err(Error::NotPrintableAscii(text.to_string()));
        }
    }
    let (a, b) = (a.as_bytes(), b.as_bytes());

    // The primary level decides over the whole strings. Each ASCII byte has one primary weight.
    for (x, y) in a.iter().zip(b) {
        match PRIMARY[usize::from(*x)].cmp(&PRIMARY[usize::from(*y)]) {
            Ordering::Equal => {}
            other => return Ok(other),
        }
    }
    match a.len().cmp(&b.len()) {
        Ordering::Equal => {}
        other => return Ok(other),
    }

    // Equal primary weights mean the same characters, up to case. The tertiary level takes the first
    // difference in case; a lowercase letter (false) sorts before an uppercase one (true).
    for (x, y) in a.iter().zip(b) {
        match x.is_ascii_uppercase().cmp(&y.is_ascii_uppercase()) {
            Ordering::Equal => {}
            other => return Ok(other),
        }
    }
    Ok(Ordering::Equal)
}

/// `Array.prototype.sort(compare)` on `items`, in the order V8 gives for fewer than 64 elements: a first run
/// (`CountAndMakeRun`), then a binary insertion of the rest (`BinaryInsertionSort`). `compare` returns the number
/// the JavaScript comparator returns, and a NaN counts as `+0`, as the specification and V8 give it.
///
/// This is not `slice::sort_by`: that sort may panic on a comparator that is not a total order, and `a - b` is
/// not one when a key is NaN. For 64 elements or more V8 runs TimSort. With a consistent comparator every stable
/// sort gives the same order, which this one gives too; with a NaN key at that length the order can differ from
/// V8's.
pub fn js_sort_by<T: Copy>(items: &mut [T], compare: impl Fn(T, T) -> f64) {
    // `SortCompareUserFn`: a NaN result is `+0`, that is "equal".
    let compare = |a: T, b: T| {
        let order = compare(a, b);
        if order.is_nan() { 0.0 } else { order }
    };
    let len = items.len();
    if len < 2 {
        return;
    }

    // CountAndMakeRun: the first run is ascending, or strictly descending and then reversed.
    let is_descending = compare(items[1], items[0]) < 0.0;
    let mut run = 2;
    let mut previous = items[1];
    while run < len {
        let current = items[run];
        let order = compare(current, previous);
        let ends_run = if is_descending {
            order >= 0.0
        } else {
            order < 0.0
        };
        if ends_run {
            break;
        }
        previous = current;
        run += 1;
    }
    if is_descending {
        items[..run].reverse();
    }

    // BinaryInsertionSort: each following element goes to its place among the sorted elements before it.
    for start in run..len {
        let pivot = items[start];
        let (mut left, mut right) = (0, start);
        while left < right {
            let mid = left + ((right - left) >> 1);
            if compare(pivot, items[mid]) < 0.0 {
                right = mid;
            } else {
                left = mid + 1;
            }
        }
        items[left..=start].rotate_right(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed(x: f64, digits: u32) -> String {
        to_fixed(x, digits)
    }

    #[test]
    fn to_fixed_ties_round_up_in_magnitude() {
        assert_eq!(fixed(0.25, 1), "0.3");
        assert_eq!(fixed(0.125, 2), "0.13");
        assert_eq!(fixed(2.5, 0), "3");
        assert_eq!(fixed(-2.5, 0), "-3");
        assert_eq!(fixed(0.5, 0), "1");
        assert_eq!(fixed(1.5, 0), "2");
        assert_eq!(fixed(-0.5, 0), "-1");
    }

    #[test]
    fn to_fixed_decides_on_the_exact_binary_value() {
        // 0.35 is 0.34999999999999997..., below the tie, so it rounds down.
        assert_eq!(fixed(0.35, 1), "0.3");
        // 1.005 is 1.00499999999999989..., and 1.45 is 1.4499999999999999556...
        assert_eq!(fixed(1.005, 2), "1.00");
        assert_eq!(fixed(1.45, 1), "1.4");
        // 8.345 is 8.3450000000000006394..., above the tie.
        assert_eq!(fixed(8.345, 2), "8.35");
    }

    #[test]
    fn to_fixed_keeps_the_sign_of_a_value_that_rounds_to_zero() {
        assert_eq!(fixed(-0.04, 1), "-0.0");
        assert_eq!(fixed(-1e-300, 2), "-0.00");
        assert_eq!(fixed(-0.0, 1), "0.0");
        assert_eq!(fixed(0.0, 0), "0");
        assert_eq!(fixed(1e-7, 6), "0.000000");
        assert_eq!(fixed(5e-324, 0), "0");
    }

    #[test]
    fn to_fixed_carries_into_a_new_digit() {
        assert_eq!(fixed(9.96, 1), "10.0");
        assert_eq!(fixed(99.5, 0), "100");
        assert_eq!(fixed(0.999, 2), "1.00");
    }

    #[test]
    fn to_fixed_special_values_and_large_magnitudes() {
        assert_eq!(fixed(f64::NAN, 2), "NaN");
        assert_eq!(fixed(f64::INFINITY, 2), "Infinity");
        assert_eq!(fixed(f64::NEG_INFINITY, 2), "-Infinity");
        assert_eq!(fixed(1e20, 0), "100000000000000000000");
        assert_eq!(fixed(1e21, 2), "1e+21");
        assert_eq!(fixed(-1e21, 2), "-1e+21");
        assert_eq!(fixed(1.2345e25, 3), "1.2345e+25");
        assert_eq!(fixed(f64::MAX, 0), "1.7976931348623157e+308");
    }

    #[test]
    fn to_fixed_hundred_digits_are_exact() {
        // The exact expansion of 0.1 has 55 significant digits, then zeros.
        let text = fixed(0.1, 100);
        assert_eq!(
            text,
            format!(
                "0.1000000000000000055511151231257827021181583404541015625{}",
                "0".repeat(45)
            )
        );
        // The smallest subnormal has the longest expansion: 1074 fraction digits, the last a 5.
        let smallest = format!("{:.*}", EXACT_DIGITS, f64::from_bits(1));
        let fraction = smallest.split_once('.').expect("fraction").1;
        assert_eq!(fraction.trim_end_matches('0').len(), 1074);
        assert!(fraction.trim_end_matches('0').ends_with('5'));
    }

    #[test]
    #[should_panic(expected = "at most 100")]
    fn to_fixed_rejects_more_than_100_digits() {
        let _ = to_fixed(1.0, 101);
    }

    #[test]
    fn js_round_ties_go_toward_positive_infinity() {
        assert_eq!(js_round(2.5), 3.0);
        assert_eq!(js_round(-2.5), -2.0);
        assert_eq!(js_round(1.5), 2.0);
        assert_eq!(js_round(-1.5), -1.0);
        assert_eq!(js_round(4_503_599_627_370_495.5), 4_503_599_627_370_496.0);
        assert_eq!(js_round(-4_503_599_627_370_495.5), -4_503_599_627_370_495.0);
    }

    #[test]
    fn js_round_keeps_the_sign_of_zero() {
        assert_eq!(js_round(-0.0).to_bits(), (-0.0f64).to_bits());
        assert_eq!(js_round(0.0).to_bits(), 0.0f64.to_bits());
        assert_eq!(js_round(-0.5).to_bits(), (-0.0f64).to_bits());
        assert_eq!(js_round(-0.4).to_bits(), (-0.0f64).to_bits());
        assert_eq!(js_round(0.4).to_bits(), 0.0f64.to_bits());
        assert_eq!(
            js_round(-0.49999999999999994).to_bits(),
            (-0.0f64).to_bits()
        );
        assert_eq!(js_round(0.49999999999999994).to_bits(), 0.0f64.to_bits());
        assert_eq!(js_round(-1e-320).to_bits(), (-0.0f64).to_bits());
    }

    #[test]
    fn js_round_leaves_integers_and_special_values_alone() {
        let big = 2f64.powi(52) + 1.0;
        assert_eq!(js_round(big), big);
        assert_eq!(js_round(-2f64.powi(52)), -2f64.powi(52));
        assert_eq!(js_round(f64::INFINITY), f64::INFINITY);
        assert_eq!(js_round(f64::NEG_INFINITY), f64::NEG_INFINITY);
        assert!(js_round(f64::NAN).is_nan());
        assert_eq!(js_round(-0.5000000000000001), -1.0);
    }

    fn sorted(mut words: Vec<&str>) -> Vec<&str> {
        words.sort_by(|a, b| locale_compare(a, b));
        words
    }

    #[test]
    fn locale_compare_orders_like_v8() {
        let expected = [
            "_x", "10", "9", "a", "A", "a_b", "a-b", "ab", "b", "B", "z", "Z",
        ];
        let mut shuffled = expected.to_vec();
        shuffled.reverse();
        assert_eq!(sorted(shuffled), expected);
        assert_eq!(locale_compare("a_b", "a-b"), Ordering::Less);
        assert_eq!(locale_compare("A", "a_b"), Ordering::Less);
        assert_eq!(locale_compare("", " "), Ordering::Less);
        assert_eq!(locale_compare("zZ", "Zz"), Ordering::Less);
        assert_eq!(locale_compare("ZZ", "Zz"), Ordering::Greater);
    }

    #[test]
    fn locale_compare_equal_strings_are_equal() {
        assert_eq!(locale_compare("", ""), Ordering::Equal);
        assert_eq!(locale_compare("Node-7_x", "Node-7_x"), Ordering::Equal);
    }

    #[test]
    fn locale_compare_punctuation_precedes_digits_and_letters() {
        let order = " _-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$0123456789aAbBcCdDeEfFgGhHiIjJkKlLmMnNoOpPqQrRsStTuUvVwWxXyYzZ";
        let chars: Vec<String> = order.chars().map(String::from).collect();
        let mut sorted_chars = chars.clone();
        sorted_chars.sort_by(|a, b| locale_compare(a, b));
        assert_eq!(sorted_chars, chars);
    }

    #[test]
    fn try_locale_compare_refuses_what_is_not_printable_ascii() {
        assert_eq!(
            try_locale_compare("a\tb", "a"),
            Err(Error::NotPrintableAscii("a\tb".to_string()))
        );
        assert_eq!(
            try_locale_compare("a", "é"),
            Err(Error::NotPrintableAscii("é".to_string()))
        );
        assert_eq!(try_locale_compare("a", "b"), Ok(Ordering::Less));
    }

    #[test]
    #[should_panic(expected = "localeCompare")]
    fn locale_compare_panics_outside_printable_ascii() {
        let _ = locale_compare("\n", "a");
    }
}
