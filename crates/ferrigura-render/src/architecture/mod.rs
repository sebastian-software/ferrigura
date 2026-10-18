//! The architecture diagram: a port of `packages/mermaid/src/diagrams/architecture/architectureRenderer.ts`
//! (`draw`) with its helpers in `svgDraw.ts`, `architectureStyles.ts` and `architectureIcons.ts`.
//!
//! The renderer lays the services, junctions and groups out with cytoscape and cytoscape-fcose, and
//! draws the result. Here the cytoscape state is [`scene::Scene`] (the nodes, the edges and the
//! functions cytoscape computes from them: bounds, endpoints, bends), the layout is [`layout`] (the
//! two runs of fcose through `ferrigura-cose`), the constraints are [`constraints`] (the alignments and
//! the relative placements of the DB's spatial maps), and the SVG is [`draw`].
//!
//! The research, with the file and line for each rule, is `docs-port/architecture/NOTES.md`, section 6
//! and section E. The decisions of this port are `docs-port/architecture/PLAN.md`, section 8.

mod constraints;
mod draw;
mod geometry;
mod icons;
mod layout;
mod scene;
mod style;

use ferrigura_model::architecture::ArchitectureDiagram;
use ferrigura_model::config::MermaidConfig;
use ferrigura_text::{Run, embed_runs};

use crate::RenderError;
use crate::a11y;
use crate::common::Element;
use crate::common::style::base_style_sheet_with;
use crate::common::viewbox::{BBox as ViewBox, setup_graph_viewbox};
use crate::svg::number;
use crate::theme::Themes;
use crate::variables::{px, text_of};
use geometry::Box2;
use scene::{Metrics, Scene};

/// The XML namespace of the root element.
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// What the architecture renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct ArchitectureInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a ArchitectureDiagram,
    /// The effective configuration, in the architecture diagram's scope.
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
/// Returns an error if the layout refuses the input (a seed of 0 is not reproducible), if a label
/// cannot be measured or laid out, or if a bundled font cannot be read or subset.
pub fn render(input: &ArchitectureInput<'_>) -> Result<String, RenderError> {
    let config = &input.config.architecture;
    let metrics = Metrics {
        icon_size: config.icon_size,
        padding: config.padding,
        font_size: config.font_size,
    };
    let mut scene = Scene::new(input.diagram, metrics, &mut |text, size| {
        scene::canvas_width(text, size)
    })?;
    let constraints = constraints::constraints(
        input.diagram,
        config.icon_size,
        config.ideal_edge_length_multiplier,
    );
    let settings = layout::Settings {
        randomize: config.randomize,
        node_separation: config.node_separation,
        num_iter: config.num_iter,
        seed: config.seed,
        icon_size: config.icon_size,
        ideal_edge_length_multiplier: config.ideal_edge_length_multiplier,
        edge_elasticity: config.edge_elasticity,
    };
    layout::lay_out(&mut scene, &constraints, &settings)?;

    // The SVG text takes the theme's font size (the root style's `font-size`), while cytoscape measures
    // the node labels at the configuration's `fontSize` (`architectureRenderer.ts` 124-145).
    let variables = input.themes.primary();
    let text_size = px(variables, "fontSize").unwrap_or(16.0);
    let drawing = draw::draw(&scene, input.id, metrics, text_size)?;

    // The fonts: every glyph of every label.
    let mut runs: Vec<Run<'_>> = Vec::new();
    for label in &drawing.labels {
        runs.extend(label.text_runs());
    }
    let families = embed_runs(&runs)?;
    let font_family = families.first().map_or_else(
        || text_of(variables, "fontFamily").unwrap_or_else(|| "sans-serif".to_owned()),
        |family| family.css_family.clone(),
    );
    let css_font_size = format!("{}px", number(text_size));
    let own_rules = style::own_rules(input.id, input.themes);
    let style_sheet = base_style_sheet_with(
        input.id,
        input.themes,
        &font_family,
        &css_font_size,
        &families,
        &own_rules,
    );

    // `setupGraphViewbox` with the box of the drawing (`architectureRenderer.ts` 642).
    let bounds = if drawing.bounds.w().is_finite() && drawing.bounds.w() > 0.0 {
        drawing.bounds
    } else {
        Box2::new(0.0, 0.0, 0.0, 0.0)
    };
    let mut root = Element::new("svg")
        .with("id", input.id)
        .with("width", "100%")
        .with("xmlns", SVG_NAMESPACE);
    setup_graph_viewbox(
        &mut root,
        ViewBox {
            x: bounds.x1,
            y: bounds.y1,
            width: bounds.w(),
            height: bounds.h(),
        },
        config.padding,
        config.use_max_width,
    );
    let title = &input.diagram.acc_title;
    let description = &input.diagram.acc_descr;
    root.set("role", a11y::SVG_ROLE);
    root.set("aria-roledescription", "architecture");
    if !description.is_empty() {
        root.set("aria-describedby", a11y::description_id(input.id));
    }
    if !title.is_empty() {
        root.set("aria-labelledby", a11y::title_id(input.id));
    }
    let content = format!(
        "{}<style>{}</style><g></g>{}{}{}",
        a11y::children(input.id, title, description),
        style_sheet,
        drawing.edges,
        drawing.services,
        drawing.groups,
    );
    Ok(root.raw_text(content).render())
}
