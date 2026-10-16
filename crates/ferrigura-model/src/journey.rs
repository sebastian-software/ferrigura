//! Canonical model of a user journey diagram.

use serde::{Deserialize, Serialize};

use crate::CANONICAL_VERSION;

/// A parsed user journey: its header texts, its sections in the order given, its tasks in the
/// order given, and the actors of all tasks.
///
/// Texts are the reader's text: the DB's placeholders for entities are resolved, and nothing else
/// changes; see `docs-port/journey/PLAN.md`, section 6. The DB's `type` of a task is its section, so
/// the model leaves it out.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JourneyDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// Diagram title: the grammar's last `title` line, or the front matter's title when the
    /// grammar has none, sanitised. Empty if neither is given.
    pub title: String,
    /// Accessible title from `accTitle`, empty if absent.
    pub acc_title: String,
    /// Accessible description from `accDescr` or its block, empty if absent.
    pub acc_descr: String,
    /// Every section statement in order, duplicates kept.
    pub sections: Vec<String>,
    /// Every task in the order it was given.
    pub tasks: Vec<JourneyTask>,
    /// The unique people of all tasks, sorted by UTF-16 code units of their source text.
    pub actors: Vec<String>,
}

/// One task of a journey.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JourneyTask {
    /// The section that was current when the task was given; empty before the first section.
    pub section: String,
    /// The task name, with its trailing blanks.
    pub task: String,
    /// The score, read as JavaScript's `Number()` reads the text. A score that is not finite is
    /// written as the string `"NaN"`, `"Infinity"` or `"-Infinity"`, see `value.rs`.
    #[serde(with = "score")]
    pub score: f64,
    /// The people of the task, trimmed; empty names and duplicates are kept.
    pub people: Vec<String>,
}

impl JourneyDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            sections: Vec::new(),
            tasks: Vec::new(),
            actors: Vec::new(),
        }
    }
}

impl Default for JourneyDiagram {
    fn default() -> Self {
        Self::new()
    }
}

/// A score as JSON: a number when it is finite, and the sentinel strings for the other values.
mod score {
    use serde::{Deserialize, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(score: &f64, serializer: S) -> Result<S::Ok, S::Error> {
        if score.is_finite() {
            serializer.serialize_f64(*score)
        } else if score.is_nan() {
            serializer.serialize_str("NaN")
        } else if *score > 0.0 {
            serializer.serialize_str("Infinity")
        } else {
            serializer.serialize_str("-Infinity")
        }
    }

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Written {
        Number(f64),
        Sentinel(String),
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
        match Written::deserialize(deserializer)? {
            Written::Number(number) => Ok(number),
            Written::Sentinel(text) => match text.as_str() {
                "NaN" => Ok(f64::NAN),
                "Infinity" => Ok(f64::INFINITY),
                "-Infinity" => Ok(f64::NEG_INFINITY),
                other => Err(serde::de::Error::custom(format!(
                    "expected a number or NaN, Infinity or -Infinity, found {other:?}"
                ))),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(score: f64) -> JourneyTask {
        JourneyTask {
            section: "s".to_owned(),
            task: "t".to_owned(),
            score,
            people: Vec::new(),
        }
    }

    #[test]
    fn a_finite_score_is_a_json_number() {
        let json = serde_json::to_value(task(5.0)).expect("serialises");
        assert_eq!(json["score"], serde_json::json!(5.0));
    }

    #[test]
    fn a_non_finite_score_is_a_sentinel_string() {
        let json = |score| serde_json::to_value(task(score)).expect("serialises")["score"].clone();
        assert_eq!(json(f64::NAN), serde_json::json!("NaN"));
        assert_eq!(json(f64::INFINITY), serde_json::json!("Infinity"));
        assert_eq!(json(f64::NEG_INFINITY), serde_json::json!("-Infinity"));
    }

    #[test]
    fn the_sentinels_read_back() {
        let read: JourneyTask = serde_json::from_value(serde_json::json!({
            "section": "s", "task": "t", "score": "-Infinity", "people": []
        }))
        .expect("reads");
        assert_eq!(read.score, f64::NEG_INFINITY);
    }
}
