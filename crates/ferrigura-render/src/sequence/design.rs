//! The design mode of the sequence diagram (`docs/decisions/0023-design-mode.md`,
//! `docs/diagrams/sequence.md#design-mode`): the measures it gives the renderer's own layout pass,
//! and the parts it colors by their role. The colors are the mode's theme variables
//! (`crate::design::theme_variables`), the rules are in the style sheet (`style.rs`).

use ferrigura_model::config::SequenceDiagramConfig;

use super::conf::Conf;

/// The weight of a participant's name: semibold, as the name of a class box stands out from its
/// members.
pub const NAME_WEIGHT: f64 = 600.0;

/// The corner radius of a participant's box: a flowchart node's.
pub const PARTICIPANT_RADIUS: f64 = 8.0;

/// The corner radius of a frame of `loop`, `alt`, `opt`, `par`, `critical` and `break`.
pub const FRAME_RADIUS: f64 = 8.0;

/// The corner radius of a box around participants and of a `rect` background.
pub const BOX_RADIUS: f64 = 12.0;

/// The corner radius of an activation bar.
pub const ACTIVATION_RADIUS: f64 = 2.0;

/// The role of a participant by its type, which the style sheet colors: a person (`actor`), and a
/// store of data (`database`, as the flowchart's cylinder). Other types stay neutral.
#[must_use]
pub fn participant_role(actor_type: &str) -> Option<&'static str> {
    match actor_type {
        "actor" => Some("actor"),
        "database" => Some("data"),
        _ => None,
    }
}

/// The role of a frame by its keyword: `alt` and `opt` choose, as the flowchart's decision does.
/// The other frames stay neutral.
#[must_use]
pub fn frame_role(keyword: &str) -> Option<&'static str> {
    match keyword {
        "alt" | "opt" => Some("decision"),
        _ => None,
    }
}

/// Applies the mode's measures to the configuration of a render. A weight the configuration gives
/// the participants' names (`actorFontWeight`) stays.
pub fn measures(conf: &mut Conf, section: &SequenceDiagramConfig) {
    if section.actor_font_weight.is_none() {
        conf.actor_font.weight = NAME_WEIGHT;
    }
}
