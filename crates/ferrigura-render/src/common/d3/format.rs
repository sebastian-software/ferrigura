//! `d3-format` 3.1.0 (`src/`): `format(specifier)` in the default locale, which `valueFormat` takes
//! (`treemap/renderer.ts` 51-77). The specifier is read as `formatSpecifier.js` reads it; the types
//! are those of `formatTypes.js`, and the locale is `defaultLocale.js` (`,` every three digits, `$`
//! for the currency, `−` for the minus sign).
//!
//! JavaScript's number text is reproduced exactly. `toFixed`, `toExponential` and `toPrecision`
//! round the exact binary value half up: the specification picks the larger of two candidates on a
//! tie, so `(2.5).toFixed(0)` is `"3"`, where Rust's formatting gives `"2"`. The digits come from
//! the exact decimal expansion of the double (`exact_digits`).
//!
//! The `s` type keeps the exponent of its last prefix in the formatter, as `formatPrefixAuto`
//! keeps `prefixExponent` in the module (a non-finite value reuses the last one, and before any
//! finite value the reference writes `undefined`). A formatter is one call's state, which matches
//! the reference, where each `valueFormat` is created for one render.

use std::cell::Cell;
use std::fmt;

use super::number::{js_number, js_round};

/// The prefixes of the `s` type, from `y` (`10^-24`) to `Y` (`10^24`), `µ` being U+00B5.
const PREFIXES: [&str; 17] = [
    "y", "z", "a", "f", "p", "n", "µ", "m", "", "k", "M", "G", "T", "P", "E", "Z", "Y",
];

/// The types `formatTypes.js` names. Any other type is the `.12~g` default.
const TYPES: [&str; 13] = [
    "%", "b", "c", "d", "e", "f", "g", "o", "p", "r", "s", "X", "x",
];

/// `2^53`: a double from this value on has no unit digit of its own (`Double#Exponent() > 0`).
const TWO_TO_THE_53: f64 = 9_007_199_254_740_992.0;

/// `1e21`: from this value on `toFixed` writes the exponent form, and `d` writes digits from
/// `toLocaleString`.
const TEN_TO_THE_21: f64 = 1e21;

/// The error of an invalid specifier: `formatSpecifier` throws `invalid format: <specifier>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatError {
    /// The specifier as it was given.
    pub specifier: String,
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid format: {}", self.specifier)
    }
}

impl std::error::Error for FormatError {}

/// A number format, as `format(specifier)` returns it. Its fields are the resolved options of
/// `newFormat` (`locale.js` 30-95).
#[derive(Debug, Clone)]
pub struct Formatter {
    fill: String,
    align: char,
    sign: String,
    zero: bool,
    width: Option<f64>,
    comma: bool,
    trim: bool,
    kind: String,
    precision: f64,
    prefix: String,
    suffix: String,
    maybe_suffix: bool,
    /// `prefixExponent` of `formatPrefixAuto`: `None` until an `s` format has a finite value.
    prefix_exponent: Cell<Option<i64>>,
}

/// `format(specifier)`: the formatter of a d3 specifier in the default locale.
///
/// # Errors
///
/// Returns [`FormatError`] when the specifier does not match `[[fill]align][sign][symbol][0][width]
/// [,][.precision][~][type]`.
pub fn format(specifier: &str) -> Result<Formatter, FormatError> {
    let parts = parse_specifier(specifier).ok_or_else(|| FormatError {
        specifier: specifier.to_owned(),
    })?;
    let Parts {
        mut fill,
        mut align,
        sign,
        symbol,
        mut zero,
        width,
        mut comma,
        mut precision,
        mut trim,
        kind: mut type_,
    } = parts;

    // The `n` type is `,g`; a type that is not one of the formats is `.12~g`.
    if type_ == "n" {
        comma = true;
        type_ = "g".to_owned();
    } else if !TYPES.contains(&type_.as_str()) {
        if precision.is_none() {
            precision = Some(12.0);
        }
        trim = true;
        type_ = "g".to_owned();
    }

    // Zero filling goes after the sign and before the digits.
    if zero || (fill == "0" && align == '=') {
        zero = true;
        fill = "0".to_owned();
        align = '=';
    }

    let prefix = if symbol == Some('$') {
        "$".to_owned()
    } else if symbol == Some('#') && matches!(type_.as_str(), "b" | "o" | "x" | "X") {
        format!("0{}", type_.to_ascii_lowercase())
    } else {
        String::new()
    };
    let suffix = if symbol == Some('$') {
        String::new()
    } else if matches!(type_.as_str(), "%" | "p") {
        "%".to_owned()
    } else {
        String::new()
    };
    let maybe_suffix = matches!(
        type_.as_str(),
        "d" | "e" | "f" | "g" | "p" | "r" | "s" | "%"
    );

    // The precision: 6 by default; significant digits for `gprs` in [1, 21], decimals otherwise
    // in [0, 20].
    let precision = match precision {
        None => 6.0,
        Some(value) if matches!(type_.as_str(), "g" | "p" | "r" | "s") => value.clamp(1.0, 21.0),
        Some(value) => value.clamp(0.0, 20.0),
    };

    Ok(Formatter {
        fill,
        align,
        sign: sign.map_or_else(|| "-".to_owned(), String::from),
        zero,
        width,
        comma,
        trim,
        kind: type_,
        precision,
        prefix,
        suffix,
        maybe_suffix,
        prefix_exponent: Cell::new(None),
    })
}

impl Formatter {
    /// The text of a number in this format, as the formatter's function gives it (`format(value)`,
    /// `locale.js` 97-156).
    #[must_use]
    pub fn call(&self, value: f64) -> String {
        let mut value_prefix = self.prefix.clone();
        let mut value_suffix = self.suffix.clone();
        let mut digits: String;
        if self.kind == "c" {
            // The `c` type writes the value itself as the suffix, unsigned and unformatted.
            value_suffix = format!("{}{value_suffix}", js_number(value));
            digits = String::new();
        } else {
            let mut value_negative = value < 0.0 || (1.0 / value) < 0.0;
            digits = if value.is_nan() {
                "NaN".to_owned()
            } else {
                self.format_type(value.abs())
            };
            if self.trim {
                digits = trim_insignificant_zeros(&digits);
            }
            // A negative value that rounds to zero has no sign, unless the sign is `+`.
            if value_negative && is_zero_text(&digits) && self.sign != "+" {
                value_negative = false;
            }
            let sign_text = if value_negative {
                if self.sign == "(" {
                    "(".to_owned()
                } else {
                    "−".to_owned()
                }
            } else if self.sign == "-" || self.sign == "(" {
                String::new()
            } else {
                self.sign.clone()
            };
            value_prefix = format!("{sign_text}{value_prefix}");
            let exponent_prefix = if self.kind == "s" {
                self.si_prefix()
            } else {
                String::new()
            };
            let closing = if value_negative && self.sign == "(" {
                ")"
            } else {
                ""
            };
            value_suffix = format!("{exponent_prefix}{value_suffix}{closing}");
            let split = if self.maybe_suffix {
                digits.bytes().position(|byte| !byte.is_ascii_digit())
            } else {
                None
            };
            if let Some(index) = split {
                // A point moves on without itself; the rest moves as it is.
                let moved = if digits.as_bytes()[index] == b'.' {
                    format!(".{}", &digits[index + 1..])
                } else {
                    digits[index..].to_owned()
                };
                value_suffix = format!("{moved}{value_suffix}");
                digits.truncate(index);
            }
        }

        // Grouping comes before the padding unless the padding is zeros.
        if self.comma && !self.zero {
            digits = group(&digits, f64::INFINITY);
        }
        let length = js_len(&value_prefix) + js_len(&digits) + js_len(&value_suffix);
        let mut padding = match self.width {
            Some(width) if (length as f64) < width => {
                self.fill.repeat((width - length as f64) as usize)
            }
            _ => String::new(),
        };
        // Grouping after the padding: the zeros are grouped with the digits.
        if self.comma && self.zero {
            let group_width = if padding.is_empty() {
                f64::INFINITY
            } else {
                self.width.unwrap_or(f64::INFINITY) - js_len(&value_suffix) as f64
            };
            digits = group(&format!("{padding}{digits}"), group_width);
            padding.clear();
        }

        match self.align {
            '<' => format!("{value_prefix}{digits}{value_suffix}{padding}"),
            '=' => format!("{value_prefix}{padding}{digits}{value_suffix}"),
            '^' => {
                let half = js_len(&padding) / 2;
                let (left, right) = split_units(&padding, half);
                format!("{left}{value_prefix}{digits}{value_suffix}{right}")
            }
            _ => format!("{padding}{value_prefix}{digits}{value_suffix}"),
        }
    }

    /// `formatTypes[type](x, precision)`, for the absolute value `x`.
    fn format_type(&self, x: f64) -> String {
        let p = self.precision;
        match self.kind.as_str() {
            "%" => to_fixed(x * 100.0, p as usize),
            "b" => radix_text(js_round(x), 2, false),
            "d" => format_decimal(x),
            "e" => to_exponential(x, p as usize),
            "f" => to_fixed(x, p as usize),
            "g" => to_precision(x, p as usize),
            "o" => radix_text(js_round(x), 8, false),
            "p" => formatted_rounded(x * 100.0, p as usize),
            "r" => formatted_rounded(x, p as usize),
            "s" => self.format_prefix_auto(x, p as usize),
            "X" => radix_text(js_round(x), 16, true),
            "x" => radix_text(js_round(x), 16, false),
            _ => js_number(x),
        }
    }

    /// `formatPrefixAuto(x, p)`: the number with the SI prefix of its exponent, which the formatter
    /// remembers for the suffix.
    fn format_prefix_auto(&self, x: f64, p: usize) -> String {
        let Some((coefficient, exponent)) = decimal_parts(x, p) else {
            return js_number(x);
        };
        let prefix_exponent = (exponent.div_euclid(3)).clamp(-8, 8) * 3;
        self.prefix_exponent.set(Some(prefix_exponent));
        let i = exponent - prefix_exponent + 1;
        let n = coefficient.len() as i64;
        if i == n {
            coefficient
        } else if i > n {
            format!("{coefficient}{}", "0".repeat((i - n) as usize))
        } else if i > 0 {
            let (head, tail) = coefficient.split_at(i as usize);
            format!("{head}.{tail}")
        } else {
            // Less than one: the zeros after the point, then the digits at the precision that
            // keeps `p` significant digits (zero digits take the shortest form).
            let precision = (p as i64 + i - 1).max(0) as usize;
            let digits = decimal_parts(x, precision).map_or_else(String::new, |parts| parts.0);
            format!("0.{}{digits}", "0".repeat((-i) as usize))
        }
    }

    /// The SI prefix of the last `s` format: `prefixes[8 + prefixExponent / 3]`. Before any finite
    /// value, the reference's lookup is `undefined`.
    fn si_prefix(&self) -> String {
        match self.prefix_exponent.get() {
            Some(exponent) => {
                let index = 8 + exponent / 3;
                PREFIXES[index as usize].to_owned()
            }
            None => "undefined".to_owned(),
        }
    }
}

/// `formatDecimal(x)` (`d`): `Math.round(x)`, written as an integer. From `1e21` on, the digits of
/// `toLocaleString("en")` without its commas: the shortest digits, with zeros after them.
fn format_decimal(x: f64) -> String {
    let rounded = js_round(x);
    if rounded.abs() >= TEN_TO_THE_21 {
        if !rounded.is_finite() {
            return "∞".to_owned();
        }
        let (digits, exponent) = shortest_digits(rounded.abs());
        let zeros = exponent + 1 - digits.len() as i64;
        let sign = if rounded < 0.0 { "−" } else { "" };
        return format!("{sign}{digits}{}", "0".repeat(zeros.max(0) as usize));
    }
    js_number(rounded)
}

/// The `r` and `p` types (`formatRounded.js`): `x` at `p` significant digits, in plain notation.
fn formatted_rounded(x: f64, p: usize) -> String {
    let Some((coefficient, exponent)) = decimal_parts(x, p) else {
        return js_number(x);
    };
    let length = coefficient.len() as i64;
    if exponent < 0 {
        format!("0.{}{coefficient}", "0".repeat((-exponent - 1) as usize))
    } else if length > exponent + 1 {
        let (head, tail) = coefficient.split_at((exponent + 1) as usize);
        format!("{head}.{tail}")
    } else {
        format!(
            "{coefficient}{}",
            "0".repeat((exponent - length + 1).max(0) as usize)
        )
    }
}

/// `formatDecimalParts(x, p)` (`formatDecimal.js`): the significant digits of `x` at `p` digits, and
/// the exponent of the first. A zero `p` (the case when `formatPrefixAuto` asks for `max(0, …)`) is
/// falsy in the reference and takes the shortest form. `None` for a non-finite `x`.
fn decimal_parts(x: f64, p: usize) -> Option<(String, i64)> {
    if !x.is_finite() {
        return None;
    }
    if p == 0 {
        return Some(shortest_digits(x));
    }
    let (digits, exponent) = rounded_digits(x, p);
    Some((digits, exponent))
}

/// The shortest digits that read back as `x` (`toExponential()` without an argument), and the
/// exponent of the first digit. `x` is finite and not negative.
fn shortest_digits(x: f64) -> (String, i64) {
    let text = format!("{:e}", x.abs());
    let (mantissa, exponent) = text.split_once('e').unwrap_or((text.as_str(), "0"));
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    (digits, exponent.parse().unwrap_or(0))
}

/// The exact decimal expansion of a finite `x >= 0`, as the digits of its significant part (no
/// leading zeros; `[0]` for zero) and the exponent of the first digit. A double has at most 767
/// significant digits, so 800 places are exact.
fn exact_digits(x: f64) -> (Vec<u8>, i64) {
    if x == 0.0 {
        return (vec![0], 0);
    }
    let text = format!("{:.800e}", x.abs());
    let (mantissa, exponent) = text.split_once('e').unwrap_or((text.as_str(), "0"));
    let digits = mantissa
        .bytes()
        .filter(u8::is_ascii_digit)
        .map(|byte| byte - b'0')
        .collect();
    (digits, exponent.parse().unwrap_or(0))
}

/// The digits of `x` rounded half up to `keep` significant digits, and the exponent of the first:
/// `(digits, exponent)` with `keep` digits. The exponent rises when the rounding carries.
fn round_digits(digits: &[u8], exponent: i64, keep: usize) -> (Vec<u8>, i64) {
    let mut kept: Vec<u8> = (0..keep)
        .map(|index| digits.get(index).copied().unwrap_or(0))
        .collect();
    let mut carry = digits.get(keep).is_some_and(|digit| *digit >= 5);
    let mut index = keep;
    while carry && index > 0 {
        index -= 1;
        if kept[index] == 9 {
            kept[index] = 0;
        } else {
            kept[index] += 1;
            carry = false;
        }
    }
    if carry {
        kept.insert(0, 1);
        kept.pop();
        return (kept, exponent + 1);
    }
    (kept, exponent)
}

/// `p` significant digits of `x`, rounded half up, and the exponent of the first. `x` is finite.
fn rounded_digits(x: f64, p: usize) -> (String, i64) {
    if x == 0.0 {
        return ("0".repeat(p), 0);
    }
    let (digits, exponent) = exact_digits(x);
    let (kept, exponent) = round_digits(&digits, exponent, p);
    (
        kept.iter().map(|digit| char::from(b'0' + digit)).collect(),
        exponent,
    )
}

/// `toExponential(f)`: `x` as `d.ddd` with `f` decimals, and `e±x`. `x` is finite and not negative.
fn to_exponential(x: f64, decimals: usize) -> String {
    if !x.is_finite() {
        return js_number(x);
    }
    let (digits, exponent) = rounded_digits(x, decimals + 1);
    let mut out = digits[..1].to_owned();
    if decimals > 0 {
        out.push('.');
        out.push_str(&digits[1..]);
    }
    out.push_str(&exponent_text(exponent));
    out
}

/// `toFixed(f)`: `x` with `f` decimals, rounded half up on the exact value. A `x` of `1e21` or more
/// is written as `toString` writes it.
fn to_fixed(x: f64, decimals: usize) -> String {
    if !x.is_finite() || x.abs() >= TEN_TO_THE_21 {
        return js_number(x);
    }
    let sign = if x < 0.0 { "-" } else { "" };
    let integer = scaled_integer(x.abs(), decimals);
    if decimals == 0 {
        return format!("{sign}{integer}");
    }
    let padded = format!("{integer:0>width$}", width = decimals + 1);
    let (whole, fraction) = padded.split_at(padded.len() - decimals);
    format!("{sign}{whole}.{fraction}")
}

/// The integer nearest `x * 10^decimals` for a finite `x >= 0`, rounded half up, as its digits.
fn scaled_integer(x: f64, decimals: usize) -> String {
    if x == 0.0 {
        return "0".to_owned();
    }
    let (digits, exponent) = exact_digits(x);
    // The integer part of `x * 10^decimals` has `keep` digits.
    let keep = exponent + 1 + decimals as i64;
    if keep < 0 {
        return "0".to_owned();
    }
    if keep == 0 {
        return if digits[0] >= 5 { "1" } else { "0" }.to_owned();
    }
    let (kept, rounded_exponent) = round_digits(&digits, exponent, keep as usize);
    if rounded_exponent != exponent {
        // The carry adds a digit: the integer is `10^keep`.
        return std::iter::once('1')
            .chain(std::iter::repeat_n('0', keep as usize))
            .collect();
    }
    kept.iter().map(|digit| char::from(b'0' + digit)).collect()
}

/// `toPrecision(p)`: `x` at `p` significant digits, in the plain or the exponent form the
/// specification picks.
fn to_precision(x: f64, precision: usize) -> String {
    if !x.is_finite() {
        return js_number(x);
    }
    let sign = if x < 0.0 { "-" } else { "" };
    let (digits, exponent) = rounded_digits(x.abs(), precision);
    let body = if exponent < -6 || exponent >= precision as i64 {
        let mut out = digits[..1].to_owned();
        if precision > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push_str(&exponent_text(exponent));
        out
    } else if exponent == precision as i64 - 1 {
        digits
    } else if exponent >= 0 {
        let (head, tail) = digits.split_at(exponent as usize + 1);
        format!("{head}.{tail}")
    } else {
        format!("0.{}{digits}", "0".repeat((-exponent - 1) as usize))
    };
    format!("{sign}{body}")
}

/// The `e±x` part of an exponent form.
fn exponent_text(exponent: i64) -> String {
    if exponent < 0 {
        format!("e-{}", -exponent)
    } else {
        format!("e+{exponent}")
    }
}

/// `Number#toString(radix)` of an integer-valued double, as V8 writes it: the digits above 2^53
/// are unrepresented, so each division by the radix that stays at 2^53 or more writes a zero.
fn radix_text(value: f64, radix: u32, upper: bool) -> String {
    if !value.is_finite() {
        return js_number(value);
    }
    if value == 0.0 {
        return "0".to_owned();
    }
    let sign = if value < 0.0 { "-" } else { "" };
    let radix_f = f64::from(radix);
    let mut integer = value.abs().floor();
    let mut reversed: Vec<char> = Vec::new();
    while integer / radix_f >= TWO_TO_THE_53 {
        integer /= radix_f;
        reversed.push('0');
    }
    loop {
        let remainder = integer % radix_f;
        reversed.push(digit_char(remainder as u32));
        integer = (integer - remainder) / radix_f;
        if integer <= 0.0 {
            break;
        }
    }
    let text: String = reversed.into_iter().rev().collect();
    let text = if upper { text.to_uppercase() } else { text };
    format!("{sign}{text}")
}

/// The digit of a value below the radix, lowercase.
fn digit_char(digit: u32) -> char {
    char::from_digit(digit, 36).unwrap_or('0')
}

/// `formatGroup` with `[3]` and `,`: the digits in groups of three from the right, the last group
/// no wider than `width` allows.
fn group(value: &str, width: f64) -> String {
    let chars: Vec<char> = value.chars().collect();
    let mut i = chars.len() as i64;
    let mut groups: Vec<String> = Vec::new();
    let mut g: i64 = 3;
    let mut length: f64 = 0.0;
    while i > 0 && g > 0 {
        if length + g as f64 + 1.0 > width {
            g = 1.max((width - length) as i64);
        }
        let end = i;
        i -= g;
        let start = i.max(0);
        groups.push(chars[start as usize..end as usize].iter().collect());
        length += g as f64 + 1.0;
        if length > width {
            break;
        }
    }
    groups.reverse();
    groups.join(",")
}

/// `formatTrim` (`formatTrim.js`): the zeros after the point, and the point when nothing follows.
fn trim_insignificant_zeros(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut i0: i64 = -1;
    let mut i1: i64 = -1;
    for (index, &byte) in bytes.iter().enumerate().skip(1) {
        match byte {
            b'.' => {
                i0 = index as i64;
                i1 = index as i64;
            }
            b'0' => {
                if i0 == 0 {
                    i0 = index as i64;
                }
                i1 = index as i64;
            }
            byte => {
                // `!+s[i]`: a character that is not a digit stops the scan.
                if !byte.is_ascii_digit() {
                    break;
                }
                if i0 > 0 {
                    i0 = 0;
                }
            }
        }
    }
    if i0 > 0 {
        format!("{}{}", &text[..i0 as usize], &text[(i1 + 1) as usize..])
    } else {
        text.to_owned()
    }
}

/// `+text === 0` for the formatted text: a number that reads as zero.
fn is_zero_text(text: &str) -> bool {
    text.trim().parse::<f64>().is_ok_and(|value| value == 0.0)
}

/// The length of a text in UTF-16 units, as JavaScript's `length` counts it.
fn js_len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// The first `units` of a text in UTF-16 units, and the rest. A unit that splits a character goes to
/// the rest, which is only reached for a fill of more than one unit.
fn split_units(text: &str, units: usize) -> (String, String) {
    let mut taken = 0;
    for (index, character) in text.char_indices() {
        if taken + character.len_utf16() > units {
            return (text[..index].to_owned(), text[index..].to_owned());
        }
        taken += character.len_utf16();
    }
    (text.to_owned(), String::new())
}

/// The fields of a specifier, as the regular expression of `formatSpecifier.js` reads them.
#[derive(Debug, Default, Clone, PartialEq)]
struct Parts {
    fill: String,
    align: char,
    sign: Option<char>,
    symbol: Option<char>,
    zero: bool,
    width: Option<f64>,
    comma: bool,
    precision: Option<f64>,
    trim: bool,
    kind: String,
}

/// `re = /^(?:(.)?([<>=^]))?([+\-( ])?([$#])?(0)?(\d+)?(,)?(\.\d+)?(~)?([a-z%])?$/i`, matched
/// greedily: each optional part takes its text when the text has one. No part can take the text
/// of a later one, so the greedy match is the only one the expression accepts.
fn parse_specifier(specifier: &str) -> Option<Parts> {
    let chars: Vec<char> = specifier.chars().collect();
    let is_align = |c: char| matches!(c, '<' | '>' | '=' | '^');
    let mut parts = Parts {
        fill: " ".to_owned(),
        align: '>',
        ..Parts::default()
    };
    let mut i = 0;
    let at = |index: usize| chars.get(index).copied();

    // `(.)?([<>=^])`: a fill and an alignment, or the alignment alone.
    if let (Some(fill), Some(align)) = (at(0), at(1))
        && fill != '\n'
        && is_align(align)
    {
        parts.fill = fill.to_string();
        parts.align = align;
        i = 2;
    } else if let Some(align) = at(0).filter(|c| is_align(*c)) {
        parts.align = align;
        i = 1;
    }
    if let Some(sign) = at(i).filter(|c| matches!(c, '+' | '-' | '(' | ' ')) {
        parts.sign = Some(sign);
        i += 1;
    }
    if let Some(symbol) = at(i).filter(|c| matches!(c, '$' | '#')) {
        parts.symbol = Some(symbol);
        i += 1;
    }
    if at(i) == Some('0') {
        parts.zero = true;
        i += 1;
    }
    let digits_from = |start: usize| -> usize {
        let mut end = start;
        while chars.get(end).is_some_and(char::is_ascii_digit) {
            end += 1;
        }
        end
    };
    let width_end = digits_from(i);
    if width_end > i {
        let text: String = chars[i..width_end].iter().collect();
        parts.width = text.parse::<f64>().ok();
        i = width_end;
    }
    if at(i) == Some(',') {
        parts.comma = true;
        i += 1;
    }
    if at(i) == Some('.') && at(i + 1).is_some_and(|c| c.is_ascii_digit()) {
        let end = digits_from(i + 1);
        let text: String = chars[i + 1..end].iter().collect();
        parts.precision = text.parse::<f64>().ok();
        i = end;
    }
    if at(i) == Some('~') {
        parts.trim = true;
        i += 1;
    }
    if let Some(kind) = at(i).filter(|c| c.is_ascii_alphabetic() || *c == '%') {
        parts.kind = kind.to_string();
        i += 1;
    }
    (i == chars.len()).then_some(parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(specifier: &str, value: f64) -> String {
        format(specifier).expect("a valid specifier").call(value)
    }

    #[test]
    fn the_default_type_keeps_twelve_significant_digits_and_trims_them() {
        assert_eq!(fmt("", 0.0000123), "0.0000123");
        assert_eq!(fmt("", 1.5), "1.5");
        assert_eq!(fmt("", 1_001_234_567.0), "1001234567");
        assert_eq!(fmt("", 1_234_567.123_456_789), "1234567.12346");
    }

    #[test]
    fn the_comma_groups_the_digits_of_the_default_type() {
        assert_eq!(fmt(",", 1_234_567.891), "1,234,567.891");
        assert_eq!(fmt(",", 1_387_459.0), "1,387,459");
        assert_eq!(fmt(",", 1e21), "1e+21");
        assert_eq!(fmt(",f", 1_234_567.0), "1,234,567.000000");
    }

    #[test]
    fn fixed_rounds_the_exact_binary_value_and_a_tie_goes_up() {
        assert_eq!(fmt(".2f", 1.005), "1.00");
        assert_eq!(fmt(".0f", 2.5), "3");
        assert_eq!(fmt(".1f", 0.25), "0.3");
        assert_eq!(fmt(".2f", 369.956), "369.96");
        assert_eq!(fmt(".0f", 0.4), "0");
        assert_eq!(fmt(".2f", 0.006), "0.01");
        assert_eq!(fmt(".2f", 0.004), "0.00");
    }

    #[test]
    fn significant_digits_and_the_group_separator_follow_g() {
        assert_eq!(fmt(",.2", 1_645_789.25), "1.6e+6");
        assert_eq!(fmt(".3", 1234.0), "1.23e+3");
        assert_eq!(fmt(".2", 0.0123), "0.012");
    }

    #[test]
    fn percent_and_prefixes() {
        assert_eq!(fmt(".1%", 0.1234), "12.3%");
        assert_eq!(fmt(".2s", 1_234_567.0), "1.2M");
        assert_eq!(fmt(".2s", 0.0000123), "12µ");
        assert_eq!(fmt("s", 1_500.0), "1.50000k");
    }

    #[test]
    fn exponent_and_rounded_forms() {
        assert_eq!(fmt("e", 12_345.0), "1.234500e+4");
        assert_eq!(fmt(".2e", 12_345.0), "1.23e+4");
        assert_eq!(fmt(".1p", 0.1234), "10%");
        assert_eq!(fmt(".2r", 1234.0), "1200");
    }

    #[test]
    fn the_integer_radixes_and_the_character_type() {
        assert_eq!(fmt("b", 10.0), "1010");
        assert_eq!(fmt("#x", 255.0), "0xff");
        assert_eq!(fmt("X", 255.0), "FF");
        assert_eq!(fmt("o", 8.0), "10");
        assert_eq!(fmt("c", 65.0), "65");
    }

    #[test]
    fn the_sign_fill_and_alignment_are_applied() {
        assert_eq!(fmt("+.1f", 2.0), "+2.0");
        assert_eq!(fmt("*^9", 42.0), "***42****");
        assert_eq!(fmt("08,", 1234.0), "0,001,234");
        assert_eq!(fmt("<6", 12.0), "12    ");
        assert_eq!(fmt("(.1f", -2.0), "(2.0)");
        assert_eq!(fmt(".1f", -0.04), "0.0");
    }

    #[test]
    fn a_negative_value_takes_the_minus_sign_of_the_locale() {
        assert_eq!(fmt(",", -1234.0), "−1,234");
    }

    #[test]
    fn a_negative_zero_keeps_its_sign_only_for_an_explicit_plus() {
        // `1 / -0` is negative, so -0 has a sign unless the rounded text is zero and the sign is not `+`.
        assert_eq!(fmt("+#.0~e", -0.0), "−0e+0");
        assert_eq!(fmt(".1f", -0.0), "0.0");
    }

    #[test]
    fn an_invalid_specifier_is_an_error_with_d3s_message() {
        let error = format("xyz").expect_err("xyz is not a specifier");
        assert_eq!(error.to_string(), "invalid format: xyz");
        assert!(format("%%").is_err());
    }

    #[test]
    fn an_integer_from_two_to_the_21st_keeps_its_shortest_digits_then_zeros() {
        assert_eq!(fmt("d", 1e21), "1000000000000000000000");
        assert_eq!(fmt("d", 1.2345e25), "12345000000000000000000000");
    }

    #[test]
    fn radix_text_writes_zeros_for_digits_past_two_to_the_53() {
        assert_eq!(
            radix_text(9_007_199_254_740_992.0, 10, false),
            "9007199254740992"
        );
        assert_eq!(radix_text(1e21, 10, false), "1000000000000000000000");
    }
}
