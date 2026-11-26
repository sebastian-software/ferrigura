//! The class diagram: a port of the unified renderer with the class `getData`
//! (`diagrams/class/classRenderer-v3-unified.ts`, `classDb.ts`). See
//! `docs/diagrams/class.md#layout-and-rendering`.

pub mod data;
pub mod member;
pub mod style;
pub mod tree;

use ferrigura_model::class::ClassDiagram;
use ferrigura_text::{
    FontStyle, FontWeight, Run, embed_runs, measure as measure_text, measure_box, text_box,
};

use crate::RenderError;
use crate::a11y;
use crate::context::Context;
use crate::svg::{escape, number, xlink_declaration};
use crate::unified::geom::BBox;
use crate::unified::markers;
use crate::unified::pipeline::{self, cover};
use crate::unified::{Config, Section};

/// The padding of the viewBox around the drawing: `setupViewPortForSVG(svg, 8, ...)`
/// (`classRenderer-v3-unified.ts` 75).
const VIEWPORT_PADDING: f64 = 8.0;

/// The font size of the title: `.classTitleText` in the class styles.
const TITLE_FONT_SIZE: f64 = 18.0;

/// Ensures rendered class DOM IDs stay unique after Mermaid entity placeholder resolution (for
/// example when a diagram defines both `Foo-Bar` and `` `Foo#45;Bar` ``), and rewrites matching
/// raw fragment links so each fragment targets its disambiguated node.
fn disambiguate_resolved_dom_ids(data: &mut crate::unified::data::LayoutData, svg_id: &str) {
    if !data.nodes.iter().any(|node| node.dom_id.contains("ﬂ°")) {
        return;
    }
    let reserved: std::collections::HashSet<String> = data
        .nodes
        .iter()
        .map(|node| crate::common::resolve_entity_placeholders(&node.dom_id).into_owned())
        .collect();
    let mut seen: std::collections::HashSet<String> =
        std::collections::HashSet::with_capacity(data.nodes.len());
    let mut remapped_fragments: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for node in &mut data.nodes {
        let resolved = crate::common::resolve_entity_placeholders(&node.dom_id).into_owned();
        if seen.insert(resolved.clone()) {
            continue;
        }
        let mut suffix = 1usize;
        let unique = loop {
            let candidate = format!("{resolved}-{suffix}");
            if !reserved.contains(&candidate) && seen.insert(candidate.clone()) {
                break candidate;
            }
            suffix += 1;
        };
        let raw_fragment = format!("#{svg_id}-{}", node.dom_id);
        let new_fragment = format!("#{svg_id}-{unique}");
        remapped_fragments.insert(raw_fragment, new_fragment);
        node.dom_id = unique;
    }
    if !remapped_fragments.is_empty() {
        for node in &mut data.nodes {
            if let Some(link) = &mut node.link
                && let Some(replacement) = remapped_fragments.get(link.as_str())
            {
                *link = replacement.clone();
            }
        }
    }
}

/// Renders the diagram to an SVG document.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read, a shape is unknown, or the layout fails.
pub fn render(input: &Context<'_, ClassDiagram, Section<'_>>) -> Result<String, RenderError> {
    let config = Config::of(input);
    let id = input.id;
    let diagram = input.diagram;

    let mut data = data::get_data(diagram, input.section, input.globals, input.theme.light);
    disambiguate_resolved_dom_ids(&mut data, id);
    // `classRenderer-v3-unified.ts` 59: the diagram type names the markers and their class.
    data.diagram_type = markers::CLASS.to_owned();
    let design = input.section.design;
    if design.is_some() {
        orient_relations(&mut data);
    }
    let mut measured = pipeline::measure(data, &config)?;
    pipeline::lay_out_for_render(&mut measured, &config)?;
    let painted = pipeline::paint(&measured, id, &config)?;
    let mut bounds = painted.bounds;

    // `utils.insertTitle` (`classRenderer-v3-unified.ts` 68-73): centered over what is drawn, the
    // state section's `titleTopMargin` above the origin.
    let sans = input
        .text_fonts()
        .face(FontWeight::Normal, FontStyle::Normal)?;
    let title = &diagram.title;
    let mut title_svg = String::new();
    let title_font = sans.css_reference().map_or_else(String::new, |family| {
        format!(" style=\"font-family:{family}\"")
    });
    if !title.is_empty() {
        let drawn = bounds.unwrap_or_default();
        let width = measure_text(&sans, title, TITLE_FONT_SIZE)?;
        let ink = measure_box(&sans, title, TITLE_FONT_SIZE)?;
        let text_box = text_box(&sans, TITLE_FONT_SIZE)?;
        let margin = input.section.state.title_top_margin as f64;
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
            "<text text-anchor=\"middle\" x=\"{}\" y=\"{}\" class=\"classDiagramTitleText\"{title_font}>{}</text>",
            number(centre),
            number(baseline),
            escape(title),
        );
    }

    let mut runs: Vec<Run<'_, '_>> = painted
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
            face: &sans,
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
    let accessibility = a11y::children(id, &diagram.acc_title, &diagram.acc_descr);
    let accessible_attributes =
        a11y::root_attributes(id, "classDiagram", &diagram.acc_title, &diagram.acc_descr);
    let style = style::style_sheet(id, input.theme, input.globals, &families, design);
    let markers = if design.is_some() {
        markers::design_class_markers(id)
    } else {
        markers::class_markers(id)
    };
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\"{xlink} id=\"{id}\" {size_attributes} class=\"classDiagram\" viewBox=\"{x} {y} {w} {h}\"{accessible_attributes}>{accessibility}<style>{style}</style><g>{markers}{root}{coloured_markers}</g>{title_svg}{defs}</svg>",
        id = escape(id),
        xlink = xlink_declaration(painted.links),
        x = number(drawn.x - VIEWPORT_PADDING),
        y = number(drawn.y - VIEWPORT_PADDING),
        w = number(view_width),
        h = number(view_height),
        root = painted.root,
        coloured_markers = painted.coloured_markers,
        defs = style::defs(id, input.theme, input.globals, design),
    ))
}

/// Turns each relation whose whole or general end is its second one, in the design mode
/// (`docs/decisions/0023-design-mode.md`): the layout runs an edge from its start to its end, so
/// `Dog --|> Animal` would put the parent below its child and `Wheel --* Car` the whole below its
/// part. A relation with a triangle or a diamond at its end and none at its start is drawn from the
/// other end: its ends, its markers and its cardinalities change places, and the drawing says the
/// same. A relation with such a marker at both ends, or with none, keeps the direction it is written
/// in.
fn orient_relations(data: &mut crate::unified::data::LayoutData) {
    let general = |arrow: &str| matches!(arrow, "extension" | "composition" | "aggregation");
    for edge in &mut data.edges {
        if general(&edge.arrow_type_end) && !general(&edge.arrow_type_start) {
            std::mem::swap(&mut edge.start, &mut edge.end);
            std::mem::swap(&mut edge.arrow_type_start, &mut edge.arrow_type_end);
            std::mem::swap(&mut edge.start_label_right, &mut edge.end_label_left);
        }
    }
}
