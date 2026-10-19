//! The quirks of the venn renderer that change its output, one test each (`docs-port/venn/NOTES.md`,
//! sections 6.5 to 6.13). Each quirk is also a probe of the geometry harness, where it has one
//! (`tools/port/geometry/probes/venn`); these tests state the port's behaviour in the SVG.

mod venn_common;

use ferrigura_render::RenderError;
use venn_common::{Options, attribute, draw, inner_text, plain, start_tags};

/// The palette of `redux-color`, `venn1` onwards.
const VENN1: &str = "#E879F9";

/// The element whose start tag carries `marker`, through its end tag (`</g>`, the groups of a venn
/// do not nest).
fn element_of<'a>(svg: &'a str, marker: &str) -> &'a str {
    let at = svg.find(marker).expect("the element is drawn");
    let open = svg[..at].rfind('<').expect("its start tag");
    let close = open + svg[open..].find("</g>").expect("its end tag") + "</g>".len();
    &svg[open..close]
}

/// The `d` of the path of the first area whose `data-venn-sets` is `sets`.
fn path_of(svg: &str, sets: &str) -> String {
    let group = element_of(svg, &format!("data-venn-sets=\"{sets}\""));
    let path = start_tags(group, "path")[0];
    attribute(path, "d").expect("a path").to_owned()
}

/// The start tag of the `text` of the first area whose `data-venn-sets` is `sets`.
fn label_of<'a>(svg: &'a str, sets: &str) -> &'a str {
    let group = element_of(svg, &format!("data-venn-sets=\"{sets}\""));
    start_tags(group, "text")[0]
}

#[test]
fn circles_use_the_chart_padding_while_text_nodes_follow_the_configured_padding() {
    // The chart draws the circles with padding 15 whatever the configuration says; the text centres
    // come from `layout()`, which takes the configured padding (`vennRenderer.ts` 91-106).
    let text = "venn-beta
  set A
    text \"One\"
  set B
  union A, B
";
    let default = plain(text);
    let wide = draw(
        text,
        &Options {
            venn: serde_json::json!({ "padding": 20 }),
            ..Options::default()
        },
    )
    .expect("the diagram renders");
    assert_eq!(path_of(&default, "A"), path_of(&wide, "A"));
    assert_eq!(path_of(&default, "A_B"), path_of(&wide, "A_B"));
    let node_x = |svg: &str| {
        let start = svg.find("class=\"venn-text-node\"").expect("a text node");
        let tspan = start_tags(&svg[start..], "tspan")[0];
        attribute(tspan, "x").expect("an x").to_owned()
    };
    assert_ne!(node_x(&default), node_x(&wide));
}

#[test]
fn a_zero_size_set_leaves_the_chart_but_its_text_nodes_are_drawn() {
    let svg = plain(
        "venn-beta
  set A:0
  set B
  union A, B
    text \"inA\"
",
    );
    assert!(!svg.contains("data-venn-sets=\"A\""), "A is not drawn");
    assert!(
        !svg.contains("data-venn-sets=\"A_B\""),
        "its union is not drawn"
    );
    assert!(svg.contains(">inA<"), "the text node under A is drawn");
}

#[test]
fn colours_follow_the_order_of_the_drawn_circles() {
    // With `A` removed, `B` is the first circle, so it takes `venn1` (`vennRenderer.ts` 128).
    let svg = plain(
        "venn-beta
  set A:0
  set B
",
    );
    let group = element_of(&svg, "data-venn-sets=\"B\"");
    assert!(group.contains("venn-set-0"));
    let path = start_tags(group, "path")[0];
    assert_eq!(attribute(path, "fill"), Some(VENN1));
}

#[test]
fn a_disjoint_labelled_union_is_placed_off_the_canvas() {
    // `computeTextCentre` gives (0, -1000) to a disjoint area, and the path of the area is `M 0 0`.
    let svg = plain(
        "venn-beta
  set A
  set B
  union A, B[\"AB\"]:0
",
    );
    assert_eq!(path_of(&svg, "A_B"), "M 0 0");
    let label = label_of(&svg, "A_B");
    assert_eq!(attribute(label, "x"), Some("0"));
    assert_eq!(attribute(label, "y"), Some("-1000"));
    assert!(svg.contains(">AB<"), "the union keeps its label");
}

#[test]
fn an_empty_diagram_is_a_render_error() {
    let result = draw("venn-beta\n", &Options::default());
    assert!(matches!(result, Err(RenderError::Layout(_))), "{result:?}");
}

#[test]
fn a_negative_size_gives_nan_coordinates_and_no_error() {
    let svg = draw(
        "venn-beta
  set A: -5
  set B
  union A, B
",
        &Options::default(),
    )
    .expect("the reference does not fail on this, and neither does the port");
    assert!(svg.contains("NaN"));
}

#[test]
fn an_invalid_colour_fails_the_render() {
    // The label colour of the circle is derived from its fill, which khroma refuses to read.
    let result = draw(
        "venn-beta
  set A
  set B
  union A, B
  style A fill:notacolour
",
        &Options::default(),
    );
    assert!(matches!(result, Err(RenderError::Config(_))), "{result:?}");
}

#[test]
fn the_title_is_drawn_at_32_pixels_by_the_style_sheet() {
    // The attribute says 16px at width 800, but `styles.ts` sets `.venn-title` to 32px and the style
    // sheet beats the attribute (`docs-port/venn/NOTES.md` 6.3).
    let svg = plain(
        "venn-beta
  title Skills
  set A
  set B
  union A, B
",
    );
    let title = start_tags(&svg, "text")
        .into_iter()
        .find(|tag| tag.contains("venn-title"))
        .expect("the title is drawn");
    assert_eq!(attribute(title, "font-size"), Some("16px"));
    assert!(svg.contains(".venn-title{font-size:32px;"));
}

#[test]
fn the_title_is_written_as_text_after_its_sanitising() {
    let svg = plain(
        "venn-beta
  title AT&T
  set A
  set B
  union A, B
",
    );
    assert_eq!(inner_text(&svg, "class=\"venn-title\""), Some("AT&amp;T"));
}

#[test]
fn a_set_label_is_one_tspan_with_its_blanks_collapsed() {
    // `wrapText` measures nothing (the detached element has no width), so the label is one `tspan`
    // of its words joined by one blank (`src/diagram.js` 384-436).
    let svg = plain(
        "venn-beta
  set A[\"a    b  c\"]
  set B
",
    );
    let label = element_of(&svg, "data-venn-sets=\"A\"");
    let tspans = start_tags(label, "tspan");
    assert_eq!(tspans.len(), 1);
    assert!(label.contains(">a b c</tspan>"));
}

#[test]
fn a_duplicated_statement_draws_a_second_circle() {
    let svg = plain(
        "venn-beta
  set A
  set A
  set B
",
    );
    assert_eq!(svg.matches("data-venn-sets=\"A\"").count(), 2);
}

#[test]
fn a_set_without_a_label_shows_its_id_and_a_pair_without_one_shows_nothing() {
    let svg = plain(
        "venn-beta
  set A
  set B
  union A, B
",
    );
    let set = element_of(&svg, "data-venn-sets=\"A\"");
    assert!(
        set.contains("<tspan x=\"190\" y=\"225\" dy=\"0.35em\">A</tspan>"),
        "{set}"
    );
    let pair = element_of(&svg, "data-venn-sets=\"A_B\"");
    assert!(pair.contains("dy=\"0.35em\"></tspan>"), "{pair}");
}

#[test]
fn a_style_of_a_set_does_not_style_its_pairs() {
    // A style key is the sorted ids joined with `|`: `style A` styles `A` only (`vennRenderer.ts`
    // 17-29).
    let svg = plain(
        "venn-beta
  set A
  set B
  union A, B
  style A fill:#00ff00
",
    );
    assert_eq!(
        attribute(
            start_tags(element_of(&svg, "data-venn-sets=\"A\""), "path")[0],
            "fill"
        ),
        Some("#00ff00")
    );
    let pair = start_tags(element_of(&svg, "data-venn-sets=\"A_B\""), "path")[0];
    assert_eq!(attribute(pair, "fill-opacity"), Some("0"));
}

#[test]
fn the_use_max_width_of_the_venn_section_sets_the_size() {
    let svg = draw(
        "venn-beta
  set A
  set B
  union A, B
",
        &Options {
            venn: serde_json::json!({ "useMaxWidth": false }),
            ..Options::default()
        },
    )
    .expect("the diagram renders");
    let root = start_tags(&svg, "svg")[0];
    assert_eq!(attribute(root, "width"), Some("800"));
    assert_eq!(attribute(root, "height"), Some("450"));
    assert!(!root.contains("max-width"));
}

#[test]
fn the_hand_drawn_look_draws_the_classic_geometry() {
    // roughjs is not ported, so `look: handDrawn` draws the classic circles (as ishikawa does).
    let text = "venn-beta
  set A
  set B
  union A, B
";
    let classic = draw(text, &Options::default()).expect("the diagram renders");
    let hand = draw(
        text,
        &Options {
            look: Some("handDrawn"),
            ..Options::default()
        },
    )
    .expect("the diagram renders");
    assert_eq!(classic, hand);
}

#[test]
fn the_layout_of_four_sets_is_repeatable_for_a_seed() {
    // With four sets the layout draws random numbers; the same seed gives the same render.
    let text = "venn-beta
  set A
  set B
  set C
  set D
  union A, B
  union C, D
  union A, B, C
";
    let first = draw(
        text,
        &Options {
            seed: Some(7),
            ..Options::default()
        },
    )
    .expect("the diagram renders");
    let again = draw(
        text,
        &Options {
            seed: Some(7),
            ..Options::default()
        },
    )
    .expect("the diagram renders");
    assert_eq!(first, again);
}

#[test]
fn a_text_node_of_an_area_that_is_not_laid_out_is_dropped() {
    // The text of `A,C` is dropped, since the layout has no area for that pair (`vennRenderer.ts`
    // 272-276). An inline `text` is at column 0 (`docs-port/venn/NOTES.md` 2.5).
    let svg = plain(
        "venn-beta
  set A
  set B
  set C
  union A, B
    text \"Kept\"
text A,C \"Dropped\"
",
    );
    assert!(svg.contains(">Kept<"));
    assert!(!svg.contains(">Dropped<"));
}
