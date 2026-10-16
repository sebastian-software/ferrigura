//! The rectangles with a feature of their own: `datastore` (a rectangle with dashed sides), `note`
//! (a rectangle drawn by rough.js), `dividedRectangle` (a rectangle with a bar across its top) and
//! `card` (a rectangle with a notched top left corner). Ports of `shapes/datastore.ts`, `note.ts`,
//! `dividedRect.ts` and `card.ts`, for the neo and classic looks. See
//! `docs-port/flowchart/render-notes/shapes-core.md` §2.1 and §2.4.

use super::rough;
use super::util::{self, Part, attribute, label_helper, path_style, stroke_width, styles2string};
use super::{Intersect, ShapeInput};
use crate::svg::number;
use crate::unified::geom::{BBox, Point};

/// `datastore.ts`: a rectangle around the label, padded by twice the padding across and the padding
/// high (`drawRect` with twice the label padding on each side). Its stroke is dashed with its own
/// size, so that the outline shows only its corners and the lines between them.
pub(super) fn datastore(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let label = label_helper(input, false);
    let total_width = (label.bbox.width + node.padding * 4.0).max(node.width.unwrap_or(0.0));
    let total_height = (label.bbox.height + node.padding * 2.0).max(node.height.unwrap_or(0.0));
    let x = -total_width / 2.0;
    let y = -total_height / 2.0;
    let svg = format!(
        "<rect class=\"basic label-container\"{} x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" stroke-dasharray=\"{} {}\"/>",
        attribute("style", &styles.node),
        number(x),
        number(y),
        number(total_width),
        number(total_height),
        number(total_width),
        number(total_height),
    );
    let label_offset = util::centred_label_offset(&label);
    Part {
        svg,
        after_label: String::new(),
        bbox: BBox::new(x, y, total_width, total_height),
        intersect: Intersect::Rect,
        label_offset: Some(label_offset),
        label: Some(label),
    }
}

/// `note.ts`: a rectangle drawn by rough.js at roughness 0, around the label. The label is moved so
/// that its centre is at the centre of the node, on both axes, as the reference's note label is.
pub(super) fn note(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let label = label_helper(input, false);
    let total_width = (label.bbox.width + node.padding * 2.0).max(node.width.unwrap_or(0.0));
    let total_height = (label.bbox.height + node.padding * 2.0).max(node.height.unwrap_or(0.0));
    let x = -total_width / 2.0;
    let y = -total_height / 2.0;
    let style = path_style(node, &styles);
    // `rc.rectangle` at roughness 0: the fill and the doubled stroke of the rectangle.
    let svg = format!(
        "<g class=\"basic label-container outer-path\">{}</g>",
        rough::rectangle_paths(
            x,
            y,
            total_width,
            total_height,
            Some(style.as_str()),
            &stroke_width(&styles)
        ),
    );
    let outline_box = BBox::new(x, y, total_width, total_height);
    let label_offset = Point::new(
        -label.bbox.width / 2.0 - label.bbox.x,
        -label.bbox.height / 2.0 - label.bbox.y,
    );
    Part {
        svg,
        after_label: String::new(),
        bbox: outline_box,
        intersect: Intersect::Rect,
        label_offset: Some(label_offset),
        label: Some(label),
    }
}

/// `dividedRect.ts`: a rectangle with a bar across its top a fifth of its height down, drawn as one
/// rough.js polygon at roughness 0. The neo look pads the label by 16 on each side; the classic
/// look by the padding.
pub(super) fn divided_rectangle(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let (padding_x, padding_y) = if node.is_neo() {
        (16.0, 16.0)
    } else {
        (node.padding, node.padding)
    };
    let label = label_helper(input, false);
    let w = label.bbox.width + padding_x;
    let h = label.bbox.height + padding_y;
    let rect_offset = h * 0.2;
    let x = -w / 2.0;
    let y = -h / 2.0 - rect_offset / 2.0;

    let points = [
        Point::new(x, y + rect_offset),
        Point::new(-x, y + rect_offset),
        Point::new(-x, -y),
        Point::new(x, -y),
        Point::new(x, y),
        Point::new(-x, y),
        Point::new(-x, y + rect_offset),
    ];
    let outline = rough::polygon(&points);
    let style = path_style(node, &styles);
    let svg = format!(
        "<g class=\"basic label-container outer-path\">{}</g>",
        rough::paths(&outline, Some(style.as_str()), &stroke_width(&styles), true),
    );
    let label_offset = Point::new(
        x + node.padding / 2.0 - label.bbox.x,
        y + rect_offset + node.padding / 2.0 - label.bbox.y,
    );
    Part {
        svg,
        after_label: String::new(),
        bbox: outline.bbox,
        intersect: Intersect::Rect,
        label_offset: Some(label_offset),
        label: Some(label),
    }
}

/// `card.ts`: a rectangle with its top left corner notched, drawn as a polygon. The neo look pads
/// the label by 28 across and 24 high; the classic look by the padding, and by the notch's 12
/// across. The polygon is moved to the centre of the node.
pub(super) fn card(input: &ShapeInput<'_>) -> Part {
    const NOTCH_SIZE: f64 = 12.0;
    let node = input.node;
    let styles = styles2string(node);
    let (label_padding_x, label_padding_y) = if node.is_neo() {
        (28.0, 24.0)
    } else {
        (node.padding, node.padding)
    };
    let label = label_helper(input, false);
    let w = node.width.unwrap_or(label.bbox.width)
        + if node.is_neo() {
            label_padding_x * 2.0
        } else {
            label_padding_x + NOTCH_SIZE
        };
    let h = node.height.unwrap_or(label.bbox.height)
        + if node.is_neo() {
            label_padding_y * 2.0
        } else {
            label_padding_y
        };
    let points = [
        Point::new(NOTCH_SIZE, -h),
        Point::new(w, -h),
        Point::new(w, 0.0),
        Point::new(0.0, 0.0),
        Point::new(0.0, -h + NOTCH_SIZE),
        Point::new(NOTCH_SIZE, -h),
    ];
    let style = (!styles.node.is_empty()).then_some(styles.node.as_str());
    let (svg, bbox) = util::polygon_shape(&points, w, h, style);
    Part {
        svg,
        after_label: String::new(),
        bbox,
        intersect: Intersect::Polygon {
            points: points.to_vec(),
        },
        label_offset: Some(util::centred_label_offset(&label)),
        label: Some(label),
    }
}
