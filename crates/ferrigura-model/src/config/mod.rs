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
mod lenient;

pub use generated::*;
pub use layers::{
    assign_with_depth, deep_merge, effective, sanitize_directive, user_theme_variables,
};
pub use lenient::{ConfigWarning, Outcome};

use std::sync::LazyLock;

use crate::value::Value;

/// The reference's defaults that its schema does not hold, as a JSON object. See `ts_defaults.json`.
const TS_DEFAULTS: &str = include_str!("ts_defaults.json");

/// The defaults of every key, computed once per process. See [`defaults`].
static DEFAULTS: LazyLock<Value> = LazyLock::new(|| {
    let mut value = Value::from(serde_json::to_value(MermaidConfig::default()).unwrap_or_default());
    let overrides =
        Value::from(serde_json::from_str::<serde_json::Value>(TS_DEFAULTS).unwrap_or_default());
    layers::assign_with_depth(&mut value, &overrides, 2);
    value
});

/// The defaults of every key: the schema's defaults, with the reference's TypeScript-only defaults
/// on top. This is a copy of the defaults that [`cached_defaults`] shares.
#[must_use]
pub fn defaults() -> Value {
    cached_defaults().clone()
}

/// The defaults of every key, borrowed. They are computed once per process, so that reading the
/// configuration does not rebuild them on every render.
pub(crate) fn cached_defaults() -> &'static Value {
    &DEFAULTS
}

/// The options of the schema, as the lenient reader walks them. Built once per process.
static SCHEMA: LazyLock<lenient::Schema> = LazyLock::new(|| lenient::Schema::new(FIELD_TYPES));

/// The typed view of a configuration, and the values of the wrong type that it read or replaced.
/// See [`typed_with_warnings`].
#[derive(Debug, Clone, PartialEq)]
pub struct TypedConfig {
    /// The configuration, with a value of its type for every option that the types name.
    pub config: MermaidConfig,
    /// The values that were read or replaced, in the order of the options. Empty when every value
    /// has its type.
    pub warnings: Vec<ConfigWarning>,
}

/// The typed view of a configuration value. It never fails for a value of the wrong type, and it
/// ignores the warnings; [`typed_with_warnings`] returns them.
///
/// # Errors
///
/// Returns an error only when the configuration is not an object, which [`effective`] never returns.
pub fn typed(value: &Value) -> Result<MermaidConfig, serde_json::Error> {
    typed_with_warnings(value).map(|typed| typed.config)
}

/// The typed view of a configuration value, with the values of the wrong type that were read or
/// replaced. Keys that the types do not name are ignored, and a non-finite number reads into an
/// `f64` field as it is. A value of the wrong type never fails: a string that JavaScript reads as
/// a finite number is read as that number, where the schema asks for a number or an integer (the
/// integer only when whole), and any other value is replaced by the option's default. Each such
/// value is reported; the rule is documented in the `lenient` module.
///
/// # Errors
///
/// Returns an error only when the configuration is not an object, which [`effective`] never returns.
pub fn typed_with_warnings(value: &Value) -> Result<TypedConfig, serde_json::Error> {
    let mut read = value.clone();
    let warnings = lenient::read_types(&mut read, cached_defaults(), &SCHEMA);
    let config: MermaidConfig = serde::Deserialize::deserialize(read)?;
    Ok(TypedConfig { config, warnings })
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
    fn the_field_types_list_each_option_once_and_its_parents_first() {
        // The reader's schema is a tree with one node per path, and it visits an option after the
        // options above it, as the list does: a path is listed once, and a parent that is listed comes
        // before the options below it.
        for (index, (path, _)) in FIELD_TYPES.iter().enumerate() {
            assert!(
                FIELD_TYPES[..index]
                    .iter()
                    .all(|(earlier, _)| earlier != path),
                "{path:?} is listed twice"
            );
            for depth in 1..path.len() {
                let parent = &path[..depth];
                if let Some(at) = FIELD_TYPES.iter().position(|(listed, _)| listed == &parent) {
                    assert!(at < index, "{parent:?} comes after {path:?}");
                }
            }
        }
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

    #[test]
    fn the_defaults_have_the_types_the_schema_gives_them() {
        let read = typed_with_warnings(&defaults()).expect("the defaults are an object");
        assert!(read.warnings.is_empty(), "{:?}", read.warnings);
    }

    #[test]
    fn a_numeric_string_for_flowchart_rank_spacing_is_read_as_its_number() {
        // The e2e fixture `visual-check/23`: `flowchart.rankSpacing: '100'` in its front matter.
        let value = json!({ "flowchart": { "rankSpacing": "100" } });
        let read = typed_with_warnings(&value).expect("a configuration object");
        assert_eq!(read.config.flowchart.rank_spacing, 100);
        assert_eq!(read.warnings.len(), 1);
        assert_eq!(read.warnings[0].path, "flowchart.rankSpacing");
        assert_eq!(read.warnings[0].outcome, Outcome::Read(100.0));
        assert_eq!(
            typed(&value)
                .expect("a configuration object")
                .flowchart
                .rank_spacing,
            100
        );
    }

    #[test]
    fn values_of_the_wrong_type_never_fail_the_typed_view() {
        let value = json!({
            "theme": 5,
            "logLevel": true,
            "fontSize": "big",
            "secure": "theme",
            "flowchart": "not a section",
            "pie": { "textPosition": true, "useMaxWidth": "maybe", "donutHole": "0.25" },
        });
        let read = typed_with_warnings(&value).expect("a configuration object");
        let defaults = typed_with_warnings(&defaults())
            .expect("the defaults")
            .config;
        assert_eq!(read.config.flowchart, defaults.flowchart);
        assert_eq!(read.config.font_size, defaults.font_size);
        assert_eq!(read.config.pie.text_position, defaults.pie.text_position);
        assert_eq!(read.config.pie.donut_hole, 0.25);
        // The options are visited in the order of the schema's keys, parents first.
        let paths: Vec<&str> = read.warnings.iter().map(|w| w.path.as_str()).collect();
        assert_eq!(
            paths,
            [
                "flowchart",
                "fontSize",
                "logLevel",
                "pie.donutHole",
                "pie.textPosition",
                "pie.useMaxWidth",
                "secure",
                "theme",
            ]
        );
        let donut = read
            .warnings
            .iter()
            .find(|w| w.path == "pie.donutHole")
            .map(|w| &w.outcome);
        assert_eq!(donut, Some(&Outcome::Read(0.25)));
    }

    #[test]
    fn a_union_option_keeps_a_string_and_a_number() {
        let read = typed_with_warnings(&json!({ "logLevel": "debug" })).expect("an object");
        assert!(read.warnings.is_empty());
    }

    #[test]
    fn a_configuration_that_is_not_an_object_is_an_error() {
        assert!(typed(&Value::Null).is_err());
    }
}
