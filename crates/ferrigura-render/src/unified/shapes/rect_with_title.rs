//! `rectWithTitle`: a state with two or more descriptions, drawn as a box with the title above a
//! divider and the descriptions below it. A port of `shapes/rectWithTitle.ts` for the neo and classic
//! looks (the handDrawn look is out of scope).
//!
//! The reference lays the title and the rows out with `createLabel`, which is `createText` with
//! `markdown: false`, no wrapping and the `isTitle` class (`title-row`). The rows are joined with
//! `<br/>`, and the reference measures them by the first child of their `<text>`, the first line:
//! its width places the rows under the title, centered when the title is wider, and the title shifts
//! when the rows are. The port keeps that reading (`rectWithTitle.ts` 55-99).

use super::util::{self, Part, attribute, styles2string};
use super::{Intersect, ShapeInput};
use crate::RenderError;
use crate::svg::number;
use crate::unified::geom::BBox;
use crate::unified::label::{self, Anchor, LabelLayout, LabelSpec};
use ferrigura_model::flowchart::LabelType;
use ferrigura_text::{FontStyle, FontWeight, text_box};

/// `rectWithTitle.ts`: the outer box, the divider under the title, and the label group with the
/// title and the rows. The box is the node's size; the label is inside it. The title and the rows
/// are returned as well, as the texts of the shape: the font subsets of the diagram cover them, so
/// their glyphs are embedded (`ShapeOutput::texts`).
///
/// # Errors
///
/// Returns an error if a font cannot be read.
pub(super) fn rect_with_title(
    input: &ShapeInput<'_>,
) -> Result<(Part, Vec<LabelLayout>), RenderError> {
    let node = input.node;
    let styles = styles2string(node);
    let text_style = util::node_text_style(&styles.label);
    let declarations: Vec<String> = styles
        .label
        .split(';')
        .filter(|declaration| !declaration.is_empty())
        .map(str::to_owned)
        .collect();
    let font_size =
        util::theme_number(&input.config.theme.font_size).unwrap_or(input.config.globals.font_size);
    // `createLabel`: plain text, never wrapped, with the title class.
    let text = |text: &str| -> Result<LabelLayout, RenderError> {
        label::layout(&LabelSpec {
            fonts: input.config.fonts.clone(),
            text,
            label_type: LabelType::Text,
            width: f64::INFINITY,
            is_node: true,
            // The state stylesheet does not center the lines of a label (`state/styles.js`).
            anchor: Anchor::Start,
            wrap_in_style: true,
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
    let primary = input
        .config
        .fonts
        .face(FontWeight::Normal, FontStyle::Normal)?;
    let line_height = text_box(&primary, rows_layout.font_size)?.height;
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
    let design = input.config.section.design.is_some();
    let room = crate::design::DESCRIBED_STATE_ROOM;
    let title_height = title.bbox.height;
    let rows_height = rows_layout.bbox.height;
    let rows_y = if design {
        title_height + 2.0 * room
    } else {
        title_height + half_padding + 5.0
    };
    let label_box = title
        .bbox
        .translate(title_x, 0.0)
        .union(rows_layout.bbox.translate(rows_x, rows_y));

    // The design mode gives the label at least the width of a plain state's label, the state's
    // minimum width, as `labelHelper` does unless the state has its own width. The title and the
    // rows stay centered in it, and the box keeps the state's padding at both sides.
    let (outer, divider_y, divider_end, group_origin, title_x, rows_x) = if design {
        let minimum_width = if !node.label.is_empty() && node.width.unwrap_or(0.0) == 0.0 {
            node.min_width.unwrap_or(0.0)
        } else {
            0.0
        };
        let label_width = label_box.width.max(minimum_width);
        let shift = (label_width - label_box.width) / 2.0;
        let width = (label_width + 2.0 * padding).max(node.width.unwrap_or(0.0));
        let height = (title_height + rows_height + 4.0 * room).max(node.height.unwrap_or(0.0));
        let outer = BBox::new(-width / 2.0, -height / 2.0, width, height);
        let divider_y = outer.y + room + title_height + room;
        (
            outer,
            divider_y,
            outer.x + outer.width,
            (-label_width / 2.0, outer.y + room),
            title_x + shift,
            rows_x + shift,
        )
    } else {
        let outer = BBox::new(
            -label_box.width / 2.0 - half_padding,
            -label_box.height / 2.0 - half_padding,
            label_box.width + padding,
            label_box.height + padding,
        );
        let divider_y = -label_box.height / 2.0 - half_padding + title_height + half_padding;
        let divider_end = label_box.width / 2.0 + half_padding;
        let group_origin = (
            -label_box.width / 2.0,
            -label_box.height / 2.0 - half_padding + 3.0,
        );
        (outer, divider_y, divider_end, group_origin, title_x, rows_x)
    };
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
        number(group_origin.0),
        number(group_origin.1),
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

    let part = Part {
        svg: format!("{frame}{label_group}"),
        after_label: String::new(),
        bbox: outer,
        intersect: Intersect::Rect,
        label_offset: None,
        label: None,
    };
    Ok((part, vec![title, rows_layout]))
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{node, theme};
    use super::super::{ShapeInput, draw};
    use crate::unified::Fixture;
    use ferrigura_model::config::MermaidConfig;

    #[test]
    fn the_title_and_the_rows_are_texts_of_the_shape() {
        // The font subsets of a diagram cover the texts of its shapes (`ShapeOutput::texts`). The
        // title and the rows are drawn here, so their glyphs must be in the subset: without them
        // Chromium sets N, w and l of "New line" in a fallback face 0.8 px wider
        // (state-diagram/pr-check).
        let mut node = node("rectWithTitle", "classic");
        node.label = "Another Long state description".to_owned();
        node.description = Some(vec!["New line".to_owned()]);
        let fixture = Fixture::new(MermaidConfig::default(), &theme());
        let config = fixture.config();
        let shape = draw(&ShapeInput {
            node: &node,
            label: None,
            config: &config,
            dir: Some("TB"),
        })
        .expect("the shape is known");
        let text: String = shape
            .texts
            .iter()
            .flat_map(|text| text.text_runs())
            .map(|run| run.text)
            .collect();
        assert!(text.contains("Another"), "{text:?}");
        assert!(text.contains("New"), "{text:?}");
        assert!(text.contains(" line"), "{text:?}");
    }
}
