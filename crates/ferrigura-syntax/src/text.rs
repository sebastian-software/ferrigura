//! String helpers that behave like the JavaScript regular expressions and string methods the
//! TypeScript reference uses.
//!
//! JavaScript's `\s` is a fixed list of code points, which is not Rust's `char::is_whitespace`
//! (that one includes U+0085 and omits U+FEFF), so the list is spelled out here.

/// Whether `c` matches JavaScript's `\s`.
pub fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' | ' ' | '\u{A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

/// Whether `c` is a line terminator for the `m` flag, i.e. what `^` and `$` match around.
pub fn is_line_term(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// Whether `c` matches `[\n\r]`.
pub fn is_cr_lf(c: char) -> bool {
    c == '\n' || c == '\r'
}

/// Whether `c` matches `\w`, which in JavaScript is ASCII only.
pub fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `String.prototype.trim` with JavaScript's whitespace.
pub fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

/// `replace(/[\t ]{2,}/g, ' ')`: each run of at least two tabs or spaces becomes one space.
pub fn collapse_blanks(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut run = String::new();
    for c in s.chars() {
        if c == '\t' || c == ' ' {
            run.push(c);
            continue;
        }
        flush_blank_run(&mut out, &mut run);
        out.push(c);
    }
    flush_blank_run(&mut out, &mut run);
    out
}

fn flush_blank_run(out: &mut String, run: &mut String) {
    match run.chars().count() {
        0 => {}
        1 => out.push_str(run),
        _ => out.push(' '),
    }
    run.clear();
}

/// `replace(/[\n\r]{2,}/g, '\n')`: each run of at least two line breaks becomes one newline.
pub fn collapse_line_breaks(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut run = String::new();
    for c in s.chars() {
        if is_cr_lf(c) {
            run.push(c);
            continue;
        }
        flush_line_break_run(&mut out, &mut run);
        out.push(c);
    }
    flush_line_break_run(&mut out, &mut run);
    out
}

fn flush_line_break_run(out: &mut String, run: &mut String) {
    // A single line break is kept as written; only a run of two or more is replaced.
    match run.chars().count() {
        0 => {}
        1 => out.push_str(run),
        _ => out.push('\n'),
    }
    run.clear();
}

/// `replace(/^\s*/gm, '')`: removes the whitespace at the start of every line, including the
/// line breaks that a `\s*` run swallows.
pub fn strip_line_starts(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        let at_line_start = i == 0 || is_line_term(chars[i - 1]);
        if at_line_start {
            let mut end = i;
            while end < chars.len() && is_js_space(chars[end]) {
                end += 1;
            }
            if end > i {
                i = end;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// `replace(/\s+$/gm, '')`: removes the whitespace at the end of every line.
///
/// The regular expression is greedy, so a run of whitespace that spans several line breaks is
/// cut at the last line break inside it: that line break and everything after it stay.
pub fn strip_line_ends(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < n {
        if !is_js_space(chars[i]) {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        let mut end = i;
        while end < n && is_js_space(chars[end]) {
            end += 1;
        }
        let cut = if end == n {
            Some(end)
        } else {
            (i + 1..end).rev().find(|&k| is_line_term(chars[k]))
        };
        match cut {
            Some(p) => i = p,
            None => {
                out.extend(&chars[i..end]);
                i = end;
            }
        }
    }
    out
}

/// `replace(/^\s+/g, '')`: removes leading whitespace of the whole string.
pub fn strip_leading_space(s: &str) -> &str {
    s.trim_start_matches(is_js_space)
}

/// `replace(/\n\s+/g, '\n')`: each line break followed by whitespace becomes a single line break.
pub fn newline_then_space_to_newline(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        out.push(chars[i]);
        if chars[i] == '\n' {
            let mut end = i + 1;
            while end < chars.len() && is_js_space(chars[end]) {
                end += 1;
            }
            i = end;
            continue;
        }
        i += 1;
    }
    out
}

/// `Number(text)`: JavaScript's conversion of a string to a number (`StringToNumber`). Blank text is
/// 0; `0x`, `0o` and `0b` take no sign; `Infinity` takes an optional sign; a decimal literal takes a
/// sign, a fraction and an exponent. Anything else, `NaN` included, is NaN.
#[must_use]
pub fn js_number(text: &str) -> f64 {
    let trimmed = js_trim(text);
    if trimmed.is_empty() {
        return 0.0;
    }
    for (prefix, radix) in [
        ("0x", 16),
        ("0X", 16),
        ("0o", 8),
        ("0O", 8),
        ("0b", 2),
        ("0B", 2),
    ] {
        if let Some(digits) = trimmed.strip_prefix(prefix) {
            return radix_number(digits, radix);
        }
    }
    match trimmed {
        "Infinity" | "+Infinity" => f64::INFINITY,
        "-Infinity" => f64::NEG_INFINITY,
        _ if is_decimal_literal(trimmed) => trimmed.parse().unwrap_or(f64::NAN),
        _ => f64::NAN,
    }
}

/// Whether `text` is a `StrDecimalLiteral` without `Infinity`: an optional sign, digits with an
/// optional fraction (at least one digit in all), and an optional exponent with digits.
fn is_decimal_literal(text: &str) -> bool {
    let unsigned = text.strip_prefix(['+', '-']).unwrap_or(text);
    let (mantissa, exponent) = match unsigned.find(['e', 'E']) {
        Some(at) => (&unsigned[..at], Some(&unsigned[at + 1..])),
        None => (unsigned, None),
    };
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let all_digits = |part: &str| part.bytes().all(|b| b.is_ascii_digit());
    let mantissa_ok =
        all_digits(whole) && all_digits(fraction) && !(whole.is_empty() && fraction.is_empty());
    let exponent_ok = exponent.is_none_or(|exp| {
        let digits = exp.strip_prefix(['+', '-']).unwrap_or(exp);
        !digits.is_empty() && all_digits(digits)
    });
    mantissa_ok && exponent_ok
}

/// The value of hexadecimal, octal or binary `digits`, rounded to the nearest double as JavaScript
/// rounds it: the bits are read in order, the first 53 are kept, and the rest decide the rounding.
fn radix_number(digits: &str, radix: u32) -> f64 {
    if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
        return f64::NAN;
    }
    let bits_per_digit = radix.trailing_zeros();
    let mut bits: Vec<bool> = Vec::with_capacity(digits.len() * bits_per_digit as usize);
    for c in digits.chars() {
        let value = c.to_digit(radix).unwrap_or(0);
        for shift in (0..bits_per_digit).rev() {
            bits.push((value >> shift) & 1 == 1);
        }
    }
    let first = bits.iter().position(|&bit| bit).unwrap_or(bits.len());
    let bits = &bits[first..];
    let length = bits.len();
    if length == 0 {
        return 0.0;
    }
    let read = |slice: &[bool]| {
        slice
            .iter()
            .fold(0u64, |acc, &bit| acc * 2 + u64::from(bit))
    };
    if length <= 53 {
        return read(bits) as f64;
    }
    let mut mantissa = read(&bits[..53]);
    let round_up = bits[53] && (bits[54..].iter().any(|&bit| bit) || mantissa & 1 == 1);
    if round_up {
        mantissa += 1;
    }
    // The value is `mantissa * 2^(length - 53)`; the scale is a power of two, so the product is exact
    // until it overflows to infinity.
    let scale = 2f64.powi(i32::try_from(length - 53).unwrap_or(i32::MAX));
    mantissa as f64 * scale
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_whitespace_around_line_breaks_like_the_js_regexes() {
        assert_eq!(
            strip_line_starts("\n        a\n        b\n      "),
            "a\nb\n"
        );
        assert_eq!(strip_line_ends("a \n  \nb"), "a\nb");
        assert_eq!(strip_line_ends("lines\n"), "lines");
        assert_eq!(newline_then_space_to_newline("a\n  \n  b"), "a\nb");
    }

    #[test]
    fn reads_a_string_as_javascript_number_reads_it() {
        assert_eq!(js_number(""), 0.0);
        assert_eq!(js_number(" \t "), 0.0);
        assert_eq!(js_number("5"), 5.0);
        assert_eq!(js_number(" 5 "), 5.0);
        assert_eq!(js_number("+3"), 3.0);
        assert_eq!(js_number("-2"), -2.0);
        assert_eq!(js_number("1e1"), 10.0);
        assert_eq!(js_number("1."), 1.0);
        assert_eq!(js_number(".5"), 0.5);
        assert_eq!(js_number("0x10"), 16.0);
        assert_eq!(js_number("0o17"), 15.0);
        assert_eq!(js_number("0b101"), 5.0);
        assert_eq!(js_number("Infinity"), f64::INFINITY);
        assert_eq!(js_number("-Infinity"), f64::NEG_INFINITY);
        assert!(js_number("-0").is_sign_negative());
    }

    #[test]
    fn reads_the_other_strings_as_nan() {
        for text in [
            "abc", "5px", "2rem", "1_000", "NaN", "infinity", "inf", "+", ".", "1e", "0x", "-0x10",
            "0x1g", "0b2",
        ] {
            assert!(js_number(text).is_nan(), "{text:?}");
        }
    }

    #[test]
    fn rounds_long_hexadecimal_numbers_as_javascript_does() {
        // 2^64 + 1 is not a double: it rounds to 2^64.
        assert_eq!(
            js_number("0x10000000000000001"),
            18_446_744_073_709_551_616.0
        );
        // 2^64 + 2048 is halfway between 2^64 and its successor 2^64 + 4096: ties go to the even
        // mantissa, which is 2^64, and one more bit goes up.
        assert_eq!(
            js_number("0x10000000000000800"),
            18_446_744_073_709_551_616.0
        );
        assert_eq!(
            js_number("0x10000000000000801"),
            18_446_744_073_709_555_712.0
        );
        assert_eq!(js_number(&format!("0x{}", "f".repeat(300))), f64::INFINITY);
    }

    #[test]
    fn collapses_runs() {
        assert_eq!(collapse_blanks("a  \t b c"), "a b c");
        assert_eq!(collapse_line_breaks("a\n\r\nb\nc"), "a\nb\nc");
    }
}
