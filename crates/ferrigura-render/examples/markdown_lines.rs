//! Prints the lines and words of Markdown labels, as `markdownToLines` gives them, for the
//! differential run (`pnpm -C tools/port port:diff markdown`).
//!
//! Usage: `cargo run -p ferrigura-render --example markdown_lines -- <inputs.json> <outputs.json>`
//! reads `[{"id": 0, "text": "…"}]` and writes `[{"id": 0, "lines": [[{"content": "…", "type":
//! "normal"}]]}]`, with the types `normal`, `strong` and `em` of the reference. A label whose
//! lexing panics gives `{"id": 0, "panic": true}`, so that the run reports it with the others.

use std::error::Error;
use std::fs;

use ferrigura_render::unified::markdown::{MarkdownWordType, markdown_to_lines};
use serde_json::{Value as Json, json};

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let [inputs, outputs] = arguments.as_slice() else {
        return Err("usage: markdown_lines <inputs.json> <outputs.json>".into());
    };
    let inputs: Vec<Json> = serde_json::from_str(&fs::read_to_string(inputs)?)?;
    let results: Vec<Json> = inputs
        .iter()
        .map(|input| {
            let text = input["text"].as_str().unwrap_or_default();
            let Ok(lines) = std::panic::catch_unwind(|| markdown_to_lines(text)) else {
                return json!({ "id": input["id"], "panic": true });
            };
            let lines: Vec<Json> = lines
                .into_iter()
                .map(|line| {
                    line.into_iter()
                        .map(|word| {
                            let kind = match word.kind {
                                MarkdownWordType::Normal => "normal",
                                MarkdownWordType::Strong => "strong",
                                MarkdownWordType::Em => "em",
                            };
                            json!({ "content": word.content, "type": kind })
                        })
                        .collect()
                })
                .collect();
            json!({ "id": input["id"], "lines": lines })
        })
        .collect();
    fs::write(outputs, serde_json::to_string(&results)?)?;
    Ok(())
}
