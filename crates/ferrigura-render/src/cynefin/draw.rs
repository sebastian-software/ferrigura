//! The groups of the diagram, each as `cynefinRenderer.ts` draws it: the backgrounds (117-132), the
//! waves and the cliff (134-154), the confusion ellipse (156-165), the labels (167-189), the subtitles
//! (191-226), the items (228-349), the transition arrows (351-421) and the title (423-433). The
//! attributes of each element are the ones the reference's calls set, in their order.

use ferrigura_model::cynefin::CynefinDiagram;
use ferrigura_model::value::Value;
use ferrigura_text::{Face, TextError};

use super::boundary;
use super::layout::{self, Domain, ITEM_HEIGHT, ITEM_PITCH, MAX_CONFUSION_ITEMS};
use super::style::{Options, attribute_text};
use super::text::{badge_width, bold, italic, markup, regular};
use crate::common::d3::js_number;
use crate::common::{Element, shown_text};

/// The text runs the diagram draws, with their faces: the characters of each go into the font subsets.
pub(super) type Runs = Vec<(String, &'static Face)>;

/// Records the characters a text shows, in the face it is set in.
fn record(runs: &mut Runs, text: &str, face: &'static Face) {
    runs.push((shown_text(text), face));
}

/// A number as the reference's attributes write it.
fn js(value: f64) -> String {
    js_number(value)
}

/// Sets a presentation attribute to a colour of the theme: the value as text, or nothing when it is
/// `null` (`d3`'s `attr` removes the attribute then). Every coloured element carries its colour so
/// (`docs-port/decisions/0007`).
fn colour(element: &mut Element, name: &str, value: &Value) {
    if let Some(text) = attribute_text(value) {
        element.set(name, text);
    }
}

/// A `<text>` at a position, with its class, its anchor and its baseline, and its colour as a
/// presentation attribute. The content is the markup of the text.
fn text(
    class: &str,
    x: f64,
    y: f64,
    anchor: &str,
    baseline: &str,
    fill: &Value,
    content: &str,
) -> Element {
    let mut element = Element::new("text")
        .with("class", class)
        .with("x", js(x))
        .with("y", js(y))
        .with("text-anchor", anchor)
        .with("dominant-baseline", baseline);
    colour(&mut element, "fill", fill);
    element.raw_text(markup(content))
}

/// `g.cynefin-backgrounds`: one rectangle per quadrant, filled with its colour at 0.4 opacity.
pub(super) fn backgrounds(width: f64, height: f64, light: &Options) -> Element {
    let mut group = Element::new("g").with("class", "cynefin-backgrounds");
    for domain in Domain::QUADRANTS {
        let place = layout::place(domain, width, height);
        let mut rect = Element::new("rect")
            .with("class", format!("cynefinDomain {}", domain.fill_class()))
            .with("x", js(place.x))
            .with("y", js(place.y))
            .with("width", js(place.w))
            .with("height", js(place.h));
        colour(&mut rect, "fill", light.background(domain));
        group.push_child(rect.with("fill-opacity", "0.4").with("stroke", "none"));
    }
    group
}

/// `g.cynefin-boundaries`: the vertical wave, the horizontal wave (the seed plus 100, renderer line
/// 146), and the cliff. The waves take the configured amplitude; the cliff has its own.
pub(super) fn boundaries(
    width: f64,
    height: f64,
    seed: f64,
    amplitude: f64,
    light: &Options,
) -> Element {
    let mut group = Element::new("g").with("class", "cynefin-boundaries");
    group.push_child(wave(
        "cynefinBoundary",
        &boundary::fold_path(width, height, seed, amplitude),
        &light.boundary_color,
    ));
    group.push_child(wave(
        "cynefinBoundary",
        &boundary::horizontal_path(width, height, seed + 100.0, amplitude),
        &light.boundary_color,
    ));
    group.push_child(wave(
        "cynefinCliff",
        &boundary::cliff_path(width, height),
        &light.cliff_color,
    ));
    group
}

/// A boundary path: its class, its outline, no fill, and its stroke.
fn wave(class: &str, d: &str, stroke: &Value) -> Element {
    let mut path = Element::new("path")
        .with("class", class)
        .with("d", d)
        .with("fill", "none");
    colour(&mut path, "stroke", stroke);
    path
}

/// `path.cynefinConfusion`: the ellipse at the centre, filled with the confusion colour at 0.5
/// opacity. Its radii are 15% of the width and of the height (`cynefinRenderer.ts` 158-165).
pub(super) fn confusion(width: f64, height: f64, light: &Options) -> Element {
    let mut ellipse = Element::new("path")
        .with(
            "class",
            format!("cynefinConfusion {}", Domain::Confusion.fill_class()),
        )
        .with(
            "d",
            boundary::confusion_path(width / 2.0, height / 2.0, width * 0.15, height * 0.15),
        );
    colour(&mut ellipse, "fill", light.background(Domain::Confusion));
    ellipse = ellipse.with("fill-opacity", "0.5");
    colour(&mut ellipse, "stroke", &light.boundary_color);
    ellipse
}

/// `g.cynefin-labels`: the four quadrant labels, then the confusion label. The quadrant labels sit
/// above their centre when the descriptions are shown, and the confusion label above the middle.
pub(super) fn labels(
    width: f64,
    height: f64,
    show: bool,
    light: &Options,
    runs: &mut Runs,
) -> Element {
    let mut group = Element::new("g").with("class", "cynefin-labels");
    for domain in Domain::QUADRANTS {
        let place = layout::place(domain, width, height);
        let y = if show { place.cy - 30.0 } else { place.cy };
        group.push_child(text(
            "cynefinDomainLabel",
            place.cx,
            y,
            "middle",
            "middle",
            &light.label_color,
            domain.label(),
        ));
        record(runs, domain.label(), bold());
    }
    let y = if show {
        height / 2.0 - 10.0
    } else {
        height / 2.0
    };
    group.push_child(text(
        "cynefinDomainLabel",
        width / 2.0,
        y,
        "middle",
        "middle",
        &light.label_color,
        Domain::Confusion.label(),
    ));
    record(runs, Domain::Confusion.label(), bold());
    group
}

/// `g.cynefin-subtitles`: the decision model and the practice of each quadrant, and the practice of
/// the confusion domain. Drawn only when the descriptions are shown.
pub(super) fn subtitles(width: f64, height: f64, light: &Options, runs: &mut Runs) -> Element {
    let mut group = Element::new("g").with("class", "cynefin-subtitles");
    for domain in Domain::QUADRANTS {
        let place = layout::place(domain, width, height);
        group.push_child(text(
            "cynefinSubtitle",
            place.cx,
            place.cy - 10.0,
            "middle",
            "middle",
            &light.text_color,
            domain.model(),
        ));
        record(runs, domain.model(), italic());
        group.push_child(text(
            "cynefinSubtitle",
            place.cx,
            place.cy + 5.0,
            "middle",
            "middle",
            &light.text_color,
            domain.practice(),
        ));
        record(runs, domain.practice(), italic());
    }
    group.push_child(text(
        "cynefinSubtitle",
        width / 2.0,
        height / 2.0 + 8.0,
        "middle",
        "middle",
        &light.text_color,
        Domain::Confusion.practice(),
    ));
    record(runs, Domain::Confusion.practice(), italic());
    group
}

/// `g.cynefin-items`: one badge per item, in the order of the domains and of their blocks. The group
/// is there even when no domain has an item. The confusion ellipse shows three items and a `+N more`
/// badge for the rest.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub(super) fn items(
    diagram: &CynefinDiagram,
    width: f64,
    height: f64,
    show: bool,
    light: &Options,
    size: f64,
    runs: &mut Runs,
) -> Result<Element, TextError> {
    let mut group = Element::new("g").with("class", "cynefin-items");
    for domain in Domain::ITEMS {
        let Some(block) = diagram
            .domains
            .iter()
            .find(|block| block.name == domain.name())
        else {
            continue;
        };
        if block.items.is_empty() {
            continue;
        }
        let place = layout::place(domain, width, height);
        let confusion = domain == Domain::Confusion;
        let overflow = if confusion && block.items.len() > MAX_CONFUSION_ITEMS {
            block.items.len() - MAX_CONFUSION_ITEMS
        } else {
            0
        };
        let shown_items = if overflow > 0 {
            &block.items[..MAX_CONFUSION_ITEMS]
        } else {
            &block.items[..]
        };
        // The confusion block sits below its label inside the ellipse; a quadrant's block below its
        // label and subtitle (lines 252-259).
        let start_y = match (confusion, show) {
            (true, true) => place.cy + 22.0,
            (true, false) => place.cy + 14.0,
            (false, true) => place.cy + 25.0,
            (false, false) => place.cy + 15.0,
        };
        for (index, label) in shown_items.iter().enumerate() {
            let y = start_y + index as f64 * ITEM_PITCH;
            let spec = Badge {
                domain,
                label,
                cx: place.cx,
                y,
                overflow: false,
            };
            group.push_child(badge(&spec, light, size, runs)?);
        }
        if overflow > 0 {
            let y = start_y + shown_items.len() as f64 * ITEM_PITCH;
            let label = format!("+{overflow} more");
            let spec = Badge {
                domain,
                label: &label,
                cx: place.cx,
                y,
                overflow: true,
            };
            group.push_child(badge(&spec, light, size, runs)?);
        }
    }
    Ok(group)
}

/// One item badge to draw: its domain, its text, the x of its centre and the y of its top, and whether it
/// is the `+N more` badge.
struct Badge<'a> {
    domain: Domain,
    label: &'a str,
    cx: f64,
    y: f64,
    overflow: bool,
}

/// One badge: a `g` at its position, with the rectangle (inserted before the text, sized to the text's
/// width plus the padding) and the text, centred in it. The text is measured at `size` pixels. The overflow
/// badge is the dashed one.
fn badge(
    spec: &Badge<'_>,
    light: &Options,
    size: f64,
    runs: &mut Runs,
) -> Result<Element, TextError> {
    let width = badge_width(spec.label, size)?;
    let x = spec.cx - width / 2.0;
    record(runs, spec.label, regular());
    let class = if spec.overflow {
        "cynefinItemOverflow"
    } else {
        "cynefinItem"
    };
    let mut rect = Element::new("rect")
        .with("class", format!("{class} {}", spec.domain.fill_class()))
        .with("x", "0")
        .with("y", "0")
        .with("width", js(width))
        .with("height", js(ITEM_HEIGHT))
        .with("rx", "4")
        .with("ry", "4");
    colour(&mut rect, "fill", light.background(spec.domain));
    rect = rect.with("fill-opacity", if spec.overflow { "0.6" } else { "0.95" });
    colour(&mut rect, "stroke", &light.boundary_color);
    let mut content = Element::new("text")
        .with("class", "cynefinItemText")
        .with("x", js(width / 2.0))
        .with("y", js(ITEM_HEIGHT / 2.0))
        .with("text-anchor", "middle")
        .with("dominant-baseline", "central");
    colour(&mut content, "fill", &light.text_color);
    let content = content.raw_text(markup(spec.label));
    Ok(Element::new("g")
        .with("transform", format!("translate({}, {})", js(x), js(spec.y)))
        .child(rect)
        .child(content))
}

/// The transition arrows: the `defs` with the arrow head, and `g.cynefin-arrows`, when the diagram has
/// a transition. Each arrow is a quadratic curve from the centre of its start to the centre of its end,
/// bent by 15% of its length to the left of its direction; a labelled one has its label on the curve.
pub(super) fn arrows(
    diagram: &CynefinDiagram,
    width: f64,
    height: f64,
    id: &str,
    light: &Options,
    runs: &mut Runs,
) -> Option<(Element, Element)> {
    if diagram.transitions.is_empty() {
        return None;
    }
    let marker_id = format!("cynefin-arrow-{id}");
    let mut head = Element::new("path")
        .with("d", "M 0 0 L 10 5 L 0 10 z")
        .with("class", "cynefinArrowHead");
    colour(&mut head, "fill", &light.arrow_color);
    let marker = Element::new("marker")
        .with("id", marker_id.as_str())
        .with("viewBox", "0 0 10 10")
        .with("refX", "9")
        .with("refY", "5")
        .with("markerWidth", "6")
        .with("markerHeight", "6")
        .with("orient", "auto-start-reverse")
        .child(head);
    let defs = Element::new("defs").child(marker);

    let mut group = Element::new("g").with("class", "cynefin-arrows");
    for transition in &diagram.transitions {
        let (Some(from), Some(to)) = (Domain::of(&transition.from), Domain::of(&transition.to))
        else {
            continue;
        };
        if from == to {
            continue;
        }
        let start = layout::place(from, width, height);
        let end = layout::place(to, width, height);
        let (x1, y1, x2, y2) = (start.cx, start.cy, end.cx, end.cy);
        let mid_x = (x1 + x2) / 2.0;
        let mid_y = (y1 + y2) / 2.0;
        let dx = x2 - x1;
        let dy = y2 - y1;
        let length = (dx * dx + dy * dy).sqrt();
        let bend = length * 0.15;
        let normal_x = -dy / length;
        let normal_y = dx / length;
        let cpx = mid_x + normal_x * bend;
        let cpy = mid_y + normal_y * bend;

        let mut line = Element::new("path")
            .with("class", "cynefinArrowLine")
            .with(
                "d",
                format!(
                    "M{},{} Q{},{} {},{}",
                    js(x1),
                    js(y1),
                    js(cpx),
                    js(cpy),
                    js(x2),
                    js(y2)
                ),
            )
            .with("fill", "none")
            .with("marker-end", format!("url(#{marker_id})"));
        colour(&mut line, "stroke", &light.arrow_color);
        group.push_child(line);

        if let Some(label) = &transition.label {
            record(runs, label, regular());
            group.push_child(text(
                "cynefinArrowLabel",
                cpx,
                cpy - 6.0,
                "middle",
                "auto",
                &light.text_color,
                label,
            ));
        }
    }
    Some((defs, group))
}

/// `text.cynefinTitle`: the title above the diagram, centred, at half the padding above the top edge
/// (`-padding / 2`). Absent when the diagram has no title.
pub(super) fn title(
    width: f64,
    padding: f64,
    title: &str,
    light: &Options,
    runs: &mut Runs,
) -> Option<Element> {
    if title.is_empty() {
        return None;
    }
    record(runs, title, bold());
    Some(text(
        "cynefinTitle",
        width / 2.0,
        -padding / 2.0,
        "middle",
        "middle",
        &light.label_color,
        title,
    ))
}
