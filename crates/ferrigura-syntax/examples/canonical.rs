//! Parses inputs with a ferrigura parser and writes the canonical JSON of each result. The
//! differential harness (`pnpm port:diff`) runs this against the JS exporters of `tools/port/`.
//!
//! Usage: `cargo run -p ferrigura-syntax --example canonical -- <pie|info|flowchart|timeline|journey|gitgraph> <inputs.json> <outputs.json>`
//!
//! Inputs are `[{"id": 0, "text": "…"}]`. Each output is `{"id": 0, "ok": true, "model": {…}}`,
//! `{"id": 0, "ok": false, "error": {"line": 1, "column": 1, "message": "…"}}`.
//!
//! The gitGraph outputs apply the named normalisation of `docs-port/gitgraph/PLAN.md` section 7
//! (`normalise_auto_ids`): the ids the port generates and the ids the reference draws at random
//! are replaced by `auto#k` in the order of the commits, on both sides.

use std::error::Error;
use std::fs;

use ferrigura_syntax::flowchart::{self, FlowchartError};
use ferrigura_syntax::{DiagramType, ParseError, detect, git_graph, info, journey, pie, timeline};
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

/// A timeline, as the reference gives it: the text must first be detected as one, as `detectType`
/// does before the parser runs.
fn timeline_outcome(text: &str) -> Outcome {
    if detect(text) != Some(DiagramType::Timeline) {
        let message = "no timeline detected".to_owned();
        return Outcome::Rejected(ParseError {
            line: 1,
            column: 1,
            message,
        });
    }
    match timeline::parse(text) {
        Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
        Err(error) => Outcome::Rejected(error),
    }
}

/// The named normalisation of the auto ids: each commit with `autoId` true gets `auto#k`, with `k`
/// counted from 1 in the order of the commits, and every parent or branch head with the same id
/// gets the same placeholder. The flag itself is kept, so a difference in which ids were generated
/// still shows.
fn normalise_auto_ids(mut model: Value) -> Value {
    let mut placeholders = std::collections::BTreeMap::new();
    if let Some(commits) = model.get_mut("commits").and_then(Value::as_array_mut) {
        for commit in commits.iter_mut() {
            if commit.get("autoId") != Some(&Value::Bool(true)) {
                continue;
            }
            let Some(id) = commit.get("id").and_then(Value::as_str).map(str::to_owned) else {
                continue;
            };
            let placeholder = format!("auto#{}", placeholders.len() + 1);
            placeholders
                .entry(id)
                .or_insert_with(|| placeholder.clone());
            commit["id"] = json!(placeholders.get(commit["id"].as_str().unwrap_or_default()));
        }
    }
    let replace = |id: &mut Value| {
        if let Some(placeholder) = id.as_str().and_then(|text| placeholders.get(text)) {
            *id = json!(placeholder);
        }
    };
    if let Some(commits) = model.get_mut("commits").and_then(Value::as_array_mut) {
        for commit in commits.iter_mut() {
            if let Some(parents) = commit.get_mut("parents").and_then(Value::as_array_mut) {
                parents.iter_mut().for_each(replace);
            }
        }
    }
    if let Some(branches) = model.get_mut("branches").and_then(Value::as_array_mut) {
        for branch in branches.iter_mut() {
            if let Some(head) = branch.get_mut("head").filter(|head| !head.is_null()) {
                replace(head);
            }
        }
    }
    model
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let [_, kind, input, output] = args.as_slice() else {
        return Err(
            "usage: canonical <pie|info|flowchart|timeline|journey|gitgraph> <inputs.json> <outputs.json>"
                .into(),
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
            "timeline" => timeline_outcome(text),
            "journey" => match journey::parse(text) {
                Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
                Err(error) => Outcome::Rejected(error),
            },
            "gitgraph" => match git_graph::parse(text) {
                Ok(model) => Outcome::Model(normalise_auto_ids(
                    serde_json::to_value(model).unwrap_or_default(),
                )),
                Err(error) => Outcome::Rejected(error),
            },
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
