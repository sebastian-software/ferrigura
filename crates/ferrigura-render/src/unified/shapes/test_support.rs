//! Helpers of the shape tests: a node in a look, a label given as its box, and the theme.

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::theme::Theme;
use ferrigura_model::value::Value;

use super::{ShapeInput, ShapeOutput, draw};
use crate::unified::Fixture;
use crate::unified::data::Node;
use crate::unified::label::LabelLayout;

/// The redux-color theme's variables, the default of flowcharts.
pub(super) fn theme() -> Value {
    Theme::ReduxColor.variables(None)
}

/// A flowchart vertex of the given shape and look, with the defaults of the flowchart: padding
/// 15 and a minimum label width of 120.
pub(super) fn node(shape: &str, look: &str) -> Node {
    Node {
        id: "A".to_owned(),
        dom_id: "flowchart-A-0".to_owned(),
        shape: shape.to_owned(),
        look: look.to_owned(),
        padding: 15.0,
        min_width: Some(120.0),
        css_classes: "default ".to_owned(),
        ..Node::default()
    }
}

/// Draws the node with a label of the given box, in the flowchart's default configuration.
pub(super) fn draw_box(node: &Node, label: &LabelLayout) -> ShapeOutput {
    let fixture = Fixture::new(MermaidConfig::default(), &theme());
    let config = fixture.config();
    draw(&ShapeInput {
        node,
        label: Some(label),
        config: &config,
        dir: Some("TB"),
    })
    .expect("the shape is known")
}
