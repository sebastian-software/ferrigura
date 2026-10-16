//! Loads YAML inputs with `ferrigura_syntax::yaml` and writes the canonical JSON of each result.
//! The differential harness (`pnpm port:diff yaml`) runs this against `tools/port/export-yaml.ts`,
//! which loads the same inputs with `js-yaml` and the `JSON_SCHEMA`.
//!
//! Usage: `cargo run -p ferrigura-syntax --example yaml -- <inputs.json> <outputs.json>`
//!
//! Inputs are `[{"id": 0, "text": "…"}]`. Each output is `{"id": 0, "ok": true, "value": …}` or
//! `{"id": 0, "ok": false, "error": "…"}`. The canonical value is JSON with two markers that JSON
//! cannot hold: `{"$undefined": true}` for an empty stream, and `{"$number": "NaN"}`, `"Infinity"`
//! or `"-Infinity"` for the non-finite numbers.

use std::error::Error;
use std::fs;

use ferrigura_syntax::yaml::{self, Yaml};
use serde_json::{Map, Value, json};

fn canonical(value: &Yaml) -> Value {
    match value {
        Yaml::Null => Value::Null,
        Yaml::Bool(flag) => Value::Bool(*flag),
        Yaml::Number(number) => match serde_json::Number::from_f64(*number) {
            Some(finite) => Value::Number(finite),
            None => json!({ "$number": non_finite_name(*number) }),
        },
        Yaml::String(text) => Value::String(text.clone()),
        Yaml::Sequence(items) => Value::Array(items.iter().map(canonical).collect()),
        Yaml::Mapping(pairs) => Value::Object(
            pairs
                .iter()
                .map(|(key, value)| (key.clone(), canonical(value)))
                .collect::<Map<String, Value>>(),
        ),
    }
}

fn non_finite_name(number: f64) -> &'static str {
    if number.is_nan() {
        "NaN"
    } else if number > 0.0 {
        "Infinity"
    } else {
        "-Infinity"
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let [_, input, output] = args.as_slice() else {
        return Err("usage: yaml <inputs.json> <outputs.json>".into());
    };

    let inputs: Value = serde_json::from_str(&fs::read_to_string(input)?)?;
    let items = inputs.as_array().ok_or("inputs must be a JSON array")?;

    let mut results = Vec::with_capacity(items.len());
    for item in items {
        let id = item["id"].clone();
        let text = item["text"].as_str().ok_or("each input needs a text")?;
        results.push(match yaml::load(text) {
            Ok(Some(value)) => json!({ "id": id, "ok": true, "value": canonical(&value) }),
            Ok(None) => json!({ "id": id, "ok": true, "value": { "$undefined": true } }),
            Err(error) => json!({ "id": id, "ok": false, "error": error.message }),
        });
    }

    fs::write(output, serde_json::to_string_pretty(&results)?)?;
    Ok(())
}
