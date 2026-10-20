//! The swimlane diagram (`swimlane-beta`): a flowchart whose top-level subgraphs are lanes. The parser, the
//! DB and the unified renderer are the flowchart's; the layout is the swimlane engine's
//! (`ferrigura-swimlane`), which draws the lanes and routes the edges orthogonally.
//!
//! The reference draws it with `flowRenderer-v3-unified.ts` and the layout's `render` (`render.ts`): the
//! `layout` option names the engine (`getRegisteredLayoutAlgorithm`). The `swimlane` engine is the default
//! (`swimlane.layout`). A diagram whose `layout` names ELK's layered algorithm, or any name that is not
//! registered, is laid out by ELK, as a flowchart is, with no lanes (`Engine::Elk`).
//!
//! The steps follow the reference: `getData`, the preparation of the lanes and the label nodes, the measure,
//! the layout, the paint with the lanes and the clipped edges, the title, and the root `svg`
//! ([`swimlane_layout`], [`crate::unified::swimlane_lane`]).

pub mod style;

use ferrigura_model::config::SwimlaneDiagramConfig;
use ferrigura_model::swimlanes::SwimlanesDiagram;
use ferrigura_text::{
    FontFamily, FontStyle, FontWeight, Run, embed_runs, face, measure as measure_text, measure_box,
    text_box,
};

use crate::RenderError;
use crate::a11y;
use crate::context::Context;
use crate::flowchart;
use crate::svg::{escape, number, xlink_declaration};
use crate::unified::geom::BBox;
use crate::unified::pipeline::{self, Layout, cover};
use crate::unified::{Config, Section, markers, swimlane_layout};

/// The configuration a swimlane diagram reads: its own `swimlane` section, for the layout's options and the
/// line hops, and the sections of the unified renderer, which the lanes' flowchart shares.
#[derive(Debug, Clone, Copy)]
pub struct SwimlaneSection<'a> {
    /// The `swimlane` section.
    pub swimlane: &'a SwimlaneDiagramConfig,
    /// The sections of the unified renderer (`flowchart`, `elk`, ...).
    pub unified: Section<'a>,
}

/// The engine that lays a diagram out, as the configuration's `layout` names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Engine {
    /// The swimlane engine: the lanes, and the orthogonal edges.
    Swimlane,
    /// ELK Layered, as a flowchart is laid out.
    Elk,
}

/// The engine of a `layout` name, as `getRegisteredLayoutAlgorithm` resolves it (`render.ts`).
///
/// # Errors
///
/// Returns [`RenderError::Unsupported`] for a registered layout that the port does not implement:
/// `cose-bilkent`, and the ELK algorithms other than the layered one.
fn engine_of(layout: &str) -> Result<Engine, RenderError> {
    match layout {
        "swimlane" => Ok(Engine::Swimlane),
        "cose-bilkent" | "elk.stress" | "elk.force" | "elk.mrtree" | "elk.sporeOverlap"
        | "elk.box" | "elk.rectpacking" => Err(RenderError::Unsupported(format!(
            "the layout {layout} is not ported for swimlane diagrams"
        ))),
        // `elk`, and any name that is not registered, which falls back to `elk` (`elk.layered` is
        // not in the registry, so it falls back too).
        _ => Ok(Engine::Elk),
    }
}

/// The font size of the title: `.flowchartTitleText` in the flowchart styles.
const TITLE_FONT_SIZE: f64 = 18.0;

/// Renders a swimlane diagram as an SVG document. The context's section is the swimlane section, and its
/// theme is the one the colour scheme chooses.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read, a shape is unknown, the layout fails, or the layout
/// is one the port does not implement (`engine_of`).
pub fn render(
    input: &Context<'_, SwimlanesDiagram, SwimlaneSection<'_>>,
) -> Result<String, RenderError> {
    let engine = engine_of(&input.globals.layout)?;
    let config = Config {
        section: input.section.unified,
        globals: input.globals,
        theme: input.theme.light,
    };
    // `getData`, with the flowchart's DB (`flowDb.ts`), which the swimlane diagram shares.
    let mut data = flowchart::data::get_data(input.diagram, &input.section.unified, input.globals);
    // `flowRenderer-v3-unified.ts` gives the layout data the diagram's type: the markers and the edges name it.
    data.diagram_type = markers::SWIMLANE.to_owned();

    let measured = match engine {
        Engine::Swimlane => {
            // `prepareLayoutForSwimlanes` and `createEdgeLabelNodes` run before the measure, so the label
            // nodes are measured with the rest (`common/index.ts`, `index.ts`).
            let prepared = swimlane_layout::prepare(&mut data, &config);
            let layout = Layout::Swimlane(swimlane_layout::line_hops(input.section.swimlane));
            let mut measured = pipeline::measure_for(data, &config, layout)?;
            swimlane_layout::lay_out(&mut measured, &config, &prepared, input.section.swimlane)?;
            measured
        }
        Engine::Elk => {
            let mut measured = pipeline::measure(data, &config)?;
            pipeline::lay_out(&mut measured, &config)?;
            measured
        }
    };

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
        // `text-anchor: middle` at the centre (`utils.ts`, `insertTitle`), as the flowchart's title.
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

    // `setupViewPortForSVG`: the drawn box with `diagramPadding` around it, as the flowchart's.
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
        a11y::root_attributes(id, "swimlane", &diagram.acc_title, &diagram.acc_descr);
    let style = style::style_sheet(id, input.theme, input.globals, &diagram.classes, &families);
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\"{xlink} id=\"{id}\" {size_attributes} class=\"flowchart\" viewBox=\"{x} {y} {w} {h}\"{accessible_attributes}>{accessibility}<style>{style}</style><g>{markers}{root}{coloured_markers}</g>{title_svg}{defs}</svg>",
        id = escape(id),
        xlink = xlink_declaration(painted.links),
        x = number(drawn.x - padding),
        y = number(drawn.y - padding),
        w = number(view_width),
        h = number(view_height),
        markers = markers::markers(id, markers::SWIMLANE),
        root = painted.root,
        coloured_markers = painted.coloured_markers,
        defs = flowchart::style::look_defs(id, input.theme, input.globals),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_swimlane_layout_is_the_swimlane_engine() {
        assert_eq!(engine_of("swimlane").ok(), Some(Engine::Swimlane));
    }

    #[test]
    fn elk_and_an_unregistered_layout_are_laid_out_by_elk() {
        assert_eq!(engine_of("elk").ok(), Some(Engine::Elk));
        assert_eq!(engine_of("elk.layered").ok(), Some(Engine::Elk));
        assert_eq!(engine_of("dagre").ok(), Some(Engine::Elk));
        assert_eq!(engine_of("").ok(), Some(Engine::Elk));
    }

    #[test]
    fn a_registered_layout_the_port_does_not_implement_is_refused() {
        for layout in ["cose-bilkent", "elk.box", "elk.stress"] {
            assert!(
                matches!(engine_of(layout), Err(RenderError::Unsupported(_))),
                "{layout} must not be drawn with another engine"
            );
        }
    }
}
