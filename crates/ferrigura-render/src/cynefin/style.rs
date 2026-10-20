//! The colours and sizes of the Cynefin diagram, and its style rules: the `cynefin` block of the
//! theme variables (`cynefinRenderer.ts` 61-66) and `styles.ts` 29-93.
//!
//! The reference merges the default theme's block with the active theme's, key by key (`cleanAndMerge`,
//! `utils.ts` 841): a key the theme or the user does not set keeps the default. The defaults are the
//! block of `theme-default.js` (lines 294-310), whose `textColor` and `labelColor` are the default
//! theme's own (`#333` and `#131300`). A value keeps the type the reference reads it with: a
//! template literal writes it as text (`${value}`), a `d3` attribute takes it as text and removes it
//! for `null`, and the arithmetic of `itemFontSize - 1` and `domainFontSize + 2` is JavaScript's.
//!
//! Every colour is a custom property declared on the diagram's id, and every coloured element also
//! carries its colour as a presentation attribute (`docs-port/decisions/0007`). The custom properties
//! of the dark scheme, when the render has one, are declared under the media query.

use ferrigura_model::theme::Token;
use ferrigura_model::theme::tokens::CynefinTokens;
use ferrigura_model::value::Value;

use super::layout::Domain;
use crate::common::d3::{js_number, js_to_number};
use crate::common::template_text;

/// The options of the cynefin block after the merge with the defaults. Each is kept as the reference
/// holds it (a value of any kind), and the readings below give what the reference writes from it.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Options {
    /// `domainFontSize`, in pixels: the labels, and the title two pixels larger.
    pub domain_font_size: Value,
    /// `itemFontSize`, in pixels: the item badges. The subtitles and the arrow labels are one less.
    pub item_font_size: Value,
    /// `boundaryColor`: the waves, the confusion ellipse, the item badges' strokes.
    pub boundary_color: Value,
    /// `boundaryWidth`: the stroke width of the waves.
    pub boundary_width: Value,
    /// `cliffColor`: the cliff between clear and chaotic.
    pub cliff_color: Value,
    /// `cliffWidth`: the stroke width of the cliff.
    pub cliff_width: Value,
    /// `arrowColor`: the transition arrows and their heads.
    pub arrow_color: Value,
    /// `arrowWidth`: the stroke width of the arrows.
    pub arrow_width: Value,
    /// `complexBg`: the background and the badges of the complex domain.
    pub complex_bg: Value,
    /// `complicatedBg`.
    pub complicated_bg: Value,
    /// `chaoticBg`.
    pub chaotic_bg: Value,
    /// `clearBg`.
    pub clear_bg: Value,
    /// `confusionBg`: the ellipse and the confusion badges.
    pub confusion_bg: Value,
    /// `textColor`: the subtitles, the items' text and the arrow labels.
    pub text_color: Value,
    /// `labelColor`: the domain labels and the title.
    pub label_color: Value,
}

impl Options {
    /// The options of the cynefin group of a theme's tokens: each key the group sets, else the default
    /// of the default theme's block. A theme without the group has the defaults.
    #[must_use]
    pub(super) fn of(group: Option<&CynefinTokens>) -> Self {
        let read = |token: fn(&CynefinTokens) -> &Token, default: Value| -> Value {
            group
                .and_then(|tokens| token(tokens).value().cloned())
                .unwrap_or(default)
        };
        Self {
            domain_font_size: read(|t| &t.domain_font_size, Value::Number(16.0)),
            item_font_size: read(|t| &t.item_font_size, Value::Number(12.0)),
            boundary_color: read(|t| &t.boundary_color, text("#333333")),
            boundary_width: read(|t| &t.boundary_width, Value::Number(2.0)),
            cliff_color: read(|t| &t.cliff_color, text("#8B0000")),
            cliff_width: read(|t| &t.cliff_width, Value::Number(4.0)),
            arrow_color: read(|t| &t.arrow_color, text("#333333")),
            arrow_width: read(|t| &t.arrow_width, Value::Number(2.0)),
            complex_bg: read(|t| &t.complex_bg, text("#E8F5E9")),
            complicated_bg: read(|t| &t.complicated_bg, text("#E3F2FD")),
            chaotic_bg: read(|t| &t.chaotic_bg, text("#FBE9E7")),
            clear_bg: read(|t| &t.clear_bg, text("#FFF8E1")),
            confusion_bg: read(|t| &t.confusion_bg, text("#F3E5F5")),
            text_color: read(|t| &t.text_color, text("#333")),
            label_color: read(|t| &t.label_color, text("#131300")),
        }
    }

    /// The fill of a domain's background, badges and ellipse.
    #[must_use]
    pub(super) fn background(&self, domain: Domain) -> &Value {
        match domain {
            Domain::Complex => &self.complex_bg,
            Domain::Complicated => &self.complicated_bg,
            Domain::Chaotic => &self.chaotic_bg,
            Domain::Clear => &self.clear_bg,
            Domain::Confusion => &self.confusion_bg,
        }
    }
}

fn text(value: &str) -> Value {
    Value::String(value.to_owned())
}

/// The text a `d3` `attr` call writes for a value: the value as a string, or nothing when the value is
/// `null`, which removes the attribute.
#[must_use]
pub(super) fn attribute_text(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        other => Some(template_text(other)),
    }
}

/// `Number(value)`: the value as a number, as JavaScript converts it for the arithmetic of the
/// reference (`itemFontSize - 1`).
#[must_use]
pub(super) fn to_number(value: &Value) -> f64 {
    match value {
        Value::Number(number) => *number,
        Value::Null => 0.0,
        Value::Bool(flag) => f64::from(u8::from(*flag)),
        Value::String(text) => js_to_number(text),
        Value::Array(_) | Value::Object(_) => js_to_number(&template_text(value)),
    }
}

/// `value - 1` as the reference writes it in a template literal: a number, JavaScript's subtraction.
#[must_use]
pub(super) fn minus_one(value: &Value) -> String {
    js_number(to_number(value) - 1.0)
}

/// `value + 2` as the reference writes it in a template literal. The `+` of JavaScript concatenates
/// when either side is a string, so a string or a list in the theme gives `"16" + 2 = "162"`.
#[must_use]
pub(super) fn plus_two(value: &Value) -> String {
    match value {
        Value::String(_) | Value::Array(_) | Value::Object(_) => {
            format!("{}2", template_text(value))
        }
        other => js_number(to_number(other) + 2.0),
    }
}

/// The length of a CSS `font-size` in pixels, when the text is one: a non-negative number followed by
/// `px`. The reference writes `${value}px` into the style sheet, so a value that is already a length
/// (`"12px"`) or is not a number is not a length, and the browser drops the declaration.
#[must_use]
pub(super) fn css_pixels(text: &str) -> Option<f64> {
    let number = text.strip_suffix("px")?;
    if number.is_empty()
        || !number
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'.' | b'+' | b'-' | b'e' | b'E'))
    {
        return None;
    }
    let value: f64 = number.parse().ok()?;
    (value.is_finite() && value >= 0.0).then_some(value)
}

/// The size an item's text is measured at (`cynefinRenderer.ts` 279-284). The text takes the item font
/// size from the style sheet; a declaration the browser drops leaves the text at its parent's size, the
/// root's font size `root`.
#[must_use]
pub(super) fn item_measure_size(light: &Options, root: f64) -> f64 {
    css_pixels(&format!("{}px", template_text(&light.item_font_size))).unwrap_or(root)
}

/// The custom properties of the colours of one scheme, as `--name:value;` declarations.
fn declarations(options: &Options) -> String {
    [
        ("--cynefinComplexBg", &options.complex_bg),
        ("--cynefinComplicatedBg", &options.complicated_bg),
        ("--cynefinChaoticBg", &options.chaotic_bg),
        ("--cynefinClearBg", &options.clear_bg),
        ("--cynefinConfusionBg", &options.confusion_bg),
        ("--cynefinBoundaryColor", &options.boundary_color),
        ("--cynefinCliffColor", &options.cliff_color),
        ("--cynefinArrowColor", &options.arrow_color),
        ("--cynefinTextColor", &options.text_color),
        ("--cynefinLabelColor", &options.label_color),
    ]
    .iter()
    .map(|(name, value)| format!("{name}:{};", template_text(value)))
    .collect()
}

/// The rules of the diagram, after the base rules and before the look rules, as `styles.ts` writes them
/// (lines 29-93): every selector starts with the id, the colours are custom properties, and the sizes
/// are the light scheme's, as the layout measures them. `light` declares the colours on the id; with a
/// `dark` scheme they are declared again under `prefers-color-scheme: dark`.
#[must_use]
pub(super) fn own_rules(id: &str, light: &Options, dark: Option<&Options>) -> String {
    let declared = match dark {
        None => format!("#{id}{{{}}}", declarations(light)),
        Some(dark) => format!(
            "#{id}{{{}}}@media (prefers-color-scheme: dark){{#{id}{{{}}}}}",
            declarations(light),
            declarations(dark)
        ),
    };
    let domain_size = template_text(&light.domain_font_size);
    let item_size = template_text(&light.item_font_size);
    let mut rules = format!(
        "#{id} .cynefinDomain{{stroke:none;}}\
#{id} .cynefinDomainLabel{{font-size:{domain_size}px;font-weight:bold;fill:var(--cynefinLabelColor);}}\
#{id} .cynefinSubtitle{{font-size:{}px;fill:var(--cynefinTextColor);font-style:italic;}}",
        minus_one(&light.item_font_size)
    );
    for domain in Domain::ITEMS {
        let variable = match domain {
            Domain::Complex => "--cynefinComplexBg",
            Domain::Complicated => "--cynefinComplicatedBg",
            Domain::Chaotic => "--cynefinChaoticBg",
            Domain::Clear => "--cynefinClearBg",
            Domain::Confusion => "--cynefinConfusionBg",
        };
        rules.push_str(&format!(
            "#{id} .{}{{fill:var({variable});}}",
            domain.fill_class()
        ));
    }
    rules.push_str(&format!(
        "#{id} .cynefinItem{{fill-opacity:0.95;stroke:var(--cynefinBoundaryColor);stroke-width:1;}}\
#{id} .cynefinItemText{{font-size:{item_size}px;fill:var(--cynefinTextColor);}}\
#{id} .cynefinItemOverflow{{fill-opacity:0.6;stroke:var(--cynefinBoundaryColor);stroke-width:1;stroke-dasharray:3 2;}}\
#{id} .cynefinBoundary{{stroke:var(--cynefinBoundaryColor);stroke-width:{};stroke-dasharray:6 3;}}\
#{id} .cynefinCliff{{stroke:var(--cynefinCliffColor);stroke-width:{};}}\
#{id} .cynefinConfusion{{stroke:var(--cynefinBoundaryColor);stroke-width:1.5;stroke-dasharray:4 2;}}\
#{id} .cynefinArrowLine{{stroke:var(--cynefinArrowColor);stroke-width:{};fill:none;}}\
#{id} .cynefinArrowHead{{fill:var(--cynefinArrowColor);stroke:none;}}\
#{id} .cynefinArrowLabel{{font-size:{}px;fill:var(--cynefinTextColor);}}\
#{id} .cynefinTitle{{font-size:{}px;font-weight:bold;fill:var(--cynefinLabelColor);}}",
        template_text(&light.boundary_width),
        template_text(&light.cliff_width),
        template_text(&light.arrow_width),
        minus_one(&light.item_font_size),
        plus_two(&light.domain_font_size),
    ));
    format!("{declared}{rules}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `${value} + 2` concatenates for text, and adds for a number, as JavaScript does.
    #[test]
    fn plus_two_adds_a_number_and_concatenates_text() {
        assert_eq!(plus_two(&Value::Number(16.0)), "18");
        assert_eq!(plus_two(&Value::String("20".to_owned())), "202");
        assert_eq!(plus_two(&Value::Array(vec![Value::Number(12.0)])), "122");
        assert_eq!(plus_two(&Value::Null), "2");
        assert_eq!(plus_two(&Value::Bool(true)), "3");
    }

    /// `value - 1` subtracts from the number that text converts to; text that is not a number is `NaN`.
    #[test]
    fn minus_one_converts_the_value_to_a_number() {
        assert_eq!(minus_one(&Value::Number(12.0)), "11");
        assert_eq!(minus_one(&Value::String("14".to_owned())), "13");
        assert_eq!(minus_one(&Value::String("12px".to_owned())), "NaN");
        assert_eq!(minus_one(&Value::Null), "-1");
    }

    /// A template literal writes an array as its items joined with commas, `null` items as nothing, and
    /// an object as `[object Object]`; a `d3` attribute of `null` is removed.
    #[test]
    fn template_text_and_attributes_follow_javascript() {
        let items = Value::Array(vec![
            Value::String("a".to_owned()),
            Value::Null,
            Value::Number(2.5),
        ]);
        assert_eq!(template_text(&items), "a,,2.5");
        assert_eq!(
            template_text(&Value::Object(Default::default())),
            "[object Object]"
        );
        assert_eq!(attribute_text(&Value::Null), None);
        assert_eq!(
            attribute_text(&Value::Bool(false)).as_deref(),
            Some("false")
        );
    }

    /// A font size is measured at its length; a value that is not one takes the root's size.
    #[test]
    fn an_item_size_that_is_not_a_length_takes_the_roots_size() {
        assert_eq!(css_pixels("14px"), Some(14.0));
        assert_eq!(css_pixels("14.5px"), Some(14.5));
        assert_eq!(css_pixels("12pxpx"), None);
        assert_eq!(css_pixels("NaNpx"), None);
        assert_eq!(css_pixels("-3px"), None);
        let options = Options::of(None);
        assert_eq!(item_measure_size(&options, 14.0), 12.0);
        let mut text_size = options.clone();
        text_size.item_font_size = Value::String("12px".to_owned());
        assert_eq!(item_measure_size(&text_size, 14.0), 14.0);
    }

    /// The default block of the default theme, and a theme without the group.
    #[test]
    fn the_defaults_are_the_default_themes_block() {
        let options = Options::of(None);
        assert_eq!(template_text(&options.text_color), "#333");
        assert_eq!(template_text(&options.label_color), "#131300");
        assert_eq!(template_text(&options.domain_font_size), "16");
        assert_eq!(template_text(&options.cliff_color), "#8B0000");
    }
}
