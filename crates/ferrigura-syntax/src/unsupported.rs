//! The headers of the Mermaid diagram types that ferrigura does not draw
//! (`docs/decisions/0024-a-fixed-list-of-diagram-types.md`).
//!
//! The detector of each such type is kept as the reference words it, so that a text of the type is
//! told apart from a text without a header: [`crate::detect_unsupported`] names the type, and the
//! facade refuses the text with an error that names it and lists the types ferrigura draws
//! ([`crate::DiagramType::ALL`]). No header here is also the header of a type that ferrigura draws,
//! so the order of the tests decides nothing.

use crate::{has_keyword_alone, text};

/// The name the error gives a type that ferrigura does not draw, with the test of its header on the
/// text from the header on.
type Header = (&'static str, fn(&str) -> bool);

/// The header of each type that ferrigura does not draw.
const HEADERS: &[Header] = &[
    // `requirement/requirementDetector.ts`: `/^\s*requirementDiagram/`.
    ("requirement", |rest| rest.starts_with("requirementDiagram")),
    // `block/blockDetector.ts`: `/^\s*block(-beta)?(?![\w-])/` (patch 08 of tools/reference).
    ("block", |rest| has_keyword_alone(rest, "block")),
    // `usecase/usecaseDetector.ts`: `/^\s*usecase-beta(?:\s|$)/`.
    ("use case", |rest| {
        keyword_followed_by(rest, "usecase-beta", text::is_js_space)
    }),
    // `agentflow/afDetector.ts`: `/^\s*agentflow-beta\b/`.
    ("agentflow", |rest| {
        has_keyword_at_word_boundary(rest, "agentflow-beta")
    }),
    // `swimlanes/detector.ts`: `/^\s*swimlane-beta\b/`.
    ("swimlane", |rest| {
        has_keyword_at_word_boundary(rest, "swimlane-beta")
    }),
    // `venn/vennDetector.ts`: `/^\s*venn-beta/`.
    ("venn", |rest| rest.starts_with("venn-beta")),
    // `treeView/detector.ts`: `/^\s*treeView-beta/`.
    ("tree view", |rest| rest.starts_with("treeView-beta")),
    // `ishikawa/ishikawaDetector.ts`: `/^\s*ishikawa(-beta)?\b/i`.
    ("ishikawa", is_ishikawa_header),
    // `git/gitGraphDetector.ts`: `/^\s*gitGraph/`.
    ("Git graph", |rest| rest.starts_with("gitGraph")),
    // `c4/c4Detector.ts`: `/^\s*C4Context|C4Container|C4Component|C4Dynamic|C4Deployment/`.
    ("C4", is_c4_header),
    // `architecture/architectureDetector.ts`: `/^\s*architecture/`.
    ("architecture", |rest| rest.starts_with("architecture")),
    // `cynefin/cynefinDetector.ts`: `/^\s*cynefin-beta(?:[\s:]|$)/`.
    ("cynefin", cynefin_header),
    // `eventmodeling/detector.ts`: `/^\s*eventmodeling/`.
    ("event modeling", |rest| rest.starts_with("eventmodeling")),
    // The four railroad detectors: `/^\s*railroad-beta/i` and its EBNF, ABNF and PEG forms.
    ("railroad", is_railroad_header),
    // `wardley/wardleyDetector.ts`: `/^\s*wardley-beta/i`.
    ("wardley", |rest| {
        starts_with_ignore_ascii_case(rest, "wardley-beta")
    }),
    // `timeline/detector.ts`: `/^\s*timeline/`.
    ("timeline", |rest| rest.starts_with("timeline")),
    // `user-journey/journeyDetector.ts`: `/^\s*journey/`.
    ("user journey", |rest| rest.starts_with("journey")),
    // `kanban/detector.ts`: `/^\s*kanban/`.
    ("kanban", |rest| rest.starts_with("kanban")),
    // `sankey/sankeyDetector.ts`: `/^\s*sankey(-beta)?(?![\w-])/` (patch 08 of tools/reference).
    ("sankey", |rest| has_keyword_alone(rest, "sankey")),
    // `radar/detector.ts`: `/^\s*radar-beta/`.
    ("radar", |rest| rest.starts_with("radar-beta")),
    // `packet/detector.ts`: `/^\s*packet(-beta)?(?![\w-])/` (patch 08 of tools/reference).
    ("packet", |rest| has_keyword_alone(rest, "packet")),
    // `treemap/detector.ts`: `/^\s*treemap/`.
    ("treemap", |rest| rest.starts_with("treemap")),
    // `info/infoDetector.ts`: `/^\s*info/`.
    ("info", |rest| rest.starts_with("info")),
];

/// The name of the type that ferrigura does not draw whose header starts `rest`, the text from the
/// header on.
pub(crate) fn name(rest: &str) -> Option<&'static str> {
    HEADERS
        .iter()
        .find(|(_, header)| header(rest))
        .map(|(name, _)| *name)
}

/// `^\s*(?:C4Context|C4Container|C4Component|C4Dynamic|C4Deployment)`, the detector of `c4`.
fn is_c4_header(text: &str) -> bool {
    [
        "C4Context",
        "C4Container",
        "C4Component",
        "C4Dynamic",
        "C4Deployment",
    ]
    .iter()
    .any(|header| text.starts_with(header))
}

/// `^\s*ishikawa(-beta)?\b` with the `i` flag, the detector of `ishikawa`. The optional `-beta` does
/// not change the outcome: the match ends at a word boundary after `ishikawa`, and a hyphen or the end
/// of the text always gives one. The `i` flag folds ASCII letters only, as it does without `u`.
fn is_ishikawa_header(text: &str) -> bool {
    text.get(..8)
        .is_some_and(|keyword| keyword.eq_ignore_ascii_case("ishikawa"))
        && !matches!(text.as_bytes().get(8), Some(&byte) if byte.is_ascii_alphanumeric() || byte == b'_')
}

/// `^\s*<keyword>\b`, the header test of the agentflow and swimlane detectors: the keyword ends at a
/// word boundary, so an ASCII letter, a digit or `_` after it is not taken, while a hyphen, a space
/// or a non-ASCII character is.
fn has_keyword_at_word_boundary(text: &str, keyword: &str) -> bool {
    text.strip_prefix(keyword).is_some_and(|after| {
        !matches!(after.as_bytes().first(), Some(&byte) if byte.is_ascii_alphanumeric() || byte == b'_')
    })
}

/// `^\s*<keyword>` with the `i` flag, for the detectors that fold case (`railroad`, `wardley`). The
/// fold covers ASCII letters only, as it does without the `u` flag.
fn starts_with_ignore_ascii_case(text: &str, keyword: &str) -> bool {
    text.get(..keyword.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(keyword))
}

/// The four railroad headers: `railroad-beta`, `railroad-ebnf-beta`, `railroad-abnf-beta` and
/// `railroad-peg-beta`, with the `i` flag. None of them is a prefix of another.
fn is_railroad_header(text: &str) -> bool {
    [
        "railroad-beta",
        "railroad-ebnf-beta",
        "railroad-abnf-beta",
        "railroad-peg-beta",
    ]
    .iter()
    .any(|keyword| starts_with_ignore_ascii_case(text, keyword))
}

/// `^\s*cynefin-beta(?:[\s:]|$)` on the text after `stripAnyComments`, as `detectType` matches it
/// (`diagram-api/detectType.ts`, `diagram-api/regexes.ts`). That function replaces a `%%` comment
/// that ends in a line break with a line break, so `cynefin-beta%%note` followed by a line is the
/// header too, and the same comment without a line break after it is not.
fn cynefin_header(text: &str) -> bool {
    if keyword_followed_by(text, "cynefin-beta", |c| text::is_js_space(c) || c == ':') {
        return true;
    }
    text.strip_prefix("cynefin-beta")
        .and_then(|after| after.strip_prefix("%%"))
        .is_some_and(|comment| comment.contains('\n'))
}

/// Whether `text` starts with `keyword`, and the character after it is absent or accepted by
/// `follows`: `^\s*<keyword>(?:[\s:]|$)` for cynefin and `^\s*<keyword>(?:\s|$)` for use case.
fn keyword_followed_by(text: &str, keyword: &str, follows: impl Fn(char) -> bool) -> bool {
    text.strip_prefix(keyword)
        .is_some_and(|after| match after.chars().next() {
            None => true,
            Some(next) => follows(next),
        })
}
