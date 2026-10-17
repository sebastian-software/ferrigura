//! The xychart DB, `xychartDb.ts`, with the header setters of `commonDb.ts`, and the conversion of
//! its state to the model. The state and the calls follow `docs-port/xychart/PLAN.md`, sections 4
//! and 5: the x and y ranges, the band, the plots, and the palette that the configuration gives.

use ferrigura_model::xychart::{DataPoint, Plot, XAxis, XyChartDiagram, YAxis};

use crate::sanitize::sanitize_text;
use crate::text::{js_trim, newline_then_space_to_newline, strip_leading_space};

/// The x-axis as the DB holds it: a band of categories, or a linear range. A new axis keeps the
/// title of the old one.
#[derive(Debug, Clone, PartialEq)]
enum XAxisState {
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

/// The y-axis: always linear.
#[derive(Debug, Clone, PartialEq)]
struct YAxisState {
    title: String,
    min: f64,
    max: f64,
}

/// The state the xychart grammar builds, with the configuration the DB read when it was cleared.
#[derive(Debug)]
pub(crate) struct XyChartDb {
    orientation: String,
    title: String,
    acc_title: String,
    acc_descr: String,
    x_axis: XAxisState,
    y_axis: YAxisState,
    /// `hasSetXAxis`: a declared x-axis stops the first plot from setting the range.
    has_set_x: bool,
    /// `hasSetYAxis`: a declared y-range stops the plots from widening it.
    has_set_y: bool,
    plot_index: usize,
    palette: Vec<String>,
    plots: Vec<Plot>,
}

impl XyChartDb {
    /// `clear()` after the configuration is read: the orientation of the configuration, the
    /// palette of the theme (`plotColorPalette`, split at commas and trimmed), and the defaults.
    pub(crate) fn new(orientation: String, plot_color_palette: &str) -> Self {
        Self {
            orientation,
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            x_axis: XAxisState::Band {
                title: String::new(),
                categories: Vec::new(),
            },
            y_axis: YAxisState {
                title: String::new(),
                min: f64::INFINITY,
                max: f64::NEG_INFINITY,
            },
            has_set_x: false,
            has_set_y: false,
            plot_index: 0,
            palette: plot_color_palette
                .split(',')
                .map(|color| js_trim(color).to_owned())
                .collect(),
            plots: Vec::new(),
        }
    }

    /// `setOrientation`: the keyword wins over the configuration. The word is compared as typed, so
    /// `HORIZONTAL` is not horizontal (quirk P2).
    pub(crate) fn set_orientation(&mut self, word: &str) {
        self.orientation = if word == "horizontal" {
            "horizontal".to_owned()
        } else {
            "vertical".to_owned()
        };
    }

    /// `setDiagramTitle` of the common DB: the text is sanitised, and a later call replaces it even
    /// when the result is empty.
    pub(crate) fn set_diagram_title(&mut self, text: &str) {
        self.title = sanitize_text(text);
    }

    /// `setAccTitle` of the common DB: sanitised, without leading white space.
    pub(crate) fn set_acc_title(&mut self, text: &str) {
        self.acc_title = strip_leading_space(&sanitize_text(text)).to_owned();
    }

    /// `setAccDescription` of the common DB: sanitised, with each line break and the white space
    /// after it reduced to one line break.
    pub(crate) fn set_acc_description(&mut self, text: &str) {
        self.acc_descr = newline_then_space_to_newline(&sanitize_text(text));
    }

    /// `textSanitizer`: trimmed, then sanitised. Axis titles, categories, plot titles and labels.
    fn text_sanitizer(text: &str) -> String {
        sanitize_text(js_trim(text))
    }

    fn x_title(&self) -> String {
        match &self.x_axis {
            XAxisState::Band { title, .. } | XAxisState::Linear { title, .. } => title.clone(),
        }
    }

    /// `setXAxisTitle`: the title of the axis, which the next axis setter keeps.
    pub(crate) fn set_x_axis_title(&mut self, text: &str) {
        let title = Self::text_sanitizer(text);
        match &mut self.x_axis {
            XAxisState::Band { title: slot, .. } | XAxisState::Linear { title: slot, .. } => {
                *slot = title;
            }
        }
    }

    /// `setXAxisRangeData`: a linear x-axis from `min` to `max`.
    pub(crate) fn set_x_axis_range(&mut self, min: f64, max: f64) {
        self.x_axis = XAxisState::Linear {
            title: self.x_title(),
            min,
            max,
        };
        self.has_set_x = true;
    }

    /// `setXAxisBand`: a band axis of the categories, each sanitised.
    pub(crate) fn set_x_axis_band(&mut self, categories: &[String]) {
        self.x_axis = XAxisState::Band {
            title: self.x_title(),
            categories: categories
                .iter()
                .map(|category| Self::text_sanitizer(category))
                .collect(),
        };
        self.has_set_x = true;
    }

    /// `setYAxisTitle`.
    pub(crate) fn set_y_axis_title(&mut self, text: &str) {
        self.y_axis.title = Self::text_sanitizer(text);
    }

    /// `setYAxisRangeData`: the y-range as declared.
    pub(crate) fn set_y_axis_range(&mut self, min: f64, max: f64) {
        self.y_axis = YAxisState {
            title: self.y_axis.title.clone(),
            min,
            max,
        };
        self.has_set_y = true;
    }

    /// `setYAxisRangeFromPlotData`: widens the y-range to cover the values. It does not count as a
    /// declared range.
    fn widen_y_axis(&mut self, data: &[f64]) {
        let min_value = data.iter().copied().fold(f64::INFINITY, f64::min);
        let max_value = data.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        self.y_axis = YAxisState {
            title: self.y_axis.title.clone(),
            min: self.y_axis.min.min(min_value),
            max: self.y_axis.max.max(max_value),
        };
    }

    /// `transformDataWithoutCategory`: the `[x, y]` pairs of one plot, as the DB makes them when
    /// the plot is declared (quirks D1 to D5, and the model's rules for the x strings).
    fn transform(&mut self, mut data: Vec<f64>) -> Vec<DataPoint> {
        if data.is_empty() {
            return Vec::new();
        }
        if !self.has_set_x {
            let (prev_min, prev_max) = match &self.x_axis {
                XAxisState::Linear { min, max, .. } => (*min, *max),
                XAxisState::Band { .. } => (f64::INFINITY, f64::NEG_INFINITY),
            };
            let count = data.len() as f64;
            self.set_x_axis_range(prev_min.min(1.0), prev_max.max(count));
        }

        // A band axis keeps only as many values as it has categories.
        if let XAxisState::Band { categories, .. } = &self.x_axis {
            data.truncate(categories.len());
        }

        if !self.has_set_y {
            self.widen_y_axis(&data);
        }

        match &self.x_axis {
            XAxisState::Band { categories, .. } => categories
                .iter()
                .enumerate()
                .map(|(index, category)| DataPoint {
                    x: category.clone(),
                    y: data.get(index).copied(),
                })
                .collect(),
            XAxisState::Linear { min, max, .. } => {
                if let [only] = data.as_slice() {
                    vec![DataPoint {
                        x: js_string(*min),
                        y: Some(*only),
                    }]
                } else {
                    let step = (max - min) / (data.len() - 1) as f64;
                    data.iter()
                        .enumerate()
                        .map(|(index, &value)| DataPoint {
                            x: js_string(min + index as f64 * step),
                            y: Some(value),
                        })
                        .collect()
                }
            }
        }
    }

    /// The palette colour of the next plot: `palette[index % length]`.
    fn next_colour(&self) -> String {
        self.palette[self.plot_index % self.palette.len()].clone()
    }

    /// `setLineData`: a line plot, with its labels when one of them is not empty.
    pub(crate) fn set_line_data(&mut self, title: &str, points: &[(f64, String)]) {
        let values: Vec<f64> = points.iter().map(|(value, _)| *value).collect();
        let labels: Vec<String> = points
            .iter()
            .map(|(_, label)| {
                if label.is_empty() {
                    String::new()
                } else {
                    Self::text_sanitizer(label)
                }
            })
            .collect();
        let data = self.transform(values);
        let has_any_label = labels.iter().any(|label| !label.is_empty());
        let stroke_fill = self.next_colour();
        self.plots.push(Plot::Line {
            title: Self::text_sanitizer(title),
            stroke_fill,
            stroke_width: 2.0,
            data,
            point_labels: has_any_label.then_some(labels),
        });
        self.plot_index += 1;
    }

    /// `setBarData`: a bar plot. Its labels are not kept.
    pub(crate) fn set_bar_data(&mut self, title: &str, points: &[(f64, String)]) {
        let values: Vec<f64> = points.iter().map(|(value, _)| *value).collect();
        let data = self.transform(values);
        let fill = self.next_colour();
        self.plots.push(Plot::Bar {
            title: Self::text_sanitizer(title),
            fill,
            data,
        });
        self.plot_index += 1;
    }

    /// The model: the DB state after the parse.
    pub(crate) fn into_model(self) -> XyChartDiagram {
        let mut model = XyChartDiagram::new();
        model.orientation = self.orientation;
        model.title = self.title;
        model.acc_title = self.acc_title;
        model.acc_descr = self.acc_descr;
        model.x_axis = match self.x_axis {
            XAxisState::Band { title, categories } => XAxis::Band { title, categories },
            XAxisState::Linear { title, min, max } => XAxis::Linear { title, min, max },
        };
        model.y_axis = YAxis::Linear {
            title: self.y_axis.title,
            min: self.y_axis.min,
            max: self.y_axis.max,
        };
        model.plots = self.plots;
        model
    }
}

/// `String(value)` of a JavaScript number: the shortest digits that read back to the same value,
/// in plain notation between `1e-7` and `1e21` (exclusive of the first) and in exponent notation
/// otherwise, as ECMAScript's `Number::toString` writes them.
pub(crate) fn js_string(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value == 0.0 {
        return "0".to_owned();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    let sign = if value < 0.0 { "-" } else { "" };
    // `{:e}` gives the shortest round-trip digits as `d.ddde<exp>`.
    let shortest = format!("{:e}", value.abs());
    let (mantissa, exponent) = shortest.split_once('e').unwrap_or((shortest.as_str(), "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let count = digits.len() as i32;
    let point = exponent + 1;
    let body = if count <= point && point <= 21 {
        format!("{digits}{}", "0".repeat((point - count) as usize))
    } else if 0 < point && point <= 21 {
        let (whole, fraction) = digits.split_at(point as usize);
        format!("{whole}.{fraction}")
    } else if -6 < point && point <= 0 {
        format!("0.{}{digits}", "0".repeat((-point) as usize))
    } else {
        let exp = point - 1;
        let exp_sign = if exp >= 0 { "+" } else { "-" };
        let (head, rest) = digits.split_at(1);
        if rest.is_empty() {
            format!("{head}e{exp_sign}{}", exp.abs())
        } else {
            format!("{head}.{rest}e{exp_sign}{}", exp.abs())
        }
    };
    format!("{sign}{body}")
}

#[cfg(test)]
mod tests {
    use super::js_string;

    #[test]
    fn numbers_print_as_javascript_prints_them() {
        assert_eq!(js_string(0.0), "0");
        assert_eq!(js_string(-0.0), "0");
        assert_eq!(js_string(1.0), "1");
        assert_eq!(js_string(-2.5), "-2.5");
        assert_eq!(js_string(1.666_666_666_666_666_7), "1.6666666666666667");
        assert_eq!(js_string(1e-7), "1e-7");
        assert_eq!(js_string(1.2e-7), "1.2e-7");
        assert_eq!(js_string(0.000001), "0.000001");
        assert_eq!(js_string(1e21), "1e+21");
        assert_eq!(js_string(1e20), "100000000000000000000");
        assert_eq!(js_string(f64::INFINITY), "Infinity");
    }
}
