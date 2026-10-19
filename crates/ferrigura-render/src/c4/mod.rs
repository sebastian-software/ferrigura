//! The C4 diagram: a port of `packages/mermaid/src/diagrams/c4/c4Renderer.ts`, `svgDraw.ts`,
//! `c4ShapeAdapter.ts` and `styles.js`, with the shared shapes of `rendering-util` that they use. The
//! research, with the reference's file and line for every rule, is in `docs-port/c4/NOTES.md` (section
//! 6); the parser and the DB are in `docs-port/c4/PLAN.md`.
//!
//! The layout is C4's own grid (`grid`), not ELK. It compares absolute coordinates with the width of
//! the screen (1200 px, `draw::SCREEN_WIDTH`), so the output does not depend on the viewer's screen. The
//! text is measured with the bundled face wherever the reference names a font of its own.

mod config;
mod draw;
mod grid;
mod icons;
mod label;
mod legacy;
mod relation;
mod shape;
mod style;

use std::collections::HashMap;

use ferrigura_model::c4::C4Diagram;
use ferrigura_model::config::C4DiagramConfig;
use ferrigura_text::{Run, embed_runs};

use crate::RenderError;
use crate::a11y;
use crate::common::decode_entities;
use crate::common::style::base_style_sheet_tokens;
use crate::common::text::TextMemo;
use crate::common::viewbox::calculate_svg_size_attrs;
use crate::context::Context;
use crate::svg::{escape, number};

use draw::{Draw, FAMILY, SCREEN_WIDTH};

/// The XML namespace of the root element.
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// The height that the title adds to the view box (`extraVertForTitle`, `c4Renderer.ts:642`).
const TITLE_HEIGHT: f64 = 60.0;

/// Renders the diagram to an SVG document.
///
/// # Errors
///
/// Returns an error for a shape or a label the renderer does not draw yet (the framed rectangle, a
/// named label), for a relation whose ends are not drawn, or if a bundled font cannot be read.
pub fn render(input: &Context<'_, C4Diagram, C4DiagramConfig>) -> Result<String, RenderError> {
    let conf = config::Conf::new(input.section);
    let tokens = input.theme.light;
    let radius = tokens
        .radius
        .value()
        .and_then(ferrigura_model::value::Value::as_f64)
        .unwrap_or(5.0);
    let diagram = input.diagram;

    let mut draw = Draw {
        conf: &conf,
        model: diagram,
        id: input.id,
        radius,
        shape_in_row: diagram.c4_shape_in_row,
        boundary_in_row: diagram.c4_boundary_in_row,
        memo: TextMemo::new(),
        max_x: conf.diagram_margin_x,
        max_y: conf.diagram_margin_y,
        body: String::new(),
        shapes: HashMap::new(),
        boundaries: HashMap::new(),
        runs: Vec::new(),
    };

    // The screen: its grid starts at the diagram margins and is as wide as the screen.
    let mut screen = grid::Bounds::new(SCREEN_WIDTH);
    screen.set_data(
        conf.diagram_margin_x,
        conf.diagram_margin_x,
        conf.diagram_margin_y,
        conf.diagram_margin_y,
    );
    let top = draw.boundaries_of("");
    draw.draw_inside(&mut screen, &top)?;

    screen.data.stopx = Some(draw.max_x);
    screen.data.stopy = Some(draw.max_y);
    let box_start_x = screen.data.startx.unwrap_or(conf.diagram_margin_x);
    let box_start_y = screen.data.starty.unwrap_or(conf.diagram_margin_y);
    let box_width = draw.max_x - box_start_x;
    let box_height = draw.max_y - box_start_y;
    let width = box_width + 2.0 * conf.diagram_margin_x;
    let height = box_height + 2.0 * conf.diagram_margin_y;

    let relations = draw.draw_rels()?;

    let title = diagram.title.as_str();
    let title_svg = if title.is_empty() {
        String::new()
    } else {
        format!(
            "<text x=\"{}\" y=\"{}\">{}</text>",
            number(box_width / 2.0 - 4.0 * conf.diagram_margin_x),
            number(box_start_y + conf.diagram_margin_y),
            decode_entities(&escape(title)),
        )
    };
    let extra = if title.is_empty() { 0.0 } else { TITLE_HEIGHT };
    let view_box = format!(
        "{} -{} {} {}",
        number(box_start_x - conf.diagram_margin_x),
        number(conf.diagram_margin_y + extra),
        number(width),
        number(height + extra),
    );

    // The font: the subsets of the diagram's text, named once they are made (decision 0005). The title
    // is set in the root's face, so its glyphs are part of the subset too.
    if !title.is_empty() {
        draw.runs.push((
            decode_entities(title),
            ferrigura_text::face(
                ferrigura_text::FontFamily::Sans,
                ferrigura_text::FontWeight::Normal,
                ferrigura_text::FontStyle::Normal,
            ),
        ));
    }
    let runs: Vec<Run<'_>> = draw
        .runs
        .iter()
        .map(|(text, face)| Run { text, face })
        .collect();
    let families = embed_runs(&runs)?;
    let family_css = families.first().map_or_else(
        || {
            format!(
                "\"{}\"",
                tokens
                    .font_family
                    .text()
                    .unwrap_or_else(|| "sans-serif".to_owned())
            )
        },
        |family| family.css_family.clone(),
    );
    let font_size = tokens.font_size.text().unwrap_or_else(|| "16px".to_owned());
    let person_border = tokens.person_border.text().unwrap_or_default();
    let person_background = tokens.person_bkg.text().unwrap_or_default();
    let own = style::own_rules(
        input.id,
        &conf,
        &family_css,
        &person_border,
        &person_background,
    );
    let style_sheet = base_style_sheet_tokens(
        input.id,
        input.theme,
        &family_css,
        &font_size,
        &families,
        &own,
    );

    let sizes = calculate_svg_size_attrs(height, width, conf.use_max_width);
    let mut size_attributes = String::new();
    for (name, value) in &sizes {
        if *name == "width" {
            size_attributes.push_str(&format!(" width=\"{}\"", escape(value)));
        }
    }
    let mut style_attributes = String::new();
    for (name, value) in &sizes {
        if *name != "width" {
            style_attributes.push_str(&format!(" {name}=\"{}\"", escape(value)));
        }
    }
    let accessible = a11y::root_attributes(input.id, "c4", "", &diagram.acc_descr);
    let accessibility = a11y::children(input.id, "", &diagram.acc_descr);

    let body = format!(
        "{}{}{}{}{}",
        icons::defs(input.id),
        draw.body,
        markers(input.id),
        relations,
        title_svg,
    );
    let body = body.replace(FAMILY, &family_css);
    Ok(format!(
        "<svg id=\"{id}\"{size_attributes} xmlns=\"{SVG_NAMESPACE}\"{style_attributes} viewBox=\"{view_box}\"{accessible}>{accessibility}<style>{style_sheet}</style><g></g>{body}</svg>",
        id = escape(input.id),
    ))
}

/// The four arrow markers, each in its own `defs`, as `svgDraw.ts:261-340` writes them.
fn markers(id: &str) -> String {
    let id = escape(id);
    format!(
        "<defs><marker id=\"{id}-arrowhead\" refX=\"9\" refY=\"5\" markerUnits=\"userSpaceOnUse\" markerWidth=\"12\" markerHeight=\"12\" orient=\"auto\"><path d=\"M 0 0 L 10 5 L 0 10 z\"></path></marker></defs>\
<defs><marker id=\"{id}-arrowend\" refX=\"1\" refY=\"5\" markerUnits=\"userSpaceOnUse\" markerWidth=\"12\" markerHeight=\"12\" orient=\"auto\"><path d=\"M 10 0 L 0 5 L 10 10 z\"></path></marker></defs>\
<defs><marker id=\"{id}-crosshead\" markerWidth=\"15\" markerHeight=\"8\" orient=\"auto\" refX=\"16\" refY=\"4\"><path fill=\"black\" stroke=\"#000000\" stroke-width=\"1px\" d=\"M 9,2 V 6 L16,4 Z\" style=\"stroke-dasharray: 0, 0;\"></path><path fill=\"none\" stroke=\"#000000\" stroke-width=\"1px\" d=\"M 0,1 L 6,7 M 6,1 L 0,7\" style=\"stroke-dasharray: 0, 0;\"></path></marker></defs>\
<defs><marker id=\"{id}-filled-head\" refX=\"18\" refY=\"7\" markerWidth=\"20\" markerHeight=\"28\" orient=\"auto\"><path d=\"M 18,7 L9,13 L14,7 L9,1 Z\"></path></marker></defs>"
    )
}
