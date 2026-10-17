//! The numbers of path data, read as Blink reads them: `GenericParseNumber` (`svg_parser_utilities.cc`
//! 43-175) with `FloatType` = `float`. The digits are accumulated in `float`, the integer part from its
//! last digit and the decimals with `frac *= 0.1f`, so a number with many digits is not the nearest float
//! to its decimal value; the port keeps those operations.

/// Whether `x` is a finite float, as `IsValidRange` (`svg_parser_utilities.cc` 34-37) tests it.
fn is_valid(x: f32) -> bool {
    (-f32::MAX..=f32::MAX).contains(&x)
}

/// `IsHTMLSpace`: space, tab, line feed, form feed and carriage return.
fn is_html_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0C | b'\r')
}

/// `SkipOptionalSVGSpaces` (`svg_parser_utilities.h` 48-55): moves `pos` past the spaces, and returns
/// whether there is more input.
pub(crate) fn skip_spaces(s: &[u8], pos: &mut usize) -> bool {
    while *pos < s.len() && is_html_space(s[*pos]) {
        *pos += 1;
    }
    *pos < s.len()
}

/// `SkipOptionalSVGSpacesOrDelimiter` (`svg_parser_utilities.h` 71-86): spaces, then an optional `,`
/// and spaces after it.
pub(crate) fn skip_spaces_or_comma(s: &[u8], pos: &mut usize) {
    if *pos < s.len() && !is_html_space(s[*pos]) && s[*pos] != b',' {
        return;
    }
    if skip_spaces(s, pos) && s[*pos] == b',' {
        *pos += 1;
        skip_spaces(s, pos);
    }
}

/// `ParseNumber` with `kAllowLeadingAndTrailingWhitespace` (`svg_parser_utilities.cc` 43-175): reads a
/// number at `pos` and moves `pos` past it, its trailing spaces and one comma. `None` where Blink's
/// parser fails.
pub(crate) fn parse_number(s: &[u8], pos: &mut usize) -> Option<f32> {
    let mut p = *pos;
    skip_spaces(s, &mut p);
    // The sign is read as an int and multiplies the number at the end (`number *= sign`).
    let mut sign = 1.0f32;
    if p < s.len() && s[p] == b'+' {
        p += 1;
    } else if p < s.len() && s[p] == b'-' {
        p += 1;
        sign = -1.0;
    }
    if p >= s.len() || (!s[p].is_ascii_digit() && s[p] != b'.') {
        return None;
    }

    // The integer part, built from its last digit back to the first.
    let digits = s[p..].iter().take_while(|c| c.is_ascii_digit()).count();
    let mut integer = 0.0f32;
    if digits != 0 {
        let mut multiplier = 1.0f32;
        for index in (0..digits).rev() {
            integer += multiplier * f32::from(s[p + index] - b'0');
            multiplier *= 10.0;
        }
        if !is_valid(integer) {
            return None;
        }
        p += digits;
    }

    // The decimals: `frac` falls by a factor of `0.1f` for each digit.
    let mut decimal = 0.0f32;
    if p < s.len() && s[p] == b'.' {
        p += 1;
        if p >= s.len() || !s[p].is_ascii_digit() {
            return None;
        }
        let mut frac = 1.0f32;
        let count = s[p..].iter().take_while(|c| c.is_ascii_digit()).count();
        for index in 0..count {
            frac *= 0.1f32;
            decimal += f32::from(s[p + index] - b'0') * frac;
        }
        p += count;
    }

    let mut number = integer + decimal;
    number *= sign;

    // The exponent, when an `e` is followed by something that is not `x` or `m` (`em` and `ex`
    // are units, not exponents).
    if p + 1 < s.len() && (s[p] == b'e' || s[p] == b'E') && s[p + 1] != b'x' && s[p + 1] != b'm' {
        p += 1;
        let mut exponent_negative = false;
        if p < s.len() && s[p] == b'+' {
            p += 1;
        } else if p < s.len() && s[p] == b'-' {
            p += 1;
            exponent_negative = true;
        }
        if p >= s.len() || !s[p].is_ascii_digit() {
            return None;
        }
        let count = s[p..].iter().take_while(|c| c.is_ascii_digit()).count();
        let mut exponent = 0.0f32;
        for index in 0..count {
            exponent *= 10.0;
            exponent += f32::from(s[p + index] - b'0');
        }
        if exponent_negative {
            exponent = -exponent;
        }
        // The largest power of ten a float holds (`max_exponent10`).
        if exponent > 38.0 {
            return None;
        }
        if exponent != 0.0 {
            number *= 10.0f64.powf(f64::from(exponent)) as f32;
        }
        p += count;
    }

    if !is_valid(number) {
        return None;
    }
    *pos = p;
    skip_spaces_or_comma(s, pos);
    Some(number)
}

/// `ParseArcFlag` (`svg_path_string_source.cc` 35-50): one `0` or `1`, then the spaces and a comma.
pub(crate) fn parse_arc_flag(s: &[u8], pos: &mut usize) -> Option<bool> {
    let flag = match s.get(*pos)? {
        b'0' => false,
        b'1' => true,
        _ => return None,
    };
    *pos += 1;
    skip_spaces_or_comma(s, pos);
    Some(flag)
}
