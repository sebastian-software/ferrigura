//! The ellipse and the business ellipse: ports of `rendering-elements/shapes/usecaseEllipse.ts` and
//! `usecaseBusiness.ts` in the plain elements of the neo and classic looks. Mermaid draws the use cases
//! of a use case diagram with them; a flowchart node can name them too.
//!
//! The ellipse is the label's box grown by twice the half padding on each side (`usecaseEllipse.ts`
//! 24-28). A business ellipse adds a chord across its upper part (`usecaseBusiness.ts` 68-86), placed
//! from the radii as the reference computes them. The stereotype line of the reference comes from the
//! fields of a use case, which a flowchart node does not carry.

use super::Intersect;
use super::ShapeInput;
use super::usecase_actor::position_label;
use super::util::{self, Part, attribute, label_helper, styles2string};
use crate::svg::number;
use crate::unified::geom::BBox;

/// `usecaseEllipse`: the ellipse around the label, with the label centered on the origin.
pub(super) fn ellipse(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let label = label_helper(input, false);
    // `halfPadding ?? 10`: the label's half padding, which is the node's padding over two.
    let half_padding = node.padding / 2.0;
    let radius_x = label.bbox.width / 2.0 + half_padding * 2.0;
    let radius_y = label.bbox.height / 2.0 + half_padding * 2.0;
    let svg = format!(
        "<ellipse class=\"basic label-container\"{} rx=\"{}\" ry=\"{}\" cx=\"0\" cy=\"0\"/>",
        attribute("style", &styles.node),
        number(radius_x),
        number(radius_y),
    );
    Part {
        svg,
        after_label: String::new(),
        bbox: BBox::new(-radius_x, -radius_y, radius_x * 2.0, radius_y * 2.0),
        intersect: Intersect::Ellipse {
            rx: radius_x,
            ry: radius_y,
        },
        label_offset: Some(util::centered_label_offset(&label)),
        label: Some(label),
    }
}

/// `usecaseBusiness`: the ellipse around the label, with a chord across its upper part.
pub(super) fn business(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let label = label_helper(input, false);
    let label_height = label.bbox.height;
    let label_width = label.bbox.width;
    let padding = node.padding / 2.0;
    let radius_x = label_width / 2.0 + padding * 2.0;
    let radius_y = label_height / 2.0 + padding * 2.0;
    // `markerInset` and the chord's ends on the ellipse (`usecaseBusiness.ts` 75-81).
    let marker_inset = (padding / 5.0).max(1.0).min(padding / 2.0);
    let marker_start_x = label_width / 2.0 + marker_inset;
    let marker_end_x = radius_x - marker_inset;
    let normalized_start = marker_start_x / radius_x;
    let normalized_end = marker_end_x / radius_x;
    let marker_start_y = radius_y * (1.0 - normalized_start * normalized_start).max(0.0).sqrt();
    let marker_end_y = -radius_y * (1.0 - normalized_end * normalized_end).max(0.0).sqrt();

    let label_offset = position_label(&label, 0.0);
    let node_style = if styles.node.is_empty() {
        String::new()
    } else {
        attribute("style", &styles.node)
    };
    let after = format!(
        "<path class=\"usecase-business-marker\" d=\"M {} {} L {} {}\" fill=\"none\"{node_style}/>",
        number(marker_start_x),
        number(marker_start_y),
        number(marker_end_x),
        number(marker_end_y),
    );

    let svg = format!(
        "<ellipse class=\"basic label-container usecase-business-ellipse\"{} rx=\"{}\" ry=\"{}\" cx=\"0\" cy=\"0\"/>",
        attribute("style", &styles.node),
        number(radius_x),
        number(radius_y),
    );
    Part {
        svg,
        after_label: after,
        bbox: BBox::new(-radius_x, -radius_y, radius_x * 2.0, radius_y * 2.0),
        intersect: Intersect::Ellipse {
            rx: radius_x,
            ry: radius_y,
        },
        label_offset: Some(label_offset),
        label: Some(label),
    }
}
