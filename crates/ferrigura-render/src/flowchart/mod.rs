//! The flowchart: a port of the reference's unified flowchart renderer on ELK
//! (`diagrams/flowchart/flowRenderer-v3-unified.ts`, `rendering-util/render.ts`,
//! `layout-algorithms/common/index.ts` and `createGraph.ts`).
//!
//! The pipeline follows the reference step by step: `getData`, the edge render data, the measure
//! step (labels and shapes), the elkjs graph, ELK Layered, the post-processing of the result, and
//! the paint. Each step is a module, so that its output can be compared with the reference's. See
//! `docs-port/flowchart/RENDER-PLAN.md`.

pub mod data;
pub mod style;

use ferrigura_model::flowchart::FlowchartDiagram;
use ferrigura_text::{
    FontFamily, FontStyle, FontWeight, Run, embed_runs, face, measure as measure_text, measure_box,
    text_box,
};

use crate::RenderError;
use crate::a11y;
use crate::context::Context;
use crate::svg::{escape, number, xlink_declaration};
use crate::unified::geom::BBox;
use crate::unified::pipeline::{self, Measured, cover};
use crate::unified::{Config, Section, edges, markers};

pub use crate::unified::pipeline::ElkStages;

/// `getData` and the measure step of the unified renderer.
fn measure(input: &Context<'_, FlowchartDiagram, Section<'_>>) -> Result<Measured, RenderError> {
    let mut data = data::get_data(input.diagram, input.section, input.globals);
    // `flowRenderer-v3-unified.ts` sets the diagram type on the layout data after `getData`.
    data.diagram_type = markers::FLOWCHART.to_owned();
    pipeline::measure(data, &Config::of(input))
}

/// The graphs of the layout step, for the differential comparison.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read or the layout fails.
pub fn elk_stages(
    input: &Context<'_, FlowchartDiagram, Section<'_>>,
) -> Result<ElkStages, RenderError> {
    let mut measured = measure(input)?;
    pipeline::lay_out(&mut measured, &Config::of(input))
}

/// Renders a flowchart as an SVG document. The context's section is the flowchart's, with the elk
/// options and the globals of the view; its theme is the one the colour scheme chooses.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read, a shape is unknown, or the layout fails.
pub fn render(input: &Context<'_, FlowchartDiagram, Section<'_>>) -> Result<String, RenderError> {
    let config = Config::of(input);
    let mut measured = measure(input)?;
    pipeline::lay_out(&mut measured, &config)?;
    let id = input.id;
    let painted = pipeline::paint(&measured, id, &config)?;
    let mut bounds = painted.bounds;

    // `insertTitle`: centred over everything drawn so far, `titleTopMargin` above the origin.
    let sans = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
    let title = &input.diagram.title;
    let mut title_svg = String::new();
    if !title.is_empty() {
        let drawn = bounds.unwrap_or_default();
        let width = measure_text(sans, title, TITLE_FONT_SIZE)?;
        let ink = measure_box(sans, title, TITLE_FONT_SIZE)?;
        let text_box = text_box(sans, TITLE_FONT_SIZE)?;
        let baseline = -(config.section.flowchart.title_top_margin as f64);
        // `text-anchor: middle` at the centre (`utils.ts`, `insertTitle`): the title's box starts half
        // its advance left of the centre, and its glyphs' box is the box `getBBox` gives.
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
            "<text text-anchor=\"middle\" x=\"{}\" y=\"{}\" class=\"flowchartTitleText\">{}</text>",
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

    // `setupViewPortForSVG`: the drawn box with `diagramPadding` around it.
    let padding = config.section.flowchart.diagram_padding as f64;
    let drawn = bounds.unwrap_or_default();
    let (view_width, view_height) = (drawn.width + padding * 2.0, drawn.height + padding * 2.0);
    let size_attributes = if config.section.flowchart.use_max_width {
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
        a11y::root_attributes(id, "flowchart-v2", &diagram.acc_title, &diagram.acc_descr);
    let style =
        style::style_sheet_with_fonts(id, input.theme, input.globals, &diagram.classes, &families);
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\"{xlink} id=\"{id}\" {size_attributes} class=\"flowchart\" viewBox=\"{x} {y} {w} {h}\"{accessible_attributes}>{accessibility}<style>{style}</style><g>{markers}{root}{coloured_markers}</g>{title_svg}{defs}</svg>",
        id = escape(id),
        xlink = xlink_declaration(painted.links),
        x = number(drawn.x - padding),
        y = number(drawn.y - padding),
        w = number(view_width),
        h = number(view_height),
        markers = edges::markers(id),
        root = painted.root,
        coloured_markers = painted.coloured_markers,
        defs = style::look_defs(id, input.theme, input.globals),
    ))
}

/// The font size of the title: `.flowchartTitleText` in the flowchart styles.
const TITLE_FONT_SIZE: f64 = 18.0;
