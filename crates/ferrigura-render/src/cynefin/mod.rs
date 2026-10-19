//! The cynefin diagram: a port of `packages/mermaid/src/diagrams/cynefin/cynefinRenderer.ts` (`draw`),
//! with its boundary waves (`boundary`), its layout (`layout`), its texts (`text`), its colours and
//! rules (`style`) and its groups (`draw`). The research is in `docs-port/cynefin/NOTES.md`, section 6,
//! and the quirks the port reproduces are in `docs-port/cynefin/PLAN.md`.
//!
//! The root is written as the reference writes it: the accessible title and description that mermaid
//! inserts first, the `style` element, the empty group of `mermaidAPI`, the diagram's own accessible
//! texts, the `g` that holds the diagram, and the `defs` with the arrow head after it. The font subset
//! holds the characters of every text, in the face the text is set in (bold, italic or regular).

pub mod boundary;
mod draw;
mod layout;
mod style;
mod text;

use ferrigura_model::config::CynefinDiagramConfig;
use ferrigura_model::cynefin::CynefinDiagram;
use ferrigura_text::{Run, embed_runs};

use crate::RenderError;
use crate::a11y;
use crate::common::Element;
use crate::common::d3::js_number;
use crate::common::style::base_style_sheet;
use crate::common::viewbox::calculate_svg_size_attrs;
use crate::context::Context;
use style::Options;
use text::markup;

/// The XML namespace of the root element.
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// Renders the diagram to an SVG document.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read or subset.
pub fn render(
    input: &Context<'_, CynefinDiagram, CynefinDiagramConfig>,
) -> Result<String, RenderError> {
    let config = input.section;
    let diagram = input.diagram;
    let light = Options::of(input.theme.light.cynefin.as_ref());
    let dark = input
        .theme
        .dark
        .map(|tokens| Options::of(tokens.cynefin.as_ref()));

    let width = config.width;
    let height = config.height;
    let padding = config.padding;
    let show = config.show_domain_descriptions;
    let total_width = width + padding * 2.0;
    let total_height = height + padding * 2.0;
    let seed = boundary::resolve_seed(config.seed, input.id);

    // The texts first: their runs give the fonts, and the elements are the same whichever order they
    // are built in.
    let mut runs = Vec::new();
    let backgrounds = draw::backgrounds(width, height, &light);
    let boundaries = draw::boundaries(width, height, seed, config.boundary_amplitude, &light);
    let confusion = draw::confusion(width, height, &light);
    let labels = draw::labels(width, height, show, &light, &mut runs);
    let subtitles = show.then(|| draw::subtitles(width, height, &light, &mut runs));
    // The root's font size is the one the style sheet sets, and the text of a badge takes it when its
    // own size is not a length (`style::item_measure_size`).
    let font_size = input
        .theme
        .light
        .font_size
        .text()
        .unwrap_or_else(|| "16px".to_owned());
    let root_size = style::css_pixels(&font_size).unwrap_or(16.0);
    let measure = style::item_measure_size(&light, root_size);
    let items = draw::items(diagram, width, height, show, &light, measure, &mut runs)?;
    let (defs, arrows) = draw::arrows(diagram, width, height, input.id, &light, &mut runs).unzip();
    let title = draw::title(width, padding, &diagram.title, &light, &mut runs);

    // The root group, in the order `cynefinRenderer.ts` appends its children.
    let mut root = Element::new("g").with(
        "transform",
        format!("translate({}, {})", js_number(padding), js_number(padding)),
    );
    root.push_child(backgrounds);
    root.push_child(boundaries);
    root.push_child(confusion);
    root.push_child(labels);
    if let Some(subtitles) = subtitles {
        root.push_child(subtitles);
    }
    root.push_child(items);
    if let Some(arrows) = arrows {
        root.push_child(arrows);
    }
    if let Some(title) = title {
        root.push_child(title);
    }

    let families = embed_runs(
        &runs
            .iter()
            .map(|(text, face)| Run {
                text: text.as_str(),
                face,
            })
            .collect::<Vec<_>>(),
    )?;
    let font_family = families.first().map_or_else(
        || {
            input
                .theme
                .light
                .font_family
                .text()
                .unwrap_or_else(|| "sans-serif".to_owned())
        },
        |family| family.css_family.clone(),
    );
    let own_rules = style::own_rules(input.id, &light, dark.as_ref());
    let style = base_style_sheet(
        input.id,
        input.theme,
        &font_family,
        &font_size,
        &families,
        &own_rules,
    );

    // `configureSvgSize` writes the size, the namespace comes between the width and the other size
    // attribute (as the quadrant chart's root does), then the `viewBox`, the role and the names.
    let mut svg = Element::new("svg").with("id", input.id);
    let sizes = calculate_svg_size_attrs(total_height, total_width, config.use_max_width);
    for (name, value) in sizes.iter().filter(|(name, _)| *name == "width") {
        svg.set(name, value.clone());
    }
    svg.set("xmlns", SVG_NAMESPACE);
    for (name, value) in sizes.into_iter().filter(|(name, _)| *name != "width") {
        svg.set(name, value);
    }
    svg.set(
        "viewBox",
        format!("0 0 {} {}", js_number(total_width), js_number(total_height)),
    );
    svg.set("role", a11y::SVG_ROLE);
    svg.set("aria-roledescription", "cynefin");
    let acc_title = input.diagram.acc_title.as_str();
    let acc_descr = input.diagram.acc_descr.as_str();
    if !acc_descr.is_empty() {
        svg.set("aria-describedby", a11y::description_id(input.id));
    }
    if !acc_title.is_empty() {
        svg.set("aria-labelledby", a11y::title_id(input.id));
    }

    // mermaid's own accessible texts come first (`accessibility.ts` 45-66), then the style, then
    // the empty group of `mermaidAPI.ts` 419, then the diagram's own texts (`cynefinRenderer.ts` 104-110).
    if !acc_title.is_empty() {
        svg.push_child(
            Element::new("title")
                .with("id", a11y::title_id(input.id))
                .raw_text(markup(acc_title)),
        );
    }
    if !acc_descr.is_empty() {
        svg.push_child(
            Element::new("desc")
                .with("id", a11y::description_id(input.id))
                .raw_text(markup(acc_descr)),
        );
    }
    svg.push_child(Element::new("style").raw_text(style));
    svg.push_child(Element::new("g"));
    if !acc_title.is_empty() {
        svg.push_child(Element::new("title").raw_text(markup(acc_title)));
    }
    if !acc_descr.is_empty() {
        svg.push_child(Element::new("desc").raw_text(markup(acc_descr)));
    }
    svg.push_child(root);
    if let Some(defs) = defs {
        svg.push_child(defs);
    }
    Ok(svg.render())
}
