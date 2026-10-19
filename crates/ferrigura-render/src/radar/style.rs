//! The colours and sizes of a radar diagram: `styles.ts` (the rules of `styles`, lines 26-75, and
//! `genIndexStyles`, lines 3-29) with the options of `themeVariables.radar`.
//!
//! The reference merges the default theme's `radar` section under the active theme's and the
//! user's (`styles.ts` 35-38, `theme-default.js` 338-350). The redux themes give no `radar` section,
//! so a diagram draws with the defaults below unless `themeVariables.radar` sets a value. The defaults
//! are the values the default theme computes, and the axis colour is its `lineColor`, `#333333`
//! (`theme-default.js` 27).
//!
//! The colours are custom properties declared on the diagram's id, one set per colour scheme, and the
//! rules refer to them with `var(…)` (`docs-port/decisions/0007`). The presentation attributes of the
//! elements carry the light scheme's values, so a viewer without custom properties draws the colours
//! too. A value the reference would write as `undefined` (a colour the theme does not give) is not
//! written, which draws the same way: the reference's declaration is invalid and is dropped.

use ferrigura_model::theme::tokens::RadarTokens;
use ferrigura_model::theme::{ThemeTokens, Token};

use crate::common::template_text;
use crate::context::ThemeView;

/// `axisColor`: the default theme's `lineColor` (`theme-default.js` 27).
const AXIS_COLOR: &str = "#333333";
/// `axisStrokeWidth` (`theme-default.js` 341).
const AXIS_STROKE_WIDTH: &str = "2";
/// `axisLabelFontSize` (`theme-default.js` 342).
const AXIS_LABEL_FONT_SIZE: &str = "12";
/// `curveOpacity` (`theme-default.js` 343).
const CURVE_OPACITY: &str = "0.5";
/// `curveStrokeWidth` (`theme-default.js` 344).
const CURVE_STROKE_WIDTH: &str = "2";
/// `graticuleColor` (`theme-default.js` 345).
const GRATICULE_COLOR: &str = "#DEDEDE";
/// `graticuleOpacity` (`theme-default.js` 347).
const GRATICULE_OPACITY: &str = "0.3";
/// `graticuleStrokeWidth` (`theme-default.js` 346).
const GRATICULE_STROKE_WIDTH: &str = "1";
/// `legendFontSize` (`theme-default.js` 349).
const LEGEND_FONT_SIZE: &str = "12";

/// The custom property of the axis colour.
const AXIS_VAR: &str = "--radarAxisColor";
/// The custom property of the graticule colour.
const GRATICULE_VAR: &str = "--radarGraticuleColor";
/// The custom property of the title colour.
const TITLE_VAR: &str = "--radarTitleColor";

/// The custom property of the colour of curve `index`: the `cScale<index>` of the theme.
fn curve_var(index: usize) -> String {
    format!("--radarCurveColor{index}")
}

/// The options of one colour scheme, as the reference's template of `styles.ts` writes them.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Options {
    /// `titleColor`, when the theme gives one.
    pub title_color: Option<String>,
    /// `fontSize`, the size of the title, when the theme gives one.
    pub font_size: Option<String>,
    /// `radar.axisColor`: the axis lines and the axis labels.
    pub axis_color: String,
    /// `radar.axisStrokeWidth`.
    pub axis_stroke_width: String,
    /// `radar.axisLabelFontSize`, in pixels.
    pub axis_label_font_size: String,
    /// `radar.curveOpacity`: the opacity of the curves and of their legend boxes.
    pub curve_opacity: String,
    /// `radar.curveStrokeWidth`.
    pub curve_stroke_width: String,
    /// `radar.graticuleColor`.
    pub graticule_color: String,
    /// `radar.graticuleOpacity`.
    pub graticule_opacity: String,
    /// `radar.graticuleStrokeWidth`.
    pub graticule_stroke_width: String,
    /// `radar.legendFontSize`, in pixels. `legendBoxSize` is not read (`radar/NOTES.md` O2).
    pub legend_font_size: String,
    /// `cScale<i>` for each `i` below the colour limit, when the theme gives it.
    pub curve_colours: Vec<Option<String>>,
}

impl Options {
    /// The options of a theme's tokens, with `limit` curve colours. Each `radar` option is the theme's
    /// value when the group gives one, and the default otherwise.
    fn of(tokens: &ThemeTokens, limit: usize) -> Self {
        let group = tokens.radar.as_ref();
        let read = |token: fn(&RadarTokens) -> &Token, default: &str| -> String {
            group
                .and_then(|options| token(options).value())
                .map_or_else(|| default.to_owned(), template_text)
        };
        Self {
            title_color: tokens.title_color.value().map(template_text),
            font_size: tokens.font_size.value().map(template_text),
            axis_color: read(|options| &options.axis_color, AXIS_COLOR),
            axis_stroke_width: read(|options| &options.axis_stroke_width, AXIS_STROKE_WIDTH),
            axis_label_font_size: read(
                |options| &options.axis_label_font_size,
                AXIS_LABEL_FONT_SIZE,
            ),
            curve_opacity: read(|options| &options.curve_opacity, CURVE_OPACITY),
            curve_stroke_width: read(|options| &options.curve_stroke_width, CURVE_STROKE_WIDTH),
            graticule_color: read(|options| &options.graticule_color, GRATICULE_COLOR),
            graticule_opacity: read(|options| &options.graticule_opacity, GRATICULE_OPACITY),
            graticule_stroke_width: read(
                |options| &options.graticule_stroke_width,
                GRATICULE_STROKE_WIDTH,
            ),
            legend_font_size: read(|options| &options.legend_font_size, LEGEND_FONT_SIZE),
            curve_colours: (0..limit)
                .map(|index| tokens.c_scale.get(index).value().map(template_text))
                .collect(),
        }
    }

    /// The custom properties of the scheme, as `--name:value;` pairs. A value that the theme does not
    /// give is not declared.
    fn declarations(&self) -> String {
        let mut declared = String::new();
        if let Some(colour) = &self.title_color {
            declared.push_str(&format!("{TITLE_VAR}:{colour};"));
        }
        declared.push_str(&format!(
            "{AXIS_VAR}:{};{GRATICULE_VAR}:{};",
            self.axis_color, self.graticule_color
        ));
        for (index, colour) in self.curve_colours.iter().enumerate() {
            if let Some(colour) = colour {
                declared.push_str(&format!("{}:{colour};", curve_var(index)));
            }
        }
        declared
    }
}

/// The options of a render: the light scheme, which gives the sizes and the presentation attributes,
/// the dark scheme when the SVG carries both, and the number of curve colours.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Style {
    /// The number of curve colours, which the reference's `THEME_COLOR_LIMIT` gives (`styles.ts` 12).
    pub limit: usize,
    /// The light scheme's options.
    pub light: Options,
    /// The dark scheme's options, when the SVG carries both schemes.
    pub dark: Option<Options>,
}

impl Style {
    /// The style of a theme view.
    pub(super) fn of(theme: ThemeView<'_>) -> Self {
        let limit = colour_limit(theme.light.theme_color_limit.value());
        Self {
            limit,
            light: Options::of(theme.light, limit),
            dark: theme.dark.map(|tokens| Options::of(tokens, limit)),
        }
    }
}

/// The number of curve colours: the reference's `for (let i = 0; i < THEME_COLOR_LIMIT; i++)`
/// (`styles.ts` 12). The theme's `THEME_COLOR_LIMIT` is 12 when the theme gives none (the default
/// theme's value, `theme-default.js` 36). The value is read as JavaScript's `<` reads it.
fn colour_limit(value: Option<&ferrigura_model::value::Value>) -> usize {
    use ferrigura_model::value::Value;
    let number = match value {
        None => 12.0,
        Some(Value::Number(number)) => *number,
        Some(Value::Bool(flag)) => f64::from(u8::from(*flag)),
        Some(Value::Null) => 0.0,
        Some(Value::String(text)) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                0.0
            } else {
                trimmed
                    .parse::<f64>()
                    .ok()
                    .filter(|number| number.is_finite())
                    .unwrap_or(f64::NAN)
            }
        }
        Some(Value::Array(_) | Value::Object(_)) => f64::NAN,
    };
    if number.is_nan() || number <= 0.0 {
        0
    } else {
        number.ceil() as usize
    }
}

/// The rules of the radar diagram, as `styles.ts` writes them after the base rules of the diagram:
/// the colour custom properties on the id (one block for each scheme), then the rules of the title,
/// the axes, the graticule, the legend and the curves. Every rule is scoped to the id.
pub(super) fn own_rules(id: &str, style: &Style) -> String {
    let light = &style.light;
    let mut css = format!("#{id}{{{}}}", light.declarations());
    if let Some(dark) = &style.dark {
        css.push_str(&format!(
            "@media (prefers-color-scheme: dark){{#{id}{{{}}}}}",
            dark.declarations()
        ));
    }
    let font_size = light
        .font_size
        .as_ref()
        .map(|size| format!("font-size:{size};"))
        .unwrap_or_default();
    css.push_str(&format!(
        "#{id} .radarTitle{{{font_size}color:var({TITLE_VAR});dominant-baseline:hanging;text-anchor:middle;}}\
#{id} .radarAxisLine{{stroke:var({AXIS_VAR});stroke-width:{};}}\
#{id} .radarAxisLabel{{font-size:{}px;color:var({AXIS_VAR});}}\
#{id} .radarGraticule{{fill:var({GRATICULE_VAR});fill-opacity:{};stroke:var({GRATICULE_VAR});stroke-width:{};}}\
#{id} .radarLegendText{{text-anchor:start;font-size:{}px;dominant-baseline:hanging;}}",
        light.axis_stroke_width,
        light.axis_label_font_size,
        light.graticule_opacity,
        light.graticule_stroke_width,
        light.legend_font_size,
    ));
    for index in 0..style.limit {
        let colour = curve_var(index);
        css.push_str(&format!(
            "#{id} .radarCurve-{index}{{color:var({colour});fill:var({colour});fill-opacity:{};stroke:var({colour});stroke-width:{};}}\
#{id} .radarLegendBox-{index}{{fill:var({colour});fill-opacity:{};stroke:var({colour});}}",
            light.curve_opacity, light.curve_stroke_width, light.curve_opacity,
        ));
    }
    css
}

#[cfg(test)]
mod tests {
    use ferrigura_model::value::Value;

    use super::*;

    /// The limit of the theme's `THEME_COLOR_LIMIT`, read as the loop reads it.
    #[test]
    fn the_colour_limit_is_read_as_the_loop_reads_it() {
        assert_eq!(colour_limit(None), 12);
        assert_eq!(colour_limit(Some(&Value::Number(12.0))), 12);
        assert_eq!(colour_limit(Some(&Value::Number(2.5))), 3);
        assert_eq!(colour_limit(Some(&Value::Number(0.0))), 0);
        assert_eq!(colour_limit(Some(&Value::Number(-3.0))), 0);
        assert_eq!(colour_limit(Some(&Value::from("5"))), 5);
        assert_eq!(colour_limit(Some(&Value::from(" "))), 0);
        assert_eq!(colour_limit(Some(&Value::from("many"))), 0);
        assert_eq!(colour_limit(Some(&Value::Null)), 0);
        assert_eq!(colour_limit(Some(&Value::Bool(true))), 1);
    }
}
