//! The legend: `chartBuilder/components/legend.ts`. A plot with a title has a marker and a name in
//! the legend; a bar has a square marker, a line a short stroke.

use super::interfaces::{
    ChartConfig, ChartData, Dimension, Drawable, HorizontalPos, PathElem, PlotData, Rect, RectElem,
    TextElem, ThemeConfig, VerticalPos,
};
use super::text::max_dimension;
use crate::RenderError;

const MARKER_TO_FONT_RATIO: f64 = 0.75;
const ITEM_SPACING_TO_FONT_RATIO: f64 = 0.5;
const MARKER_SPACING_TO_FONT_RATIO: f64 = 0.35;

/// The legend of the chart.
#[derive(Debug, Clone)]
pub struct ChartLegend<'a> {
    config: &'a ChartConfig,
    data: &'a ChartData,
    theme: &'a ThemeConfig,
    bounding: Rect,
    visible: Vec<&'a PlotData>,
}

impl<'a> ChartLegend<'a> {
    /// The legend of the chart, not yet placed.
    #[must_use]
    pub fn new(config: &'a ChartConfig, data: &'a ChartData, theme: &'a ThemeConfig) -> Self {
        Self {
            config,
            data,
            theme,
            bounding: Rect::default(),
            visible: Vec::new(),
        }
    }

    /// `setBoundingBoxXY(point)`.
    pub fn set_bounding_box_xy(&mut self, x: f64, y: f64) {
        self.bounding.x = x;
        self.bounding.y = y;
    }

    /// `calculateSpace(availableSpace)`: the legend's size, or none when it does not fit.
    ///
    /// # Errors
    ///
    /// Returns an error if the bundled font cannot be read.
    pub fn calculate_space(&mut self, available: Dimension) -> Result<Dimension, RenderError> {
        self.visible = if self.config.show_legend {
            self.data
                .plots
                .iter()
                .filter(|plot| !plot.title().is_empty())
                .collect()
        } else {
            Vec::new()
        };
        if self.visible.is_empty() {
            self.bounding.width = 0.0;
            self.bounding.height = 0.0;
            return Ok(Dimension::default());
        }

        let font = self.config.legend_font_size;
        let layout = Layout::of(font);
        let titles: Vec<&str> = self.visible.iter().map(|plot| plot.title()).collect();
        let text = max_dimension(&titles, font, self.config.screen_scale)?;
        let count = self.visible.len() as f64;
        let width_required = self.config.legend_padding * 2.0
            + layout.marker_size
            + layout.marker_spacing
            + text.width;
        let height_required =
            self.config.legend_padding * 2.0 + count * font + (count - 1.0) * layout.item_spacing;

        if width_required <= available.width && height_required <= available.height {
            self.bounding.width = width_required;
            self.bounding.height = height_required;
        } else {
            self.visible = Vec::new();
            self.bounding.width = 0.0;
            self.bounding.height = 0.0;
        }
        Ok(Dimension {
            width: self.bounding.width,
            height: self.bounding.height,
        })
    }

    /// `getDrawableElements()`: the markers and the names of the visible plots.
    #[must_use]
    pub fn drawables(&self) -> Vec<Drawable> {
        if self.visible.is_empty() {
            return Vec::new();
        }
        let font = self.config.legend_font_size;
        let layout = Layout::of(font);
        let row_height = font + layout.item_spacing;
        let start_x = self.bounding.x + self.config.legend_padding;
        let start_y = self.bounding.y + self.config.legend_padding;
        let mut bars = Vec::new();
        let mut lines = Vec::new();
        for (index, plot) in self.visible.iter().enumerate() {
            let row = start_y + index as f64 * row_height;
            match plot {
                PlotData::Bar { fill, .. } => bars.push(RectElem {
                    x: start_x,
                    y: Some(row),
                    width: layout.marker_size,
                    height: layout.marker_size,
                    fill: fill.clone(),
                    stroke_fill: fill.clone(),
                    stroke_width: 0.0,
                }),
                PlotData::Line {
                    stroke_fill,
                    stroke_width,
                    ..
                } => {
                    let marker_y = row + layout.marker_size / 2.0;
                    lines.push(PathElem {
                        path: format!(
                            "M {},{} L {},{}",
                            crate::common::d3::js_number(start_x),
                            crate::common::d3::js_number(marker_y),
                            crate::common::d3::js_number(start_x + layout.marker_size),
                            crate::common::d3::js_number(marker_y)
                        ),
                        fill: None,
                        stroke_fill: stroke_fill.clone(),
                        stroke_width: *stroke_width,
                    });
                }
            }
        }
        let labels = self
            .visible
            .iter()
            .enumerate()
            .map(|(index, plot)| TextElem {
                text: plot.title().to_owned(),
                x: start_x + layout.marker_size + layout.marker_spacing,
                y: start_y + index as f64 * row_height + layout.marker_size / 2.0,
                fill: self.theme.legend_text_color.clone(),
                vertical_pos: VerticalPos::Middle,
                horizontal_pos: HorizontalPos::Left,
                font_size: font,
                rotation: 0.0,
            })
            .collect();
        vec![
            Drawable::Rect {
                groups: vec!["legend".to_owned(), "markers".to_owned()],
                data: bars,
            },
            Drawable::Path {
                groups: vec!["legend".to_owned(), "markers".to_owned()],
                data: lines,
            },
            Drawable::Text {
                groups: vec!["legend".to_owned(), "label".to_owned()],
                data: labels,
            },
        ]
    }
}

/// The sizes of the legend at its font size (`getLegendLayout`).
struct Layout {
    marker_size: f64,
    marker_spacing: f64,
    item_spacing: f64,
}

impl Layout {
    fn of(font: f64) -> Self {
        Self {
            marker_size: font * MARKER_TO_FONT_RATIO,
            marker_spacing: font * MARKER_SPACING_TO_FONT_RATIO,
            item_spacing: font * ITEM_SPACING_TO_FONT_RATIO,
        }
    }
}
