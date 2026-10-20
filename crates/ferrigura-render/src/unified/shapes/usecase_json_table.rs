//! The JSON table of the use case diagram: a port of `rendering-elements/shapes/usecaseJsonTable.ts` in the
//! plain elements. A title row names the table, and each row has a key cell and a value cell, sized from
//! their texts. The shape draws its own labels: the title and every key and value (`insertLabel`, the
//! flowchart's wrapping width, centred).
//!
//! The reference's rough.js cells (the hand-drawn look) are out of scope; the plain rectangles are the
//! output of the other looks.

use super::Intersect;
use super::ShapeInput;
use super::usecase_actor::{declarations, insert_label, label_group, root_font_size};
use super::util::{self, Part, attribute, label_helper, styles2string};
use crate::RenderError;
use crate::svg::number;
use crate::unified::geom::BBox;
use crate::unified::geom::Point;
use crate::unified::label::Anchor;
use crate::unified::label::LabelLayout;

/// The padding of a cell's text on each side (`CELL_PADDING_X` and `CELL_PADDING_Y`, `usecaseJsonTable.ts`).
const CELL_PADDING_X: f64 = 8.0;
const CELL_PADDING_Y: f64 = 4.0;
/// The border width when the style sets none that parses (`DEFAULT_BORDER_WIDTH`).
const DEFAULT_BORDER_WIDTH: f64 = 1.0;

/// A label centred on a point, as `positionLabel` places it: its box's origin taken out.
fn position_at(label: &LabelLayout, centre: Point) -> Point {
    Point::new(
        centre.x - label.bbox.width / 2.0 - label.bbox.x,
        centre.y - label.bbox.height / 2.0 - label.bbox.y,
    )
}

/// The width of a border from its `stroke-width` declaration (`numericStrokeWidth`).
fn border_width(styles: &util::Styles) -> f64 {
    styles
        .entries
        .iter()
        .find(|(key, _)| key == "stroke-width")
        .and_then(|(_, value)| parse_float(value))
        .filter(|width| width.is_finite() && *width >= 0.0)
        .unwrap_or(DEFAULT_BORDER_WIDTH)
}

/// `Number.parseFloat`: the longest prefix of the text (after its blanks) that is a decimal number, so
/// that `2px` is 2 and `thick` is none.
fn parse_float(text: &str) -> Option<f64> {
    let trimmed = text.trim_start();
    let bytes = trimmed.as_bytes();
    let mut end = 0;
    let mut seen_digit = false;
    let mut seen_dot = false;
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        end = 1;
    }
    while end < bytes.len() {
        match bytes[end] {
            b'0'..=b'9' => seen_digit = true,
            b'.' if !seen_dot => seen_dot = true,
            _ => break,
        }
        end += 1;
    }
    if !seen_digit {
        return None;
    }
    trimmed[..end].parse::<f64>().ok()
}

/// A rectangle of a cell, with the node's style (`insertCell`, non-rough form).
fn cell(class: &str, x: f64, y: f64, width: f64, height: f64, style: &str) -> String {
    format!(
        "<rect class=\"{class}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{}/>",
        number(x),
        number(y),
        number(width),
        number(height),
        attribute("style", style),
    )
}

/// The JSON table (`usecaseJsonTable.ts` 57-195). The cells and the labels are drawn in the shape's own
/// elements; the title and the cell texts are returned with the part for the fonts of the diagram.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read.
pub(super) fn json_table(input: &ShapeInput<'_>) -> Result<(Part, Vec<LabelLayout>), RenderError> {
    let node = input.node;
    let styles = styles2string(node);
    let label_style = styles.label.clone();
    let title = label_helper(input, false);
    let rows: Vec<(String, String)> = node
        .use_case
        .as_ref()
        .map(|fields| {
            fields
                .json_rows
                .iter()
                .map(|row| (row.key.clone(), row.value.clone()))
                .collect()
        })
        .unwrap_or_default();
    let cell_width = input.config.section.flowchart.wrapping_width;
    let _ = declarations(&label_style);
    let _ = root_font_size(input);

    // The key and value labels, measured as `insertLabel` measures them (`usecaseJsonTable.ts` 79-91).
    let mut measured = Vec::with_capacity(rows.len());
    for (key, value) in &rows {
        let key_label = insert_label(input, key, &label_style, cell_width)?;
        let value_label = insert_label(input, value, &label_style, cell_width)?;
        measured.push((key_label, value_label));
    }

    let key_width = measured
        .iter()
        .map(|(key, _)| key.bbox.width)
        .fold(0.0, f64::max)
        + CELL_PADDING_X * 2.0;
    let measured_value_width = measured
        .iter()
        .map(|(_, value)| value.bbox.width)
        .fold(0.0, f64::max)
        + CELL_PADDING_X * 2.0;
    let title_width = title.bbox.width + CELL_PADDING_X * 2.0;
    let inner_width = title_width.max(key_width + measured_value_width);
    let value_width =
        measured_value_width + (inner_width - key_width - measured_value_width).max(0.0);
    let title_height = title.bbox.height + CELL_PADDING_Y * 2.0;
    let row_heights: Vec<f64> = measured
        .iter()
        .map(|(key, value)| key.bbox.height.max(value.bbox.height) + CELL_PADDING_Y * 2.0)
        .collect();
    let inner_height = title_height + row_heights.iter().sum::<f64>();
    let border = border_width(&styles);
    let total_width = inner_width + border * 2.0;
    let total_height = inner_height + border * 2.0;
    let left = -inner_width / 2.0;
    let top = -inner_height / 2.0;
    let style = styles.node.as_str();

    let outer = cell(
        "label-container usecase-json-border",
        -total_width / 2.0,
        -total_height / 2.0,
        total_width,
        total_height,
        style,
    );

    // The grid: the title cell, the title, then one group per row (`usecaseJsonTable.ts` 156-190).
    let mut grid = String::new();
    grid.push_str(&cell(
        "usecase-json-cell usecase-json-title-cell",
        left,
        top,
        inner_width,
        title_height,
        style,
    ));
    let title_offset = position_at(&title, Point::new(0.0, top + title_height / 2.0));
    grid.push_str(&label_group(
        "label usecase-json-title",
        &label_style,
        title_offset,
        &title,
        &label_style,
    ));
    let mut row_top = top + title_height;
    for (index, ((key, value), row_height)) in measured.iter().zip(&row_heights).enumerate() {
        let key_centre = Point::new(left + key_width / 2.0, row_height / 2.0);
        let value_centre = Point::new(left + key_width + value_width / 2.0, row_height / 2.0);
        grid.push_str(&format!(
            "<g class=\"usecase-json-row\" data-row-index=\"{index}\" transform=\"translate(0, {})\">{}{}{}{}</g>",
            number(row_top),
            cell(
                "usecase-json-cell usecase-json-key-cell",
                left,
                0.0,
                key_width,
                *row_height,
                style,
            ),
            cell(
                "usecase-json-cell usecase-json-value-cell",
                left + key_width,
                0.0,
                value_width,
                *row_height,
                style,
            ),
            label_group(
                "label usecase-json-key",
                &label_style,
                position_at(key, key_centre),
                key,
                &label_style,
            ),
            label_group(
                "label usecase-json-value",
                &label_style,
                position_at(value, value_centre),
                value,
                &label_style,
            ),
        ));
        row_top += row_height;
    }

    let svg = format!("{outer}<g class=\"usecase-json-table-grid\">{grid}</g>",);
    let mut texts = vec![title.clone()];
    for (key, value) in measured {
        texts.push(key);
        texts.push(value);
    }
    let _ = Anchor::Middle;
    let part = Part {
        svg,
        after_label: String::new(),
        bbox: BBox::new(
            -total_width / 2.0,
            -total_height / 2.0,
            total_width,
            total_height,
        ),
        intersect: Intersect::Rect,
        label_offset: None,
        label: None,
    };
    Ok((part, texts))
}
