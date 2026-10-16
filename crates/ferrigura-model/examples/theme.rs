//! Writes the theme variables of inputs, as ferrigura computes them. The differential harness
//! (`pnpm exec tsx tools/port/diff-theme.ts`) runs this against the TypeScript exporter
//! `tools/port/export-theme.ts`.
//!
//! Usage: `cargo run -p ferrigura-model --example theme -- <inputs.json> <outputs.json>`
//!
//! Inputs are `[{"id": 0, "theme": "redux-color" | "redux-dark-color", "overrides": {…} | null}]`;
//! `null` stands for no overrides. The output is `[{"id": 0, "ok": true, "variables": {…}}]`, or
//! `{"id": 0, "ok": false, "variables": {}, "error": "…"}` when a colour cannot be read.
//!
//! The variables keep `NaN` and infinite numbers as numbers; JSON cannot hold them, so this writes
//! them as the strings `"NaN"`, `"Infinity"` and `"-Infinity"`, the same as the exporter.

use std::error::Error;
use std::fs;

use ferrigura_model::theme::Theme;
use ferrigura_model::value::Value;
use serde_json::json;

/// The variables as JSON, with the non-finite numbers written as strings.
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
        return Err("usage: theme <inputs.json> <outputs.json>".into());
    };

    let inputs: serde_json::Value = serde_json::from_str(&fs::read_to_string(input)?)?;
    let items = inputs.as_array().ok_or("inputs must be a JSON array")?;

    let mut results = Vec::with_capacity(items.len());
    for item in items {
        let id = item["id"].clone();
        let name = item["theme"].as_str().ok_or("each input needs a theme")?;
        let overrides = item
            .get("overrides")
            .filter(|overrides| !overrides.is_null())
            .map(|overrides| Value::from(overrides.clone()));
        results.push(
            match Theme::from_name(name).try_variables(overrides.as_ref()) {
                Ok(variables) => {
                    json!({ "id": id, "ok": true, "variables": differential_json(&variables) })
                }
                Err(error) => json!({
                    "id": id,
                    "ok": false,
                    "variables": {},
                    "error": error.message(),
                }),
            },
        );
    }

    fs::write(output, serde_json::to_string_pretty(&results)?)?;
    Ok(())
}
