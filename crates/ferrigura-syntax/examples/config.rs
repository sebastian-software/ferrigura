//! Writes the effective configuration of inputs, as ferrigura computes it, and the key set that
//! directives may use. The differential harness (`pnpm port:diff config`) runs this against the
//! TypeScript exporter `tools/port/export-config.ts`.
//!
//! Usage: `cargo run -p ferrigura-syntax --example config -- <inputs.json> <outputs.json>`
//!
//! Inputs are `[{"id": 0, "text": "…", "site": {…}}]`; `site` is optional and stands for the
//! `initialize` settings. The output is `{"configKeys": […], "results": [...]}` where each result is
//! `{"id": 0, "ok": true, "scope": "pie", "config": {…}}` or `{"id": 0, "ok": false, "error": "…"}`.
//! `scope` is the configuration section of the text's diagram type, or `null` when none is detected.
//!
//! The configuration keeps `NaN` and infinite numbers as numbers; JSON cannot hold them, so this
//! writes them as the strings `"NaN"`, `"Infinity"` and `"-Infinity"`, the same as the exporter.

use std::error::Error;
use std::fs;

use ferrigura_model::config::{CONFIG_KEYS, effective};
use ferrigura_model::value::Value;
use ferrigura_syntax::{DiagramType, ParseError, config, detect};
use serde_json::json;

/// The configuration section of a text's diagram type, as the reference names it.
fn scope_of(text: &str) -> Option<&'static str> {
    detect(text).map(DiagramType::config_key)
}

/// The effective configuration of one text, with the site settings it was given.
fn effective_config(text: &str, site: Option<&Value>) -> Result<Value, ParseError> {
    let directive = config::read(text)?;
    Ok(effective(site, scope_of(text), &directive))
}

/// The configuration as JSON, with the non-finite numbers written as strings.
fn differential_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Null => serde_json::Value::Null,
        Value::Bool(flag) => serde_json::Value::Bool(*flag),
        Value::Number(number) => match serde_json::Number::from_f64(*number) {
            Some(finite) => serde_json::Value::Number(finite),
            None => serde_json::Value::String(
                if number.is_nan() {
                    "NaN"
                } else if *number > 0.0 {
                    "Infinity"
                } else {
                    "-Infinity"
                }
                .to_owned(),
            ),
        },
        Value::String(text) => serde_json::Value::String(text.clone()),
        Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(differential_json).collect())
        }
        Value::Object(map) => serde_json::Value::Object(
            map.iter()
                .map(|(key, value)| (key.clone(), differential_json(value)))
                .collect(),
        ),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let [_, input, output] = args.as_slice() else {
        return Err("usage: config <inputs.json> <outputs.json>".into());
    };

    let inputs: serde_json::Value = serde_json::from_str(&fs::read_to_string(input)?)?;
    let items = inputs.as_array().ok_or("inputs must be a JSON array")?;

    let mut results = Vec::with_capacity(items.len());
    for item in items {
        let id = item["id"].clone();
        let text = item["text"].as_str().ok_or("each input needs a text")?;
        let site = item
            .get("site")
            .filter(|site| !site.is_null())
            .map(|site| Value::from(site.clone()));
        let scope = scope_of(text);
        results.push(match effective_config(text, site.as_ref()) {
            Ok(config) => json!({
                "id": id,
                "ok": true,
                "scope": scope,
                "config": differential_json(&config),
            }),
            Err(error) => json!({ "id": id, "ok": false, "error": error.message }),
        });
    }

    let mut keys: Vec<&str> = CONFIG_KEYS.to_vec();
    keys.sort_unstable();
    let output_value = json!({ "configKeys": keys, "results": results });
    fs::write(output, serde_json::to_string_pretty(&output_value)?)?;
    Ok(())
}
