//! The headers of the Mermaid diagram types that ferrigura does not draw
//! (`docs/decisions/0024-a-fixed-list-of-diagram-types.md`).
//!
//! The detector of each such type is kept as the reference words it, so that a text of the type is
//! told apart from a text without a header: [`crate::detect_unsupported`] names the type, and the
//! facade refuses the text with an error that names it and lists the types ferrigura draws
//! ([`crate::DiagramType::ALL`]). No header here is also the header of a type that ferrigura draws,
//! so the order of the tests decides nothing.

use crate::has_keyword_at_word_boundary;

/// The name the error gives a type that ferrigura does not draw, with the test of its header on the
/// text from the header on.
type Header = (&'static str, fn(&str) -> bool);

/// The header of each type that ferrigura does not draw.
const HEADERS: &[Header] = &[
    // `swimlanes/detector.ts`: `/^\s*swimlane-beta\b/`.
    ("swimlane", |rest| {
        has_keyword_at_word_boundary(rest, "swimlane-beta")
    }),
    // `venn/vennDetector.ts`: `/^\s*venn-beta/`.
    ("venn", |rest| rest.starts_with("venn-beta")),
];

/// The name of the type that ferrigura does not draw whose header starts `rest`, the text from the
/// header on.
pub(crate) fn name(rest: &str) -> Option<&'static str> {
    HEADERS
        .iter()
        .find(|(_, header)| header(rest))
        .map(|(name, _)| *name)
}
