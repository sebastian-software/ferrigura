//! The use case's ellipse and business ellipse: ports of `rendering-elements/shapes/usecaseEllipse.ts` and
//! `usecaseBusiness.ts` in the plain elements of the neo and classic looks.
//!
//! The ellipse is the label's box grown by twice the half padding on each side (`usecaseEllipse.ts`
//! 24-28). A business ellipse adds a stereotype line above the label and a chord across its upper part
//! (`usecaseBusiness.ts` 68-86), placed from the radii as the reference computes them.

use super::Intersect;
use super::ShapeInput;
use super::usecase_actor::{drawn_label, insert_label, label_group, node_width, position_label};
use super::util::{self, Part, attribute, label_helper, styles2string};
use crate::RenderError;
use crate::svg::number;
use crate::unified::geom::BBox;
use crate::unified::label::LabelLayout;

/// `usecaseEllipse`: the ellipse around the label, with the label centred on the origin.
pub(super) fn ellipse(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let label = label_helper(input, false);
    // `halfPadding ?? 10`: the label's half padding, which is the node's padding over two.
    let half_padding = node.padding / 2.0;
    let radius_x = label.bbox.width / 2.0 + half_padding * 2.0;
    let radius_y = label.bbox.height / 2.0 + half_padding * 2.0;
    let svg = format!(
        "<ellipse class=\"basic label-container\"{} rx=\"{}\" ry=\"{}\" cx=\"0\" cy=\"0\"/>",
        attribute("style", &styles.node),
        number(radius_x),
        number(radius_y),
    );
    Part {
        svg,
        after_label: String::new(),
        bbox: BBox::new(-radius_x, -radius_y, radius_x * 2.0, radius_y * 2.0),
        intersect: Intersect::Ellipse {
            rx: radius_x,
            ry: radius_y,
        },
        label_offset: Some(util::centred_label_offset(&label)),
        label: Some(label),
    }
}

/// `usecaseBusiness`: the ellipse around the label and the stereotype above it, with a chord across its
/// upper part. Returns the stereotype's label with the part, so that the fonts cover it.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read.
pub(super) fn business(input: &ShapeInput<'_>) -> Result<(Part, Vec<LabelLayout>), RenderError> {
    let node = input.node;
    let styles = styles2string(node);
    let label = label_helper(input, false);
    let stereotype_text = node
        .use_case
        .as_ref()
        .and_then(|fields| fields.stereotype.clone());
    let stereotype = match stereotype_text {
        Some(text) => Some(insert_label(
            input,
            &format!("«{text}»"),
            &styles.label,
            node_width(input),
        )?),
        None => None,
    };
    let stereotype_height = stereotype.as_ref().map_or(0.0, |label| label.bbox.height);
    let stereotype_width = stereotype.as_ref().map_or(0.0, |label| label.bbox.width);
    let label_gap = if stereotype.is_some() { 2.0 } else { 0.0 };
    let label_height = label.bbox.height + stereotype_height + label_gap;
    let label_width = label.bbox.width.max(stereotype_width);
    let padding = node.padding / 2.0;
    let radius_x = label_width / 2.0 + padding * 2.0;
    let radius_y = label_height / 2.0 + padding * 2.0;
    // `markerInset` and the chord's ends on the ellipse (`usecaseBusiness.ts` 75-81).
    let marker_inset = (padding / 5.0).max(1.0).min(padding / 2.0);
    let marker_start_x = label_width / 2.0 + marker_inset;
    let marker_end_x = radius_x - marker_inset;
    let normalised_start = marker_start_x / radius_x;
    let normalised_end = marker_end_x / radius_x;
    let marker_start_y = radius_y * (1.0 - normalised_start * normalised_start).max(0.0).sqrt();
    let marker_end_y = -radius_y * (1.0 - normalised_end * normalised_end).max(0.0).sqrt();

    let mut after = String::new();
    let mut texts = Vec::new();
    if let Some(stereotype) = &stereotype {
        after.push_str(&label_group(
            "label usecase-stereotype",
            &styles.label,
            position_label(stereotype, -label_height / 2.0 + stereotype_height / 2.0),
            stereotype,
            &styles.label,
        ));
        texts.push(stereotype.clone());
    }
    let label_offset = position_label(&label, label_height / 2.0 - label.bbox.height / 2.0);
    let node_style = if styles.node.is_empty() {
        String::new()
    } else {
        attribute("style", &styles.node)
    };
    after.push_str(&format!(
        "<path class=\"usecase-business-marker\" d=\"M {} {} L {} {}\" fill=\"none\"{node_style}/>",
        number(marker_start_x),
        number(marker_start_y),
        number(marker_end_x),
        number(marker_end_y),
    ));

    let svg = format!(
        "<ellipse class=\"basic label-container usecase-business-ellipse\"{} rx=\"{}\" ry=\"{}\" cx=\"0\" cy=\"0\"/>",
        attribute("style", &styles.node),
        number(radius_x),
        number(radius_y),
    );
    let drawn = drawn_label(input, &label)?;
    let part = Part {
        svg,
        after_label: after,
        bbox: BBox::new(-radius_x, -radius_y, radius_x * 2.0, radius_y * 2.0),
        intersect: Intersect::Ellipse {
            rx: radius_x,
            ry: radius_y,
        },
        label_offset: Some(label_offset),
        label: Some(drawn),
    };
    Ok((part, texts))
}
