//! The shapes without a label: `forkJoin`, `stateStart`, `stateEnd`, `filledCircle` and `anchor`.
//! Ports of `shapes/forkJoin.ts`, `stateStart.ts`, `stateEnd.ts`, `filledCircle.ts` and
//! `anchor.ts`. The rings and bars are rough.js outlines at roughness 0 (`rough`), except the
//! start circle, which is a plain `<circle>`.

use super::rough;
use super::util::{self, Part, attribute, styles2string};
use super::{Intersect, SVG_ID_TOKEN, ShapeInput};
use crate::svg::number;
use crate::unified::geom::{BBox, Point};

/// The diameter of the state diagram's start and end circles: the node's size when it has none.
const STATE_DIAMETER: f64 = 14.0;

/// Whether a small shape takes the small drop shadow: nodes under 25 wide, in a theme with node
/// shadows, which the reference writes as an inline filter.
fn small_shadow(input: &ShapeInput<'_>, width: f64) -> Option<String> {
    (width < 25.0 && util::theme_flag(input.theme, "nodeShadow") && input.node.look != "handDrawn")
        .then(|| format!("filter:url(#{SVG_ID_TOKEN}-drop-shadow-small)"))
}

/// `forkJoin.ts`: a bar, 70 by 10 across the flow, or 10 by 70 when the flow is horizontal.
pub(super) fn fork_join(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let horizontal = matches!(input.dir, Some("LR" | "RL"));
    let (width, height) = if horizontal {
        (10.0, 70.0)
    } else {
        (70.0, 10.0)
    };
    let x = -width / 2.0;
    let y = -height / 2.0;
    let outline = rough::solid_polygon(
        &[
            Point::new(x, y),
            Point::new(x + width, y),
            Point::new(x + width, y + height),
            Point::new(x, y + height),
        ],
        false,
    );
    let svg = format!(
        "<g>{}</g>",
        rough::paths(
            &outline,
            Some(util::path_style(node, &styles).as_str()),
            &util::stroke_width(&styles),
            true,
        ),
    );
    Part::without_label(svg, BBox::new(x, y, width, height), Intersect::Rect)
}

/// `stateStart.ts`: the filled start circle of a state diagram, 14 across. It is a plain circle
/// with the class `state-start`, and no label.
pub(super) fn state_start(input: &ShapeInput<'_>) -> Part {
    let width = STATE_DIAMETER;
    let radius = width / 2.0;
    let filter =
        small_shadow(input, width).map_or_else(String::new, |filter| attribute("style", &filter));
    let svg = format!(
        "<circle class=\"state-start\" r=\"{}\" width=\"{}\" height=\"{}\"{filter}/>",
        number(radius),
        number(width),
        number(width),
    );
    Part::without_label(
        svg,
        BBox::new(-radius, -radius, width, width),
        Intersect::Circle { radius },
    )
}

/// `stateEnd.ts`: the end circle of a state diagram, a ring of 14 with a dot of 5 inside it. Both
/// are rough.js outlines at roughness 0; the outer ring's stroke is 2 wide.
pub(super) fn state_end(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let width = STATE_DIAMETER;
    let inner_diameter = width * 5.0 / 14.0;
    // The outer ring's paths take the node's declarations, and so does the inner ring's.
    let style = util::path_style(node, &styles);
    let outer = rough::ellipse(0.0, 0.0, width, width);
    let inner = rough::ellipse(0.0, 0.0, inner_diameter, inner_diameter);
    let filter =
        small_shadow(input, width).map_or_else(String::new, |filter| attribute("style", &filter));
    let svg = format!(
        "<g class=\"outer-path\"{filter}>{}<g>{}</g></g>",
        rough::paths(&outer, Some(style.as_str()), "2", true),
        rough::paths(&inner, Some(style.as_str()), "2", true),
    );
    let bbox = outer.bbox.union(inner.bbox);
    Part::without_label(
        svg,
        bbox,
        Intersect::Circle {
            radius: width / 2.0,
        },
    )
}

/// `filledCircle.ts`: a filled dot of radius 7, a rough.js circle at roughness 0. It has no
/// label.
pub(super) fn filled_circle(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let outline = rough::ellipse(0.0, 0.0, 14.0, 14.0);
    // The reference fills the dot with `nodeBorder` unless the node's declarations say
    // otherwise; the fill is the style task's, and the node's declarations are written here.
    let style = if !styles.node.is_empty() {
        Some(styles.node.clone())
    } else if !node.css_styles.is_empty() {
        Some(node.css_styles.join(","))
    } else {
        None
    };
    let svg = format!(
        "<g>{}</g>",
        rough::paths(
            &outline,
            style.as_deref(),
            &util::stroke_width(&styles),
            true
        ),
    );
    Part::without_label(svg, outline.bbox, Intersect::Circle { radius: 7.0 })
}

/// `anchor.ts`: a point of radius 1, a rough.js circle at roughness 0 with no stroke. It has no
/// label, and its group is the `anchor` class.
pub(super) fn anchor(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let outline = rough::ellipse(0.0, 0.0, 2.0, 2.0);
    let style = node.css_styles.join(",");
    let svg = format!(
        "<g class=\"anchor\"{}>{}</g>",
        attribute("style", &style),
        rough::paths(&outline, None, &util::stroke_width(&styles), false),
    );
    Part::without_label(svg, outline.bbox, Intersect::Circle { radius: 1.0 })
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{draw_box, node, theme};
    use super::*;
    use crate::unified::data::Node;
    use crate::unified::label::LabelLayout;
    use crate::unified::shapes::{ShapeInput, draw};
    use ferrigura_model::config::MermaidConfig;

    fn drawn(shape: &str) -> crate::unified::shapes::ShapeOutput {
        draw_box(&node(shape, "neo"), &LabelLayout::of_box(BBox::default()))
    }

    #[test]
    fn a_state_start_is_a_fourteen_pixel_circle_without_a_label() {
        let shape = drawn("stateStart");
        assert_eq!(shape.label, None);
        assert_eq!(shape.label_offset, None);
        assert_eq!(shape.bbox, BBox::new(-7.0, -7.0, 14.0, 14.0));
        assert!(shape.svg.contains("class=\"state-start\""), "{}", shape.svg);
        assert!(
            shape
                .svg
                .contains("filter:url(#{svg-id}-drop-shadow-small)"),
            "{}",
            shape.svg
        );
    }

    #[test]
    fn a_state_end_is_the_rough_ring_of_the_reference() {
        let shape = drawn("stateEnd");
        // The outer ring is 14.0133 wide and 14 high (shapes-core.md R5).
        assert!(
            (shape.bbox.width - 14.0133).abs() < 5e-5,
            "{:?}",
            shape.bbox
        );
        assert!((shape.bbox.height - 14.0).abs() < 1e-9, "{:?}", shape.bbox);
    }

    #[test]
    fn a_filled_circle_is_the_rough_dot_of_radius_seven() {
        let shape = drawn("f-circ");
        assert!(
            (shape.bbox.width - 14.0133).abs() < 5e-5,
            "{:?}",
            shape.bbox
        );
        assert_eq!(shape.intersect, Intersect::Circle { radius: 7.0 });
    }

    #[test]
    fn an_anchor_is_the_rough_point_of_radius_one() {
        let shape = drawn("anchor");
        assert!((shape.bbox.width - 2.0019).abs() < 5e-5, "{:?}", shape.bbox);
        assert!(
            shape.svg.starts_with("<g class=\"anchor\""),
            "{}",
            shape.svg
        );
    }

    #[test]
    fn a_fork_bar_is_seventy_by_ten_across_the_flow_and_turns_in_a_horizontal_flow() {
        let config = MermaidConfig::default();
        let theme = theme();
        let node = node("fork", "neo");
        let vertical = draw(&ShapeInput {
            node: &node,
            label: None,
            config: &config,
            theme: &theme,
            dir: Some("TB"),
        })
        .expect("known");
        assert_eq!(vertical.bbox, BBox::new(-35.0, -5.0, 70.0, 10.0));
        let horizontal = draw(&ShapeInput {
            node: &node,
            label: None,
            config: &config,
            theme: &theme,
            dir: Some("LR"),
        })
        .expect("known");
        assert_eq!(horizontal.bbox, BBox::new(-5.0, -35.0, 10.0, 70.0));
    }

    #[test]
    fn a_node_without_a_small_shadow_theme_has_no_filter() {
        let mut theme = theme();
        if let Some(object) = theme.as_object_mut() {
            object.insert(
                "nodeShadow".to_owned(),
                ferrigura_model::value::Value::Bool(false),
            );
        }
        let config = MermaidConfig::default();
        let node: Node = node("stateStart", "neo");
        let shape = draw(&ShapeInput {
            node: &node,
            label: None,
            config: &config,
            theme: &theme,
            dir: None,
        })
        .expect("known");
        assert!(!shape.svg.contains("filter"), "{}", shape.svg);
    }
}
