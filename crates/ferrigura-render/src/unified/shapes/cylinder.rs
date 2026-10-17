//! The cylinder: a can with a curved top and bottom. A port of `shapes/cylinder.ts`, in the plain
//! path the classic and neo looks draw (the rough version is for the hand-drawn look only).

use super::util::{Part, attribute, label_helper, styles2string};
use super::{Intersect, ShapeInput};
use crate::svg::number;
use crate::unified::data::Node;
use crate::unified::geom::{BBox, Point};

/// The cylinder's outline: the top ellipse's near half, the side, and the bottom's near half
/// (`createCylinderPathD`).
fn path_data(width: f64, height: f64, rx: f64, ry: f64) -> String {
    [
        format!("M0,{}", number(ry)),
        format!("a{},{} 0,0,0 {},0", number(rx), number(ry), number(width)),
        format!("a{},{} 0,0,0 {},0", number(rx), number(ry), number(-width)),
        format!("l0,{}", number(height)),
        format!("a{},{} 0,0,0 {},0", number(rx), number(ry), number(width)),
        format!("l0,{}", number(-height)),
    ]
    .join(" ")
}

/// `cylinder.ts`: the cylinder around the label, its cap's radii, and the label moved to sit under
/// the cap. The neo look pads the cylinder by 24 in both directions; the classic look by the
/// padding.
/// `MIN_WIDTH` and `MIN_HEIGHT` of `cylinder.ts`.
const MIN_SIZE: f64 = 8.0;

/// The padding of a cylinder: 24 in the neo look, else the node's padding (`cylinder.ts` 32-34).
fn cylinder_padding(node: &Node) -> f64 {
    if node.is_neo() { 24.0 } else { node.padding }
}

/// The width a cylinder's label wraps at, when the node has a width: the node's own width less the
/// padding, at least `MIN_SIZE` (`cylinder.ts` 65-71 sets `node.width` before `labelHelper` lays out the
/// label). `None` for a node without a width.
#[must_use]
pub(super) fn narrowed_width(node: &Node) -> Option<f64> {
    node.width
        .filter(|width| *width != 0.0)
        .map(|width| (width - cylinder_padding(node)).max(MIN_SIZE))
}

pub(super) fn cylinder(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let pad = cylinder_padding(node);
    // A node with a size (a block in its placed pass) is narrowed and shortened first, as `cylinder.ts`
    // 59-80 writes its `node.width` and `node.height`; a node without one keeps the label's box.
    let original_width = node.width.unwrap_or(0.0);
    let node_width = narrowed_width(node).or(node.width);
    let mut node_height = node.height;
    if node.height.is_some_and(|height| height != 0.0) {
        let rx = original_width / 2.0;
        let ry = rx / (2.5 + original_width / 50.0);
        node_height = Some((node.height.unwrap_or(0.0) - pad - ry * 3.0).max(MIN_SIZE));
    }
    let label = label_helper(input, false);

    let width = node_width.unwrap_or(0.0).max(label.bbox.width) + pad;
    let rx = width / 2.0;
    let ry = rx / (2.5 + width / 50.0);
    let height = node_height.unwrap_or(0.0).max(label.bbox.height) + pad + ry;

    // The outline is drawn from its top-left corner; its box is `(0, 0, w, h + 2ry)`, and the
    // transform centres it on the node.
    let transform_x = -width / 2.0;
    let transform_y = -(height / 2.0 + ry);
    let svg = format!(
        "<path{} d=\"{}\"{} label-offset-y=\"{}\" transform=\"translate({}, {})\"/>",
        attribute("class", "basic label-container outer-path"),
        path_data(width, height, rx, ry),
        attribute("style", &styles.node),
        number(ry),
        number(transform_x),
        number(transform_y),
    );
    let bbox = BBox::new(0.0, 0.0, width, height + 2.0 * ry).translate(transform_x, transform_y);

    // `translate(-W/2 - x, -H/2 + P/1.5 - y)`: the label sits below the cap.
    let label_offset = Point::new(
        -(label.bbox.width / 2.0) - label.bbox.x,
        -(label.bbox.height / 2.0) + node.padding / 1.5 - label.bbox.y,
    );
    Part {
        svg,
        after_label: String::new(),
        bbox,
        intersect: Intersect::Cylinder { rx, ry },
        label_offset: Some(label_offset),
        label: Some(label),
    }
}

/// The outline of `linedCylinder.ts`: the cylinder of `path_data`, with a second cap line
/// drawn a tenth of the height below the first (`createCylinderPathD` in that file).
fn lined_path_data(width: f64, height: f64, rx: f64, ry: f64, outer_offset: f64) -> String {
    [
        format!("M0,{}", number(ry)),
        format!("a{},{} 0,0,0 {},0", number(rx), number(ry), number(width)),
        format!("a{},{} 0,0,0 {},0", number(rx), number(ry), number(-width)),
        format!("l0,{}", number(height)),
        format!("a{},{} 0,0,0 {},0", number(rx), number(ry), number(width)),
        format!("l0,{}", number(-height)),
        format!("M0,{}", number(ry + outer_offset)),
        format!("a{},{} 0,0,0 {},0", number(rx), number(ry), number(width)),
    ]
    .join(" ")
}

/// `linedCylinder.ts` (`lin-cyl`): a cylinder with a second cap line. Its label is set under the
/// cap. The neo look pads the label by 16 and 24; the classic look by the padding. The node has no
/// explicit size, so the reference's sized branch does not run.
pub(super) fn lined_cylinder(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let (pad_x, pad_y) = if node.is_neo() {
        (16.0, 24.0)
    } else {
        (node.padding, node.padding)
    };
    let label = label_helper(input, false);
    let width = label.bbox.width + pad_x * 2.0;
    let rx = width / 2.0;
    let ry = rx / (2.5 + width / 50.0);
    let height = label.bbox.height + ry + pad_y * 2.0;
    let outer_offset = height * 0.1;

    let svg = format!(
        "<path{} d=\"{}\"{} label-offset-y=\"{}\" transform=\"translate({}, {})\"/>",
        attribute("class", "basic label-container outer-path"),
        lined_path_data(width, height, rx, ry, outer_offset),
        attribute("style", &styles.node),
        number(ry),
        number(-width / 2.0),
        number(-(height / 2.0 + ry)),
    );
    // The path reaches from the top of the upper cap to the bottom of the lower one.
    let bbox = BBox::new(-width / 2.0, -(height / 2.0 + ry), width, height + 2.0 * ry);
    let label_offset = Point::new(
        -(label.bbox.width / 2.0) - label.bbox.x,
        -(label.bbox.height / 2.0) + ry - label.bbox.y,
    );
    Part {
        svg,
        after_label: String::new(),
        bbox,
        intersect: Intersect::Cylinder { rx, ry },
        label_offset: Some(label_offset),
        label: Some(label),
    }
}

/// The outline of `tiltedCylinder.ts`: a cylinder on its side, its caps at the left and right
/// ends (`createCylinderPathD` in that file).
fn tilted_path_data(width: f64, height: f64, rx: f64, ry: f64) -> String {
    [
        format!(
            "M0,0 a{},{} 0,0,1 0,{}",
            number(rx),
            number(ry),
            number(-height)
        ),
        format!("l{},0", number(width)),
        format!("a{},{} 0,0,1 0,{}", number(rx), number(ry), number(height)),
        format!("M{},{}", number(width), number(-height)),
        format!("a{},{} 0,0,0 0,{}", number(rx), number(ry), number(height)),
        format!("l{},0", number(-width)),
    ]
    .join(" ")
}

/// `tiltedCylinder.ts` (`h-cyl`): a cylinder lying on its side, with the label centred in it. The
/// neo look pads its height by 12; the classic look by half the padding. The node has no explicit
/// size, so the reference's sized branch does not run.
pub(super) fn tilted_cylinder(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let label_padding = if node.is_neo() {
        12.0
    } else {
        node.padding / 2.0
    };
    let label = label_helper(input, false);
    let height = label.bbox.height.max(0.0) + label_padding;
    let ry = height / 2.0;
    let rx = ry / (2.5 + height / 50.0);
    let width = label.bbox.width.max(0.0) + rx + label_padding;

    let svg = format!(
        "<path{} d=\"{}\"{} label-offset-x=\"{}\" transform=\"translate({}, {})\"/>",
        attribute("class", "basic label-container outer-path"),
        tilted_path_data(width, height, rx, ry),
        attribute("style", &styles.node),
        number(rx),
        number(-width / 2.0),
        number(height / 2.0),
    );
    // The cap bulges by `rx` past each end of the body.
    let bbox = BBox::new(-width / 2.0 - rx, -height / 2.0, width + 2.0 * rx, height);
    let label_offset = Point::new(
        -(label.bbox.width / 2.0) - rx - label.bbox.x,
        -(label.bbox.height / 2.0) - label.bbox.y,
    );
    Part {
        svg,
        after_label: String::new(),
        bbox,
        intersect: Intersect::TiltedCylinder { rx, ry },
        label_offset: Some(label_offset),
        label: Some(label),
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{draw_box, node};
    use super::*;
    use crate::unified::label::LabelLayout;

    #[test]
    fn a_neo_cylinder_is_the_label_plus_the_neo_padding_and_its_caps() {
        let mut node = node("cylinder", "neo");
        node.label = "a label".to_owned();
        let label = LabelLayout::of_box(BBox::new(-60.0, 0.0, 120.0, 17.0));
        let shape = draw_box(&node, &label);
        // width 120 + 24; ry = (w/2) / (2.5 + w/50); height 17 + 24 + ry; the outline is
        // h + 2ry tall.
        let width = 144.0;
        let rx = width / 2.0;
        let ry = rx / (2.5 + width / 50.0);
        let height = 17.0 + 24.0 + ry;
        assert!((shape.bbox.width - width).abs() < 1e-9, "{:?}", shape.bbox);
        assert!(
            (shape.bbox.height - (height + 2.0 * ry)).abs() < 1e-9,
            "{:?}",
            shape.bbox
        );
    }
}
