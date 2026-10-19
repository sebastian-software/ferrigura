//! The venn renderer against the specs of the reference, `vennRenderer.spec.ts` and
//! `vennPalette.spec.ts` (`packages/mermaid/src/diagrams/venn/`), for what the renderer draws. The
//! reference's specs read the DOM of a page; these read the SVG the renderer writes, with the same
//! diagrams and the same expectations. The ensure-pairwise specs are unit tests in `venn/data.rs`,
//! beside the function they test. The theme-registry specs of `vennPalette.spec.ts` (every
//! registered theme) do not apply: ferrigura ships the two redux themes only.

mod venn_common;

use ferrigura_model::theme::khroma::{darken, lighten};
use venn_common::{Options, attribute, draw, inner_text, plain, start_tags};

/// The palette slots of `redux-color`, `venn1` to `venn8`.
const PALETTE_LIGHT: [&str; 8] = [
    "#E879F9", "#2DD4BF", "#FB923C", "#22D3EE", "#4ADE80", "#A78BFA", "#F87171", "#FACC15",
];

/// Two sets and their union: the reference's stub diagram of the palette specs.
const TWO_SETS: &str = "venn-beta
  set A
  set B
  union A, B
";

/// The circle groups of a render, as their start tags, in document order.
fn circles(svg: &str) -> Vec<&str> {
    start_tags(svg, "g")
        .into_iter()
        .filter(|tag| tag.contains("venn-circle"))
        .collect()
}

/// The text after the first tag that contains `marker`, through the end of the element.
fn group_at<'a>(svg: &'a str, group: &str) -> &'a str {
    let at = svg.find(group).expect("the group is in the render");
    &svg[at..]
}

/// The fill presentation attribute of the path of the circle `index`.
fn fill_of_circle(svg: &str, index: usize) -> String {
    let group = circles(svg)[index];
    let path = start_tags(group_at(svg, group), "path")[0];
    attribute(path, "fill").expect("a fill").to_owned()
}

#[test]
fn renders_a_title_when_provided() {
    let svg = plain(
        "venn-beta
  title My Venn Title
  set A
  set B
  union A, B
",
    );
    let title = start_tags(&svg, "text")
        .into_iter()
        .find(|tag| tag.contains("venn-title"))
        .expect("the title is drawn");
    assert_eq!(attribute(title, "y"), Some("16"));
    assert_eq!(
        inner_text(&svg, "class=\"venn-title\""),
        Some("My Venn Title")
    );
}

#[test]
fn renders_text_nodes_with_custom_color_via_style_data() {
    let svg = plain(
        "venn-beta
  set A
    text alpha
  set B
  union A, B
    text shared
  style alpha color:#ff0000
",
    );
    let nodes = start_tags(&svg, "text")
        .into_iter()
        .filter(|tag| tag.contains("venn-text-node"))
        .count();
    assert!(nodes >= 2, "one text node per text, found {nodes}");
    // The start tag of the text element that holds the label `alpha`.
    let label = svg.find(">alpha<").expect("the text node alpha is drawn");
    let start = svg[..label].rfind("<text").expect("its text element");
    let end = start + svg[start..].find('>').expect("the start tag ends") + 1;
    let alpha = &svg[start..end];
    assert_eq!(attribute(alpha, "fill"), Some("#ff0000"));
    assert!(
        alpha.contains("fill: #ff0000;"),
        "the style carries the colour: {alpha}"
    );
}

#[test]
fn applies_theme_colors_to_circles() {
    let svg = plain(TWO_SETS);
    let groups = circles(&svg);
    assert!(groups.len() >= 2);
    assert!(groups[0].contains("venn-set-0"));
    assert!(groups[1].contains("venn-set-1"));
}

#[test]
fn user_override_colors_take_priority_over_theme_via_style_data() {
    let svg = plain(
        "venn-beta
  set A
  set B
  union A, B
  style A,B color:#00ff00, fill:gold
",
    );
    let group = group_at(&svg, "class=\"venn-area venn-intersection\"");
    let path = start_tags(group, "path")[0];
    assert_eq!(attribute(path, "fill"), Some("gold"));
    assert_eq!(attribute(path, "fill-opacity"), Some("1"));
    let label = start_tags(group, "text")[0];
    assert_eq!(attribute(label, "fill"), Some("#00ff00"));
}

#[test]
fn computes_contrasting_text_color_for_dark_backgrounds() {
    // A dark background lightens the label of a circle, a light one darkens it (`vennRenderer.ts`
    // 120 and 163-164).
    let base = PALETTE_LIGHT[0];
    let dark = draw(
        TWO_SETS,
        &Options {
            variables: serde_json::json!({ "background": "#1a1a2e" }),
            ..Options::default()
        },
    )
    .expect("the diagram renders");
    let light = plain(TWO_SETS);
    let label_of = |svg: &str| {
        let group = circles(svg)[0];
        let text = start_tags(group_at(svg, group), "text")[0];
        attribute(text, "fill").expect("a fill").to_owned()
    };
    assert_eq!(
        label_of(&dark),
        lighten(base, 30.0).expect("the colour reads")
    );
    assert_eq!(
        label_of(&light),
        darken(base, 30.0).expect("the colour reads")
    );
}

#[test]
fn renders_debug_layout_helpers_when_enabled() {
    let svg = draw(
        "venn-beta
  set A
    text alpha
  set B
  union A, B
",
        &Options {
            venn: serde_json::json!({ "useDebugLayout": true }),
            ..Options::default()
        },
    )
    .expect("the diagram renders");
    assert!(svg.contains("class=\"venn-text-debug-circle\""));
    assert!(svg.contains("class=\"venn-text-debug-cell\""));
}

#[test]
fn the_redux_palettes_take_the_theme_categorical_palette() {
    for dark_theme in [false, true] {
        let tokens = venn_common::tokens(&Options {
            dark_theme,
            ..Options::default()
        });
        let palette = tokens.border_color_array.texts();
        assert!(!palette.is_empty());
        for slot in 1..=8 {
            let colour = tokens.venn.get(slot).text().expect("a palette slot");
            assert_eq!(colour, palette[(slot - 1) % palette.len()]);
        }
    }
}

#[test]
fn the_redux_palettes_give_every_slot_a_distinct_colour() {
    for dark_theme in [false, true] {
        let tokens = venn_common::tokens(&Options {
            dark_theme,
            ..Options::default()
        });
        let slots: std::collections::BTreeSet<String> = (1..=8)
            .map(|slot| tokens.venn.get(slot).text().expect("a slot"))
            .collect();
        assert_eq!(slots.len(), 8);
    }
}

#[test]
fn paints_each_circle_its_own_colour_under_a_palette_theme() {
    let svg = plain(TWO_SETS);
    let first = fill_of_circle(&svg, 0);
    let second = fill_of_circle(&svg, 1);
    assert_ne!(first, second);
    assert_eq!(first, PALETTE_LIGHT[0]);
}

#[test]
fn falls_back_to_one_colour_not_to_undefined_without_a_palette() {
    // Empty slots are not colours, so the palette is empty and every circle takes the primary
    // colour (`vennRenderer.ts` 128-129).
    let variables = serde_json::json!({
        "venn1": "", "venn2": "", "venn3": "", "venn4": "",
        "venn5": "", "venn6": "", "venn7": "", "venn8": "",
    });
    let svg = draw(
        TWO_SETS,
        &Options {
            variables,
            ..Options::default()
        },
    )
    .expect("the diagram renders");
    let first = fill_of_circle(&svg, 0);
    let second = fill_of_circle(&svg, 1);
    assert_eq!(first, second);
    assert!(!first.is_empty() && first != "undefined");
}
