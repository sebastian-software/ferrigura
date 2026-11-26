//! `blockArrow.ts`: the arrow shape. A port of `rendering-util/rendering-elements/shapes/blockArrow.ts`,
//! which Mermaid draws the arrows of a block diagram with; a flowchart node can name it too.
//!
//! The outline of the arrow follows the directions it points to, which only a block of a block
//! diagram has. A flowchart node has none, and a key with no direction is the single point at the
//! origin (`getArrowPoints`).

use super::util::{Part, label_helper, polygon_shape, styles_from_array};
use super::{Intersect, ShapeInput};
use crate::unified::geom::Point;

/// `block_arrow` (`blockArrow.ts` 200-217): the label, and the arrow's outline at the natural width of
/// the label, which for a node without directions is the point at the origin.
pub(super) fn block_arrow(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let label = label_helper(input, false);
    let padding = node.padding;
    let height = label.bbox.height + 2.0 * padding;
    let midpoint = height / 2.0;
    let width = label.bbox.width + 2.0 * midpoint + padding;
    let points = vec![Point::new(0.0, 0.0)];
    // `blockArrow.attr('style', blockNode.style)`: the declarations that are not label styles, as
    // `getStylesFromArray` joins them.
    let (style, _) = styles_from_array(&node.css_styles);
    let style = (!style.is_empty()).then_some(style.as_str());
    let (svg, outline) = polygon_shape(&points, width, height, style);
    Part::with_centered_label(svg, outline, Intersect::Polygon { points }, label)
}
