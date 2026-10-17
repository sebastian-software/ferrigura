//! The kanban section: a port of `kanbanSection` in `packages/mermaid/src/rendering-util/rendering-elements/clusters.js`
//! (308-417), the cluster shape the kanban renderer inserts for each section (`kanbanRenderer.ts`
//! 42-63).
//!
//! The section is a group of a rectangle and its title. The rectangle is `width` wide and centred on
//! `x`; its top is `top` (the section's `node.y - node.height / 2`, which is `-3 * width / 2`), and
//! its height is set by the renderer after the items are placed. The title is centred over the
//! rectangle: the label is laid out start-anchored, so its group moves by half its width
//! (clusters.js 391-396). The width rule of `insertCluster` (346-351) never widens a section,
//! because a section has no padding, so the rectangle keeps `width`.

use crate::svg::{escape, number};
use crate::unified::geom::BBox;
use crate::unified::label::LabelLayout;

use super::util::attribute;

/// The colours a section takes, from the theme's `cScale` slot of its number. `None` when the theme
/// has no slot for the section, in which case the rules give it no colour either.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionPaint<'a> {
    /// The fill and stroke of the rectangle.
    pub fill: &'a str,
    /// The colour of the title's text (`.section-N text`).
    pub label: &'a str,
    /// The colour of the title's group (`.cluster-label`, `textColor`).
    pub text: &'a str,
}

/// What a section draws.
#[derive(Debug, Clone, Copy)]
pub struct KanbanSection<'a> {
    /// The id of the section's group: `domId`, the diagram id and the section id.
    pub dom_id: &'a str,
    /// The number of the section, from 1: its class is `section-<number>`.
    pub number: usize,
    /// `data-look`: the configuration's look.
    pub look: &'a str,
    /// The centre of the section on x (`node.x`).
    pub x: f64,
    /// The width of the rectangle (`node.width`, 200 by default).
    pub width: f64,
    /// The top of the rectangle (`node.y - node.height / 2`).
    pub top: f64,
    /// The height of the rectangle, as the renderer sets it after the items are placed.
    pub height: f64,
    /// The corner radii (`node.rx`, `node.ry`).
    pub corner_radius: f64,
    /// The title, laid out at the section's width.
    pub label: &'a LabelLayout,
    /// The colours from the theme; `None` for a section without a colour slot.
    pub paint: Option<SectionPaint<'a>>,
}

/// A drawn section: its group's markup and the box of the group.
#[derive(Debug, Clone, PartialEq)]
pub struct KanbanSectionShape {
    /// The group, `g.cluster`, with the rectangle first and the title after it.
    pub svg: String,
    /// The box of the group in the root's coordinates: `getBBox` of the group, which is the union of
    /// the rectangle and the title.
    pub bbox: BBox,
    /// The box of the title in the root's coordinates, as `labelBBox` gives it (the renderer takes
    /// the tallest title for its layout).
    pub label_bbox: BBox,
}

/// Draws a section (`kanbanSection`).
#[must_use]
pub fn kanban_section(section: &KanbanSection<'_>) -> KanbanSectionShape {
    let label_size = section.label.bbox.browser();
    let rect_x = section.x - section.width / 2.0;
    let rect = BBox::new(rect_x, section.top, section.width, section.height);
    // `translate(node.x - bbox.width / 2, node.y - node.height / 2 + subGraphTitleTopMargin)`, where
    // the top margin is 0 (`flowchart.subGraphTitleMargin.top`).
    let label_offset = (section.x - label_size.width / 2.0, section.top);
    let label_bbox = label_size.translate(label_offset.0, label_offset.1);

    let mut rect_attributes = format!(
        " style=\"\" rx=\"{}\" ry=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"",
        number(section.corner_radius),
        number(section.corner_radius),
        number(rect_x),
        number(section.top),
        number(section.width),
        number(section.height),
    );
    let (label_style_attribute, cluster_label_attribute) = match section.paint {
        Some(paint) => {
            rect_attributes.push_str(&format!(
                " fill=\"{fill}\" stroke=\"{fill}\"",
                fill = escape(paint.fill)
            ));
            (
                format!(" fill=\"{}\"", escape(paint.label)),
                format!(" fill=\"{}\"", escape(paint.text)),
            )
        }
        None => (String::new(), String::new()),
    };
    let label = format!(
        "<g class=\"cluster-label\"{cluster_label_attribute} transform=\"translate({}, {})\">{}</g>",
        number(label_offset.0),
        number(label_offset.1),
        section.label.svg_parts("", "", &label_style_attribute),
    );
    let svg = format!(
        "<g class=\"cluster undefined section-{number}\" id=\"{id}\"{look}><rect{rect_attributes}></rect>{label}</g>",
        number = section.number,
        id = escape(section.dom_id),
        look = attribute("data-look", section.look),
    );
    // A title without ink adds no box (`getBBox` leaves it out of the union).
    let bbox = if label_size.is_empty() {
        rect
    } else {
        rect.union(label_bbox)
    };
    KanbanSectionShape {
        svg,
        bbox,
        label_bbox,
    }
}
