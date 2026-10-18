//! The shared corpus harness: loads the cases in `tests/corpus` and compares a layout snapshot with the one the
//! reference wrote, exactly.
//!
//! A case holds `input` (the layout data at the entry of `runSwimlaneLayoutCore`), `stages` (in call order:
//! `sugiyama`, `writeback`, `routed`, then the `post:<pass>` stages that changed something), and `output`. A
//! snapshot is `{direction, config, nodes, edges}`. Nodes and edges are compared by index, in JavaScript array
//! order: the router drops layout-only edges from `data.edges`, and the routed stage holds the list as the
//! reference left it, so every stage is compared with the same rule.
//!
//! The comparison is exact: a number is compared with `==` on `f64` (JSON writes -0 as 0), a string and a
//! boolean exactly, and an absent key is not `null`. A key present on one side only is a failure.
//!
//! A `post:<pass>` stage is written only when the pass changed the layout, so the state before a pass is the
//! last stage written before it (or `routed`), and a pass with no stage of its own changed nothing.

#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;

use serde_json::Value;

/// One captured call of `runSwimlaneLayoutCore`.
pub struct Case {
    pub name: String,
    pub group: String,
    pub input: Value,
    pub stages: Vec<(String, Value)>,
    pub output: Value,
    pub returned_direction: String,
}

/// The directory of the corpus, `tests/corpus` of this crate.
pub fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("corpus")
}

/// Every case in the corpus, sorted by name. The manifest is not a case.
pub fn load_cases() -> Vec<Case> {
    let mut names: Vec<PathBuf> = fs::read_dir(corpus_dir())
        .expect("the corpus directory exists")
        .map(|entry| entry.expect("a readable entry").path())
        .filter(|path| {
            path.extension().is_some_and(|ext| ext == "json")
                && path.file_name().is_some_and(|name| name != "manifest.json")
        })
        .collect();
    names.sort();
    names.iter().map(load_case).collect()
}

fn load_case(path: &PathBuf) -> Case {
    let text = fs::read_to_string(path).expect("a readable case file");
    let value: Value = serde_json::from_str(&text).expect("a case file is JSON");
    let stages = value["stages"]
        .as_array()
        .expect("stages is an array")
        .iter()
        .map(|stage| {
            (
                stage["stage"]
                    .as_str()
                    .expect("a stage has a name")
                    .to_string(),
                stage["data"].clone(),
            )
        })
        .collect();
    Case {
        name: value["name"].as_str().expect("a name").to_string(),
        group: value["group"].as_str().expect("a group").to_string(),
        input: value["input"].clone(),
        stages,
        output: value["output"].clone(),
        returned_direction: value["returnedDirection"]
            .as_str()
            .expect("a returned direction")
            .to_string(),
    }
}

impl Case {
    /// The data of the stage with this name, when the reference wrote it.
    pub fn stage(&self, name: &str) -> Option<&Value> {
        self.stages
            .iter()
            .find(|(stage, _)| stage == name)
            .map(|(_, data)| data)
    }
}

/// Compares two snapshots exactly, index by index for nodes and edges. An error names the first difference by
/// its path.
pub fn compare_snapshot(expected: &Value, actual: &Value) -> Result<(), String> {
    compare_values(expected, actual, "snapshot")
}

/// Compares two JSON values exactly: numbers with `==` on `f64`, objects by their union of keys, arrays by
/// length and index. The path of the first difference is in the error.
pub fn compare_values(expected: &Value, actual: &Value, path: &str) -> Result<(), String> {
    match (expected, actual) {
        (Value::Number(e), Value::Number(a)) => {
            let (e, a) = (e.as_f64(), a.as_f64());
            if e == a {
                Ok(())
            } else {
                Err(format!("{path}: expected {e:?}, got {a:?}"))
            }
        }
        (Value::Array(e), Value::Array(a)) => {
            if e.len() != a.len() {
                return Err(format!(
                    "{path}: {} items expected, {} got",
                    e.len(),
                    a.len()
                ));
            }
            for (index, (e, a)) in e.iter().zip(a).enumerate() {
                compare_values(e, a, &format!("{path}[{index}]"))?;
            }
            Ok(())
        }
        (Value::Object(e), Value::Object(a)) => {
            for (key, value) in e {
                match a.get(key) {
                    Some(actual) => compare_values(value, actual, &format!("{path}.{key}"))?,
                    None => {
                        return Err(format!(
                            "{path}.{key}: present in the reference, absent here"
                        ));
                    }
                }
            }
            for key in a.keys() {
                if !e.contains_key(key) {
                    return Err(format!(
                        "{path}.{key}: absent in the reference, present here"
                    ));
                }
            }
            Ok(())
        }
        (e, a) if e == a => Ok(()),
        (e, a) => Err(format!("{path}: expected {e}, got {a}")),
    }
}

/// The node and edge counts of a snapshot, for the messages of the tests.
pub fn counts(snapshot: &Value) -> (usize, usize) {
    let len = |key: &str| snapshot[key].as_array().map_or(0, Vec::len);
    (len("nodes"), len("edges"))
}
