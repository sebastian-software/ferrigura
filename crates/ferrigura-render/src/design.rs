//! The design mode: ferrigura's own look for a diagram, beside the reference's
//! (`docs/decisions/0023-design-mode.md`).
//!
//! The mode changes paint, and the place of a group's title. Its colors are theme variables that the facade lays under the user's
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
            ("nodeBorder", "#7C8698"),
            ("lineColor", "#5B6577"),
            ("arrowheadColor", "#5B6577"),
            ("textColor", "#1A1F2B"),
            ("nodeTextColor", "#1A1F2B"),
            ("titleColor", "#4A5364"),
            ("clusterBkg", "#F2F4F8"),
            ("clusterBorder", "#8891A2"),
            ("edgeLabelBackground", "#FFFFFF"),
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
        for declaration in declarations(tone).split(';') {
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
        // connections apart (1.4.11). A line is checked on the page and inside a group.
        const TEXT: [(&str, &str); 7] = [
            ("textColor", "background"),
            ("nodeTextColor", "mainBkg"),
            ("titleColor", "clusterBkg"),
            ("titleColor", "edgeLabelBackground"),
            ("terminalText", "terminalBkg"),
            ("decisionText", "decisionBkg"),
            ("dataText", "dataBkg"),
        ];
        const LINES: [&str; 6] = [
            "lineColor",
            "arrowheadColor",
            "nodeBorder",
            "terminalBorder",
            "decisionBorder",
            "dataBorder",
        ];
        for tone in [Tone::Light, Tone::Dark] {
            let colours = colours(tone);
            let of = |name: &str| colours[name].as_str();
            for (text, ground) in TEXT {
                let ratio = contrast(of(text), of(ground));
                assert!(ratio >= 4.5, "{tone:?}: {text} on {ground} is {ratio:.2}");
            }
            for line in LINES {
                for ground in ["background", "clusterBkg"] {
                    let ratio = contrast(of(line), of(ground));
                    assert!(ratio >= 3.0, "{tone:?}: {line} on {ground} is {ratio:.2}");
                }
            }
            let ratio = contrast(of("clusterBorder"), of("background"));
            assert!(ratio >= 3.0, "{tone:?}: clusterBorder is {ratio:.2}");
        }
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
        }
    }
}
