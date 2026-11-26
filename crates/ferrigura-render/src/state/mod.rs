//! The state diagram on the unified renderer: a port of `diagrams/state/stateRenderer-v3-unified.ts`.
//! `data` is the state's `getData` (`dataFetcher.ts`), `style` its style sheet (`styles.ts` and
//! `styles.js`). See `docs/diagrams/state.md#layout-and-rendering`.

pub mod data;
mod notes;
pub mod style;

use ferrigura_model::state::{Link, StateDiagram};
use ferrigura_text::{
    FontStyle, FontWeight, Run, embed_runs, measure as measure_text, measure_box, text_box,
};

use crate::RenderError;
use crate::a11y;
use crate::context::Context;
use crate::svg::{escape, number, xlink_declaration};
use crate::unified::data::LayoutData;
use crate::unified::geom::BBox;
use crate::unified::markers;
use crate::unified::pipeline::{self, Measured, cover};
use crate::unified::{Config, Section};

/// The diagram type the layout data names: the detector's id, for both headers
/// (`stateDetector-V2.ts`). It names the markers and their ids (`stateRenderer-v3-unified.ts`).
pub const DIAGRAM_TYPE: &str = "stateDiagram";

/// `getData` and the measure step of the unified renderer.
fn measure(
    input: &Context<'_, StateDiagram, Section<'_>>,
    unified: &Config<'_>,
) -> Result<Measured, RenderError> {
    let mut data = data::get_data(input.diagram, input.section, input.globals);
    // `stateRenderer-v3-unified.ts` 59 sets the diagram type on the layout data after `getData`.
    data.diagram_type = DIAGRAM_TYPE.to_owned();
    if input.section.design.is_some() {
        split_end_states(&mut data);
    }
    pipeline::measure(data, unified)
}

/// Gives each transition into an end state an end state of its own, in the design mode
/// (`docs/decisions/0023-design-mode.md`). The end of a scope is one node in the layout data, and
/// every transition that ends the scope would otherwise cross the drawing to meet the others at a
/// circle too small to hold their arrowheads. A final state may be drawn more than once; each copy
/// stands beside the state it ends. The first transition keeps the node, and a copy takes the
/// node's id and DOM id with `-<k>` added, for the k-th transition after the first.
fn split_end_states(data: &mut LayoutData) {
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut copies = Vec::new();
    for edge in &mut data.edges {
        let Some(end) = data
            .nodes
            .iter()
            .find(|node| node.id == edge.end && !node.is_group && node.shape == "stateEnd")
        else {
            continue;
        };
        let count = seen.entry(edge.end.clone()).or_default();
        *count += 1;
        if *count == 1 {
            continue;
        }
        let mut copy = end.clone();
        copy.id = format!("{}-{count}", end.id);
        copy.dom_id = format!("{}-{count}", end.dom_id);
        edge.end.clone_from(&copy.id);
        copies.push(copy);
    }
    data.nodes.extend(copies);
}

/// Lays a diagram out in the design mode, with each note beside its state at the state's height
/// (`notes`). Where the notes of a state cannot stand there, its transitions do not leave it
/// straight or a note would touch anything it does not belong to, the diagram is laid out again
/// with that state's notes where the reference puts them.
fn lay_out_with_notes_beside(
    measured: &mut Measured,
    config: &Config<'_>,
) -> Result<(), RenderError> {
    let original = measured.data.clone();
    let mut kept = std::collections::HashSet::new();
    while let Some(notes) = notes::take_out(&mut measured.data, &kept) {
        let mut failed = None;
        let reserved = notes.reserved();
        let laid =
            pipeline::lay_out_adjusted(measured, config, &reserved, |graph, data, state| {
                notes
                    .put_back(graph, data, state)
                    .map_err(|host| failed = Some(host))
                    .is_ok()
            })?;
        if laid {
            failed = notes.blocked(&measured.data);
            if failed.is_none() {
                return Ok(());
            }
        }
        measured.data.clone_from(&original);
        if !failed.is_some_and(|host| kept.insert(host)) {
            break;
        }
    }
    measured.data = original;
    pipeline::lay_out_for_render(measured, config)
}

/// Renders a state diagram as an SVG document (`stateRenderer-v3-unified.ts`, `draw`): the layout
/// of the unified renderer, the barb markers of the look, the title and the viewport.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read, a shape is unknown, or the layout fails.
pub fn render(input: &Context<'_, StateDiagram, Section<'_>>) -> Result<String, RenderError> {
    let config = Config::of(input);
    let mut measured = measure(input, &config)?;
    apply_links(&mut measured.data, &input.diagram.links);
    if input.section.design.is_some() {
        lay_out_with_notes_beside(&mut measured, &config)?;
    } else {
        pipeline::lay_out_for_render(&mut measured, &config)?;
    }
    let id = input.id;
    let painted = pipeline::paint(&measured, id, &config)?;
    let mut bounds = painted.bounds;

    // `insertTitle`: centered over everything drawn so far, `titleTopMargin` above the origin.
    let sans = input
        .text_fonts()
        .face(FontWeight::Normal, FontStyle::Normal)?;
    let title = &input.diagram.title;
    let mut title_svg = String::new();
    let title_font = sans.css_reference().map_or_else(String::new, |family| {
        format!(" style=\"font-family:{family}\"")
    });
    if !title.is_empty() {
        let drawn = bounds.unwrap_or_default();
        let width = measure_text(&sans, title, TITLE_FONT_SIZE)?;
        let ink = measure_box(&sans, title, TITLE_FONT_SIZE)?;
        let text_box = text_box(&sans, TITLE_FONT_SIZE)?;
        let baseline = -(input.section.state.title_top_margin as f64);
        // The title is start-anchored at x, so its box is the glyphs' box from there.
        let x = drawn.x + drawn.width / 2.0 - width / 2.0;
        cover(
            &mut bounds,
            BBox::new(
                x + ink.left,
                baseline - text_box.top,
                ink.width(),
                text_box.height,
            ),
        );
        title_svg = format!(
            "<text text-anchor=\"start\" x=\"{}\" y=\"{}\" class=\"statediagramTitleText\"{title_font}>{}</text>",
            number(x),
            number(baseline),
            escape(title)
        );
    }

    let mut runs: Vec<Run<'_, '_>> = painted
        .all_labels()
        .flat_map(|label| label.text_runs())
        .collect();
    if !title.is_empty() {
        runs.push(Run {
            text: title,
            face: &sans,
        });
    }
    let families = embed_runs(&runs)?;

    // `setupViewPortForSVG` with the padding of 8 and `state.useMaxWidth` (default true).
    let drawn = bounds.unwrap_or_default();
    let (view_width, view_height) = (
        drawn.width + VIEWPORT_PADDING * 2.0,
        drawn.height + VIEWPORT_PADDING * 2.0,
    );
    let size_attributes = if input.section.state.use_max_width {
        format!(
            "width=\"100%\" style=\"max-width: {}px;\"",
            number(view_width)
        )
    } else {
        format!(
            "height=\"{}\" width=\"{}\"",
            number(view_height),
            number(view_width)
        )
    };
    let diagram = input.diagram;
    let accessibility = a11y::children(id, &diagram.acc_title, &diagram.acc_descr);
    let accessible_attributes =
        a11y::root_attributes(id, "stateDiagram", &diagram.acc_title, &diagram.acc_descr);
    let design = input.section.design;
    let style = style::style_sheet(
        id,
        input.theme,
        input.globals,
        &diagram.classes,
        &families,
        design,
    );
    let transition = input
        .theme
        .light
        .transition_color
        .text()
        .unwrap_or_default();
    let neo = input.globals.look == "neo";
    let markers = if design.is_some() {
        markers::design_barb_markers(id, DIAGRAM_TYPE, neo)
    } else {
        markers::barb_markers(id, DIAGRAM_TYPE, neo, &transition)
    };
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\"{xlink} id=\"{id}\" {size_attributes} class=\"statediagram\" viewBox=\"{x} {y} {w} {h}\"{accessible_attributes}>{accessibility}<style>{style}</style><g>{markers}{root}{coloured_markers}</g>{title_svg}{defs}</svg>",
        id = escape(id),
        xlink = xlink_declaration(painted.links),
        x = number(drawn.x - VIEWPORT_PADDING),
        y = number(drawn.y - VIEWPORT_PADDING),
        w = number(view_width),
        h = number(view_height),
        root = painted.root,
        coloured_markers = painted.coloured_markers,
        // `insertLookDefs` is shared with the flowchart: the drop shadows and the gradient.
        // The design mode draws its own shadows under the same ids.
        defs = crate::flowchart::style::look_defs_for(id, input.theme, input.globals, design),
    ))
}

/// The links of the clicked states (`stateRenderer-v3-unified.ts` 80-133): each link wraps the node of
/// its state, if that node is a state and not a composite. The quotes around an address and a tooltip
/// are removed, as the reference removes them; entity placeholder resolution and URL validation run
/// once at the SVG anchor sink in `shapes::node_svg_with_security`.
fn apply_links(data: &mut LayoutData, links: &[Link]) {
    for link in links {
        let Some(node) = data.nodes.iter_mut().find(|node| node.id == link.id) else {
            continue;
        };
        if node.is_group {
            continue;
        }
        node.link = Some(unquote(&link.url));
        let tooltip = unquote(&link.tooltip);
        node.tooltip = (!tooltip.is_empty()).then_some(tooltip);
    }
}

/// The text without the quotes that surround it, as `replace(/^"+|"+$/g, '')` leaves it.
fn unquote(text: &str) -> String {
    text.trim_matches('"').to_owned()
}

/// The font size of the title: `.statediagramTitleText` in the state styles.
const TITLE_FONT_SIZE: f64 = 18.0;

/// The padding of the viewport: `setupViewPortForSVG(svg, 8, ...)` (`stateRenderer-v3-unified.ts` 77).
const VIEWPORT_PADDING: f64 = 8.0;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unified::data::{Edge, Node};

    fn node(id: &str, shape: &str) -> Node {
        Node {
            id: id.to_owned(),
            dom_id: format!("state-{id}-3"),
            shape: shape.to_owned(),
            ..Node::default()
        }
    }

    fn edge(start: &str, end: &str) -> Edge {
        Edge {
            id: format!("{start}-{end}"),
            start: start.to_owned(),
            end: end.to_owned(),
            ..Edge::default()
        }
    }

    #[test]
    fn each_transition_into_an_end_state_gets_one_of_its_own() {
        let mut data = LayoutData {
            nodes: vec![
                node("A", "rect"),
                node("B", "rect"),
                node("C", "rect"),
                node("root_end", "stateEnd"),
            ],
            edges: vec![
                edge("A", "root_end"),
                edge("A", "B"),
                edge("B", "root_end"),
                edge("C", "root_end"),
            ],
            ..LayoutData::default()
        };
        split_end_states(&mut data);
        let ends: Vec<&str> = data.edges.iter().map(|edge| edge.end.as_str()).collect();
        assert_eq!(ends, ["root_end", "B", "root_end-2", "root_end-3"]);
        let copies: Vec<(&str, &str)> = data.nodes[4..]
            .iter()
            .map(|node| (node.id.as_str(), node.dom_id.as_str()))
            .collect();
        assert_eq!(
            copies,
            [
                ("root_end-2", "state-root_end-3-2"),
                ("root_end-3", "state-root_end-3-3")
            ]
        );
        assert!(data.nodes[4..].iter().all(|node| node.shape == "stateEnd"));
    }

    #[test]
    fn apply_links_unquotes_urls_and_tooltips_for_leaf_states() {
        let mut data = LayoutData {
            nodes: vec![node("s1", "rect"), node("s2", "rect")],
            ..LayoutData::default()
        };
        let links = vec![
            Link {
                id: "s1".to_owned(),
                url: "\"javascript:alert(1)\"".to_owned(),
                tooltip: "\"tip\"".to_owned(),
            },
            Link {
                id: "s2".to_owned(),
                url: "\"https://example.com/page\"".to_owned(),
                tooltip: String::new(),
            },
        ];
        apply_links(&mut data, &links);
        assert_eq!(data.nodes[0].link.as_deref(), Some("javascript:alert(1)"));
        assert_eq!(data.nodes[0].tooltip.as_deref(), Some("tip"));
        assert_eq!(
            data.nodes[1].link.as_deref(),
            Some("https://example.com/page")
        );
    }
}
