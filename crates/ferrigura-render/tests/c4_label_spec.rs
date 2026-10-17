//! Port of `packages/mermaid/src/rendering-util/rendering-elements/shapes/c4LabelHelper.spec.ts`: its
//! two cases. The spec counts the outer tspan elements of an element's label: one per line, so a label
//! with nothing wrapped has one for each of its three sections (name, stereotype and description). The
//! port renders the element with the real measurements, where the reference's spec measures with a
//! flat 7 px per character.

use ferrigura_model::config::{MermaidConfig, defaults, typed};
use ferrigura_model::theme::Theme;
use ferrigura_render::c4::{C4Input, render};
use ferrigura_render::theme::Themes;

/// A `System(...)` element as the spec's node: the name, the stereotype and a long description.
const SYSTEM: &str = "C4Context
  System(SystemAA, \"Internet Banking System\", \"Allows customers to view information about their bank accounts\")
";

/// Renders the diagram with the redux colour theme and the given configuration.
fn draw(config: &MermaidConfig) -> String {
    let diagram = ferrigura_syntax::c4::parse(SYSTEM).expect("the C4 diagram parses");
    let variables = Theme::ReduxColor.variables(None);
    render(&C4Input {
        diagram: &diagram,
        config,
        themes: Themes::Single(&variables),
        id: "c4-label",
    })
    .expect("the C4 diagram renders")
}

/// The configuration of the defaults, with `c4.wrap` set as the case needs it.
fn configured(wrap: bool) -> MermaidConfig {
    let mut config = typed(&defaults()).expect("the default configuration is typed");
    config.c4.wrap = wrap;
    config
}

/// The number of outer tspan elements, one for each line of each section of the labels.
fn outer_tspans(svg: &str) -> usize {
    svg.matches("class=\"text-outer-tspan").count()
}

// c4LabelHelper.spec.ts:47
#[test]
fn wraps_element_labels_by_default_so_the_label_stays_within_c4_width() {
    // Three sections give exactly three outer tspan elements when nothing wraps; the long description
    // has to break further (spec line 52).
    let svg = draw(&configured(true));
    assert!(
        outer_tspans(&svg) > 3,
        "{} outer tspans",
        outer_tspans(&svg)
    );
}

// c4LabelHelper.spec.ts:55
#[test]
fn does_not_wrap_when_c4_wrap_is_disabled() {
    // One line per section, none of them broken (spec line 60).
    let svg = draw(&configured(false));
    assert_eq!(outer_tspans(&svg), 3);
}
