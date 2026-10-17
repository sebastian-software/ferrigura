//! The branches of an ishikawa diagram: a cause's diagonal line, its label, and the bones of its
//! descendants (`ishikawaRenderer.ts` 232-263 and 340-423).
//!
//! The descendants are placed in two orders: the even depths (horizontal bones) in pre-order, the
//! odd depths (diagonal bones) in post-order, so that each diagonal stays in its parent's wedge. The
//! upper side reads its children in reverse, at every depth, so the picture is mirror-symmetric.

use ferrigura_model::ishikawa::IshikawaNode;

use super::text::{Anchor, Baseline, Label, Style, wrap_text};
use super::{BONE_BASE, BONE_PER_CHILD, BONE_STUB, CAUSE_OFFSET, Context, Drawing, lerp};
use crate::RenderError;
use crate::common::Element;
use crate::svg::number;
use crate::unified::geom::BBox;

/// The maximum length of a sub-cause's line, in characters, before it wraps (`flattenTree`).
const SUB_CAUSE_CHARS: f64 = 15.0;

/// A descendant in the order the tree is walked: its depth (the cause is 1), its wrapped text, the
/// index of its parent among the descendants (`None` for a cause), and its number of children.
struct Entry {
    depth: usize,
    text: String,
    parent: Option<usize>,
    child_count: usize,
}

/// A bone the descendants hang from: a line from `(x0, y0)` to `(x1, y1)`, its number of children
/// and how many of them are drawn already (the diagonal bones are spaced by it).
#[derive(Clone, Copy)]
struct Bone {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    child_count: usize,
    children_drawn: usize,
}

/// The descendants of a cause, in the order `flattenTree` lists them, and the order their y
/// positions are given (`yOrder`). A cause's children are walked in reverse on the upper side.
fn flatten(children: &[IshikawaNode], reversed: bool) -> (Vec<Entry>, Vec<usize>) {
    let mut entries = Vec::new();
    let mut y_order = Vec::new();
    walk(children, None, 2, reversed, &mut entries, &mut y_order);
    (entries, y_order)
}

fn walk(
    nodes: &[IshikawaNode],
    parent: Option<usize>,
    depth: usize,
    reversed: bool,
    entries: &mut Vec<Entry>,
    y_order: &mut Vec<usize>,
) {
    let ordered: Vec<&IshikawaNode> = if reversed {
        nodes.iter().rev().collect()
    } else {
        nodes.iter().collect()
    };
    for child in ordered {
        let index = entries.len();
        entries.push(Entry {
            depth,
            text: wrap_text(&child.text, SUB_CAUSE_CHARS),
            parent,
            child_count: child.children.len(),
        });
        if depth.is_multiple_of(2) {
            // Even depths: pre-order, close to the spine.
            y_order.push(index);
            if !child.children.is_empty() {
                walk(
                    &child.children,
                    Some(index),
                    depth + 1,
                    reversed,
                    entries,
                    y_order,
                );
            }
        } else {
            // Odd depths: post-order, within the parent's diagonal.
            if !child.children.is_empty() {
                walk(
                    &child.children,
                    Some(index),
                    depth + 1,
                    reversed,
                    entries,
                    y_order,
                );
            }
            y_order.push(index);
        }
    }
}

/// The line of a branch or a bone, from `(x1, y1)` to `(x2, y2)`. A branch and a sub-branch take the
/// arrow of the marker at their start (`ishikawaRenderer.ts` 147).
fn line(
    cx: &Context,
    drawing: &mut Drawing,
    class: &str,
    from: (f64, f64),
    to: (f64, f64),
    marker: bool,
) -> Element {
    drawing.include(BBox::new(
        from.0.min(to.0),
        from.1.min(to.1),
        (from.0 - to.0).abs(),
        (from.1 - to.1).abs(),
    ));
    let mut element = Element::new("line")
        .with("class", class)
        .with("x1", number(from.0))
        .with("y1", number(from.1))
        .with("x2", number(to.0))
        .with("y2", number(to.1))
        .with("stroke", cx.line.as_str());
    if marker {
        element.set("marker-start", format!("url(#{})", cx.marker_id));
    }
    element
}

/// The label of a cause: its box is the group's first child, a rectangle 20 px wider on each side
/// and 2 px taller on each side (`ishikawaRenderer.ts` 304-309).
fn label_group(cx: &Context, drawing: &mut Drawing, label: &Label) -> Element {
    let bbox = label.bbox;
    let frame = BBox::new(
        bbox.x - 20.0,
        bbox.y - 2.0,
        bbox.width + 40.0,
        bbox.height + 4.0,
    );
    drawing.include(frame);
    let rect = Element::new("rect")
        .with("class", "ishikawa-label-box")
        .with("x", number(frame.x))
        .with("y", number(frame.y))
        .with("width", number(frame.width))
        .with("height", number(frame.height))
        .with("fill", cx.main.as_str())
        .with("stroke", cx.line.as_str());
    Element::new("g")
        .with("class", "ishikawa-label-group")
        .child(rect)
        .child(drawing.label(label, &cx.text))
}

/// The style of a cause's label: centred, on its middle (`.ishikawa-label.cause`).
fn cause_style(cx: &Context) -> Style {
    Style {
        class: "ishikawa-label cause",
        anchor_attribute: "middle",
        anchor: Anchor::Middle,
        baseline: Baseline::Middle,
        face: cx.regular,
        size: cx.label_size,
        line_height: cx.line_height(),
    }
}

/// Draws a cause and its descendants, from the spine at `start` in direction `direction` (`-1`
/// up, `1` down) with the length `length` of the side (`ishikawaRenderer.ts` 340-423). Returns the
/// elements of the branch, in the order the reference appends them, and the least `x` of its
/// texts, which the spine takes for the next pair.
pub(super) fn draw_branch(
    cx: &Context,
    drawing: &mut Drawing,
    node: &IshikawaNode,
    start: (f64, f64),
    direction: f64,
    length: f64,
) -> Result<(Vec<Element>, f64), RenderError> {
    let children = &node.children;
    let line_length = length * if children.is_empty() { 0.2 } else { 1.0 };
    let dx = -cx.cos_a * line_length;
    let dy = cx.sin_a * line_length * direction;
    let end = (start.0 + dx, start.1 + dy);

    let mut elements = vec![line(cx, drawing, "ishikawa-branch", start, end, true)];
    let cause = Label::new(
        cause_style(cx),
        &node.text,
        end.0,
        end.1 + CAUSE_OFFSET * direction,
    )?;
    let mut text_left = cause.bbox.x;
    elements.push(label_group(cx, drawing, &cause));
    if children.is_empty() {
        return Ok((elements, text_left));
    }

    let (entries, y_order) = flatten(children, direction < 0.0);
    let count = entries.len();
    // Each descendant's y: its slot in the order of `yOrder`, spaced along the branch.
    let mut ys = vec![0.0; count];
    for (slot, &index) in y_order.iter().enumerate() {
        ys[index] = start.1 + dy * ((slot as f64 + 1.0) / (count as f64 + 1.0));
    }

    // Bone 0 is the cause's own diagonal; the descendant `i` is bone `i + 1`.
    let mut bones: Vec<Option<Bone>> = vec![None; count + 1];
    bones[0] = Some(Bone {
        x0: start.0,
        y0: start.1,
        x1: end.0,
        y1: end.1,
        child_count: children.len(),
        children_drawn: 0,
    });
    let diagonal_x = -cx.cos_a;
    let diagonal_y = cx.sin_a * direction;
    let diagonal_label = if direction < 0.0 {
        ("ishikawa-label up", Baseline::Alphabetic)
    } else {
        ("ishikawa-label down", Baseline::Hanging)
    };

    for (index, entry) in entries.iter().enumerate() {
        let y = ys[index];
        let parent_slot = entry.parent.map_or(0, |parent| parent + 1);
        let parent = bones[parent_slot].expect("a descendant's parent has children");
        let (from, to, label) = if entry.depth.is_multiple_of(2) {
            // A horizontal bone, from the parent's line at height `y`, extending left.
            let parent_height = parent.y1 - parent.y0;
            let along = if parent_height != 0.0 && !parent_height.is_nan() {
                (y - parent.y0) / parent_height
            } else {
                0.5
            };
            let bx0 = lerp(parent.x0, parent.x1, along);
            let reach = if entry.child_count > 0 {
                BONE_BASE + entry.child_count as f64 * BONE_PER_CHILD
            } else {
                BONE_STUB
            };
            let bx1 = bx0 - reach;
            let style = Style {
                class: "ishikawa-label align",
                anchor_attribute: "end",
                anchor: Anchor::End,
                baseline: Baseline::Middle,
                face: cx.regular,
                size: cx.label_size,
                line_height: cx.line_height(),
            };
            ((bx0, y), (bx1, y), Label::new(style, &entry.text, bx1, y)?)
        } else {
            // A diagonal bone: from a point of the parent's line, spaced by the children the parent
            // has drawn already, towards the height `y`.
            let drawn = bones[parent_slot]
                .as_mut()
                .map(|bone| {
                    bone.children_drawn += 1;
                    bone.children_drawn - 1
                })
                .expect("a descendant's parent has children");
            let along =
                (parent.child_count as f64 - drawn as f64) / (parent.child_count as f64 + 1.0);
            let bx0 = lerp(parent.x0, parent.x1, along);
            let by0 = parent.y0;
            let bx1 = bx0 + diagonal_x * ((y - by0) / diagonal_y);
            let style = Style {
                class: diagonal_label.0,
                anchor_attribute: "end",
                anchor: Anchor::End,
                baseline: diagonal_label.1,
                face: cx.regular,
                size: cx.label_size,
                line_height: cx.line_height(),
            };
            (
                (bx0, by0),
                (bx1, y),
                Label::new(style, &entry.text, bx1, y)?,
            )
        };

        let mut group = Element::new("g").with("class", "ishikawa-sub-group");
        group.push_child(line(cx, drawing, "ishikawa-sub-branch", from, to, true));
        text_left = text_left.min(label.bbox.x);
        group.push_child(drawing.label(&label, &cx.text));
        elements.push(group);

        if entry.child_count > 0 {
            bones[index + 1] = Some(Bone {
                x0: from.0,
                y0: from.1,
                x1: to.0,
                y1: to.1,
                child_count: entry.child_count,
                children_drawn: 0,
            });
        }
    }
    Ok((elements, text_left))
}
