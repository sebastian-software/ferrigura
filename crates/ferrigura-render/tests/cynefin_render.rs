//! The cynefin renderer's output for the reference's quirks (`docs-port/cynefin/NOTES.md`, section 6,
//! and `docs-port/cynefin/PLAN.md`): the order of the root and its groups, the sizes, the items and their
//! badges (the fallback width, the measurement of a placeholder), the confusion cap, the layout without
//! descriptions, the arrows, the title, the colours and the theme's overrides. The geometry against the
//! reference is compared by `pnpm port:geometry cynefin`; these tests pin what that comparison relies on.

use ferrigura_model::config::{defaults, typed};
use ferrigura_model::cynefin::{CynefinDiagram, CynefinDomain, CynefinTransition};
use ferrigura_model::theme::{Theme, ThemeTokens};
use ferrigura_model::value::Value;
use ferrigura_render::common::d3::js_number;
use ferrigura_render::common::text::collapsed_box;
use ferrigura_render::context::{Context, Globals, ThemeView};
use ferrigura_render::cynefin::render;
use ferrigura_syntax::cynefin::parse;
use ferrigura_text::{FontFamily, FontStyle, FontWeight, face};

/// The configuration of a render: the `cynefin` settings, the theme's overrides, and a second scheme.
#[derive(Default)]
struct Setup {
    /// The `cynefin` section's keys, as the configuration sets them.
    cynefin: Vec<(&'static str, Value)>,
    /// The `themeVariables` overrides, as the theme takes them.
    overrides: Option<Value>,
    /// The overrides of the dark scheme, when the render draws both.
    dark_overrides: Option<Option<Value>>,
}

/// Renders a diagram with the setup and the id `chart`.
fn draw(diagram: &CynefinDiagram, setup: &Setup) -> String {
    draw_id(diagram, setup, "chart")
}

/// Renders a diagram with the setup and the given id.
fn draw_id(diagram: &CynefinDiagram, setup: &Setup, id: &str) -> String {
    let mut value = defaults();
    for (key, setting) in &setup.cynefin {
        value
            .get_mut("cynefin")
            .and_then(Value::as_object_mut)
            .expect("the cynefin section")
            .insert((*key).to_owned(), setting.clone());
    }
    let config = typed(&value).expect("the configuration");
    let light = ThemeTokens::from_value(&Theme::ReduxColor.variables(setup.overrides.as_ref()));
    let globals = Globals::of(&config);
    let dark = setup.dark_overrides.as_ref().map(|overrides| {
        ThemeTokens::from_value(&Theme::ReduxDarkColor.variables(overrides.as_ref()))
    });
    render(&Context {
        diagram,
        section: &config.cynefin,
        globals: &globals,
        theme: ThemeView {
            light: &light,
            dark: dark.as_ref(),
        },
        id,
    })
    .expect("the diagram renders")
}

/// Parses and renders a text with the default setup.
fn draw_text(text: &str) -> String {
    let diagram = parse(text).expect("the diagram parses");
    draw(&diagram, &Setup::default())
}

/// The opening tags of the elements with the given name, in document order.
fn tags<'a>(svg: &'a str, name: &str) -> Vec<&'a str> {
    let open = format!("<{name}");
    svg.match_indices(&open)
        .filter(|(start, _)| {
            svg[start + open.len()..]
                .chars()
                .next()
                .is_some_and(|c| c == ' ' || c == '>' || c == '/')
        })
        .filter_map(|(start, _)| {
            let rest = &svg[start..];
            Some(&rest[..=rest.find('>')?])
        })
        .collect()
}

/// The first attribute value of a name in an opening tag.
fn attribute_of<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!(" {name}=\"");
    let start = tag.find(&key)? + key.len();
    let end = tag[start..].find('"')? + start;
    Some(&tag[start..end])
}

/// The position of an element's opening tag in the document.
fn position(svg: &str, needle: &str) -> usize {
    svg.find(needle)
        .unwrap_or_else(|| panic!("{needle} is in the document"))
}

/// The diagram of the accessibility directives, with a title and one item in each quadrant.
const ACCESSIBLE: &str = "cynefin-beta
  accTitle: Cynefin framework for software delivery
  accDescr: A Cynefin map
  title Incident response
  complex
  \"Probe\"
  clear
  \"Standard\"
";

/// The root of the document is written as the reference writes it (`cynefinRenderer.ts` 104-112 and
/// `mermaidAPI.ts` 419): mermaid's accessible texts, the style, the empty group, the diagram's own
/// accessible texts, the group of the diagram, then the defs of the arrows.
#[test]
fn the_root_and_its_children_are_in_the_references_order() {
    let svg = draw_text(
        "cynefin-beta
  accTitle: Cynefin framework for software delivery
  accDescr: A Cynefin map
  title Incident response
  complex
  \"Probe\"
  clear
  \"Standard\"
  complex --> clear
",
    );
    let mermaid_title = position(&svg, "<title id=\"chart-title-chart\">");
    let mermaid_desc = position(&svg, "<desc id=\"chart-desc-chart\">");
    let style = position(&svg, "<style>");
    let empty_group = position(&svg, "<g/>");
    let own_title = position(&svg, "<title>Cynefin");
    let own_desc = position(&svg, "<desc>A Cynefin");
    let diagram_group = position(&svg, "<g transform=\"translate(40, 40)\">");
    let defs = position(&svg, "<defs>");
    assert!(mermaid_title < mermaid_desc && mermaid_desc < style);
    assert!(style < empty_group && empty_group < own_title && own_title < own_desc);
    assert!(own_desc < diagram_group && diagram_group < defs);
    assert!(svg.ends_with("</defs></svg>"), "{svg}");
}

/// The root attributes: the role, the description and the title references, and the size rule of
/// `setupGraphViewbox`: a percentage width with a maximum, or the height and the width.
#[test]
fn the_root_carries_the_size_rule_and_the_accessible_references() {
    let diagram = parse(ACCESSIBLE).expect("the diagram parses");
    let svg = draw(&diagram, &Setup::default());
    let root = tags(&svg, "svg")[0];
    assert_eq!(attribute_of(root, "width"), Some("100%"), "{root}");
    assert_eq!(
        attribute_of(root, "style"),
        Some("max-width: 880px;"),
        "{root}"
    );
    assert_eq!(attribute_of(root, "viewBox"), Some("0 0 880 680"), "{root}");
    assert_eq!(attribute_of(root, "aria-roledescription"), Some("cynefin"));
    assert_eq!(
        attribute_of(root, "aria-labelledby"),
        Some("chart-title-chart")
    );
    assert_eq!(
        attribute_of(root, "aria-describedby"),
        Some("chart-desc-chart")
    );

    let setup = Setup {
        cynefin: vec![("useMaxWidth", Value::Bool(false))],
        ..Setup::default()
    };
    let fixed = tags(&draw(&diagram, &setup), "svg")[0].to_owned();
    assert_eq!(attribute_of(&fixed, "width"), Some("880"), "{fixed}");
    assert_eq!(attribute_of(&fixed, "height"), Some("680"), "{fixed}");
    assert!(!fixed.contains("max-width"), "{fixed}");
}

/// The items group is written even when no domain has an item (`cynefinRenderer.ts` 229, unconditional).
#[test]
fn the_items_group_is_written_even_without_items() {
    let svg = draw_text("cynefin-beta\n  complex\n  clear\n");
    assert!(svg.contains("class=\"cynefin-items\""), "{svg}");
    assert!(!svg.contains("class=\"cynefinItemText\""), "{svg}");
}

/// The title sits at `-padding / 2` (`cynefinRenderer.ts` 429). With no padding that is `-0`, which
/// the reference writes as `0`.
#[test]
fn a_zero_padding_writes_the_title_at_zero_not_minus_zero() {
    let diagram = parse("cynefin-beta\n  title Zero\n  complex\n  \"A\"\n").expect("parses");
    let setup = Setup {
        cynefin: vec![("padding", Value::Number(0.0))],
        ..Setup::default()
    };
    let svg = draw(&diagram, &setup);
    let title = tags(&svg, "text")
        .into_iter()
        .find(|tag| tag.contains("cynefinTitle"))
        .expect("the title");
    assert_eq!(attribute_of(title, "y"), Some("0"), "{title}");
    assert!(!title.contains("-0"), "{title}");
}

/// The width of a badge is the measured text plus 20, and an empty label measures 0, which falls back to
/// its length times 7 (`cynefinRenderer.ts` 277-286): an empty label gives a badge of 20.
#[test]
fn an_empty_label_gets_the_badge_of_twenty() {
    let svg = draw_text("cynefin-beta\n  complex\n  \"\"\n");
    let rect = tags(&svg, "rect")
        .into_iter()
        .find(|tag| tag.contains("cynefinItem"))
        .expect("the badge");
    assert_eq!(attribute_of(rect, "width"), Some("20"), "{rect}");
}

/// The badge is measured on the label as the DB holds it, with its entity placeholder, and the text is
/// written decoded: `Probe #amp; sense` is measured as `Probe ﬂ°amp¶ß sense` and shows `Probe & sense`
/// (`mermaidAPI.ts` 359 decodes the whole SVG after the draw).
#[test]
fn a_badge_is_measured_on_the_placeholder_and_shows_the_decoded_text() {
    let svg = draw_text("cynefin-beta\n  complex\n  \"Probe #amp; sense\"\n");
    assert!(svg.contains(">Probe &amp; sense</text>"), "{svg}");
    let measured = collapsed_box(
        face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal),
        "Probe ﬂ°amp¶ß sense",
        12.0,
    )
    .expect("measured")
    .width();
    let rect = tags(&svg, "rect")
        .into_iter()
        .find(|tag| tag.contains("cynefinItem"))
        .expect("the badge");
    assert_eq!(
        attribute_of(rect, "width"),
        Some(js_number(measured + 20.0).as_str()),
        "{rect}"
    );
}

/// The confusion ellipse shows three items and a `+N more` badge below them (`cynefinRenderer.ts`
/// 247-250 and 307-332): with five items, the badge is the fourth row, at the start plus three rows.
#[test]
fn the_confusion_ellipse_shows_three_items_and_a_count_of_the_rest() {
    let svg = draw_text(
        "cynefin-beta\n  confusion\n  \"One\"\n  \"Two\"\n  \"Three\"\n  \"Four\"\n  \"Five\"\n",
    );
    assert!(svg.contains(">+2 more</text>"), "{svg}");
    assert!(!svg.contains(">Four</text>"), "{svg}");
    assert!(
        svg.contains("class=\"cynefinItemOverflow cynefin-fill-confusion\""),
        "{svg}"
    );
    // The start is the centre plus 22 with descriptions; the overflow badge follows three rows of 30.
    assert!(
        svg.contains("translate(") && svg.contains(", 412)"),
        "{svg}"
    );
}

/// Four confusion items give a `+1 more` badge at the same place (the cap is three).
#[test]
fn four_confusion_items_give_one_more() {
    let svg =
        draw_text("cynefin-beta\n  confusion\n  \"One\"\n  \"Two\"\n  \"Three\"\n  \"Four\"\n");
    assert!(svg.contains(">+1 more</text>"), "{svg}");
}

/// With `showDomainDescriptions` false the subtitles are not written, and the labels and the items sit
/// at the centres (`cynefinRenderer.ts` 175, 186, 192, 255 and 258).
#[test]
fn without_descriptions_the_labels_and_the_items_move_to_the_centres() {
    let diagram =
        parse("cynefin-beta\n  complex\n  \"A\"\n  confusion\n  \"B\"\n").expect("parses");
    let setup = Setup {
        cynefin: vec![("showDomainDescriptions", Value::Bool(false))],
        ..Setup::default()
    };
    let svg = draw(&diagram, &setup);
    assert!(!svg.contains("cynefin-subtitles"), "{svg}");
    assert!(
        svg.contains("x=\"200\" y=\"150\" text-anchor=\"middle\""),
        "{svg}"
    );
    assert!(
        svg.contains("x=\"400\" y=\"300\" text-anchor=\"middle\""),
        "{svg}"
    );
    // The quadrant's first item starts at the centre plus 15, the confusion's at the centre plus 14.
    assert!(
        svg.contains("translate(") && svg.contains(", 165)"),
        "{svg}"
    );
    assert!(svg.contains(", 314)"), "{svg}");
}

/// A transition is a quadratic curve from the centre of its start to the centre of its end, bent by 15%
/// of its length to the left of its direction; its label sits 6 above the curve's control point
/// (`cynefinRenderer.ts` 385-418). From complex to complicated the bend is 60 down.
#[test]
fn an_arrow_is_a_curve_bent_to_the_left_of_its_direction() {
    let svg = draw_text("cynefin-beta\n  complex --> complicated : \"Pattern\"\n");
    assert!(svg.contains("d=\"M200,150 Q400,210 600,150\""), "{svg}");
    assert!(svg.contains("x=\"400\" y=\"204\""), "{svg}");
    assert!(svg.contains(">Pattern</text>"), "{svg}");
    assert!(
        svg.contains("marker-end=\"url(#cynefin-arrow-chart)\""),
        "{svg}"
    );
}

/// The marker is written with the diagram's id, and the defs come after the diagram's group.
#[test]
fn the_arrow_head_is_a_marker_in_defs_after_the_diagram() {
    let svg = draw_text("cynefin-beta\n  complex --> clear\n");
    let marker = tags(&svg, "marker")[0];
    assert_eq!(
        attribute_of(marker, "id"),
        Some("cynefin-arrow-chart"),
        "{marker}"
    );
    assert_eq!(attribute_of(marker, "orient"), Some("auto-start-reverse"));
    assert!(
        svg.contains("<path d=\"M 0 0 L 10 5 L 0 10 z\" class=\"cynefinArrowHead\""),
        "{svg}"
    );
}

/// A diagram without transitions has no defs and no group of arrows (`cynefinRenderer.ts` 352).
#[test]
fn a_diagram_without_transitions_has_no_arrows() {
    let svg = draw_text("cynefin-beta\n  complex\n  \"A\"\n");
    assert!(!svg.contains("<defs>"), "{svg}");
    assert!(!svg.contains("cynefin-arrows"), "{svg}");
}

/// The renderer skips a transition whose start or end is not a domain (`cynefinRenderer.ts` 374), but
/// it still writes the defs and the group, since the transition list is not empty.
#[test]
fn a_transition_to_an_unknown_domain_is_not_drawn_but_the_defs_are() {
    let mut diagram = CynefinDiagram::new();
    diagram.transitions.push(CynefinTransition {
        from: "nowhere".to_owned(),
        to: "clear".to_owned(),
        label: None,
    });
    let svg = draw(&diagram, &Setup::default());
    assert!(svg.contains("<defs>"), "{svg}");
    assert!(svg.contains("class=\"cynefin-arrows\""), "{svg}");
    assert!(!svg.contains("class=\"cynefinArrowLine\""), "{svg}");
}

/// A self-loop that reaches the renderer is skipped (the DB drops them before, `cynefinRenderer.ts` 379).
#[test]
fn a_self_loop_given_to_the_renderer_is_skipped() {
    let mut diagram = CynefinDiagram::new();
    diagram.domains.push(CynefinDomain {
        name: "complex".to_owned(),
        items: Vec::new(),
    });
    diagram.transitions.push(CynefinTransition {
        from: "complex".to_owned(),
        to: "complex".to_owned(),
        label: Some("Loop".to_owned()),
    });
    let svg = draw(&diagram, &Setup::default());
    assert!(svg.contains("class=\"cynefin-arrows\""), "{svg}");
    assert!(!svg.contains("class=\"cynefinArrowLine\""), "{svg}");
    assert!(!svg.contains("Loop"), "{svg}");
}

/// The colours are custom properties on the diagram's id and presentation attributes on its elements
/// (`docs-port/decisions/0007`): the background of the complex domain is the theme's `complexBg`.
#[test]
fn the_domain_colours_are_custom_properties_and_attributes() {
    let svg = draw_text("cynefin-beta\n  complex\n  \"A\"\n");
    assert!(svg.contains("#chart{--cynefinComplexBg:#E8F5E9;"), "{svg}");
    let rect = tags(&svg, "rect")
        .into_iter()
        .find(|tag| tag.contains("cynefin-fill-complex"))
        .expect("the background");
    assert_eq!(attribute_of(rect, "fill"), Some("#E8F5E9"), "{rect}");
    assert!(
        svg.contains("#chart .cynefin-fill-complex{fill:var(--cynefinComplexBg);}"),
        "{svg}"
    );
}

/// A `null` override of a colour removes the presentation attribute, as `d3`'s `attr` does with `null`,
/// and the custom property then holds the text `null`.
#[test]
fn a_null_colour_override_removes_the_fill_attribute() {
    let diagram = parse("cynefin-beta\n  complex\n  \"A\"\n").expect("parses");
    let setup = Setup {
        overrides: Some(Value::from(
            serde_json::json!({"cynefin": {"complexBg": null}}),
        )),
        ..Setup::default()
    };
    let svg = draw(&diagram, &setup);
    let rect = tags(&svg, "rect")
        .into_iter()
        .find(|tag| tag.contains("cynefin-fill-complex") && tag.contains("cynefinDomain"))
        .expect("the background");
    assert_eq!(attribute_of(rect, "fill"), None, "{rect}");
    assert!(svg.contains("--cynefinComplexBg:null;"), "{svg}");
}

/// A string font size is concatenated by `+` (`domainFontSize + 2` in `styles.ts` 88): `"20" + 2` is
/// `"202"`, so the title is 202 pixels, while the labels take `20px` (`styles.ts` 36).
#[test]
fn a_string_font_size_concatenates_in_the_title_size() {
    let diagram = parse("cynefin-beta\n  title T\n  complex\n  \"A\"\n").expect("parses");
    let setup = Setup {
        overrides: Some(Value::from(
            serde_json::json!({"cynefin": {"domainFontSize": "20"}}),
        )),
        ..Setup::default()
    };
    let svg = draw(&diagram, &setup);
    assert!(svg.contains(".cynefinTitle{font-size:202px;"), "{svg}");
    assert!(svg.contains(".cynefinDomainLabel{font-size:20px;"), "{svg}");
}

/// A numeric item font size sets the badges and, one less, the subtitles and the arrow labels.
#[test]
fn a_numeric_item_font_size_sets_the_badges_and_one_less_the_subtitles() {
    let diagram =
        parse("cynefin-beta\n  complex\n  \"A\"\n  complex --> clear : \"Go\"\n").expect("parses");
    let setup = Setup {
        overrides: Some(Value::from(
            serde_json::json!({"cynefin": {"itemFontSize": 14}}),
        )),
        ..Setup::default()
    };
    let svg = draw(&diagram, &setup);
    assert!(svg.contains(".cynefinItemText{font-size:14px;"), "{svg}");
    assert!(svg.contains(".cynefinSubtitle{font-size:13px;"), "{svg}");
    assert!(svg.contains(".cynefinArrowLabel{font-size:13px;"), "{svg}");
}

/// An item font size with a unit (`"14px"`) makes the style sheet's declaration `14pxpx`, which the browser
/// drops: the text keeps its parent's size, the root's 14 px, and the badge is measured at that size.
#[test]
fn an_item_size_with_a_unit_is_measured_at_the_root_size() {
    let diagram = parse("cynefin-beta\n  complex\n  \"Measured\"\n").expect("parses");
    let setup = Setup {
        overrides: Some(Value::from(
            serde_json::json!({"cynefin": {"itemFontSize": "14px"}}),
        )),
        ..Setup::default()
    };
    let svg = draw(&diagram, &setup);
    assert!(svg.contains(".cynefinItemText{font-size:14pxpx;"), "{svg}");
    let measured = collapsed_box(
        face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal),
        "Measured",
        14.0,
    )
    .expect("measured")
    .width();
    let rect = tags(&svg, "rect")
        .into_iter()
        .find(|tag| tag.contains("cynefinItem"))
        .expect("the badge");
    assert_eq!(
        attribute_of(rect, "width"),
        Some(js_number(measured + 20.0).as_str()),
        "{rect}"
    );
}

/// The seed of the waves is the configured one, or the hash of the id when it is 0: under two ids the
/// same diagram with seed 0 draws other waves, and with a seed it draws the same ones.
#[test]
fn the_seed_decides_the_waves_and_zero_takes_the_id() {
    let diagram = parse("cynefin-beta\n  complex\n  \"A\"\n").expect("parses");
    // The ids differ in text that the SVG does not otherwise contain, so one can be replaced by the other.
    let under = |setup: &Setup, id: &str| draw_id(&diagram, setup, id).replace(id, "ID");
    let hashed = Setup {
        cynefin: vec![("seed", Value::Number(0.0))],
        ..Setup::default()
    };
    assert_ne!(
        under(&hashed, "probe-alpha"),
        under(&hashed, "probe-beta"),
        "a seed of 0 hashes the id"
    );
    let seeded = Setup {
        cynefin: vec![("seed", Value::Number(42.0))],
        ..Setup::default()
    };
    assert_eq!(
        under(&seeded, "probe-alpha"),
        under(&seeded, "probe-beta"),
        "a configured seed ignores the id"
    );
}

/// Under `colorScheme: auto` the colours are declared again under the media query (decision 0007), with
/// the dark scheme's own values.
#[test]
fn the_dark_scheme_declares_its_colours_under_the_media_query() {
    let diagram = parse("cynefin-beta\n  complex\n  \"A\"\n").expect("parses");
    let setup = Setup {
        dark_overrides: Some(Some(Value::from(
            serde_json::json!({"cynefin": {"complexBg": "#000001"}}),
        ))),
        ..Setup::default()
    };
    let svg = draw(&diagram, &setup);
    assert!(
        svg.contains("@media (prefers-color-scheme: dark){#chart{--cynefinComplexBg:#000001;"),
        "{svg}"
    );
    // The presentation attribute keeps the light scheme's colour.
    let rect = tags(&svg, "rect")
        .into_iter()
        .find(|tag| tag.contains("cynefin-fill-complex"))
        .expect("the background");
    assert_eq!(attribute_of(rect, "fill"), Some("#E8F5E9"), "{rect}");
}

/// The fonts: the faces of the text, bold for the labels and italic for the subtitles, and the symbol
/// face for the arrows of the subtitles (`docs-port/P3-PLAN.md`, decision 1 of 2026-10-18).
#[test]
fn the_font_subset_holds_the_bold_and_italic_faces_and_the_symbols() {
    let svg = draw_text("cynefin-beta\n  complex\n  \"A\"\n");
    assert!(svg.contains("font-weight:700;font-style:normal"), "{svg}");
    assert!(svg.contains("font-weight:400;font-style:italic"), "{svg}");
    assert!(svg.contains("-symbols\""), "{svg}");
    assert!(svg.contains("\"fg-"), "{svg}");
    // The faces the labels use are the bundled ones.
    assert!(face(FontFamily::Sans, FontWeight::Bold, FontStyle::Normal).covers('C'));
}

/// The diagram's text is escaped and decoded as the reference's pass does: a title with an entity
/// placeholder shows the character, and the stored title's entities show as text (NOTES 6.12).
#[test]
fn the_title_shows_its_entities_decoded_and_its_markup_escaped() {
    let svg = draw_text("cynefin-beta\n  title A #amp; B & C\n  complex\n  \"A\"\n");
    assert!(svg.contains(">A &amp; B &amp; C</text>"), "{svg}");
    // The stored title is sanitised: a `<` is kept as the entity DOMPurify writes, which then shows.
    let svg = draw_text("cynefin-beta\n  title A < B\n  complex\n  \"A\"\n");
    assert!(svg.contains(">A &amp;lt; B</text>"), "{svg}");
}

/// The numbers of the paths and the attributes are JavaScript's: a whole number has no decimal point, a
/// fraction has its shortest round-trip digits.
#[test]
fn the_numbers_are_written_as_javascript_writes_them() {
    let svg = draw_text("cynefin-beta\n  complex\n  \"A\"\n");
    assert!(svg.contains("x=\"200\" y=\"120\""), "{svg}");
    assert!(svg.contains("M400,300 C424,360"), "{svg}");
}
