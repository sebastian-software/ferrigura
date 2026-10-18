//! The mindmap diagram: a port of `diagrams/mindmap/mindmapRenderer.ts` (`draw`), `mindmapDb.ts`
//! `getData`, `styles.ts` and the layout renderers of `rendering-util/layout-algorithms/cose-bilkent`
//! and the tidy tree, on the unified shapes (`unified/shapes`). The research is in
//! `docs-port/mindmap/NOTES.md` (sections 3.5 and 6) and `docs-port/cose/NOTES.md` (section 5).
//!
//! The steps follow the reference: the layout data is built (`data`), each node is measured with its
//! shape and its Markdown label, the layout engine places the nodes and gives the edges their points
//! (`layout`), the diagram is painted (`paint`), and the root `svg` takes the box the drawing covers.
//! Labels are SVG text, the harness's form (decision 0004).

pub mod data;
mod layout;
mod paint;
mod style;

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::mindmap::MindmapDiagram;
use ferrigura_text::{EmbeddedFamily, Run, embed_runs};

use crate::RenderError;
use crate::a11y;
use crate::common::Element;
use crate::common::viewbox::{BBox as ViewBox, setup_graph_viewbox};
use crate::flowchart::style::look_defs;
use crate::svg::{escape, number};
use crate::theme::Themes;
use crate::unified::pipeline;
use crate::variables::{px, text_of};

/// The XML namespace of the root element.
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// The font size when the theme has none (`createText`'s default, the reference's `fontSize` of 16).
const DEFAULT_FONT_SIZE: f64 = 16.0;

/// What the mindmap renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct MindmapInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a MindmapDiagram,
    /// The effective configuration, in the mindmap diagram's scope.
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
/// Returns an error if a bundled font cannot be read, a shape is unknown, or the layout fails (the
/// tidy tree on a cycle; cose-bilkent on a graph that is not a forest).
pub fn render(input: &MindmapInput<'_>) -> Result<String, RenderError> {
    let config = input.config;
    let theme = input.themes.primary();

    // A tree without a root draws nothing: the reference returns after the placeholder `svg`
    // (`mindmapRenderer.ts` 53-56), which keeps no class, viewBox or style (`mindmap/NOTES.md` 6.6).
    if input.diagram.root.is_none() {
        return Ok(format!(
            "<svg xmlns=\"{SVG_NAMESPACE}\" id=\"{id}\"{accessible}><g></g></svg>",
            id = escape(input.id),
            accessible = a11y::root_attributes(input.id, "mindmap", "", ""),
        ));
    }

    // `getData` and the overrides of the shapes (`mindmapRenderer.ts` 58-74), then the measure step:
    // every node's shape and label, and the sizes of their groups.
    let data = data::layout_data(input.diagram, config);
    let mut measured = pipeline::measure(data, config, theme)?;

    // The layout: the name the configuration gives, or cose-bilkent for the names that are not
    // registered (`mindmapRenderer.ts` 47-49).
    match layout::of(&config.layout) {
        layout::Layout::CoseBilkent => layout::cose(&mut measured.data)?,
        layout::Layout::TidyTree => layout::tidy(&mut measured.data)?,
        layout::Layout::Elk => {
            pipeline::lay_out(&mut measured, config)?;
        }
    }

    let painted = paint::paint(&measured, input.id);

    // The fonts: every glyph of every label, in the faces the labels use.
    let mut runs: Vec<Run<'_>> = Vec::new();
    for node in &measured.data.nodes {
        if let Some(shape) = measured.shapes.get(&node.id) {
            if let Some(label) = &shape.label {
                runs.extend(label.text_runs());
            }
            for text in &shape.texts {
                runs.extend(text.text_runs());
            }
        }
    }
    let families: Vec<EmbeddedFamily> = embed_runs(&runs)?;
    let font_family = families.first().map_or_else(
        || text_of(theme, "fontFamily").unwrap_or_else(|| "sans-serif".to_owned()),
        |family| family.css_family.clone(),
    );
    let font_size = px(theme, "fontSize").unwrap_or(DEFAULT_FONT_SIZE);
    let style = crate::common::style::base_style_sheet_with(
        input.id,
        input.themes,
        &font_family,
        &format!("{}px", number(font_size)),
        &families,
        &style::own_rules(input.id, input.themes, &config.look),
    );

    // `setupViewPortForSVG` (`mindmapRenderer.ts` 110-115): the class, the size and the viewBox from
    // the box of the drawing, with `mindmap.padding` on each side.
    let bounds = painted.bounds.unwrap_or_default();
    let mut root = Element::new("svg")
        .with("id", input.id)
        .with("xmlns", SVG_NAMESPACE)
        .with("class", "mindmapDiagram");
    setup_graph_viewbox(
        &mut root,
        ViewBox {
            x: bounds.x,
            y: bounds.y,
            width: bounds.width,
            height: bounds.height,
        },
        config.mindmap.padding,
        config.mindmap.use_max_width,
    );
    root.set("role", a11y::SVG_ROLE);
    root.set("aria-roledescription", "mindmap");

    // The root group is the one `mermaidAPI` appends (`mermaidAPI.ts` 419); the look's shadow and
    // gradient definitions come after it (`insertLookDefs`).
    let defs = look_defs(input.id, input.themes, config);
    let root = root.raw_text(format!(
        "<style>{style}</style><g>{group}</g>{defs}",
        group = painted.root,
    ));
    Ok(root.render())
}
