//! The `treemap` section of the configuration, as `treemap/db.ts` `getConfig` reads it (26-35) with
//! the defaults of `defaultConfig.ts` 323-334.
//!
//! The typed configuration has no treemap section (the schema does not name the keys), so the
//! section is read from the effective configuration as the reference reads its `treemap` object.
//! A value the reference reads with `??` or `||` keeps the reference's rule: `padding` and
//! `diagramPadding` fall back only when they are missing, `nodeWidth` and `nodeHeight` fall back
//! when they are falsy, and `showValues` is off only for `false`.

use ferrigura_model::value::Value;

use crate::common::d3::js_to_number;

/// What the treemap renderer reads from the configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct TreemapSettings {
    /// `padding`: the inner padding of the layout (default 10).
    pub padding: f64,
    /// `diagramPadding`: the padding of the viewport (default 8).
    pub diagram_padding: f64,
    /// `showValues` is not `false`: the values are drawn.
    pub show_values: bool,
    /// `nodeWidth`: the width of the layout in tenths of a pixel (default 100). Falsy means 960 px.
    pub node_width: f64,
    /// `nodeHeight`: the height of the layout in tenths of a pixel (default 40). Falsy means 500 px.
    pub node_height: f64,
    /// `useMaxWidth` is true (default true).
    pub use_max_width: bool,
    /// `valueFormat`: the d3 specifier of the values (default `,`). A value that is not a non-empty
    /// string gives the default, as `config.valueFormat || ','` does for the empty string.
    pub value_format: String,
}

impl Default for TreemapSettings {
    fn default() -> Self {
        Self {
            padding: 10.0,
            diagram_padding: 8.0,
            show_values: true,
            node_width: 100.0,
            node_height: 40.0,
            use_max_width: true,
            value_format: ",".to_owned(),
        }
    }
}

impl TreemapSettings {
    /// The settings of the effective configuration: its `treemap` object, with the defaults for the
    /// keys it does not give.
    #[must_use]
    pub fn of(effective: &Value) -> Self {
        let section = effective.get("treemap");
        let mut settings = Self::default();
        let Some(section) = section else {
            return settings;
        };
        if let Some(value) = number_of(section, "padding") {
            settings.padding = value;
        }
        if let Some(value) = number_of(section, "diagramPadding") {
            settings.diagram_padding = value;
        }
        if let Some(Value::Bool(false)) = section.get("showValues") {
            settings.show_values = false;
        }
        if let Some(value) = number_of(section, "nodeWidth") {
            settings.node_width = value;
        }
        if let Some(value) = number_of(section, "nodeHeight") {
            settings.node_height = value;
        }
        if let Some(value) = section.get("useMaxWidth") {
            settings.use_max_width = truthy(value);
        }
        if let Some(Value::String(format)) = section.get("valueFormat")
            && !format.is_empty()
        {
            settings.value_format = format.clone();
        }
        settings
    }

    /// `config.nodeWidth ? config.nodeWidth * 10 : 960`.
    #[must_use]
    pub fn layout_width(&self) -> f64 {
        if truthy_number(self.node_width) {
            self.node_width * 10.0
        } else {
            960.0
        }
    }

    /// `config.nodeHeight ? config.nodeHeight * 10 : 500`.
    #[must_use]
    pub fn layout_height(&self) -> f64 {
        if truthy_number(self.node_height) {
            self.node_height * 10.0
        } else {
            500.0
        }
    }
}

/// A number in the section: a number as it is, a numeric string as `+text` reads it, and NaN for a
/// value that is not a number. A missing key or `null` is `None`, so the default stays.
fn number_of(section: &Value, key: &str) -> Option<f64> {
    match section.get(key)? {
        Value::Number(value) => Some(*value),
        Value::String(text) => Some(js_to_number(text)),
        Value::Bool(flag) => Some(f64::from(u8::from(*flag))),
        _ => None,
    }
}

/// JavaScript's truth for a configuration value: null, false, zero, NaN and the empty string are
/// false.
fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(flag) => *flag,
        Value::Number(number) => truthy_number(*number),
        Value::String(text) => !text.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// JavaScript's truth for a number: zero and NaN are false.
fn truthy_number(value: f64) -> bool {
    value != 0.0 && !value.is_nan()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn section(entries: &[(&str, Value)]) -> Value {
        let mut treemap = BTreeMap::new();
        for (key, value) in entries {
            treemap.insert((*key).to_owned(), value.clone());
        }
        let mut root = BTreeMap::new();
        root.insert("treemap".to_owned(), Value::Object(treemap));
        Value::Object(root)
    }

    #[test]
    fn the_defaults_are_the_reference_defaults() {
        let settings = TreemapSettings::of(&Value::empty_object());
        assert_eq!(settings, TreemapSettings::default());
        assert_eq!(settings.layout_width(), 1000.0);
        assert_eq!(settings.layout_height(), 400.0);
    }

    #[test]
    fn a_zero_node_width_is_the_default_size_of_960() {
        let settings = TreemapSettings::of(&section(&[("nodeWidth", Value::Number(0.0))]));
        assert_eq!(settings.layout_width(), 960.0);
    }

    #[test]
    fn show_values_is_off_only_for_false() {
        let off = TreemapSettings::of(&section(&[("showValues", Value::Bool(false))]));
        assert!(!off.show_values);
        let zero = TreemapSettings::of(&section(&[("showValues", Value::Number(0.0))]));
        assert!(zero.show_values);
    }

    #[test]
    fn an_empty_value_format_is_the_default_comma() {
        let settings =
            TreemapSettings::of(&section(&[("valueFormat", Value::String(String::new()))]));
        assert_eq!(settings.value_format, ",");
    }
}
