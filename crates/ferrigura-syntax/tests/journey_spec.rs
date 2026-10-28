//! Port of `packages/mermaid/src/diagrams/user-journey/parser/journey.spec.js`. Test names follow the
//! spec's titles in snake_case, with the parenthesised part of a title written as words.
//!
//! The reference's `not.toThrow()` becomes `parse(…).is_ok()`. Its DB checks read the model. The
//! journey reference has no render or DOM spec, so no test here is ignored.

use ferrigura_model::journey::JourneyDiagram;
use ferrigura_syntax::journey::parse;

fn parses(text: &str) -> JourneyDiagram {
    parse(text).expect("parses")
}

#[test]
fn should_handle_a_title_definition() {
    let diagram = parses("journey\ntitle Adding journey diagram functionality to mermaid");
    assert_eq!(
        diagram.title,
        "Adding journey diagram functionality to mermaid"
    );
}

#[test]
fn should_handle_an_accessibility_description_accdescr() {
    let diagram = parses(
        "journey\n\
         accDescr: A user journey for family shopping\n\
         title Adding journey diagram functionality to mermaid\n\
         section Order from website",
    );
    assert_eq!(diagram.acc_descr, "A user journey for family shopping");
}

#[test]
fn should_handle_an_accessibility_multiline_description_accdescr() {
    let diagram = parses(
        "journey\n\
         accDescr {\n        A user journey for\n        family shopping\n      }\
         title Adding journey diagram functionality to mermaid\n\
         accTitle: Adding acc journey diagram functionality to mermaid\n\
         section Order from website",
    );
    assert_eq!(diagram.acc_descr, "A user journey for\nfamily shopping");
    assert_eq!(
        diagram.title,
        "Adding journey diagram functionality to mermaid"
    );
    assert_eq!(
        diagram.acc_title,
        "Adding acc journey diagram functionality to mermaid"
    );
}

#[test]
fn should_handle_an_accessibility_title_accdescr() {
    let diagram = parses("journey\n    accTitle: The title\n    section Order from website");
    assert_eq!(diagram.acc_descr, "");
    assert_eq!(diagram.acc_title, "The title");
}

#[test]
fn should_handle_a_section_definition() {
    let diagram = parses(
        "journey\n\
         title Adding journey diagram functionality to mermaid\n\
         section Order from website",
    );
    assert_eq!(diagram.sections, ["Order from website"]);
}

#[test]
fn should_handle_multiline_section_titles_with_different_line_breaks() {
    let diagram = parses(
        "journey\n\
         title Adding gantt diagram functionality to mermaid\n\
         section Line1<br>Line2<br/>Line3</br />Line4<br\t/>Line5",
    );
    assert_eq!(
        diagram.sections,
        ["Line1<br>Line2<br/>Line3</br />Line4<br\t/>Line5"]
    );
}

#[test]
fn should_handle_a_task_definition() {
    let diagram = parses(
        "journey\n\
         title Adding journey diagram functionality to mermaid\n\
         section Documentation\n\
         A task: 5: Alice, Bob, Charlie\n\
         B task: 3:Bob, Charlie\n\
         C task: 5\n\
         D task: 5: Charlie, Alice\n\
         E task: 5:\n\
         section Another section\n\
         P task: 5:\n\
         Q task: 5:\n\
         R task: 5:",
    );

    let tasks: Vec<(f64, Vec<&str>, &str, &str)> = diagram
        .tasks
        .iter()
        .map(|task| {
            (
                task.score,
                task.people.iter().map(String::as_str).collect(),
                task.section.as_str(),
                task.task.as_str(),
            )
        })
        .collect();
    assert_eq!(
        tasks,
        vec![
            (
                5.0,
                vec!["Alice", "Bob", "Charlie"],
                "Documentation",
                "A task"
            ),
            (3.0, vec!["Bob", "Charlie"], "Documentation", "B task"),
            (5.0, vec![], "Documentation", "C task"),
            (5.0, vec!["Charlie", "Alice"], "Documentation", "D task"),
            (5.0, vec![""], "Documentation", "E task"),
            (5.0, vec![""], "Another section", "P task"),
            (5.0, vec![""], "Another section", "Q task"),
            (5.0, vec![""], "Another section", "R task"),
        ]
    );
}
