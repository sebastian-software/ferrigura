//! The data the chart builder lays out and the drawable elements it returns: the reference's
//! `chartBuilder/interfaces.ts`. The model's values are read into these types by `config.rs`.
//!
//! A value that the reference may leave `undefined` is an `Option`: a bar's `y` is written only when
//! the value is there, and `attr` removes an attribute given `undefined`. An arithmetic result that
//! the reference computes from an `undefined` is `NaN`, and is written as `NaN`.

/// The margins and sizes of the chart, as the reference's `XYChartConfig` gives them.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartConfig {
    pub width: f64,
    /// The scale of the SVG on the screen, at which the texts are measured (`config.rs`).
    pub screen_scale: f64,
    pub height: f64,
    pub title_font_size: f64,
    pub title_padding: f64,
    pub show_title: bool,
    pub show_legend: bool,
    pub legend_font_size: f64,
    pub legend_padding: f64,
    pub show_data_label: bool,
    pub show_data_label_outside_bar: bool,
    pub x_axis: AxisConfig,
    pub y_axis: AxisConfig,
    /// `chartOrientation` is `horizontal`.
    pub horizontal: bool,
    pub plot_reserved_space_percent: f64,
}

/// The settings of one axis (`XYChartAxisConfig`).
#[derive(Debug, Clone, PartialEq)]
pub struct AxisConfig {
    pub show_label: bool,
    pub label_font_size: f64,
    pub label_padding: f64,
    pub show_title: bool,
    pub title_font_size: f64,
    pub title_padding: f64,
    pub show_tick: bool,
    pub tick_length: f64,
    pub tick_width: f64,
    pub show_axis_line: bool,
    pub axis_line_width: f64,
    pub label_rotation: f64,
}

/// The colours of the chart, from the theme's `xyChart` values.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeConfig {
    pub background_color: String,
    pub title_color: String,
    pub data_label_color: String,
    pub legend_text_color: String,
    pub x_axis: AxisThemeConfig,
    pub y_axis: AxisThemeConfig,
}

/// The colours of one axis.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisThemeConfig {
    pub title_color: String,
    pub label_color: String,
    pub tick_color: String,
    pub axis_line_color: String,
}

/// An axis of the data: a band of categories, or a linear range.
#[derive(Debug, Clone, PartialEq)]
pub enum AxisData {
    Band {
        title: String,
        categories: Vec<String>,
    },
    Linear {
        title: String,
        min: f64,
        max: f64,
    },
}

/// One point of a plot: the x value as the DB stores it (a category, or the JavaScript string of a
/// linear position) and the y value, `None` for an absent one.
#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    pub x: String,
    pub y: Option<f64>,
}

/// A plot of the data: a line or a bar.
#[derive(Debug, Clone, PartialEq)]
pub enum PlotData {
    Line {
        title: String,
        stroke_fill: String,
        stroke_width: f64,
        data: Vec<Point>,
        point_labels: Option<Vec<String>>,
    },
    Bar {
        title: String,
        fill: String,
        data: Vec<Point>,
    },
}

impl PlotData {
    /// The title of the plot, `""` when it has none.
    #[must_use]
    pub fn title(&self) -> &str {
        match self {
            Self::Line { title, .. } | Self::Bar { title, .. } => title,
        }
    }

    /// The points of the plot.
    #[must_use]
    pub fn data(&self) -> &[Point] {
        match self {
            Self::Line { data, .. } | Self::Bar { data, .. } => data,
        }
    }
}

/// The chart as the DB leaves it: its axes, its title and its plots in the order declared.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartData {
    pub x_axis: AxisData,
    pub y_axis: AxisData,
    pub title: String,
    pub plots: Vec<PlotData>,
}

/// A box: the position of its top left corner and its size (the reference's `BoundingRect`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// A width and a height.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Dimension {
    pub width: f64,
    pub height: f64,
}

/// A bar or a legend marker: its position, its size, and its fill and stroke (`RectElem`).
#[derive(Debug, Clone, PartialEq)]
pub struct RectElem {
    pub x: f64,
    /// `None` for a bar whose value is absent: the reference writes no `y` attribute.
    pub y: Option<f64>,
    pub width: f64,
    pub height: f64,
    pub fill: String,
    pub stroke_fill: String,
    pub stroke_width: f64,
}

/// Where a text sits vertically (`TextVerticalPos`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalPos {
    Top,
    Middle,
}

/// Where a text sits horizontally (`TextHorizontalPos`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalPos {
    Left,
    Center,
    Right,
}

/// A text: its anchor, the rotation about it, and its font size.
#[derive(Debug, Clone, PartialEq)]
pub struct TextElem {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub fill: String,
    pub vertical_pos: VerticalPos,
    pub horizontal_pos: HorizontalPos,
    pub font_size: f64,
    pub rotation: f64,
}

/// A line or an axis path. A path without a fill is written with `fill="none"`.
#[derive(Debug, Clone, PartialEq)]
pub struct PathElem {
    pub path: String,
    pub fill: Option<String>,
    pub stroke_fill: String,
    pub stroke_width: f64,
}

/// A shape to draw: its elements, and the classes of the groups they sit in (`groupTexts`).
#[derive(Debug, Clone, PartialEq)]
pub enum Drawable {
    Rect {
        groups: Vec<String>,
        data: Vec<RectElem>,
    },
    Text {
        groups: Vec<String>,
        data: Vec<TextElem>,
    },
    Path {
        groups: Vec<String>,
        data: Vec<PathElem>,
    },
}

impl Drawable {
    /// The classes of the groups the elements sit in.
    #[must_use]
    pub fn groups(&self) -> &[String] {
        match self {
            Self::Rect { groups, .. } | Self::Text { groups, .. } | Self::Path { groups, .. } => {
                groups
            }
        }
    }

    /// The number of elements of the group.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Rect { data, .. } => data.len(),
            Self::Text { data, .. } => data.len(),
            Self::Path { data, .. } => data.len(),
        }
    }

    /// Whether the group has no element; the reference skips such a shape.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
