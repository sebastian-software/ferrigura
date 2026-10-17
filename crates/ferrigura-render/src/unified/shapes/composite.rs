//! `composite.ts`: the rectangle that frames a composite block of a block diagram, with the label of
//! the composite (its id-less text, which is empty). A port of `rendering-util/rendering-elements/
//! shapes/composite.ts`.
//!
//! The rectangle is the node's size when the node is placed (its layout gave it a width and a height),
//! and the label's box plus the node's padding when it is measured. The composite's padding is zero,
//! since `getNodeFromBlock` sets it (`renderHelpers.ts` 36-41).

use super::util::{Part, attribute, label_helper, styles_from_array};
use super::{Intersect, ShapeInput};
use crate::svg::number;
use crate::unified::geom::BBox;

/// `composite.ts`: a `rect.basic.cluster.composite.label-container`, then the label group.
pub(super) fn composite(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let label = label_helper(input, false);
    let positioned = node.block.as_ref().is_some_and(|block| block.positioned);
    // `halfPadding` is the node's padding over two; a composite has none (`getNodeFromBlock`).
    let node_padding = node.padding;
    let half_padding = node_padding / 2.0;
    let (width, height, x, y) = if positioned {
        let width = node.width.unwrap_or(0.0);
        let height = node.height.unwrap_or(0.0);
        (width, height, -width / 2.0, -height / 2.0)
    } else {
        let width = label.bbox.width + node_padding;
        let height = label.bbox.height + node_padding;
        (
            width,
            height,
            -label.bbox.width / 2.0 - half_padding,
            -label.bbox.height / 2.0 - half_padding,
        )
    };
    let (style, _) = styles_from_array(&node.css_styles);
    let style = if style.is_empty() {
        String::new()
    } else {
        attribute("style", &style)
    };
    // `rx` and `ry` are the node's radii, zero for a composite (`getNodeFromBlock`: `radius = 0`).
    let svg = format!(
        "<rect class=\"basic cluster composite label-container\"{style} rx=\"0\" ry=\"0\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>",
        number(x),
        number(y),
        number(width),
        number(height),
    );
    let bbox = BBox::new(x, y, width, height);
    Part::with_centred_label(svg, bbox, Intersect::Rect, label)
}
