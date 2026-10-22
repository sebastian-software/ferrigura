//! Port of `packages/mermaid/src/diagrams/xychart/parser/xychart.jison.spec.ts`. Test names follow the
//! spec's titles in snake_case: the title of a `describe` is joined in front of the title of its `it`
//! (`point_labels` and `accessibility`), and the quotation marks and punctuation of a title become
//! separators.
//!
//! The reference's `not.toThrow()` becomes `parse(…).expect("parses")`, and its `toThrow()` becomes
//! `parse(…).is_err()`. Each DB call the spec asserts becomes an assertion on the state the call
//! leaves in the model: the axis titles, the axis ranges and categories, the plots found by their
//! titles, and the accessible texts. The model's axis titles are trimmed, as the reference's
//! `textSanitizer` trims them (`xychartDb.ts` 73-76), so the spec's untrimmed titles are compared
//! trimmed. The DB reduces the blanks after a line break in `accDescr`, as `commonDb.ts` 23 does.
//!
//! The spec's `type: 'text'` objects have no counterpart in the model, so they are not asserted. A
//! bar plot keeps no point labels in the model, so the spec's bar labels are not asserted either.
//!
//! No test here renders the chart, so none is ignored.

use ferrigura_model::xychart::{DataPoint, Plot, XAxis, XyChartDiagram, YAxis};
use ferrigura_syntax::xychart::parse;

fn parses(text: &str) -> XyChartDiagram {
    parse(text).expect("parses")
}

/// The title of the x-axis, whichever kind it is.
fn x_title(diagram: &XyChartDiagram) -> &str {
    match &diagram.x_axis {
        XAxis::Band { title, .. } | XAxis::Linear { title, .. } => title,
    }
}

/// The title of the y-axis.
fn y_title(diagram: &XyChartDiagram) -> &str {
    let YAxis::Linear { title, .. } = &diagram.y_axis;
    title
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

/// A band x-axis with its title and categories.
fn band(title: &str, categories: &[&str]) -> XAxis {
    XAxis::Band {
        title: title.to_owned(),
        categories: strings(categories),
    }
}

/// A linear x-axis with its title and range.
fn linear(title: &str, min: f64, max: f64) -> XAxis {
    XAxis::Linear {
        title: title.to_owned(),
        min,
        max,
    }
}

/// A linear y-axis with its title and range.
fn y_axis(title: &str, min: f64, max: f64) -> YAxis {
    YAxis::Linear {
        title: title.to_owned(),
        min,
        max,
    }
}

/// The y values of the points of a plot, `None` where the data has no value.
fn y_values(data: &[DataPoint]) -> Vec<Option<f64>> {
    data.iter().map(|point| point.y).collect()
}

fn plot_title(plot: &Plot) -> &str {
    match plot {
        Plot::Line { title, .. } | Plot::Bar { title, .. } => title,
    }
}

/// The plot with this title.
fn find_plot<'a>(diagram: &'a XyChartDiagram, title: &str) -> &'a Plot {
    diagram
        .plots
        .iter()
        .find(|plot| plot_title(plot) == title)
        .unwrap_or_else(|| panic!("no plot titled {title:?} in {:?}", diagram.plots))
}

/// The y values and the point labels of the line with this title. The labels are `None` when the
/// line has none.
fn line(diagram: &XyChartDiagram, title: &str) -> (Vec<Option<f64>>, Option<Vec<String>>) {
    match find_plot(diagram, title) {
        Plot::Line {
            data, point_labels, ..
        } => (y_values(data), point_labels.clone()),
        other => panic!("expected a line titled {title:?}, found {other:?}"),
    }
}

/// The y values of the bar with this title.
fn bar(diagram: &XyChartDiagram, title: &str) -> Vec<Option<f64>> {
    match find_plot(diagram, title) {
        Plot::Bar { data, .. } => y_values(data),
        other => panic!("expected a bar titled {title:?}, found {other:?}"),
    }
}

#[test]
fn should_throw_error_if_xychart_text_is_not_there() {
    assert!(parse("xychart-1").is_err());
}

#[test]
fn should_not_throw_error_if_only_xychart_is_there() {
    parses("xychart");
}

/// The lexer's `"xychart-beta"` rule, which upstream keeps next to `"xychart"`.
#[test]
fn should_not_throw_error_if_only_xychart_beta_is_there() {
    parses("xychart-beta");
    assert!(parse("xychart-beta-1").is_err());
}

#[test]
fn parse_title_of_the_chart_within() {
    let diagram = parses("xychart \n title \"This is a title\"");
    assert_eq!(diagram.title, "This is a title");
}

#[test]
fn parse_title_of_the_chart_without() {
    let diagram = parses("xychart \n title oneLinertitle");
    assert_eq!(diagram.title, "oneLinertitle");
}

#[test]
fn parse_chart_orientation() {
    let diagram = parses("xychart vertical");
    assert_eq!(diagram.orientation, "vertical");
}

#[test]
fn parse_chart_orientation_with_spaces() {
    let diagram = parses("xychart        horizontal        ");
    assert_eq!(diagram.orientation, "horizontal");
    assert!(parse("xychart abc").is_err());
}

#[test]
fn parse_x_axis() {
    let diagram = parses("xychart \nx-axis xAxisName\n");
    assert_eq!(x_title(&diagram), "xAxisName");
}

#[test]
fn parse_x_axis_with_axis_name_without() {
    let diagram = parses("xychart \nx-axis        xAxisName     \n");
    assert_eq!(x_title(&diagram), "xAxisName");
}

#[test]
fn parse_x_axis_with_axis_name_with() {
    let diagram = parses("xychart \n    x-axis \"xAxisName has space\"\n");
    assert_eq!(x_title(&diagram), "xAxisName has space");
}

#[test]
fn parse_x_axis_with_axis_name_with_with_spaces() {
    let diagram = parses("xychart \n   x-axis    \"  xAxisName has space   \"         \n");
    assert_eq!(x_title(&diagram), "xAxisName has space");
}

#[test]
fn parse_x_axis_with_axis_name_and_range_data() {
    let diagram = parses("xychart \nx-axis xAxisName    45.5   -->   33   \n");
    assert_eq!(diagram.x_axis, linear("xAxisName", 45.5, 33.0));
}

#[test]
fn parse_x_axis_throw_error_for_invalid_range_data() {
    assert!(parse("xychart \nx-axis xAxisName    aaa   -->   33   \n").is_err());
}

#[test]
fn parse_x_axis_with_axis_name_and_range_data_with_only_decimal_part() {
    let diagram = parses("xychart \nx-axis xAxisName    45.5   -->   .34   \n");
    assert_eq!(diagram.x_axis, linear("xAxisName", 45.5, 0.34));
}

#[test]
fn parse_x_axis_without_axis_name_and_range_data() {
    let diagram = parses("xychart \nx-axis   45.5   -->   1.34   \n");
    assert_eq!(diagram.x_axis, linear("", 45.5, 1.34));
}

#[test]
fn parse_x_axis_with_axis_name_and_category_data() {
    let diagram = parses("xychart \nx-axis xAxisName    [  \"cat1\"  ,   cat2a  ]   \n   ");
    assert_eq!(diagram.x_axis, band("xAxisName", &["cat1", "cat2a"]));
}

#[test]
fn parse_x_axis_without_axis_name_and_category_data() {
    let diagram = parses("xychart \nx-axis    [  \"cat1\"  ,   cat2a  ]   \n   ");
    assert_eq!(diagram.x_axis, band("", &["cat1", "cat2a"]));
}

#[test]
fn parse_x_axis_throw_error_if_unbalanced_bracket() {
    assert!(parse("xychart \nx-axis xAxisName    [  \"cat1\"  [   cat2a  ]   \n   ").is_err());
    assert!(parse("xychart \nx-axis xAxisName    [  \"cat1\"  ,   cat2a ] ]   \n   ").is_err());
}

#[test]
fn parse_x_axis_complete_variant_1() {
    let diagram =
        parses("xychart \n x-axis \"this is x axis\" [category1, \"category 2\", category3]\n");
    assert_eq!(
        diagram.x_axis,
        band("this is x axis", &["category1", "category 2", "category3"])
    );
}

#[test]
fn parse_x_axis_complete_variant_2() {
    let diagram =
        parses("xychart \nx-axis xAxisName    [  \"cat1  with space\"  ,   cat2 , cat3]   \n   ");
    assert_eq!(
        diagram.x_axis,
        band("xAxisName", &["cat1  with space", "cat2", "cat3"])
    );
}

#[test]
fn parse_x_axis_complete_variant_3() {
    let diagram = parses(
        "xychart \nx-axis xAxisName    [  \"cat1  with space\"  ,   cat2 asdf , cat3]   \n   ",
    );
    assert_eq!(
        diagram.x_axis,
        band("xAxisName", &["cat1  with space", "cat2asdf", "cat3"])
    );
}

#[test]
fn parse_y_axis_with_axis_name() {
    let diagram = parses("xychart \ny-axis yAxisName\n");
    assert_eq!(y_title(&diagram), "yAxisName");
}

#[test]
fn parse_y_axis_with_axis_name_with_spaces() {
    let diagram = parses("xychart \ny-axis        yAxisName     \n");
    assert_eq!(y_title(&diagram), "yAxisName");
}

#[test]
fn parse_y_axis_with_axis_name_with() {
    let diagram = parses("xychart \n    y-axis \"yAxisName has space\"\n");
    assert_eq!(y_title(&diagram), "yAxisName has space");
}

#[test]
fn parse_y_axis_with_axis_name_with_and_spaces() {
    let diagram = parses("xychart \n   y-axis    \"  yAxisName has space   \"         \n");
    assert_eq!(y_title(&diagram), "yAxisName has space");
}

#[test]
fn parse_y_axis_with_axis_name_with_range_data() {
    let diagram = parses("xychart \ny-axis yAxisName    45.5   -->   33   \n");
    assert_eq!(diagram.y_axis, y_axis("yAxisName", 45.5, 33.0));
}

#[test]
fn parse_y_axis_without_axis_name_with_range_data() {
    let diagram = parses("xychart \ny-axis    45.5   -->   33   \n");
    assert_eq!(diagram.y_axis, y_axis("", 45.5, 33.0));
}

#[test]
fn parse_y_axis_with_axis_name_with_range_data_with_only_decimal_part() {
    let diagram = parses("xychart \ny-axis yAxisName    45.5   -->   .33   \n");
    assert_eq!(diagram.y_axis, y_axis("yAxisName", 45.5, 0.33));
}

#[test]
fn parse_y_axis_throw_error_for_invalid_number_in_range_data() {
    assert!(parse("xychart \ny-axis yAxisName    45.5   -->   abc   \n").is_err());
}

#[test]
fn parse_y_axis_throws_error_if_range_data_is_passed() {
    assert!(parse("xychart \ny-axis yAxisName    [ 45.3,   33 ]   \n").is_err());
}

#[test]
fn parse_both_axis_at_once() {
    let diagram = parses("xychart\nx-axis xAxisName\ny-axis yAxisName\n");
    assert_eq!(x_title(&diagram), "xAxisName");
    assert_eq!(y_title(&diagram), "yAxisName");
}

#[test]
fn parse_line_data() {
    let diagram =
        parses("xychart\nx-axis xAxisName\ny-axis yAxisName\n line lineTitle [23, 45, 56.6]");
    let (ys, labels) = line(&diagram, "lineTitle");
    assert_eq!(ys, [Some(23.0), Some(45.0), Some(56.6)]);
    assert_eq!(labels, None);
    assert_eq!(x_title(&diagram), "xAxisName");
    assert_eq!(y_title(&diagram), "yAxisName");
}

#[test]
fn parse_line_data_with_spaces_and_plus_minus_symbols() {
    let diagram = parses(
        "xychart\nx-axis xAxisName\ny-axis yAxisName\n line \"lineTitle with space\"   [  +23 , -45  , 56.6 ]   ",
    );
    let (ys, labels) = line(&diagram, "lineTitle with space");
    assert_eq!(ys, [Some(23.0), Some(-45.0), Some(56.6)]);
    assert_eq!(labels, None);
    assert_eq!(y_title(&diagram), "yAxisName");
    assert_eq!(x_title(&diagram), "xAxisName");
}

#[test]
fn parse_line_data_without_title() {
    let diagram =
        parses("xychart\nx-axis xAxisName\ny-axis yAxisName\n line [  +23 , -45  , 56.6 , .33]   ");
    let (ys, labels) = line(&diagram, "");
    assert_eq!(ys, [Some(23.0), Some(-45.0), Some(56.6), Some(0.33)]);
    assert_eq!(labels, None);
    assert_eq!(y_title(&diagram), "yAxisName");
    assert_eq!(x_title(&diagram), "xAxisName");
}

#[test]
fn parse_line_data_throws_error_unbalanced_brackets() {
    assert!(parse(
        "xychart\nx-axis xAxisName\ny-axis yAxisName\n line \"lineTitle with space\"   [  +23 [ -45  , 56.6 ]   "
    )
    .is_err());
    assert!(parse(
        "xychart\nx-axis xAxisName\ny-axis yAxisName\n line \"lineTitle with space\"   [  +23 , -45  ] 56.6 ]   "
    )
    .is_err());
}

#[test]
fn parse_line_data_throws_error_if_data_is_not_provided() {
    assert!(
        parse("xychart\nx-axis xAxisName\ny-axis yAxisName\n line \"lineTitle with space\"   ")
            .is_err()
    );
}

#[test]
fn parse_line_data_throws_error_if_data_is_empty() {
    assert!(
        parse("xychart\nx-axis xAxisName\ny-axis yAxisName\n line \"lineTitle with space\"  [ ] ")
            .is_err()
    );
}

#[test]
fn parse_line_data_throws_error_if_comma_is_not_in_proper() {
    assert!(parse(
        "xychart\nx-axis xAxisName\ny-axis yAxisName\n line \"lineTitle with space\"   [  +23 ,  , -45  , 56.6 ]   "
    )
    .is_err());
}

#[test]
fn parse_line_data_throws_error_if_not_number() {
    assert!(parse(
        "xychart\nx-axis xAxisName\ny-axis yAxisName\n line \"lineTitle with space\"   [  +23 , -4aa5  , 56.6 ]   "
    )
    .is_err());
}

#[test]
fn parse_bar_data() {
    let diagram =
        parses("xychart\nx-axis xAxisName\ny-axis yAxisName\n bar barTitle [23, 45, 56.6, .22]");
    assert_eq!(
        bar(&diagram, "barTitle"),
        [Some(23.0), Some(45.0), Some(56.6), Some(0.22)]
    );
    assert_eq!(y_title(&diagram), "yAxisName");
    assert_eq!(x_title(&diagram), "xAxisName");
}

#[test]
fn parse_bar_data_spaces_and_plus_minus_symbol() {
    let diagram = parses(
        "xychart\nx-axis xAxisName\ny-axis yAxisName\n bar \"barTitle with space\"   [  +23 , -45  , 56.6 ]   ",
    );
    assert_eq!(
        bar(&diagram, "barTitle with space"),
        [Some(23.0), Some(-45.0), Some(56.6)]
    );
    assert_eq!(y_title(&diagram), "yAxisName");
    assert_eq!(x_title(&diagram), "xAxisName");
}

#[test]
fn parse_bar_data_without_plot_title() {
    let diagram =
        parses("xychart\nx-axis xAxisName\ny-axis yAxisName\n bar   [  +23 , -45  , 56.6 ]   ");
    assert_eq!(bar(&diagram, ""), [Some(23.0), Some(-45.0), Some(56.6)]);
    assert_eq!(y_title(&diagram), "yAxisName");
    assert_eq!(x_title(&diagram), "xAxisName");
}

#[test]
fn parse_bar_should_throw_for_unbalanced_brackets() {
    assert!(parse(
        "xychart\nx-axis xAxisName\ny-axis yAxisName\n bar \"barTitle with space\"   [  +23 [ -45  , 56.6 ]   "
    )
    .is_err());
    assert!(parse(
        "xychart\nx-axis xAxisName\ny-axis yAxisName\n bar \"barTitle with space\"   [  +23 , -45  ] 56.6 ]   "
    )
    .is_err());
}

#[test]
fn parse_bar_should_throw_error_if_data_is_not_provided() {
    assert!(
        parse("xychart\nx-axis xAxisName\ny-axis yAxisName\n bar \"barTitle with space\"    ")
            .is_err()
    );
}

#[test]
fn parse_bar_should_throw_error_if_data_is_empty() {
    assert!(
        parse(
            "xychart\nx-axis xAxisName\ny-axis yAxisName\n bar \"barTitle with space\"   [   ]   "
        )
        .is_err()
    );
}

#[test]
fn parse_bar_should_throw_error_if_comma_is_not_proper() {
    assert!(parse(
        "xychart\nx-axis xAxisName\ny-axis yAxisName\n bar \"barTitle with space\"   [  +23 , , -45  , 56.6 ]   "
    )
    .is_err());
}

#[test]
fn parse_bar_should_throw_error_if_number_is_not_passed() {
    assert!(parse(
        "xychart\nx-axis xAxisName\ny-axis yAxisName\n bar \"barTitle with space\"   [  +23 , -4aa5  , 56.6 ]   "
    )
    .is_err());
}

#[test]
fn parse_multiple_bar_and_line_variant_1() {
    let diagram = parses(
        "xychart\nx-axis xAxisName\ny-axis yAxisName\n bar barTitle1 [23, 45, 56.6] \n line lineTitle1 [11, 45.5, 67, 23] \n bar barTitle2 [13, 42, 56.89] \n line lineTitle2 [45, 99, 012]",
    );
    assert_eq!(y_title(&diagram), "yAxisName");
    assert_eq!(x_title(&diagram), "xAxisName");
    assert_eq!(
        bar(&diagram, "barTitle1"),
        [Some(23.0), Some(45.0), Some(56.6)]
    );
    assert_eq!(
        bar(&diagram, "barTitle2"),
        [Some(13.0), Some(42.0), Some(56.89)]
    );
    assert_eq!(
        line(&diagram, "lineTitle1"),
        (vec![Some(11.0), Some(45.5), Some(67.0), Some(23.0)], None)
    );
    assert_eq!(
        line(&diagram, "lineTitle2"),
        (vec![Some(45.0), Some(99.0), Some(12.0)], None)
    );
}

#[test]
fn parse_multiple_bar_and_line_variant_2() {
    let diagram = parses(
        r#"
    xychart horizontal
    title "Basic xychart"
    x-axis "this is x axis" [category1, "category 2", category3]
    y-axis yaxisText 10 --> 150
 bar barTitle1 [23, 45, 56.6]
 line lineTitle1 [11, 45.5, 67, 23]
 bar barTitle2 [13, 42, 56.89]
    line lineTitle2 [45, 99, 012]"#,
    );
    assert_eq!(diagram.y_axis, y_axis("yaxisText", 10.0, 150.0));
    assert_eq!(
        diagram.x_axis,
        band("this is x axis", &["category1", "category 2", "category3"])
    );
    assert_eq!(
        bar(&diagram, "barTitle1"),
        [Some(23.0), Some(45.0), Some(56.6)]
    );
    assert_eq!(
        bar(&diagram, "barTitle2"),
        [Some(13.0), Some(42.0), Some(56.89)]
    );
    // The spec's call carries four values, but the band axis has three categories, and the DB
    // keeps only as many values as there are categories (`xychartDb.ts` 139-140). The model holds
    // the three values, so the fourth (23) is not asserted.
    assert_eq!(
        line(&diagram, "lineTitle1"),
        (vec![Some(11.0), Some(45.5), Some(67.0)], None)
    );
    assert_eq!(
        line(&diagram, "lineTitle2"),
        (vec![Some(45.0), Some(99.0), Some(12.0)], None)
    );
}

#[test]
fn point_labels_parse_line_data_with_point_labels() {
    let diagram = parses(
        "xychart\nx-axis xAxisName\ny-axis yAxisName\n line [10 \"First\", 20 \"Second\", 30 \"Third\"]",
    );
    let (ys, labels) = line(&diagram, "");
    assert_eq!(ys, [Some(10.0), Some(20.0), Some(30.0)]);
    assert_eq!(labels, Some(strings(&["First", "Second", "Third"])));
}

#[test]
fn point_labels_parse_line_data_with_mixed_labels_and_no_labels() {
    let diagram =
        parses("xychart\nx-axis xAxisName\ny-axis yAxisName\n line [10 \"Start\", 20, 30 \"End\"]");
    let (ys, labels) = line(&diagram, "");
    assert_eq!(ys, [Some(10.0), Some(20.0), Some(30.0)]);
    assert_eq!(labels, Some(strings(&["Start", "", "End"])));
}

#[test]
fn point_labels_parse_line_data_with_labels_containing_spaces() {
    let diagram = parses(
        "xychart\nx-axis xAxisName\ny-axis yAxisName\n line [10 \"Hello World\", 20 \"Another Label\"]",
    );
    let (ys, labels) = line(&diagram, "");
    assert_eq!(ys, [Some(10.0), Some(20.0)]);
    assert_eq!(labels, Some(strings(&["Hello World", "Another Label"])));
}

#[test]
fn point_labels_parse_line_data_with_title_and_point_labels() {
    let diagram = parses(
        "xychart\nx-axis xAxisName\ny-axis yAxisName\n line \"My Line\" [10 \"A\", 20 \"B\"]",
    );
    let (ys, labels) = line(&diagram, "My Line");
    assert_eq!(ys, [Some(10.0), Some(20.0)]);
    assert_eq!(labels, Some(strings(&["A", "B"])));
}

#[test]
fn point_labels_parse_bar_data_with_point_labels() {
    // A bar keeps no point labels in the model, so only the values are asserted.
    let diagram =
        parses("xychart\nx-axis xAxisName\ny-axis yAxisName\n bar [10 \"A\", 20 \"B\", 30 \"C\"]");
    assert_eq!(bar(&diagram, ""), [Some(10.0), Some(20.0), Some(30.0)]);
}

#[test]
fn point_labels_parse_line_data_with_decimal_values_and_labels() {
    let diagram = parses(
        "xychart\nx-axis xAxisName\ny-axis yAxisName\n line [3.8 \"Phi-3\", 7 \"Mistral\", 540 \"PaLM\"]",
    );
    let (ys, labels) = line(&diagram, "");
    assert_eq!(ys, [Some(3.8), Some(7.0), Some(540.0)]);
    assert_eq!(labels, Some(strings(&["Phi-3", "Mistral", "PaLM"])));
}

#[test]
fn accessibility_should_handle_acc_title() {
    let diagram = parses("xychart\naccTitle: Accessible Title\nx-axis [Q1, Q2]\nline [1, 2]");
    assert_eq!(diagram.acc_title, "Accessible Title");
}

#[test]
fn accessibility_should_handle_single_line_acc_descr() {
    let diagram = parses("xychart\naccDescr: This is a description\nx-axis [Q1, Q2]\nline [1, 2]");
    assert_eq!(diagram.acc_descr, "This is a description");
}

#[test]
fn accessibility_should_handle_multiline_acc_descr() {
    let diagram = parses(
        "xychart\naccDescr {\n  This is a multiline\n  description\n}\nx-axis [Q1, Q2]\nline [1, 2]",
    );
    // The blanks after a line break are reduced to none, as `commonDb.ts` 23 does.
    assert_eq!(diagram.acc_descr, "This is a multiline\ndescription");
}

#[test]
fn accessibility_should_handle_both_acc_title_and_acc_descr() {
    let diagram = parses(
        "xychart\naccTitle: Sales Overview\naccDescr: Revenue report for Q1-Q4\ntitle \"Revenue Report\"\nx-axis [Q1, Q2, Q3, Q4]\nline [45, 67, 89, 55]",
    );
    assert_eq!(diagram.acc_title, "Sales Overview");
    assert_eq!(diagram.acc_descr, "Revenue report for Q1-Q4");
    assert_eq!(diagram.title, "Revenue Report");
}
