//! The settings, the colours and the style sheet of a railroad diagram: `styles.ts`, with the values
//! that `buildRailroadStyleOptions` (lines 189-254) gives them.
//!
//! The reference takes each colour from the `railroad` section when the section sets a valid one,
//! and from the theme otherwise: the first of two theme variables that is set, then the default of
//! `DEFAULT_RAILROAD_CONFIG` (`railroadTypes.ts` 123-167). A value is valid when it passes the
//! sanitisers of `styles.ts` (lines 18-123). The numbers are valid when they are finite and not
//! negative; the font size also falls back to the theme's, and the theme's to 14.
//!
//! The colours are custom properties on the diagram's id, one set for each scheme (decision 0007),
//! and each element carries its light colour as a presentation attribute. A font family is not
//! written: every text is drawn in the bundled face, so the family in the sheet is the one the
//! embedded faces give (`common::style::base_style_sheet`).

use std::collections::BTreeMap;

use ferrigura_model::config::RailroadDiagramConfig;
use ferrigura_model::theme::tokens::ThemeTokens;
use ferrigura_model::value::Value;

use super::layout::Spacing;
use crate::common::js_trim;
use crate::context::ThemeView;
use crate::svg::number;

/// The default colours and numbers of `DEFAULT_RAILROAD_CONFIG` (`railroadTypes.ts` 123-167).
const DEFAULT_FONT_SIZE: f64 = 14.0;
const DEFAULT_PADDING: f64 = 10.0;
const DEFAULT_VERTICAL_SEPARATION: f64 = 8.0;
const DEFAULT_HORIZONTAL_SEPARATION: f64 = 10.0;
const DEFAULT_ARC_RADIUS: f64 = 10.0;
const DEFAULT_STROKE_WIDTH: f64 = 2.0;
const DEFAULT_MARKER_RADIUS: f64 = 5.0;

/// The colours of one scheme: the values that the elements and the sheet use.
#[derive(Debug, Clone, PartialEq)]
pub struct Colours {
    /// The fill of a terminal's box (`terminalFill`).
    pub terminal_fill: String,
    /// The stroke of a terminal's box (`terminalStroke`).
    pub terminal_stroke: String,
    /// The colour of a terminal's text (`terminalTextColor`).
    pub terminal_text: String,
    /// The fill of a non-terminal's box (`nonTerminalFill`).
    pub non_terminal_fill: String,
    /// The stroke of a non-terminal's box (`nonTerminalStroke`).
    pub non_terminal_stroke: String,
    /// The colour of a non-terminal's text (`nonTerminalTextColor`), also the text of a special.
    pub non_terminal_text: String,
    /// The colour of the lines (`lineColor`).
    pub line: String,
    /// The fill of the start and the end markers (`markerFill`).
    pub marker_fill: String,
    /// The fill of a comment's ellipse (`commentFill`). No element of the renderer is a comment.
    pub comment_fill: String,
    /// The stroke of a comment's ellipse (`commentStroke`).
    pub comment_stroke: String,
    /// The colour of a comment's text (`commentTextColor`).
    pub comment_text: String,
    /// The fill of a special's box (`specialFill`).
    pub special_fill: String,
    /// The stroke of a special's box (`specialStroke`).
    pub special_stroke: String,
    /// The colour of a rule's name (`ruleNameColor`).
    pub rule_name: String,
}

/// The values a railroad diagram is drawn with: the spacing, the sizes, and the colours of each
/// scheme.
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    /// The spacing of the layout.
    pub spacing: Spacing,
    /// The font size in pixels, which the texts are measured and drawn at (`fontSize`).
    pub font_size: f64,
    /// The width of the strokes in pixels (`strokeWidth`).
    pub stroke_width: f64,
    /// `useMaxWidth`: the size of the root, as a percentage or in pixels.
    pub use_max_width: bool,
    /// The colours of the light scheme, or of the one scheme of the render.
    pub light: Colours,
    /// The colours of the dark scheme, when the SVG carries both.
    pub dark: Option<Colours>,
}

impl Options {
    /// The options of a configuration section and a theme (`buildRailroadStyleOptions`).
    #[must_use]
    pub fn new(config: &RailroadDiagramConfig, theme: ThemeView<'_>) -> Self {
        let light = Variables::of(theme.light);
        let font_size = sanitize_number(
            config.font_size,
            theme_font_size(&light).unwrap_or(DEFAULT_FONT_SIZE),
        );
        Self {
            spacing: Spacing {
                padding: sanitize_number(Some(config.padding), DEFAULT_PADDING),
                vertical_separation: sanitize_number(
                    Some(config.vertical_separation),
                    DEFAULT_VERTICAL_SEPARATION,
                ),
                horizontal_separation: sanitize_number(
                    Some(config.horizontal_separation),
                    DEFAULT_HORIZONTAL_SEPARATION,
                ),
                arc_radius: sanitize_number(Some(config.arc_radius), DEFAULT_ARC_RADIUS),
                marker_radius: sanitize_number(Some(config.marker_radius), DEFAULT_MARKER_RADIUS),
            },
            font_size,
            stroke_width: sanitize_number(Some(config.stroke_width), DEFAULT_STROKE_WIDTH),
            use_max_width: config.use_max_width,
            light: colours_of(config, &light),
            dark: theme
                .dark
                .map(|tokens| colours_of(config, &Variables::of(tokens))),
        }
    }
}

/// The font size the theme gives (`parseThemeFontSize`, `styles.ts` 114-123): a number or a text
/// that starts with one, when it is finite and above zero.
fn theme_font_size(vars: &Variables) -> Option<f64> {
    let value = present(vars.get("fontSize"))?;
    let parsed = match value {
        Value::Number(number) => *number,
        Value::String(text) => js_parse_float(text),
        _ => f64::NAN,
    };
    (parsed.is_finite() && parsed > 0.0).then_some(parsed)
}

/// The variables of the default theme that the railroad styles read, as `theme-default.js` gives them
/// with no overrides (`getThemeVariables()`, `styles.ts` 193). `buildRailroadStyleOptions` merges them
/// under the active theme's variables, so a variable the active theme does not set takes this value:
/// `redux-color` sets neither `secondBkg` nor `labelBackground`, so its terminals are `#ffffde`.
const DEFAULT_THEME_VARIABLES: [(&str, &str); 17] = [
    ("fontFamily", "\"trebuchet ms\", verdana, arial, sans-serif"),
    ("fontSize", "16px"),
    ("secondBkg", "#ffffde"),
    ("secondaryColor", "#ffffde"),
    ("secondaryBorderColor", "hsl(60, 60%, 83.5294117647%)"),
    ("lineColor", "#333333"),
    ("secondaryTextColor", "#000021"),
    ("textColor", "#333"),
    ("mainBkg", "#ECECFF"),
    ("background", "white"),
    ("primaryBorderColor", "hsl(240, 60%, 86.2745098039%)"),
    ("primaryTextColor", "#131300"),
    ("labelBackground", "rgba(232,232,232, 0.8)"),
    ("tertiaryColor", "hsl(80, 100%, 96.2745098039%)"),
    ("tertiaryBorderColor", "hsl(80, 60%, 86.2745098039%)"),
    ("tertiaryTextColor", "rgb(6.3333333334, 0, 19.0000000001)"),
    ("titleColor", "#333"),
];

/// The theme variables the styles read, as the reference's merge has them: each one the theme sets
/// (a null included, which is a value the merge keeps), and the default theme's for the others.
struct Variables(BTreeMap<&'static str, Value>);

impl Variables {
    fn of(tokens: &ThemeTokens) -> Self {
        let entries = tokens.entries();
        let map = DEFAULT_THEME_VARIABLES
            .iter()
            .map(|&(name, default)| {
                let set = entries
                    .iter()
                    .find(|(key, _)| key.as_str() == name)
                    .and_then(|(_, token)| token.value().cloned());
                (
                    name,
                    set.unwrap_or_else(|| Value::String(default.to_owned())),
                )
            })
            .collect();
        Self(map)
    }

    /// The value of a variable, or `None` when the merge has no key of that name.
    fn get(&self, name: &str) -> Option<&Value> {
        self.0.get(name)
    }
}

/// The colours of one scheme: each one is the section's when that is valid, else the theme's, else
/// the default (`buildRailroadStyleOptions`, the defaults of `buildThemeDefaults`).
fn colours_of(config: &RailroadDiagramConfig, vars: &Variables) -> Colours {
    // `themeVariables.a ?? themeVariables.b`, then the colour sanitiser of `styles.ts` 85-92.
    let themed = |first: &str, second: Option<&str>, default: &str| {
        let value =
            present(vars.get(first)).or_else(|| second.and_then(|name| present(vars.get(name))));
        colour_or(value.and_then(Value::as_str), default)
    };
    // The section's own colour, when it is valid, else the theme's value.
    let set = |section: Option<&str>, themed: String| colour_or(section, &themed);

    let terminal_fill = set(
        config.terminal_fill.as_deref(),
        themed("secondBkg", Some("secondaryColor"), "#FFFFC0"),
    );
    let terminal_stroke = set(
        config.terminal_stroke.as_deref(),
        themed("secondaryBorderColor", Some("lineColor"), "#000000"),
    );
    let terminal_text = set(
        config.terminal_text_color.as_deref(),
        themed("secondaryTextColor", Some("textColor"), "#000000"),
    );
    let non_terminal_fill = set(
        config.non_terminal_fill.as_deref(),
        themed("mainBkg", Some("background"), "#FFFFFF"),
    );
    let non_terminal_stroke = set(
        config.non_terminal_stroke.as_deref(),
        themed("primaryBorderColor", Some("lineColor"), "#000000"),
    );
    let non_terminal_text = set(
        config.non_terminal_text_color.as_deref(),
        themed("primaryTextColor", Some("textColor"), "#000000"),
    );
    let line = set(
        config.line_color.as_deref(),
        themed("lineColor", None, "#000000"),
    );
    let marker_fill = set(
        config.marker_fill.as_deref(),
        themed("lineColor", None, "#000000"),
    );
    let comment_fill = set(
        config.comment_fill.as_deref(),
        themed("labelBackground", Some("tertiaryColor"), "#E8E8E8"),
    );
    let comment_stroke = set(
        config.comment_stroke.as_deref(),
        themed("tertiaryBorderColor", Some("lineColor"), "#888888"),
    );
    let comment_text = set(
        config.comment_text_color.as_deref(),
        themed("tertiaryTextColor", Some("textColor"), "#666666"),
    );
    let special_fill = set(
        config.special_fill.as_deref(),
        themed("tertiaryColor", Some("secondaryColor"), "#F0E0FF"),
    );
    let special_stroke = set(
        config.special_stroke.as_deref(),
        themed(
            "tertiaryBorderColor",
            Some("secondaryBorderColor"),
            "#8800CC",
        ),
    );
    let rule_name = set(
        config.rule_name_color.as_deref(),
        themed("titleColor", Some("textColor"), "#000066"),
    );
    Colours {
        terminal_fill,
        terminal_stroke,
        terminal_text,
        non_terminal_fill,
        non_terminal_stroke,
        non_terminal_text,
        line,
        marker_fill,
        comment_fill,
        comment_stroke,
        comment_text,
        special_fill,
        special_stroke,
        rule_name,
    }
}

/// A value that the map holds: a value that is not null (`??` skips null as it skips nothing).
fn present(value: Option<&Value>) -> Option<&Value> {
    value.filter(|value| !matches!(value, Value::Null))
}

/// `sanitizeColorValue` (`styles.ts` 85-92): the text, trimmed, when it is a colour, else the
/// fallback. A value that is not text is not a colour.
fn colour_or(text: Option<&str>, fallback: &str) -> String {
    text.map(js_trim)
        .filter(|text| is_colour(text))
        .unwrap_or(fallback)
        .to_owned()
}

/// `sanitizeNumberValue` (`styles.ts` 103-112): a number, or a text that starts with one, when it is
/// finite and not negative; else the fallback.
fn sanitize_number(value: Option<f64>, fallback: f64) -> f64 {
    match value {
        Some(number) if number.is_finite() && number >= 0.0 => number,
        _ => fallback,
    }
}

/// `COLOR_VALUE_PATTERN` (`styles.ts` 18-19), case-insensitive: a hex colour of 3, 4, 6 or 8 digits;
/// a function of the colour spaces, with digits, blanks, `%`, `+`, `,`, `.`, `/` and `-` in it; or a
/// bare name of letters.
fn is_colour(text: &str) -> bool {
    if let Some(hex) = text.strip_prefix('#') {
        return matches!(hex.len(), 3 | 4 | 6 | 8)
            && hex.bytes().all(|byte| byte.is_ascii_hexdigit());
    }
    if let Some(open) = text.find('(') {
        let name = &text[..open];
        let inner = text[open + 1..].strip_suffix(')');
        let function = [
            "rgb", "rgba", "hsl", "hsla", "hwb", "lab", "lch", "oklab", "oklch",
        ]
        .iter()
        .any(|known| name.eq_ignore_ascii_case(known));
        return function
            && inner.is_some_and(|inner| {
                !inner.is_empty()
                    && inner.chars().all(|c| {
                        c.is_ascii_digit()
                            || crate::common::is_js_whitespace(c)
                            || matches!(c, '%' | '+' | ',' | '.' | '/' | '-')
                    })
            });
    }
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_alphabetic())
}

/// `parseFloat` of a text (`styles.ts` 103-123): the longest prefix that is a decimal number, after
/// the white space that leads it; `Infinity` reads as infinity; no number reads as NaN.
fn js_parse_float(text: &str) -> f64 {
    let trimmed = text.trim_start_matches(crate::common::is_js_whitespace);
    let (sign, rest) = match trimmed.as_bytes().first() {
        Some(b'-') => (-1.0, &trimmed[1..]),
        Some(b'+') => (1.0, &trimmed[1..]),
        _ => (1.0, trimmed),
    };
    if rest.starts_with("Infinity") {
        return sign * f64::INFINITY;
    }
    let bytes = rest.as_bytes();
    let mut end = 0;
    let mut digits = 0;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
        digits += 1;
    }
    if end < bytes.len() && bytes[end] == b'.' {
        let mut fraction = end + 1;
        while fraction < bytes.len() && bytes[fraction].is_ascii_digit() {
            fraction += 1;
            digits += 1;
        }
        if digits > 0 {
            end = fraction;
        }
    }
    if digits == 0 {
        return f64::NAN;
    }
    if end < bytes.len() && (bytes[end] == b'e' || bytes[end] == b'E') {
        let mut exponent = end + 1;
        if exponent < bytes.len() && (bytes[exponent] == b'+' || bytes[exponent] == b'-') {
            exponent += 1;
        }
        let start = exponent;
        while exponent < bytes.len() && bytes[exponent].is_ascii_digit() {
            exponent += 1;
        }
        if exponent > start {
            end = exponent;
        }
    }
    rest[..end]
        .parse::<f64>()
        .map_or(f64::NAN, |value| sign * value)
}

/// The rules of the diagram's own style sheet (`styles.ts` 259-366), compiled under the id as the
/// reference's `compileCSS` writes them. The colours are the custom properties of the id; the
/// font family and the sizes are the diagram's own. `font_family` is the family as CSS reads it.
#[must_use]
pub fn own_rules(id: &str, options: &Options, font_family: &str) -> String {
    let size = number(options.font_size);
    let stroke = number(options.stroke_width);
    let mut rules = String::new();
    rules.push_str(&colour_declarations(id, &options.light));
    if let Some(dark) = &options.dark {
        rules.push_str(&format!(
            "@media (prefers-color-scheme: dark){{{}}}",
            colour_declarations(id, dark)
        ));
    }
    let text = format!("font-family:{font_family};font-size:{size}px;");
    rules.push_str(&format!(
        "#{id} .railroad-diagram{{{text}}}\
#{id} .railroad-terminal rect{{fill:var(--railroadTerminalFill);stroke:var(--railroadTerminalStroke);stroke-width:{stroke}px;}}\
#{id} .railroad-terminal text{{fill:var(--railroadTerminalTextColor);{text}text-anchor:middle;dominant-baseline:middle;}}\
#{id} .railroad-nonterminal rect{{fill:var(--railroadNonTerminalFill);stroke:var(--railroadNonTerminalStroke);stroke-width:{stroke}px;}}\
#{id} .railroad-nonterminal text{{fill:var(--railroadNonTerminalTextColor);{text}text-anchor:middle;dominant-baseline:middle;}}\
#{id} .railroad-line{{stroke:var(--railroadLineColor);stroke-width:{stroke}px;fill:none;}}\
#{id} .railroad-start circle,#{id} .railroad-end circle{{fill:var(--railroadMarkerFill);}}\
#{id} .railroad-comment ellipse{{fill:var(--railroadCommentFill);stroke:var(--railroadCommentStroke);stroke-width:{stroke}px;}}\
#{id} .railroad-comment text{{fill:var(--railroadCommentTextColor);font-style:italic;{text}text-anchor:middle;dominant-baseline:middle;}}\
#{id} .railroad-special rect{{fill:var(--railroadSpecialFill);stroke:var(--railroadSpecialStroke);stroke-width:{stroke}px;stroke-dasharray:5,3;}}\
#{id} .railroad-special text{{fill:var(--railroadNonTerminalTextColor);{text}text-anchor:middle;dominant-baseline:middle;}}\
#{id} .railroad-rule-name{{font-weight:bold;fill:var(--railroadRuleNameColor);{text}}}\
#{id} .railroad-group{{}}"
    ));
    rules
}

/// The custom properties of the colours, declared on the id.
fn colour_declarations(id: &str, colours: &Colours) -> String {
    format!(
        "#{id}{{--railroadTerminalFill:{};--railroadTerminalStroke:{};--railroadTerminalTextColor:{};\
--railroadNonTerminalFill:{};--railroadNonTerminalStroke:{};--railroadNonTerminalTextColor:{};\
--railroadLineColor:{};--railroadMarkerFill:{};--railroadCommentFill:{};--railroadCommentStroke:{};\
--railroadCommentTextColor:{};--railroadSpecialFill:{};--railroadSpecialStroke:{};\
--railroadRuleNameColor:{};}}",
        colours.terminal_fill,
        colours.terminal_stroke,
        colours.terminal_text,
        colours.non_terminal_fill,
        colours.non_terminal_stroke,
        colours.non_terminal_text,
        colours.line,
        colours.marker_fill,
        colours.comment_fill,
        colours.comment_stroke,
        colours.comment_text,
        colours.special_fill,
        colours.special_stroke,
        colours.rule_name,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_colour_sanitiser_accepts_the_reference_forms_and_rejects_the_rest() {
        for good in [
            "#fff",
            "#FFFF",
            "#00ff00",
            "#00ff00ff",
            "red",
            "rgb(1, 2, 3)",
            "hsl(60, 60%, 83.5%)",
            "OKLCH(0.5 0.1 200/0.5)",
        ] {
            assert!(is_colour(good), "{good} should pass");
        }
        for bad in [
            "#fff; stroke: red;",
            "#ff",
            "url(x)",
            "rgb(1, 2, 3",
            "",
            "#ggg",
            "rgb(a)",
        ] {
            assert!(!is_colour(bad), "{bad} should fail");
        }
    }

    #[test]
    fn parse_float_reads_the_leading_number_as_javascript_does() {
        assert_eq!(js_parse_float("14px"), 14.0);
        assert_eq!(js_parse_float("  -2.5e1em"), -25.0);
        assert_eq!(js_parse_float(".5"), 0.5);
        assert_eq!(js_parse_float("5."), 5.0);
        assert!(js_parse_float("px").is_nan());
        assert_eq!(js_parse_float("Infinity"), f64::INFINITY);
    }

    #[test]
    fn sanitize_number_keeps_finite_values_that_are_not_negative() {
        assert_eq!(sanitize_number(Some(0.0), 7.0), 0.0);
        assert_eq!(sanitize_number(Some(-1.0), 7.0), 7.0);
        assert_eq!(sanitize_number(Some(f64::NAN), 7.0), 7.0);
        assert_eq!(sanitize_number(None, 7.0), 7.0);
    }
}
