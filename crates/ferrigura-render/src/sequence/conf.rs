//! The configuration the sequence renderer reads: the `sequence` section, with the top-level keys
//! that `setConf` copies over it (`sequenceRenderer.ts:876-888`).
//!
//! `setConf(getConfig())` assigns every top-level key into `conf` first, so `look`, `theme` and
//! `useMaxWidth`-like keys of the top level win where they exist. The three font families, sizes
//! and weights are then replaced by the top-level `fontFamily` and `fontSize` when those are set.
//! The reference renders actor, note and message text at the top-level size (16 px by default),
//! whatever the section says (`docs-port/sequence/NOTES.md` 5.2), and this module does the same.

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::value::Value;

use crate::common::TextStyle;

/// The text style of one kind of text: its family as the CSS of the reference has it, its size in
/// pixels and its weight.
#[derive(Debug, Clone, PartialEq)]
pub struct Font {
    /// The family, quotes included, as the reference's CSS takes it.
    pub family: String,
    /// The size in pixels.
    pub size: f64,
    /// The CSS weight (400 is normal).
    pub weight: f64,
}

impl Font {
    /// The measurement style of this font, as `calculateTextDimensions` takes it.
    #[must_use]
    pub fn text_style(&self) -> TextStyle {
        TextStyle {
            font_size: self.size,
            font_weight: self.weight,
            font_family: self.family.clone(),
        }
    }

    /// The CSS `font-size` value, as `parseFontSize` gives it: `16px`.
    #[must_use]
    pub fn size_css(&self) -> String {
        format!("{}px", crate::svg::number(self.size))
    }
}

/// The resolved configuration of one render (`conf` in `sequenceRenderer.ts`).
#[derive(Debug, Clone, PartialEq)]
pub struct Conf {
    /// `conf.look === 'neo'` (the top-level look, as `setConf` copies it).
    pub neo: bool,
    /// `conf.width`: the minimum width of an actor and of a note.
    pub width: f64,
    /// `conf.height`: the minimum height of an actor, replaced by `calculateActorMargins`.
    pub height: f64,
    /// `conf.boxMargin`.
    pub box_margin: f64,
    /// `conf.boxTextMargin`.
    pub box_text_margin: f64,
    /// `conf.noteMargin`.
    pub note_margin: f64,
    /// `conf.wrapPadding`.
    pub wrap_padding: f64,
    /// `conf.actorMargin`.
    pub actor_margin: f64,
    /// `conf.activationWidth`.
    pub activation_width: f64,
    /// `conf.labelBoxWidth`.
    pub label_box_width: f64,
    /// `conf.labelBoxHeight`.
    pub label_box_height: f64,
    /// `conf.diagramMarginX`.
    pub diagram_margin_x: f64,
    /// `conf.diagramMarginY`.
    pub diagram_margin_y: f64,
    /// `conf.bottomMarginAdj`.
    pub bottom_margin_adj: f64,
    /// `conf.rightAngles`.
    pub right_angles: bool,
    /// `conf.mirrorActors`.
    pub mirror_actors: bool,
    /// `conf.hideUnusedParticipants`.
    pub hide_unused_participants: bool,
    /// `conf.forceMenus`.
    pub force_menus: bool,
    /// `conf.showSequenceNumbers`.
    pub show_sequence_numbers: bool,
    /// `conf.useMaxWidth`: the root's `width="100%"` and `max-width`.
    pub use_max_width: bool,
    /// `conf.messageAlign`, the anchor of a message's text.
    pub message_align: String,
    /// `conf.noteAlign`, the anchor of a note's text.
    pub note_align: String,
    /// The font of the actors' labels (`actorFont*`).
    pub actor_font: Font,
    /// The font of the notes (`noteFont*`).
    pub note_font: Font,
    /// The font of the messages and of the block labels (`messageFont*`).
    pub message_font: Font,
    /// `conf.theme`, the top-level theme name (the palettes are chosen by it).
    pub theme: String,
}

impl Conf {
    /// The configuration of a render, from the effective configuration of the diagram.
    #[must_use]
    pub fn new(config: &MermaidConfig) -> Self {
        let section = &config.sequence;
        // The top-level fontFamily and fontSize replace the section's three fonts (setConf).
        let family = config.font_family.clone();
        let size = config.font_size;
        let font = |weight: Option<&Value>| Font {
            family: family.clone(),
            size,
            weight: weight.and_then(number_of).unwrap_or(400.0),
        };
        Self {
            neo: config.look == "neo",
            width: section.width as f64,
            height: section.height as f64,
            box_margin: section.box_margin as f64,
            box_text_margin: section.box_text_margin as f64,
            note_margin: section.note_margin as f64,
            wrap_padding: section.wrap_padding,
            actor_margin: section.actor_margin as f64,
            activation_width: section.activation_width as f64,
            label_box_width: section.label_box_width,
            label_box_height: section.label_box_height,
            diagram_margin_x: section.diagram_margin_x as f64,
            diagram_margin_y: section.diagram_margin_y as f64,
            bottom_margin_adj: section.bottom_margin_adj as f64,
            right_angles: section.right_angles,
            mirror_actors: section.mirror_actors,
            hide_unused_participants: section.hide_unused_participants,
            force_menus: section.force_menus,
            show_sequence_numbers: section.show_sequence_numbers,
            use_max_width: section.use_max_width,
            message_align: section.message_align.clone(),
            note_align: section.note_align.clone(),
            actor_font: font(section.actor_font_weight.as_ref()),
            note_font: font(section.note_font_weight.as_ref()),
            message_font: font(section.message_font_weight.as_ref()),
            theme: config.theme.clone(),
        }
    }
}

/// The number a configuration value holds, when it holds one (a string of digits is read too, as
/// `parseFontSize` reads a size).
fn number_of(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => Some(*number),
        Value::String(text) => text.trim().parse::<f64>().ok(),
        _ => None,
    }
}
