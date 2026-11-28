//! The design mode of the sequence diagram (`docs/decisions/0023-design-mode.md`,
//! `docs/diagrams/sequence.md#design-mode`): the measures it gives the renderer's own layout pass,
//! and the parts it colors by their role. The colors are the mode's theme variables
//! (`crate::design::theme_variables`), the rules are in the style sheet (`style.rs`).

use super::conf::Conf;

/// The weight of a participant's name: semibold, as the name of a class box stands out from its
/// members.
pub const NAME_WEIGHT: f64 = 600.0;

/// The room between a participant's name and the sides of its box: a flowchart node's.
pub const PARTICIPANT_SIDE_ROOM: f64 = 16.0;

/// The room between a participant's name and the top and the bottom of its box.
pub const PARTICIPANT_ROW_ROOM: f64 = 12.0;

/// The least width of a participant's column, where the reference has 150 (`width`).
pub const PARTICIPANT_MIN_WIDTH: f64 = 96.0;

/// The room a queue's column adds for the end of its cylinder, which stands at its right.
pub const QUEUE_CAP: f64 = 24.0;

/// The room a message's label keeps from the lifelines it runs between, as the configuration's
/// `actorMargin`, where the reference has 50: the label's width with the wrap padding (10) at both
/// sides and this much more makes the distance of the lifelines, so 30 is left at either side. It
/// is also the least gap between two participant boxes.
pub const ACTOR_MARGIN: f64 = 40.0;

/// The least distance between two neighboring lifelines. The columns share one pitch, this or the
/// widest participant's box and [`ACTOR_MARGIN`], unless a label between them needs more.
pub const MIN_PITCH: f64 = 150.0;

/// The room at the left and the right of a frame's keyword in its tag.
pub const TAG_SIDE_ROOM: f64 = 10.0;

/// The height of a frame's tag.
pub const TAG_HEIGHT: f64 = 26.0;

/// The room between a frame's tag and its title, and the inset of a section's title.
pub const TITLE_GAP: f64 = 10.0;

/// The room a label's gap in the lifelines leaves around its text.
pub const GAP_PADDING: f64 = 4.0;

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

/// The weight of the configuration's defaults, which the mode replaces for the names.
const DEFAULT_WEIGHT: f64 = 400.0;

/// The `actorMargin` of the configuration's defaults, which the mode replaces.
const DEFAULT_ACTOR_MARGIN: f64 = 50.0;

/// Applies the mode's measures to the configuration of a render. The mode draws the neo geometry
/// whatever the look, with the row of participants as high as its tallest participant needs. A
/// weight of the participants' names (`actorFontWeight`) or an `actorMargin` the configuration
/// gives stays: a value other than the default is the user's.
pub fn measures(conf: &mut Conf) {
    if conf.actor_font.weight == DEFAULT_WEIGHT {
        conf.actor_font.weight = NAME_WEIGHT;
    }
    if conf.actor_margin == DEFAULT_ACTOR_MARGIN {
        conf.actor_margin = ACTOR_MARGIN;
    }
    conf.neo = true;
    conf.height = 0.0;
}

/// The height of a participant's box for the height of its name.
#[must_use]
pub fn participant_height(text_height: f64) -> f64 {
    text_height + 2.0 * PARTICIPANT_ROW_ROOM
}

/// The width of a participant's column for the width of its name: the box's, at least
/// [`PARTICIPANT_MIN_WIDTH`].
#[must_use]
pub fn participant_width(text_width: f64) -> f64 {
    (text_width + 2.0 * PARTICIPANT_SIDE_ROOM).max(PARTICIPANT_MIN_WIDTH)
}

/// A box in which a label interrupts the lifelines: its left, top, width and height.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gap {
    /// The left edge.
    pub x: f64,
    /// The top edge.
    pub y: f64,
    /// The width.
    pub width: f64,
    /// The height.
    pub height: f64,
}

impl Gap {
    /// The gap of a text whose box is `width` by `height` at (`x`, `y`), with [`GAP_PADDING`]
    /// around it.
    #[must_use]
    pub fn around(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x: x - GAP_PADDING,
            y: y - GAP_PADDING,
            width: width + 2.0 * GAP_PADDING,
            height: height + 2.0 * GAP_PADDING,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_participant_box_fits_its_name_with_a_least_width() {
        assert_eq!(participant_width(20.0), PARTICIPANT_MIN_WIDTH);
        assert_eq!(participant_width(100.0), 132.0);
        assert_eq!(participant_height(19.0), 43.0);
    }

    #[test]
    fn the_frame_roles_are_the_choices() {
        assert_eq!(frame_role("alt"), Some("decision"));
        assert_eq!(frame_role("opt"), Some("decision"));
        assert_eq!(frame_role("loop"), None);
        assert_eq!(participant_role("database"), Some("data"));
        assert_eq!(participant_role("queue"), None);
    }
}
