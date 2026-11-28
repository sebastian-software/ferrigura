//! The entity box of the design mode (`docs/decisions/0023-design-mode.md`): a rounded box with
//! the entity's name bold and centered in a row of its own and, when it has attributes, a row per
//! attribute below a hairline divider. An attribute's name, type, keys and comment stand in
//! columns that start on one line; the name is set in the text color, the rest muted, the keys
//! smaller and bold, so that the name of each attribute leads its row. Faint hairlines divide the
//! rows; a column that no attribute fills takes no room. An entity without attributes is its name
//! alone, centered in its box however much the box grows.
//!
//! The texts are plain text, as the class box's are: entities are decoded, a generic type shows
//! its angle brackets, and no Markdown is read, so a name such as `*id` keeps its asterisk. The
//! box is as large as the node's size when the layout data gives one, so that it can grow along
//! its busiest side.

use ferrigura_model::er::ErAttribute;
use ferrigura_model::flowchart::LabelType;
use ferrigura_syntax::class::parse_generic_types;
use ferrigura_text::{FontStyle, FontWeight};

use super::util::{self, attribute};
use super::{Intersect, ShapeInput, ShapeOutput};
use crate::RenderError;
use crate::common::decode_entities;
use crate::svg::number;
use crate::unified::geom::BBox;
use crate::unified::label::{Anchor, LabelLayout, LabelSpec, layout_in_style};

/// The room at the left and the right of the rows: the flowchart node's.
const PADDING_X: f64 = 16.0;

/// The room above and below the name.
const NAME_PADDING_Y: f64 = 10.0;

/// The room above and below the text of an attribute row.
const ROW_PADDING_Y: f64 = 6.0;

/// The room above the first and below the last attribute row, besides the row's own.
const ROWS_PADDING_Y: f64 = 3.0;

/// The gap between two columns.
const COLUMN_GAP: f64 = 16.0;

/// The size of an attribute's comment, relative to the diagram's font size.
const COMMENT_SCALE: f64 = 0.875;

/// The size of an attribute's keys, relative to the diagram's font size.
const KEYS_SCALE: f64 = 0.75;

/// The narrowest box: an entity with a short name still reads as a box, not as a tag. It is the
/// class box's least width.
pub const MIN_WIDTH: f64 = 120.0;

/// The classes of the columns, in the order they stand.
const COLUMNS: [&str; 4] = [
    "attribute-name",
    "attribute-type",
    "attribute-keys",
    "attribute-comment",
];

/// Lays out one line of plain text at `size`.
fn line(
    input: &ShapeInput<'_>,
    text: &str,
    size: f64,
    anchor: Anchor,
    weight: Option<FontWeight>,
) -> Result<LabelLayout, RenderError> {
    // An angle bracket is text, not the start of a tag that would be a word of its own: a generic
    // type such as `list<string>` stays one word.
    let decoded = decode_entities(text)
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let spec = LabelSpec {
        fonts: input.config.fonts.clone(),
        text: &decoded,
        label_type: LabelType::Text,
        width: f64::INFINITY,
        is_node: true,
        anchor,
        wrap_in_style: true,
        add_background: false,
        styles: &[],
        font_size: size,
        title: false,
    };
    layout_in_style(&spec, weight, FontStyle::Normal)
}

/// The cells of an attribute row in the order of [`COLUMNS`]; an empty text has no cell.
fn cells(
    input: &ShapeInput<'_>,
    attribute: &ErAttribute,
    size: f64,
) -> Result<[Option<LabelLayout>; 4], RenderError> {
    let cell = |text: &str, size: f64, weight: Option<FontWeight>| {
        if text.is_empty() {
            Ok(None)
        } else {
            line(input, text, size, Anchor::Start, weight).map(Some)
        }
    };
    Ok([
        cell(&attribute.name, size, None)?,
        cell(&parse_generic_types(&attribute.attr_type), size, None)?,
        cell(
            &attribute.keys.join(", "),
            size * KEYS_SCALE,
            Some(FontWeight::Bold),
        )?,
        cell(&attribute.comment, size * COMMENT_SCALE, None)?,
    ])
}

/// The baseline of a laid-out line, in its own coordinates.
fn baseline(label: &LabelLayout) -> f64 {
    label.lines.first().map_or(0.0, |line| line.baseline)
}

/// A text group whose baseline stands at `(x, y)`: its class and the label, with the size it was
/// laid out at and the entity's text style.
fn text_group(class: &str, x: f64, y: f64, label: &LabelLayout, style: &str) -> String {
    let separator = if style.is_empty() { "" } else { ";" };
    format!(
        "<g class=\"label {class}\" transform=\"translate({}, {})\">{}</g>",
        number(x),
        number(y - baseline(label)),
        label.svg(&format!(
            "font-size:{}px{separator}{style}",
            number(label.font_size)
        )),
    )
}

/// One attribute row: its cells, and the room above and below their common baseline.
struct Row {
    cells: [Option<LabelLayout>; 4],
    ascent: f64,
    descent: f64,
}

impl Row {
    fn height(&self) -> f64 {
        self.ascent + self.descent + 2.0 * ROW_PADDING_Y
    }
}

/// Draws the design mode's entity box around the origin. Its label is its alias when it has one.
///
/// # Errors
///
/// Returns an error if a font cannot be read.
pub(super) fn er_box(input: &ShapeInput<'_>) -> Result<ShapeOutput, RenderError> {
    let node = input.node;
    let fields = node.entity.clone().unwrap_or_default();
    let label = if fields.alias.is_empty() {
        node.label.as_str()
    } else {
        fields.alias.as_str()
    };
    let styles = util::styles2string(node);
    let text_style = util::node_text_style(&styles.label);
    let font_size =
        util::theme_number(&input.config.theme.font_size).unwrap_or(input.config.globals.font_size);

    let name = line(
        input,
        label,
        font_size,
        Anchor::Middle,
        Some(FontWeight::Bold),
    )?;
    let mut rows = Vec::with_capacity(fields.attributes.len());
    for attribute in &fields.attributes {
        let cells = cells(input, attribute, font_size)?;
        let (mut ascent, mut descent) = (0.0_f64, 0.0_f64);
        for cell in cells.iter().flatten() {
            ascent = ascent.max(baseline(cell) - cell.bbox.y);
            descent = descent.max(cell.bbox.bottom() - baseline(cell));
        }
        rows.push(Row {
            cells,
            ascent,
            descent,
        });
    }

    // The width of each column, and where it starts after the room at the left.
    let mut widths = [0.0_f64; 4];
    for row in &rows {
        for (width, cell) in widths.iter_mut().zip(&row.cells) {
            if let Some(cell) = cell {
                *width = width.max(cell.bbox.right().max(0.0));
            }
        }
    }
    let mut starts = [0.0_f64; 4];
    let mut x = 0.0;
    let mut first = true;
    for (start, width) in starts.iter_mut().zip(widths) {
        if width > 0.0 {
            if !first {
                x += COLUMN_GAP;
            }
            first = false;
        }
        *start = x;
        x += width;
    }
    let rows_width = x;

    let width = (rows_width.max(name.bbox.width) + 2.0 * PADDING_X)
        .max(MIN_WIDTH)
        .max(node.width.unwrap_or(0.0));
    let head_height = name.bbox.height + 2.0 * NAME_PADDING_Y;
    let rows_height = if rows.is_empty() {
        0.0
    } else {
        rows.iter().map(Row::height).sum::<f64>() + 2.0 * ROWS_PADDING_Y
    };
    // A box grown along its height keeps its rows at the top; the room goes below the last one.
    let height = (head_height + rows_height).max(node.height.unwrap_or(0.0));
    let left = -width / 2.0;
    let top = -height / 2.0;

    let mut svg = format!(
        "<g class=\"basic label-container outer-path\"><rect class=\"outline\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{radius}\" ry=\"{radius}\"{}/></g>",
        number(left),
        number(top),
        number(width),
        number(height),
        attribute("style", &styles.node),
        radius = number(crate::design::NODE_RADIUS),
    );
    // The name, centered in its row; the name of an entity without attributes is centered in a
    // grown box too.
    let name_top = if rows.is_empty() {
        -name.bbox.height / 2.0
    } else {
        top + NAME_PADDING_Y
    };
    svg.push_str(&text_group(
        "name",
        0.0,
        name_top + baseline(&name) - name.bbox.y,
        &name,
        &text_style,
    ));
    let mut dividers = String::new();
    if !rows.is_empty() {
        let divider = |class: &str, y: f64| {
            format!(
                "<line class=\"{class}\" x1=\"{}\" y1=\"{y}\" x2=\"{}\" y2=\"{y}\"/>",
                number(left),
                number(left + width),
                y = number(y),
            )
        };
        let mut y = top + head_height;
        dividers.push_str(&divider("divider", y));
        y += ROWS_PADDING_Y;
        let mut cells_svg = String::new();
        for (index, row) in rows.iter().enumerate() {
            if index > 0 {
                dividers.push_str(&divider("row-divider", y));
            }
            let row_baseline = y + ROW_PADDING_Y + row.ascent;
            for (column, cell) in row.cells.iter().enumerate() {
                if let Some(cell) = cell {
                    cells_svg.push_str(&text_group(
                        COLUMNS[column],
                        left + PADDING_X + starts[column],
                        row_baseline,
                        cell,
                        &text_style,
                    ));
                }
            }
            y += row.height();
        }
        svg.push_str(&format!("<g class=\"attributes\">{cells_svg}</g>"));
    }
    svg.push_str(&dividers);

    let bbox = BBox::new(left, top, width, height);
    let texts = std::iter::once(name)
        .chain(
            rows.into_iter()
                .flat_map(|row| row.cells.into_iter().flatten()),
        )
        .collect();
    Ok(ShapeOutput {
        svg,
        after_label: String::new(),
        label_offset: None,
        label: None,
        texts,
        group_attributes: String::new(),
        colour_slot: String::new(),
        element: bbox,
        bbox,
        intersect: Intersect::Rect,
        extent: None,
    })
}
