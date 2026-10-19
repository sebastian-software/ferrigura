//! Canonical model of a radar diagram: the state of the reference's `radar/db.ts` after the parse.
//!
//! The model holds the header texts, the axes in text order, the curves in text order with their
//! entries, and the options. The entries of a curve are in axis order when the curve names its
//! entries, and in text order when it lists numbers. `max` is the one nullable field: it is `null`
//! when the text does not give one. A number that is not finite is written as the sentinel string
//! `"Infinity"` or `"NaN"`, as in the other models. The configuration and the theme are not in the
//! model, and nor is anything the renderer derives. See `docs-port/radar/PLAN.md`.

use serde::{Deserialize, Serialize};

use crate::CANONICAL_VERSION;

/// A parsed radar diagram.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RadarDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// The diagram title, sanitised: the grammar's `title` line when it is non-empty, else the
    /// front matter's title. Empty if neither is given.
    pub title: String,
    /// The accessible title from `accTitle`, sanitised, empty if absent.
    pub acc_title: String,
    /// The accessible description from `accDescr` or its block, sanitised, empty if absent.
    pub acc_descr: String,
    /// The axes in text order. A duplicate name is kept.
    pub axes: Vec<RadarAxis>,
    /// The curves in text order.
    pub curves: Vec<RadarCurve>,
    /// The options, each with its default when the text does not give it.
    pub options: RadarOptions,
}

/// An axis: its name as written, and its label, which is the name when no label is given.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadarAxis {
    /// The name as written in the text.
    pub name: String,
    /// The label, from `name["label"]`, or the name.
    pub label: String,
}

/// A curve: its name, its label, and its values, one per axis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RadarCurve {
    /// The name as written in the text.
    pub name: String,
    /// The label, from `name["label"]`, or the name.
    pub label: String,
    /// The values. Their count is not checked against the axes.
    #[serde(with = "number::list")]
    pub entries: Vec<f64>,
}

/// The options of a radar diagram, after the DB's defaults and its clamp of `ticks`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RadarOptions {
    /// Whether the legend is drawn.
    pub show_legend: bool,
    /// The number of ticks, at most 32. A fraction and zero are kept.
    #[serde(with = "number")]
    pub ticks: f64,
    /// The largest value of the scale, or `null` when the text gives none. Zero is kept.
    #[serde(default, with = "number::optional")]
    pub max: Option<f64>,
    /// The smallest value of the scale, 0 by default.
    #[serde(with = "number")]
    pub min: f64,
    /// The shape of the graticule.
    pub graticule: Graticule,
}

/// The shape of the graticule, as the text names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Graticule {
    /// `circle`, the default.
    Circle,
    /// `polygon`.
    Polygon,
}

impl RadarDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`] and the default options.
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            axes: Vec::new(),
            curves: Vec::new(),
            options: RadarOptions::default(),
        }
    }
}

impl Default for RadarDiagram {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for RadarOptions {
    fn default() -> Self {
        Self {
            show_legend: true,
            ticks: 5.0,
            max: None,
            min: 0.0,
            graticule: Graticule::Circle,
        }
    }
}

/// A number as JSON: a number when it is finite, and the sentinel strings for the other values
/// (`docs-port/canonical-model.md`).
mod number {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// One number, written with its sentinel when it is not finite.
    struct Written(f64);

    impl Serialize for Written {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let value = self.0;
            if value.is_finite() {
                serializer.serialize_f64(value)
            } else if value.is_nan() {
                serializer.serialize_str("NaN")
            } else if value > 0.0 {
                serializer.serialize_str("Infinity")
            } else {
                serializer.serialize_str("-Infinity")
            }
        }
    }

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Read {
        Number(f64),
        Sentinel(String),
    }

    fn read<E: serde::de::Error>(value: Read) -> Result<f64, E> {
        match value {
            Read::Number(number) => Ok(number),
            Read::Sentinel(text) => match text.as_str() {
                "NaN" => Ok(f64::NAN),
                "Infinity" => Ok(f64::INFINITY),
                "-Infinity" => Ok(f64::NEG_INFINITY),
                other => Err(E::custom(format!(
                    "expected a number or NaN, Infinity or -Infinity, found {other:?}"
                ))),
            },
        }
    }

    pub(super) fn serialize<S: Serializer>(value: &f64, serializer: S) -> Result<S::Ok, S::Error> {
        Written(*value).serialize(serializer)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
        read(Read::deserialize(deserializer)?)
    }

    /// The `Option` of a number: `null` for none.
    pub(super) mod optional {
        use super::{Deserialize, Deserializer, Read, Serialize, Serializer, Written, read};

        pub(in crate::radar) fn serialize<S: Serializer>(
            value: &Option<f64>,
            serializer: S,
        ) -> Result<S::Ok, S::Error> {
            match value {
                Some(number) => Written(*number).serialize(serializer),
                None => serializer.serialize_none(),
            }
        }

        pub(in crate::radar) fn deserialize<'de, D: Deserializer<'de>>(
            deserializer: D,
        ) -> Result<Option<f64>, D::Error> {
            Option::<Read>::deserialize(deserializer)?
                .map(read)
                .transpose()
        }
    }

    /// A list of numbers, each written as [`serialize`] writes one.
    pub(super) mod list {
        use super::{Deserialize, Deserializer, Read, Serializer, Written, read};

        pub(in crate::radar) fn serialize<S: Serializer>(
            values: &[f64],
            serializer: S,
        ) -> Result<S::Ok, S::Error> {
            serializer.collect_seq(values.iter().map(|value| Written(*value)))
        }

        pub(in crate::radar) fn deserialize<'de, D: Deserializer<'de>>(
            deserializer: D,
        ) -> Result<Vec<f64>, D::Error> {
            Vec::<Read>::deserialize(deserializer)?
                .into_iter()
                .map(read)
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json(value: &RadarDiagram) -> serde_json::Value {
        serde_json::to_value(value).expect("the model serialises")
    }

    #[test]
    fn an_absent_max_is_null_and_a_zero_max_is_kept() {
        let mut diagram = RadarDiagram::new();
        assert_eq!(json(&diagram)["options"]["max"], serde_json::Value::Null);
        diagram.options.max = Some(0.0);
        assert_eq!(json(&diagram)["options"]["max"], serde_json::json!(0.0));
    }

    #[test]
    fn a_non_finite_number_is_its_sentinel_string() {
        let mut diagram = RadarDiagram::new();
        diagram.curves.push(RadarCurve {
            name: "c".into(),
            label: "c".into(),
            entries: vec![f64::INFINITY, 1.5],
        });
        diagram.options.min = f64::NEG_INFINITY;
        let value = json(&diagram);
        assert_eq!(
            value["curves"][0]["entries"],
            serde_json::json!(["Infinity", 1.5])
        );
        assert_eq!(value["options"]["min"], serde_json::json!("-Infinity"));
    }

    #[test]
    fn the_keys_are_camel_case() {
        let value = json(&RadarDiagram::new());
        assert_eq!(value["version"], serde_json::json!(CANONICAL_VERSION));
        assert!(value.get("accTitle").is_some());
        assert_eq!(value["options"]["showLegend"], serde_json::json!(true));
        assert_eq!(value["options"]["graticule"], serde_json::json!("circle"));
    }

    #[test]
    fn a_model_reads_back_from_its_json() {
        let mut diagram = RadarDiagram::new();
        diagram.axes.push(RadarAxis {
            name: "A".into(),
            label: "Alpha".into(),
        });
        diagram.options.max = None;
        diagram.options.min = f64::INFINITY;
        let back: RadarDiagram =
            serde_json::from_value(json(&diagram)).expect("the model reads back");
        assert_eq!(back.axes, diagram.axes);
        assert!(back.options.min.is_infinite());
        assert_eq!(back.options.max, None);
    }
}
