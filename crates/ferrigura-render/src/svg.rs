//! SVG text helpers: escaping, numbers, the paths of arc segments and the embedded font faces.

use std::f64::consts::PI;

use ferrigura_jsmath::{cos, sin};
use ferrigura_text::EmbeddedFamily;

use crate::common::d3::d3_number;

/// Escapes text for an XML element or attribute value.
#[must_use]
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

/// The XLink namespace, which the `xlink:href` of a link or an image is in.
pub const XLINK_NAMESPACE: &str = "http://www.w3.org/1999/xlink";

/// The declaration of the `xlink` prefix that a root needs when the drawing writes `xlink:href`,
/// with a space before it, or nothing. Without it the prefix is unbound, and an XML parser rejects
/// the SVG. The reference declares it after `xmlns` on the root that `mermaid.run` draws
/// (`appendDivSvgG` with `XMLNS_XLINK_STD`, `mermaidAPI.ts:414-416`); the root of `mermaid.render`
/// has none, and the HTML serialisation writes `xlink:href` without it.
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

/// A number as the reference writes it in an attribute: JavaScript's `String(number)`, written by
/// [`ferrigura_model::value::number_text`], which the theme tokens read with.
#[must_use]
pub fn number(value: f64) -> String {
    ferrigura_model::value::number_text(value)
}

/// The `@font-face` rules of the embedded families, in the order given.
#[must_use]
pub fn font_faces(families: &[EmbeddedFamily]) -> String {
    families
        .iter()
        .map(|family| family.css.as_str())
        .collect::<Vec<_>>()
        .join("\n")
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
        let middle = a0 + if clockwise { PI } else { -PI };
        let (x0, y0) = point(outer, a0);
        let (xm, ym) = point(outer, middle);
        let sweep = u8::from(clockwise);
        path.push_str(&format!(
            "M{},{}A{},{},0,1,{},{},{},{}A{},{},0,1,{},{},{},{}",
            d3_number(x0),
            d3_number(y0),
            d3_number(outer),
            d3_number(outer),
            sweep,
            d3_number(xm),
            d3_number(ym),
            d3_number(outer),
            d3_number(outer),
            sweep,
            d3_number(x0),
            d3_number(y0),
            d3_number(outer),
            d3_number(outer),
        ));
        if inner > EPSILON {
            let (xi, yi) = point(inner, a0);
            let (xim, yim) = point(inner, middle);
            let back = 1 - sweep;
            path.push_str(&format!(
                "M{},{}A{},{},0,1,{},{},{},{}A{},{},0,1,{},{},{},{}Z",
                d3_number(xi),
                d3_number(yi),
                d3_number(inner),
                d3_number(inner),
                back,
                d3_number(xim),
                d3_number(yim),
                d3_number(inner),
                d3_number(inner),
                back,
                d3_number(xi),
                d3_number(yi),
                d3_number(inner),
                d3_number(inner),
            ));
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
    fn escapes_markup_and_quotes() {
        assert_eq!(
            escape("a<b & \"c\" 'd'"),
            "a&lt;b &amp; &quot;c&quot; &#39;d&#39;"
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
        let path = arc_path(0.0, 10.0, 0.0, 2.0 * PI);
        assert_eq!(path.matches('A').count(), 2, "{path}");
    }
}
