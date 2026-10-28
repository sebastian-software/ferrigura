//! Port of `packages/mermaid/src/diagrams/user-journey/journeyDb.spec.js`. Test names follow the
//! spec's titles in snake_case; a title that repeats gets `_2` in source order, and the rows of an
//! `it.each` table are named after their `$fn`.
//!
//! The DB has no public surface, so each test drives `parse`, as the flowchart DB spec does. The
//! reference's `clear` runs at the start of every parse (`Diagram.fromText`), so a parse that follows
//! another one starts from an empty DB, which is what the `clear` tests check.

use ferrigura_syntax::journey::parse;

#[test]
fn should_clear_get_tasks() {
    // The reference's beforeEach adds a section and two tasks, then clears the DB.
    parse("journey\nsection weekends skip test\ntest1:4: id1, id3\ntest2:2: id2\n")
        .expect("parses");
    let diagram = parse("journey").expect("parses");
    assert!(diagram.tasks.is_empty());
}

#[test]
fn should_clear_get_acc_title() {
    parse("journey\naccTitle: first\nsection weekends skip test\ntest1:4: id1, id3\n")
        .expect("parses");
    let diagram = parse("journey").expect("parses");
    assert_eq!(diagram.acc_title, "");
}

#[test]
fn should_clear_get_sections() {
    parse("journey\nsection weekends skip test\ntest1:4: id1, id3\ntest2:2: id2\n")
        .expect("parses");
    let diagram = parse("journey").expect("parses");
    assert!(diagram.sections.is_empty());
}

#[test]
fn should_clear_get_actors() {
    parse("journey\nsection weekends skip test\ntest1:4: id1, id3\ntest2:2: id2\n")
        .expect("parses");
    let diagram = parse("journey").expect("parses");
    assert!(diagram.actors.is_empty());
}

#[test]
fn should_clear_get_tasks_2() {
    parse(
        "journey\naccDescr: first\nsection weekends skip test\ntest1:3: id1, id3\ntest2:1: id2\n",
    )
    .expect("parses");
    let diagram = parse("journey").expect("parses");
    assert!(diagram.tasks.is_empty());
}

#[test]
fn should_clear_get_acc_title_2() {
    parse("journey\naccTitle: first\nsection weekends skip test\ntest1:3: id1, id3\n")
        .expect("parses");
    let diagram = parse("journey").expect("parses");
    assert_eq!(diagram.acc_title, "");
}

#[test]
fn should_clear_get_acc_description() {
    parse("journey\naccDescr: first\nsection weekends skip test\ntest1:3: id1, id3\n")
        .expect("parses");
    let diagram = parse("journey").expect("parses");
    assert_eq!(diagram.acc_descr, "");
}

#[test]
fn should_clear_get_sections_2() {
    parse("journey\nsection weekends skip test\ntest1:3: id1, id3\ntest2:1: id2\n")
        .expect("parses");
    let diagram = parse("journey").expect("parses");
    assert!(diagram.sections.is_empty());
}

/// The reference's `tasks and actors should be added`, with the DB calls written as the grammar
/// makes them. The actor `Child#1` is written `Child#35;1`: a `#` starts a comment in the text, and
/// `#35;` is the entity that reads as `#`.
#[test]
fn tasks_and_actors_should_be_added() {
    let diagram = parse(
        "journey\n\
         accTitle: Shopping\n\
         accDescr: A user journey for family shopping\n\
         section Journey to the shops\n\
         Get car keys:5:Dad\n\
         Go to car:3:Dad, Mum, Child#35;1, Child#35;2\n\
         Drive to supermarket:4:Dad\n\
         section Do shopping\n\
         Go shopping:5:Mum\n",
    )
    .expect("parses");

    assert_eq!(diagram.acc_title, "Shopping");
    assert_eq!(diagram.acc_descr, "A user journey for family shopping");

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
            (5.0, vec!["Dad"], "Journey to the shops", "Get car keys"),
            (
                3.0,
                vec!["Dad", "Mum", "Child#1", "Child#2"],
                "Journey to the shops",
                "Go to car"
            ),
            (
                4.0,
                vec!["Dad"],
                "Journey to the shops",
                "Drive to supermarket"
            ),
            (5.0, vec!["Mum"], "Do shopping", "Go shopping"),
        ]
    );
    assert_eq!(diagram.actors, ["Child#1", "Child#2", "Dad", "Mum"]);
    assert_eq!(diagram.sections, ["Journey to the shops", "Do shopping"]);
}
