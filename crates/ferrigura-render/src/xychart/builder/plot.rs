//! The plots: `chartBuilder/components/plot/` of the reference (`index.ts`, `linePlot.ts`,
//! `barPlot.ts`). A line is one path with its point labels; a bar is one rectangle per point.

use super::axis::Axis;
use super::interfaces::{
    ChartConfig, ChartData, Dimension, Drawable, HorizontalPos, PathElem, PlotData, Point, Rect,
    RectElem, TextElem, VerticalPos,
};
use crate::common::d3::linear_path;

/// The share of a bar's slot that is left empty on each bar (`barPaddingPercent`).
const BAR_PADDING_PERCENT: f64 = 0.05;
/// How far a point label sits from its point (`labelOffset`).
const LABEL_OFFSET: f64 = 10.0;
/// The font size of a point label (`fontSize` in `linePlot.ts`).
const POINT_LABEL_FONT_SIZE: f64 = 12.0;

/// The plots of the chart, as one component with its box.
#[derive(Debug, Clone)]
pub struct Plot<'a> {
    config: &'a ChartConfig,
    data: &'a ChartData,
    bounding: Rect,
}

impl<'a> Plot<'a> {
    /// The plots of the chart, not yet placed.
    #[must_use]
    pub fn new(config: &'a ChartConfig, data: &'a ChartData) -> Self {
        Self {
            config,
            data,
            bounding: Rect::default(),
        }
    }

    /// `calculateSpace(availableSpace)`: the plots take the whole space they are given.
    pub fn calculate_space(&mut self, available: Dimension) -> Dimension {
        self.bounding.width = available.width;
        self.bounding.height = available.height;
        Dimension {
            width: self.bounding.width,
            height: self.bounding.height,
        }
    }

    /// `setBoundingBoxXY(point)`.
    pub fn set_bounding_box_xy(&mut self, x: f64, y: f64) {
        self.bounding.x = x;
        self.bounding.y = y;
    }

    /// `getDrawableElements()`: the plots in the order they were declared, each with its index.
    #[must_use]
    pub fn drawables(&self, x_axis: &Axis, y_axis: &Axis) -> Vec<Drawable> {
        let horizontal = self.config.horizontal;
        let mut out = Vec::new();
        for (index, plot) in self.data.plots.iter().enumerate() {
            match plot {
                PlotData::Line { .. } => {
                    out.extend(line_drawables(plot, x_axis, y_axis, horizontal, index));
                }
                PlotData::Bar { .. } => {
                    out.extend(bar_drawables(
                        plot,
                        self.bounding,
                        x_axis,
                        y_axis,
                        horizontal,
                        index,
                    ));
                }
            }
        }
        out
    }
}

/// The scaled points of a plot. The x position is `NaN` when the scale gives none, as the reference's
/// arithmetic on `undefined` gives. The y position stays optional: a bar writes its `y` only when it
/// has one.
fn scaled(data: &[Point], x_axis: &Axis, y_axis: &Axis) -> Vec<(f64, Option<f64>)> {
    data.iter()
        .map(|point| {
            (
                x_axis.scale_text(&point.x).unwrap_or(f64::NAN),
                y_axis.scale_number(point.y),
            )
        })
        .collect()
}

/// `LinePlot.getDrawableElement()`.
fn line_drawables(
    plot: &PlotData,
    x_axis: &Axis,
    y_axis: &Axis,
    horizontal: bool,
    index: usize,
) -> Vec<Drawable> {
    let PlotData::Line {
        stroke_fill,
        stroke_width,
        data,
        point_labels,
        ..
    } = plot
    else {
        return Vec::new();
    };
    let points: Vec<(f64, f64)> = scaled(data, x_axis, y_axis)
        .into_iter()
        .map(|(px, py)| (px, py.unwrap_or(f64::NAN)))
        .collect();
    // The path is `line().x(d[0]).y(d[1])`, or `.y(d[0]).x(d[1])` when horizontal.
    let path_points: Vec<(f64, f64)> = if horizontal {
        points.iter().map(|&(px, py)| (py, px)).collect()
    } else {
        points.clone()
    };
    let Some(path) = linear_path(&path_points) else {
        return Vec::new();
    };
    let mut out = vec![Drawable::Path {
        groups: vec!["plot".to_owned(), format!("line-plot-{index}")],
        data: vec![PathElem {
            path,
            fill: None,
            stroke_fill: stroke_fill.clone(),
            stroke_width: *stroke_width,
        }],
    }];

    if let Some(labels) = point_labels.as_ref().filter(|labels| !labels.is_empty()) {
        let mut texts = Vec::new();
        for (i, &(px, py)) in points.iter().enumerate() {
            let Some(label) = labels.get(i).filter(|label| !label.is_empty()) else {
                continue;
            };
            let text = if horizontal {
                TextElem {
                    x: py + LABEL_OFFSET,
                    y: px,
                    text: label.clone(),
                    fill: stroke_fill.clone(),
                    vertical_pos: VerticalPos::Middle,
                    horizontal_pos: HorizontalPos::Left,
                    font_size: POINT_LABEL_FONT_SIZE,
                    rotation: 0.0,
                }
            } else {
                TextElem {
                    x: px,
                    y: py - LABEL_OFFSET,
                    text: label.clone(),
                    fill: stroke_fill.clone(),
                    vertical_pos: VerticalPos::Middle,
                    horizontal_pos: HorizontalPos::Center,
                    font_size: POINT_LABEL_FONT_SIZE,
                    rotation: 0.0,
                }
            };
            texts.push(text);
        }
        if !texts.is_empty() {
            out.push(Drawable::Text {
                groups: vec![
                    "plot".to_owned(),
                    format!("line-plot-{index}"),
                    "labels".to_owned(),
                ],
                data: texts,
            });
        }
    }
    out
}

/// `BarPlot.getDrawableElement()`.
fn bar_drawables(
    plot: &PlotData,
    plot_box: Rect,
    x_axis: &Axis,
    y_axis: &Axis,
    horizontal: bool,
    index: usize,
) -> Vec<Drawable> {
    let PlotData::Bar { fill, data, .. } = plot else {
        return Vec::new();
    };
    let points = scaled(data, x_axis, y_axis);
    let bar_width =
        (x_axis.outer_padding() * 2.0).min(x_axis.tick_distance()) * (1.0 - BAR_PADDING_PERCENT);
    let half = bar_width / 2.0;
    let bars = points
        .into_iter()
        .map(|(px, py)| {
            if horizontal {
                RectElem {
                    x: plot_box.x,
                    y: Some(px - half),
                    width: py.unwrap_or(f64::NAN) - plot_box.x,
                    height: bar_width,
                    fill: fill.clone(),
                    stroke_fill: fill.clone(),
                    stroke_width: 0.0,
                }
            } else {
                RectElem {
                    x: px - half,
                    y: py,
                    width: bar_width,
                    height: plot_box.y + plot_box.height - py.unwrap_or(f64::NAN),
                    fill: fill.clone(),
                    stroke_fill: fill.clone(),
                    stroke_width: 0.0,
                }
            }
        })
        .collect();
    vec![Drawable::Rect {
        groups: vec!["plot".to_owned(), format!("bar-plot-{index}")],
        data: bars,
    }]
}
