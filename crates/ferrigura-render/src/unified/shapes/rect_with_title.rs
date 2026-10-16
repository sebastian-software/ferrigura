//! `rectWithTitle`: a state with two or more descriptions, drawn as a box with the title above a
//! divider and the descriptions below it. A port of `shapes/rectWithTitle.ts` for the neo and classic
//! looks (the handDrawn look is out of scope).
//!
//! The reference lays the title and the rows out with `createLabel`, which is `createText` with
//! `markdown: false`, no wrapping and the `isTitle` class (`title-row`). The rows are joined with
//! `<br/>`, and the reference measures them by the first child of their `<text>`, the first line:
//! its width places the rows under the title, centred when the title is wider, and the title shifts
//! when the rows are. The port keeps that reading (`rectWithTitle.ts` 55-99).

use super::util::{self, Part, attribute, styles2string};
use super::{Intersect, ShapeInput};
use crate::RenderError;
use crate::svg::number;
use crate::unified::geom::BBox;
use crate::unified::label::{self, Anchor, LabelLayout, LabelSpec};
use ferrigura_model::flowchart::LabelType;
use ferrigura_text::{FontFamily, FontStyle, FontWeight, face, text_box};

/// `rectWithTitle.ts`: the outer box, the divider under the title, and the label group with the
/// title and the rows. The box is the node's size; the label is inside it.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read.
pub(super) fn rect_with_title(input: &ShapeInput<'_>) -> Result<Part, RenderError> {
    let node = input.node;
    let styles = styles2string(node);
    let text_style = util::node_text_style(&styles.label);
    let declarations: Vec<String> = styles
        .label
        .split(';')
        .filter(|declaration| !declaration.is_empty())
        .map(str::to_owned)
        .collect();
    let font_size = util::theme_number(input.theme, "fontSize").unwrap_or(input.config.font_size);
    // `createLabel`: plain text, never wrapped, with the title class.
    let text = |text: &str| -> Result<LabelLayout, RenderError> {
        label::layout(&LabelSpec {
            text,
            label_type: LabelType::Text,
            width: f64::INFINITY,
            is_node: true,
            // The state stylesheet does not centre the lines of a label (`state/styles.js`).
            anchor: Anchor::Start,
            add_background: false,
            styles: &declarations,
            font_size,
            title: true,
        })
    };

    let title = text(&node.label)?;
    let rows = node
        .description
        .as_deref()
        .map_or_else(String::new, |rows| rows.join("<br/>"));
    let rows_layout = text(&rows)?;

    let padding = node.padding;
    let half_padding = padding / 2.0;
    // The rows are measured by their first child: the `<text>`'s first line, whose box is the line's
    // width and the height of a line of the font (`rectWithTitle.ts` 55-63, 88-99).
    let line_width = rows_layout.lines.first().map_or(0.0, |line| line.width);
    let line_height = text_box(
        face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal),
        rows_layout.font_size,
    )?
    .height;
    let title_width = title.bbox.width;
    let title_x = if line_width < title_width {
        0.0
    } else {
        -(title_width - line_width) / 2.0
    };
    let rows_x = if line_width > title_width {
        0.0
    } else {
        (title_width - line_width) / 2.0
    };
    let rows_y = title.bbox.height + half_padding + 5.0;
    let label_box = title
        .bbox
        .translate(title_x, 0.0)
        .union(rows_layout.bbox.translate(rows_x, rows_y));

    let outer = BBox::new(
        -label_box.width / 2.0 - half_padding,
        -label_box.height / 2.0 - half_padding,
        label_box.width + padding,
        label_box.height + padding,
    );
    let divider_y = -label_box.height / 2.0 - half_padding + title.bbox.height + half_padding;
    let divider_end = label_box.width / 2.0 + half_padding;
    let node_style = attribute("style", &styles.node);

    let frame = format!(
        "<g><rect class=\"outer title-state\"{node_style} x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/><line class=\"divider\" x1=\"{}\" x2=\"{}\" y1=\"{}\" y2=\"{}\"/></g>",
        number(outer.x),
        number(outer.y),
        number(outer.width),
        number(outer.height),
        number(outer.x),
        number(divider_end),
        number(divider_y),
        number(divider_y),
    );
    let label_group = format!(
        "<g class=\"label\"{node_style} transform=\"translate({}, {})\">{}{}</g>",
        number(-label_box.width / 2.0),
        number(-label_box.height / 2.0 - half_padding + 3.0),
        title.svg_parts(
            &text_style,
            "",
            &format!(" transform=\"translate( {}, 0)\"", number(title_x)),
        ),
        rows_layout.svg_parts(
            &text_style,
            "",
            &format!(
                " width=\"{}\" height=\"{}\" transform=\"translate( {}, {})\"",
                number(line_width),
                number(line_height),
                number(rows_x),
                number(rows_y),
            ),
        ),
    );

    Ok(Part {
        svg: format!("{frame}{label_group}"),
        after_label: String::new(),
        bbox: outer,
        intersect: Intersect::Rect,
        label_offset: None,
        label: None,
    })
}
