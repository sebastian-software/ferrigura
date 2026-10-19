//! One test for each quirk of the reference's railroad renderer that changes its output
//! (`docs-port/railroad/NOTES.md`, sections 6.2 to 6.8). Each names the reference's line, and the
//! probe of `tools/port/geometry/probes/railroad` that checks it against Chromium.

mod support;

use ferrigura_model::config::RailroadDiagramConfig;
use ferrigura_model::theme::Theme;
use ferrigura_render::common::Element;
use ferrigura_render::railroad::layout::{Spacing, draw_rules};
use ferrigura_render::railroad::text::TextMeasure;
use support::{
    Stub, children_classed, classed, colours, descendants, draw_real, draw_stub, parse, path_data,
    render_default, render_with, spacing, tokens, tokens_of,
};

/// The rectangle of a box element: the child with a width.
fn rect_of(element: &Element) -> &Element {
    element
        .children()
        .iter()
        .find(|child| child.get("width").is_some())
        .expect("a rectangle")
}

/// The markup of the rule of a text, drawn with the stub.
fn rule_markup(text: &str) -> String {
    draw_stub(text).rules[0].render()
}

/// The `r = …` rule of `railroad-abnf-beta`, `1*3DIGIT`, repeats at least once, and `3DIGIT` is
/// drawn as the same loop as the reference's `+`: the maximum is not drawn (`railroadRenderer.ts`
/// 521-522, `node.min` alone, and 378).
#[test]
fn a_repetition_draws_its_minimum_and_not_its_maximum() {
    let bounded = rule_markup("railroad-abnf-beta\nr = 1*3DIGIT ;\n");
    let plus = rule_markup("railroad-beta\nr = oneOrMore(nonterminal(\"DIGIT\")) ;\n");
    assert_eq!(bounded, plus);
    let exact = rule_markup("railroad-abnf-beta\nr = 3DIGIT ;\n");
    assert_eq!(exact, plus);
}

/// `*0DIGIT` is a repetition from zero, and the bypass is drawn above the loop (`e05-rep-zero-zero`).
#[test]
fn a_repetition_from_zero_draws_the_bypass_above_the_loop() {
    let zero = rule_markup("railroad-abnf-beta\nr = *0DIGIT ;\n");
    let star = rule_markup("railroad-beta\nr = zeroOrMore(nonterminal(\"DIGIT\")) ;\n");
    assert_eq!(zero, star);
    let drawing = draw_stub("railroad-abnf-beta\nr = *0DIGIT ;\n");
    let repetition = classed(&descendants(&drawing.rules[0]), "railroad-repetition")[0];
    // Two forward lines, the loop, and the bypass.
    assert_eq!(children_classed(repetition, "railroad-line").len(), 4);
}

/// The `+` repetition has no bypass: three lines, the forward pair and the loop.
#[test]
fn a_plus_repetition_has_no_bypass() {
    let drawing = draw_stub("railroad-beta\nr = oneOrMore(terminal(\"a\")) ;\n");
    let repetition = classed(&descendants(&drawing.rules[0]), "railroad-repetition")[0];
    assert_eq!(children_classed(repetition, "railroad-line").len(), 3);
}

/// An empty diagram is 200 by 100, and draws no rule (`railroadRenderer.ts` 650-654, `e04-empty`).
#[test]
fn an_empty_diagram_is_200_by_100_with_no_rule() {
    let svg = render_default("railroad-beta\n");
    assert!(svg.contains("viewBox=\"0 0 200 100\""), "{svg}");
    assert!(svg.contains("style=\"max-width: 200px;\""), "{svg}");
    assert!(!svg.contains("class=\"railroad-rule\""));
}

/// An empty terminal is the box of its padding: an empty text has no glyph, so it measures 0 by 0
/// (`svg/c12-emptyterminal.svg`).
#[test]
fn an_empty_terminal_is_the_box_of_its_padding() {
    let drawing = draw_real("railroad-beta\nr = terminal(\"\") ;\n");
    let terminal = classed(&descendants(&drawing.rules[0]), "railroad-terminal")[0];
    let rect = rect_of(terminal);
    assert_eq!(rect.get("width"), Some("20"));
    assert_eq!(rect.get("height"), Some("20"));
}

/// A line break in a terminal is drawn and measured as a space (`svg/c13-newline.svg`): the box of
/// `a\nb` is the box of `a b`, 39.21875 wide.
#[test]
fn a_line_break_in_a_terminal_is_measured_as_a_space() {
    let broken = draw_real("railroad-beta\nr = terminal(\"a\\nb\") ;\n");
    let spaced = draw_real("railroad-beta\nr = terminal(\"a b\") ;\n");
    let width = |drawing: &ferrigura_render::railroad::layout::Drawing| {
        rect_of(classed(&descendants(&drawing.rules[0]), "railroad-terminal")[0])
            .get("width")
            .map(str::to_owned)
    };
    assert_eq!(width(&broken).as_deref(), Some("39.21875"));
    assert_eq!(width(&broken), width(&spaced));
}

/// A rule's name is measured in the regular weight, though it is drawn in bold
/// (`railroadRenderer.ts` 542: `measureText(name + ' =')`, and `.railroad-rule-name` is bold). The
/// start line begins at the regular width of `digit =` plus 20 plus the marker radius (`NOTES.md` 6.4).
#[test]
fn the_rule_name_is_measured_in_the_regular_weight() {
    let drawing = draw_real("railroad-beta\ndigit = terminal(\"0\") ;\n");
    let start_line = children_classed(&drawing.rules[0], "railroad-line")[0];
    assert_eq!(path_data(start_line), "M 65.40625 20 L 80.40625 20");
}

/// A special is `? text ?`, measured and drawn with its question marks (`railroadRenderer.ts` 473-475).
#[test]
fn a_special_is_measured_and_drawn_with_its_question_marks() {
    let drawing = draw_real("railroad-beta\nr = special(\"x y\") ;\n");
    let markup = drawing.rules[0].render();
    assert!(markup.contains(">? x y ?</text>"), "{markup}");
    let special = classed(&descendants(&drawing.rules[0]), "railroad-special")[0];
    // `? x y ?` is 38.78125 wide (`NOTES.md` 6.4), and the padding is on each side.
    assert_eq!(rect_of(special).get("width"), Some("58.78125"));
}

/// A choice's alternative on the centre line is joined by a straight line, not by arcs
/// (`railroadRenderer.ts` 245-247 and 269-271). With three alternatives of 36 and a gap of 8, the
/// middle one is centred on 62, so its joins are the straight lines `M 0 62 L 20 62` and
/// `M 48 62 L 68 62`, while the first and the last are joined by arcs.
#[test]
fn a_choice_alternative_on_the_centre_line_is_joined_straight() {
    let drawing = draw_stub(
        "railroad-beta\nr = choice(terminal(\"a\"), terminal(\"b\"), terminal(\"c\")) ;\n",
    );
    let choice = classed(&descendants(&drawing.rules[0]), "railroad-choice")[0];
    let lines: Vec<&str> = children_classed(choice, "railroad-line")
        .into_iter()
        .map(path_data)
        .collect();
    assert_eq!(lines.len(), 6);
    assert_eq!(lines[2], "M 0 62 L 20 62");
    assert_eq!(lines[3], "M 48 62 L 68 62");
    assert!(lines[0].contains('A'), "{}", lines[0]);
    assert!(lines[4].contains('A'), "{}", lines[4]);
}

/// The bypass of an optional is the reference's arc over the item (`railroadRenderer.ts` 345-355).
#[test]
fn an_optional_bypass_is_the_reference_arc_above_the_item() {
    let drawing = draw_stub("railroad-beta\nr = optional(terminal(\"a\")) ;\n");
    let optional = classed(&descendants(&drawing.rules[0]), "railroad-optional")[0];
    let lines = children_classed(optional, "railroad-line");
    assert_eq!(lines.len(), 3);
    assert_eq!(
        path_data(lines[2]),
        "M 0 38 A 10 10 0 0 0 10 28 L 10 10 A 10 10 0 0 1 20 0 L 48 0 A 10 10 0 0 1 58 10 L 58 28 A 10 10 0 0 0 68 38"
    );
}

/// A padding of 0 keeps a rule at least 40 high (`railroadRenderer.ts` 600-601): an empty terminal
/// would be 20 high, and the diagram 20 + 8 + 20 with no padding (`max(40, …)`, the rule then 40).
#[test]
fn a_rule_is_at_least_forty_high() {
    let diagram = parse("railroad-beta\nr = terminal(\"\") ;\n");
    let spacing = Spacing {
        padding: 0.0,
        ..spacing()
    };
    let mut measure = TextMeasure::new(14.0);
    let drawing = draw_rules(&diagram.rules, &spacing, &colours(), &mut measure).expect("measures");
    // The rule is 40 high, followed by one vertical separation of 8, and no padding.
    assert_eq!(drawing.height, 48.0);
}

/// The compact mode and the show-markers keys change nothing: they are not read by the renderer
/// (`styles.ts` 22, 44, and `NOTES.md` 6.8; probe `g04-inert-keys`).
#[test]
fn compact_mode_and_show_markers_change_nothing() {
    let text = "railroad-beta\ndigit = terminal(\"0\") ;\n";
    let light = tokens(Theme::ReduxColor);
    let default = render_with(
        text,
        &RailroadDiagramConfig::default(),
        &light,
        None,
        "railroad",
    );
    let inert = RailroadDiagramConfig {
        compact_mode: true,
        show_markers: false,
        ..RailroadDiagramConfig::default()
    };
    assert_eq!(default, render_with(text, &inert, &light, None, "railroad"));
}

/// The marker radius sets the radius of the start and the end markers (`railroadRenderer.ts` 566, 574).
#[test]
fn the_marker_radius_sets_the_markers() {
    let config = RailroadDiagramConfig {
        marker_radius: 8.0,
        ..RailroadDiagramConfig::default()
    };
    let svg = render_with(
        "railroad-beta\ndigit = terminal(\"0\") ;\n",
        &config,
        &tokens(Theme::ReduxColor),
        None,
        "railroad",
    );
    assert_eq!(svg.matches(" r=\"8\"").count(), 2, "{svg}");
}

/// A theme colour that is set but is not a colour does not fall through to the next variable: the
/// `??` of `styles.ts` 136-138 keeps the first, and the sanitiser then gives the default.
#[test]
fn a_set_invalid_theme_colour_does_not_fall_through_to_the_next_variable() {
    let light =
        tokens_of(serde_json::json!({ "secondBkg": "url(x)", "secondaryColor": "#123456" }));
    let svg = render_with(
        "railroad-beta\ndigit = terminal(\"0\") ;\n",
        &RailroadDiagramConfig::default(),
        &light,
        None,
        "railroad",
    );
    assert!(svg.contains("--railroadTerminalFill:#FFFFC0;"), "{svg}");
    assert!(!svg.contains("#123456"));
}

/// A section colour that is not a colour falls back to the theme's value (`styles.ts` 85-92; probe
/// `g05-colours`).
#[test]
fn a_section_colour_that_is_not_a_colour_falls_back_to_the_theme() {
    let config = RailroadDiagramConfig {
        non_terminal_stroke: Some("url(x)".to_owned()),
        ..RailroadDiagramConfig::default()
    };
    let svg = render_with(
        "railroad-beta\ndigit = terminal(\"0\") ;\n",
        &config,
        &tokens(Theme::ReduxColor),
        None,
        "railroad",
    );
    // `primaryBorderColor` of redux-color.
    assert!(svg.contains("--railroadNonTerminalStroke:hsl(247.5, 0%, 9.2156862745%);"));
}

/// A negative number falls back to the default (`styles.ts` 103-112; the probes `g06-bad-numbers`).
#[test]
fn a_negative_number_falls_back_to_the_default() {
    let config = RailroadDiagramConfig {
        padding: -1.0,
        ..RailroadDiagramConfig::default()
    };
    let svg = render_with(
        "railroad-beta\ndigit = terminal(\"0\") ;\n",
        &config,
        &tokens(Theme::ReduxColor),
        None,
        "railroad",
    );
    // The padding of 10 gives the probed viewBox of the simple rule.
    assert!(svg.contains("viewBox=\"0 0 144.484375 86.5\""), "{svg}");
}

/// The theme's font size is read as a number with its unit (`styles.ts` 114-123): `16px` gives 16,
/// so a terminal of `0` is 10.375 wide plus the padding (`NOTES.md` 6.4).
#[test]
fn the_theme_font_size_is_read_with_its_unit() {
    let light = tokens_of(serde_json::json!({ "fontSize": "16px" }));
    let svg = render_with(
        "railroad-beta\ndigit = terminal(\"0\") ;\n",
        &RailroadDiagramConfig::default(),
        &light,
        None,
        "railroad",
    );
    assert!(svg.contains("font-size:16px;"), "{svg}");
    assert!(svg.contains("width=\"30.375\""), "{svg}");
}

/// With `useMaxWidth: false` the root has the size as its `width` and `height` attributes, and no
/// `max-width` (`setupGraphViewbox.js` 23-33; probe `g02-nomaxwidth`).
#[test]
fn use_max_width_false_writes_the_size_as_attributes() {
    let config = RailroadDiagramConfig {
        use_max_width: false,
        ..RailroadDiagramConfig::default()
    };
    let svg = render_with(
        "railroad-beta\ndigit = terminal(\"0\") ;\n",
        &config,
        &tokens(Theme::ReduxColor),
        None,
        "railroad",
    );
    let root = &svg[..svg.find('>').expect("the root tag")];
    assert!(root.contains("width=\"144.484375\""), "{root}");
    assert!(root.contains("height=\"86.5\""), "{root}");
    assert!(!root.contains("max-width"));
}

/// A hash entity the user typed is drawn as the character it stands for: `encodeEntities` writes a
/// placeholder for `#amp;`, and the decoding of the whole SVG string (`mermaidAPI.ts` 359) turns it
/// into `&amp;` in the markup, which the browser reads as `&` (`probes/railroad/e01-entity`).
#[test]
fn a_hash_entity_is_drawn_as_the_character_it_stands_for() {
    let svg = render_default("railroad-beta\nr = terminal(\"#amp;\") ;\n");
    assert!(svg.contains(">&amp;</text>"), "{svg}");
}

/// A plain `&amp;` is not a placeholder: `encodeEntities` leaves it, the database keeps it, and the
/// text node shows the five characters, as the reference does (`railroadRenderer.ts` 99 sets the
/// text, and the serialiser escapes its `&`).
#[test]
fn a_plain_entity_is_shown_as_written() {
    let svg = render_default("railroad-beta\nr = terminal(\"a &amp; b\") ;\n");
    assert!(svg.contains(">a &amp;amp; b</text>"), "{svg}");
}

/// Two rules with the same name are both drawn, in order (`railroadRenderer.ts` 609-623; probe
/// `c10-duplicates`).
#[test]
fn duplicate_rules_are_both_drawn() {
    let svg = render_default("railroad-beta\nr = terminal(\"a\") ;\nr = terminal(\"b\") ;\n");
    assert_eq!(svg.matches("class=\"railroad-rule\"").count(), 2);
}

/// The stub is the reference's `getBBox` (`railroadRenderer.spec.ts`): its width is the UTF-16 length
/// times 8, at least 1.
#[test]
fn the_reference_stub_measures_by_utf16_length() {
    use ferrigura_render::railroad::layout::Measure;
    let mut stub = Stub;
    assert_eq!(stub.text_size("").expect("measures").width, 1.0);
    assert_eq!(stub.text_size("abc").expect("measures").width, 24.0);
}

/// The root names the detector of the header the text starts with, after the front matter and the
/// directives, in any letter case: the detectors of `railroadDetector.ts` and its three siblings are
/// case-insensitive (`/i`), and the diagram type of the reference's SVG is the detector's id.
#[test]
fn the_detector_is_the_one_of_the_header() {
    use ferrigura_syntax::railroad::detector_id;
    assert_eq!(
        detector_id("---\ntitle: x\n---\n%%{init: {}}%%\nrailroad-peg-beta\nR <- \"a\" ;\n"),
        "railroadPeg"
    );
    assert_eq!(detector_id("RAILROAD-EBNF-BETA\n"), "railroadEbnf");
    assert_eq!(detector_id("railroad-abnf-beta"), "railroadAbnf");
    assert_eq!(detector_id("railroad-beta"), "railroad");
    assert_eq!(detector_id("not a railroad"), "railroad");
}
