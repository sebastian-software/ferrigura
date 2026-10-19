//! The radar DB, as `packages/mermaid/src/diagrams/radar/db.ts` holds it after a parse, through the
//! public parse. Each expected value is a probe of `docs-port/radar/NOTES.md`, sections 3.2 to 3.6,
//! and the reference gives the same value on the same text (`pnpm port:diff radar`).

use ferrigura_model::radar::{Graticule, RadarAxis, RadarCurve, RadarDiagram};
use ferrigura_syntax::radar::parse;

fn parsed(text: &str) -> RadarDiagram {
    parse(text).expect("the diagram parses")
}

fn rejected(text: &str) -> String {
    parse(text).expect_err("the diagram is rejected").message
}

fn axis(name: &str, label: &str) -> RadarAxis {
    RadarAxis {
        name: name.into(),
        label: label.into(),
    }
}

fn curve(name: &str, label: &str, entries: &[f64]) -> RadarCurve {
    RadarCurve {
        name: name.into(),
        label: label.into(),
        entries: entries.to_vec(),
    }
}

mod titles_and_accessibility {
    use super::*;

    #[test]
    fn the_title_is_sanitised_and_its_entity_shows_double_encoded() {
        // D18: the `&` of the title is held as `&amp;`, which the renderer shows as written.
        let diagram = parsed("radar-beta\n  title Hello <b>bold</b> & more\n  axis A");
        assert_eq!(diagram.title, "Hello <b>bold</b> &amp; more");
    }

    #[test]
    fn a_script_attribute_is_removed_from_the_title() {
        // D19: `onerror` is removed, and the quoted attribute is written with single quotes.
        let diagram = parsed("radar-beta\n  title <img src=x onerror=alert(1)>T\n  axis A");
        assert_eq!(diagram.title, "<img src=\"x\">T");
    }

    #[test]
    fn the_accessibility_texts_are_sanitised() {
        // D20.
        let diagram = parsed(
            "radar-beta\n  accTitle: A <b>t</b>\n  accDescr: x<img src=x onerror=alert(1)>y\n  axis A",
        );
        assert_eq!(diagram.acc_title, "A <b>t</b>");
        assert_eq!(diagram.acc_descr, "x<img src=\"x\">y");
    }

    #[test]
    fn the_axis_and_curve_labels_are_not_sanitised() {
        // D21: the labels keep their markup as text.
        let diagram = parsed("radar-beta\n  axis A[\"<b>x</b> & y\"]\n  curve c[\"<i>k</i>\"]{1}");
        assert_eq!(diagram.axes, vec![axis("A", "<b>x</b> & y")]);
        assert_eq!(diagram.curves, vec![curve("c", "<i>k</i>", &[1.0])]);
    }

    #[test]
    fn an_entity_in_a_label_or_a_title_holds_the_placeholder() {
        // D22, D23: the placeholder of the entity, which the renderer turns back into `&#35;`.
        let label = parsed("radar-beta\n  axis A[\"a #35; b\"]\n  curve c{1}");
        assert_eq!(label.axes, vec![axis("A", "a ﬂ°°35¶ß b")]);
        let title = parsed("radar-beta\n  title a #35; b\n  axis A");
        assert_eq!(title.title, "a ﬂ°°35¶ß b");
    }

    #[test]
    fn the_title_of_the_front_matter_is_used_unless_the_grammar_gives_one() {
        // D24 to D26.
        let front = parsed("---\ntitle: FM Title\n---\nradar-beta\n  axis A");
        assert_eq!(front.title, "FM Title");
        let body = parsed("---\ntitle: FM Title\n---\nradar-beta\n  title Body Title\n  axis A");
        assert_eq!(body.title, "Body Title");
        let blank = parsed("---\ntitle: FM Title\n---\nradar-beta\n  title\n  axis A");
        assert_eq!(blank.title, "FM Title");
    }

    #[test]
    fn the_last_title_line_wins_even_when_it_is_blank() {
        // The grammar's last title is `''`, which the DB does not set, so no title is held.
        let diagram = parsed("radar-beta\n  title T\n  title\n  axis A");
        assert_eq!(diagram.title, "");
    }

    #[test]
    fn a_title_takes_the_rest_of_the_line_after_a_tab() {
        // `title\taxis A` is the title `axis A`, and no axis is declared.
        let diagram = parsed("radar-beta\n  title\taxis A");
        assert_eq!(diagram.title, "axis A");
        assert!(diagram.axes.is_empty());
    }

    #[test]
    fn a_title_collapses_its_blank_runs() {
        let diagram = parsed("radar-beta\n  title   My  Title");
        assert_eq!(diagram.title, "My Title");
    }

    #[test]
    fn a_block_accessibility_description_loses_its_line_indentation() {
        let diagram =
            parsed("radar-beta\n  accDescr {\n    first line\n    second line\n  }\n  axis A");
        assert_eq!(diagram.acc_descr, "first line\nsecond line");
    }

    #[test]
    fn a_title_word_after_an_axis_is_a_title_line() {
        // `axis A title` is the axis A and an empty title, as the title token takes the blank.
        let diagram = parsed("radar-beta\n  axis A title\n");
        assert_eq!(diagram.axes, vec![axis("A", "A")]);
        assert_eq!(diagram.title, "");
        assert_eq!(parsed("radar-beta\n  axis A title x\n").title, "x");
    }
}

mod axes_and_curves {
    use super::*;

    #[test]
    fn an_axis_label_defaults_to_its_name_and_an_empty_label_is_kept() {
        // D2, and the empty label `B[""]`, which is a label, not an absent one.
        let diagram = parsed("radar-beta\n  axis A[\"Alpha\"], B\n  curve c{1,2}");
        assert_eq!(diagram.axes, vec![axis("A", "Alpha"), axis("B", "B")]);
        let empty = parsed("radar-beta\n  axis A\n  axis B[\"\"]\n  curve c{1,2}");
        assert_eq!(empty.axes[1], axis("B", ""));
    }

    #[test]
    fn a_label_keeps_its_escapes() {
        let diagram = parsed("radar-beta\n  axis A[\"a\\\"b\"], B[\"c\\td\"]");
        assert_eq!(diagram.axes, vec![axis("A", "a\"b"), axis("B", "c\td")]);
    }

    #[test]
    fn a_duplicate_axis_name_is_kept() {
        // D11.
        let diagram = parsed("radar-beta\n  axis A, A, B\n  curve c{1,2,3}");
        assert_eq!(
            diagram.axes,
            vec![axis("A", "A"), axis("A", "A"), axis("B", "B")]
        );
    }

    #[test]
    fn a_curve_label_defaults_to_its_name() {
        // D3.
        let diagram =
            parsed("radar-beta\n  axis A, B\n  curve c1[\"First one\"]{1,2}\n  curve c2{2,1}");
        assert_eq!(
            diagram.curves,
            vec![
                curve("c1", "First one", &[1.0, 2.0]),
                curve("c2", "c2", &[2.0, 1.0])
            ]
        );
    }

    #[test]
    fn named_entries_are_read_in_axis_order_whatever_the_text_order() {
        // D4.
        let diagram = parsed("radar-beta\n  axis A, B, C\n  curve c{C 3, A 1, B 2}");
        assert_eq!(diagram.curves, vec![curve("c", "c", &[1.0, 2.0, 3.0])]);
    }

    #[test]
    fn named_entries_may_come_before_the_axes_that_they_name() {
        // D5: the axes are set first, whatever the text order.
        let diagram = parsed("radar-beta\n  curve c{A 1, B 2}\n  axis A, B");
        assert_eq!(diagram.curves, vec![curve("c", "c", &[1.0, 2.0])]);
    }

    #[test]
    fn a_named_curve_with_no_axis_is_rejected() {
        // D6.
        assert_eq!(
            rejected("radar-beta\n  curve c{A 1}"),
            "Axes must be populated before curves for reference entries"
        );
    }

    #[test]
    fn an_axis_without_an_entry_is_rejected_with_its_label() {
        // D7, D7b.
        assert!(rejected("radar-beta\n  axis A, B\n  curve c{A 1}").contains("B"));
        let message = rejected("radar-beta\n  axis A, B[\"Bee\"]\n  curve c{A 1}");
        assert!(message.contains("Bee"), "{message}");
    }

    #[test]
    fn an_entry_that_names_no_axis_is_dropped() {
        // D8.
        let diagram = parsed("radar-beta\n  axis A, B\n  curve c{A 1, B 2, Z 9}");
        assert_eq!(diagram.curves, vec![curve("c", "c", &[1.0, 2.0])]);
    }

    #[test]
    fn the_first_entry_of_an_axis_wins() {
        // D9.
        let diagram = parsed("radar-beta\n  axis A, B\n  curve c{A 1, A 5, B 2}");
        assert_eq!(diagram.curves, vec![curve("c", "c", &[1.0, 2.0])]);
    }

    #[test]
    fn a_number_list_is_kept_whatever_its_length() {
        // D10.
        let diagram = parsed("radar-beta\n  axis A, B, C\n  curve c{1,2}");
        assert_eq!(diagram.curves, vec![curve("c", "c", &[1.0, 2.0])]);
    }

    #[test]
    fn a_very_long_curve_is_accepted() {
        // Probed in the reference: 125,000 entries parse and reach the DB. The renderer's spread of
        // the values is its own limit (`docs-port/radar/NOTES.md`, section 6.11, item 21).
        let text = format!(
            "radar-beta\n  axis A\n  curve c{{{}}}",
            vec!["1"; 125_000].join(",")
        );
        assert_eq!(parsed(&text).curves[0].entries.len(), 125_000);
    }

    #[test]
    fn a_curve_with_a_detailed_entry_and_a_colon_reads_its_value() {
        let diagram = parsed("radar-beta\n  axis A\n  curve c{A: 1}");
        assert_eq!(diagram.curves, vec![curve("c", "c", &[1.0])]);
    }

    #[test]
    fn two_curves_keep_their_text_order() {
        let diagram = parsed("radar-beta\n  axis A\n  curve c{1,2,3}\n  curve d{4,5}");
        assert_eq!(
            diagram.curves,
            vec![
                curve("c", "c", &[1.0, 2.0, 3.0]),
                curve("d", "d", &[4.0, 5.0])
            ]
        );
    }

    #[test]
    fn a_number_that_overflows_is_infinite() {
        // O9: a 400-digit number reads as Infinity, and its canonical form is the sentinel.
        let text = format!("radar-beta\n  axis A\n  curve c{{{}}}", "1".repeat(400));
        assert_eq!(parsed(&text).curves[0].entries, vec![f64::INFINITY]);
    }

    #[test]
    fn a_number_of_309_digits_is_still_finite() {
        // O9: 309 digits is about 1.1e308, below the largest double.
        let text = format!("radar-beta\n  axis A\n  curve c{{{}}}", "1".repeat(309));
        let entries = parsed(&text).curves[0].entries.clone();
        assert!(entries[0].is_finite() && entries[0] > 1e308);
    }

    #[test]
    fn a_number_of_310_digits_overflows_to_infinity() {
        // O9: the reference gives Infinity from 310 digits, as the JavaScript `Number` does.
        let text = format!("radar-beta\n  axis A\n  curve c{{{}}}", "1".repeat(310));
        assert_eq!(parsed(&text).curves[0].entries, vec![f64::INFINITY]);
    }
}

mod options {
    use super::*;

    #[test]
    fn the_options_default_when_the_text_gives_none() {
        // D12.
        let diagram = parsed("radar-beta\n  axis A");
        assert!(diagram.options.show_legend);
        assert_eq!(diagram.options.ticks, 5.0);
        assert_eq!(diagram.options.max, None);
        assert_eq!(diagram.options.min, 0.0);
        assert_eq!(diagram.options.graticule, Graticule::Circle);
    }

    #[test]
    fn the_options_take_their_values_and_ticks_are_clamped() {
        // D13: `ticks 40` is clamped to 32.
        let diagram = parsed("radar-beta\n  showLegend false, ticks 40, max 10, min 2\n  axis A");
        assert!(!diagram.options.show_legend);
        assert_eq!(diagram.options.ticks, 32.0);
        assert_eq!(diagram.options.max, Some(10.0));
        assert_eq!(diagram.options.min, 2.0);
    }

    #[test]
    fn the_last_option_of_a_name_wins() {
        // D15.
        let diagram = parsed("radar-beta\n  ticks 3\n  ticks 4\n  axis A");
        assert_eq!(diagram.options.ticks, 4.0);
    }

    #[test]
    fn ticks_keep_their_fraction_and_zero() {
        // D16, D17.
        assert_eq!(
            parsed("radar-beta\n  ticks 2.5\n  axis A").options.ticks,
            2.5
        );
        assert_eq!(parsed("radar-beta\n  ticks 0\n  axis A").options.ticks, 0.0);
    }

    #[test]
    fn a_zero_maximum_and_minimum_are_kept() {
        // D30: zero is a value, and only the absence of a maximum is `null`.
        let diagram = parsed("radar-beta\n  max 0\n  min 0\n  axis A");
        assert_eq!(diagram.options.max, Some(0.0));
        assert_eq!(diagram.options.min, 0.0);
    }

    #[test]
    fn a_maximum_below_the_minimum_is_kept_as_written() {
        // O9: accepted, with both values as written. The renderer divides by the negative range.
        let diagram = parsed("radar-beta\n  axis A, B\n  curve c{1, 2}\n  max 1\n  min 5");
        assert_eq!(diagram.options.max, Some(1.0));
        assert_eq!(diagram.options.min, 5.0);
    }

    #[test]
    fn a_minimum_of_four_hundred_digits_is_infinite() {
        // O9: the sentinel `"Infinity"` in the canonical model.
        let text = format!("radar-beta\n  axis A\n  min {}", "1".repeat(400));
        assert_eq!(parsed(&text).options.min, f64::INFINITY);
    }

    #[test]
    fn the_graticule_takes_polygon_or_circle() {
        assert_eq!(
            parsed("radar-beta\n  graticule polygon\n  showLegend true")
                .options
                .graticule,
            Graticule::Polygon
        );
    }
}
