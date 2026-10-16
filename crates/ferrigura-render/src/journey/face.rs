//! The score face of a task (`svgDraw.js` 8-95): a circle with two eyes and a mouth. A score above
//! 3 is a smile, below 3 a frown, and 3 (or a score that is not a number) a straight line.
//!
//! The face is a circle and then a group with the eyes and the mouth, as the reference appends them.

use std::f64::consts::PI;

use crate::common::Element;
use crate::svg::{arc_path, number};

/// The radius of the face (`svgDraw.js` 9).
const RADIUS: f64 = 15.0;

/// The face at the centre `(cx, cy)` for a score: the circle, then its group.
#[must_use]
pub fn draw(cx: f64, cy: f64, score: f64) -> Vec<Element> {
    let circle = Element::new("circle")
        .with("cx", number(cx))
        .with("cy", number(cy))
        .with("class", "face")
        .with("r", number(RADIUS))
        .with("stroke-width", "2")
        .with("overflow", "visible");
    let mut group = Element::new("g")
        .child(eye(cx - RADIUS / 3.0, cy - RADIUS / 3.0))
        .child(eye(cx + RADIUS / 3.0, cy - RADIUS / 3.0));
    // The comparisons are false for a score that is not a number, so it is a straight mouth,
    // as in the reference (`svgDraw.js` 86-92).
    if score > 3.0 {
        group = group.child(mouth(cx, cy + 2.0, PI / 2.0, 3.0 * (PI / 2.0)));
    } else if score < 3.0 {
        group = group.child(mouth(cx, cy + 7.0, 3.0 * (PI / 2.0), 5.0 * (PI / 2.0)));
    } else {
        group = group.child(
            Element::new("line")
                .with("class", "mouth")
                .with("stroke", "#666")
                .with("x1", number(cx - 5.0))
                .with("y1", number(cy + 7.0))
                .with("x2", number(cx + 5.0))
                .with("y2", number(cy + 7.0))
                .with("stroke-width", "1px"),
        );
    }
    vec![circle, group]
}

/// An eye: a dot of radius 1.5, filled and stroked in the mouth's grey.
fn eye(cx: f64, cy: f64) -> Element {
    Element::new("circle")
        .with("cx", number(cx))
        .with("cy", number(cy))
        .with("r", "1.5")
        .with("stroke-width", "2")
        .with("fill", "#666")
        .with("stroke", "#666")
}

/// `d3-path`'s `pathRound(3)`: every number of a path rounded to three decimals (`Math.round(x *
/// 1000) / 1000`), as d3 writes the path of an arc. The reference's mouth is `M7.5,0A7.5,7.5,0,1,1,
/// -7.5,0L-6.818,0A6.818,6.818,0,1,0,6.818,0Z`, with the radius 6.818 and not 6.8181818....
fn round_path(d: &str) -> String {
    fn flush(token: &mut String, out: &mut String) {
        if token.is_empty() {
            return;
        }
        match token.parse::<f64>() {
            Ok(value) => out.push_str(&number((value * 1000.0).round() / 1000.0)),
            Err(_) => out.push_str(token),
        }
        token.clear();
    }
    let mut out = String::with_capacity(d.len());
    let mut token = String::new();
    for character in d.chars() {
        if character.is_ascii_digit() || character == '.' || (character == '-' && token.is_empty())
        {
            token.push(character);
        } else {
            flush(&mut token, &mut out);
            out.push(character);
        }
    }
    flush(&mut token, &mut out);
    out
}

/// A mouth: the arc between two angles of the face's radius, from `radius / 2.2` to `radius / 2`
/// (`svgDraw.js` 42-69), placed at `(cx, cy)`. `d3-shape` swaps the radii when the inner one is the
/// larger, which is the case here, so the inner radius of the path is the smaller one.
fn mouth(cx: f64, cy: f64, start: f64, end: f64) -> Element {
    Element::new("path")
        .with("class", "mouth")
        .with(
            "d",
            round_path(&arc_path(RADIUS / 2.2, RADIUS / 2.0, start, end)),
        )
        .with(
            "transform",
            format!("translate({},{})", number(cx), number(cy)),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reference's smile at score 5 (`NOTES.md` 7.5.1), which d3 writes with three decimals;
    /// the geometry compares the path's samples, so the digits of the radius are not compared.
    #[test]
    fn a_smile_is_the_arc_from_the_right_to_the_left_below_the_centre() {
        let group = &draw(225.0, 300.0, 5.0)[1];
        let mouth = &group.children()[2];
        assert_eq!(mouth.get("class"), Some("mouth"));
        assert_eq!(mouth.get("transform"), Some("translate(225,302)"));
        assert_eq!(
            mouth.get("d"),
            Some("M7.5,0A7.5,7.5,0,1,1,-7.5,0L-6.818,0A6.818,6.818,0,1,0,6.818,0Z")
        );
    }

    #[test]
    fn a_frown_and_a_straight_mouth_are_chosen_by_the_score() {
        let sad = &draw(0.0, 0.0, 1.0)[1];
        assert_eq!(sad.children()[2].get("transform"), Some("translate(0,7)"));
        let straight = &draw(0.0, 0.0, 3.0)[1];
        assert!(straight.children()[2].render().starts_with("<line "));
        let not_a_number = &draw(0.0, 0.0, f64::NAN)[1];
        assert!(not_a_number.children()[2].render().starts_with("<line "));
    }

    #[test]
    fn a_score_that_is_not_finite_writes_its_position_as_javascript_does() {
        let face = draw(75.0, f64::NAN, f64::NAN);
        assert!(face[0].render().contains("cy=\"NaN\""));
    }
}
