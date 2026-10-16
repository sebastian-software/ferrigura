//! The entity box of ER diagrams: a port of `rendering-util/rendering-elements/shapes/erBox.ts`.
//!
//! An entity without attributes is a rectangle around its name, drawn as `drawRect` draws it
//! (`erBox.ts` 52-80). An entity with attributes is a table: the name in a header row, then one row
//! per attribute with its type, name, keys and comment, the columns sized from their texts, and the
//! dividers between them (`erBox.ts` 87-318). The texts are SVG text laid out by `createText` with
//! Markdown and no wrapping inside a cell, and each one is placed with a `translate`. The research,
//! with the reference's lines for every rule, is in `docs-port/er/NOTES.md`, sections 5.3 and 5.4.
//!
//! The entity draws its own labels, so the shape returns its whole output: its label is not one of
//! the shape's labels (`Kind::draws_label` is false), and the texts go to `ShapeOutput::texts`.

use ferrigura_model::er::ErAttribute;
use ferrigura_model::flowchart::LabelType;
use ferrigura_syntax::class::parse_generic_types;
use ferrigura_syntax::sanitize::sanitize_text;
use ferrigura_text::{FontFamily, FontStyle, FontWeight, face, measure};

use super::rough::{Fill, double_stroked_polygon};
use super::util::{self, attribute};
use super::{Intersect, ShapeInput, ShapeOutput};
use crate::RenderError;
use crate::svg::number;
use crate::unified::geom::{BBox, Point};
use crate::unified::label::{self, Anchor, LabelLayout, LabelSpec};
use crate::unified::markdown::br_tag_length;
use crate::unified::palette;
use crate::variables::text_of;

/// The thickness of a divider line (`erBox.ts` 270). A divider is a polygon of this width.
const DIVIDER_THICKNESS: f64 = 0.0001;

/// The width a cell's text may take before it wraps: its calculated width plus 100 (`erBox.ts`
/// 370), so that a cell's own text never wraps.
const CELL_WRAP_MARGIN: f64 = 100.0;

/// Draws an entity. Its label is its alias when it has one (`erBox.ts` 22-24).
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read.
pub(super) fn er_box(input: &ShapeInput<'_>) -> Result<ShapeOutput, RenderError> {
    let node = input.node;
    let fields = node.entity.clone().unwrap_or_default();
    let label = if fields.alias.is_empty() {
        node.label.clone()
    } else {
        fields.alias.clone()
    };
    let styles = util::styles2string(node);
    let colour = colour_group_attribute(input);
    if fields.attributes.is_empty() && !label.is_empty() {
        return plain_box(input, &label, &styles, colour);
    }
    attribute_table(input, &label, &fields.attributes, &styles, colour)
}

/// The `data-color-id` of the entity's group: `color-<colorIndex % borderColorArray.length>` in a
/// colour theme (`erBox.ts` 68-71 and 241-244). An entity without a colour index has index 0.
fn colour_group_attribute(input: &ShapeInput<'_>) -> String {
    let Some(palette) = palette::palette(input.config, input.theme) else {
        return String::new();
    };
    let slot = palette::palette_slot(Some(input.node.color_index.unwrap_or(0)), &palette);
    attribute("data-color-id", &format!("color-{}", slot.unwrap_or(0)))
}

/// An entity without attributes (`erBox.ts` 52-80): a `drawRect` with the padding of the entity
/// and a minimum width. Its label is laid out with `labelHelper`, which wraps at the node's width
/// when the minimum applies (`node.width`), else at the flowchart's wrapping width.
fn plain_box(
    input: &ShapeInput<'_>,
    label_text: &str,
    styles: &util::Styles,
    colour: String,
) -> Result<ShapeOutput, RenderError> {
    let config = input.config;
    // Unscaled: the padding is scaled only on the attribute path (`erBox.ts` 82-85).
    let padding = config.er.diagram_padding as f64;
    let min_width = config.er.min_entity_width as f64;
    // `calculateTextWidth(node.label) + 2 * labelPaddingX < minEntityWidth` widens the node
    // (`erBox.ts` 61-65).
    let node_width = (calculate_text_width(label_text, config.font_size)? + padding * 2.0
        < min_width)
        .then_some(min_width);

    let text = sanitize_text(&label::decode_entities(label_text));
    let label = label_layout(
        input,
        &text,
        styles,
        node_width.unwrap_or(config.flowchart.wrapping_width),
        LabelType::Markdown,
    )?;
    // `drawRect` (`drawRect.ts` 25-35): the label padded by `labelPaddingX` on each side and by
    // `labelPaddingY` (1.5 padding) above and below, and at least the node's size.
    let total_width = (label.bbox.width + padding * 2.0).max(node_width.unwrap_or(0.0));
    let total_height = (label.bbox.height + padding * 3.0).max(0.0);
    let x = -total_width / 2.0;
    let y = -total_height / 2.0;
    let rect = BBox::new(x, y, total_width, total_height);

    let text_offset = Point::new(-label.bbox.width / 2.0, 0.0);
    let label_group = format!(
        "<g class=\"label\"{} transform=\"translate(0, {})\"><rect></rect><g>{}{}</g></g>",
        attribute("style", &styles.label),
        number(-label.bbox.height / 2.0),
        label.background_rect(),
        label.text_element(&util::node_text_style(&styles.label), Some(text_offset)),
    );
    let svg = format!(
        "<rect class=\"basic label-container\"{} x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>{label_group}",
        attribute("style", &styles.node),
        number(x),
        number(y),
        number(total_width),
        number(total_height),
    );
    let text_box = label
        .bbox
        .translate(text_offset.x, -label.bbox.height / 2.0);
    Ok(ShapeOutput {
        svg,
        after_label: String::new(),
        label_offset: None,
        label: None,
        texts: vec![label],
        group_attributes: colour,
        bbox: rect.union(text_box),
        intersect: Intersect::Rect,
    })
}

/// The cells of an entity with attributes, each with where it is drawn: the name in the header,
/// and the four texts of each row (`erBox.ts` 98-156).
struct Row {
    /// The top of the row, below the header.
    y_offset: f64,
    /// The row's height, texts and `entityPadding`.
    height: f64,
    /// The type, name, keys and comment texts, in that order.
    cells: [Cell; 4],
}

/// An entity with attributes (`erBox.ts` 87-318): the header with the name, the rows, the columns
/// sized from the widest text of each, and the dividers between them.
fn attribute_table(
    input: &ShapeInput<'_>,
    label_text: &str,
    attributes: &[ErAttribute],
    styles: &util::Styles,
    colour: String,
) -> Result<ShapeOutput, RenderError> {
    let config = input.config;
    // `htmlLabels` false scales both paddings by 1.25 (`erBox.ts` 82-85). The entity is drawn with
    // SVG text, so the scale is the one of the reference's SVG path.
    let scale = if config.html_labels.unwrap_or(true) {
        1.0
    } else {
        1.25
    };
    let padding = config.er.diagram_padding as f64 * scale;
    let text_padding = config.er.entity_padding as f64 * scale;

    let name = cell(input, label_text, styles)?;
    let name_width = name.label.bbox.width;
    let name_height = name.label.bbox.height + text_padding;

    let mut max_type = 0.0_f64;
    let mut max_name = 0.0_f64;
    let mut max_keys = 0.0_f64;
    let mut max_comment = 0.0_f64;
    let mut rows = Vec::with_capacity(attributes.len());
    let mut y_offset = 0.0;
    for attribute in attributes {
        let cells = [
            cell(input, &attribute.attr_type, styles)?,
            cell(input, &attribute.name, styles)?,
            cell(input, &attribute.keys.join(","), styles)?,
            cell(input, &attribute.comment, styles)?,
        ];
        max_type = max_type.max(cells[0].label.bbox.width + padding);
        max_name = max_name.max(cells[1].label.bbox.width + padding);
        max_keys = max_keys.max(cells[2].label.bbox.width + padding);
        max_comment = max_comment.max(cells[3].label.bbox.width + padding);
        let height = cells
            .iter()
            .map(|cell| cell.label.bbox.height)
            .fold(0.0, f64::max)
            + text_padding;
        rows.push(Row {
            y_offset,
            height,
            cells,
        });
        y_offset += height;
    }

    // The keys and the comments are columns only when some of their texts is wider than the padding
    // (`erBox.ts` 159-168); a column that is not present is zero wide and takes no section.
    let mut sections = 4.0;
    let keys_present = max_keys > padding;
    if !keys_present {
        max_keys = 0.0;
        sections -= 1.0;
    }
    let comment_present = max_comment > padding;
    if !comment_present {
        max_comment = 0.0;
        sections -= 1.0;
    }

    // The group's own box before the rectangle is drawn: the widest text, since every text starts
    // at x = 0 (`erBox.ts` 170).
    let shape_width = rows
        .iter()
        .flat_map(|row| row.cells.iter())
        .map(|cell| cell.label.bbox.width)
        .fold(name_width, f64::max);
    // The name's width shares the columns, when it is wider than them (`erBox.ts` 172-185).
    let difference = name_width + padding * 2.0 - (max_type + max_name + max_keys + max_comment);
    if difference > 0.0 {
        let share = difference / sections;
        max_type += share;
        max_name += share;
        if max_keys > 0.0 {
            max_keys += share;
        }
        if max_comment > 0.0 {
            max_comment += share;
        }
    }
    let max_width = max_type + max_name + max_keys + max_comment;
    let rows_height: f64 = rows.iter().map(|row| row.height).sum();
    let width = (shape_width + padding * 2.0).max(max_width);
    let height = (rows_height + name_height).max(0.0);
    let x = -width / 2.0;
    let y = -height / 2.0;

    let theme_fill = text_of(input.theme, "mainBkg").unwrap_or_default();
    let fill = style_value(styles, "fill").unwrap_or(theme_fill);
    let stroke = style_value(styles, "stroke")
        .unwrap_or_else(|| text_of(input.theme, "nodeBorder").unwrap_or_default());
    let stroke_width = util::stroke_width(styles);
    let path_style = (!styles.node.is_empty()).then_some(styles.node.as_str());

    let mut svg = String::new();
    // `rc.rectangle(x, y, w, h)` as the outer path (`erBox.ts` 230-245).
    let outer = rectangle_points(x, y, width, height);
    svg.push_str(&format!(
        "<g class=\"outer-path\"{}>{}</g>",
        attribute("style", &node_css_styles(input)),
        double_stroked_polygon(
            &outer,
            Some(Fill {
                colour: &fill,
                evenodd: false,
            }),
            &stroke,
            &stroke_width,
            path_style,
        ),
    ));

    // The row rectangles, which the reference inserts before the first label (`erBox.ts` 248-266).
    // A row takes the odd or the even fill; the redux themes give neither, so such a row has no
    // fill path (roughjs writes none for an undefined fill).
    let row_fill = text_of(input.theme, "rowOdd");
    let even_fill = text_of(input.theme, "rowEven");
    for (index, row) in rows.iter().enumerate() {
        let content_row = index + 1;
        let even = content_row % 2 == 0 && row.y_offset != 0.0;
        let row_colour = if even { &even_fill } else { &row_fill };
        let points = rectangle_points(x, name_height + y + row.y_offset, width, row.height);
        svg.push_str(&format!(
            "<g{} class=\"row-rect-{}\">{}</g>",
            attribute("style", &node_css_styles(input)),
            if even { "even" } else { "odd" },
            double_stroked_polygon(
                &points,
                row_colour.as_deref().map(|colour| Fill {
                    colour,
                    evenodd: false,
                }),
                &stroke,
                &stroke_width,
                path_style,
            ),
        ));
    }

    let mut texts = Vec::with_capacity(rows.len() * 4 + 1);
    let mut bbox = BBox::new(x, y, width, height);
    let name_translate = Point::new(-name_width / 2.0, y + text_padding / 2.0);
    svg.push_str(&cell_group(&name, "name", name_translate, styles));
    bbox = bbox.union(
        name.label
            .bbox
            .translate(name_translate.x, name_translate.y),
    );
    texts.push(name.label);

    // The columns start at the padding's half from the left edge; each one is offset by the
    // widths of the columns before it (`erBox.ts` 288-296, the translate loop).
    let column_offsets = [
        0.0,
        max_type,
        max_type + max_name,
        max_type + max_name + max_keys,
    ];
    const CLASSES: [&str; 4] = [
        "attribute-type",
        "attribute-name",
        "attribute-keys",
        "attribute-comment",
    ];
    for row in rows {
        for (column, cell) in row.cells.into_iter().enumerate() {
            let translate = Point::new(
                x + padding / 2.0 + column_offsets[column],
                row.y_offset + y + name_height + text_padding / 2.0,
            );
            svg.push_str(&cell_group(&cell, CLASSES[column], translate, styles));
            bbox = bbox.union(cell.label.bbox.translate(translate.x, translate.y));
            texts.push(cell.label);
        }
    }

    // The dividers (`erBox.ts` 270-316): the line under the header, the column lines that are
    // present, and one more line at the header's bottom (`yOffsets` holds only 0).
    let header_y = name_height + y;
    let mut dividers = vec![horizontal_line(x, header_y, width + x)];
    dividers.push(vertical_line(max_type + x, header_y, height + y));
    if keys_present {
        dividers.push(vertical_line(max_type + max_name + x, header_y, height + y));
    }
    if comment_present {
        dividers.push(vertical_line(
            max_type + max_name + max_keys + x,
            header_y,
            height + y,
        ));
    }
    dividers.push(horizontal_line(x, header_y, width + x));
    for points in dividers {
        svg.push_str(&format!(
            "<g class=\"divider\">{}</g>",
            double_stroked_polygon(
                &points,
                Some(Fill {
                    colour: &fill,
                    evenodd: true,
                }),
                &stroke,
                &stroke_width,
                path_style,
            ),
        ));
    }

    Ok(ShapeOutput {
        svg,
        after_label: String::new(),
        label_offset: None,
        label: None,
        texts,
        group_attributes: colour,
        bbox,
        intersect: Intersect::Rect,
    })
}

/// A text of the entity and whether the reference writes it flat (see `cell_group`).
struct Cell {
    /// The text's layout.
    label: LabelLayout,
    /// Whether the text has an escaped angle bracket, which the reference's undo step flattens.
    flat: bool,
}

/// A text of the entity, laid out as `addText` lays it out (`erBox.ts` 343-399): the generic types
/// parsed, Markdown, no wrapping inside the cell, and the label's style.
fn cell(input: &ShapeInput<'_>, text: &str, styles: &util::Styles) -> Result<Cell, RenderError> {
    let generic = parse_generic_types(text);
    // `addText` (`erBox.ts` 357-363) writes the angle brackets of a generic type as entities when
    // the types changed, so that Markdown reads `type<T>` as one word of text, not as an HTML tag
    // that starts a word of its own; the entities are decoded for the display.
    let parsed = if generic == text {
        generic
    } else {
        generic.replace('<', "&lt;").replace('>', "&gt;")
    };
    let width = calculate_text_width(&parsed, input.config.font_size)? + CELL_WRAP_MARGIN;
    let flat = parsed.contains("&lt;") || parsed.contains("&gt;");
    Ok(Cell {
        label: label_layout(input, &parsed, styles, width, LabelType::Markdown)?,
        flat,
    })
}

/// The label of a text as `createText` lays it out for a node: the text at the start of the box
/// (the cells and the minimum-width entity label are positioned by their own `translate`), with the
/// style's font size, or the theme's `fontSize` when the style has none.
fn label_layout(
    input: &ShapeInput<'_>,
    text: &str,
    styles: &util::Styles,
    width: f64,
    label_type: LabelType,
) -> Result<LabelLayout, RenderError> {
    let declarations: Vec<String> = styles
        .label
        .split(';')
        .filter(|declaration| !declaration.is_empty())
        .map(str::to_owned)
        .collect();
    label::layout(&LabelSpec {
        text,
        label_type,
        width,
        is_node: true,
        anchor: Anchor::Start,
        wrap_in_style: true,
        add_background: false,
        styles: &declarations,
        font_size: util::theme_number(input.theme, "fontSize").unwrap_or(input.config.font_size),
        title: false,
    })
}

/// The `<g>` of a cell, as `addText` writes it: the label group with its style and `translate`,
/// holding the background rectangle, and the text beside it. A text with an escaped angle bracket is
/// written flat, as the reference's undo step leaves it.
fn cell_group(cell: &Cell, classes: &str, translate: Point, styles: &util::Styles) -> String {
    let text_style = util::node_text_style(&styles.label);
    let text = if cell.flat {
        cell.label.text_element_flat(&text_style, None)
    } else {
        cell.label.text_element(&text_style, None)
    };
    format!(
        "<g class=\"label {classes}\" transform=\"translate({}, {})\"{}><g>{}</g>{text}</g>",
        number(translate.x),
        number(translate.y),
        attribute("style", &styles.label),
        cell.label.background_rect(),
    )
}

/// The declarations the entity's `cssStyles` give its outer path and rows (`erBox.ts` 233 and 257):
/// they are joined without a separator, as the reference joins them.
fn node_css_styles(input: &ShapeInput<'_>) -> String {
    input.node.css_styles.join("")
}

/// The value of a declaration in the compiled styles of the node, as `stylesMap.get` gives it.
fn style_value(styles: &util::Styles, key: &str) -> Option<String> {
    styles
        .entries
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.clone())
}

/// The corners of a rectangle as rough.js takes them: top-left, top-right, bottom-right, bottom-left.
fn rectangle_points(x: f64, y: f64, width: f64, height: f64) -> [Point; 4] {
    [
        Point::new(x, y),
        Point::new(x + width, y),
        Point::new(x + width, y + height),
        Point::new(x, y + height),
    ]
}

/// `lineToPolygon` for a horizontal line from `(x1, y)` to `(x2, y)` (`erBox.ts` 401-424).
fn horizontal_line(x1: f64, y: f64, x2: f64) -> [Point; 4] {
    let half = DIVIDER_THICKNESS / 2.0;
    [
        Point::new(x1, y - half),
        Point::new(x1, y + half),
        Point::new(x2, y + half),
        Point::new(x2, y - half),
    ]
}

/// `lineToPolygon` for a vertical line from `(x, y1)` to `(x, y2)`.
fn vertical_line(x: f64, y1: f64, y2: f64) -> [Point; 4] {
    let half = DIVIDER_THICKNESS / 2.0;
    [
        Point::new(x - half, y1),
        Point::new(x + half, y1),
        Point::new(x + half, y2),
        Point::new(x - half, y2),
    ]
}

/// `calculateTextWidth` (`utils.ts` 649-731) for SVG text: the widest line of the text at
/// `font_size`, rounded to a whole pixel. Lines are split at the `<br>` tags (`lineBreakRegex`).
///
/// The reference measures in both `sans-serif` and the configured family and keeps the second unless
/// the first is larger in every dimension (`utils.ts` 712-720). ferrigura has only the bundled
/// family, which the second measure is, so the result is the bundled family's width.
fn calculate_text_width(text: &str, font_size: f64) -> Result<f64, RenderError> {
    if text.is_empty() {
        return Ok(0.0);
    }
    let bundled = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
    let mut width = 0.0_f64;
    for line in split_at_breaks(text) {
        // An empty line is measured as a zero-width space, which has no advance.
        let measured = if line.is_empty() {
            0.0
        } else {
            measure(bundled, line, font_size)?
        };
        width = width.max(measured);
    }
    Ok(width.round())
}

/// The lines of a text, split at each `<br>` tag in any case.
fn split_at_breaks(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut index = 0;
    while index < text.len() {
        if let Some(length) = br_tag_length(&text[index..], true) {
            lines.push(&text[start..index]);
            index += length;
            start = index;
        } else {
            index += text[index..].chars().next().map_or(1, char::len_utf8);
        }
    }
    lines.push(&text[start..]);
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_text_splits_at_the_break_tags_in_any_case() {
        assert_eq!(
            split_at_breaks("a<br>b<BR/>c<br />d"),
            vec!["a", "b", "c", "d"]
        );
        // `lineBreakRegex` takes no space after the `<`.
        assert_eq!(split_at_breaks("a< br >d"), vec!["a< br >d"]);
        assert_eq!(split_at_breaks("plain"), vec!["plain"]);
        assert_eq!(split_at_breaks("<br>"), vec!["", ""]);
    }

    #[test]
    fn a_line_break_is_not_a_tag_when_it_is_not_one() {
        assert_eq!(split_at_breaks("a<b>c"), vec!["a<b>c"]);
    }

    #[test]
    fn an_empty_text_is_zero_wide() {
        assert_eq!(calculate_text_width("", 16.0).expect("measures"), 0.0);
    }

    #[test]
    fn the_width_is_rounded_to_a_whole_pixel() {
        let width = calculate_text_width("AUTHOR", 16.0).expect("measures");
        assert_eq!(width, width.round());
        assert!(width > 0.0);
    }
}
