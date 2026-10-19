//! The container shapes: `folder` (a rectangle with a tab), `bucket` (an open container with an
//! elliptical rim), `browser` and `consoleWindow` (a rounded window with a title bar). Ports of
//! `shapes/folder.ts`, `bucket.ts`, `browser.ts` and `console.ts`, for the neo and classic looks.
//! See `docs-port/flowchart/render-notes/shapes-core.md` §2.4.

use ferrigura_jsmath::{cos, sin};

use super::arcs::arc_points;
use super::util::{Part, attribute, create_path_from_points, label_helper, styles2string};
use super::{Intersect, ShapeInput};
use crate::svg::{escape, number};
use crate::unified::geom::{BBox, Point};

/// The accent colour of the window and bucket outlines: the theme's node border, else its line
/// colour, else the current colour (`themeVariables?.nodeBorder ?? lineColor ?? 'currentColor'`).
fn accent(input: &ShapeInput<'_>) -> String {
    input
        .config
        .theme
        .node_border
        .text()
        .or_else(|| input.config.theme.line_color.text())
        .unwrap_or_else(|| "currentColor".to_owned())
}

/// `folder.ts`: a rectangle with a tab on its top left, around the label. The path is written as
/// it is, so its box is the box of its points.
pub(super) fn folder(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let label = label_helper(input, false);
    let padding = node.padding;
    let w = (label.bbox.width + padding * 2.0)
        .max(node.width.unwrap_or(0.0))
        .max(90.0);
    let content_height = label.bbox.height + padding * 2.0;
    let tab_height = (content_height * 0.16).clamp(8.0, 14.0);
    let total_height = (content_height + tab_height).max(node.height.unwrap_or(0.0));
    let body_height = total_height - tab_height;
    let tab_width = (w * 0.38).max(28.0);
    let top = -total_height / 2.0;

    let points = [
        Point::new(-w / 2.0, top),
        Point::new(-w / 2.0 + tab_width, top),
        Point::new(-w / 2.0 + tab_width, top + tab_height),
        Point::new(w / 2.0, top + tab_height),
        Point::new(w / 2.0, total_height / 2.0),
        Point::new(-w / 2.0, total_height / 2.0),
    ];
    let svg = format!(
        "<path d=\"{}\" class=\"basic label-container\"{}/>",
        create_path_from_points(&points),
        attribute("style", &styles.node),
    );
    let body_centre_y = top + tab_height + body_height / 2.0;
    let label_offset = Point::new(
        -label.bbox.width / 2.0 - label.bbox.x,
        body_centre_y - label.bbox.height / 2.0 - label.bbox.y,
    );
    Part {
        svg,
        after_label: String::new(),
        bbox: BBox::from_points(points).unwrap_or_default(),
        intersect: Intersect::Polygon {
            points: points.to_vec(),
        },
        label_offset: Some(label_offset),
        label: Some(label),
    }
}

/// `bucket.ts`: an open container that narrows to a curved base, with an elliptical rim at its
/// top. The body is a path of lines and elliptical arcs and the rim an ellipse; the intersect
/// follows the rim's upper arc and the base's lower arc.
pub(super) fn bucket(input: &ShapeInput<'_>) -> Part {
    const ARC_SEGMENTS: usize = 12;
    let node = input.node;
    let styles = styles2string(node);
    let label = label_helper(input, false);
    let accent = accent(input);
    let padding = node.padding;
    let w = (label.bbox.width + padding * 2.0)
        .max(node.width.unwrap_or(0.0))
        .max(80.0);
    let rim_ry = (w * 0.08).clamp(5.0, 12.0);
    let total_height = (label.bbox.height + padding * 2.0 + rim_ry).max(node.height.unwrap_or(0.0));
    let top_y = -total_height / 2.0 + rim_ry;
    let bottom_y = total_height / 2.0;
    let bottom_w = w * 0.72;

    let left_top = Point::new(-w / 2.0, top_y);
    let left_bottom = Point::new(-bottom_w / 2.0, bottom_y);
    let right_bottom = Point::new(bottom_w / 2.0, bottom_y);
    let right_top = Point::new(w / 2.0, top_y);
    // `M`, `L`, `A`, `L`, `A` and `Z`: the arcs are drawn from the bottom left to the bottom right,
    // and from the top right back to the top left, both with a sweep of 0.
    let mut body_points = vec![left_top, left_bottom];
    body_points.extend(arc_points(
        left_bottom,
        right_bottom,
        bottom_w / 2.0,
        rim_ry,
        0.0,
        false,
        false,
    ));
    body_points.push(right_top);
    body_points.extend(arc_points(
        right_top,
        left_top,
        w / 2.0,
        rim_ry,
        0.0,
        false,
        false,
    ));
    let body_box = BBox::from_points(body_points).unwrap_or_default();
    let body_d = format!(
        "M{},{} L{},{} A{},{} 0 0 0 {},{} L{},{} A{},{} 0 0 0 {},{} Z",
        number(left_top.x),
        number(left_top.y),
        number(left_bottom.x),
        number(left_bottom.y),
        number(bottom_w / 2.0),
        number(rim_ry),
        number(right_bottom.x),
        number(right_bottom.y),
        number(right_top.x),
        number(right_top.y),
        number(w / 2.0),
        number(rim_ry),
        number(left_top.x),
        number(left_top.y),
    );
    let ellipse_box = BBox::new(-w / 2.0, top_y - rim_ry, w, 2.0 * rim_ry);
    let svg = format!(
        "<g class=\"basic label-container\"><path d=\"{body_d}\"{}/><ellipse cx=\"0\" cy=\"{}\" rx=\"{}\" ry=\"{}\"{}/></g>",
        attribute("style", &styles.node),
        number(top_y),
        number(w / 2.0),
        number(rim_ry),
        attribute(
            "style",
            &format!("fill:none;stroke:{accent};stroke-width:1px")
        ),
    );

    // The outline for edge intersection: the rim's upper arc and the base's lower arc, joined by
    // the tapered sides, each arc in 12 segments (`bucket.ts`).
    let outline: Vec<Point> = rim_arc(w / 2.0, top_y, -1.0, rim_ry, ARC_SEGMENTS)
        .into_iter()
        .chain(
            rim_arc(bottom_w / 2.0, bottom_y, 1.0, rim_ry, ARC_SEGMENTS)
                .into_iter()
                .rev(),
        )
        .collect();

    let body_centre_y = top_y + (bottom_y - top_y) / 2.0;
    let label_offset = Point::new(
        -label.bbox.width / 2.0 - label.bbox.x,
        body_centre_y - label.bbox.height / 2.0 - label.bbox.y,
    );
    Part {
        svg,
        after_label: String::new(),
        bbox: body_box.union(ellipse_box),
        intersect: Intersect::Polygon { points: outline },
        label_offset: Some(label_offset),
        label: Some(label),
    }
}

/// One half of a bucket's rim: the half ellipse of radius `rx` by `rim_ry` centred on `cy`, on the
/// side `y_sign` gives, from the left end to the right end (`bucket.ts`).
fn rim_arc(rx: f64, cy: f64, y_sign: f64, rim_ry: f64, segments: usize) -> Vec<Point> {
    (0..=segments)
        .map(|index| {
            let theta =
                std::f64::consts::PI - (index as f64 * std::f64::consts::PI) / segments as f64;
            Point::new(rx * cos(theta), cy + y_sign * rim_ry * sin(theta))
        })
        .collect()
}

/// `browser.ts`: a rounded window with a title bar, three dots and an address bar, around the
/// label. The window's box is its outline; the bar, dots and address field lie inside it.
pub(super) fn browser(input: &ShapeInput<'_>) -> Part {
    window(input, WindowKind::Browser)
}

/// `console.ts`: a rounded terminal window with a command prompt in its top left corner.
pub(super) fn console(input: &ShapeInput<'_>) -> Part {
    window(input, WindowKind::Console)
}

/// The two windows: the browser's title bar and the console's glyph band.
#[derive(Clone, Copy)]
enum WindowKind {
    Browser,
    Console,
}

/// A window with its band across the top, as `browser.ts` and `console.ts` draw it. The band is 18
/// high for the browser and 20 for the console, and the window is 12 across the corners.
fn window(input: &ShapeInput<'_>, kind: WindowKind) -> Part {
    const RADIUS: f64 = 12.0;
    let node = input.node;
    let styles = styles2string(node);
    let label = label_helper(input, false);
    let accent = accent(input);
    let padding = node.padding;
    let band = match kind {
        WindowKind::Browser => 18.0,
        WindowKind::Console => 20.0,
    };
    let w = (label.bbox.width + padding * 2.0)
        .max(node.width.unwrap_or(0.0))
        .max(90.0);
    let h = (label.bbox.height + padding * 2.0 + band).max(node.height.unwrap_or(0.0));
    let top = -h / 2.0;

    let rect = format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" ry=\"{}\"{}/>",
        number(-w / 2.0),
        number(top),
        number(w),
        number(h),
        number(RADIUS),
        number(RADIUS),
        attribute("style", &styles.node),
    );
    let mut outline = BBox::new(-w / 2.0, top, w, h);
    let mut svg = format!("<g class=\"basic label-container\">{rect}");
    match kind {
        WindowKind::Browser => {
            let line_y = top + band;
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"{}/>",
                number(-w / 2.0),
                number(line_y),
                number(w / 2.0),
                number(line_y),
                attribute("style", &format!("stroke:{accent};stroke-width:1px")),
            ));
            outline = outline.union(
                BBox::from_points([Point::new(-w / 2.0, line_y), Point::new(w / 2.0, line_y)])
                    .unwrap_or_default(),
            );
            for index in 0..3 {
                let cx = -w / 2.0 + 12.0 + f64::from(index) * 9.0;
                let cy = top + band / 2.0;
                svg.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"2.5\"{}/>",
                    number(cx),
                    number(cy),
                    attribute("style", &format!("fill:{accent};stroke:none")),
                ));
                outline = outline.union(BBox::new(cx - 2.5, cy - 2.5, 5.0, 5.0));
            }
            let (x, y) = (-w / 2.0 + 44.0, top + 4.0);
            let (bar_width, bar_height) = ((w - 56.0).max(10.0), band - 8.0);
            svg.push_str(&format!(
                "<rect class=\"browser-address-bar\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" ry=\"3\"{}/>",
                number(x),
                number(y),
                number(bar_width),
                number(bar_height),
                attribute(
                    "style",
                    &format!("fill:none;stroke:{accent};stroke-width:1px;opacity:0.6")
                ),
            ));
            outline = outline.union(BBox::new(x, y, bar_width, bar_height));
        }
        WindowKind::Console => {
            // The prompt's glyphs lie inside the window: their box does not reach its edges, so
            // the window's box is the outline's.
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" class=\"console-glyph\"{}>{}</text>",
                number(-w / 2.0 + 12.0),
                number(top + 16.0),
                attribute(
                    "style",
                    &format!("font-family:monospace;font-weight:bold;font-size:14px;fill:{accent}")
                ),
                escape(">_"),
            ));
        }
    }
    svg.push_str("</g>");

    // The label sits in the window's body, below the band.
    let body_centre_y = top + band + (h - band) / 2.0;
    let label_offset = Point::new(
        -label.bbox.width / 2.0 - label.bbox.x,
        body_centre_y - label.bbox.height / 2.0 - label.bbox.y,
    );
    Part {
        svg,
        after_label: String::new(),
        bbox: outline,
        intersect: Intersect::Rect,
        label_offset: Some(label_offset),
        label: Some(label),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rim_arc_runs_from_one_end_of_its_ellipse_to_the_other() {
        let arc = rim_arc(10.0, 0.0, 1.0, 4.0, 12);
        let first = arc.first().copied().expect("a point");
        let last = arc.last().copied().expect("a point");
        assert!((first.x + 10.0).abs() < 1e-9, "{first:?}");
        assert!((last.x - 10.0).abs() < 1e-9, "{last:?}");
        assert_eq!(arc.len(), 13);
    }
}
