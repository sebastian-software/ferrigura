//! One test per quirk that the use case diagram reproduces from its reference, `packages/mermaid/src/diagrams/
//! usecase/` and `rendering-util/rendering-elements/shapes/usecase*.ts`, as `docs-port/usecase/NOTES.md`
//! section 6 records them. Each test names the reference line of its rule and checks the output that the
//! quirk shows.
//!
//! The quirks are checked on the SVG the renderer writes, or on the layout data it is built from, whichever
//! shows the rule. Measured values are the ones the stylesheet and the harness give: the theme's font is
//! 14 pixels (`redux-color`), and the stylesheet's defaults are 12 pixels for the use cases and 10 for the
//! edge labels.

use ferrigura_model::config::{MermaidConfig, defaults, typed};
use ferrigura_model::flowchart::LabelType;
use ferrigura_model::theme::{Theme, ThemeTokens};
use ferrigura_render::context::{Context, Globals, ThemeView};
use ferrigura_render::unified::Section;
use ferrigura_render::unified::data::{LayoutData, Node};
use ferrigura_render::use_case::data::{
    NodeFields, edge_dom_id, layout_data, node_dom_id, safe_id,
};
use ferrigura_render::use_case::{UseCaseSection, render};

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

/// The text from `marker` to the next node group, the end of the nodes or the end of the SVG: the
/// group's own markup, its label and its inner elements.
fn group<'a>(svg: &'a str, marker: &str) -> &'a str {
    let start = svg
        .find(marker)
        .unwrap_or_else(|| panic!("no {marker} in the SVG"));
    let rest = &svg[start..];
    let end = ["<g class=\"node ", "<defs>", "</svg>"]
        .iter()
        .filter_map(|next| rest[1..].find(next).map(|offset| offset + 1))
        .min()
        .unwrap_or(rest.len());
    &rest[..end]
}

/// The value of an attribute of a tag, as the text between its quotes.
fn attr<'a>(tag: &'a str, name: &str) -> &'a str {
    let key = format!(" {name}=\"");
    let start = tag
        .find(&key)
        .unwrap_or_else(|| panic!("no {name} in {tag}"))
        + key.len();
    let end = tag[start..]
        .find('"')
        .unwrap_or_else(|| panic!("unclosed {name} in {tag}"));
    &tag[start..start + end]
}

fn number(tag: &str, name: &str) -> f64 {
    attr(tag, name)
        .parse()
        .unwrap_or_else(|_| panic!("{name} of {tag} is not a number"))
}

/// The `aria-label` of the element that carries `data-usecase-id="<id>"`.
fn aria_label<'a>(svg: &'a str, id: &str) -> &'a str {
    attr(tag(svg, &format!("data-usecase-id=\"{id}\"")), "aria-label")
}

#[test]
fn a_actor_label_is_measured_at_the_theme_font_and_drawn_at_the_actor_font_size() {
    // `usecaseActor.ts` 165-190: the label is laid out at the theme's 14 pixels, its outline is that box, and
    // the class that sets the actor's size is applied after the measure, so the line is drawn at 16 pixels
    // when `actorFontSize` is 16. The line's baseline is at the size, as the other labels show.
    let plain = draw("usecase-beta\nactor User", |_| {});
    let larger = draw("usecase-beta\nactor User", |config| {
        config.usecase.actor_font_size = 16.0;
    });
    assert_eq!(
        tag(&plain, "class=\"usecase-actor-outline\""),
        tag(&larger, "class=\"usecase-actor-outline\""),
        "the outline is the measured box, whatever the drawn size"
    );
    assert!(
        group(&plain, "data-usecase-id=\"User\"")
            .contains("class=\"text-outer-tspan row\" x=\"0\" y=\"14\"")
    );
    assert!(
        group(&larger, "data-usecase-id=\"User\"")
            .contains("class=\"text-outer-tspan row\" x=\"0\" y=\"16\"")
    );
}

#[test]
fn b_a_business_use_case_label_is_drawn_at_the_usecase_font_size() {
    // `usecaseBusiness.ts` 49: the label gets its class after `labelHelper`, so it is drawn at
    // `usecaseFontSize` (12 by default) and the ellipse is the measured one.
    let plain = draw(
        "usecase-beta\nCheckout(Checkout)@{ business: true }",
        |_| {},
    );
    let larger = draw(
        "usecase-beta\nCheckout(Checkout)@{ business: true }",
        |config| {
            config.usecase.usecase_font_size = 20.0;
        },
    );
    assert_eq!(
        tag(&plain, "usecase-business-ellipse"),
        tag(&larger, "usecase-business-ellipse")
    );
    assert!(
        group(&plain, "data-usecase-id=\"Checkout\"")
            .contains("class=\"text-outer-tspan row\" x=\"0\" y=\"12\"")
    );
    assert!(
        group(&larger, "data-usecase-id=\"Checkout\"")
            .contains("class=\"text-outer-tspan row\" x=\"0\" y=\"20\"")
    );
}

#[test]
fn c_an_icon_actors_label_has_no_background() {
    // `usecaseActor.ts` 165-172: the label is laid out from the node without its icon, so the label takes no
    // background rectangle. A background would carry its box (`x`, `width`), which the empty one does not.
    let text = "usecase-beta\nactor Robot@{ icon: \"fa:robot\" }";
    let svg = draw(text, |_| {});
    assert!(
        group(&svg, "data-usecase-id=\"Robot\"")
            .contains("<rect class=\"background\" style=\"stroke: none\"></rect>")
    );
    assert_eq!(node(&layout_of(text), "Robot").icon, None);
}

#[test]
fn d_a_folded_ellipse_stereotype_takes_the_stereotype_class_and_a_folded_rectangle_does_not() {
    // `usecaseRenderer.ts` 168-188 gives the first line of a folded ellipse the class (fixture 20), and no
    // class is given to a folded rectangle (fixture 09).
    let ellipse = draw("usecase-beta\nLogin(\"Sign in\") <<Primary>>", |_| {});
    assert!(
        group(&ellipse, "data-usecase-id=\"Login\"")
            .contains("class=\"text-outer-tspan row usecase-stereotype\"")
    );
    let rectangle = draw("usecase-beta\nBrowse[Browse] <<Main>>", |_| {});
    assert!(!group(&rectangle, "data-usecase-id=\"Browse\"").contains("usecase-stereotype"));
}

#[test]
fn e_an_edge_label_is_measured_at_the_theme_font_and_centred_on_its_drawn_box() {
    // `usecaseDb.ts` and `styles.ts` 361-383: the label is 10 pixels, but its group is the box that the measure
    // at 14 pixels gives (the background is that box plus 2 on each side). The text starts at minus half of
    // its drawn width, the measured width scaled to 10 pixels (`positionRenderedEdgeLabel`).
    let svg = draw(
        "usecase-beta\nactor User\nLogin(\"Sign in\")\nUser pay@-- \"Pay now\" ----> Login",
        |_| {},
    );
    let label = group(&svg, "data-id=\"pay\"");
    assert!(
        label.contains(" y=\"10\" text-anchor=\"start\">"),
        "the line is drawn at 10 pixels"
    );
    let background = tag(label, "class=\"background\" style=\"stroke: none\" x=");
    let measured = number(background, "width") - 4.0;
    let drawn = measured * 10.0 / 14.0;
    let first_line = &label[label
        .find("class=\"text-outer-tspan row\" x=\"")
        .expect("a line")..];
    let start = first_line
        .split("x=\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .and_then(|text| text.parse::<f64>().ok())
        .expect("the line's x");
    // The measured box is float32, so the drawn width is the scaled one to about a hundredth of a pixel.
    assert!(
        (start + drawn / 2.0).abs() < 0.01,
        "the line starts at minus half its drawn width"
    );
    assert!(
        measured > drawn,
        "the group is wider than the text it holds"
    );
}

#[test]
fn f_an_animated_edge_class_is_named_by_its_animation() {
    // `usecaseDb.ts` 286-289: `animation` names the class, and `animate` alone is `fast`.
    let svg = draw(
        "usecase-beta\nactor User\nLogin(\"Sign in\")\nUser ride@--> Login\nride@{ animate: true, animation: slow }\nUser walk@--> Login\nwalk@{ animate: true }",
        |_| {},
    );
    assert!(tag(&svg, "id=\"usecase-t-ride\"").contains("edge-animation-slow"));
    assert!(tag(&svg, "id=\"usecase-t-walk\"").contains("edge-animation-fast"));
}

#[test]
fn g_a_folded_stereotype_is_a_markdown_line_with_its_markers_escaped() {
    // `usecaseRenderer.ts` 89 and 122-134: the stereotype and a plain label lose their Markdown markers to
    // a backslash, and the stereotype is written before the label as a Markdown line.
    let data = layout_of("usecase-beta\nLogin(\"Sign *in*\") <<Primary *actor*>>");
    let login = node(&data, "Login");
    assert_eq!(login.label, "«Primary \\*actor\\*»<br/>Sign \\*in\\*");
    assert_eq!(login.label_type, LabelType::Markdown);
    assert!(fields(login).folded_stereotype);
    assert_eq!(fields(login).stereotype, None);
}

#[test]
fn h_plain_labels_are_escaped_in_the_layout_data_and_the_names_use_the_sanitised_label() {
    // `usecaseRenderer.ts` 86-88 escapes a plain label after `getData` has sanitised it, while the accessible
    // name is made from the sanitised label, before the escape (`draw`, the names of 245-274). The sanitiser
    // (DOMPurify) returns a text without `<` as it is, so `&` stays, and a text with `<` is serialised, so
    // `<` becomes `&lt;` before the escape makes it `&amp;lt;`.
    let data = layout_of("usecase-beta\nLogin(\"Sign & in\")\nBrowse(\"x < y\")");
    let login = node(&data, "Login");
    assert_eq!(login.label, "Sign &amp; in");
    assert_eq!(fields(login).accessible_name, "use case Sign & in");
    let browse = node(&data, "Browse");
    assert_eq!(browse.label, "x &amp;lt; y");
    assert_eq!(fields(browse).accessible_name, "use case x &lt; y");
}

#[test]
fn i_accessible_names_follow_the_reference_forms() {
    // `usecaseRenderer.ts` 46-84: actors, use cases, notes, JSON tables, boundaries and relationships. The
    // note's edge has no name and is hidden from assistive technology.
    let svg = draw(
        "usecase-beta\nactor User\nactor Customer@{ type: hollow, business: true } <<External>>\nactor Alpha\nactor Beta\nLogin(\"Sign in\")\nBrowse(Browse)\nLogin ..>:include Browse\nnote for Login \"Remember\"\njson Payload@{ \"fruit\": \"Apple\", \"colors\": [\"Red\", \"Green\"] }\nsystemBoundary Auth[\"Authentication\"]@{ type: package }\nend\nsystemBoundary Shop[\"Store\"]\nend\nAlpha labelled@-- \"includes reporting\" ----> Beta",
        |_| {},
    );
    assert_eq!(aria_label(&svg, "User"), "actor User");
    assert_eq!(
        aria_label(&svg, "Customer"),
        "business hollow actor Customer, stereotype External"
    );
    assert_eq!(aria_label(&svg, "Login"), "use case Sign in");
    assert_eq!(aria_label(&svg, "note-0"), "Note for Sign in: Remember");
    assert_eq!(
        aria_label(&svg, "Payload"),
        "Payload: fruit: Apple; colors: Red; colors: Green"
    );
    // A boundary's name is made in the layout data (`usecaseRenderer.ts` 67-73); its SVG group does not carry
    // the annotation yet, so the name is checked here.
    let boundaries = layout_of(
        "usecase-beta\nsystemBoundary Auth[\"Authentication\"]@{ type: package }\nend\nsystemBoundary Shop[\"Store\"]\nend",
    );
    assert_eq!(
        fields(node(&boundaries, "Auth")).accessible_name,
        "package system boundary Authentication"
    );
    assert_eq!(
        fields(node(&boundaries, "Shop")).accessible_name,
        "rect system boundary Store"
    );
    assert_eq!(
        aria_label(&svg, "labelled"),
        "association includes reporting from Alpha to Beta"
    );
    assert_eq!(aria_label(&svg, "edge-0"), "include from Sign in to Browse");
    let note_edge = tag(&svg, "data-usecase-id=\"note-0-edge\"");
    assert!(note_edge.contains("aria-hidden=\"true\""));
    assert!(
        !note_edge.contains("aria-label"),
        "the note's edge has no name"
    );
}

#[test]
fn j_node_and_path_ids_are_prefixed_and_a_user_id_is_made_safe() {
    // `usecaseRenderer.ts` 27-35: a node's id is `usecase-<id>`, prefixed with the diagram's id by the
    // pipeline, and an edge's path is `usecase-<diagram>-<edge>`. Runs of other characters are one `_`, and
    // an empty id is `element`.
    let svg = draw("usecase-beta\nactor User\nUser edge@--> Login", |_| {});
    assert!(svg.contains("id=\"t-usecase-User\""));
    assert!(svg.contains("id=\"usecase-t-edge\""));
    assert_eq!(node_dom_id("quoted label/id"), "usecase-quoted_label_id");
    assert_eq!(safe_id("diagram:id 1"), "diagram_id_1");
    assert_eq!(safe_id("a  b:c"), "a_b_c");
    assert_eq!(safe_id("//"), "element");
    assert_eq!(edge_dom_id("<diagram>", "<model>"), "usecase-diagram-model");
}

#[test]
fn k_a_json_tables_compiled_styles_keep_the_first_place_of_a_property_and_its_last_value() {
    // `usecaseDb.ts` 118-135: a property keeps the position of its first declaration and the value of its last,
    // across the `default` class and the node's classes, and within one class.
    let across = layout_of(
        "usecase-beta\nclassDef default fill:#111,stroke:#222\nclassDef data stroke:#333,fill:#444\njson P@{\"a\": 1}:::data",
    );
    assert_eq!(
        node(&across, "P").css_compiled_styles,
        ["fill:#444", "stroke:#333"]
    );
    let within = layout_of(
        "usecase-beta\nclassDef data fill:#111,stroke:#222,fill:#333\njson P@{\"a\": 1}:::data",
    );
    assert_eq!(
        node(&within, "P").css_compiled_styles,
        ["fill:#333", "stroke:#222"]
    );
}

#[test]
fn l_the_extension_marker_is_named_by_the_use_case_and_the_root_escapes_the_font_quotes() {
    // `markers.js` 12-75 named by the diagram (`usecaseRenderer`'s marker set): `-margin` is the neo look's
    // variant. The root's style holds the font families with their quotes escaped (`applyUsecaseFonts`).
    let text = "usecase-beta\nLogin(\"Sign in\")\nBrowse(\"Browse\")\nLogin --|> Browse";
    let classic = draw(text, |_| {});
    assert!(classic.contains("marker-end=\"url(#t_usecase-extensionEnd)\""));
    assert!(
        classic.contains("--mermaid-usecase-actor-font-family: &quot;Open Sans&quot;, sans-serif;")
    );
    let neo = draw(text, |config| config.look = "neo".to_owned());
    assert!(neo.contains("marker-end=\"url(#t_usecase-extensionEnd-margin)\""));
}

#[test]
fn m_a_label_is_start_anchored_at_the_origin_without_a_shift_by_half_its_width() {
    // The lines are anchored at x = 0 and start there (the port's `Anchor::Origin`). `createText.ts` 109 and
    // 164 centre a label only when its anchor is the middle, so the label group is not moved left by half of
    // its width.
    let svg = draw("usecase-beta\nLogin(\"Sign in\")", |_| {});
    let node = group(&svg, "data-usecase-id=\"Login\"");
    assert!(node.contains("<g class=\"label\" style=\"\" transform=\"translate(0, "));
    assert!(
        node.contains(
            "<tspan class=\"text-outer-tspan row\" x=\"0\" y=\"14\" text-anchor=\"start\">"
        )
    );
}

#[test]
fn n_an_empty_diagram_has_the_padding_around_nothing() {
    // `setupViewPortForSVG` on an empty box: the viewBox is the padding of 20 on each side of nothing, and
    // the maximum width is 40 pixels.
    let svg = draw("usecase-beta", |_| {});
    assert!(svg.contains("viewBox=\"-20 -20 40 40\""));
    assert!(svg.contains("max-width: 40px;"));
}

#[test]
fn o_a_business_ellipses_stereotype_is_its_own_label_and_its_marker_is_a_chord_with_no_fill() {
    // `usecaseBusiness.ts` 64 and 75-81: the stereotype is a label of its own, not folded into the label,
    // and the marker is a chord whose ends lie on the ellipse, with `fill="none"`.
    let text = "usecase-beta\nAds(Ads)@{ business: true } <<Ext>>";
    let data = layout_of(text);
    let ads = node(&data, "Ads");
    assert_eq!(ads.label, "Ads");
    assert!(!fields(ads).folded_stereotype);
    assert_eq!(fields(ads).stereotype.as_deref(), Some("Ext"));

    let svg = draw(text, |_| {});
    assert!(
        group(&svg, "data-usecase-id=\"Ads\"").contains("<g class=\"label usecase-stereotype\"")
    );
    let ellipse = tag(&svg, "usecase-business-ellipse");
    let (radius_x, radius_y) = (number(ellipse, "rx"), number(ellipse, "ry"));
    let marker = tag(&svg, "class=\"usecase-business-marker\"");
    assert!(marker.contains("fill=\"none\""));
    let points: Vec<f64> = attr(marker, "d")
        .split_whitespace()
        .filter_map(|token| token.parse().ok())
        .collect();
    assert_eq!(points.len(), 4, "a chord has two ends");
    for ends in [(points[0], points[1]), (points[2], points[3])] {
        let (x, y) = ends;
        let on_ellipse = (x / radius_x).powi(2) + (y / radius_y).powi(2);
        assert!(
            (on_ellipse - 1.0).abs() < 1e-9,
            "the end ({x}, {y}) is on the ellipse"
        );
    }
}

#[test]
fn p_a_json_rows_show_numbers_and_booleans_as_their_scalar_display() {
    // The display of a scalar is the parser's (`json.rs`, `js_number_to_string`): an infinite number reads as
    // `Infinity` and `-0` as `0`. The layout data holds the rows as the table shows them.
    let data =
        layout_of("usecase-beta\njson Payload@{ \"big\": 1e400, \"zero\": -0, \"on\": true }");
    let rows: Vec<(&str, &str)> = fields(node(&data, "Payload"))
        .json_rows
        .iter()
        .map(|row| (row.key.as_str(), row.value.as_str()))
        .collect();
    assert_eq!(rows, [("big", "Infinity"), ("zero", "0"), ("on", "true")]);
}
