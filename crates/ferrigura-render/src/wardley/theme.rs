//! The colours of the wardley map: `getTheme` of `wardleyRenderer.ts` (31-50). Each colour is the
//! `wardley` group's variable of the same name when the user sets one, else the variable the
//! reference falls back to, else the colour the code writes. The redux themes give no `wardley`
//! group, so the fallbacks are the colours of a redux render (NOTES 6.10).
//!
//! The reference reads `annotationStroke`, `annotationTextColor` and `annotationFill` as well, but
//! nothing its draw writes takes them, so they are not read here. The reference's `wardley` tokens
//! of the `base`, `dark`, `default`, `forest` and `neutral` themes reach the output only through
//! these colours and the stylesheet, which the reference discards (NOTES 6.9).

use ferrigura_model::theme::{ThemeTokens, Token};
use ferrigura_model::value::Value;

use crate::common::d3::js_number;

/// The colours the map draws with, each resolved as `getTheme` resolves it.
#[derive(Debug, Clone, PartialEq)]
pub struct Colours {
    /// `wardley.backgroundColor`, else `background`, else `#fff`.
    pub background: String,
    /// `wardley.axisColor`, else `#000`.
    pub axis: String,
    /// `wardley.axisTextColor`, else `primaryTextColor`, else `#222`.
    pub axis_text: String,
    /// `wardley.gridColor`, else `rgba(100, 100, 100, 0.2)`.
    pub grid: String,
    /// `wardley.componentFill`, else `#fff`.
    pub component_fill: String,
    /// `wardley.componentStroke`, else `#000`.
    pub component_stroke: String,
    /// `wardley.componentLabelColor`, else `primaryTextColor`, else `#222`.
    pub component_label: String,
    /// `wardley.linkStroke`, else `#000`.
    pub link_stroke: String,
    /// `wardley.evolutionStroke`, else `#dc3545`.
    pub evolution_stroke: String,
}

impl Colours {
    /// The colours of the light scheme of a render.
    #[must_use]
    pub fn of(tokens: &ThemeTokens) -> Self {
        let own = tokens.wardley.as_ref();
        let primary_text = js(&tokens.primary_text_color);
        Self {
            background: own
                .and_then(|group| js(&group.background_color))
                .or_else(|| js(&tokens.background))
                .unwrap_or_else(|| "#fff".to_owned()),
            axis: own
                .and_then(|group| js(&group.axis_color))
                .unwrap_or_else(|| "#000".to_owned()),
            axis_text: own
                .and_then(|group| js(&group.axis_text_color))
                .or_else(|| primary_text.clone())
                .unwrap_or_else(|| "#222".to_owned()),
            grid: own
                .and_then(|group| js(&group.grid_color))
                .unwrap_or_else(|| "rgba(100, 100, 100, 0.2)".to_owned()),
            component_fill: own
                .and_then(|group| js(&group.component_fill))
                .unwrap_or_else(|| "#fff".to_owned()),
            component_stroke: own
                .and_then(|group| js(&group.component_stroke))
                .unwrap_or_else(|| "#000".to_owned()),
            component_label: own
                .and_then(|group| js(&group.component_label_color))
                .or_else(|| primary_text.clone())
                .unwrap_or_else(|| "#222".to_owned()),
            link_stroke: own
                .and_then(|group| js(&group.link_stroke))
                .unwrap_or_else(|| "#000".to_owned()),
            evolution_stroke: own
                .and_then(|group| js(&group.evolution_stroke))
                .unwrap_or_else(|| "#dc3545".to_owned()),
        }
    }
}

/// The text a variable gives an attribute: `String(value)` for a value that is not `null`, and
/// `None` for an absent variable or a `null`, which the `??` of the reference passes over.
fn js(token: &Token) -> Option<String> {
    token.value().and_then(js_string)
}

/// `String(value)` of a configuration value, or `None` for `null`.
#[must_use]
pub fn js_string(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        other => Some(string_of(other)),
    }
}

/// `String(value)` for a value of any kind, as JavaScript writes it: an array joins its items with
/// commas (a `null` item is empty), and an object is `[object Object]`.
fn string_of(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => js_number(*number),
        Value::String(text) => text.clone(),
        Value::Array(items) => items.iter().map(string_of).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_colour_is_written_as_javascript_writes_it() {
        assert_eq!(js_string(&Value::Bool(true)).as_deref(), Some("true"));
        assert_eq!(js_string(&Value::Number(1.5)).as_deref(), Some("1.5"));
        assert_eq!(js_string(&Value::Number(f64::NAN)).as_deref(), Some("NaN"));
        assert_eq!(
            js_string(&Value::Array(vec![
                Value::String("a".to_owned()),
                Value::Null,
                Value::Number(2.0),
            ]))
            .as_deref(),
            Some("a,,2")
        );
        assert_eq!(
            js_string(&Value::empty_object()).as_deref(),
            Some("[object Object]")
        );
    }

    #[test]
    fn null_is_passed_over_as_undefined_is() {
        assert_eq!(js_string(&Value::Null), None);
    }
}
