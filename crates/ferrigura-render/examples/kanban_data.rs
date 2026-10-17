//! Prints the layout data of kanban diagram text, as `getData` gives it, for the differential run
//! (`pnpm port:diff kanban-data`). The configuration is the effective one of the text, with the front
//! matter and directives applied, as the facade gives it to the renderer.
//!
//! Usage:
//! - `cargo run -p ferrigura-render --example kanban_data < diagram.mmd` reads the text from stdin
//!   and prints `{"ok": true, "model": {…}}`, or `{"ok": false, "error": "…"}` when the text does
//!   not parse.
//! - `cargo run -p ferrigura-render --example kanban_data -- <inputs.json> <outputs.json>` does the
//!   same for every input of `[{"id": 0, "text": "…"}]`, and writes the outputs as a JSON array of
//!   `{"id": 0, "ok": …, "model": …}` or `{"id": 0, "ok": false, "error": "…"}`.

use std::error::Error;
use std::io::Read;
use std::{fs, io};

use ferrigura_model::config::{MermaidConfig, effective, typed};
use ferrigura_render::kanban::data::{DataNode, Shape, get_data};
use ferrigura_syntax::{DiagramType, config, detect, kanban};
use serde_json::{Map, Value as Json, json};

/// The effective configuration of the text, as the facade gives it to the renderer.
fn configuration(text: &str) -> Result<MermaidConfig, String> {
    let directive = config::read(text).map_err(|error| error.message)?;
    let scope = detect(text).map(DiagramType::config_key);
    let effective_config = effective(None, scope, &directive);
    typed(&effective_config).map_err(|error| error.to_string())
}

/// One node of the layout data: the fields `getData` sets, and no others.
fn node_model(node: &DataNode) -> Result<Json, String> {
    let mut model = Map::new();
    model.insert("id".to_owned(), json!(node.id));
    if let Some(parent) = &node.parent_id {
        model.insert("parentId".to_owned(), json!(parent));
    }
    model.insert("label".to_owned(), json!(node.label));
    model.insert("labelType".to_owned(), json!("markdown"));
    model.insert("isGroup".to_owned(), json!(node.is_group));
    if let Some(ticket) = &node.ticket {
        model.insert("ticket".to_owned(), json!(ticket));
    }
    if let Some(priority) = &node.priority {
        model.insert(
            "priority".to_owned(),
            serde_json::to_value(priority).map_err(|error| error.to_string())?,
        );
    }
    if let Some(assigned) = &node.assigned {
        model.insert("assigned".to_owned(), json!(assigned));
    }
    if let Some(icon) = &node.icon {
        model.insert("icon".to_owned(), json!(icon));
    }
    model.insert(
        "shape".to_owned(),
        json!(match node.shape {
            Shape::Section => "kanbanSection",
            Shape::Item => "kanbanItem",
        }),
    );
    model.insert("level".to_owned(), json!(node.level));
    if let Some(look) = &node.look {
        model.insert("look".to_owned(), json!(look));
    }
    if let Some(radius) = node.corner_radius {
        model.insert("rx".to_owned(), json!(radius));
        model.insert("ry".to_owned(), json!(radius));
    }
    if node.shape == Shape::Item {
        model.insert("cssStyles".to_owned(), json!(node.css_styles));
    }
    Ok(Json::Object(model))
}

/// The layout data of the text as JSON, or the reason it does not parse.
fn layout(text: &str) -> Result<Json, String> {
    let diagram = kanban::parse(text).map_err(|error| error.to_string())?;
    let config = configuration(text)?;
    let nodes = get_data(&diagram, &config.look)
        .iter()
        .map(node_model)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({ "nodes": nodes }))
}

/// The outcome of one text as the differential run reads it.
fn outcome(text: &str) -> (bool, Json) {
    match layout(text) {
        Ok(model) => (true, model),
        Err(error) => (false, Json::String(error)),
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
            let inputs: Json = serde_json::from_str(&fs::read_to_string(input)?)?;
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
                "usage: kanban_data  (text on stdin) | kanban_data <inputs.json> <outputs.json>"
                    .into(),
            );
        }
    }
    Ok(())
}
