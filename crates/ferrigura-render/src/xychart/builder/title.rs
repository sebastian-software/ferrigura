//! The chart title: `chartBuilder/components/chartTitle.ts`.

use super::interfaces::{
    ChartConfig, ChartData, Dimension, Drawable, HorizontalPos, Rect, TextElem, ThemeConfig,
    VerticalPos,
};
use super::text::max_dimension;
use crate::RenderError;

/// The title of the chart. It is drawn only when `showTitle` is on and the title is not empty, and
/// when its height fits the space the chart gives it.
#[derive(Debug, Clone)]
pub struct ChartTitle<'a> {
    config: &'a ChartConfig,
    data: &'a ChartData,
    theme: &'a ThemeConfig,
    bounding: Rect,
    show: bool,
    title_width: f64,
}

impl<'a> ChartTitle<'a> {
    /// The title of the chart, not yet placed.
    #[must_use]
    pub fn new(config: &'a ChartConfig, data: &'a ChartData, theme: &'a ThemeConfig) -> Self {
        Self {
            config,
            data,
            theme,
            bounding: Rect::default(),
            show: false,
            title_width: 0.0,
        }
    }

    /// `calculateSpace(availableSpace)`: the height the title takes, or none when it is not drawn.
    ///
    /// # Errors
    ///
    /// Returns an error if the bundled font cannot be read.
    pub fn calculate_space(&mut self, available: Dimension) -> Result<Dimension, RenderError> {
        let title = max_dimension(
            &[self.data.title.as_str()],
            self.config.title_font_size,
            self.config.screen_scale,
        )?;
        let width_required = title.width.max(available.width);
        let height_required = title.height + 2.0 * self.config.title_padding;
        if height_required <= available.height
            && self.config.show_title
            && !self.data.title.is_empty()
        {
            self.bounding.width = width_required;
            self.bounding.height = height_required;
            self.title_width = title.width;
            self.show = true;
        }
        Ok(Dimension {
            width: self.bounding.width,
            height: self.bounding.height,
        })
    }

    /// `alignTo(start, end)`: centres the title over `[start, end]`, moved as needed to stay inside
    /// the chart.
    pub fn align_to(&mut self, start: f64, end: f64) {
        let chart_width = self.config.width;
        let half_width = self.title_width.min(chart_width) / 2.0;
        let centre = ((start + end) / 2.0)
            .max(half_width)
            .min(chart_width - half_width);
        self.bounding.x = centre - half_width;
        self.bounding.width = half_width * 2.0;
    }

    /// `getDrawableElements()`: the title, when it is drawn.
    #[must_use]
    pub fn drawables(&self) -> Vec<Drawable> {
        if !self.show {
            return Vec::new();
        }
        vec![Drawable::Text {
            groups: vec!["chart-title".to_owned()],
            data: vec![TextElem {
                text: self.data.title.clone(),
                x: self.bounding.x + self.bounding.width / 2.0,
                y: self.bounding.y + self.bounding.height / 2.0,
                fill: self.theme.title_color.clone(),
                vertical_pos: VerticalPos::Middle,
                horizontal_pos: HorizontalPos::Center,
                font_size: self.config.title_font_size,
                rotation: 0.0,
            }],
        }]
    }
}
