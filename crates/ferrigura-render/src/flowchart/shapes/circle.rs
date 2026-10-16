//! The circles: `circle` and `doublecircle`. Ports of `shapes/circle.ts` and `shapes/doubleCircle.ts`.
//! Both size themselves from the label's diagonal, not from the minimum node width
//! (`ignoreMinWidth`). See `docs-port/flowchart/render-notes/shapes-core.md` §2.1.

use super::util::{Part, attribute, label_helper, styles2string};
use super::{Intersect, ShapeInput};
use crate::flowchart::geom::BBox;
use crate::svg::number;

/// `circle.ts`: a circle around the label. Its radius is half the label's diagonal plus 16 in the
/// neo look, and plus half the padding in the classic look.
pub(super) fn circle(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let label = label_helper(input, true);
    let label_radius = (label.bbox.width.powi(2) + label.bbox.height.powi(2)).sqrt() / 2.0;
    let radius = if node.is_neo() {
        label_radius + 16.0
    } else {
        label_radius + node.padding / 2.0
    };
    let svg = format!(
        "<circle class=\"basic label-container\"{} r=\"{}\" cx=\"0\" cy=\"0\"/>",
        attribute("style", &styles.node),
        number(radius),
    );
    let bbox = BBox::new(-radius, -radius, 2.0 * radius, 2.0 * radius);
    Part::with_centred_label(svg, bbox, Intersect::Circle { radius }, label)
}

/// `doubleCircle.ts`: two concentric circles around the label. The inner ring is the label's
/// diagonal plus 16 in neo and plus the padding in classic; the outer ring is 12 (neo) or 5
/// (classic) further out.
pub(super) fn double_circle(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let (gap, label_padding) = if node.is_neo() {
        (12.0, 16.0)
    } else {
        (5.0, node.padding)
    };
    let label = label_helper(input, true);
    let label_radius = (label.bbox.width.powi(2) + label.bbox.height.powi(2)).sqrt() / 2.0;
    let inner_radius = label_radius + label_padding;
    let outer_radius = inner_radius + gap;
    let style = attribute("style", &styles.node);
    let svg = format!(
        "<g class=\"basic label-container\"{style}><circle class=\"outer-circle\"{style} r=\"{}\" cx=\"0\" cy=\"0\"/><circle class=\"inner-circle\"{style} r=\"{}\" cx=\"0\" cy=\"0\"/></g>",
        number(outer_radius),
        number(inner_radius),
    );
    let bbox = BBox::new(
        -outer_radius,
        -outer_radius,
        2.0 * outer_radius,
        2.0 * outer_radius,
    );
    Part::with_centred_label(
        svg,
        bbox,
        Intersect::Circle {
            radius: outer_radius,
        },
        label,
    )
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{draw_box, node};
    use super::*;
    use crate::flowchart::label::LabelLayout;
    use crate::flowchart::shapes::ShapeOutput;

    /// The radius of the circle or the ring with the given class in the markup.
    fn radius_of(svg: &str, class: &str) -> f64 {
        let marker = format!("class=\"{class}\"");
        let start = svg
            .find(&marker)
            .unwrap_or_else(|| panic!("no {class} in {svg}"));
        let rest = &svg[start..];
        let r = rest.find("r=\"").expect("a radius") + 3;
        let end = rest[r..].find('"').expect("the end of the radius");
        rest[r..r + end].parse().expect("a number")
    }

    fn labelled(shape: &str, look: &str, width: f64, height: f64) -> ShapeOutput {
        let mut node = node(shape, look);
        node.padding = 16.0;
        node.label = "a".to_owned();
        draw_box(
            &node,
            &LabelLayout::of_box(BBox::new(-width / 2.0, 0.0, width, height)),
        )
    }

    // util.spec.ts, "circle shapes and minNodeWidth": the label box is empty here, so only the
    // padding and the floor of the minimum width could size the circle.

    #[test]
    fn sizes_a_circle_from_its_label_not_the_minimum_node_width() {
        let shape = labelled("circle", "classic", 0.0, 0.0);
        assert!(
            (radius_of(&shape.svg, "basic label-container") - 8.0).abs() < 1e-9,
            "{}",
            shape.svg
        );
    }

    #[test]
    fn pads_a_neo_circle_like_the_inner_ring_of_a_neo_double_circle() {
        let shape = labelled("circle", "neo", 0.0, 0.0);
        assert!(
            (radius_of(&shape.svg, "basic label-container") - 16.0).abs() < 1e-9,
            "{}",
            shape.svg
        );
    }

    #[test]
    fn sizes_a_double_circle_from_its_label_not_the_minimum_node_width() {
        let shape = labelled("doublecircle", "classic", 0.0, 0.0);
        assert!(
            (radius_of(&shape.svg, "inner-circle") - 16.0).abs() < 1e-9,
            "{}",
            shape.svg
        );
        assert!(
            (radius_of(&shape.svg, "outer-circle") - 21.0).abs() < 1e-9,
            "{}",
            shape.svg
        );
    }

    // labelContainment.spec.ts, "label containment": the circles fit the label's corners.

    const LABEL_BOXES: [(f64, f64); 3] = [(200.0, 24.0), (120.0, 110.0), (120.0, 240.0)];

    #[test]
    fn fits_inside_the_circle() {
        for look in ["neo", "classic"] {
            for (width, height) in LABEL_BOXES {
                let shape = labelled("circle", look, width, height);
                let r = radius_of(&shape.svg, "basic label-container");
                assert!(
                    r >= (width * width + height * height).sqrt() / 2.0,
                    "{look} {width}x{height}"
                );
            }
        }
    }

    #[test]
    fn fits_inside_the_double_circle() {
        for look in ["neo", "classic"] {
            for (width, height) in LABEL_BOXES {
                let shape = labelled("doublecircle", look, width, height);
                let inner = radius_of(&shape.svg, "inner-circle");
                assert!(
                    inner >= (width * width + height * height).sqrt() / 2.0,
                    "{look} {width}x{height}"
                );
            }
        }
    }
}
