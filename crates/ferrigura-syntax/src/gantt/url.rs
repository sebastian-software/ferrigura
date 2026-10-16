//! `sanitizeUrl` of `@braintree/sanitize-url` 7.1.2, for the `href` of a `click` statement: the
//! steps of `dist/index.js` (decoding, control characters, the scheme check), and for `http:` and
//! `https:` the WHATWG `URL` serialisation that the package uses (`new URL(…).toString()`).
//!
//! The serialiser covers the forms that occur in practice: the authority with its userinfo, port
//! and host (lower case, the default port removed), the path with `/` for an empty path, and the
//! query and fragment with their percent-encode sets. Dot segments (`./`, `../`) and IPv6 hosts are
//! not resolved here: such an address is rejected as `about:blank`, which the differential run
//! reports if an input reaches it.

use crate::text::{is_js_space, js_trim};

use super::date::js_to_number;

const BLANK_URL: &str = "about:blank";

/// `sanitizeUrl(url)`: the address the link gets, or `about:blank` for an address that is unsafe or
/// that does not parse.
pub(crate) fn sanitize_url(url: &str) -> String {
    if url.is_empty() {
        return BLANK_URL.to_owned();
    }
    let mut decoded = decode_uri(js_trim(url));
    loop {
        decoded = decode_html_characters(&decoded);
        decoded = remove_html_ctrl_entities(&decoded);
        decoded = remove_control_characters(&decoded);
        decoded = remove_whitespace_escapes(&decoded);
        decoded = js_trim(&decoded).to_owned();
        decoded = decode_uri(&decoded);
        if !has_decodable(&decoded) {
            break;
        }
    }
    if decoded.is_empty() {
        return BLANK_URL.to_owned();
    }
    if decoded.starts_with(['.', '/']) {
        return decoded;
    }
    let trimmed = decoded.trim_start_matches(is_js_space);
    let Some(scheme_text) = url_scheme(trimmed) else {
        return decoded;
    };
    let scheme = scheme_text.to_lowercase();
    let scheme = js_trim(&scheme);
    if invalid_protocol(scheme) {
        return BLANK_URL.to_owned();
    }
    let back_sanitized = trimmed.replace('\\', "/");
    if scheme == "mailto:" || scheme.contains("://") {
        return back_sanitized;
    }
    if scheme == "http:" || scheme == "https:" {
        return normalize_http(&back_sanitized).unwrap_or_else(|| BLANK_URL.to_owned());
    }
    back_sanitized
}

/// `decodeURIComponent`, and the original text when it throws.
fn decode_uri(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes
                .get(i + 1..i + 3)
                .and_then(|pair| std::str::from_utf8(pair).ok());
            match hex.and_then(|pair| u8::from_str_radix(pair, 16).ok()) {
                Some(byte) => out.push(byte),
                None => return text.to_owned(),
            }
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| text.to_owned())
}

/// Whether a character is removed by `ctrlCharactersRegex`:
/// `[\u0000-\u001F\u007F-\u009F -‍﻿]`.
fn is_control(c: char) -> bool {
    matches!(c, '\u{0}'..='\u{1F}' | '\u{7F}'..='\u{9F}' | '\u{2000}'..='\u{200D}' | '\u{FEFF}')
}

fn remove_control_characters(text: &str) -> String {
    text.chars().filter(|&c| !is_control(c)).collect()
}

/// `replace(/&#(\w+)(^\w|;)?/g, …)`: the character of the number that follows `&#`, read as
/// `String.fromCharCode` reads it (a non-number is 0).
fn decode_html_characters(text: &str) -> String {
    let text = remove_control_characters(text);
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '&' && chars.get(i + 1) == Some(&'#') {
            let word = chars[i + 2..]
                .iter()
                .take_while(|c| c.is_ascii_alphanumeric() || **c == '_')
                .count();
            if word > 0 {
                let digits: String = chars[i + 2..i + 2 + word].iter().collect();
                let code = js_to_number(&digits);
                let unit = if code.is_finite() {
                    (code.trunc().rem_euclid(65_536.0)) as u32
                } else {
                    0
                };
                out.push(char::from_u32(unit).unwrap_or('\u{FFFD}'));
                i += 2 + word;
                if chars.get(i) == Some(&';') {
                    i += 1;
                }
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// `replace(/&(newline|tab);/gi, '')`.
fn remove_html_ctrl_entities(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        let rest = &lower[i..];
        if rest.starts_with("&newline;") {
            i += 9;
        } else if rest.starts_with("&tab;") {
            i += 5;
        } else {
            let width = text[i..].chars().next().map_or(1, char::len_utf8);
            out.push_str(&text[i..i + width]);
            i += width;
        }
    }
    out
}

/// `replace(/(\\|%5[cC])((%(6[eE]|72|74))|[nrt])/g, '')`.
fn remove_whitespace_escapes(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        let escape_len = if bytes[i] == b'\\' {
            1
        } else if bytes[i..].starts_with(b"%5c") || bytes[i..].starts_with(b"%5C") {
            3
        } else {
            0
        };
        let after = i + escape_len;
        let rest = &bytes[after..];
        let consumed = if escape_len > 0
            && (rest.starts_with(b"%6e")
                || rest.starts_with(b"%6E")
                || rest.starts_with(b"%72")
                || rest.starts_with(b"%74"))
        {
            Some(3)
        } else if escape_len > 0 && matches!(rest.first(), Some(b'n' | b'r' | b't')) {
            Some(1)
        } else {
            None
        };
        match consumed {
            Some(length) => i = after + length,
            None => {
                let width = text[i..].chars().next().map_or(1, char::len_utf8);
                out.push_str(&text[i..i + width]);
                i += width;
            }
        }
    }
    out
}

/// Whether one of the four patterns of the decoding loop still matches.
fn has_decodable(text: &str) -> bool {
    let has_control = text.chars().any(is_control);
    let has_entity = text.contains("&#")
        && text.split("&#").skip(1).any(|rest| {
            rest.chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        });
    let lower = text.to_ascii_lowercase();
    let has_ctrl_entity = lower.contains("&newline;") || lower.contains("&tab;");
    let has_escape = remove_whitespace_escapes(text) != text;
    has_control || has_entity || has_ctrl_entity || has_escape
}

/// Whether a character ends a line for the `m` flag: `\n`, `\r`, U+2028 and U+2029.
fn is_line_terminator(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// `text.match(/^.+(:|&colon;)/gim)[0]`: the scheme as written, from the first line that has a
/// colon after at least one character, up to the last such colon of that line.
fn url_scheme(text: &str) -> Option<&str> {
    let mut line_start = 0;
    loop {
        let line_end = text[line_start..]
            .find(is_line_terminator)
            .map_or(text.len(), |offset| line_start + offset);
        let line = &text[line_start..line_end];
        let lower = line.to_ascii_lowercase();
        let colon = lower
            .char_indices()
            .filter(|&(index, _)| index > 0)
            .filter_map(|(index, c)| {
                if c == ':' {
                    Some(index + 1)
                } else if lower[index..].starts_with("&colon;") {
                    Some(index + 7)
                } else {
                    None
                }
            })
            .last();
        if let Some(end) = colon {
            return Some(&text[line_start..line_start + end]);
        }
        if line_end >= text.len() {
            return None;
        }
        let width = text[line_end..].chars().next().map_or(1, char::len_utf8);
        line_start = line_end + width;
    }
}

/// `/^([^\w]*)(javascript|data|vbscript)/im` on the scheme.
fn invalid_protocol(scheme: &str) -> bool {
    let is_word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    scheme.split(is_line_terminator).any(|line| {
        let rest = line
            .trim_start_matches(|c: char| !is_word(c))
            .to_ascii_lowercase();
        ["javascript", "data", "vbscript"]
            .iter()
            .any(|word| rest.starts_with(word))
    })
}

/// The WHATWG serialisation of an `http` or `https` address, or `None` where `new URL` throws.
fn normalize_http(url: &str) -> Option<String> {
    let colon = url.find(':')?;
    let scheme = url[..colon].to_ascii_lowercase();
    let default_port: u32 = if scheme == "https" { 443 } else { 80 };
    let rest = url[colon + 1..].trim_start_matches('/');
    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    let after = &rest[authority_end..];

    let (userinfo, hostport) = match authority.rfind('@') {
        Some(at) => (Some(&authority[..at]), &authority[at + 1..]),
        None => (None, authority),
    };
    if hostport.contains('[') {
        return None;
    }
    let (host, port_text) = match hostport.rfind(':') {
        Some(colon) => (&hostport[..colon], Some(&hostport[colon + 1..])),
        None => (hostport, None),
    };
    if host.is_empty()
        || !host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_')
    {
        return None;
    }
    let mut out = format!("{scheme}://");
    if let Some(userinfo) = userinfo {
        out.push_str(&percent_encode(userinfo, USERINFO_SET));
        out.push('@');
    }
    out.push_str(&host.to_ascii_lowercase());
    if let Some(port_text) = port_text.filter(|text| !text.is_empty()) {
        if !port_text.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let port: u32 = port_text.parse().ok()?;
        if port > 65_535 {
            return None;
        }
        if port != default_port {
            out.push_str(&format!(":{port}"));
        }
    }

    let (before_fragment, fragment) = match after.find('#') {
        Some(hash) => (&after[..hash], Some(&after[hash + 1..])),
        None => (after, None),
    };
    let (path, query) = match before_fragment.find('?') {
        Some(question) => (
            &before_fragment[..question],
            Some(&before_fragment[question + 1..]),
        ),
        None => (before_fragment, None),
    };
    let path = if path.is_empty() { "/" } else { path };
    out.push_str(&percent_encode(path, PATH_SET));
    if let Some(query) = query {
        out.push('?');
        out.push_str(&percent_encode(query, SPECIAL_QUERY_SET));
    }
    if let Some(fragment) = fragment {
        out.push('#');
        out.push_str(&percent_encode(fragment, FRAGMENT_SET));
    }
    Some(out)
}

/// The percent-encode sets of the WHATWG URL standard, as ASCII bytes that are encoded in addition
/// to the C0 controls and the bytes above 0x7E.
const SPECIAL_QUERY_SET: &[u8] = b" \"#<>'";
const PATH_SET: &[u8] = b" \"#<>?`{}";
const FRAGMENT_SET: &[u8] = b" \"<>`";
const USERINFO_SET: &[u8] = b" \"#<>?`{}/:;=@[\\]^|";

fn percent_encode(text: &str, set: &[u8]) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte < 0x20 || byte > 0x7E || set.contains(&byte) {
            out.push_str(&format!("%{byte:02X}"));
        } else {
            out.push(char::from(byte));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsafe_schemes_become_blank() {
        assert_eq!(sanitize_url("javascript:alert(1)"), "about:blank");
        assert_eq!(sanitize_url("&#x6A;avascript:alert(1)"), "avascript:alert(1)");
        assert_eq!(sanitize_url("  DATA:text/html,x"), "about:blank");
        assert_eq!(sanitize_url(""), "about:blank");
    }

    #[test]
    fn relative_and_safe_addresses_are_kept() {
        assert_eq!(sanitize_url("/relative/path"), "/relative/path");
        assert_eq!(sanitize_url("#fragment"), "#fragment");
        assert_eq!(
            sanitize_url("mailto:someone@example.org"),
            "mailto:someone@example.org"
        );
        assert_eq!(
            sanitize_url("https://mermaidjs.github.io/"),
            "https://mermaidjs.github.io/"
        );
    }

    #[test]
    fn http_addresses_are_serialised() {
        assert_eq!(
            sanitize_url("https://example.org/x y"),
            "https://example.org/x%20y"
        );
        assert_eq!(
            sanitize_url("HTTPS://Example.ORG:443/a?b=1#c"),
            "HTTPS://Example.ORG:443/a?b=1#c"
        );
        assert_eq!(sanitize_url("https://example.org"), "https://example.org/");
        assert_eq!(sanitize_url("http://"), "about:blank");
    }
}
