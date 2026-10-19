//! The system boundary of the use case diagram: a port of `usecaseSystemBoundary` and
//! `getUsecaseSystemBoundaryGeometry` in `rendering-util/rendering-elements/clusters.js` (659-745), in the
//! plain elements of the neo and classic looks.
//!
//! A boundary is a rectangle around its members, with its title at the top. A `package` boundary has a tab
//! above the body that holds the title. The frame is the laid-out box, widened to the title and the
//! padding when they are wider (`max(node.width, title + horizontal padding)`).

use super::super::Config;
use super::super::clusters::Cluster;
use super::super::data::Node;
use super::super::geom::BBox;
use super::super::label::{self, Anchor, LabelSpec};
use super::super::palette;
use super::util::{attribute, styles2string};
use crate::RenderError;
use crate::svg::{escape, number};
use crate::unified::geom::Point;

/// The font size of the labels when the theme gives none.
const DEFAULT_FONT_SIZE: f64 = 16.0;

/// The width a title wraps at when the layout has not sized it: none (`createText` without a width).
const TITLE_WIDTH: f64 = f64::INFINITY;

/// The horizontal padding of a package's title (`clusters.js` 662-663): the title and the tab have 20.
const PACKAGE_TITLE_PADDING: f64 = 20.0;

/// Paints a use case system boundary (`usecaseSystemBoundary`, `clusters.js` 688-745).
///
/// # Errors
///
/// Returns an error if the boundary has no position or size after the layout, or if a bundled font cannot
/// be read.
pub(crate) fn paint(node: &Node, config: &Config<'_>) -> Result<Cluster, RenderError> {
    let (Some(layout_width), Some(layout_height), Some(x), Some(y)) =
        (node.width, node.height, node.x, node.y)
    else {
        return Err(RenderError::Layout(format!(
            "the system boundary {} has no position or size after the layout",
            node.id
        )));
    };
    let styles = styles2string(node);
    let boundary_type = node
        .use_case
        .as_ref()
        .and_then(|fields| fields.boundary_type)
        .unwrap_or(ferrigura_model::use_case::BoundaryType::Rect);
    let package = boundary_type == ferrigura_model::use_case::BoundaryType::Package;
    let font_size = config.theme.font_size.px().unwrap_or(DEFAULT_FONT_SIZE);
    let label_declarations = super::usecase_actor::declarations(&styles.label);
    let title = label::layout(&LabelSpec {
        text: &node.label,
        label_type: node.label_type,
        width: TITLE_WIDTH,
        is_node: true,
        add_background: false,
        anchor: Anchor::Start,
        wrap_in_style: false,
        styles: &label_declarations,
        font_size,
        title: false,
    })?;
    let title_box = title.bbox;

    // `getUsecaseSystemBoundaryGeometry` (`clusters.js` 659-680).
    let horizontal_padding = if package {
        PACKAGE_TITLE_PADDING
    } else {
        node.padding
    };
    let tab_height = if package {
        title_box.height + 10.0
    } else {
        0.0
    };
    let width = layout_width.max(title_box.width + horizontal_padding);
    let height = if package {
        layout_height.max(tab_height + node.padding * 2.0)
    } else {
        layout_height
    };
    let left = x - width / 2.0;
    let top = y - height / 2.0;
    let body_y = top + tab_height;
    let body_height = height - tab_height;
    let tab_width = if package {
        width.min(80.0_f64.max(title_box.width + 20.0))
    } else {
        0.0
    };

    let palette = palette::palette(config.globals, config.theme);
    let slot = palette
        .as_ref()
        .and_then(|palette| palette::palette_slot(node.color_index, palette));
    let colour_id = slot.map_or_else(String::new, |slot| {
        format!(" data-color-id=\"color-{slot}\"")
    });
    let title_position = if package {
        Point::new(
            left + (tab_width - title_box.width) / 2.0,
            top + (tab_height - title_box.height) / 2.0,
        )
    } else {
        Point::new(x - title_box.width / 2.0, top + title_top_margin(config))
    };

    let mut body = String::new();
    if package {
        body.push_str(&format!(
            "<rect class=\"boundary-tab system-boundary-package-tab label-container\"{} x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>",
            attribute("style", &styles.node),
            number(left),
            number(top),
            number(tab_width),
            number(tab_height),
        ));
    }
    body.push_str(&format!(
        "<rect class=\"boundary-body label-container\"{} x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>",
        attribute("style", &styles.node),
        number(left),
        number(body_y),
        number(width),
        number(body_height),
    ));
    let label_style = if styles.label.is_empty() {
        String::new()
    } else {
        attribute("style", &styles.label)
    };
    let classes = node.css_classes.trim();
    let svg = format!(
        "<g class=\"cluster usecase-system-boundary usecase-system-boundary-{boundary}{classes_sep}{classes}\" id=\"{id}\" data-look=\"{look}\" data-boundary-type=\"{boundary}\"{colour_id}>{body}<g class=\"cluster-label system-boundary-title\"{label_style} transform=\"translate({tx}, {ty})\">{title}</g></g>",
        boundary = if package { "package" } else { "rect" },
        classes_sep = if classes.is_empty() { "" } else { " " },
        id = escape(&node.dom_id),
        look = escape(&node.look),
        tx = number(title_position.x),
        ty = number(title_position.y),
        title = title.svg(""),
    );

    let bbox = BBox::new(left, top, width, height);
    Ok(Cluster {
        svg,
        bbox,
        title_box: title_box.translate(title_position.x, title_position.y),
        label: Some(title),
    })
}

/// The top margin of a title below the top edge (`subGraphTitleTopMargin`).
fn title_top_margin(config: &Config<'_>) -> f64 {
    config
        .section
        .flowchart
        .sub_graph_title_margin
        .as_ref()
        .and_then(|margin| margin.top)
        .unwrap_or(0) as f64
}
