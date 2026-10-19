//! The ER diagram: a port of the unified renderer with the ER `getData` (`diagrams/er/erRenderer-unified.ts`, `erDb.ts`). The research is in
//! `docs-port/er/NOTES.md`, section 5.
//!
//! The pipeline is the flowchart's: `getData` builds the layout data, the measure step sizes the
//! entities and the edge labels, ELK lays the graph out, and the paint writes the clusters, the
//! edges and the entities. The root `<svg>` carries the cardinality markers, the style sheet, the
//! title and the accessibility elements (`erRenderer-unified.ts` 10-79, `setupViewPortForSVG` with
//! the padding of 8).

pub mod data;
pub mod style;

use crate::context::Globals;
use crate::unified::{Bridge, Config};
use ferrigura_model::config::MermaidConfig;
use ferrigura_model::er::ErDiagram;
use ferrigura_text::{
    FontFamily, FontStyle, FontWeight, Run, embed_runs, face, measure as measure_text, measure_box,
    text_box,
};

use crate::RenderError;
use crate::a11y;
use crate::flowchart::style::look_defs;
use crate::svg::{escape, number};
use crate::theme::Themes;
use crate::unified::geom::BBox;
use crate::unified::markers;
use crate::unified::pipeline::{self, Measured, cover};
use crate::variables::{px, text_of};

/// What the ER renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct ErInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a ErDiagram,
    /// The effective configuration, in the ER diagram's scope.
    pub config: &'a MermaidConfig,
    /// The colours and sizes of the theme.
    pub themes: Themes<'a>,
    /// The id of the SVG element. It scopes every selector and prefixes every element id, so it
    /// must be a CSS identifier.
    pub id: &'a str,
}

/// The padding of the viewBox: `setupViewPortForSVG(svg, 8, 'erDiagram', …)` (`erRenderer-unified.ts`
/// 78). The ER diagram does not use `diagramPadding` for its viewBox.
const VIEWBOX_PADDING: f64 = 8.0;

/// The font size of the title and of the labels that the theme does not size: the root of the
/// reference's SVG sets `font-size` from the theme (`styles.ts` `svg`), and the title inherits it.
const DEFAULT_FONT_SIZE: f64 = 16.0;

/// `getData` and the measure step of the unified renderer (`erRenderer-unified.ts` 17-28).
fn measure(input: &ErInput<'_>, unified: &Config<'_>) -> Result<Measured, RenderError> {
    let mut data = data::get_data(input.diagram, input.config);
    // `erRenderer-unified.ts` 22: the diagram type of the layout data, which names the markers.
    data.diagram_type = markers::ER.to_owned();
    pipeline::measure(data, unified)
}

/// Renders the diagram to an SVG document.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read, a shape is unknown, or the layout fails.
pub fn render(input: &ErInput<'_>) -> Result<String, RenderError> {
    let theme = input.themes.primary();
    let bridge = Bridge::new(input.config, input.themes);
    let unified = bridge.config(input.config);
    let mut measured = measure(input, &unified)?;
    pipeline::lay_out(&mut measured, &unified)?;
    let config = input.config;
    let id = input.id;
    let painted = pipeline::paint(&measured, id, &unified)?;
    let mut bounds = painted.bounds;

    // `utils.insertTitle` (`erRenderer-unified.ts` 71-76): centred over the drawing, at
    // `-titleTopMargin`, in the theme's font size.
    let sans = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
    let title = &input.diagram.title;
    let font_size = px(theme, "fontSize").unwrap_or(DEFAULT_FONT_SIZE);
    let mut title_svg = String::new();
    if !title.is_empty() {
        let drawn = bounds.unwrap_or_default();
        let width = measure_text(sans, title, font_size)?;
        let ink = measure_box(sans, title, font_size)?;
        let text_box = text_box(sans, font_size)?;
        let baseline = -(config.er.title_top_margin as f64);
        let centre = drawn.x + drawn.width / 2.0;
        cover(
            &mut bounds,
            BBox::new(
                centre - width / 2.0 + ink.left,
                baseline - text_box.top,
                ink.width(),
                text_box.height,
            ),
        );
        title_svg = format!(
            "<text text-anchor=\"middle\" x=\"{}\" y=\"{}\" class=\"erDiagramTitleText\">{}</text>",
            number(centre),
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

    // `setupViewPortForSVG`: the box with the padding of 8 around it, which includes the title.
    let drawn = bounds.unwrap_or_default();
    let (view_width, view_height) = (
        drawn.width + VIEWBOX_PADDING * 2.0,
        drawn.height + VIEWBOX_PADDING * 2.0,
    );
    let size_attributes = if config.er.use_max_width {
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
        a11y::root_attributes(id, "er", &diagram.acc_title, &diagram.acc_descr);
    let style = style::style_sheet(id, input.themes, config, &families);
    // `insertMarkers` with `data4Layout.markers` (`erRenderer-unified.ts` 32-41): the `neo` set for
    // the neo look, else the classic one.
    let stroke_width = text_of(theme, "strokeWidth").unwrap_or_else(|| "1".to_owned());
    let background = text_of(theme, "mainBkg").unwrap_or_default();
    let marker_defs = markers::er_markers(id, config.look == "neo", &stroke_width, &background);
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" id=\"{id}\" {size_attributes} class=\"erDiagram\" viewBox=\"{x} {y} {w} {h}\"{accessible_attributes}>{accessibility}<style>{style}</style><g>{marker_defs}{root}{coloured_markers}</g>{title_svg}{defs}</svg>",
        id = escape(id),
        x = number(drawn.x - VIEWBOX_PADDING),
        y = number(drawn.y - VIEWBOX_PADDING),
        w = number(view_width),
        h = number(view_height),
        root = painted.root,
        coloured_markers = painted.coloured_markers,
        defs = look_defs(
            id,
            Bridge::new(config, input.themes).view(),
            &Globals::of(config)
        ),
    ))
}
