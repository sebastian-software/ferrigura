//! Detection of the diagram type from the start of the text, as the detectors of the reference
//! decide it: `pieDetector.ts`, `infoDetector.ts`, `flowDetector-v2.ts`, `stateDetector-V2.ts` and
//! the P2 and P3 detectors named in each test (`packages/mermaid/src/diagrams/<type>/`).

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
    let error = parse("unknown diagram\n  x").expect_err("no diagram type matches");
    assert_eq!((error.line, error.column), (1, 1));
    // A P3 header is detected, and its parser is not ported yet.
    let error = parse("radar-beta\n  axis a").expect_err("the radar parser is not ported yet");
    assert_eq!((error.line, error.column), (1, 1));
    assert!(
        error.message.contains("not ported yet"),
        "{}",
        error.message
    );
}

#[test]
fn front_matter_after_blank_lines_or_comments_is_skipped_as_the_reference_does() {
    // `cleanupComments` trims the start before `detectType` strips the front matter.
    assert_eq!(
        detect("\n---\ntitle: x\n---\npie\n\"a\": 1"),
        Some(DiagramType::Pie)
    );
    assert_eq!(
        detect("%% a comment\n---\ntitle: x\n---\nflowchart LR\n  a-->b"),
        Some(DiagramType::Flowchart)
    );
    assert_eq!(
        detect("\n---\n  theme: forest\n---\n  requirementDiagram"),
        Some(DiagramType::Requirement)
    );
}

/// Every header of the P3 types, one per detector: `agentflow/afDetector` `/^\s*agentflow-beta\b/`,
/// `swimlanes/detector` `/^\s*swimlane-beta\b/`, `treeView/detector` `/^\s*treeView-beta/`,
/// `radar/detector` `/^\s*radar-beta/`, the four railroad detectors `/^\s*railroad-beta/i` and its
/// EBNF, ABNF and PEG forms, `venn/vennDetector` `/^\s*venn-beta/`, `wardley/wardleyDetector`
/// `/^\s*wardley-beta/i`, `cynefin/cynefinDetector` `/^\s*cynefin-beta(?:[\s:]|$)/` and
/// `usecase/usecaseDetector` `/^\s*usecase-beta(?:\s|$)/`.
#[test]
fn detects_every_header_of_the_p3_types() {
    let cases = [
        ("agentflow-beta\n  node a", DiagramType::Agentflow),
        ("cynefin-beta\n  title T", DiagramType::Cynefin),
        ("cynefin-beta:\n  title T", DiagramType::Cynefin),
        ("radar-beta\n  axis a", DiagramType::Radar),
        ("railroad-beta\n  rule", DiagramType::Railroad),
        ("railroad-ebnf-beta\n  rule", DiagramType::Railroad),
        ("railroad-abnf-beta\n  rule", DiagramType::Railroad),
        ("railroad-peg-beta\n  rule", DiagramType::Railroad),
        ("RAILROAD-BETA\n  rule", DiagramType::Railroad),
        ("Railroad-Peg-Beta\n  rule", DiagramType::Railroad),
        ("swimlane-beta\n  lane A", DiagramType::Swimlanes),
        ("treeView-beta\n  root", DiagramType::TreeView),
        ("usecase-beta\n  actor A", DiagramType::UseCase),
        ("venn-beta\n  set A", DiagramType::Venn),
        ("wardley-beta\n  title T", DiagramType::Wardley),
        ("WARDLEY-BETA\n  title T", DiagramType::Wardley),
    ];
    for (text, expected) in cases {
        assert_eq!(detect(text), Some(expected), "{text:?}");
    }
}

#[test]
fn the_word_boundary_headers_reject_a_word_character_after_the_keyword() {
    // `agentflow-beta\b` and `swimlane-beta\b`: an ASCII letter, a digit or `_` after the keyword
    // continues the word, while a hyphen or the end of the text does not.
    for text in [
        "agentflow-betax\n",
        "agentflow-beta_x\n",
        "agentflow-beta1\n",
        "swimlane-betax\n",
        "swimlane-beta_\n",
    ] {
        assert_eq!(detect(text), None, "{text:?}");
    }
    assert_eq!(detect("agentflow-beta-x\n"), Some(DiagramType::Agentflow));
    assert_eq!(detect("agentflow-beta"), Some(DiagramType::Agentflow));
    assert_eq!(detect("swimlane-beta-x\n"), Some(DiagramType::Swimlanes));
    assert_eq!(detect("swimlane-beta"), Some(DiagramType::Swimlanes));
}

#[test]
fn the_cynefin_and_use_case_headers_need_a_space_or_the_end_after_the_keyword() {
    // `cynefin-beta(?:[\s:]|$)` takes a colon, `usecase-beta(?:\s|$)` does not; neither takes a hyphen.
    assert_eq!(detect("cynefin-beta:x\n"), Some(DiagramType::Cynefin));
    assert_eq!(detect("cynefin-beta\tx\n"), Some(DiagramType::Cynefin));
    assert_eq!(detect("cynefin-beta"), Some(DiagramType::Cynefin));
    assert_eq!(detect("cynefin-beta-x\n"), None);
    assert_eq!(detect("cynefin-betax\n"), None);
    // `detectType` strips the comment first: a comment that ends a line is a boundary, one that does not is not.
    assert_eq!(
        detect("cynefin-beta%%note\ncomplex \"A\"\n"),
        Some(DiagramType::Cynefin)
    );
    assert_eq!(detect("cynefin-beta%%note"), None);
    assert_eq!(detect("usecase-beta\n"), Some(DiagramType::UseCase));
    assert_eq!(detect("usecase-beta\u{a0}x\n"), Some(DiagramType::UseCase));
    assert_eq!(detect("usecase-beta"), Some(DiagramType::UseCase));
    assert_eq!(detect("usecase-beta:\n"), None);
    assert_eq!(detect("usecase-beta-x\n"), None);
}

#[test]
fn the_radar_treeview_venn_and_railroad_headers_take_any_suffix() {
    // Their reference detectors are plain prefixes, with no `\b`.
    assert_eq!(detect("radar-betaX\n"), Some(DiagramType::Radar));
    assert_eq!(detect("treeView-betaX\n"), Some(DiagramType::TreeView));
    assert_eq!(detect("venn-betaX\n"), Some(DiagramType::Venn));
    assert_eq!(detect("railroad-betaX\n"), Some(DiagramType::Railroad));
    assert_eq!(detect("railroad-ebnf-betax\n"), Some(DiagramType::Railroad));
}

#[test]
fn the_p3_headers_are_case_sensitive_except_railroad_and_wardley() {
    for text in [
        "treeview-beta\n",
        "TreeView-beta\n",
        "RADAR-beta\n",
        "Venn-beta\n",
        "Agentflow-beta\n",
        "Swimlane-Beta\n",
        "CYNEFIN-BETA\n",
        "UseCase-beta\n",
    ] {
        assert_eq!(detect(text), None, "{text:?}");
    }
}

#[test]
fn the_p3_headers_are_taken_only_at_the_start_of_the_text() {
    // The agentflow and swimlane headers come before the flowchart detector in the reference, and a
    // header in the body of another diagram never takes the diagram.
    assert_eq!(
        detect("swimlane-beta\n  graph TD\n  a-->b"),
        Some(DiagramType::Swimlanes)
    );
    assert_eq!(
        detect("agentflow-beta\n  graph LR"),
        Some(DiagramType::Agentflow)
    );
    assert_eq!(
        detect("flowchart LR\n  swimlane-beta\n"),
        Some(DiagramType::Flowchart)
    );
    assert_eq!(
        detect("graph TD\n  agentflow-beta"),
        Some(DiagramType::Flowchart)
    );
    assert_eq!(
        detect("sequenceDiagram\n  swimlane-beta"),
        Some(DiagramType::Sequence)
    );
    assert_eq!(
        detect("timeline\n  railroad-beta"),
        Some(DiagramType::Timeline)
    );
}

#[test]
fn the_four_railroad_headers_are_each_taken_and_none_is_a_prefix_of_another() {
    // The family has four keywords with one type. `railroad-beta` is a prefix of none of the others,
    // and the family name alone is not a header.
    for text in [
        "railroad-beta\n",
        "railroad-ebnf-beta\n",
        "railroad-abnf-beta\n",
        "railroad-peg-beta\n",
    ] {
        assert_eq!(detect(text), Some(DiagramType::Railroad), "{text:?}");
    }
    assert_eq!(detect("railroad\n  rule"), None);
    assert_eq!(detect("railroad-ebnf\n  rule"), None);
    assert_eq!(detect("railroad-x-beta\n"), None);
}
