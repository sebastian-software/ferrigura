// DRAFT: not compiled or run yet. Target: crates/ferrigura-syntax/tests/flowchart_singlenode_spec.rs

//! Port of `packages/mermaid/src/diagrams/flowchart/parser/flow-singlenode.spec.js`. Test names
//! follow the spec's titles in snake_case. Each `it.each` case takes the case as a name suffix.

use ferrigura_model::flowchart::{FlowVertex, FlowchartDiagram};
use ferrigura_syntax::flowchart::{FlowchartError, parse_grammar};

fn vertex<'a>(diagram: &'a FlowchartDiagram, id: &str) -> &'a FlowVertex {
    diagram
        .vertices
        .iter()
        .find(|vertex| vertex.id == id)
        .unwrap_or_else(|| panic!("no vertex with id {id}"))
}

/// `it.each(keywords)('should handle keywords between dashes')`
macro_rules! dash_keyword_tests {
    ($($(#[$meta:meta])* $name:ident => $kw:literal,)*) => {
        $(
            $(#[$meta])*
            #[test]
            fn $name() {
                let kw = $kw;
                let id = format!("a-{kw}-node");
                let diagram = parse_grammar(&format!("graph TD;{id};")).expect("parses");
                assert_eq!(vertex(&diagram, &id).text, id);
            }
        )*
    };
}

/// `it.each(keywords)('should handle keywords between periods')`
macro_rules! period_keyword_tests {
    ($($(#[$meta:meta])* $name:ident => $kw:literal,)*) => {
        $(
            $(#[$meta])*
            #[test]
            fn $name() {
                let kw = $kw;
                let id = format!("a.{kw}.node");
                let diagram = parse_grammar(&format!("graph TD;{id};")).expect("parses");
                assert_eq!(vertex(&diagram, &id).text, id);
            }
        )*
    };
}

/// `it.each(keywords)('should handle keywords between underscores')`
macro_rules! underscore_keyword_tests {
    ($($(#[$meta:meta])* $name:ident => $kw:literal,)*) => {
        $(
            $(#[$meta])*
            #[test]
            fn $name() {
                let kw = $kw;
                let id = format!("a_{kw}_node");
                let diagram = parse_grammar(&format!("graph TD;{id};")).expect("parses");
                assert_eq!(vertex(&diagram, &id).text, id);
            }
        )*
    };
}

/// `it.each(keywords)('should handle nodes ending in %s')`
macro_rules! ending_keyword_tests {
    ($($(#[$meta:meta])* $name:ident => $kw:literal,)*) => {
        $(
            $(#[$meta])*
            #[test]
            fn $name() {
                let kw = $kw;
                let diagram =
                    parse_grammar(&format!("graph TD;node_{kw};node.{kw};node-{kw};")).expect("parses");
                assert_eq!(vertex(&diagram, &format!("node_{kw}")).text, format!("node_{kw}"));
                assert_eq!(vertex(&diagram, &format!("node.{kw}")).text, format!("node.{kw}"));
                assert_eq!(vertex(&diagram, &format!("node-{kw}")).text, format!("node-{kw}"));
            }
        )*
    };
}

/// `it.each(errorKeywords)('should throw error at nodes beginning with %s')`
macro_rules! error_keyword_tests {
    ($($(#[$meta:meta])* $name:ident => $kw:literal,)*) => {
        $(
            $(#[$meta])*
            #[test]
            fn $name() {
                let kw = $kw;
                let text = format!("graph TD;{kw}.node;{kw}-node;{kw}/node");
                assert!(matches!(parse_grammar(&text), Err(FlowchartError::Syntax(_))));
            }
        )*
    };
}

/// `it.each(workingKeywords)('should parse node beginning with %s')`
macro_rules! working_keyword_tests {
    ($($(#[$meta:meta])* $name:ident => $kw:literal,)*) => {
        $(
            $(#[$meta])*
            #[test]
            fn $name() {
                let kw = $kw;
                let diagram =
                    parse_grammar(&format!("graph TD; {kw}.node;{kw}-node;{kw}/node;")).expect("parses");
                assert_eq!(vertex(&diagram, &format!("{kw}.node")).text, format!("{kw}.node"));
                assert_eq!(vertex(&diagram, &format!("{kw}-node")).text, format!("{kw}-node"));
                assert_eq!(vertex(&diagram, &format!("{kw}/node")).text, format!("{kw}/node"));
            }
        )*
    };
}

/// `it.each(specialChars)('should allow node ids of single special characters')`
macro_rules! lone_special_char_tests {
    ($($name:ident => $c:literal,)*) => {
        $(
            #[test]
            fn $name() {
                let c: &str = $c;
                let diagram = parse_grammar(&format!("graph TD; {c} --> A")).expect("parses");
                assert_eq!(vertex(&diagram, c).text, c);
            }
        )*
    };
}

/// `it.each(specialChars)('should allow node ids with special characters at start of id')`
macro_rules! start_special_char_tests {
    ($($name:ident => $c:literal,)*) => {
        $(
            #[test]
            fn $name() {
                let c: &str = $c;
                let id = format!("{c}node");
                let diagram = parse_grammar(&format!("graph TD; {id} --> A")).expect("parses");
                assert_eq!(vertex(&diagram, &id).text, id);
            }
        )*
    };
}

/// `it.each(specialChars)('should allow node ids with special characters at end of id')`
macro_rules! end_special_char_tests {
    ($($name:ident => $c:literal,)*) => {
        $(
            #[test]
            fn $name() {
                let c: &str = $c;
                let id = format!("node{c}");
                let diagram = parse_grammar(&format!("graph TD; {id} --> A")).expect("parses");
                assert_eq!(vertex(&diagram, &id).text, id);
            }
        )*
    };
}

mod singlenodes_when_parsing {
    use super::*;

    #[test]
    fn should_handle_a_single_node() {
        let diagram = parse_grammar("graph TD;A;").expect("parses");
        assert!(diagram.edges.is_empty());
        assert!(vertex(&diagram, "A").styles.is_empty());
    }

    #[test]
    fn should_handle_a_single_node_with_white_space_after_it_sn1() {
        let diagram = parse_grammar("graph TD;A ;").expect("parses");
        assert!(diagram.edges.is_empty());
        assert!(vertex(&diagram, "A").styles.is_empty());
    }

    #[test]
    fn should_handle_a_single_square_node() {
        let diagram = parse_grammar("graph TD;a[A];").expect("parses");
        assert!(diagram.edges.is_empty());
        assert!(vertex(&diagram, "a").styles.is_empty());
        assert_eq!(vertex(&diagram, "a").shape.as_deref(), Some("square"));
    }

    #[test]
    fn should_handle_a_single_round_square_node() {
        let diagram = parse_grammar("graph TD;a[A];").expect("parses");
        assert!(diagram.edges.is_empty());
        assert!(vertex(&diagram, "a").styles.is_empty());
        assert_eq!(vertex(&diagram, "a").shape.as_deref(), Some("square"));
    }

    #[test]
    fn should_handle_a_single_circle_node() {
        let diagram = parse_grammar("graph TD;a((A));").expect("parses");
        assert!(diagram.edges.is_empty());
        assert_eq!(vertex(&diagram, "a").shape.as_deref(), Some("circle"));
    }

    #[test]
    fn should_handle_a_single_round_node() {
        let diagram = parse_grammar("graph TD;a(A);").expect("parses");
        assert!(diagram.edges.is_empty());
        assert_eq!(vertex(&diagram, "a").shape.as_deref(), Some("round"));
    }

    #[test]
    fn should_handle_a_single_odd_node() {
        let diagram = parse_grammar("graph TD;a>A];").expect("parses");
        assert!(diagram.edges.is_empty());
        assert_eq!(vertex(&diagram, "a").shape.as_deref(), Some("odd"));
    }

    #[test]
    fn should_handle_a_single_diamond_node() {
        let diagram = parse_grammar("graph TD;a{A};").expect("parses");
        assert!(diagram.edges.is_empty());
        assert_eq!(vertex(&diagram, "a").shape.as_deref(), Some("diamond"));
    }

    #[test]
    fn should_handle_a_single_diamond_node_with_whitespace_after_it() {
        let diagram = parse_grammar("graph TD;a{A}   ;").expect("parses");
        assert!(diagram.edges.is_empty());
        assert_eq!(vertex(&diagram, "a").shape.as_deref(), Some("diamond"));
    }

    #[test]
    fn should_handle_a_single_diamond_node_with_html_in_it_sn3() {
        let diagram = parse_grammar("graph TD;a{A <br> end};").expect("parses");
        assert!(diagram.edges.is_empty());
        assert_eq!(vertex(&diagram, "a").shape.as_deref(), Some("diamond"));
        assert_eq!(vertex(&diagram, "a").text, "A <br> end");
    }

    #[test]
    fn should_handle_a_single_hexagon_node() {
        let diagram = parse_grammar("graph TD;a{{A}};").expect("parses");
        assert!(diagram.edges.is_empty());
        assert_eq!(vertex(&diagram, "a").shape.as_deref(), Some("hexagon"));
    }

    #[test]
    fn should_handle_a_single_hexagon_node_with_html_in_it() {
        let diagram = parse_grammar("graph TD;a{{A <br> end}};").expect("parses");
        assert!(diagram.edges.is_empty());
        assert_eq!(vertex(&diagram, "a").shape.as_deref(), Some("hexagon"));
        assert_eq!(vertex(&diagram, "a").text, "A <br> end");
    }

    #[test]
    fn should_handle_a_single_round_node_with_html_in_it() {
        let diagram = parse_grammar("graph TD;a(A <br> end);").expect("parses");
        assert!(diagram.edges.is_empty());
        assert_eq!(vertex(&diagram, "a").shape.as_deref(), Some("round"));
        assert_eq!(vertex(&diagram, "a").text, "A <br> end");
    }

    #[test]
    fn should_handle_a_single_double_circle_node() {
        let diagram = parse_grammar("graph TD;a(((A)));").expect("parses");
        assert!(diagram.edges.is_empty());
        assert_eq!(vertex(&diagram, "a").shape.as_deref(), Some("doublecircle"));
    }

    #[test]
    fn should_handle_a_single_double_circle_node_with_whitespace_after_it() {
        let diagram = parse_grammar("graph TD;a(((A)))   ;").expect("parses");
        assert!(diagram.edges.is_empty());
        assert_eq!(vertex(&diagram, "a").shape.as_deref(), Some("doublecircle"));
    }

    #[test]
    fn should_handle_a_single_double_circle_node_with_html_in_it_sn3() {
        let diagram = parse_grammar("graph TD;a(((A <br> end)));").expect("parses");
        assert!(diagram.edges.is_empty());
        assert_eq!(vertex(&diagram, "a").shape.as_deref(), Some("doublecircle"));
        assert_eq!(vertex(&diagram, "a").text, "A <br> end");
    }

    #[test]
    fn should_handle_a_single_node_with_alphanumerics_starting_on_a_char() {
        let diagram = parse_grammar("graph TD;id1;").expect("parses");
        assert!(diagram.edges.is_empty());
        assert!(vertex(&diagram, "id1").styles.is_empty());
    }

    #[test]
    fn should_handle_a_single_node_with_a_single_digit() {
        let diagram = parse_grammar("graph TD;1;").expect("parses");
        assert!(diagram.edges.is_empty());
        assert_eq!(vertex(&diagram, "1").text, "1");
    }

    #[test]
    fn should_handle_a_single_node_with_a_single_digit_in_a_subgraph() {
        let diagram = parse_grammar("graph TD;subgraph \"hello\";1;end;").expect("parses");
        assert!(diagram.edges.is_empty());
        assert_eq!(vertex(&diagram, "1").text, "1");
    }

    #[test]
    fn should_handle_a_single_node_with_alphanumerics_starting_on_a_num() {
        let diagram = parse_grammar("graph TD;1id;").expect("parses");
        assert!(diagram.edges.is_empty());
        assert!(vertex(&diagram, "1id").styles.is_empty());
    }

    #[test]
    fn should_handle_a_single_node_with_alphanumerics_containing_a_minus_sign() {
        let diagram = parse_grammar("graph TD;i-d;").expect("parses");
        assert!(diagram.edges.is_empty());
        assert!(vertex(&diagram, "i-d").styles.is_empty());
    }

    #[test]
    fn should_handle_a_single_node_with_alphanumerics_containing_a_underscore_sign() {
        let diagram = parse_grammar("graph TD;i_d;").expect("parses");
        assert!(diagram.edges.is_empty());
        assert!(vertex(&diagram, "i_d").styles.is_empty());
    }

    dash_keyword_tests! {
        should_handle_keywords_between_dashes_graph => "graph",
        should_handle_keywords_between_dashes_flowchart => "flowchart",
        should_handle_keywords_between_dashes_style => "style",
        should_handle_keywords_between_dashes_default => "default",
        should_handle_keywords_between_dashes_linkstyle => "linkStyle",
        should_handle_keywords_between_dashes_interpolate => "interpolate",
        should_handle_keywords_between_dashes_classdef => "classDef",
        should_handle_keywords_between_dashes_class => "class",
        should_handle_keywords_between_dashes_href => "href",
        should_handle_keywords_between_dashes_call => "call",
        should_handle_keywords_between_dashes_click => "click",
        should_handle_keywords_between_dashes_self => "_self",
        should_handle_keywords_between_dashes_blank => "_blank",
        should_handle_keywords_between_dashes_parent => "_parent",
        should_handle_keywords_between_dashes_top => "_top",
        should_handle_keywords_between_dashes_end => "end",
        should_handle_keywords_between_dashes_subgraph => "subgraph",
    }

    period_keyword_tests! {
        should_handle_keywords_between_periods_graph => "graph",
        should_handle_keywords_between_periods_flowchart => "flowchart",
        should_handle_keywords_between_periods_style => "style",
        should_handle_keywords_between_periods_default => "default",
        should_handle_keywords_between_periods_linkstyle => "linkStyle",
        should_handle_keywords_between_periods_interpolate => "interpolate",
        should_handle_keywords_between_periods_classdef => "classDef",
        should_handle_keywords_between_periods_class => "class",
        should_handle_keywords_between_periods_href => "href",
        should_handle_keywords_between_periods_call => "call",
        should_handle_keywords_between_periods_click => "click",
        should_handle_keywords_between_periods_self => "_self",
        should_handle_keywords_between_periods_blank => "_blank",
        should_handle_keywords_between_periods_parent => "_parent",
        should_handle_keywords_between_periods_top => "_top",
        should_handle_keywords_between_periods_end => "end",
        should_handle_keywords_between_periods_subgraph => "subgraph",
    }

    underscore_keyword_tests! {
        should_handle_keywords_between_underscores_graph => "graph",
        should_handle_keywords_between_underscores_flowchart => "flowchart",
        should_handle_keywords_between_underscores_style => "style",
        should_handle_keywords_between_underscores_default => "default",
        should_handle_keywords_between_underscores_linkstyle => "linkStyle",
        should_handle_keywords_between_underscores_interpolate => "interpolate",
        should_handle_keywords_between_underscores_classdef => "classDef",
        should_handle_keywords_between_underscores_class => "class",
        should_handle_keywords_between_underscores_href => "href",
        should_handle_keywords_between_underscores_call => "call",
        should_handle_keywords_between_underscores_click => "click",
        should_handle_keywords_between_underscores_self => "_self",
        should_handle_keywords_between_underscores_blank => "_blank",
        should_handle_keywords_between_underscores_parent => "_parent",
        should_handle_keywords_between_underscores_top => "_top",
        should_handle_keywords_between_underscores_end => "end",
        should_handle_keywords_between_underscores_subgraph => "subgraph",
    }

    ending_keyword_tests! {
        should_handle_nodes_ending_in_graph => "graph",
        should_handle_nodes_ending_in_flowchart => "flowchart",
        should_handle_nodes_ending_in_style => "style",
        should_handle_nodes_ending_in_default => "default",
        should_handle_nodes_ending_in_linkstyle => "linkStyle",
        should_handle_nodes_ending_in_interpolate => "interpolate",
        should_handle_nodes_ending_in_classdef => "classDef",
        should_handle_nodes_ending_in_class => "class",
        should_handle_nodes_ending_in_href => "href",
        should_handle_nodes_ending_in_call => "call",
        should_handle_nodes_ending_in_click => "click",
        should_handle_nodes_ending_in_self => "_self",
        should_handle_nodes_ending_in_blank => "_blank",
        should_handle_nodes_ending_in_parent => "_parent",
        should_handle_nodes_ending_in_top => "_top",
        should_handle_nodes_ending_in_end => "end",
        should_handle_nodes_ending_in_subgraph => "subgraph",
    }

    error_keyword_tests! {
        should_throw_error_at_nodes_beginning_with_graph => "graph",
        should_throw_error_at_nodes_beginning_with_flowchart => "flowchart",
        should_throw_error_at_nodes_beginning_with_style => "style",
        should_throw_error_at_nodes_beginning_with_linkstyle => "linkStyle",
        should_throw_error_at_nodes_beginning_with_interpolate => "interpolate",
        should_throw_error_at_nodes_beginning_with_classdef => "classDef",
        should_throw_error_at_nodes_beginning_with_class => "class",
        should_throw_error_at_nodes_beginning_with_self => "_self",
        should_throw_error_at_nodes_beginning_with_blank => "_blank",
        should_throw_error_at_nodes_beginning_with_parent => "_parent",
        should_throw_error_at_nodes_beginning_with_top => "_top",
        should_throw_error_at_nodes_beginning_with_end => "end",
        should_throw_error_at_nodes_beginning_with_subgraph => "subgraph",
    }

    working_keyword_tests! {
        should_parse_node_beginning_with_default => "default",
        should_parse_node_beginning_with_href => "href",
        should_parse_node_beginning_with_click => "click",
        should_parse_node_beginning_with_call => "call",
    }

    lone_special_char_tests! {
        should_allow_node_ids_of_single_special_characters_hash => "#",
        should_allow_node_ids_of_single_special_characters_colon => ":",
        should_allow_node_ids_of_single_special_characters_zero => "0",
        should_allow_node_ids_of_single_special_characters_amp => "&",
        should_allow_node_ids_of_single_special_characters_comma => ",",
        should_allow_node_ids_of_single_special_characters_star => "*",
        should_allow_node_ids_of_single_special_characters_dot => ".",
        should_allow_node_ids_of_single_special_characters_backslash => "\\",
        should_allow_node_ids_of_single_special_characters_v => "v",
        should_allow_node_ids_of_single_special_characters_minus => "-",
        should_allow_node_ids_of_single_special_characters_slash => "/",
        should_allow_node_ids_of_single_special_characters_underscore => "_",
    }

    start_special_char_tests! {
        should_allow_node_ids_with_special_characters_at_start_of_id_hash => "#",
        should_allow_node_ids_with_special_characters_at_start_of_id_colon => ":",
        should_allow_node_ids_with_special_characters_at_start_of_id_zero => "0",
        should_allow_node_ids_with_special_characters_at_start_of_id_amp => "&",
        should_allow_node_ids_with_special_characters_at_start_of_id_comma => ",",
        should_allow_node_ids_with_special_characters_at_start_of_id_star => "*",
        should_allow_node_ids_with_special_characters_at_start_of_id_dot => ".",
        should_allow_node_ids_with_special_characters_at_start_of_id_backslash => "\\",
        should_allow_node_ids_with_special_characters_at_start_of_id_v => "v",
        should_allow_node_ids_with_special_characters_at_start_of_id_minus => "-",
        should_allow_node_ids_with_special_characters_at_start_of_id_slash => "/",
        should_allow_node_ids_with_special_characters_at_start_of_id_underscore => "_",
    }

    end_special_char_tests! {
        should_allow_node_ids_with_special_characters_at_end_of_id_hash => "#",
        should_allow_node_ids_with_special_characters_at_end_of_id_colon => ":",
        should_allow_node_ids_with_special_characters_at_end_of_id_zero => "0",
        should_allow_node_ids_with_special_characters_at_end_of_id_amp => "&",
        should_allow_node_ids_with_special_characters_at_end_of_id_comma => ",",
        should_allow_node_ids_with_special_characters_at_end_of_id_star => "*",
        should_allow_node_ids_with_special_characters_at_end_of_id_dot => ".",
        should_allow_node_ids_with_special_characters_at_end_of_id_backslash => "\\",
        should_allow_node_ids_with_special_characters_at_end_of_id_v => "v",
        should_allow_node_ids_with_special_characters_at_end_of_id_minus => "-",
        should_allow_node_ids_with_special_characters_at_end_of_id_slash => "/",
        should_allow_node_ids_with_special_characters_at_end_of_id_underscore => "_",
    }
}
