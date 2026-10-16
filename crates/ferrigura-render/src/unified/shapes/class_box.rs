//! The class box: a port of `rendering-elements/shapes/classBox.ts` with the text rows of
//! `diagrams/class/shapeUtil.ts` (`textHelper` and `addText`), for the HTML-free labels the harness
//! forces (`htmlLabels: false`, so `TEXT_PADDING` is 3 and every row takes the `- 4` shift). The
//! research is in `docs-port/class/NOTES.md` section 5.3.
//!
//! The box is three text groups and two dividers inside a rough outline. Each group is measured
//! where the reference measures it: `textHelper` lays the groups out first, `classBox` reads their
//! `getBBox` heights (with `P/2` taken off when the empty box is drawn) and moves them. Every value
//! that the reference reads from a `getBBox` is rounded to float32 (`BBox::browser`), as the browser
//! returns it, and an empty box (no width or no height) is left out of a union, as `getBBox` does.
//!
//! The reference's quirks are kept: an empty annotation or members group has a negative height
//! once `P/2` is taken off (the `|| 0` keeps it), the methods group is placed from the members'
//! height floored at `P/2`, and the second divider sits at the members' height, which is not the
//! height of the box that `textHelper` measured.

use ferrigura_model::class::ClassMember;
use ferrigura_model::flowchart::LabelType;
use ferrigura_syntax::sanitize::sanitize_text;
use ferrigura_text::{FontFamily, FontStyle, FontWeight, face, measure};

use super::util::{self, Part};
use super::{Intersect, ShapeInput, rough};
use crate::RenderError;
use crate::svg::{escape, number};
use crate::unified::data::ClassNodeData;
use crate::unified::geom::{BBox, Point};
use crate::unified::label::{self, Anchor, LabelLayout, LabelSpec, layout_inheriting};

/// `config.class.padding ?? 12` (`classBox.ts` 18-19); `GAP` is the same value.
const DEFAULT_PADDING: f64 = 12.0;

/// `TEXT_PADDING` of a label without HTML (`shapeUtil.ts` 22): the gap between two rows.
const TEXT_PADDING: f64 = 3.0;

/// The extra width a row's wrapping width takes over its own width (`shapeUtil.ts` 131).
const WRAP_SLACK: f64 = 50.0;

/// The shift of every group for a label without HTML (`classBox.ts` 182-186).
const SVG_LABEL_SHIFT: f64 = 4.0;

/// The height a divider's second end adds to its first one (`classBox.ts` 207-237).
const DIVIDER_THICKNESS: f64 = 0.001;

/// A text row of a class box: the `g.label` of `addText`, with the box of its text.
#[derive(Debug)]
struct Row {
    /// The `<g class="label">`, with its translation, and the text.
    svg: String,
    /// The text's box, before the row's own translation (`text.getBBox()`, float32).
    bbox: BBox,
    /// The row's translation in y: `-H / (2 · lines) + yOffset`.
    dy: f64,
    /// The row's laid-out text, whose fonts the diagram embeds.
    label: LabelLayout,
}

/// One of the four text groups: the annotation, the name, the members and the methods.
#[derive(Debug)]
struct Group {
    /// The group's class attribute, as the reference names it (`annotation-group text`, ...).
    class: &'static str,
    /// The rows, in order.
    rows: Vec<Row>,
}

impl Group {
    /// `getBBox()` of the group: the union of its rows' boxes, each moved by its translation, with
    /// the empty boxes left out. A group without rows has the empty box.
    fn bbox(&self) -> BBox {
        union_all(self.rows.iter().map(|row| row.bbox.translate(0.0, row.dy))).browser()
    }

    /// The group's element with the translation `transform`, and its rows.
    fn svg(&self, x: f64, y: f64) -> String {
        let rows: String = self.rows.iter().map(|row| row.svg.as_str()).collect();
        format!(
            "<g class=\"{}\" transform=\"translate({}, {})\">{rows}</g>",
            self.class,
            number(x),
            number(y),
        )
    }
}

/// The union of the boxes that `getBBox` would count: the empty ones are left out. `None` when none
/// counts.
fn union_all(boxes: impl IntoIterator<Item = BBox>) -> BBox {
    let mut all: Option<BBox> = None;
    for bbox in boxes {
        if bbox.is_empty() {
            continue;
        }
        all = Some(all.map_or(bbox, |all| all.union(bbox)));
    }
    all.unwrap_or_default()
}

/// The `|| 0` of the reference: NaN and zero (negative zero included) become zero, anything else
/// stays.
fn or_zero(value: f64) -> f64 {
    if value.is_nan() || value == 0.0 {
        0.0
    } else {
        value
    }
}

/// `parseClassifier` (`classTypes.ts` 136-146): the style of a member's classifier.
fn classifier_style(member: &ClassMember) -> &'static str {
    match member.classifier.as_str() {
        "*" => "font-style:italic;",
        "$" => "text-decoration:underline;",
        _ => "",
    }
}

/// `addText` (`shapeUtil.ts` 99-189) for a row without HTML: the text is sanitised as the
/// reference sanitises it, laid out at its wrapping width (its own width plus 50, from the
/// measured text), and moved to `yOffset` on its centre. `bold` is the label row, whose tspans
/// clear their weight so that the group's `font-weight: bolder` shows.
fn add_text(
    input: &ShapeInput<'_>,
    font_size: f64,
    text: &str,
    style: &str,
    y_offset: f64,
    bold: bool,
) -> Result<Row, RenderError> {
    // `textContent.substring(1)` when the text starts with a backslash (`shapeUtil.ts` 120-122).
    let content = text.strip_prefix('\\').unwrap_or(text);
    let sanitized = sanitize_text(&label::decode_entities(content));
    let sans = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
    // `calculateTextWidth(textContent, config) + 50`: the DOM width of the text, rounded to a pixel.
    let width = measure(sans, content, input.config.font_size)?.round() + WRAP_SLACK;
    let spec = LabelSpec {
        text: &sanitized,
        label_type: LabelType::Markdown,
        width,
        is_node: true,
        anchor: Anchor::Start,
        add_background: false,
        styles: &[],
        font_size,
    };
    let inherited = bold.then_some(FontWeight::Bold);
    let layout: LabelLayout = layout_inheriting(&spec, inherited)?;
    let bbox = layout.bbox.browser();
    // `numberOfLines` is the number of tspans, which is one for an empty text.
    let lines = layout.lines.len().max(1) as f64;
    let dy = -bbox.height / (2.0 * lines) + y_offset;
    let svg = format!(
        "<g class=\"label\" style=\"{}\" transform=\"translate(0,{})\">{}</g>",
        escape(style),
        number(dy),
        layout.svg(""),
    );
    Ok(Row {
        svg,
        bbox,
        dy,
        label: layout,
    })
}

/// The divider between two groups: `rc.line` at roughness 0, a stroke path only.
fn divider(x1: f64, y1: f64, x2: f64, y2: f64, styles: &str, stroke_width: &str) -> String {
    rough::line(
        Point::new(x1, y1),
        Point::new(x2, y2),
        Some(styles),
        stroke_width,
    )
}

/// What a class box draws: its part, the box the drawing covers, and the texts of its rows.
#[derive(Debug)]
pub(super) struct ClassBox {
    /// The part, with the node's outline as its box.
    pub part: Part,
    /// The box of every element of the class box.
    pub covered: BBox,
    /// The laid-out rows, whose fonts the diagram embeds.
    pub texts: Vec<LabelLayout>,
}

/// Draws a class box: `classBox.ts` with `textHelper`. The shape is the whole node group, its rough
/// outline and rows, and its two dividers; the node's box is the outline's (`updateNodeBounds`).
/// The second value is the drawing's box, the union of the parts, which the viewBox covers.
pub(super) fn class_box(input: &ShapeInput<'_>) -> Result<ClassBox, RenderError> {
    let config = input.config;
    let padding = config.class.padding.unwrap_or(DEFAULT_PADDING);
    let gap = padding;
    let empty = ClassNodeData::default();
    let data = input.node.class.as_ref().unwrap_or(&empty);
    let font_size = util::theme_number(input.theme, "fontSize").unwrap_or(config.font_size);

    // The style of the paths is the class's own declarations, joined (`classNode.styles`), and the
    // stroke width is read from them as `userNodeOverrides` reads them.
    let styles = data.styles.join(";");
    let mut styled = input.node.clone();
    styled.css_styles = data.styles.clone();
    let stroke_width = util::stroke_width(&util::styles2string(&styled));

    let no_members = data.members.is_empty() && data.methods.is_empty();
    let render_extra = no_members && !config.class.hide_empty_members_box;

    // The rows, as `textHelper` lays them out.
    let mut annotation = Group {
        class: "annotation-group text",
        rows: Vec::new(),
    };
    if let Some(first) = data.annotations.first() {
        annotation.rows.push(add_text(
            input,
            font_size,
            &format!("«{first}»"),
            "",
            0.0,
            false,
        )?);
    }
    let name = Group {
        class: "label-group text",
        rows: vec![add_text(
            input,
            font_size,
            &data.text,
            "font-weight: bolder",
            0.0,
            true,
        )?],
    };
    let members = member_group("members-group text", input, font_size, &data.members)?;
    let methods = member_group("methods-group text", input, font_size, &data.methods)?;

    let annotation_box = annotation.bbox();
    let label_box = name.bbox();
    let members_box = members.bbox();
    let annotation_height = annotation_box.height;
    let label_height = label_box.height;
    // `textHelper` falls back to `GAP / 2` for an empty members group (`shapeUtil.ts` 58-61).
    let members_height_text = if members_box.height <= 0.0 {
        gap / 2.0
    } else {
        members_box.height
    };

    // The shape's box as `textHelper` leaves it: the groups at their first translations.
    let text_bbox = union_all([
        annotation_box.translate(-annotation_box.width / 2.0, 0.0),
        label_box.translate(-label_box.width / 2.0, annotation_height),
        members_box.translate(0.0, annotation_height + label_height + gap * 2.0),
        methods.bbox().translate(
            0.0,
            annotation_height
                + label_height
                + if members_height_text != 0.0 {
                    members_height_text + gap * 4.0
                } else {
                    gap * 2.0
                },
        ),
    ])
    .browser();

    // The box's size (`classBox.ts` 57-75): the text, plus the room the empty and the one-group
    // boxes need, and the rectangle around it.
    let width = text_bbox.width;
    let mut height = text_bbox.height;
    if no_members {
        height += gap;
    } else if !data.members.is_empty() && data.methods.is_empty() {
        height += gap * 2.0;
    }
    let x = -width / 2.0;
    let y = -height / 2.0;
    let extra_height = if render_extra {
        padding * 2.0
    } else if no_members {
        -padding
    } else {
        0.0
    };
    let rect_offset = if render_extra {
        padding
    } else if no_members {
        -padding / 2.0
    } else {
        0.0
    };
    let rect_x = x - padding;
    let rect_y = y - padding - rect_offset;
    let rect_width = width + 2.0 * padding;
    let rect_height = height + 2.0 * padding + extra_height;
    let rect_box = BBox::new(rect_x, rect_y, rect_width, rect_height).browser();

    // The heights that `classBox` reads back from the groups, less `P/2` when the empty box is
    // drawn (`classBox.ts` 97-105). The `|| 0` keeps a negative height.
    let correction = if render_extra { padding / 2.0 } else { 0.0 };
    let annotation_h = or_zero(annotation_box.height - correction);
    let label_h = or_zero(label_box.height - correction);
    let members_h = or_zero(members_box.height - correction);

    // The groups' positions (`classBox.ts` 138-200): each keeps its translation, moved by the
    // offset of the box, and shifted up by four when the labels are not HTML. An empty box with
    // `hideEmptyMembersBox` keeps the translation instead, less a gap under an annotation (171-181).
    let offset = y + padding - rect_offset;
    let hide_empty = no_members && config.class.hide_empty_members_box;
    let kept = |translate_y: f64| -> f64 {
        if data.annotations.is_empty() {
            translate_y
        } else {
            translate_y - gap
        }
    };
    let annotation_y = if hide_empty { kept(0.0) } else { offset };
    let label_y = if hide_empty {
        kept(annotation_height)
    } else {
        annotation_height + offset
    };
    let members_y = if hide_empty {
        kept(annotation_height + label_height + gap * 2.0)
    } else {
        annotation_height + label_height + gap * 2.0 + offset
    };
    let methods_translate = annotation_height
        + label_height
        + if members_height_text != 0.0 {
            members_height_text + gap * 4.0
        } else {
            gap * 2.0
        };
    let methods_y = if hide_empty {
        kept(methods_translate)
    } else {
        annotation_h + label_h + members_h.max(gap / 2.0) + y + gap * 4.0 + padding
    };
    let annotation_at = (
        or_zero(-annotation_box.width / 2.0),
        annotation_y - SVG_LABEL_SHIFT,
    );
    let label_at = (or_zero(-label_box.width / 2.0), label_y - SVG_LABEL_SHIFT);
    let members_at = (x, members_y - SVG_LABEL_SHIFT);
    let methods_at = (x, methods_y - SVG_LABEL_SHIFT);

    // The dividers (`classBox.ts` 206-237): under the name, and under the members.
    let mut dividers = String::new();
    let mut divider_boxes = Vec::new();
    let divider_class = if input.node.look == "neo" {
        "divider neo-line"
    } else {
        "divider"
    };
    let has_rows_below = !no_members || render_extra;
    let mut push_divider = |at_y: f64| {
        let (x1, x2) = (rect_box.x, rect_box.right());
        dividers.push_str(&format!(
            "<g class=\"{divider_class}\" style=\"{}\">{}</g>",
            escape(&styles),
            divider(
                x1,
                at_y,
                x2,
                at_y + DIVIDER_THICKNESS,
                &styles,
                &stroke_width
            ),
        ));
        divider_boxes.push(BBox::new(x1, at_y, x2 - x1, DIVIDER_THICKNESS));
    };
    if has_rows_below {
        push_divider(annotation_h + label_h + y + padding);
        push_divider(annotation_h + label_h + members_h + y + gap * 2.0 + padding);
    }

    let rect_svg = format!(
        "<g class=\"basic label-container outer-path\">{}</g>",
        rough::rectangle_paths(
            rect_x,
            rect_y,
            rect_width,
            rect_height,
            Some(styles.as_str()),
            &stroke_width,
        ),
    );
    let annotation_svg = annotation.svg(annotation_at.0, annotation_at.1);
    let label_svg = name.svg(label_at.0, label_at.1);
    let members_svg = members.svg(members_at.0, members_at.1);
    let methods_svg = methods.svg(methods_at.0, methods_at.1);

    // The drawing's box: every part, moved to its place (the viewBox covers it).
    let mut parts = vec![rect_box];
    for (group, (at_x, at_y)) in [
        (&annotation, annotation_at),
        (&name, label_at),
        (&members, members_at),
        (&methods, methods_at),
    ] {
        parts.push(group.bbox().translate(at_x, at_y));
    }
    parts.extend(divider_boxes);
    let extent = union_all(parts);

    let part = Part {
        svg: format!("{rect_svg}{annotation_svg}{label_svg}{members_svg}{methods_svg}{dividers}"),
        after_label: String::new(),
        bbox: rect_box,
        intersect: Intersect::Rect,
        label_offset: None,
        label: None,
    };
    let texts = annotation
        .rows
        .into_iter()
        .chain(name.rows)
        .chain(members.rows)
        .chain(methods.rows)
        .map(|row| row.label)
        .collect();
    Ok(ClassBox {
        part,
        covered: extent,
        texts,
    })
}

/// A group of member rows, as `textHelper` writes them: each row below the last one, with
/// `TEXT_PADDING` between them, and the classifier's style on its row.
fn member_group(
    class: &'static str,
    input: &ShapeInput<'_>,
    font_size: f64,
    members: &[ClassMember],
) -> Result<Group, RenderError> {
    let mut rows = Vec::with_capacity(members.len());
    let mut y_offset = 0.0;
    for member in members {
        let row = add_text(
            input,
            font_size,
            &member.text,
            classifier_style(member),
            y_offset,
            false,
        )?;
        y_offset += row.bbox.height + TEXT_PADDING;
        rows.push(row);
    }
    Ok(Group { class, rows })
}
