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
        chart_variables(tone, map);
    }
    object
}

/// The number of colors of the series palette.
pub const SERIES_LENGTH: usize = 8;

/// The series palette of the light set: one calm color for each series of a chart, in a fixed
/// order: blue, teal, amber, violet, green, rose, cyan and brick. A series takes the color of its
/// position and wraps after the last. Each is dark enough for white text on it
/// ([`series_text`]), and neighbors, the last and the first included, stay apart for a reader
/// with a red-green deficiency.
pub const SERIES_LIGHT: [&str; SERIES_LENGTH] = [
    "#3E63DD", "#0F8466", "#9C6310", "#7A55C7", "#4A812B", "#C2456B", "#1C75AB", "#B0573A",
];

/// The series palette of the dark set: the hues of [`SERIES_LIGHT`], light enough for the dark
/// background's color as text on them.
pub const SERIES_DARK: [&str; SERIES_LENGTH] = [
    "#7C9AF2", "#4DB89A", "#D9A04A", "#A88DE8", "#8CBF66", "#E98BA6", "#5FAED8", "#E09A7A",
];

/// The tints of the light series: each color at 12 percent on the background, the fill of a box
/// that a series color outlines, with the text color on it.
pub const SERIES_TINT_LIGHT: [&str; SERIES_LENGTH] = [
    "#E8ECFB", "#E2F0ED", "#F3ECE2", "#EFEBF8", "#E9F0E6", "#F8E9ED", "#E4EEF5", "#F6EBE7",
];

/// The tints of the dark series: each color at 18 percent on the background.
pub const SERIES_TINT_DARK: [&str; SERIES_LENGTH] = [
    "#252C41", "#1D3231", "#362D23", "#2D2A3F", "#283328", "#392933", "#20303C", "#372C2B",
];

/// The series palette of a tone: the solid colors of the marks of a chart (a pie's slices, an XY
/// chart's bars and lines) and the outlines and edges of a series of boxes.
#[must_use]
pub const fn series(tone: Tone) -> &'static [&'static str; SERIES_LENGTH] {
    match tone {
        Tone::Light => &SERIES_LIGHT,
        Tone::Dark => &SERIES_DARK,
    }
}

/// The tints of the series palette of a tone: the fills of a series of boxes (a gantt section's
/// bars, a mindmap branch's nodes).
#[must_use]
pub const fn series_tint(tone: Tone) -> &'static [&'static str; SERIES_LENGTH] {
    match tone {
        Tone::Light => &SERIES_TINT_LIGHT,
        Tone::Dark => &SERIES_TINT_DARK,
    }
}

/// The color of text on a solid series color: white on the light set, the background's on the dark.
#[must_use]
pub const fn series_text(tone: Tone) -> &'static str {
    match tone {
        Tone::Light => "#FFFFFF",
        Tone::Dark => "#12141A",
    }
}

/// The color of a critical gantt task's outline and of the today marker: a red that the series
/// palette does not use, for a part that means attention.
#[must_use]
pub const fn critical(tone: Tone) -> (&'static str, &'static str) {
    match tone {
        Tone::Light => ("#FBEAEA", "#C93C37"),
        Tone::Dark => ("#3A2224", "#F0776F"),
    }
}

/// The stroke width of a gantt task's outline in the mode, the node outline's.
pub const TASK_STROKE_WIDTH: &str = NODE_STROKE_WIDTH;

/// The stroke width of an active gantt task's outline: heavier than a task's, so that it stands out
/// without a color of its own.
pub const ACTIVE_TASK_STROKE_WIDTH: &str = "2.5px";

/// The theme variables of the charts and the series types for a tone: the pie, the XY chart, the
/// quadrant chart, the gantt chart and the mindmap read them from the shared theme, and the user's
/// variables win over them as over the others.
fn chart_variables(tone: Tone, map: &mut std::collections::BTreeMap<String, Value>) {
    let series = series(tone);
    let tint = series_tint(tone);
    let (critical_fill, critical_line) = critical(tone);
    let (background, text, muted, border, band, grid, done) = match tone {
        Tone::Light => (
            "#FFFFFF", "#1A1F2B", "#4A5364", "#7C8698", "#F6F7F9", "#E3E6EC", "#EEF0F3",
        ),
        Tone::Dark => (
            "#12141A", "#E7E9EE", "#A3ACBC", "#667085", "#171A22", "#2A2F3A", "#1D212A",
        ),
    };
    let mut set = |name: String, colour: &str| {
        map.insert(name, Value::from(colour));
    };
    // The pie: the slices in the series colors, parted by the background, with no outer ring.
    for index in 0..12 {
        set(format!("pie{}", index + 1), series[index % SERIES_LENGTH]);
    }
    set("pieStrokeColor".into(), background);
    set("pieOuterStrokeColor".into(), background);
    set("pieStrokeWidth".into(), "2px");
    set("pieOuterStrokeWidth".into(), "0px");
    set("pieOpacity".into(), "1");
    set("pieTitleTextColor".into(), text);
    set("pieSectionTextColor".into(), series_text(tone));
    set("pieLegendTextColor".into(), text);
    set("pieTitleTextSize".into(), "18px");
    set("pieSectionTextSize".into(), "14px");
    set("pieLegendTextSize".into(), "14px");
    // The quadrant chart: four tints of the palette parted by the background, muted labels.
    for (index, colour) in tint.iter().take(4).enumerate() {
        set(format!("quadrant{}Fill", index + 1), colour);
        set(format!("quadrant{}TextFill", index + 1), muted);
    }
    set("quadrantPointFill".into(), series[0]);
    set("quadrantPointTextFill".into(), text);
    set("quadrantXAxisTextFill".into(), muted);
    set("quadrantYAxisTextFill".into(), muted);
    set("quadrantInternalBorderStrokeFill".into(), background);
    set("quadrantExternalBorderStrokeFill".into(), background);
    set("quadrantTitleFill".into(), text);
    // The gantt chart: quiet section bands and grid, the bars of a section in its series color,
    // done tasks neutral and critical ones red.
    set("sectionBkgColor".into(), band);
    set("sectionBkgColor2".into(), band);
    set("altSectionBkgColor".into(), background);
    set("excludeBkgColor".into(), done);
    set("gridColor".into(), grid);
    set("todayLineColor".into(), critical_line);
    set("taskBkgColor".into(), tint[0]);
    set("taskBorderColor".into(), series[0]);
    set("activeTaskBkgColor".into(), tint[0]);
    set("activeTaskBorderColor".into(), series[0]);
    set("doneTaskBkgColor".into(), done);
    set("doneTaskBorderColor".into(), border);
    set("critBkgColor".into(), critical_fill);
    set("critBorderColor".into(), critical_line);
    for name in [
        "taskTextColor",
        "taskTextOutsideColor",
        "taskTextDarkColor",
        "taskTextLightColor",
    ] {
        set(name.into(), text);
    }
    // A link in a bar's text: the terminal's blue, which reads on the bar's tint.
    let link = match tone {
        Tone::Light => "#2F54D6",
        Tone::Dark => series[0],
    };
    set("taskTextClickableColor".into(), link);
    set("vertLineColor".into(), muted);
    // The mindmap: a branch's nodes in the tint of its series color, outlined and joined in it;
    // the root in the flowchart's terminal color. Slot `i` is the section `i - 1`, so the first
    // branch takes the second color, and the root's blue returns only after the eighth.
    for index in 0..12 {
        let slot = index % SERIES_LENGTH;
        set(format!("cScale{index}"), tint[slot]);
        set(format!("cScalePeer{index}"), series[slot]);
        set(format!("cScaleLabel{index}"), text);
    }
    let (root, root_text) = match tone {
        Tone::Light => ("#2F54D6", "#FFFFFF"),
        Tone::Dark => ("#4263EB", "#FFFFFF"),
    };
    set("git0".into(), root);
    set("gitBranchLabel0".into(), root_text);
    // The XY chart: the plots in the series colors, the axes and their texts muted.
    let mut xy = std::collections::BTreeMap::new();
    for (name, colour) in [
        ("backgroundColor", background),
        ("titleColor", text),
        ("legendTextColor", text),
        ("xAxisTitleColor", muted),
        ("xAxisLabelColor", muted),
        ("xAxisTickColor", border),
        ("xAxisLineColor", border),
        ("yAxisTitleColor", muted),
        ("yAxisLabelColor", muted),
        ("yAxisTickColor", border),
        ("yAxisLineColor", border),
    ] {
        xy.insert(name.to_owned(), Value::from(colour));
    }
    xy.insert("plotColorPalette".to_owned(), Value::from(series.join(",")));
    map.insert("xyChart".into(), Value::Object(xy));
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
        if let Value::Object(map) = &theme_variables(tone) {
            for (name, value) in map {
                if let Some(value) = value.as_str() {
                    colours.insert(name.clone(), value.to_owned());
                }
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
            // The divider under an entity's or a class's name is a line on the box's ground; an
            // ER relation's marker stands on the page, its ring filled with the page's ground.
            let ratio = contrast(of("nodeBorder"), of("mainBkg"));
            assert!(
                ratio >= 3.0,
                "{tone:?}: nodeBorder on mainBkg is {ratio:.2}"
            );
        }
    }

    #[test]
    fn the_series_and_the_charts_keep_their_contrast_in_both_tones() {
        // The series palette: a mark reaches 3 to 1 on the background (1.4.11), the text on a
        // mark 4.5 (1.4.3), the text on a tint 4.5, and the outline of a tint 3.
        for tone in [Tone::Light, Tone::Dark] {
            let colours = colours(tone);
            let of = |name: &str| colours[name].as_str();
            for ((colour, tint), slot) in series(tone).iter().zip(series_tint(tone)).zip(0..) {
                let on = |a: &str, b: &str| contrast(a, b);
                assert!(
                    on(colour, of("background")) >= 3.0,
                    "{tone:?}: series {slot}"
                );
                assert!(
                    on(series_text(tone), colour) >= 4.5,
                    "{tone:?}: text on {slot}"
                );
                assert!(
                    on(of("textColor"), tint) >= 4.5,
                    "{tone:?}: text on tint {slot}"
                );
                assert!(
                    on(colour, tint) >= 3.0,
                    "{tone:?}: series {slot} on its tint"
                );
            }
            // The charts' text on its ground, and their lines on theirs.
            const TEXT: [(&str, &str); 16] = [
                ("pieTitleTextColor", "background"),
                ("pieLegendTextColor", "background"),
                ("taskTextColor", "critBkgColor"),
                ("taskTextColor", "doneTaskBkgColor"),
                ("taskTextColor", "sectionBkgColor"),
                ("taskTextOutsideColor", "background"),
                ("taskTextClickableColor", "taskBkgColor"),
                ("taskTextClickableColor", "background"),
                ("titleColor", "sectionBkgColor"),
                ("vertLineColor", "background"),
                ("quadrant1TextFill", "quadrant1Fill"),
                ("quadrant2TextFill", "quadrant2Fill"),
                ("quadrant3TextFill", "quadrant3Fill"),
                ("quadrant4TextFill", "quadrant4Fill"),
                ("quadrantXAxisTextFill", "background"),
                ("gitBranchLabel0", "git0"),
            ];
            for (text, ground) in TEXT {
                let ratio = contrast(of(text), of(ground));
                assert!(ratio >= 4.5, "{tone:?}: {text} on {ground} is {ratio:.2}");
            }
            const LINES: [(&str, &str); 7] = [
                ("critBorderColor", "background"),
                ("critBorderColor", "critBkgColor"),
                ("doneTaskBorderColor", "background"),
                ("doneTaskBorderColor", "doneTaskBkgColor"),
                ("todayLineColor", "background"),
                ("quadrantPointFill", "quadrant1Fill"),
                ("git0", "background"),
            ];
            for (line, ground) in LINES {
                let ratio = contrast(of(line), of(ground));
                assert!(ratio >= 3.0, "{tone:?}: {line} on {ground} is {ratio:.2}");
            }
            // The XY chart's texts and axes.
            let variables = theme_variables(tone);
            let xy = variables.get("xyChart").expect("the XY chart's group");
            let xy = |name: &str| xy.get(name).and_then(Value::as_str).expect(name).to_owned();
            for name in [
                "titleColor",
                "legendTextColor",
                "xAxisLabelColor",
                "yAxisTitleColor",
            ] {
                let ratio = contrast(&xy(name), of("background"));
                assert!(ratio >= 4.5, "{tone:?}: xyChart.{name} is {ratio:.2}");
            }
            for name in ["xAxisLineColor", "yAxisTickColor"] {
                let ratio = contrast(&xy(name), of("background"));
                assert!(ratio >= 3.0, "{tone:?}: xyChart.{name} is {ratio:.2}");
            }
            assert_eq!(xy("plotColorPalette"), series(tone).join(","));
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
        let names = |tone| match &theme_variables(tone) {
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
