//! The definitions the sequence diagram appends to its SVG before the drawing: the three icon
//! symbols, the arrowheads and the sequence number marker, and the drop shadow of the neo look
//! (`svgDraw.js:1631-1810`, `:2076-2140`). Each is a `defs` element, which holds no geometry of
//! its own but is referred to by id from the drawing.

use super::icons::{CLOCK_ICON, COMPUTER_ICON, DATABASE_ICON};
use crate::common::Element;

/// The `defs` of a symbol with one path: the icons of the diagram (`insertDatabaseIcon` …).
fn symbol(id: String, width: Option<&str>, path: &str, fill_rule: bool) -> Element {
    let mut symbol = Element::new("symbol").with("id", id);
    if fill_rule {
        symbol.set("fill-rule", "evenodd");
        symbol.set("clip-rule", "evenodd");
    }
    if let Some(width) = width {
        symbol.set("width", width);
        symbol.set("height", width);
    }
    Element::new("defs").child(symbol.child(
        Element::new("path")
            .with("transform", "scale(.5)")
            .with("d", path),
    ))
}

/// `insertDatabaseIcon`, `insertComputerIcon` and `insertClockIcon`, in that order.
#[must_use]
pub fn icons(id: &str) -> Vec<Element> {
    vec![
        symbol(format!("{id}-computer"), Some("24"), COMPUTER_ICON, false),
        symbol(format!("{id}-database"), None, DATABASE_ICON, true),
        symbol(format!("{id}-clock"), Some("24"), CLOCK_ICON, false),
    ]
}

/// A marker: its `refX`, `refY`, size and orientation, with the path it draws.
fn marker(id: String, ref_x: f64, ref_y: f64, size: (f64, f64), orient: &str, units: bool, path: Element) -> Element {
    let mut marker = Element::new("marker")
        .with("id", id)
        .with("refX", crate::svg::number(ref_x))
        .with("refY", crate::svg::number(ref_y));
    if units {
        marker.set("markerUnits", "userSpaceOnUse");
    }
    marker.set("markerWidth", crate::svg::number(size.0));
    marker.set("markerHeight", crate::svg::number(size.1));
    marker.set("orient", orient);
    marker.push_child(path);
    Element::new("defs").child(marker)
}

/// The `defs` of the arrowhead markers, in the order the reference appends them: the arrowhead,
/// the cross, the filled head, the sequence number, the four half arrows (`insertArrowHead`,
/// `insertArrowCrossHead`, `insertArrowFilledHead`, `insertSequenceNumber`, `insertSolidTopArrowHead`,
/// `insertSolidBottomArrowHead`, `insertStickTopArrowHead`, `insertStickBottomArrowHead`).
#[must_use]
pub fn arrow_markers(id: &str) -> Vec<Element> {
    let path = |d: &str| Element::new("path").with("d", d);
    vec![
        marker(
            format!("{id}-arrowhead"),
            7.9,
            5.0,
            (12.0, 12.0),
            "auto-start-reverse",
            true,
            path("M -1 0 L 10 5 L 0 10 z"),
        ),
        // The cross: a path with no fill, drawn with a stroke of 1pt.
        Element::new("defs").child(
            Element::new("marker")
                .with("id", format!("{id}-crosshead"))
                .with("markerWidth", "15")
                .with("markerHeight", "8")
                .with("orient", "auto")
                .with("refX", "4")
                .with("refY", "4.5")
                .child(
                    Element::new("path")
                        .with("fill", "none")
                        .with("stroke", "#000000")
                        .with("stroke-width", "1pt")
                        .with("style", "stroke-dasharray: 0, 0;")
                        .with("d", "M 1,2 L 6,7 M 6,2 L 1,7"),
                ),
        ),
        marker(
            format!("{id}-filled-head"),
            15.5,
            7.0,
            (20.0, 28.0),
            "auto",
            false,
            path("M 18,7 L9,13 L14,7 L9,1 Z"),
        ),
        marker(
            format!("{id}-sequencenumber"),
            15.0,
            15.0,
            (60.0, 40.0),
            "auto",
            false,
            Element::new("circle")
                .with("cx", "15")
                .with("cy", "15")
                .with("r", "6"),
        ),
        marker(
            format!("{id}-solidTopArrowHead"),
            7.9,
            7.25,
            (12.0, 12.0),
            "auto-start-reverse",
            true,
            path("M 0 0 L 10 8 L 0 8 z"),
        ),
        marker(
            format!("{id}-solidBottomArrowHead"),
            7.9,
            0.75,
            (12.0, 12.0),
            "auto-start-reverse",
            true,
            path("M 0 0 L 10 0 L 0 8 z"),
        ),
        marker(
            format!("{id}-stickTopArrowHead"),
            7.5,
            7.0,
            (12.0, 12.0),
            "auto-start-reverse",
            true,
            Element::new("path")
                .with("d", "M 0 0 L 7 7")
                .with("stroke", "black")
                .with("stroke-width", "1.5")
                .with("fill", "none"),
        ),
        marker(
            format!("{id}-stickBottomArrowHead"),
            7.5,
            0.0,
            (12.0, 12.0),
            "auto-start-reverse",
            true,
            Element::new("path")
                .with("d", "M 0 7 L 7 0")
                .with("stroke", "black")
                .with("stroke-width", "1.5")
                .with("fill", "none"),
        ),
    ]
}

/// The drop shadow of the neo look: a filter on the diagram's id (`insertDropShadow`). The flood
/// is black in the redux light theme and white otherwise.
#[must_use]
pub fn drop_shadow(id: &str, theme: &str) -> Element {
    let flood = if theme == "redux" || theme == "redux-color" {
        "#000000"
    } else {
        "#FFFFFF"
    };
    Element::new("defs").child(
        Element::new("filter")
            .with("id", format!("{id}-drop-shadow"))
            .with("height", "130%")
            .with("width", "130%")
            .child(
                Element::new("feDropShadow")
                    .with("dx", "4")
                    .with("dy", "4")
                    .with("stdDeviation", "0")
                    .with("flood-opacity", "0.06")
                    .with("flood-color", flood),
            ),
    )
}
