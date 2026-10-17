//! `blockArrow.ts`: the arrow of a block diagram (`<["text"]>(right)`), a polygon whose outline
//! depends on the set of directions it points to, with the label in it. A port of
//! `rendering-util/rendering-elements/shapes/blockArrow.ts`, with its sixteen point tables.
//!
//! The directions are expanded (`x` is left and right, `y` is up and down), written in the order
//! right, left, up, down, and looked up as a key (`right|left|up|down`, …). A key with no table, or no
//! direction, is the single point at the origin.

use super::util::{Part, label_helper, polygon_shape, styles_from_array};
use super::{Intersect, ShapeInput};
use crate::unified::geom::{BBox, Point};

/// The directions in the order the key is written (`DIRECTION_ORDER`).
const ORDER: [&str; 4] = ["right", "left", "up", "down"];

/// The metrics a table reads (`ArrowMetrics`).
#[derive(Clone, Copy)]
struct Metrics {
    height: f64,
    midpoint: f64,
    padding: f64,
    width: f64,
}

/// `getArrowPoints` (`blockArrow.ts` 183-198): the outline of a block arrow with the given directions,
/// in the node's coordinates, the bottom-left corner of the arrow at the origin.
///
/// `padding` is the node's padding over two; `total_width` is the width the arrow is drawn at.
#[must_use]
pub(super) fn arrow_points(
    directions: &[String],
    bbox: BBox,
    node_padding: f64,
    total_width: f64,
) -> Vec<Point> {
    let padding = node_padding / 2.0;
    let height = bbox.height + 4.0 * padding;
    let midpoint = height / 2.0;
    let metrics = Metrics {
        height,
        midpoint,
        padding,
        width: total_width,
    };
    let key = direction_key(directions);
    table(&key).map_or_else(|| point(metrics), |factory| factory(metrics))
}

/// The directions of a list, expanded and deduplicated, in the key's order, joined with `|`; `point` for
/// no direction (`expandAndDeduplicateDirections` and `getDirectionKey`).
fn direction_key(directions: &[String]) -> String {
    let mut present = [false; 4];
    for direction in directions {
        match direction.as_str() {
            "x" => {
                present[0] = true;
                present[1] = true;
            }
            "y" => {
                present[2] = true;
                present[3] = true;
            }
            other => {
                if let Some(index) = ORDER.iter().position(|name| *name == other) {
                    present[index] = true;
                }
            }
        }
    }
    let key: Vec<&str> = ORDER
        .iter()
        .zip(present)
        .filter(|(_, set)| *set)
        .map(|(name, _)| *name)
        .collect();
    if key.is_empty() {
        "point".to_owned()
    } else {
        key.join("|")
    }
}

/// The point table of a key, if the key has one.
fn table(key: &str) -> Option<fn(Metrics) -> Vec<Point>> {
    Some(match key {
        "right|left|up|down" => all_four,
        "right|left|up" => right_left_up,
        "right|left|down" => right_left_down,
        "right|up|down" => right_up_down,
        "left|up|down" => left_up_down,
        "right|left" => right_left,
        "up|down" => up_down,
        "right|up" => right_up,
        "right|down" => right_down,
        "left|up" => left_up,
        "left|down" => left_down,
        "right" => right,
        "left" => left,
        "up" => up,
        "down" => down,
        _ => return None,
    })
}

fn point(_: Metrics) -> Vec<Point> {
    vec![Point::new(0.0, 0.0)]
}

fn all_four(
    Metrics {
        height,
        midpoint,
        padding,
        width,
    }: Metrics,
) -> Vec<Point> {
    vec![
        Point::new(0.0, 0.0),
        Point::new(midpoint, 0.0),
        Point::new(width / 2.0, 2.0 * padding),
        Point::new(width - midpoint, 0.0),
        Point::new(width, 0.0),
        Point::new(width, -height / 3.0),
        Point::new(width + 2.0 * padding, -height / 2.0),
        Point::new(width, (-2.0 * height) / 3.0),
        Point::new(width, -height),
        Point::new(width - midpoint, -height),
        Point::new(width / 2.0, -height - 2.0 * padding),
        Point::new(midpoint, -height),
        Point::new(0.0, -height),
        Point::new(0.0, (-2.0 * height) / 3.0),
        Point::new(-2.0 * padding, -height / 2.0),
        Point::new(0.0, -height / 3.0),
    ]
}

fn right_left_up(
    Metrics {
        height,
        midpoint,
        width,
        ..
    }: Metrics,
) -> Vec<Point> {
    vec![
        Point::new(midpoint, 0.0),
        Point::new(width - midpoint, 0.0),
        Point::new(width, -height / 2.0),
        Point::new(width - midpoint, -height),
        Point::new(midpoint, -height),
        Point::new(0.0, -height / 2.0),
    ]
}

fn right_left_down(
    Metrics {
        height,
        midpoint,
        width,
        ..
    }: Metrics,
) -> Vec<Point> {
    vec![
        Point::new(0.0, 0.0),
        Point::new(midpoint, -height),
        Point::new(width - midpoint, -height),
        Point::new(width, 0.0),
    ]
}

fn right_up_down(
    Metrics {
        height,
        midpoint,
        width,
        ..
    }: Metrics,
) -> Vec<Point> {
    vec![
        Point::new(0.0, 0.0),
        Point::new(width, -midpoint),
        Point::new(width, -height + midpoint),
        Point::new(0.0, -height),
    ]
}

fn left_up_down(
    Metrics {
        height,
        midpoint,
        width,
        ..
    }: Metrics,
) -> Vec<Point> {
    vec![
        Point::new(width, 0.0),
        Point::new(0.0, -midpoint),
        Point::new(0.0, -height + midpoint),
        Point::new(width, -height),
    ]
}

fn right_left(
    Metrics {
        height,
        midpoint,
        padding,
        width,
    }: Metrics,
) -> Vec<Point> {
    vec![
        Point::new(midpoint, 0.0),
        Point::new(midpoint, -padding),
        Point::new(width - midpoint, -padding),
        Point::new(width - midpoint, 0.0),
        Point::new(width, -height / 2.0),
        Point::new(width - midpoint, -height),
        Point::new(width - midpoint, -height + padding),
        Point::new(midpoint, -height + padding),
        Point::new(midpoint, -height),
        Point::new(0.0, -height / 2.0),
    ]
}

fn up_down(
    Metrics {
        height,
        midpoint,
        padding,
        width,
    }: Metrics,
) -> Vec<Point> {
    vec![
        Point::new(width / 2.0, 0.0),
        Point::new(0.0, -padding),
        Point::new(midpoint, -padding),
        Point::new(midpoint, -height + padding),
        Point::new(0.0, -height + padding),
        Point::new(width / 2.0, -height),
        Point::new(width, -height + padding),
        Point::new(width - midpoint, -height + padding),
        Point::new(width - midpoint, -padding),
        Point::new(width, -padding),
    ]
}

fn right_up(
    Metrics {
        height,
        midpoint,
        width,
        ..
    }: Metrics,
) -> Vec<Point> {
    vec![
        Point::new(0.0, 0.0),
        Point::new(width, -midpoint),
        Point::new(0.0, -height),
    ]
}

fn right_down(Metrics { height, width, .. }: Metrics) -> Vec<Point> {
    vec![
        Point::new(0.0, 0.0),
        Point::new(width, 0.0),
        Point::new(0.0, -height),
    ]
}

fn left_up(
    Metrics {
        height,
        midpoint,
        width,
        ..
    }: Metrics,
) -> Vec<Point> {
    vec![
        Point::new(width, 0.0),
        Point::new(0.0, -midpoint),
        Point::new(width, -height),
    ]
}

fn left_down(Metrics { height, width, .. }: Metrics) -> Vec<Point> {
    vec![
        Point::new(width, 0.0),
        Point::new(0.0, 0.0),
        Point::new(width, -height),
    ]
}

fn right(
    Metrics {
        height,
        midpoint,
        padding,
        width,
    }: Metrics,
) -> Vec<Point> {
    vec![
        Point::new(midpoint, -padding),
        Point::new(midpoint, -padding),
        Point::new(width - midpoint, -padding),
        Point::new(width - midpoint, 0.0),
        Point::new(width, -height / 2.0),
        Point::new(width - midpoint, -height),
        Point::new(width - midpoint, -height + padding),
        Point::new(midpoint, -height + padding),
        Point::new(midpoint, -height + padding),
    ]
}

fn left(
    Metrics {
        height,
        midpoint,
        padding,
        width,
    }: Metrics,
) -> Vec<Point> {
    vec![
        Point::new(midpoint, 0.0),
        Point::new(midpoint, -padding),
        Point::new(width - midpoint, -padding),
        Point::new(width - midpoint, -height + padding),
        Point::new(midpoint, -height + padding),
        Point::new(midpoint, -height),
        Point::new(0.0, -height / 2.0),
    ]
}

fn up(
    Metrics {
        height,
        midpoint,
        padding,
        width,
    }: Metrics,
) -> Vec<Point> {
    vec![
        Point::new(midpoint, -padding),
        Point::new(midpoint, -height + padding),
        Point::new(0.0, -height + padding),
        Point::new(width / 2.0, -height),
        Point::new(width, -height + padding),
        Point::new(width - midpoint, -height + padding),
        Point::new(width - midpoint, -padding),
    ]
}

fn down(
    Metrics {
        height,
        midpoint,
        padding,
        width,
    }: Metrics,
) -> Vec<Point> {
    vec![
        Point::new(width / 2.0, 0.0),
        Point::new(0.0, -padding),
        Point::new(midpoint, -padding),
        Point::new(midpoint, -height + padding),
        Point::new(width - midpoint, -height + padding),
        Point::new(width - midpoint, -padding),
        Point::new(width, -padding),
    ]
}

/// `block_arrow` (`blockArrow.ts` 200-217): the label, and the arrow's outline at the node's width when
/// the node is placed, spans its columns and is wider than the label needs; else at the natural width.
pub(super) fn block_arrow(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let label = label_helper(input, false);
    let padding = node.padding;
    let height = label.bbox.height + 2.0 * padding;
    let midpoint = height / 2.0;
    let natural_width = label.bbox.width + 2.0 * midpoint + padding;
    let node_width = node.width.unwrap_or(0.0);
    let fields = node.block.clone().unwrap_or_default();
    let is_spanning =
        fields.positioned && fields.width_in_columns > 1.0 && node_width > natural_width;
    let width = if is_spanning {
        node_width
    } else {
        natural_width
    };
    let points = arrow_points(&fields.directions, label.bbox, padding, width);
    // `blockArrow.attr('style', blockNode.style)`: the declarations that are not label styles, as
    // `getStylesFromArray` joins them.
    let (style, _) = styles_from_array(&node.css_styles);
    let style = (!style.is_empty()).then_some(style.as_str());
    let (svg, outline) = polygon_shape(&points, width, height, style);
    Part::with_centred_label(svg, outline, Intersect::Polygon { points }, label)
}

#[cfg(test)]
mod tests {
    //! `blockArrow.spec.ts`: the outline of each direction set, for a 100 by 20 label and a padding of 10
    //! (5 each side), drawn at 140 wide. The expected points are the reference's expressions.

    use super::*;

    const WIDTH: f64 = 140.0;
    const HEIGHT: f64 = 40.0;
    const MIDPOINT: f64 = 20.0;
    const PADDING: f64 = 5.0;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn dirs(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    fn bbox() -> BBox {
        BBox::new(0.0, 0.0, 100.0, 20.0)
    }

    fn points(names: &[&str]) -> Vec<Point> {
        arrow_points(&dirs(names), bbox(), 10.0, WIDTH)
    }

    fn all_directions() -> Vec<Point> {
        vec![
            p(0.0, 0.0),
            p(MIDPOINT, 0.0),
            p(WIDTH / 2.0, 2.0 * PADDING),
            p(WIDTH - MIDPOINT, 0.0),
            p(WIDTH, 0.0),
            p(WIDTH, -HEIGHT / 3.0),
            p(WIDTH + 2.0 * PADDING, -HEIGHT / 2.0),
            p(WIDTH, (-2.0 * HEIGHT) / 3.0),
            p(WIDTH, -HEIGHT),
            p(WIDTH - MIDPOINT, -HEIGHT),
            p(WIDTH / 2.0, -HEIGHT - 2.0 * PADDING),
            p(MIDPOINT, -HEIGHT),
            p(0.0, -HEIGHT),
            p(0.0, (-2.0 * HEIGHT) / 3.0),
            p(-2.0 * PADDING, -HEIGHT / 2.0),
            p(0.0, -HEIGHT / 3.0),
        ]
    }

    #[test]
    fn four_way_arrow() {
        assert_eq!(points(&["right", "left", "up", "down"]), all_directions());
    }

    #[test]
    fn right_left_up_arrow() {
        assert_eq!(
            points(&["right", "left", "up"]),
            vec![
                p(MIDPOINT, 0.0),
                p(WIDTH - MIDPOINT, 0.0),
                p(WIDTH, -HEIGHT / 2.0),
                p(WIDTH - MIDPOINT, -HEIGHT),
                p(MIDPOINT, -HEIGHT),
                p(0.0, -HEIGHT / 2.0),
            ]
        );
    }

    #[test]
    fn right_left_down_arrow() {
        assert_eq!(
            points(&["right", "left", "down"]),
            vec![
                p(0.0, 0.0),
                p(MIDPOINT, -HEIGHT),
                p(WIDTH - MIDPOINT, -HEIGHT),
                p(WIDTH, 0.0),
            ]
        );
    }

    #[test]
    fn right_up_down_arrow() {
        assert_eq!(
            points(&["right", "up", "down"]),
            vec![
                p(0.0, 0.0),
                p(WIDTH, -MIDPOINT),
                p(WIDTH, -HEIGHT + MIDPOINT),
                p(0.0, -HEIGHT),
            ]
        );
    }

    #[test]
    fn left_up_down_arrow() {
        assert_eq!(
            points(&["left", "up", "down"]),
            vec![
                p(WIDTH, 0.0),
                p(0.0, -MIDPOINT),
                p(0.0, -HEIGHT + MIDPOINT),
                p(WIDTH, -HEIGHT),
            ]
        );
    }

    #[test]
    fn horizontal_arrow() {
        assert_eq!(
            points(&["right", "left"]),
            vec![
                p(MIDPOINT, 0.0),
                p(MIDPOINT, -PADDING),
                p(WIDTH - MIDPOINT, -PADDING),
                p(WIDTH - MIDPOINT, 0.0),
                p(WIDTH, -HEIGHT / 2.0),
                p(WIDTH - MIDPOINT, -HEIGHT),
                p(WIDTH - MIDPOINT, -HEIGHT + PADDING),
                p(MIDPOINT, -HEIGHT + PADDING),
                p(MIDPOINT, -HEIGHT),
                p(0.0, -HEIGHT / 2.0),
            ]
        );
    }

    #[test]
    fn vertical_arrow() {
        assert_eq!(
            points(&["up", "down"]),
            vec![
                p(WIDTH / 2.0, 0.0),
                p(0.0, -PADDING),
                p(MIDPOINT, -PADDING),
                p(MIDPOINT, -HEIGHT + PADDING),
                p(0.0, -HEIGHT + PADDING),
                p(WIDTH / 2.0, -HEIGHT),
                p(WIDTH, -HEIGHT + PADDING),
                p(WIDTH - MIDPOINT, -HEIGHT + PADDING),
                p(WIDTH - MIDPOINT, -PADDING),
                p(WIDTH, -PADDING),
            ]
        );
    }

    #[test]
    fn right_up_angle() {
        assert_eq!(
            points(&["right", "up"]),
            vec![p(0.0, 0.0), p(WIDTH, -MIDPOINT), p(0.0, -HEIGHT)]
        );
    }

    #[test]
    fn right_down_angle() {
        assert_eq!(
            points(&["right", "down"]),
            vec![p(0.0, 0.0), p(WIDTH, 0.0), p(0.0, -HEIGHT)]
        );
    }

    #[test]
    fn left_up_angle() {
        assert_eq!(
            points(&["left", "up"]),
            vec![p(WIDTH, 0.0), p(0.0, -MIDPOINT), p(WIDTH, -HEIGHT)]
        );
    }

    #[test]
    fn left_down_angle() {
        assert_eq!(
            points(&["left", "down"]),
            vec![p(WIDTH, 0.0), p(0.0, 0.0), p(WIDTH, -HEIGHT)]
        );
    }

    #[test]
    fn right_arrow() {
        assert_eq!(
            points(&["right"]),
            vec![
                p(MIDPOINT, -PADDING),
                p(MIDPOINT, -PADDING),
                p(WIDTH - MIDPOINT, -PADDING),
                p(WIDTH - MIDPOINT, 0.0),
                p(WIDTH, -HEIGHT / 2.0),
                p(WIDTH - MIDPOINT, -HEIGHT),
                p(WIDTH - MIDPOINT, -HEIGHT + PADDING),
                p(MIDPOINT, -HEIGHT + PADDING),
                p(MIDPOINT, -HEIGHT + PADDING),
            ]
        );
    }

    #[test]
    fn left_arrow() {
        assert_eq!(
            points(&["left"]),
            vec![
                p(MIDPOINT, 0.0),
                p(MIDPOINT, -PADDING),
                p(WIDTH - MIDPOINT, -PADDING),
                p(WIDTH - MIDPOINT, -HEIGHT + PADDING),
                p(MIDPOINT, -HEIGHT + PADDING),
                p(MIDPOINT, -HEIGHT),
                p(0.0, -HEIGHT / 2.0),
            ]
        );
    }

    #[test]
    fn up_arrow() {
        assert_eq!(
            points(&["up"]),
            vec![
                p(MIDPOINT, -PADDING),
                p(MIDPOINT, -HEIGHT + PADDING),
                p(0.0, -HEIGHT + PADDING),
                p(WIDTH / 2.0, -HEIGHT),
                p(WIDTH, -HEIGHT + PADDING),
                p(WIDTH - MIDPOINT, -HEIGHT + PADDING),
                p(WIDTH - MIDPOINT, -PADDING),
            ]
        );
    }

    #[test]
    fn down_arrow() {
        assert_eq!(
            points(&["down"]),
            vec![
                p(WIDTH / 2.0, 0.0),
                p(0.0, -PADDING),
                p(MIDPOINT, -PADDING),
                p(MIDPOINT, -HEIGHT + PADDING),
                p(WIDTH - MIDPOINT, -HEIGHT + PADDING),
                p(WIDTH - MIDPOINT, -PADDING),
                p(WIDTH, -PADDING),
            ]
        );
    }

    #[test]
    fn point_fallback_for_no_direction() {
        assert_eq!(arrow_points(&[], bbox(), 10.0, WIDTH), vec![p(0.0, 0.0)]);
    }

    #[test]
    fn expands_axis_shorthands_and_deduplicates_directions() {
        assert_eq!(points(&["x", "right", "y", "up"]), all_directions());
    }

    #[test]
    fn a_direction_with_a_blank_is_not_a_direction() {
        // The grammar keeps the blanks of a direction as written (B18): ` right` has no table.
        assert_eq!(points(&[" right"]), vec![p(0.0, 0.0)]);
    }
}
