//! What the custom renderers of the reference share: the text measurement of `utils.ts`
//! ([`text`]), the drawing helpers of `diagrams/common/svgDrawCommon.ts` ([`draw`], with the URL
//! sanitiser of [`url`]), the sizing of `setupGraphViewbox.js` ([`viewbox`]) and the title of
//! `insertTitle` ([`title`]). The element they write is [`element::Element`].

pub mod draw;
pub mod element;
pub mod style;
pub mod text;
pub mod title;
pub mod url;
pub mod viewbox;

pub use element::Element;
pub use text::{BreakString, MeasureError, TextDimensions, TextMemo, TextStyle, WrapStyle};

/// JavaScript's `\s`: white space and the line terminators, as `String.prototype.trim` removes them.
#[must_use]
pub fn is_js_whitespace(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// `String.prototype.trim`.
#[must_use]
pub fn js_trim(text: &str) -> &str {
    text.trim_matches(is_js_whitespace)
}

/// `String.prototype.trimStart`.
#[must_use]
pub fn js_trim_start(text: &str) -> &str {
    text.trim_start_matches(is_js_whitespace)
}
