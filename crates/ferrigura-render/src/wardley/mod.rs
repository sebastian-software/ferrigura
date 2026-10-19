//! The wardley map: `wardleyRenderer.ts` of the reference (`draw`), over the DB's state. The research
//! is in `docs-port/wardley/NOTES.md`, section 6 (the rules, the DOM order and the quirks), and the
//! decisions in `docs-port/wardley/PLAN.md`.
//!
//! The map is a projection, with no layout engine. The reference draws it element by element, so the
//! root is the same sequence of groups and elements, in the same order:
//!
//! - `svg`: the size (`width="100%"` and `style` with `useMaxWidth`, else `height` and `width`), the
//!   `viewBox`, the role and the names, and the title and description when they are not empty.
//! - `g.wardley-map`: the background, the title, the axes, the stages, the grid, the pipelines, the
//!   links, the trends, the nodes, the annotations, the notes, the accelerators and the
//!   deaccelerators, each group in the order and with the conditions of the reference.
//! - `defs`: the three arrow markers, named with the diagram's id.
//!
//! The reference's stylesheet is never in its output: `draw` removes every child of the root before
//! it draws, and the `<style>` that mermaid inserted goes with them (NOTES 6.9). This port writes
//! none of the rules. It writes the bundled face's `@font-face` and names it on `g.wardley-map`,
//! because the reference's text takes the page's default font, and ferrigura draws the text in the
//! face it measures with (P3-PLAN, "Fonts the reference picks"; the harness pins the reference's
//! text to the same face).
//!
//! The reference's text is measured in one place, the annotations box (`annotations.rs`). The
//! theme's colours are read from the `wardley` group of the theme variables, with the fallbacks of
//! `getTheme` (`theme.rs`).

mod annotations;
mod axes;
mod links;
mod marks;
mod nodes;
mod pipelines;
mod plot;
mod text;
mod theme;

use ferrigura_model::config::WardleyDiagramConfig;
use ferrigura_model::wardley::WardleyDiagram;
use ferrigura_text::embed_runs;

use crate::RenderError;
use crate::a11y;
use crate::common::Element;
use crate::common::d3::js_number;
use crate::common::viewbox::calculate_svg_size_attrs;
use crate::context::Context;
use crate::svg::font_faces;
use plot::{Placements, Plot};
use text::{Drawn, markup};
use theme::Colours;

/// The XML namespace of the root element.
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// What the parts of the map read in common: the diagram, its configuration, the plot, the colours
/// and the id of the SVG, which the marker and the arrow ids carry.
pub struct Canvas<'a> {
    /// The parsed map.
    pub diagram: &'a WardleyDiagram,
    /// The `wardley-beta` section of the configuration.
    pub config: &'a WardleyDiagramConfig,
    /// The plot and the canvas, with the size the `size` statement or the configuration gives.
    pub plot: Plot,
    /// The colours of the light scheme.
    pub colours: Colours,
    /// The id of the SVG.
    pub id: &'a str,
}

impl Canvas<'_> {
    /// The radius of a node's circle (`nodeRadius`).
    #[must_use]
    pub fn node_radius(&self) -> f64 {
        self.config.node_radius
    }

    /// The side of a pipeline parent's square: `nodeRadius * 1.6` (`wardleyRenderer.ts` 72).
    #[must_use]
    pub fn square_size(&self) -> f64 {
        self.config.node_radius * 1.6
    }
}

/// Renders the map to an SVG document.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read or subset, or if the text of the annotations
/// box cannot be measured.
pub fn render(
    input: &Context<'_, WardleyDiagram, WardleyDiagramConfig>,
) -> Result<String, RenderError> {
    let diagram = input.diagram;
    let config = input.section;
    // The `size` statement overrides the configuration's size (`wardleyRenderer.ts` 81-82).
    let width = diagram.size.map_or(config.width, |size| size.width.0);
    let height = diagram.size.map_or(config.height, |size| size.height.0);
    let canvas = Canvas {
        diagram,
        config,
        plot: Plot::new(width, height, config.padding),
        colours: Colours::of(input.theme.light),
        id: input.id,
    };
    let mut placements = Placements::of(diagram, &canvas.plot);
    let mut drawn = Drawn::default();

    let mut map = Element::new("g").with("class", "wardley-map");
    map.push_child(axes::background(&canvas));
    if let Some(title) = axes::title(&canvas, &mut drawn) {
        map.push_child(title);
    }
    map.push_child(axes::axes(&canvas, &mut drawn));
    map.push_child(axes::stages(&canvas, &mut drawn));
    if config.show_grid {
        map.push_child(axes::grid(&canvas));
    }
    // The pipelines move their parents, so they come before the links, the trends and the nodes.
    if let Some((boxes, links)) = pipelines::draw(&canvas, &mut placements) {
        map.push_child(boxes);
        map.push_child(links);
    }
    map.push_child(links::links(&canvas, &placements, &mut drawn));
    map.push_child(links::trends(&canvas, &placements));
    map.push_child(nodes::draw(&canvas, &placements, &mut drawn));
    if let Some(group) = annotations::draw(&canvas, &mut drawn)? {
        map.push_child(group);
    }
    if let Some(group) = marks::notes(&canvas, &mut drawn) {
        map.push_child(group);
    }
    if let Some(group) = marks::accelerators(&canvas, &mut drawn) {
        map.push_child(group);
    }
    if let Some(group) = marks::deaccelerators(&canvas, &mut drawn) {
        map.push_child(group);
    }

    // The bundled face: every character the map shows, in the weights the reference sets it in. Its
    // family names the map's text, and its `@font-face` rules are the only style the output has.
    let families = embed_runs(&drawn.runs())?;
    let style = (!families.is_empty()).then(|| font_faces(&families));
    if let Some(family) = families.first() {
        map.set("font-family", family.css_family.clone());
    }

    let mut root = Element::new("svg").with("id", input.id);
    // `configureSvgSize(svg, height, width, useMaxWidth)`: the width is set first, as the reference's
    // root takes it from the creation of the SVG.
    let sizes = calculate_svg_size_attrs(height, width, config.use_max_width);
    for (name, value) in sizes.iter().filter(|(name, _)| *name == "width") {
        root.set(name, value.clone());
    }
    root.set("xmlns", SVG_NAMESPACE);
    for (name, value) in sizes.into_iter().filter(|(name, _)| *name != "width") {
        root.set(name, value);
    }
    root.set(
        "viewBox",
        format!("0 0 {} {}", js_number(width), js_number(height)),
    );
    root.set("role", a11y::SVG_ROLE);
    root.set("aria-roledescription", "wardley");
    let title = diagram.acc_title.as_str();
    let description = diagram.acc_descr.as_str();
    if !description.is_empty() {
        root.set("aria-describedby", a11y::description_id(input.id));
    }
    if !title.is_empty() {
        root.set("aria-labelledby", a11y::title_id(input.id));
        root.push_child(
            Element::new("title")
                .with("id", a11y::title_id(input.id))
                .raw_text(markup(title)),
        );
    }
    if !description.is_empty() {
        root.push_child(
            Element::new("desc")
                .with("id", a11y::description_id(input.id))
                .raw_text(markup(description)),
        );
    }
    if let Some(style) = style {
        root.push_child(Element::new("style").raw_text(style));
    }
    root.push_child(map);
    root.push_child(markers(&canvas));
    Ok(root.render())
}

/// `defs`: the three arrow markers of the map, with the diagram's id in their names
/// (`wardleyRenderer.ts` 92-138). The trend's arrow is the evolution colour; the link arrows take
/// the link colour, and the one at the start points back.
fn markers(canvas: &Canvas<'_>) -> Element {
    let id = canvas.id;
    Element::new("defs")
        .child(marker(
            format!("arrow-{id}"),
            "9",
            "6",
            "auto-start-reverse",
            "M 0 0 L 10 5 L 0 10 z",
            &canvas.colours.evolution_stroke,
        ))
        .child(marker(
            format!("link-arrow-end-{id}"),
            "9",
            "5",
            "auto",
            "M 0 0 L 10 5 L 0 10 z",
            &canvas.colours.link_stroke,
        ))
        .child(marker(
            format!("link-arrow-start-{id}"),
            "1",
            "5",
            "auto",
            "M 10 0 L 0 5 L 10 10 z",
            &canvas.colours.link_stroke,
        ))
}

/// One arrow marker: a ten-unit box, with the arrow's path in its fill and no stroke.
fn marker(name: String, ref_x: &str, size: &str, orient: &str, path: &str, fill: &str) -> Element {
    Element::new("marker")
        .with("id", name)
        .with("viewBox", "0 0 10 10")
        .with("refX", ref_x)
        .with("refY", "5")
        .with("markerWidth", size)
        .with("markerHeight", size)
        .with("orient", orient)
        .child(
            Element::new("path")
                .with("d", path)
                .with("fill", fill)
                .with("stroke", "none"),
        )
}
