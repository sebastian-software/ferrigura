//! Port of `packages/parser/tests/pie.test.ts`. Each `it.each` table is one test that runs every
//! input of the table, under the table's title.
//!
//! The reference asserts Langium AST fields (`$type`, `showData`, `sections[i].label`); the model
//! of this port carries the same facts after the DB rules, so the assertions read the model.

use ferrigura_model::pie::PieDiagram;
use ferrigura_syntax::pie::parse;

fn accepts(input: &str) -> PieDiagram {
    parse(input).unwrap_or_else(|error| panic!("{input:?} should parse: {error}"))
}

#[test]
fn should_handle_regular_pie() {
    for input in ["pie", "  pie  ", "\tpie\t", "\n    \tpie\n    "] {
        let diagram = accepts(input);
        assert!(!diagram.show_data, "{input:?}");
    }
}

#[test]
fn should_handle_regular_show_data() {
    for input in [
        "pie showData",
        "  pie  showData  ",
        "\tpie\tshowData\t",
        "\n    pie\tshowData\n    ",
    ] {
        assert!(accepts(input).show_data, "{input:?}");
    }
}

#[test]
fn should_handle_regular_pie_title_in_same_line() {
    for input in [
        "pie title sample title",
        "  pie  title sample title  ",
        "\tpie\ttitle sample title\t",
        "pie\n        \ttitle sample title\n        ",
    ] {
        assert_eq!(accepts(input).title, "sample title", "{input:?}");
    }
}

#[test]
fn should_handle_regular_pie_title_in_different_line() {
    for input in [
        "pie\n        title sample title",
        "pie\n        title sample title\n        ",
        "pie\n        title sample title",
        "pie\n        title sample title\n        ",
    ] {
        assert_eq!(accepts(input).title, "sample title", "{input:?}");
    }
}

#[test]
fn should_handle_regular_pie_show_data_title() {
    for input in [
        "pie showData title sample title",
        "pie showData title sample title\n        ",
    ] {
        let diagram = accepts(input);
        assert!(diagram.show_data, "{input:?}");
        assert_eq!(diagram.title, "sample title", "{input:?}");
    }
}

#[test]
fn should_handle_regular_show_data_title_in_different_line() {
    for input in [
        "pie showData\n        title sample title",
        "pie showData\n        title sample title\n        ",
        "pie showData\n        title sample title",
        "pie showData\n        title sample title\n        ",
    ] {
        let diagram = accepts(input);
        assert!(diagram.show_data, "{input:?}");
        assert_eq!(diagram.title, "sample title", "{input:?}");
    }
}

#[test]
fn should_handle_regular_sections() {
    for input in [
        "pie\n        \"GitHub\":100\n        \"GitLab\":50",
        "pie\n        \"GitHub\"   :   100\n        \"GitLab\"   :   50",
        "pie\n        \"GitHub\"\t:\t100\n        \"GitLab\"\t:\t50",
        "pie\n        \t\"GitHub\" \t : \t 100\n        \t\"GitLab\" \t : \t  50\n        ",
    ] {
        let diagram = accepts(input);
        assert_eq!(diagram.sections[0].label, "GitHub", "{input:?}");
        assert_eq!(diagram.sections[0].value, 100.0, "{input:?}");
        assert_eq!(diagram.sections[1].label, "GitLab", "{input:?}");
        assert_eq!(diagram.sections[1].value, 50.0, "{input:?}");
    }
}

#[test]
fn should_handle_sections_with_show_data() {
    let diagram = accepts("pie showData\n        \"GitHub\": 100\n        \"GitLab\": 50");
    assert!(diagram.show_data);
    assert_eq!(diagram.sections[0].label, "GitHub");
    assert_eq!(diagram.sections[0].value, 100.0);
    assert_eq!(diagram.sections[1].label, "GitLab");
    assert_eq!(diagram.sections[1].value, 50.0);
}

#[test]
fn should_handle_sections_with_title() {
    let diagram = accepts("pie title sample wow\n        \"GitHub\": 100\n        \"GitLab\": 50");
    assert_eq!(diagram.title, "sample wow");
    assert_eq!(diagram.sections[0].label, "GitHub");
    assert_eq!(diagram.sections[0].value, 100.0);
    assert_eq!(diagram.sections[1].label, "GitLab");
    assert_eq!(diagram.sections[1].value, 50.0);
}

#[test]
fn should_handle_value_with_positive_decimal() {
    let diagram = accepts("pie\n        \"ash\": 60.67\n        \"bat\": 40");
    assert_eq!(diagram.sections[0].label, "ash");
    assert_eq!(diagram.sections[0].value, 60.67);
    assert_eq!(diagram.sections[1].label, "bat");
    assert_eq!(diagram.sections[1].value, 40.0);
}

#[test]
fn should_handle_sections_with_acc_title() {
    let diagram =
        accepts("pie accTitle: sample wow\n        \"GitHub\": 100\n        \"GitLab\": 50");
    assert_eq!(diagram.acc_title, "sample wow");
    assert_eq!(diagram.sections[0].label, "GitHub");
    assert_eq!(diagram.sections[0].value, 100.0);
    assert_eq!(diagram.sections[1].label, "GitLab");
    assert_eq!(diagram.sections[1].value, 50.0);
}

#[test]
fn should_handle_sections_with_single_line_acc_descr() {
    let diagram =
        accepts("pie accDescr: sample wow\n        \"GitHub\": 100\n        \"GitLab\": 50");
    assert_eq!(diagram.acc_descr, "sample wow");
    assert_eq!(diagram.sections[0].label, "GitHub");
    assert_eq!(diagram.sections[0].value, 100.0);
    assert_eq!(diagram.sections[1].label, "GitLab");
    assert_eq!(diagram.sections[1].value, 50.0);
}

#[test]
fn should_handle_sections_with_multi_line_acc_descr() {
    let diagram = accepts(
        "pie accDescr {\n            sample wow\n        }\n        \"GitHub\": 100\n        \"GitLab\": 50",
    );
    assert_eq!(diagram.acc_descr, "sample wow");
    assert_eq!(diagram.sections[0].label, "GitHub");
    assert_eq!(diagram.sections[0].value, 100.0);
    assert_eq!(diagram.sections[1].label, "GitLab");
    assert_eq!(diagram.sections[1].value, 50.0);
}
