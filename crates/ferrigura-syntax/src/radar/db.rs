//! The radar DB, `packages/mermaid/src/diagrams/radar/db.ts`: the state the `populate` calls
//! change, with the reference's checks and messages. The header setters are the shared ones of
//! `diagrams/common/commonDb.ts`, which sanitise, and the options keep the last value given per name.
//! The order of the calls is in `docs-port/radar/PLAN.md`, section 3.

use ferrigura_model::CANONICAL_VERSION;
use ferrigura_model::radar::{RadarAxis, RadarCurve, RadarDiagram, RadarOptions};

use super::grammar::{AxisStmt, CurveStmt, Entries, OptionStmt};
use crate::sanitize::sanitize_text;
use crate::text::{newline_then_space_to_newline, strip_leading_space};

/// The ticks above this are set to it. The reference's warning is silent at its default log level.
const MAX_TICKS: f64 = 32.0;

/// The state of a radar diagram while its statements are applied.
pub(crate) struct Db {
    title: String,
    acc_title: String,
    acc_descr: String,
    axes: Vec<RadarAxis>,
    curves: Vec<RadarCurve>,
    options: RadarOptions,
}

impl Db {
    /// A cleared DB, as `clear()` leaves it.
    pub(crate) fn new() -> Self {
        Self {
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            axes: Vec::new(),
            curves: Vec::new(),
            options: RadarOptions::default(),
        }
    }

    /// `setDiagramTitle`: the text is sanitised.
    pub(crate) fn set_title(&mut self, text: &str) {
        self.title = sanitize_text(text);
    }

    /// `setAccTitle`: the text is sanitised, and its leading whitespace is removed.
    pub(crate) fn set_acc_title(&mut self, text: &str) {
        self.acc_title = strip_leading_space(&sanitize_text(text)).to_owned();
    }

    /// `setAccDescription`: the text is sanitised, and the whitespace after each line break is
    /// removed.
    pub(crate) fn set_acc_descr(&mut self, text: &str) {
        self.acc_descr = newline_then_space_to_newline(&sanitize_text(text));
    }

    /// `setAxes`: one axis per statement, in text order. The label is the name when none is given.
    pub(crate) fn set_axes(&mut self, axes: &[AxisStmt]) {
        self.axes = axes
            .iter()
            .map(|axis| RadarAxis {
                name: axis.name.clone(),
                label: axis.label.clone().unwrap_or_else(|| axis.name.clone()),
            })
            .collect();
    }

    /// `setCurves`: one curve per statement, with its entries computed against the axes that are
    /// set already, so the axes must come first. The error is the offset of the curve that failed,
    /// with the message the reference throws.
    pub(crate) fn set_curves(&mut self, curves: &[CurveStmt]) -> Result<(), (usize, String)> {
        let mut result = Vec::with_capacity(curves.len());
        for curve in curves {
            let entries = self
                .compute_entries(&curve.entries)
                .map_err(|message| (curve.offset, message))?;
            result.push(RadarCurve {
                name: curve.name.clone(),
                label: curve.label.clone().unwrap_or_else(|| curve.name.clone()),
                entries,
            });
        }
        self.curves = result;
        Ok(())
    }

    /// `computeCurveEntries`: a number list is kept as it is, whatever its length. A list of
    /// detailed entries gives one value per axis, in axis order, the first entry that names the
    /// axis. An entry that names no axis is dropped.
    fn compute_entries(&self, entries: &Entries) -> Result<Vec<f64>, String> {
        match entries {
            Entries::Numbers(values) => Ok(values.clone()),
            Entries::Named(named) => {
                if self.axes.is_empty() {
                    return Err("Axes must be populated before curves for reference entries".into());
                }
                self.axes
                    .iter()
                    .map(|axis| {
                        named
                            .iter()
                            .find(|entry| entry.axis == axis.name)
                            .map(|entry| entry.value)
                            .ok_or_else(|| format!("Missing entry for axis {}", axis.label))
                    })
                    .collect()
            }
        }
    }

    /// `setOptions`: the last option of each name wins, and each option not given keeps its
    /// default. `ticks` above the maximum is set to the maximum.
    pub(crate) fn set_options(&mut self, options: &[OptionStmt]) {
        let mut result = RadarOptions::default();
        for option in options {
            match option {
                OptionStmt::ShowLegend(value) => result.show_legend = *value,
                OptionStmt::Ticks(value) => result.ticks = *value,
                OptionStmt::Max(value) => result.max = Some(*value),
                OptionStmt::Min(value) => result.min = *value,
                OptionStmt::Graticule(shape) => result.graticule = *shape,
            }
        }
        if result.ticks > MAX_TICKS {
            result.ticks = MAX_TICKS;
        }
        self.options = result;
    }

    /// The model of the DB's state.
    pub(crate) fn into_model(self) -> RadarDiagram {
        RadarDiagram {
            version: CANONICAL_VERSION,
            title: self.title,
            acc_title: self.acc_title,
            acc_descr: self.acc_descr,
            axes: self.axes,
            curves: self.curves,
            options: self.options,
        }
    }
}
