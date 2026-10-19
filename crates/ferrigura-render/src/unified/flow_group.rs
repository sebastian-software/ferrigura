//! The `flowGroup` cluster: the open container of agentflow, a port of `createContainerGroup` and
//! `flowGroup` in `packages/mermaid/src/rendering-util/rendering-elements/clusters.js` (524-657).
//!
//! It is the frame of [`super::clusters`]'s `rect` with the rules of its own: the class `flow-cluster`,
//! a transparent frame with a `0.75px` stroke in the flow container colour, a corner radius of 10,
//! and a title that is laid out without the rect's `cluster-label ` class and without the rect's
//! style attribute (`createContainerGroup` writes neither). The palette slot is stamped on the group
//! as every container's is, and the palette's rules colour the frame.

use ferrigura_model::flowchart::LabelType;

use super::Config;
use super::clusters::Cluster;
use super::data::Node;
use super::geom::{BBox, Point};
use super::label::{self, Anchor, LabelSpec};
use super::palette as style;
use crate::RenderError;
use crate::common::js_trim;
use crate::svg::{escape, number};

/// The wrap width of a Markdown title that the layout has not sized yet (`createText`'s default).
const MARKDOWN_TITLE_WIDTH: f64 = 200.0;

/// The font size of the labels when the theme gives none (the schema's default).
const DEFAULT_FONT_SIZE: f64 = 16.0;

/// The corner radius of the frame (`rx: 10` in `flowGroup`).
const RADIUS: f64 = 10.0;

/// Paints a laid-out `flowGroup` subgraph (`createContainerGroup` with the `flow-cluster` options). The
/// node's `dom_id` is already prefixed with the diagram's id, as the caller does for every node.
///
/// # Errors
///
/// Returns an error if the subgraph has no position or size after the layout, or if a bundled font
/// cannot be read.
pub fn paint(node: &Node, config: &Config<'_>) -> Result<Cluster, RenderError> {
    // The title is measured as `createContainerGroup` measures it: a Markdown title at the node's width,
    // any other unwrapped (`createLabel`, `isNode` true). A blank title is no label (`hasLabel`).
    let has_label = !js_trim(&node.label).is_empty();
    let title = if has_label {
        let width = match node.label_type {
            LabelType::Markdown => node.width.unwrap_or(MARKDOWN_TITLE_WIDTH),
            _ => f64::INFINITY,
        };
        Some(label::layout(&LabelSpec {
            text: &node.label,
            label_type: node.label_type,
            width,
            is_node: true,
            add_background: false,
            // No rule centres a cluster title: its lines start at the label's origin.
            anchor: Anchor::Start,
            wrap_in_style: false,
            styles: &style::style_list(&node.label_style),
            font_size: config.theme.font_size.px().unwrap_or(DEFAULT_FONT_SIZE),
            title: false,
        })?)
    } else {
        None
    };
    let title_width = title.as_ref().map_or(0.0, |title| title.bbox.width);

    let (Some(layout_width), Some(height), Some(x), Some(y)) =
        (node.width, node.height, node.x, node.y)
    else {
        return Err(RenderError::Layout(format!(
            "the subgraph {} has no position or size after the layout",
            node.id
        )));
    };
    // `node.width <= bbox.width + node.padding`: the frame takes the title's width and its padding
    // when the layout is narrower, else the layout's width.
    let width = if layout_width <= title_width + node.padding {
        title_width + node.padding
    } else {
        layout_width
    };
    let bbox = BBox::new(x - width / 2.0, y - height / 2.0, width, height);

    let palette = style::palette(config.globals, config.theme);
    let slot = palette
        .as_ref()
        .and_then(|palette| style::palette_slot(node.color_index, palette));
    let colour_id = slot.map_or_else(String::new, |slot| {
        format!(" data-color-id=\"color-{slot}\"")
    });
    // `flowContainerStroke || secondaryBorderColor` (`flowGroup`, clusters.js 530-531).
    let stroke = config
        .theme
        .flow_container_stroke
        .text()
        .or_else(|| config.theme.secondary_border_color.text());
    let stroke = stroke.map_or_else(String::new, |stroke| {
        format!(" stroke=\"{}\"", escape(&stroke))
    });

    // `labelEl.attr('transform', translate(node.x - bbox.width / 2, node.y - node.height / 2 +
    // subGraphTitleTopMargin))`: the title is centred on the frame's centre line, and it moves down by
    // the top margin.
    let title_box = title.as_ref().map(|title| {
        let origin = Point::new(
            x - title_width / 2.0,
            y - height / 2.0 + title_top_margin(config),
        );
        (origin, title.bbox.translate(origin.x, origin.y))
    });
    let (label_attr, label_svg, title_box_out) = match (&title, title_box) {
        (Some(title), Some((origin, box_))) => (
            format!(
                " transform=\"translate({}, {})\"",
                number(origin.x),
                number(origin.y)
            ),
            title.svg(""),
            box_,
        ),
        _ => (String::new(), String::new(), BBox::default()),
    };

    // `'cluster ' + opts.cssClass + ' ' + node.cssClasses` (`createContainerGroup`), which has no
    // trailing blank when the node has no classes in the reference's output.
    let classes = if node.css_classes.is_empty() {
        String::new()
    } else {
        format!(" {}", escape(&node.css_classes))
    };
    let svg = format!(
        "<g class=\"cluster flow-cluster{classes}\" id=\"{id}\" data-look=\"{look}\"{colour_id}>\
         <rect rx=\"{radius}\" ry=\"{radius}\" x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" fill=\"none\"{stroke} stroke-width=\"0.75px\"/>\
         <g class=\"cluster-label\"{label_attr}>{label_svg}</g>\
         </g>",
        id = escape(&node.dom_id),
        look = escape(&node.look),
        radius = number(RADIUS),
        x = number(bbox.x),
        y = number(bbox.y),
        width = number(bbox.width),
        height = number(bbox.height),
    );
    Ok(Cluster {
        svg,
        bbox,
        title_box: title_box_out,
        label: title,
    })
}

/// The top margin of a subgraph title (`getSubGraphTitleMargins`): zero unless the configuration sets it.
fn title_top_margin(config: &Config<'_>) -> f64 {
    config
        .section
        .flowchart
        .sub_graph_title_margin
        .as_ref()
        .and_then(|margin| margin.top)
        .unwrap_or(0) as f64
}

#[cfg(test)]
mod tests {
    use ferrigura_model::config::{MermaidConfig, defaults, typed};
    use ferrigura_model::theme::Theme;
    use ferrigura_model::value::Value;

    use super::super::Fixture;
    use super::*;

    /// A configuration with the given theme and look, from the defaults.
    fn config(theme: &str, look: &str) -> MermaidConfig {
        let mut value = defaults();
        if let Some(object) = value.as_object_mut() {
            object.insert("theme".to_owned(), Value::from(theme));
            object.insert("look".to_owned(), Value::from(look));
        }
        typed(&value).expect("the defaults read back into the configuration types")
    }

    /// A positioned `flowGroup` with the given label.
    fn group(label: &str) -> Node {
        Node {
            id: "reviewer".to_owned(),
            dom_id: "p0-reviewer".to_owned(),
            label: label.to_owned(),
            is_group: true,
            look: "neo".to_owned(),
            shape: "flowGroup".to_owned(),
            padding: 8.0,
            width: Some(200.0),
            height: Some(140.0),
            x: Some(150.0),
            y: Some(100.0),
            ..Node::default()
        }
    }

    /// The frame and the title of a group are painted with the flow container's rules: a transparent rect
    /// with the flow container stroke, the radius of 10 and a `0.75px` stroke, in a group of the class
    /// `flow-cluster` (`createContainerGroup`).
    #[test]
    fn a_group_is_a_transparent_frame_with_the_flow_container_stroke() {
        let theme = Theme::ReduxColor.variables(None);
        let fixture = Fixture::new(config("default", "neo"), &theme);
        let cluster = paint(&group("Review"), &fixture.config()).expect("a positioned group");
        assert!(
            cluster.svg.starts_with(
                "<g class=\"cluster flow-cluster\" id=\"p0-reviewer\" data-look=\"neo\">"
            ),
            "{}",
            cluster.svg
        );
        assert!(
            cluster.svg.contains(
                "<rect rx=\"10\" ry=\"10\" x=\"50\" y=\"30\" width=\"200\" height=\"140\" fill=\"none\" stroke=\"hsl(-120, 0%, 70%)\" stroke-width=\"0.75px\"/>"
            ),
            "{}",
            cluster.svg
        );
        assert_eq!(cluster.bbox, BBox::new(50.0, 30.0, 200.0, 140.0));
        assert!(
            cluster
                .svg
                .contains("<g class=\"cluster-label\" transform=\"translate(")
        );
    }

    /// A blank title is no label: the title group is written without a transform, and the frame keeps
    /// its layout width (`hasLabel` in `createContainerGroup`).
    #[test]
    fn a_blank_title_is_no_label() {
        let theme = Theme::ReduxColor.variables(None);
        let fixture = Fixture::new(config("default", "neo"), &theme);
        let cluster = paint(&group("  "), &fixture.config()).expect("a positioned group");
        assert!(
            cluster.svg.contains("<g class=\"cluster-label\"></g>"),
            "{}",
            cluster.svg
        );
        assert_eq!(cluster.bbox.width, 200.0);
        assert!(cluster.label.is_none());
    }

    /// Under a colour theme the group carries its palette slot, which the stylesheet's rules match.
    #[test]
    fn a_palette_slot_is_stamped_on_the_group() {
        let theme = Theme::ReduxColor.variables(None);
        let fixture = Fixture::new(config("redux-color", "neo"), &theme);
        let node = Node {
            color_index: Some(7),
            ..group("Review")
        };
        let cluster = paint(&node, &fixture.config()).expect("a positioned group");
        assert!(
            cluster.svg.starts_with(
                "<g class=\"cluster flow-cluster\" id=\"p0-reviewer\" data-look=\"neo\" data-color-id=\"color-7\">"
            ),
            "{}",
            cluster.svg
        );
    }
}
