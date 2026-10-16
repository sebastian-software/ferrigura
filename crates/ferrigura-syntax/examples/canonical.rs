//! Parses inputs with a ferrigura parser and writes the canonical JSON of each result. The
//! differential harness (`pnpm port:diff`) runs this against the JS exporters of `tools/port/`.
//!
//! Usage: `cargo run -p ferrigura-syntax --example canonical -- <pie|info|flowchart|sequence> <inputs.json> <outputs.json>`
//!
//! Inputs are `[{"id": 0, "text": "…"}]`. Each output is `{"id": 0, "ok": true, "model": {…}}`,
//! `{"id": 0, "ok": false, "error": {"line": 1, "column": 1, "message": "…"}}`.

use std::error::Error;
use std::fs;

use ferrigura_model::config::effective;
use ferrigura_model::value::Value as Config;
use ferrigura_syntax::flowchart::{self, FlowchartError};
use ferrigura_syntax::sequence::{self, Options};
use ferrigura_syntax::{ParseError, config, info, pie};
use serde_json::{Value, json};

/// What a parser made of one input.
enum Outcome {
    Model(Value),
    Rejected(ParseError),
}

fn error_value(error: &ParseError) -> Value {
    json!({ "line": error.line, "column": error.column, "message": error.message })
}

fn flowchart_outcome(text: &str) -> Outcome {
    match flowchart::parse(text) {
        Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
        Err(FlowchartError::Syntax(error)) => Outcome::Rejected(error),
    }
}

/// `autoWrap()` of the reference after `init` and `clear`: `init` copies a truthy top-level `wrap`
/// into `sequence.wrap` (`sequenceDiagram.ts:21-23`), and `autoWrap` then reads `sequence.wrap`,
/// else `false`. The top-level value only matters when it is `true`.
fn sequence_wrap(text: &str) -> bool {
    let Ok(directive) = config::read(text) else {
        return false;
    };
    let resolved = effective(None, Some("sequence"), &directive);
    let top = matches!(resolved.get("wrap"), Some(Config::Bool(true)));
    top || matches!(
        resolved
            .get("sequence")
            .and_then(|section| section.get("wrap")),
        Some(Config::Bool(true))
    )
}

fn sequence_outcome(text: &str) -> Outcome {
    match sequence::parse_with(
        text,
        Options {
            wrap: sequence_wrap(text),
        },
    ) {
        Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
        Err(error) => Outcome::Rejected(error),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let [_, kind, input, output] = args.as_slice() else {
        return Err(
            "usage: canonical <pie|info|flowchart|sequence> <inputs.json> <outputs.json>".into(),
        );
    };

    let inputs: Value = serde_json::from_str(&fs::read_to_string(input)?)?;
    let items = inputs.as_array().ok_or("inputs must be a JSON array")?;

    let mut results = Vec::with_capacity(items.len());
    for item in items {
        let id = item["id"].clone();
        let text = item["text"].as_str().ok_or("each input needs a text")?;
        let outcome = match kind.as_str() {
            "pie" => match pie::parse(text) {
                Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
                Err(error) => Outcome::Rejected(error),
            },
            "info" => match info::parse(text) {
                Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
                Err(error) => Outcome::Rejected(error),
            },
            "flowchart" => flowchart_outcome(text),
            "sequence" => sequence_outcome(text),
            other => return Err(format!("unknown diagram type {other}").into()),
        };
        results.push(match outcome {
            Outcome::Model(model) => json!({ "id": id, "ok": true, "model": model }),
            Outcome::Rejected(error) => {
                json!({ "id": id, "ok": false, "error": error_value(&error) })
            }
        });
    }

    fs::write(output, serde_json::to_string_pretty(&results)?)?;
    Ok(())
}
