//! The timeline renderer against the numbers and markup of the reference (`docs-port/timeline/NOTES.md`
//! 5). The numbers are the ones Chromium gives for the fixtures of `e2e/diagrams/timeline`, read with
//! `pnpm port:geometry timeline`; the markup is the reference's, quirks included.

use ferrigura_model::config::{defaults, typed};
use ferrigura_model::theme::Theme;
use ferrigura_render::theme::Themes;
use ferrigura_render::timeline::{TimelineInput, render};

/// The first fixture of `e2e/diagrams/timeline/pr-check`: a horizontal timeline without sections.
const SIMPLE: &str = "timeline
  title History of Social Media Platform
  2002 : LinkedIn
  2004 : Facebook : Google
  2005 : YouTube
  2006 : Twitter
";

/// Renders a timeline with the redux colour theme and the default configuration.
fn draw(text: &str, id: &str) -> String {
    let diagram = ferrigura_syntax::timeline::parse(text).expect("the timeline parses");
    let config = typed(&defaults()).expect("the default configuration is typed");
    let variables = Theme::ReduxColor.variables(None);
    render(&TimelineInput {
        diagram: &diagram,
        config: &config,
        themes: Themes::Single(&variables),
        id,
    })
    .expect("the timeline renders")
}

/// The attribute `name` of the first element that starts with `<tag `, as written.
fn attribute<'a>(svg: &'a str, tag: &str, name: &str) -> Option<&'a str> {
    let start = svg.find(&format!("<{tag} "))?;
    let element = &svg[start..svg[start..].find('>')? + start];
    let needle = format!(" {name}=\"");
    let value_start = element.find(&needle)? + needle.len();
    let value_end = element[value_start..].find('"')? + value_start;
    Some(&element[value_start..value_end])
}

#[test]
fn the_simple_timeline_has_the_box_chromium_gives_it() {
    // `pnpm port:geometry timeline`, fixture `pr-check/1`: the reference's viewBox is
    // `100 -56 1190 523.4000244140625`, and its root is 1190 px wide at most.
    let svg = draw(SIMPLE, "t1");
    let view_box: Vec<f64> = attribute(&svg, "svg", "viewBox")
        .expect("a viewBox")
        .split_whitespace()
        .map(|number| number.parse().expect("a number"))
        .collect();
    for (actual, expected) in view_box.iter().zip([100.0, -56.0, 1190.0, 523.4]) {
        assert!((actual - expected).abs() < 0.05, "{view_box:?}");
    }
    assert_eq!(attribute(&svg, "svg", "width"), Some("100%"));
    assert!(svg.contains("style=\"max-width: 1190px;\""), "{svg}");
}

#[test]
fn the_title_is_centred_over_the_content_and_set_in_four_ex() {
    // The reference's title is at x 245 (the centre of the box of 790 less the 150 of the margin), at
    // y 20, in 4ex and bold.
    let svg = draw(SIMPLE, "t1");
    assert!(
        svg.contains("<text x=\"245\" font-size=\"4ex\" font-weight=\"bold\" y=\"20\">History of Social Media Platform</text>"),
        "{svg}"
    );
}

#[test]
fn a_horizontal_timeline_names_its_nodes_and_arrows_by_the_diagram() {
    let svg = draw(SIMPLE, "t1");
    assert!(svg.contains("id=\"t1-node-0\""), "{svg}");
    assert!(svg.contains("marker-end=\"url(#t1-arrowhead)\""), "{svg}");
    assert!(svg.contains("<marker id=\"t1-arrowhead\""), "{svg}");
}

#[test]
fn a_vertical_timeline_keeps_the_reference_marker_and_node_names() {
    // `timelineRendererVertical.ts` calls `initGraphics` without the id and draws its arrows with
    // `url(#arrowhead)`, so no arrow is drawn, and its nodes are named `undefined-node-N` (NOTES 5.12,
    // R1 and R2).
    let svg = draw(
        "timeline TD\n  section A\n  2000 : one : two\n  2001 : three\n",
        "t2",
    );
    assert!(svg.contains("<marker id=\"undefined-arrowhead\""), "{svg}");
    assert!(svg.contains("marker-end=\"url(#arrowhead)\""), "{svg}");
    assert!(!svg.contains("t2-arrowhead"), "{svg}");
    assert!(svg.contains("id=\"undefined-node-0\""), "{svg}");
}

#[test]
fn an_event_is_wrapped_where_the_browser_wraps_it_and_a_break_tag_starts_a_line() {
    // The fixture `pr-check/14`: "Machinery, Water power, Steam <br>power" in a 300 px column. The
    // separators are joined with blanks, so the line has three blanks where the source has one.
    let svg = draw(
        "timeline TD\n  section A\n  2000 : Machinery, Water power, Steam <br>power\n",
        "t3",
    );
    assert!(
        svg.contains(
            "<tspan x=\"0\" dy=\"1em\">Machinery,   Water   power,   Steam</tspan><tspan x=\"0\" dy=\"1.1em\">power</tspan>"
        ),
        "{svg}"
    );
}

#[test]
fn the_task_pre_pass_measures_the_object_and_not_the_task() {
    // `timelineRenderer.ts` 103-114 gives the task object as the text, so the pre-pass measures
    // `[object Object]` as one line: 17 + 8.8 + 20 = 45.8, and the largest task height is 65.8. A task
    // drawn in four lines is 92 high, its dashed line starts at its foot (50 + 92), and the axis is
    // still at the pre-pass height (65.8 + 100). Checked against the reference by the comparison of
    // the same input.
    let svg = draw(
        "timeline\n  Quality Management System Audit Process Review Long Heading Words Here Now : x\n  2000 : y\n",
        "t4",
    );
    assert!(svg.contains("d=\"M0 87 v-87 h190 v92 H0 Z\""), "{svg}");
    assert!(
        svg.contains("<line x1=\"295\" y1=\"142\" x2=\"295\" y2=\"387.8\""),
        "{svg}"
    );
    assert!(svg.contains("<line x1=\"150\" y1=\"165.8\" x2=\""), "{svg}");
}

#[test]
fn accessibility_is_not_drawn() {
    // The reference draws neither `accTitle` nor `accDescr`: the timeline database has no getter for
    // them, and the renderer's call is commented out (NOTES 5.12, R7).
    let svg = draw(
        "timeline\n  accTitle: Accessible\n  accDescr: Described\n  2000 : x\n",
        "t5",
    );
    assert!(!svg.contains("<title"), "{svg}");
    assert!(!svg.contains("<desc"), "{svg}");
    assert!(!svg.contains("aria-labelledby"), "{svg}");
    assert!(!svg.contains("aria-describedby"), "{svg}");
    assert!(svg.contains("aria-roledescription=\"timeline\""), "{svg}");
}

#[test]
fn the_rules_of_the_timeline_are_scoped_to_the_diagram_and_its_colours_are_properties() {
    // Decision 0007: every selector starts with the id, and the colours are custom properties on it.
    let svg = draw(SIMPLE, "t1");
    let style_start = svg.find("<style>").expect("a style element") + "<style>".len();
    let style =
        &svg[style_start..svg[style_start..].find("</style>").expect("the end") + style_start];
    for selector in [".section-", ".lineWrapper", ".eventWrapper"] {
        for (index, _) in style.match_indices(selector) {
            assert!(
                style[..index].ends_with("#t1 "),
                "{selector} at {index} is not scoped: {style}"
            );
        }
    }
    assert!(style.contains("#t1{--mainBkg:"), "{style}");
    assert!(style.contains("--sectionFill0:"), "{style}");
    assert!(style.contains("fill:var(--sectionFill1)"), "{style}");
}
