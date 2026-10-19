//! The radar diagram: a port of `packages/mermaid/src/diagrams/radar/renderer.ts` (`draw`, lines 8-46,
//! and the helpers below it), with the styles of `styles.ts` in [`style`] and the polar arithmetic in
//! [`geometry`]. The research is in `docs-port/radar/NOTES.md`, section 6.
//!
//! Nothing is measured and nothing is laid out. The rings, the axes and the curves are placed from the
//! radius and the number of axes; the labels from the radius and constants. The browser measures the
//! text when it shows the SVG, with the fonts the SVG embeds.

pub mod geometry;
mod style;

use ferrigura_jsmath::{cos, sin};
use ferrigura_model::config::RadarDiagramConfig;
use ferrigura_model::radar::{Graticule, RadarDiagram};
use ferrigura_text::{EmbeddedFamily, FontFamily, FontStyle, FontWeight, Run, embed_runs, face};

use crate::RenderError;
use crate::a11y;
use crate::common::style::base_style_sheet;
use crate::common::viewbox::configure_svg_size;
use crate::common::{Element, shown_text};
use crate::context::Context;
use crate::svg::number;
use geometry::{
    Point, axis_angle, closed_round_curve, js_max, js_min, point_angle, relative_radius, tick_count,
};
use style::{Options, Style};

/// The XML namespace of the root element.
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// The side of a legend box: the reference writes 12 whatever `legendBoxSize` says (`renderer.ts`
/// 232-233; the theme's `legendBoxSize` is not read, `radar/NOTES.md` section 6.11, item 11).
const LEGEND_BOX: f64 = 12.0;
/// The distance between two legend items (`renderer.ts` 222).
const LEGEND_LINE_HEIGHT: f64 = 20.0;
/// The offset of a legend text from the left of its box (`renderer.ts` 239).
const LEGEND_TEXT_X: f64 = 16.0;
/// The push of an axis label away from its axis end (`renderer.ts` 123).
const LABEL_PAD: f64 = 4.0;
/// The cosine and sine beyond which an axis label is anchored at a side, not in the middle
/// (`renderer.ts` 118-119).
const AXIS_EPSILON: f64 = 0.01;

/// Renders the diagram to an SVG document. The section is the radar section of the configuration, and
/// the theme's tokens give the colours and the sizes of the options.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read or subset.
pub fn render(
    input: &Context<'_, RadarDiagram, RadarDiagramConfig>,
) -> Result<String, RenderError> {
    let diagram = input.diagram;
    let id = input.id;
    let config = input.section;
    let style = Style::of(input.theme);
    let light = &style.light;

    // `draw` (`renderer.ts` 8-46): the frame, then the graticule, the axes, the curves, the legend and
    // the title, in that order. The frame's `g` is the last child of the root, after the empty `g`
    // that the pipeline appends first.
    let radius = radius(config);
    let total_width = config.width + config.margin_left + config.margin_right;
    let total_height = config.height + config.margin_top + config.margin_bottom;
    let mut frame = Element::new("g").with(
        "transform",
        format!(
            "translate({}, {})",
            number(config.margin_left + config.width / 2.0),
            number(config.margin_top + config.height / 2.0)
        ),
    );
    draw_graticule(&mut frame, diagram, radius, light);
    draw_axes(&mut frame, diagram, radius, config, light);
    draw_curves(&mut frame, diagram, radius, config, &style);
    if diagram.options.show_legend {
        draw_legend(&mut frame, diagram, config, &style);
    }
    frame.push_child(title(diagram, config));

    // `addA11yInfo`: the title and the description are the first children of the root, and the root
    // refers to them.
    let mut accessibility = Vec::new();
    if !diagram.acc_title.is_empty() {
        accessibility.push(
            Element::new("title")
                .with("id", a11y::title_id(id))
                .text(shown_text(&diagram.acc_title)),
        );
    }
    if !diagram.acc_descr.is_empty() {
        accessibility.push(
            Element::new("desc")
                .with("id", a11y::description_id(id))
                .text(shown_text(&diagram.acc_descr)),
        );
    }

    // The fonts: every character the SVG draws, in the regular face (the radar sets no other).
    let mut texts = String::new();
    for element in accessibility.iter().chain(std::iter::once(&frame)) {
        element.collect_text(&mut texts);
    }
    let families: Vec<EmbeddedFamily> = embed_runs(&[Run {
        text: &texts,
        face: face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal),
    }])?;
    let font_family = families.first().map_or_else(
        || {
            input
                .theme
                .light
                .font_family
                .text()
                .unwrap_or_else(|| "sans-serif".to_owned())
        },
        |family| family.css_family.clone(),
    );
    let font_size = input
        .theme
        .light
        .font_size
        .text()
        .unwrap_or_else(|| "16px".to_owned());
    let css = base_style_sheet(
        id,
        input.theme,
        &font_family,
        &font_size,
        &families,
        &style::own_rules(id, &style),
    );

    // `drawFrame`'s `viewBox` and `overflow`, after `configureSvgSize` (`renderer.ts` 50-62).
    let mut root = Element::new("svg")
        .with("id", id)
        .with("width", "100%")
        .with("xmlns", SVG_NAMESPACE);
    configure_svg_size(&mut root, total_height, total_width, config.use_max_width);
    root.set(
        "viewBox",
        format!("0 0 {} {}", number(total_width), number(total_height)),
    );
    root.set("overflow", "visible");
    root.set("role", a11y::SVG_ROLE);
    root.set("aria-roledescription", "radar");
    if !diagram.acc_descr.is_empty() {
        root.set("aria-describedby", a11y::description_id(id));
    }
    if !diagram.acc_title.is_empty() {
        root.set("aria-labelledby", a11y::title_id(id));
    }

    for element in accessibility {
        root.push_child(element);
    }
    root.push_child(Element::new("style").raw_text(css));
    // The empty group `mermaidAPI` appends before the renderer draws.
    root.push_child(Element::new("g"));
    root.push_child(frame);
    Ok(root.render())
}

/// The radius of the chart: half the smaller of the width and the height (`renderer.ts` 26).
fn radius(config: &RadarDiagramConfig) -> f64 {
    js_min(config.width, config.height) / 2.0
}

/// The largest value of the scale: the `max` option, or the largest entry of any curve, drawn or not
/// (`renderer.ts` 23-24). With no curve and no `max` it is negative infinity, as `Math.max()` is, and
/// nothing is drawn with it.
fn max_value(diagram: &RadarDiagram) -> f64 {
    diagram.options.max.unwrap_or_else(|| {
        diagram
            .curves
            .iter()
            .flat_map(|curve| curve.entries.iter().copied())
            .fold(f64::NEG_INFINITY, js_max)
    })
}

/// `drawGraticule` (`renderer.ts` 64-93): one circle or polygon for each tick, the first at the
/// smallest radius.
fn draw_graticule(group: &mut Element, diagram: &RadarDiagram, radius: f64, options: &Options) {
    let ticks = diagram.options.ticks;
    let count = diagram.axes.len();
    for index in 0..tick_count(ticks) {
        let r = (radius * (index + 1) as f64) / ticks;
        let element = match diagram.options.graticule {
            Graticule::Circle => Element::new("circle").with("r", number(r)),
            Graticule::Polygon => {
                let points = (0..count)
                    .map(|j| {
                        let angle = axis_angle(j, count);
                        format!("{},{}", number(r * cos(angle)), number(r * sin(angle)))
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                Element::new("polygon").with("points", points)
            }
        };
        group.push_child(graticule_paint(
            element.with("class", "radarGraticule"),
            options,
        ));
    }
}

/// The presentation attributes of the graticule, as its rule gives them (`styles.ts` 60-65).
fn graticule_paint(element: Element, options: &Options) -> Element {
    element
        .with("fill", options.graticule_color.as_str())
        .with("fill-opacity", options.graticule_opacity.as_str())
        .with("stroke", options.graticule_color.as_str())
        .with("stroke-width", options.graticule_stroke_width.as_str())
}

/// `drawAxes` (`renderer.ts` 95-133): for each axis, its line from the centre, and its label beyond
/// the end of the line, anchored by the side of the chart it is on.
fn draw_axes(
    group: &mut Element,
    diagram: &RadarDiagram,
    radius: f64,
    config: &RadarDiagramConfig,
    options: &Options,
) {
    let count = diagram.axes.len();
    for (index, axis) in diagram.axes.iter().enumerate() {
        let angle = axis_angle(index, count);
        let cos_a = cos(angle);
        let sin_a = sin(angle);
        group.push_child(
            Element::new("line")
                .with("x1", "0")
                .with("y1", "0")
                .with("x2", number(radius * config.axis_scale_factor * cos_a))
                .with("y2", number(radius * config.axis_scale_factor * sin_a))
                .with("class", "radarAxisLine")
                .with("stroke", options.axis_color.as_str())
                .with("stroke-width", options.axis_stroke_width.as_str()),
        );

        // Anchor labels by their angular position, so that the text extends away from the centre.
        let anchor = if cos_a > AXIS_EPSILON {
            "start"
        } else if cos_a < -AXIS_EPSILON {
            "end"
        } else {
            "middle"
        };
        let baseline = if sin_a > AXIS_EPSILON {
            "hanging"
        } else if sin_a < -AXIS_EPSILON {
            "auto"
        } else {
            "central"
        };
        group.push_child(
            Element::new("text")
                .with(
                    "x",
                    number(radius * config.axis_label_factor * cos_a + LABEL_PAD * cos_a),
                )
                .with(
                    "y",
                    number(radius * config.axis_label_factor * sin_a + LABEL_PAD * sin_a),
                )
                .with("text-anchor", anchor)
                .with("dominant-baseline", baseline)
                .with("class", "radarAxisLabel")
                .text(shown_text(&axis.label)),
        );
    }
}

/// `drawCurves` (`renderer.ts` 135-173): a curve is drawn only when it has one entry for each axis.
/// Its index is kept, so a skipped curve leaves a gap in the numbering of the others.
fn draw_curves(
    group: &mut Element,
    diagram: &RadarDiagram,
    radius: f64,
    config: &RadarDiagramConfig,
    style: &Style,
) {
    let count = diagram.axes.len();
    let min = diagram.options.min;
    let max = max_value(diagram);
    for (index, curve) in diagram.curves.iter().enumerate() {
        if curve.entries.len() != count {
            continue;
        }
        let points: Vec<Point> = curve
            .entries
            .iter()
            .enumerate()
            .map(|(point, entry)| {
                let angle = point_angle(point, count);
                let r = relative_radius(*entry, min, max, radius);
                Point {
                    x: r * cos(angle),
                    y: r * sin(angle),
                }
            })
            .collect();
        let class = format!("radarCurve-{index}");
        let element = match diagram.options.graticule {
            Graticule::Circle => {
                Element::new("path").with("d", closed_round_curve(&points, config.curve_tension))
            }
            Graticule::Polygon => Element::new("polygon").with(
                "points",
                points
                    .iter()
                    .map(|p| format!("{},{}", number(p.x), number(p.y)))
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
        };
        let element = element.with("class", class);
        group.push_child(curve_paint(element, index, style));
    }
}

/// The presentation attributes of curve `index`, as its rule gives them (`styles.ts` 16-22). A curve
/// past the colour limit has no rule, so it has none either and draws with the root's fill.
fn curve_paint(element: Element, index: usize, style: &Style) -> Element {
    if index >= style.limit {
        return element;
    }
    let options = &style.light;
    let mut element = element;
    if let Some(colour) = options.curve_colours.get(index).and_then(Option::as_deref) {
        element.set("fill", colour);
        element.set("stroke", colour);
    }
    element.set("fill-opacity", options.curve_opacity.as_str());
    element.set("stroke-width", options.curve_stroke_width.as_str());
    element
}

/// `drawLegend` (`renderer.ts` 209-244): one item for each curve, skipped or not, in a column at the
/// top right of the chart.
fn draw_legend(
    group: &mut Element,
    diagram: &RadarDiagram,
    config: &RadarDiagramConfig,
    style: &Style,
) {
    let legend_x = ((config.width / 2.0 + config.margin_right) * 3.0) / 4.0;
    let legend_y = (-(config.height / 2.0 + config.margin_top) * 3.0) / 4.0;
    for (index, curve) in diagram.curves.iter().enumerate() {
        let rect = legend_box(
            Element::new("rect")
                .with("width", number(LEGEND_BOX))
                .with("height", number(LEGEND_BOX))
                .with("class", format!("radarLegendBox-{index}")),
            index,
            style,
        );
        let text = Element::new("text")
            .with("x", number(LEGEND_TEXT_X))
            .with("y", "0")
            .with("class", "radarLegendText")
            .text(shown_text(&curve.label));
        let item = Element::new("g")
            .with(
                "transform",
                format!(
                    "translate({}, {})",
                    number(legend_x),
                    number(legend_y + index as f64 * LEGEND_LINE_HEIGHT)
                ),
            )
            .child(rect)
            .child(text);
        group.push_child(item);
    }
}

/// The presentation attributes of the legend box of curve `index`, as its rule gives them
/// (`styles.ts` 23-27). A box past the colour limit has no rule either.
fn legend_box(element: Element, index: usize, style: &Style) -> Element {
    if index >= style.limit {
        return element;
    }
    let options = &style.light;
    let mut element = element;
    if let Some(colour) = options.curve_colours.get(index).and_then(Option::as_deref) {
        element.set("fill", colour);
        element.set("stroke", colour);
    }
    element.set("fill-opacity", options.curve_opacity.as_str());
    element
}

/// The title (`renderer.ts` 41-45): always written, even when empty, as the last child of the frame.
/// Its position is above the chart, by the top margin.
fn title(diagram: &RadarDiagram, config: &RadarDiagramConfig) -> Element {
    Element::new("text")
        .with("class", "radarTitle")
        .with("x", "0")
        .with("y", number(-config.height / 2.0 - config.margin_top))
        .text(shown_text(&diagram.title))
}
