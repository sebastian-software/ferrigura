//! The use case diagram: a port of `packages/mermaid/src/diagrams/usecase/` (`usecaseRenderer.ts`'s `draw`,
//! the `getData` of `usecaseDb.ts`, the stylesheet `styles.ts`) on the unified renderer, with its own shapes
//! (`unified/shapes/usecase_*`) and its system boundary (`unified/shapes/usecase_boundary`). The research is
//! in `docs-port/usecase/NOTES.md` (section 6).
//!
//! The steps follow `draw` (`usecaseRenderer.ts` 237-296): the layout data, the measure step, the layout,
//! the paint, the DOM that the reference's `annotateUsecaseElements` adds, the root with its viewBox, and the
//! fonts and style sheet.

pub mod data;
pub mod style;

use ferrigura_model::config::UsecaseDiagramConfig;
use ferrigura_model::use_case::UseCaseDiagram;
use ferrigura_text::{EmbeddedFamily, Run, embed_runs};

use crate::RenderError;
use crate::a11y;
use crate::context::Context;
use crate::svg::{escape, number, xlink_declaration};
use crate::unified::pipeline::{self, Painted};
use crate::unified::{Config, Section, markers, palette};

/// The configuration the use case renderer reads: its own `usecase` section, and the sections of the
/// unified renderer that its pipeline and shapes read (`flowchart`, `elk`, and the rest, as
/// [`Section::of`] gives them).
#[derive(Debug, Clone, Copy)]
pub struct UseCaseSection<'a> {
    /// The `usecase` section.
    pub usecase: &'a UsecaseDiagramConfig,
    /// The sections that the unified renderer reads.
    pub unified: Section<'a>,
}

/// The font size of the labels when the theme has none (the root's default).
const DEFAULT_FONT_SIZE: f64 = 16.0;

/// Renders the diagram to an SVG document.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read, a shape is unknown, or the layout fails.
pub fn render(
    input: &Context<'_, UseCaseDiagram, UseCaseSection<'_>>,
) -> Result<String, RenderError> {
    let id = input.id;
    let config = Config {
        section: input.section.unified,
        globals: input.globals,
        theme: input.theme.light,
    };

    // `getData` and the measure step (`usecaseRenderer.ts` 244-283): every node's shape and label, and the
    // edges' labels.
    let mut layout = data::layout_data(input.diagram, input.section, input.globals, id);
    layout.diagram_type = markers::USECASE.to_owned();
    let mut measured = pipeline::measure(layout, &config)?;

    // `annotateUsecaseElements` (`usecaseRenderer.ts` 144-209): the attributes of each node group, written
    // with the colour slot the shape takes (`stampColorSlot`).
    let palette = palette::palette(input.globals, input.theme.light);
    for node in &measured.data.nodes {
        let Some(fields) = node.use_case.as_ref() else {
            continue;
        };
        if node.is_group {
            continue;
        }
        let colour = colour_attribute(node.color_index, palette.as_ref());
        let annotation = annotation(&node.id, fields.kind.name(), &fields.accessible_name);
        if let Some(shape) = measured.shapes.get_mut(&node.id) {
            shape.group_attributes = format!("{colour}{annotation}");
            // `annotateUsecaseElements` 168-188: a folded ellipse's stereotype gets its class on the first line of
            // the label. The reference's captures give a folded rectangle no such class (fixture 09), so the rule
            // applies to the ellipse only, as the reference's DOM shows it.
            if let (true, Some(label)) = (
                fields.folded_stereotype && node.shape == "usecaseEllipse",
                shape.label.as_mut(),
            ) {
                label.first_line_class = Some("usecase-stereotype");
            }
        }
    }

    pipeline::lay_out(&mut measured, &config)?;
    let painted = pipeline::paint(&measured, id, &config)?;
    let bounds = painted.bounds;

    // The fonts of every label are subset once, as the flowchart's are (`embed_runs`).
    let families = font_families(&painted)?;
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
    let font_size = input
        .theme
        .light
        .font_size
        .px()
        .unwrap_or(DEFAULT_FONT_SIZE);
    let rotate = input.section.usecase.color_scheme == "rotate";
    // Under `colorScheme: auto` the dark set of the rules follows the light set, under the media query.
    let rules = style::scheme_rules(id, input.theme, input.globals, rotate);
    let style = crate::common::style::base_style_sheet(
        id,
        input.theme,
        &font_family,
        &format!("{}px", number(font_size)),
        &families,
        &rules,
    );

    // `setupViewPortForSVG` (`usecaseRenderer.ts` 293): the drawn box with `diagramPadding` around it, and the
    // use case's fonts, which `applyUsecaseFonts` puts on the root's style.
    let usecase = input.section.usecase;
    let padding = usecase.diagram_padding;
    let drawn = bounds.unwrap_or_default();
    let (view_width, view_height) = (drawn.width + padding * 2.0, drawn.height + padding * 2.0);
    // The style attribute holds the font families, which carry quotes: they are escaped as an attribute.
    let fonts = escape(&font_custom_properties(usecase));
    let size_attributes = if usecase.use_max_width {
        format!(
            "width=\"100%\" style=\"max-width: {}px; {fonts}\"",
            number(view_width)
        )
    } else {
        format!(
            "height=\"{}\" width=\"{}\" style=\"{fonts}\"",
            number(view_height),
            number(view_width)
        )
    };
    let accessibility =
        a11y::children(id, &input.diagram.acc_title, &input.diagram.acc_description);
    let accessible_attributes = a11y::root_attributes(
        id,
        "usecase",
        &input.diagram.acc_title,
        &input.diagram.acc_description,
    );
    let defs = crate::flowchart::style::look_defs(id, input.theme, input.globals);
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\"{xlink} id=\"{id}\" {size_attributes} class=\"usecaseDiagram\" viewBox=\"{x} {y} {w} {h}\"{accessible_attributes}>{accessibility}<style>{style}</style><g>{markers}{root}{coloured}</g>{defs}</svg>",
        id = escape(id),
        xlink = xlink_declaration(painted.links),
        x = number(drawn.x - padding),
        y = number(drawn.y - padding),
        w = number(view_width),
        h = number(view_height),
        markers = markers::usecase_markers(id),
        root = painted.root,
        coloured = painted.coloured_markers,
    ))
}

/// The font subsets of the diagram: every label's runs, in the faces they are set in.
fn font_families(painted: &Painted<'_>) -> Result<Vec<EmbeddedFamily>, RenderError> {
    let runs: Vec<Run<'_>> = painted
        .all_labels()
        .flat_map(|label| label.text_runs())
        .collect();
    Ok(embed_runs(&runs)?)
}

/// The custom properties `applyUsecaseFonts` sets on the root (`usecaseRenderer.ts` 224-232), in the order
/// the reference's style attribute holds them.
fn font_custom_properties(usecase: &UsecaseDiagramConfig) -> String {
    format!(
        "--mermaid-usecase-actor-font-size: {}px; --mermaid-usecase-actor-font-family: {}; --mermaid-usecase-actor-font-weight: {}; --mermaid-usecase-font-size: {}px; --mermaid-usecase-font-family: {}; --mermaid-usecase-font-weight: {};",
        number(usecase.actor_font_size),
        usecase.actor_font_family,
        usecase.actor_font_weight,
        number(usecase.usecase_font_size),
        usecase.usecase_font_family,
        usecase.usecase_font_weight,
    )
}

/// The `data-color-id` of a node, when the theme has a palette (`stampColorSlot`).
fn colour_attribute(index: Option<u32>, palette: Option<&palette::Palette>) -> String {
    palette
        .and_then(|palette| palette::palette_slot(index, palette))
        .map_or_else(String::new, |slot| {
            format!(" data-color-id=\"color-{slot}\"")
        })
}

/// The DOM attributes that `annotateUsecaseElements` writes on a node or a boundary group.
fn annotation(id: &str, kind: &str, accessible_name: &str) -> String {
    format!(
        " data-usecase-id=\"{}\" data-usecase-kind=\"{kind}\" role=\"img\" aria-label=\"{}\"",
        escape(id),
        escape(accessible_name),
    )
}
