//! The class diagram: a port of the unified renderer with the class `getData` (`diagrams/class/classRenderer-v3-unified.ts`, `classDb.ts`). The research is in
//! `docs-port/class/NOTES.md`, section 5.

pub mod data;
pub mod member;
pub mod style;

use ferrigura_model::class::ClassDiagram;
use ferrigura_model::config::MermaidConfig;
use ferrigura_text::{
    FontFamily, FontStyle, FontWeight, Run, embed_runs, face, measure as measure_text, measure_box,
    text_box,
};

use crate::RenderError;
use crate::a11y;
use crate::svg::{escape, number};
use crate::theme::Themes;
use crate::unified::Bridge;
use crate::unified::geom::BBox;
use crate::unified::markers;
use crate::unified::pipeline::{self, cover};

/// What the class renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct ClassInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a ClassDiagram,
    /// The effective configuration, in the class diagram's scope.
    pub config: &'a MermaidConfig,
    /// The colours and sizes of the theme.
    pub themes: Themes<'a>,
    /// The id of the SVG element. It scopes every selector and prefixes every element id, so it
    /// must be a CSS identifier.
    pub id: &'a str,
}

/// The padding of the viewBox around the drawing: `setupViewPortForSVG(svg, 8, ...)`
/// (`classRenderer-v3-unified.ts` 75).
const VIEWPORT_PADDING: f64 = 8.0;

/// The font size of the title: `.classTitleText` in the class styles.
const TITLE_FONT_SIZE: f64 = 18.0;

/// Renders the diagram to an SVG document.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read, a shape is unknown, or the layout fails.
pub fn render(input: &ClassInput<'_>) -> Result<String, RenderError> {
    let config = input.config;
    let theme = input.themes.primary();
    let id = input.id;
    let diagram = input.diagram;

    let mut data = data::get_data(diagram, config, theme);
    // `classRenderer-v3-unified.ts` 59: the diagram type names the markers and their class.
    data.diagram_type = markers::CLASS.to_owned();
    let bridge = Bridge::new(config, input.themes);
    let unified = bridge.config(config);
    let mut measured = pipeline::measure(data, &unified)?;
    pipeline::lay_out(&mut measured, &unified)?;
    let painted = pipeline::paint(&measured, id, &unified)?;
    let mut bounds = painted.bounds;

    // `utils.insertTitle` (`classRenderer-v3-unified.ts` 68-73): centred over what is drawn, the
    // state section's `titleTopMargin` above the origin.
    let sans = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
    let title = &diagram.title;
    let mut title_svg = String::new();
    if !title.is_empty() {
        let drawn = bounds.unwrap_or_default();
        let width = measure_text(sans, title, TITLE_FONT_SIZE)?;
        let ink = measure_box(sans, title, TITLE_FONT_SIZE)?;
        let text_box = text_box(sans, TITLE_FONT_SIZE)?;
        let margin = config.state.title_top_margin as f64;
        let centre = drawn.x + drawn.width / 2.0;
        let baseline = -margin;
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
            "<text text-anchor=\"middle\" x=\"{}\" y=\"{}\" class=\"classDiagramTitleText\">{}</text>",
            number(centre),
            number(baseline),
            escape(title),
        );
    }

    let mut runs: Vec<Run<'_>> = painted
        .all_labels()
        .flat_map(|label| label.text_runs())
        .collect();
    for shape in measured.shapes.values() {
        for text in &shape.texts {
            runs.extend(text.text_runs());
        }
    }
    if !title.is_empty() {
        runs.push(Run {
            text: title,
            face: sans,
        });
    }
    let families = embed_runs(&runs)?;

    // `setupViewPortForSVG`: the drawn box with the padding around it.
    let drawn = bounds.unwrap_or_default();
    let (view_width, view_height) = (
        drawn.width + VIEWPORT_PADDING * 2.0,
        drawn.height + VIEWPORT_PADDING * 2.0,
    );
    // `conf.useMaxWidth ?? true` of the state section (`classRenderer-v3-unified.ts` 75).
    let size_attributes = if config.state.use_max_width {
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
    let accessibility = a11y::children(id, &diagram.acc_title, &diagram.acc_descr);
    let accessible_attributes =
        a11y::root_attributes(id, "classDiagram", &diagram.acc_title, &diagram.acc_descr);
    let style = style::style_sheet(id, input.themes, config, &families);
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" id=\"{id}\" {size_attributes} class=\"classDiagram\" viewBox=\"{x} {y} {w} {h}\"{accessible_attributes}>{accessibility}<style>{style}</style><g>{markers}{root}{coloured_markers}</g>{title_svg}{defs}</svg>",
        id = escape(id),
        x = number(drawn.x - VIEWPORT_PADDING),
        y = number(drawn.y - VIEWPORT_PADDING),
        w = number(view_width),
        h = number(view_height),
        markers = markers::class_markers(id),
        root = painted.root,
        coloured_markers = painted.coloured_markers,
        defs = style::defs(id, input.themes, config),
    ))
}
