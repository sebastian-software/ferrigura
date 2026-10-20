//! The state diagram on the unified renderer: a port of `diagrams/state/stateRenderer-v3-unified.ts`.
//! `data` is the state's `getData` (`dataFetcher.ts`), `style` its style sheet (`styles.ts` and
//! `styles.js`). The research is in `docs-port/state/NOTES.md`, section 5.

pub mod data;
pub mod style;

use ferrigura_model::state::{Link, StateDiagram};
use ferrigura_text::{
    FontFamily, FontStyle, FontWeight, Run, embed_runs, face, measure as measure_text, measure_box,
    text_box,
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
    pipeline::measure(data, unified)
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
            "<text text-anchor=\"start\" x=\"{}\" y=\"{}\" class=\"statediagramTitleText\">{}</text>",
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
    let style = style::style_sheet(id, input.theme, input.globals, &diagram.classes, &families);
    let transition = input
        .theme
        .light
        .transition_color
        .text()
        .unwrap_or_default();
    let markers = markers::barb_markers(id, DIAGRAM_TYPE, input.globals.look == "neo", &transition);
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
        defs = crate::flowchart::style::look_defs(id, input.theme, input.globals),
    ))
}

/// The links of the clicked states (`stateRenderer-v3-unified.ts` 80-133): each link wraps the node of
/// its state, if that node is a state and not a composite. The quotes around an address and a tooltip
/// are removed, as the reference removes them.
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
