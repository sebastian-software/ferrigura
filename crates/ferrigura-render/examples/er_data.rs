//! Prints the layout data of ER text, as `getData` gives it, for the differential run
//! (`pnpm port:diff er-data`). The configuration is the effective one of the text, with the front
//! matter and directives applied, as `config` gives it.
//!
//! Usage:
//! - `cargo run -p ferrigura-render --example er_data < diagram.mmd` reads the text from stdin and
//!   prints `{"ok": true, "model": {…}}`, or `{"ok": false, "error": "…"}` when the text does not
//!   parse.
//! - `cargo run -p ferrigura-render --example er_data -- <inputs.json> <outputs.json>` does the
//!   same for every input of `[{"id": 0, "text": "…"}]`, and writes the outputs as a JSON array of
//!   `{"id": 0, "ok": …, "model": …}` or `{"id": 0, "ok": false, "error": "…"}`.

use std::error::Error;
use std::io::Read;
use std::{fs, io};

use ferrigura_model::config::{MermaidConfig, effective, typed};
use ferrigura_render::context::Globals;
use ferrigura_render::er::data::get_data;
use ferrigura_syntax::{DiagramType, config, detect, er};
use serde_json::{Value, json};

/// The effective configuration of the text, as the render gives it to `getData`.
fn configuration(text: &str) -> Result<MermaidConfig, String> {
    let directive = config::read(text).map_err(|error| error.message)?;
    let scope = detect(text).map(DiagramType::config_key);
    typed(&effective(None, scope, &directive)).map_err(|error| error.to_string())
}

/// The layout data of the text as JSON, or the reason it does not parse.
fn layout(text: &str) -> Result<Value, String> {
    let diagram = er::parse(text).map_err(|error| error.to_string())?;
    let config = configuration(text)?;
    let globals = Globals::of(&config);
    serde_json::to_value(get_data(&diagram, &globals)).map_err(|error| error.to_string())
}

/// The outcome of one text as the differential run reads it.
fn outcome(text: &str) -> (bool, Value) {
    match layout(text) {
        Ok(model) => (true, model),
        Err(error) => (false, Value::String(error)),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    match args.as_slice() {
        [_] => {
            let mut text = String::new();
            io::stdin().read_to_string(&mut text)?;
            let line = match outcome(&text) {
                (true, model) => json!({ "ok": true, "model": model }),
                (false, error) => json!({ "ok": false, "error": error }),
            };
            println!("{line}");
        }
        [_, input, output] => {
            let inputs: Value = serde_json::from_str(&fs::read_to_string(input)?)?;
            let items = inputs.as_array().ok_or("inputs must be a JSON array")?;
            let mut results = Vec::with_capacity(items.len());
            for item in items {
                let id = item["id"].clone();
                let text = item["text"].as_str().ok_or("each input needs a text")?;
                results.push(match outcome(text) {
                    (true, model) => json!({ "id": id, "ok": true, "model": model }),
                    (false, error) => json!({ "id": id, "ok": false, "error": error }),
                });
            }
            fs::write(output, serde_json::to_string_pretty(&results)?)?;
        }
        _ => {
            return Err(
                "usage: er_data  (text on stdin) | er_data <inputs.json> <outputs.json>".into(),
            );
        }
    }
    Ok(())
}
