//! The design mode: ferrigura's own look for a diagram, beside the reference's
//! (`docs/decisions/0023-design-mode.md`).
//!
//! The mode has its own look and its own measures. Its colors are theme variables that the facade
//! lays under the user's own, so that every element keeps reading its colors from the theme tokens;
//! the roles, the stroke widths, the radii and the shadow are written by the style sheet and the
//! shapes of a diagram type that supports the mode. Its measures (the room a node gives the edges
//! of its busiest side, the place of the edges on a side, the spacing, the padding of a group)
//! change what the layout engine is given, never the engine.

use ferrigura_model::value::Value;

/// Whether the one set of tokens a render draws with is light or dark. A render with both schemes
/// draws with the light set and carries the dark one beside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// A light ground.
    Light,
    /// A dark ground.
    Dark,
}

/// The stroke width of a node outline.
pub const NODE_STROKE_WIDTH: &str = "1.25px";

/// The stroke width of an edge of normal thickness.
pub const EDGE_STROKE_WIDTH: &str = "1.5px";

/// The stroke width of a thick edge: twice the normal one, and still slimmer than the arrowhead.
pub const THICK_EDGE_STROKE_WIDTH: &str = "3px";

/// How many times the wrapping width a single word may measure and still stay whole in a label.
/// A word is never broken inside in the mode; a word wider than this bound (a pasted URL) is
/// broken as the default breaks it, so that one label cannot blow a diagram up.
pub const WHOLE_WORD_LIMIT: f64 = 4.0;

/// The corner radius of a rectangular node.
pub const NODE_RADIUS: f64 = 8.0;

/// The corner radius of a group frame.
pub const GROUP_RADIUS: f64 = 12.0;

/// The corner radius of a concurrency region of a state diagram, a dashed frame inside its
/// composite state's.
pub const REGION_RADIUS: f64 = 8.0;

/// The corner radius of a rounded edge: twice the reference's, so that a turn reads as a curve.
pub const EDGE_RADIUS: f64 = 10.0;

/// The distance between two edges that meet a node on the same side: the layout is given it as the
/// spacing of ports, which sit around the middle of the side. A node grows along that side until
/// its busiest side has this much room for each of its edges.
pub const PORT_PITCH: f64 = 20.0;

/// How far a self-loop stands out from the side of its node. Its ends meet the side
/// [`PORT_PITCH`] apart around the middle, or half the side apart where the side is shorter than
/// twice that, so the loop is a rounded rectangle this deep.
pub const LOOP_REACH: f64 = 32.0;

/// The room between a self-loop and the box of its label beside it.
pub const LOOP_LABEL_GAP: f64 = 4.0;

/// The room a node keeps free at each end of its busiest side, so that no edge meets the node
/// where its corner turns.
pub const PORT_MARGIN: f64 = 24.0;

/// The base value of the layout's spacings at the root, where the reference has 40.
pub const ROOT_SPACING: f64 = 48.0;

/// The padding of a group around its members at the left, the right and the bottom, where the
/// reference has 24.
pub const GROUP_SIDE_PADDING: f64 = 32.0;

/// The padding of a group as the layout's option: [`GROUP_SIDE_PADDING`], and a row for the title
/// at the top.
pub const GROUP_PADDING: &str = "[top=44,left=32,bottom=32,right=32]";

/// The padding of a group that draws no title, as the layout's option: [`GROUP_SIDE_PADDING`] on
/// every side. A concurrency region of a state diagram has it.
pub const REGION_PADDING: &str = "[top=32,left=32,bottom=32,right=32]";

/// The distance of a group's title from the top of its frame.
pub const GROUP_TITLE_TOP: f64 = 12.0;

/// The room between the text of a state and its outline at each side, the flowchart's node's.
pub const STATE_SIDE_ROOM: f64 = 16.0;

/// The padding at each side of a state whose text is `text` wide in a label `label` wide: the
/// text widened to the state's least width. A short text keeps the state's own padding `least`
/// around that label, so the state keeps its least width; a text that fills the label gets
/// [`STATE_SIDE_ROOM`].
#[must_use]
pub fn state_side_padding(text: f64, label: f64, least: f64) -> f64 {
    let least = least.min(STATE_SIDE_ROOM);
    ((text + 2.0 * STATE_SIDE_ROOM - label) / 2.0).clamp(least, STATE_SIDE_ROOM)
}

/// The room of a state with descriptions above its title, between the title and its divider,
/// between the divider and its rows, and below its last row. Its sides have
/// [`STATE_SIDE_ROOM`].
pub const DESCRIBED_STATE_ROOM: f64 = 10.0;

/// The room the box of an edge label adds to the left and the right of its text's box. The edge
/// is masked in that box, so the label sits in a gap of its edge.
pub const CHIP_PADDING: f64 = 5.0;

/// The room the layout keeps free at the left and the right of an edge label's box, so that the
/// box does not touch the frame of a group or a node beside it.
pub const CHIP_MARGIN: f64 = 6.0;

/// The corner radius of an edge label's box.
pub const CHIP_RADIUS: f64 = 6.0;

/// The theme variables of the mode for a tone, as an object of `themeVariables`. The user's own
/// variables are merged on top, key by key.
#[must_use]
pub fn theme_variables(tone: Tone) -> Value {
    let variables: &[(&str, &str)] = match tone {
        Tone::Light => &[
            ("background", "#FFFFFF"),
            ("mainBkg", "#FFFFFF"),
            ("nodeBorder", "#7C8698"),
            ("lineColor", "#5B6577"),
            ("arrowheadColor", "#5B6577"),
            ("textColor", "#1A1F2B"),
            ("nodeTextColor", "#1A1F2B"),
            ("titleColor", "#4A5364"),
            ("clusterBkg", "#F2F4F8"),
            ("clusterBorder", "#8891A2"),
            ("edgeLabelBackground", "#FFFFFF"),
            // The state diagram's own tokens, which its rules read.
            ("stateBorder", "#7C8698"),
            ("stateLabelColor", "#1A1F2B"),
            ("transitionLabelColor", "#4A5364"),
            ("labelBackgroundColor", "#FFFFFF"),
            ("compositeBackground", "#F2F4F8"),
            ("compositeTitleBackground", "#F2F4F8"),
            ("altBackground", "#F2F4F8"),
            ("noteBkgColor", "#F6F7F9"),
            ("noteBorderColor", "#7C8698"),
            ("noteTextColor", "#3D4555"),
        ],
        Tone::Dark => &[
            ("background", "#12141A"),
            ("mainBkg", "#1D212A"),
            ("nodeBorder", "#667085"),
            ("lineColor", "#8E98AA"),
            ("arrowheadColor", "#8E98AA"),
            ("textColor", "#E7E9EE"),
            ("nodeTextColor", "#E7E9EE"),
            ("titleColor", "#A3ACBC"),
            ("clusterBkg", "#171A22"),
            ("clusterBorder", "#5E677A"),
            ("edgeLabelBackground", "#12141A"),
            // The state diagram's own tokens, which its rules read.
            ("stateBorder", "#667085"),
            ("stateLabelColor", "#E7E9EE"),
            ("transitionLabelColor", "#A3ACBC"),
            ("labelBackgroundColor", "#12141A"),
            ("compositeBackground", "#171A22"),
            ("compositeTitleBackground", "#171A22"),
            ("altBackground", "#171A22"),
            ("noteBkgColor", "#191C24"),
            ("noteBorderColor", "#667085"),
            ("noteTextColor", "#B4BCCB"),
        ],
    };
    let mut object = Value::empty_object();
    if let Value::Object(map) = &mut object {
        for (name, colour) in variables {
            map.insert((*name).into(), Value::from(*colour));
        }
    }
    object
}

/// The length a node needs along the side its edges meet, for the number of edges on its busiest
/// side: [`PORT_PITCH`] between neighbors and [`PORT_MARGIN`] at both ends. A node with one edge
/// or none needs nothing.
#[must_use]
pub fn side_length(edges: usize) -> f64 {
    if edges < 2 {
        0.0
    } else {
        (edges - 1) as f64 * PORT_PITCH + 2.0 * PORT_MARGIN
    }
}

/// What a node stands for, read from its shape. A role gives the node its color, so that color
/// carries meaning and does not follow the order of the nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Where a flow starts or ends: the stadium and the circles.
    Terminal,
    /// A decision: the diamond.
    Decision,
    /// A data store: the cylinder.
    Data,
    /// A point where transitions split or meet: the fork and the join bar of a state diagram. It
    /// takes the color of the lines it joins.
    Junction,
    /// A type that is not instantiated: an interface or an abstract class of a class diagram.
    Interface,
    /// A fixed set of values: an enumeration of a class diagram.
    Enumeration,
}

/// Which shapes carry a role: the flowchart's, or the state diagram's. A shape that both diagram
/// types can draw keeps the role of the type it is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Roles {
    /// The flowchart's: the stadium and the circles, the diamond, the cylinder.
    #[default]
    Flowchart,
    /// The state diagram's: the start and the end state, the choice, the fork and the join.
    State,
    /// The class diagram's: an interface or an abstract class, and an enumeration, read from the
    /// class's annotation.
    Class,
}

impl Role {
    /// The value of the node's `data-role` attribute, which the style sheet matches.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Terminal => "terminal",
            Self::Decision => "decision",
            Self::Data => "data",
            Self::Junction => "junction",
            Self::Interface => "interface",
            Self::Enumeration => "enumeration",
        }
    }

    /// The custom properties of the role's fill, outline and text.
    #[must_use]
    pub const fn properties(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Terminal => ("terminalBkg", "terminalBorder", "terminalText"),
            Self::Decision => ("decisionBkg", "decisionBorder", "decisionText"),
            Self::Data => ("dataBkg", "dataBorder", "dataText"),
            Self::Junction => ("lineColor", "lineColor", "textColor"),
            Self::Interface => ("interfaceBkg", "interfaceBorder", "interfaceText"),
            Self::Enumeration => ("enumerationBkg", "enumerationBorder", "enumerationText"),
        }
    }
}

/// The roles of a flowchart, in the order of their rules.
pub const ROLES: [Role; 3] = [Role::Terminal, Role::Decision, Role::Data];

/// The roles of a state diagram, in the order of their rules.
pub const STATE_ROLES: [Role; 3] = [Role::Terminal, Role::Decision, Role::Junction];

/// The roles of a class diagram, in the order of their rules.
pub const CLASS_ROLES: [Role; 2] = [Role::Interface, Role::Enumeration];

/// The custom properties of the mode that no theme variable carries, for a tone: the role colors
/// and the flood of the shadow, as declarations.
#[must_use]
pub fn declarations(tone: Tone) -> &'static str {
    match tone {
        Tone::Light => {
            "--terminalBkg:#2F54D6;--terminalBorder:#2F54D6;--terminalText:#FFFFFF;\
             --decisionBkg:#FFF5DB;--decisionBorder:#B7791F;--decisionText:#684700;\
             --dataBkg:#E5F5F0;--dataBorder:#2E957B;--dataText:#0E5243;\
             --floodColor:rgba(20, 28, 45, 0.14);"
        }
        Tone::Dark => {
            "--terminalBkg:#4263EB;--terminalBorder:#4263EB;--terminalText:#FFFFFF;\
             --decisionBkg:#2A2310;--decisionBorder:#92752A;--decisionText:#F0CB6E;\
             --dataBkg:#10271F;--dataBorder:#37856F;--dataText:#9BDDCB;\
             --floodColor:rgba(0, 0, 0, 0.5);"
        }
    }
}

/// The custom properties of the class diagram's roles for a tone, as declarations: a quiet tint
/// and outline for an interface or an abstract class and for an enumeration, with the text of a
/// neutral class.
#[must_use]
pub fn class_declarations(tone: Tone) -> &'static str {
    match tone {
        Tone::Light => {
            "--interfaceBkg:#F3F6FE;--interfaceBorder:#6B83D3;--interfaceText:#1A1F2B;\
             --enumerationBkg:#F1F8F5;--enumerationBorder:#3F8F79;--enumerationText:#1A1F2B;"
        }
        Tone::Dark => {
            "--interfaceBkg:#1B2131;--interfaceBorder:#5D74C4;--interfaceText:#E7E9EE;\
             --enumerationBkg:#17231F;--enumerationBorder:#3F8572;--enumerationText:#E7E9EE;"
        }
    }
}

/// The flood of the shadow as presentation attributes, for a viewer that reads no style sheet.
#[must_use]
pub const fn flood_attributes(tone: Tone) -> (&'static str, &'static str) {
    match tone {
        Tone::Light => ("#141C2D", "0.14"),
        Tone::Dark => ("#000000", "0.5"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The relative luminance of a `#RRGGBB` color (WCAG 2).
    fn luminance(colour: &str) -> f64 {
        let channel = |at: usize| {
            let value = f64::from(u8::from_str_radix(&colour[at..at + 2], 16).unwrap()) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ferrigura_jsmath::pow((value + 0.055) / 1.055, 2.4)
            }
        };
        0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5)
    }

    /// The contrast ratio of two colors (WCAG 2).
    fn contrast(a: &str, b: &str) -> f64 {
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    /// The colors of a tone by name: its theme variables and its role properties.
    fn colours(tone: Tone) -> std::collections::BTreeMap<String, String> {
        let mut colours = std::collections::BTreeMap::new();
        if let Value::Object(map) = theme_variables(tone) {
            for (name, value) in map {
                colours.insert(name, value.as_str().unwrap().to_owned());
            }
        }
        for declaration in declarations(tone)
            .split(';')
            .chain(class_declarations(tone).split(';'))
        {
            if let Some((name, value)) = declaration.trim().split_once(':')
                && value.starts_with('#')
            {
                colours.insert(name.trim_start_matches("--").to_owned(), value.to_owned());
            }
        }
        colours
    }

    #[test]
    fn text_and_lines_keep_their_contrast_in_both_tones() {
        // WCAG 2.1: 4.5 for text (1.4.3), 3 for the lines a reader needs to tell shapes and
        // connections apart (1.4.11). A line is checked on the page and inside a group: a
        // subgraph, a composite state and a concurrency region. A class box's annotation and
        // visibility marks and a cardinality are muted text, on a class's ground and on the page.
        const TEXT: [(&str, &str); 19] = [
            ("textColor", "background"),
            ("nodeTextColor", "mainBkg"),
            ("titleColor", "clusterBkg"),
            ("titleColor", "edgeLabelBackground"),
            ("terminalText", "terminalBkg"),
            ("decisionText", "decisionBkg"),
            ("dataText", "dataBkg"),
            ("stateLabelColor", "mainBkg"),
            ("titleColor", "compositeBackground"),
            ("transitionLabelColor", "background"),
            ("transitionLabelColor", "compositeBackground"),
            ("transitionLabelColor", "altBackground"),
            ("noteTextColor", "noteBkgColor"),
            ("interfaceText", "interfaceBkg"),
            ("enumerationText", "enumerationBkg"),
            ("titleColor", "mainBkg"),
            ("titleColor", "background"),
            ("titleColor", "interfaceBkg"),
            ("titleColor", "enumerationBkg"),
        ];
        const LINES: [&str; 10] = [
            "lineColor",
            "arrowheadColor",
            "nodeBorder",
            "terminalBorder",
            "decisionBorder",
            "dataBorder",
            "stateBorder",
            "noteBorderColor",
            "interfaceBorder",
            "enumerationBorder",
        ];
        for tone in [Tone::Light, Tone::Dark] {
            let colours = colours(tone);
            let of = |name: &str| colours[name].as_str();
            for (text, ground) in TEXT {
                let ratio = contrast(of(text), of(ground));
                assert!(ratio >= 4.5, "{tone:?}: {text} on {ground} is {ratio:.2}");
            }
            for line in LINES {
                for ground in [
                    "background",
                    "clusterBkg",
                    "compositeBackground",
                    "altBackground",
                ] {
                    let ratio = contrast(of(line), of(ground));
                    assert!(ratio >= 3.0, "{tone:?}: {line} on {ground} is {ratio:.2}");
                }
            }
            let ratio = contrast(of("clusterBorder"), of("background"));
            assert!(ratio >= 3.0, "{tone:?}: clusterBorder is {ratio:.2}");
        }
    }

    #[test]
    fn a_side_has_room_for_each_of_its_edges() {
        assert_eq!(side_length(0), 0.0);
        assert_eq!(side_length(1), 0.0);
        assert_eq!(side_length(2), 68.0);
        assert_eq!(side_length(4), 108.0);
    }

    #[test]
    fn the_theme_variables_of_both_tones_name_the_same_variables() {
        let names = |tone| match theme_variables(tone) {
            Value::Object(map) => map.keys().cloned().collect::<Vec<_>>(),
            _ => Vec::new(),
        };
        assert!(!names(Tone::Light).is_empty());
        assert_eq!(names(Tone::Light), names(Tone::Dark));
    }

    #[test]
    fn every_role_property_is_declared_for_both_tones() {
        for tone in [Tone::Light, Tone::Dark] {
            let declared = declarations(tone);
            for role in ROLES {
                let (fill, stroke, text) = role.properties();
                for property in [fill, stroke, text] {
                    assert!(declared.contains(&format!("--{property}:")), "{property}");
                }
            }
            assert!(declared.contains("--floodColor:"));
            let declared = class_declarations(tone);
            for role in CLASS_ROLES {
                let (fill, stroke, text) = role.properties();
                for property in [fill, stroke, text] {
                    assert!(declared.contains(&format!("--{property}:")), "{property}");
                }
            }
        }
    }
}
