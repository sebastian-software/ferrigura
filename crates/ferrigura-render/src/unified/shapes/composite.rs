//! `composite.ts`: a rectangle around the label. A port of `rendering-util/rendering-elements/
//! shapes/composite.ts`, which Mermaid draws the composite blocks of a block diagram with; a flowchart
//! node can name it too.
//!
//! A flowchart node is never placed by a block layout, so the rectangle is the label's box plus the
//! node's padding.

use super::util::{Part, attribute, label_helper, styles_from_array};
use super::{Intersect, ShapeInput};
use crate::svg::number;
use crate::unified::geom::BBox;

/// `composite.ts`: a `rect.basic.cluster.composite.label-container`, then the label group.
pub(super) fn composite(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let label = label_helper(input, false);
    // `halfPadding` is the node's padding over two.
    let node_padding = node.padding;
    let half_padding = node_padding / 2.0;
    let width = label.bbox.width + node_padding;
    let height = label.bbox.height + node_padding;
    let x = -label.bbox.width / 2.0 - half_padding;
    let y = -label.bbox.height / 2.0 - half_padding;
    let (style, _) = styles_from_array(&node.css_styles);
    let style = if style.is_empty() {
        String::new()
    } else {
        attribute("style", &style)
    };
    // `rx` and `ry` are zero: the reference passes the block's radius, which a flowchart node does not
    // set.
    let svg = format!(
        "<rect class=\"basic cluster composite label-container\"{style} rx=\"0\" ry=\"0\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>",
        number(x),
        number(y),
        number(width),
        number(height),
    );
    let bbox = BBox::new(x, y, width, height);
    Part::with_centered_label(svg, bbox, Intersect::Rect, label)
}
