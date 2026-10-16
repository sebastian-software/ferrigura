//! SVG text helpers: escaping, numbers, the paths of arc segments and the embedded font faces.

use std::f64::consts::PI;

use ferrigura_text::EmbeddedFamily;

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

/// A number as SVG writes it. Rust prints the shortest text that reads back to the same value,
/// without an exponent for the sizes that diagrams use.
#[must_use]
pub fn number(value: f64) -> String {
    if value.abs() < 1e-10 {
        return "0".to_owned();
    }
    format!("{value}")
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
    let point = |radius: f64, angle: f64| (radius * angle.cos(), radius * angle.sin());

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
            number(x0),
            number(y0),
            number(outer),
            number(outer),
            sweep,
            number(xm),
            number(ym),
            number(outer),
            number(outer),
            sweep,
            number(x0),
            number(y0),
            number(outer),
            number(outer),
        ));
        if inner > EPSILON {
            let (xi, yi) = point(inner, a0);
            let (xim, yim) = point(inner, middle);
            let back = 1 - sweep;
            path.push_str(&format!(
                "M{},{}A{},{},0,1,{},{},{},{}A{},{},0,1,{},{},{},{}Z",
                number(xi),
                number(yi),
                number(inner),
                number(inner),
                back,
                number(xim),
                number(yim),
                number(inner),
                number(inner),
                back,
                number(xi),
                number(yi),
                number(inner),
                number(inner),
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
        number(x0),
        number(y0),
        number(outer),
        number(outer),
        large,
        sweep,
        number(x1),
        number(y1),
    ));
    if inner > EPSILON {
        let (xi1, yi1) = point(inner, a1);
        let (xi0, yi0) = point(inner, a0);
        path.push_str(&format!(
            "L{},{}A{},{},0,{},{},{},{}",
            number(xi1),
            number(yi1),
            number(inner),
            number(inner),
            large,
            1 - sweep,
            number(xi0),
            number(yi0),
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

    #[test]
    fn a_full_circle_is_two_half_arcs() {
        let path = arc_path(0.0, 10.0, 0.0, 2.0 * PI);
        assert_eq!(path.matches('A').count(), 2, "{path}");
    }
}
