//! The event modeling diagram: a port of `diagrams/eventmodeling/renderer.ts` and `styles.js`, with
//! the box texts of `db.ts` (`calculateTextProps`). The research is in
//! `docs-port/eventmodeling/NOTES.md`, sections 6 (the renderer) and 8.
//!
//! The drawing is in three parts: the layout (`layout`: the texts and their measured sizes, the
//! boxes, the lanes and the relations), the box texts as SVG text (`label`, decision 0004), and the
//! colours and rules (`style`). This file sizes the root `<svg>`, writes the elements in the order
//! the reference writes them, and embeds the fonts.
//!
//! The measurement is the reference's, with the bundled face (`docs-port/P2-RENDERERS.md`): the
//! names and data are measured in the family the reference names, as `ferrigura_text` measures the
//! bundled Lexica face at 16 px bold.

mod label;
mod layout;
mod style;

use ferrigura_model::config::EventModelingDiagramConfig;
use ferrigura_model::event_modeling::EventModelingDiagram;
use ferrigura_text::{EmbeddedFamily, FontFamily, Run, embed_runs, measure_box, text_box};

use self::label::{Cell, LabelLine, LineKind};
use self::layout::{Boxed, Layout};
use crate::RenderError;
use crate::a11y;
use crate::common::style::base_style_sheet;
use crate::common::viewbox::setup_graph_viewbox;
use crate::common::{Element, TextMemo};
use crate::context::Context;
use crate::journey::text::bold as bold_face;
use crate::svg::number;
use crate::unified::geom::{BBox, Point};

/// The XML namespace of the root element.
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// The x of a lane's label (`renderer.ts` 120-124).
const LANE_LABEL_X: f64 = 30.0;

/// The space the lanes add to the widest box on the right (`swimlanePadding`, `renderer.ts` 115).
const SWIMLANE_PADDING: f64 = 15.0;

/// The inset of a box's text cell from the box's rectangle (`boxPadding`, `renderer.ts` 32-33).
const BOX_PADDING: f64 = 10.0;

/// Renders the diagram to an SVG document.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read or subset, or a text cannot be measured.
pub fn render(
    input: &Context<'_, EventModelingDiagram, EventModelingDiagramConfig>,
) -> Result<String, RenderError> {
    let diagram = input.diagram;
    let config = input.section;
    let id = input.id;
    let tokens = input.theme.light;
    // The root's font size is the theme's, and the labels are drawn at it (`fontSize`, 14px in the
    // redux themes).
    let font_size = tokens.font_size.px().unwrap_or(16.0);
    let css_font_size = tokens.font_size.text().unwrap_or_else(|| "16px".to_owned());

    let mut memo = TextMemo::new();
    let layout = layout::lay_out(diagram, &mut memo)?;
    let boxed_labels = box_labels(&layout, font_size)?;

    // The fonts: the lanes' labels and the names are bold, the code is mono, and every glyph drawn
    // is in an embedded subset.
    let mut runs: Vec<Run<'_>> = Vec::new();
    for lane in &layout.lanes {
        runs.push(Run {
            text: &lane.label,
            face: bold_face(),
        });
    }
    for lines in &boxed_labels {
        for line in lines {
            for run in &line.runs {
                runs.push(Run {
                    text: &run.text,
                    face: match line.kind {
                        LineKind::Name => bold_face(),
                        LineKind::Code => label::code_face(run.style),
                    },
                });
            }
        }
    }
    let families: Vec<EmbeddedFamily> = embed_runs(&runs)?;
    let font_family = families.first().map_or_else(
        || {
            tokens
                .font_family
                .text()
                .unwrap_or_else(|| "sans-serif".to_owned())
        },
        |family| family.css_family.clone(),
    );
    let mono_family = families
        .iter()
        .find(|family| family.family == FontFamily::Mono)
        .map(|family| family.css_family.clone());
    let sheet = base_style_sheet(
        id,
        input.theme,
        &font_family,
        &css_font_size,
        &families,
        &format!("{}{}", style::tokens(id, input.theme), style::rules(id)),
    );

    let bounds = bounds_of(&layout, font_size)?;
    let mut root = Element::new("svg")
        .with("id", id)
        .with("width", "100%")
        .with("xmlns", SVG_NAMESPACE);
    setup_graph_viewbox(
        &mut root,
        crate::common::viewbox::BBox {
            x: bounds.x,
            y: bounds.y,
            width: bounds.width,
            height: bounds.height,
        },
        config.padding,
        config.use_max_width,
    );
    root.set("role", a11y::SVG_ROLE);
    root.set("aria-roledescription", "eventmodeling");
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

    let colours = style::colours(tokens);
    let entities = style::entity_colours(tokens);
    root = root
        .child(Element::new("style").raw_text(sheet))
        // The empty group `mermaidAPI` appends before the diagram draws.
        .child(Element::new("g"));
    for lane in &layout.lanes {
        root = root.child(lane_group(lane, layout.max_r, &colours));
    }
    for (boxed, lines) in layout.boxes.iter().zip(&boxed_labels) {
        root = root.child(box_group(
            boxed,
            &layout,
            lines,
            &entities,
            mono_family.as_deref(),
        ));
    }
    for relation in &layout.relations {
        root = root.child(
            Element::new("path")
                .with("class", "em-relation")
                .with("fill", "none")
                .with("stroke", colours.relation_stroke.as_str())
                .with("stroke-width", "1")
                .with("marker-end", format!("url(#em-arrowhead-{id})"))
                .with(
                    "d",
                    format!(
                        "M{} {} L{} {}",
                        number(relation.from.0),
                        number(relation.from.1),
                        number(relation.to.0),
                        number(relation.to.1)
                    ),
                ),
        );
    }
    root = root.child(arrowhead(id, &colours));
    Ok(root.render())
}

/// A lane: its background and its label (`renderer.ts` 99-126).
fn lane_group(lane: &layout::Lane, max_r: f64, colours: &style::Colours) -> Element {
    Element::new("g")
        .with("class", "em-swimlane")
        .child(
            Element::new("rect")
                .with("x", "0")
                .with("y", number(lane.y))
                .with("rx", "3")
                .with("width", number(max_r + SWIMLANE_PADDING))
                .with("height", number(lane.height))
                .with("fill", colours.swimlane_fill.as_str())
                .with("stroke", colours.swimlane_stroke.as_str()),
        )
        .child(
            Element::new("text")
                .with("font-weight", "bold")
                .with("x", number(LANE_LABEL_X))
                .with("y", number(lane.y + 30.0))
                .text(lane.label.as_str()),
        )
}

/// A box: its rectangle in its lane, and its text (`renderer.ts` 12-50). The text is a `switch`
/// holding SVG text (decision 0004), where the reference has a `foreignObject`.
fn box_group(
    boxed: &Boxed,
    layout: &Layout,
    lines: &[LabelLine],
    entities: &style::EntityColours,
    mono_family: Option<&str>,
) -> Element {
    let lane = &layout.lanes[boxed.lane];
    let visual = entities.of(&boxed.entity_type);
    let (fill_token, stroke_token) = style::box_token(&boxed.entity_type);
    let rect = Element::new("rect")
        .with("x", number(boxed.x))
        .with("y", number(lane.y + SWIMLANE_PADDING))
        .with("rx", "3")
        .with("width", number(boxed.width))
        .with("height", number(boxed.height))
        .with("stroke", visual.stroke.as_str())
        .with("fill", visual.fill.as_str())
        .with(
            "style",
            format!("fill:var({fill_token});stroke:var({stroke_token});"),
        );
    let mut text = Element::new("text")
        .with("class", "label")
        .with("fill", "#000000");
    // The first run of a line is placed at the line's start; the others follow it, each in its own
    // weight and slant (`tspan`s without a position continue the text).
    for line in lines {
        for (index, run) in line.runs.iter().enumerate() {
            let mut tspan = Element::new("tspan");
            if index == 0 {
                tspan = tspan.with("x", number(line.x)).with("y", number(line.y));
            }
            match line.kind {
                LineKind::Name => tspan.set("font-weight", "bold"),
                LineKind::Code => {
                    if let Some(family) = mono_family {
                        tspan.set("font-family", family);
                    }
                    if run.style.bold {
                        tspan.set("font-weight", "bold");
                    }
                    if run.style.italic {
                        tspan.set("font-style", "italic");
                    }
                }
            }
            text = text.child(tspan.text(run.text.as_str()));
        }
    }
    Element::new("g")
        .with("class", "em-box")
        .child(rect)
        .child(Element::new("switch").child(text))
}

/// The arrowhead of the relations, a marker of the diagram's id, written last (`renderer.ts`
/// 150-160).
fn arrowhead(id: &str, colours: &style::Colours) -> Element {
    Element::new("defs").child(
        Element::new("marker")
            .with("id", format!("em-arrowhead-{id}"))
            .with("markerWidth", "10")
            .with("markerHeight", "7")
            .with("refX", "10")
            .with("refY", "3.5")
            .with("orient", "auto")
            .child(
                Element::new("polygon")
                    .with("points", "0 0, 10 3.5, 0 7")
                    .with("fill", colours.arrowhead.as_str()),
            ),
    )
}

/// The lines of every box, in the box order: the cell of each box's text is the box's
/// `foreignObject` (`renderer.ts` 30-33).
fn box_labels(layout: &Layout, font_size: f64) -> Result<Vec<Vec<LabelLine>>, RenderError> {
    layout
        .boxes
        .iter()
        .map(|boxed| {
            let lane = &layout.lanes[boxed.lane];
            let cell = Cell {
                x: boxed.x + BOX_PADDING,
                y: lane.y + SWIMLANE_PADDING + BOX_PADDING,
                width: boxed.width - 2.0 * BOX_PADDING,
                height: boxed.height - 2.0 * BOX_PADDING,
            };
            label::lines_of(&boxed.text, cell, font_size)
        })
        .collect()
}

/// The box of the content, as `getBBox` takes it on the root: the lanes' rectangles, the lanes'
/// labels, the boxes' rectangles and the relations' points, rounded to float32 (`setupGraphViewbox`).
/// A box's text is not in it: the reference's `foreignObject` is inside the box's rectangle.
fn bounds_of(layout: &Layout, font_size: f64) -> Result<BBox, RenderError> {
    let mut bounds: Option<BBox> = None;
    let mut include = |extra: BBox| {
        if extra.is_empty() {
            return;
        }
        bounds = Some(bounds.map_or(extra, |so_far| so_far.union(extra)));
    };
    let lane_width = layout.max_r + SWIMLANE_PADDING;
    for lane in &layout.lanes {
        include(BBox::new(0.0, lane.y, lane_width, lane.height));
        let extent = measure_box(bold_face(), &lane.label, font_size)?;
        let metrics = text_box(bold_face(), font_size)?;
        let baseline = lane.y + 30.0;
        include(BBox::new(
            LANE_LABEL_X + extent.left,
            baseline - metrics.top,
            extent.width(),
            metrics.height,
        ));
    }
    for boxed in &layout.boxes {
        let lane = &layout.lanes[boxed.lane];
        include(BBox::new(
            boxed.x,
            lane.y + SWIMLANE_PADDING,
            boxed.width,
            boxed.height,
        ));
    }
    for relation in &layout.relations {
        if let Some(extra) = BBox::from_points([
            Point::new(relation.from.0, relation.from.1),
            Point::new(relation.to.0, relation.to.1),
        ]) {
            include(extra);
        }
    }
    Ok(bounds.unwrap_or_default().browser())
}
