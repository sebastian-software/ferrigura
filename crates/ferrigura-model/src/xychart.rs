//! Canonical model of an xychart diagram: the state of the reference's `xychartDb.ts` after the
//! parse, before the chart builder runs.
//!
//! The model holds the orientation, the title and the accessibility texts, the x-axis (a band of
//! categories or a linear range), the y-axis (always linear), and the plots in the order they were
//! declared, with their data. The x value of a point is the string the DB stores: the category, or
//! the JavaScript string of the linear position. An absent value is `null`, and a number that is not
//! finite is written as `"NaN"`, `"Infinity"` or `"-Infinity"`, as in the other models. See
//! `docs-port/xychart/PLAN.md`.

use serde::ser::SerializeTuple;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::CANONICAL_VERSION;

/// A parsed xychart diagram.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XyChartDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// `horizontal` or `vertical`: the keyword of the header when it is given, else the
    /// configuration's `chartOrientation`.
    pub orientation: String,
    /// The diagram title, sanitised: the grammar's `title` line when it has one, even a blank one,
    /// else the front matter's title. Empty if neither is given.
    pub title: String,
    /// The accessible title from `accTitle`, sanitised, empty if absent.
    pub acc_title: String,
    /// The accessible description from `accDescr` or its block, sanitised, empty if absent.
    pub acc_descr: String,
    /// The x-axis: its title and either its categories or its range.
    pub x_axis: XAxis,
    /// The y-axis: its title and its range, which is widened by the data unless it is declared.
    pub y_axis: YAxis,
    /// The plots (lines and bars) in the order they were declared.
    pub plots: Vec<Plot>,
}

/// The x-axis of a chart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum XAxis {
    /// A band axis: one band per category, in the order given.
    Band {
        /// The axis title.
        title: String,
        /// The categories, each sanitised; duplicates are kept.
        categories: Vec<String>,
    },
    /// A linear axis from `min` to `max`.
    #[serde(rename_all = "camelCase")]
    Linear {
        /// The axis title.
        title: String,
        /// The first value of the range.
        #[serde(with = "js_number")]
        min: f64,
        /// The last value of the range.
        #[serde(with = "js_number")]
        max: f64,
    },
}

/// The y-axis of a chart, which is always linear.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum YAxis {
    /// The linear range. Without a declared range it covers the data; with no plot at all it is
    /// `Infinity` to `-Infinity`.
    #[serde(rename_all = "camelCase")]
    Linear {
        /// The axis title.
        title: String,
        /// The lower end of the range.
        #[serde(with = "js_number")]
        min: f64,
        /// The upper end of the range.
        #[serde(with = "js_number")]
        max: f64,
    },
}

/// A plot, a line or a bar, with its data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Plot {
    /// A line plot. It keeps its point labels when at least one of them is not empty.
    #[serde(rename_all = "camelCase")]
    Line {
        /// The plot title, sanitised, empty if absent.
        title: String,
        /// The colour of the line, from the theme's palette.
        stroke_fill: String,
        /// The stroke width, always 2.
        stroke_width: f64,
        /// The points, as `[x, y]` pairs.
        data: Vec<DataPoint>,
        /// The label of each point, `""` for a point without one. Absent when no point has a label.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        point_labels: Option<Vec<String>>,
    },
    /// A bar plot. Its labels are not kept by the DB.
    #[serde(rename_all = "camelCase")]
    Bar {
        /// The plot title, sanitised, empty if absent.
        title: String,
        /// The fill of the bars, from the theme's palette.
        fill: String,
        /// The points, as `[x, y]` pairs.
        data: Vec<DataPoint>,
    },
}

/// One point of a plot: the x value as the DB stores it, and the y value, `None` for a point the
/// data does not reach (a category without a value).
#[derive(Debug, Clone, PartialEq)]
pub struct DataPoint {
    /// The category, or the JavaScript string of the linear position.
    pub x: String,
    /// The value, or `None`, written as `null`.
    pub y: Option<f64>,
}

impl Serialize for DataPoint {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&self.x)?;
        match self.y {
            Some(value) => tuple.serialize_element(&js_number::Written::from(value))?,
            None => tuple.serialize_element(&None::<js_number::Written>)?,
        }
        tuple.end()
    }
}

impl<'de> Deserialize<'de> for DataPoint {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (x, y) = <(String, Option<js_number::Written>)>::deserialize(deserializer)?;
        Ok(Self {
            x,
            y: y.map(js_number::Written::into_f64),
        })
    }
}

impl XyChartDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`]: a vertical band axis with no
    /// categories, a y-axis from `Infinity` to `-Infinity`, and no plot.
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            orientation: "vertical".to_owned(),
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            x_axis: XAxis::Band {
                title: String::new(),
                categories: Vec::new(),
            },
            y_axis: YAxis::Linear {
                title: String::new(),
                min: f64::INFINITY,
                max: f64::NEG_INFINITY,
            },
            plots: Vec::new(),
        }
    }
}

impl Default for XyChartDiagram {
    fn default() -> Self {
        Self::new()
    }
}

/// A number as JSON: a number when it is finite, and the sentinel strings for the other values.
pub(crate) mod js_number {
    use super::{Deserialize, Deserializer, Serialize, Serializer};

    /// A number in its JSON form: a JSON number, or one of the sentinel strings.
    #[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
    #[serde(untagged)]
    pub(crate) enum Written {
        /// A finite number.
        Number(f64),
        /// `NaN`, `Infinity` or `-Infinity`.
        Sentinel(Sentinel),
    }

    /// The sentinel strings of the numbers JSON cannot hold.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum Sentinel {
        /// `"NaN"`.
        NaN,
        /// `"Infinity"`.
        Infinity,
        /// `"-Infinity"`.
        NegativeInfinity,
    }

    impl From<f64> for Written {
        fn from(value: f64) -> Self {
            if value.is_finite() {
                Self::Number(value)
            } else if value.is_nan() {
                Self::Sentinel(Sentinel::NaN)
            } else if value > 0.0 {
                Self::Sentinel(Sentinel::Infinity)
            } else {
                Self::Sentinel(Sentinel::NegativeInfinity)
            }
        }
    }

    impl Written {
        /// The number this JSON form stands for.
        pub(crate) fn into_f64(self) -> f64 {
            match self {
                Self::Number(value) => value,
                Self::Sentinel(Sentinel::NaN) => f64::NAN,
                Self::Sentinel(Sentinel::Infinity) => f64::INFINITY,
                Self::Sentinel(Sentinel::NegativeInfinity) => f64::NEG_INFINITY,
            }
        }
    }

    impl Serialize for Sentinel {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_str(match self {
                Self::NaN => "NaN",
                Self::Infinity => "Infinity",
                Self::NegativeInfinity => "-Infinity",
            })
        }
    }

    impl<'de> Deserialize<'de> for Sentinel {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let text = String::deserialize(deserializer)?;
            match text.as_str() {
                "NaN" => Ok(Self::NaN),
                "Infinity" => Ok(Self::Infinity),
                "-Infinity" => Ok(Self::NegativeInfinity),
                other => Err(serde::de::Error::custom(format!(
                    "expected a number or NaN, Infinity or -Infinity, found {other:?}"
                ))),
            }
        }
    }

    pub(crate) fn serialize<S: Serializer>(value: &f64, serializer: S) -> Result<S::Ok, S::Error> {
        Written::from(*value).serialize(serializer)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
        Written::deserialize(deserializer).map(Written::into_f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_value_is_null_and_a_non_finite_one_is_a_sentinel() {
        let point = DataPoint {
            x: "b".to_owned(),
            y: None,
        };
        assert_eq!(
            serde_json::to_value(&point).expect("serialises"),
            serde_json::json!(["b", null])
        );
        let point = DataPoint {
            x: "1".to_owned(),
            y: Some(f64::INFINITY),
        };
        assert_eq!(
            serde_json::to_value(&point).expect("serialises"),
            serde_json::json!(["1", "Infinity"])
        );
    }

    #[test]
    fn an_empty_model_has_an_inverted_y_range() {
        let json = serde_json::to_value(XyChartDiagram::new()).expect("serialises");
        assert_eq!(json["yAxis"]["min"], serde_json::json!("Infinity"));
        assert_eq!(json["yAxis"]["max"], serde_json::json!("-Infinity"));
        assert_eq!(json["xAxis"]["type"], serde_json::json!("band"));
    }

    #[test]
    fn the_sentinels_read_back() {
        let read: YAxis = serde_json::from_value(serde_json::json!({
            "type": "linear", "title": "", "min": "-Infinity", "max": 4.5
        }))
        .expect("reads");
        assert_eq!(
            read,
            YAxis::Linear {
                title: String::new(),
                min: f64::NEG_INFINITY,
                max: 4.5
            }
        );
    }
}
