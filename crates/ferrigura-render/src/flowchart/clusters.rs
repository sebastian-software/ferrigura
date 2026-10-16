//! Subgraph frames: a port of the `rect` cluster in
//! `packages/mermaid/src/rendering-util/rendering-elements/clusters.js`, for the `neo` and
//! `classic` looks. See `docs-port/flowchart/render-notes/labels.md` §13 and `styles.md`.
//!
//! The frame is the laid-out box of the subgraph, widened to its title when the title and the
//! padding do not fit. The title is a label laid out with the wrapping of `clusters.js`, placed on
//! the top edge. The colours of the frame are the presentation attributes of decision 0007; the
//! rules of `style.rs` give the same colours through custom properties.

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::flowchart::LabelType;
use ferrigura_model::value::Value;

use super::data::Node;
use super::geom::{BBox, Point};
use super::label::{self, Anchor, LabelLayout, LabelSpec};
use super::style::{self, Palette};
use crate::RenderError;
use crate::svg::{escape, number};
use crate::variables::{px, text_of, truthy};

/// The wrap width of a Markdown title that the layout has not sized yet (`createText`'s default).
const MARKDOWN_TITLE_WIDTH: f64 = 200.0;

/// The font size of the labels when the theme gives none (the schema's default).
const DEFAULT_FONT_SIZE: f64 = 16.0;

/// A painted subgraph.
#[derive(Debug, Clone, PartialEq)]
pub struct Cluster {
    /// The `<g class="cluster">` element.
    pub svg: String,
    /// Its box in absolute coordinates.
    pub bbox: BBox,
    /// The title as it is painted, so that the coordinator can subset the fonts for its runs
    /// (`LabelLayout::text_runs`).
    pub label: Option<LabelLayout>,
}

/// Paints a laid-out subgraph: its frame and its title, as `insertCluster` does.
///
/// The node's `id` of the element is its `dom_id` as given: the caller prefixes it with the diagram's
/// id first, as `render.ts` does for every node, so that the ids of the subgraphs are unique in the
/// page.
///
/// # Errors
///
/// Returns an error if the subgraph has no position or size after the layout, or if a bundled font
/// cannot be read.
pub fn paint(node: &Node, config: &MermaidConfig, theme: &Value) -> Result<Cluster, RenderError> {
    let styles = style::style_list(&node.label_style);
    let width = match node.label_type {
        LabelType::Markdown => node.width.unwrap_or(MARKDOWN_TITLE_WIDTH),
        _ => f64::INFINITY,
    };
    let title = label::layout(&LabelSpec {
        text: &node.label,
        label_type: node.label_type,
        width,
        is_node: true,
        add_background: false,
        // No rule centres a cluster title: its lines start at the label's origin.
        anchor: Anchor::Start,
        styles: &styles,
        font_size: px(theme, "fontSize").unwrap_or(DEFAULT_FONT_SIZE),
    })?;
    let frame = frame(node, title.bbox, title_top_margin(config))?;

    let palette = style::palette(config, theme);
    let slot = palette
        .as_ref()
        .and_then(|palette| style::palette_slot(node.color_index, palette));
    let (fill, stroke) = presentation_colours(node, theme, palette.as_ref(), slot);
    let colour_id = slot.map_or_else(String::new, |slot| {
        format!(" data-color-id=\"color-{slot}\"")
    });
    let style = style::node_style(node);
    let style = if style.is_empty() {
        String::new()
    } else {
        format!(" style=\"{}\"", escape(&style))
    };
    let fill = fill.map_or_else(String::new, |fill| format!(" fill=\"{}\"", escape(&fill)));
    let stroke = stroke.map_or_else(String::new, |stroke| {
        format!(" stroke=\"{}\"", escape(&stroke))
    });
    let bbox = frame.bbox;
    let svg = format!(
        "<g class=\"cluster {classes}\" id=\"{id}\" data-look=\"{look}\"{colour_id}>\
         <rect{style} x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\"{fill}{stroke}/>\
         <g class=\"cluster-label \" transform=\"translate({tx}, {ty})\">{title}</g>\
         </g>",
        classes = escape(&node.css_classes),
        id = escape(&node.dom_id),
        look = escape(&node.look),
        x = number(bbox.x),
        y = number(bbox.y),
        width = number(bbox.width),
        height = number(bbox.height),
        tx = number(frame.title.x),
        ty = number(frame.title.y),
        title = title.svg(""),
    );
    Ok(Cluster {
        svg,
        bbox,
        label: Some(title),
    })
}

/// The frame of a rect cluster and the place of its title.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Frame {
    /// The rect, in absolute coordinates.
    bbox: BBox,
    /// The top-left corner of the title's group.
    title: Point,
}

/// The frame of a cluster: its laid-out centre and height, and the width of its layout, widened to
/// the title plus the padding when that is wider (`clusters.js` 53-72). The title is centred on the
/// frame's centre line, `title_top_margin` below the top edge.
fn frame(node: &Node, title: BBox, title_top_margin: f64) -> Result<Frame, RenderError> {
    let (Some(layout_width), Some(height), Some(x), Some(y)) =
        (node.width, node.height, node.x, node.y)
    else {
        return Err(RenderError::Layout(format!(
            "the subgraph {} has no position or size after the layout",
            node.id
        )));
    };
    let padding = node.padding;
    let width = if layout_width <= title.width + padding {
        title.width + padding
    } else {
        layout_width
    };
    Ok(Frame {
        bbox: BBox::new(x - width / 2.0, y - height / 2.0, width, height),
        title: Point::new(x - title.width / 2.0, y - height / 2.0 + title_top_margin),
    })
}

/// The top margin of a subgraph title (`getSubGraphTitleMargins`): zero unless the configuration sets it.
fn title_top_margin(config: &MermaidConfig) -> f64 {
    config
        .flowchart
        .sub_graph_title_margin
        .as_ref()
        .and_then(|margin| margin.top)
        .unwrap_or(0) as f64
}

/// The fill and the stroke of a frame as presentation attributes (decision 0007). They are the
/// colours the rules give the frame: the palette slot's fill and border, else the cluster background
/// and border, where a neo frame takes the node border (`[data-look="neo"].cluster rect`). A neo
/// frame with a gradient stroke gets no stroke attribute, since the gradient has no colour of its own
/// here.
fn presentation_colours(
    node: &Node,
    theme: &Value,
    palette: Option<&Palette>,
    slot: Option<usize>,
) -> (Option<String>, Option<String>) {
    let fill = match (palette, slot) {
        (Some(palette), Some(slot)) if !palette.background.is_empty() => {
            Some(palette.background[slot % palette.background.len()].clone())
        }
        _ => text_of(theme, "clusterBkg"),
    };
    let stroke = match (palette, slot) {
        (Some(palette), Some(slot)) => Some(palette.border[slot].clone()),
        _ if node.look == "neo" => {
            if truthy(theme, "useGradient") {
                None
            } else {
                text_of(theme, "nodeBorder")
            }
        }
        _ => text_of(theme, "clusterBorder"),
    };
    (fill, stroke)
}

#[cfg(test)]
mod tests {
    use ferrigura_model::config::{defaults, typed};
    use ferrigura_model::theme::Theme;

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

    /// A subgraph of the given layout: its centre, its size and its padding.
    fn subgraph(width: f64, height: f64, x: f64, y: f64) -> Node {
        Node {
            id: "group".to_owned(),
            is_group: true,
            look: "neo".to_owned(),
            padding: 8.0,
            width: Some(width),
            height: Some(height),
            x: Some(x),
            y: Some(y),
            ..Node::default()
        }
    }

    /// The box of a title of the given size, as a label of that size measures it.
    fn title(width: f64, height: f64) -> BBox {
        LabelLayout::of_box(BBox::new(0.0, 0.0, width, height)).bbox
    }

    #[test]
    fn the_frame_keeps_its_layout_width_when_the_title_and_padding_fit() {
        let frame = frame(
            &subgraph(200.0, 140.0, 150.0, 100.0),
            title(100.0, 18.0),
            0.0,
        )
        .expect("a positioned subgraph");
        assert_eq!(frame.bbox, BBox::new(50.0, 30.0, 200.0, 140.0));
        assert_eq!(frame.title, Point::new(100.0, 30.0));
    }

    #[test]
    fn the_frame_grows_to_the_title_and_its_padding_when_the_layout_is_narrower() {
        let frame = frame(
            &subgraph(50.0, 140.0, 150.0, 100.0),
            title(100.0, 18.0),
            0.0,
        )
        .expect("a positioned subgraph");
        assert_eq!(frame.bbox, BBox::new(96.0, 30.0, 108.0, 140.0));
        assert_eq!(frame.title, Point::new(100.0, 30.0));
    }

    #[test]
    fn the_title_moves_down_by_the_top_margin_and_the_frame_does_not() {
        let frame = frame(
            &subgraph(200.0, 140.0, 150.0, 100.0),
            title(100.0, 18.0),
            5.0,
        )
        .expect("a positioned subgraph");
        assert_eq!(frame.bbox.y, 30.0);
        assert_eq!(frame.title.y, 35.0);
    }

    #[test]
    fn a_subgraph_without_a_position_is_an_error() {
        let node = Node {
            x: None,
            ..subgraph(200.0, 140.0, 0.0, 0.0)
        };
        assert!(frame(&node, title(10.0, 10.0), 0.0).is_err());
    }

    #[test]
    fn a_palette_slot_gives_the_frame_its_fill_and_border() {
        let theme = Theme::ReduxColor.variables(None);
        let config = config("redux-color", "neo");
        let palette = style::palette(&config, &theme);
        let node = Node {
            color_index: Some(1),
            ..subgraph(200.0, 140.0, 0.0, 0.0)
        };
        let slot = palette
            .as_ref()
            .and_then(|palette| style::palette_slot(node.color_index, palette));
        assert_eq!(
            presentation_colours(&node, &theme, palette.as_ref(), slot),
            (Some("#F0FDFA".to_owned()), Some("#2DD4BF".to_owned()))
        );
    }

    #[test]
    fn a_dark_palette_frame_is_filled_with_the_cluster_background() {
        let theme = Theme::ReduxDarkColor.variables(None);
        let config = config("redux-dark-color", "neo");
        let palette = style::palette(&config, &theme);
        let node = Node {
            color_index: Some(0),
            ..subgraph(200.0, 140.0, 0.0, 0.0)
        };
        let slot = palette
            .as_ref()
            .and_then(|palette| style::palette_slot(node.color_index, palette));
        assert_eq!(
            presentation_colours(&node, &theme, palette.as_ref(), slot),
            (Some("#1E1A2E".to_owned()), Some("#E879F9".to_owned()))
        );
    }

    #[test]
    fn a_neo_frame_without_a_slot_takes_the_node_border() {
        let theme = Theme::ReduxColor.variables(None);
        let node = subgraph(200.0, 140.0, 0.0, 0.0);
        assert_eq!(
            presentation_colours(&node, &theme, None, None),
            (Some("#F9F9FB".to_owned()), Some("#28253D".to_owned()))
        );
    }

    #[test]
    fn a_classic_frame_takes_the_cluster_border() {
        let theme = Theme::ReduxColor.variables(None);
        let node = Node {
            look: "classic".to_owned(),
            ..subgraph(200.0, 140.0, 0.0, 0.0)
        };
        assert_eq!(
            presentation_colours(&node, &theme, None, None),
            (Some("#F9F9FB".to_owned()), Some("#BDBCCC".to_owned()))
        );
    }

    #[test]
    fn a_gradient_neo_frame_gets_no_stroke_attribute() {
        let mut overrides = Value::empty_object();
        if let Some(object) = overrides.as_object_mut() {
            object.insert("useGradient".to_owned(), Value::Bool(true));
        }
        let theme = Theme::ReduxColor.variables(Some(&overrides));
        let node = subgraph(200.0, 140.0, 0.0, 0.0);
        assert_eq!(
            presentation_colours(&node, &theme, None, None),
            (Some("#F9F9FB".to_owned()), None)
        );
    }
}
