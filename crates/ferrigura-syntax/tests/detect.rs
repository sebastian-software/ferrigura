//! Detection of the diagram type from the start of the text, as the detectors of the reference
//! decide it (`packages/mermaid/src/diagrams/<type>/`): the types that ferrigura draws with
//! `detect`, and the headers of the other types of Mermaid with `detect_unsupported`, which names
//! them for the facade's error.

use ferrigura_syntax::{Diagram, DiagramType, detect, detect_unsupported, parse};

#[test]
fn detects_every_header_of_the_types_that_ferrigura_draws() {
    let cases = [
        ("flowchart LR\n  a-->b", DiagramType::Flowchart),
        ("graph TD\n  a-->b", DiagramType::Flowchart),
        ("stateDiagram\n  A --> B", DiagramType::State),
        ("stateDiagram-v2\n  A --> B", DiagramType::State),
        ("classDiagram\n  class A", DiagramType::Class),
        ("classDiagram-v2\n  class A", DiagramType::Class),
        ("erDiagram\n  A ||--o{ B : has", DiagramType::Er),
        ("mindmap\n  root", DiagramType::Mindmap),
        ("sequenceDiagram\n  A->>B: hi", DiagramType::Sequence),
        ("gantt\n  title T", DiagramType::Gantt),
        ("xychart\n  x-axis [a, b]", DiagramType::XyChart),
        ("xychart-beta\n  x-axis [a, b]", DiagramType::XyChart),
        ("pie\n\"a\": 1", DiagramType::Pie),
        ("quadrantChart\n  title T", DiagramType::QuadrantChart),
    ];
    for (text, expected) in cases {
        assert_eq!(detect(text), Some(expected), "{text:?}");
        assert_eq!(detect_unsupported(text), None, "{text:?}");
    }
    for kind in DiagramType::ALL {
        assert!(
            cases.iter().any(|(_, expected)| *expected == kind),
            "{kind:?} has a case"
        );
    }
}

/// The headers of the types that ferrigura does not draw, one or more per detector of the reference,
/// with the name the error gives each type.
#[test]
fn names_every_header_of_the_types_that_ferrigura_does_not_draw() {
    let cases = [
        ("info", "info"),
        ("  \n\tinfo", "info"),
        ("timeline\n  title T", "timeline"),
        ("journey\n  title T", "user journey"),
        ("gitGraph\n  commit", "Git graph"),
        ("requirementDiagram\n  requirement r {}", "requirement"),
        ("block\n  A", "block"),
        ("block-beta\n  A", "block"),
        ("block.A\n", "block"),
        ("packet\n  0-15: \"Header\"", "packet"),
        ("packet-beta\n  0-15: \"Header\"", "packet"),
        ("sankey\n  a,b,8", "sankey"),
        ("sankey-beta\n  a,b,8", "sankey"),
        ("C4Context\n  title T", "C4"),
        ("C4Container\n  title T", "C4"),
        ("C4Component\n  title T", "C4"),
        ("C4Dynamic\n  title T", "C4"),
        ("C4Deployment\n  title T", "C4"),
        ("kanban\n  Todo", "kanban"),
        ("architecture\n  service db(database)[DB]", "architecture"),
        (
            "architecture-beta\n  service db(database)[DB]",
            "architecture",
        ),
        ("treemap\n  \"Root\"", "treemap"),
        ("treemap-beta\n  \"Root\"", "treemap"),
        ("ishikawa\n  Cause", "ishikawa"),
        ("ishikawa-beta\n  Cause", "ishikawa"),
        ("ISHIKAWA\n  Cause", "ishikawa"),
        ("Ishikawa-Beta\n  Cause", "ishikawa"),
        ("eventmodeling\n  tf 01 cmd", "event modeling"),
        ("agentflow-beta\n  node a", "agentflow"),
        ("cynefin-beta\n  title T", "cynefin"),
        ("cynefin-beta:\n  title T", "cynefin"),
        ("radar-beta\n  axis a", "radar"),
        ("railroad-beta\n  rule", "railroad"),
        ("railroad-ebnf-beta\n  rule", "railroad"),
        ("railroad-abnf-beta\n  rule", "railroad"),
        ("railroad-peg-beta\n  rule", "railroad"),
        ("RAILROAD-BETA\n  rule", "railroad"),
        ("Railroad-Peg-Beta\n  rule", "railroad"),
        ("swimlane-beta\n  lane A", "swimlane"),
        ("treeView-beta\n  root", "tree view"),
        ("usecase-beta\n  actor A", "use case"),
        ("venn-beta\n  set A", "venn"),
        ("wardley-beta\n  title T", "wardley"),
        ("WARDLEY-BETA\n  title T", "wardley"),
    ];
    for (text, name) in cases {
        assert_eq!(detect(text), None, "{text:?}");
        assert_eq!(detect_unsupported(text), Some(name), "{text:?}");
    }
}

#[test]
fn the_keyword_detectors_end_at_the_keyword_or_its_beta_form() {
    // Upstream's detectors are `^\s*<keyword>(-beta)?`; ferrigura ends them at `(?![\w-])`, so a
    // longer word is not taken (`sankeyfoo`, `packet-0-7`), a fix the reference gets as a patch.
    for text in [
        "sankeyfoo\n",
        "packet0-7\n",
        "packet-0-7\n",
        "sankey-betax\n",
        "blockX\n",
        "xychartX\n",
        "xychart-beta-x\n",
    ] {
        assert_eq!(detect(text), None, "{text:?}");
        assert_eq!(detect_unsupported(text), None, "{text:?}");
    }
}

#[test]
fn detects_flowchart_elk_as_a_flowchart() {
    // `flowchart-elk` is upstream's own diagram id with the flowchart's parser and ELK as its
    // layout, which is ferrigura's layout of every flowchart.
    assert_eq!(
        detect("flowchart-elk TD; A-->B"),
        Some(DiagramType::Flowchart)
    );
    assert!(matches!(
        parse("flowchart-elk TD\n  A-->B"),
        Ok(Diagram::Flowchart(_))
    ));
}

#[test]
fn the_headers_are_case_sensitive_except_ishikawa_railroad_and_wardley() {
    for text in [
        "MINDMAP\n  root",
        "STATEDIAGRAM-V2\n  A --> B",
        "Pie\n\"a\": 1",
        "c4Context\n  title T",
        "Kanban\n  Todo",
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
        assert_eq!(detect_unsupported(text), None, "{text:?}");
    }
}

#[test]
fn the_detectors_without_a_word_boundary_take_any_suffix() {
    // The reference's detectors for these types are plain prefixes, with no `\b`.
    assert_eq!(detect("mindmapper\n  root"), Some(DiagramType::Mindmap));
    assert_eq!(detect_unsupported("kanbanboard\n  Todo"), Some("kanban"));
    assert_eq!(detect_unsupported("treemapX\n  \"Root\""), Some("treemap"));
    assert_eq!(detect_unsupported("radar-betaX\n"), Some("radar"));
    assert_eq!(detect_unsupported("treeView-betaX\n"), Some("tree view"));
    assert_eq!(detect_unsupported("venn-betaX\n"), Some("venn"));
    assert_eq!(detect_unsupported("railroad-betaX\n"), Some("railroad"));
    assert_eq!(
        detect_unsupported("railroad-ebnf-betax\n"),
        Some("railroad")
    );
}

#[test]
fn the_word_boundary_headers_reject_a_word_character_after_the_keyword() {
    // `ishikawa(-beta)?\b`, `agentflow-beta\b` and `swimlane-beta\b`: an ASCII letter, a digit or `_`
    // after the keyword continues the word, while a hyphen or the end of the text does not.
    for text in [
        "ishikawa_x\n  Cause",
        "ishikawax\n  Cause",
        "agentflow-betax\n",
        "agentflow-beta_x\n",
        "agentflow-beta1\n",
        "swimlane-betax\n",
        "swimlane-beta_\n",
    ] {
        assert_eq!(detect_unsupported(text), None, "{text:?}");
    }
    assert_eq!(detect_unsupported("agentflow-beta-x\n"), Some("agentflow"));
    assert_eq!(detect_unsupported("agentflow-beta"), Some("agentflow"));
    assert_eq!(detect_unsupported("swimlane-beta-x\n"), Some("swimlane"));
    assert_eq!(detect_unsupported("swimlane-beta"), Some("swimlane"));
}

#[test]
fn the_cynefin_and_use_case_headers_need_a_space_or_the_end_after_the_keyword() {
    // `cynefin-beta(?:[\s:]|$)` takes a colon, `usecase-beta(?:\s|$)` does not; neither takes a hyphen.
    assert_eq!(detect_unsupported("cynefin-beta:x\n"), Some("cynefin"));
    assert_eq!(detect_unsupported("cynefin-beta\tx\n"), Some("cynefin"));
    assert_eq!(detect_unsupported("cynefin-beta"), Some("cynefin"));
    assert_eq!(detect_unsupported("cynefin-beta-x\n"), None);
    assert_eq!(detect_unsupported("cynefin-betax\n"), None);
    // `detectType` strips the comment first: a comment that ends a line is a boundary, one that does not is not.
    assert_eq!(
        detect_unsupported("cynefin-beta%%note\ncomplex \"A\"\n"),
        Some("cynefin")
    );
    assert_eq!(detect_unsupported("cynefin-beta%%note"), None);
    assert_eq!(detect_unsupported("usecase-beta\n"), Some("use case"));
    assert_eq!(
        detect_unsupported("usecase-beta\u{a0}x\n"),
        Some("use case")
    );
    assert_eq!(detect_unsupported("usecase-beta"), Some("use case"));
    assert_eq!(detect_unsupported("usecase-beta:\n"), None);
    assert_eq!(detect_unsupported("usecase-beta-x\n"), None);
}

#[test]
fn the_four_railroad_headers_are_each_taken_and_the_family_name_is_not() {
    // The family has four keywords with one type. `railroad-beta` is a prefix of none of the others,
    // and the family name alone is not a header.
    assert_eq!(detect_unsupported("railroad\n  rule"), None);
    assert_eq!(detect_unsupported("railroad-ebnf\n  rule"), None);
    assert_eq!(detect_unsupported("railroad-x-beta\n"), None);
}

#[test]
fn a_header_is_taken_only_at_the_start_of_the_text() {
    // A header in the body of another diagram never takes the diagram.
    assert_eq!(
        detect_unsupported("swimlane-beta\n  graph TD\n  a-->b"),
        Some("swimlane")
    );
    assert_eq!(
        detect_unsupported("agentflow-beta\n  graph LR"),
        Some("agentflow")
    );
    for (text, kind) in [
        ("flowchart LR\n  swimlane-beta\n", DiagramType::Flowchart),
        ("graph TD\n  agentflow-beta", DiagramType::Flowchart),
        ("sequenceDiagram\n  swimlane-beta", DiagramType::Sequence),
        ("pie\n  \"timeline\": 1", DiagramType::Pie),
    ] {
        assert_eq!(detect(text), Some(kind), "{text:?}");
        assert_eq!(detect_unsupported(text), None, "{text:?}");
    }
    assert_eq!(
        detect_unsupported("timeline\n  railroad-beta"),
        Some("timeline")
    );
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
    assert_eq!(
        detect_unsupported("%%{init: {\"theme\": \"dark\"}}%%\n%% note\nkanban"),
        Some("kanban")
    );
}

#[test]
fn an_unclosed_directive_is_not_skipped() {
    assert_eq!(detect("%%{init: {}\npie"), None);
    assert_eq!(detect_unsupported("%%{init: {}\nkanban"), None);
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
        detect_unsupported("\n---\n  theme: forest\n---\n  requirementDiagram"),
        Some("requirement")
    );
}

#[test]
fn parse_dispatches_on_the_detected_type() {
    assert!(matches!(parse("pie\n\"a\": 1"), Ok(Diagram::Pie(_))));
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
    for text in ["unknown diagram\n  x", "sankey-beta\n  a,b,8"] {
        let error = parse(text).expect_err("no diagram type that ferrigura draws matches");
        assert_eq!((error.line, error.column), (1, 1));
        assert_eq!(error.message, "no diagram type detected");
    }
}
