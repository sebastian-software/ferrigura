//! The frame of the map: the background, the title, the two axes and their labels, the stages and
//! the grid (`wardleyRenderer.ts` 140-295). Each group and element is written in the reference's
//! order, and each text adds its shown characters to the font subset.

use super::Canvas;
use super::text::{Drawn, label};
use crate::common::Element;
use crate::common::d3::js_number;

/// The stages of a map with no `evolution` statement (`wardleyRenderer.ts` 12).
const DEFAULT_STAGES: [&str; 4] = ["Genesis", "Custom Built", "Product", "Commodity"];

/// The label of the x axis. The reference reads `axes.xLabel`, which no statement sets.
const X_LABEL: &str = "Evolution";
/// The label of the y axis. The reference reads `axes.yLabel`, which no statement sets.
const Y_LABEL: &str = "Visibility";

/// `rect.wardley-background`: the canvas, in the background colour (`wardleyRenderer.ts` 140-145).
#[must_use]
pub fn background(canvas: &Canvas<'_>) -> Element {
    Element::new("rect")
        .with("class", "wardley-background")
        .with("width", js_number(canvas.plot.width))
        .with("height", js_number(canvas.plot.height))
        .with("fill", canvas.colours.background.as_str())
}

/// `text.wardley-title`, when the diagram has a title (`wardleyRenderer.ts` 151-163).
#[must_use]
pub fn title(canvas: &Canvas<'_>, drawn: &mut Drawn) -> Option<Element> {
    let title = canvas.diagram.title.as_str();
    if title.is_empty() {
        return None;
    }
    drawn.bold(title);
    Some(label(
        title,
        vec![
            ("class", "wardley-title".to_owned()),
            ("x", js_number(canvas.plot.width / 2.0)),
            ("y", js_number(canvas.config.padding / 2.0)),
            ("fill", canvas.colours.axis_text.clone()),
            ("font-size", js_number(canvas.config.axis_font_size * 1.05)),
            ("font-weight", "bold".to_owned()),
            ("text-anchor", "middle".to_owned()),
            ("dominant-baseline", "middle".to_owned()),
        ],
    ))
}

/// `g.wardley-axes`: the two axis lines and their labels (`wardleyRenderer.ts` 168-212).
#[must_use]
pub fn axes(canvas: &Canvas<'_>, drawn: &mut Drawn) -> Element {
    let plot = &canvas.plot;
    let colours = &canvas.colours;
    let padding = plot.padding;
    let bottom = plot.height - padding;
    let mut group = Element::new("g").with("class", "wardley-axes");
    group.push_child(
        Element::new("line")
            .with("x1", js_number(padding))
            .with("x2", js_number(plot.width - padding))
            .with("y1", js_number(bottom))
            .with("y2", js_number(bottom))
            .with("stroke", colours.axis.as_str())
            .with("stroke-width", "1"),
    );
    group.push_child(
        Element::new("line")
            .with("x1", js_number(padding))
            .with("x2", js_number(padding))
            .with("y1", js_number(padding))
            .with("y2", js_number(bottom))
            .with("stroke", colours.axis.as_str())
            .with("stroke-width", "1"),
    );
    drawn.bold(X_LABEL);
    drawn.bold(Y_LABEL);
    let axis_font = js_number(canvas.config.axis_font_size);
    group.push_child(label(
        X_LABEL,
        vec![
            (
                "class",
                "wardley-axis-label wardley-axis-label-x".to_owned(),
            ),
            ("x", js_number(padding + plot.chart_width / 2.0)),
            ("y", js_number(plot.height - padding / 4.0)),
            ("fill", colours.axis_text.clone()),
            ("font-size", axis_font.clone()),
            ("font-weight", "bold".to_owned()),
            ("text-anchor", "middle".to_owned()),
        ],
    ));
    let y_x = padding / 3.0;
    let y_y = padding + plot.chart_height / 2.0;
    group.push_child(label(
        Y_LABEL,
        vec![
            (
                "class",
                "wardley-axis-label wardley-axis-label-y".to_owned(),
            ),
            ("x", js_number(y_x)),
            ("y", js_number(y_y)),
            ("fill", colours.axis_text.clone()),
            ("font-size", axis_font),
            ("font-weight", "bold".to_owned()),
            ("text-anchor", "middle".to_owned()),
            (
                "transform",
                format!("rotate(-90 {} {})", js_number(y_x), js_number(y_y)),
            ),
        ],
    ));
    group
}

/// The share of the chart's width where each stage starts and ends. The boundaries of the statement
/// place the stages when there is one for each stage; otherwise the stages are of equal width
/// (`wardleyRenderer.ts` 221-238).
fn spans(canvas: &Canvas<'_>, stage_count: usize) -> Vec<(f64, f64)> {
    match canvas
        .diagram
        .axes
        .stage_boundaries
        .as_ref()
        .filter(|boundaries| boundaries.len() == stage_count)
    {
        Some(boundaries) => {
            let mut previous = 0.0;
            boundaries
                .iter()
                .map(|boundary| {
                    let span = (previous, boundary.0);
                    previous = boundary.0;
                    span
                })
                .collect()
        }
        None => {
            let width = 1.0 / stage_count as f64;
            (0..stage_count)
                .map(|index| (index as f64 * width, (index + 1) as f64 * width))
                .collect()
        }
    }
}

/// `g.wardley-stages`: a dashed divider before each stage but the first, and the stage's name
/// centred under it (`wardleyRenderer.ts` 214-271). The stages are the statement's, or the four of
/// the default when it gives none.
#[must_use]
pub fn stages(canvas: &Canvas<'_>, drawn: &mut Drawn) -> Element {
    let stages: Vec<String> = match &canvas.diagram.axes.stages {
        Some(stages) if !stages.is_empty() => stages.clone(),
        _ => DEFAULT_STAGES
            .iter()
            .map(|stage| (*stage).to_owned())
            .collect(),
    };
    let plot = &canvas.plot;
    let padding = plot.padding;
    let bottom = plot.height - padding;
    let mut group = Element::new("g").with("class", "wardley-stages");
    for (index, (stage, (start, end))) in stages.iter().zip(spans(canvas, stages.len())).enumerate()
    {
        let start_x = padding + start * plot.chart_width;
        let end_x = padding + end * plot.chart_width;
        let centre_x = (start_x + end_x) / 2.0;
        if index > 0 {
            group.push_child(
                Element::new("line")
                    .with("x1", js_number(start_x))
                    .with("x2", js_number(start_x))
                    .with("y1", js_number(padding))
                    .with("y2", js_number(bottom))
                    .with("stroke", "#000")
                    .with("stroke-width", "1")
                    .with("stroke-dasharray", "5 5")
                    .with("opacity", "0.8"),
            );
        }
        drawn.regular(stage);
        group.push_child(label(
            stage,
            vec![
                ("class", "wardley-stage-label".to_owned()),
                ("x", js_number(centre_x)),
                ("y", js_number(plot.height - padding / 1.5)),
                ("fill", canvas.colours.axis_text.clone()),
                ("font-size", js_number(canvas.config.axis_font_size - 2.0)),
                ("text-anchor", "middle".to_owned()),
            ],
        ));
    }
    group
}

/// `g.wardley-grid`, when the configuration asks for it: three dotted lines of each direction, at a
/// quarter, a half and three quarters (`wardleyRenderer.ts` 273-295).
#[must_use]
pub fn grid(canvas: &Canvas<'_>) -> Element {
    let plot = &canvas.plot;
    let colours = &canvas.colours;
    let padding = plot.padding;
    let bottom = plot.height - padding;
    let mut group = Element::new("g").with("class", "wardley-grid");
    for step in 1..4 {
        let ratio = f64::from(step) / 4.0;
        let x = padding + plot.chart_width * ratio;
        group.push_child(
            Element::new("line")
                .with("x1", js_number(x))
                .with("x2", js_number(x))
                .with("y1", js_number(padding))
                .with("y2", js_number(bottom))
                .with("stroke", colours.grid.as_str())
                .with("stroke-dasharray", "2 6"),
        );
        let y = plot.height - padding - plot.chart_height * ratio;
        group.push_child(
            Element::new("line")
                .with("x1", js_number(padding))
                .with("x2", js_number(plot.width - padding))
                .with("y1", js_number(y))
                .with("y2", js_number(y))
                .with("stroke", colours.grid.as_str())
                .with("stroke-dasharray", "2 6"),
        );
    }
    group
}
