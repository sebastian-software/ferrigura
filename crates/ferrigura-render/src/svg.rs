//! SVG text helpers: escaping, numbers, the paths of arc segments and the embedded font faces.

use std::f64::consts::PI;
use std::fmt::Write as _;

use ferrigura_jsmath::{cos, sin};
use ferrigura_syntax::decode_entities;
use ferrigura_text::EmbeddedFamily;

use crate::common::d3::d3_number;

/// Escapes text for an XML element or attribute value. A character that XML 1.0 cannot hold (a
/// control other than tab, line feed and carriage return) is written as U+FFFD.
#[must_use]
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    escape_into(&mut out, text);
    out
}

/// Appends the text to `out`, escaped as [`escape`] escapes it. The runs between the characters
/// that need a replacement are copied whole.
pub fn escape_into(out: &mut String, text: &str) {
    let mut start = 0;
    for (at, c) in text.char_indices() {
        let replacement = match c {
            '&' => "&amp;",
            '<' => "&lt;",
            '>' => "&gt;",
            '"' => "&quot;",
            '\'' => "&#39;",
            other if !is_xml_char(other) => "\u{fffd}",
            _ => continue,
        };
        out.push_str(&text[start..at]);
        out.push_str(replacement);
        start = at + c.len_utf8();
    }
    out.push_str(&text[start..]);
}

/// Escapes a URL for an XML attribute (`href` / `xlink:href`), additionally encoding tab, line
/// feed, and carriage return as numeric character references so XML 1.0 attribute-value
/// normalization does not turn them into spaces.
#[must_use]
pub(crate) fn escape_url_attr(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    escape_url_attr_into(&mut out, text);
    out
}

/// Appends `text` to `out`, escaped as [`escape_url_attr`] escapes it.
pub(crate) fn escape_url_attr_into(out: &mut String, text: &str) {
    let mut start = 0;
    for (at, c) in text.char_indices() {
        let replacement = match c {
            '&' => "&amp;",
            '<' => "&lt;",
            '>' => "&gt;",
            '"' => "&quot;",
            '\'' => "&#39;",
            '\t' => "&#9;",
            '\n' => "&#10;",
            '\r' => "&#13;",
            other if !is_xml_char(other) => "\u{fffd}",
            _ => continue,
        };
        out.push_str(&text[start..at]);
        out.push_str(replacement);
        start = at + c.len_utf8();
    }
    out.push_str(&text[start..]);
}

/// The markup of a text as the reference's final pass leaves it, for an XML parser: the text
/// escaped, then the placeholders of `encodeEntities` decoded on the markup (`decodeEntities`,
/// `mermaidAPI.ts` 359), then the references that XML does not define rewritten
/// ([`xml_references`]).
#[must_use]
pub fn entity_markup(text: &str) -> String {
    xml_references(&decode_entities(&escape(text)))
}

/// Markup with every character reference that XML does not read as HTML does replaced by the text
/// that the HTML parser reads from it. The placeholders of the entities a user types decode to
/// references by name (`#nbsp;` to `&nbsp;`), which the browser's HTML parser reads and an XML
/// parser rejects. The five references XML defines and a numeric reference to a character that XML
/// allows and HTML reads as itself are kept. Any other reference is decoded as the HTML parser's
/// text state decodes it and escaped again: `&nbsp;` becomes U+00A0, `&#128;` becomes `€`, and a
/// name HTML does not know, `&amp;name;`.
#[must_use]
pub fn xml_references(markup: &str) -> String {
    let mut out = String::with_capacity(markup.len());
    let mut rest = markup;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        // `&`, a `#` for a number, the letters and digits, and the `;` that ends them.
        let start = if rest[1..].starts_with('#') { 2 } else { 1 };
        let end = rest[start..]
            .find(|c: char| !c.is_ascii_alphanumeric())
            .map_or(rest.len(), |offset| start + offset);
        let length = if rest[end..].starts_with(';') {
            end + 1
        } else {
            end
        };
        let reference = &rest[..length];
        if is_xml_reference(reference) {
            out.push_str(reference);
        } else {
            out.push_str(&escape(&ferrigura_syntax::sanitize::decode_references(
                reference,
            )));
        }
        rest = &rest[length..];
    }
    out.push_str(rest);
    out
}

/// Whether XML reads the reference as the HTML parser does: one of the five that XML defines, or a
/// numeric reference to a character that XML allows and HTML reads as itself.
fn is_xml_reference(reference: &str) -> bool {
    if matches!(reference, "&amp;" | "&lt;" | "&gt;" | "&quot;" | "&apos;") {
        return true;
    }
    let Some(number) = reference
        .strip_prefix("&#")
        .and_then(|rest| rest.strip_suffix(';'))
    else {
        return false;
    };
    let value = match number.strip_prefix(['x', 'X']) {
        Some(hex) => u32::from_str_radix(hex, 16),
        None => number.parse(),
    };
    value.ok().and_then(char::from_u32).is_some_and(|c| {
        is_xml_char(c) && ferrigura_syntax::sanitize::decode_references(reference) == c.to_string()
    })
}

/// Whether XML 1.0 allows the character in a document.
pub(crate) fn is_xml_char(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..)
}

/// The declaration of the `xlink` prefix that a root needs when the drawing writes `xlink:href`,
/// with a space before it, or nothing. Without it the prefix is unbound, and an XML parser rejects
/// the SVG. The reference declares it after `xmlns` on the root that `mermaid.run` draws
/// (`appendDivSvgG` with `XMLNS_XLINK_STD`, `mermaidAPI.ts:414-416`); the root of `mermaid.render`
/// has none, and the HTML serialization writes `xlink:href` without it.
#[must_use]
pub fn xlink_declaration(links: bool) -> &'static str {
    if links {
        " xmlns:xlink=\"http://www.w3.org/1999/xlink\""
    } else {
        ""
    }
}

/// Text for a `<style>` element: the characters that would end the element or start a reference
/// are escaped, and the quotes of CSS are kept as they are.
pub fn css_text(css: &str) -> String {
    css.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Appends the style text to `out`, escaped as [`css_text`] escapes it. The runs between the
/// characters that need a replacement are copied whole.
pub(crate) fn css_text_into(out: &mut String, css: &str) {
    let mut start = 0;
    for (at, byte) in css.bytes().enumerate() {
        let replacement = match byte {
            b'&' => "&amp;",
            b'<' => "&lt;",
            b'>' => "&gt;",
            _ => continue,
        };
        out.push_str(&css[start..at]);
        out.push_str(replacement);
        start = at + 1;
    }
    out.push_str(&css[start..]);
}

/// Escape owned style text, retaining its allocation when no XML escaping is needed.
pub(crate) fn css_text_owned(css: String) -> String {
    if memchr::memchr3(b'&', b'<', b'>', css.as_bytes()).is_some() {
        css_text(&css)
    } else {
        css
    }
}

/// A number as the reference writes it in an attribute: JavaScript's `String(number)`, written by
/// [`ferrigura_model::value::number_text`], which the theme tokens read with.
#[must_use]
pub fn number(value: f64) -> String {
    ferrigura_model::value::number_text(value)
}

/// The `@font-face` rules of the embedded families, in the order given.
#[must_use]
pub fn font_faces(id: &str, families: &[EmbeddedFamily]) -> String {
    let mut css = String::new();
    font_faces_into(&mut css, id, families);
    css
}

/// Appends the `@font-face` rules of [`font_faces`] to `css`.
pub(crate) fn font_faces_into(css: &mut String, id: &str, families: &[EmbeddedFamily]) {
    for (index, family) in families.iter().enumerate() {
        if index > 0 {
            css.push('\n');
        }
        css.push_str(&family.css);
    }
    let mut variables = families
        .iter()
        .flat_map(|family| &family.variables)
        .peekable();
    if variables.peek().is_some() {
        let _ = write!(css, "#{id}{{font-synthesis:none;");
        for (name, value) in variables {
            let _ = write!(css, "{name}:{value};");
        }
        css.push('}');
    }
}

/// The path of an annular sector, as `d3-shape`'s `arc` writes it: the outer arc from `start` to
/// `end`, then the inner arc back. Angles are in radians, measured clockwise from twelve o'clock.
/// Every number has three decimals at most (`Math.round(x * 1000) / 1000`), the default `digits`
/// of the shapes since `d3-shape` 3.2.0, so a point on an axis is `0` and not `1e-14`.
///
/// A full circle is drawn as two half arcs, as `d3-path` does, so that it is not a zero-length arc.
#[must_use]
pub fn arc_path(inner: f64, outer: f64, start: f64, end: f64) -> String {
    const TAU: f64 = 2.0 * PI;
    const EPSILON: f64 = 1e-12;
    let half_pi = PI / 2.0;
    let a0 = start - half_pi;
    let a1 = end - half_pi;
    let da = (a1 - a0).abs();
    let clockwise = a1 > a0;
    let point = |radius: f64, angle: f64| (radius * cos(angle), radius * sin(angle));

    if outer.is_nan() || outer <= EPSILON {
        return "M0,0Z".to_owned();
    }

    let mut path = String::new();
    if da > TAU - EPSILON {
        let (x0, y0) = point(outer, a0);
        let sweep = u8::from(clockwise);
        let r_outer = d3_number(outer);
        let x0_s = d3_number(x0);
        let y0_s = d3_number(y0);
        let _ = write!(
            path,
            "M{x0_s},{y0_s}A{r_outer},{r_outer},0,1,{sweep},{},{}A{r_outer},{r_outer},0,1,{sweep},{x0_s},{y0_s}",
            d3_number(-x0),
            d3_number(-y0),
        );
        if inner > EPSILON {
            let (xi, yi) = point(inner, a1);
            let back = 1 - sweep;
            let r_inner = d3_number(inner);
            let xi_s = d3_number(xi);
            let yi_s = d3_number(yi);
            let _ = write!(
                path,
                "M{xi_s},{yi_s}A{r_inner},{r_inner},0,1,{back},{},{}A{r_inner},{r_inner},0,1,{back},{xi_s},{yi_s}Z",
                d3_number(-xi),
                d3_number(-yi),
            );
        } else {
            path.push('Z');
        }
        return path;
    }

    let large = u8::from(da >= PI);
    let sweep = u8::from(clockwise);
    let (x0, y0) = point(outer, a0);
    let (x1, y1) = point(outer, a1);
    path.push_str(&format!(
        "M{},{}A{},{},0,{},{},{},{}",
        d3_number(x0),
        d3_number(y0),
        d3_number(outer),
        d3_number(outer),
        large,
        sweep,
        d3_number(x1),
        d3_number(y1),
    ));
    if inner > EPSILON {
        let (xi1, yi1) = point(inner, a1);
        let (xi0, yi0) = point(inner, a0);
        path.push_str(&format!(
            "L{},{}A{},{},0,{},{},{},{}",
            d3_number(xi1),
            d3_number(yi1),
            d3_number(inner),
            d3_number(inner),
            large,
            1 - sweep,
            d3_number(xi0),
            d3_number(yi0),
        ));
    } else {
        path.push_str("L0,0");
    }
    path.push('Z');
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owned_css_escapes_xml_and_reuses_safe_text() {
        let safe = String::from("a{content:\"é\"}");
        let ptr = safe.as_ptr();
        let escaped = css_text_owned(safe);
        assert_eq!(escaped.as_ptr(), ptr);
        assert_eq!(escaped, "a{content:\"é\"}");
        assert_eq!(css_text_owned("&<>é".into()), "&amp;&lt;&gt;é");
    }

    #[test]
    fn escapes_markup_and_quotes() {
        assert_eq!(
            escape("a<b & \"c\" 'd'"),
            "a&lt;b &amp; &quot;c&quot; &#39;d&#39;"
        );
        assert_eq!(escape("a\u{1}b\tc\u{fffe}"), "a\u{fffd}b\tc\u{fffd}");
    }

    #[test]
    fn references_xml_reads_as_html_does_are_kept() {
        let markup = "&amp; &lt; &gt; &quot; &apos; &#39; &#9829; &#x2665;";
        assert_eq!(xml_references(markup), markup);
    }

    #[test]
    fn other_references_are_written_as_the_html_parser_reads_them() {
        assert_eq!(xml_references("a&nbsp;b"), "a\u{a0}b");
        assert_eq!(xml_references("&LT;&copy;"), "&lt;\u{a9}");
        assert_eq!(xml_references("&#128;&#0;&#1;"), "\u{20ac}\u{fffd}\u{fffd}");
        assert_eq!(
            xml_references("&unknown; &not_x;"),
            "&amp;unknown; \u{ac}_x;"
        );
        assert_eq!(xml_references("& x &"), "&amp; x &amp;");
    }

    #[test]
    fn typed_entities_are_xml() {
        assert_eq!(
            entity_markup(
                "Plan\u{fb02}\u{b0}nbsp\u{b6}\u{df}A <\u{fb02}\u{b0}\u{b0}9829\u{b6}\u{df}>"
            ),
            "Plan\u{a0}A &lt;&#9829;&gt;"
        );
    }

    #[test]
    fn a_half_slice_is_a_large_arc_free_sector() {
        let path = arc_path(0.0, 10.0, 0.0, PI);
        assert!(path.starts_with("M0,-10A10,10,0,"), "{path}");
        assert!(path.ends_with("L0,0Z"), "{path}");
    }

    /// The start of a slice at twelve o'clock is `185 * cos(-π / 2)`, `1.1327982892113017e-14`,
    /// which d3 writes as `0`, as it writes every number with three decimals (the path is
    /// `arc().outerRadius(185)` of `d3-shape` 3.2.0 in Node).
    #[test]
    fn numbers_have_three_decimals_as_d3_writes_them() {
        assert_eq!(
            arc_path(0.0, 185.0, 0.0, 1.2),
            "M0,-185A185,185,0,0,1,172.427,-67.036L0,0Z"
        );
    }

    #[test]
    fn a_full_circle_is_two_half_arcs() {
        let clockwise = arc_path(0.0, 10.0, 0.0, 2.0 * PI);
        assert_eq!(clockwise, "M0,-10A10,10,0,1,1,0,10A10,10,0,1,1,0,-10Z");
        assert_eq!(
            ferrigura_path::bounding_box(&clockwise),
            ferrigura_path::Rect {
                x: -10.0,
                y: -10.0,
                width: 20.0,
                height: 20.0,
            }
        );

        let counter_clockwise = arc_path(0.0, 10.0, 0.0, -2.0 * PI);
        assert_eq!(
            counter_clockwise,
            "M0,-10A10,10,0,1,0,0,10A10,10,0,1,0,0,-10Z"
        );
        assert_eq!(
            ferrigura_path::bounding_box(&counter_clockwise),
            ferrigura_path::Rect {
                x: -10.0,
                y: -10.0,
                width: 20.0,
                height: 20.0,
            }
        );
    }

    #[test]
    fn a_full_donut_has_two_half_arcs_on_each_ring() {
        let clockwise = arc_path(5.0, 10.0, 0.0, 2.0 * PI);
        assert_eq!(
            clockwise,
            "M0,-10A10,10,0,1,1,0,10A10,10,0,1,1,0,-10M0,-5A5,5,0,1,0,0,5A5,5,0,1,0,0,-5Z"
        );
        assert_eq!(
            ferrigura_path::bounding_box(&clockwise),
            ferrigura_path::Rect {
                x: -10.0,
                y: -10.0,
                width: 20.0,
                height: 20.0,
            }
        );
        let inner = &clockwise[clockwise.rfind('M').expect("inner subpath")..];
        assert_eq!(
            ferrigura_path::bounding_box(inner),
            ferrigura_path::Rect {
                x: -5.0,
                y: -5.0,
                width: 10.0,
                height: 10.0,
            }
        );

        let counter_clockwise = arc_path(5.0, 10.0, 0.0, -2.0 * PI);
        assert_eq!(
            counter_clockwise,
            "M0,-10A10,10,0,1,0,0,10A10,10,0,1,0,0,-10M0,-5A5,5,0,1,1,0,5A5,5,0,1,1,0,-5Z"
        );
        assert_eq!(
            ferrigura_path::bounding_box(&counter_clockwise),
            ferrigura_path::Rect {
                x: -10.0,
                y: -10.0,
                width: 20.0,
                height: 20.0,
            }
        );
        let inner_ccw = &counter_clockwise[counter_clockwise.rfind('M').expect("inner subpath")..];
        assert_eq!(
            ferrigura_path::bounding_box(inner_ccw),
            ferrigura_path::Rect {
                x: -5.0,
                y: -5.0,
                width: 10.0,
                height: 10.0,
            }
        );

        // Matches `d3-shape`: the inner ring starts at `endAngle`, even when `da > 2π`.
        assert_eq!(
            arc_path(5.0, 10.0, 0.0, 3.0 * PI),
            "M0,-10A10,10,0,1,1,0,10A10,10,0,1,1,0,-10M0,5A5,5,0,1,0,0,-5A5,5,0,1,0,0,5Z"
        );
    }
}
