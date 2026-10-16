//! `sanitizeUrl` of `@braintree/sanitize-url` (7.1.2, as Mermaid 12.2 bundles it), which the
//! reference applies to the `href` of an image (`drawImage`, `drawEmbeddedImage` in `svgDrawCommon.ts`).
//!
//! The patterns are the library's own, written for the `regex` crate, which has no look-around: the
//! library's `(^\w|;)?` after an entity's digits can only match a `;`, so the pattern takes the `;`.
//! `\w` is ASCII in JavaScript and is written as `[0-9A-Za-z_]` here. The `http:` and `https:` case
//! is `new URL(url).toString()` in the library, which is the `url` crate's WHATWG parser.

use std::sync::LazyLock;

use regex::Regex;

use super::{js_trim, js_trim_start};

/// What an unsafe or empty URL becomes.
pub const BLANK_URL: &str = "about:blank";

static INVALID_PROTOCOL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?im)^([^0-9A-Za-z_]*)(javascript|data|vbscript)").expect("valid pattern")
});
static HTML_ENTITY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"&#([0-9A-Za-z_]+);?").expect("valid pattern"));
static HTML_CTRL_ENTITY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)&(newline|tab);").expect("valid pattern"));
static CTRL_CHARACTERS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[\x00-\x1F\x7F-\x9F\x{2000}-\x{200D}\x{FEFF}]").expect("valid pattern")
});
static URL_SCHEME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?im)^.+(:|&colon;)").expect("valid pattern"));
static WHITESPACE_ESCAPE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\\|%5[cC])((%(6[eE]|72|74))|[nrt])").expect("valid pattern"));

/// `sanitizeUrl(url)`: the URL the image is written with, or `about:blank` for an empty or unsafe URL.
#[must_use]
pub fn sanitize_url(url: &str) -> String {
    if url.is_empty() {
        return BLANK_URL.to_owned();
    }
    let mut decoded = decode_uri(js_trim(url));
    loop {
        decoded = decode_html_characters(&decoded);
        decoded = HTML_CTRL_ENTITY.replace_all(&decoded, "").into_owned();
        decoded = CTRL_CHARACTERS.replace_all(&decoded, "").into_owned();
        decoded = WHITESPACE_ESCAPE.replace_all(&decoded, "").into_owned();
        decoded = js_trim(&decoded).to_owned();
        decoded = decode_uri(&decoded);
        let more = CTRL_CHARACTERS.is_match(&decoded)
            || HTML_ENTITY.is_match(&decoded)
            || HTML_CTRL_ENTITY.is_match(&decoded)
            || WHITESPACE_ESCAPE.is_match(&decoded);
        if !more {
            break;
        }
    }
    if decoded.is_empty() {
        return BLANK_URL.to_owned();
    }
    if decoded.starts_with(['.', '/']) {
        return decoded;
    }
    let trimmed = js_trim_start(&decoded);
    let Some(scheme_match) = URL_SCHEME.find(trimmed) else {
        return decoded;
    };
    let scheme = js_trim(&scheme_match.as_str().to_lowercase()).to_owned();
    if INVALID_PROTOCOL.is_match(&scheme) {
        return BLANK_URL.to_owned();
    }
    let back_sanitized = trimmed.replace('\\', "/");
    if scheme == "mailto:" || scheme.contains("://") {
        return back_sanitized;
    }
    if scheme == "http:" || scheme == "https:" {
        return match url::Url::parse(&back_sanitized) {
            Ok(parsed) => parsed.to_string(),
            Err(_) => BLANK_URL.to_owned(),
        };
    }
    back_sanitized
}

/// `decodeURIComponent`, which the library calls through `decodeURI`: a malformed escape or a
/// sequence that is not UTF-8 leaves the text as it is.
fn decode_uri(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let digits = bytes.get(index + 1..index + 3);
            let value = digits
                .and_then(|pair| std::str::from_utf8(pair).ok())
                .and_then(|pair| u8::from_str_radix(pair, 16).ok());
            match value {
                Some(byte) => {
                    out.push(byte);
                    index += 3;
                }
                None => return text.to_owned(),
            }
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| text.to_owned())
}

/// `decodeHtmlCharacters`: the control characters are removed, then each `&#...` entity becomes the
/// character of its code, taken as `String.fromCharCode` takes it (the number is converted as
/// JavaScript converts it, and a value that is not a number becomes NUL). A code of a surrogate is
/// not a character of a Rust string and becomes U+FFFD.
fn decode_html_characters(text: &str) -> String {
    let stripped = CTRL_CHARACTERS.replace_all(text, "");
    HTML_ENTITY
        .replace_all(&stripped, |captures: &regex::Captures<'_>| {
            let code = char_code(&captures[1]);
            char::from_u32(u32::from(code))
                .unwrap_or('\u{fffd}')
                .to_string()
        })
        .into_owned()
}

/// `ToUint16(Number(text))` for the digits of an entity.
fn char_code(text: &str) -> u16 {
    let value = if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16)
            .map(|v| v as f64)
            .unwrap_or(f64::NAN)
    } else if let Some(binary) = text.strip_prefix("0b").or_else(|| text.strip_prefix("0B")) {
        u64::from_str_radix(binary, 2)
            .map(|v| v as f64)
            .unwrap_or(f64::NAN)
    } else if let Some(octal) = text.strip_prefix("0o").or_else(|| text.strip_prefix("0O")) {
        u64::from_str_radix(octal, 8)
            .map(|v| v as f64)
            .unwrap_or(f64::NAN)
    } else {
        text.parse::<f64>().unwrap_or(f64::NAN)
    };
    if value.is_finite() {
        (value.trunc().rem_euclid(65_536.0)) as u16
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The inputs of the Chromium probe (`tools/port/text/common-fixture.ts`, `sanitize`), whose
    // results the reference produced in the browser.
    #[test]
    fn matches_the_reference_on_the_probed_urls() {
        assert_eq!(
            sanitize_url("https://example.com/a.png?x=1&y=2"),
            "https://example.com/a.png?x=1&y=2"
        );
        assert_eq!(sanitize_url("javascript:alert(1)"), BLANK_URL);
        assert_eq!(sanitize_url("data:image/png;base64,AAAA"), BLANK_URL);
        assert_eq!(sanitize_url("pics/a b.png"), "pics/a b.png");
        assert_eq!(
            sanitize_url(" https://example.com/space.png "),
            "https://example.com/space.png"
        );
        assert_eq!(
            sanitize_url("HTTPS://EXAMPLE.COM/UPPER"),
            "https://example.com/UPPER"
        );
        assert_eq!(
            sanitize_url("mailto:someone@example.com"),
            "mailto:someone@example.com"
        );
    }

    #[test]
    fn an_empty_url_is_blank() {
        assert_eq!(sanitize_url(""), BLANK_URL);
        assert_eq!(sanitize_url("   "), BLANK_URL);
    }

    #[test]
    fn entities_and_escapes_are_decoded_before_the_check() {
        assert_eq!(sanitize_url("&#106;avascript:alert(1)"), BLANK_URL);
        // `&#x09;` is NaN as a number, so it becomes NUL, and NUL is removed before the check.
        assert_eq!(sanitize_url("java&#x09;script:alert(1)"), BLANK_URL);
        assert_eq!(sanitize_url("java&Tab;script:alert(1)"), BLANK_URL);
        assert_eq!(sanitize_url("%6Aavascript:alert(1)"), BLANK_URL);
    }
}
