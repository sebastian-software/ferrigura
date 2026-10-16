//! Pie diagrams through the public entry point: the slices, their labels, the legend and the text.

use ferrigura::render;
use ferrigura_model::value::Value;

fn svg(text: &str) -> String {
    render(text, &Value::empty_object()).expect("the pie renders")
}

/// The labels of the slices, in document order, from the `class="slice"` texts.
fn slice_labels(svg: &str) -> Vec<String> {
    svg.split("class=\"slice\"")
        .skip(1)
        .filter_map(|rest| rest.split_once('>'))
        .filter_map(|(_, after)| after.split_once('<'))
        .map(|(label, _)| label.to_owned())
        .collect()
}

#[test]
fn should_render_slices_in_input_order_not_sorted_by_size() {
    let out = svg("pie\n  \"A\" : 10\n  \"B\" : 100\n  \"C\" : 50\n");
    assert_eq!(out.matches("class=\"pieCircle ").count(), 3);
    assert_eq!(slice_labels(&out), ["6%", "63%", "31%"]);
}

#[test]
fn should_leave_out_slices_that_round_to_zero_percent_but_keep_their_legend() {
    let out = svg("pie\n  \"A\" : 10\n  \"B\" : 100\n  \"C\" : 0.1\n  \"D\" : 50\n");
    assert_eq!(out.matches("class=\"pieCircle ").count(), 3);
    assert_eq!(out.matches("class=\"legend\"").count(), 4);
    assert!(!slice_labels(&out).iter().any(|label| label == "0%"));
}

#[test]
fn should_write_one_legend_entry_per_section_with_its_value_when_show_data_is_set() {
    let out = svg("pie showData\n  \"Dogs\" : 386\n  \"Cats\" : 85\n");
    assert_eq!(out.matches("class=\"legend\"").count(), 2);
    assert!(out.contains(">Dogs [386]</text>"), "{out}");
    assert!(out.contains(">Cats [85]</text>"), "{out}");
}

#[test]
fn should_escape_the_title_and_the_labels() {
    let out = svg("pie title Tom & \"Jerry\" <3\n  \"a<b\" : 1\n");
    assert!(out.contains("Tom &amp; &quot;Jerry&quot; &lt;3"), "{out}");
    assert!(out.contains(">a&lt;b</text>"), "{out}");
}

#[test]
fn should_not_render_a_diagram_type_that_is_not_ported() {
    assert!(render("gantt\n  title A", &Value::empty_object()).is_err());
}

#[test]
fn should_embed_the_font_as_a_subset_family() {
    let out = svg("pie\n  \"A\" : 1\n");
    assert!(out.contains("@font-face{font-family:\"fg-"), "{out}");
    assert!(out.contains("text{white-space: pre;}"), "{out}");
}
