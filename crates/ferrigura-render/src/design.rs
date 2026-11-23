//! The design mode: ferrigura's own look for a diagram, beside the reference's
//! (`docs/decisions/0023-design-mode.md`).
//!
//! The mode changes paint only. Its colors are theme variables that the facade lays under the user's
//! own, so that every element keeps reading its colors from the theme tokens; the roles, the stroke
//! widths, the radii and the shadow are written by the style sheet and the shapes of a diagram type
//! that supports the mode. Layout input, node sizes and routes are those of the default rendering.

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

/// The corner radius of a rectangular node.
pub const NODE_RADIUS: f64 = 8.0;

/// The corner radius of a group frame.
pub const GROUP_RADIUS: f64 = 12.0;

/// The theme variables of the mode for a tone, as an object of `themeVariables`. The user's own
/// variables are merged on top, key by key.
#[must_use]
pub fn theme_variables(tone: Tone) -> Value {
    let variables: &[(&str, &str)] = match tone {
        Tone::Light => &[
            ("background", "#FFFFFF"),
            ("mainBkg", "#FFFFFF"),
            ("nodeBorder", "#C4CBD6"),
            ("lineColor", "#8791A3"),
            ("arrowheadColor", "#8791A3"),
            ("textColor", "#1A1F2B"),
            ("nodeTextColor", "#1A1F2B"),
            ("titleColor", "#5F6878"),
            ("clusterBkg", "#F2F4F8"),
            ("clusterBorder", "#D6DCE5"),
            ("edgeLabelBackground", "#FFFFFF"),
        ],
        Tone::Dark => &[
            ("background", "#12141A"),
            ("mainBkg", "#1D212A"),
            ("nodeBorder", "#3A4150"),
            ("lineColor", "#6E788A"),
            ("arrowheadColor", "#6E788A"),
            ("textColor", "#E7E9EE"),
            ("nodeTextColor", "#E7E9EE"),
            ("titleColor", "#98A1B2"),
            ("clusterBkg", "#171A22"),
            ("clusterBorder", "#2C313D"),
            ("edgeLabelBackground", "#12141A"),
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
}

impl Role {
    /// The value of the node's `data-role` attribute, which the style sheet matches.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Terminal => "terminal",
            Self::Decision => "decision",
            Self::Data => "data",
        }
    }

    /// The custom properties of the role's fill, outline and text.
    #[must_use]
    pub const fn properties(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Terminal => ("terminalBkg", "terminalBorder", "terminalText"),
            Self::Decision => ("decisionBkg", "decisionBorder", "decisionText"),
            Self::Data => ("dataBkg", "dataBorder", "dataText"),
        }
    }
}

/// The roles, in the order of their rules.
pub const ROLES: [Role; 3] = [Role::Terminal, Role::Decision, Role::Data];

/// The custom properties of the mode that no theme variable carries, for a tone: the role colors
/// and the flood of the shadow, as declarations.
#[must_use]
pub fn declarations(tone: Tone) -> &'static str {
    match tone {
        Tone::Light => {
            "--terminalBkg:#2F54D6;--terminalBorder:#2F54D6;--terminalText:#FFFFFF;\
             --decisionBkg:#FFF5DB;--decisionBorder:#E2B54A;--decisionText:#684700;\
             --dataBkg:#E5F5F0;--dataBorder:#79C4B1;--dataText:#0E5243;\
             --floodColor:rgba(20, 28, 45, 0.14);"
        }
        Tone::Dark => {
            "--terminalBkg:#4F70F0;--terminalBorder:#4F70F0;--terminalText:#FFFFFF;\
             --decisionBkg:#2A2310;--decisionBorder:#7C6422;--decisionText:#F0CB6E;\
             --dataBkg:#10271F;--dataBorder:#2F6F5E;--dataText:#9BDDCB;\
             --floodColor:rgba(0, 0, 0, 0.5);"
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
        }
    }
}
