//! The shapes with waves on their edges: `waveEdgedRectangle` (`doc`), `linedWaveEdgedRect`
//! (`lin-doc`), `multiWaveEdgedRectangle` (`docs`), `taggedWaveEdgedRectangle` (`tag-doc`) and
//! `waveRectangle` (`flag`). Ports of the handlers in `shapes/`, with the sine waves of
//! `generateFullSineWavePoints`. The reference draws each outline with rough.js at roughness 0,
//! so it is the polygon through its points (`rough`). See
//! `docs-port/flowchart/render-notes/shapes-core.md` §2.1 and §2.4.

use super::rough;
use super::util::{self, Part, label_helper, styles2string};
use super::{Intersect, ShapeInput};
use crate::unified::geom::Point;

/// The narrowest a document's outline is: the reference widens a narrower one to this
/// (`waveEdgedRectangle.ts`, `minWidth`).
const MIN_DOCUMENT_WIDTH: f64 = 14.0;
/// The offset of the second and third outlines of a stacked document, in pixels.
const STACK_OFFSET: f64 = 10.0;

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

/// `waveEdgedRectangle.ts`: a rectangle whose bottom edge is a wave, with the label above the
/// middle of the box. The neo look pads the label by 16 and 12 and makes the wave a quarter of
/// the height, the classic look pads by the padding and makes it an eighth.
pub(super) fn wave_edged_rectangle(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let (pad_x, pad_y) = if node.is_neo() {
        (16.0, 12.0)
    } else {
        (node.padding, node.padding)
    };
    let label = label_helper(input, false);
    let w = label.bbox.width + 2.0 * pad_x;
    let h = label.bbox.height + 2.0 * pad_y;
    let wave = if node.is_neo() { h / 4.0 } else { h / 8.0 };
    let final_h = h + wave;
    let width_difference = MIN_DOCUMENT_WIDTH - w;
    let extra_w = if width_difference > 0.0 {
        width_difference / 2.0
    } else {
        0.0
    };

    let mut points = vec![Point::new(-w / 2.0 - extra_w, final_h / 2.0)];
    points.extend(util::generate_full_sine_wave_points(
        -w / 2.0 - extra_w,
        final_h / 2.0,
        w / 2.0 + extra_w,
        final_h / 2.0,
        wave,
        0.8,
    ));
    points.push(Point::new(w / 2.0 + extra_w, -final_h / 2.0));
    points.push(Point::new(-w / 2.0 - extra_w, -final_h / 2.0));

    let outline = rough::polygon(&points);
    let style = util::path_style(node, &styles);
    let svg = group(
        "basic label-container outer-path",
        Some(Point::new(0.0, -wave / 2.0)),
        &rough::paths(
            &outline,
            Some(style.as_str()),
            &util::stroke_width(&styles),
            true,
        ),
    );
    let label_offset = Point::new(
        -w / 2.0 + node.padding - label.bbox.x,
        -h / 2.0 + node.padding - wave - label.bbox.y,
    );
    Part {
        svg,
        after_label: String::new(),
        bbox: outline.bbox.translate(0.0, -wave / 2.0),
        intersect: Intersect::Polygon { points },
        label_offset: Some(label_offset),
        label: Some(label),
    }
}

/// `linedWaveEdgedRect.ts`: a wave-edged rectangle with a vertical line near each side, drawn
/// as one outline that runs out to the sides and back. Its sides stick out by a tenth of half
/// the width.
pub(super) fn lined_wave_edged_rect(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let (pad_x, pad_y) = if node.is_neo() {
        (16.0, 12.0)
    } else {
        (node.padding, node.padding)
    };
    let label = label_helper(input, false);
    let w = label.bbox.width + 2.0 * pad_x;
    let h = label.bbox.height + 2.0 * pad_y;
    let wave = if node.is_neo() { h / 4.0 } else { h / 8.0 };
    let final_h = h + wave;

    let left = -w / 2.0 - (w / 2.0) * 0.1;
    let right = w / 2.0 + (w / 2.0) * 0.1;
    let mut points = vec![
        Point::new(left, -final_h / 2.0),
        Point::new(left, final_h / 2.0),
    ];
    points.extend(util::generate_full_sine_wave_points(
        left,
        final_h / 2.0,
        right,
        final_h / 2.0,
        wave,
        0.8,
    ));
    points.extend([
        Point::new(right, -final_h / 2.0),
        Point::new(left, -final_h / 2.0),
        Point::new(-w / 2.0, -final_h / 2.0),
        Point::new(-w / 2.0, (final_h / 2.0) * 1.1),
        Point::new(-w / 2.0, -final_h / 2.0),
    ]);

    let outline = rough::polygon(&points);
    let style = util::path_style(node, &styles);
    let svg = group(
        "basic label-container outer-path",
        Some(Point::new(0.0, -wave / 2.0)),
        &rough::paths(
            &outline,
            Some(style.as_str()),
            &util::stroke_width(&styles),
            true,
        ),
    );
    let label_offset = Point::new(
        -w / 2.0 + node.padding + ((w / 2.0) * 0.1) / 2.0 - label.bbox.x,
        -h / 2.0 + node.padding - wave - label.bbox.y,
    );
    Part {
        svg,
        after_label: String::new(),
        bbox: outline.bbox.translate(0.0, -wave / 2.0),
        intersect: Intersect::Polygon { points },
        label_offset: Some(label_offset),
        label: Some(label),
    }
}

/// `multiWaveEdgedRectangle.ts`: a stack of two wave-edged documents, the back one offset up and
/// to the right. The neo look pads the label by 16 and 12 (three times 12 high) and makes the
/// wave a quarter of the height; the classic look pads by the padding.
pub(super) fn multi_wave_edged_rectangle(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let label = label_helper(input, false);
    let (pad_x, pad_y) = if node.is_neo() {
        (16.0, 12.0)
    } else {
        (node.padding, node.padding)
    };
    let w = label.bbox.width + 2.0 * pad_x;
    let h = label.bbox.height + 3.0 * pad_y;
    let wave = if node.is_neo() { h / 4.0 } else { h / 8.0 };
    let final_h = h + wave / 2.0;
    let x = -w / 2.0;
    let y = -final_h / 2.0;
    let offset = STACK_OFFSET;

    let wave_points = util::generate_full_sine_wave_points(
        x - offset,
        y + final_h + offset,
        x + w - offset,
        y + final_h + offset,
        wave,
        0.8,
    );
    let last = wave_points.last().copied().unwrap_or_default();
    let mut outer = vec![
        Point::new(x - offset, y + offset),
        Point::new(x - offset, y + final_h + offset),
    ];
    outer.extend(wave_points);
    outer.extend([
        Point::new(x + w - offset, last.y - offset),
        Point::new(x + w, last.y - offset),
        Point::new(x + w, last.y - 2.0 * offset),
        Point::new(x + w + offset, last.y - 2.0 * offset),
        Point::new(x + w + offset, y - offset),
        Point::new(x + offset, y - offset),
        Point::new(x + offset, y),
        Point::new(x, y),
        Point::new(x, y + offset),
    ]);
    let inner = [
        Point::new(x, y + offset),
        Point::new(x + w - offset, y + offset),
        Point::new(x + w - offset, last.y - offset),
        Point::new(x + w, last.y - offset),
        Point::new(x + w, y),
        Point::new(x, y),
    ];

    let outer_outline = rough::polygon(&outer);
    let inner_outline = rough::polygon(&inner);
    let style = util::path_style(node, &styles);
    let stroke = util::stroke_width(&styles);
    let body = format!(
        "{}<g>{}</g>",
        rough::paths(&outer_outline, Some(style.as_str()), &stroke, true),
        rough::paths(&inner_outline, Some(style.as_str()), &stroke, true),
    );
    let svg = group(
        "basic label-container outer-path",
        Some(Point::new(0.0, -wave / 2.0)),
        &body,
    );
    let label_offset = Point::new(
        -(label.bbox.width / 2.0) - offset - label.bbox.x,
        -(label.bbox.height / 2.0) + offset - wave - label.bbox.y,
    );
    let bbox = outer_outline
        .bbox
        .union(inner_outline.bbox)
        .translate(0.0, -wave / 2.0);
    Part {
        svg,
        after_label: String::new(),
        bbox,
        intersect: Intersect::Polygon { points: outer },
        label_offset: Some(label_offset),
        label: Some(label),
    }
}

/// `taggedWaveEdgedRectangle.ts`: a wave-edged document with a folded tag in its bottom right
/// corner, which the reference draws as a second, filled outline. The padding and the wave are
/// the same in both looks: the wave is an eighth of the height.
pub(super) fn tagged_wave_edged_rectangle(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let label = label_helper(input, false);
    let w = (label.bbox.width + 2.0 * node.padding).max(node.width.unwrap_or(0.0));
    let h = (label.bbox.height + 2.0 * node.padding).max(node.height.unwrap_or(0.0));
    let wave = h / 8.0;
    let tag_width = 0.2 * w;
    let tag_height = 0.2 * h;
    let final_h = h + wave;
    let extra = (w / 2.0) * 0.1;

    let mut points = vec![Point::new(-w / 2.0 - extra, final_h / 2.0)];
    points.extend(util::generate_full_sine_wave_points(
        -w / 2.0 - extra,
        final_h / 2.0,
        w / 2.0 + extra,
        final_h / 2.0,
        wave,
        0.8,
    ));
    points.push(Point::new(w / 2.0 + extra, -final_h / 2.0));
    points.push(Point::new(-w / 2.0 - extra, -final_h / 2.0));

    let x = -w / 2.0 + extra;
    let y = -final_h / 2.0 - tag_height * 0.4;
    let mut tag_points = vec![
        Point::new(x + w - tag_width, (y + h) * 1.3),
        Point::new(x + w, y + h - tag_height),
        Point::new(x + w, (y + h) * 0.9),
    ];
    tag_points.extend(util::generate_full_sine_wave_points(
        x + w,
        (y + h) * 1.25,
        x + w - tag_width,
        (y + h) * 1.3,
        -h * 0.02,
        0.5,
    ));

    let outline = rough::polygon(&points);
    let tag = rough::polygon(&tag_points);
    let style = util::path_style(node, &styles);
    let stroke = util::stroke_width(&styles);
    let body = format!(
        "<g>{}</g>{}",
        rough::paths(&outline, Some(style.as_str()), &stroke, true),
        rough::paths(&tag, Some(style.as_str()), &stroke, true),
    );
    let svg = group(
        "basic label-container outer-path",
        Some(Point::new(0.0, -wave / 2.0)),
        &body,
    );
    let label_offset = Point::new(
        -w / 2.0 + node.padding - label.bbox.x,
        -h / 2.0 + node.padding - wave / 2.0 - label.bbox.y,
    );
    let bbox = outline.bbox.union(tag.bbox).translate(0.0, -wave / 2.0);
    Part {
        svg,
        after_label: String::new(),
        bbox,
        intersect: Intersect::Polygon { points },
        label_offset: Some(label_offset),
        label: Some(label),
    }
}

/// `waveRectangle.ts` (`flag`): a rectangle with a wave on its top and bottom edges. The label is
/// centred as the default shapes centre theirs. The neo look pads the label by 16 and 20; the
/// classic look by the padding.
pub(super) fn wave_rectangle(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let (pad_x, pad_y) = if node.is_neo() {
        (16.0, 20.0)
    } else {
        (node.padding, node.padding)
    };
    let label = label_helper(input, false);
    let w = label.bbox.width + 2.0 * pad_x;
    let h = label.bbox.height + pad_y;
    let wave = h / 8.0;
    let final_h = h + 2.0 * wave;

    let mut points = vec![Point::new(-w / 2.0, final_h / 2.0)];
    points.extend(util::generate_full_sine_wave_points(
        -w / 2.0,
        final_h / 2.0,
        w / 2.0,
        final_h / 2.0,
        wave,
        1.0,
    ));
    points.push(Point::new(w / 2.0, -final_h / 2.0));
    points.extend(util::generate_full_sine_wave_points(
        w / 2.0,
        -final_h / 2.0,
        -w / 2.0,
        -final_h / 2.0,
        wave,
        -1.0,
    ));

    let outline = rough::polygon(&points);
    let style = util::path_style(node, &styles);
    let svg = group(
        "basic label-container",
        None,
        &rough::paths(
            &outline,
            Some(style.as_str()),
            &util::stroke_width(&styles),
            true,
        ),
    );
    Part::with_centred_label(svg, outline.bbox, Intersect::Polygon { points }, label)
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{draw_box, node};
    use super::*;
    use crate::unified::geom::BBox;
    use crate::unified::label::LabelLayout;

    fn drawn(shape: &str, look: &str) -> crate::unified::shapes::ShapeOutput {
        let label = LabelLayout::of_box(BBox::new(-60.0, 0.0, 120.0, 17.0));
        draw_box(&node(shape, look), &label)
    }

    #[test]
    fn a_neo_document_is_the_label_plus_the_neo_padding_and_a_quarter_wave() {
        let shape = drawn("doc", "neo");
        // w = 120 + 32 = 152; h = 17 + 24 = 41 with a wave of h/4 on either side of the midline.
        assert!((shape.bbox.width - 152.0).abs() < 1e-9, "{:?}", shape.bbox);
        assert!(shape.bbox.height >= 41.0, "{:?}", shape.bbox);
    }

    #[test]
    fn a_stacked_document_is_offset_by_the_stack_offset_on_the_right() {
        let shape = drawn("docs", "classic");
        // The label is 120 wide and the classic look pads it by 15 on each side; the stack is
        // offset by 10 on each side of the front document.
        let w = 120.0 + 30.0;
        assert!(
            (shape.bbox.width - (w + 2.0 * STACK_OFFSET)).abs() < 1e-9,
            "{:?}",
            shape.bbox
        );
    }

    #[test]
    fn a_flag_has_its_label_centred_by_default() {
        let shape = drawn("flag", "neo");
        assert_eq!(shape.label_offset, Some(Point::new(0.0, -8.5)));
    }
}
