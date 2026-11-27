//! `sanitizeUrl` of `@braintree/sanitize-url` 7.1.2 and `formatUrl` of the reference (`utils.ts`).
//!
//! The one port of `sanitizeUrl`: the flowchart, class, agentflow and gantt DBs call it, and so do
//! the renderers for state and kanban links, image shapes (`imageSquare`, `drawImage`,
//! `drawEmbeddedImage`), node anchors and sequence actor menu links.
//!
//! The reference is `index.js` and `constants.js` of the package. Each regular expression of
//! `constants.js` is a function that finds its match at a position, and each global `replace` is a
//! left-to-right scan with that function. The text is handled as `char`s. The JavaScript `\w` is
//! the ASCII word set, `\s` is the JavaScript whitespace, and `.` excludes the line terminators.

use crate::text::{is_js_space, is_line_term, is_word, js_trim};

/// The `securityLevel` of the configuration, as far as URL and text sanitization look at it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SecurityLevel {
    /// `strict`, the default.
    #[default]
    Strict,
    /// `antiscript`.
    Antiscript,
    /// `sandbox`: link targets are `_top` in class diagrams.
    Sandbox,
    /// `loose`: URLs are not sanitized.
    Loose,
    /// Any other value: the reference treats it as neither strict nor loose.
    Other,
}

impl SecurityLevel {
    /// Reads the `securityLevel` configuration value as Mermaid reads it.
    #[must_use]
    pub fn from_config(value: &str) -> Self {
        match value {
            "strict" => Self::Strict,
            "antiscript" => Self::Antiscript,
            "sandbox" => Self::Sandbox,
            "loose" => Self::Loose,
            _ => Self::Other,
        }
    }

    /// Whether URL sanitization is skipped (`securityLevel === 'loose'`).
    #[must_use]
    pub const fn is_loose(self) -> bool {
        matches!(self, Self::Loose)
    }
}

/// What `sanitizeUrl` returns for an empty, unsafe or invalid URL.
const BLANK_URL: &str = "about:blank";

/// Schemes permitted by DOMPurify's default `IS_ALLOWED_URI` (`purify.cjs.js`).
const ALLOWED_SINK_SCHEMES: &[&str] = &[
    "http", "https", "ftp", "ftps", "mailto", "tel", "callto", "sms", "cid", "xmpp", "matrix",
];

/// Raster image MIME subtypes permitted in `data:image/...` URIs on SVG `<image href>`.
/// Excludes active formats such as `svg+xml`.
const SAFE_DATA_IMAGE_SUBTYPES: &[&str] = &[
    "png",
    "x-png",
    "apng",
    "jpeg",
    "jpg",
    "pjpeg",
    "gif",
    "webp",
    "avif",
    "jxl",
    "tiff",
    "tif",
    "heic",
    "heif",
    "bmp",
    "x-ms-bmp",
    "x-icon",
    "vnd.microsoft.icon",
];

/// Maximum obfuscation-decoding passes permitted in [`sanitize_url`] (parser path) and
/// [`guard_url`] (render-sink prefix check). Legitimate URLs stabilize in at most a couple of
/// passes; inputs still carrying nested escapes past this bound fail closed to `about:blank` to
/// prevent quadratic decoding work.
const MAX_DECODE_PASSES: usize = 8;

/// Whether `url` starts with a same-document fragment marker — either literal `#` or a leading
/// Mermaid entity placeholder that decodes to `#` (such as `#35;` / `ﬂ°°35¶ß` or `#num;` /
/// `ﬂ°num¶ß`).
fn starts_with_fragment_marker(url: &str) -> bool {
    url.starts_with('#')
        || decode_mermaid_placeholder_at(url, 0).is_some_and(|(_, decoded)| decoded == "#")
}

/// `formatUrl` of the reference.
///
/// The link is trimmed with JavaScript's `trim`, and an empty result gives `None`. `loose` is
/// `securityLevel === 'loose'`: the trimmed link is returned as it is, and otherwise it is
/// validated through [`sanitize_url`], except same-document fragments (starting with `#` or an
/// entity spelling of `#` like `#35;`), which enforce the bounded decoding limit without running
/// scheme detection on the fragment body and preserve their percent-encoded spelling for DOM
/// lookup. Safe URLs without control characters, HTML entities, or literal whitespace escapes
/// preserve percent-encoded delimiters (such as `%26`, `%2F`, or `%5C`) instead of emitting their
/// decoded form.
#[must_use]
pub fn format_url(link: &str, loose: bool) -> Option<String> {
    let url = js_trim(link);
    if url.is_empty() {
        None
    } else if loose {
        Some(url.to_owned())
    } else if starts_with_fragment_marker(url) {
        let (prepared, url_tokens) = tokenize_url_entity_placeholders(url);
        if decode_url_obfuscations_bounded(prepared.as_ref(), MAX_DECODE_PASSES).is_none() {
            return Some(BLANK_URL.to_owned());
        }
        let finalized = finalize_raw(prepared.into_owned(), url_tokens.as_ref());
        let cleaned: String = finalized
            .chars()
            .filter(|&c| !matches!(c, '\u{0000}'..='\u{001F}' | '\u{007F}'..='\u{009F}'))
            .collect();
        Some(cleaned)
    } else {
        if let Some(relative) = format_relative_with_scheme_delimiter(url) {
            return Some(relative);
        }
        if url.contains('%')
            && !has_non_percent_obfuscation(url)
            && let Some(preserved_http) = preserve_valid_http_encoded_url(url)
        {
            return Some(preserved_http);
        }
        let sanitized = sanitize_url(url);
        if sanitized == BLANK_URL {
            return Some(sanitized);
        }
        if url.contains('%')
            && !has_non_percent_obfuscation(url)
            && let Some(preserved) = preserve_encoded_url(url, &sanitized)
        {
            return Some(preserved);
        }
        Some(sanitized)
    }
}

/// Validates and normalizes an original `http:` / `https:` URL carrying percent-escapes (such as
/// `https://%23alice@example.org/report`) BEFORE `sanitize_url` percent-decodes delimiters into
/// invalid syntax, while still enforcing the bounded obfuscation-decoding limit and blocked-scheme
/// check.
fn preserve_valid_http_encoded_url(url: &str) -> Option<String> {
    let (prepared, url_tokens) = tokenize_url_entity_placeholders(url);
    let effective = resolve_decoded_http_scheme_tokens(prepared.as_ref(), url_tokens.as_ref());
    if !is_http_scheme_prefix(effective.as_ref()) {
        return None;
    }
    if decode_url_obfuscations_bounded(prepared.as_ref(), MAX_DECODE_PASSES).is_none() {
        return Some(BLANK_URL.to_owned());
    }
    if has_invalid_decoded_scheme(effective.as_ref(), url_tokens.as_ref()) {
        return Some(BLANK_URL.to_owned());
    }
    let back_sanitized = effective.replace('\\', "/");
    let resolved = resolve_http_host_tokens(&back_sanitized, url_tokens.as_ref());
    if ::url::Url::parse(resolved.as_ref()).is_err() {
        return None;
    }
    let scheme = url_scheme(&back_sanitized)
        .map(|s| js_trim(&s.to_lowercase()).to_owned())
        .unwrap_or_default();
    if scheme.contains("://") {
        if let Some(ref tokens) = url_tokens {
            return Some(restore_http_tokens_by_component(resolved.as_ref(), tokens));
        }
        return Some(resolved.into_owned());
    }
    Some(normalize_http_with_tokens(
        resolved.as_ref(),
        url_tokens.as_ref(),
    ))
}

/// Whether `text` carries non-percent obfuscation (literal control characters, `&#...` numeric
/// references, `&tab;`/`&newline;` entities, or literal `\n`/`\r`/`\t` backslash escapes).
/// Percent-encoded backslashes such as `%5Cn` in an already-validated path/query are not treated
/// as obfuscation here because `sanitize_url` already checked scheme and authority safety.
fn has_non_percent_obfuscation(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    chars.iter().any(|&c| is_ctrl(c))
        || (0..chars.len()).any(|index| {
            entity_at(&chars, index).is_some()
                || html_ctrl_entity_at(&chars, index).is_some()
                || (chars[index] == '\\'
                    && matches!(chars.get(index + 1).copied(), Some('n' | 'r' | 't')))
        })
}

/// Handles relative URLs where a path, query, or non-entity fragment delimiter (`/`, `?`, or `#`)
/// precedes any colon (such as `data/chart.svg?time=12:30`, `data#part:two`, or
/// `docs%2Freport%3A2026.pdf`), which `@braintree/sanitize-url`'s greedy scheme regex would
/// otherwise misclassify as a blocked `data:` / `javascript:` / `vbscript:` scheme or decode into
/// a different path.
fn format_relative_with_scheme_delimiter(url: &str) -> Option<String> {
    if matches!(url.chars().next(), Some('.' | '/')) {
        return None;
    }
    let raw_delim = first_scheme_delimiter_is_relative(url);
    if raw_delim.is_none() && url_scheme(url).is_some() {
        return None;
    }
    let (prepared, url_tokens) = tokenize_url_entity_placeholders(url);
    let Some(decoded) = decode_url_obfuscations_bounded(prepared.as_ref(), MAX_DECODE_PASSES)
    else {
        return Some(BLANK_URL.to_owned());
    };
    if decoded.is_empty()
        || first_scheme_delimiter_is_relative(&decoded).is_none()
        || guard_url(url, false) == BLANK_URL
        || guard_url(&decoded, false) == BLANK_URL
    {
        return None;
    }
    if let Some(ref tokens) = url_tokens {
        let substituted = substitute_token_values(&decoded, &tokens.prefix, &tokens.decoded);
        let cleaned: String = substituted.chars().filter(|&c| !is_ctrl(c)).collect();
        let head = cleaned.trim_start_matches(is_js_space);
        if first_scheme_delimiter_is_relative(head).is_none() || guard_url(head, false) == BLANK_URL
        {
            return None;
        }
    }
    if !has_non_percent_obfuscation(url) {
        return Some(finalize_raw(prepared.into_owned(), url_tokens.as_ref()));
    }
    Some(finalize_raw(decoded, url_tokens.as_ref()))
}

/// Byte offset of the earliest relative URL delimiter (`/`, `?`, or a `#` fragment marker that
/// cannot form a numeric character reference with its surroundings) when it precedes any `:` or
/// `&colon;` scheme separator.
fn first_scheme_delimiter_is_relative(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut has_escape_or_ctrl = false;
    for (idx, ch) in text.char_indices() {
        match ch {
            '/' | '?' => return Some(idx),
            ':' => return None,
            '&' if bytes
                .get(idx..idx + "&colon;".len())
                .is_some_and(|slice| slice.eq_ignore_ascii_case(b"&colon;")) =>
            {
                return None;
            }
            '#' => {
                if !starts_with_sink_entity_body(&text[idx + 1..]) || !has_escape_or_ctrl {
                    return Some(idx);
                }
            }
            _ => {
                if matches!(ch, '&' | '%' | '\\') || is_ctrl(ch) {
                    has_escape_or_ctrl = true;
                }
            }
        }
    }
    None
}

/// Returns the leading literal RFC-3986 scheme if it is in [`ALLOWED_SINK_SCHEMES`] and is not
/// `http` or `https` (which go through WHATWG authority validation).
fn literal_allowed_non_http_scheme(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    if bytes.first().is_none_or(|b| !b.is_ascii_alphabetic()) {
        return None;
    }
    let mut end = 1;
    while end < bytes.len()
        && (bytes[end].is_ascii_alphanumeric() || matches!(bytes[end], b'+' | b'-' | b'.'))
    {
        end += 1;
    }
    if bytes.get(end) != Some(&b':') {
        return None;
    }
    let scheme = &text[..end];
    if scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https") {
        return None;
    }
    ALLOWED_SINK_SCHEMES
        .iter()
        .any(|allowed| scheme.eq_ignore_ascii_case(allowed))
        .then_some(scheme)
}

/// Preserves percent-encoded bytes (such as `%26`, `%2F`, `%23`, `%25`) in an already-validated
/// URL that carries no literal control characters, HTML entities, or whitespace escapes.
fn preserve_encoded_url(url: &str, sanitized: &str) -> Option<String> {
    let (prepared, url_tokens) = tokenize_url_entity_placeholders(url);
    if matches!(url.chars().next(), Some('.' | '/')) {
        return Some(finalize_raw(prepared.into_owned(), url_tokens.as_ref()));
    }
    let effective = resolve_decoded_http_scheme_tokens(prepared.as_ref(), url_tokens.as_ref());
    if is_http_scheme_prefix(effective.as_ref()) && is_http_scheme_prefix(sanitized) {
        let back_sanitized = effective.replace('\\', "/");
        let resolved = resolve_http_host_tokens(&back_sanitized, url_tokens.as_ref());
        if ::url::Url::parse(resolved.as_ref()).is_err() {
            return None;
        }
        let scheme = url_scheme(&back_sanitized)
            .map(|s| js_trim(&s.to_lowercase()).to_owned())
            .unwrap_or_default();
        if scheme.contains("://") {
            if let Some(ref tokens) = url_tokens {
                return Some(restore_http_tokens_by_component(resolved.as_ref(), tokens));
            }
            return Some(resolved.into_owned());
        }
        return Some(normalize_http_with_tokens(
            resolved.as_ref(),
            url_tokens.as_ref(),
        ));
    }
    if let Some(raw_scheme) = literal_allowed_non_http_scheme(url)
        && literal_allowed_non_http_scheme(sanitized)
            .is_some_and(|s| s.eq_ignore_ascii_case(raw_scheme))
    {
        return Some(finalize_raw(
            prepared.replace('\\', "/"),
            url_tokens.as_ref(),
        ));
    }
    if url_scheme(url).is_none() && url_scheme(sanitized).is_none() {
        return Some(finalize_raw(prepared.into_owned(), url_tokens.as_ref()));
    }
    if first_scheme_delimiter_is_relative(url).is_some()
        && first_scheme_delimiter_is_relative(sanitized).is_some()
    {
        return Some(finalize_raw(
            prepared.replace('\\', "/"),
            url_tokens.as_ref(),
        ));
    }
    None
}

/// Validates `url` at an SVG link sink without altering accepted URLs.
///
/// When `loose` is `true` (`securityLevel === 'loose'`), `url` is returned unchanged. Otherwise
/// both `url` and its bounded obfuscation-decoded scheme prefix must either be a relative URL (no
/// scheme before a path, query, or fragment delimiter) or use one of DOMPurify's permitted schemes
/// (`http`, `https`, `ftp`, `ftps`, `mailto`, `tel`, `callto`, `sms`, `cid`, `xmpp`, `matrix`).
/// Rejected, empty, or overly nested obfuscated URLs become `"about:blank"`. Accepted URLs are
/// returned verbatim so already-sanitized parser links are not decoded a second time and
/// percent-encoded delimiters (such as `%23` or `%26`) are preserved.
#[must_use]
pub fn guard_url(url: &str, loose: bool) -> &str {
    if loose {
        return url;
    }
    if !is_allowed_sink_candidate(url) {
        return BLANK_URL;
    }
    let head = match find_scheme_delimiter_end(url) {
        Some(end) => &url[..end],
        None => url,
    };
    match decode_sink_prefix_bounded(head, MAX_DECODE_PASSES) {
        Some(decoded)
            if decoded.chars().all(is_sink_ignored) || is_allowed_sink_candidate(&decoded) =>
        {
            url
        }
        _ => BLANK_URL,
    }
}

/// Validates an image URL at an SVG `<image href>` sink without altering accepted URLs.
///
/// Permits everything [`guard_url`] permits, plus embedded raster `data:image/...` URIs (`png`,
/// `x-png`, `apng`, `jpeg`, `jpg`, `pjpeg`, `gif`, `webp`, `avif`, `jxl`, `tiff`, `tif`, `heic`,
/// `heif`, `bmp`, `x-ms-bmp`, `x-icon`, `vnd.microsoft.icon`). Active `data:` payloads such as
/// `data:image/svg+xml` and `data:text/html` are rejected to `"about:blank"`.
#[must_use]
pub fn guard_image_url(url: &str, loose: bool) -> &str {
    if loose || is_safe_raster_data_image_url(url) {
        return url;
    }
    guard_url(url, false)
}

/// Whether `url` starts (after optional leading whitespace) with `data:` followed by a raster
/// `image/<subtype>` media type, normalizing embedded ASCII tab, LF, and CR characters as WHATWG
/// URL parsing does before reading the scheme and MIME type (matching data-URL parsing and
/// DOMPurify's trimmed `data:` check for `DEFAULT_DATA_URI_TAGS`, restricted to non-scriptable
/// raster image subtypes).
fn is_safe_raster_data_image_url(url: &str) -> bool {
    let trimmed = url.trim_start_matches(is_js_space);
    if !trimmed
        .as_bytes()
        .first()
        .is_some_and(|byte| matches!(byte, b'd' | b'D'))
    {
        return false;
    }
    let Some(delim) = trimmed.find([';', ',']) else {
        return false;
    };
    let header: String = trimmed[..delim]
        .chars()
        .filter(|&c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();
    let Some(after_scheme) = header
        .get(..5)
        .filter(|prefix| prefix.eq_ignore_ascii_case("data:"))
        .map(|_| &header[5..])
    else {
        return false;
    };
    let media = after_scheme.trim_start_matches(' ');
    let Some(subtype) = media
        .get(..6)
        .filter(|prefix| prefix.eq_ignore_ascii_case("image/"))
        .map(|_| media[6..].trim_end_matches(' '))
    else {
        return false;
    };
    SAFE_DATA_IMAGE_SUBTYPES
        .iter()
        .any(|allowed| subtype.eq_ignore_ascii_case(allowed))
}

/// Runs one obfuscation-decoding step of `sanitizeUrl`: HTML character references, control
/// entities, control characters, whitespace escapes, and `decodeURI`.
fn decode_obfuscation_step(text: &str) -> String {
    let entities = decode_html_characters(text);
    let without_newlines = remove_matches(&entities, html_ctrl_entity_at);
    let without_controls = remove_ctrl(&without_newlines);
    let without_escapes = remove_matches(&without_controls, whitespace_escape_at);
    decode_uri(js_trim(&without_escapes))
}

/// Runs one obfuscation-decoding step of `sanitizeUrl` with lossy percent decoding, used to check
/// nesting depth independently of unrelated malformed `%` escapes elsewhere in the URL.
fn decode_obfuscation_step_lossy(text: &str) -> String {
    let entities = decode_html_characters(text);
    let without_newlines = remove_matches(&entities, html_ctrl_entity_at);
    let without_controls = remove_ctrl(&without_newlines);
    let without_escapes = remove_matches(&without_controls, whitespace_escape_at);
    decode_uri_lossy(js_trim(&without_escapes))
}

/// Runs one obfuscation-decoding step for a render-sink prefix without trimming trailing
/// whitespace, so a literal space between `&` and `#` (such as `& #123`) is preserved.
fn decode_sink_step_untrimmed(text: &str) -> String {
    let entities = decode_html_characters_sink(text);
    let without_newlines = remove_matches(&entities, html_ctrl_entity_at);
    let without_controls = remove_ctrl(&without_newlines);
    let without_escapes = remove_matches(&without_controls, whitespace_escape_at);
    decode_uri_lossy(&without_escapes)
}

/// Runs one obfuscation-decoding step for a render-sink scheme prefix: HTML5 numeric character
/// references (including `&#x...;` hex references), control entities, control characters,
/// whitespace escapes, and lossy percent decoding so a malformed `%XX` later in the segment
/// cannot mask an encoded scheme.
fn decode_sink_step(text: &str) -> String {
    let entities = decode_html_characters_sink(text);
    let without_newlines = remove_matches(&entities, html_ctrl_entity_at);
    let without_controls = remove_ctrl(&without_newlines);
    let without_escapes = remove_matches(&without_controls, whitespace_escape_at);
    decode_uri_lossy(js_trim(&without_escapes))
}

/// Whether the slice immediately following `#` can begin a numeric character reference body
/// (decimal digits, `x`/`X` followed by a hex digit, or a `%`/`&`/`\` escape that may decode to
/// one after stripping control characters).
fn starts_with_sink_entity_body(after_hash: &str) -> bool {
    let mut chars = after_hash.chars().filter(|&c| !is_ctrl(c));
    match chars.next() {
        Some('0'..='9' | '%' | '&' | '\\') => true,
        Some('x' | 'X') => matches!(
            chars.next(),
            Some('0'..='9' | 'a'..='f' | 'A'..='F' | '%' | '&' | '\\')
        ),
        _ => false,
    }
}

/// Decodes numeric character references for render-sink validation using a radix-aware scanner so
/// non-numeric `#section` fragments stay untouched and semicolonless references (such as
/// `&#106avascript:`) stop after their valid digits instead of consuming following letters.
fn decode_html_characters_sink(text: &str) -> String {
    let chars: Vec<char> = text.chars().filter(|&c| !is_ctrl(c)).collect();
    let mut out = String::with_capacity(chars.len());
    let mut index = 0;
    while let Some(&c) = chars.get(index) {
        match sink_entity_at(&chars, index) {
            Some((end, ch)) => {
                out.push(ch);
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

/// Radix-aware numeric character reference scanner for render-sink validation.
/// Matches HTML5 decimal (`&#106;`), HTML5 hex (`&#x3A;`, `&#X6A;`), and JS-radix (`&#0x6a;`,
/// `&#0o152;`, `&#0b1101010;`) references with an optional trailing `;`, stopping at the first
/// non-digit for that radix.
fn sink_entity_at(chars: &[char], start: usize) -> Option<(usize, char)> {
    if chars.get(start) != Some(&'&') || chars.get(start + 1) != Some(&'#') {
        return None;
    }
    let body = start + 2;
    let (digits_start, radix, is_digit): (usize, u32, fn(char) -> bool) =
        match (chars.get(body).copied(), chars.get(body + 1).copied()) {
            (Some('x' | 'X'), _) => (body + 1, 16, |c: char| c.is_ascii_hexdigit()),
            (Some('0'), Some('x' | 'X')) => (body + 2, 16, |c: char| c.is_ascii_hexdigit()),
            (Some('0'), Some('o' | 'O')) => (body + 2, 8, |c: char| matches!(c, '0'..='7')),
            (Some('0'), Some('b' | 'B')) => (body + 2, 2, |c: char| matches!(c, '0' | '1')),
            (Some(c), _) if c.is_ascii_digit() => (body, 10, |c: char| c.is_ascii_digit()),
            _ => return None,
        };

    let mut end = digits_start;
    while let Some(&c) = chars.get(end) {
        if !is_digit(c) {
            break;
        }
        end += 1;
    }
    if end == digits_start {
        return None;
    }
    let digits: String = chars[digits_start..end].iter().collect();
    if chars.get(end) == Some(&';') {
        end += 1;
    }
    let decoded = u32::from_str_radix(&digits, radix)
        .ok()
        .and_then(char::from_u32)
        .unwrap_or(char::REPLACEMENT_CHARACTER);
    Some((end, decoded))
}

/// Decodes each valid UTF-8 `%XX` sequence independently, keeping malformed `%` sequences as-is.
fn decode_uri_lossy(uri: &str) -> String {
    let bytes = uri.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        if byte != b'%' {
            out.push(byte);
            index += 1;
            continue;
        }
        if let Some((decoded_len, width_bytes)) = decode_utf8_percent_at(bytes, index, &mut out) {
            index += decoded_len;
            let _ = width_bytes;
        } else {
            out.push(byte);
            index += 1;
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| uri.to_owned())
}

/// Decodes one valid UTF-8 percent-encoded character starting at `index`, appending its UTF-8
/// bytes to `out` and returning `(consumed_input_bytes, decoded_utf8_bytes)`.
fn decode_utf8_percent_at(bytes: &[u8], index: usize, out: &mut Vec<u8>) -> Option<(usize, usize)> {
    let first = escaped_byte(bytes, index)?;
    let width = match first {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => return None,
    };
    let mut seq = [0u8; 4];
    seq[0] = first;
    let mut cursor = index + 3;
    for slot in seq.iter_mut().take(width).skip(1) {
        if bytes.get(cursor) != Some(&b'%') {
            return None;
        }
        let cont = escaped_byte(bytes, cursor)?;
        if (cont & 0xC0) != 0x80 {
            return None;
        }
        *slot = cont;
        cursor += 3;
    }
    let decoded = std::str::from_utf8(&seq[..width]).ok()?;
    out.extend_from_slice(decoded.as_bytes());
    Some((cursor - index, width))
}

/// Whether another render-sink pass is needed: a control character, a radix-aware numeric entity,
/// a `&newline;`/`&tab;` entity, or a whitespace escape is still present.
fn needs_another_sink_pass(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    chars.iter().any(|&c| is_ctrl(c))
        || (0..chars.len()).any(|index| {
            sink_entity_at(&chars, index).is_some()
                || html_ctrl_entity_at(&chars, index).is_some()
                || whitespace_escape_at(&chars, index).is_some()
        })
}

/// Byte index immediately after the earliest literal `/`, `?`, `:`, or ASCII-case-insensitive
/// `&colon;` scheme delimiter in `text`, if any.
fn find_scheme_delimiter_end(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    for (idx, &byte) in bytes.iter().enumerate() {
        match byte {
            b'/' | b'?' | b':' => return Some(idx + 1),
            b'&' if bytes
                .get(idx..idx + "&colon;".len())
                .is_some_and(|slice| slice.eq_ignore_ascii_case(b"&colon;")) =>
            {
                return Some(idx + "&colon;".len());
            }
            _ => {}
        }
    }
    None
}

/// Whether `text` contains a scheme delimiter (`/`, `?`, `:`, `&colon;`, or a `#` fragment
/// marker that cannot form a numeric character reference with its surroundings).
fn has_scheme_delimiter(text: &str) -> bool {
    if find_scheme_delimiter_end(text).is_some() {
        return true;
    }
    let mut has_escape_or_ctrl = false;
    for (idx, ch) in text.char_indices() {
        if ch == '#' {
            if !starts_with_sink_entity_body(&text[idx + 1..]) || !has_escape_or_ctrl {
                return true;
            }
        } else if matches!(ch, '&' | '%' | '\\') || is_ctrl(ch) {
            has_escape_or_ctrl = true;
        }
    }
    false
}

/// Whether `before` (the slice immediately preceding `#`) can decode to end with `&` within
/// `max_passes` modifying steps. Returns `None` (fail-closed) if `before` itself exceeds
/// `max_passes`. Stops early with `Some(false)` if decoding reveals a `/`, `?`, `:`, `&colon;`,
/// or non-entity `#` delimiter, since nothing after such a delimiter can participate in a URI
/// scheme.
fn prefix_can_end_with_amp(before: &str, max_passes: usize) -> Option<bool> {
    if before.ends_with('&') {
        return Some(true);
    }
    if !before.contains(['&', '%', '\\']) && !before.chars().any(is_ctrl) {
        return Some(false);
    }
    let mut decoded = decode_uri_lossy(before);
    let mut passes_used = usize::from(decoded != before);
    if passes_used > max_passes {
        return None;
    }
    if has_scheme_delimiter(&decoded) {
        return Some(false);
    }
    while passes_used < max_passes {
        let next = decode_sink_step_untrimmed(&decoded);
        if has_scheme_delimiter(&next) {
            return Some(false);
        }
        if next == decoded && !needs_another_sink_pass(&next) {
            return Some(decoded.ends_with('&'));
        }
        if next == decoded {
            return None;
        }
        decoded = next;
        passes_used += 1;
    }
    (decode_sink_step_untrimmed(&decoded) == decoded && !needs_another_sink_pass(&decoded))
        .then(|| decoded.ends_with('&'))
}

/// Truncates `text` at the first literal `/`, `?`, `:`, or `&colon;` delimiter, or at a literal
/// `#` fragment delimiter that cannot form a numeric character reference with its surroundings,
/// since nothing after such a delimiter can participate in a URI scheme.
///
/// Tracks whether any escape or control character has appeared in the forward scan so candidate
/// `#` markers after a plain prefix truncate in O(1) without rescanning the prefix. Returns `None`
/// (fail-closed) if the prefix before a candidate numeric-entity `#` itself exceeds `max_passes`.
fn truncate_at_scheme_delimiter_bounded(text: &str, max_passes: usize) -> Option<&str> {
    let head = match find_scheme_delimiter_end(text) {
        Some(end) => &text[..end],
        None => text,
    };
    let mut has_escape_or_ctrl = false;
    let mut non_trivial_checks = 0usize;
    for (idx, ch) in head.char_indices() {
        if ch == '#' {
            if !starts_with_sink_entity_body(&head[idx + 1..]) || !has_escape_or_ctrl {
                return Some(&head[..=idx]);
            }
            let before = &head[..idx];
            if before.ends_with('&') {
                continue;
            }
            if non_trivial_checks >= MAX_DECODE_PASSES {
                continue;
            }
            non_trivial_checks += 1;
            if !prefix_can_end_with_amp(before, max_passes)? {
                return Some(&head[..=idx]);
            }
        } else if matches!(ch, '&' | '%' | '\\') || is_ctrl(ch) {
            has_escape_or_ctrl = true;
        }
    }
    Some(head)
}

/// Runs [`decode_sink_step`] on a sink scheme prefix until it reaches a fixpoint (catching
/// nested percent escapes such as `notes%25253Adoc`, malformed trailing percent escapes such as
/// `notes%3Afoo%FF`, and nested HTML entities) or exceeds `max_passes`, returning `None`
/// (fail-closed) if it does not stabilize within `max_passes` modifying passes. Re-truncates at
/// any `/`, `?`, `:`, or non-entity `#` delimiter before the initial decode and after each step
/// so suffixes past a delimiter cannot exhaust the budget.
fn decode_sink_prefix_bounded(url: &str, max_passes: usize) -> Option<String> {
    let trimmed = truncate_at_scheme_delimiter_bounded(js_trim(url), max_passes)?;
    let initial = decode_uri_lossy(trimmed);
    let mut passes_used = usize::from(initial != trimmed);
    let mut decoded =
        truncate_at_scheme_delimiter_bounded(&initial, max_passes.saturating_sub(passes_used))?
            .to_owned();
    while passes_used < max_passes {
        let step = decode_sink_step(&decoded);
        let next_passes = passes_used + usize::from(step != decoded);
        let next =
            truncate_at_scheme_delimiter_bounded(&step, max_passes.saturating_sub(next_passes))?
                .to_owned();
        if next == decoded && !needs_another_sink_pass(&next) {
            return Some(decoded);
        }
        if next == decoded {
            return None;
        }
        decoded = next;
        passes_used += 1;
    }
    let final_step = decode_sink_step(&decoded);
    let check = truncate_at_scheme_delimiter_bounded(&final_step, 0)?;
    (check == decoded && !needs_another_sink_pass(&decoded)).then_some(decoded)
}

/// Whether `trimmed` stabilizes under lossy percent and entity decoding within `max_passes`
/// modifying passes (counting the initial `decode_uri_lossy` pass when it modifies the input).
/// Using lossy decoding here ensures an unrelated malformed `%` escape elsewhere in the URL (such
/// as `?value=50%`) cannot mask over-budget nested percent escapes in another segment.
fn within_lossy_decode_budget(trimmed: &str, max_passes: usize) -> bool {
    let mut probe = decode_uri_lossy(trimmed);
    let mut passes_used = usize::from(probe != trimmed);
    while passes_used < max_passes {
        let next = decode_obfuscation_step_lossy(&probe);
        if next == probe && !needs_another_pass(&probe) {
            return true;
        }
        if next == probe {
            return false;
        }
        probe = next;
        passes_used += 1;
    }
    decode_obfuscation_step_lossy(&probe) == probe && !needs_another_pass(&probe)
}

/// Runs the [`sanitize_url`] fixpoint loop up to `max_passes` times (counting the initial
/// `decodeURI` pass when it modifies the input), returning `None` (fail-closed) when nested
/// obfuscations—including percent-only or mixed percent/entity nesting, even alongside unrelated
/// malformed `%` escapes—exceed `max_passes`. For ordinary URLs within budget, returns the exact
/// `@braintree/sanitize-url` decoded spelling.
fn decode_url_obfuscations_bounded(url: &str, max_passes: usize) -> Option<String> {
    let trimmed = js_trim(url);
    if !trimmed.contains(['%', '&', '\\']) && !trimmed.chars().any(is_ctrl) {
        return Some(trimmed.to_owned());
    }
    let mut decoded = decode_uri(trimmed);
    let mut passes_used = usize::from(decoded != trimmed);
    while passes_used < max_passes {
        let next = decode_obfuscation_step(&decoded);
        let changed = next != decoded;
        if changed {
            passes_used += 1;
        }
        decoded = next;
        if !needs_another_pass(&decoded) {
            if !decoded.contains('%') {
                return Some(decoded);
            }
            return within_lossy_decode_budget(trimmed, max_passes).then_some(decoded);
        }
        if !changed {
            return None;
        }
    }
    None
}

/// DOMPurify's `ATTR_WHITESPACE` plus `sanitizeUrl`'s control characters (`is_ctrl`).
fn is_sink_ignored(c: char) -> bool {
    is_ctrl(c)
        || matches!(
            c,
            ' ' | '\u{00A0}' | '\u{1680}' | '\u{180E}' | '\u{2000}'
                ..='\u{2029}' | '\u{205F}' | '\u{3000}'
        )
}

/// Whether `candidate` is non-empty after stripping ignored whitespace/controls and has either no
/// RFC-3986 scheme or a scheme in [`ALLOWED_SINK_SCHEMES`].
///
/// Colons after path, query, or fragment delimiters (for example `data/chart.svg?time=12:30`) do
/// not form a scheme. A `&colon;` entity immediately after a scheme-shaped prefix is rejected.
fn is_allowed_sink_candidate(candidate: &str) -> bool {
    let stripped: String = candidate.chars().filter(|&c| !is_sink_ignored(c)).collect();
    if stripped.is_empty() {
        return false;
    }
    let bytes = stripped.as_bytes();
    if !bytes[0].is_ascii_alphabetic() {
        return true;
    }
    let mut end = 1;
    while end < bytes.len()
        && (bytes[end].is_ascii_alphanumeric() || matches!(bytes[end], b'+' | b'-' | b'.'))
    {
        end += 1;
    }
    let rest = &stripped[end..];
    let has_colon = rest.starts_with(':')
        || rest
            .get(.."&colon;".len())
            .is_some_and(|head| head.eq_ignore_ascii_case("&colon;"));
    if has_colon {
        let scheme = &stripped[..end];
        ALLOWED_SINK_SCHEMES
            .iter()
            .any(|allowed| scheme.eq_ignore_ascii_case(allowed))
    } else {
        true
    }
}

/// `sanitizeUrl` of `@braintree/sanitize-url` 7.1.2.
///
/// The URL is decoded until no escape, entity, control character or whitespace escape is left,
/// bounded to prevent quadratic decoding work on adversarial nested entities (which fail closed to
/// `about:blank`). It comes back as it is when it starts with `.` or `/`, and when it has no
/// scheme. A scheme of `mailto:`, or one that contains `://`, comes back with backslashes turned
/// into slashes. An `http:` or `https:` scheme gives the WHATWG serialization of the URL. An
/// empty URL, a `javascript`, `data` or `vbscript` scheme, and an `http(s)` URL that does not
/// parse give `about:blank`.
#[must_use]
pub fn sanitize_url(url: &str) -> String {
    if url.is_empty() {
        return BLANK_URL.to_owned();
    }
    // Tokenize all literal Mermaid entity placeholders BEFORE `decode_url_obfuscations_bounded`,
    // then classify URL components using the ALREADY-DECODED separators (`%2F`, `%40`, `%2568`,
    // `&#58;`, etc.) so:
    // 1. Source-typed `%EF%AC%82%C2%B0...` escapes are never confused with Mermaid placeholders.
    // 2. Percent-encoded path/authority separators (`https://example.org%2Fdocs#63;b`) place each
    //    placeholder in its true decoded component (`Path` -> `%3F`) rather than the host.
    // 3. Entity-spelled HTTP schemes (`https#58;`) and host/port entities resolve before WHATWG
    //    authority validation, while userinfo/path/query/fragment entities are restored using
    //    each component's exact WHATWG percent-encode set.
    let (prepared, url_tokens) = tokenize_url_entity_placeholders(url);
    let Some(decoded) = decode_url_obfuscations_bounded(prepared.as_ref(), MAX_DECODE_PASSES)
    else {
        return BLANK_URL.to_owned();
    };

    if decoded.is_empty() {
        return BLANK_URL.to_owned();
    }
    if matches!(decoded.chars().next(), Some('.' | '/')) {
        return finalize_raw(decoded, url_tokens.as_ref());
    }
    // The scheme is searched after the leading whitespace is gone. Without a scheme the decoded
    // text itself is returned, with its leading whitespace.
    let trimmed = decoded.trim_start_matches(is_js_space);
    if has_invalid_decoded_scheme(trimmed, url_tokens.as_ref()) {
        return BLANK_URL.to_owned();
    }
    let effective_trimmed = resolve_decoded_http_scheme_tokens(trimmed, url_tokens.as_ref());
    let Some(raw_scheme) = url_scheme(effective_trimmed.as_ref()) else {
        return finalize_raw(decoded, url_tokens.as_ref());
    };
    let scheme = js_trim(&raw_scheme.to_lowercase()).to_owned();
    if is_invalid_protocol(&scheme) {
        return BLANK_URL.to_owned();
    }
    let back_sanitized = effective_trimmed.replace('\\', "/");
    if scheme == "mailto:" || scheme.contains("://") {
        if is_http_scheme_prefix(&back_sanitized) {
            let resolved = resolve_http_host_tokens(&back_sanitized, url_tokens.as_ref());
            if ::url::Url::parse(resolved.as_ref()).is_err() {
                return BLANK_URL.to_owned();
            }
            let safe = reencode_decoded_placeholders(resolved.as_ref());
            if let Some(ref tokens) = url_tokens {
                return restore_http_tokens_by_component(safe.as_ref(), tokens);
            }
            return safe.into_owned();
        }
        return finalize_raw(back_sanitized, url_tokens.as_ref());
    }
    if scheme == "http:" || scheme == "https:" {
        let resolved = resolve_http_host_tokens(&back_sanitized, url_tokens.as_ref());
        return normalize_http_with_tokens(resolved.as_ref(), url_tokens.as_ref());
    }
    finalize_raw(back_sanitized, url_tokens.as_ref())
}

/// Tokenized Mermaid entity placeholders extracted from a URL before obfuscation decoding.
struct UrlEntityTokens {
    prefix: String,
    raw: Vec<String>,
    decoded: Vec<String>,
}

/// WHATWG URL percent-encode component for substituting decoded Mermaid entity placeholders back
/// into a normalized `http:` / `https:` URL.
#[derive(Clone, Copy)]
enum UrlEncodeComponent {
    Userinfo,
    Path,
    SpecialQuery,
    Fragment,
}

fn percent_encode_component(out: &mut String, decoded: &str, component: UrlEncodeComponent) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for byte in decoded.bytes() {
        let encode = match component {
            UrlEncodeComponent::Userinfo => {
                byte <= 0x20
                    || byte >= 0x7F
                    || matches!(
                        byte,
                        b'"' | b'#'
                            | b'<'
                            | b'>'
                            | b'?'
                            | b'`'
                            | b'{'
                            | b'}'
                            | b'/'
                            | b':'
                            | b';'
                            | b'='
                            | b'@'
                            | b'['
                            | b'\\'
                            | b']'
                            | b'^'
                            | b'|'
                    )
            }
            UrlEncodeComponent::Path => {
                byte <= 0x20
                    || byte >= 0x7F
                    || matches!(
                        byte,
                        b'"' | b'#' | b'<' | b'>' | b'?' | b'^' | b'`' | b'{' | b'}'
                    )
            }
            UrlEncodeComponent::SpecialQuery => {
                byte <= 0x20 || byte >= 0x7F || matches!(byte, b'"' | b'#' | b'\'' | b'<' | b'>')
            }
            UrlEncodeComponent::Fragment => {
                byte <= 0x20 || byte >= 0x7F || matches!(byte, b'"' | b'<' | b'>' | b'`')
            }
        };
        if encode {
            out.push('%');
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 0x0F) as usize] as char);
        } else {
            out.push(byte as char);
        }
    }
}

fn is_http_scheme_prefix(s: &str) -> bool {
    s.get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http:"))
        || s.get(..6)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https:"))
}

/// Re-percent-encodes literal `ﬂ°` and `¶ß` produced by percent/entity decoding of a filename so
/// render-time Mermaid placeholder resolution cannot confuse decoded filenames with source entities.
fn reencode_decoded_placeholders(s: &str) -> std::borrow::Cow<'_, str> {
    if !s.contains("ﬂ°") && !s.contains("¶ß") {
        return std::borrow::Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len() + 16);
    let mut cursor = 0;
    let mut next_open = s.find("ﬂ°");
    let mut next_close = s.find("¶ß");
    loop {
        if let Some(pos) = next_open
            && pos < cursor
        {
            next_open = s[cursor..].find("ﬂ°").map(|rel| cursor + rel);
        }
        if let Some(pos) = next_close
            && pos < cursor
        {
            next_close = s[cursor..].find("¶ß").map(|rel| cursor + rel);
        }
        match (next_open, next_close) {
            (Some(o), Some(c)) if o <= c => {
                out.push_str(&s[cursor..o]);
                out.push_str("%EF%AC%82%C2%B0");
                cursor = o + "ﬂ°".len();
            }
            (Some(o), None) => {
                out.push_str(&s[cursor..o]);
                out.push_str("%EF%AC%82%C2%B0");
                cursor = o + "ﬂ°".len();
            }
            (_, Some(c)) => {
                out.push_str(&s[cursor..c]);
                out.push_str("%C2%B6%C3%9F");
                cursor = c + "¶ß".len();
            }
            (None, None) => {
                out.push_str(&s[cursor..]);
                return std::borrow::Cow::Owned(out);
            }
        }
    }
}

fn finalize_raw(s: String, tokens: Option<&UrlEntityTokens>) -> String {
    let safe = reencode_decoded_placeholders(&s);
    if let Some(tokens) = tokens {
        return substitute_token_values(safe.as_ref(), &tokens.prefix, &tokens.raw);
    }
    match safe {
        std::borrow::Cow::Borrowed(_) => s,
        std::borrow::Cow::Owned(owned) => owned,
    }
}

fn match_token_at<'a>(
    segment: &str,
    pos: usize,
    prefix: &str,
    values: &'a [String],
) -> Option<(usize, &'a str)> {
    let rest = segment.get(pos..)?.strip_prefix(prefix)?;
    let digits_len = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits_len == 0 || !rest[digits_len..].starts_with("__") {
        return None;
    }
    let index = rest[..digits_len].parse::<usize>().ok()?;
    let value = values.get(index)?;
    Some((pos + prefix.len() + digits_len + 2, value.as_str()))
}

fn substitute_component_tokens(
    out: &mut String,
    segment: &str,
    tokens: &UrlEntityTokens,
    component: UrlEncodeComponent,
) {
    let prefix = tokens.prefix.as_str();
    let mut cursor = 0;
    while cursor < segment.len() {
        let Some(rel) = segment[cursor..].find(prefix) else {
            out.push_str(&segment[cursor..]);
            return;
        };
        let pos = cursor + rel;
        out.push_str(&segment[cursor..pos]);
        if let Some((end, decoded)) = match_token_at(segment, pos, prefix, &tokens.decoded) {
            percent_encode_component(out, decoded, component);
            cursor = end;
            continue;
        }
        out.push_str(prefix);
        cursor = pos + prefix.len();
    }
}

fn substitute_token_values(segment: &str, prefix: &str, values: &[String]) -> String {
    let mut out = String::with_capacity(segment.len());
    let mut cursor = 0;
    while cursor < segment.len() {
        let Some(rel) = segment[cursor..].find(prefix) else {
            out.push_str(&segment[cursor..]);
            return out;
        };
        let pos = cursor + rel;
        out.push_str(&segment[cursor..pos]);
        if let Some((end, value)) = match_token_at(segment, pos, prefix, values) {
            out.push_str(value);
            cursor = end;
            continue;
        }
        out.push_str(prefix);
        cursor = pos + prefix.len();
    }
    out
}

/// Collects every existing `__fg<8-hex-digits>_` token ID in `s` in a single O(n) pass so the
/// chosen 13-byte prefix is guaranteed collision-free and bounded-size.
fn collect_used_token_ids(s: &str, used: &mut std::collections::HashSet<u32>) {
    let bytes = s.as_bytes();
    if bytes.len() < 13 {
        return;
    }
    let mut i = 0;
    while i + 13 <= bytes.len() {
        if bytes[i] == b'_'
            && bytes[i + 1] == b'_'
            && matches!(bytes[i + 2], b'f' | b'F')
            && matches!(bytes[i + 3], b'g' | b'G')
            && bytes[i + 12] == b'_'
            && bytes[i + 4..i + 12].iter().all(u8::is_ascii_hexdigit)
        {
            if let Ok(hex) = std::str::from_utf8(&bytes[i + 4..i + 12])
                && let Ok(id) = u32::from_str_radix(hex, 16)
            {
                used.insert(id);
            }
            // Resume at the trailing `_` so overlapping prefixes like
            // `__fg00000001__fg00000000_0__` cannot hide the second ID.
            i += 12;
        } else {
            i += 1;
        }
    }
}

/// Picks a fixed 13-byte ASCII prefix (`__fgXXXXXXXX_`) that does not occur in either `raw` or
/// `decoded_probe`. Runs in strictly O(n) time and produces O(1)-size tokens.
fn pick_token_prefix(raw: &str, decoded_probe: &str) -> String {
    let mut used = std::collections::HashSet::new();
    collect_used_token_ids(raw, &mut used);
    collect_used_token_ids(decoded_probe, &mut used);
    let mut id = 0u32;
    while used.contains(&id) {
        id = id.wrapping_add(1);
    }
    format!("__fg{id:08x}_")
}

/// Decodes one Mermaid entity placeholder starting at `pos` (`ﬂ°°<digits>¶ß`, `ﬂ°<name>¶ß`,
/// standalone `ﬂ°°`, standalone `ﬂ°`, or standalone `¶ß`).
fn decode_mermaid_placeholder_at(segment: &str, pos: usize) -> Option<(usize, String)> {
    let rest = &segment[pos..];
    if let Some(after_open) = rest.strip_prefix("ﬂ°") {
        let (numeric, body_start_rel) = if after_open.starts_with('°') {
            (true, '°'.len_utf8())
        } else {
            (false, 0)
        };
        let body_slice = &after_open[body_start_rel..];
        let name_len = body_slice
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(body_slice.len());
        if name_len > 0 && body_slice[name_len..].starts_with("¶ß") {
            let name = &body_slice[..name_len];
            let reference = if numeric {
                format!("&#{name};")
            } else {
                format!("&{name};")
            };
            let decoded = crate::sanitize::decode_attribute_references(&reference);
            let end = pos + "ﬂ°".len() + body_start_rel + name_len + "¶ß".len();
            return Some((end, decoded));
        }
        let (decoded, step) = if numeric {
            ("&#".to_owned(), "ﬂ°°".len())
        } else {
            ("&".to_owned(), "ﬂ°".len())
        };
        return Some((pos + step, decoded));
    }
    if rest.starts_with("¶ß") {
        return Some((pos + "¶ß".len(), ";".to_owned()));
    }
    None
}

/// Decodes an HTML numeric character reference that `encode_entities` split into `&ﬂ°°<digits>¶ß`
/// or `&ﬂ°x<hex>¶ß` (starting at the `ﬂ°` position immediately after `&`), returning
/// `(end_byte_pos, raw_placeholder_without_amp, unicode_decoded)`.
fn decode_encoded_html_numeric_at(segment: &str, pos: usize) -> Option<(usize, String, String)> {
    let after_open = segment[pos..].strip_prefix("ﬂ°")?;
    if let Some(after_deg) = after_open.strip_prefix('°') {
        let digits_len = after_deg.bytes().take_while(u8::is_ascii_digit).count();
        if digits_len > 0 && after_deg[digits_len..].starts_with("¶ß") {
            let digits = &after_deg[..digits_len];
            let reference = format!("&#{digits};");
            let decoded = crate::sanitize::decode_attribute_references(&reference);
            let end = pos + "ﬂ°°".len() + digits_len + "¶ß".len();
            return Some((end, segment[pos..end].to_owned(), decoded));
        }
    } else if matches!(after_open.as_bytes().first(), Some(b'x' | b'X')) {
        let hex_len = after_open[1..]
            .bytes()
            .take_while(u8::is_ascii_hexdigit)
            .count();
        if hex_len > 0 && after_open[1 + hex_len..].starts_with("¶ß") {
            let x_hex = &after_open[..1 + hex_len];
            let reference = format!("&#{x_hex};");
            let decoded = crate::sanitize::decode_attribute_references(&reference);
            let cp = decoded.chars().next().map_or(0xFFFD, u32::from);
            let end = pos + "ﬂ°".len() + 1 + hex_len + "¶ß".len();
            return Some((end, format!("ﬂ°°{cp}¶ß"), decoded));
        }
    }
    None
}

/// Whether substituting all tokenized entity placeholders in `trimmed` with their Unicode decoded
/// values reveals a blocked scheme (`javascript:`, `data:`, or `vbscript:`) before any relative
/// URL delimiter (`/`, `?`, or non-entity `#`).
fn has_invalid_decoded_scheme(trimmed: &str, tokens: Option<&UrlEntityTokens>) -> bool {
    let Some(tokens) = tokens else {
        return false;
    };
    let substituted = substitute_token_values(trimmed, &tokens.prefix, &tokens.decoded);
    let cleaned: String = substituted.chars().filter(|&c| !is_ctrl(c)).collect();
    let head = cleaned.trim_start_matches(is_js_space);
    if matches!(head.chars().next(), Some('.' | '/'))
        || first_scheme_delimiter_is_relative(head).is_some()
    {
        return false;
    }
    url_scheme(head).is_some_and(|raw| is_invalid_protocol(js_trim(&raw.to_lowercase())))
}

/// Tokenizes every literal Mermaid entity placeholder (`ﬂ°` / `¶ß`) in `candidate` in a single
/// O(n) forward pass into bounded-size ASCII tokens before obfuscation decoding runs. Complete
/// HTML numeric character references that `encode_entities` split into `&ﬂ°°<digits>¶ß` or
/// `&ﬂ°x<hex>¶ß` are consumed together with their leading `&` and decoded as Unicode so
/// percent-encoded components elsewhere in the URL stay intact.
fn tokenize_url_entity_placeholders(
    candidate: &str,
) -> (std::borrow::Cow<'_, str>, Option<UrlEntityTokens>) {
    if !candidate.contains("ﬂ°") && !candidate.contains("¶ß") {
        return (std::borrow::Cow::Borrowed(candidate), None);
    }
    let decoded_probe =
        decode_url_obfuscations_bounded(candidate, MAX_DECODE_PASSES).unwrap_or_default();
    let prefix = pick_token_prefix(candidate, &decoded_probe);
    let mut raw = Vec::new();
    let mut decoded = Vec::new();
    let mut out = String::with_capacity(candidate.len());
    let mut cursor = 0;
    let mut next_open = candidate.find("ﬂ°");
    let mut next_close = candidate.find("¶ß");
    while cursor < candidate.len() {
        if let Some(pos) = next_open
            && pos < cursor
        {
            next_open = candidate[cursor..].find("ﬂ°").map(|rel| cursor + rel);
        }
        if let Some(pos) = next_close
            && pos < cursor
        {
            next_close = candidate[cursor..].find("¶ß").map(|rel| cursor + rel);
        }
        let pos = match (next_open, next_close) {
            (Some(o), Some(c)) if o <= c => o,
            (Some(o), None) => o,
            (_, Some(c)) => c,
            (None, None) => {
                out.push_str(&candidate[cursor..]);
                break;
            }
        };
        if pos > cursor
            && candidate.as_bytes()[pos - 1] == b'&'
            && let Some((end, raw_val, decoded_val)) =
                decode_encoded_html_numeric_at(candidate, pos)
        {
            out.push_str(&candidate[cursor..pos - 1]);
            let idx = raw.len();
            out.push_str(&prefix);
            out.push_str(&idx.to_string());
            out.push_str("__");
            raw.push(raw_val);
            decoded.push(decoded_val);
            cursor = end;
            continue;
        }
        out.push_str(&candidate[cursor..pos]);
        let Some((end, decoded_val)) = decode_mermaid_placeholder_at(candidate, pos) else {
            out.push_str(&candidate[pos..]);
            break;
        };
        let idx = raw.len();
        out.push_str(&prefix);
        out.push_str(&idx.to_string());
        out.push_str("__");
        raw.push(candidate[pos..end].to_owned());
        decoded.push(decoded_val);
        cursor = end;
    }
    let tokens = (!raw.is_empty()).then_some(UrlEntityTokens {
        prefix,
        raw,
        decoded,
    });
    (std::borrow::Cow::Owned(out), tokens)
}

/// After bounded decoding has expanded percent/entity escapes, resolves any Mermaid entity tokens
/// that form an `http:` or `https:` scheme prefix (e.g. `https#58;`) and its leading slashes.
fn resolve_decoded_http_scheme_tokens<'a>(
    trimmed: &'a str,
    tokens: Option<&UrlEntityTokens>,
) -> std::borrow::Cow<'a, str> {
    let Some(tokens) = tokens else {
        return std::borrow::Cow::Borrowed(trimmed);
    };
    let mut pos = 0;
    let mut acc = String::with_capacity(6);
    let mut used_token_in_scheme = false;
    while pos < trimmed.len() {
        if let Some((end, text)) = match_token_at(trimmed, pos, &tokens.prefix, &tokens.decoded) {
            used_token_in_scheme = true;
            for ch in text.chars() {
                if !matches!(ch, '\u{0000}'..='\u{001F}' | '\u{007F}'..='\u{009F}') {
                    acc.push(ch);
                }
            }
            pos = end;
        } else {
            let Some(ch) = trimmed[pos..].chars().next() else {
                break;
            };
            pos += ch.len_utf8();
            if matches!(ch, '\u{0000}'..='\u{001F}' | '\u{007F}'..='\u{009F}') {
                continue;
            }
            acc.push(ch);
        }
        if acc.len() > 6 {
            return std::borrow::Cow::Borrowed(trimmed);
        }
        let lower = acc.to_ascii_lowercase();
        if lower == "http:" || lower == "https:" {
            let mut slash_pos = pos;
            let mut slash_count = 0usize;
            let mut used_token_in_slashes = false;
            while slash_pos < trimmed.len() {
                let rest = &trimmed[slash_pos..];
                if rest.starts_with('/') || rest.starts_with('\\') {
                    slash_pos += 1;
                    slash_count += 1;
                } else if let Some((end, text)) =
                    match_token_at(trimmed, slash_pos, &tokens.prefix, &tokens.decoded)
                    && !text.is_empty()
                    && text.bytes().all(|b| b == b'/' || b == b'\\')
                {
                    used_token_in_slashes = true;
                    slash_count += text.len();
                    slash_pos = end;
                } else {
                    break;
                }
            }
            if !used_token_in_scheme && !used_token_in_slashes {
                return std::borrow::Cow::Borrowed(trimmed);
            }
            let mut out = String::with_capacity(trimmed.len());
            if used_token_in_scheme {
                out.push_str(&lower);
            } else {
                out.push_str(&trimmed[..pos]);
            }
            for _ in 0..slash_count {
                out.push('/');
            }
            out.push_str(&trimmed[slash_pos..]);
            return std::borrow::Cow::Owned(out);
        }
        if !"http:".starts_with(&lower) && !"https:".starts_with(&lower) {
            return std::borrow::Cow::Borrowed(trimmed);
        }
    }
    std::borrow::Cow::Borrowed(trimmed)
}

/// Resolves any tokenized Mermaid entity placeholders inside the decoded `host[:port]` authority
/// slice of an `http:` / `https:` URL before WHATWG authority validation and IDNA parsing.
/// Stops substituting as soon as a token expands to an authority-ending delimiter (`/`, `\`, `?`,
/// or `#`), turning `\` into `/` and leaving later tokens intact so
/// `restore_http_tokens_by_component` reclassifies them into their new path, query, or fragment
/// component.
fn resolve_http_host_tokens<'a>(
    url: &'a str,
    tokens: Option<&UrlEntityTokens>,
) -> std::borrow::Cow<'a, str> {
    let Some(tokens) = tokens else {
        return std::borrow::Cow::Borrowed(url);
    };
    let scheme_len = if url
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http:"))
    {
        5
    } else if url
        .get(..6)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https:"))
    {
        6
    } else {
        return std::borrow::Cow::Borrowed(url);
    };
    let slashes = url[scheme_len..].bytes().take_while(|&b| b == b'/').count();
    let authority_start = scheme_len + slashes;
    let authority_end = url[authority_start..]
        .find(['/', '?', '#'])
        .map_or(url.len(), |offset| authority_start + offset);
    let authority = &url[authority_start..authority_end];
    let host_start = authority
        .rfind('@')
        .map_or(authority_start, |at| authority_start + at + 1);
    let host_port = &url[host_start..authority_end];
    let prefix = tokens.prefix.as_str();
    if !host_port.contains(prefix) {
        return std::borrow::Cow::Borrowed(url);
    }
    let mut out = String::with_capacity(url.len());
    out.push_str(&url[..host_start]);
    let mut cursor = 0;
    while cursor < host_port.len() {
        let Some(rel) = host_port[cursor..].find(prefix) else {
            out.push_str(&host_port[cursor..]);
            break;
        };
        let pos = cursor + rel;
        out.push_str(&host_port[cursor..pos]);
        if let Some((end, value)) = match_token_at(host_port, pos, prefix, &tokens.decoded) {
            if value.contains(['/', '\\', '?', '#']) {
                for ch in value.chars() {
                    out.push(if ch == '\\' { '/' } else { ch });
                }
                out.push_str(&host_port[end..]);
                break;
            }
            out.push_str(value);
            cursor = end;
            continue;
        }
        out.push_str(prefix);
        cursor = pos + prefix.len();
    }
    out.push_str(&url[authority_end..]);
    std::borrow::Cow::Owned(out)
}

/// Restores tokenized Mermaid placeholders in an already-sanitized `http:` / `https:` URL that
/// bypassed `Url::parse` (for example, an URL with an explicit `:port` or a `:` in query/fragment
/// that made `url_scheme` include `://`).
fn restore_http_tokens_by_component(url: &str, tokens: &UrlEntityTokens) -> String {
    let scheme_len = if url
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http:"))
    {
        5
    } else if url
        .get(..6)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https:"))
    {
        6
    } else {
        return url.to_owned();
    };
    let slashes = url[scheme_len..].bytes().take_while(|&b| b == b'/').count();
    let authority_start = scheme_len + slashes;
    let authority_end = url[authority_start..]
        .find(['/', '?', '#'])
        .map_or(url.len(), |offset| authority_start + offset);
    let authority = &url[authority_start..authority_end];
    let host_start = authority
        .rfind('@')
        .map_or(authority_start, |at| authority_start + at + 1);

    let frag_start = url[authority_end..]
        .find('#')
        .map_or(url.len(), |offset| authority_end + offset);
    let query_start = url[authority_end..frag_start]
        .find('?')
        .map_or(frag_start, |offset| authority_end + offset);

    let mut out = String::with_capacity(url.len());
    out.push_str(&url[..authority_start]);
    substitute_component_tokens(
        &mut out,
        &url[authority_start..host_start],
        tokens,
        UrlEncodeComponent::Userinfo,
    );
    out.push_str(&url[host_start..authority_end]);
    substitute_component_tokens(
        &mut out,
        &url[authority_end..query_start],
        tokens,
        UrlEncodeComponent::Path,
    );
    substitute_component_tokens(
        &mut out,
        &url[query_start..frag_start],
        tokens,
        UrlEncodeComponent::SpecialQuery,
    );
    substitute_component_tokens(
        &mut out,
        &url[frag_start..],
        tokens,
        UrlEncodeComponent::Fragment,
    );
    out
}

/// The `http:` and `https:` branch of `sanitizeUrl`: `about:blank` when `URL.canParse` is false,
/// otherwise the serialized `URL` with any tokenized Mermaid placeholders substituted back using
/// each component's WHATWG percent-encode set.
fn normalize_http_with_tokens(candidate: &str, tokens: Option<&UrlEntityTokens>) -> String {
    let Ok(parsed) = ::url::Url::parse(candidate) else {
        return BLANK_URL.to_owned();
    };
    let Some(tokens) = tokens else {
        return parsed.to_string();
    };
    use ::url::Position;
    let mut out = String::with_capacity(parsed.as_str().len());
    out.push_str(&parsed[..Position::BeforeUsername]);
    substitute_component_tokens(
        &mut out,
        &parsed[Position::BeforeUsername..Position::BeforeHost],
        tokens,
        UrlEncodeComponent::Userinfo,
    );
    out.push_str(&parsed[Position::BeforeHost..Position::BeforePath]);
    substitute_component_tokens(
        &mut out,
        &parsed[Position::BeforePath..Position::AfterPath],
        tokens,
        UrlEncodeComponent::Path,
    );
    substitute_component_tokens(
        &mut out,
        &parsed[Position::AfterPath..Position::AfterQuery],
        tokens,
        UrlEncodeComponent::SpecialQuery,
    );
    substitute_component_tokens(
        &mut out,
        &parsed[Position::AfterQuery..],
        tokens,
        UrlEncodeComponent::Fragment,
    );
    out
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
    use super::{decode_uri, format_url, guard_image_url, guard_url, sanitize_url, word_to_uint16};

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
        ("https://example.com/a^b", "https://example.com/a%5Eb"),
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

    #[test]
    fn guard_url_blocks_unsafe_schemes_without_rewriting_accepted_urls() {
        for unsafe_url in [
            "",
            "   ",
            "javascript:alert(1)",
            "JAVASCRIPT:alert(1)",
            "vbscript:msgbox(1)",
            "data:text/html,<script>alert(1)</script>",
            "notes://do-your-thing/id",
            "file:///etc/passwd",
            "custom:thing",
            "a:b",
            "a%3Ab",
            "notes%25253Adoc",
            "notes%3Afoo%FF",
            "notes%253Afoo%FF",
            "notes&#x3A;item",
            "notes&#X3a;item",
            "&#x6a;avascript:alert(1)",
            "&#106avascript:alert(1)",
            "javascript&#x3A;alert(1)#frag",
            "javascript%3Aalert(1)#123",
            "item%25252525252525253Fq=A#123",
            "%26#106;avascript:alert(1)",
            "%&tab;26#106;avascript:alert(1)",
            "&\n#106;avascript:alert(1)",
            "&#38;#106;avascript:alert(1)",
            "&#106;avascript:alert(1)",
            "javascript&colon;alert(1)",
            "%6aavascript:alert(1)",
            "java\tscript:alert(1)",
            "java\\tscript:alert(1)",
            "java script:alert(1)",
        ] {
            assert_eq!(
                guard_url(unsafe_url, false),
                "about:blank",
                "{unsafe_url:?}"
            );
            assert_eq!(guard_url(unsafe_url, true), unsafe_url, "{unsafe_url:?}");
        }

        for safe_url in [
            "https://google.com",
            "https&colon;//example.org",
            "https://example.com/A%23B.png",
            "https://example.com/files?q=A%26B",
            "https://tickets.example.com/browse/A%2526B",
            "mailto:user@user.user",
            "mailto:user%2525252525252525252541@example.com",
            "cid:asset%2525252525252525252541@example.com",
            "cid&colon;asset%2525252525252525252541@example.org",
            "cid%26colon%3Basset%2525252525252525252541@example.org",
            "tel:+1234567",
            "ftp://example.com/file",
            "data/chart.svg?time=12:30",
            "javascript/docs.html?t=1:2",
            "dir/a%3Ab.png",
            "#section",
            "R&#section:1",
            "R&D#A%2525252525252525252541",
            "my%20notes#A%2525252525252525252541",
            "item%3Fq=A%2525252525252525252541",
            "item%3Fq=A%2525252525252525252541#123",
            "item%252525252525253Fq=A%2525252525252525252541#123",
            "item%23section%2525252525252525252541",
            "item%23section%2525252525252525252541#123",
            "item%23123%2525252525252525252541",
            "item%23123%2525252525252525252541#123",
            "rate%25x%23123%2525252525252525252541",
            "rate%25%23123%2525252525252525252541",
            "foo%2525252525252523123%2525252525252525252541",
            "foo\\n#123%2525252525252525252541",
            "&%20#123%2525252525252525252541",
            "R&D%23123%2525252525252525252541",
            "/images/logo.png",
            "./relative:path.png",
            "%20",
            "%20#section",
        ] {
            assert_eq!(guard_url(safe_url, false), safe_url, "{safe_url:?}");
            assert_eq!(
                guard_url(guard_url(safe_url, false), false),
                safe_url,
                "idempotent {safe_url:?}"
            );
        }

        // Deeply nested entities in the scheme-relevant prefix fail closed once the pass budget is
        // exhausted, while entities after a path/query delimiter do not trigger extra passes.
        let mut deeply_nested = "&#106;avascript:alert(1)".to_owned();
        for _ in 0..16 {
            deeply_nested = deeply_nested.replace('&', "&#38;");
        }
        assert_eq!(guard_url(&deeply_nested, false), "about:blank");
        assert_eq!(sanitize_url(&deeply_nested), "about:blank");
        // Percent-only nesting enforces the exact 8-pass boundary on the parser path:
        // 8 layers (%25 repeated 7 times + 41) stay accepted with sanitizeUrl parity,
        // while 9 layers (%25 repeated 8 times + 41) fail closed to about:blank.
        assert_eq!(
            sanitize_url("https://example.com/a%2525252525252541"),
            "https://example.com/a%252525252541"
        );
        assert_eq!(
            sanitize_url("https://example.com/a%2525252525252541?value=50%"),
            "https://example.com/a%2525252525252541?value=50%"
        );
        assert_eq!(
            sanitize_url("https://example.com/a%252525252525252541"),
            "about:blank"
        );
        assert_eq!(
            sanitize_url("https://example.com/a%252525252525252541?value=50%"),
            "about:blank"
        );
        assert_eq!(
            sanitize_url("https://example.com/a%25252525252525252541"),
            "about:blank"
        );
        assert_eq!(
            sanitize_url("https://example.com/files?q=A%252526B"),
            "https://example.com/files?q=A%26B"
        );
        // Mixed percent-then-entity nesting past 8 passes also fails closed on the parser path.
        let mixed_over_budget = format!("%252526{}#65;", "#38;".repeat(9));
        assert_eq!(sanitize_url(&mixed_over_budget), "about:blank");

        // Exact 8-pass boundary on the sink path for nested entities: 8 layers (`&` encoded 7
        // times around `&#65;`) pass, while 9 layers (`&` encoded 8 times) fail closed.
        let eight_entity_layers = format!("&{}#65;", "#38;".repeat(7));
        let nine_entity_layers = format!("&{}#65;", "#38;".repeat(8));
        assert_eq!(guard_url(&eight_entity_layers, false), eight_entity_layers);
        assert_eq!(guard_url(&nine_entity_layers, false), "about:blank");

        let query_nested = format!("https://example.com/?q={deeply_nested}");
        assert_eq!(guard_url(&query_nested, false), query_nested);
    }

    #[test]
    fn guard_image_url_allows_safe_raster_data_uris_but_blocks_active_payloads() {
        for safe_data_url in [
            "data:image/png;base64,iVBORw0KGgo=",
            "data:image/x-png;base64,iVBORw0KGgo=",
            "data:image/pjpeg;base64,/9j/4AAQ",
            "data:image/x-ms-bmp;base64,Qk0=",
            " data:image/png;base64,iVBORw0KGgo=",
            "\tdata:image/png;base64,iVBORw0KGgo=",
            "data: image/png;base64,iVBORw0KGgo=",
            "data:im\tage/png;base64,iVBORw0KGgo=",
            "data:image/p\nng;base64,iVBORw0KGgo=",
            "data:\timage/png ;base64,iVBORw0KGgo=",
            "data:image/png ;base64,iVBORw0KGgo=",
            "data:image/png  ,raw",
            "data:image/apng;base64,iVBORw0KGgo=",
            "data:image/tiff;base64,SUkqAAg=",
            "data:image/jxl;base64,/wo=",
            "DATA:IMAGE/PNG;base64,iVBORw0KGgo=",
            "data:image/jpeg;base64,/9j/4AAQ",
            "data:image/gif;base64,R0lGODlh",
            "data:image/webp;base64,UklGR",
            "data:image/png,raw",
        ] {
            assert_eq!(
                guard_image_url(safe_data_url, false),
                safe_data_url,
                "{safe_data_url:?}"
            );
            // Links never permit data: URLs even for raster images.
            assert_eq!(
                guard_url(safe_data_url, false),
                "about:blank",
                "link {safe_data_url:?}"
            );
        }

        for unsafe_data_url in [
            "data:image/svg+xml,<svg onload=alert(1)>",
            " data:image/svg+xml,<svg onload=alert(1)>",
            "data:image/svg+xml;base64,PHN2Zz4=",
            "data:text/html,<script>alert(1)</script>",
            "data:image/png",
            "javascript:alert(1)",
        ] {
            assert_eq!(
                guard_image_url(unsafe_data_url, false),
                "about:blank",
                "{unsafe_data_url:?}"
            );
            assert_eq!(
                guard_image_url(unsafe_data_url, true),
                unsafe_data_url,
                "loose {unsafe_data_url:?}"
            );
        }
    }

    #[test]
    fn http_placeholder_tokenization_stays_linear_and_bounded() {
        // Adversarial input trying to force a growing prefix + many placeholders stays O(n) in
        // time and space, and scans standalone closing/opening markers in one forward pass.
        let mut prefix_collisions = String::from("https://example.org/");
        for i in 0..256u32 {
            prefix_collisions.push_str(&format!("__fg{i:08x}_"));
        }
        prefix_collisions.push_str(&"ﬂ°copy¶ß".repeat(512));
        let sanitized = sanitize_url(&prefix_collisions);
        assert!(sanitized.starts_with("https://example.org/__fg00000000_"));
        assert!(sanitized.ends_with(&"%C2%A9".repeat(512)));

        let many_closes = format!("https://example.org/{}", "¶ß".repeat(4000));
        let sanitized_closes = sanitize_url(&many_closes);
        assert_eq!(
            sanitized_closes,
            format!("https://example.org/{}", ";".repeat(4000))
        );

        let many_opens = format!("https://example.org/{}", "ﬂ°".repeat(4000));
        let sanitized_opens = sanitize_url(&many_opens);
        assert_eq!(
            sanitized_opens,
            format!("https://example.org/{}", "&".repeat(4000))
        );

        // Entity-spelled HTTP scheme and invalid host with port / query colon.
        assert_eq!(
            sanitize_url("httpsﬂ°°58¶ß//example.org/Partﬂ°°63¶ßA.pdf"),
            "https://example.org/Part%3FA.pdf"
        );
        assert_eq!(
            sanitize_url("https://exampleﬂ°°32¶ßorg:8080/docs"),
            "about:blank"
        );
        assert_eq!(
            sanitize_url("https://exampleﬂ°°32¶ßorg/?time=12:00"),
            "about:blank"
        );
        // Decoded percent-encoded placeholder lookalikes stay percent-encoded in relative and
        // explicit-port URLs while preserving real source entities.
        assert_eq!(
            sanitize_url("/files/%EF%AC%82%C2%B0copy%C2%B6%C3%9F.pdf"),
            "/files/%EF%AC%82%C2%B0copy%C2%B6%C3%9F.pdf"
        );
        assert_eq!(
            sanitize_url("https://example.org:8080/%EF%AC%82%C2%B0copy%C2%B6%C3%9F.pdf"),
            "https://example.org:8080/%EF%AC%82%C2%B0copy%C2%B6%C3%9F.pdf"
        );
        assert_eq!(
            sanitize_url("/files/%EF%AC%82%C2%B0copy%C2%B6%C3%9F.pdf?q=ﬂ°copy¶ß"),
            "/files/%EF%AC%82%C2%B0copy%C2%B6%C3%9F.pdf?q=ﬂ°copy¶ß"
        );
        // Leading `&` followed by multibyte UTF-8 crossing the 32-byte entity-probe boundary
        // must never panic on a UTF-8 char boundary.
        let multibyte_amp = format!("&{}ﬂ°copy¶ß", "é".repeat(20));
        assert_eq!(sanitize_url(&multibyte_amp), multibyte_amp);
        // Overlapping `__fgXXXXXXXX_` prefixes sharing an underscore must both be detected.
        assert_eq!(
            sanitize_url("https://example.org/__fg00000001__fg00000000_0__/ﬂ°copy¶ß"),
            "https://example.org/__fg00000001__fg00000000_0__/%C2%A9"
        );
        // Multi-pass obfuscated HTTP schemes (`%2568ttps://`) still get component-aware encoding
        // and authority validation once decoded to HTTP.
        assert_eq!(
            sanitize_url("%2568ttps://example.org/partﬂ°°63¶ßone"),
            "https://example.org/part%3Fone"
        );
        assert_eq!(
            sanitize_url("%2568ttps://exampleﬂ°°32¶ßorg:8080/partﬂ°°63¶ßone"),
            "about:blank"
        );
        // Percent-encoded path separators before an entity classify the entity in the decoded path.
        assert_eq!(
            sanitize_url("https://example.org%2Fdocsﬂ°°63¶ßb"),
            "https://example.org/docs%3Fb"
        );
        // Entity-encoded authority terminators (`#47;` / `#92;`) stop host-token substitution so
        // following entities like `#63;` are classified and percent-encoded in the path.
        assert_eq!(
            sanitize_url("https://example.orgﬂ°°47¶ßdocsﬂ°°63¶ßb"),
            "https://example.org/docs%3Fb"
        );
        assert_eq!(
            sanitize_url("https://example.orgﬂ°°92¶ßdocsﬂ°°63¶ßb"),
            "https://example.org/docs%3Fb"
        );
        // Entity-encoded dots in IPv4 hosts resolve before WHATWG URL parsing, and resolving a
        // host entity never triggers a second percent-decoding pass on the query string.
        assert_eq!(
            sanitize_url("https://192ﬂ°°46¶ß168.1.1/x"),
            "https://192.168.1.1/x"
        );
        assert_eq!(
            sanitize_url("https://exampleﬂ°°46¶ßorg/?q=A%252526B"),
            "https://example.org/?q=A%26B"
        );
        // Carets (`^`) restored into HTTP paths are percent-encoded per WHATWG URL path rules.
        assert_eq!(
            sanitize_url("https://example.org/aﬂ°°94¶ßb"),
            "https://example.org/a%5Eb"
        );
        // Same-document fragment links preserve their percent-encoded spelling in `format_url`
        // (for both literal `#` and entity `#35;`), never misread colons like `#database:overview`
        // as schemes, and still fail closed on over-budget nested encodings.
        assert_eq!(
            format_url("#graph-flowchart-A%2520B-0", false).as_deref(),
            Some("#graph-flowchart-A%2520B-0")
        );
        assert_eq!(
            format_url("ﬂ°°35¶ßgraph-classId-A%2520B", false).as_deref(),
            Some("ﬂ°°35¶ßgraph-classId-A%2520B")
        );
        assert_eq!(
            format_url("#database:overview", false).as_deref(),
            Some("#database:overview")
        );
        assert_eq!(
            format_url("#foo%25252525252525252541", false).as_deref(),
            Some("about:blank")
        );
        // Safe URLs preserve percent-encoded delimiters in `format_url` while still rejecting
        // obfuscated unsafe schemes and invalid hosts.
        assert_eq!(
            format_url("https://example.org/?q=A%26B", false).as_deref(),
            Some("https://example.org/?q=A%26B")
        );
        assert_eq!(
            format_url("https://EXAMPLE.ORG/a%2Fb?q=1%3D2#s%231", false).as_deref(),
            Some("https://example.org/a%2Fb?q=1%3D2#s%231")
        );
        assert_eq!(
            format_url("https://example.org:8080/a%2Fb?q=A%26B", false).as_deref(),
            Some("https://example.org:8080/a%2Fb?q=A%26B")
        );
        assert_eq!(
            format_url("https://exampleﬂ°°46¶ßorg/Chapterﬂ°°32¶ßOne?q=A%26B", false).as_deref(),
            Some("https://example.org/Chapter%20One?q=A%26B")
        );
        assert_eq!(
            format_url("/docs/a%2Fb?q=A%26B", false).as_deref(),
            Some("/docs/a%2Fb?q=A%26B")
        );
        assert_eq!(
            format_url("docs/a%2Fb?q=A%26B", false).as_deref(),
            Some("docs/a%2Fb?q=A%26B")
        );
        assert_eq!(
            format_url("docs/a%2Fb?t=12:30&q=A%26B", false).as_deref(),
            Some("docs/a%2Fb?t=12:30&q=A%26B")
        );
        assert_eq!(
            format_url("mailto:a@b.c?subject=A%26B", false).as_deref(),
            Some("mailto:a@b.c?subject=A%26B")
        );
        assert_eq!(
            format_url("%6aavascript:alert(1)", false).as_deref(),
            Some("about:blank")
        );
        assert_eq!(
            format_url("https://example.org%2Fdocsﬂ°°63¶ßb", false).as_deref(),
            Some("https://example.org/docs%3Fb")
        );
        assert_eq!(
            sanitize_url("https://example.org/%5f_fg00000000_0__/ﬂ°copy¶ß"),
            "https://example.org/__fg00000000_0__/%C2%A9"
        );
        // Relative links whose path/query precedes a colon (like `data/chart.svg?time=12:30`)
        // stay intact instead of triggering `data:` scheme rejection, while encoded colons before
        // `/` still fail closed.
        assert_eq!(
            format_url("data/chart.svg?time=12:30", false).as_deref(),
            Some("data/chart.svg?time=12:30")
        );
        assert_eq!(
            format_url("data/chart.svg?time=12:30&q=A%26B", false).as_deref(),
            Some("data/chart.svg?time=12:30&q=A%26B")
        );
        assert_eq!(
            format_url("javascript/docs.html?t=1:2", false).as_deref(),
            Some("javascript/docs.html?t=1:2")
        );
        assert_eq!(
            format_url("data%3A/chart.svg", false).as_deref(),
            Some("about:blank")
        );
        assert_eq!(
            format_url("data&#58;/chart.svg", false).as_deref(),
            Some("about:blank")
        );
        // Encoded backslashes followed by `n`/`r`/`t` inside validated HTTP paths/queries stay
        // intact while scheme obfuscation (`java%5Cnscript:`) still fails closed.
        assert_eq!(
            format_url("https://example.com/backups%5Cnew.zip", false).as_deref(),
            Some("https://example.com/backups%5Cnew.zip")
        );
        assert_eq!(
            format_url(
                "https://example.com/files?path=C:%5Ctemp%5Creport.pdf",
                false
            )
            .as_deref(),
            Some("https://example.com/files?path=C:%5Ctemp%5Creport.pdf")
        );
        assert_eq!(
            format_url("java%5Cnscript:alert(1)", false).as_deref(),
            Some("about:blank")
        );
        // Relative URLs with a fragment delimiter before a colon (`data#part:two`) stay intact
        // while encoded colons before `#` still fail closed.
        assert_eq!(
            format_url("data#part:two", false).as_deref(),
            Some("data#part:two")
        );
        assert_eq!(
            format_url("data#123:two", false).as_deref(),
            Some("data#123:two")
        );
        assert_eq!(
            format_url("data%3A#part", false).as_deref(),
            Some("about:blank")
        );
        assert_eq!(
            format_url("data&#58;#part", false).as_deref(),
            Some("about:blank")
        );
        // Encoded numeric references (`&ﬂ°°169¶ß`, `&ﬂ°xA9¶ß`, `&ﬂ°°128512¶ß`) decode as Unicode
        // without leaving a stray `&` or clobbering percent-escapes in other components, and
        // scheme obfuscation (`&ﬂ°°106¶ßavascript:`, `&ﬂ°x6A¶ßavascript:`) fails closed.
        assert_eq!(
            format_url("https://example.com/A&ﬂ°°169¶ßB", false).as_deref(),
            Some("https://example.com/A%C2%A9B")
        );
        assert_eq!(
            format_url("https://example.org/a%2Fb?note=&ﬂ°°169¶ß", false).as_deref(),
            Some("https://example.org/a%2Fb?note=%C2%A9")
        );
        assert_eq!(
            format_url("https://example.org/a&ﬂ°xA9¶ßb", false).as_deref(),
            Some("https://example.org/a%C2%A9b")
        );
        assert_eq!(
            format_url("https://example.org/a&ﬂ°°128512¶ßb", false).as_deref(),
            Some("https://example.org/a%F0%9F%98%80b")
        );
        assert_eq!(
            format_url("&ﬂ°°106¶ßavascript:alert(1)", false).as_deref(),
            Some("about:blank")
        );
        assert_eq!(
            format_url("&ﬂ°x6A¶ßavascript:alert(1)", false).as_deref(),
            Some("about:blank")
        );
        // Valid escaped HTTP(S) userinfo (`%23alice@`) is validated before percent-decoding turns
        // `%23` into `#`, while same-document fragments restore numeric references without leaving
        // a stray `&` or decoding `%` escapes.
        assert_eq!(
            format_url("https://%23alice@example.org/report", false).as_deref(),
            Some("https://%23alice@example.org/report")
        );
        assert_eq!(
            format_url("#graph-classId-Caf&ﬂ°°233¶ß", false).as_deref(),
            Some("#graph-classId-Cafﬂ°°233¶ß")
        );
        assert_eq!(
            format_url("#graph-classId-Caf&ﬂ°xE9¶ß%201", false).as_deref(),
            Some("#graph-classId-Cafﬂ°°233¶ß%201")
        );
        // Relative URLs with percent-encoded path/colon delimiters (`docs%2Freport%3A2026.pdf`,
        // `data%2Freport%3A2026.pdf`) preserve their escapes while encoded schemes fail closed.
        assert_eq!(
            format_url("docs%2Freport%3A2026.pdf", false).as_deref(),
            Some("docs%2Freport%3A2026.pdf")
        );
        assert_eq!(
            format_url("data%2Freport%3A2026.pdf", false).as_deref(),
            Some("data%2Freport%3A2026.pdf")
        );
        assert_eq!(
            format_url("javascript%3Aalert(1)", false).as_deref(),
            Some("about:blank")
        );
    }
}
