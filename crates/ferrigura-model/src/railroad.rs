//! Canonical model of a railroad diagram: the DB state after the parse of any of the four syntaxes
//! (`railroad-beta`, `railroad-ebnf-beta`, `railroad-abnf-beta`, `railroad-peg-beta`), which share
//! one DB and one model. The rules are in `docs-port/canonical-model.md` (Railroad) and
//! `docs-port/railroad/PLAN.md`.

use serde::{Deserialize, Serialize};

use crate::CANONICAL_VERSION;

/// A parsed railroad diagram: the title and the accessibility texts of the DB, and the rules in the
/// order they were written.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RailroadDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// The grammar's last `title` line, sanitised, or `""`. The front matter title does not reach
    /// the model: the reference's parser clears the DB after the front matter is set.
    pub title: String,
    /// The last `accTitle` line, sanitised, or `""`.
    pub acc_title: String,
    /// The last `accDescr` line, sanitised, or `""`.
    pub acc_descr: String,
    /// Every rule, in text order. Two rules with the same name are both kept.
    pub rules: Vec<RailroadRule>,
}

/// A rule: its name and the expression it defines.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RailroadRule {
    /// The rule name, as the DB sanitises it (the names the grammars accept hold no markup).
    pub name: String,
    /// The defining expression.
    pub definition: RailroadExpr,
}

/// An expression of a rule. The `type` field names the form, as the reference's AST does; the texts
/// are the DB's sanitised values (see `docs-port/canonical-model.md`, Railroad).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RailroadExpr {
    /// A literal text, drawn as a terminal box.
    Terminal {
        /// The sanitised text, with its escapes decoded as the grammar's own rule decodes them.
        value: String,
    },
    /// A reference to another rule.
    Nonterminal {
        /// The name of the rule referred to.
        name: String,
    },
    /// A free text, drawn as a special box: a `? … ?` sequence, a PEG lookahead label or `.`.
    Special {
        /// The sanitised text.
        text: String,
    },
    /// Its elements, one after the other. A sequence of one element is that element.
    Sequence {
        /// The elements, in order.
        elements: Vec<RailroadExpr>,
    },
    /// Its alternatives. A choice of one alternative is that alternative.
    Choice {
        /// The alternatives, in order.
        alternatives: Vec<RailroadExpr>,
    },
    /// An element that may be left out.
    Optional {
        /// The element.
        element: Box<RailroadExpr>,
    },
    /// An element repeated between `min` and `max` times.
    Repetition {
        /// The repeated element.
        element: Box<RailroadExpr>,
        /// The least number of repeats, as the grammar gives it.
        #[serde(with = "count")]
        min: f64,
        /// The most number of repeats, as the grammar gives it; `Infinity` is written as the string
        /// `"Infinity"`, as the other models write non-finite numbers.
        #[serde(with = "count")]
        max: f64,
    },
}

/// A count as JSON: the number when it is finite, else the string `"Infinity"` (see `value.rs`).
mod count {
    use serde::{Deserialize, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(value: &f64, serializer: S) -> Result<S::Ok, S::Error> {
        if value.is_finite() {
            serializer.serialize_f64(*value)
        } else if value.is_nan() {
            serializer.serialize_str("NaN")
        } else if *value > 0.0 {
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

impl RailroadDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            rules: Vec::new(),
        }
    }
}

impl Default for RailroadDiagram {
    fn default() -> Self {
        Self::new()
    }
}
