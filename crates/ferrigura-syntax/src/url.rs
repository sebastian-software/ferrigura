//! `sanitizeUrl` of `@braintree/sanitize-url` 7.1.2 and `formatUrl` of the reference (`utils.ts`).
//!
//! The one port of `sanitizeUrl`: the class DB (`formatUrl`) and the gantt DB (`setLink`) call it,
//! and so do the renderers for the `href` of an image (`drawImage`, `drawEmbeddedImage`) and of a
//! sequence actor's menu link.
//!
//! The reference is `index.js` and `constants.js` of the package. Each regular expression of
//! `constants.js` is a function that finds its match at a position, and each global `replace` is a
//! left-to-right scan with that function. The text is handled as `char`s. The JavaScript `\w` is
//! the ASCII word set, `\s` is the JavaScript whitespace, and `.` excludes the line terminators.

use crate::text::{is_js_space, is_line_term, is_word, js_trim};

/// What `sanitizeUrl` returns for an empty, unsafe or invalid URL.
const BLANK_URL: &str = "about:blank";

/// `formatUrl` of the reference.
///
/// The link is trimmed with JavaScript's `trim`, and an empty result gives `None`. `loose` is
/// `securityLevel === 'loose'`: the trimmed link is returned as it is, and otherwise it goes
/// through [`sanitize_url`].
pub(crate) fn format_url(link: &str, loose: bool) -> Option<String> {
    let url = js_trim(link);
    if url.is_empty() {
        None
    } else if loose {
        Some(url.to_owned())
    } else {
        Some(sanitize_url(url))
    }
}

/// `sanitizeUrl` of `@braintree/sanitize-url` 7.1.2.
///
/// The URL is decoded until no escape, entity, control character or whitespace escape is left. It
/// comes back as it is when it starts with `.` or `/`, and when it has no scheme. A scheme of
/// `mailto:`, or one that contains `://`, comes back with backslashes turned into slashes. An
/// `http:` or `https:` scheme gives the WHATWG serialisation of the URL. An empty URL, a
/// `javascript`, `data` or `vbscript` scheme, and an `http(s)` URL that does not parse give
/// `about:blank`.
#[must_use]
pub fn sanitize_url(url: &str) -> String {
    if url.is_empty() {
        return BLANK_URL.to_owned();
    }
    let mut decoded = decode_uri(js_trim(url));
    loop {
        let entities = decode_html_characters(&decoded);
        let without_newlines = remove_matches(&entities, html_ctrl_entity_at);
        let without_controls = remove_ctrl(&without_newlines);
        let without_escapes = remove_matches(&without_controls, whitespace_escape_at);
        decoded = decode_uri(js_trim(&without_escapes));
        if !needs_another_pass(&decoded) {
            break;
        }
    }

    if decoded.is_empty() {
        return BLANK_URL.to_owned();
    }
    if matches!(decoded.chars().next(), Some('.' | '/')) {
        return decoded;
    }
    // The scheme is searched after the leading whitespace is gone. Without a scheme the decoded
    // text itself is returned, with its leading whitespace.
    let trimmed = decoded.trim_start_matches(is_js_space);
    let Some(raw_scheme) = url_scheme(trimmed) else {
        return decoded;
    };
    let scheme = js_trim(&raw_scheme.to_lowercase()).to_owned();
    if is_invalid_protocol(&scheme) {
        return BLANK_URL.to_owned();
    }
    let back_sanitized = trimmed.replace('\\', "/");
    if scheme == "mailto:" || scheme.contains("://") {
        return back_sanitized;
    }
    if scheme == "http:" || scheme == "https:" {
        return normalize_http(&back_sanitized);
    }
    back_sanitized
}

/// The `http:` and `https:` branch of `sanitizeUrl`: `about:blank` when `URL.canParse` is false,
/// otherwise the serialised `URL`.
///
/// The reference then lowercases the protocol and the hostname of the `URL` object. The WHATWG
/// parser already stores both in lowercase, so the two assignments change nothing.
///
/// The `url` crate (2.5.8) parses two inputs differently from node 24, and this port keeps its
/// result. A `^` in the path is not percent-encoded (node gives `%5E`). A `xn--` label whose
/// punycode does not decode is rejected, where node keeps the label, so the reference's URL
/// becomes `about:blank` here.
fn normalize_http(candidate: &str) -> String {
    match ::url::Url::parse(candidate) {
        Ok(parsed) => parsed.to_string(),
        Err(_) => BLANK_URL.to_owned(),
    }
}

/// `invalidProtocolRegex`, `/^([^\w]*)(javascript|data|vbscript)/im`, tested on the scheme.
///
/// Any number of non-word characters may come before the keyword, and the keyword matches in any
/// case.
fn is_invalid_protocol(scheme: &str) -> bool {
    let rest = scheme.trim_start_matches(|c: char| !is_word(c));
    ["javascript", "data", "vbscript"]
        .into_iter()
        .any(|keyword| {
            rest.get(..keyword.len())
                .is_some_and(|head| head.eq_ignore_ascii_case(keyword))
        })
}

/// The match of `/^.+(:|&colon;)/gim` in `text`, which is the scheme of the URL.
///
/// The first line that has a match gives it. `^` also matches after a line terminator, and `.`
/// does not cross one. The `.+` is greedy, so the match ends at the last `:` or `&colon;` of the
/// line that comes after the line's first character, and `&colon;` matches in any case.
fn url_scheme(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut line_start = 0;
    loop {
        let line_end = chars
            .iter()
            .enumerate()
            .skip(line_start)
            .find(|(_, c)| is_line_term(**c))
            .map_or(chars.len(), |(index, _)| index);
        let colon_end = (line_start + 1..line_end)
            .rev()
            .find_map(|index| colon_end_at(&chars, index));
        if let Some(end) = colon_end {
            return Some(
                chars
                    .iter()
                    .skip(line_start)
                    .take(end - line_start)
                    .collect(),
            );
        }
        if line_end >= chars.len() {
            return None;
        }
        line_start = line_end + 1;
    }
}

/// The end of a `:` or of an `&colon;` (in any case) that starts at `index`.
fn colon_end_at(chars: &[char], index: usize) -> Option<usize> {
    if chars.get(index) == Some(&':') {
        Some(index + 1)
    } else if matches_ignore_case(chars, index, "&colon;") {
        Some(index + "&colon;".len())
    } else {
        None
    }
}

/// Whether the ASCII `pattern` occurs at `index`, ignoring the ASCII case.
fn matches_ignore_case(chars: &[char], index: usize, pattern: &str) -> bool {
    pattern.chars().enumerate().all(|(offset, p)| {
        chars
            .get(index + offset)
            .is_some_and(|c| c.eq_ignore_ascii_case(&p))
    })
}

/// `&(newline|tab);` with the `i` flag, at `start`: the index after the match.
fn html_ctrl_entity_at(chars: &[char], start: usize) -> Option<usize> {
    if chars.get(start) != Some(&'&') {
        return None;
    }
    ["newline;", "tab;"]
        .into_iter()
        .find(|name| matches_ignore_case(chars, start + 1, name))
        .map(|name| start + 1 + name.len())
}

/// `(\\|%5[cC])((%(6[eE]|72|74))|[nrt])` at `start`: the index after the match.
fn whitespace_escape_at(chars: &[char], start: usize) -> Option<usize> {
    let after_escape = match *chars.get(start)? {
        '\\' => start + 1,
        '%' if chars.get(start + 1) == Some(&'5')
            && matches!(chars.get(start + 2).copied(), Some('c' | 'C')) =>
        {
            start + 3
        }
        _ => return None,
    };
    let encoded = chars.get(after_escape) == Some(&'%')
        && matches!(
            (
                chars.get(after_escape + 1).copied(),
                chars.get(after_escape + 2).copied()
            ),
            (Some('6'), Some('e' | 'E')) | (Some('7'), Some('2' | '4'))
        );
    if encoded {
        Some(after_escape + 3)
    } else if matches!(chars.get(after_escape).copied(), Some('n' | 'r' | 't')) {
        Some(after_escape + 1)
    } else {
        None
    }
}

/// `&#(\w+)(^\w|;)?` at `start`: the index after the match and the digits.
///
/// The reference's `^\w` alternative cannot match after the digits, since `^` is the start of the
/// text, so only an optional `;` can follow them.
fn entity_at(chars: &[char], start: usize) -> Option<(usize, String)> {
    if chars.get(start) != Some(&'&') || chars.get(start + 1) != Some(&'#') {
        return None;
    }
    let mut end = start + 2;
    let mut digits = String::new();
    while let Some(&c) = chars.get(end) {
        if !is_word(c) {
            break;
        }
        digits.push(c);
        end += 1;
    }
    if digits.is_empty() {
        return None;
    }
    if chars.get(end) == Some(&';') {
        end += 1;
    }
    Some((end, digits))
}

/// Removes every match of `matcher` from `text`, scanning from the left like a global `replace`
/// with an empty string.
fn remove_matches(text: &str, matcher: fn(&[char], usize) -> Option<usize>) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    while let Some(&c) = chars.get(index) {
        match matcher(&chars, index) {
            Some(end) => index = end.max(index + 1),
            None => {
                out.push(c);
                index += 1;
            }
        }
    }
    out
}

/// Removes the control characters of `ctrlCharactersRegex`.
fn remove_ctrl(text: &str) -> String {
    text.chars().filter(|&c| !is_ctrl(c)).collect()
}

/// Whether `c` matches `ctrlCharactersRegex`: C0 and C1 controls, U+2000 to U+200D, and U+FEFF.
fn is_ctrl(c: char) -> bool {
    matches!(
        c,
        '\u{0}'..='\u{1F}' | '\u{7F}'..='\u{9F}' | '\u{2000}'..='\u{200D}' | '\u{FEFF}'
    )
}

/// `decodeHtmlCharacters` of the reference: the control characters are removed, then each
/// `&#...` entity is replaced by `String.fromCharCode` of its digits.
fn decode_html_characters(text: &str) -> String {
    let chars: Vec<char> = text.chars().filter(|&c| !is_ctrl(c)).collect();
    let mut out = String::with_capacity(chars.len());
    let mut index = 0;
    while let Some(&c) = chars.get(index) {
        match entity_at(&chars, index) {
            Some((end, digits)) => {
                out.push(from_char_code(&digits));
                index = end;
            }
            None => {
                out.push(c);
                index += 1;
            }
        }
    }
    out
}

/// Whether another pass of the reference's loop is due: a control character, an entity, a
/// `&newline;`-style entity or a whitespace escape is still in the text.
fn needs_another_pass(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    chars.iter().any(|&c| is_ctrl(c))
        || (0..chars.len()).any(|index| {
            entity_at(&chars, index).is_some()
                || html_ctrl_entity_at(&chars, index).is_some()
                || whitespace_escape_at(&chars, index).is_some()
        })
}

/// `String.fromCharCode(digits)` for the digits of an entity, as one `char`.
///
/// A code unit in the surrogate range (U+D800 to U+DFFF) is not a `char`, so it becomes U+FFFD.
/// The reference keeps a lone surrogate, and it joins two surrogate entities that form a pair
/// into one code point. Here each surrogate unit is one U+FFFD, so a pair gives two of them.
fn from_char_code(digits: &str) -> char {
    char::from_u32(u32::from(word_to_uint16(digits))).unwrap_or(char::REPLACEMENT_CHARACTER)
}

/// `ToUint16(ToNumber(word))` for a `\w+` string, which is the code unit that
/// `String.fromCharCode` takes from an entity.
///
/// `0x`, `0o` and `0b` (in either case) start a hexadecimal, octal or binary number. Any other
/// word is decimal, and its digits may have an `e` or `E` exponent. Anything else, such as `NaN`
/// or `Infinity`, is 0.
fn word_to_uint16(word: &str) -> u16 {
    if let Some(digits) = word.strip_prefix("0x").or_else(|| word.strip_prefix("0X")) {
        return radix_to_uint16(digits, 4, |c| c.is_ascii_hexdigit());
    }
    if let Some(digits) = word.strip_prefix("0o").or_else(|| word.strip_prefix("0O")) {
        return radix_to_uint16(digits, 3, |c| matches!(c, '0'..='7'));
    }
    if let Some(digits) = word.strip_prefix("0b").or_else(|| word.strip_prefix("0B")) {
        return radix_to_uint16(digits, 1, |c| matches!(c, '0' | '1'));
    }
    decimal_to_uint16(word)
}

/// `ToUint16` of the number that `digits` spells in base `2^bits_per_digit`.
///
/// The number becomes a double first, which keeps 53 significant bits, rounded to nearest with
/// ties to even. From 16 dropped bits on, the double is a multiple of 2^16, so the result is 0.
fn radix_to_uint16(digits: &str, bits_per_digit: u32, is_digit: impl Fn(char) -> bool) -> u16 {
    if digits.is_empty() || !digits.chars().all(&is_digit) {
        return 0;
    }
    let bits: Vec<bool> = digits
        .chars()
        .filter_map(|c| c.to_digit(16))
        .flat_map(|digit| {
            (0..bits_per_digit)
                .rev()
                .map(move |shift| (digit >> shift) & 1 == 1)
        })
        .skip_while(|&bit| !bit)
        .collect();
    if bits.len() <= 53 {
        let value = bits.iter().fold(0u64, |acc, &bit| acc * 2 + u64::from(bit));
        return (value % 65_536) as u16;
    }
    let dropped = bits.len() - 53;
    if dropped >= 16 {
        return 0;
    }
    let mut mantissa = bits
        .iter()
        .take(53)
        .fold(0u64, |acc, &bit| acc * 2 + u64::from(bit));
    let round = bits.get(53).copied().unwrap_or(false);
    let sticky = bits.iter().skip(54).any(|&bit| bit);
    if round && (sticky || mantissa % 2 == 1) {
        mantissa += 1;
    }
    let low = (mantissa % 65_536) << dropped;
    (low % 65_536) as u16
}

/// `ToUint16(Number(word))` for a decimal `\w+`: digits, optionally followed by an `e` or `E`
/// and more digits. A sign cannot appear, since `+` and `-` are not word characters.
fn decimal_to_uint16(word: &str) -> u16 {
    let all_digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    let is_decimal = match word.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => all_digits(mantissa) && all_digits(exponent),
        None => all_digits(word),
    };
    if !is_decimal {
        return 0;
    }
    match word.parse::<f64>() {
        Ok(value) if value.is_finite() => (value % 65_536.0) as u16,
        _ => 0,
    }
}

/// `decodeURI` of the reference: `decodeURIComponent`, or the input when that throws.
fn decode_uri(uri: &str) -> String {
    decode_uri_component(uri).unwrap_or_else(|| uri.to_owned())
}

/// `decodeURIComponent`: every `%XX` escape is decoded. The UTF-8 that the escapes form must be
/// valid, and any other `%` is an error, which gives `None` here (the `URIError` of the reference).
fn decode_uri_component(uri: &str) -> Option<String> {
    let bytes = uri.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        if byte != b'%' {
            out.push(byte);
            index += 1;
            continue;
        }
        let first = escaped_byte(bytes, index)?;
        index += 3;
        let width = match first {
            0x00..=0x7F => 1,
            0xC0..=0xDF => 2,
            0xE0..=0xEF => 3,
            0xF0..=0xF7 => 4,
            _ => return None,
        };
        let mut sequence = vec![first];
        for _ in 1..width {
            if bytes.get(index) != Some(&b'%') {
                return None;
            }
            let continuation = escaped_byte(bytes, index)?;
            if (continuation & 0xC0) != 0x80 {
                return None;
            }
            sequence.push(continuation);
            index += 3;
        }
        let decoded = std::str::from_utf8(&sequence).ok()?;
        out.extend_from_slice(decoded.as_bytes());
    }
    String::from_utf8(out).ok()
}

/// The byte of the `%XX` escape at `index`, with two hexadecimal digits in either case.
fn escaped_byte(bytes: &[u8], index: usize) -> Option<u8> {
    let high = (*bytes.get(index + 1)? as char).to_digit(16)?;
    let low = (*bytes.get(index + 2)? as char).to_digit(16)?;
    u8::try_from(high * 16 + low).ok()
}

#[cfg(test)]
mod tests {
    use super::{decode_uri, format_url, sanitize_url, word_to_uint16};

    // The expected values are the output of the reference in node 24, which runs `index.js` of
    // @braintree/sanitize-url 7.1.2. Each surrogate code unit in an expected value is written as
    // U+FFFD, the way `from_char_code` writes it.

    /// `(input, sanitizeUrl(input))`.
    const SANITIZE_CASES: &[(&str, &str)] = &[
        // plain and common URLs
        (
            "https://example.com/path?query=1#frag",
            "https://example.com/path?query=1#frag",
        ),
        ("http://example.com/a/b", "http://example.com/a/b"),
        ("example.com", "example.com"),
        (
            "HTTPS://WWW.EXAMPLE.COM/Path",
            "https://www.example.com/Path",
        ),
        ("https://www.Google.com", "https://www.google.com/"),
        ("https://example.com/a b", "https://example.com/a%20b"),
        (
            "https://example.com/?q=a&b=c",
            "https://example.com/?q=a&b=c",
        ),
        (
            "https://example.com/\u{e9}t\u{e9}",
            "https://example.com/%C3%A9t%C3%A9",
        ),
        (
            "https://example.com/%C3%A9t%C3%A9",
            "https://example.com/%C3%A9t%C3%A9",
        ),
        (
            "https://example.com/%E4%BD%A0",
            "https://example.com/%E4%BD%A0",
        ),
        ("https://example.com/%zz", "https://example.com/%zz"),
        ("https://example.com/%", "https://example.com/%"),
        ("https://example.com/%E2%82", "https://example.com/%E2%82"),
        (
            "https://example.com/%ED%A0%80",
            "https://example.com/%ED%A0%80",
        ),
        ("https://example.com/%C0%AF", "https://example.com/%C0%AF"),
        (
            "https://example.com/%F4%90%80%80",
            "https://example.com/%F4%90%80%80",
        ),
        (
            "https://example.com/%E2%82%AC",
            "https://example.com/%E2%82%AC",
        ),
        ("https://example.com/%2e%2e/x", "https://example.com/x"),
        ("http://a.b/%2e%2e/x", "http://a.b/x"),
        ("%", "%"),
        ("%25", "%"),
        ("a%3Ab", "a:b"),
        ("https://B\u{dc}CHER.de/", "https://xn--bcher-kva.de/"),
        ("https://xn--bcher-kva.de/", "https://xn--bcher-kva.de/"),
        ("https://\u{4f8b}\u{3048}.jp/", "https://xn--r8jz45g.jp/"),
        ("https://[::1]:8080/", "https://[::1]:8080/"),
        ("https://[2001:DB8::1]/x", "https://[2001:DB8::1]/x"),
        ("https://127.1/", "https://127.0.0.1/"),
        ("https://0x7f.1/", "https://127.0.0.1/"),
        ("https://ex%41mple.com/", "https://example.com/"),
        ("https://EXAMPLE.com./", "https://example.com./"),
        (
            "https://user:pass@example.com/",
            "https://user:pass@example.com/",
        ),
        ("HTTP://EXAMPLE.COM:80/", "HTTP://EXAMPLE.COM:80/"),
        (
            "https://example.com:8080/a:b",
            "https://example.com:8080/a:b",
        ),
        (
            "https://a.com/?next=https://b.com",
            "https://a.com/?next=https://b.com",
        ),
        ("https://example.com/a\\b", "https://example.com/a/b"),
        ("https:\\\\example.com\\x", "https://example.com/x"),
        ("http:/example.com", "http://example.com/"),
        ("https:example.com", "https://example.com/"),
        ("https://exa mple.com", "about:blank"),
        ("https://exa\u{0}mple.com", "https://example.com/"),
        (
            "https://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.com/",
            "https://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.com/",
        ),
        ("ftp://example.com/x", "ftp://example.com/x"),
        ("tel:+1234567", "tel:+1234567"),
        ("file:///etc/passwd", "file:///etc/passwd"),
        ("about:blank", "about:blank"),
        ("foo:bar", "foo:bar"),
        ("java script:alert(1)", "java script:alert(1)"),
        (
            "https://example.com/?q=\u{e4}#\u{fc}",
            "https://example.com/?q=%C3%A4#%C3%BC",
        ),
        (
            "https://example.com/%20%20%20%20%20%20%20%20%20%20",
            "https://example.com/",
        ),
        // relative, empty and scheme-less
        ("./a", "./a"),
        ("/a", "/a"),
        ("../a", "../a"),
        ("/\\evil.com", "/\\evil.com"),
        (".\\a", ".\\a"),
        ("\\\\evil.com", "\\\\evil.com"),
        ("", "about:blank"),
        ("   ", "about:blank"),
        ("\u{9}\u{a}", "about:blank"),
        ("/", "/"),
        (".", "."),
        ("javascript", "javascript"),
        (":", ":"),
        ("a:", "a:"),
        (":a", ":a"),
        ("::", "::"),
        ("a::b", "a::b"),
        (
            "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx:",
            "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx:",
        ),
        (
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa:b",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa:b",
        ),
        // unsafe schemes
        ("javascript:alert(1)", "about:blank"),
        ("JAVASCRIPT:alert(1)", "about:blank"),
        ("JavaScript:alert(1)", "about:blank"),
        ("javascript:", "about:blank"),
        ("data:text/html;base64,PHNjcmlwdD4=", "about:blank"),
        ("DaTa:text/plain,hello", "about:blank"),
        ("vbscript:msgbox(1)", "about:blank"),
        ("javascript&colon;alert(1)", "about:blank"),
        ("JavaScript&COLON;alert(1)", "about:blank"),
        ("vbscript&colon;x", "about:blank"),
        ("data&colon;text", "about:blank"),
        ("javascript&#58;alert(1)", "about:blank"),
        ("javascript&#0000058;alert(1)", "about:blank"),
        ("%6Aavascript:alert(1)", "about:blank"),
        ("%6aavascript:alert(1)", "about:blank"),
        ("%2520javascript:alert(1)", "about:blank"),
        ("%26%23106%3Bavascript:alert(1)", "about:blank"),
        ("%0Ajavascript:alert(1)", "about:blank"),
        ("javascript%3Aalert(1)", "about:blank"),
        ("%09javascript:alert(1)", "about:blank"),
        ("https&colon;//example.com", "https&colon;//example.com"),
        ("\\j\\a\\v\\a\\script:alert(1)", "/j/a/v/a/script:alert(1)"),
        ("j\\avascript:alert(1)", "j/avascript:alert(1)"),
        (
            "&amp;#106;avascript:alert(1)",
            "&amp;#106;avascript:alert(1)",
        ),
        ("%25%25%25%25%25javascript:x", "about:blank"),
        // whitespace and control characters
        (" \u{9}\u{1} javascript:alert(1)", "about:blank"),
        ("\u{0}\u{1f}javascript:alert(1)", "about:blank"),
        ("\u{7f}javascript:alert(1)", "about:blank"),
        ("\u{85}javascript:alert(1)", "about:blank"),
        ("\u{9f}javascript:alert(1)", "about:blank"),
        ("\u{a0}javascript:alert(1)", "about:blank"),
        ("\u{200b}javascript:alert(1)", "about:blank"),
        ("java\u{200c}script:alert(1)", "about:blank"),
        ("java\u{200d}script:alert(1)", "about:blank"),
        ("\u{feff}javascript:alert(1)", "about:blank"),
        ("\u{2028}javascript:alert(1)", "about:blank"),
        ("java\u{a}script:alert(1)", "about:blank"),
        ("java\u{d}script:alert(1)", "about:blank"),
        ("java\u{2028}script:alert(1)", "java\u{2028}script:alert(1)"),
        ("java\u{2029}script:alert(1)", "java\u{2029}script:alert(1)"),
        ("java\u{2000}script:alert(1)", "about:blank"),
        ("java\u{3000}script:alert(1)", "java\u{3000}script:alert(1)"),
        ("  https://example.com  ", "https://example.com/"),
        ("\u{a0}https://example.com\u{3000}", "https://example.com/"),
        ("\u{85}https://example.com", "https://example.com/"),
        ("java\\tscript:alert(1)", "about:blank"),
        ("java\\nscript:alert(1)", "about:blank"),
        ("java\\rscript:alert(1)", "about:blank"),
        ("java%5ctscript:alert(1)", "about:blank"),
        ("java%5Ctscript:alert(1)", "about:blank"),
        ("java%5C%74script:alert(1)", "about:blank"),
        ("java%5C%6Escript:alert(1)", "about:blank"),
        ("java%5C%72script:alert(1)", "about:blank"),
        ("java&tab;script:alert(1)", "about:blank"),
        ("java&NewLine;script:alert(1)", "about:blank"),
        ("java&TAB;script:alert(1)", "about:blank"),
        ("&newline;javascript:alert(1)", "about:blank"),
        ("java%09script:alert(1)", "about:blank"),
        ("java&tab&tab;script:alert(1)", "java&tabscript:alert(1)"),
        ("java\\\\tscript:alert(1)", "java/script:alert(1)"),
        // numeric entities
        ("&#106;avascript:alert(1)", "about:blank"),
        ("&#x6a;avascript:alert(1)", "avascript:alert(1)"),
        ("&#X6A;avascript:alert(1)", "avascript:alert(1)"),
        ("&#0x6a;avascript:alert(1)", "about:blank"),
        ("&#0106;avascript:alert(1)", "about:blank"),
        ("&#0b1101010;avascript:alert(1)", "about:blank"),
        ("&#0o152;avascript:alert(1)", "about:blank"),
        ("&#0B1101010;avascript:alert(1)", "about:blank"),
        ("&#0000000000000106;avascript:alert(1)", "about:blank"),
        ("&#65601;avascript:alert(1)", "Aavascript:alert(1)"),
        ("&#65601;", "A"),
        ("&#1e2;", "d"),
        ("&#1E2;", "d"),
        ("&#1e;", "about:blank"),
        ("&#x;", "about:blank"),
        ("&#;", "&#;"),
        ("&#", "&#"),
        ("&#x110000;", "about:blank"),
        ("&#1114112;", "about:blank"),
        ("&#55296;", "\u{fffd}"),
        ("&#57344;", "\u{e000}"),
        ("&#55357;", "\u{fffd}"),
        ("&#99999999999999999999;", "about:blank"),
        ("&#9007199254740993;", "about:blank"),
        ("&#18446744073709551617;", "about:blank"),
        ("&#Infinity;", "about:blank"),
        ("&#xD800;", "about:blank"),
        ("&#x6a", "about:blank"),
        ("&#106&#97vascript:alert(1)", "j:alert(1)"),
        ("&#1a;", "about:blank"),
        ("&#65;BC", "ABC"),
        ("&#97;&#58;b", "a:b"),
        ("&#58;", ":"),
        ("java&#x09;script:alert(1)", "about:blank"),
        ("java&#9;script:alert(1)", "about:blank"),
        ("java&#x0a;script:alert(1)", "about:blank"),
        ("java&#10;script:alert(1)", "about:blank"),
        ("java&#x0D;script:alert(1)", "about:blank"),
        (
            "&#106;&#97;&#118;&#97;&#115;&#99;&#114;&#105;&#112;&#116;&#58;alert(1)",
            "about:blank",
        ),
        ("&#x6A;&#x61;&#x76;&#x61;script:x", "script:x"),
        ("&amp;", "&amp;"),
        // mailto and deep links
        ("mailto:user@example.com", "mailto:user@example.com"),
        ("mailto:\\\\user@example.com", "mailto://user@example.com"),
        ("mailto:a\\b@example.com", "mailto:a/b@example.com"),
        ("MAILTO:User@Example.com", "MAILTO:User@Example.com"),
        ("  mailto:a@b.c  ", "mailto:a@b.c"),
        ("myapp://open?x=1", "myapp://open?x=1"),
        ("myapp://a:b", "myapp://a:b"),
        ("x-custom://a:b/c", "x-custom://a:b/c"),
        ("custom:thing", "custom:thing"),
        // backslashes
        ("https://example.com\\..\\x", "https://example.com/x"),
        ("https://a.com\\#frag", "https://a.com/#frag"),
        ("http:\\\\evil.com\\x", "http://evil.com/x"),
        // token combinations (seeded)
        ("\u{2028}\u{2028}..x", "..x"),
        ("..\u{a0}%6E", "..\u{a0}n"),
        ("&%72", "&r"),
        ("a\u{2028}6a", "a\u{2028}6a"),
        ("%5C@", "\\@"),
        (
            "www.Google.comwww.Google.comvbscript",
            "www.Google.comwww.Google.comvbscript",
        ),
        ("https%6e://", "httpsn://"),
        ("\u{e9}&#x\u{e9}&NewLine;", "\u{e9}\u{e9}"),
        ("&", "&"),
        ("\u{a}example.com", "example.com"),
        ("%E2%80%8B..", ".."),
        ("%2F?\u{85}javascript&newline;", "/?javascript"),
        ("x", "x"),
        ("n?://", "n?://"),
        ("\u{200b}%6E%5Cexample.com?", "n\\example.com?"),
        ("0x6atab;", "0x6atab;"),
        ("%2Fbwww.Google.com", "/bwww.Google.com"),
        ("&newline;&%0a106", "&106"),
        ("%6E\u{0}&COLON;%6e\u{2028}", "n&COLON;n"),
        ("example.com0\u{200b}", "example.com0"),
        ("\u{2028}example.com%26&tab;", "example.com&"),
        ("%72", "r"),
        ("JavaScript/https%0a0%6e", "JavaScript/https0n"),
        ("1e2%23bdata%5c", "1e2#bdata\\"),
        ("\u{a0}1e2%72", "1e2r"),
        ("&tab;%0dhttps &#", "https &#"),
        ("javascript%74", "javascriptt"),
        (
            "6a1e2%0dwww.Google.com%E2%80%A8%26",
            "6a1e2www.Google.com\u{2028}&",
        ),
        ("0%72\u{e9}?&COLON;", "0r\u{e9}?&COLON;"),
        ("&#data%6Emailto%091e2", "about:blank"),
        ("&COLON;&#x", "&COLON;"),
        ("%&%72%C3%A9b%25", "%&%72%C3%A9b%25"),
        ("%C3%A9%", "%C3%A9%"),
        (";", ";"),
        ("%6E@https%26%E2%80%8B", "n@https&"),
        ("tab;%3Ahttps%6e", "tab;:httpsn"),
        ("106%0a", "106"),
        ("%20b1101010", "b1101010"),
        ("0b1101010n%6e%6ea%26", "0b1101010nnna&"),
        ("..106", "..106"),
        ("..https..%0d%23?", "..https..#?"),
        ("http&colon;", "http&colon;"),
        ("example.com%74", "example.comt"),
        (" %20", "about:blank"),
        ("%6E1e2:\u{9}", "n1e2:"),
        ("..1e2", "..1e2"),
        ("\u{85}&#", "&#"),
        ("/0t\u{a}.https", "/0t.https"),
        (" %0d", "about:blank"),
        ("%0d&", "&"),
        ("&newline;\\%09tab;", "ab;"),
        (
            "\u{0}www.Google.comJavaScript.a",
            "www.Google.comJavaScript.a",
        ),
        (":%E2%80%8B\u{0}1e2\u{2028}", ":1e2"),
        ("&NewLine;106x6a:&COLON;", "106x6a:&COLON;"),
        ("&newline;", "about:blank"),
        ("mailto", "mailto"),
        ("..&newline;\u{a0}&#x%", "..\u{a0}%"),
        ("@\u{9}%E2%80%A8%5Cn&COLON;", "@\u{2028}&COLON;"),
        ("example.com%0Ahttps&", "example.comhttps&"),
        ("\u{2028}://%5cwww.Google.com", "://\\www.Google.com"),
        ("mailto\u{a0}%72", "mailto\u{a0}r"),
        ("0b1101010%6E\u{85}\u{2028}", "0b1101010n"),
        ("example.com%6E%E2%80%8B\u{200b}%26", "example.comn&"),
        ("\u{2028}@", "@"),
        ("&#\u{a}", "&#"),
        ("0o152", "0o152"),
        ("https0x6abx6a", "https0x6abx6a"),
        ("@@data%0a", "@@data"),
        ("x%2FJavaScriptx6a\u{0}", "x/JavaScriptx6a"),
        ("%261&COLON;106mailto%6E", "&1&COLON;106mailton"),
        ("://", "://"),
        ("x6a%5c%5Cdata%6E&#x", "x6a\\\\datan"),
        ("%5cx6a%2", "%5cx6a%2"),
        ("\u{a0}mailto", "mailto"),
        ("..#\u{a0}a1e2x6a", "..#\u{a0}a1e2x6a"),
        ("@ntab;r", "@ntab;r"),
        ("%0a", "about:blank"),
        ("1e2&colon;", "1e2&colon;"),
        ("\u{a0}%&COLON;\\http\u{200b}", "%&COLON;/http"),
        ("\\%C3%A9;http", "\\\u{e9};http"),
        ("r1www.Google.com%25", "r1www.Google.com%"),
        ("%23%720o152", "#r0o152"),
        ("n%0d%E2%80%8Bx%0d", "nx"),
        ("b%0a%20%72", "b r"),
        ("%%09tab;%C3%A9t", "%%09tab;%C3%A9t"),
        ("#%0A", "#"),
        ("%740o152", "t0o152"),
        ("%20%3Ax6a", ":x6a"),
        ("t%2/", "t%2/"),
        ("\u{9}:// ", "://"),
        ("www.Google.com", "www.Google.com"),
        ("\u{9}", "about:blank"),
        ("n%25x6a/", "n%x6a/"),
        ("106\u{d}%2F\u{9}&COLON;", "106/&COLON;"),
        ("106n%%25", "106n%%25"),
        ("www.Google.comr&#%0a@0o152", "www.Google.comr&#@0o152"),
        ("1", "1"),
        ("r", "r"),
        ("..", ".."),
        ("0x6a", "0x6a"),
        ("%6e&tab;%3AJavaScripttab;", "n:JavaScripttab;"),
        ("%C3%A91:", "\u{e9}1:"),
        ("0o152datahttp%6E", "0o152datahttpn"),
        ("a", "a"),
        ("%20%5C&#x\u{e9}x6a", "\\\u{e9}x6a"),
        ("106/\u{d}", "106/"),
        ("@/tab;?", "@/tab;?"),
        ("%25https", "%https"),
        ("%0ahttp%5Chttpshttp?", "http\\httpshttp?"),
        ("n%6e&newline;@ tab;", "nn@ tab;"),
        (";\u{a0}JavaScript%25%20", ";\u{a0}JavaScript%"),
        ("\u{9}/", "/"),
        (
            "%5Cwww.Google.comr0b11010106a%0a",
            "\\www.Google.comr0b11010106a",
        ),
        ("%74", "t"),
        ("?", "?"),
        ("&colon;%0d", "&colon;"),
        ("?0o152%25", "?0o152%"),
        ("tab;://r%6E&tab;\u{9}", "tab;://rn"),
        (";%72 ..www.Google.com", ";r ..www.Google.com"),
        (" 6a\\", "6a\\"),
        ("%25https&#x", "%https"),
        ("&#&newline;%3A", "&#:"),
        ("n&tab;", "n"),
        ("%2x6a&newline;", "%2x6a"),
        ("http1e2 ", "http1e2"),
        ("\\0b1101010?", "\\0b1101010?"),
        ("%25n/.", "%n/."),
        (
            "1e2t0b1101010javascript%23vbscript",
            "1e2t0b1101010javascript#vbscript",
        ),
        ("\u{2028}", "about:blank"),
        ("r%25%0da", "r%a"),
        ("0o1520&colon;1", "0o1520&colon;1"),
        ("%E2%80%A8%2", "%E2%80%A8%2"),
        ("%5Ct106%E2%80%A8data", "106\u{2028}data"),
        ("%5C&#x", "\\"),
        ("&@example.com", "&@example.com"),
        (".a%E2%80%8B", ".a"),
        ("\u{9}https%6e%E2%80%8B%2F", "httpsn/"),
        ("%5c \u{9}\u{2028}", "\\"),
        ("%E2%80%8B0&newline;0o152", "00o152"),
        ("httpsdata/", "httpsdata/"),
        // the image links of the renderers' Chromium probe and the gantt click links
        (
            "https://example.com/a.png?x=1&y=2",
            "https://example.com/a.png?x=1&y=2",
        ),
        ("data:image/png;base64,AAAA", "about:blank"),
        ("pics/a b.png", "pics/a b.png"),
        (
            " https://example.com/space.png ",
            "https://example.com/space.png",
        ),
        ("HTTPS://EXAMPLE.COM/UPPER", "https://example.com/UPPER"),
        ("mailto:someone@example.com", "mailto:someone@example.com"),
        ("java&Tab;script:alert(1)", "about:blank"),
        ("https://example.org/x y", "https://example.org/x%20y"),
        (
            "HTTPS://Example.ORG:443/a?b=1#c",
            "HTTPS://Example.ORG:443/a?b=1#c",
        ),
        ("https://example.org", "https://example.org/"),
        ("http://", "about:blank"),
        ("/relative/path", "/relative/path"),
        ("#fragment", "#fragment"),
        ("  DATA:text/html,x", "about:blank"),
        // JavaScript's `i` flag without `u` does not fold `ſ` to `s`
        ("javaſcript:alert(1)", "javaſcript:alert(1)"),
    ];

    /// `(link, loose, formatUrl(link, config))`, where `None` is `undefined`.
    const FORMAT_CASES: &[(&str, bool, Option<&str>)] = &[
        (
            "  https://example.com  ",
            false,
            Some("https://example.com/"),
        ),
        ("  https://example.com  ", true, Some("https://example.com")),
        ("", false, None),
        ("   ", true, None),
        ("\u{9}\u{a}\u{2028}", false, None),
        (
            "\u{a0} javascript:alert(1) \u{3000}",
            false,
            Some("about:blank"),
        ),
        (
            "\u{a0} javascript:alert(1) \u{3000}",
            true,
            Some("javascript:alert(1)"),
        ),
        ("  mailto:a@b.c ", false, Some("mailto:a@b.c")),
        ("  ./a  ", false, Some("./a")),
        ("\u{85}x", false, Some("x")),
        ("\u{85}x", true, Some("\u{85}x")),
        (
            "https://www.Google.com",
            false,
            Some("https://www.google.com/"),
        ),
        (
            "https://www.Google.com",
            true,
            Some("https://www.Google.com"),
        ),
        (
            "HTTPS://WWW.EXAMPLE.COM/Path",
            false,
            Some("https://www.example.com/Path"),
        ),
        ("&#106;avascript:alert(1)", false, Some("about:blank")),
        (
            "&#106;avascript:alert(1)",
            true,
            Some("&#106;avascript:alert(1)"),
        ),
        ("http://a.b/%zz", false, Some("http://a.b/%zz")),
        ("  a:b  ", true, Some("a:b")),
        ("x", false, Some("x")),
        (
            "java\u{2028}script:alert(1)",
            false,
            Some("java\u{2028}script:alert(1)"),
        ),
        ("https://exa mple.com", false, Some("about:blank")),
        (
            "  https://example.com/a?b=c  ",
            true,
            Some("https://example.com/a?b=c"),
        ),
        (
            "\u{feff}\u{a0} https://x.y \u{2029}",
            false,
            Some("https://x.y/"),
        ),
        (
            "   javascript:alert(1)   ",
            true,
            Some("javascript:alert(1)"),
        ),
    ];

    /// `(uri, decodeURI(uri))`. A URI that `decodeURIComponent` rejects comes back unchanged.
    const DECODE_CASES: &[(&str, &str)] = &[
        ("abc", "abc"),
        ("%41%42", "AB"),
        ("%2F%3A", "/:"),
        ("%00", "\u{0}"),
        ("%", "%"),
        ("%4", "%4"),
        ("%4g", "%4g"),
        ("%zz", "%zz"),
        ("a%", "a%"),
        ("%25", "%"),
        ("%2525", "%25"),
        ("%C3%A9", "\u{e9}"),
        ("%C3", "%C3"),
        ("%C3%28", "%C3%28"),
        ("%E2%82%AC", "\u{20ac}"),
        ("%F0%9F%98%80", "\u{1f600}"),
        ("%ED%A0%80", "%ED%A0%80"),
        ("%F4%90%80%80", "%F4%90%80%80"),
        ("%C0%AF", "%C0%AF"),
        ("%80", "%80"),
        ("%FF", "%FF"),
        ("%E2%82%AC%", "%E2%82%AC%"),
        ("%F0%9F%98", "%F0%9F%98"),
        ("\u{e9}%20x", "\u{e9} x"),
        ("%e9", "%e9"),
        ("%E4%BD%A0", "\u{4f60}"),
        ("%7e%5c%7C", "~\\|"),
        ("a%20b%20", "a b "),
        ("%C3%A9%zz", "%C3%A9%zz"),
        ("%F0%9F%98%80%", "%F0%9F%98%80%"),
    ];

    /// `(digits, String.fromCharCode(digits).charCodeAt(0))`.
    const NUMBER_CASES: &[(&str, u16)] = &[
        ("106", 106),
        ("x6a", 0),
        ("X6A", 0),
        ("0x6a", 106),
        ("0X6A", 106),
        ("0o152", 106),
        ("0O152", 106),
        ("0b1101010", 106),
        ("0B1101010", 106),
        ("0o8", 0),
        ("0b2", 0),
        ("0x", 0),
        ("0o", 0),
        ("0b", 0),
        ("0xg", 0),
        ("0x1ff", 511),
        ("0x1FFFF", 65535),
        ("1e2", 100),
        ("1E2", 100),
        ("1e", 0),
        ("e2", 0),
        ("0e0", 0),
        ("00065", 65),
        ("65601", 65),
        ("65536", 0),
        ("131072", 0),
        ("1e5", 34464),
        ("1e10", 58368),
        ("1e400", 0),
        ("Infinity", 0),
        ("infinity", 0),
        ("NaN", 0),
        ("1_0", 0),
        ("99999999999999999999", 0),
        ("9007199254740993", 0),
        ("9007199254740995", 4),
        ("18446744073709551617", 0),
        ("0x20000000000001", 0),
        ("0x20000000000003", 4),
        ("0x1ffffffffffffff", 0),
        ("0x1fffffffffffff", 65535),
        ("0x10000000000000", 0),
        ("0x7fffffffffffffffffff", 0),
        ("0x1234567890abcdef1", 57344),
        ("0x20000000000000", 0),
        (
            "0b111111111111111111111111111111111111111111111111111111111111",
            0,
        ),
        (
            "0b111111111111111111111111111111111111111111111111111111",
            0,
        ),
        (
            "1111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111",
            0,
        ),
        (
            "10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            0,
        ),
        ("0o7777777777777777777777", 0),
        ("0", 0),
        ("00", 0),
        ("9", 9),
        ("9a", 0),
        ("0xffff", 65535),
        ("0x10000", 0),
        ("0x1ffff8000", 32768),
        (
            "0b11111111111111111111111111111111111111111111111111111",
            65535,
        ),
        ("0xFFFFFFFFFFFFFFFFFFFF", 0),
        ("0x1000000000000000000000000000000", 0),
        ("1e0100", 0),
        ("1e308", 0),
        ("1e309", 0),
    ];

    #[test]
    fn sanitize_url_matches_the_reference() {
        for &(input, expected) in SANITIZE_CASES {
            assert_eq!(sanitize_url(input), expected, "sanitizeUrl({input:?})");
        }
    }

    #[test]
    fn format_url_matches_the_reference() {
        for &(link, loose, expected) in FORMAT_CASES {
            assert_eq!(
                format_url(link, loose).as_deref(),
                expected,
                "formatUrl({link:?}, loose = {loose})"
            );
        }
    }

    #[test]
    fn decode_uri_matches_the_reference() {
        for &(uri, expected) in DECODE_CASES {
            assert_eq!(decode_uri(uri), expected, "decodeURI({uri:?})");
        }
    }

    #[test]
    fn number_of_an_entity_matches_the_reference() {
        for &(digits, expected) in NUMBER_CASES {
            assert_eq!(word_to_uint16(digits), expected, "fromCharCode({digits:?})");
        }
    }

    #[test]
    fn a_surrogate_entity_pair_gives_two_replacement_characters() {
        // The reference joins the pair into U+1F600. A Rust string cannot hold the two halves,
        // so each one is a replacement character here (see `from_char_code`).
        assert_eq!(sanitize_url("&#55357;&#56832;"), "\u{FFFD}\u{FFFD}");
    }
}
