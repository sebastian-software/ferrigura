//! The actor glyphs of the use case diagram: a port of `rendering-elements/shapes/usecaseActor.ts` and its
//! variants `usecaseActorHollow.ts`, `usecaseActorAwesome.ts` and `usecaseActorIcon.ts`, which share one
//! handler (`renderUsecaseActor`). The figure is drawn in the plain elements that the neo and classic
//! looks draw; the hand-drawn look is out of scope.
//!
//! The label sits under the figure, and a stereotype above it: `label`, then the stereotype and the
//! figure (`after_label`), as the handler appends them. The outline is an invisible rectangle around the
//! whole, which sizes the node. See `docs-port/usecase/NOTES.md` section 6.6.

use ferrigura_syntax::sanitize::sanitize_text;

use super::icon::fallback_icon_svg;
use super::registry::Kind;
use super::util::{self, Part, attribute, label_helper, styles2string};
use super::{Intersect, ShapeInput};
use crate::RenderError;
use crate::common::decode_entities;
use crate::svg::number;
use crate::unified::geom::{BBox, Point};
use crate::unified::label::{self, Anchor, LabelLayout, LabelSpec};
use ferrigura_model::flowchart::LabelType;

/// The glyph's variant (`UsecaseActorVariant`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Variant {
    /// The stick figure.
    Normal,
    /// The hollow figure: a circle head and an outlined body.
    Hollow,
    /// The silhouette of a user.
    Awesome,
    /// The icon in a frame.
    Icon,
}

impl Variant {
    /// The name of the variant in the classes of the node (`usecase-actor-<name>`).
    fn name(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Hollow => "hollow",
            Self::Awesome => "awesome",
            Self::Icon => "icon",
        }
    }

    /// The classes of the glyph group (`variantClass`, `usecaseActor.ts` 28-33).
    fn glyph_class(self) -> &'static str {
        match self {
            Self::Normal => "usecase-actor-shape usecase-actor-normal",
            Self::Hollow => "usecase-actor-hollow",
            Self::Awesome => "usecase-actor-awesome",
            Self::Icon => "usecase-actor-icon",
        }
    }
}

/// The figure's box (`ACTOR_FIGURE_WIDTH`, `ACTOR_FIGURE_HEIGHT`, `usecaseActor.ts` 21-22).
const FIGURE_WIDTH: f64 = 56.0;
const FIGURE_HEIGHT: f64 = 72.0;
/// The gap under the figure and between the stereotype and the label (`usecaseActor.ts` 24-25).
const LABEL_GAP: f64 = 8.0;
const STEREOTYPE_GAP: f64 = 2.0;

/// The stick figure's path (`usecaseActor.ts` 255-265).
const STICK_FIGURE: &str = "M 0 -12 C 6.627 -12 12 -17.373 12 -24 C 12 -30.627 6.627 -36 0 -36 C -6.627 -36 -12 -30.627 -12 -24 C -12 -17.373 -6.627 -12 0 -12 Z M 0 -12 V 8 M -17 -5 H 17 M 0 8 L -15 28 M 0 8 L 15 28";

/// The hollow figure's body (`usecaseActorHollow.ts` 10-23).
const HOLLOW_BODY: &str =
    "M -22 -10 H 22 V 0 H 6 L 22 17 L 13 28 L 0 13 L -13 28 L -22 17 L -6 0 H -22 Z";

/// The awesome figure's silhouette (`usecaseActorAwesome.ts` 6-18).
const AWESOME_SILHOUETTE: &str = "M 0 -34 C 7.18 -34 13 -28.18 13 -21 C 13 -13.82 7.18 -8 0 -8 C -7.18 -8 -13 -13.82 -13 -21 C -13 -28.18 -7.18 -34 0 -34 Z M -24 25 C -24 7 -14 -3 0 -3 C 14 -3 24 7 24 25 C 24 28 21 30 18 30 H -18 C -21 30 -24 28 -24 25 Z";

/// The business marker's path for a circle of this centre and radius (`usecaseActor.ts` 35-51): a chord
/// across the top of the figure, at 60 per cent of the radius, at the angle of 60 degrees.
fn business_marker_for_circle(centre_y: f64, radius: f64) -> String {
    const OFFSET_RATIO: f64 = 0.6;
    let angle = std::f64::consts::PI / 3.0;
    let centre_offset = radius * OFFSET_RATIO;
    let half_chord = radius * (1.0 - OFFSET_RATIO * OFFSET_RATIO).sqrt();
    let direction_x = ferrigura_jsmath::cos(angle);
    let direction_y = -ferrigura_jsmath::sin(angle);
    let centre_x = centre_offset * -direction_y;
    let marker_centre_y = centre_y + centre_offset * direction_x;
    let delta_x = half_chord * direction_x;
    let delta_y = half_chord * direction_y;
    format!(
        "M {} {} L {} {}",
        number(centre_x - delta_x),
        number(marker_centre_y - delta_y),
        number(centre_x + delta_x),
        number(marker_centre_y + delta_y),
    )
}

/// The business marker of a variant (`businessMarkerPath`, `usecaseActor.ts` 53-58).
fn business_marker(variant: Variant) -> String {
    match variant {
        Variant::Normal => business_marker_for_circle(-24.0, 12.0),
        Variant::Hollow => business_marker_for_circle(-23.0, 9.0),
        Variant::Awesome => business_marker_for_circle(-21.0, 13.0),
        Variant::Icon => "M 12 -8 L 26 -26".to_owned(),
    }
}

/// The declarations of a label's style, as `labelHelper` hands them to the label (`node_label`).
pub(super) fn declarations(style: &str) -> Vec<String> {
    style
        .split(';')
        .filter(|declaration| !declaration.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The font size of the diagram's text when the theme gives none (the root's `font-size`).
pub(super) fn root_font_size(input: &ShapeInput<'_>) -> f64 {
    util::theme_number(&input.config.theme.font_size).unwrap_or(input.config.globals.font_size)
}

/// The width at which the label of a node wraps: the node's wrapping width, which `getData` sets, else the
/// flowchart's (`insertLabel`'s `options.width || flowchart.wrappingWidth`).
pub(super) fn node_width(input: &ShapeInput<'_>) -> f64 {
    input
        .node
        .wrapping_width
        .unwrap_or(input.config.section.flowchart.wrapping_width)
}

/// The label as the stylesheet draws it: the measured label laid out again at the font size its class sets
/// (`drawn_font_size`), which the reference applies after it has measured the label. The measured label
/// gives the box that positions the drawn one, so it is kept for that.
pub(super) fn drawn_label(
    input: &ShapeInput<'_>,
    measured: &LabelLayout,
) -> Result<LabelLayout, RenderError> {
    let Some(size) = input
        .node
        .use_case
        .as_ref()
        .and_then(|fields| fields.drawn_font_size)
    else {
        return Ok(measured.clone());
    };
    let mut node = input.node.clone();
    node.font_size = Some(size);
    Ok(
        super::node_label(&node, input.config, None, Anchor::Origin)?
            .unwrap_or_else(|| measured.clone()),
    )
}

/// `insertLabel` (`shapes/util.ts` 155-219) for a label that the shape adds to its own elements: the
/// text as `createText` lays it out at the node's wrapping width, in the node's label style, with no
/// minimum width. The caller places it with `centerLabel` or its own offset.
pub(super) fn insert_label(
    input: &ShapeInput<'_>,
    text: &str,
    label_style: &str,
    width: f64,
) -> Result<LabelLayout, RenderError> {
    let text = sanitize_text(&decode_entities(text));
    let styles = declarations(label_style);
    label::layout(&LabelSpec {
        text: &text,
        label_type: LabelType::Text,
        width,
        is_node: false,
        anchor: Anchor::Origin,
        wrap_in_style: true,
        add_background: false,
        styles: &styles,
        font_size: root_font_size(input),
        title: false,
    })
}

/// `positionLabel` (`usecaseActor.ts` 74-81): the translation that centres a label on `centre_y`, with the
/// origin of its box taken out: `translate(-w/2 - x, centre - h/2 - y)`.
pub(super) fn position_label(label: &LabelLayout, centre_y: f64) -> Point {
    Point::new(
        -label.bbox.width / 2.0 - label.bbox.x,
        centre_y - label.bbox.height / 2.0 - label.bbox.y,
    )
}

/// A label group with its rectangle first, as `labelHelper` and `insertLabel` leave it.
pub(super) fn label_group(
    class: &str,
    style: &str,
    offset: Point,
    label: &LabelLayout,
    text_style: &str,
) -> String {
    format!(
        "<g class=\"{class}\"{style_attribute} transform=\"translate({}, {})\"><rect></rect>{}</g>",
        number(offset.x),
        number(offset.y),
        label.svg(&util::node_text_style(text_style)),
        style_attribute = attribute("style", style),
    )
}

/// The glyph of the variant, drawn inside its group at the node's origin (`drawGlyph`).
fn glyph(variant: Variant) -> String {
    let hollow = attribute("fill", "none");
    match variant {
        Variant::Normal => format!("<path class=\"usecase-actor-stick\" d=\"{STICK_FIGURE}\"/>"),
        Variant::Hollow => format!(
            "<circle class=\"usecase-actor-hollow-head\" cx=\"0\" cy=\"-23\" r=\"9\"{hollow}/><path class=\"usecase-actor-hollow-body\" d=\"{HOLLOW_BODY}\"{hollow}/>"
        ),
        Variant::Awesome => {
            format!("<path class=\"usecase-actor-awesome-silhouette\" d=\"{AWESOME_SILHOUETTE}\"/>")
        }
        Variant::Icon => icon_glyph(),
    }
}

/// The icon actor's frame and symbol (`usecaseActorIcon.ts` 16-63). ferrigura registers no icon pack, so
/// the symbol is always the reference's fallback, which carries the fallback class; it is 42 by 42 and
/// centred on the frame's centre line.
fn icon_glyph() -> String {
    const FRAME: f64 = 52.0;
    const SYMBOL: f64 = 42.0;
    const CENTRE_Y: f64 = -2.0;
    let frame_x = -FRAME / 2.0;
    let frame_y = CENTRE_Y - FRAME / 2.0;
    format!(
        "<rect class=\"usecase-actor-icon-frame\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\"/><g class=\"usecase-actor-icon-symbol usecase-actor-icon-fallback\" aria-hidden=\"true\" transform=\"translate({}, {})\"><g>{}</g></g>",
        number(frame_x),
        number(frame_y),
        number(FRAME),
        number(FRAME),
        number(-SYMBOL / 2.0),
        number(CENTRE_Y - SYMBOL / 2.0),
        fallback_icon_svg(SYMBOL),
    )
}

/// `renderUsecaseActor` for one variant (`usecaseActor.ts` 154-265): the label, the stereotype above it,
/// the glyph and the business marker, and the outline that sizes the node. The stereotype is returned
/// with the shape, so that the fonts of the diagram cover its text.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read.
pub(super) fn actor(
    input: &ShapeInput<'_>,
    variant: Variant,
) -> Result<(Part, Vec<LabelLayout>), RenderError> {
    let node = input.node;
    let styles = styles2string(node);
    // The label as `labelHelper` lays it out: an actor has no minimum width (`usecaseDb.ts` 512-516).
    let label = label_helper(input, false);
    let stereotype = node
        .use_case
        .as_ref()
        .and_then(|fields| fields.stereotype.clone());
    let stereotype_label = match stereotype {
        Some(text) => Some(insert_label(
            input,
            &format!("«{text}»"),
            &styles.label,
            node_width(input),
        )?),
        None => None,
    };
    let stereotype_height = stereotype_label
        .as_ref()
        .map_or(0.0, |label| label.bbox.height);
    let stereotype_width = stereotype_label
        .as_ref()
        .map_or(0.0, |label| label.bbox.width);
    let content_height = FIGURE_HEIGHT
        + LABEL_GAP
        + label.bbox.height
        + stereotype_label
            .as_ref()
            .map_or(0.0, |_| stereotype_height + STEREOTYPE_GAP);
    let padding = node.padding;
    let total_width = FIGURE_WIDTH.max(label.bbox.width).max(stereotype_width) + padding * 2.0;
    let total_height = content_height + padding * 2.0;
    let content_top = -content_height / 2.0;

    // The stereotype and the label, from the top of the content down (`usecaseActor.ts` 229-237).
    let mut next_centre = content_top + FIGURE_HEIGHT + LABEL_GAP;
    let mut after = String::new();
    let mut texts = Vec::new();
    if let Some(stereotype) = &stereotype_label {
        next_centre += stereotype_height / 2.0;
        after.push_str(&label_group(
            "label usecase-stereotype",
            &styles.label,
            position_label(stereotype, next_centre),
            stereotype,
            &styles.label,
        ));
        next_centre += stereotype_height / 2.0 + STEREOTYPE_GAP;
        texts.push(stereotype.clone());
    }
    let label_offset = position_label(&label, next_centre + label.bbox.height / 2.0);

    // The glyph group (`usecaseActor.ts` 210-229): its style only when the node has one.
    let glyph_style = if styles.node.is_empty() {
        String::new()
    } else {
        attribute("style", &styles.node)
    };
    let business = node.use_case.as_ref().is_some_and(|fields| fields.business);
    let marker = if business {
        format!(
            "<path class=\"usecase-business-marker usecase-actor-business-marker\" d=\"{}\" fill=\"none\" style=\"stroke: inherit !important; stroke-width: inherit !important\"/>",
            business_marker(variant),
        )
    } else {
        String::new()
    };
    after.push_str(&format!(
        "<g class=\"usecase-actor-glyph {}\"{glyph_style} transform=\"translate(0, {})\">{}{marker}</g>",
        variant.glyph_class(),
        number(content_top + FIGURE_HEIGHT / 2.0),
        glyph(variant),
    ));

    let outline = format!(
        "<rect class=\"usecase-actor-outline\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" opacity=\"0\" aria-hidden=\"true\"/>",
        number(-total_width / 2.0),
        number(-total_height / 2.0),
        number(total_width),
        number(total_height),
    );
    let drawn = drawn_label(input, &label)?;
    let part = Part {
        svg: outline,
        after_label: after,
        bbox: BBox::new(
            -total_width / 2.0,
            -total_height / 2.0,
            total_width,
            total_height,
        ),
        intersect: Intersect::Rect,
        label_offset: Some(label_offset),
        label: Some(drawn),
    };
    Ok((part, texts))
}

/// The `class` of the group of an actor's node: `node`, the node's classes, then the variant's
/// (`getNodeClasses(node, 'usecase-actor-variant usecase-actor-<variant>')`).
pub(super) fn group_class(css_classes: &str, variant: Variant) -> String {
    format!(
        "node {} usecase-actor-variant usecase-actor-{}",
        css_classes.trim(),
        variant.name()
    )
}

/// The variant of an actor's shape (`usecaseDb.ts` 213-224). Any other kind is the normal figure.
pub(super) fn variant_of_kind(kind: Kind) -> Variant {
    match kind {
        Kind::UsecaseActorHollow => Variant::Hollow,
        Kind::UsecaseActorAwesome => Variant::Awesome,
        Kind::UsecaseActorIcon => Variant::Icon,
        _ => Variant::Normal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_business_marker_of_the_normal_figure_is_a_chord_across_its_head() {
        // `usecaseActor.ts` 54: a circle of radius 12 at centre -24. The chord is level at its ends and
        // starts left of the figure's axis, as the 60-degree direction gives it.
        let marker = business_marker(Variant::Normal);
        let coordinates: Vec<f64> = marker
            .split(' ')
            .filter_map(|token| token.parse::<f64>().ok())
            .collect();
        assert_eq!(coordinates.len(), 4, "{marker}");
        assert!((coordinates[0] - 1.4353828).abs() < 1e-6, "{marker}");
        assert!((coordinates[2] - 11.0353828).abs() < 1e-6, "{marker}");
        assert!((coordinates[1] - coordinates[3]).abs() < 1e-9, "{marker}");
        assert!((coordinates[1] + 12.0861562).abs() < 1e-6, "{marker}");
    }
}
