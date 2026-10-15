//! Parses inputs with a ferrigura parser and writes the canonical JSON of each result. The
//! differential harness (`pnpm port:diff`) runs this against the JS exporters of `tools/port/`.
//!
//! Usage: `cargo run -p ferrigura-syntax --example canonical -- <pie|info> <inputs.json> <outputs.json>`
//!
//! Inputs are `[{"id": 0, "text": "…"}]`. Each output is `{"id": 0, "ok": true, "model": {…}}` or
//! `{"id": 0, "ok": false, "error": {"line": 1, "column": 1, "message": "…"}}`.

use std::error::Error;
use std::fs;

use ferrigura_syntax::{ParseError, info, pie};
use serde_json::{Value, json};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let [_, kind, input, output] = args.as_slice() else {
        return Err("usage: canonical <pie|info> <inputs.json> <outputs.json>".into());
    };

    let inputs: Value = serde_json::from_str(&fs::read_to_string(input)?)?;
    let items = inputs.as_array().ok_or("inputs must be a JSON array")?;

    let mut results = Vec::with_capacity(items.len());
    for item in items {
        let id = item["id"].clone();
        let text = item["text"].as_str().ok_or("each input needs a text")?;
        let outcome: Result<Value, ParseError> = match kind.as_str() {
            "pie" => pie::parse(text).map(|model| serde_json::to_value(model).unwrap_or_default()),
            "info" => {
                info::parse(text).map(|model| serde_json::to_value(model).unwrap_or_default())
            }
            other => return Err(format!("unknown diagram type {other}").into()),
        };
        results.push(match outcome {
            Ok(model) => json!({ "id": id, "ok": true, "model": model }),
            Err(error) => json!({
                "id": id,
                "ok": false,
                "error": { "line": error.line, "column": error.column, "message": error.message },
            }),
        });
    }

    fs::write(output, serde_json::to_string_pretty(&results)?)?;
    Ok(())
}
