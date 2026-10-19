//! The xychart renderer's output for the reference's quirks (`docs-port/xychart/NOTES.md`, 6.8): the
//! bar labels from the first plot, the colour of the data labels, the misspelt class of the left axis
//! line, the bars without a value or below the axis, a plot before a band axis, and the measurement
//! on the screen. The geometry against the reference is compared by `pnpm port:geometry xychart`;
//! these tests pin the structure the comparison relies on.

use ferrigura_model::config::{defaults, typed};
use ferrigura_model::theme::{Theme, ThemeTokens};
use ferrigura_model::value::Value;
use ferrigura_render::RenderError;
use ferrigura_render::context::{Context, Globals, ThemeView};
use ferrigura_render::xychart::render;
use ferrigura_syntax::xychart::parse;

/// Renders a chart with the given `xyChart` settings and the redux-color theme (or dark).
fn draw_with(text: &str, xy_chart: &[(&str, Value)], dark: bool) -> Result<String, RenderError> {
    let diagram = parse(text).expect("the diagram parses");
    let mut value = defaults();
    for (key, setting) in xy_chart {
        value
            .get_mut("xyChart")
            .and_then(Value::as_object_mut)
            .expect("the xyChart section")
            .insert((*key).to_owned(), setting.clone());
    }
    let config = typed(&value).expect("the configuration");
    let variables = if dark {
        Theme::ReduxDarkColor.variables(None)
    } else {
        Theme::ReduxColor.variables(None)
    };
    let tokens = ThemeTokens::from_value(&variables);
    let globals = Globals::of(&config);
    render(&Context {
        diagram: &diagram,
        section: &config.xy_chart,
        globals: &globals,
        theme: ThemeView {
            light: &tokens,
            dark: None,
        },
        id: "chart",
    })
}

fn draw(text: &str) -> String {
    draw_with(text, &[], false).expect("the diagram renders")
}

fn labels_on() -> Vec<(&'static str, Value)> {
    vec![("showDataLabel", Value::from(serde_json::json!(true)))]
}

/// The elements of the given tag that carry the given attribute text, as their markup. A `text`
/// ends at its closing tag; the other elements here are written with `/>`.
fn elements<'a>(svg: &'a str, tag: &str, needle: &str) -> Vec<&'a str> {
    let open = format!("<{tag} ");
    svg.match_indices(&open)
        .map(|(start, _)| {
            let end = if tag == "text" {
                let close = format!("</{tag}>");
                start + svg[start..].find(&close).expect("the text ends") + close.len()
            } else {
                start + svg[start..].find("/>").expect("the element ends") + 2
            };
            &svg[start..end]
        })
        .filter(|element| element.contains(needle))
        .collect()
}

/// The first path of the group of the given class (`line-plot-0`, for the line of the first plot).
fn first_path_of_group<'a>(svg: &'a str, class: &str) -> &'a str {
    let group = svg
        .split(&format!("<g class=\"{class}\">"))
        .nth(1)
        .expect("the group");
    let start = group.find("<path ").expect("a path");
    let end = start + group[start..].find("/>").expect("the path ends") + 2;
    &group[start..end]
}

/// The x coordinates of the first subpath of a path's `d`, as the numbers written after `M` and `L`.
fn path_xs(d: &str) -> Vec<f64> {
    d.split(['M', 'L'])
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.split(',').next()?.trim().parse::<f64>().ok())
        .collect()
}

#[test]
fn the_bar_labels_take_their_numbers_from_the_first_plot() {
    // `labelData` is `plots[0]`'s values: the line's 1 and 2 label the bars of 100 and 200.
    let svg = draw_with(
        "xychart\n  line [1, 2]\n  bar [100, 200]\n",
        &labels_on(),
        false,
    )
    .expect("the diagram renders");
    let labels = elements(&svg, "text", "dominant-baseline=\"hanging\"");
    assert_eq!(labels.len(), 2, "{svg}");
    assert!(labels[0].contains(">1</text>"), "{svg}");
    assert!(labels[1].contains(">2</text>"), "{svg}");
}

#[test]
fn a_first_plot_shorter_than_a_bar_fails_the_render() {
    let result = draw_with("xychart\n  line [1]\n  bar [5, 6]\n", &labels_on(), false);
    assert!(matches!(result, Err(RenderError::Layout(_))), "{result:?}");
}

#[test]
fn the_data_labels_are_the_classic_text_colour_in_both_themes() {
    for dark in [false, true] {
        let svg = draw_with("xychart\n  line [1, 2]\n  bar [3, 4]\n", &labels_on(), dark)
            .expect("the diagram renders");
        assert!(svg.contains("fill=\"#131300\""), "dark: {dark}\n{svg}");
    }
}

#[test]
fn the_left_axis_line_has_the_misspelt_class() {
    let svg = draw("xychart\n  line [1, 2]\n");
    assert!(svg.contains("<g class=\"axisl-line\">"), "{svg}");
    assert!(
        svg.contains("<g class=\"axis-line\">") || !svg.contains("bottom-axis"),
        "{svg}"
    );
}

#[test]
fn a_bar_without_a_value_has_no_y_and_a_nan_height() {
    // The second category has no value: the bar is written with no `y` and `height="NaN"`.
    let svg = draw("xychart\n  x-axis [a, b, c]\n  bar [1, 2]\n");
    let bars = elements(&svg, "rect", "height=\"NaN\"");
    assert_eq!(bars.len(), 1, "{svg}");
    assert!(!bars[0].contains(" y=\""), "{}", bars[0]);
}

#[test]
fn a_horizontal_bar_below_the_axis_minimum_has_a_negative_width() {
    let svg = draw("xychart horizontal\n  y-axis 0 --> 10\n  bar [-5, 3]\n");
    let bars = elements(&svg, "rect", "stroke-width=\"0\"");
    assert_eq!(bars.len(), 2, "{svg}");
    assert!(bars[0].contains("width=\"-"), "{}", bars[0]);
}

#[test]
fn a_plot_before_a_band_axis_collapses_onto_its_first_category() {
    // The line's x values are linear positions, which the band axis does not know: all of them sit
    // at the start of the range.
    let svg = draw("xychart\n  line [1, 2, 3]\n  x-axis [a, b, c]\n");
    let path = first_path_of_group(&svg, "line-plot-0");
    let d = path
        .split("d=\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("a path");
    let xs = path_xs(d);
    assert_eq!(xs.len(), 3, "{d}");
    assert!(xs.windows(2).all(|pair| pair[0] == pair[1]), "{d}");
}

#[test]
fn duplicate_categories_both_sit_at_the_first_place() {
    let svg = draw("xychart\n  x-axis [a, a, b]\n  bar [1, 2, 3]\n");
    let bars = elements(&svg, "rect", "stroke-width=\"0\"");
    assert_eq!(bars.len(), 3, "{svg}");
    let x = |bar: &str| {
        bar.split("x=\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .expect("an x")
            .to_owned()
    };
    assert_eq!(x(bars[0]), x(bars[1]), "{svg}");
    assert_ne!(x(bars[0]), x(bars[2]), "{svg}");
}

#[test]
fn a_legend_marker_takes_the_data_labels_of_its_bar() {
    // The reference's `rect` case labels every rectangle of a shape, legend markers included.
    let svg = draw_with(
        "xychart\n  bar \"Sales\" [1, 2]\n  line \"Cost\" [3, 4]\n",
        &labels_on(),
        false,
    )
    .expect("the diagram renders");
    assert!(svg.contains("<g class=\"markers\">"), "{svg}");
    let markers = svg
        .split("<g class=\"markers\">")
        .nth(1)
        .expect("the markers");
    assert!(markers.contains("dominant-baseline=\"hanging\""), "{svg}");
}

#[test]
fn a_chart_without_a_plot_is_an_error() {
    let result = draw_with("xychart\n  title \"Only\"\n", &[], false);
    assert!(matches!(result, Err(RenderError::Layout(_))), "{result:?}");
}

#[test]
fn an_entity_placeholder_in_the_title_reads_as_an_entity() {
    // `#quot;` is `&quot;` on the page, which the browser reads as a quotation mark.
    let svg = draw("xychart\n  title \"#quot;hi#quot;\"\n  line [1]\n");
    assert!(svg.contains(">&quot;hi&quot;</text>"), "{svg}");
}

#[test]
fn the_root_is_as_wide_as_its_container_allows() {
    let svg = draw("xychart\n  line [1, 2]\n");
    assert!(
        svg.contains("width=\"100%\" style=\"max-width: 700px;\""),
        "{svg}"
    );
    // The empty groups the reference writes before `main` and last are written as empty elements.
    assert!(
        svg.ends_with("<g class=\"mermaid-tmp-group\"/></svg>"),
        "{svg}"
    );
}
