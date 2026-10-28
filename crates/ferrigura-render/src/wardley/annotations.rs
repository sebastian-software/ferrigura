//! The annotations (`g.wardley-annotations`): the numbered circles on the map, the dashed lines that
//! join the coordinates of one annotation, and the box that lists the annotations' texts
//! (`wardleyRenderer.ts` 825-956).
//!
//! The box is the one place where the reference measures text. Its width is the widest line's
//! advance plus a fixed buffer of 105 px, so the box is wider than its text, and it is clamped to
//! the plot's margins with `Math.max` and `Math.min`, which give NaN for a NaN argument.

use ferrigura_model::wardley::{Annotation, Point};

use super::Canvas;
use super::text::{ANNOTATION_FONT_SIZE, Drawn, annotation_height, annotation_width, label};
use crate::RenderError;
use crate::common::Element;
use crate::common::d3::{js_number, math_max, math_min, sort_js};

/// The radius of an annotation's circle (`wardleyRenderer.ts` 861).
const CIRCLE_RADIUS: f64 = 10.0;
/// The padding of the box around its lines, on each side (`:884`, the local `padding`).
const BOX_PADDING: f64 = 10.0;
/// The line height of the box's lines (`:885`, `lineHeight`).
const BOX_LINE_HEIGHT: f64 = 16.0;
/// The buffer added to the widest line for the box's width (`:925`, "for safety").
const BOX_BUFFER: f64 = 105.0;
/// The font size of a number inside an annotation's circle (`:873`).
const NUMBER_FONT_SIZE: f64 = 10.0;

/// The point of the canvas where an annotation's coordinate is.
fn project(canvas: &Canvas<'_>, point: &Point) -> (f64, f64) {
    (canvas.plot.x(point.x), canvas.plot.y(point.y))
}

/// `g.wardley-annotations`, when the diagram has an annotation (`wardleyRenderer.ts` 826-878). The
/// annotations box follows the annotations in the group, and it is present when a statement gave
/// its position.
///
/// # Errors
///
/// Returns an error if the bundled face cannot be read or shaped.
pub fn draw(canvas: &Canvas<'_>, drawn: &mut Drawn) -> Result<Option<Element>, RenderError> {
    let annotations = &canvas.diagram.annotations;
    if annotations.is_empty() {
        return Ok(None);
    }
    let colours = &canvas.colours;
    let mut group = Element::new("g").with("class", "wardley-annotations");
    for annotation in annotations {
        let coordinates: Vec<(f64, f64)> = annotation
            .coordinates
            .iter()
            .map(|point| project(canvas, point))
            .collect();
        // The dashed lines join the coordinates of one annotation, in order.
        for pair in coordinates.windows(2) {
            group.push_child(
                Element::new("line")
                    .with("class", "wardley-annotation-line")
                    .with("x1", js_number(pair[0].0))
                    .with("y1", js_number(pair[0].1))
                    .with("x2", js_number(pair[1].0))
                    .with("y2", js_number(pair[1].1))
                    .with("stroke", colours.axis.as_str())
                    .with("stroke-width", "1.5")
                    .with("stroke-dasharray", "4 4"),
            );
        }
        let number = js_number(annotation.number.0);
        for (x, y) in coordinates {
            drawn.bold(&number);
            group.push_child(
                Element::new("g")
                    .with("class", "wardley-annotation")
                    .child(
                        Element::new("circle")
                            .with("cx", js_number(x))
                            .with("cy", js_number(y))
                            .with("r", js_number(CIRCLE_RADIUS))
                            .with("fill", "white")
                            .with("stroke", colours.axis.as_str())
                            .with("stroke-width", "1.5"),
                    )
                    .child(label(
                        &number,
                        vec![
                            ("x", js_number(x)),
                            ("y", js_number(y)),
                            ("text-anchor", "middle".to_owned()),
                            ("dominant-baseline", "central".to_owned()),
                            ("font-size", js_number(NUMBER_FONT_SIZE)),
                            ("fill", colours.axis_text.clone()),
                            ("font-weight", "bold".to_owned()),
                        ],
                    )),
            );
        }
    }
    if let Some(position) = canvas.diagram.annotations_box {
        group.push_child(annotations_box(canvas, annotations, position, drawn)?);
    }
    Ok(Some(group))
}

/// `g.wardley-annotations-box`: the box with the texts of the annotations that have one, sorted by
/// their numbers. The box is placed where the statement puts it, then clamped to the margins of
/// the plot (`wardleyRenderer.ts` 880-955).
fn annotations_box(
    canvas: &Canvas<'_>,
    annotations: &[Annotation],
    position: Point,
    drawn: &mut Drawn,
) -> Result<Element, RenderError> {
    let colours = &canvas.colours;
    let padding = canvas.config.padding;
    let mut box_x = canvas.plot.x(position.x);
    let mut box_y = canvas.plot.y(position.y);

    // The annotations with a text, by their numbers (`sortedAnnotations`).
    let mut sorted: Vec<&Annotation> = annotations
        .iter()
        .filter(|annotation| !annotation.text.is_empty())
        .collect();
    sort_js(&mut sorted, |a, b| a.number.0 - b.number.0);

    // The lines as the DOM holds them: the number and the text, with the placeholders of the text.
    let lines: Vec<String> = sorted
        .iter()
        .map(|annotation| format!("{}. {}", js_number(annotation.number.0), annotation.text))
        .collect();

    let mut group = Element::new("g").with("class", "wardley-annotations-box");
    if lines.is_empty() {
        return Ok(group);
    }

    // The widest line's advance (`getComputedTextLength`) and the text box's height (`getBBox`).
    let mut max_width = 0.0;
    for line in &lines {
        max_width = math_max(max_width, annotation_width(line)?);
    }
    let max_height = annotation_height()?;
    let box_width = max_width + BOX_PADDING * 2.0 + BOX_BUFFER;
    let box_height = lines.len() as f64 * BOX_LINE_HEIGHT + BOX_PADDING * 2.0 + max_height / 2.0;

    // The clamp to the margins of the plot.
    let min_x = padding;
    let max_x = canvas.plot.width - padding - box_width;
    let min_y = padding;
    let max_y = canvas.plot.height - padding - box_height;
    box_x = math_max(min_x, math_min(box_x, max_x));
    box_y = math_max(min_y, math_min(box_y, max_y));

    // The background first, then the lines: the rectangle is inserted before the first text.
    group.push_child(
        Element::new("rect")
            .with("x", js_number(box_x))
            .with("y", js_number(box_y))
            .with("width", js_number(box_width))
            .with("height", js_number(box_height))
            .with("fill", "white")
            .with("stroke", colours.axis.as_str())
            .with("stroke-width", "1.5")
            .with("rx", "4")
            .with("ry", "4"),
    );
    for (index, line) in lines.iter().enumerate() {
        drawn.regular(line);
        group.push_child(label(
            line,
            vec![
                ("x", js_number(box_x + BOX_PADDING)),
                (
                    "y",
                    js_number(box_y + BOX_PADDING + (index + 1) as f64 * BOX_LINE_HEIGHT),
                ),
                ("font-size", js_number(ANNOTATION_FONT_SIZE)),
                ("fill", colours.axis_text.clone()),
                ("text-anchor", "start".to_owned()),
                ("dominant-baseline", "middle".to_owned()),
            ],
        ));
    }
    Ok(group)
}
