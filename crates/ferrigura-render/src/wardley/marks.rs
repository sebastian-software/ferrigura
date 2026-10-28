//! The notes (`g.wardley-notes`) and the accelerators and deaccelerators: the large arrows that point
//! right or left, with their names below them (`wardleyRenderer.ts` 958-1066).

use ferrigura_model::wardley::Mark;

use super::Canvas;
use super::text::{Drawn, label};
use crate::common::Element;
use crate::common::d3::js_number;

/// The length of an arrow, from its tail to its tip (`wardleyRenderer.ts` 987).
const ARROW_WIDTH: f64 = 60.0;
/// The height of an arrow's shaft (`:988`).
const ARROW_HEIGHT: f64 = 30.0;
/// The width of an arrow's head (`:989`).
const ARROW_HEAD_WIDTH: f64 = 20.0;
/// How far the head's barbs stand out from the shaft (the `- 8` and `+ 8` of the path).
const ARROW_BARB: f64 = 8.0;

/// `g.wardley-notes`, when the diagram has a note (`wardleyRenderer.ts` 958-976).
#[must_use]
pub fn notes(canvas: &Canvas<'_>, drawn: &mut Drawn) -> Option<Element> {
    if canvas.diagram.notes.is_empty() {
        return None;
    }
    let mut group = Element::new("g").with("class", "wardley-notes");
    for note in &canvas.diagram.notes {
        drawn.bold(&note.text);
        group.push_child(label(
            &note.text,
            vec![
                ("x", js_number(canvas.plot.x(note.x))),
                ("y", js_number(canvas.plot.y(note.y))),
                ("text-anchor", "start".to_owned()),
                ("font-size", "11".to_owned()),
                ("fill", canvas.colours.axis_text.clone()),
                ("font-weight", "bold".to_owned()),
            ],
        ));
    }
    Some(group)
}

/// `g.wardley-accelerators`, when the diagram has an accelerator (`wardleyRenderer.ts` 978-1021).
#[must_use]
pub fn accelerators(canvas: &Canvas<'_>, drawn: &mut Drawn) -> Option<Element> {
    arrows(
        canvas,
        drawn,
        &canvas.diagram.accelerators,
        "wardley-accelerators",
        true,
    )
}

/// `g.wardley-deaccelerators`, when the diagram has a deaccelerator (`wardleyRenderer.ts`
/// 1023-1066).
#[must_use]
pub fn deaccelerators(canvas: &Canvas<'_>, drawn: &mut Drawn) -> Option<Element> {
    arrows(
        canvas,
        drawn,
        &canvas.diagram.deaccelerators,
        "wardley-deaccelerators",
        false,
    )
}

/// The group of the arrows of one kind: a path and its name for each mark, in the order of the text.
/// An accelerator points right from its position, a deaccelerator left.
fn arrows(
    canvas: &Canvas<'_>,
    drawn: &mut Drawn,
    marks: &[Mark],
    class: &str,
    right: bool,
) -> Option<Element> {
    if marks.is_empty() {
        return None;
    }
    let colours = &canvas.colours;
    let mut group = Element::new("g").with("class", class);
    for mark in marks {
        let x = canvas.plot.x(mark.x);
        let y = canvas.plot.y(mark.y);
        let d = if right {
            accelerator_path(x, y)
        } else {
            deaccelerator_path(x, y)
        };
        group.push_child(
            Element::new("path")
                .with("d", d)
                .with("fill", "white")
                .with("stroke", colours.component_stroke.as_str())
                .with("stroke-width", "1"),
        );
        drawn.bold(&mark.name);
        group.push_child(label(
            &mark.name,
            vec![
                ("x", js_number(x + ARROW_WIDTH / 2.0)),
                ("y", js_number(y + ARROW_HEIGHT / 2.0 + 15.0)),
                ("text-anchor", "middle".to_owned()),
                ("font-size", "10".to_owned()),
                ("fill", colours.axis_text.clone()),
                ("font-weight", "bold".to_owned()),
            ],
        ));
    }
    Some(group)
}

/// The outline of an accelerator at `(x, y)`, as the reference's template writes it: the shaft, the
/// head, and back, with the template's line breaks and indentation (`wardleyRenderer.ts` 992-1001).
fn accelerator_path(x: f64, y: f64) -> String {
    let (top, bottom) = (y - ARROW_HEIGHT / 2.0, y + ARROW_HEIGHT / 2.0);
    let head = x + ARROW_WIDTH - ARROW_HEAD_WIDTH;
    format!(
        "\n        M {} {}\n        L {} {}\n        L {} {}\n        L {} {}\n        L {} {}\n        L {} {}\n        L {} {}\n        Z\n      ",
        js_number(x),
        js_number(top),
        js_number(head),
        js_number(top),
        js_number(head),
        js_number(top - ARROW_BARB),
        js_number(x + ARROW_WIDTH),
        js_number(y),
        js_number(head),
        js_number(bottom + ARROW_BARB),
        js_number(head),
        js_number(bottom),
        js_number(x),
        js_number(bottom),
    )
}

/// The outline of a deaccelerator at `(x, y)`, the accelerator's mirror image: its tip is at `x`,
/// its tail at `x + 60` (`wardleyRenderer.ts` 1037-1046).
fn deaccelerator_path(x: f64, y: f64) -> String {
    let (top, bottom) = (y - ARROW_HEIGHT / 2.0, y + ARROW_HEIGHT / 2.0);
    let head = x + ARROW_HEAD_WIDTH;
    format!(
        "\n        M {} {}\n        L {} {}\n        L {} {}\n        L {} {}\n        L {} {}\n        L {} {}\n        L {} {}\n        Z\n      ",
        js_number(x + ARROW_WIDTH),
        js_number(top),
        js_number(head),
        js_number(top),
        js_number(head),
        js_number(top - ARROW_BARB),
        js_number(x),
        js_number(y),
        js_number(head),
        js_number(bottom + ARROW_BARB),
        js_number(head),
        js_number(bottom),
        js_number(x + ARROW_WIDTH),
        js_number(bottom),
    )
}
