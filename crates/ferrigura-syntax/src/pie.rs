//! Parser of the pie diagram, following `packages/parser/src/language/pie/pie.langium`, and the
//! diagram DB of `packages/mermaid/src/diagrams/pie/pieDb.ts`, which decides the model.
//!
//! ```text
//! Pie:        NEWLINE* "pie" showData? (TitleAndAccessibilities | PieSection | NEWLINE)*
//! PieSection: STRING ":" NUMBER EOL
//! ```

use std::collections::HashSet;

use ferrigura_model::pie::{PieDiagram, PieSection};

use crate::common::Header;
use crate::cursor::Cursor;
use crate::error::{ParseError, Source};
use crate::lexer::{Token, lex};
use crate::preprocess::prepare;
use crate::text::{newline_then_space_to_newline, strip_leading_space};

/// A slice as written, with the offset of its value for error positions.
struct Slice {
    label: String,
    value: f64,
    offset: usize,
}

/// Parses a pie diagram, with its front matter, and returns its model.
///
/// The error position is the first problem in the source, counted from its first line.
pub fn parse(raw: &str) -> Result<PieDiagram, ParseError> {
    let prepared = prepare(raw);
    let source = Source::new(&prepared.text, prepared.line_offset);
    let mut cursor = Cursor::new(source, lex(&source)?);

    cursor.skip_newlines();
    cursor.expect_word("pie")?;
    let show_data = cursor.eat_word("showData");

    let mut header = Header::default();
    let mut slices = Vec::new();
    while let Some(token) = cursor.peek().cloned() {
        match token {
            Token::Newline => cursor.skip_newlines(),
            Token::Str(label) => {
                cursor.advance();
                cursor.expect(&Token::Colon, "':' after the slice label")?;
                let (value, offset) = cursor.expect_number()?;
                cursor.expect_eol()?;
                slices.push(Slice {
                    label,
                    value,
                    offset,
                });
            }
            _ => {
                if !header.take_line(&mut cursor)? {
                    return Err(cursor.error("expected a slice, a title or the end of the diagram"));
                }
            }
        }
    }

    let mut diagram = PieDiagram::new();
    // The grammar's title wins when it has one; otherwise the front matter's title is kept.
    diagram.title = if header.title.is_empty() {
        prepared.title.unwrap_or_default()
    } else {
        header.title
    };
    // The DB keeps the accessibility texts without leading blanks, and joins indented lines.
    diagram.acc_title = strip_leading_space(&header.acc_title).to_owned();
    diagram.acc_descr = newline_then_space_to_newline(&header.acc_descr);
    diagram.show_data = show_data;

    // `addSection`: a negative value is an error even for a label that is already there, and the
    // first value given for a label is the one kept.
    let mut seen = HashSet::new();
    for slice in slices {
        if slice.value < 0.0 {
            return Err(source.error(
                slice.offset,
                format!(
                    "slice \"{}\" has a negative value {}; slice values must be zero or more",
                    slice.label, slice.value
                ),
            ));
        }
        if seen.insert(slice.label.clone()) {
            diagram.sections.push(PieSection {
                label: slice.label,
                value: slice.value,
            });
        }
    }
    Ok(diagram)
}
