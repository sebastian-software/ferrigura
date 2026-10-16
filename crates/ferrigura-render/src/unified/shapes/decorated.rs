//! The rectangles with a decoration: `multiRect` (`st-rect`), `shadedProcess` (`lin-rect`),
//! `slopedRect` (`sl-rect`), `taggedRect` (`tag-rect`) and `windowPane` (`win-pane`). Ports of the
//! handlers in `shapes/`. Their outlines are rough.js paths at roughness 0, so each is the
//! polygon through its points (`rough`). See `docs-port/flowchart/render-notes/shapes-core.md`
//! §2.1 and §2.4.

use super::rough::{self, Outline};
use super::util::{self, Part, attribute, label_helper, styles2string};
use super::{Intersect, ShapeInput};
use crate::unified::geom::{BBox, Point, sloped_outline_height_f32};

/// The width of the frame of a shaded process, on its left side.
const FRAME_WIDTH: f64 = 8.0;
/// The size of a tag, as a share of the height of the tagged rectangle.
const TAG_RATIO: f64 = 0.2;
/// The offset of the outlines behind a stacked rectangle: 10 in the neo look, 5 in the classic.
const RECT_OFFSET_NEO: f64 = 10.0;
const RECT_OFFSET_CLASSIC: f64 = 5.0;
/// The offset of a window pane's outline from its box, in pixels (the same in both looks).
const WINDOW_OFFSET: f64 = 10.0;

/// The `<g>` of an outline's paths, with its class and an optional translation.
fn group(class: &str, translate: Option<Point>, body: &str) -> String {
    let transform = translate.map_or_else(String::new, |offset| {
        format!(
            " transform=\"translate({}, {})\"",
            crate::svg::number(offset.x),
            crate::svg::number(offset.y)
        )
    });
    format!("<g class=\"{class}\"{transform}>{body}</g>")
}

/// The group that `mergePaths` returns for an outline: one path holding the data of the rough paths,
/// the fill's first, joined by a space (`util.ts`, `mergePaths`).
fn merged_path(outline: &Outline, style: &str, stroke_width: &str) -> String {
    let data: Vec<String> = [outline.fill.as_deref(), outline.stroke.as_deref()]
        .into_iter()
        .flatten()
        .map(rough::data)
        .collect();
    format!(
        "<g><path d=\"{}\" stroke-width=\"{stroke_width}\" stroke-dasharray=\"0 0\"{}/></g>",
        data.join(" "),
        attribute("style", style),
    )
}

/// `multiRect.ts` (`st-rect`): a rectangle with two more outlines behind it, offset up and to the
/// right. The outline is a single path per outline. The neo look offsets by 10 and pads by 16 and
/// 12; the classic look offsets by 5 and pads by the padding.
pub(super) fn multi_rect(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let neo = node.is_neo();
    let label = label_helper(input, false);
    let (pad_x, pad_y, offset) = if neo {
        (16.0, 12.0, RECT_OFFSET_NEO)
    } else {
        (node.padding, node.padding, RECT_OFFSET_CLASSIC)
    };
    let total_width = label.bbox.width + pad_x * 2.0 + 2.0 * offset;
    let total_height = label.bbox.height + pad_y * 2.0 + 2.0 * offset;
    let w = total_width - 2.0 * offset;
    let h = total_height - 2.0 * offset;
    let x = -w / 2.0;
    let y = -h / 2.0;

    let outer = vec![
        Point::new(x - offset, y + offset),
        Point::new(x - offset, y + h + offset),
        Point::new(x + w - offset, y + h + offset),
        Point::new(x + w - offset, y + h),
        Point::new(x + w, y + h),
        Point::new(x + w, y + h - offset),
        Point::new(x + w + offset, y + h - offset),
        Point::new(x + w + offset, y - offset),
        Point::new(x + offset, y - offset),
        Point::new(x + offset, y),
        Point::new(x, y),
        Point::new(x, y + offset),
    ];
    let inner = [
        Point::new(x, y + offset),
        Point::new(x + w - offset, y + offset),
        Point::new(x + w - offset, y + h),
        Point::new(x + w, y + h),
        Point::new(x + w, y),
        Point::new(x, y),
    ];

    let outer_outline = rough::closed_polygon(&outer, rough::Paint::BOTH);
    let inner_outline = rough::closed_polygon(&inner, rough::Paint::BOTH);
    let style = util::path_style(node, &styles);
    let stroke = util::stroke_width(&styles);
    let svg = group(
        "basic label-container outer-path",
        None,
        &format!(
            "{}{}",
            merged_path(&outer_outline, &style, &stroke),
            merged_path(&inner_outline, &style, &stroke),
        ),
    );
    let label_offset = Point::new(
        -(label.bbox.width / 2.0) - offset - label.bbox.x,
        -(label.bbox.height / 2.0) + offset - label.bbox.y,
    );
    Part {
        svg,
        after_label: String::new(),
        bbox: outer_outline.bbox.union(inner_outline.bbox),
        intersect: Intersect::Polygon { points: outer },
        label_offset: Some(label_offset),
        label: Some(label),
    }
}

/// `shadedProcess.ts` (`lin-rect`): a rectangle with a frame line on its left side, the frame
/// reaching out 8 to the left. The neo look pads by 16 and 12 and the frame is 8 wide; the
/// classic look pads by the padding and the frame is 16 wide.
pub(super) fn shaded_process(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let (pad_x, pad_y) = if node.is_neo() {
        (16.0, 12.0)
    } else {
        (node.padding, node.padding)
    };
    let label = label_helper(input, false);
    let total_width = label.bbox.width
        + pad_x * 2.0
        + if node.is_neo() {
            FRAME_WIDTH
        } else {
            FRAME_WIDTH * 2.0
        };
    let total_height = label.bbox.height + pad_y * 2.0;
    let w = total_width - FRAME_WIDTH;
    let h = total_height;
    let x = FRAME_WIDTH - total_width / 2.0;
    let y = -total_height / 2.0;

    let points = vec![
        Point::new(x, y),
        Point::new(x + w, y),
        Point::new(x + w, y + h),
        Point::new(x - FRAME_WIDTH, y + h),
        Point::new(x - FRAME_WIDTH, y),
        Point::new(x, y),
        Point::new(x, y + h),
    ];
    let outline = rough::polygon(&points, rough::Paint::BOTH);
    let style = util::path_style(node, &styles);
    let svg = group(
        "basic label-container outer-path",
        None,
        &rough::paths(&outline, Some(style.as_str()), &util::stroke_width(&styles)),
    );
    let label_offset = Point::new(
        FRAME_WIDTH / 2.0 - label.bbox.width / 2.0 - label.bbox.x,
        -(label.bbox.height / 2.0) - label.bbox.y,
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

/// `slopedRect.ts` (`sl-rect`): a rectangle whose right side slopes up to half its height above
/// the top, drawn as an open outline and moved down by a quarter of its height. The neo look
/// pads by 16 and 12; the classic look by the padding.
pub(super) fn sloped_rect(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let (pad_x, pad_y) = if node.is_neo() {
        (16.0, 12.0)
    } else {
        (node.padding, node.padding)
    };
    let label = label_helper(input, false);
    let w = label.bbox.width + pad_x * 2.0;
    let total_height = (label.bbox.height + pad_y * 2.0) * 1.5;
    let h = total_height / 1.5;
    let x = -w / 2.0;
    let y = -h / 2.0;

    let points = vec![
        Point::new(x, y),
        Point::new(x, y + h),
        Point::new(x + w, y + h),
        Point::new(x + w, y - h / 2.0),
    ];
    let outline = rough::closed_polygon(&points, rough::Paint::BOTH);
    // `updateNodeBounds(node, polygon)` sizes the node by the `getBBox` of the outline's own group,
    // which has no transform of its own, so the height is the browser's height of the outline's
    // path (`geom::sloped_outline_height_f32`), not the height of the translated box.
    let height = sloped_outline_height_f32(outline.bbox.y, outline.bbox.bottom());
    let style = util::path_style(node, &styles);
    let svg = group(
        "basic label-container outer-path",
        Some(Point::new(0.0, h / 4.0)),
        &rough::paths(&outline, Some(style.as_str()), &util::stroke_width(&styles)),
    );
    let label_offset = Point::new(
        -w / 2.0 + node.padding - label.bbox.x,
        -h / 4.0 + node.padding - label.bbox.y,
    );
    Part {
        svg,
        after_label: String::new(),
        bbox: BBox::new(
            outline.bbox.x,
            outline.bbox.y + h / 4.0,
            outline.bbox.width,
            height,
        ),
        intersect: Intersect::Polygon { points },
        label_offset: Some(label_offset),
        label: Some(label),
    }
}

/// `taggedRect.ts` (`tag-rect`): a rectangle with a tag in its bottom right corner, drawn as a
/// rectangle and a filled triangle of the same outline. The label is centred by default. The neo
/// look pads by 16 and 12.
pub(super) fn tagged_rect(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let (pad_x, pad_y) = if node.is_neo() {
        (16.0, 12.0)
    } else {
        (node.padding, node.padding)
    };
    let label = label_helper(input, false);
    let total_height = label.bbox.height + pad_y * 2.0;
    let tag_width = TAG_RATIO * total_height;
    let tag_height = TAG_RATIO * total_height;
    let total_width = label.bbox.width + pad_x * 2.0 + tag_width;
    let w = total_width - tag_width;
    let h = total_height;
    let x = -w / 2.0;
    let y = -h / 2.0;

    let rect_points = [
        Point::new(x - tag_width / 2.0, y),
        Point::new(x + w + tag_width / 2.0, y),
        Point::new(x + w + tag_width / 2.0, y + h),
        Point::new(x - tag_width / 2.0, y + h),
    ];
    let tag_points = [
        Point::new(x + w - tag_width / 2.0, y + h),
        Point::new(x + w + tag_width / 2.0, y + h),
        Point::new(x + w + tag_width / 2.0, y + h - tag_height),
    ];
    let rect = rough::closed_polygon(&rect_points, rough::Paint::BOTH);
    let tag = rough::closed_polygon(&tag_points, rough::Paint::BOTH);
    let style = util::path_style(node, &styles);
    let stroke = util::stroke_width(&styles);
    let svg = group(
        "basic label-container outer-path",
        None,
        &format!(
            "<g>{}</g>{}",
            rough::paths(&rect, Some(style.as_str()), &stroke),
            rough::paths(&tag, Some(style.as_str()), &stroke),
        ),
    );
    Part::with_centred_label(
        svg,
        rect.bbox.union(tag.bbox),
        Intersect::Polygon {
            points: rect_points.to_vec(),
        },
        label,
    )
}

/// `windowPane.ts` (`win-pane`): a rectangle with a cross of two lines, drawn as an outline that
/// stands 10 out of the box on its top and left and is moved 5 down and to the right. The neo
/// look pads by 16 and 12; the classic look by the padding.
pub(super) fn window_pane(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let (pad_x, pad_y) = if node.is_neo() {
        (16.0, 12.0)
    } else {
        (node.padding, node.padding)
    };
    let label = label_helper(input, false);
    let total_width = label.bbox.width + pad_x * 2.0 + WINDOW_OFFSET;
    let total_height = label.bbox.height + pad_y * 2.0 + WINDOW_OFFSET;
    let w = total_width - WINDOW_OFFSET;
    let h = total_height - WINDOW_OFFSET;
    let x = -w / 2.0;
    let y = -h / 2.0;
    let o = WINDOW_OFFSET;

    // `windowPane.ts`: the frame's sides and the two bars, as one rc.path of `M L ... M L ... M L`
    // with no `Z`. Its fill has one sub-path of its own (the frame), so the fill is an open polyline
    // through the frame's points (`generator.js`, `path`).
    let at = Point::new;
    let segments = [
        rough::Segment::Move(at(x - o, y - o)),
        rough::Segment::Line(at(x + w, y - o)),
        rough::Segment::Line(at(x + w, y + h)),
        rough::Segment::Line(at(x - o, y + h)),
        rough::Segment::Line(at(x - o, y - o)),
        rough::Segment::Move(at(x - o, y)),
        rough::Segment::Line(at(x + w, y)),
        rough::Segment::Move(at(x, y - o)),
        rough::Segment::Line(at(x, y + h)),
    ];
    let outline = rough::path(&segments, rough::Paint::BOTH);
    let style = util::path_style(node, &styles);
    let svg = group(
        "basic label-container outer-path",
        Some(Point::new(WINDOW_OFFSET / 2.0, WINDOW_OFFSET / 2.0)),
        &rough::paths(&outline, Some(style.as_str()), &util::stroke_width(&styles)),
    );
    let label_offset = Point::new(
        -(label.bbox.width / 2.0) + WINDOW_OFFSET / 2.0 - label.bbox.x,
        -(label.bbox.height / 2.0) + WINDOW_OFFSET / 2.0 - label.bbox.y,
    );
    Part {
        svg,
        after_label: String::new(),
        bbox: outline
            .bbox
            .translate(WINDOW_OFFSET / 2.0, WINDOW_OFFSET / 2.0),
        intersect: Intersect::Polygon {
            points: vec![
                Point::new(x - o, y - o),
                Point::new(x - o, y + h),
                Point::new(x + w, y + h),
                Point::new(x + w, y - o),
            ],
        },
        label_offset: Some(label_offset),
        label: Some(label),
    }
}
