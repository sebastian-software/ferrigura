//! The sizing of the root `<svg>` of the custom renderers: `configureSvgSize` and
//! `setupGraphViewbox` of `packages/mermaid/src/setupGraphViewbox.js`.
//!
//! The reference measures the content with `getBBox` and then sets the size and the `viewBox` from
//! that box. ferrigura passes the box it computed for the content (the same numbers as the browser's
//! in the probed cases, `tools/port/text/common-fixture.ts`).

use crate::svg::number;

use super::Element;

/// A box in user units: `getBBox` of an element.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BBox {
    /// The left edge.
    pub x: f64,
    /// The top edge.
    pub y: f64,
    /// The width.
    pub width: f64,
    /// The height.
    pub height: f64,
}

/// `calculateSvgSizeAttrs`: with `use_max_width`, `width="100%"` and `style="max-width: …px;"`;
/// otherwise `height` and then `width` in pixels.
#[must_use]
pub fn calculate_svg_size_attrs(
    height: f64,
    width: f64,
    use_max_width: bool,
) -> Vec<(&'static str, String)> {
    if use_max_width {
        vec![
            ("width", "100%".to_owned()),
            ("style", format!("max-width: {}px;", number(width))),
        ]
    } else {
        vec![("height", number(height)), ("width", number(width))]
    }
}

/// `configureSvgSize`: applies [`calculate_svg_size_attrs`] to the root element.
pub fn configure_svg_size(element: &mut Element, height: f64, width: f64, use_max_width: bool) {
    for (name, value) in calculate_svg_size_attrs(height, width, use_max_width) {
        element.set(name, value);
    }
}

/// `setupGraphViewbox(graph, svgElem, padding, useMaxWidth)` with the content's box given: the size
/// is the box plus the padding on both sides, and the `viewBox` starts at the box's corner less the
/// padding. The size is set first, then the `viewBox`, as the reference does.
pub fn setup_graph_viewbox(element: &mut Element, bounds: BBox, padding: f64, use_max_width: bool) {
    let width = bounds.width + padding * 2.0;
    let height = bounds.height + padding * 2.0;
    configure_svg_size(element, height, width, use_max_width);
    let view_box = format!(
        "{} {} {} {}",
        number(bounds.x - padding),
        number(bounds.y - padding),
        number(bounds.width + 2.0 * padding),
        number(bounds.height + 2.0 * padding)
    );
    element.set("viewBox", view_box);
}

#[cfg(test)]
mod tests {
    use super::*;

    // The cases of the probe (`viewbox` and `sizes` in the fixture), with the box it measured.
    const PROBED_BOX: BBox = BBox {
        x: 5.0,
        y: 7.0,
        width: 120.5,
        height: 57.0,
    };

    #[test]
    fn a_size_with_max_width_is_a_percentage_and_a_style() {
        let mut svg = Element::new("svg");
        configure_svg_size(&mut svg, 100.0, 400.0, true);
        assert_eq!(
            svg.render(),
            "<svg width=\"100%\" style=\"max-width: 400px;\"/>"
        );
    }

    #[test]
    fn a_size_without_max_width_is_height_then_width() {
        let mut svg = Element::new("svg");
        configure_svg_size(&mut svg, 450.0, 300.5, false);
        assert_eq!(svg.render(), "<svg height=\"450\" width=\"300.5\"/>");
    }

    #[test]
    fn the_viewbox_with_max_width_matches_the_reference() {
        let mut svg = Element::new("svg");
        setup_graph_viewbox(&mut svg, PROBED_BOX, 8.0, true);
        assert_eq!(
            svg.render(),
            "<svg width=\"100%\" style=\"max-width: 136.5px;\" viewBox=\"-3 -1 136.5 73\"/>"
        );
    }

    #[test]
    fn the_viewbox_without_max_width_matches_the_reference() {
        let mut svg = Element::new("svg");
        setup_graph_viewbox(&mut svg, PROBED_BOX, 0.0, false);
        assert_eq!(
            svg.render(),
            "<svg height=\"57\" width=\"120.5\" viewBox=\"5 7 120.5 57\"/>"
        );
    }

    #[test]
    fn a_fractional_padding_matches_the_reference() {
        let mut svg = Element::new("svg");
        setup_graph_viewbox(&mut svg, PROBED_BOX, 5.5, true);
        assert_eq!(
            svg.render(),
            "<svg width=\"100%\" style=\"max-width: 131.5px;\" viewBox=\"-0.5 1.5 131.5 68\"/>"
        );
    }
}
