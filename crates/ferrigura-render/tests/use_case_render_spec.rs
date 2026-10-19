//! Port of the renderer and colour specs of the use case diagram: `usecaseRenderer.spec.ts` (its six tests and
//! the table), `usecaseRoleColors.spec.ts` (the role colours of the stylesheet), and the assertions of
//! `rendering-util/rendering-elements/shapes/usecaseShapes.spec.ts` that the SVG shows. The stylesheet is read
//! through `use_case::style::own_rules`, the rules that `styles.ts` writes under the diagram's id.
//!
//! Not ported: the `handDrawn` look (`styles.ts` has `isHandDrawn` splits that ferrigura does not draw, see
//! `use_case/style.rs`), HTML labels (decision 0004), and the icon pack registry (ferrigura registers none, so an
//! icon actor always draws the fallback symbol). The reference's `usecaseShapes` checks of the shapes' `intersect`
//! functions are not SVG output and are left to the geometry harness.

use ferrigura_jsmath::{atan2, pow};
use ferrigura_model::config::{MermaidConfig, defaults, typed};
use ferrigura_model::flowchart::LabelType;
use ferrigura_model::theme::{Theme, ThemeTokens};
use ferrigura_model::value::Value;
use ferrigura_render::context::{Context, Globals, ThemeView};
use ferrigura_render::unified::Section;
use ferrigura_render::unified::data::{LayoutData, Node};
use ferrigura_render::use_case::data::{NodeFields, layout_data};
use ferrigura_render::use_case::style::own_rules;
use ferrigura_render::use_case::{UseCaseSection, render};
use regex::Regex;
use serde_json::json;

/// The colour themes, which carry a palette (`colorThemeGate.ts`).
const COLOUR_THEMES: [(&str, Theme); 2] = [
    ("redux-color", Theme::ReduxColor),
    ("redux-dark-color", Theme::ReduxDarkColor),
];

/// The SVG of a text, with the configuration that `configure` gives (the defaults otherwise).
fn draw(text: &str, configure: impl FnOnce(&mut MermaidConfig)) -> String {
    let diagram = ferrigura_syntax::use_case::parse(text).expect("the text parses");
    let mut config = typed(&defaults()).expect("the defaults are typed");
    configure(&mut config);
    let globals = Globals::of(&config);
    let section = UseCaseSection {
        usecase: &config.usecase,
        unified: Section::of(&config),
    };
    let tokens = ThemeTokens::from_value(&Theme::ReduxColor.variables(None));
    render(&Context {
        diagram: &diagram,
        section: &section,
        globals: &globals,
        theme: ThemeView {
            light: &tokens,
            dark: None,
        },
        id: "t",
    })
    .expect("the diagram renders")
}

/// The layout data of a text, with the default configuration.
fn layout_of(text: &str) -> LayoutData {
    let diagram = ferrigura_syntax::use_case::parse(text).expect("the text parses");
    let config = typed(&defaults()).expect("the defaults are typed");
    let globals = Globals::of(&config);
    let section = UseCaseSection {
        usecase: &config.usecase,
        unified: Section::of(&config),
    };
    layout_data(&diagram, &section, &globals, "t")
}

fn node<'a>(data: &'a LayoutData, id: &str) -> &'a Node {
    data.nodes
        .iter()
        .find(|node| node.id == id)
        .unwrap_or_else(|| panic!("no node {id}"))
}

fn fields(node: &Node) -> &NodeFields {
    node.use_case
        .as_ref()
        .unwrap_or_else(|| panic!("{} has no use case fields", node.id))
}

/// The stylesheet of the use case diagram for a theme's name (the palette is kept for the colour themes only),
/// the tokens of the theme, a look and a colour scheme.
fn style_of(theme_name: &str, tokens: &ThemeTokens, look: &str, rotate: bool) -> String {
    let config = typed(&defaults()).expect("the defaults are typed");
    let mut globals = Globals::of(&config);
    globals.theme = theme_name.to_owned();
    globals.look = look.to_owned();
    own_rules("t", tokens, &globals, rotate)
}

/// The tokens of a built-in theme.
fn theme_tokens(theme: Theme) -> ThemeTokens {
    ThemeTokens::from_value(&theme.variables(None))
}

/// The number of palette slot rules that paint an actor or a use case (`usecaseRoleColors.spec.ts`).
fn leaf_slot_rules(css: &str) -> usize {
    Regex::new(r#"\[data-color-id="color-\d+"]\.usecase-(?:actor|element)"#)
        .expect("a valid pattern")
        .find_iter(css)
        .count()
}

/// The number of palette slot rules that paint a system boundary, which are emitted under both schemes.
fn boundary_slot_rules(css: &str) -> usize {
    Regex::new(r#"\[data-color-id="color-\d+"]\.system-boundary"#)
        .expect("a valid pattern")
        .find_iter(css)
        .count()
}

/// The number of palette slot rules of any element.
fn slot_rules(css: &str) -> usize {
    Regex::new(r#"\[data-color-id="color-\d+"]"#)
        .expect("a valid pattern")
        .find_iter(css)
        .count()
}

/// The `d` of a path, as its four coordinates.
fn chord(marker: &str) -> Vec<f64> {
    let start = marker.find(" d=\"").expect("a path") + 4;
    let end = marker[start..].find('"').expect("a closed d") + start;
    marker[start..end]
        .split_whitespace()
        .filter_map(|token| token.parse().ok())
        .collect()
}

/// The element that starts at `marker`, through its closing `>`.
fn tag<'a>(svg: &'a str, marker: &str) -> &'a str {
    let start = svg
        .find(marker)
        .unwrap_or_else(|| panic!("no {marker} in the SVG"));
    let end = svg[start..]
        .find('>')
        .map_or(svg.len(), |offset| start + offset + 1);
    &svg[start..end]
}

mod renderer {
    //! `usecaseRenderer.spec.ts`.
    use super::*;

    #[test]
    fn requests_every_relationship_marker_and_names_them_by_the_diagram() {
        // `USECASE_MARKERS` (`usecaseRenderer.ts` 13-18): point, circle, cross and extension, each with its start
        // and end, and the extension's margin variants: sixteen markers, named by the use case diagram.
        let svg = draw(
            "usecase-beta\nLogin(\"Sign in\")\nBrowse(\"Browse\")\nLogin --|> Browse",
            |_| {},
        );
        for name in [
            "pointEnd",
            "pointStart",
            "pointEnd-margin",
            "pointStart-margin",
            "circleEnd",
            "circleStart",
            "circleEnd-margin",
            "circleStart-margin",
            "crossEnd",
            "crossStart",
            "crossEnd-margin",
            "crossStart-margin",
            "extensionEnd",
            "extensionStart",
            "extensionEnd-margin",
            "extensionStart-margin",
        ] {
            assert!(
                svg.contains(&format!("id=\"t_usecase-{name}\"")),
                "the marker {name} is in the SVG"
            );
        }
    }

    #[test]
    fn renders_plain_labels_as_text_while_keeping_a_markdown_stereotype_and_label() {
        // The plain labels and edge labels are escaped (`usecaseRenderer.ts` 86-88, 109-114, 137-139); a Markdown
        // label passes through, and a stereotype of a folded use case is a Markdown line before it (122-134).
        let data = layout_of(
            "usecase-beta\nactor A\nLogin(\"`**formatted**`\") <<Main>>\nA labelled@-- \"<b>edge</b> **literal**\" ----> Login",
        );
        let login = node(&data, "Login");
        assert_eq!(login.label, "«Main»<br/>**formatted**");
        assert_eq!(login.label_type, LabelType::Markdown);
        assert_eq!(data.edges[0].label, "&lt;b&gt;edge&lt;/b&gt; **literal**");
        assert_eq!(data.edges[0].label_type, LabelType::Text);
    }

    #[test]
    fn names_each_variant_of_an_actor_and_a_use_case_in_the_reference_form() {
        // The table of `usecaseRenderer.spec.ts`: `getUsecaseNodeAccessibleName`, one row per variant.
        let rows = [
            ("actor Customer", "Customer", "actor Customer"),
            (
                "actor Customer@{ type: hollow }",
                "Customer",
                "hollow actor Customer",
            ),
            (
                "actor Customer@{ type: awesome }",
                "Customer",
                "awesome actor Customer",
            ),
            (
                "actor Robot@{ icon: \"fa:robot\" }",
                "Robot",
                "icon actor Robot",
            ),
            (
                "actor Customer@{ type: hollow, business: true } <<External>>",
                "Customer",
                "business hollow actor Customer, stereotype External",
            ),
            (
                "Checkout(Checkout)@{ business: true } <<Main>>",
                "Checkout",
                "business use case Checkout, stereotype Main",
            ),
        ];
        for (text, id, expected) in rows {
            let data = layout_of(&format!("usecase-beta\n{text}"));
            assert_eq!(fields(node(&data, id)).accessible_name, expected, "{text}");
        }
    }

    #[test]
    fn names_boundaries_notes_and_json_rows_in_their_semantic_order() {
        // `getUsecaseBoundaryAccessibleName`, the note's `Note for <target>: <label>` and the JSON rows as
        // `label: key: value; …` (`usecaseRenderer.ts` 46-73).
        let data = layout_of(
            "usecase-beta\nLogin(\"Sign in\")\nnote for Login \"Requires a session\"\njson Payload@{ \"fruit\": \"Apple\", \"colors\": [\"Red\", \"Green\"] }\nsystemBoundary Auth[\"Authentication\"]@{ type: package }\nend",
        );
        assert_eq!(
            fields(node(&data, "Auth")).accessible_name,
            "package system boundary Authentication"
        );
        assert_eq!(
            fields(node(&data, "note-0")).accessible_name,
            "Note for Sign in: Requires a session"
        );
        assert_eq!(
            fields(node(&data, "Payload")).accessible_name,
            "Payload: fruit: Apple; colors: Red; colors: Green"
        );
    }

    #[test]
    fn names_semantic_and_labelled_association_relationships_without_reading_the_label_text() {
        // `getUsecaseEdgeAccessibleName`: an include is named by its type even when its label says `include`, and
        // an association is named by its label, whatever the label holds.
        let data = layout_of(
            "usecase-beta\nactor Alpha\nactor Beta\nCheckout(Checkout)\nPayment(Payment)\nCheckout ..>:include Payment\nAlpha labelled@-- \"includes reporting\" ----> Beta",
        );
        let attributes = |id: &str| {
            data.edges
                .iter()
                .find(|edge| edge.id == id)
                .map(|edge| edge.path_attributes.clone())
                .unwrap_or_default()
        };
        assert!(attributes("edge-0").contains("aria-label=\"include from Checkout to Payment\""));
        assert!(
            attributes("labelled")
                .contains("aria-label=\"association includes reporting from Alpha to Beta\"")
        );
    }

    #[test]
    fn keeps_the_label_fonts_on_the_root_so_that_they_are_not_dropped_before_the_paint() {
        // `applyUsecaseFonts` (`usecaseRenderer.ts` 224-232): the six custom properties are on the root's style,
        // the families with their quotes escaped.
        let svg = draw("usecase-beta\nactor User", |_| {});
        for declaration in [
            "--mermaid-usecase-actor-font-size: 14px;",
            "--mermaid-usecase-actor-font-family: &quot;Open Sans&quot;, sans-serif;",
            "--mermaid-usecase-actor-font-weight: normal;",
            "--mermaid-usecase-font-size: 12px;",
            "--mermaid-usecase-font-family: &quot;Open Sans&quot;, sans-serif;",
            "--mermaid-usecase-font-weight: normal;",
        ] {
            assert!(svg.contains(declaration), "{declaration}");
        }
        assert!(svg.contains("class=\"usecaseDiagram\""));
    }
}

mod role_colours {
    //! `usecaseRoleColors.spec.ts`: each kind takes a role colour, and the palette slots are only for the
    //! boundaries unless the scheme rotates.
    use super::*;

    #[test]
    fn gives_actors_and_use_cases_no_palette_slot_under_either_colour_theme() {
        // The default is `role`, so a colour theme alone must not start rotating the leaves.
        for (name, theme) in COLOUR_THEMES {
            let css = style_of(name, &theme_tokens(theme), "classic", false);
            assert_eq!(leaf_slot_rules(&css), 0, "{name}");
        }
    }

    #[test]
    fn still_numbers_the_system_boundaries_under_either_colour_theme() {
        // Boundaries are the exception: slot N is "the Nth group", as flowchart subgraphs are numbered.
        for (name, theme) in COLOUR_THEMES {
            let css = style_of(name, &theme_tokens(theme), "classic", false);
            assert!(boundary_slot_rules(&css) > 0, "{name}");
        }
    }

    #[test]
    fn gives_actors_and_use_cases_a_slot_under_either_colour_theme_when_rotating() {
        // Pinned with the test above, so that a stylesheet with no slot rules at all cannot pass both.
        for (name, theme) in COLOUR_THEMES {
            let css = style_of(name, &theme_tokens(theme), "classic", true);
            assert!(leaf_slot_rules(&css) > 0, "{name}");
        }
    }

    #[test]
    fn emits_no_slot_rules_at_all_on_a_theme_without_a_palette() {
        // Including the boundary rules: with no palette there is nothing to number with.
        let tokens = theme_tokens(Theme::ReduxColor);
        assert_eq!(
            slot_rules(&style_of("neutral", &tokens, "classic", false)),
            0
        );
        assert_eq!(
            slot_rules(&style_of("neutral", &tokens, "classic", true)),
            0
        );
    }

    #[test]
    fn gives_actors_use_cases_and_boundaries_three_distinct_role_colours() {
        // The boundary token is the fallback for a theme with no palette, so it must differ from the other two.
        let variables = Theme::ReduxColor.variables(None);
        let text = |name: &str| {
            variables
                .get(name)
                .and_then(Value::as_str)
                .unwrap_or_else(|| panic!("{name} is a colour"))
                .to_owned()
        };
        let roles = [
            text("usecaseActorBorder"),
            text("usecaseBorder"),
            text("usecaseBoundaryBorder"),
        ];
        let distinct: std::collections::BTreeSet<&String> = roles.iter().collect();
        assert_eq!(distinct.len(), 3, "the three role colours differ");
        let css = style_of(
            "redux-color",
            &theme_tokens(Theme::ReduxColor),
            "classic",
            false,
        );
        for colour in &roles {
            assert!(
                css.contains(colour.as_str()),
                "{colour} is in the stylesheet"
            );
        }
    }

    #[test]
    fn falls_back_to_the_previous_colours_when_a_theme_declares_no_role_tokens() {
        // A theme without the usecase tokens keeps its previous colours: the body is `mainBkg`, the border
        // `nodeBorder`.
        let tokens = ThemeTokens::from_value(&Value::from(json!({
            "mainBkg": "#fefefe",
            "nodeBorder": "#101010",
            "primaryColor": "#fff8dc",
        })));
        let css = style_of("neutral", &tokens, "classic", false);
        assert!(css.contains("fill:var(--mainBkg, #fefefe)"));
        assert!(css.contains("stroke:var(--nodeBorder, #101010)"));
    }

    #[test]
    fn separates_include_from_extend_by_hue_and_not_only_by_dash() {
        // Both are dashed (`styles.ts` 335-359), so their colours are told apart by the role tokens.
        let variables = Theme::ReduxColor.variables(None);
        let include = variables
            .get("usecaseIncludeLine")
            .and_then(Value::as_str)
            .expect("the include colour");
        let extend = variables
            .get("usecaseExtendLine")
            .and_then(Value::as_str)
            .expect("the extend colour");
        assert_ne!(include, extend);
        let css = style_of(
            "redux-color",
            &theme_tokens(Theme::ReduxColor),
            "classic",
            false,
        );
        assert!(css.contains(&format!(
            "#t .relationship-include{{stroke:var(--usecaseIncludeLine, {include});}}"
        )));
        assert!(css.contains(&format!(
            "#t .relationship-extend{{stroke:var(--usecaseExtendLine, {extend});}}"
        )));
    }

    #[test]
    fn qualifies_the_use_case_body_rules_under_the_neo_look() {
        // The neo look's `[data-look="neo"].node rect` and `path` rules would tie with the plain selectors, so the
        // body rules are qualified with the node class (`styles.ts` 252-273).
        let css = style_of(
            "redux-color",
            &theme_tokens(Theme::ReduxColor),
            "neo",
            false,
        );
        assert!(css.contains("[data-look=\"neo\"].node.usecase-element ellipse"));
        assert!(css.contains("[data-look=\"neo\"].node.usecase-element rect"));
    }

    #[test]
    fn qualifies_the_business_marker_rule_under_the_neo_look() {
        let css = style_of(
            "redux-color",
            &theme_tokens(Theme::ReduxColor),
            "neo",
            false,
        );
        assert!(css.contains("[data-look=\"neo\"].node.usecase-element .usecase-business-marker"));
    }

    #[test]
    fn qualifies_the_actor_glyph_rule_under_the_neo_look() {
        // Three classes deep, which already clears `[data-look="neo"].node path`.
        let css = style_of(
            "redux-color",
            &theme_tokens(Theme::ReduxColor),
            "neo",
            false,
        );
        assert!(css.contains(".node.usecase-actor .usecase-actor-glyph path"));
    }

    #[test]
    fn honours_a_theme_variables_override_of_a_role_token() {
        // A role token is reachable through `themeVariables` even on a theme that never declares it.
        let tokens = ThemeTokens::from_value(&Value::from(json!({ "usecaseBorder": "#123456" })));
        let css = style_of("neutral", &tokens, "classic", false);
        assert!(css.contains("stroke:var(--usecaseBorder, #123456)"));
    }
}

#[test]
fn consolidates_the_canonical_selectors_and_uses_no_keyframes() {
    // `styles.ts` 196-395: the selectors of the actors, the business marker, the boundaries and their titles, the
    // notes, the JSON tables, the include, the edge labels and their backgrounds, and the markers. The font sizes
    // are the custom properties with their defaults, and the stylesheet has no keyframes.
    let css = style_of(
        "redux-color",
        &theme_tokens(Theme::ReduxColor),
        "classic",
        false,
    );
    for selector in [
        ".usecase-actor-hollow",
        ".usecase-actor-awesome",
        ".usecase-business-marker",
        ".system-boundary rect.boundary-body",
        ".system-boundary-title text",
        ".usecase-note",
        ".usecase-json-table",
        ".relationship-include",
        ".edgeLabel",
        ".labelBkg",
        ".marker.extension",
    ] {
        assert!(css.contains(selector), "{selector}");
    }
    assert!(css.contains("var(--mermaid-usecase-actor-font-size, 14px)"));
    assert!(css.contains("var(--mermaid-usecase-font-size, 12px)"));
    assert!(!css.contains("@keyframes"));
}

mod shapes {
    //! `usecaseShapes.spec.ts`: the glyphs, the business marker and the stereotypes, as the SVG shows them.
    use super::*;

    #[test]
    fn the_actor_variants_share_one_outline_size() {
        // The four variants have one measured footprint (`usecaseShapes.spec.ts`, the footprint test).
        let outlines: Vec<String> = [
            "actor Customer",
            "actor Customer@{ type: hollow }",
            "actor Customer@{ type: awesome }",
            "actor Customer@{ icon: \"fa:customer\" }",
        ]
        .iter()
        .map(|text| {
            let svg = draw(&format!("usecase-beta\n{text}"), |_| {});
            tag(&svg, "class=\"usecase-actor-outline\"").to_owned()
        })
        .collect();
        assert!(
            outlines.windows(2).all(|pair| pair[0] == pair[1]),
            "{outlines:?}"
        );
    }

    #[test]
    fn the_hollow_actor_has_wide_shoulders_a_body_and_a_head_with_no_fill() {
        // `usecaseActorHollow.ts` 10-23 and 30-31: the body path and the head circle of radius 9 at (0, -23).
        let svg = draw("usecase-beta\nactor Hollow@{ type: hollow }", |_| {});
        assert!(svg.contains(
            "<path class=\"usecase-actor-hollow-body\" d=\"M -22 -10 H 22 V 0 H 6 L 22 17 L 13 28 L 0 13 L -13 28 L -22 17 L -6 0 H -22 Z\" fill=\"none\"/>"
        ));
        assert!(svg.contains(
            "<circle class=\"usecase-actor-hollow-head\" cx=\"0\" cy=\"-23\" r=\"9\" fill=\"none\"/>"
        ));
    }

    #[test]
    fn the_business_marker_of_a_normal_or_hollow_head_is_a_chord_at_sixty_degrees() {
        // `usecaseActor.ts` 35-58: a chord that starts right of the axis and rises to the right at 60 degrees,
        // with both ends on the head's circle. The parser rejects a business awesome actor, so only the normal
        // and the hollow heads have a business marker in a diagram.
        for (text, centre_y, radius) in [
            ("actor Normal@{ business: true }", -24.0, 12.0),
            ("actor Hollow@{ type: hollow, business: true }", -23.0, 9.0),
        ] {
            let svg = draw(&format!("usecase-beta\n{text}"), |_| {});
            let marker = chord(tag(&svg, "class=\"usecase-business-marker"));
            assert_eq!(marker.len(), 4, "{text}: a chord has four coordinates");
            let (start_x, start_y, end_x, end_y) = (marker[0], marker[1], marker[2], marker[3]);
            assert!(start_x > 0.0 && start_x < end_x, "{text}");
            assert!(start_y > end_y, "{text}");
            for (x, y) in [(start_x, start_y), (end_x, end_y)] {
                let on_circle = pow(x / radius, 2.0) + pow((y - centre_y) / radius, 2.0);
                assert!(
                    (on_circle - 1.0).abs() < 1e-9,
                    "{text}: ({x}, {y}) is on the head"
                );
            }
            let angle = atan2(start_y - end_y, end_x - start_x).to_degrees();
            assert!(
                (angle - 60.0).abs() < 1e-9,
                "{text}: the chord is at {angle} degrees"
            );
        }
    }

    #[test]
    fn an_actor_stereotype_is_a_label_of_its_own_class() {
        // `usecaseActor.ts` 199-207: the stereotype is a label with the class `usecase-stereotype`, not folded.
        let svg = draw("usecase-beta\nactor Customer <<External>>", |_| {});
        assert!(svg.contains("<g class=\"label usecase-stereotype\""));
        assert!(svg.contains("«External»"));
    }

    #[test]
    fn the_awesome_actor_is_bundled_and_the_icon_actor_draws_the_fallback_symbol() {
        // `usecaseActorAwesome.ts`: the silhouette is drawn without a pack. `usecaseActorIcon.ts` 51: with no icon
        // pack, the symbol carries the fallback class.
        let awesome = draw("usecase-beta\nactor Customer@{ type: awesome }", |_| {});
        assert!(awesome.contains("class=\"usecase-actor-awesome-silhouette\""));
        let icon = draw("usecase-beta\nactor Robot@{ icon: \"fa:robot\" }", |_| {});
        assert!(icon.contains(
            "<g class=\"usecase-actor-icon-symbol usecase-actor-icon-fallback\" aria-hidden=\"true\""
        ));
    }
}
