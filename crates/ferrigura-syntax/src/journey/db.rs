//! The journey DB, `journeyDb.js`, with the header setters of `commonDb.ts`, and the conversion of
//! its values to the model.
//!
//! The DB keeps what the grammar gives it: the texts are encoded (`ﬂ°name¶ß` for an entity, see
//! [`crate::preprocess::encode_entities`]), and the header texts are sanitised. The model holds the
//! text the reader sees, which [`display_text`] computes: see `docs-port/journey/PLAN.md`, section 6.

use ferrigura_model::journey::{JourneyDiagram, JourneyTask};

use crate::sanitize::{decode_references, sanitize_text};
use crate::text::{js_number, js_trim, newline_then_space_to_newline, strip_leading_space};

/// The state the journey grammar builds: the header texts, the sections and the tasks.
#[derive(Debug, Default)]
pub(crate) struct JourneyDb {
    title: String,
    acc_title: String,
    acc_descr: String,
    current_section: String,
    sections: Vec<String>,
    tasks: Vec<RawTask>,
}

/// A task as the DB holds it: its section is the one current when it was given.
#[derive(Debug)]
struct RawTask {
    section: String,
    task: String,
    people: Vec<String>,
    score: f64,
}

impl JourneyDb {
    /// `setDiagramTitle`: the last title given wins, even when it sanitises to nothing (J5, J14).
    pub(crate) fn set_diagram_title(&mut self, text: &str) {
        self.title = sanitize_text(text);
    }

    /// `setAccTitle`: sanitised, without leading white space.
    pub(crate) fn set_acc_title(&mut self, text: &str) {
        self.acc_title = strip_leading_space(&sanitize_text(text)).to_owned();
    }

    /// `setAccDescription`: sanitised, with the white space after each line break removed.
    pub(crate) fn set_acc_description(&mut self, text: &str) {
        self.acc_descr = newline_then_space_to_newline(&sanitize_text(text));
    }

    /// `addSection`: the name is kept as it is, duplicates included (J6).
    pub(crate) fn add_section(&mut self, text: &str) {
        self.current_section = text.to_owned();
        self.sections.push(text.to_owned());
    }

    /// `addTask`: the data is `:score:people`. The score is `Number()` of the first piece, and the
    /// people are the second piece split at commas and trimmed; the pieces after the second are
    /// ignored (J8, J9, J10).
    pub(crate) fn add_task(&mut self, descr: &str, task_data: &str) {
        // `taskData.substr(1)`: the data starts with its colon, which is one byte.
        let mut pieces = task_data.get(1..).unwrap_or("").split(':');
        let score = js_number(pieces.next().unwrap_or(""));
        let people = match pieces.next() {
            Some(list) => list
                .split(',')
                .map(|person| js_trim(person).to_owned())
                .collect(),
            None => Vec::new(),
        };
        self.tasks.push(RawTask {
            section: self.current_section.clone(),
            task: descr.to_owned(),
            people,
            score,
        });
    }

    /// The model: every text as the reader sees it, and the actors sorted as `getActors` sorts
    /// them, on the DB's texts (J11).
    pub(crate) fn into_model(self) -> JourneyDiagram {
        let mut model = JourneyDiagram::new();
        model.title = display_text(&self.title);
        model.acc_title = display_text(&self.acc_title);
        model.acc_descr = display_text(&self.acc_descr);
        model.sections = self
            .sections
            .iter()
            .map(|section| display_text(section))
            .collect();

        let mut people: Vec<&str> = self
            .tasks
            .iter()
            .flat_map(|task| task.people.iter().map(String::as_str))
            .collect();
        people.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
        people.dedup();
        model.actors = people.into_iter().map(display_text).collect();

        model.tasks = self
            .tasks
            .into_iter()
            .map(|task| JourneyTask {
                section: display_text(&task.section),
                task: display_text(&task.task),
                score: task.score,
                people: task
                    .people
                    .iter()
                    .map(|person| display_text(person))
                    .collect(),
            })
            .collect();
        model
    }
}

/// The text a reader sees for a DB text. The renderer sets it as text, which the serialiser
/// escapes (`&`, `<`, `>`); `decodeEntities` then turns the placeholders back into references
/// (`ﬂ°°` to `&#`, `ﬂ°` to `&`, `¶ß` to `;`) in the whole SVG string; and the HTML parser reads the
/// result. Raw text therefore reads as it was written, and only the references change.
pub(crate) fn display_text(encoded: &str) -> String {
    let mut escaped = String::with_capacity(encoded.len());
    for c in encoded.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            other => escaped.push(other),
        }
    }
    let decoded = escaped
        .replace("ﬂ°°", "&#")
        .replace("ﬂ°", "&")
        .replace("¶ß", ";");
    decode_references(&decoded)
}

#[cfg(test)]
mod tests {
    use super::display_text;

    #[test]
    fn raw_text_reads_as_written() {
        assert_eq!(display_text("Task &amp; x"), "Task &amp; x");
        assert_eq!(display_text("A & B"), "A & B");
        assert_eq!(display_text("a < b > c"), "a < b > c");
    }

    #[test]
    fn placeholders_read_as_their_characters() {
        assert_eq!(display_text("ﬂ°°9829¶ß"), "\u{2665}");
        assert_eq!(
            display_text("Coffee ﬂ°°9829¶ß time"),
            "Coffee \u{2665} time"
        );
        assert_eq!(display_text("Task ﬂ°amp¶ß x"), "Task & x");
        assert_eq!(display_text("ﬂ°hearts¶ß"), "\u{2665}");
        assert_eq!(display_text("ﬂ°lt¶ß"), "<");
    }

    #[test]
    fn an_unknown_name_stays_as_written() {
        assert_eq!(display_text("Task ﬂ°foo¶ß x"), "Task &foo; x");
    }

    #[test]
    fn the_marks_of_the_text_are_decoded_too() {
        // The reader decodes the whole SVG string, so a mark typed by the user counts as well.
        assert_eq!(display_text("a¶ßb"), "a;b");
        assert_eq!(display_text("ﬂ°°65¶ß"), "A");
    }

    #[test]
    fn a_zero_number_reads_as_the_replacement_character() {
        assert_eq!(display_text("ﬂ°°0¶ß"), "\u{fffd}");
    }
}
