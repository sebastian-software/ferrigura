//! Parses inputs with a ferrigura parser and writes the canonical JSON of each result. The
//! differential harness (`pnpm port:diff`) runs this against the JS exporters of `tools/port/`.
//!
//! Usage: `cargo run -p ferrigura-syntax --example canonical -- <pie|info|flowchart|timeline|journey|gitgraph|er|class|sequence|state|gantt|mindmap|block|packet|xychart|sankey|quadrantChart|requirement|c4|kanban|architecture|treemap|ishikawa|eventmodeling|agentflow|cynefin|radar|railroad|swimlanes|treeView|usecase|venn|wardley> <inputs.json> <outputs.json>`
//!
//! Inputs are `[{"id": 0, "text": "…"}]`. Each output is `{"id": 0, "ok": true, "model": {…}}`,
//! `{"id": 0, "ok": false, "error": {"line": 1, "column": 1, "message": "…"}}`.
//!
//! The gitGraph outputs apply the named normalisation of `docs-port/gitgraph/PLAN.md` section 7
//! (`normalise_auto_ids`): the ids the port generates and the ids the reference draws at random
//! are replaced by `auto#k` in the order of the commits, on both sides.

use std::error::Error;
use std::fs;

use ferrigura_model::config::effective;
use ferrigura_model::value::Value as Config;
use ferrigura_syntax::flowchart::{self, FlowchartError};
use ferrigura_syntax::gantt;
use ferrigura_syntax::sequence::{self, Options};
use ferrigura_syntax::{
    DiagramType, ParseError, agentflow, architecture, block, c4, class, config, cynefin, detect,
    er, event_modeling, git_graph, info, ishikawa, journey, kanban, mindmap, packet, pie,
    quadrant_chart, radar, railroad, requirement, sankey, state, swimlanes, timeline, tree_view,
    treemap, use_case, venn, wardley, xychart,
};
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

/// The architecture diagram. The text must first be detected as one, as `detectType` does before
/// the parser runs, so that a text of another type is a rejection.
fn architecture_outcome(text: &str) -> Outcome {
    if detect(text) != Some(DiagramType::Architecture) {
        return Outcome::Rejected(ParseError {
            line: 1,
            column: 1,
            message: "no architecture diagram detected".to_owned(),
        });
    }
    match architecture::parse(text) {
        Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
        Err(error) => Outcome::Rejected(error),
    }
}

/// The clock of the gantt differential run: 1010-10-10T00:00Z, the instant the reference runs with
/// (`tools/port/export-gantt.ts`).
const GANTT_NOW_MS: i64 = -30_270_326_400_000;

fn gantt_outcome(text: &str) -> Outcome {
    match gantt::parse_with(text, gantt::Options { now: GANTT_NOW_MS }) {
        Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
        Err(error) => Outcome::Rejected(error),
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

/// A sequence model, with the links of each actor written as `[key, value]` pairs in their order.
/// The model keeps the key order of a JavaScript object, but `serde_json::Value` sorts the keys of
/// an object and the differential run compares objects without regard to their order, so the order
/// is compared as a list (`docs-port/canonical-model.md`, Sequence).
/// The C4 diagram. The top-level `wrap` configuration is what the DB reads (`init`), so the
/// example resolves it from the text's directives and front matter, as the sequence arm does.
fn c4_outcome(text: &str) -> Outcome {
    match c4::parse_with(
        text,
        c4::Options {
            wrap: c4_wrap(text),
        },
    ) {
        Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
        Err(error) => Outcome::Rejected(error),
    }
}

/// The top-level `wrap` of the effective configuration, `true` only when it is set to `true`.
fn c4_wrap(text: &str) -> bool {
    let Ok(directive) = config::read(text) else {
        return false;
    };
    let resolved = effective(None, Some("c4"), &directive);
    matches!(resolved.get("wrap"), Some(Config::Bool(true)))
}

fn sequence_outcome(text: &str) -> Outcome {
    match sequence::parse_with(
        text,
        Options {
            wrap: sequence_wrap(text),
        },
    ) {
        Ok(model) => {
            let mut value = serde_json::to_value(&model).unwrap_or_default();
            if let Some(actors) = value.get_mut("actors").and_then(Value::as_array_mut) {
                for (actor, record) in actors.iter_mut().zip(&model.actors) {
                    actor["links"] = record
                        .links
                        .iter()
                        .map(|(key, link)| json!([key, link]))
                        .collect();
                }
            }
            Outcome::Model(value)
        }
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

/// The placeholders of the generated block ids, numbered in the order the tree gives them.
#[derive(Default)]
struct BlockNames(std::collections::HashMap<String, String>);

impl BlockNames {
    /// The placeholder of a base id: `$genK` for the k-th base first seen.
    fn placeholder(&mut self, base: &str) -> String {
        let next = self.0.len() + 1;
        self.0
            .entry(base.to_owned())
            .or_insert_with(|| format!("$gen{next}"))
            .clone()
    }
}

/// The generated ids of a block diagram, as `normaliseGenerated` in `tools/port/export-block.ts`
/// replaces them: a generated id `id-<random>-<n>` becomes the placeholder of its base, with the
/// cell suffix `-<j>` of a space kept; the label of a space cell is its base and takes the same
/// placeholder. The blocks are visited in tree order, id before label before children.
fn normalise_block_ids(mut model: Value) -> Value {
    let generated = regex::Regex::new(r"^(id-[0-9a-z]*-[0-9]+)(-[0-9]+)?$").expect("a pattern");
    let base = regex::Regex::new(r"^id-[0-9a-z]*-[0-9]+$").expect("a pattern");
    let mut names = BlockNames::default();
    if let Some(blocks) = model.get_mut("blocks").and_then(Value::as_array_mut) {
        visit_blocks(blocks, &mut names, &generated, &base);
    }
    model
}

fn visit_blocks(
    blocks: &mut [Value],
    names: &mut BlockNames,
    generated: &regex::Regex,
    base: &regex::Regex,
) {
    for block in blocks {
        if let Some(caps) = block["id"].as_str().and_then(|id| generated.captures(id)) {
            let name = names.placeholder(&caps[1]);
            let suffix = caps.get(2).map_or("", |m| m.as_str());
            block["id"] = json!(format!("{name}{suffix}"));
        }
        let space_label = (block["type"] == json!("space"))
            .then(|| block["label"].as_str().filter(|label| base.is_match(label)))
            .flatten()
            .map(str::to_owned);
        if let Some(label) = space_label {
            block["label"] = json!(names.placeholder(&label));
        }
        if let Some(children) = block.get_mut("children").and_then(Value::as_array_mut) {
            visit_blocks(children, names, generated, base);
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let [_, kind, input, output] = args.as_slice() else {
        return Err(
            "usage: canonical <pie|info|flowchart|timeline|journey|gitgraph|er|class|sequence|state|gantt|mindmap|block|packet|xychart|sankey|quadrantChart|requirement|c4|kanban|architecture|treemap|ishikawa|eventmodeling|agentflow|cynefin|radar|railroad|swimlanes|treeView|usecase|venn|wardley> <inputs.json> <outputs.json>".into(),
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
            "er" => match er::parse(text) {
                Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
                Err(error) => Outcome::Rejected(error),
            },
            "class" => match class::parse(text) {
                Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
                Err(error) => Outcome::Rejected(error),
            },
            "sequence" => sequence_outcome(text),
            "state" if detect(text) != Some(DiagramType::State) => Outcome::Rejected(ParseError {
                line: 1,
                column: 1,
                message: "no state diagram detected".to_owned(),
            }),
            "state" => match state::parse(text) {
                Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
                Err(error) => Outcome::Rejected(error),
            },
            "gantt" => gantt_outcome(text),
            "mindmap" => match mindmap::parse(text) {
                Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
                Err(error) => Outcome::Rejected(error),
            },
            "block" => match block::parse(text) {
                Ok(model) => Outcome::Model(normalise_block_ids(
                    serde_json::to_value(model).unwrap_or_default(),
                )),
                Err(error) => Outcome::Rejected(error),
            },
            "packet" => match packet::parse(text) {
                Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
                Err(error) => Outcome::Rejected(error),
            },
            "xychart" => match xychart::parse(text) {
                Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
                Err(error) => Outcome::Rejected(error),
            },
            "sankey" => match sankey::parse(text) {
                Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
                Err(error) => Outcome::Rejected(error),
            },
            "quadrantChart" => match quadrant_chart::parse(text) {
                Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
                Err(error) => Outcome::Rejected(error),
            },
            "requirement" => match requirement::parse(text) {
                Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
                Err(error) => Outcome::Rejected(error),
            },
            "c4" => c4_outcome(text),
            "kanban" => match kanban::parse(text) {
                Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
                Err(error) => Outcome::Rejected(error),
            },
            "architecture" => architecture_outcome(text),
            "treemap" => match treemap::parse(text) {
                Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
                Err(error) => Outcome::Rejected(error),
            },
            "ishikawa" => match ishikawa::parse(text) {
                Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
                Err(error) => Outcome::Rejected(error),
            },
            "eventmodeling" => match event_modeling::parse(text) {
                Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
                Err(error) => Outcome::Rejected(error),
            },
            "agentflow" => match agentflow::parse(text) {
                Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
                Err(error) => Outcome::Rejected(error),
            },
            "cynefin" => match cynefin::parse(text) {
                Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
                Err(error) => Outcome::Rejected(error),
            },
            "radar" => match radar::parse(text) {
                Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
                Err(error) => Outcome::Rejected(error),
            },
            "railroad" => match railroad::parse(text) {
                Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
                Err(error) => Outcome::Rejected(error),
            },
            "swimlanes" => match swimlanes::parse(text) {
                Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
                Err(error) => Outcome::Rejected(error),
            },
            "treeView" => match tree_view::parse(text) {
                Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
                Err(error) => Outcome::Rejected(error),
            },
            "usecase" => match use_case::parse(text) {
                Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
                Err(error) => Outcome::Rejected(error),
            },
            "venn" => match venn::parse(text) {
                Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
                Err(error) => Outcome::Rejected(error),
            },
            "wardley" => match wardley::parse(text) {
                Ok(model) => Outcome::Model(serde_json::to_value(model).unwrap_or_default()),
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
