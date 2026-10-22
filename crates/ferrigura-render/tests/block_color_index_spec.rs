//! Port of packages/mermaid/src/diagrams/block/blockColorIndex.spec.ts. The first four tests check the
//! palette slots of the composites: the `colorIndex` the parser gives them, in declaration order, outer
//! composites first, with one counter for the whole diagram. The last four check the palette rules of
//! the style sheet and the stamping of `data-color-id`: a slot with no rule renders uncoloured, and a rule
//! with no slot is dead CSS. The simple shapes keep the flat theme colour.

use ferrigura_model::block::Block;
use ferrigura_model::config::MermaidConfig;
use ferrigura_model::theme::{Theme, ThemeTokens};
use ferrigura_model::value::Value;
use ferrigura_render::block::style::style_sheet_with_fonts;
use ferrigura_render::context::{Globals, ThemeView};
use ferrigura_syntax::block::parse;

/// The `colorIndex` of the block with the id, anywhere in the tree.
fn color_index(blocks: &[Block], id: &str) -> Option<u32> {
    blocks.iter().find_map(|block| {
        if block.id.as_deref() == Some(id) {
            block.color_index
        } else {
            block
                .children
                .as_deref()
                .and_then(|children| color_index(children, id))
        }
    })
}

fn indices(text: &str, ids: &[&str]) -> Vec<Option<u32>> {
    let diagram = parse(text).expect("parses");
    ids.iter()
        .map(|id| color_index(&diagram.blocks, id))
        .collect()
}

#[test]
fn numbers_composites_in_declaration_order() {
    let text = "block-beta
      block:first
        a
      end
      block:second
        b
      end
      block:third
        c
      end
    ";
    assert_eq!(
        indices(text, &["first", "second", "third"]),
        [Some(0), Some(1), Some(2)]
    );
}

#[test]
fn gives_no_slot_to_a_simple_block() {
    // The flowchart colours containers and leaves its nodes alone; so does this.
    let text = "block-beta
      a
      b
      block:group
        c
      end
    ";
    assert_eq!(indices(text, &["a", "b", "c"]), [None, None, None]);
    assert_eq!(indices(text, &["group"]), [Some(0)]);
}

#[test]
fn numbers_an_outer_composite_before_one_nested_inside_it() {
    // Source order, not completion order: a container is declared before its children, so it takes the
    // lower slot even though it closes last.
    let text = "block-beta
      block:outer
        block:inner
          a
        end
      end
      block:sibling
        b
      end
    ";
    assert_eq!(
        indices(text, &["outer", "inner", "sibling"]),
        [Some(0), Some(1), Some(2)]
    );
}

#[test]
fn runs_one_counter_across_the_whole_diagram() {
    // Not per container: restarting the count inside each one would open every container's first child on
    // the same colour as its cousins.
    let text = "block-beta
      block:a1
        block:a2
          x
        end
      end
      block:b1
        block:b2
          y
        end
      end
    ";
    assert_eq!(
        indices(text, &["a1", "a2", "b1", "b2"]),
        [Some(0), Some(1), Some(2), Some(3)]
    );
}

/// The palette of the colour theme with two slots, and the look `neo`.
fn palette_config(look: &str) -> MermaidConfig {
    MermaidConfig {
        theme: "redux-color".to_owned(),
        look: look.to_owned(),
        ..MermaidConfig::default()
    }
}

fn palette_variables() -> Value {
    let overrides = serde_json::json!({
        "clusterBkg": "#f4f4f4",
        "clusterBorder": "#cccccc",
        "mainBkg": "#eeeeee",
        "nodeBorder": "#999999",
        "borderColorArray": ["#111111", "#222222"],
        "bkgColorArray": ["#eeeeee", "#dddddd"],
    });
    Theme::ReduxColor.variables(Some(&Value::from(overrides)))
}

fn style(config: &MermaidConfig, variables: &Value) -> String {
    let tokens = ThemeTokens::from_value(variables);
    let theme = ThemeView {
        light: &tokens,
        dark: None,
    };
    style_sheet_with_fonts("svg", theme, &Globals::of(config), &[], &[])
}

#[test]
fn emits_one_composite_rule_per_palette_entry_under_a_colour_theme() {
    let variables = palette_variables();
    let styles = style(&palette_config("neo"), &variables);

    assert!(
        styles.contains("[data-look=\"neo\"][data-color-id=\"color-0\"].node rect.composite"),
        "{styles}"
    );
    assert!(
        styles.contains("[data-look=\"neo\"][data-color-id=\"color-1\"].node rect.composite"),
        "{styles}"
    );
    // Exactly as many slots as the palette has entries.
    assert!(!styles.contains("color-2"), "{styles}");
    assert!(styles.contains("--borderColor0:#111111"), "{styles}");
    assert!(styles.contains("--bkgColor0:#eeeeee"), "{styles}");
}

#[test]
fn leaves_the_simple_shapes_to_the_flat_theme_colour() {
    let variables = palette_variables();
    let styles = style(&palette_config("neo"), &variables);

    // Every palette selector names `rect.composite`; nothing reaches a plain block.
    let palette_selectors: Vec<&str> = styles
        .split('{')
        .filter(|part| part.contains("data-color-id"))
        .collect();
    assert_eq!(palette_selectors.len(), 2, "{styles}");
    for selector in palette_selectors {
        assert!(selector.contains("rect.composite"), "{selector}");
    }
}

#[test]
fn emits_nothing_for_a_theme_that_carries_no_palette() {
    let variables = palette_variables();
    let mut config = palette_config("neo");
    config.theme = "default".to_owned();
    let styles = style(&config, &variables);

    assert!(!styles.contains("data-color-id"), "{styles}");
}

#[test]
fn rejects_a_look_that_would_break_out_of_the_selector() {
    // `look` is a top-level config key, so it is reachable from diagram frontmatter, and braces and quotes
    // survive the sanitiser; an unsafe look must not open a rule of its own.
    let variables = palette_variables();
    let styles = style(&palette_config("neo\"] { fill: red } [x=\""), &variables);

    assert!(!styles.contains("{ fill: red }"), "{styles}");
    assert!(!styles.contains("fill: red"), "{styles}");
    assert!(styles.contains("[data-look=\"classic\"]"), "{styles}");
}
