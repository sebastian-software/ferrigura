use super::*;
use crate::seed_selection::completed::CompletedGraph;
use crate::seed_selection::completed::tests::{EDGE, NODES};

const INSIDE_TOP_LEFT: i8 = 13;
const INSIDE_BOTTOM_RIGHT: i8 = 21;
const OUTSIDE_TOP_LEFT: i8 = 1;
const OUTSIDE_BOTTOM_RIGHT: i8 = 12;
const RIGHT: i64 = 5;
const LEFT: i64 = 7;
const LABEL: LabelMetadata<'static> = LabelMetadata {
    text: "node",
    position: INSIDE_TOP_LEFT,
    size: Dimensions {
        width: 30.0,
        height: 12.0,
    },
};
const NODE: NodeMetadata<'static> = NodeMetadata {
    id: 41,
    shape: "Table",
    container_id: None,
    table_columns: 2,
    is_3d: false,
    is_multiple: false,
    step_paint: None,
    is_invisible: false,
    fixed_top_left: Some(Point { x: 40.0, y: -20.0 }),
    desired_width: Some(140.0),
    desired_height: Some(90.0),
    force_hierarchy: true,
    label: Some(LABEL),
    icon_position: Some(OUTSIDE_TOP_LEFT),
    font_size: Some(16),
};
const EDGE_METADATA: EdgeMetadata<'static> = EdgeMetadata {
    id: 118,
    reversed_for_layout: false,
    source_arrowhead: "diamond",
    target_arrowhead: "triangle",
    source_arrowhead_label: Some(LabelMetadata {
        text: "source",
        ..LABEL
    }),
    target_arrowhead_label: Some(LabelMetadata {
        text: "target",
        ..LABEL
    }),
    minimum_size: Dimensions {
        width: 64.0,
        height: 32.0,
    },
    from_table_column: Some(0),
    to_table_column: Some(1),
    keep_explicit: false,
    is_invisible: false,
    style: EdgeStyle {
        opacity: Some("0"),
        stroke: Some("0"),
        fill: Some("0"),
        fill_pattern: Some("0"),
        stroke_width: Some("0"),
        stroke_dash: Some("0"),
        border_radius: Some("0"),
        shadow: Some("0"),
        three_dee: Some("0"),
        multiple: Some("0"),
        font: Some("0"),
        font_size: Some("0"),
        font_color: Some("0"),
        animated: Some("0"),
        bold: Some("0"),
        italic: Some("0"),
        underline: Some("0"),
        filled: Some("0"),
        double_border: Some("0"),
        text_transform: Some("0"),
    },
    label: Some(LabelMetadata {
        text: "edge",
        position: UNSET_LABEL_POSITION,
        size: Dimensions {
            width: 28.0,
            height: 10.0,
        },
    }),
};

fn authored() -> GraphMetadata<'static> {
    GraphMetadata {
        root_hierarchy: true,
        directions: &[(None, RIGHT)],
        nodes: &[NODE, NodeMetadata { id: 79, ..NODE }],
        edges: &[EDGE_METADATA],
    }
}

#[test]
fn rejects_changed_immutable_geometry_metadata() {
    let input = authored();
    let mut actual = authored();
    let missing = HashSet::new();
    assert_eq!(validate(&input, &actual, &missing, &|| false), Ok(()));
    actual.root_hierarchy = false;
    assert_eq!(
        validate(&input, &actual, &missing, &|| false),
        Err(ValidationError::ChangedGraphMetadata("root hierarchy"))
    );
    actual.root_hierarchy = true;
    actual.directions = &[(None, LEFT)];
    assert_eq!(
        validate(&input, &actual, &missing, &|| false),
        Err(ValidationError::ChangedGraphMetadata("graph direction"))
    );
    actual.directions = input.directions;

    macro_rules! node_case {
        ($field:ident = $value:expr, $reason:expr) => {{
            let nodes = [
                NodeMetadata {
                    $field: $value,
                    ..NODE
                },
                input.nodes[1],
            ];
            let result = GraphMetadata {
                nodes: &nodes,
                ..actual
            };
            assert_eq!(
                validate(&input, &result, &missing, &|| false),
                Err(ValidationError::ChangedNodeMetadata {
                    id: NODE.id,
                    field: $reason
                })
            );
        }};
    }
    node_case!(shape = "Circle", "shape");
    node_case!(container_id = Some(79), "container ownership");
    node_case!(table_columns = 3, "table column count");
    node_case!(is_3d = true, "3D outline metadata");
    node_case!(is_multiple = true, "multiple outline metadata");
    node_case!(
        step_paint = Some(StepPaint {
            order: 1,
            z_index: 0,
            opaque: true
        }),
        "step paint metadata"
    );
    node_case!(is_invisible = true, "visibility metadata");
    node_case!(
        fixed_top_left = Some(Point { x: 41.0, y: -20.0 }),
        "fixed position metadata"
    );
    node_case!(desired_width = Some(141.0), "desired-size metadata");
    node_case!(desired_height = None, "desired-size metadata");
    node_case!(force_hierarchy = false, "forced-hierarchy metadata");
    node_case!(
        label = Some(LabelMetadata {
            text: "replacement",
            ..LABEL
        }),
        "label identity"
    );
    node_case!(label = None, "label identity");
    node_case!(
        label = Some(LabelMetadata {
            position: INSIDE_BOTTOM_RIGHT,
            ..LABEL
        }),
        "fixed label position"
    );
    node_case!(icon_position = None, "icon presence");
    node_case!(
        icon_position = Some(OUTSIDE_BOTTOM_RIGHT),
        "fixed icon position"
    );
    node_case!(font_size = None, "font size outside the output domain");
    node_case!(font_size = Some(17), "font size outside the output domain");

    macro_rules! edge_case {
        ($field:ident = $value:expr, $reason:expr) => {{
            let edges = [EdgeMetadata {
                $field: $value,
                ..EDGE_METADATA
            }];
            let result = GraphMetadata {
                edges: &edges,
                ..actual
            };
            assert_eq!(
                validate(&input, &result, &missing, &|| false),
                Err(ValidationError::ChangedEdgeMetadata {
                    id: EDGE_METADATA.id,
                    field: $reason
                })
            );
        }};
    }
    let source = EDGE_METADATA.source_arrowhead_label.unwrap();
    let target = EDGE_METADATA.target_arrowhead_label.unwrap();
    let label = EDGE_METADATA.label.unwrap();
    edge_case!(source_arrowhead = "triangle", "arrowhead metadata");
    edge_case!(reversed_for_layout = true, "arrowhead metadata");
    edge_case!(
        source_arrowhead_label = Some(LabelMetadata {
            size: Dimensions {
                width: source.size.width + 1.0,
                ..source.size
            },
            ..source
        }),
        "source arrowhead label metadata"
    );
    edge_case!(
        target_arrowhead_label = Some(LabelMetadata {
            text: "replacement",
            ..target
        }),
        "target arrowhead label metadata"
    );
    edge_case!(
        minimum_size = Dimensions {
            width: 65.0,
            height: 32.0
        },
        "minimum geometry"
    );
    edge_case!(
        from_table_column = Some(1),
        "table column attachment metadata"
    );
    edge_case!(to_table_column = None, "table column attachment metadata");
    edge_case!(is_invisible = true, "visibility metadata");
    edge_case!(keep_explicit = true, "explicit-connection metadata");
    edge_case!(
        label = Some(LabelMetadata {
            text: "replacement",
            ..label
        }),
        "label identity or dimensions"
    );
    edge_case!(
        label = Some(LabelMetadata {
            size: Dimensions {
                width: label.size.width + 1.0,
                ..label.size
            },
            ..label
        }),
        "label identity or dimensions"
    );
    edge_case!(
        label = Some(LabelMetadata {
            size: Dimensions {
                height: label.size.height + 1.0,
                ..label.size
            },
            ..label
        }),
        "label identity or dimensions"
    );
    edge_case!(label = None, "label identity or dimensions");

    macro_rules! style_cases {
        ($($field:ident),+) => {
            $(edge_case!(style = EdgeStyle { $field: Some("corrupted"), ..EDGE_METADATA.style }, "style metadata");)+
        };
    }
    style_cases!(
        opacity,
        stroke,
        fill,
        fill_pattern,
        stroke_width,
        stroke_dash,
        border_radius,
        shadow,
        three_dee,
        multiple,
        font,
        font_size,
        font_color,
        animated,
        bold,
        italic,
        underline,
        filled,
        double_border,
        text_transform
    );

    for font_size in ENGINE_FONT_SIZES {
        let nodes = [
            NodeMetadata {
                font_size: Some(font_size),
                ..NODE
            },
            input.nodes[1],
        ];
        assert_eq!(
            validate(
                &input,
                &GraphMetadata {
                    nodes: &nodes,
                    ..actual
                },
                &missing,
                &|| false
            ),
            Ok(())
        );
    }
}

#[test]
fn validate_layout_result_metadata_does_not_validate_output_constraints() {
    let input = authored();
    let graph = CompletedGraph {
        nodes: &NODES,
        edges: &[EDGE],
    };
    assert_ne!(graph.nodes[0].top_left, input.nodes[0].fixed_top_left);
    assert_ne!(
        Some(graph.nodes[0].size.width),
        input.nodes[0].desired_width
    );
    assert_ne!(
        Some(graph.nodes[0].size.height),
        input.nodes[0].desired_height
    );
    assert_eq!(graph.validate(&|| false), Ok(()));
    assert_eq!(validate(&input, &input, &HashSet::new(), &|| false), Ok(()));
}

#[test]
fn allows_layout_owned_label_and_icon_positions_and_measured_node_label_size() {
    let input_nodes = [NodeMetadata {
        label: Some(LabelMetadata {
            position: UNSET_LABEL_POSITION,
            ..LABEL
        }),
        icon_position: Some(UNSET_LABEL_POSITION),
        ..NODE
    }];
    let output_nodes = [NodeMetadata {
        label: Some(LabelMetadata {
            size: Dimensions {
                width: 60.0,
                height: 20.0,
            },
            ..LABEL
        }),
        ..NODE
    }];
    let edges = [EdgeMetadata {
        label: Some(LabelMetadata {
            position: INSIDE_TOP_LEFT,
            ..EDGE_METADATA.label.unwrap()
        }),
        ..EDGE_METADATA
    }];
    let input = GraphMetadata {
        nodes: &input_nodes,
        ..authored()
    };
    let output = GraphMetadata {
        nodes: &output_nodes,
        edges: &edges,
        ..authored()
    };
    assert_eq!(
        validate(&input, &output, &HashSet::new(), &|| false),
        Ok(())
    );
}

#[test]
fn seed_metadata_validation_rejects_canceled_context() {
    let input = authored();
    let actual = GraphMetadata {
        root_hierarchy: false,
        ..authored()
    };
    assert_eq!(
        validate(&input, &actual, &HashSet::new(), &|| true),
        Err(ValidationError::Cancelled)
    );
}
