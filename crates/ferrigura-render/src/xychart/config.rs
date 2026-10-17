//! Reads the chart's inputs into the builder's types: the typed configuration (`xyChart`), the
//! theme's `xyChart` colours, and the model's axes and plots.
//!
//! The colours follow the reference's `getChartDefaultThemeConfig`: `cleanAndMerge` of the classic
//! default theme's `xyChart` with the configured one. The shipped themes do not set `dataLabelColor`,
//! so the classic theme's value (its primary text colour, `#131300`) stands, unless the configuration
//! sets it.

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::value::Value;
use ferrigura_model::xychart::{Plot, XAxis, XyChartDiagram, YAxis};

use crate::xychart::builder::interfaces::{
    AxisConfig, AxisData, AxisThemeConfig, ChartConfig, ChartData, PlotData, Point, ThemeConfig,
};

/// The classic default theme's primary text colour, which its `xyChart` colours fall back to.
const CLASSIC_TEXT_COLOUR: &str = "#131300";
/// The classic default theme's background.
const CLASSIC_BACKGROUND: &str = "#ffffff";

/// The width of the container the harness renders in: the page's viewport, 1200 px
/// (`docs-port/P2-RENDERERS.md`, screen and container).
const CONTAINER_WIDTH: f64 = 1200.0;

/// The scale of the SVG on the screen when the reference measures its texts. The SVG has the
/// configured width as its `max-width` and fills the container up to it, so the scale is the
/// container's width over the SVG's width, and 1 when the SVG is narrower than the container.
fn screen_scale(width: f64) -> f64 {
    if width > CONTAINER_WIDTH {
        CONTAINER_WIDTH / width
    } else {
        1.0
    }
}

/// The chart's configuration. The orientation is the model's (the keyword of the header, else the
/// configuration's `chartOrientation`).
pub fn chart_config(config: &MermaidConfig, horizontal: bool) -> ChartConfig {
    let xy = &config.xy_chart;
    ChartConfig {
        width: xy.width,
        screen_scale: screen_scale(xy.width),
        height: xy.height,
        title_font_size: xy.title_font_size,
        title_padding: xy.title_padding,
        show_title: xy.show_title,
        show_legend: xy.show_legend,
        legend_font_size: xy.legend_font_size,
        legend_padding: xy.legend_padding,
        show_data_label: xy.show_data_label,
        show_data_label_outside_bar: xy.show_data_label_outside_bar,
        x_axis: axis_config(&xy.x_axis),
        y_axis: axis_config(&xy.y_axis),
        horizontal,
        plot_reserved_space_percent: xy.plot_reserved_space_percent,
    }
}

fn axis_config(axis: &ferrigura_model::config::XYChartAxisConfig) -> AxisConfig {
    AxisConfig {
        show_label: axis.show_label,
        label_font_size: axis.label_font_size,
        label_padding: axis.label_padding,
        show_title: axis.show_title,
        title_font_size: axis.title_font_size,
        title_padding: axis.title_padding,
        show_tick: axis.show_tick,
        tick_length: axis.tick_length,
        tick_width: axis.tick_width,
        show_axis_line: axis.show_axis_line,
        axis_line_width: axis.axis_line_width,
        label_rotation: axis.label_rotation,
    }
}

/// The colours of the chart, from the theme's `xyChart` object.
pub fn theme_config(variables: &Value) -> ThemeConfig {
    let xy = variables.get("xyChart");
    let text = |key: &str| colour(xy, key, CLASSIC_TEXT_COLOUR);
    ThemeConfig {
        background_color: colour(xy, "backgroundColor", CLASSIC_BACKGROUND),
        title_color: text("titleColor"),
        data_label_color: text("dataLabelColor"),
        legend_text_color: text("legendTextColor"),
        x_axis: AxisThemeConfig {
            title_color: text("xAxisTitleColor"),
            label_color: text("xAxisLabelColor"),
            tick_color: text("xAxisTickColor"),
            axis_line_color: text("xAxisLineColor"),
        },
        y_axis: AxisThemeConfig {
            title_color: text("yAxisTitleColor"),
            label_color: text("yAxisLabelColor"),
            tick_color: text("yAxisTickColor"),
            axis_line_color: text("yAxisLineColor"),
        },
    }
}

/// A colour of the `xyChart` object: its text when it is text, else the default.
fn colour(xy: Option<&Value>, key: &str, default: &str) -> String {
    match xy.and_then(|xy| xy.get(key)) {
        Some(Value::String(text)) => text.clone(),
        _ => default.to_owned(),
    }
}

/// The chart's data, as the DB holds it after the parse.
pub fn chart_data(diagram: &XyChartDiagram) -> ChartData {
    let x_axis = match &diagram.x_axis {
        XAxis::Band { title, categories } => AxisData::Band {
            title: title.clone(),
            categories: categories.clone(),
        },
        XAxis::Linear { title, min, max } => AxisData::Linear {
            title: title.clone(),
            min: *min,
            max: *max,
        },
    };
    let y_axis = match &diagram.y_axis {
        YAxis::Linear { title, min, max } => AxisData::Linear {
            title: title.clone(),
            min: *min,
            max: *max,
        },
    };
    let plots = diagram
        .plots
        .iter()
        .map(|plot| match plot {
            Plot::Line {
                title,
                stroke_fill,
                stroke_width,
                data,
                point_labels,
            } => PlotData::Line {
                title: title.clone(),
                stroke_fill: stroke_fill.clone(),
                stroke_width: *stroke_width,
                data: points(data),
                point_labels: point_labels.clone(),
            },
            Plot::Bar { title, fill, data } => PlotData::Bar {
                title: title.clone(),
                fill: fill.clone(),
                data: points(data),
            },
        })
        .collect();
    ChartData {
        x_axis,
        y_axis,
        title: diagram.title.clone(),
        plots,
    }
}

fn points(data: &[ferrigura_model::xychart::DataPoint]) -> Vec<Point> {
    data.iter()
        .map(|point| Point {
            x: point.x.clone(),
            y: point.y,
        })
        .collect()
}
