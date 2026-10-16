//! The axis of d3-axis 3.0.0 as the gantt draws it: `axisBottom` and `axisTop` with the tick size,
//! the tick padding of 3, and the format of the tick text (`d3-axis/src/axis.js`, read for version
//! 3.0.0). The gantt then restyles the tick text (`ganttRenderer.js:723-728` and `781-790`).

use super::format::utc_format;
use super::scale::TimeScale;
use crate::svg::{escape, number};

/// Which side of the chart the axis is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Orient {
    /// `axisTop`: the ticks point down from the top grid line.
    Top,
    /// `axisBottom`: the ticks point up from the bottom grid line.
    Bottom,
}

/// The tick padding of d3-axis (`tickPadding`, 3 by default).
const TICK_PADDING: f64 = 3.0;

/// What the gantt sets on one axis.
pub(crate) struct Axis<'a> {
    /// The side of the axis.
    pub(crate) orient: Orient,
    /// The scale the ticks are read from.
    pub(crate) scale: &'a TimeScale,
    /// `tickSize` (inner and outer): the negative height of the grid lines.
    pub(crate) tick_size: f64,
    /// The half pixel that d3-axis adds to crisp lines: 0.5 at a device pixel ratio of 1, 0 above.
    pub(crate) offset: f64,
    /// The tick values, as the scale's ticks or a custom interval gives them.
    pub(crate) values: Vec<f64>,
    /// The `tickFormat` specifier: `timeFormat(axisFormat)`.
    pub(crate) format: &'a str,
    /// The fill of the tick text, written as the gantt's presentation attribute.
    pub(crate) text_fill: &'a str,
}

impl Axis<'_> {
    /// The axis's ticks in the form the gantt writes: the `g.grid` group with the `transform` the
    /// caller gives, then the domain path and one group per tick (`d3-axis/src/axis.js:31-130`).
    pub(crate) fn group(&self, transform: &str) -> String {
        let k = match self.orient {
            Orient::Top => -1.0,
            Orient::Bottom => 1.0,
        };
        let spacing = self.tick_size.max(0.0) + TICK_PADDING;
        let range0 = self.offset;
        let range1 = self.scale.range_max() + self.offset;
        let outer = k * self.tick_size;
        let path = if self.tick_size != 0.0 && !self.tick_size.is_nan() {
            format!(
                "M{},{}V{}H{}V{}",
                number(range0),
                number(outer),
                number(self.offset),
                number(range1),
                number(outer)
            )
        } else {
            format!(
                "M{},{}H{}",
                number(range0),
                number(self.offset),
                number(range1)
            )
        };
        let mut out = format!(
            "<g class=\"grid\" transform=\"{}\" fill=\"none\" font-size=\"10\" font-family=\"sans-serif\" text-anchor=\"middle\">",
            escape(transform)
        );
        out.push_str(&format!(
            "<path class=\"domain\" stroke=\"currentColor\" d=\"{path}\"></path>"
        ));
        for &value in &self.values {
            let position = self.scale.apply(value) + self.offset;
            let text = escape(&utc_format(self.format, value));
            // The gantt restyles the bottom text to `1em`; the top text keeps d3's `0em`.
            let dy = match self.orient {
                Orient::Top => "0em",
                Orient::Bottom => "1em",
            };
            let y = k * spacing;
            out.push_str(&format!(
                "<g class=\"tick\" opacity=\"1\" transform=\"translate({},0)\"><line stroke=\"currentColor\" y2=\"{}\"></line><text fill=\"{}\" y=\"{}\" dy=\"{}\" stroke=\"none\" font-size=\"10\" style=\"text-anchor: middle;\">{}</text></g>",
                number(position),
                number(k * self.tick_size),
                escape(self.text_fill),
                number(y),
                dy,
                text,
            ));
        }
        out.push_str("</g>");
        out
    }
}
