//! The actor glyphs: a port of `rendering-elements/shapes/usecaseActor.ts` and its variants
//! `usecaseActorHollow.ts`, `usecaseActorAwesome.ts` and `usecaseActorIcon.ts`, which share one handler
//! (`renderUsecaseActor`). Mermaid draws the actors of a use case diagram with them; a flowchart node can
//! name them too. The figure is drawn in the plain elements that the neo and classic looks draw; the
//! hand-drawn look is out of scope.
//!
//! The label sits under the figure: `label`, then the figure (`after_label`), as the handler appends them.
//! The outline is an invisible rectangle around the whole, which sizes the node. The stereotype and the
//! business marker of the reference come from the fields of a use case's actor, which a flowchart node
//! does not carry.

use super::icon::fallback_icon_svg;
use super::registry::Kind;
use super::util::{self, Part, attribute, label_helper, styles2string};
use super::{Intersect, ShapeInput};
use crate::RenderError;
use crate::svg::number;
use crate::unified::geom::{BBox, Point};
use crate::unified::label::LabelLayout;

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
/// The gap under the figure (`usecaseActor.ts` 24).
const LABEL_GAP: f64 = 8.0;

/// The stick figure's path (`usecaseActor.ts` 255-265).
const STICK_FIGURE: &str = "M 0 -12 C 6.627 -12 12 -17.373 12 -24 C 12 -30.627 6.627 -36 0 -36 C -6.627 -36 -12 -30.627 -12 -24 C -12 -17.373 -6.627 -12 0 -12 Z M 0 -12 V 8 M -17 -5 H 17 M 0 8 L -15 28 M 0 8 L 15 28";

/// The hollow figure's body (`usecaseActorHollow.ts` 10-23).
const HOLLOW_BODY: &str =
    "M -22 -10 H 22 V 0 H 6 L 22 17 L 13 28 L 0 13 L -13 28 L -22 17 L -6 0 H -22 Z";

/// The awesome figure's silhouette (`usecaseActorAwesome.ts` 6-18).
const AWESOME_SILHOUETTE: &str = "M 0 -34 C 7.18 -34 13 -28.18 13 -21 C 13 -13.82 7.18 -8 0 -8 C -7.18 -8 -13 -13.82 -13 -21 C -13 -28.18 -7.18 -34 0 -34 Z M -24 25 C -24 7 -14 -3 0 -3 C 14 -3 24 7 24 25 C 24 28 21 30 18 30 H -18 C -21 30 -24 28 -24 25 Z";

/// `positionLabel` (`usecaseActor.ts` 74-81): the translation that centers a label on `center_y`, with the
/// origin of its box taken out: `translate(-w/2 - x, center - h/2 - y)`.
pub(super) fn position_label(label: &LabelLayout, center_y: f64) -> Point {
    Point::new(
        -label.bbox.width / 2.0 - label.bbox.x,
        center_y - label.bbox.height / 2.0 - label.bbox.y,
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
/// centered on the frame's center line.
fn icon_glyph() -> String {
    const FRAME: f64 = 52.0;
    const SYMBOL: f64 = 42.0;
    const CENTER_Y: f64 = -2.0;
    let frame_x = -FRAME / 2.0;
    let frame_y = CENTER_Y - FRAME / 2.0;
    format!(
        "<rect class=\"usecase-actor-icon-frame\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\"/><g class=\"usecase-actor-icon-symbol usecase-actor-icon-fallback\" aria-hidden=\"true\" transform=\"translate({}, {})\"><g>{}</g></g>",
        number(frame_x),
        number(frame_y),
        number(FRAME),
        number(FRAME),
        number(-SYMBOL / 2.0),
        number(CENTER_Y - SYMBOL / 2.0),
        fallback_icon_svg(SYMBOL),
    )
}

/// `renderUsecaseActor` for one variant (`usecaseActor.ts` 154-265): the label, the glyph, and the
/// outline that sizes the node. A flowchart node has no stereotype, so no further text is returned with
/// the shape.
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
    let content_height = FIGURE_HEIGHT + LABEL_GAP + label.bbox.height;
    let padding = node.padding;
    let total_width = FIGURE_WIDTH.max(label.bbox.width) + padding * 2.0;
    let total_height = content_height + padding * 2.0;
    let content_top = -content_height / 2.0;

    // The label, under the figure (`usecaseActor.ts` 229-237).
    let next_center = content_top + FIGURE_HEIGHT + LABEL_GAP;
    let label_offset = position_label(&label, next_center + label.bbox.height / 2.0);

    // The glyph group (`usecaseActor.ts` 210-229): its style only when the node has one.
    let glyph_style = if styles.node.is_empty() {
        String::new()
    } else {
        attribute("style", &styles.node)
    };
    let after = format!(
        "<g class=\"usecase-actor-glyph {}\"{glyph_style} transform=\"translate(0, {})\">{}</g>",
        variant.glyph_class(),
        number(content_top + FIGURE_HEIGHT / 2.0),
        glyph(variant),
    );

    let outline = format!(
        "<rect class=\"usecase-actor-outline\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" opacity=\"0\" aria-hidden=\"true\"/>",
        number(-total_width / 2.0),
        number(-total_height / 2.0),
        number(total_width),
        number(total_height),
    );
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
        label: Some(label),
    };
    Ok((part, Vec::new()))
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
