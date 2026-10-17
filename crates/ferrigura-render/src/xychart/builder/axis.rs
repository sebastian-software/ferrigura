//! The axes of the chart: `chartBuilder/components/axis/` of the reference (`baseAxis.ts`,
//! `bandAxis.ts`, `linearAxis.ts`, `index.ts`).
//!
//! The reference rebuilds its d3 scale in `recalculateScale` after each change of the range, the
//! position or the outer padding. This port builds the scale when it is used, from the same state,
//! which gives the same values because every change in the reference is followed by a rebuild.

use std::f64::consts::PI;

use ferrigura_jsmath::{cos, sin};

use super::interfaces::{
    AxisConfig, AxisData, AxisThemeConfig, Dimension, Drawable, HorizontalPos, PathElem, Rect,
    TextElem, VerticalPos,
};
use super::text::{TextBox, max_dimension};
use crate::RenderError;
use crate::common::d3::{BandScale, LinearScale, js_number, js_to_number};

/// The share of a tick step that a bar takes (`BAR_WIDTH_TO_TICK_WIDTH_RATIO`).
const BAR_WIDTH_TO_TICK_WIDTH_RATIO: f64 = 0.7;
/// The most the outer padding may take of the space, for the labels (`MAX_OUTER_PADDING_PERCENT_FOR_WRT_LABEL`).
const MAX_OUTER_PADDING_PERCENT_FOR_WRT_LABEL: f64 = 0.2;
/// The number of ticks d3 asks for (`scale.ticks()` with no count).
const TICK_COUNT: f64 = 10.0;

/// Where an axis is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisPosition {
    Left,
    Top,
    Bottom,
}

/// A tick of an axis: a category of a band axis, or a value of a linear one.
#[derive(Debug, Clone, PartialEq)]
pub enum Tick {
    Text(String),
    Value(f64),
}

impl Tick {
    /// `tick.toString()`: the text of the tick, a value as JavaScript writes it.
    fn text(&self) -> String {
        match self {
            Self::Text(text) => text.clone(),
            Self::Value(value) => js_number(*value),
        }
    }
}

/// The domain of an axis.
#[derive(Debug, Clone, PartialEq)]
enum Domain {
    Band(Vec<String>),
    Linear([f64; 2]),
}

/// An axis: its range, its position, its ticks and its labels.
#[derive(Debug, Clone, PartialEq)]
pub struct Axis {
    domain: Domain,
    screen_scale: f64,
    title: String,
    config: AxisConfig,
    theme: AxisThemeConfig,
    position: AxisPosition,
    range: [f64; 2],
    bounding: Rect,
    show_title: bool,
    show_label: bool,
    show_tick: bool,
    show_axis_line: bool,
    outer_padding: f64,
    title_text_height: f64,
    /// The label rotation in radians, or 0 when the rotation is outside -90 to 90.
    rotation: f64,
}

/// `undefined` as the reference writes it through arithmetic: NaN.
fn or_nan(value: Option<f64>) -> f64 {
    value.unwrap_or(f64::NAN)
}

impl Axis {
    /// `getAxis(data, config, theme)`: a band axis for a band of categories, else a linear one.
    #[must_use]
    pub fn new(
        data: &AxisData,
        config: &AxisConfig,
        theme: &AxisThemeConfig,
        screen_scale: f64,
    ) -> Self {
        let (domain, title) = match data {
            AxisData::Band { title, categories } => (Domain::Band(categories.clone()), title),
            AxisData::Linear { title, min, max } => (Domain::Linear([*min, *max]), title),
        };
        let rotation = if config.label_rotation >= -90.0 && config.label_rotation <= 90.0 {
            (config.label_rotation * PI) / 180.0
        } else {
            0.0
        };
        Self {
            domain,
            screen_scale,
            title: title.clone(),
            config: config.clone(),
            theme: theme.clone(),
            position: AxisPosition::Left,
            range: [0.0, 10.0],
            bounding: Rect::default(),
            show_title: false,
            show_label: false,
            show_tick: false,
            show_axis_line: false,
            outer_padding: 0.0,
            title_text_height: 0.0,
            rotation,
        }
    }

    /// `getRange()`: the range inset by the outer padding at both ends.
    fn get_range(&self) -> [f64; 2] {
        [
            self.range[0] + self.outer_padding,
            self.range[1] - self.outer_padding,
        ]
    }

    /// `setRange(range)`: the range, and the size of the box along the axis.
    pub fn set_range(&mut self, range: [f64; 2]) {
        self.range = range;
        if self.position == AxisPosition::Left {
            self.bounding.height = range[1] - range[0];
        } else {
            self.bounding.width = range[1] - range[0];
        }
    }

    /// `setAxisPosition(position)`: the position, and the range again, which sets the other side.
    pub fn set_axis_position(&mut self, position: AxisPosition) {
        self.position = position;
        self.set_range(self.range);
    }

    /// `setBoundingBoxXY(point)`.
    pub fn set_bounding_box_xy(&mut self, x: f64, y: f64) {
        self.bounding.x = x;
        self.bounding.y = y;
    }

    /// `getAxisOuterPadding()`.
    #[must_use]
    pub fn outer_padding(&self) -> f64 {
        self.outer_padding
    }

    /// The d3 band scale of a band axis, built on the current range.
    fn band_scale(&self, categories: &[String]) -> BandScale {
        BandScale::new(categories, self.get_range())
            .padding_inner(1.0)
            .padding_outer(0.0)
            .align(0.5)
    }

    /// The d3 linear scale of a linear axis. The left axis reverses its domain, since the SVG's y
    /// axis runs down from the top (`linearAxis.ts` 27-33).
    fn linear_scale(&self, domain: [f64; 2]) -> LinearScale {
        let mut domain = domain;
        if self.position == AxisPosition::Left {
            domain.reverse();
        }
        LinearScale::new(domain, self.get_range())
    }

    /// `getTickValues()`: every category of a band axis (repeated ones included), or the ticks of a
    /// linear axis.
    fn tick_values(&self) -> Vec<Tick> {
        match &self.domain {
            Domain::Band(categories) => categories.iter().cloned().map(Tick::Text).collect(),
            Domain::Linear(domain) => self
                .linear_scale(*domain)
                .ticks(TICK_COUNT)
                .into_iter()
                .map(Tick::Value)
                .collect(),
        }
    }

    /// `getTickDistance()`: the range over the number of ticks.
    #[must_use]
    pub fn tick_distance(&self) -> f64 {
        let range = self.get_range();
        (range[0] - range[1]).abs() / self.tick_values().len() as f64
    }

    /// `getScaleValue(value)` for a text: the start of a category, or the position of a number
    /// written as text. A category that the axis does not have is at the start of the range, as the
    /// reference falls back to `range[0]`.
    #[must_use]
    pub fn scale_text(&self, text: &str) -> Option<f64> {
        match &self.domain {
            Domain::Band(categories) => Some(
                self.band_scale(categories)
                    .apply(text)
                    .unwrap_or(self.get_range()[0]),
            ),
            Domain::Linear(domain) => self.linear_scale(*domain).apply(js_to_number(text)),
        }
    }

    /// `getScaleValue(value)` for a number: its position on a linear axis, `None` when the value is
    /// absent. A band axis gives its range start for any number, as the reference does.
    #[must_use]
    pub fn scale_number(&self, value: Option<f64>) -> Option<f64> {
        match &self.domain {
            Domain::Band(_) => Some(self.get_range()[0]),
            Domain::Linear(domain) => {
                value.and_then(|value| self.linear_scale(*domain).apply(value))
            }
        }
    }

    /// `getScaleValue(tick)`.
    fn scale_tick(&self, tick: &Tick) -> Option<f64> {
        match tick {
            Tick::Text(text) => self.scale_text(text),
            Tick::Value(value) => self.scale_number(Some(*value)),
        }
    }

    /// `getLabelDimension()`: the largest tick label at the label font size.
    fn label_dimension(&self) -> Result<TextBox, RenderError> {
        let texts: Vec<String> = self.tick_values().iter().map(Tick::text).collect();
        max_dimension(&texts, self.config.label_font_size, self.screen_scale)
    }

    /// `recalculateOuterPaddingToDrawBar()`: the outer padding that leaves room for a bar, half of
    /// the bar's share of a tick step, when that is more than the padding has.
    pub fn recalculate_outer_padding_to_draw_bar(&mut self) {
        let tick_distance = self.tick_distance();
        if BAR_WIDTH_TO_TICK_WIDTH_RATIO * tick_distance > self.outer_padding * 2.0 {
            self.outer_padding = ((BAR_WIDTH_TO_TICK_WIDTH_RATIO * tick_distance) / 2.0).floor();
        }
    }

    /// `calculateSpace(availableSpace)`: the space the axis takes, and the flags of what it shows.
    ///
    /// # Errors
    ///
    /// Returns an error if the bundled font cannot be read.
    pub fn calculate_space(&mut self, available: Dimension) -> Result<Dimension, RenderError> {
        if self.position == AxisPosition::Left {
            self.calculate_space_if_drawn_vertical(available)?;
        } else {
            self.calculate_space_if_drawn_horizontally(available)?;
        }
        Ok(Dimension {
            width: self.bounding.width,
            height: self.bounding.height,
        })
    }

    /// `calculateSpaceIfDrawnHorizontally`: the axis is drawn along the bottom or the top.
    fn calculate_space_if_drawn_horizontally(
        &mut self,
        available: Dimension,
    ) -> Result<(), RenderError> {
        let config = self.config.clone();
        let mut available_height = available.height;
        if config.show_axis_line && available_height > config.axis_line_width {
            available_height -= config.axis_line_width;
            self.show_axis_line = true;
        }
        if config.show_label {
            let space = self.label_dimension()?;
            let max_padding = MAX_OUTER_PADDING_PERCENT_FOR_WRT_LABEL * available.width;
            self.outer_padding = (space.width / 2.0).min(max_padding);

            let mut height_required = space.height;
            if self.position == AxisPosition::Bottom && self.rotation != 0.0 {
                height_required = height_required.max(
                    (sin(self.rotation) * space.width).abs()
                        + (cos(self.rotation) * space.height).abs(),
                );
            }
            height_required += config.label_padding * 2.0;

            if height_required <= available_height {
                available_height -= height_required;
                self.show_label = true;
            }
        }
        if config.show_tick && available_height >= config.tick_length {
            self.show_tick = true;
            available_height -= config.tick_length;
        }
        if config.show_title && !self.title.is_empty() {
            let space = max_dimension(
                &[self.title.as_str()],
                config.title_font_size,
                self.screen_scale,
            )?;
            let height_required = space.height + config.title_padding * 2.0;
            self.title_text_height = space.height;
            if height_required <= available_height {
                available_height -= height_required;
                self.show_title = true;
            }
        }
        self.bounding.width = available.width;
        self.bounding.height = available.height - available_height;
        Ok(())
    }

    /// `calculateSpaceIfDrawnVertical`: the axis is drawn along the left.
    fn calculate_space_if_drawn_vertical(
        &mut self,
        available: Dimension,
    ) -> Result<(), RenderError> {
        let config = self.config.clone();
        let mut available_width = available.width;
        if config.show_axis_line && available_width > config.axis_line_width {
            available_width -= config.axis_line_width;
            self.show_axis_line = true;
        }
        if config.show_label {
            let space = self.label_dimension()?;
            let max_padding = MAX_OUTER_PADDING_PERCENT_FOR_WRT_LABEL * available.height;
            self.outer_padding = (space.height / 2.0).min(max_padding);
            let width_required = space.width + config.label_padding * 2.0;
            if width_required <= available_width {
                available_width -= width_required;
                self.show_label = true;
            }
        }
        if config.show_tick && available_width >= config.tick_length {
            self.show_tick = true;
            available_width -= config.tick_length;
        }
        if config.show_title && !self.title.is_empty() {
            let space = max_dimension(
                &[self.title.as_str()],
                config.title_font_size,
                self.screen_scale,
            )?;
            let width_required = space.height + config.title_padding * 2.0;
            self.title_text_height = space.height;
            if width_required <= available_width {
                available_width -= width_required;
                self.show_title = true;
            }
        }
        self.bounding.width = available.width - available_width;
        self.bounding.height = available.height;
        Ok(())
    }

    /// `calculateOffsetByRotation(reference)`: how far a rotated label moves along one axis.
    fn offset_by_rotation(&self, use_height: bool) -> Result<f64, RenderError> {
        if self.rotation == 0.0 {
            return Ok(0.0);
        }
        let label = self.label_dimension()?;
        let extent = if use_height {
            label.height
        } else {
            label.width
        };
        Ok((sin(self.rotation) * extent) / 2.0)
    }

    /// `getDrawableElements()`: the elements of the axis, in the reference's order.
    ///
    /// # Errors
    ///
    /// Returns an error if the font cannot be read.
    pub fn get_drawable_elements(&self) -> Result<Vec<Drawable>, RenderError> {
        match self.position {
            AxisPosition::Left => self.left_elements(),
            AxisPosition::Bottom => self.bottom_elements(),
            AxisPosition::Top => self.top_elements(),
        }
    }

    fn group(classes: &[&str]) -> Vec<String> {
        classes.iter().map(|class| (*class).to_owned()).collect()
    }

    /// The labels, one text per tick.
    fn label_texts(
        &self,
        mut at: impl FnMut(&Tick) -> (f64, f64),
        horizontal: HorizontalPos,
        vertical: VerticalPos,
        rotation: f64,
    ) -> Vec<TextElem> {
        self.tick_values()
            .iter()
            .map(|tick| {
                let (x, y) = at(tick);
                TextElem {
                    text: tick.text(),
                    x,
                    y,
                    fill: self.theme.label_color.clone(),
                    vertical_pos: vertical,
                    horizontal_pos: horizontal,
                    font_size: self.config.label_font_size,
                    rotation,
                }
            })
            .collect()
    }

    fn left_elements(&self) -> Result<Vec<Drawable>, RenderError> {
        let b = self.bounding;
        let c = &self.config;
        let mut out = Vec::new();
        if self.show_axis_line {
            let x = b.x + b.width - c.axis_line_width / 2.0;
            out.push(Drawable::Path {
                groups: Self::group(&["left-axis", "axisl-line"]),
                data: vec![PathElem {
                    path: format!(
                        "M {},{} L {},{} ",
                        js_number(x),
                        js_number(b.y),
                        js_number(x),
                        js_number(b.y + b.height)
                    ),
                    fill: None,
                    stroke_fill: self.theme.axis_line_color.clone(),
                    stroke_width: c.axis_line_width,
                }],
            });
        }
        if self.show_label {
            let label_x = b.x + b.width
                - c.label_padding
                - if self.show_tick { c.tick_length } else { 0.0 }
                - if self.show_axis_line {
                    c.axis_line_width
                } else {
                    0.0
                };
            out.push(Drawable::Text {
                groups: Self::group(&["left-axis", "label"]),
                data: self.label_texts(
                    |tick| (label_x, or_nan(self.scale_tick(tick))),
                    HorizontalPos::Right,
                    VerticalPos::Middle,
                    0.0,
                ),
            });
        }
        if self.show_tick {
            let x = b.x + b.width
                - if self.show_axis_line {
                    c.axis_line_width
                } else {
                    0.0
                };
            let data = self
                .tick_values()
                .iter()
                .map(|tick| {
                    let s = js_number(or_nan(self.scale_tick(tick)));
                    PathElem {
                        path: format!(
                            "M {},{} L {},{}",
                            js_number(x),
                            s,
                            js_number(x - c.tick_length),
                            s
                        ),
                        fill: None,
                        stroke_fill: self.theme.tick_color.clone(),
                        stroke_width: c.tick_width,
                    }
                })
                .collect();
            out.push(Drawable::Path {
                groups: Self::group(&["left-axis", "ticks"]),
                data,
            });
        }
        if self.show_title {
            out.push(Drawable::Text {
                groups: Self::group(&["left-axis", "title"]),
                data: vec![TextElem {
                    text: self.title.clone(),
                    x: b.x + c.title_padding,
                    y: b.y + b.height / 2.0,
                    fill: self.theme.title_color.clone(),
                    vertical_pos: VerticalPos::Top,
                    horizontal_pos: HorizontalPos::Center,
                    font_size: c.title_font_size,
                    rotation: 270.0,
                }],
            });
        }
        Ok(out)
    }

    fn bottom_elements(&self) -> Result<Vec<Drawable>, RenderError> {
        let b = self.bounding;
        let c = &self.config;
        let mut out = Vec::new();
        if self.show_axis_line {
            let y = b.y + c.axis_line_width / 2.0;
            out.push(Drawable::Path {
                groups: Self::group(&["bottom-axis", "axis-line"]),
                data: vec![PathElem {
                    path: format!(
                        "M {},{} L {},{}",
                        js_number(b.x),
                        js_number(y),
                        js_number(b.x + b.width),
                        js_number(y)
                    ),
                    fill: None,
                    stroke_fill: self.theme.axis_line_color.clone(),
                    stroke_width: c.axis_line_width,
                }],
            });
        }
        if self.show_label {
            let offset_x = self.offset_by_rotation(true)?;
            let offset_y = self.offset_by_rotation(false)?.abs();
            let y = b.y
                + c.label_padding
                + if self.show_tick { c.tick_length } else { 0.0 }
                + if self.show_axis_line {
                    c.axis_line_width
                } else {
                    0.0
                }
                + offset_y;
            let rotation = (self.rotation * 180.0) / PI;
            out.push(Drawable::Text {
                groups: Self::group(&["bottom-axis", "label"]),
                data: self.label_texts(
                    |tick| (or_nan(self.scale_tick(tick)) + offset_x, y),
                    HorizontalPos::Center,
                    VerticalPos::Top,
                    rotation,
                ),
            });
        }
        if self.show_tick {
            let y = b.y
                + if self.show_axis_line {
                    c.axis_line_width
                } else {
                    0.0
                };
            let data = self
                .tick_values()
                .iter()
                .map(|tick| {
                    let s = js_number(or_nan(self.scale_tick(tick)));
                    PathElem {
                        path: format!(
                            "M {},{} L {},{}",
                            s,
                            js_number(y),
                            s,
                            js_number(y + c.tick_length)
                        ),
                        fill: None,
                        stroke_fill: self.theme.tick_color.clone(),
                        stroke_width: c.tick_width,
                    }
                })
                .collect();
            out.push(Drawable::Path {
                groups: Self::group(&["bottom-axis", "ticks"]),
                data,
            });
        }
        if self.show_title {
            out.push(Drawable::Text {
                groups: Self::group(&["bottom-axis", "title"]),
                data: vec![TextElem {
                    text: self.title.clone(),
                    x: self.range[0] + (self.range[1] - self.range[0]) / 2.0,
                    y: b.y + b.height - c.title_padding - self.title_text_height,
                    fill: self.theme.title_color.clone(),
                    vertical_pos: VerticalPos::Top,
                    horizontal_pos: HorizontalPos::Center,
                    font_size: c.title_font_size,
                    rotation: 0.0,
                }],
            });
        }
        Ok(out)
    }

    fn top_elements(&self) -> Result<Vec<Drawable>, RenderError> {
        let b = self.bounding;
        let c = &self.config;
        let mut out = Vec::new();
        let line = if self.show_axis_line {
            c.axis_line_width
        } else {
            0.0
        };
        if self.show_axis_line {
            let y = b.y + b.height - c.axis_line_width / 2.0;
            out.push(Drawable::Path {
                groups: Self::group(&["top-axis", "axis-line"]),
                data: vec![PathElem {
                    path: format!(
                        "M {},{} L {},{}",
                        js_number(b.x),
                        js_number(y),
                        js_number(b.x + b.width),
                        js_number(y)
                    ),
                    fill: None,
                    stroke_fill: self.theme.axis_line_color.clone(),
                    stroke_width: c.axis_line_width,
                }],
            });
        }
        if self.show_label {
            let y =
                b.y + if self.show_title {
                    self.title_text_height + c.title_padding * 2.0
                } else {
                    0.0
                } + c.label_padding;
            out.push(Drawable::Text {
                groups: Self::group(&["top-axis", "label"]),
                data: self.label_texts(
                    |tick| (or_nan(self.scale_tick(tick)), y),
                    HorizontalPos::Center,
                    VerticalPos::Top,
                    0.0,
                ),
            });
        }
        if self.show_tick {
            let y = b.y;
            let data = self
                .tick_values()
                .iter()
                .map(|tick| {
                    let s = js_number(or_nan(self.scale_tick(tick)));
                    PathElem {
                        path: format!(
                            "M {},{} L {},{}",
                            s,
                            js_number(y + b.height - line),
                            s,
                            js_number(y + b.height - c.tick_length - line)
                        ),
                        fill: None,
                        stroke_fill: self.theme.tick_color.clone(),
                        stroke_width: c.tick_width,
                    }
                })
                .collect();
            out.push(Drawable::Path {
                groups: Self::group(&["top-axis", "ticks"]),
                data,
            });
        }
        if self.show_title {
            out.push(Drawable::Text {
                groups: Self::group(&["top-axis", "title"]),
                data: vec![TextElem {
                    text: self.title.clone(),
                    x: b.x + b.width / 2.0,
                    y: b.y + c.title_padding,
                    fill: self.theme.title_color.clone(),
                    vertical_pos: VerticalPos::Top,
                    horizontal_pos: HorizontalPos::Center,
                    font_size: c.title_font_size,
                    rotation: 0.0,
                }],
            });
        }
        Ok(out)
    }
}
