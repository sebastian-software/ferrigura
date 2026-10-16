// DRAFT: not compiled or run yet. Target: crates/ferrigura-syntax/tests/flowchart_text_spec.rs

//! Port of `packages/mermaid/src/diagrams/flowchart/parser/flow-text.spec.js`. Test names follow
//! the spec's titles in snake_case. Each `it.each` case takes the case as a name suffix, and the
//! shape tests put the shape in the name.
//!
//! The Jison messages of the reference are not copied. Those tests assert that parsing fails and
//! that the failure is on the expected line.

use ferrigura_model::flowchart::{FlowVertex, FlowchartDiagram};
use ferrigura_syntax::flowchart::{FlowchartError, parse_grammar};

fn vertex<'a>(diagram: &'a FlowchartDiagram, id: &str) -> &'a FlowVertex {
    diagram
        .vertices
        .iter()
        .find(|vertex| vertex.id == id)
        .unwrap_or_else(|| panic!("no vertex with id {id}"))
}

/// The line of the syntax error of `text`. Panics when the text parses, or fails without a
/// syntax error.
fn syntax_error_line(text: &str) -> usize {
    match parse_grammar(text) {
        Err(FlowchartError::Syntax(error)) => error.line,
        other => panic!("expected a syntax error, got {other:?}"),
    }
}

/// `it.each(keywords)('should handle %s keyword in <shape> vertex')`
macro_rules! shape_keyword_tests {
    ($shape:literal, $start:literal, $end:literal; $($name:ident => $kw:literal,)*) => {
        $(
            #[test]
            fn $name() {
                let kw = $kw;
                let (start, end) = ($start, $end);
                let diagram = parse_grammar(&format!(
                    "graph TD;A_{kw}_node-->B{start}This node has a {kw} as text{end};"
                ))
                .expect("parses");
                assert_eq!(vertex(&diagram, "B").shape.as_deref(), Some($shape));
                assert_eq!(
                    vertex(&diagram, "B").text,
                    format!("This node has a {kw} as text")
                );
            }
        )*
    };
}

/// `it.each(keywords)('should handle %s keyword in rect vertex')`
macro_rules! rect_keyword_tests {
    ($($name:ident => $kw:literal,)*) => {
        $(
            #[test]
            fn $name() {
                let kw = $kw;
                let diagram = parse_grammar(&format!(
                    "graph TD;A_{kw}_node-->B[|borders:lt|This node has a {kw} as text];"
                ))
                .expect("parses");
                assert_eq!(vertex(&diagram, "B").shape.as_deref(), Some("rect"));
                assert_eq!(
                    vertex(&diagram, "B").text,
                    format!("This node has a {kw} as text")
                );
            }
        )*
    };
}

mod text_when_parsing {
    use super::*;

    mod it_should_handle_text_on_edges {
        use super::*;

        #[test]
        fn should_handle_text_without_space() {
            let diagram = parse_grammar("graph TD;A--x|textNoSpace|B;").expect("parses");
            assert_eq!(diagram.edges[0].edge_type, "arrow_cross");
        }

        #[test]
        fn should_handle_with_space() {
            let diagram = parse_grammar("graph TD;A--x|text including space|B;").expect("parses");
            assert_eq!(diagram.edges[0].edge_type, "arrow_cross");
        }

        #[test]
        fn should_handle_text_with_slash() {
            let diagram =
                parse_grammar("graph TD;A--x|text with / should work|B;").expect("parses");
            assert_eq!(diagram.edges[0].text, "text with / should work");
        }

        #[test]
        fn should_handle_space_and_space_between_vertices_and_link() {
            let diagram = parse_grammar("graph TD;A --x|textNoSpace| B;").expect("parses");
            assert_eq!(diagram.edges[0].edge_type, "arrow_cross");
        }

        #[test]
        fn should_handle_space_and_caps() {
            let diagram =
                parse_grammar("graph TD;A--x|text including CAPS space|B;").expect("parses");
            assert_eq!(diagram.edges[0].edge_type, "arrow_cross");
        }

        #[test]
        fn should_handle_space_and_dir() {
            let diagram =
                parse_grammar("graph TD;A--x|text including URL space|B;").expect("parses");
            assert_eq!(diagram.edges[0].edge_type, "arrow_cross");
            assert_eq!(diagram.edges[0].text, "text including URL space");
        }

        #[test]
        fn should_handle_space_and_send() {
            let diagram = parse_grammar("graph TD;A--text including URL space and send-->B;")
                .expect("parses");
            assert_eq!(diagram.edges[0].edge_type, "arrow_point");
            assert_eq!(diagram.edges[0].text, "text including URL space and send");
        }

        #[test]
        fn should_handle_space_and_send_2() {
            let diagram = parse_grammar("graph TD;A-- text including URL space and send -->B;")
                .expect("parses");
            assert_eq!(diagram.edges[0].edge_type, "arrow_point");
            assert_eq!(diagram.edges[0].text, "text including URL space and send");
        }

        #[test]
        fn should_handle_space_and_dir_td() {
            let diagram =
                parse_grammar("graph TD;A--x|text including R TD space|B;").expect("parses");
            assert_eq!(diagram.edges[0].edge_type, "arrow_cross");
            assert_eq!(diagram.edges[0].text, "text including R TD space");
        }

        #[test]
        fn should_handle() {
            let diagram = parse_grammar("graph TD;A--x|text including `|B;").expect("parses");
            assert_eq!(diagram.edges[0].edge_type, "arrow_cross");
            assert_eq!(diagram.edges[0].text, "text including `");
        }

        #[test]
        fn should_handle_v_in_node_ids_only_v() {
            // only v
            let diagram = parse_grammar("graph TD;A--xv(my text);").expect("parses");
            assert_eq!(diagram.edges[0].edge_type, "arrow_cross");
            assert_eq!(vertex(&diagram, "v").text, "my text");
        }

        #[test]
        fn should_handle_v_in_node_ids_v_at_end() {
            // v at end
            let diagram = parse_grammar("graph TD;A--xcsv(my text);").expect("parses");
            assert_eq!(diagram.edges[0].edge_type, "arrow_cross");
            assert_eq!(vertex(&diagram, "csv").text, "my text");
        }

        #[test]
        fn should_handle_v_in_node_ids_v_in_middle() {
            // v in middle
            let diagram = parse_grammar("graph TD;A--xava(my text);").expect("parses");
            assert_eq!(diagram.edges[0].edge_type, "arrow_cross");
            assert_eq!(vertex(&diagram, "ava").text, "my text");
        }

        #[test]
        fn should_handle_v_in_node_ids_v_at_start() {
            // v at start
            let diagram = parse_grammar("graph TD;A--xva(my text);").expect("parses");
            assert_eq!(diagram.edges[0].edge_type, "arrow_cross");
            assert_eq!(vertex(&diagram, "va").text, "my text");
        }

        #[test]
        fn should_handle_keywords() {
            let diagram =
                parse_grammar("graph TD;A--x|text including graph space|B;").expect("parses");
            assert_eq!(diagram.edges[0].text, "text including graph space");
        }

        #[test]
        fn should_handle_keywords_2() {
            let diagram = parse_grammar("graph TD;V-->a[v]").expect("parses");
            assert_eq!(vertex(&diagram, "a").text, "v");
        }

        #[test]
        fn should_handle_quoted_text() {
            let diagram = parse_grammar("graph TD;V-- \"test string()\" -->a[v]").expect("parses");
            assert_eq!(diagram.edges[0].text, "test string()");
        }
    }

    mod it_should_handle_text_on_lines {
        use super::*;

        #[test]
        fn should_handle_normal_text_on_lines() {
            let diagram = parse_grammar("graph TD;A-- test text with == -->B;").expect("parses");
            assert_eq!(diagram.edges[0].stroke, "normal");
        }

        #[test]
        fn should_handle_dotted_text_on_lines_td3() {
            let diagram = parse_grammar("graph TD;A-. test text with == .->B;").expect("parses");
            assert_eq!(diagram.edges[0].stroke, "dotted");
        }

        #[test]
        fn should_handle_thick_text_on_lines() {
            let diagram = parse_grammar("graph TD;A== test text with - ==>B;").expect("parses");
            assert_eq!(diagram.edges[0].stroke, "thick");
        }
    }

    mod it_should_handle_text_on_edges_using_the_new_notation {
        use super::*;

        #[test]
        fn should_handle_text_without_space() {
            let diagram = parse_grammar("graph TD;A-- textNoSpace --xB;").expect("parses");
            assert_eq!(diagram.edges[0].edge_type, "arrow_cross");
        }

        #[test]
        fn should_handle_text_with_multiple_leading_space() {
            let diagram = parse_grammar("graph TD;A--    textNoSpace --xB;").expect("parses");
            assert_eq!(diagram.edges[0].edge_type, "arrow_cross");
        }

        #[test]
        fn should_handle_with_space() {
            let diagram = parse_grammar("graph TD;A-- text including space --xB;").expect("parses");
            assert_eq!(diagram.edges[0].edge_type, "arrow_cross");
        }

        #[test]
        fn should_handle_text_with_slash() {
            let diagram =
                parse_grammar("graph TD;A -- text with / should work --x B;").expect("parses");
            assert_eq!(diagram.edges[0].text, "text with / should work");
        }

        #[test]
        fn should_handle_space_and_space_between_vertices_and_link() {
            let diagram = parse_grammar("graph TD;A -- textNoSpace --x B;").expect("parses");
            assert_eq!(diagram.edges[0].edge_type, "arrow_cross");
        }

        #[test]
        fn should_handle_space_and_caps() {
            let diagram =
                parse_grammar("graph TD;A-- text including CAPS space --xB;").expect("parses");
            assert_eq!(diagram.edges[0].edge_type, "arrow_cross");
        }

        #[test]
        fn should_handle_space_and_dir() {
            let diagram =
                parse_grammar("graph TD;A-- text including URL space --xB;").expect("parses");
            assert_eq!(diagram.edges[0].edge_type, "arrow_cross");
            assert_eq!(diagram.edges[0].text, "text including URL space");
        }

        #[test]
        fn should_handle_space_and_dir_td2() {
            let diagram =
                parse_grammar("graph TD;A-- text including R TD space --xB;").expect("parses");
            assert_eq!(diagram.edges[0].edge_type, "arrow_cross");
            assert_eq!(diagram.edges[0].text, "text including R TD space");
        }

        #[test]
        fn should_handle_keywords() {
            let diagram = parse_grammar("graph TD;A-- text including graph space and v --xB;")
                .expect("parses");
            assert_eq!(diagram.edges[0].text, "text including graph space and v");
        }

        #[test]
        fn should_handle_keywords_2() {
            let diagram = parse_grammar("graph TD;A-- text including graph space and v --xB[blav]")
                .expect("parses");
            assert_eq!(diagram.edges[0].text, "text including graph space and v");
        }
    }

    mod it_should_handle_text_in_vertices {
        use super::*;

        #[test]
        fn should_handle_space() {
            let diagram = parse_grammar("graph TD;A-->C(Chimpansen hoppar);").expect("parses");
            assert_eq!(vertex(&diagram, "C").shape.as_deref(), Some("round"));
            assert_eq!(vertex(&diagram, "C").text, "Chimpansen hoppar");
        }

        shape_keyword_tests! {
            "square", "[", "]";
            should_handle_graph_keyword_in_square_vertex => "graph",
            should_handle_flowchart_keyword_in_square_vertex => "flowchart",
            should_handle_style_keyword_in_square_vertex => "style",
            should_handle_default_keyword_in_square_vertex => "default",
            should_handle_linkstyle_keyword_in_square_vertex => "linkStyle",
            should_handle_interpolate_keyword_in_square_vertex => "interpolate",
            should_handle_classdef_keyword_in_square_vertex => "classDef",
            should_handle_class_keyword_in_square_vertex => "class",
            should_handle_href_keyword_in_square_vertex => "href",
            should_handle_call_keyword_in_square_vertex => "call",
            should_handle_click_keyword_in_square_vertex => "click",
            should_handle_self_keyword_in_square_vertex => "_self",
            should_handle_blank_keyword_in_square_vertex => "_blank",
            should_handle_parent_keyword_in_square_vertex => "_parent",
            should_handle_top_keyword_in_square_vertex => "_top",
            should_handle_end_keyword_in_square_vertex => "end",
            should_handle_subgraph_keyword_in_square_vertex => "subgraph",
            should_handle_kitty_keyword_in_square_vertex => "kitty",
        }

        shape_keyword_tests! {
            "round", "(", ")";
            should_handle_graph_keyword_in_round_vertex => "graph",
            should_handle_flowchart_keyword_in_round_vertex => "flowchart",
            should_handle_style_keyword_in_round_vertex => "style",
            should_handle_default_keyword_in_round_vertex => "default",
            should_handle_linkstyle_keyword_in_round_vertex => "linkStyle",
            should_handle_interpolate_keyword_in_round_vertex => "interpolate",
            should_handle_classdef_keyword_in_round_vertex => "classDef",
            should_handle_class_keyword_in_round_vertex => "class",
            should_handle_href_keyword_in_round_vertex => "href",
            should_handle_call_keyword_in_round_vertex => "call",
            should_handle_click_keyword_in_round_vertex => "click",
            should_handle_self_keyword_in_round_vertex => "_self",
            should_handle_blank_keyword_in_round_vertex => "_blank",
            should_handle_parent_keyword_in_round_vertex => "_parent",
            should_handle_top_keyword_in_round_vertex => "_top",
            should_handle_end_keyword_in_round_vertex => "end",
            should_handle_subgraph_keyword_in_round_vertex => "subgraph",
            should_handle_kitty_keyword_in_round_vertex => "kitty",
        }

        shape_keyword_tests! {
            "diamond", "{", "}";
            should_handle_graph_keyword_in_diamond_vertex => "graph",
            should_handle_flowchart_keyword_in_diamond_vertex => "flowchart",
            should_handle_style_keyword_in_diamond_vertex => "style",
            should_handle_default_keyword_in_diamond_vertex => "default",
            should_handle_linkstyle_keyword_in_diamond_vertex => "linkStyle",
            should_handle_interpolate_keyword_in_diamond_vertex => "interpolate",
            should_handle_classdef_keyword_in_diamond_vertex => "classDef",
            should_handle_class_keyword_in_diamond_vertex => "class",
            should_handle_href_keyword_in_diamond_vertex => "href",
            should_handle_call_keyword_in_diamond_vertex => "call",
            should_handle_click_keyword_in_diamond_vertex => "click",
            should_handle_self_keyword_in_diamond_vertex => "_self",
            should_handle_blank_keyword_in_diamond_vertex => "_blank",
            should_handle_parent_keyword_in_diamond_vertex => "_parent",
            should_handle_top_keyword_in_diamond_vertex => "_top",
            should_handle_end_keyword_in_diamond_vertex => "end",
            should_handle_subgraph_keyword_in_diamond_vertex => "subgraph",
            should_handle_kitty_keyword_in_diamond_vertex => "kitty",
        }

        shape_keyword_tests! {
            "ellipse", "(-", "-)";
            should_handle_graph_keyword_in_ellipse_vertex => "graph",
            should_handle_flowchart_keyword_in_ellipse_vertex => "flowchart",
            should_handle_style_keyword_in_ellipse_vertex => "style",
            should_handle_default_keyword_in_ellipse_vertex => "default",
            should_handle_linkstyle_keyword_in_ellipse_vertex => "linkStyle",
            should_handle_interpolate_keyword_in_ellipse_vertex => "interpolate",
            should_handle_classdef_keyword_in_ellipse_vertex => "classDef",
            should_handle_class_keyword_in_ellipse_vertex => "class",
            should_handle_href_keyword_in_ellipse_vertex => "href",
            should_handle_call_keyword_in_ellipse_vertex => "call",
            should_handle_click_keyword_in_ellipse_vertex => "click",
            should_handle_self_keyword_in_ellipse_vertex => "_self",
            should_handle_blank_keyword_in_ellipse_vertex => "_blank",
            should_handle_parent_keyword_in_ellipse_vertex => "_parent",
            should_handle_top_keyword_in_ellipse_vertex => "_top",
            should_handle_end_keyword_in_ellipse_vertex => "end",
            should_handle_subgraph_keyword_in_ellipse_vertex => "subgraph",
            should_handle_kitty_keyword_in_ellipse_vertex => "kitty",
        }

        shape_keyword_tests! {
            "stadium", "([", "])";
            should_handle_graph_keyword_in_stadium_vertex => "graph",
            should_handle_flowchart_keyword_in_stadium_vertex => "flowchart",
            should_handle_style_keyword_in_stadium_vertex => "style",
            should_handle_default_keyword_in_stadium_vertex => "default",
            should_handle_linkstyle_keyword_in_stadium_vertex => "linkStyle",
            should_handle_interpolate_keyword_in_stadium_vertex => "interpolate",
            should_handle_classdef_keyword_in_stadium_vertex => "classDef",
            should_handle_class_keyword_in_stadium_vertex => "class",
            should_handle_href_keyword_in_stadium_vertex => "href",
            should_handle_call_keyword_in_stadium_vertex => "call",
            should_handle_click_keyword_in_stadium_vertex => "click",
            should_handle_self_keyword_in_stadium_vertex => "_self",
            should_handle_blank_keyword_in_stadium_vertex => "_blank",
            should_handle_parent_keyword_in_stadium_vertex => "_parent",
            should_handle_top_keyword_in_stadium_vertex => "_top",
            should_handle_end_keyword_in_stadium_vertex => "end",
            should_handle_subgraph_keyword_in_stadium_vertex => "subgraph",
            should_handle_kitty_keyword_in_stadium_vertex => "kitty",
        }

        shape_keyword_tests! {
            "odd", ">", "]";
            should_handle_graph_keyword_in_odd_vertex => "graph",
            should_handle_flowchart_keyword_in_odd_vertex => "flowchart",
            should_handle_style_keyword_in_odd_vertex => "style",
            should_handle_default_keyword_in_odd_vertex => "default",
            should_handle_linkstyle_keyword_in_odd_vertex => "linkStyle",
            should_handle_interpolate_keyword_in_odd_vertex => "interpolate",
            should_handle_classdef_keyword_in_odd_vertex => "classDef",
            should_handle_class_keyword_in_odd_vertex => "class",
            should_handle_href_keyword_in_odd_vertex => "href",
            should_handle_call_keyword_in_odd_vertex => "call",
            should_handle_click_keyword_in_odd_vertex => "click",
            should_handle_self_keyword_in_odd_vertex => "_self",
            should_handle_blank_keyword_in_odd_vertex => "_blank",
            should_handle_parent_keyword_in_odd_vertex => "_parent",
            should_handle_top_keyword_in_odd_vertex => "_top",
            should_handle_end_keyword_in_odd_vertex => "end",
            should_handle_subgraph_keyword_in_odd_vertex => "subgraph",
            should_handle_kitty_keyword_in_odd_vertex => "kitty",
        }

        shape_keyword_tests! {
            "cylinder", "[(", ")]";
            should_handle_graph_keyword_in_cylinder_vertex => "graph",
            should_handle_flowchart_keyword_in_cylinder_vertex => "flowchart",
            should_handle_style_keyword_in_cylinder_vertex => "style",
            should_handle_default_keyword_in_cylinder_vertex => "default",
            should_handle_linkstyle_keyword_in_cylinder_vertex => "linkStyle",
            should_handle_interpolate_keyword_in_cylinder_vertex => "interpolate",
            should_handle_classdef_keyword_in_cylinder_vertex => "classDef",
            should_handle_class_keyword_in_cylinder_vertex => "class",
            should_handle_href_keyword_in_cylinder_vertex => "href",
            should_handle_call_keyword_in_cylinder_vertex => "call",
            should_handle_click_keyword_in_cylinder_vertex => "click",
            should_handle_self_keyword_in_cylinder_vertex => "_self",
            should_handle_blank_keyword_in_cylinder_vertex => "_blank",
            should_handle_parent_keyword_in_cylinder_vertex => "_parent",
            should_handle_top_keyword_in_cylinder_vertex => "_top",
            should_handle_end_keyword_in_cylinder_vertex => "end",
            should_handle_subgraph_keyword_in_cylinder_vertex => "subgraph",
            should_handle_kitty_keyword_in_cylinder_vertex => "kitty",
        }

        shape_keyword_tests! {
            "doublecircle", "(((", ")))";
            should_handle_graph_keyword_in_doublecircle_vertex => "graph",
            should_handle_flowchart_keyword_in_doublecircle_vertex => "flowchart",
            should_handle_style_keyword_in_doublecircle_vertex => "style",
            should_handle_default_keyword_in_doublecircle_vertex => "default",
            should_handle_linkstyle_keyword_in_doublecircle_vertex => "linkStyle",
            should_handle_interpolate_keyword_in_doublecircle_vertex => "interpolate",
            should_handle_classdef_keyword_in_doublecircle_vertex => "classDef",
            should_handle_class_keyword_in_doublecircle_vertex => "class",
            should_handle_href_keyword_in_doublecircle_vertex => "href",
            should_handle_call_keyword_in_doublecircle_vertex => "call",
            should_handle_click_keyword_in_doublecircle_vertex => "click",
            should_handle_self_keyword_in_doublecircle_vertex => "_self",
            should_handle_blank_keyword_in_doublecircle_vertex => "_blank",
            should_handle_parent_keyword_in_doublecircle_vertex => "_parent",
            should_handle_top_keyword_in_doublecircle_vertex => "_top",
            should_handle_end_keyword_in_doublecircle_vertex => "end",
            should_handle_subgraph_keyword_in_doublecircle_vertex => "subgraph",
            should_handle_kitty_keyword_in_doublecircle_vertex => "kitty",
        }

        shape_keyword_tests! {
            "trapezoid", "[/", "\\]";
            should_handle_graph_keyword_in_trapezoid_vertex => "graph",
            should_handle_flowchart_keyword_in_trapezoid_vertex => "flowchart",
            should_handle_style_keyword_in_trapezoid_vertex => "style",
            should_handle_default_keyword_in_trapezoid_vertex => "default",
            should_handle_linkstyle_keyword_in_trapezoid_vertex => "linkStyle",
            should_handle_interpolate_keyword_in_trapezoid_vertex => "interpolate",
            should_handle_classdef_keyword_in_trapezoid_vertex => "classDef",
            should_handle_class_keyword_in_trapezoid_vertex => "class",
            should_handle_href_keyword_in_trapezoid_vertex => "href",
            should_handle_call_keyword_in_trapezoid_vertex => "call",
            should_handle_click_keyword_in_trapezoid_vertex => "click",
            should_handle_self_keyword_in_trapezoid_vertex => "_self",
            should_handle_blank_keyword_in_trapezoid_vertex => "_blank",
            should_handle_parent_keyword_in_trapezoid_vertex => "_parent",
            should_handle_top_keyword_in_trapezoid_vertex => "_top",
            should_handle_end_keyword_in_trapezoid_vertex => "end",
            should_handle_subgraph_keyword_in_trapezoid_vertex => "subgraph",
            should_handle_kitty_keyword_in_trapezoid_vertex => "kitty",
        }

        shape_keyword_tests! {
            "inv_trapezoid", "[\\", "/]";
            should_handle_graph_keyword_in_inv_trapezoid_vertex => "graph",
            should_handle_flowchart_keyword_in_inv_trapezoid_vertex => "flowchart",
            should_handle_style_keyword_in_inv_trapezoid_vertex => "style",
            should_handle_default_keyword_in_inv_trapezoid_vertex => "default",
            should_handle_linkstyle_keyword_in_inv_trapezoid_vertex => "linkStyle",
            should_handle_interpolate_keyword_in_inv_trapezoid_vertex => "interpolate",
            should_handle_classdef_keyword_in_inv_trapezoid_vertex => "classDef",
            should_handle_class_keyword_in_inv_trapezoid_vertex => "class",
            should_handle_href_keyword_in_inv_trapezoid_vertex => "href",
            should_handle_call_keyword_in_inv_trapezoid_vertex => "call",
            should_handle_click_keyword_in_inv_trapezoid_vertex => "click",
            should_handle_self_keyword_in_inv_trapezoid_vertex => "_self",
            should_handle_blank_keyword_in_inv_trapezoid_vertex => "_blank",
            should_handle_parent_keyword_in_inv_trapezoid_vertex => "_parent",
            should_handle_top_keyword_in_inv_trapezoid_vertex => "_top",
            should_handle_end_keyword_in_inv_trapezoid_vertex => "end",
            should_handle_subgraph_keyword_in_inv_trapezoid_vertex => "subgraph",
            should_handle_kitty_keyword_in_inv_trapezoid_vertex => "kitty",
        }

        shape_keyword_tests! {
            "lean_right", "[/", "/]";
            should_handle_graph_keyword_in_lean_right_vertex => "graph",
            should_handle_flowchart_keyword_in_lean_right_vertex => "flowchart",
            should_handle_style_keyword_in_lean_right_vertex => "style",
            should_handle_default_keyword_in_lean_right_vertex => "default",
            should_handle_linkstyle_keyword_in_lean_right_vertex => "linkStyle",
            should_handle_interpolate_keyword_in_lean_right_vertex => "interpolate",
            should_handle_classdef_keyword_in_lean_right_vertex => "classDef",
            should_handle_class_keyword_in_lean_right_vertex => "class",
            should_handle_href_keyword_in_lean_right_vertex => "href",
            should_handle_call_keyword_in_lean_right_vertex => "call",
            should_handle_click_keyword_in_lean_right_vertex => "click",
            should_handle_self_keyword_in_lean_right_vertex => "_self",
            should_handle_blank_keyword_in_lean_right_vertex => "_blank",
            should_handle_parent_keyword_in_lean_right_vertex => "_parent",
            should_handle_top_keyword_in_lean_right_vertex => "_top",
            should_handle_end_keyword_in_lean_right_vertex => "end",
            should_handle_subgraph_keyword_in_lean_right_vertex => "subgraph",
            should_handle_kitty_keyword_in_lean_right_vertex => "kitty",
        }

        shape_keyword_tests! {
            "lean_left", "[\\", "\\]";
            should_handle_graph_keyword_in_lean_left_vertex => "graph",
            should_handle_flowchart_keyword_in_lean_left_vertex => "flowchart",
            should_handle_style_keyword_in_lean_left_vertex => "style",
            should_handle_default_keyword_in_lean_left_vertex => "default",
            should_handle_linkstyle_keyword_in_lean_left_vertex => "linkStyle",
            should_handle_interpolate_keyword_in_lean_left_vertex => "interpolate",
            should_handle_classdef_keyword_in_lean_left_vertex => "classDef",
            should_handle_class_keyword_in_lean_left_vertex => "class",
            should_handle_href_keyword_in_lean_left_vertex => "href",
            should_handle_call_keyword_in_lean_left_vertex => "call",
            should_handle_click_keyword_in_lean_left_vertex => "click",
            should_handle_self_keyword_in_lean_left_vertex => "_self",
            should_handle_blank_keyword_in_lean_left_vertex => "_blank",
            should_handle_parent_keyword_in_lean_left_vertex => "_parent",
            should_handle_top_keyword_in_lean_left_vertex => "_top",
            should_handle_end_keyword_in_lean_left_vertex => "end",
            should_handle_subgraph_keyword_in_lean_left_vertex => "subgraph",
            should_handle_kitty_keyword_in_lean_left_vertex => "kitty",
        }

        shape_keyword_tests! {
            "subroutine", "[[", "]]";
            should_handle_graph_keyword_in_subroutine_vertex => "graph",
            should_handle_flowchart_keyword_in_subroutine_vertex => "flowchart",
            should_handle_style_keyword_in_subroutine_vertex => "style",
            should_handle_default_keyword_in_subroutine_vertex => "default",
            should_handle_linkstyle_keyword_in_subroutine_vertex => "linkStyle",
            should_handle_interpolate_keyword_in_subroutine_vertex => "interpolate",
            should_handle_classdef_keyword_in_subroutine_vertex => "classDef",
            should_handle_class_keyword_in_subroutine_vertex => "class",
            should_handle_href_keyword_in_subroutine_vertex => "href",
            should_handle_call_keyword_in_subroutine_vertex => "call",
            should_handle_click_keyword_in_subroutine_vertex => "click",
            should_handle_self_keyword_in_subroutine_vertex => "_self",
            should_handle_blank_keyword_in_subroutine_vertex => "_blank",
            should_handle_parent_keyword_in_subroutine_vertex => "_parent",
            should_handle_top_keyword_in_subroutine_vertex => "_top",
            should_handle_end_keyword_in_subroutine_vertex => "end",
            should_handle_subgraph_keyword_in_subroutine_vertex => "subgraph",
            should_handle_kitty_keyword_in_subroutine_vertex => "kitty",
        }

        shape_keyword_tests! {
            "hexagon", "{{", "}}";
            should_handle_graph_keyword_in_hexagon_vertex => "graph",
            should_handle_flowchart_keyword_in_hexagon_vertex => "flowchart",
            should_handle_style_keyword_in_hexagon_vertex => "style",
            should_handle_default_keyword_in_hexagon_vertex => "default",
            should_handle_linkstyle_keyword_in_hexagon_vertex => "linkStyle",
            should_handle_interpolate_keyword_in_hexagon_vertex => "interpolate",
            should_handle_classdef_keyword_in_hexagon_vertex => "classDef",
            should_handle_class_keyword_in_hexagon_vertex => "class",
            should_handle_href_keyword_in_hexagon_vertex => "href",
            should_handle_call_keyword_in_hexagon_vertex => "call",
            should_handle_click_keyword_in_hexagon_vertex => "click",
            should_handle_self_keyword_in_hexagon_vertex => "_self",
            should_handle_blank_keyword_in_hexagon_vertex => "_blank",
            should_handle_parent_keyword_in_hexagon_vertex => "_parent",
            should_handle_top_keyword_in_hexagon_vertex => "_top",
            should_handle_end_keyword_in_hexagon_vertex => "end",
            should_handle_subgraph_keyword_in_hexagon_vertex => "subgraph",
            should_handle_kitty_keyword_in_hexagon_vertex => "kitty",
        }

        rect_keyword_tests! {
            should_handle_graph_keyword_in_rect_vertex => "graph",
            should_handle_flowchart_keyword_in_rect_vertex => "flowchart",
            should_handle_style_keyword_in_rect_vertex => "style",
            should_handle_default_keyword_in_rect_vertex => "default",
            should_handle_linkstyle_keyword_in_rect_vertex => "linkStyle",
            should_handle_interpolate_keyword_in_rect_vertex => "interpolate",
            should_handle_classdef_keyword_in_rect_vertex => "classDef",
            should_handle_class_keyword_in_rect_vertex => "class",
            should_handle_href_keyword_in_rect_vertex => "href",
            should_handle_call_keyword_in_rect_vertex => "call",
            should_handle_click_keyword_in_rect_vertex => "click",
            should_handle_self_keyword_in_rect_vertex => "_self",
            should_handle_blank_keyword_in_rect_vertex => "_blank",
            should_handle_parent_keyword_in_rect_vertex => "_parent",
            should_handle_top_keyword_in_rect_vertex => "_top",
            should_handle_end_keyword_in_rect_vertex => "end",
            should_handle_subgraph_keyword_in_rect_vertex => "subgraph",
            should_handle_kitty_keyword_in_rect_vertex => "kitty",
        }

        #[test]
        fn should_handle_edge_case_for_odd_vertex_with_node_id_ending_with_minus() {
            let diagram = parse_grammar("graph TD;A_node-->odd->Vertex Text];").expect("parses");
            assert_eq!(vertex(&diagram, "odd-").shape.as_deref(), Some("odd"));
            assert_eq!(vertex(&diagram, "odd-").text, "Vertex Text");
        }

        #[test]
        fn should_allow_forward_slashes_in_lean_right_vertices() {
            let diagram =
                parse_grammar("graph TD;A_node-->B[/This node has a / as text/];").expect("parses");
            assert_eq!(vertex(&diagram, "B").shape.as_deref(), Some("lean_right"));
            assert_eq!(vertex(&diagram, "B").text, "This node has a / as text");
        }

        #[test]
        fn should_allow_back_slashes_in_lean_left_vertices() {
            let diagram = parse_grammar("graph TD;A_node-->B[\\This node has a \\ as text\\];")
                .expect("parses");
            assert_eq!(vertex(&diagram, "B").shape.as_deref(), Some("lean_left"));
            assert_eq!(vertex(&diagram, "B").text, "This node has a \\ as text");
        }

        #[test]
        fn should_handle_åäö_and_minus() {
            let diagram =
                parse_grammar("graph TD;A-->C{Chimpansen hoppar åäö-ÅÄÖ};").expect("parses");
            assert_eq!(vertex(&diagram, "C").shape.as_deref(), Some("diamond"));
            assert_eq!(vertex(&diagram, "C").text, "Chimpansen hoppar åäö-ÅÄÖ");
        }

        #[test]
        fn should_handle_with_åäö_minus_and_space_and_br() {
            let diagram = parse_grammar("graph TD;A-->C(Chimpansen hoppar åäö  <br> -  ÅÄÖ);")
                .expect("parses");
            assert_eq!(vertex(&diagram, "C").shape.as_deref(), Some("round"));
            assert_eq!(
                vertex(&diagram, "C").text,
                "Chimpansen hoppar åäö  <br> -  ÅÄÖ"
            );
        }

        #[test]
        fn should_handle_unicode_chars() {
            let diagram = parse_grammar("graph TD;A-->C(Начало);").expect("parses");
            assert_eq!(vertex(&diagram, "C").text, "Начало");
        }

        #[test]
        fn should_handle_backslash() {
            let diagram = parse_grammar("graph TD;A-->C(c:\\windows);").expect("parses");
            assert_eq!(vertex(&diagram, "C").text, "c:\\windows");
        }

        #[test]
        fn should_handle_caps() {
            let diagram = parse_grammar("graph TD;A-->C(some CAPS);").expect("parses");
            assert_eq!(vertex(&diagram, "C").shape.as_deref(), Some("round"));
            assert_eq!(vertex(&diagram, "C").text, "some CAPS");
        }

        #[test]
        fn should_handle_directions() {
            let diagram = parse_grammar("graph TD;A-->C(some URL);").expect("parses");
            assert_eq!(vertex(&diagram, "C").shape.as_deref(), Some("round"));
            assert_eq!(vertex(&diagram, "C").text, "some URL");
        }
    }

    #[test]
    fn should_handle_multi_line_text() {
        let diagram = parse_grammar("graph TD;A--o|text space|B;\n B-->|more text with space|C;")
            .expect("parses");
        assert_eq!(diagram.edges[0].edge_type, "arrow_circle");
        assert_eq!(diagram.edges[1].edge_type, "arrow_point");
        assert_eq!(vertex(&diagram, "A").id, "A");
        assert_eq!(vertex(&diagram, "B").id, "B");
        assert_eq!(vertex(&diagram, "C").id, "C");
        assert_eq!(diagram.edges.len(), 2);
        assert_eq!(diagram.edges[0].start, "A");
        assert_eq!(diagram.edges[0].end, "B");
        assert_eq!(diagram.edges[1].start, "B");
        assert_eq!(diagram.edges[1].end, "C");
        assert_eq!(diagram.edges[1].text, "more text with space");
    }

    #[test]
    fn should_handle_text_in_vertices_with_space() {
        let diagram = parse_grammar("graph TD;A[chimpansen hoppar]-->C;").expect("parses");
        assert_eq!(vertex(&diagram, "A").shape.as_deref(), Some("square"));
        assert_eq!(vertex(&diagram, "A").text, "chimpansen hoppar");
    }

    #[test]
    fn should_handle_text_in_vertices_with_space_with_spaces_between_vertices_and_link() {
        let diagram = parse_grammar("graph TD;A[chimpansen hoppar] --> C;").expect("parses");
        assert_eq!(vertex(&diagram, "A").shape.as_deref(), Some("square"));
        assert_eq!(vertex(&diagram, "A").text, "chimpansen hoppar");
    }

    #[test]
    fn should_handle_text_including_in_vertices() {
        let diagram = parse_grammar("graph TD;A[chimpansen_hoppar] --> C;").expect("parses");
        assert_eq!(vertex(&diagram, "A").shape.as_deref(), Some("square"));
        assert_eq!(vertex(&diagram, "A").text, "chimpansen_hoppar");
    }

    #[test]
    fn should_handle_quoted_text_in_vertices() {
        let diagram =
            parse_grammar("graph TD;A[\"chimpansen hoppar ()[]\"] --> C;").expect("parses");
        assert_eq!(vertex(&diagram, "A").shape.as_deref(), Some("square"));
        assert_eq!(vertex(&diagram, "A").text, "chimpansen hoppar ()[]");
    }

    #[test]
    fn should_handle_text_in_circle_vertices_with_space() {
        let diagram = parse_grammar("graph TD;A((chimpansen hoppar))-->C;").expect("parses");
        assert_eq!(vertex(&diagram, "A").shape.as_deref(), Some("circle"));
        assert_eq!(vertex(&diagram, "A").text, "chimpansen hoppar");
    }

    #[test]
    fn should_handle_text_in_ellipse_vertices() {
        let diagram = parse_grammar("graph TD\nA(-this is an ellipse-)-->B").expect("parses");
        assert_eq!(vertex(&diagram, "A").shape.as_deref(), Some("ellipse"));
        assert_eq!(vertex(&diagram, "A").text, "this is an ellipse");
    }

    #[test]
    fn should_not_freeze_when_ellipse_text_has_a_paren() {
        assert!(parse_grammar("graph\nX(- My Text (").is_err());
    }

    #[test]
    fn should_handle_text_in_diamond_vertices_with_space() {
        let diagram = parse_grammar("graph TD;A(chimpansen hoppar)-->C;").expect("parses");
        assert_eq!(vertex(&diagram, "A").shape.as_deref(), Some("round"));
        assert_eq!(vertex(&diagram, "A").text, "chimpansen hoppar");
    }

    #[test]
    fn should_handle_text_in_with() {
        let diagram = parse_grammar("graph TD;A(?)-->|?|C;").expect("parses");
        assert_eq!(vertex(&diagram, "A").text, "?");
        assert_eq!(diagram.edges[0].text, "?");
    }

    #[test]
    fn should_handle_text_in_with_éèêàçô() {
        let diagram = parse_grammar("graph TD;A(éèêàçô)-->|éèêàçô|C;").expect("parses");
        assert_eq!(vertex(&diagram, "A").text, "éèêàçô");
        assert_eq!(diagram.edges[0].text, "éèêàçô");
    }

    #[test]
    fn should_handle_text_in_with_2() {
        let diagram = parse_grammar("graph TD;A(,.?!+-*)-->|,.?!+-*|C;").expect("parses");
        assert_eq!(vertex(&diagram, "A").text, ",.?!+-*");
        assert_eq!(diagram.edges[0].text, ",.?!+-*");
    }

    #[test]
    fn should_throw_error_at_nested_set_of_brackets() {
        // Jison: "got 'PS'"
        assert_eq!(syntax_error_line("graph TD; A[This is a () in text];"), 1);
    }

    #[test]
    fn should_throw_error_for_strings_and_text_at_the_same_time() {
        // Jison: "got 'STR'"
        let text =
            "graph TD;A(this node has \"string\" and text)-->|this link has \"string\" and text|C;";
        assert_eq!(syntax_error_line(text), 1);
    }

    #[test]
    fn should_throw_error_for_escaping_quotes_in_text_state() {
        // Jison: "got 'STR'"
        assert_eq!(
            syntax_error_line("graph TD; A[This is a \"()\" in text];"),
            1
        );
    }

    #[test]
    fn should_throw_error_for_nested_quotation_marks() {
        // Jison: "Expecting 'SQE'"
        assert_eq!(
            syntax_error_line(r#"graph TD; A["This is a "()" in text"];"#),
            1
        );
    }

    #[test]
    fn should_throw_error() {
        // Jison: "got 'PE'"
        assert_eq!(
            syntax_error_line("graph TD; node[hello ) world] --> works"),
            1
        );
    }
}
