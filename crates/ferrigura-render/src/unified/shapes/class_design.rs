//! The class box of the design mode (`docs/decisions/0023-design-mode.md`): a rounded box with a
//! name compartment and a compartment for the attributes and one for the methods, each only when
//! it has rows, divided by hairlines. The annotation is a small, muted line above the name; the
//! name is bold and centered; the members start at the left, after a column for their visibility
//! marks, which are muted. A static member is underlined and an abstract one set in italics.
//!
//! The rows are plain text: entities are decoded (so a generic type shows its angle brackets) and
//! no Markdown is read, so a member such as `__init__` keeps its underscores. The box is as large as
//! the node's size when the layout data gives one, so that it can grow along its busiest side.

use ferrigura_model::class::ClassMember;
use ferrigura_model::flowchart::LabelType;
use ferrigura_text::{FontStyle, FontWeight};

use super::util::{self, Part};
use super::{Intersect, ShapeInput};
use crate::RenderError;
use crate::common::decode_entities;
use crate::design::Role;
use crate::svg::{escape, number};
use crate::unified::data::ClassNodeData;
use crate::unified::geom::BBox;
use crate::unified::label::{Anchor, LabelLayout, LabelSpec, layout_in_style};

/// The room at the left and the right of the rows.
const PADDING_X: f64 = 16.0;

/// The room above and below the annotation and the name.
const NAME_PADDING_Y: f64 = 10.0;

/// The room above the first row of a member compartment.
const MEMBERS_PADDING_TOP: f64 = 8.0;

/// The room below the last row of a member compartment.
const MEMBERS_PADDING_BOTTOM: f64 = 10.0;

/// The gap between two member rows.
const ROW_GAP: f64 = 3.0;

/// The gap between the annotation and the name.
const ANNOTATION_GAP: f64 = 1.0;

/// The size of a member row, relative to the diagram's font size: the size of the name, which its
/// weight sets apart.
const MEMBER_SCALE: f64 = 1.0;

/// The size of the annotation, relative to the diagram's font size.
const ANNOTATION_SCALE: f64 = 0.875;

/// The width of the column of the visibility marks, relative to the member size: the names of the
/// members start on one line whatever their mark.
const MARK_COLUMN: f64 = 1.0;

/// The narrowest box: a class with a short name still reads as a box, not as a tag.
pub const MIN_WIDTH: f64 = 120.0;

/// The role of a class in the design mode, from its first annotation: an interface or an abstract
/// class, which is not instantiated, and an enumeration, a fixed set of values. Other annotations,
/// such as `<<service>>`, carry none.
#[must_use]
pub fn role(class: &ClassNodeData) -> Option<Role> {
    let annotation = class.annotations.first()?.trim().to_ascii_lowercase();
    match annotation.as_str() {
        "interface" | "abstract" => Some(Role::Interface),
        "enumeration" | "enum" => Some(Role::Enumeration),
        _ => None,
    }
}

/// One laid-out row: its text, and, for a member, its visibility mark.
struct Row {
    mark: Option<LabelLayout>,
    text: LabelLayout,
    style: &'static str,
}

/// Lays out one line of plain text at `size`, anchored as `anchor` says.
fn line(
    input: &ShapeInput<'_>,
    text: &str,
    size: f64,
    anchor: Anchor,
    weight: Option<FontWeight>,
    style: FontStyle,
) -> Result<LabelLayout, RenderError> {
    let decoded = decode_entities(text);
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
    layout_in_style(&spec, weight, style)
}

/// The visibility mark of a member's text and the rest of it: `+`, `-`, `#` or `~` before a name.
fn split_mark(text: &str) -> (Option<char>, &str) {
    let text = text.strip_prefix('\\').unwrap_or(text);
    let mut chars = text.chars();
    match chars.next() {
        Some(mark @ ('+' | '-' | '#' | '~')) if !chars.as_str().is_empty() => {
            (Some(mark), chars.as_str())
        }
        _ => (None, text),
    }
}

/// The rows of a member compartment.
fn rows(
    input: &ShapeInput<'_>,
    members: &[ClassMember],
    size: f64,
) -> Result<Vec<Row>, RenderError> {
    members
        .iter()
        .map(|member| {
            let (mark, rest) = split_mark(&member.text);
            let (style, text_style) = match member.classifier.as_str() {
                "*" => (FontStyle::Italic, ""),
                "$" => (FontStyle::Normal, "text-decoration:underline"),
                _ => (FontStyle::Normal, ""),
            };
            Ok(Row {
                mark: mark
                    .map(|mark| {
                        line(
                            input,
                            &mark.to_string(),
                            size,
                            Anchor::Start,
                            None,
                            FontStyle::Normal,
                        )
                    })
                    .transpose()?,
                text: line(input, rest, size, Anchor::Start, None, style)?,
                style: text_style,
            })
        })
        .collect()
}

/// A text group at `(x, y)`: its class, the text's own style, and the label. The text carries the
/// size it was laid out at, which the style sheet's size of the diagram would not give it.
fn text_group(class: &str, x: f64, y: f64, label: &LabelLayout, style: &str) -> String {
    let separator = if style.is_empty() { "" } else { ";" };
    format!(
        "<g class=\"{class}\" transform=\"translate({}, {})\">{}</g>",
        number(x),
        number(y),
        label.svg(&format!(
            "font-size:{}px{separator}{style}",
            number(label.font_size)
        )),
    )
}

/// The height of a member compartment with these rows, and the y of each row's top below the
/// compartment's top.
fn compartment(rows: &[Row]) -> (f64, Vec<f64>) {
    let mut tops = Vec::with_capacity(rows.len());
    let mut y = MEMBERS_PADDING_TOP;
    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            y += ROW_GAP;
        }
        tops.push(y);
        y += row.text.bbox.height;
    }
    (y + MEMBERS_PADDING_BOTTOM, tops)
}

/// What the design mode's class box draws: its part, and the texts of its rows.
pub(super) struct DesignClassBox {
    /// The part, with the outline as its box.
    pub part: Part,
    /// The laid-out texts, whose fonts the diagram embeds.
    pub texts: Vec<LabelLayout>,
}

/// Draws the design mode's class box around the origin.
pub(super) fn class_box(input: &ShapeInput<'_>) -> Result<DesignClassBox, RenderError> {
    let empty = ClassNodeData::default();
    let data = input.node.class.as_ref().unwrap_or(&empty);
    let font_size =
        util::theme_number(&input.config.theme.font_size).unwrap_or(input.config.globals.font_size);
    let member_size = font_size * MEMBER_SCALE;
    let annotation_size = font_size * ANNOTATION_SCALE;
    let raw_styles = data.styles.join(";");
    let styles = crate::common::resolve_entity_placeholders(&raw_styles);
    let style = if styles.is_empty() {
        String::new()
    } else {
        util::attribute("style", &styles)
    };

    let annotation = data
        .annotations
        .first()
        .map(|first| {
            line(
                input,
                &format!("«{first}»"),
                annotation_size,
                Anchor::Middle,
                None,
                FontStyle::Normal,
            )
        })
        .transpose()?;
    let name = line(
        input,
        &data.text,
        font_size,
        Anchor::Middle,
        Some(FontWeight::Bold),
        FontStyle::Normal,
    )?;
    let members = rows(input, &data.members, member_size)?;
    let methods = rows(input, &data.methods, member_size)?;

    // The column of the marks, when any member of the box has one.
    let marked = members.iter().chain(&methods).any(|row| row.mark.is_some());
    let column = if marked {
        member_size * MARK_COLUMN
    } else {
        0.0
    };
    let row_width = members
        .iter()
        .chain(&methods)
        .map(|row| column + row.text.bbox.right().max(0.0))
        .fold(0.0, f64::max);
    let head_width = name
        .bbox
        .width
        .max(annotation.as_ref().map_or(0.0, |label| label.bbox.width));
    let width = (row_width.max(head_width) + 2.0 * PADDING_X)
        .max(MIN_WIDTH)
        .max(input.node.width.unwrap_or(0.0));

    let annotation_height = annotation
        .as_ref()
        .map_or(0.0, |label| label.bbox.height + ANNOTATION_GAP);
    let head_height = NAME_PADDING_Y * 2.0 + annotation_height + name.bbox.height;
    let (members_height, member_tops) = compartment(&members);
    let (methods_height, method_tops) = compartment(&methods);
    let mut height = head_height
        + if members.is_empty() {
            0.0
        } else {
            members_height
        }
        + if methods.is_empty() {
            0.0
        } else {
            methods_height
        };
    // A box grown along its height keeps its compartments at the top; the last one takes the room.
    height = height.max(input.node.height.unwrap_or(0.0));
    let x = -width / 2.0;
    let y = -height / 2.0;

    let mut svg = format!(
        "<g class=\"basic label-container outer-path\"><rect class=\"outline\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{radius}\" ry=\"{radius}\"{}/></g>",
        number(x),
        number(y),
        number(width),
        number(height),
        style,
        radius = number(crate::design::NODE_RADIUS),
    );
    let tspans = super::class_box::tspan_style(&styles);
    let paint = |svg: String| match &tspans {
        Some(fill) => svg.replace("<tspan ", &format!("<tspan style=\"{}\" ", escape(fill))),
        None => svg,
    };

    let mut top = y + NAME_PADDING_Y;
    let mut head = String::new();
    if let Some(annotation) = &annotation {
        head.push_str(&text_group(
            "annotation-group text",
            0.0,
            top - annotation.bbox.y,
            annotation,
            "",
        ));
        top += annotation.bbox.height + ANNOTATION_GAP;
    }
    head.push_str(&text_group(
        "label-group text",
        0.0,
        top - name.bbox.y,
        &name,
        "",
    ));
    svg.push_str(&paint(head));

    let mut dividers = String::new();
    let mut divider_y = y + head_height;
    for (class, rows, tops, compartment_height) in [
        ("members-group text", &members, &member_tops, members_height),
        ("methods-group text", &methods, &method_tops, methods_height),
    ] {
        if rows.is_empty() {
            continue;
        }
        dividers.push_str(&format!(
            "<line class=\"divider\" x1=\"{}\" y1=\"{y}\" x2=\"{}\" y2=\"{y}\"{}/>",
            number(x),
            number(x + width),
            style,
            y = number(divider_y),
        ));
        let mut group = String::new();
        for (row, &row_top) in rows.iter().zip(tops) {
            let baseline_y = divider_y + row_top - row.text.bbox.y;
            if let Some(mark) = &row.mark {
                group.push_str(&text_group(
                    "visibility",
                    x + PADDING_X,
                    baseline_y,
                    mark,
                    "",
                ));
            }
            group.push_str(&text_group(
                "label",
                x + PADDING_X + column,
                baseline_y,
                &row.text,
                row.style,
            ));
        }
        svg.push_str(&paint(format!("<g class=\"{class}\">{group}</g>")));
        divider_y += compartment_height;
    }
    svg.push_str(&dividers);

    let bbox = BBox::new(x, y, width, height);
    let texts = annotation
        .into_iter()
        .chain(std::iter::once(name))
        .chain(
            members
                .into_iter()
                .chain(methods)
                .flat_map(|row| row.mark.into_iter().chain(std::iter::once(row.text))),
        )
        .collect();
    Ok(DesignClassBox {
        part: Part::without_label(svg, bbox, Intersect::Rect),
        texts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_visibility_mark_is_split_from_its_member() {
        assert_eq!(
            split_mark("\\+area() : double"),
            (Some('+'), "area() : double")
        );
        assert_eq!(split_mark("-radius"), (Some('-'), "radius"));
        assert_eq!(split_mark("PENDING"), (None, "PENDING"));
        assert_eq!(split_mark("+"), (None, "+"));
    }

    #[test]
    fn the_first_annotation_gives_the_role() {
        let with = |annotation: &str| ClassNodeData {
            annotations: vec![annotation.to_owned()],
            ..ClassNodeData::default()
        };
        assert_eq!(role(&with("interface")), Some(Role::Interface));
        assert_eq!(role(&with("Abstract")), Some(Role::Interface));
        assert_eq!(role(&with("enumeration")), Some(Role::Enumeration));
        assert_eq!(role(&with("service")), None);
        assert_eq!(role(&ClassNodeData::default()), None);
    }
}
