//! Port of `packages/mermaid/src/diagrams/timeline/timeline.spec.js`. Test names follow the spec's
//! titles in snake_case. The spec reads the DB after the parse, and this port reads the model. The
//! reference's loops over the tasks become exact checks, which is stronger: a loop passes when it
//! has no task to check.

use ferrigura_model::timeline::{TimelineDiagram, TimelinePeriod};
use ferrigura_syntax::timeline::parse;

fn parsed(text: &str) -> TimelineDiagram {
    parse(text).unwrap_or_else(|error| panic!("{text:?} should parse: {error}"))
}

fn period(section: &str, name: &str, events: &[&str]) -> TimelinePeriod {
    TimelinePeriod {
        section: section.to_owned(),
        period: name.to_owned(),
        events: events.iter().map(|event| (*event).to_owned()).collect(),
    }
}

mod when_parsing_a_timeline {
    use super::*;

    #[test]
    fn should_default_to_lr_direction_when_none_is_provided() {
        let diagram = parsed("timeline\n    section abc-123");
        assert_eq!(diagram.direction, "LR");
    }

    #[test]
    fn should_parse_td_direction() {
        let diagram = parsed("timeline TD\n    section abc-123");
        assert_eq!(diagram.direction, "TD");
    }

    #[test]
    fn should_parse_lr_direction() {
        let diagram = parsed("timeline LR\n    section abc-123");
        assert_eq!(diagram.direction, "LR");
    }

    #[test]
    fn should_handle_a_simple_section_definition_abc_123() {
        let diagram = parsed("timeline\n    section abc-123");
        assert_eq!(diagram.sections, ["abc-123"]);
    }

    #[test]
    fn should_handle_a_simple_section_and_only_two_tasks() {
        let text = r#"timeline
    section abc-123
    task1
    task2"#;
        let diagram = parsed(text);
        assert_eq!(diagram.sections, ["abc-123"]);
        assert_eq!(
            diagram.periods,
            [
                period("abc-123", "task1", &[]),
                period("abc-123", "task2", &[]),
            ]
        );
    }

    #[test]
    fn should_handle_a_two_section_and_two_corresponding_tasks() {
        let text = r#"timeline
    section abc-123
    task1
    task2
    section abc-456
    task3
    task4"#;
        let diagram = parsed(text);
        assert_eq!(diagram.sections, ["abc-123", "abc-456"]);
        assert_eq!(
            diagram.periods,
            [
                period("abc-123", "task1", &[]),
                period("abc-123", "task2", &[]),
                period("abc-456", "task3", &[]),
                period("abc-456", "task4", &[]),
            ]
        );
    }

    #[test]
    fn should_handle_a_section_and_task_and_its_events() {
        let text = r#"timeline
    section abc-123
      task1: event1
      task2: event2: event3
   "#;
        let diagram = parsed(text);
        assert_eq!(diagram.sections[0], "abc-123");
        assert_eq!(
            diagram.periods,
            [
                period("abc-123", "task1", &["event1"]),
                period("abc-123", "task2", &["event2", "event3"]),
            ]
        );
    }

    #[test]
    fn should_handle_a_section_and_task_and_its_events_including_markdown_link() {
        let text = r#"timeline
    section abc-123
      task1: [event1](http://example.com)
      task2: event2: event3
   "#;
        let diagram = parsed(text);
        assert_eq!(diagram.sections[0], "abc-123");
        assert_eq!(
            diagram.periods,
            [
                period("abc-123", "task1", &["[event1](http://example.com)"]),
                period("abc-123", "task2", &["event2", "event3"]),
            ]
        );
    }

    #[test]
    fn should_handle_a_section_and_task_and_its_multi_line_events() {
        let text = r#"timeline
    section abc-123
      task1: event1
      task2: event2: event3
           : event4: event5
   "#;
        let diagram = parsed(text);
        assert_eq!(diagram.sections[0], "abc-123");
        assert_eq!(
            diagram.periods,
            [
                period("abc-123", "task1", &["event1"]),
                period(
                    "abc-123",
                    "task2",
                    &["event2", "event3", "event4", "event5"]
                ),
            ]
        );
    }

    #[test]
    fn should_handle_a_title_section_task_and_events_with_semicolons() {
        let text = r#"timeline
      title ;my;title;
      section ;a;bc-123;
      ;ta;sk1;: ;ev;ent1; : ;ev;ent2; : ;ev;ent3;
   "#;
        let diagram = parsed(text);
        assert_eq!(diagram.title, ";my;title;");
        assert_eq!(diagram.sections, [";a;bc-123;"]);
        assert_eq!(
            diagram.periods[0].events,
            [";ev;ent1; ", ";ev;ent2; ", ";ev;ent3;"]
        );
    }

    #[test]
    fn should_handle_a_title_section_task_and_events_with_hashtags() {
        let text = r#"timeline
      title #my#title#
      section #a#bc-123#
      task1: #ev#ent1# : #ev#ent2# : #ev#ent3#
   "#;
        let diagram = parsed(text);
        assert_eq!(diagram.title, "#my#title#");
        assert_eq!(diagram.sections, ["#a#bc-123#"]);
        assert_eq!(diagram.periods[0].period, "task1");
        assert_eq!(
            diagram.periods[0].events,
            ["#ev#ent1# ", "#ev#ent2# ", "#ev#ent3#"]
        );
    }
}
