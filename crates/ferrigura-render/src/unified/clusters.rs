//! Subgraph frames: a port of the `rect` cluster in
//! `packages/mermaid/src/rendering-util/rendering-elements/clusters.js`, for the `neo` and
//! `classic` looks. See `docs-port/flowchart/render-notes/labels.md` §13 and `styles.md`.
//!
//! The frame is the laid-out box of the subgraph, widened to its title when the title and the
//! padding do not fit. The title is a label laid out with the wrapping of `clusters.js`, placed on
//! the top edge. The colours of the frame are the presentation attributes of decision 0007; the
//! rules of `style.rs` give the same colours through custom properties.

use ferrigura_model::flowchart::LabelType;
use ferrigura_model::theme::ThemeTokens;

use super::Config;
use super::data::Node;
use super::geom::{BBox, Point};
use super::label::{self, Anchor, LabelLayout, LabelSpec};
use super::palette::{self as style, Palette};
use crate::RenderError;
use crate::svg::{escape, number};

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
    /// The box of its title's text in absolute coordinates. A title is text, so `getBBox` counts it
    /// even when the frame has no height.
    pub title_box: BBox,
    /// The title as it is painted, so that the coordinator can subset the fonts for its runs
    /// (`LabelLayout::text_runs`).
    pub label: Option<LabelLayout>,
}

/// Paints a laid-out subgraph, as `insertCluster` does with the shape of the node: the `rect` frame
/// of a flowchart subgraph, or one of the state diagram's cluster kinds (`clusters.js`).
///
/// The node's `id` of the element is its `dom_id` as given: the caller prefixes it with the diagram's
/// id first, as `render.ts` does for every node, so that the ids of the subgraphs are unique in the
/// page.
///
/// # Errors
///
/// Returns an error if the subgraph has no position or size after the layout, or if a bundled font
/// cannot be read.
pub fn paint(node: &Node, config: &Config<'_>) -> Result<Cluster, RenderError> {
    match node.shape.as_str() {
        "roundedWithTitle" => rounded_with_title(node, config),
        "noteGroup" => Ok(note_group(node)),
        "divider" => Ok(divider(node, config)),
        "usecaseSystemBoundary" => super::shapes::usecase_boundary::paint(node, config),
        _ => rect(node, config),
    }
}

/// Whether a subgraph's shape is one of the state diagram's cluster kinds, which have no entry in the
/// shape registry (`clusters.js` 806-815). Their labels are still measured as the others' are.
#[must_use]
pub fn is_cluster_kind(shape: &str) -> bool {
    matches!(
        shape,
        "roundedWithTitle" | "noteGroup" | "divider" | "usecaseSystemBoundary"
    )
}

/// The `rect` cluster: the frame of a flowchart subgraph, with its title.
fn rect(node: &Node, config: &Config<'_>) -> Result<Cluster, RenderError> {
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
        wrap_in_style: false,
        styles: &styles,
        font_size: config.theme.font_size.px().unwrap_or(DEFAULT_FONT_SIZE),
        title: false,
    })?;
    let frame = frame(node, title.bbox, title_top_margin(config))?;

    let palette = style::palette(config.globals, config.theme);
    let slot = palette
        .as_ref()
        .and_then(|palette| style::palette_slot(node.color_index, palette));
    let (fill, stroke) = presentation_colours(node, config.theme, palette.as_ref(), slot);
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
    let title_box = title.bbox.translate(frame.title.x, frame.title.y);
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
        title_box,
        label: Some(title),
    })
}

/// The colour slot of a container as its `[data-color-id]` attribute and its palette colours
/// (`stampColorSlot`, `colorThemeGate.ts`): the attribute is written only when the theme has a
/// palette and the container has a slot.
fn colour_slot(node: &Node, config: &Config<'_>) -> (Option<usize>, String) {
    let palette = style::palette(config.globals, config.theme);
    let slot = palette
        .as_ref()
        .and_then(|palette| style::palette_slot(node.color_index, palette));
    let colour_id = slot.map_or_else(String::new, |slot| {
        format!(" data-color-id=\"color-{slot}\"")
    });
    (slot, colour_id)
}

/// The composite state (`roundedWithTitle`, `clusters.js` 175-306, classic and neo looks): an outer
/// rect in its own group, the title's label, and an inner rect for the body, under the title. The
/// title is `createLabel`, the plain text with the `row` class, and it is placed at the subgraph's
/// left edge plus the title's offset.
fn rounded_with_title(node: &Node, config: &Config<'_>) -> Result<Cluster, RenderError> {
    let (_, colour_id) = colour_slot(node, config);
    let title = label::layout(&LabelSpec {
        text: &node.label,
        label_type: LabelType::Text,
        width: f64::INFINITY,
        is_node: true,
        add_background: false,
        // A cluster title has no `text-anchor` of its own: its lines start at the label's origin.
        anchor: Anchor::Start,
        wrap_in_style: false,
        styles: &[],
        font_size: config.theme.font_size.px().unwrap_or(DEFAULT_FONT_SIZE),
        title: false,
    })?;
    let (Some(layout_width), Some(layout_height), Some(x), Some(y)) =
        (node.width, node.height, node.x, node.y)
    else {
        return Err(RenderError::Layout(format!(
            "the subgraph {} has no position or size after the layout",
            node.id
        )));
    };
    let padding = 0.0;
    let text_width = title.bbox.width;
    let width = if layout_width <= text_width + node.padding {
        text_width + node.padding
    } else {
        layout_width
    } + padding;
    let height = layout_height + padding;
    let inner_height = layout_height + padding - title.bbox.height - 6.0;
    let outer_x = x - width / 2.0;
    let outer_y = y - height / 2.0;
    let inner_y = y - layout_height / 2.0 + title.bbox.height + 2.0;
    let label_x = x - text_width / 2.0;
    let label_y = outer_y + 1.0 - 3.0;
    let outer = BBox::new(outer_x, outer_y, width, height);
    let svg = format!(
        "<g class=\"{classes}\" id=\"{id}\" data-id=\"{node_id}\" data-look=\"{look}\"{colour_id}>\
         <g><rect class=\"outer\" x=\"{outer_x}\" y=\"{outer_y}\" width=\"{width}\" height=\"{height}\" data-look=\"{look}\"/></g>\
         <g class=\"cluster-label\" transform=\"translate({label_x}, {label_y})\">{title}</g>\
         <rect class=\"inner\" x=\"{outer_x}\" y=\"{inner_y}\" width=\"{width}\" height=\"{inner_height}\"/>\
         </g>",
        classes = escape(&node.css_classes),
        id = escape(&node.dom_id),
        node_id = escape(&node.id),
        look = escape(&node.look),
        outer_x = number(outer_x),
        outer_y = number(outer_y),
        width = number(width),
        height = number(height),
        inner_y = number(inner_y),
        inner_height = number(inner_height),
        label_x = number(label_x),
        label_y = number(label_y),
        title = title.svg(""),
    );
    Ok(Cluster {
        svg,
        bbox: outer,
        title_box: title.bbox.translate(label_x, label_y),
        label: Some(title),
    })
}

/// The note group of a composite's note (`noteGroup`, `clusters.js` 144-173): an unfilled rect
/// around the note, in a group of its own. It has no title.
fn note_group(node: &Node) -> Cluster {
    let width = node.width.unwrap_or(0.0);
    let height = node.height.unwrap_or(0.0);
    let x = node.x.unwrap_or(0.0) - width / 2.0;
    let y = node.y.unwrap_or(0.0) - height / 2.0;
    let bbox = BBox::new(x, y, width, height);
    let svg = format!(
        "<g class=\"note-cluster\" id=\"{id}\"><rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" fill=\"none\"/></g>",
        id = escape(&node.dom_id),
        x = number(x),
        y = number(y),
        width = number(width),
        height = number(height),
    );
    Cluster {
        svg,
        bbox,
        title_box: BBox::default(),
        label: None,
    }
}

/// A concurrency region (`divider`, `clusters.js` 419-522): a rect with the `divider` class, in a
/// group of its own, and no title. Its colour slot is its parent's, see `dataFetcher.ts`.
fn divider(node: &Node, config: &Config<'_>) -> Cluster {
    let (_, colour_id) = colour_slot(node, config);
    let width = node.width.unwrap_or(0.0);
    let height = node.height.unwrap_or(0.0);
    let x = node.x.unwrap_or(0.0) - width / 2.0;
    let y = node.y.unwrap_or(0.0) - height / 2.0;
    let bbox = BBox::new(x, y, width, height);
    let svg = format!(
        "<g class=\"{classes}\" id=\"{id}\" data-look=\"{look}\"{colour_id}><g><rect class=\"divider\" x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" data-look=\"{look}\"/></g></g>",
        classes = escape(&node.css_classes),
        id = escape(&node.dom_id),
        look = escape(&node.look),
        x = number(x),
        y = number(y),
        width = number(width),
        height = number(height),
    );
    Cluster {
        svg,
        bbox,
        title_box: BBox::default(),
        label: None,
    }
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
fn title_top_margin(config: &Config<'_>) -> f64 {
    config
        .section
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
    theme: &ThemeTokens,
    palette: Option<&Palette>,
    slot: Option<usize>,
) -> (Option<String>, Option<String>) {
    let fill = match (palette, slot) {
        (Some(palette), Some(slot)) if !palette.background.is_empty() => {
            Some(palette.background[slot % palette.background.len()].clone())
        }
        _ => theme.cluster_bkg.text(),
    };
    let stroke = match (palette, slot) {
        (Some(palette), Some(slot)) => Some(palette.border[slot].clone()),
        _ if node.look == "neo" => {
            if theme.use_gradient.truthy() {
                None
            } else {
                theme.node_border.text()
            }
        }
        _ => theme.cluster_border.text(),
    };
    (fill, stroke)
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
        let fixture = Fixture::new(config("redux-color", "neo"), &theme);
        let config = fixture.config();
        let palette = style::palette(config.globals, config.theme);
        let node = Node {
            color_index: Some(1),
            ..subgraph(200.0, 140.0, 0.0, 0.0)
        };
        let slot = palette
            .as_ref()
            .and_then(|palette| style::palette_slot(node.color_index, palette));
        assert_eq!(
            presentation_colours(
                &node,
                &ThemeTokens::from_value(&theme),
                palette.as_ref(),
                slot
            ),
            (Some("#F0FDFA".to_owned()), Some("#2DD4BF".to_owned()))
        );
    }

    #[test]
    fn a_dark_palette_frame_is_filled_with_the_cluster_background() {
        let theme = Theme::ReduxDarkColor.variables(None);
        let fixture = Fixture::new(config("redux-dark-color", "neo"), &theme);
        let config = fixture.config();
        let palette = style::palette(config.globals, config.theme);
        let node = Node {
            color_index: Some(0),
            ..subgraph(200.0, 140.0, 0.0, 0.0)
        };
        let slot = palette
            .as_ref()
            .and_then(|palette| style::palette_slot(node.color_index, palette));
        assert_eq!(
            presentation_colours(
                &node,
                &ThemeTokens::from_value(&theme),
                palette.as_ref(),
                slot
            ),
            (Some("#1E1A2E".to_owned()), Some("#E879F9".to_owned()))
        );
    }

    #[test]
    fn a_neo_frame_without_a_slot_takes_the_node_border() {
        let theme = Theme::ReduxColor.variables(None);
        let node = subgraph(200.0, 140.0, 0.0, 0.0);
        assert_eq!(
            presentation_colours(&node, &ThemeTokens::from_value(&theme), None, None),
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
            presentation_colours(&node, &ThemeTokens::from_value(&theme), None, None),
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
            presentation_colours(&node, &ThemeTokens::from_value(&theme), None, None),
            (Some("#F9F9FB".to_owned()), None)
        );
    }
}
