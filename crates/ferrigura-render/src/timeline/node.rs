//! A node of the timeline: the section header, the task or the event. `drawNode`,
//! `getVirtualNodeHeight` and `defaultBkg` of `svgDraw.js` (498-610), in the redux branch, which is
//! the only one drawn (`docs-port/timeline/NOTES.md` 5.5).
//!
//! The reference measures the wrapped text of a node twice: once with `getVirtualNodeHeight`, when the
//! layout needs the largest height, outside the `section-N` groups; and once with `drawNode`, inside
//! them, where the stylesheet sets the weight of the node text. The two use the faces of
//! `Context::regular` and `Context::node_face`.

use super::text::{self, Extent};
use super::{Canvas, Context, translate};
use crate::RenderError;
use crate::common::Element;
use crate::common::viewbox::BBox;
use crate::svg::number;

/// The radius of the corners of a node: `r = 0` in the redux themes (`svgDraw.js` 588), so the path
/// has straight sides. The bottom-left corner is cut by `rd`, which is only used with the path.
const RD: f64 = 5.0;

/// What `drawNode` is given: the text, the width of the text, the padding and the least height.
#[derive(Debug, Clone, Copy)]
pub(super) struct NodeSpec<'a> {
    /// The text of the node.
    pub descr: &'a str,
    /// The width the text is wrapped to, before the padding is added.
    pub width: f64,
    /// The padding on each side.
    pub padding: f64,
    /// The least height of the node (`node.maxHeight`).
    pub max_height: f64,
}

/// A node drawn: its element, and its size and box in the node's own user units.
#[derive(Debug)]
pub(super) struct Node {
    /// The `g.timeline-node` element.
    pub element: Element,
    /// The width of the node with its padding on both sides.
    pub width: f64,
    /// The height of the node.
    pub height: f64,
    /// The box of the path and the text, in the node's user units.
    pub bounds: BBox,
}

/// `getVirtualNodeHeight`: the height the node would have, measured with the regular face and without
/// the least height. It draws nothing.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub(super) fn virtual_height(
    cx: &Context<'_>,
    descr: &str,
    width: f64,
    padding: f64,
) -> Result<f64, RenderError> {
    let laid = text::layout(cx, descr, width, cx.regular)?;
    Ok(extent_height(laid.extent) + cx.node_font_size * 1.1 * 0.5 + padding)
}

/// `drawNode` (svgDraw.js 498-568) with the redux branch. `full_section` is the colour number of the
/// node; `is_event` moves the text of an event up by the 3 of the redux event offset. The node's
/// path is named after `diagram_id`, or `undefined` for the vertical layout (NOTES 5.12, R2).
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub(super) fn draw(
    cx: &Context<'_>,
    canvas: &mut Canvas,
    spec: NodeSpec<'_>,
    full_section: usize,
    is_event: bool,
    diagram_id: Option<&str>,
) -> Result<Node, RenderError> {
    // `section = (fullSection % maxSections) - 1`: section 0 has the class `section--1`.
    let slot = full_section % cx.limit;
    let section = slot as i64 - 1;
    let laid = text::layout(cx, spec.descr, spec.width, cx.node_face)?;
    let mut height = extent_height(laid.extent) + cx.node_font_size * 1.1 * 0.5 + spec.padding;
    height = height.max(spec.max_height);
    let width = spec.width + 2.0 * spec.padding;

    // The text sits at the middle of the node, and an event's text is higher (redux, `svgDraw.js`
    // 529-533).
    let text_x = width / 2.0;
    let text_y = if is_event {
        spec.padding / 2.0 + 3.0
    } else {
        spec.padding
    };

    let colours = cx.primary.colours(slot);
    let bkg = background(
        canvas,
        diagram_id,
        height,
        width,
        colours.fill.as_str(),
        colours.stroke.as_str(),
    );

    let mut text = Element::new("text")
        .with("dy", "1em")
        .with("alignment-baseline", "middle")
        .with("dominant-baseline", "middle")
        .with("text-anchor", "middle")
        .with("fill", cx.primary.node_border.as_str());
    for tspan in &laid.tspans {
        text = text.child(
            Element::new("tspan")
                .with("x", "0")
                .with("dy", if tspan.first { "1em" } else { "1.1em" })
                .raw_text(super::markup(&tspan.text)),
        );
    }
    let text_group = Element::new("g")
        .with("transform", translate(text_x, text_y))
        .child(text);

    let mut node = Element::new("g").with("class", format!("timeline-node section-{section}"));
    if cx.neo {
        node.set("data-look", "neo");
        drop_shadow(canvas, cx);
    }
    let element = node.child(bkg).child(text_group);

    let mut bounds = BBox {
        x: 0.0,
        y: 0.0,
        width,
        height,
    };
    if let Some(extent) = laid.extent {
        bounds = super::union(bounds, text_box(extent, text_x, text_y));
    }
    Ok(Node {
        element,
        width,
        height,
        bounds,
    })
}

/// The box of the text of a node, given the translation of its text group.
fn text_box(extent: Extent, x: f64, y: f64) -> BBox {
    BBox {
        x: x - extent.half_width,
        y: y + extent.top,
        width: extent.half_width * 2.0,
        height: extent.bottom - extent.top,
    }
}

/// `defaultBkg` (svgDraw.js 586-595): the path of the node, in its own group, with the redux corner
/// radius of 0 (`M0 h-rd v-(h-rd) h W v h H0 Z`). The path gets the colours of its section as
/// presentation attributes, as well as the stylesheet's (decision 0007).
fn background(
    canvas: &mut Canvas,
    diagram_id: Option<&str>,
    height: f64,
    width: f64,
    fill: &str,
    stroke: &str,
) -> Element {
    let id = format!(
        "{}-node-{}",
        diagram_id.unwrap_or("undefined"),
        canvas.node_count
    );
    canvas.node_count += 1;
    let d = format!(
        "M0 {} v{} h{} v{} H0 Z",
        number(height - RD),
        number(-(height - RD)),
        number(width),
        number(height)
    );
    Element::new("g").child(
        Element::new("path")
            .with("id", id)
            .with("class", "node-bkg node-undefined")
            .with("d", d)
            .with("fill", fill)
            .with("stroke", stroke),
    )
}

/// The drop shadow of the neo look: a filter in `defs`, written once for the diagram
/// (`svgDraw.js` 546-563). The flood is white in a dark theme and black in a light one.
fn drop_shadow(canvas: &mut Canvas, cx: &Context<'_>) {
    if canvas.filter_added {
        return;
    }
    canvas.filter_added = true;
    let (opacity, colour) = if cx.dark {
        ("0.2", "#FFFFFF")
    } else {
        ("0.06", "#000000")
    };
    canvas.defs.push(
        Element::new("filter")
            .with("id", format!("{}-drop-shadow", cx.svg_id))
            .with("height", "130%")
            .with("width", "130%")
            .child(
                Element::new("feDropShadow")
                    .with("dx", "4")
                    .with("dy", "4")
                    .with("stdDeviation", "0")
                    .with("flood-opacity", opacity)
                    .with("flood-color", colour),
            ),
    );
}

/// The height of a text's box, which is 0 for a text with no glyph.
fn extent_height(extent: Option<Extent>) -> f64 {
    extent.map_or(0.0, |extent| extent.bottom - extent.top)
}
