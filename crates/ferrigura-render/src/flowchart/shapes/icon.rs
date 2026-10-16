//! The icon and image shapes: `icon`, `iconSquare`, `iconRounded`, `iconCircle` and `imageSquare`.
//! Ports of `shapes/icon.ts`, `iconSquare.ts`, `iconRounded.ts`, `iconCircle.ts` and
//! `imageSquare.ts`, for the neo and classic looks. See
//! `docs-port/flowchart/render-notes/shapes-core.md` §2.1 and §2.4.
//!
//! **Icons.** ferrigura registers no icon pack, and the harness renders the reference the same
//! way, so every icon name is unknown: `fa:bell`, `logos:aws` and a bare name alike. The reference
//! then draws its fallback, `unknownIcon` in `rendering-util/icons.ts` (`getIconSVG` catches the
//! failed lookup): a blue square with a question mark, at the icon size. That is the only icon
//! drawn, and its box is the icon size, so every icon's geometry is the reference's. The `fa:`
//! names of labels are a different path: the reference replaces them only in HTML labels
//! (`replaceIconSubstring`), and ferrigura's labels are SVG text, so they stay text as they do in
//! the reference's SVG labels.
//!
//! **Images.** The reference loads the image to read its natural size (`img.decode()`), and the
//! render fails when the image does not decode. ferrigura does not read image files. An image
//! therefore needs both `assetWidth` and `assetHeight`, and a `constraint` other than `on` (which
//! needs the aspect ratio); the asset sizes are then all the geometry uses, as in the reference.
//! Without them, `imageSquare` is an error. A reference render whose image does not decode fails;
//! ferrigura does not check that, so it draws an image whose sizes it was given.

use ferrigura_model::value::Value;

use super::rough::{self, Outline};
use super::util::{self, Part, Styles, label_helper, styles2string};
use super::{Intersect, ShapeInput};
use crate::RenderError;
use crate::flowchart::data::Node;
use crate::flowchart::geom::{BBox, Point};
use crate::flowchart::label::LabelLayout;
use crate::svg::{escape, number};

/// The icon size when the node gives none (`assetWidth ?? 48`).
const DEFAULT_ICON_SIZE: f64 = 48.0;
/// The space under the label, between the icon and the label (`node.label ? 8 : 0`).
const LABEL_PADDING: f64 = 8.0;
/// The space between the icon and the circle around it (`iconCircle.ts`).
const CIRCLE_PADDING: f64 = 20.0;

/// The icon size of a node: the larger of its asset width and height.
#[must_use]
pub(super) fn icon_size(node: &Node) -> f64 {
    let height = node.asset_height.unwrap_or(DEFAULT_ICON_SIZE);
    let width = node.asset_width.unwrap_or(DEFAULT_ICON_SIZE);
    height.max(width)
}

/// The width the label of an icon shape is wrapped at: the icon size or the flowchart's wrapping
/// width, whichever is larger (`node.width = Math.max(iconSize, defaultWidth)` in the handler).
#[must_use]
pub(super) fn label_width(node: &Node, default_width: f64) -> f64 {
    icon_size(node).max(default_width)
}

/// The size of an image: its width and height as `imageSquare.ts` gives them when the natural
/// size is not needed (see the module note).
fn image_size(node: &Node, default_width: f64) -> Result<(f64, f64), RenderError> {
    let (Some(asset_width), Some(asset_height)) = (node.asset_width, node.asset_height) else {
        return Err(RenderError::Options(format!(
            "imageSquare needs assetWidth and assetHeight: ferrigura does not read image files, so the natural size of {} is unknown",
            node.img.as_deref().unwrap_or_default(),
        )));
    };
    if node.constraint.as_deref() == Some("on") {
        return Err(RenderError::Options(
            "imageSquare with constraint on needs the natural aspect ratio of its image, which ferrigura does not read".to_owned(),
        ));
    }
    let label_width = if node.label.is_empty() {
        0.0
    } else {
        default_width
    };
    Ok((label_width.max(asset_width), asset_height))
}

/// The width the label of an image shape is wrapped at: the image's width or the flowchart's
/// wrapping width, whichever is larger (`node.width` in `imageSquare.ts`).
///
/// # Errors
///
/// Returns an error for an image whose size ferrigura cannot know (see the module note).
pub(super) fn image_label_width(node: &Node, default_width: f64) -> Result<f64, RenderError> {
    let (image_width, _) = image_size(node, default_width)?;
    Ok(image_width.max(default_width))
}

/// The markup `getIconSVG` gives a name the registry does not know: iconify's rendering of
/// `unknownIcon` at `size` (`iconToSVG` and `iconToHTML`).
fn fallback_icon_svg(size: f64) -> String {
    let size = number(size);
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{size}\" height=\"{size}\" viewBox=\"0 0 80 80\"><g><rect width=\"80\" height=\"80\" style=\"fill: #087ebf; stroke-width: 0px;\"/><text transform=\"translate(21.16 64.67)\" style=\"fill: #fff; font-family: ArialMT, Arial; font-size: 67.75px;\"><tspan x=\"0\" y=\"0\">?</tspan></text></g></svg>"
    )
}

/// Whether the node names an icon. An empty name is no icon, as in the reference's truthiness test.
fn has_icon(node: &Node) -> bool {
    node.icon.as_deref().is_some_and(|name| !name.is_empty())
}

/// Whether the label is above the shape (`node.pos === 't'`).
fn label_on_top(node: &Node) -> bool {
    node.pos.as_deref() == Some("t")
}

/// The label of an icon or an image, as `labelHelper` leaves it: the box is the label's
/// background when the shape has one (the reference measures the group that holds it), and the
/// minimum width does not apply, because the handler sets the node's width first.
fn icon_label(input: &ShapeInput<'_>) -> LabelLayout {
    let mut label = label_helper(input, true);
    if let Some(background) = label.background {
        label.bbox = background;
    }
    label
}

/// The `<g>` of the icon (`iconElem`): the fallback icon in the stroke colour (the node's
/// `stroke`, else the theme's `nodeBorder`), positioned at `at`.
fn icon_element(input: &ShapeInput<'_>, styles: &Styles, at: Point) -> String {
    let colour = styles
        .entries
        .iter()
        .find(|(key, _)| key == "stroke")
        .map(|(_, value)| value.clone())
        .unwrap_or_else(|| match input.theme.get("nodeBorder") {
            Some(Value::String(text)) => text.clone(),
            _ => String::new(),
        });
    format!(
        "<g style=\"color: {colour};\" transform=\"translate({}, {})\"><g>{}</g></g>",
        number(at.x),
        number(at.y),
        fallback_icon_svg(icon_size(input.node)),
    )
}

/// The vertical offset of an icon or label row: the icon sits above the label when the label is at
/// the bottom, and the offset is measured from the centre of the whole node (`icon.ts`).
fn icon_row_y(top: bool, label_height: f64, label_padding: f64, icon_height: f64) -> f64 {
    if top {
        label_height / 2.0 + label_padding / 2.0 - icon_height / 2.0
    } else {
        -label_height / 2.0 - label_padding / 2.0 - icon_height / 2.0
    }
}

/// The points of an icon's outline for an edge that meets the label's side (`icon.ts`'s
/// intersect), relative to the node's centre. `node_height` is the height of the node's box.
fn label_polygon(
    top: bool,
    shape: (f64, f64),
    label: (f64, f64),
    label_padding: f64,
    node_height: f64,
) -> Vec<Point> {
    let (width, height) = shape;
    let (label_width, label_height) = label;
    let nh = node_height;
    if top {
        vec![
            Point::new(-label_width / 2.0, -nh / 2.0),
            Point::new(label_width / 2.0, -nh / 2.0),
            Point::new(label_width / 2.0, -nh / 2.0 + label_height + label_padding),
            Point::new(width / 2.0, -nh / 2.0 + label_height + label_padding),
            Point::new(width / 2.0, nh / 2.0),
            Point::new(-width / 2.0, nh / 2.0),
            Point::new(-width / 2.0, -nh / 2.0 + label_height + label_padding),
            Point::new(-label_width / 2.0, -nh / 2.0 + label_height + label_padding),
        ]
    } else {
        vec![
            Point::new(-width / 2.0, -nh / 2.0),
            Point::new(width / 2.0, -nh / 2.0),
            Point::new(width / 2.0, -nh / 2.0 + height),
            Point::new(label_width / 2.0, -nh / 2.0 + height),
            Point::new(label_width / 4.0, nh / 2.0),
            Point::new(-label_width / 2.0, nh / 2.0),
            Point::new(-label_width / 2.0, -nh / 2.0 + height),
            Point::new(-width / 2.0, -nh / 2.0 + height),
        ]
    }
}

/// The intersect of an icon with a label: the rectangle when there is no label, else the outline
/// of the icon and the label together.
fn icon_intersect(
    node: &Node,
    top: bool,
    shape: (f64, f64),
    label: &LabelLayout,
    label_padding: f64,
    node_height: f64,
) -> Intersect {
    if node.label.is_empty() {
        return Intersect::Rect;
    }
    Intersect::Polygon {
        points: label_polygon(
            top,
            shape,
            (label.bbox.width, label.bbox.height),
            label_padding,
            node_height,
        ),
    }
}

/// The paths of a rough outline with no stroke or style of its own, as the icon handlers draw
/// their rectangles and circles (`rc.rectangle` and `rc.circle` with the icon options).
fn outline_group(outline: &Outline, stroke_width: &str, stroke: bool) -> String {
    format!(
        "<g>{}</g>",
        rough::paths(outline, None, stroke_width, stroke)
    )
}

/// The outline of the icon's shape, moved vertically by `shift` (`iconShape.attr('transform')`):
/// the shape sits above or below the centre so that the label fits under or over it.
fn shifted_outline_group(
    outline: &Outline,
    stroke_width: &str,
    stroke: bool,
    shift: f64,
) -> String {
    format!(
        "<g transform=\"translate(0, {})\">{}</g>",
        number(shift),
        rough::paths(outline, None, stroke_width, stroke)
    )
}

/// `icon.ts`: a square of the icon size with the label below it (or above it, with `pos: t`). The
/// square has no stroke, and the icon is the fallback icon.
pub(super) fn icon(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let size = icon_size(node);
    let top = label_on_top(node);
    let label = icon_label(input);
    let label_padding = if node.label.is_empty() {
        0.0
    } else {
        LABEL_PADDING
    };
    let stroke_width = util::stroke_width(&styles);

    let shape = rough::rectangle(-size / 2.0, -size / 2.0, size, size);
    let outer_width = size.max(label.bbox.width);
    let outer_height = size + label.bbox.height + label_padding;
    let outer = rough::rectangle(
        -outer_width / 2.0,
        -outer_height / 2.0,
        outer_width,
        outer_height,
    );
    let shift = if top {
        label.bbox.height / 2.0 + label_padding / 2.0
    } else {
        -label.bbox.height / 2.0 - label_padding / 2.0
    };
    let label_offset = Point::new(
        -label.bbox.width / 2.0 - label.bbox.x,
        if top {
            -outer_height / 2.0
        } else {
            outer_height / 2.0 - label.bbox.height
        },
    );

    let mut bbox = BBox::new(
        -outer_width / 2.0,
        -outer_height / 2.0,
        outer_width,
        outer_height,
    )
    .union(BBox::new(-size / 2.0, -size / 2.0 + shift, size, size))
    .union(label.bbox.translate(label_offset.x, label_offset.y));
    let mut after_label = outline_group(&outer, &stroke_width, false);
    if has_icon(node) {
        let at = Point::new(
            -size / 2.0,
            icon_row_y(top, label.bbox.height, label_padding, size),
        );
        bbox = bbox.union(BBox::new(at.x, at.y, size, size));
        after_label.push_str(&icon_element(input, &styles, at));
    }
    let intersect = icon_intersect(node, top, (size, size), &label, label_padding, bbox.height);
    Part {
        svg: shifted_outline_group(&shape, &stroke_width, false, shift),
        after_label,
        bbox,
        intersect,
        label_offset: Some(label_offset),
        label: Some(label),
    }
}

/// `iconSquare.ts` and `iconRounded.ts`: the icon in a square with corners of `radius`, padded by
/// half the node's padding on each side, with the label below it or above it.
fn framed(input: &ShapeInput<'_>, radius: f64) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let size = icon_size(node);
    let top = label_on_top(node);
    let label = icon_label(input);
    let label_padding = if node.label.is_empty() {
        0.0
    } else {
        LABEL_PADDING
    };
    let half_padding = node.padding / 2.0;
    let width = size + half_padding * 2.0;
    let height = width;
    let stroke_width = util::stroke_width(&styles);

    let shape = rough::rounded_rectangle(-width / 2.0, -height / 2.0, width, height, radius);
    // The reference gives the framed outline a stroke (`stroke = fill ?? mainBkg`).
    let outer_width = width.max(label.bbox.width);
    let outer_height = height + label.bbox.height + label_padding;
    let outer = rough::rectangle(
        -outer_width / 2.0,
        -outer_height / 2.0,
        outer_width,
        outer_height,
    );
    let shift = if top {
        label.bbox.height / 2.0 + label_padding / 2.0
    } else {
        -label.bbox.height / 2.0 - label_padding / 2.0
    };
    let label_offset = Point::new(
        -label.bbox.width / 2.0 - label.bbox.x,
        if top {
            -outer_height / 2.0
        } else {
            outer_height / 2.0 - label.bbox.height
        },
    );

    let mut bbox = BBox::new(
        -outer_width / 2.0,
        -outer_height / 2.0,
        outer_width,
        outer_height,
    )
    .union(BBox::new(
        -width / 2.0,
        -height / 2.0 + shift,
        width,
        height,
    ))
    .union(label.bbox.translate(label_offset.x, label_offset.y));
    let mut after_label = outline_group(&outer, &stroke_width, false);
    if has_icon(node) {
        let at = Point::new(
            -size / 2.0,
            icon_row_y(top, label.bbox.height, label_padding, size),
        );
        bbox = bbox.union(BBox::new(at.x, at.y, size, size));
        after_label.push_str(&icon_element(input, &styles, at));
    }
    let intersect = icon_intersect(
        node,
        top,
        (width, height),
        &label,
        label_padding,
        bbox.height,
    );
    Part {
        svg: shifted_outline_group(&shape, &stroke_width, true, shift),
        after_label,
        bbox,
        intersect,
        label_offset: Some(label_offset),
        label: Some(label),
    }
}

/// `iconSquare.ts`: the icon in a square with almost square corners.
pub(super) fn icon_square(input: &ShapeInput<'_>) -> Part {
    framed(input, 0.1)
}

/// `iconRounded.ts`: the icon in a square with corners of 5.
pub(super) fn icon_rounded(input: &ShapeInput<'_>) -> Part {
    framed(input, 5.0)
}

/// `iconCircle.ts`: the icon in a circle that is its diagonal plus padding, with the label below
/// it (or above it). The circle has the stroke of the node; the intersect is the rectangle.
pub(super) fn icon_circle(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let top = label_on_top(node);
    let label = icon_label(input);
    let label_padding = if node.label.is_empty() {
        0.0
    } else {
        LABEL_PADDING
    };
    let stroke_width = util::stroke_width(&styles);

    // The icon's box is the icon's size, or nothing without an icon (an empty `<g>`).
    let icon_box = if has_icon(node) { icon_size(node) } else { 0.0 };
    let diameter = icon_box * std::f64::consts::SQRT_2 + CIRCLE_PADDING * 2.0;
    let circle = rough::ellipse(0.0, 0.0, diameter, diameter);
    let outer_width = diameter.max(label.bbox.width);
    let outer_height = diameter + label.bbox.height + label_padding;
    let outer = rough::rectangle(
        -outer_width / 2.0,
        -outer_height / 2.0,
        outer_width,
        outer_height,
    );
    let shift = if top {
        label.bbox.height / 2.0 + label_padding / 2.0
    } else {
        -label.bbox.height / 2.0 - label_padding / 2.0
    };
    let label_offset = Point::new(
        -label.bbox.width / 2.0 - label.bbox.x,
        if top {
            -outer_height / 2.0
        } else {
            outer_height / 2.0 - label.bbox.height
        },
    );

    let mut bbox = BBox::new(
        -outer_width / 2.0,
        -outer_height / 2.0,
        outer_width,
        outer_height,
    )
    .union(circle.bbox.translate(0.0, shift))
    .union(label.bbox.translate(label_offset.x, label_offset.y));
    // The icon element comes before the outer box in the reference, after the label.
    let mut after_label = String::new();
    if has_icon(node) {
        let at = Point::new(
            -icon_box / 2.0,
            icon_row_y(top, label.bbox.height, label_padding, icon_box),
        );
        bbox = bbox.union(BBox::new(at.x, at.y, icon_box, icon_box));
        after_label.push_str(&icon_element(input, &styles, at));
    }
    after_label.push_str(&outline_group(&outer, &stroke_width, false));
    Part {
        svg: shifted_outline_group(&circle, &stroke_width, true, shift),
        after_label,
        bbox,
        intersect: Intersect::Rect,
        label_offset: Some(label_offset),
        label: Some(label),
    }
}

/// `imageSquare.ts`: an image of its asset size, with the label below it (or above it). The image
/// is drawn as an `<image>` element of that size over the rectangle that outlines it.
///
/// # Errors
///
/// Returns an error for an image whose size ferrigura cannot know (see the module note).
pub(super) fn image_square(input: &ShapeInput<'_>) -> Result<Part, RenderError> {
    let node = input.node;
    let styles = styles2string(node);
    let default_width = input.config.flowchart.wrapping_width;
    let (image_width, image_height) = image_size(node, default_width)?;
    let top = label_on_top(node);
    let label = icon_label(input);
    let label_padding = if node.label.is_empty() {
        0.0
    } else {
        LABEL_PADDING
    };
    let stroke_width = util::stroke_width(&styles);

    let shape = rough::rectangle(
        -image_width / 2.0,
        -image_height / 2.0,
        image_width,
        image_height,
    );
    let outer_width = image_width.max(label.bbox.width);
    let outer_height = image_height + label.bbox.height + label_padding;
    let outer = rough::rectangle(
        -outer_width / 2.0,
        -outer_height / 2.0,
        outer_width,
        outer_height,
    );
    let shift = if top {
        label.bbox.height / 2.0 + label_padding / 2.0
    } else {
        -label.bbox.height / 2.0 - label_padding / 2.0
    };
    let label_offset = Point::new(
        -label.bbox.width / 2.0 - label.bbox.x,
        if top {
            -image_height / 2.0 - label.bbox.height / 2.0 - label_padding / 2.0
        } else {
            image_height / 2.0 - label.bbox.height / 2.0 + label_padding / 2.0
        },
    );

    let mut bbox = BBox::new(
        -outer_width / 2.0,
        -outer_height / 2.0,
        outer_width,
        outer_height,
    )
    .union(BBox::new(
        -image_width / 2.0,
        -image_height / 2.0 + shift,
        image_width,
        image_height,
    ))
    .union(label.bbox.translate(label_offset.x, label_offset.y));
    let mut after_label = outline_group(&outer, &stroke_width, false);
    if let Some(img) = node.img.as_deref().filter(|img| !img.is_empty()) {
        let image_y = if top {
            outer_height / 2.0 - image_height
        } else {
            -outer_height / 2.0
        };
        after_label.push_str(&format!(
            "<image href=\"{}\" width=\"{}\" height=\"{}\" preserveAspectRatio=\"none\" transform=\"translate({}, {})\"/>",
            escape(img),
            number(image_width),
            number(image_height),
            number(-image_width / 2.0),
            number(image_y),
        ));
        bbox = bbox.union(BBox::new(
            -image_width / 2.0,
            image_y,
            image_width,
            image_height,
        ));
    }
    let intersect = icon_intersect(
        node,
        top,
        (image_width, image_height),
        &label,
        label_padding,
        bbox.height,
    );
    Ok(Part {
        svg: shifted_outline_group(&shape, &stroke_width, true, shift),
        after_label,
        bbox,
        intersect,
        label_offset: Some(label_offset),
        label: Some(label),
    })
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{draw_box, node, theme};
    use super::*;
    use crate::flowchart::shapes::draw;
    use ferrigura_model::config::MermaidConfig;

    fn labelled(shape: &str, icon: Option<&str>) -> Node {
        let mut node = node(shape, "neo");
        node.label = "A".to_owned();
        node.icon = icon.map(str::to_owned);
        node
    }

    #[test]
    fn the_icon_size_is_the_larger_asset_side_or_forty_eight() {
        assert_eq!(icon_size(&Node::default()), 48.0);
        let sized = Node {
            asset_width: Some(30.0),
            asset_height: Some(60.0),
            ..Node::default()
        };
        assert_eq!(icon_size(&sized), 60.0);
    }

    #[test]
    fn an_icon_square_is_the_icon_plus_half_the_padding_on_each_side() {
        // The icon is 48 and the square adds 7.5 on each side: 63. A narrow label leaves the
        // width at the square's, and the height is the square, the label and the 8 between them.
        let node = labelled("iconSquare", Some("fa:bell"));
        let label = LabelLayout::of_box(BBox::new(-10.0, 0.0, 20.0, 17.0));
        let shape = draw_box(&node, &label);
        assert!((shape.bbox.width - 63.0).abs() < 1e-9, "{:?}", shape.bbox);
        assert!(
            (shape.bbox.height - (63.0 + 17.0 + 8.0)).abs() < 1e-9,
            "{:?}",
            shape.bbox
        );
    }

    #[test]
    fn an_icon_without_a_name_is_a_bare_square_of_the_icon_size() {
        let node = labelled("icon", None);
        let shape = draw_box(
            &node,
            &LabelLayout::of_box(BBox::new(-10.0, 0.0, 20.0, 17.0)),
        );
        assert!(!shape.after_label.contains("<svg"), "{}", shape.after_label);
        assert!((shape.bbox.width - 48.0).abs() < 1e-9, "{:?}", shape.bbox);
    }

    #[test]
    fn an_icon_name_is_the_fallback_icon_of_the_reference() {
        let node = labelled("icon", Some("fa:bell"));
        let shape = draw_box(
            &node,
            &LabelLayout::of_box(BBox::new(-10.0, 0.0, 20.0, 17.0)),
        );
        assert!(
            shape.after_label.contains("#087ebf"),
            "{}",
            shape.after_label
        );
        assert!(
            shape.after_label.contains("width=\"48\""),
            "{}",
            shape.after_label
        );
    }

    #[test]
    fn the_icon_outline_is_moved_by_the_label_shift_as_the_reference_moves_it() {
        // A 17 high label under the square: the square moves up by half the label and the
        // padding, 8.5 + 4 (`iconShape.attr('transform')`).
        let node = labelled("iconSquare", Some("fa:bell"));
        let shape = draw_box(
            &node,
            &LabelLayout::of_box(BBox::new(-10.0, 0.0, 20.0, 17.0)),
        );
        assert!(
            shape
                .svg
                .starts_with("<g transform=\"translate(0, -12.5)\">"),
            "{}",
            shape.svg
        );
    }

    #[test]
    fn an_image_without_its_asset_sizes_is_an_error_as_ferrigura_reads_no_images() {
        let mut image = node("imageSquare", "neo");
        image.img = Some("https://example.com/a.png".to_owned());
        let config = MermaidConfig::default();
        let theme = theme();
        let result = draw(&ShapeInput {
            node: &image,
            label: None,
            config: &config,
            theme: &theme,
            dir: Some("TB"),
        });
        assert!(result.is_err());
    }
}
