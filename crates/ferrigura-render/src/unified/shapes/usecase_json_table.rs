//! The JSON table: a port of `rendering-elements/shapes/usecaseJsonTable.ts` in the plain elements.
//! Mermaid draws the JSON tables of a use case diagram with it; a flowchart node can name it too. A title
//! row names the table; the key and value rows of the reference come from the fields of a use case,
//! which a flowchart node does not carry, so the table is its title row. The shape draws its own label.
//!
//! The reference's rough.js cells (the hand-drawn look) are out of scope; the plain rectangles are the
//! output of the other looks.

use super::Intersect;
use super::ShapeInput;
use super::usecase_actor::label_group;
use super::util::{self, Part, attribute, label_helper, styles2string};
use crate::svg::number;
use crate::unified::geom::BBox;
use crate::unified::geom::Point;
use crate::unified::label::LabelLayout;

/// The padding of a cell's text on each side (`CELL_PADDING_X` and `CELL_PADDING_Y`, `usecaseJsonTable.ts`).
const CELL_PADDING_X: f64 = 8.0;
const CELL_PADDING_Y: f64 = 4.0;
/// The border width when the style sets none that parses (`DEFAULT_BORDER_WIDTH`).
const DEFAULT_BORDER_WIDTH: f64 = 1.0;

/// A label centered on a point, as `positionLabel` places it: its box's origin taken out.
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

/// The JSON table (`usecaseJsonTable.ts` 57-195) without rows: the title cell inside the border, at least
/// as wide as an empty key cell and an empty value cell. The title is returned with the part for the fonts
/// of the diagram.
pub(super) fn json_table(input: &ShapeInput<'_>) -> (Part, Vec<LabelLayout>) {
    let node = input.node;
    let styles = styles2string(node);
    let label_style = styles.label.clone();
    let title = label_helper(input, false);
    // An empty key cell and an empty value cell are their padding wide.
    let empty_cell_width = CELL_PADDING_X * 2.0;
    let title_width = title.bbox.width + CELL_PADDING_X * 2.0;
    let inner_width = title_width.max(empty_cell_width + empty_cell_width);
    let title_height = title.bbox.height + CELL_PADDING_Y * 2.0;
    let inner_height = title_height;
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

    // The grid: the title cell, then the title (`usecaseJsonTable.ts` 156-164).
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

    let svg = format!("{outer}<g class=\"usecase-json-table-grid\">{grid}</g>",);
    let texts = vec![title.clone()];
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
    (part, texts)
}
