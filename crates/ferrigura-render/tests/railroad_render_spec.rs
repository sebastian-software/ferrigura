//! The reference's railroad renderer and styles specs (`railroadRenderer.spec.ts` and `styles.spec.ts`),
//! ported to the renderer of `ferrigura_render::railroad` (`docs-port/railroad/NOTES.md`, section 5).
//!
//! The renderer specs measure the text with the reference's stub (`support::Stub`), so their numbers
//! are the reference's own. The styles specs read the stylesheet of the rendered SVG, with the bundled
//! face measurement.

mod support;

use ferrigura_model::config::RailroadDiagramConfig;
use ferrigura_model::theme::Theme;
use regex::Regex;
use support::{
    children_classed, classed, descendants, draw_real, draw_stub, path_data, render_default,
    render_with, tokens, tokens_of,
};

/// The viewBox of a rendered SVG, as its numbers.
fn view_box(svg: &str) -> Vec<f64> {
    let pattern = Regex::new(r#"viewBox="([^"]*)""#).expect("the pattern is valid");
    let text = pattern.captures(svg).expect("a viewBox")[1].to_owned();
    text.split(' ')
        .map(|part| part.parse().expect("a number"))
        .collect()
}

/// The first attribute value of the root element, or the first element of a class, in a rendered SVG.
fn attribute_of_root(svg: &str, name: &str) -> Option<String> {
    let root_end = svg.find('>').expect("the root tag ends");
    let root = &svg[..root_end];
    let pattern = Regex::new(&format!(r#" {name}="([^"]*)""#)).expect("the pattern is valid");
    pattern
        .captures(root)
        .map(|captures| captures[1].to_owned())
}

// --- railroadRenderer.spec.ts -------------------------------------------------------------------

#[test]
fn sets_a_view_box_that_includes_the_full_rendered_diagram_height() {
    let text = "railroad-beta
sign = choice(terminal(\"+\"), terminal(\"-\")) ;
number = sequence(optional(nonterminal(\"sign\")), oneOrMore(nonterminal(\"digit\"))) ;
list = sequence(terminal(\"[\"), optional(sequence(nonterminal(\"number\"), zeroOrMore(sequence(terminal(\",\"), nonterminal(\"number\"))))), terminal(\"]\")) ;
digit = choice(terminal(\"0\"), terminal(\"1\"), terminal(\"2\"), terminal(\"3\")) ;
";
    // The reference's view box at the bundled measurement: the root takes its width from it.
    let svg = render_default(text);
    let view = view_box(&svg);
    assert_eq!(view[0], 0.0);
    assert_eq!(view[1], 0.0);
    assert!(view[2] > 0.0);
    assert!(view[3] > 150.0, "the height is {}", view[3]);
    assert_eq!(attribute_of_root(&svg, "width").as_deref(), Some("100%"));
    let style = attribute_of_root(&svg, "style").expect("a max-width style");
    assert_eq!(
        style,
        format!("max-width: {}px;", ferrigura_render::svg::number(view[2]))
    );
}

#[test]
fn aligns_rule_markers_with_the_centerline_of_tall_expressions() {
    let drawing = draw_stub("railroad-beta\nsign = choice(terminal(\"+\"), terminal(\"-\")) ;\n");
    let rule = &drawing.rules[0];
    let parts = descendants(rule);
    let start = classed(&parts, "railroad-start");
    let end = classed(&parts, "railroad-end");
    let name = classed(&parts, "railroad-rule-name");
    let choice = classed(&parts, "railroad-choice");
    assert_eq!(start.len(), 1);
    assert_eq!(start[0].children()[0].get("cy"), Some("40"));
    assert_eq!(end[0].children()[0].get("cy"), Some("40"));
    assert_eq!(name[0].get("y"), Some("40"));
    let entry = children_classed(choice[0], "railroad-line");
    assert!(
        path_data(entry[0]).contains("M 0 40"),
        "{}",
        path_data(entry[0])
    );
}

#[test]
fn connects_centered_choice_alternatives_from_the_left_edge() {
    let drawing = draw_stub(
        "railroad-beta\nterm = choice(nonterminal(\"number\"), sequence(terminal(\"(\"), nonterminal(\"expression\"), terminal(\")\"))) ;\n",
    );
    let parts = descendants(&drawing.rules[0]);
    let choice = classed(&parts, "railroad-choice")[0];
    let number = classed(&descendants(choice), "railroad-nonterminal")[0];
    assert_eq!(number.get("transform"), Some("translate(74, 0)"));
    let entry = children_classed(choice, "railroad-line");
    assert!(
        path_data(entry[0]).contains("L 74 18"),
        "{}",
        path_data(entry[0])
    );
}

#[test]
fn connects_centered_choice_alternatives_to_the_right_lane_before_merging() {
    let drawing = draw_stub(
        "railroad-beta\nidentifierPart = choice(nonterminal(\"letter\"), terminal(\"_\")) ;\n",
    );
    let parts = descendants(&drawing.rules[0]);
    let choice = classed(&parts, "railroad-choice")[0];
    let terminal = classed(&descendants(choice), "railroad-terminal")[0];
    assert_eq!(terminal.get("transform"), Some("translate(40, 44)"));
    let lines = children_classed(choice, "railroad-line");
    let exit = lines
        .iter()
        .map(|line| path_data(line))
        .find(|d| d.starts_with("M 68 62"))
        .expect("the exit of the lower alternative");
    assert!(exit.contains("L 88 62"), "{exit}");
    assert!(exit.contains("L 98 50"), "{exit}");
}

#[test]
fn orients_choice_corner_arcs_in_the_direction_of_travel() {
    let drawing = draw_stub("railroad-beta\nsign = choice(terminal(\"+\"), terminal(\"-\")) ;\n");
    let parts = descendants(&drawing.rules[0]);
    let choice = classed(&parts, "railroad-choice")[0];
    let paths: Vec<&str> = children_classed(choice, "railroad-line")
        .into_iter()
        .map(path_data)
        .collect();
    assert_eq!(paths.len(), 4);
    assert!(paths[0].contains("A 10 10 0 0 0 10 30"), "{}", paths[0]);
    assert!(paths[0].contains("A 10 10 0 0 1 20 18"), "{}", paths[0]);
    assert!(paths[1].contains("A 10 10 0 0 1 58 28"), "{}", paths[1]);
    assert!(paths[1].contains("A 10 10 0 0 0 68 40"), "{}", paths[1]);
    assert!(paths[2].contains("A 10 10 0 0 1 10 50"), "{}", paths[2]);
    assert!(paths[2].contains("A 10 10 0 0 0 20 62"), "{}", paths[2]);
    assert!(paths[3].contains("A 10 10 0 0 0 58 52"), "{}", paths[3]);
    assert!(paths[3].contains("A 10 10 0 0 1 68 40"), "{}", paths[3]);
}

#[test]
fn orients_repetition_loop_corners_in_the_direction_of_travel() {
    let drawing = draw_stub("railroad-beta\nnumber = oneOrMore(nonterminal(\"digit\")) ;\n");
    let parts = descendants(&drawing.rules[0]);
    let repetition = classed(&parts, "railroad-repetition")[0];
    let loop_path = children_classed(repetition, "railroad-line")
        .into_iter()
        .map(path_data)
        .find(|d| d.contains("L 90 46"))
        .expect("the loop");
    assert_eq!(
        loop_path,
        "M 80 18 A 10 10 0 0 1 90 28 L 90 46 A 10 10 0 0 1 80 56 L 20 56 A 10 10 0 0 1 10 46 L 10 28 A 10 10 0 0 1 20 18"
    );
}

// --- styles.spec.ts -----------------------------------------------------------------------------

/// The stylesheet of a rendered SVG: the `<style>` element's text.
fn style_sheet(svg: &str) -> &str {
    let start = svg.find("<style>").expect("a style element") + "<style>".len();
    let end = svg[start..].find("</style>").expect("the end of the style") + start;
    &svg[start..end]
}

#[test]
fn should_generate_default_styles_when_no_options_provided() {
    let svg = render_default("railroad-beta\ndigit = terminal(\"0\") ;\n");
    let sheet = style_sheet(&svg);
    assert!(sheet.contains(".railroad-diagram{font-family:"), "{sheet}");
    assert!(sheet.contains("font-size:14px;"), "{sheet}");
    assert!(sheet.contains(".railroad-terminal"));
    assert!(sheet.contains(".railroad-nonterminal"));
    assert!(sheet.contains(".railroad-line"));
}

#[test]
fn should_derive_railroad_colors_from_the_provided_theme_variables() {
    let dark = tokens(Theme::ReduxDarkColor);
    let svg = render_with(
        "railroad-beta\ndigit = terminal(\"0\") ;\n",
        &RailroadDiagramConfig::default(),
        &dark,
        None,
        "railroad",
    );
    let fill = dark.second_bkg.text().expect("the theme's terminal fill");
    assert!(style_sheet(&svg).contains(&format!("--railroadTerminalFill:{fill};")));
    assert!(!svg.contains("#FFFFC0"));
}

#[test]
fn should_use_custom_font_family() {
    // Every text is drawn in the bundled face, so the sheet names the embedded family and not the
    // configured one (decision 0005).
    let config = RailroadDiagramConfig {
        font_family: Some("Arial".to_owned()),
        ..RailroadDiagramConfig::default()
    };
    let svg = render_with(
        "railroad-beta\ndigit = terminal(\"0\") ;\n",
        &config,
        &tokens(Theme::ReduxColor),
        None,
        "railroad",
    );
    let sheet = style_sheet(&svg);
    assert!(
        sheet.contains(".railroad-diagram{font-family:\"fg-"),
        "{sheet}"
    );
    assert!(!sheet.contains("Arial"));
}

#[test]
fn should_use_custom_font_size() {
    let config = RailroadDiagramConfig {
        font_size: Some(18.0),
        ..RailroadDiagramConfig::default()
    };
    let svg = render_with(
        "railroad-beta\ndigit = terminal(\"0\") ;\n",
        &config,
        &tokens(Theme::ReduxColor),
        None,
        "railroad",
    );
    assert!(style_sheet(&svg).contains("font-size:18px;"));
}

#[test]
fn should_use_custom_terminal_colors() {
    let config = RailroadDiagramConfig {
        terminal_fill: Some("#FF0000".to_owned()),
        terminal_stroke: Some("#00FF00".to_owned()),
        terminal_text_color: Some("#0000FF".to_owned()),
        ..RailroadDiagramConfig::default()
    };
    let svg = render_with(
        "railroad-beta\ndigit = terminal(\"0\") ;\n",
        &config,
        &tokens(Theme::ReduxColor),
        None,
        "railroad",
    );
    let sheet = style_sheet(&svg);
    assert!(sheet.contains("--railroadTerminalFill:#FF0000;"));
    assert!(sheet.contains("--railroadTerminalStroke:#00FF00;"));
    assert!(sheet.contains("--railroadTerminalTextColor:#0000FF;"));
    assert!(svg.contains("fill=\"#FF0000\""));
}

#[test]
fn should_use_custom_non_terminal_colors() {
    let config = RailroadDiagramConfig {
        non_terminal_fill: Some("#AAAAAA".to_owned()),
        non_terminal_stroke: Some("#BBBBBB".to_owned()),
        non_terminal_text_color: Some("#CCCCCC".to_owned()),
        ..RailroadDiagramConfig::default()
    };
    let svg = render_with(
        "railroad-beta\nr = nonterminal(\"d\") ;\n",
        &config,
        &tokens(Theme::ReduxColor),
        None,
        "railroad",
    );
    let sheet = style_sheet(&svg);
    assert!(sheet.contains("--railroadNonTerminalFill:#AAAAAA;"));
    assert!(sheet.contains("--railroadNonTerminalStroke:#BBBBBB;"));
    assert!(sheet.contains("--railroadNonTerminalTextColor:#CCCCCC;"));
}

#[test]
fn should_use_custom_line_color_and_stroke_width() {
    let config = RailroadDiagramConfig {
        line_color: Some("#FF00FF".to_owned()),
        stroke_width: 4.0,
        ..RailroadDiagramConfig::default()
    };
    let svg = render_with(
        "railroad-beta\ndigit = terminal(\"0\") ;\n",
        &config,
        &tokens(Theme::ReduxColor),
        None,
        "railroad",
    );
    let sheet = style_sheet(&svg);
    assert!(sheet.contains("--railroadLineColor:#FF00FF;"));
    assert!(sheet.contains("stroke-width:4px;"));
}

#[test]
fn should_use_custom_marker_fill() {
    let config = RailroadDiagramConfig {
        marker_fill: Some("#123456".to_owned()),
        ..RailroadDiagramConfig::default()
    };
    let svg = render_with(
        "railroad-beta\ndigit = terminal(\"0\") ;\n",
        &config,
        &tokens(Theme::ReduxColor),
        None,
        "railroad",
    );
    assert!(style_sheet(&svg).contains("--railroadMarkerFill:#123456;"));
    assert!(svg.contains("<circle cx=\"") && svg.contains("fill=\"#123456\""));
}

#[test]
fn should_use_custom_comment_colors() {
    let config = RailroadDiagramConfig {
        comment_fill: Some("#F1F1F1".to_owned()),
        comment_stroke: Some("#777777".to_owned()),
        comment_text_color: Some("#555555".to_owned()),
        ..RailroadDiagramConfig::default()
    };
    let svg = render_with(
        "railroad-beta\ndigit = terminal(\"0\") ;\n",
        &config,
        &tokens(Theme::ReduxColor),
        None,
        "railroad",
    );
    let sheet = style_sheet(&svg);
    assert!(sheet.contains("--railroadCommentFill:#F1F1F1;"));
    assert!(sheet.contains("--railroadCommentStroke:#777777;"));
    assert!(sheet.contains("--railroadCommentTextColor:#555555;"));
}

#[test]
fn should_use_custom_special_sequence_colors() {
    let config = RailroadDiagramConfig {
        special_fill: Some("#E0E0FF".to_owned()),
        special_stroke: Some("#9900DD".to_owned()),
        ..RailroadDiagramConfig::default()
    };
    let svg = render_with(
        "railroad-beta\nr = special(\"x\") ;\n",
        &config,
        &tokens(Theme::ReduxColor),
        None,
        "railroad",
    );
    let sheet = style_sheet(&svg);
    assert!(sheet.contains("--railroadSpecialFill:#E0E0FF;"));
    assert!(sheet.contains("--railroadSpecialStroke:#9900DD;"));
}

#[test]
fn should_use_custom_rule_name_color() {
    let config = RailroadDiagramConfig {
        rule_name_color: Some("#990000".to_owned()),
        ..RailroadDiagramConfig::default()
    };
    let svg = render_with(
        "railroad-beta\ndigit = terminal(\"0\") ;\n",
        &config,
        &tokens(Theme::ReduxColor),
        None,
        "railroad",
    );
    assert!(style_sheet(&svg).contains("--railroadRuleNameColor:#990000;"));
}

#[test]
fn should_include_all_css_classes() {
    let sheet = style_sheet(&render_default(
        "railroad-beta\ndigit = terminal(\"0\") ;\n",
    ))
    .to_owned();
    for selector in [
        ".railroad-terminal rect",
        ".railroad-terminal text",
        ".railroad-nonterminal rect",
        ".railroad-nonterminal text",
        ".railroad-line",
        ".railroad-start circle",
        ".railroad-end circle",
        ".railroad-comment ellipse",
        ".railroad-comment text",
        ".railroad-special rect",
        ".railroad-special text",
        ".railroad-rule-name",
        ".railroad-group",
    ] {
        assert!(sheet.contains(selector), "{selector} is missing");
    }
}

#[test]
fn should_include_special_sequence_stroke_dasharray() {
    assert!(
        style_sheet(&render_default("railroad-beta\nr = special(\"x\") ;\n"))
            .contains("stroke-dasharray:5,3;")
    );
}

#[test]
fn should_include_comment_text_font_style_italic() {
    assert!(
        style_sheet(&render_default("railroad-beta\nr = terminal(\"x\") ;\n"))
            .contains("font-style:italic;")
    );
}

#[test]
fn should_include_rule_name_font_weight_bold() {
    assert!(
        style_sheet(&render_default("railroad-beta\nr = terminal(\"x\") ;\n"))
            .contains("font-weight:bold;")
    );
}

#[test]
fn should_include_text_anchor_and_dominant_baseline() {
    let sheet = style_sheet(&render_default("railroad-beta\nr = terminal(\"x\") ;\n")).to_owned();
    assert!(sheet.contains("text-anchor:middle;dominant-baseline:middle;"));
}

#[test]
fn should_fall_back_to_safe_defaults_when_css_values_are_invalid() {
    let config = RailroadDiagramConfig {
        font_family: Some("safe\"} .railroad-terminal { display: none; } /*".to_owned()),
        terminal_fill: Some("#fff; stroke: red;".to_owned()),
        ..RailroadDiagramConfig::default()
    };
    let light = tokens(Theme::ReduxColor);
    let svg = render_with(
        "railroad-beta\ndigit = terminal(\"0\") ;\n",
        &config,
        &light,
        None,
        "railroad",
    );
    assert!(!svg.contains("display: none"));
    assert!(!svg.contains("stroke: red;"));
    // `redux-color` sets no `secondBkg`, so the default theme's shows (`styles.spec.ts` reads it from
    // `themes.default`).
    assert!(light.second_bkg.is_absent());
    assert!(style_sheet(&svg).contains("--railroadTerminalFill:#ffffde;"));
}

#[test]
fn should_handle_all_options_at_once() {
    let config = RailroadDiagramConfig {
        font_size: Some(16.0),
        terminal_fill: Some("#111111".to_owned()),
        terminal_stroke: Some("#222222".to_owned()),
        terminal_text_color: Some("#333333".to_owned()),
        non_terminal_fill: Some("#444444".to_owned()),
        non_terminal_stroke: Some("#555555".to_owned()),
        non_terminal_text_color: Some("#666666".to_owned()),
        line_color: Some("#777777".to_owned()),
        stroke_width: 3.0,
        marker_fill: Some("#888888".to_owned()),
        comment_fill: Some("#999999".to_owned()),
        comment_stroke: Some("#AAAAAA".to_owned()),
        comment_text_color: Some("#BBBBBB".to_owned()),
        special_fill: Some("#CCCCCC".to_owned()),
        special_stroke: Some("#DDDDDD".to_owned()),
        rule_name_color: Some("#EEEEEE".to_owned()),
        ..RailroadDiagramConfig::default()
    };
    let svg = render_with(
        "railroad-beta\nr = terminal(\"0\") ;\n",
        &config,
        &tokens(Theme::ReduxColor),
        None,
        "railroad",
    );
    let sheet = style_sheet(&svg);
    assert!(sheet.contains("font-size:16px;"));
    assert!(sheet.contains("--railroadTerminalFill:#111111;"));
    assert!(sheet.contains("--railroadNonTerminalTextColor:#666666;"));
    assert!(sheet.contains("--railroadLineColor:#777777;"));
    assert!(sheet.contains("stroke-width:3px;"));
    assert!(sheet.contains("--railroadMarkerFill:#888888;"));
    assert!(sheet.contains("--railroadCommentTextColor:#BBBBBB;"));
    assert!(sheet.contains("--railroadSpecialStroke:#DDDDDD;"));
    assert!(sheet.contains("--railroadRuleNameColor:#EEEEEE;"));
}

#[test]
fn should_match_the_renderer_fallbacks_and_leave_colours_and_typography_to_the_theme() {
    // `defaultConfig.railroad` holds the nine numbers and booleans and leaves the colours and the
    // typography unset, so that the theme supplies them (`styles.spec.ts`, 'default config').
    let config = RailroadDiagramConfig::default();
    assert!(config.use_max_width);
    assert!(!config.compact_mode);
    assert_eq!(config.padding, 10.0);
    assert_eq!(config.vertical_separation, 8.0);
    assert_eq!(config.horizontal_separation, 10.0);
    assert_eq!(config.arc_radius, 10.0);
    assert_eq!(config.stroke_width, 2.0);
    assert!(config.show_markers);
    assert_eq!(config.marker_radius, 5.0);
    assert_eq!(config.font_size, None);
    assert_eq!(config.font_family, None);
    assert_eq!(config.terminal_fill, None);
    assert_eq!(config.rule_name_color, None);
    // The theme's variables give the colours; the variables it does not set give the default theme's.
    let light = tokens_of(serde_json::json!({ "secondBkg": "#abcdef", "fontSize": "14px" }));
    let svg = render_with(
        "railroad-beta\nr = terminal(\"0\") ;\n",
        &config,
        &light,
        None,
        "railroad",
    );
    assert!(style_sheet(&svg).contains("--railroadTerminalFill:#abcdef;"));
}

// The root element is the reference's: the class, the size and the detector id, read from the SVG.
#[test]
fn the_root_names_the_detector_of_the_syntax() {
    let light = tokens(Theme::ReduxColor);
    let config = RailroadDiagramConfig::default();
    for (text, detector) in [
        ("railroad-beta\nr = terminal(\"a\") ;\n", "railroad"),
        ("railroad-ebnf-beta\nr = \"a\" ;\n", "railroadEbnf"),
        ("railroad-abnf-beta\nr = \"a\" ;\n", "railroadAbnf"),
        ("railroad-peg-beta\nR <- \"a\" ;\n", "railroadPeg"),
    ] {
        let svg = render_with(text, &config, &light, None, detector);
        assert!(
            svg.contains(&format!("aria-roledescription=\"{detector}\"")),
            "{detector}"
        );
        assert!(svg.contains("class=\"railroad-diagram\""));
    }
}

/// The real measurement gives the probed box of a terminal: `0` is 9.078125 wide at 14 px, so the box
/// is that plus the padding on both sides, and the text height plus the padding (`NOTES.md` 6.4).
#[test]
fn the_real_measurement_gives_the_probed_terminal_box() {
    let drawing = draw_real("railroad-beta\ndigit = terminal(\"0\") ;\n");
    let parts = descendants(&drawing.rules[0]);
    let terminal = classed(&parts, "railroad-terminal")[0];
    let rect = terminal
        .children()
        .iter()
        .find(|child| child.get("width").is_some())
        .expect("the box");
    assert_eq!(rect.get("width"), Some("29.078125"));
    assert_eq!(rect.get("height"), Some("37"));
}
