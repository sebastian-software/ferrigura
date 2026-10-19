//! The requirement diagram: a port of the unified renderer with the requirement `getData`
//! (`diagrams/requirement/requirementRenderer.ts`, `requirementDb.ts`). The research is in
//! `docs-port/requirement/NOTES.md`, section 6.
//!
//! The pipeline is the flowchart's: `getData` builds the layout data, the measure step sizes the
//! boxes, ELK lays the graph out, and the paint writes the boxes and the relationships. The root
//! `<svg>` carries the two markers of the look, the style sheet, the title and the accessibility
//! elements. The title margin and `useMaxWidth` come from the `state` section, as the reference
//! reads them (`requirementRenderer.ts` 12, 31-38).

pub mod data;
pub mod style;

use std::collections::HashSet;

use crate::context::Context;
use crate::unified::{Config, Section};
use ferrigura_model::requirement::RequirementDiagram;
use ferrigura_text::{
    FontFamily, FontStyle, FontWeight, Run, embed_runs, face, measure as measure_text, measure_box,
    text_box,
};

use crate::RenderError;
use crate::a11y;
use crate::flowchart::style::look_defs;
use crate::svg::{escape, number};
use crate::unified::geom::BBox;
use crate::unified::markers;
use crate::unified::pipeline::{self, Measured, cover};

/// The padding of the viewBox: `setupViewPortForSVG(svg, padding, …)` with `padding = 8`
/// (`requirementRenderer.ts` 36).
const VIEWBOX_PADDING: f64 = 8.0;

/// The title's font size when the theme has none: the root's `font-size` (`styles.ts` `svg`).
const DEFAULT_FONT_SIZE: f64 = 16.0;

/// `getData` and the measure step of the unified renderer (`requirementRenderer.ts` 14-27).
fn measure(
    input: &Context<'_, RequirementDiagram, Section<'_>>,
    config: &Config<'_>,
) -> Result<Measured, RenderError> {
    let mut data = data::get_data(input.diagram, input.globals);
    // `requirementRenderer.ts` 19: the diagram type of the layout data, which names the markers.
    data.diagram_type = markers::REQUIREMENT.to_owned();
    pipeline::measure(data, config)
}

/// The two inputs that the reference cannot draw, which are named exceptions of the port
/// (`docs-port/requirement/NOTES.md`, section 8.3, questions 2 and 3).
///
/// A relationship to a name that is no requirement and no element fails inside ELK in the
/// reference, so it has no output to reproduce. A requirement and an element with one name make
/// the reference write two nodes with one id, and a `translate(undefined…)` error.
fn check_names(diagram: &RequirementDiagram) -> Result<(), RenderError> {
    let mut requirements = HashSet::new();
    for requirement in &diagram.requirements {
        requirements.insert(requirement.name.as_str());
    }
    let mut elements = HashSet::new();
    for element in &diagram.elements {
        elements.insert(element.name.as_str());
    }
    if let Some(name) = requirements.intersection(&elements).next() {
        return Err(RenderError::Layout(format!(
            "the requirement and the element {name} have the same name, which the drawing cannot give two ids"
        )));
    }
    for relation in &diagram.relationships {
        for name in [&relation.src, &relation.dst] {
            if !requirements.contains(name.as_str()) && !elements.contains(name.as_str()) {
                return Err(RenderError::Layout(format!(
                    "the relationship names {name}, which is no requirement and no element"
                )));
            }
        }
    }
    Ok(())
}

/// Renders the diagram to an SVG document.
///
/// # Errors
///
/// Returns an error if a relationship names no requirement or element, if a requirement and an
/// element share a name (both are named exceptions, see [`check_names`]), if a bundled font cannot
/// be read, or if the layout fails.
pub fn render(input: &Context<'_, RequirementDiagram, Section<'_>>) -> Result<String, RenderError> {
    check_names(input.diagram)?;
    let config = Config::of(input);
    let mut measured = measure(input, &config)?;
    pipeline::lay_out(&mut measured, &config)?;
    let id = input.id;
    let painted = pipeline::paint(&measured, id, &config)?;
    let mut bounds = painted.bounds;

    // `utils.insertTitle` (`requirementRenderer.ts` 31-36): centred over the drawing, at
    // `-titleTopMargin` of the state section, in the theme's font size.
    let sans = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
    let title = &input.diagram.title;
    let font_size = input
        .theme
        .light
        .font_size
        .px()
        .unwrap_or(DEFAULT_FONT_SIZE);
    let mut title_svg = String::new();
    if !title.is_empty() {
        let drawn = bounds.unwrap_or_default();
        let width = measure_text(sans, title, font_size)?;
        let ink = measure_box(sans, title, font_size)?;
        let text_box = text_box(sans, font_size)?;
        let baseline = -(input.section.state.title_top_margin as f64);
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
            "<text text-anchor=\"middle\" x=\"{}\" y=\"{}\" class=\"requirementDiagramTitleText\">{}</text>",
            number(centre),
            number(baseline),
            escape(title)
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

    // `setupViewPortForSVG` (`setupViewPortForSVG.ts` 5-40): the drawn box with the padding around it,
    // and `max-width` when the state section's `useMaxWidth` holds.
    let drawn = bounds.unwrap_or_default();
    let (view_width, view_height) = (
        drawn.width + VIEWBOX_PADDING * 2.0,
        drawn.height + VIEWBOX_PADDING * 2.0,
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
        a11y::root_attributes(id, "requirement", &diagram.acc_title, &diagram.acc_descr);
    let style = style::style_sheet(id, input.theme, input.globals, &families);
    // `insertMarkers` with `data4Layout.markers` (`requirementRenderer.ts` 24-27): the `neo` set for
    // the neo look, else the classic one.
    let stroke_width = input
        .theme
        .light
        .stroke_width
        .text()
        .unwrap_or_else(|| "1".to_owned());
    let marker_defs = markers::requirement_markers(id, input.globals.look == "neo", &stroke_width);
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" id=\"{id}\" {size_attributes} class=\"requirementDiagram\" viewBox=\"{x} {y} {w} {h}\"{accessible_attributes}>{accessibility}<style>{style}</style><g>{marker_defs}{root}{coloured_markers}</g>{title_svg}{defs}</svg>",
        id = escape(id),
        x = number(drawn.x - VIEWBOX_PADDING),
        y = number(drawn.y - VIEWBOX_PADDING),
        w = number(view_width),
        h = number(view_height),
        root = painted.root,
        coloured_markers = painted.coloured_markers,
        defs = look_defs(id, input.theme, input.globals),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrigura_syntax::requirement::parse;

    #[test]
    fn a_relation_to_an_undeclared_name_is_refused() {
        let diagram =
            parse("requirementDiagram\nrequirement a {\n}\na - traces -> missing").expect("parses");
        let error = check_names(&diagram).expect_err("refused");
        assert!(error.to_string().contains("missing"), "{error}");
    }

    #[test]
    fn a_requirement_and_an_element_with_one_name_are_refused() {
        let diagram =
            parse("requirementDiagram\nrequirement a {\n}\nelement a {\n}").expect("parses");
        assert!(check_names(&diagram).is_err());
    }

    #[test]
    fn declared_names_are_accepted() {
        let diagram =
            parse("requirementDiagram\nrequirement a {\n}\nelement b {\n}\nb - satisfies -> a")
                .expect("parses");
        assert!(check_names(&diagram).is_ok());
    }
}
