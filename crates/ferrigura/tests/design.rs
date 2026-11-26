//! The design mode (`docs/decisions/0023-design-mode.md`): ferrigura's own look and measures for a
//! flowchart, a state diagram and a class diagram, laid out by the engine of the default rendering.

use ferrigura::{ColorScheme, Error, RenderOptions, Renderer, Value};

/// A flowchart with every role, a neutral node, a labeled edge and a subgraph.
const FLOW: &str = "flowchart TD\n  A([Request]) --> B{Authenticated?}\n  B -- yes --> C[Load profile]\n  B -- no --> D[Login page]\n  C --> E[(User DB)]\n  subgraph Billing\n    F[Create invoice] --> G[[Charge card]]\n  end\n  C --> F\n  G --> H((Done))\n  D -.-> A\n";

/// The options of a render in the design mode with a fixed id.
fn design(scheme: Option<ColorScheme>) -> RenderOptions {
    RenderOptions {
        design: true,
        id: Some("d".to_owned()),
        color_scheme: scheme,
        ..RenderOptions::default()
    }
}

/// The options of a default render with the same id.
fn default(scheme: Option<ColorScheme>) -> RenderOptions {
    RenderOptions {
        id: Some("d".to_owned()),
        color_scheme: scheme,
        ..RenderOptions::default()
    }
}

/// The values of every occurrence of an attribute, in document order.
fn attribute_values<'a>(svg: &'a str, name: &str) -> Vec<&'a str> {
    let pattern = format!(" {name}=\"");
    svg.match_indices(&pattern)
        .filter_map(|(at, _)| {
            let rest = &svg[at + pattern.len()..];
            rest.find('"').map(|end| &rest[..end])
        })
        .collect()
}

/// The horizontal place of the first group title of an SVG.
fn title_x(svg: &str) -> f64 {
    let marker = "<g class=\"cluster-label \" transform=\"translate(";
    let rest = &svg[svg.find(marker).expect("a group title") + marker.len()..];
    rest[..rest.find(',').expect("the x")]
        .parse()
        .expect("a number")
}

/// The left edge of the first group frame of an SVG.
fn frame_x(svg: &str) -> f64 {
    let group = &svg[svg.find("<g class=\"cluster ").expect("a group")..];
    let marker = " x=\"";
    let rest = &group[group.find(marker).expect("the frame's x") + marker.len()..];
    rest[..rest.find('"').expect("the x's end")]
        .parse()
        .expect("a number")
}

/// The part of an SVG that holds its nodes.
fn nodes(svg: &str) -> &str {
    &svg[svg.find("<g class=\"nodes\">").expect("the nodes")..]
}

/// The style sheet of an SVG.
fn style_sheet(svg: &str) -> &str {
    let start = svg.find("<style>").expect("a style sheet") + "<style>".len();
    let end = svg.find("</style>").expect("the style sheet's end");
    &svg[start..end]
}

/// The size of the outline of the node with the given id: the `rect` of its group.
fn node_size(svg: &str, id: &str) -> (f64, f64) {
    let group = &svg[svg
        .find(&format!("id=\"d-flowchart-{id}-"))
        .expect("the node")..];
    let rect = &group[group
        .find("<rect class=\"basic label-container\"")
        .expect("its rect")..];
    let rect = &rect[..rect.find("/>").expect("the rect's end")];
    let number = |name: &str| -> f64 { attribute_values(rect, name)[0].parse().unwrap() };
    (number("width"), number("height"))
}

#[test]
fn a_node_gives_the_edges_of_its_busiest_side_room() {
    let renderer = Renderer::new(&Value::empty_object());
    // Four edges meet `E` on one side, three leave `A`, one meets `F`.
    let across = "flowchart LR\n  A[Alpha] --> E[Echo]\n  B[Beta] --> E\n  C[Gamma] --> E\n  D[Delta] --> E\n  A --> F[Foxtrot]\n  A --> G[Golf]\n";
    let plain = renderer.render_with(across, &default(None)).unwrap();
    let designed = renderer.render_with(across, &design(None)).unwrap();
    let (width, height) = node_size(&plain, "E");
    // 20 for each edge after the first, and 24 at both ends of the side.
    assert_eq!(node_size(&designed, "E"), (width, 108.0));
    assert_eq!(node_size(&designed, "A"), (width, 88.0));
    assert_eq!(node_size(&designed, "F"), (width, height));
    assert!(height < 88.0);

    // From top to bottom the side runs along the width, which a node usually has already.
    let down = across.replace("flowchart LR", "flowchart TD");
    let plain = renderer.render_with(&down, &default(None)).unwrap();
    let designed = renderer.render_with(&down, &design(None)).unwrap();
    assert_eq!(node_size(&designed, "E"), node_size(&plain, "E"));
    let many = "flowchart TD\n  A --> Z[Sink]\n  B --> Z\n  C --> Z\n  D --> Z\n  E --> Z\n  F --> Z\n  G --> Z\n  H --> Z\n  I --> Z\n";
    let plain = renderer.render_with(many, &default(None)).unwrap();
    let designed = renderer.render_with(many, &design(None)).unwrap();
    let (width, height) = node_size(&plain, "Z");
    assert!(width < 208.0);
    assert_eq!(node_size(&designed, "Z"), (208.0, height));
}

#[test]
fn the_edges_of_a_side_meet_the_node_around_its_middle() {
    let renderer = Renderer::new(&Value::empty_object());
    // Three edges leave `A`, and four meet `E`.
    let across = "flowchart LR\n  A[Alpha] --> E[Echo]\n  B[Beta] --> E\n  C[Gamma] --> E\n  D[Delta] --> E\n  A --> F[Foxtrot]\n  A --> G[Golf]\n";
    let designed = renderer.render_with(across, &design(None)).unwrap();
    // Where the edges with the given ids start or end, along the layer, in order.
    let places = |svg: &str, ids: &[&str], end: bool, along_y: bool| -> Vec<f64> {
        let mut places: Vec<f64> = ids
            .iter()
            .map(|id| {
                let points = route(svg, id);
                let point = if end {
                    points[points.len() - 1]
                } else {
                    points[0]
                };
                if along_y { point.1 } else { point.0 }
            })
            .collect();
        places.sort_by(f64::total_cmp);
        places
    };
    // Neighbors are 20 apart.
    let assert_bundle = |places: &[f64]| {
        for pair in places.windows(2) {
            assert!((pair[1] - pair[0] - 20.0).abs() < 1e-6, "{places:?}");
        }
    };
    let out = places(&designed, &["L_A_E_0", "L_A_F_0", "L_A_G_0"], false, true);
    assert_bundle(&out);
    let into = places(
        &designed,
        &["L_A_E_0", "L_B_E_0", "L_C_E_0", "L_D_E_0"],
        true,
        true,
    );
    assert_bundle(&into);
    // `A` is 88 high for its three edges: the outer ones are 24 from its corners, not on them.
    assert_eq!(node_size(&designed, "A").1, 88.0);
    assert!((out[2] - out[0] - (88.0 - 2.0 * 24.0)).abs() < 1e-6);

    // From top to bottom the side runs along the width, and a node is wider than its edges need:
    // they still sit 20 apart, around the middle.
    let down = across.replace("flowchart LR", "flowchart TD");
    let designed = renderer.render_with(&down, &design(None)).unwrap();
    let out = places(&designed, &["L_A_E_0", "L_A_F_0", "L_A_G_0"], false, false);
    assert_bundle(&out);
    let (width, _) = node_size(&designed, "A");
    assert!(width > 88.0);
    // The default rendering spreads them over the side.
    let plain = renderer.render_with(&down, &default(None)).unwrap();
    let spread = places(&plain, &["L_A_E_0", "L_A_F_0", "L_A_G_0"], false, false);
    assert!(spread[1] - spread[0] > 20.0 + 1e-6, "{spread:?}");
}

/// The points of the route of the edge with the given id.
fn route(svg: &str, id: &str) -> Vec<(f64, f64)> {
    let edge = &svg[svg.find(&format!("data-id=\"{id}\"")).expect("the edge")..];
    let points = attribute_values(edge, "data-points")[0];
    let json = String::from_utf8(base64_decode(points)).expect("the points are text");
    let json: serde_json::Value = serde_json::from_str(&json).expect("the points are JSON");
    json.as_array()
        .expect("a list of points")
        .iter()
        .map(|point| (point["x"].as_f64().unwrap(), point["y"].as_f64().unwrap()))
        .collect()
}

/// Decodes standard base64 with padding.
fn base64_decode(text: &str) -> Vec<u8> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let (mut bits, mut count) = (0_u32, 0);
    for byte in text.bytes().filter(|byte| *byte != b'=') {
        let value = ALPHABET.iter().position(|c| *c == byte).expect("base64") as u32;
        bits = (bits << 6) | value;
        count += 6;
        if count >= 8 {
            count -= 8;
            out.push((bits >> count) as u8);
            bits &= (1 << count) - 1;
        }
    }
    out
}

#[test]
fn the_mode_places_nodes_so_that_edges_run_straight() {
    let renderer = Renderer::new(&Value::empty_object());
    // Two sources, two middle nodes and a sink: Brandes & Koepf leaves steps that a placement by
    // network simplex does not.
    let text = "flowchart LR\n  A[Alpha] --> D[Delta]\n  B[Beta] --> C[Gamma]\n  A --> C\n  B --> D\n  A -- sync --> E[Echo]\n  B == bulk ==> E\n  C <--> E\n  D --o E\n  D --x F[Foxtrot]\n";
    let straight = |svg: &str, id: &str| {
        let points = route(svg, id);
        points
            .iter()
            .all(|point| (point.1 - points[0].1).abs() < 1e-6)
    };
    let plain = renderer.render_with(text, &default(None)).unwrap();
    let designed = renderer.render_with(text, &design(None)).unwrap();
    let edges = ["L_A_C_0", "L_B_D_0", "L_C_E_0"];
    assert!(edges.iter().any(|edge| !straight(&plain, edge)));
    for edge in edges {
        assert!(straight(&designed, edge), "{edge} bends");
    }
    // A placement strategy the configuration names is the user's: the mode keeps it.
    let config = Value::from(serde_json::json!({
        "elk": { "nodePlacementStrategy": "BRANDES_KOEPF" }
    }));
    let kept = Renderer::new(&config)
        .render_with(text, &design(None))
        .unwrap();
    assert!(edges.iter().any(|edge| !straight(&kept, edge)));
}

/// The middle of the node with the given id.
fn node_middle(svg: &str, id: &str) -> (f64, f64) {
    let group = &svg[svg
        .find(&format!("id=\"d-flowchart-{id}-"))
        .expect("the node")..];
    let marker = "transform=\"translate(";
    let rest = &group[group.find(marker).expect("its place") + marker.len()..];
    let (x, y) = rest[..rest.find(')').expect("the place's end")]
        .split_once(", ")
        .expect("two numbers");
    (x.parse().unwrap(), y.parse().unwrap())
}

/// Whether the route of the edge with the given id has one place across the flow, for a flow
/// from top to bottom.
fn runs_straight_down(svg: &str, id: &str) -> bool {
    let points = route(svg, id);
    points
        .iter()
        .all(|point| (point.0 - points[0].0).abs() < 1e-6)
}

#[test]
fn the_main_flow_stands_on_one_axis_and_a_fan_around_it() {
    let renderer = Renderer::new(&Value::empty_object());
    // A decision with two branches that meet again.
    let text = "flowchart TD\n  S([Start]) --> V{Valid?}\n  V -- yes --> P[Process]\n  V -- no --> R[Reject]\n  P --> M[Merge results]\n  R --> M\n  M --> N[Notify]\n  N --> E([End])\n";
    let designed = renderer.render_with(text, &design(None)).unwrap();
    for edge in ["L_S_V_0", "L_M_N_0", "L_N_E_0"] {
        assert!(runs_straight_down(&designed, edge), "{edge} bends");
    }
    // The decision and the node the branches meet in stand in the middle of the two branches,
    // and on one line with the start and the end.
    let x = |id: &str| node_middle(&designed, id).0;
    let between = (x("P") + x("R")) / 2.0;
    for node in ["S", "V", "M", "N", "E"] {
        assert!((x(node) - between).abs() <= 1.0, "{node} at {}", x(node));
    }
    // ELK's network simplex placement, which a configuration can name, puts the decision over
    // one branch.
    let config = Value::from(serde_json::json!({
        "elk": { "nodePlacementStrategy": "NETWORK_SIMPLEX" }
    }));
    let placed = Renderer::new(&config)
        .render_with(text, &design(None))
        .unwrap();
    let off = (node_middle(&placed, "V").0
        - (node_middle(&placed, "P").0 + node_middle(&placed, "R").0) / 2.0)
        .abs();
    assert!(off > 1.0, "{off}");
}

#[test]
fn a_chain_stays_straight_beside_an_edge_that_skips_it() {
    let renderer = Renderer::new(&Value::empty_object());
    // `A --> D` skips two nodes, and `B --> A` runs against the flow.
    let text = "flowchart TD\n  A[Checkout] --> B[Payment]\n  B --> C[Fulfil]\n  C --> D[Ship]\n  A --> D\n  B -- declined --> A\n  D --> E([Done])\n";
    let designed = renderer.render_with(text, &design(None)).unwrap();
    for edge in ["L_A_B_0", "L_B_C_0", "L_C_D_0", "L_D_E_0"] {
        assert!(runs_straight_down(&designed, edge), "{edge} bends");
    }
    assert!(!runs_straight_down(&designed, "L_A_D_0"));
    let x = |id: &str| node_middle(&designed, id).0;
    for node in ["B", "C", "D", "E"] {
        assert!((x(node) - x("A")).abs() <= 1.0, "{node} at {}", x(node));
    }
}

#[test]
fn an_even_bundle_moves_so_that_its_straight_edge_leaves_from_the_middle() {
    let renderer = Renderer::new(&Value::empty_object());
    // `B` has two edges out: one to `E` below it, and one that runs past `E` to `D`.
    let text = "flowchart TD\n  A --> B\n  A --> C\n  B --> E\n  B --> D\n  C --> D\n  C --> F\n  E --> D\n  F --> D\n";
    let designed = renderer.render_with(text, &design(None)).unwrap();
    let x = |id: &str| node_middle(&designed, id).0;
    for (edge, from, to) in [("L_B_E_0", "B", "E"), ("L_C_F_0", "C", "F")] {
        assert!(runs_straight_down(&designed, edge), "{edge} bends");
        assert!((x(from) - x(to)).abs() <= 1.0);
        // The edge leaves from the middle of its node, and the other edge 20 beside it.
        assert!((route(&designed, edge)[0].0 - x(from)).abs() <= 1.0);
    }
    assert!(((route(&designed, "L_B_D_0")[0].0 - x("B")).abs() - 20.0).abs() <= 1.0);
    // The whole is symmetric: `A` and `D` stand between `B` and `C`.
    let between = (x("B") + x("C")) / 2.0;
    assert!((x("A") - between).abs() <= 1.0);
    assert!((x("D") - between).abs() <= 1.0);
}

#[test]
fn the_mode_sets_a_diagram_with_more_room() {
    let renderer = Renderer::new(&Value::empty_object());
    let plain = renderer.render_with(FLOW, &default(None)).unwrap();
    let designed = renderer.render_with(FLOW, &design(None)).unwrap();
    // A node without a busy side keeps its size.
    assert_eq!(node_size(&plain, "C"), node_size(&designed, "C"));
    // The group pads its members by 32, and by 44 at the top.
    let frame = |svg: &str| -> (f64, f64) {
        let group = &svg[svg.find("<g class=\"cluster ").expect("a group")..];
        let rect = &group[..group.find("/>").expect("the frame's end")];
        let number = |name: &str| -> f64 { attribute_values(rect, name)[0].parse().unwrap() };
        (number("width"), number("height"))
    };
    let ((width, height), (roomy_width, roomy_height)) = (frame(&plain), frame(&designed));
    assert!((roomy_width - (width + 2.0 * (32.0 - 24.0))).abs() < 1e-6);
    assert!((roomy_height - (height + (44.0 - 24.0) + (32.0 - 24.0))).abs() < 1e-6);
}

#[test]
fn the_arrowheads_are_the_modes_own() {
    let renderer = Renderer::new(&Value::empty_object());
    let plain = renderer.render_with(FLOW, &default(None)).unwrap();
    let designed = renderer.render_with(FLOW, &design(None)).unwrap();
    let head = "id=\"d_flowchart-v2-pointEnd-margin\" class=\"marker flowchart-v2\" viewBox=";
    assert!(plain.contains(&format!("{head}\"0 0 11.5 14\"")));
    assert!(designed.contains(&format!("{head}\"0 0 10 9\" refX=\"10\" refY=\"4.5\"")));
    assert!(designed.contains("<path d=\"M 0 0 L 10 4.5 L 0 9 L 2.25 4.5 z\""));
    // The other markers are the reference's.
    let circle = "id=\"d_flowchart-v2-circleEnd-margin\"";
    let definition = |svg: &str| {
        let at = svg.find(circle).expect("the circle marker");
        svg[at..at + svg[at..].find("</marker>").expect("its end")].to_owned()
    };
    assert_eq!(definition(&plain), definition(&designed));
    // A colored edge takes a colored copy of the mode's arrowhead.
    let coloured = format!("{FLOW}  linkStyle 0 stroke:#d33\n");
    let designed = renderer.render_with(&coloured, &design(None)).unwrap();
    let at = designed
        .find("id=\"d_flowchart-v2-pointEnd-margin__d33\"")
        .expect("the colored marker");
    assert!(designed[at..].starts_with(
        "id=\"d_flowchart-v2-pointEnd-margin__d33\" class=\"marker flowchart-v2\" viewBox=\"0 0 10 9\""
    ));
    // A thick edge stays slimmer than its arrowhead.
    assert!(style_sheet(&designed).contains("#d .edge-thickness-thick{stroke-width:3px;}"));
}

#[test]
fn an_edge_label_interrupts_its_edge() {
    let renderer = Renderer::new(&Value::empty_object());
    let plain = renderer.render_with(FLOW, &default(None)).unwrap();
    let designed = renderer.render_with(FLOW, &design(None)).unwrap();
    let background = |svg: &str| {
        let labels = &svg[svg.find("<g class=\"edgeLabels\">").expect("the labels")..];
        let labels = &labels[..labels.find("<g class=\"nodes\">").expect("the nodes")];
        let number = |name: &str| -> f64 { attribute_values(labels, name)[0].parse().unwrap() };
        (
            number("x"),
            number("width"),
            number("height"),
            attribute_values(labels, "rx")
                .first()
                .copied()
                .map(str::to_owned),
        )
    };
    // The label's box is 5 wider on each side than the default's background, with round corners.
    let (x, width, height, radius) = background(&plain);
    let (gap_x, gap_width, gap_height, gap_radius) = background(&designed);
    assert_eq!(radius, None);
    assert_eq!(gap_radius.as_deref(), Some("6"));
    assert_eq!(gap_x, x - 5.0);
    assert_eq!(gap_width, width + 10.0);
    assert_eq!(gap_height, height);
    // The box is not painted: the edges are masked there, with one hole for each of the two labels.
    assert!(
        style_sheet(&designed).contains("#d .edgeLabel rect.background{fill:none;stroke:none;}")
    );
    let mask = &designed[designed
        .find("<mask id=\"d-label-gaps\"")
        .expect("the mask")..];
    let mask = &mask[..mask.find("</mask>").expect("the mask's end")];
    assert_eq!(mask.matches("fill=\"black\"").count(), 2);
    assert_eq!(mask.matches("fill=\"white\"").count(), 1);
    assert!(designed.contains("</mask><g mask=\"url(#d-label-gaps)\"><path "));
    assert!(!plain.contains("<mask"));
    // A diagram without edge labels has no mask.
    let bare = renderer
        .render_with("flowchart TD\n  A --> B\n", &design(None))
        .unwrap();
    assert!(!bare.contains("<mask"));
}

#[test]
fn the_gap_of_a_label_interrupts_no_edge_beside_it() {
    let renderer = Renderer::new(&Value::empty_object());
    // Edges between the same two nodes run 20 apart in the mode. A label of one line fits between
    // them, and the layout moves an edge aside for a higher one.
    let one_line = "flowchart LR\n  A[Draft] --> B{Review}\n  B -->|approved| C[Published]\n  B -->|changes| A\n";
    let three_lines = "flowchart LR\n  A[Draft] --> B{Review}\n  B -->|approved| C[Published]\n  B -->|changes<br/>requested<br/>again| A\n  A -->|second<br/>look| B\n";
    let down = "flowchart TD\n  A[Author] -- submits draft --> R[Reviewer]\n  R -- requests changes --> A\n  R -- approves --> P[Publish]\n";
    for text in [one_line, three_lines, down] {
        let designed = renderer.render_with(text, &design(None)).unwrap();
        let mask = &designed[designed
            .find("<mask id=\"d-label-gaps\"")
            .expect("the mask")..];
        let mask = &mask[..mask.find("</mask>").expect("the mask's end")];
        let number = |rect: &str, name: &str| -> f64 {
            attribute_values(rect, name)[0].parse().expect("a number")
        };
        let gaps: Vec<(f64, f64, f64, f64)> = mask
            .split("<rect")
            .filter(|rect| rect.contains("fill=\"black\""))
            .map(|rect| {
                (
                    number(rect, "x"),
                    number(rect, "y"),
                    number(rect, "width"),
                    number(rect, "height"),
                )
            })
            .collect();
        let edges: Vec<&str> = designed
            .match_indices(" data-id=\"L_")
            .map(|(at, _)| {
                let rest = &designed[at + " data-id=\"".len()..];
                &rest[..rest.find('"').expect("the id's end")]
            })
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        assert!(!gaps.is_empty() && edges.len() >= 3, "{text}");
        for &(x, y, width, height) in &gaps {
            // The routes are orthogonal, so a part of a route crosses the box when their extents
            // overlap.
            let crossing = edges
                .iter()
                .filter(|edge| {
                    route(&designed, edge).windows(2).any(|part| {
                        let (left, right) = (part[0].0.min(part[1].0), part[0].0.max(part[1].0));
                        let (top, bottom) = (part[0].1.min(part[1].1), part[0].1.max(part[1].1));
                        right > x && left < x + width && bottom > y && top < y + height
                    })
                })
                .count();
            assert_eq!(crossing, 1, "the gap at {x}, {y} in {text}");
        }
    }
}

#[test]
fn a_group_title_sits_at_the_left_and_clear_of_the_edges() {
    let renderer = Renderer::new(&Value::empty_object());
    let designed = renderer.render_with(FLOW, &design(None)).unwrap();
    // The title of `Billing` is 16 from the frame's left and 12 below its top.
    assert_eq!(title_x(&designed), frame_x(&designed) + 16.0);
    let frame_y: f64 = {
        let group = &designed[designed.find("<g class=\"cluster ").expect("a group")..];
        attribute_values(group, "y")[0].parse().unwrap()
    };
    let marker = "<g class=\"cluster-label \" transform=\"translate(";
    let title = &designed[designed.find(marker).expect("a title") + marker.len()..];
    let title_y: f64 = title
        [title.find(", ").expect("the y") + 2..title.find(')').expect("the end")]
        .parse()
        .unwrap();
    assert!((title_y - (frame_y + 12.0)).abs() < 1e-6);

    // An edge that enters where the title would sit moves the title past it.
    let crossed = "flowchart TD\n  A[Start] --> B[One]\n  A --> C[Two]\n  subgraph G [A long group title]\n    B\n    C\n  end\n";
    let designed = renderer.render_with(crossed, &design(None)).unwrap();
    assert!(title_x(&designed) > frame_x(&designed) + 16.0);
    // A short title has room before that edge and stays at the left.
    let short = crossed.replace("A long group title", "Group");
    let designed = renderer.render_with(&short, &design(None)).unwrap();
    assert_eq!(title_x(&designed), frame_x(&designed) + 16.0);
}

#[test]
fn an_absent_option_leaves_the_output_unchanged() {
    let renderer = Renderer::new(&Value::empty_object());
    let plain = renderer.render(FLOW).unwrap();
    let off = RenderOptions {
        design: false,
        ..RenderOptions::default()
    };
    assert_eq!(renderer.render_with(FLOW, &off).unwrap(), plain);
    assert!(!plain.contains("data-role"));
    // A render in the mode does not leak into the next one.
    renderer.render_with(FLOW, &design(None)).unwrap();
    assert_eq!(renderer.render(FLOW).unwrap(), plain);
}

/// The width and the height of the outline of the decision with the given id: its `polygon`.
fn decision_size(svg: &str, id: &str) -> (f64, f64) {
    let group = &svg[svg
        .find(&format!("id=\"d-flowchart-{id}-"))
        .expect("the node")..];
    let points = attribute_values(group, "points")[0];
    let corners: Vec<(f64, f64)> = points
        .split(' ')
        .map(|pair| {
            let (x, y) = pair.split_once(',').expect("a pair");
            (x.parse().unwrap(), y.parse().unwrap())
        })
        .collect();
    let extent = |pick: fn(&(f64, f64)) -> f64| {
        let values: Vec<f64> = corners.iter().map(pick).collect();
        values.iter().copied().fold(f64::MIN, f64::max)
            - values.iter().copied().fold(f64::MAX, f64::min)
    };
    (extent(|p| p.0), extent(|p| p.1))
}

/// The middle of the label of the edge with the given id.
fn label_middle(svg: &str, id: &str) -> (f64, f64) {
    let at = svg
        .find(&format!("<g class=\"label\" data-id=\"{id}\""))
        .expect("the label");
    let marker = "<g class=\"edgeLabel\" transform=\"translate(";
    let start = svg[..at].rfind(marker).expect("its place") + marker.len();
    let (x, y) = svg[start..start + svg[start..].find(')').expect("the place's end")]
        .split_once(", ")
        .expect("two numbers");
    (x.parse().unwrap(), y.parse().unwrap())
}

#[test]
fn a_decision_is_a_flat_diamond() {
    let renderer = Renderer::new(&Value::empty_object());
    let text = "flowchart TD\n  A{Valid?} --> B[Process]\n  B --> C{\"Is the customer allowed<br/>to order on credit?\"}\n";
    let plain = renderer.render_with(text, &default(None)).unwrap();
    let designed = renderer.render_with(text, &design(None)).unwrap();
    let (_, box_height) = node_size(&designed, "B");
    for id in ["A", "C"] {
        let (width, height) = decision_size(&designed, id);
        let (square, _) = decision_size(&plain, id);
        // Wider than tall, and far smaller than the reference's square; a label of one line
        // makes it more than twice as wide.
        assert!(width > height, "{id}: {width} by {height}");
        assert!(id != "A" || width > 2.0 * height, "{width} by {height}");
        assert!(height < square * 0.65, "{id}: {height} of {square}");
        assert!(height > box_height, "{id}: {height}");
    }
    // The edge into it and out of it meet its tips.
    let (x, y) = node_middle(&designed, "A");
    let (_, height) = decision_size(&designed, "A");
    let out = route(&designed, "L_A_B_0");
    assert!((out[0].0 - x).abs() < 1e-6 && (out[0].1 - (y + height / 2.0)).abs() < 1e-3);
    let (x, y) = node_middle(&designed, "C");
    let (_, height) = decision_size(&designed, "C");
    let into = route(&designed, "L_B_C_0");
    let end = into[into.len() - 1];
    // The layout puts the port on a whole unit, which a decision of a fractional width is not.
    assert!((end.0 - x).abs() < 0.5 && (end.1 - (y - height / 2.0)).abs() < 0.5);
}

/// A decision with three labeled branches, the middle one on its axis.
const THREE: &str = "flowchart TD\n  A[Order] --> B{Size?}\n  B -->|small| C[Envelope]\n  B -->|medium| D[Box]\n  B -->|large| E[Crate]\n  C --> F[Ship]\n  D --> F\n  E --> F\n";

#[test]
fn a_branch_beside_a_decision_leaves_from_its_side_tip() {
    let renderer = Renderer::new(&Value::empty_object());
    for direction in ["TD", "BT", "LR", "RL"] {
        let text = THREE.replace("TD", direction);
        let svg = renderer.render_with(&text, &design(None)).unwrap();
        let across_y = matches!(direction, "LR" | "RL");
        // Across the flow, and along it.
        let split = |p: (f64, f64)| if across_y { (p.1, p.0) } else { p };
        let middle = split(node_middle(&svg, "B"));
        let (width, height) = decision_size(&svg, "B");
        let half = if across_y { height } else { width } / 2.0;
        for (edge, target, side) in [("L_B_C_0", "C", -1.0), ("L_B_E_0", "E", 1.0)] {
            let points: Vec<(f64, f64)> = route(&svg, edge).into_iter().map(split).collect();
            // Out of the tip, across, one turn, and into the target along the flow.
            assert_eq!(points.len(), 3, "{direction} {edge}: {points:?}");
            assert!((points[0].0 - (middle.0 + side * half)).abs() < 1e-3);
            assert!((points[0].1 - middle.1).abs() < 1e-3);
            assert!((points[1].1 - middle.1).abs() < 1e-3);
            assert!((points[1].0 - points[2].0).abs() < 1e-6);
            let goal = split(node_middle(&svg, target));
            assert!((points[2].0 - goal.0).abs() <= 1.0, "{direction} {edge}");
            // The label sits on the last part, where the edge has a gap for it.
            let label = split(label_middle(&svg, edge));
            assert!((label.0 - points[2].0).abs() <= 2.0, "{direction} {edge}");
            let (low, high) = (points[1].1.min(points[2].1), points[1].1.max(points[2].1));
            assert!(low < label.1 && label.1 < high, "{direction} {edge}");
        }
        // The branch on the axis goes straight on from the tip facing the flow.
        let straight: Vec<(f64, f64)> = route(&svg, "L_B_D_0").into_iter().map(split).collect();
        assert!(straight.iter().all(|p| (p.0 - middle.0).abs() < 1e-6));
        assert!(svg.contains("-label-gaps\""));
    }
}

#[test]
fn a_branch_that_passes_a_layer_keeps_its_route() {
    let renderer = Renderer::new(&Value::empty_object());
    // `Express` stands a layer below `Queue`: its branch passes the layer of `Queue`.
    let text = "flowchart TD\n  A{Route?} -->|fast| B[Express]\n  A -->|slow| C[Queue]\n  C --> D[Batch]\n  D --> E[Deliver]\n  A --> E\n  B --> E\n";
    let svg = renderer.render_with(text, &design(None)).unwrap();
    let points = route(&svg, "L_A_B_0");
    assert!(points.len() > 3, "{points:?}");
    let (x, y) = node_middle(&svg, "A");
    assert!(points[0].1 > y && (points[0].0 - x).abs() < decision_size(&svg, "A").0 / 4.0);
}

#[test]
fn a_shape_with_a_meaning_carries_its_role() {
    let renderer = Renderer::new(&Value::empty_object());
    let svg = renderer.render_with(FLOW, &design(None)).unwrap();
    assert_eq!(
        attribute_values(&svg, "data-role"),
        ["terminal", "decision", "data", "terminal"]
    );
    let sheet = style_sheet(&svg);
    for rule in [
        "#d [data-role=\"terminal\"].node rect,",
        "fill:var(--terminalBkg);stroke:var(--terminalBorder);",
        "#d [data-role=\"decision\"].node .label text{fill:var(--decisionText);",
        "fill:var(--dataBkg);stroke:var(--dataBorder);",
    ] {
        assert!(sheet.contains(rule), "{rule}");
    }
}

#[test]
fn outlines_are_hairlines_and_corners_are_round() {
    let renderer = Renderer::new(&Value::empty_object());
    let svg = renderer.render_with(FLOW, &design(None)).unwrap();
    let sheet = style_sheet(&svg);
    assert!(sheet.contains("#d .edge-thickness-normal{stroke-width:1.5px;}"));
    assert!(sheet.contains("stroke:var(--nodeBorder);stroke-width:1.25px;"));
    assert!(!sheet.contains("stroke-width:2px"));
    // Three rectangle nodes and the frame of the subgraph.
    assert_eq!(attribute_values(nodes(&svg), "rx"), ["8", "8", "8"]);
    assert!(svg.contains("<rect rx=\"12\" ry=\"12\""));
    assert!(svg.contains("<feDropShadow dx=\"0\" dy=\"1\" stdDeviation=\"1.5\""));
}

#[test]
fn a_group_is_neutral_and_takes_no_palette_slot() {
    let renderer = Renderer::new(&Value::empty_object());
    let plain = renderer.render_with(FLOW, &default(None)).unwrap();
    let designed = renderer.render_with(FLOW, &design(None)).unwrap();
    assert!(plain.contains("data-color-id=\"color-0\""));
    assert!(!designed.contains("data-color-id"));
    assert!(!style_sheet(&designed).contains("--borderColor0"));
    assert!(designed.contains("fill=\"#F2F4F8\" stroke=\"#8891A2\""));
    assert!(
        style_sheet(&designed).contains(
            "#d [data-look=\"neo\"].cluster rect{stroke:var(--clusterBorder);filter:none;}"
        )
    );
}

#[test]
fn the_color_scheme_chooses_the_tone() {
    let renderer = Renderer::new(&Value::empty_object());
    let light = renderer
        .render_with(FLOW, &design(Some(ColorScheme::Light)))
        .unwrap();
    let dark = renderer
        .render_with(FLOW, &design(Some(ColorScheme::Dark)))
        .unwrap();
    let auto = renderer
        .render_with(FLOW, &design(Some(ColorScheme::Auto)))
        .unwrap();
    assert!(style_sheet(&light).contains("--mainBkg:#FFFFFF;"));
    assert!(style_sheet(&light).contains("--terminalBkg:#2F54D6;"));
    assert!(style_sheet(&dark).contains("--mainBkg:#1D212A;"));
    assert!(style_sheet(&dark).contains("--terminalBkg:#4263EB;"));
    assert!(style_sheet(&auto).contains("--mainBkg:light-dark(#FFFFFF,#1D212A);"));
    assert!(style_sheet(&auto).contains("--terminalBkg:light-dark(#2F54D6,#4263EB);"));
    // Without a color scheme the render is light.
    assert_eq!(
        style_sheet(&renderer.render_with(FLOW, &design(None)).unwrap()),
        style_sheet(&light)
    );
}

#[test]
fn a_configured_dark_theme_chooses_the_dark_tone() {
    let config = Value::from(serde_json::json!({ "theme": "redux-dark-color" }));
    let renderer = Renderer::new(&config);
    let svg = renderer.render_with(FLOW, &design(None)).unwrap();
    assert!(style_sheet(&svg).contains("--mainBkg:#1D212A;"));
    assert!(style_sheet(&svg).contains("--terminalBkg:#4263EB;"));
    // A directive that sets the theme does the same.
    let text = format!("%%{{init: {{\"theme\": \"redux-dark-color\"}}}}%%\n{FLOW}");
    let directed = Renderer::new(&Value::empty_object())
        .render_with(&text, &design(None))
        .unwrap();
    assert!(style_sheet(&directed).contains("--mainBkg:#1D212A;"));
}

#[test]
fn the_users_theme_variables_and_classes_keep_the_last_word() {
    let config = Value::from(serde_json::json!({
        "themeVariables": { "mainBkg": "#fafafa", "lineColor": "#ff0000" }
    }));
    let renderer = Renderer::new(&config);
    let svg = renderer.render_with(FLOW, &design(None)).unwrap();
    let sheet = style_sheet(&svg);
    assert!(sheet.contains("--mainBkg:#fafafa;"));
    assert!(sheet.contains("--lineColor:#ff0000;"));
    assert!(sheet.contains("--nodeBorder:#7C8698;"));

    let classed = format!("{FLOW}  classDef hot fill:#f96,stroke:#333\n  class A hot\n");
    let svg = Renderer::new(&Value::empty_object())
        .render_with(&classed, &design(None))
        .unwrap();
    let sheet = style_sheet(&svg);
    let role = sheet
        .find("#d [data-role=\"terminal\"].node rect")
        .expect("the role's rule");
    let class = sheet.find("#d .hot rect{").expect("the class's rule");
    assert!(role < class, "the class rule comes after the role's");
    assert!(sheet[class..].starts_with("#d .hot rect{fill:#f96 !important;"));
}

#[test]
fn the_draw_animation_works_in_the_design_mode() {
    let renderer = Renderer::new(&Value::empty_object());
    let options = RenderOptions {
        animation: Some(ferrigura::Animation::default()),
        ..design(None)
    };
    let svg = renderer.render_with(FLOW, &options).unwrap();
    assert!(svg.contains("data-role=\"terminal\""));
    assert!(svg.contains("@keyframes"));
}

#[test]
fn the_mode_excludes_compat_and_other_diagram_types() {
    let renderer = Renderer::new(&Value::empty_object());
    let both = RenderOptions {
        compat: true,
        ..design(None)
    };
    match renderer.render_with(FLOW, &both) {
        Err(Error::Options(message)) => assert!(message.contains("compat"), "{message}"),
        other => panic!("expected an option error, got {other:?}"),
    }
    match renderer.render_with(STATES, &both) {
        Err(Error::Options(message)) => assert!(message.contains("compat"), "{message}"),
        other => panic!("expected an option error, got {other:?}"),
    }
    match renderer.render_with(CLASSES, &both) {
        Err(Error::Options(message)) => assert!(message.contains("compat"), "{message}"),
        other => panic!("expected an option error, got {other:?}"),
    }
    for other in [
        "pie\n \"a\" : 1\n",
        "sequenceDiagram\n  A->>B: hi\n",
        "erDiagram\n  A ||--o{ B : has\n",
    ] {
        match renderer.render_with(other, &design(None)) {
            Err(Error::Options(message)) => assert!(message.contains("design mode"), "{message}"),
            result => panic!("expected an option error, got {result:?}"),
        }
    }
}

/// A state diagram with every role, a labeled transition, two transitions into the end, a
/// composite state, concurrent regions and a note.
const STATES: &str = "stateDiagram-v2\n  state check <<choice>>\n  state split <<fork>>\n  state merge <<join>>\n  [*] --> Draft\n  Draft --> check : submit\n  check --> Published : approve\n  check --> Rejected : reject\n  Published --> split\n  split --> Indexing\n  split --> Mailing\n  Indexing --> merge\n  Mailing --> merge\n  merge --> Live\n  state Live {\n    [*] --> Shown\n    Shown --> Hidden\n    --\n    [*] --> Counted\n  }\n  Live --> [*]\n  Rejected --> [*]\n  note right of Draft : Written by the author\n";

#[test]
fn a_state_diagram_draws_in_the_design_mode() {
    let renderer = Renderer::new(&Value::empty_object());
    let svg = renderer.render_with(STATES, &design(None)).unwrap();
    let default = renderer.render_with(STATES, &default(None)).unwrap();
    assert_ne!(svg, default);
    assert!(svg.contains("class=\"statediagram\""));
    // The roles: the start states, the end states, the choice, the fork and the join.
    let nodes = nodes(&svg);
    assert_eq!(nodes.matches("data-role=\"terminal\"").count(), 3 + 2);
    assert_eq!(nodes.matches("data-role=\"decision\"").count(), 1);
    assert_eq!(nodes.matches("data-role=\"junction\"").count(), 2);
    assert!(!default.contains("data-role="));
    // The mode's own arrowhead under the barb's ids, no palette, and the soft shadow.
    assert!(svg.contains("M 0 0 L 10 4.5 L 0 9 L 2.25 4.5 z"));
    assert!(!svg.contains("data-color-id"));
    assert!(svg.contains("stdDeviation=\"1.5\""));
    let sheet = style_sheet(&svg);
    for rule in [
        "#d .edge-thickness-normal,#d .transition{stroke-width:1.5px;}",
        "#d .statediagram-state rect.basic,#d .statediagram-state rect.title-state{rx:8px;ry:8px;}",
        "#d [data-look].statediagram-cluster rect.inner{fill:none;stroke:none;}",
        "stroke-dasharray:4 4;",
        "#d [data-role=\"junction\"].node path{fill:var(--lineColor);",
        "--terminalBkg:#2F54D6;",
        "--titleColor:#4A5364;",
        "--noteBkgColor:#F6F7F9;",
        "--stateBorder:#7C8698;",
    ] {
        assert!(sheet.contains(rule), "{rule}");
    }
}

#[test]
fn each_transition_into_an_end_state_meets_one_of_its_own() {
    let renderer = Renderer::new(&Value::empty_object());
    let svg = renderer.render_with(STATES, &design(None)).unwrap();
    assert_eq!(
        nodes(&svg).matches("-state-root_end-").count(),
        2,
        "one end state for each of the two transitions"
    );
    let default = renderer.render_with(STATES, &default(None)).unwrap();
    assert_eq!(nodes(&default).matches("-state-root_end-").count(), 1);
}

#[test]
fn a_composite_state_has_its_title_at_the_left_in_a_row_of_its_own() {
    let text = "stateDiagram-v2\n  [*] --> Work\n  state Work {\n    [*] --> Load\n    Load --> Save\n  }\n";
    let svg = Renderer::new(&Value::empty_object())
        .render_with(text, &design(None))
        .unwrap();
    let group = &svg[svg.find("statediagram-cluster").expect("the composite")..];
    let outer = &group[group.find("<rect class=\"outer\"").expect("its frame")..];
    let number = |name: &str| -> f64 { attribute_values(outer, name)[0].parse().unwrap() };
    let (x, y) = (number("x"), number("y"));
    let marker = "<g class=\"cluster-label\" transform=\"translate(";
    let rest = &group[group.find(marker).expect("the title") + marker.len()..];
    let (tx, rest) = rest.split_once(", ").unwrap();
    let ty = &rest[..rest.find(')').unwrap()];
    assert_eq!(tx.parse::<f64>().unwrap() - x, 16.0);
    assert_eq!(ty.parse::<f64>().unwrap() - y, 12.0);
}

#[test]
fn a_state_label_is_centered_in_the_design_mode() {
    let text = "stateDiagram-v2\n  [*] --> Idle\n";
    let renderer = Renderer::new(&Value::empty_object());
    let first_x = |svg: &str| -> f64 {
        let idle = &svg[svg.find("-state-Idle-").expect("the state")..];
        let tspan = &idle[idle.find("<tspan").expect("its text")..];
        attribute_values(tspan, "x")[0].parse().unwrap()
    };
    // The default starts the line at the label's origin; the mode centers it on the node.
    assert_eq!(
        first_x(&renderer.render_with(text, &default(None)).unwrap()),
        0.0
    );
    assert!(first_x(&renderer.render_with(text, &design(None)).unwrap()) < 0.0);
}

#[test]
fn the_state_diagrams_dark_set_and_user_styles_apply() {
    let renderer = Renderer::new(&Value::empty_object());
    let svg = renderer
        .render_with(STATES, &design(Some(ColorScheme::Dark)))
        .unwrap();
    let sheet = style_sheet(&svg);
    assert!(sheet.contains("--terminalBkg:#4263EB;"));
    assert!(sheet.contains("--compositeBackground:#171A22;"));
    let both = renderer
        .render_with(STATES, &design(Some(ColorScheme::Auto)))
        .unwrap();
    assert!(style_sheet(&both).contains("light-dark("));

    let config = Value::from(serde_json::json!({
        "themeVariables": { "transitionColor": "#ff0000", "noteBkgColor": "#ffffcc" }
    }));
    let svg = Renderer::new(&config)
        .render_with(STATES, &design(None))
        .unwrap();
    let sheet = style_sheet(&svg);
    assert!(sheet.contains("--transitionColor:#ff0000;"));
    assert!(sheet.contains("--noteBkgColor:#ffffcc;"));

    let classed = format!("{STATES}  classDef hot fill:#f96\n  class Draft hot\n");
    let svg = renderer.render_with(&classed, &design(None)).unwrap();
    let sheet = style_sheet(&svg);
    let role = sheet.find("[data-role=").expect("a role's rule");
    let class = sheet.find("#d .hot &gt; *{").expect("the class's rule");
    assert!(role < class, "the class rule comes after the roles'");
}

#[test]
fn an_edge_meets_a_group_in_the_middle_of_its_side() {
    // ELK keeps the ports of a group at the start of its side; the mode's placement moves them to
    // the middle, so that the edges into and out of the group run straight down its axis.
    let text = "flowchart TB\n  S((start)) --> G\n  subgraph G [Group]\n    A --> B\n  end\n  G --> E[End]\n";
    let svg = Renderer::new(&Value::empty_object())
        .render_with(text, &design(None))
        .unwrap();
    let frame = &svg[svg.find("<g class=\"cluster ").expect("the group")..];
    let rect = &frame[frame.find("<rect").expect("its frame")..];
    let number = |name: &str| -> f64 { attribute_values(rect, name)[0].parse().unwrap() };
    let middle = number("x") + number("width") / 2.0;
    for id in ["L_S_G_0", "L_G_E_0"] {
        // The circle's outline meets the edge a hair off its axis.
        for (x, _) in route(&svg, id) {
            assert!((x - middle).abs() <= 1e-3, "{id}: {x} against {middle}");
        }
    }
}

/// Two concurrency regions, each with two labeled transitions between the same two states.
const REGIONS: &str = "stateDiagram-v2\n  [*] --> Active\n  state Active {\n    [*] --> NumLockOff\n    NumLockOff --> NumLockOn : EvNumLockPressed\n    NumLockOn --> NumLockOff : EvNumLockPressed\n    --\n    [*] --> CapsLockOff\n    CapsLockOff --> CapsLockOn : EvCapsLockPressed\n    CapsLockOn --> CapsLockOff : EvCapsLockPressed\n  }\n  Active --> [*]\n";

/// The frames of the concurrency regions of a state diagram: their `x`, `y`, width and height.
fn region_frames(svg: &str) -> Vec<(f64, f64, f64, f64)> {
    svg.match_indices("<rect class=\"divider\"")
        .map(|(at, _)| {
            let rect = &svg[at..];
            let rect = &rect[..rect.find("/>").expect("the rect's end")];
            let number = |name: &str| -> f64 { attribute_values(rect, name)[0].parse().unwrap() };
            (number("x"), number("y"), number("width"), number("height"))
        })
        .collect()
}

/// The middle of the state with the given id.
fn state_middle(svg: &str, id: &str) -> (f64, f64) {
    let group = &svg[svg.find(&format!("id=\"d-state-{id}-")).expect("the state")..];
    let marker = "transform=\"translate(";
    let rest = &group[group.find(marker).expect("its place") + marker.len()..];
    let (x, y) = rest[..rest.find(')').expect("the place's end")]
        .split_once(", ")
        .expect("two numbers");
    (x.parse().unwrap(), y.parse().unwrap())
}

#[test]
fn concurrency_regions_stand_side_by_side_around_their_states() {
    let renderer = Renderer::new(&Value::empty_object());
    let svg = renderer.render_with(REGIONS, &design(None)).unwrap();
    let frames = region_frames(&svg);
    assert_eq!(frames.len(), 2);
    // Side by side across the flow, as the default puts them, not one above the other.
    let (left, right) = (frames[0], frames[1]);
    assert_eq!(left.1, right.1);
    assert!(left.0 + left.2 < right.0, "{frames:?}");
    // The states stand in the middle of their region.
    for (frame, states) in [
        (left, ["NumLockOff", "NumLockOn"]),
        (right, ["CapsLockOff", "CapsLockOn"]),
    ] {
        let middle = frame.0 + frame.2 / 2.0;
        for state in states {
            let x = state_middle(&svg, state).0;
            assert!((x - middle).abs() < 1.0, "{state}: {x} against {middle}");
        }
    }
    // Three regions stand in one row too, where the default starts a second row.
    let three = "stateDiagram-v2\n  state Active {\n    [*] --> A\n    --\n    [*] --> B\n    --\n    [*] --> C\n  }\n";
    let rows = |svg: &str| {
        let mut tops: Vec<f64> = region_frames(svg).iter().map(|frame| frame.1).collect();
        tops.sort_by(f64::total_cmp);
        tops.dedup();
        tops.len()
    };
    assert_eq!(
        rows(&renderer.render_with(three, &design(None)).unwrap()),
        1
    );
    assert_eq!(
        rows(&renderer.render_with(three, &default(None)).unwrap()),
        2
    );
}

#[test]
fn regions_whose_states_run_from_left_to_right_stand_one_above_the_other() {
    let text = "stateDiagram-v2\n  state Active {\n    direction LR\n    [*] --> A\n    A --> B\n    --\n    direction LR\n    [*] --> C\n    C --> D\n  }\n";
    let svg = Renderer::new(&Value::empty_object())
        .render_with(text, &design(None))
        .unwrap();
    let mut frames = region_frames(&svg);
    frames.sort_by(|a, b| a.1.total_cmp(&b.1));
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].0, frames[1].0);
    assert!(frames[0].1 + frames[0].3 < frames[1].1, "{frames:?}");
}

#[test]
fn a_branch_beside_a_choice_leaves_from_its_side_tip() {
    let text = "stateDiagram-v2\n  state check <<choice>>\n  [*] --> Received\n  Received --> check\n  check --> Accepted : valid\n  check --> Rejected : invalid\n";
    let svg = Renderer::new(&Value::empty_object())
        .render_with(text, &design(None))
        .unwrap();
    let (x, y) = state_middle(&svg, "check");
    for (edge, target, side) in [("edge2", "Accepted", -1.0), ("edge3", "Rejected", 1.0)] {
        // Out of the tip of a choice 28 wide, across, one turn, and down into the state.
        let points = route(&svg, edge);
        assert_eq!(points.len(), 3, "{edge}: {points:?}");
        assert!((points[0].0 - (x + side * 14.0)).abs() < 1e-3, "{edge}");
        assert!((points[0].1 - y).abs() < 1e-3, "{edge}");
        assert!((points[1].1 - y).abs() < 1e-3, "{edge}");
        assert!((points[1].0 - points[2].0).abs() < 1e-6, "{edge}");
        let goal = state_middle(&svg, target).0;
        assert!((points[2].0 - goal).abs() <= 1.0, "{edge}");
    }
}

#[test]
fn two_labeled_transitions_between_two_states_stand_on_both_sides_of_them() {
    let renderer = Renderer::new(&Value::empty_object());
    // Short labels leave room for both transitions to run straight.
    let text = "stateDiagram-v2\n  a --> b : Start\n  a --> b : Stop\n";
    let svg = renderer.render_with(text, &design(None)).unwrap();
    let middle = state_middle(&svg, "a").0;
    assert_eq!(middle, state_middle(&svg, "b").0);
    let mut lines = Vec::new();
    for edge in ["edge0", "edge1"] {
        let points = route(&svg, edge);
        assert!(
            points
                .iter()
                .all(|point| (point.0 - points[0].0).abs() < 1e-6),
            "{edge} bends: {points:?}"
        );
        lines.push(points[0].0 - middle);
    }
    lines.sort_by(f64::total_cmp);
    assert!((lines[0] + lines[1]).abs() < 1e-6, "{lines:?}");
    assert!(lines[1] > 10.0, "{lines:?}");
    // Long labels: the transitions turn out around the states alike, neither runs straight past
    // the other's label.
    let svg = renderer.render_with(REGIONS, &design(None)).unwrap();
    let middle = state_middle(&svg, "NumLockOff").0;
    let reach = |edge: &str| {
        route(&svg, edge)
            .iter()
            .map(|point| (point.0 - middle).abs())
            .fold(0.0_f64, f64::max)
    };
    assert!((reach("edge2") - reach("edge3")).abs() < 1.0);
}

#[test]
fn a_state_with_descriptions_keeps_its_room_in_the_design_mode() {
    // The rows start 16 from the outline at the left, the box is at least as wide as a plain
    // state (120 plus 8 on each side), 10 lies above the title, and the divider runs from edge to
    // edge.
    let text = "stateDiagram-v2\n  state \"Waiting for input\" as W\n  W : entry / reset timer\n  W : exit / stop timer\n  [*] --> W\n";
    let renderer = Renderer::new(&Value::empty_object());
    let svg = renderer.render_with(text, &design(None)).unwrap();
    let state = &svg[svg.find("-state-W-").expect("the state")..];
    let outline = &state[state
        .find("class=\"outer title-state\"")
        .expect("the outline")..];
    let outline = &outline[..outline.find("/>").expect("the outline's end")];
    let number =
        |part: &str, name: &str| -> f64 { attribute_values(part, name)[0].parse().unwrap() };
    let (x, y, width) = (
        number(outline, "x"),
        number(outline, "y"),
        number(outline, "width"),
    );
    let label = &state[state.find("class=\"label\"").expect("the label")..];
    let place = &label[label.find("translate(").expect("its place") + "translate(".len()..];
    let place = &place[..place.find(')').expect("the place's end")];
    let (label_x, label_y): (f64, f64) = {
        let (a, b) = place.split_once(", ").expect("two numbers");
        (a.parse().unwrap(), b.parse().unwrap())
    };
    assert!(
        (label_x - x - 16.0).abs() < 1e-6,
        "left room: {}",
        label_x - x
    );
    assert!(
        (label_y - y - 10.0).abs() < 1e-6,
        "room above: {}",
        label_y - y
    );
    assert!(width >= 136.0, "width: {width}");
    let divider = &state[state.find("class=\"divider\"").expect("the divider")..];
    assert!((number(divider, "x1") - x).abs() < 1e-6);
    assert!((number(divider, "x2") - (x + width)).abs() < 1e-6);
}

/// A chain of three states with a note on each side.
const NOTES: &str = "stateDiagram-v2\n  [*] --> Pending\n  Pending --> Paid : pay\n  note right of Pending\n    Waits for the payment\n    for at most 7 days.\n  end note\n  Paid --> Shipped : ship\n  note left of Shipped : Tracked by the carrier\n  Shipped --> [*]\n";

/// The box of the state with the given id: its `x`, `y`, width and height.
fn state_box(svg: &str, id: &str) -> (f64, f64, f64, f64) {
    let prefix = format!("id=\"d-state-{id}-");
    let at = svg
        .match_indices(&prefix)
        .map(|(at, _)| at)
        .find(|&at| {
            let rest = &svg[at + prefix.len()..];
            let digits = rest.find('"').expect("the id's end");
            rest[..digits].bytes().all(|byte| byte.is_ascii_digit())
        })
        .expect("the state");
    let group = &svg[at..];
    let (x, y) = state_middle(group, id);
    let rect = &group[group.find("<rect").expect("its outline")..];
    let rect = &rect[..rect.find("/>").expect("the rect's end")];
    let number = |name: &str| -> f64 { attribute_values(rect, name)[0].parse().unwrap() };
    let (width, height) = (number("width"), number("height"));
    (x - width / 2.0, y - height / 2.0, width, height)
}

/// The box of the notes of the state with the given id: the frame of their group.
fn note_box(svg: &str, id: &str) -> (f64, f64, f64, f64) {
    let group = &svg[svg
        .find(&format!(
            "<g class=\"note-cluster\" id=\"d-state-{id}----parent-"
        ))
        .expect("the note's group")..];
    let rect = &group[group.find("<rect").expect("its frame")..];
    let rect = &rect[..rect.find("/>").expect("the rect's end")];
    let number = |name: &str| -> f64 { attribute_values(rect, name)[0].parse().unwrap() };
    (number("x"), number("y"), number("width"), number("height"))
}

#[test]
fn a_note_stands_beside_its_state_and_the_chain_on_one_axis() {
    let svg = Renderer::new(&Value::empty_object())
        .render_with(NOTES, &design(None))
        .unwrap();
    let chain = ["Pending", "Paid", "Shipped"].map(|id| state_box(&svg, id));
    let axis = chain[0].0 + chain[0].2 / 2.0;
    for state in chain {
        assert!((state.0 + state.2 / 2.0 - axis).abs() < 1e-6, "{chain:?}");
    }
    // The transitions of the chain run straight down the axis.
    for edge in ["edge1", "edge3"] {
        for (x, _) in route(&svg, edge) {
            assert!((x - axis).abs() < 1e-6, "{edge} bends");
        }
    }
    // `right of` to the right of its state, `left of` to the left, both at the state's height and
    // clear of every state and transition.
    let (pending, shipped) = (chain[0], chain[2]);
    let right = note_box(&svg, "Pending");
    assert!(right.0 > pending.0 + pending.2 + 8.0, "{right:?}");
    let left = note_box(&svg, "Shipped");
    assert!(left.0 + left.2 < shipped.0 - 8.0, "{left:?}");
    for (note, state) in [(right, pending), (left, shipped)] {
        let middle = note.1 + note.3 / 2.0;
        assert!((middle - (state.1 + state.3 / 2.0)).abs() < 1e-6);
        for other in chain {
            let apart = note.0 + note.2 < other.0
                || other.0 + other.2 < note.0
                || note.1 + note.3 < other.1
                || other.1 + other.3 < note.1;
            assert!(apart, "{note:?} touches {other:?}");
        }
        for edge in ["edge0", "edge1", "edge3", "edge5"] {
            for (x, _) in route(&svg, edge) {
                assert!(x < note.0 || x > note.0 + note.2, "{edge} meets {note:?}");
            }
        }
    }
    // The connector runs straight across from the state to its note.
    let id = attribute_values(&svg, "data-id")
        .into_iter()
        .find(|id| id.starts_with("Pending-Pending----note-"))
        .expect("the connector");
    let connector = route(&svg, id);
    assert_eq!(connector.len(), 2, "{connector:?}");
    assert_eq!(connector[0].1, connector[1].1);
    assert_eq!(connector[0].0, pending.0 + pending.2);
    assert_eq!(connector[1].0, right.0);
    // The default keeps the reference's place: the note stands below its state.
    let plain = Renderer::new(&Value::empty_object())
        .render_with(NOTES, &default(None))
        .unwrap();
    let below = note_box(&plain, "Pending");
    let state = state_box(&plain, "Pending");
    assert!(below.1 > state.1 + state.3, "{below:?}");
}

#[test]
fn a_note_stands_below_or_above_its_state_from_left_to_right() {
    let text = NOTES.replace("stateDiagram-v2\n", "stateDiagram-v2\n  direction LR\n");
    let svg = Renderer::new(&Value::empty_object())
        .render_with(&text, &design(None))
        .unwrap();
    let (pending, shipped) = (state_box(&svg, "Pending"), state_box(&svg, "Shipped"));
    let axis = pending.1 + pending.3 / 2.0;
    assert!((shipped.1 + shipped.3 / 2.0 - axis).abs() < 1e-6);
    // `right of` below, `left of` above, centered on the state.
    let below = note_box(&svg, "Pending");
    assert!(below.1 > pending.1 + pending.3 + 8.0, "{below:?}");
    assert!((below.0 + below.2 / 2.0 - (pending.0 + pending.2 / 2.0)).abs() < 1e-6);
    let above = note_box(&svg, "Shipped");
    assert!(above.1 + above.3 < shipped.1 - 8.0, "{above:?}");
}

#[test]
fn a_composite_state_holds_the_notes_of_its_states() {
    let text = "stateDiagram-v2\n  [*] --> Work\n  state Work {\n    [*] --> Load\n    Load --> Save\n    note right of Load : Reads the input\n  }\n  Work --> [*]\n";
    let svg = Renderer::new(&Value::empty_object())
        .render_with(text, &design(None))
        .unwrap();
    let group = &svg[svg.find("statediagram-cluster").expect("the composite")..];
    let outer = &group[group.find("<rect class=\"outer\"").expect("its frame")..];
    let number = |name: &str| -> f64 { attribute_values(outer, name)[0].parse().unwrap() };
    let frame = (number("x"), number("y"), number("width"), number("height"));
    let note = note_box(&svg, "Load");
    let load = state_box(&svg, "Load");
    assert!(note.0 > load.0 + load.2, "{note:?}");
    assert!(
        note.0 >= frame.0 && note.0 + note.2 <= frame.0 + frame.2,
        "{note:?} {frame:?}"
    );
    assert!(
        note.1 >= frame.1 && note.1 + note.3 <= frame.1 + frame.3,
        "{note:?} {frame:?}"
    );
}

/// A class diagram with an interface, an abstract class, an enumeration, a class without members,
/// an inheritance tree, a composition with cardinalities and a dependency.
const CLASSES: &str = "classDiagram\n  class Shape {\n    <<interface>>\n    +area() double\n  }\n  class Polygon {\n    <<abstract>>\n    #List~Point~ vertices\n    +area()* double\n    +count()$ int\n  }\n  class Kind {\n    <<enumeration>>\n    PLAIN\n    FANCY\n  }\n  class Square\n  Shape <|.. Polygon\n  Polygon <|-- Square\n  Polygon <|-- Triangle\n  Polygon \"1\" *-- \"3..*\" Point : corners\n  Polygon ..> Kind\n";

/// The middle of the class box with the given name.
fn class_middle(svg: &str, name: &str) -> (f64, f64) {
    let group = &svg[svg.find(&format!("-classId-{name}\"")).expect("the class")..];
    let marker = "transform=\"translate(";
    let rest = &group[group.find(marker).expect("its place") + marker.len()..];
    let (x, rest) = rest.split_once(", ").expect("x");
    let y = &rest[..rest.find(')').expect("y")];
    (x.parse().unwrap(), y.parse().unwrap())
}

/// The part of an SVG that draws the class box with the given name, up to the next node.
fn class_box(svg: &str, name: &str) -> String {
    let start = svg.find(&format!("-classId-{name}\"")).expect("the class");
    let rest = &svg[start..];
    let end = rest[1..]
        .find("<g class=\"node ")
        .map_or(rest.len(), |end| end + 1);
    rest[..end].to_owned()
}

#[test]
fn a_class_diagram_draws_in_the_design_mode() {
    let renderer = Renderer::new(&Value::empty_object());
    let svg = renderer.render_with(CLASSES, &design(None)).unwrap();
    let default = renderer.render_with(CLASSES, &default(None)).unwrap();
    assert!(svg.contains("class=\"classDiagram\""));
    // The roles: an interface and an abstract class are one role, an enumeration another; other
    // classes are neutral and take no palette slot.
    let nodes = nodes(&svg);
    assert_eq!(nodes.matches("data-role=\"interface\"").count(), 2);
    assert_eq!(nodes.matches("data-role=\"enumeration\"").count(), 1);
    assert!(!svg.contains("data-color-id"));
    assert!(!default.contains("data-role="));
    assert!(default.contains("data-color-id"));
    // The mode's markers under the reference's ids, and the soft shadow.
    assert!(svg.contains("class=\"design-hollow\""));
    assert!(svg.contains("class=\"design-filled\""));
    assert!(svg.contains("class=\"design-open\""));
    assert!(svg.contains("stdDeviation=\"1.5\""));
    let sheet = style_sheet(&svg);
    for rule in [
        "#d .edge-thickness-normal,#d .relation{stroke-width:1.5px;}",
        "#d .node line.divider{stroke:var(--nodeBorder);stroke-width:1px;}",
        "#d [data-role=\"interface\"].node rect.outline{fill:var(--interfaceBkg);stroke:var(--interfaceBorder);}",
        "#d .annotation-group text,#d .visibility text{fill:var(--titleColor);}",
        "--interfaceBkg:#F3F6FE;",
        "--enumerationBorder:#3F8F79;",
    ] {
        assert!(sheet.contains(rule), "{rule}");
    }
}

#[test]
fn a_class_box_draws_only_the_compartments_it_has() {
    let renderer = Renderer::new(&Value::empty_object());
    let svg = renderer.render_with(CLASSES, &design(None)).unwrap();
    let dividers = |name: &str| {
        class_box(&svg, name)
            .matches("<line class=\"divider\"")
            .count()
    };
    // A class without members is its name alone; an interface with methods only has one
    // compartment below its name; a class with both has two.
    assert_eq!(dividers("Square"), 0);
    assert_eq!(dividers("Shape"), 1);
    assert_eq!(dividers("Polygon"), 2);
    let square = class_box(&svg, "Square");
    let height: f64 = attribute_values(&square, "height")[0].parse().unwrap();
    assert!(height < 50.0, "{height}");
    // The corners are round.
    assert!(square.contains("rx=\"8\""));
    // The default draws two empty compartments.
    let plain = renderer.render_with(CLASSES, &default(None)).unwrap();
    assert_eq!(
        class_box(&plain, "Square")
            .matches("class=\"divider")
            .count(),
        2
    );
}

#[test]
fn a_member_shows_its_visibility_static_and_abstract_marks() {
    let renderer = Renderer::new(&Value::empty_object());
    let svg = renderer.render_with(CLASSES, &design(None)).unwrap();
    let polygon = class_box(&svg, "Polygon");
    // The annotation is a line of its own above the name, and the generic type shows.
    assert!(polygon.contains("class=\"annotation-group text\""));
    assert!(polygon.contains("«abstract»"));
    assert!(polygon.contains(">List&lt;Point&gt;</tspan>"));
    // The marks stand in a column of their own.
    assert_eq!(polygon.matches("<g class=\"visibility\"").count(), 3);
    // An abstract member is set in italics, a static one underlined.
    assert!(polygon.contains("font-style=\"italic\""));
    assert!(polygon.contains("text-decoration:underline"));
    // An enumeration's values have no marks, so no column.
    assert!(!class_box(&svg, "Kind").contains("class=\"visibility\""));
}

#[test]
fn the_general_end_of_a_relation_stands_above() {
    let renderer = Renderer::new(&Value::empty_object());
    let text = "classDiagram\n  Dog --|> Animal\n  Wheel --* Car\n  Car --o Fleet\n";
    let svg = renderer.render_with(text, &design(None)).unwrap();
    assert!(class_middle(&svg, "Animal").1 < class_middle(&svg, "Dog").1);
    assert!(class_middle(&svg, "Car").1 < class_middle(&svg, "Wheel").1);
    assert!(class_middle(&svg, "Fleet").1 < class_middle(&svg, "Car").1);
    // The default lays them out as written.
    let plain = renderer.render_with(text, &default(None)).unwrap();
    assert!(class_middle(&plain, "Animal").1 > class_middle(&plain, "Dog").1);
}

#[test]
fn the_children_of_a_class_share_one_triangle() {
    let renderer = Renderer::new(&Value::empty_object());
    let svg = renderer.render_with(CLASSES, &design(None)).unwrap();
    let square = route(&svg, "id_Polygon_Square_2");
    let triangle = route(&svg, "id_Polygon_Triangle_3");
    // Both start at one point of the parent's side, the triangle's tip, near its middle.
    assert_eq!(square[0], triangle[0]);
    let (x, _) = class_middle(&svg, "Polygon");
    assert!((square[0].0 - x).abs() <= 10.0, "{square:?} {x}");
    let plain = renderer.render_with(CLASSES, &default(None)).unwrap();
    assert_ne!(
        route(&plain, "id_Polygon_Square_2")[0],
        route(&plain, "id_Polygon_Triangle_3")[0]
    );
}

#[test]
fn a_cardinality_stands_beside_its_end() {
    let renderer = Renderer::new(&Value::empty_object());
    let svg = renderer.render_with(CLASSES, &design(None)).unwrap();
    let points = route(&svg, "id_Polygon_Point_4");
    let marker = "<g class=\"edgeTerminals\" transform=\"translate(";
    let centres: Vec<(f64, f64)> = svg
        .match_indices(marker)
        .map(|(at, _)| {
            let rest = &svg[at + marker.len()..];
            let (x, rest) = rest.split_once(", ").unwrap();
            (
                x.parse().unwrap(),
                rest[..rest.find(')').unwrap()].parse().unwrap(),
            )
        })
        .collect();
    assert_eq!(centres.len(), 2);
    for end in [points[0], points[points.len() - 1]] {
        let distance = |p: (f64, f64)| (p.0 - end.0).hypot(p.1 - end.1);
        let (x, y) = centres
            .iter()
            .copied()
            .min_by(|a, b| distance(*a).total_cmp(&distance(*b)))
            .unwrap();
        // Near the end, beside the line and its diamond, not on them.
        assert!((y - end.1).abs() < 20.0, "{end:?} {x},{y}");
        assert!((x - end.0).abs() > 5.5 + 4.0, "{end:?} {x},{y}");
        assert!((x - end.0).abs() < 30.0, "{end:?} {x},{y}");
    }
}

#[test]
fn the_class_diagrams_dark_set_and_user_styles_apply() {
    let renderer = Renderer::new(&Value::empty_object());
    let svg = renderer
        .render_with(CLASSES, &design(Some(ColorScheme::Dark)))
        .unwrap();
    assert!(style_sheet(&svg).contains("--interfaceBkg:#1B2131;"));
    let styled = format!("{CLASSES}  style Square fill:#FFE4E1,stroke:#C0392B\n");
    let svg = renderer.render_with(&styled, &design(None)).unwrap();
    let square = class_box(&svg, "Square");
    assert!(
        square.contains("class=\"outline\"")
            && square.contains("style=\"fill:#FFE4E1;stroke:#C0392B\""),
        "{square}"
    );
}

/// The side of a node a self-loop stands on.
#[derive(Debug, Clone, Copy, PartialEq)]
enum LoopSide {
    Right,
    Left,
    Top,
}

/// Checks the self-loop with the given edge id against the middle `(x, y)` of its node: it leaves
/// and enters `side` `apart` around the middle of the side, stands 32 out, and its label stands
/// beside it, centered on it, clear of the loop. Returns the loop's box and the label's middle.
fn assert_loop_beside(
    svg: &str,
    id: &str,
    middle: (f64, f64),
    side: LoopSide,
    apart: f64,
) -> ((f64, f64, f64, f64), (f64, f64)) {
    let points = route(svg, id);
    assert_eq!(points.len(), 4, "{id}: {points:?}");
    let (first, last) = (points[0], points[3]);
    let (xs, ys): (Vec<f64>, Vec<f64>) = points.iter().copied().unzip();
    let (left, right) = (
        xs.iter().copied().fold(f64::MAX, f64::min),
        xs.iter().copied().fold(f64::MIN, f64::max),
    );
    let (top, bottom) = (
        ys.iter().copied().fold(f64::MAX, f64::min),
        ys.iter().copied().fold(f64::MIN, f64::max),
    );
    let label = label_middle(svg, id);
    match side {
        LoopSide::Right | LoopSide::Left => {
            assert!((first.0 - last.0).abs() < 1e-6, "{id}: {points:?}");
            assert!((last.1 - first.1 - apart).abs() < 1e-6, "{id}: {points:?}");
            assert!(((first.1 + last.1) / 2.0 - middle.1).abs() < 1e-6, "{id}");
            assert!((right - left - 32.0).abs() < 1e-6, "{id}: {points:?}");
            assert!((label.1 - middle.1).abs() < 1e-6, "{id}: {label:?}");
            if side == LoopSide::Right {
                assert!(
                    first.0 > middle.0 && label.0 > right + 8.0,
                    "{id}: {label:?}"
                );
            } else {
                assert!(
                    first.0 < middle.0 && label.0 < left - 8.0,
                    "{id}: {label:?}"
                );
            }
            assert!((label.0 - middle.0).abs() - (right - left) < 200.0, "{id}");
        }
        LoopSide::Top => {
            assert!((first.1 - last.1).abs() < 1e-6, "{id}: {points:?}");
            assert!((last.0 - first.0 - apart).abs() < 1e-6, "{id}: {points:?}");
            assert!(((first.0 + last.0) / 2.0 - middle.0).abs() < 1e-6, "{id}");
            assert!((bottom - top - 32.0).abs() < 1e-6, "{id}: {points:?}");
            assert!(first.1 < middle.1, "{id}: {points:?}");
            assert!(
                (label.1 - (top + bottom) / 2.0).abs() < 1e-6,
                "{id}: {label:?}"
            );
            assert!(label.0 > right + 8.0, "{id}: {label:?}");
        }
    }
    ((left, top, right - left, bottom - top), label)
}

/// Whether two boxes `(x, y, width, height)` share more than an edge.
fn boxes_meet(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3
}

/// A flowchart with a loop on its first node.
const LOOP: &str = "flowchart TD\n  A[Retry] -->|again| A\n  A --> B[Done]\n";

#[test]
fn a_self_loop_stands_on_the_free_side_of_its_node() {
    let renderer = Renderer::new(&Value::empty_object());
    let svg = renderer.render_with(LOOP, &design(None)).unwrap();
    let (x, y) = node_middle(&svg, "A");
    let (width, _) = node_size(&svg, "A");
    let (loop_box, _) = assert_loop_beside(&svg, "L_A_A_0", (x, y), LoopSide::Right, 20.0);
    assert!(
        (loop_box.0 - (x + width / 2.0)).abs() < 1e-6,
        "{loop_box:?}"
    );
    // The edge along the flow stays on the axis, and the loop stays clear of the next node.
    assert!(runs_straight_down(&svg, "L_A_B_0"));
    let (bx, by) = node_middle(&svg, "B");
    let (bw, bh) = node_size(&svg, "B");
    assert!(!boxes_meet(
        loop_box,
        (bx - bw / 2.0, by - bh / 2.0, bw, bh)
    ));
    // From left to right, the loop stands on the top with its label at its right.
    let across = LOOP.replace("flowchart TD", "flowchart LR");
    let svg = renderer.render_with(&across, &design(None)).unwrap();
    assert_loop_beside(&svg, "L_A_A_0", node_middle(&svg, "A"), LoopSide::Top, 20.0);
    // The default keeps ELK's loop.
    let plain = renderer.render_with(LOOP, &default(None)).unwrap();
    let points = route(&plain, "L_A_A_0");
    assert!(points[0].0 < node_middle(&plain, "A").0, "{points:?}");
}

#[test]
fn two_self_loops_stand_on_both_free_sides() {
    let text = "flowchart TD\n  A[Start] --> W[Worker]\n  W -->|retry| W\n  W -->|refresh| W\n  W --> D[Done]\n";
    let svg = Renderer::new(&Value::empty_object())
        .render_with(text, &design(None))
        .unwrap();
    let middle = node_middle(&svg, "W");
    assert_loop_beside(&svg, "L_W_W_0", middle, LoopSide::Right, 20.0);
    assert_loop_beside(&svg, "L_W_W_2", middle, LoopSide::Left, 20.0);
    assert!(runs_straight_down(&svg, "L_A_W_0") && runs_straight_down(&svg, "L_W_D_0"));
}

#[test]
fn a_state_loops_beside_itself_and_beside_its_note() {
    let renderer = Renderer::new(&Value::empty_object());
    let text = "stateDiagram-v2\n  [*] --> Pending\n  Pending --> Pending : retry\n  Pending --> Paid : pay\n  Paid --> [*]\n";
    let svg = renderer.render_with(text, &design(None)).unwrap();
    let pending = state_box(&svg, "Pending");
    let middle = (pending.0 + pending.2 / 2.0, pending.1 + pending.3 / 2.0);
    // The state keeps its height, as high as its neighbor, and the ends of the loop are half its
    // side apart.
    assert_eq!(pending.3, state_box(&svg, "Paid").3);
    let (loop_box, _) = assert_loop_beside(&svg, "edge1", middle, LoopSide::Right, pending.3 / 2.0);
    assert!((loop_box.0 - (pending.0 + pending.2)).abs() < 1e-6);
    // A note on the right puts the loop on the left; the note keeps its place beside the state.
    let noted = text.replace(
        "Paid --> [*]\n",
        "Paid --> [*]\n  note right of Pending : waits for the payment\n",
    );
    let svg = renderer.render_with(&noted, &design(None)).unwrap();
    let pending = state_box(&svg, "Pending");
    let middle = (pending.0 + pending.2 / 2.0, pending.1 + pending.3 / 2.0);
    let (loop_box, _) = assert_loop_beside(&svg, "edge1", middle, LoopSide::Left, pending.3 / 2.0);
    let note = note_box(&svg, "Pending");
    assert!(note.0 > pending.0 + pending.2, "{note:?}");
    assert!(!boxes_meet(loop_box, note));
}

#[test]
fn a_class_loops_beside_itself_with_its_cardinalities_at_its_ends() {
    let text = "classDiagram\n  class Employee {\n    +String name\n    +Employee manager\n  }\n  Department \"1\" o-- \"*\" Employee : staffs\n  Employee \"*\" --> \"0..1\" Employee : reports to\n";
    let svg = Renderer::new(&Value::empty_object())
        .render_with(text, &design(None))
        .unwrap();
    let middle = class_middle(&svg, "Employee");
    let id = "id_Employee_Employee_2";
    let (loop_box, label) = assert_loop_beside(&svg, id, middle, LoopSide::Right, 20.0);
    let points = route(&svg, id);
    let marker = "<g class=\"edgeTerminals\" transform=\"translate(";
    let centres: Vec<(f64, f64)> = svg
        .match_indices(marker)
        .map(|(at, _)| {
            let rest = &svg[at + marker.len()..];
            let (x, rest) = rest.split_once(", ").unwrap();
            (
                x.parse().unwrap(),
                rest[..rest.find(')').unwrap()].parse().unwrap(),
            )
        })
        .collect();
    // The start's cardinality above the loop's upper end, the end's below its lower end, both
    // beside the node and short of the label.
    for (end, above) in [(points[0], true), (points[3], false)] {
        let near = centres.iter().copied().find(|centre| {
            (centre.0 - end.0) > 0.0
                && (centre.0 - end.0) < loop_box.2
                && (centre.1 < end.1) == above
                && (centre.1 - end.1).abs() < 20.0
        });
        assert!(near.is_some(), "{end:?} {centres:?}");
        assert!(near.unwrap().0 < label.0);
    }
}

/// The texts of the inner tspans of an SVG: the words of its labels.
fn words(svg: &str) -> Vec<&str> {
    let marker = "class=\"text-inner-tspan\"";
    svg.match_indices(marker)
        .filter_map(|(at, _)| {
            let rest = &svg[at..];
            let start = rest.find('>')? + 1;
            Some(&rest[start..start + rest[start..].find('<')?])
        })
        .collect()
}

/// The width of the first rectangle of an SVG's nodes.
fn first_node_width(svg: &str) -> f64 {
    let nodes = nodes(svg);
    let rect = &nodes[nodes.find("<rect").expect("a rect")..];
    let marker = " width=\"";
    let rest = &rect[rect.find(marker).expect("a width") + marker.len()..];
    rest[..rest.find('"').expect("the end")]
        .parse()
        .expect("a number")
}

#[test]
fn a_word_stays_whole_in_the_label_of_a_state_and_a_node() {
    let renderer = Renderer::new(&Value::empty_object());
    let word = "WaitingForConfirmation";
    for text in [
        format!("stateDiagram-v2\n  {word} --> Done\n"),
        format!("flowchart TD\n  A[{word}] --> B[Done]\n"),
        format!("flowchart LR\n  A[{word}] --> B[Done]\n"),
    ] {
        let plain = renderer.render_with(&text, &default(None)).unwrap();
        assert!(!words(&plain).contains(&word), "the default breaks it");
        let designed = renderer.render_with(&text, &design(None)).unwrap();
        assert!(words(&designed).contains(&word), "{designed}");
        // The node grows as wide as the word needs, where the default's holds half of it. The word
        // is the only one of its line, so the node's text is as wide as the line and its padding.
        assert!(
            first_node_width(&designed) > first_node_width(&plain),
            "{text}"
        );
    }
}

#[test]
fn a_label_still_wraps_between_words_and_an_absurd_word_breaks() {
    let renderer = Renderer::new(&Value::empty_object());
    let text = "flowchart TD\n  A[Send ConfirmationNotification now]\n";
    let designed = renderer.render_with(text, &design(None)).unwrap();
    assert_eq!(
        words(&designed),
        ["Send", "ConfirmationNotification", "now"]
    );
    let url = "https://example.com/a/very/long/pasted/url/that/goes/on/and/on/forever/and/ever/and/ever/and/ever/and/ever";
    let text = format!("flowchart TD\n  A[{url}]\n");
    let designed = renderer.render_with(&text, &design(None)).unwrap();
    assert!(!words(&designed).contains(&url), "{designed}");
}

#[test]
fn a_word_stays_whole_in_an_edge_label_and_the_default_still_breaks_it() {
    let renderer = Renderer::new(&Value::empty_object());
    let word = "UnbelievablyLongEdgeLabelWordHere";
    let text = format!("flowchart TD\n  A -->|{word}| B\n");
    let designed = renderer.render_with(&text, &design(None)).unwrap();
    assert!(words(&designed).contains(&word), "{designed}");
    let plain = renderer.render_with(&text, &default(None)).unwrap();
    assert!(!words(&plain).contains(&word));
}
