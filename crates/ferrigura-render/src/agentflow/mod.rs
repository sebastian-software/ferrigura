//! The agentflow diagram: a port of `packages/mermaid/src/diagrams/agentflow/renderer.ts`, which runs the
//! unified renderer on ELK Layered with the agentflow `getData`, the `flowGroup` containers, the palette
//! of the colour themes and its own style sheet (`docs-port/agentflow/NOTES.md`, section 6).
//!
//! The steps follow `draw`: `getData` ([`data`]), the measure step, the elkjs graph and ELK Layered, the
//! paint of `paintLayoutData`, then the root element: the markers, the two drop-shadow filters and the
//! title, and `setupViewPortForSVG`.

pub mod colours;
pub mod data;
pub mod style;

use ferrigura_model::agentflow::AgentflowDiagram;
use ferrigura_model::config::AgentflowDiagramConfig;
use ferrigura_text::{
    FontFamily, FontStyle, FontWeight, Run, embed_runs, face, measure as measure_text, measure_box,
    text_box,
};

use crate::RenderError;
use crate::a11y;
use crate::context::Context;
use crate::flowchart::style::look_defs;
use crate::svg::{escape, number, xlink_declaration};
use crate::unified::geom::BBox;
use crate::unified::pipeline::{self, cover};
use crate::unified::{Config, Section, markers};

/// The configuration that agentflow reads: its own section, and the sections of the unified renderer
/// that its layout and its shapes read (`flowchart`, `elk`, and `state` for `arrowMarkerAbsolute`).
#[derive(Debug, Clone, Copy)]
pub struct AgentflowSection<'a> {
    /// The `agentflow` section.
    pub agentflow: &'a AgentflowDiagramConfig,
    /// The sections of the unified renderer.
    pub unified: Section<'a>,
}

/// The font size of the title: `.agentflowTitleText` in the agentflow styles.
const TITLE_FONT_SIZE: f64 = 18.0;

/// Renders an agentflow diagram as an SVG document. The context's section is the agentflow section, and
/// its theme is the one the colour scheme chooses.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read, a shape is unknown, or the layout fails.
pub fn render(
    input: &Context<'_, AgentflowDiagram, AgentflowSection<'_>>,
) -> Result<String, RenderError> {
    let config = Config {
        section: input.section.unified,
        globals: input.globals,
        theme: input.theme.light,
    };
    // `getData` takes the palette's length from the colours of the theme (`assignColorSlots`).
    let palette_len = input.theme.light.border_color_array.texts().len();
    let data = data::get_data(input.diagram, input.section, input.globals, palette_len);
    let mut measured = pipeline::measure(data, &config)?;
    pipeline::lay_out(&mut measured, &config)?;
    let id = input.id;
    let painted = pipeline::paint(&measured, id, &config)?;
    let mut bounds = painted.bounds;

    // `utils.insertTitle`: centred over everything drawn so far, `titleTopMargin` above the origin.
    let sans = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
    let title = &input.diagram.title;
    let mut title_svg = String::new();
    if !title.is_empty() {
        let drawn = bounds.unwrap_or_default();
        let width = measure_text(sans, title, TITLE_FONT_SIZE)?;
        let ink = measure_box(sans, title, TITLE_FONT_SIZE)?;
        let text_box = text_box(sans, TITLE_FONT_SIZE)?;
        let baseline = -(input.section.agentflow.title_top_margin as f64);
        // `text-anchor: middle` at the centre: the title's box starts half its advance left of the
        // centre, and its glyphs' box is the box `getBBox` gives.
        let x = drawn.x + drawn.width / 2.0;
        cover(
            &mut bounds,
            BBox::new(
                x - width / 2.0 + ink.left,
                baseline - text_box.top,
                ink.width(),
                text_box.height,
            ),
        );
        title_svg = format!(
            "<text text-anchor=\"middle\" x=\"{}\" y=\"{}\" class=\"agentflowTitleText\">{}</text>",
            number(x),
            number(baseline),
            escape(title)
        );
    }

    let mut runs: Vec<Run<'_>> = painted
        .all_labels()
        .flat_map(|label| label.text_runs())
        .collect();
    if !title.is_empty() {
        runs.push(Run {
            text: title,
            face: sans,
        });
    }
    let families = embed_runs(&runs)?;

    // `setupViewPortForSVG` with `diagramPadding` around the drawn box.
    let padding = input.section.agentflow.diagram_padding as f64;
    let drawn = bounds.unwrap_or_default();
    let (view_width, view_height) = (drawn.width + padding * 2.0, drawn.height + padding * 2.0);
    let size_attributes = if input.section.agentflow.use_max_width {
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
        a11y::root_attributes(id, "agentflow", &diagram.acc_title, &diagram.acc_descr);
    let style =
        style::style_sheet_with_fonts(id, input.theme, input.globals, &diagram.classes, &families);
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\"{xlink} id=\"{id}\" {size_attributes} class=\"agentflow\" viewBox=\"{x} {y} {w} {h}\"{accessible_attributes}>{accessibility}<style>{style}</style><g>{markers}{root}{coloured_markers}</g>{defs}{title_svg}</svg>",
        id = escape(id),
        xlink = xlink_declaration(painted.links),
        x = number(drawn.x - padding),
        y = number(drawn.y - padding),
        w = number(view_width),
        h = number(view_height),
        markers = marker_definitions(id),
        root = painted.root,
        coloured_markers = painted.coloured_markers,
        defs = look_defs(id, input.theme, input.globals),
    ))
}

/// The markers of an agentflow diagram, as `insertMarkers(element, ['point', 'circle', 'cross',
/// 'hierarchy'], 'agentflow', id)` writes them (`markers.js`): the flowchart's twelve, with the diagram's
/// own name in their ids, and the two hierarchy markers.
fn marker_definitions(svg_id: &str) -> String {
    let mut out = markers::markers(svg_id, data::DIAGRAM_TYPE);
    out.push_str(&hierarchy_markers(svg_id));
    out
}

/// The `hierarchy` markers (`markers.js` 959-996): an end and a start, each a double chevron with no fill.
fn hierarchy_markers(svg_id: &str) -> String {
    let marker = |name: &str, view_box: &str, ref_x: &str, d: &str| {
        format!(
            "<marker id=\"{svg_id}_{}-{name}\" class=\"marker hierarchy {}\" viewBox=\"{view_box}\" refX=\"{ref_x}\" refY=\"5\" markerUnits=\"userSpaceOnUse\" markerWidth=\"12\" markerHeight=\"10\" orient=\"auto\"><path d=\"{d}\" class=\"arrowMarkerPath\" style=\"stroke-width: 1; stroke-dasharray: 1, 0; fill: none;\"></path></marker>",
            data::DIAGRAM_TYPE,
            data::DIAGRAM_TYPE,
        )
    };
    format!(
        "{}{}",
        marker(
            "hierarchyEnd",
            "0 0 12 10",
            "10",
            "M 0 0 L 6 5 L 0 10 M 4 0 L 10 5 L 4 10"
        ),
        marker(
            "hierarchyStart",
            "0 0 12 10",
            "2",
            "M 12 0 L 6 5 L 12 10 M 8 0 L 2 5 L 8 10"
        ),
    )
}
