//! The block renderer's behaviour on the inputs the reference treats specially: the quirks that the
//! geometry harness compares by their output (`docs-port/block/NOTES.md`, section 6 and the findings at a
//! glance), checked here as plain assertions on the SVG.

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::theme::Theme;
use ferrigura_render::RenderError;
use ferrigura_render::block::{BlockInput, render};
use ferrigura_render::theme::Themes;

fn render_text(text: &str, config: &MermaidConfig) -> Result<String, RenderError> {
    let diagram = ferrigura_syntax::block::parse(text).expect("the text parses");
    let variables = Theme::ReduxColor.variables(None);
    render(&BlockInput {
        diagram: &diagram,
        config,
        themes: Themes::Single(&variables),
        id: "b1",
    })
}

#[test]
fn a_drawing_of_only_spaces_has_a_nan_height_without_max_width() {
    // The reference writes `height="NaN"`: an empty group's box divided by its height (`blockRenderer.ts`
    // 70-79). The named exception of the port, `docs-port/block/NOTES.md`, finding 10.
    let mut config = MermaidConfig::default();
    config.block.use_max_width = false;
    let svg = render_text("block\n  space\n  space:2", &config).expect("renders");

    assert!(svg.contains("height=\"NaN\""), "{svg}");
}

#[test]
fn a_drawing_of_only_spaces_takes_the_max_width_of_ten_with_max_width() {
    let svg = render_text("block\n  space", &MermaidConfig::default()).expect("renders");

    assert!(svg.contains("style=\"max-width: 10px;\""), "{svg}");
}

#[test]
fn a_grid_of_zero_columns_is_a_render_error() {
    // `calculateBlockPosition` throws for `columns 0` (Q12).
    assert!(render_text("block\n  columns 0\n  A", &MermaidConfig::default()).is_err());
}

#[test]
fn a_composite_writes_its_classes_twice() {
    // `composite.ts` 14: the node's classes, then its `class` again.
    let svg = render_text(
        "block\n  columns 2\n  block:g\n    a\n  end",
        &MermaidConfig::default(),
    )
    .expect("renders");

    assert!(
        svg.contains("class=\"node default flowchart-label default flowchart-label\""),
        "{svg}"
    );
}

#[test]
fn a_chained_link_draws_its_first_block_with_no_id_and_no_edge() {
    // B2 and Q2: `A --> B --> C` keeps only the last link, whose first block has no id, so the drawing has
    // the `undefined` block and `C`, and no edge (its edge has no start).
    let svg = render_text(
        "block\n  columns 3\n  A --> B --> C",
        &MermaidConfig::default(),
    )
    .expect("renders");

    assert!(svg.contains("id=\"b1-undefined\""), "{svg}");
    assert!(svg.contains("id=\"b1-C\""), "{svg}");
    assert!(!svg.contains("id=\"b1-A\""), "{svg}");
    assert!(!svg.contains("data-edge"), "{svg}");
}

#[test]
fn a_composite_takes_a_palette_slot_on_a_colour_theme() {
    let config = MermaidConfig {
        theme: "redux-color".to_owned(),
        ..MermaidConfig::default()
    };
    let svg = render_text("block\n  columns 2\n  block:g\n    a\n  end", &config).expect("renders");

    assert!(svg.contains("data-color-id=\"color-0\""), "{svg}");
}

#[test]
fn a_plain_block_takes_no_palette_slot() {
    let config = MermaidConfig {
        theme: "redux-color".to_owned(),
        ..MermaidConfig::default()
    };
    let svg = render_text("block\n  columns 2\n  a b", &config).expect("renders");

    // The style sheet names the slots in its selectors; no element may carry one.
    assert!(!svg.contains(" data-color-id=\""), "{svg}");
}

#[test]
fn a_composite_takes_the_dark_cluster_colours_under_the_dark_scheme() {
    // Both schemes write the composite's cluster fill: the dark one under `prefers-color-scheme`, since its
    // colours carry an alpha and cannot be a custom property.
    let diagram = ferrigura_syntax::block::parse("block\n  columns 2\n  block:g\n    a\n  end")
        .expect("the text parses");
    let light = Theme::ReduxColor.variables(None);
    let dark = Theme::ReduxDarkColor.variables(None);
    let svg = render(&BlockInput {
        diagram: &diagram,
        config: &MermaidConfig::default(),
        themes: Themes::Schemes {
            light: &light,
            dark: &dark,
        },
        id: "b1",
    })
    .expect("renders");

    let (_, scheme) = svg
        .split_once("@media (prefers-color-scheme: dark)")
        .expect("the dark scheme is in a media query");
    assert!(scheme.contains("#b1 .node .cluster{fill:"), "{svg}");
}
