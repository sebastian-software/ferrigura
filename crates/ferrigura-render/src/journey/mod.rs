//! The journey diagram: a port of `diagrams/user-journey/journeyRenderer.ts`, `svgDraw.js` and
//! `styles.js`. The research is in `docs-port/journey/NOTES.md`, sections 7 (renderer) and 8
//! (configuration).
//!
//! The drawing is in four parts: the legend of the actors (`legend`), the sections and the tasks
//! with their faces (`tasks`, `face`), the labels (`text`), and the stylesheet (`style`). This file
//! sizes the root `<svg>`, writes the title and the activity line, and embeds the fonts.
//!
//! Labels are SVG text, not HTML (`docs-port/decisions/0004`): the default placement, `fo`, draws
//! the lines the browser would break the HTML into. Every text is set in the bundled face, as in
//! the other renderers, so `titleFontFamily` and `taskFontFamily` of the configuration are not read;
//! the geometry harness pins them to the bundled face on the reference side (`journey-labels.ts`).

mod face;
mod legend;
mod style;
mod tasks;
mod text;

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::journey::JourneyDiagram;
use ferrigura_text::{Run, embed_runs};

use self::legend::Actors;
use self::tasks::{Bounds, Context};
use self::text::Placement;
use crate::RenderError;
use crate::a11y;
use crate::common::Element;
use crate::common::viewbox::configure_svg_size;
use crate::svg::number;
use crate::theme::Themes;
use crate::variables::{px, text_of};

/// The XML namespace of the root element.
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// What the journey renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct JourneyInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a JourneyDiagram,
    /// The effective configuration, in the journey diagram's scope.
    pub config: &'a MermaidConfig,
    /// The colours and sizes of the theme.
    pub themes: Themes<'a>,
    /// The id of the SVG element. It scopes every selector and prefixes every element id, so it
    /// must be a CSS identifier.
    pub id: &'a str,
}

/// Renders the diagram to an SVG document.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read or subset.
pub fn render(input: &JourneyInput<'_>) -> Result<String, RenderError> {
    let diagram = input.diagram;
    let config = &input.config.journey;
    let id = input.id;
    let variables = input.themes.primary();
    // The root's font size is the theme's, and the legend and labels are measured at it.
    let font_size = px(variables, "fontSize").unwrap_or(16.0);
    let font_size_css = text_of(variables, "fontSize").unwrap_or_else(|| "16px".to_owned());
    let text_colour = style::text_colour(variables);
    let fills = style::fills(variables);
    let placement = Placement::of(&config.text_placement);

    let actors = Actors::new(&diagram.actors, &config.actor_colours);
    // The text drawn in the regular face, for the embedded subset (every character drawn is in it).
    let mut texts = Vec::new();
    let drawn_legend = legend::draw(&actors, config, font_size, &mut texts)?;

    // `leftMargin` is the configured margin plus the widest legend line past it.
    let left_margin = config.left_margin as f64 + drawn_legend.max_width;
    let mut bounds = Bounds::default();
    bounds.insert(0.0, 0.0, left_margin, actors.legend_len() as f64 * 50.0);
    let context = Context {
        config,
        actors: &actors,
        id,
        fills: &fills,
        text_colour: &text_colour,
        placement,
        font_size,
        task_font_size: text::task_font_size(config),
        left_margin,
    };
    let task_elements = tasks::draw(&diagram.tasks, &context, &mut bounds, &mut texts)?;

    let (startx, starty, stopx, stopy) = bounds.corners();
    let height = stopy - starty + 2.0 * config.diagram_margin_y as f64;
    let width = left_margin + stopx + 2.0 * config.diagram_margin_x as f64;
    let title = (!diagram.title.is_empty()).then(|| {
        Element::new("text")
            .with("x", number(left_margin))
            .with("font-size", config.title_font_size.as_str())
            .with("font-weight", "bold")
            .with("y", "25")
            .with("fill", config.title_color.as_str())
            .text(diagram.title.as_str())
    });

    // The fonts: the regular face for the legend, the labels and the tasks, the bold one for the
    // title. A face is embedded only when a run of it is not empty.
    let regular_text = texts.join(" ");
    let mut runs = Vec::new();
    if !regular_text.is_empty() {
        runs.push(Run {
            text: &regular_text,
            face: text::regular(),
        });
    }
    if title.is_some() {
        runs.push(Run {
            text: &diagram.title,
            face: text::bold(),
        });
    }
    let families = embed_runs(&runs)?;
    let font_family = families.first().map_or_else(
        || text_of(variables, "fontFamily").unwrap_or_else(|| "sans-serif".to_owned()),
        |family| format!("\"{}\"", family.name),
    );
    let style = style::style_sheet(id, input.themes, &font_family, &font_size_css, &families);

    // The root: the size, the view box and the roles, in the order `mermaidAPI` and the renderer
    // set them.
    let extra = if title.is_some() { 70.0 } else { 0.0 };
    let mut root = Element::new("svg")
        .with("id", id)
        .with("width", "100%")
        .with("xmlns", SVG_NAMESPACE);
    configure_svg_size(&mut root, height, width, config.use_max_width);
    root.set(
        "viewBox",
        format!(
            "{} -25 {} {}",
            number(startx),
            number(width),
            number(height + extra)
        ),
    );
    root.set("preserveAspectRatio", "xMinYMin meet");
    root.set("height", number(height + extra + 25.0));
    root.set("role", a11y::SVG_ROLE);
    root.set("aria-roledescription", "journey");
    if !diagram.acc_descr.is_empty() {
        root.set("aria-describedby", a11y::description_id(id));
    }
    if !diagram.acc_title.is_empty() {
        root.set("aria-labelledby", a11y::title_id(id));
    }

    if !diagram.acc_title.is_empty() {
        root = root.child(
            Element::new("title")
                .with("id", a11y::title_id(id))
                .text(diagram.acc_title.as_str()),
        );
    }
    if !diagram.acc_descr.is_empty() {
        root = root.child(
            Element::new("desc")
                .with("id", a11y::description_id(id))
                .text(diagram.acc_descr.as_str()),
        );
    }
    root = root
        .child(Element::new("style").raw_text(style))
        // The empty group `mermaidAPI` appends before the diagram draws.
        .child(Element::new("g"))
        .child(arrowhead(id));
    for element in drawn_legend.elements.into_iter().chain(task_elements) {
        root = root.child(element);
    }
    if let Some(title) = title {
        root = root.child(title);
    }
    root = root.child(activity_line(
        left_margin,
        width,
        config.height as f64 * 4.0,
        id,
    ));
    Ok(root.render())
}

/// The arrowhead of the activity line: a marker of the diagram's id, written in `defs`
/// (`svgDraw.js` 392-405).
fn arrowhead(id: &str) -> Element {
    Element::new("defs").child(
        Element::new("marker")
            .with("id", format!("{id}-arrowhead"))
            .with("refX", "5")
            .with("refY", "2")
            .with("markerWidth", "6")
            .with("markerHeight", "4")
            .with("orient", "auto")
            .child(Element::new("path").with("d", "M 0,0 V 4 L6,2 Z")),
    )
}

/// The activity line, from the left margin to the right edge, with the arrowhead at its end
/// (`journeyRenderer.ts` 183-192). It sits at four times the task height, below one section head
/// and one task. It stops four units short of the right edge, so the point of the arrow stays
/// inside the diagram.
fn activity_line(left_margin: f64, width: f64, y: f64, id: &str) -> Element {
    Element::new("line")
        .with("x1", number(left_margin))
        .with("y1", number(y))
        .with("x2", number(width - left_margin - 4.0))
        .with("y2", number(y))
        .with("stroke-width", "4")
        .with("stroke", "black")
        .with("marker-end", format!("url(#{id}-arrowhead)"))
}
