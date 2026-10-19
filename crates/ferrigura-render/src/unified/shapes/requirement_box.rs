//! The requirement box: a port of `rendering-util/rendering-elements/shapes/requirementBox.ts`.
//!
//! A requirement or an element is a stack of texts inside an outline: the type (`<<Requirement>>`,
//! or `<<Element>>`), the name in bold, then the body (ID, Text, Risk and Verification for a
//! requirement; Type and Doc Ref for an element), with a divider under the name when the body has a
//! text. The shape draws its own labels, as `erBox` does (`Kind::draws_label` is false), so it
//! returns its whole output.
//!
//! The reference lays the texts out one after another, each at the height the one above it took,
//! and measures each with `getBBox` plus 6 pixels of height; the outline is the box of the texts
//! plus 20 on each side; then every text is moved to its place in the outline by its index among the
//! labels (the type and the name keep their centred x, the body starts 10 pixels in). The research,
//! with the reference's lines for every rule, is in `docs-port/requirement/NOTES.md`, section 6.4.

use ferrigura_model::flowchart::LabelType;
use ferrigura_syntax::sanitize::sanitize_text;
use ferrigura_text::FontWeight;

use super::er_box::calculate_text_width;
use super::rough::{self, Attrs, Fill, Paint};
use super::util::{self, attribute};
use super::{Intersect, ShapeInput, ShapeOutput};
use crate::RenderError;
use crate::common::decode_entities;
use crate::unified::data::RequirementFields;
use crate::unified::geom::{BBox, Point};
use crate::unified::label::{self, Anchor, LabelLayout, LabelSpec};
use crate::unified::palette;

/// The padding of the outline around the texts, and the gap under the type and the name
/// (`requirementBox.ts` 23-24).
const PADDING: f64 = 20.0;

/// The gap under the type and the name, which the body texts start after (`gap`, line 24).
const GAP: f64 = 20.0;

/// The height a text takes beyond its box, so that it is like the HTML label (`requirementBox.ts` 247).
const LABEL_EXTRA_HEIGHT: f64 = 6.0;

/// The width a text may take before it wraps, beyond its own width (`requirementBox.ts` 222).
const WRAP_MARGIN: f64 = 50.0;

/// The thickness of the neo divider, a polygon (`requirementBox.ts` 172).
const DIVIDER_THICKNESS: f64 = 0.001;

/// One text of the box: its label, the style of its group, and where it sits. `translate` is the
/// group's transform after `addText` placed it, which the re-translation then replaces.
struct Text {
    /// The laid-out text.
    label: LabelLayout,
    /// The style of the text's group (`textEl`), with the bold of the name.
    style: String,
    /// The group's transform after `addText`: centred on its own box, offset by its `yOffset`.
    translate: Point,
}

impl Text {
    /// The text's box in the group, after the group's transform.
    fn placed(&self, translate: Point) -> BBox {
        self.label.bbox.translate(translate.x, translate.y)
    }
}

/// A body text with its prefix, or nothing when the value is unset (`requirementBox.ts` 63-105: the
/// reference writes `ID: <id>` only for a non-empty id).
fn labelled(prefix: &str, value: &str) -> String {
    if value.is_empty() {
        String::new()
    } else {
        format!("{prefix}{value}")
    }
}

/// Draws a requirement or an element (`requirementBox.ts` 15-202).
///
/// # Errors
///
/// Returns an error if the node carries no requirement fields, or a bundled font cannot be read.
pub(super) fn requirement_box(input: &ShapeInput<'_>) -> Result<ShapeOutput, RenderError> {
    let node = input.node;
    let Some(fields) = node.requirement.as_ref() else {
        return Err(RenderError::Options(format!(
            "the requirement box {} has no requirement fields",
            node.id
        )));
    };
    let styles = util::styles2string(node);
    let label_style = styles.label.clone();
    let node_style = styles.node.clone();
    let font_size =
        util::theme_number(&input.config.theme.font_size).unwrap_or(input.config.globals.font_size);
    let mut texts: Vec<Text> = Vec::new();

    // The type, then the name, which is bold (`requirementBox.ts` 40-57). Their heights are the
    // labels' boxes plus 6, and the body starts under the name.
    let mut build = Builder {
        input,
        font_size,
        texts: &mut texts,
    };
    let type_height = match fields {
        RequirementFields::Requirement { kind, .. } => {
            build.add(&format!("&lt;&lt;{kind}&gt;&gt;"), 0.0, &label_style)?
        }
        RequirementFields::Element { .. } => {
            build.add("&lt;&lt;Element&gt;&gt;", 0.0, &label_style)?
        }
    };
    let name = match fields {
        RequirementFields::Requirement { name, .. } | RequirementFields::Element { name, .. } => {
            name
        }
    };
    let name_height = build.add(
        name,
        type_height,
        &format!("{label_style}; font-weight: bold;"),
    )?;
    let mut accumulated = type_height + name_height + GAP;

    // The body (`requirementBox.ts` 59-110). A requirement adds the heights of ID, Text and Risk,
    // not of Verification; an element adds the height of Type, not of Doc Ref.
    match fields {
        RequirementFields::Requirement {
            requirement_id,
            text,
            risk,
            verify_method,
            ..
        } => {
            let id = build.add(&labelled("ID: ", requirement_id), accumulated, &label_style)?;
            accumulated += id;
            let body = build.add(&labelled("Text: ", text), accumulated, &label_style)?;
            accumulated += body;
            let level = build.add(&labelled("Risk: ", risk), accumulated, &label_style)?;
            accumulated += level;
            build.add(
                &labelled("Verification: ", verify_method),
                accumulated,
                &label_style,
            )?;
        }
        RequirementFields::Element { kind, doc_ref, .. } => {
            let kind_height = build.add(&labelled("Type: ", kind), accumulated, &label_style)?;
            accumulated += kind_height;
            build.add(&labelled("Doc Ref: ", doc_ref), accumulated, &label_style)?;
        }
    }

    // The outline is the box of the texts as the group has them, plus the padding (lines 112-115).
    let group = texts
        .iter()
        .map(|text| text.placed(text.translate))
        .reduce(BBox::union)
        .unwrap_or_default()
        .browser();
    let total_width = group.width + PADDING;
    let total_height = group.height + PADDING;
    let x = -total_width / 2.0;
    let y = -total_height / 2.0;

    // The outline and the divider take the node's fill and stroke, else the theme's (`userNodeOverrides`,
    // `requirementBox.ts` 119-125): roughness 0 and a solid fill for every look but handDrawn.
    let theme_fill = input.config.theme.main_bkg.text().unwrap_or_default();
    let fill = style_value(&styles, "fill").unwrap_or(theme_fill);
    let stroke = style_value(&styles, "stroke")
        .unwrap_or_else(|| input.config.theme.node_border.text().unwrap_or_default());
    let stroke_width = util::stroke_width(&styles);
    // Every path takes the node's style when the node has one and the theme has an edge label
    // background or a border palette (`requirementBox.ts` 193-199).
    let border = palette::palette(input.config.globals, input.config.theme);
    let paths_styled = !node_style.is_empty()
        && (input
            .config
            .theme
            .requirement_edge_label_background
            .text()
            .is_some_and(|colour| !colour.is_empty())
            || border.is_some());
    let path_style = paths_styled.then_some(node_style.as_str());

    // The re-translation (`requirementBox.ts` 138-165): the type and the name keep their x, the body
    // starts at `x + 10`, and every label moves up to the outline's top plus the padding.
    let mut svg = String::new();
    let outline = rough::rectangle(x, y, total_width, total_height, Paint::BOTH);
    svg.push_str(&format!(
        "<g class=\"basic label-container outer-path\"{}>{}</g>",
        attribute("style", &node_style),
        rough::attributed(
            &outline,
            &Attrs {
                style: path_style,
                fill: Some(Fill {
                    colour: &fill,
                    evenodd: false,
                }),
                stroke: Some(&stroke),
                stroke_width: &stroke_width,
            },
        ),
    ));

    let mut placed_boxes = vec![BBox::new(x, y, total_width, total_height)];
    for (index, text) in texts.iter().enumerate() {
        let translate_x = if index < 2 {
            text.translate.x
        } else {
            x + PADDING / 2.0
        };
        let translate = Point::new(translate_x, text.translate.y - total_height / 2.0 + PADDING);
        placed_boxes.push(text.placed(translate));
        svg.push_str(&format!(
            "<g class=\"label\"{} transform=\"translate({}, {})\">{}</g>",
            attribute("style", &text.style),
            crate::svg::number(translate.x),
            crate::svg::number(translate.y),
            text.label.svg(&util::node_text_style(&text.style)),
        ));
    }

    // The divider under the name, when the body has a text (`requirementBox.ts` 167-185): a
    // requirement has one when ID, Text or Risk is there, an element when Type is.
    if accumulated > type_height + name_height + GAP {
        let line_y = y + type_height + name_height + GAP;
        let divider = if input.node.look == "neo" {
            let points = [
                Point::new(x, line_y),
                Point::new(x + total_width, line_y),
                Point::new(x + total_width, line_y + DIVIDER_THICKNESS),
                Point::new(x, line_y + DIVIDER_THICKNESS),
            ];
            let outline = rough::polygon(&points, Paint::BOTH);
            rough::attributed(
                &outline,
                &Attrs {
                    style: path_style,
                    fill: Some(Fill {
                        colour: &fill,
                        evenodd: true,
                    }),
                    stroke: Some(&stroke),
                    stroke_width: &stroke_width,
                },
            )
        } else {
            let outline = rough::line(
                Point::new(x, line_y),
                Point::new(x + total_width, line_y),
                Paint::BOTH,
            );
            rough::attributed(
                &outline,
                &Attrs {
                    style: path_style,
                    fill: None,
                    stroke: Some(&stroke),
                    stroke_width: &stroke_width,
                },
            )
        };
        svg.push_str(&format!("<g class=\"divider\">{divider}</g>"));
    }

    let bbox = placed_boxes
        .into_iter()
        .reduce(BBox::union)
        .unwrap_or_default();
    Ok(ShapeOutput {
        svg,
        after_label: String::new(),
        label_offset: None,
        label: None,
        texts: texts.into_iter().map(|text| text.label).collect(),
        group_attributes: colour_group_attribute(input),
        colour_slot: String::new(),
        // `updateNodeBounds(node, rect)` reads the outline (`requirementBox.ts` 187).
        element: BBox::new(x, y, total_width, total_height),
        bbox,
        intersect: Intersect::Rect,
        extent: None,
    })
}

/// Adds the texts of the box in their order, as `addText` does (`requirementBox.ts` 204-263).
struct Builder<'a, 'b> {
    input: &'a ShapeInput<'b>,
    font_size: f64,
    texts: &'a mut Vec<Text>,
}

impl Builder<'_, '_> {
    /// Adds one text at `y_offset` and returns its height: zero and no text for an empty input. The
    /// width it wraps at comes from the input as written, with its entities (`calculateTextWidth`
    /// of the raw text), and the text is drawn decoded and sanitised.
    fn add(&mut self, raw: &str, y_offset: f64, style: &str) -> Result<f64, RenderError> {
        if raw.is_empty() {
            return Ok(0.0);
        }
        let width = calculate_text_width(raw, self.input.config.globals.font_size)? + WRAP_MARGIN;
        let text = sanitize_text(&decode_entities(raw));
        let declarations: Vec<String> = style
            .split(';')
            .filter(|declaration| !declaration.is_empty())
            .map(str::to_owned)
            .collect();
        let label = label::layout_inheriting(
            &LabelSpec {
                text: &text,
                label_type: LabelType::Markdown,
                width,
                is_node: true,
                // The lines start at the group's origin: the reference's text has no `text-anchor`
                // of its own (`createFormattedText`), which is the `start` of SVG text.
                anchor: Anchor::Start,
                wrap_in_style: true,
                add_background: false,
                styles: &declarations,
                font_size: self.font_size,
                title: false,
            },
            style_weight(style),
        )?;
        // `getBBox` is a float32 box; the height gains 6, and the group is centred on the box.
        let bbox = label.bbox.browser();
        let height = bbox.height + LABEL_EXTRA_HEIGHT;
        let translate = Point::new(-bbox.width / 2.0, -height / 2.0 + y_offset);
        self.texts.push(Text {
            label,
            style: style.to_owned(),
            translate,
        });
        Ok(height)
    }
}

/// The weight that a group's style gives its words. The reference writes the style on the text's
/// lines, and a `font-weight` there sets the measured weight of every word, whatever the word's own
/// `strong` (`requirementBox.ts` 214-237; the name's bold and a node's `font-weight:bold` are both
/// measured bold). The last declaration wins, and `!important` is not part of the value.
fn style_weight(style: &str) -> Option<FontWeight> {
    let value = style.split(';').rev().find_map(|declaration| {
        let (key, value) = declaration.split_once(':')?;
        (key.trim() == "font-weight").then(|| value.replace("!important", "").trim().to_owned())
    })?;
    match value.to_ascii_lowercase().as_str() {
        "bold" | "bolder" => Some(FontWeight::Bold),
        "normal" | "lighter" => Some(FontWeight::Normal),
        numeric => numeric.parse::<u32>().ok().map(|weight| {
            if weight >= 600 {
                FontWeight::Bold
            } else {
                FontWeight::Normal
            }
        }),
    }
}

/// The `data-color-id` of the box: `color-<colorIndex % borderColorArray.length>` on a colour theme
/// (`requirementBox.ts` 133-136).
fn colour_group_attribute(input: &ShapeInput<'_>) -> String {
    let Some(palette) = palette::palette(input.config.globals, input.config.theme) else {
        return String::new();
    };
    let slot = palette::palette_slot(Some(input.node.color_index.unwrap_or(0)), &palette);
    attribute("data-color-id", &format!("color-{}", slot.unwrap_or(0)))
}

/// The value of a declaration in the compiled styles of the node, as `stylesMap.get` gives it.
fn style_value(styles: &util::Styles, key: &str) -> Option<String> {
    styles
        .entries
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_padding_and_gap_are_the_reference_constants() {
        assert_eq!(PADDING, 20.0);
        assert_eq!(GAP, 20.0);
    }

    #[test]
    fn a_font_weight_in_the_style_sets_the_weight_of_every_word() {
        assert_eq!(
            style_weight("fill:red !important;font-weight:bold !important"),
            Some(FontWeight::Bold)
        );
        assert_eq!(style_weight("font-weight: 700"), Some(FontWeight::Bold));
        assert_eq!(style_weight("font-weight:normal"), Some(FontWeight::Normal));
        assert_eq!(style_weight("fill:red !important"), None);
    }
}
