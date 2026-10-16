//! Writes the effective configuration of inputs, as ferrigura computes it, and the key set that
//! directives may use. The differential harness (`pnpm port:diff config`) runs this against the
//! TypeScript exporter `tools/port/export-config.ts`.
//!
//! Usage: `cargo run -p ferrigura-syntax --example config -- <inputs.json> <outputs.json>`
//!
//! Inputs are `[{"id": 0, "text": "…", "site": {…}}]`; `site` is optional and stands for the
//! `initialize` settings. The output is `{"configKeys": […], "results": [...]}` where each result is
//! `{"id": 0, "ok": true, "config": {…}}` or `{"id": 0, "ok": false, "error": "…"}`.

use std::error::Error;
use std::fs;

use ferrigura_model::config::{CONFIG_KEYS, effective};
use ferrigura_syntax::{DiagramType, ParseError, config, detect};
use serde_json::{Value, json};

/// The effective configuration of one text, with the site settings it was given.
fn effective_config(text: &str, site: Option<&Value>) -> Result<Value, ParseError> {
    let directive = config::read(text)?;
    let scope = detect(text).map(DiagramType::config_key);
    Ok(effective(site, scope, &directive))
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let [_, input, output] = args.as_slice() else {
        return Err("usage: config <inputs.json> <outputs.json>".into());
    };

    let inputs: Value = serde_json::from_str(&fs::read_to_string(input)?)?;
    let items = inputs.as_array().ok_or("inputs must be a JSON array")?;

    let mut results = Vec::with_capacity(items.len());
    for item in items {
        let id = item["id"].clone();
        let text = item["text"].as_str().ok_or("each input needs a text")?;
        let site = item.get("site").filter(|site| !site.is_null());
        results.push(match effective_config(text, site) {
            Ok(config) => json!({ "id": id, "ok": true, "config": config }),
            Err(error) => json!({ "id": id, "ok": false, "error": error.message }),
        });
    }

    let mut keys: Vec<&str> = CONFIG_KEYS.to_vec();
    keys.sort_unstable();
    let output_value = json!({ "configKeys": keys, "results": results });
    fs::write(output, serde_json::to_string_pretty(&output_value)?)?;
    Ok(())
}
