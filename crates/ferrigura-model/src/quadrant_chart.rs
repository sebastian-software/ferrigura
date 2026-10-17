//! Canonical model of a quadrant chart diagram: the DB state after the parse, before the chart
//! builder lays it out (`docs-port/quadrant/PLAN.md`, section 7).
//!
//! Texts are what the DB holds: sanitised where the DB sanitises, with the entity placeholders
//! where the DB keeps them. A coordinate is the number the scale reads, and a coordinate that is
//! not finite is the sentinel string `"NaN"`, as `docs-port/canonical-model.md` writes it.

use serde::{Deserialize, Serialize};

use crate::CANONICAL_VERSION;

/// A parsed quadrant chart: the header texts, the quadrant and axis texts, the points in the order
/// the DB holds them (the last source line first), and the classes in the order they were first
/// defined.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuadrantChartDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// Diagram title, sanitised. Empty if none is given.
    pub title: String,
    /// Accessible title, empty if absent.
    pub acc_title: String,
    /// Accessible description, empty if absent.
    pub acc_descr: String,
    /// The text of the first quadrant (top right).
    pub quadrant1_text: String,
    /// The text of the second quadrant (top left).
    pub quadrant2_text: String,
    /// The text of the third quadrant (bottom left).
    pub quadrant3_text: String,
    /// The text of the fourth quadrant (bottom right).
    pub quadrant4_text: String,
    /// The left text of the x-axis, or the only text.
    pub x_axis_left_text: String,
    /// The right text of the x-axis.
    pub x_axis_right_text: String,
    /// The bottom text of the y-axis, or the only text.
    pub y_axis_bottom_text: String,
    /// The top text of the y-axis.
    pub y_axis_top_text: String,
    /// The points, in the order the DB holds them.
    pub points: Vec<QuadrantPoint>,
    /// The classes, each with its name and styles, in the order of their first definition. A
    /// later definition of a name replaces its styles in place.
    pub classes: Vec<(String, QuadrantStyle)>,
}

/// A point of the chart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuadrantPoint {
    /// The point's text, sanitised, trimmed.
    pub text: String,
    /// The class named with `:::`, empty when none is.
    pub class_name: String,
    /// The x coordinate, read as JavaScript's `Number()` reads the text.
    #[serde(with = "coordinate")]
    pub x: f64,
    /// The y coordinate, read as JavaScript's `Number()` reads the text.
    #[serde(with = "coordinate")]
    pub y: f64,
    /// The styles the point or its class sets; only those that were set appear.
    #[serde(flatten)]
    pub style: QuadrantStyle,
}

/// The styles of a point or a class. A field is absent when the text does not set it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuadrantStyle {
    /// The radius in pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<f64>,
    /// The fill colour, as written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// The stroke colour, as written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_color: Option<String>,
    /// The stroke width, as written (`2px`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_width: Option<String>,
}

impl QuadrantChartDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            quadrant1_text: String::new(),
            quadrant2_text: String::new(),
            quadrant3_text: String::new(),
            quadrant4_text: String::new(),
            x_axis_left_text: String::new(),
            x_axis_right_text: String::new(),
            y_axis_bottom_text: String::new(),
            y_axis_top_text: String::new(),
            points: Vec::new(),
            classes: Vec::new(),
        }
    }
}

impl Default for QuadrantChartDiagram {
    fn default() -> Self {
        Self::new()
    }
}

/// A coordinate as JSON: a number when it is finite, and the sentinel strings for the other values.
mod coordinate {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: f64) -> QuadrantPoint {
        QuadrantPoint {
            text: "p".to_owned(),
            class_name: String::new(),
            x,
            y: 0.5,
            style: QuadrantStyle::default(),
        }
    }

    #[test]
    fn a_finite_coordinate_is_a_json_number() {
        let json = serde_json::to_value(point(0.25)).expect("serialises");
        assert_eq!(json["x"], serde_json::json!(0.25));
    }

    #[test]
    fn a_non_finite_coordinate_is_a_sentinel_string() {
        let json = serde_json::to_value(point(f64::NAN)).expect("serialises");
        assert_eq!(json["x"], serde_json::json!("NaN"));
    }

    #[test]
    fn unset_styles_are_absent() {
        let json = serde_json::to_value(point(0.5)).expect("serialises");
        let keys: Vec<&str> = json
            .as_object()
            .expect("object")
            .keys()
            .map(String::as_str)
            .collect();
        assert!(!keys.iter().any(|key| *key == "radius" || *key == "color"));
    }
}
