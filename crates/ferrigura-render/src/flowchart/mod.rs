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

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::flowchart::FlowchartDiagram;
use ferrigura_text::{
    FontFamily, FontStyle, FontWeight, Run, embed_runs, face, measure as measure_text, text_box,
};

use crate::RenderError;
use crate::a11y;
use crate::svg::{escape, number};
use crate::theme::Themes;
use crate::unified::geom::BBox;
use crate::unified::pipeline::{self, Measured, cover};
use crate::unified::{edges, markers};

pub use crate::unified::pipeline::ElkStages;

/// What the flowchart renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct FlowchartInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a FlowchartDiagram,
    /// The effective configuration, with the flowchart's `theme`, `look` and `layout` resolved.
    pub config: &'a MermaidConfig,
    /// The colours and sizes of the theme.
    pub themes: Themes<'a>,
    /// The id of the SVG element. It scopes every selector and prefixes every element id, so it
    /// must be a CSS identifier.
    pub id: &'a str,
}

/// `getData` and the measure step of the unified renderer.
fn measure(input: &FlowchartInput<'_>) -> Result<Measured, RenderError> {
    let mut data = data::get_data(input.diagram, input.config);
    // `flowRenderer-v3-unified.ts` sets the diagram type on the layout data after `getData`.
    data.diagram_type = markers::FLOWCHART.to_owned();
    pipeline::measure(data, input.config, input.themes.primary())
}

/// The graphs of the layout step, for the differential comparison.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read or the layout fails.
pub fn elk_stages(input: &FlowchartInput<'_>) -> Result<ElkStages, RenderError> {
    let mut measured = measure(input)?;
    pipeline::lay_out(&mut measured, input.config)
}

/// Renders a flowchart as an SVG document.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read, a shape is unknown, or the layout fails.
pub fn render(input: &FlowchartInput<'_>) -> Result<String, RenderError> {
    let mut measured = measure(input)?;
    pipeline::lay_out(&mut measured, input.config)?;
    let config = input.config;
    let theme = input.themes.primary();
    let id = input.id;
    let painted = pipeline::paint(&measured, id, config, theme)?;
    let mut bounds = painted.bounds;

    // `insertTitle`: centred over everything drawn so far, `titleTopMargin` above the origin.
    let sans = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
    let title = &input.diagram.title;
    let mut title_svg = String::new();
    if !title.is_empty() {
        let drawn = bounds.unwrap_or_default();
        let width = measure_text(sans, title, TITLE_FONT_SIZE)?;
        let text_box = text_box(sans, TITLE_FONT_SIZE)?;
        let baseline = -(config.flowchart.title_top_margin as f64);
        let x = drawn.x + drawn.width / 2.0 - width / 2.0;
        cover(
            &mut bounds,
            BBox::new(x, baseline - text_box.top, width, text_box.height),
        );
        title_svg = format!(
            "<text text-anchor=\"start\" x=\"{}\" y=\"{}\" class=\"flowchartTitleText\">{}</text>",
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
    let padding = config.flowchart.diagram_padding as f64;
    let drawn = bounds.unwrap_or_default();
    let (view_width, view_height) = (drawn.width + padding * 2.0, drawn.height + padding * 2.0);
    let size_attributes = if config.flowchart.use_max_width {
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
        style::style_sheet_with_fonts(id, input.themes, config, &diagram.classes, &families);
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" id=\"{id}\" {size_attributes} class=\"flowchart\" viewBox=\"{x} {y} {w} {h}\"{accessible_attributes}>{accessibility}<style>{style}</style><g>{markers}{root}{coloured_markers}</g>{title_svg}{defs}</svg>",
        id = escape(id),
        x = number(drawn.x - padding),
        y = number(drawn.y - padding),
        w = number(view_width),
        h = number(view_height),
        markers = edges::markers(id, config),
        root = painted.root,
        coloured_markers = painted.coloured_markers,
        defs = style::look_defs(id, input.themes, config),
    ))
}

/// The font size of the title: `.flowchartTitleText` in the flowchart styles.
const TITLE_FONT_SIZE: f64 = 18.0;
