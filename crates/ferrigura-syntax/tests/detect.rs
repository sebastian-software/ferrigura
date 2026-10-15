//! Detection of the diagram type from the start of the text, as the detectors of the reference
//! `pieDetector.ts` and `infoDetector.ts` decide it.

use ferrigura_syntax::{Diagram, DiagramType, detect, parse};

#[test]
fn detects_pie_and_info_at_the_first_content_line() {
    assert_eq!(detect("pie\n\"a\": 1"), Some(DiagramType::Pie));
    assert_eq!(detect("  \n\tinfo"), Some(DiagramType::Info));
    assert_eq!(detect("flowchart LR\n  a-->b"), None);
}

#[test]
fn skips_front_matter_directives_and_comment_lines_before_the_keyword() {
    assert_eq!(
        detect("---\ntitle: T\n---\npie\n\"a\": 1"),
        Some(DiagramType::Pie)
    );
    assert_eq!(
        detect("%%{init: {\"theme\": \"dark\"}}%%\n%% note\npie"),
        Some(DiagramType::Pie)
    );
}

#[test]
fn an_unclosed_directive_is_not_skipped() {
    assert_eq!(detect("%%{init: {}\npie"), None);
}

#[test]
fn parse_dispatches_on_the_detected_type() {
    assert!(matches!(parse("pie\n\"a\": 1"), Ok(Diagram::Pie(_))));
    assert!(matches!(parse("info title T"), Ok(Diagram::Info(_))));
    let error = parse("flowchart LR").expect_err("no diagram type matches");
    assert_eq!((error.line, error.column), (1, 1));
}
