//! The chart builder of `quadrantBuilder.ts`: the layout of the chart as numbers and texts, in the
//! reference's order of operations (`calculateSpace`, `getAxisLabels`, `getQuadrants`,
//! `getQuadrantPoints`, `getBorders`, `getTitle`, `build`). Nothing is measured: every box is the
//! configuration's arithmetic (`docs-port/quadrant/NOTES.md`, 6.3 and 6.7).
//!
//! The arithmetic keeps the reference's operand order, because the last bit of a coordinate is
//! part of the output. The point positions are the d3 `scaleLinear` of [0, 1] (`common::d3`), which
//! computes `a * (1 - t) + b * t`, not `a + t * (b - a)` (NOTES 6.4).

use ferrigura_model::config::QuadrantChartConfig;
use ferrigura_model::quadrant_chart::{QuadrantChartDiagram, QuadrantStyle};
use ferrigura_model::theme::ThemeTokens;

use crate::common::d3::LinearScale;

/// The colour of a point when its style gives no `strokeWidth` (`builder` 509).
const DEFAULT_STROKE_WIDTH: &str = "0px";

/// `verticalPos` of a text: `left` is written as the text-anchor `start`, `center` as `middle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalPos {
    /// The text starts at its x.
    Left,
    /// The text is centred on its x.
    Center,
}

/// `horizontalPos` of a text: `top` is written as the baseline `hanging`, `middle` as `middle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalPos {
    /// The text hangs from its y.
    Top,
    /// The text is centred on its y.
    Middle,
}

/// A text of the chart (`QuadrantTextType`). A coordinate that is `None` is the `undefined` of the
/// reference, which the transform writes as the word; the point texts of a value that is not a
/// number take `NaN` instead (`builder` 502).
#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    /// The text, as the model holds it.
    pub text: String,
    /// The fill, or `None` when the theme gives none (the attribute is then left out).
    pub fill: Option<String>,
    /// The x of the transform.
    pub x: Option<f64>,
    /// The y of the transform.
    pub y: Option<f64>,
    /// The font size, in pixels.
    pub font_size: f64,
    /// Where the text sits across its x.
    pub vertical_pos: VerticalPos,
    /// Where the text sits across its y.
    pub horizontal_pos: HorizontalPos,
    /// The rotation in degrees: 0, or -90 for the y axis.
    pub rotation: f64,
}

/// The colours of the chart (`QuadrantBuilderThemeConfig`), read from the theme variables. A
/// variable the theme does not set is `None`, which the reference's `undefined` leaves out.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    /// `quadrantTitleFill`.
    pub title_fill: Option<String>,
    /// `quadrant1Fill` … `quadrant4Fill`.
    pub quadrant_fills: [Option<String>; 4],
    /// `quadrant1TextFill` … `quadrant4TextFill`.
    pub quadrant_text_fills: [Option<String>; 4],
    /// `quadrantPointFill`: the fill of a point with no colour of its own, and the default stroke.
    pub point_fill: Option<String>,
    /// `quadrantPointTextFill`.
    pub point_text_fill: Option<String>,
    /// `quadrantXAxisTextFill`.
    pub x_axis_text_fill: Option<String>,
    /// `quadrantYAxisTextFill`.
    pub y_axis_text_fill: Option<String>,
    /// `quadrantInternalBorderStrokeFill`.
    pub internal_border_fill: Option<String>,
    /// `quadrantExternalBorderStrokeFill`.
    pub external_border_fill: Option<String>,
}

impl Theme {
    /// The colours of a theme's tokens (`getQuadrantData`, `quadrantDb.ts` 126-142).
    #[must_use]
    pub fn new(tokens: &ThemeTokens) -> Self {
        Self {
            title_fill: tokens.quadrant_title_fill.text(),
            quadrant_fills: [
                tokens.quadrant1_fill.text(),
                tokens.quadrant2_fill.text(),
                tokens.quadrant3_fill.text(),
                tokens.quadrant4_fill.text(),
            ],
            quadrant_text_fills: [
                tokens.quadrant1_text_fill.text(),
                tokens.quadrant2_text_fill.text(),
                tokens.quadrant3_text_fill.text(),
                tokens.quadrant4_text_fill.text(),
            ],
            point_fill: tokens.quadrant_point_fill.text(),
            point_text_fill: tokens.quadrant_point_text_fill.text(),
            x_axis_text_fill: tokens.quadrant_x_axis_text_fill.text(),
            y_axis_text_fill: tokens.quadrant_y_axis_text_fill.text(),
            internal_border_fill: tokens.quadrant_internal_border_stroke_fill.text(),
            external_border_fill: tokens.quadrant_external_border_stroke_fill.text(),
        }
    }
}

/// A point of the chart (`QuadrantPointType`).
#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    /// The centre x of the circle, or `None` when the scale gives no value (the attribute is then
    /// left out, as d3 does with `undefined`).
    pub x: Option<f64>,
    /// The centre y of the circle, or `None`.
    pub y: Option<f64>,
    /// The fill of the circle.
    pub fill: Option<String>,
    /// The radius of the circle.
    pub radius: f64,
    /// The label of the point, under the circle.
    pub text: Text,
    /// The stroke of the circle.
    pub stroke_color: Option<String>,
    /// The stroke width of the circle, as written (`0px` by default).
    pub stroke_width: String,
}

/// A quadrant: its rectangle and its label (`QuadrantQuadrantsType`).
#[derive(Debug, Clone, PartialEq)]
pub struct Quadrant {
    /// The label of the quadrant.
    pub text: Text,
    /// The rectangle's x, y, width and height.
    pub x: f64,
    /// The top of the rectangle.
    pub y: f64,
    /// The width of the rectangle.
    pub width: f64,
    /// The height of the rectangle.
    pub height: f64,
    /// The fill of the rectangle.
    pub fill: Option<String>,
}

/// A border line (`QuadrantLineType`).
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    /// The stroke, or `None` when the theme gives none.
    pub stroke: Option<String>,
    /// The stroke width.
    pub stroke_width: f64,
    /// The start point.
    pub x1: f64,
    /// The start point.
    pub y1: f64,
    /// The end point.
    pub x2: f64,
    /// The end point.
    pub y2: f64,
}

/// The chart: what `build()` returns (`QuadrantBuildType`).
#[derive(Debug, Clone, PartialEq)]
pub struct Chart {
    /// The points, in the order the DB holds them.
    pub points: Vec<Point>,
    /// The quadrants, in the order 1 (top right), 2 (top left), 3 (bottom left), 4 (bottom right).
    pub quadrants: Vec<Quadrant>,
    /// The axis labels: the x texts, then the y texts, the ones that are shown.
    pub axis_labels: Vec<Text>,
    /// The six border lines: the external top, right, bottom and left, then the internal vertical
    /// and horizontal lines.
    pub borders: Vec<Line>,
    /// The title, when the chart has one.
    pub title: Option<Text>,
}

/// `calculateSpace`: the space the axes, the title and the quadrants take.
struct Space {
    /// The space of the title above the quadrants.
    title_top: f64,
    /// The quadrants' box.
    left: f64,
    top: f64,
    width: f64,
    half_width: f64,
    height: f64,
    half_height: f64,
}

/// Builds the chart of a parsed diagram with its configuration and theme (`QuadrantBuilder.build`
/// after `getQuadrantData`, `quadrantDb.ts` 120-145).
#[must_use]
pub fn build(config: &QuadrantChartConfig, data: &QuadrantChartDiagram, theme: &Theme) -> Chart {
    // `showXAxis` and `showYAxis` are the configuration's flags (true by default, not in the
    // schema), and the texts decide (`builder` 605-609).
    let show_x_axis = !data.x_axis_left_text.is_empty() || !data.x_axis_right_text.is_empty();
    let show_y_axis = !data.y_axis_top_text.is_empty() || !data.y_axis_bottom_text.is_empty();
    let show_title = !data.title.is_empty();
    // With points, the x axis is at the bottom whatever the configuration says (`builder` 611).
    let x_position = if data.points.is_empty() {
        config.x_axis_position.as_str()
    } else {
        "bottom"
    };
    let space = calculate_space(config, x_position, show_x_axis, show_y_axis, show_title);
    Chart {
        points: points(config, data, theme, &space),
        quadrants: quadrants(config, data, theme, &space),
        axis_labels: axis_labels(
            config,
            data,
            theme,
            &space,
            x_position,
            show_x_axis,
            show_y_axis,
        ),
        borders: borders(config, theme, &space),
        title: show_title.then(|| title(config, data, theme)),
    }
}

/// `calculateSpace` (`builder` 231-284).
fn calculate_space(
    config: &QuadrantChartConfig,
    x_position: &str,
    show_x_axis: bool,
    show_y_axis: bool,
    show_title: bool,
) -> Space {
    let x_axis = config.x_axis_label_padding * 2.0 + config.x_axis_label_font_size;
    let x_top = if x_position == "top" && show_x_axis {
        x_axis
    } else {
        0.0
    };
    let x_bottom = if x_position == "bottom" && show_x_axis {
        x_axis
    } else {
        0.0
    };
    let y_axis = config.y_axis_label_padding * 2.0 + config.y_axis_label_font_size;
    let y_left = if config.y_axis_position == "left" && show_y_axis {
        y_axis
    } else {
        0.0
    };
    let y_right = if config.y_axis_position == "right" && show_y_axis {
        y_axis
    } else {
        0.0
    };
    let title_top = if show_title {
        config.title_font_size + config.title_padding * 2.0
    } else {
        0.0
    };

    let padding = config.quadrant_padding;
    let left = padding + y_left;
    let top = padding + x_top + title_top;
    let width = config.chart_width - padding * 2.0 - y_left - y_right;
    let height = config.chart_height - padding * 2.0 - x_top - x_bottom - title_top;
    Space {
        title_top,
        left,
        top,
        width,
        half_width: width / 2.0,
        height,
        half_height: height / 2.0,
    }
}

/// `getQuadrantPoints` (`builder` 475-514). A point's own style wins over its class (`{ ...class,
/// ...point }`), and a class is looked up when the chart is built, so a class defined after its
/// point still applies (NOTES 2.4, Q15 and Q16).
fn points(
    config: &QuadrantChartConfig,
    data: &QuadrantChartDiagram,
    theme: &Theme,
    space: &Space,
) -> Vec<Point> {
    let x_scale = LinearScale::new([0.0, 1.0], [space.left, space.width + space.left]);
    let y_scale = LinearScale::new([0.0, 1.0], [space.height + space.top, space.top]);
    data.points
        .iter()
        .map(|point| {
            let class = data
                .classes
                .iter()
                .find(|(name, _)| *name == point.class_name)
                .map(|(_, style)| style);
            let style = merged_style(&point.style, class);
            let x = x_scale.apply(point.x);
            let y = y_scale.apply(point.y);
            Point {
                x,
                y,
                fill: style.color.clone().or_else(|| theme.point_fill.clone()),
                radius: style.radius.unwrap_or(config.point_radius),
                text: Text {
                    text: point.text.clone(),
                    fill: theme.point_text_fill.clone(),
                    x,
                    // `yAxis(point.y) + pointTextPadding`: `undefined + 5` is NaN.
                    y: Some(y.map_or(f64::NAN, |y| y + config.point_text_padding)),
                    vertical_pos: VerticalPos::Center,
                    horizontal_pos: HorizontalPos::Top,
                    font_size: config.point_label_font_size,
                    rotation: 0.0,
                },
                // The default stroke is the theme's point fill, not the point's own colour.
                stroke_color: style
                    .stroke_color
                    .clone()
                    .or_else(|| theme.point_fill.clone()),
                stroke_width: style
                    .stroke_width
                    .clone()
                    .unwrap_or_else(|| DEFAULT_STROKE_WIDTH.to_owned()),
            }
        })
        .collect()
}

/// The style of a point after its class: each field of the point, when it is set, else the
/// class's (`{ ...classStyles, ...point }`).
fn merged_style(point: &QuadrantStyle, class: Option<&QuadrantStyle>) -> QuadrantStyle {
    let class = class.cloned().unwrap_or_default();
    QuadrantStyle {
        radius: point.radius.or(class.radius),
        color: point.color.clone().or(class.color),
        stroke_color: point.stroke_color.clone().or(class.stroke_color),
        stroke_width: point.stroke_width.clone().or(class.stroke_width),
    }
}

/// `getQuadrants` (`builder` 384-473): four rectangles and their labels, the labels centred in the
/// rectangle when there are no points, and placed under its top edge when there are.
fn quadrants(
    config: &QuadrantChartConfig,
    data: &QuadrantChartDiagram,
    theme: &Theme,
    space: &Space,
) -> Vec<Quadrant> {
    let half_width = space.half_width;
    let half_height = space.half_height;
    // The rectangles in the order of the reference: 1 (top right), 2 (top left), 3 (bottom left)
    // and 4 (bottom right), with the texts the model holds.
    let texts = [
        &data.quadrant1_text,
        &data.quadrant2_text,
        &data.quadrant3_text,
        &data.quadrant4_text,
    ];
    let boxes = [
        (space.left + half_width, space.top),
        (space.left, space.top),
        (space.left, space.top + half_height),
        (space.left + half_width, space.top + half_height),
    ];
    texts
        .iter()
        .zip(boxes)
        .enumerate()
        .map(|(index, (text, (x, y)))| {
            let (horizontal_pos, label_y) = if data.points.is_empty() {
                (HorizontalPos::Middle, y + half_height / 2.0)
            } else {
                (HorizontalPos::Top, y + config.quadrant_text_top_padding)
            };
            Quadrant {
                text: Text {
                    text: (*text).clone(),
                    fill: theme.quadrant_text_fills[index].clone(),
                    x: Some(x + half_width / 2.0),
                    y: Some(label_y),
                    font_size: config.quadrant_label_font_size,
                    vertical_pos: VerticalPos::Center,
                    horizontal_pos,
                    rotation: 0.0,
                },
                x,
                y,
                width: half_width,
                height: half_height,
                fill: theme.quadrant_fills[index].clone(),
            }
        })
        .collect()
}

/// `getAxisLabels` (`builder` 286-382): the x texts, then the y texts, each when it is shown and
/// has a text. A middle text (a second x text, or a top y text) moves the labels to the middle of
/// their half.
fn axis_labels(
    config: &QuadrantChartConfig,
    data: &QuadrantChartDiagram,
    theme: &Theme,
    space: &Space,
    x_position: &str,
    show_x_axis: bool,
    show_y_axis: bool,
) -> Vec<Text> {
    let x_middle = !data.x_axis_right_text.is_empty();
    let y_middle = !data.y_axis_top_text.is_empty();
    let x_vertical = if x_middle {
        VerticalPos::Center
    } else {
        VerticalPos::Left
    };
    let y_vertical = if y_middle {
        VerticalPos::Center
    } else {
        VerticalPos::Left
    };
    let x_offset = if x_middle {
        space.half_width / 2.0
    } else {
        0.0
    };
    let y_offset = if y_middle {
        space.half_height / 2.0
    } else {
        0.0
    };
    let left_side = config.y_axis_position == "left";

    // The x labels: at the top under the title, or at the bottom under the quadrants.
    let x_y = if x_position == "top" {
        config.x_axis_label_padding + space.title_top
    } else {
        config.x_axis_label_padding + space.top + space.height + config.quadrant_padding
    };
    // The y labels: at the left, or at the right of the quadrants.
    let y_x = if left_side {
        config.y_axis_label_padding
    } else {
        config.y_axis_label_padding + space.left + space.width + config.quadrant_padding
    };

    let x_text = |text: &str, x: f64| Text {
        text: text.to_owned(),
        fill: theme.x_axis_text_fill.clone(),
        x: Some(x),
        y: Some(x_y),
        font_size: config.x_axis_label_font_size,
        vertical_pos: x_vertical,
        horizontal_pos: HorizontalPos::Top,
        rotation: 0.0,
    };
    let y_text = |text: &str, y: f64| Text {
        text: text.to_owned(),
        fill: theme.y_axis_text_fill.clone(),
        x: Some(y_x),
        y: Some(y),
        font_size: config.y_axis_label_font_size,
        vertical_pos: y_vertical,
        horizontal_pos: HorizontalPos::Top,
        rotation: -90.0,
    };

    let mut labels = Vec::new();
    if show_x_axis {
        if !data.x_axis_left_text.is_empty() {
            labels.push(x_text(&data.x_axis_left_text, space.left + x_offset));
        }
        if !data.x_axis_right_text.is_empty() {
            labels.push(x_text(
                &data.x_axis_right_text,
                space.left + space.half_width + x_offset,
            ));
        }
    }
    if show_y_axis {
        if !data.y_axis_bottom_text.is_empty() {
            labels.push(y_text(
                &data.y_axis_bottom_text,
                space.top + space.height - y_offset,
            ));
        }
        if !data.y_axis_top_text.is_empty() {
            labels.push(y_text(
                &data.y_axis_top_text,
                space.top + space.half_height - y_offset,
            ));
        }
    }
    labels
}

/// `getBorders` (`builder` 516-586). The external lines are inset by half their width at the
/// corners; the internal lines run between the inner edges.
fn borders(config: &QuadrantChartConfig, theme: &Theme, space: &Space) -> Vec<Line> {
    let external_width = config.quadrant_external_border_stroke_width;
    let internal_width = config.quadrant_internal_border_stroke_width;
    let half = external_width / 2.0;
    let left = space.left;
    let top = space.top;
    let width = space.width;
    let height = space.height;
    let external = |x1: f64, y1: f64, x2: f64, y2: f64| Line {
        stroke: theme.external_border_fill.clone(),
        stroke_width: external_width,
        x1,
        y1,
        x2,
        y2,
    };
    let internal = |x1: f64, y1: f64, x2: f64, y2: f64| Line {
        stroke: theme.internal_border_fill.clone(),
        stroke_width: internal_width,
        x1,
        y1,
        x2,
        y2,
    };
    vec![
        // The top, right, bottom and left edges.
        external(left - half, top, left + width + half, top),
        external(left + width, top + half, left + width, top + height - half),
        external(left - half, top + height, left + width + half, top + height),
        external(left, top + half, left, top + height - half),
        // The inner vertical and horizontal lines.
        internal(
            left + space.half_width,
            top + half,
            left + space.half_width,
            top + height - half,
        ),
        internal(
            left + half,
            top + space.half_height,
            left + width - half,
            top + space.half_height,
        ),
    ]
}

/// `getTitle` (`builder` 588-602): centred on the chart's width, `titlePadding` from the top.
fn title(config: &QuadrantChartConfig, data: &QuadrantChartDiagram, theme: &Theme) -> Text {
    Text {
        text: data.title.clone(),
        fill: theme.title_fill.clone(),
        x: Some(config.chart_width / 2.0),
        y: Some(config.title_padding),
        font_size: config.title_font_size,
        vertical_pos: VerticalPos::Center,
        horizontal_pos: HorizontalPos::Top,
        rotation: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrigura_model::config::{defaults, typed};
    use ferrigura_model::quadrant_chart::QuadrantPoint;

    fn configuration() -> QuadrantChartConfig {
        typed(&defaults())
            .expect("the defaults are a configuration")
            .quadrant_chart
    }

    fn theme() -> Theme {
        Theme::new(&ThemeTokens::from_value(
            &ferrigura_model::theme::Theme::ReduxColor.variables(None),
        ))
    }

    fn diagram_with_points(points: Vec<QuadrantPoint>) -> QuadrantChartDiagram {
        let mut diagram = QuadrantChartDiagram::new();
        diagram.points = points;
        diagram
    }

    fn point(x: f64, y: f64) -> QuadrantPoint {
        QuadrantPoint {
            text: "p".to_owned(),
            class_name: String::new(),
            x,
            y,
            style: QuadrantStyle::default(),
        }
    }

    /// The complete chart of `should-render-a-complete-quadrant-chart`: a title, a y-axis text at
    /// the left and an x-axis text, so that the quadrants start at (31, 45) and are 464 by 424.
    fn complete_chart() -> QuadrantChartDiagram {
        let mut diagram = QuadrantChartDiagram::new();
        diagram.title = "Reach and engagement of campaigns".to_owned();
        diagram.x_axis_left_text = "Low Reach".to_owned();
        diagram.y_axis_bottom_text = "Low Engagement".to_owned();
        diagram
    }

    #[test]
    fn a_point_takes_the_scale_of_the_quadrants_with_the_reference_bits() {
        // `scaleLinear` of d3 on [0, 1] over [31, 495] and [469, 45]: the reference writes these
        // values for `Campaign C: [0.57, 0.69]` (`cx="295.47999999999996" cy="176.44"`). The
        // formula `a + t * (b - a)` gives 284.3 for the bare chart and is not the reference's.
        let mut diagram = complete_chart();
        diagram.points = vec![point(0.57, 0.69)];
        let chart = build(&configuration(), &diagram, &theme());
        let x = chart.points[0].x.expect("a finite coordinate");
        let y = chart.points[0].y.expect("a finite coordinate");
        assert_eq!(x.to_bits(), 295.47999999999996_f64.to_bits());
        assert_eq!(y.to_bits(), 176.44_f64.to_bits());
    }

    #[test]
    fn a_coordinate_the_scale_cannot_read_has_no_position() {
        // `undefined` from the scale: no x for the circle, and the text's x is `undefined`.
        let x_missing = build(
            &configuration(),
            &diagram_with_points(vec![point(f64::NAN, 0.5)]),
            &theme(),
        );
        assert_eq!(x_missing.points[0].x, None);
        assert_eq!(x_missing.points[0].text.x, None);
        // `undefined + 5` is NaN, so the text's y is NaN.
        let y_missing = build(
            &configuration(),
            &diagram_with_points(vec![point(0.5, f64::NAN)]),
            &theme(),
        );
        assert_eq!(y_missing.points[0].y, None);
        assert!(y_missing.points[0].text.y.expect("a y").is_nan());
    }

    #[test]
    fn the_x_axis_is_at_the_bottom_when_there_are_points() {
        let mut diagram = diagram_with_points(vec![point(0.5, 0.5)]);
        diagram.x_axis_left_text = "Low".to_owned();
        let chart = build(&configuration(), &diagram, &theme());
        // The label's y is the bottom formula: padding + top + height + padding.
        assert_eq!(chart.axis_labels[0].y, Some(479.0));
    }

    #[test]
    fn the_quadrant_labels_are_centred_without_points() {
        let mut diagram = QuadrantChartDiagram::new();
        diagram.quadrant1_text = "Top right".to_owned();
        let chart = build(&configuration(), &diagram, &theme());
        assert_eq!(
            chart.quadrants[0].text.horizontal_pos,
            HorizontalPos::Middle
        );
        // No y axis and no title: the box is the chart less the padding, so the first quadrant's
        // label is at (372.5, 127.5), as the reference writes `should-render-if-only-chart-type`.
        assert_eq!(chart.quadrants[0].text.x, Some(372.5));
        assert_eq!(chart.quadrants[0].text.y, Some(127.5));
    }

    #[test]
    fn a_class_style_applies_unless_the_point_sets_its_own() {
        let mut styled = point(0.5, 0.5);
        styled.class_name = "hot".to_owned();
        styled.style.radius = Some(20.0);
        let mut diagram = diagram_with_points(vec![styled]);
        diagram.classes = vec![(
            "hot".to_owned(),
            QuadrantStyle {
                radius: Some(10.0),
                color: Some("#908342".to_owned()),
                stroke_color: None,
                stroke_width: Some("10px".to_owned()),
            },
        )];
        let chart = build(&configuration(), &diagram, &theme());
        assert_eq!(chart.points[0].radius, 20.0);
        assert_eq!(chart.points[0].fill.as_deref(), Some("#908342"));
        assert_eq!(chart.points[0].stroke_width, "10px");
    }

    #[test]
    fn the_default_stroke_is_the_theme_point_fill() {
        let diagram = diagram_with_points(vec![point(0.5, 0.5)]);
        let chart = build(&configuration(), &diagram, &theme());
        assert_eq!(chart.points[0].stroke_color, chart.points[0].fill);
        assert_eq!(chart.points[0].stroke_width, "0px");
    }

    #[test]
    fn a_chart_without_a_title_has_no_title() {
        let chart = build(&configuration(), &QuadrantChartDiagram::new(), &theme());
        assert!(chart.title.is_none());
        assert_eq!(chart.borders.len(), 6);
        assert_eq!(chart.quadrants.len(), 4);
    }
}
