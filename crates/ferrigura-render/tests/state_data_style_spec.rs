//! Port of the three `getData` expectations of `packages/mermaid/src/diagrams/state/parser/
//! state-style.spec.js` ("defining (style)"): the `cssStyles` of the nodes after a `style` line.
//! The other tests of that spec are the parser's, in `ferrigura-syntax/tests/state_style_spec.rs`.

use ferrigura_model::config::MermaidConfig;
use ferrigura_render::context::Globals;
use ferrigura_render::state::data::get_data;
use ferrigura_render::unified::Section;
use ferrigura_syntax::state::parse_grammar;

/// The nodes of a text's layout data, after the grammar and the DB have parsed it.
fn nodes(text: &str) -> Vec<ferrigura_render::unified::data::Node> {
    let diagram = parse_grammar(text).expect("parses");
    get_data(
        &diagram,
        &Section::of(&MermaidConfig::default()),
        &Globals::of(&MermaidConfig::default()),
    )
    .nodes
}

mod defining_style {
    use super::*;

    #[test]
    fn has_style_as_a_keyword_an_id_and_can_set_a_css_style_attribute() {
        let data = nodes(
            "stateDiagram-v2
        id1
        style id1 background:#bbb",
        );
        assert_eq!(data[0].css_styles, ["background:#bbb"]);
    }

    #[test]
    fn has_style_as_a_keyword_an_id_and_can_set_a_css_style_attribute_2() {
        let data = nodes(
            "stateDiagram-v2
        id1
        id2
        style id1,id2 background:#bbb",
        );
        assert_eq!(data[0].css_styles, ["background:#bbb"]);
        assert_eq!(data[1].css_styles, ["background:#bbb"]);
    }

    #[test]
    fn can_define_multiple_attributes_separated_by_commas_2() {
        let data = nodes(
            "stateDiagram-v2
        id1
        id2
        style id1,id2 background:#bbb, font-weight:bold, font-style:italic;",
        );
        let expected = ["background:#bbb", "font-weight:bold", "font-style:italic"];
        assert_eq!(data[0].css_styles, expected);
        assert_eq!(data[1].css_styles, expected);
    }
}
