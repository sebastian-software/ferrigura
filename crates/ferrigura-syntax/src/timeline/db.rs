//! The timeline DB, `timelineDb.js`, with the setters of the common DB for the header texts. The
//! DB's state is the model; the rules are in `docs-port/timeline/PLAN.md`, section 4.

use ferrigura_model::timeline::{TimelineDiagram, TimelinePeriod};

use crate::sanitize::sanitize_text;
use crate::text::{newline_then_space_to_newline, strip_leading_space};

/// The state the grammar's actions build up.
#[derive(Debug, Default)]
pub(crate) struct Db {
    diagram: TimelineDiagram,
    current_section: String,
}

impl Db {
    /// An empty DB, in the state `clear` leaves it.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// The model the actions built.
    pub(crate) fn into_model(self) -> TimelineDiagram {
        self.diagram
    }

    /// `setDirection`.
    pub(crate) fn set_direction(&mut self, direction: &str) {
        self.diagram.direction = direction.to_owned();
    }

    /// `setDiagramTitle` of the common DB: sanitised, and not trimmed.
    pub(crate) fn set_title(&mut self, text: &str) {
        self.diagram.title = sanitize_text(text);
    }

    /// `setAccTitle`: sanitised, then without leading whitespace.
    pub(crate) fn set_acc_title(&mut self, text: &str) {
        self.diagram.acc_title = strip_leading_space(&sanitize_text(text)).to_owned();
    }

    /// `setAccDescription`: sanitised, then each line break followed by whitespace becomes one
    /// line break.
    pub(crate) fn set_acc_descr(&mut self, text: &str) {
        self.diagram.acc_descr = newline_then_space_to_newline(&sanitize_text(text));
    }

    /// `addSection`: the name becomes the current section, and is kept even when it repeats.
    pub(crate) fn add_section(&mut self, name: &str) {
        self.current_section = name.to_owned();
        self.diagram.sections.push(name.to_owned());
    }

    /// `addTask`: a period of the current section, with no events yet.
    pub(crate) fn add_period(&mut self, period: &str) {
        self.diagram.periods.push(TimelinePeriod {
            section: self.current_section.clone(),
            period: period.to_owned(),
            events: Vec::new(),
        });
    }

    /// `addEvent`: appended to the last period. Returns `false` when no period exists yet, which
    /// the reference reports as an error.
    pub(crate) fn add_event(&mut self, event: &str) -> bool {
        match self.diagram.periods.last_mut() {
            Some(period) => {
                period.events.push(event.to_owned());
                true
            }
            None => false,
        }
    }
}
