//! The lane of a swimlane diagram: a port of the `swimlane` cluster shape in
//! `packages/mermaid/src/rendering-util/rendering-elements/clusters/swimlane.js`. A lane is a title
//! band and a body, two rectangles, and the label in the band. The band is on top of the lane, or on
//! its left for `LR`, where the label is turned.
//!
//! The lane is drawn after the layout has placed it, and it takes its own width from its title when
//! the title and its padding need more room than the layout gave (`swimlane.js` 62-69). The handDrawn
//! look is drawn with rough.js in the reference; this module draws the classic rectangles for it,
//! which is outside the port's scope.

use super::Config;
use super::clusters::Cluster;
use super::data::Node;
use super::geom::{BBox, Point};
use super::label::{self, Anchor, LabelSpec};
use super::palette::{self, node_style, palette_slot, style_list};
use crate::RenderError;
use crate::svg::{escape, number};

/// The wrap width of a lane's title when the layout gave the lane no width: the default of
/// `createText` (`width = 200`), which `swimlane.js` 40-45 gets by passing `node.width` as undefined.
const DEFAULT_TITLE_WIDTH: f64 = 200.0;

/// The font size of the labels when the theme gives none (the schema's default).
const DEFAULT_FONT_SIZE: f64 = 16.0;

/// The padding of the title band above and below its label in `LR`, `titlePaddingY` (`swimlane.js` 86).
/// `TB` takes none, so that the first row of nodes sits lower (`swimlane.js` 159-160).
const LR_TITLE_PADDING_Y: f64 = 4.0;

/// Paints a lane (`swimlane`, `swimlane.js` 16-246): its body and title band, which are rectangles,
/// and its label. Returns the lane's box, which the edges clip to, and the box of its label.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read.
pub fn paint(node: &Node, config: &Config<'_>) -> Result<Cluster, RenderError> {
    let lane = node.swimlane.clone().unwrap_or_default();
    let theme = config.theme;
    let styles = style_list(&node.label_style);
    let title = label::layout(&LabelSpec {
        text: &node.label,
        label_type: node.label_type,
        width: node.width.unwrap_or(DEFAULT_TITLE_WIDTH),
        is_node: true,
        anchor: Anchor::Start,
        wrap_in_style: false,
        add_background: false,
        styles: &styles,
        font_size: theme.font_size.px().unwrap_or(DEFAULT_FONT_SIZE),
        title: false,
    })?;
    let text = title.bbox;

    // `node.padding ?? 0`, and the lane's width: its title and padding, when they need more than the
    // layout gave the lane (`swimlane.js` 62-69).
    let padding = node.padding;
    let (x, y) = (node.x.unwrap_or(0.0), node.y.unwrap_or(0.0));
    let height = node.height.unwrap_or(0.0);
    let laid_out_width = node.width.unwrap_or(0.0);
    let width = if laid_out_width <= text.width + padding {
        text.width + padding
    } else {
        laid_out_width
    };

    let lane_top = y - height / 2.0;
    let lane_bottom = y + height / 2.0;
    let lane_left = x - width / 2.0;
    // The top of the lanes' content, which the layout wrote back, or a third of the lane (`swimlane.js` 78).
    let content_top = lane.content_top.unwrap_or(lane_top + height / 3.0);

    // The title band's size and the body's, and where the label's box sits (`swimlane.js` 92-224).
    let is_lr = lane.direction == "LR";
    let (title_band, body, label_transform, title_box) = if is_lr {
        // `Math.max(desiredTitleSize, bbox.height + 2 * titlePaddingY)` (`swimlane.js` 160-163) is the same value.
        let title_width = text.height + 2.0 * LR_TITLE_PADDING_Y;
        let body_x = lane_left + title_width;
        let body_width = (width - title_width).max(0.0);
        let centre = Point::new(lane_left + title_width / 2.0, y);
        let transform = format!(
            "translate({}, {}) rotate(-90) translate({}, {})",
            number(centre.x),
            number(centre.y),
            number(-text.width / 2.0),
            number(-text.height / 2.0),
        );
        (
            BBox::new(lane_left, lane_top, title_width, height),
            BBox::new(body_x, lane_top, body_width, height),
            transform,
            turned_box(centre, text),
        )
    } else {
        let header_max_height = (content_top - lane_top).max(0.0);
        let title_height = text.height.min(header_max_height);
        let body_y = lane_top + title_height;
        let content_height = (lane_bottom - body_y).max(0.0);
        let label_x = x - text.width / 2.0;
        let label_y = lane_top + (title_height - text.height) / 2.0;
        (
            BBox::new(lane_left, lane_top, width, title_height),
            BBox::new(lane_left, body_y, width, content_height),
            format!("translate({}, {})", number(label_x), number(label_y)),
            text.translate(label_x, label_y),
        )
    };

    let fill = theme.cluster_bkg.text();
    let stroke = theme.cluster_border.text();
    let style = escape(&node_style(node));
    let palette = palette::palette(config.globals, theme);
    let colour = palette
        .as_ref()
        .and_then(|palette| palette_slot(node.color_index, palette))
        .map_or_else(String::new, |slot| {
            format!(" data-color-id=\"color-{slot}\"")
        });
    let look = if node.look.is_empty() {
        String::new()
    } else {
        format!(" data-look=\"{}\"", escape(&node.look))
    };
    let stroke_attribute = optional_attribute("stroke", stroke.as_deref());
    // The body is drawn first, then the title band: each is inserted as the first child (`swimlane.js`
    // 126-156), and the label comes last.
    let body_svg = rect_svg(
        "swimlane-body",
        &style,
        body,
        &optional_attribute("fill", Some("none")),
        &stroke_attribute,
    );
    let title_svg = rect_svg(
        "swimlane-title",
        &style,
        title_band,
        &optional_attribute("fill", fill.as_deref()),
        &stroke_attribute,
    );
    let label_svg = title.svg("");
    let svg = format!(
        "<g class=\"cluster swimlane {classes}\" id=\"{id}\" data-id=\"{data_id}\" data-et=\"cluster\"{look}{colour}>{body_svg}{title_svg}<g class=\"cluster-label swimlane-label\" transform=\"{transform}\">{label_svg}</g></g>",
        classes = escape(&node.css_classes),
        id = escape(&node.dom_id),
        data_id = escape(&node.id),
        transform = label_transform,
    );
    Ok(Cluster {
        svg,
        bbox: BBox::new(lane_left, lane_top, width, height),
        title_box,
        label: Some(title),
    })
}

/// A lane rectangle, with its style, its box, and its fill and stroke as written attributes, each with
/// its leading space (the palette's rules override the presentation attributes).
fn rect_svg(class: &str, style: &str, bbox: BBox, fill: &str, stroke: &str) -> String {
    format!(
        "<rect class=\"{class}\" style=\"{style}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{fill}{stroke}/>",
        number(bbox.x),
        number(bbox.y),
        number(bbox.width),
        number(bbox.height),
    )
}

/// An attribute that the reference sets only when its value exists: d3 removes an attribute that is set
/// to `undefined`, and the theme's colour is `undefined` when the theme does not give one.
fn optional_attribute(name: &str, value: Option<&str>) -> String {
    value.map_or_else(String::new, |value| {
        format!(" {name}=\"{}\"", escape(value))
    })
}

/// The box a label's `<text>` covers once its group is turned by `rotate(-90)` about `centre`, as
/// `getBBox` of the lane reports it: the label is first moved by minus half its own size, then turned,
/// then moved to the centre (`swimlane.js` 213-224).
fn turned_box(centre: Point, text: BBox) -> BBox {
    let (half_width, half_height) = (text.width / 2.0, text.height / 2.0);
    let corners = [
        (text.x, text.y),
        (text.x + text.width, text.y),
        (text.x, text.y + text.height),
        (text.x + text.width, text.y + text.height),
    ]
    .into_iter()
    .map(|(x, y)| {
        // translate(-w/2, -h/2), then rotate(-90): (x, y) becomes (y, -x), then translate to the centre.
        let (x, y) = (x - half_width, y - half_height);
        Point::new(y + centre.x, -x + centre.y)
    });
    BBox::from_points(corners).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_turned_label_covers_its_rotated_box() {
        // A 10 by 4 label turned about (50, 50): it is 4 wide and 10 tall around the centre.
        let bbox = turned_box(Point::new(50.0, 50.0), BBox::new(0.0, 0.0, 10.0, 4.0));
        assert_eq!(bbox, BBox::new(48.0, 45.0, 4.0, 10.0));
    }
}
