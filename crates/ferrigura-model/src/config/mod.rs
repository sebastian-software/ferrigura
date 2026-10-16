//! Configuration of ferrigura.
//!
//! The types and their defaults are generated from the schema that the reference uses, by
//! `build.rs`; the schema stays the single source. [`effective`] computes the configuration of one
//! diagram from the `initialize` settings and the text's directives, in the order the reference
//! applies them, and [`deep_merge`] merges front matter and directives. The differential run
//! (`pnpm port:diff config`) compares the results with the reference; see
//! `docs-port/decisions/0003-differential-testing.md`.

mod generated {
    include!(concat!(env!("OUT_DIR"), "/config_schema.rs"));
}

mod layers;

pub use generated::*;
pub use layers::{assign_with_depth, deep_merge, effective, sanitize_directive};

use crate::value::Value;

/// The reference's defaults that its schema does not hold, as a JSON object. See `ts_defaults.json`.
const TS_DEFAULTS: &str = include_str!("ts_defaults.json");

/// The defaults of every key: the schema's defaults, with the reference's TypeScript-only defaults
/// on top.
#[must_use]
pub fn defaults() -> Value {
    let mut value = Value::from(serde_json::to_value(MermaidConfig::default()).unwrap_or_default());
    let overrides =
        Value::from(serde_json::from_str::<serde_json::Value>(TS_DEFAULTS).unwrap_or_default());
    layers::assign_with_depth(&mut value, &overrides, 2);
    value
}

/// The typed view of a configuration value. Keys that the types do not name are ignored; a value of
/// the wrong type is an error. A non-finite number reads into an `f64` field as it is.
pub fn typed(value: &Value) -> Result<MermaidConfig, serde_json::Error> {
    serde::Deserialize::deserialize(value.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `serde_json::json!`, as a configuration value.
    macro_rules! json {
        ($($tokens:tt)*) => {
            Value::from(serde_json::json!($($tokens)*))
        };
    }

    #[test]
    fn the_defaults_are_valid_json_for_the_typed_view() {
        assert!(serde_json::from_str::<serde_json::Value>(TS_DEFAULTS).is_ok());
        let config = typed(&defaults()).expect("the defaults read back into the types");
        assert_eq!(config.pie.text_position, 0.75);
    }

    #[test]
    fn config_keys_are_sorted_and_unique() {
        assert!(CONFIG_KEYS.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn config_keys_name_the_schema_and_the_reference_only_keys() {
        for key in [
            "theme",
            "pie",
            "textPosition",
            "fontFamily",
            "primaryColor",
            "treemap",
        ] {
            assert!(CONFIG_KEYS.contains(&key), "missing {key}");
        }
        for key in ["secure", "dompurifyConfig"] {
            assert!(!CONFIG_KEYS.contains(&key), "unexpected {key}");
        }
    }

    #[test]
    fn defaults_hold_the_schema_and_typescript_values() {
        let value = defaults();
        assert_eq!(value["theme"], json!("default"));
        assert_eq!(value["pie"]["useWidth"], json!(984));
        assert_eq!(value["securityLevel"], json!("strict"));
    }
}
