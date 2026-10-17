//! Port of `packages/mermaid/src/diagrams/xychart/xychartDb.spec.ts`. Test names follow the spec's
//! titles in snake_case, with the title of its nested `describe` joined in front of the title of its
//! `it`.
//!
//! The DB has no public surface, so each test drives `parse` with the text that makes the spec's DB
//! calls: `setXAxisBand` is an `x-axis [...]` line, and `setLineData` and `setBarData` are `line` and
//! `bar` lines. The spec reads `getXYChartData()`, whose plots and y-axis are the model's `plots` and
//! `y_axis`. The spec's `beforeEach` and `afterEach` `clear()` have no counterpart: every parse starts
//! from an empty DB, so every test starts clean.
//!
//! The spec's `yAxis.type === 'linear'` guard is always true for the model, whose y-axis is always
//! linear, so its assertion is made without the guard.
//!
//! No test renders the chart, so none is ignored.

use ferrigura_model::xychart::{DataPoint, Plot, XyChartDiagram, YAxis};
use ferrigura_syntax::xychart::parse;

fn parses(text: &str) -> XyChartDiagram {
    parse(text).expect("parses")
}

/// A data point as the DB stores it: the category and the value, `None` where the data has none.
fn point(x: &str, y: Option<f64>) -> DataPoint {
    DataPoint { x: x.to_owned(), y }
}

/// The points of a plot, whichever kind it is.
fn points(plot: &Plot) -> &[DataPoint] {
    match plot {
        Plot::Line { data, .. } | Plot::Bar { data, .. } => data.as_slice(),
    }
}

/// The kind and the title of a plot: `line` or `bar`.
fn kind_and_title(plot: &Plot) -> (&'static str, &str) {
    match plot {
        Plot::Line { title, .. } => ("line", title.as_str()),
        Plot::Bar { title, .. } => ("bar", title.as_str()),
    }
}

#[test]
fn preserves_sanitized_line_and_bar_titles_for_legends() {
    let diagram = parses(
        "xychart\nx-axis [\"90d\", \"60d\"]\nline \" avg \" [48.1, 41.5]\nbar \" p95 \" [112.2, 75.3]",
    );
    assert_eq!(
        diagram.plots.iter().map(kind_and_title).collect::<Vec<_>>(),
        [("line", "avg"), ("bar", "p95")]
    );
}

#[test]
fn data_length_validation_with_band_axis_should_truncate_bar_data_when_it_exceeds_the_number_of_x_axis_categories()
 {
    // Three categories, five values: the last two values are dropped.
    let diagram = parses("xychart\nx-axis [cat1, cat2, cat3]\nbar testBar [10, 20, 30, 40, 50]");
    assert_eq!(diagram.plots.len(), 1);
    assert_eq!(
        points(&diagram.plots[0]),
        [
            point("cat1", Some(10.0)),
            point("cat2", Some(20.0)),
            point("cat3", Some(30.0)),
        ]
    );
}

#[test]
fn data_length_validation_with_band_axis_should_truncate_line_data_when_it_exceeds_the_number_of_x_axis_categories()
 {
    // Two categories, four values: the last two values are dropped.
    let diagram = parses("xychart\nx-axis [A, B]\nline testLine [100, 200, 300, 400]");
    assert_eq!(diagram.plots.len(), 1);
    assert_eq!(
        points(&diagram.plots[0]),
        [point("A", Some(100.0)), point("B", Some(200.0))]
    );
}

#[test]
fn data_length_validation_with_band_axis_should_not_affect_data_when_data_length_matches_category_count()
 {
    let diagram = parses("xychart\nx-axis [X, Y, Z]\nbar exact [5, 10, 15]");
    assert_eq!(diagram.plots.len(), 1);
    assert_eq!(
        points(&diagram.plots[0]),
        [
            point("X", Some(5.0)),
            point("Y", Some(10.0)),
            point("Z", Some(15.0)),
        ]
    );
}

#[test]
fn data_length_validation_with_band_axis_should_not_affect_data_when_data_length_is_less_than_category_count()
 {
    // Each category gets a point, and the categories without a value get none (`undefined` in the
    // reference, `None` here).
    let diagram = parses("xychart\nx-axis [A, B, C, D]\nbar short [1, 2]");
    assert_eq!(diagram.plots.len(), 1);
    assert_eq!(
        points(&diagram.plots[0]),
        [
            point("A", Some(1.0)),
            point("B", Some(2.0)),
            point("C", None),
            point("D", None),
        ]
    );
}

#[test]
fn data_length_validation_with_band_axis_should_compute_y_axis_range_only_from_visible_truncated_data()
 {
    // The value 999 belongs to a point that is dropped, so it does not widen the range.
    let diagram = parses("xychart\nx-axis [Q1, Q2]\nbar sales [10, 50, 999, 800]");
    let YAxis::Linear { max, .. } = &diagram.y_axis;
    assert_eq!(*max, 50.0);
}
