//! Parser of the info diagram, following `packages/parser/src/language/info/info.langium`.
//!
//! ```text
//! Info: NEWLINE* "info" NEWLINE* ("showInfo" NEWLINE*)? TitleAndAccessibilities?
//! ```
//!
//! The reference does not copy the header into its DB, so the model keeps the header as the
//! grammar gives it: without the front matter title.

use ferrigura_model::info::InfoDiagram;

use crate::common::Header;
use crate::cursor::Cursor;
use crate::error::{ParseError, Source};
use crate::lexer::lex;
use crate::preprocess::prepare;

/// Parses an info diagram, with its front matter, and returns its model.
pub fn parse(raw: &str) -> Result<InfoDiagram, ParseError> {
    let prepared = prepare(raw);
    let source = Source::new(&prepared.text, prepared.line_offset);
    let mut cursor = Cursor::new(source, lex(&source)?);

    cursor.skip_newlines();
    cursor.expect_word("info")?;
    cursor.skip_newlines();
    if cursor.eat_word("showInfo") {
        cursor.skip_newlines();
    }

    let mut header = Header::default();
    while cursor.peek().is_some() {
        if !header.take_line(&mut cursor)? {
            return Err(
                cursor.error("expected a title, an accessibility line or the end of the diagram")
            );
        }
    }

    let mut diagram = InfoDiagram::new();
    diagram.title = header.title;
    diagram.acc_title = header.acc_title;
    diagram.acc_descr = header.acc_descr;
    Ok(diagram)
}
