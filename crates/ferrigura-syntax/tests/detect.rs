//! Detection of the diagram type from the start of the text, as the detectors of the reference
//! decide it: `pieDetector.ts`, `infoDetector.ts`, `flowDetector-v2.ts`, `stateDetector-V2.ts` and
//! the P2 detectors named in each test (`packages/mermaid/src/diagrams/<type>/`).

use ferrigura_syntax::{Diagram, DiagramType, detect, parse};

#[test]
fn detects_pie_info_and_flowchart_at_the_first_content_line() {
    assert_eq!(detect("pie\n\"a\": 1"), Some(DiagramType::Pie));
    assert_eq!(detect("  \n\tinfo"), Some(DiagramType::Info));
    assert_eq!(
        detect("flowchart LR\n  a-->b"),
        Some(DiagramType::Flowchart)
    );
    assert_eq!(detect("graph TD\n  a-->b"), Some(DiagramType::Flowchart));
    assert_eq!(
        detect("sequenceDiagram\n  A->>B: hi"),
        Some(DiagramType::Sequence)
    );
}

/// Every header of the P2 types that the registry knows, one per detector. `mindmapDetector`
/// `/^\s*mindmap/`, `architectureDetector` `/^\s*architecture/`, `c4Detector` (five headers),
/// `kanban/detector` `/^\s*kanban/`, `requirementDetector` `/^\s*requirementDiagram/`,
/// `quadrantDetector` `/^\s*quadrantChart/`, `eventmodeling/detector` `/^\s*eventmodeling/`,
/// `treemap/detector` `/^\s*treemap/`, `ishikawaDetector` `/^\s*ishikawa(-beta)?\b/i`.
#[test]
fn detects_every_header_of_the_p2_types() {
    let cases = [
        ("mindmap\n  root", DiagramType::Mindmap),
        (
            "architecture\n  service db(database)[DB]",
            DiagramType::Architecture,
        ),
        (
            "architecture-beta\n  service db(database)[DB]",
            DiagramType::Architecture,
        ),
        ("C4Context\n  title T", DiagramType::C4),
        ("C4Container\n  title T", DiagramType::C4),
        ("C4Component\n  title T", DiagramType::C4),
        ("C4Dynamic\n  title T", DiagramType::C4),
        ("C4Deployment\n  title T", DiagramType::C4),
        ("kanban\n  Todo", DiagramType::Kanban),
        (
            "requirementDiagram\n  requirement r {}",
            DiagramType::Requirement,
        ),
        ("quadrantChart\n  title T", DiagramType::QuadrantChart),
        ("sankey\n  a,b,8", DiagramType::Sankey),
        ("packet\n  0-15: \"Header\"", DiagramType::Packet),
        ("xychart\n  x-axis [a, b]", DiagramType::XyChart),
        ("block\n  A", DiagramType::Block),
        ("eventmodeling\n  tf 01 cmd", DiagramType::EventModeling),
        ("treemap\n  \"Root\"", DiagramType::Treemap),
        ("treemap-beta\n  \"Root\"", DiagramType::Treemap),
        ("ishikawa\n  Cause", DiagramType::Ishikawa),
        ("ishikawa-beta\n  Cause", DiagramType::Ishikawa),
        ("ISHIKAWA\n  Cause", DiagramType::Ishikawa),
        ("Ishikawa-Beta\n  Cause", DiagramType::Ishikawa),
    ];
    for (text, expected) in cases {
        assert_eq!(detect(text), Some(expected), "{text:?}");
    }
}

#[test]
fn the_stable_p2_types_reject_their_beta_keyword() {
    // `sankey`, `packet`, `xychart` and `block` graduated from beta: `(?![\w-])` in their detectors.
    for text in [
        "sankey-beta\n  a,b,8",
        "packet-beta\n  0-15: \"Header\"",
        "xychart-beta\n  x-axis [a, b]",
        "block-beta\n  A",
    ] {
        assert_eq!(detect(text), None, "{text:?}");
    }
    assert_eq!(detect("block.A\n"), Some(DiagramType::Block));
    assert_eq!(detect("blockX\n"), None);
}

#[test]
fn the_p2_headers_are_case_sensitive_except_ishikawa() {
    assert_eq!(detect("c4Context\n  title T"), None);
    assert_eq!(detect("Kanban\n  Todo"), None);
    assert_eq!(detect("MINDMAP\n  root"), None);
    assert_eq!(detect("ishikawa_x\n  Cause"), None);
    assert_eq!(detect("ishikawax\n  Cause"), None);
}

#[test]
fn the_p2_detectors_without_a_word_boundary_take_any_suffix() {
    // The reference's detectors for these types are plain prefixes, with no `\b`.
    assert_eq!(detect("kanbanboard\n  Todo"), Some(DiagramType::Kanban));
    assert_eq!(detect("mindmapper\n  root"), Some(DiagramType::Mindmap));
    assert_eq!(detect("treemapX\n  \"Root\""), Some(DiagramType::Treemap));
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
fn detects_state_diagrams_in_both_headers_case_sensitively() {
    assert_eq!(detect("stateDiagram\n  A --> B"), Some(DiagramType::State));
    assert_eq!(
        detect("stateDiagram-v2\n  A --> B"),
        Some(DiagramType::State)
    );
    assert_eq!(detect("STATEDIAGRAM-V2\n  A --> B"), None);
}

#[test]
fn an_unclosed_directive_is_not_skipped() {
    assert_eq!(detect("%%{init: {}\npie"), None);
}

#[test]
fn parse_dispatches_on_the_detected_type() {
    assert!(matches!(parse("pie\n\"a\": 1"), Ok(Diagram::Pie(_))));
    assert!(matches!(parse("info title T"), Ok(Diagram::Info(_))));
    assert!(matches!(
        parse("flowchart LR\n  a-->b"),
        Ok(Diagram::Flowchart(_))
    ));
    assert!(matches!(
        parse("sequenceDiagram\n  A->>B: hi"),
        Ok(Diagram::Sequence(_))
    ));
    assert!(matches!(
        parse("stateDiagram-v2\nA --> B"),
        Ok(Diagram::State(_))
    ));
    let error = parse("radar-beta\n  axis a").expect_err("no diagram type matches");
    assert_eq!((error.line, error.column), (1, 1));
}
