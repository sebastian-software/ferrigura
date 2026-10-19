//! The shapes of stages A and C against the reference, measured in Chromium (`tools/port/geometry/
//! shape-probe.ts`, stored in `tests/fixtures/flowchart-shapes.json`).
//!
//! Each case is a single-node flowchart. The reference's label box is fed to `draw` as the
//! label's layout, so the text is isolated: what is compared is the shape. The node's box, the
//! label's offset and the outline element's box must agree within 0.05 px.

use std::f64::consts::PI;

use ferrigura_jsmath as jsmath;
use ferrigura_model::config::MermaidConfig;
use ferrigura_model::theme::{Theme, ThemeTokens};
use ferrigura_model::value::Value;
use ferrigura_render::context::Globals;
use ferrigura_render::unified::data::Node;
use ferrigura_render::unified::geom::{BBox, Point};
use ferrigura_render::unified::label::LabelLayout;
use ferrigura_render::unified::shapes::{ShapeInput, ShapeOutput, draw};
use ferrigura_render::unified::{Config, Section};

/// The tolerance of every comparison, in pixels.
const TOLERANCE: f64 = 0.05;

const FIXTURE: &str = include_str!("fixtures/flowchart-shapes.json");
/// The stage C shapes (T3c): the shapes that stage A does not have, the icons and the images.
const FIXTURE_T3C: &str = include_str!("fixtures/flowchart-shapes-t3c.json");

/// The node of a case, as the flowchart's getData makes it: a vertex of the given shape and look,
/// with the defaults of the flowchart (padding 15, a minimum label width of 120). An empty label
/// is written as the id, as the reference does.
///
/// A collapsed subgraph is the `collapsedGroup` node: padding 8 and no minimum label width
/// (`flowDb.ts` 1237-1257).
fn node_of(case: &serde_json::Value) -> Node {
    let source = case["labelSource"].as_str().unwrap_or_default();
    // The directive's other properties (`icon`, `pos`, `img`, and `w` and `h` for the asset size).
    let props = &case["props"];
    // An empty label is the id, except on an icon or an image, where flowDb empties it
    // (`flowDb.ts` 245-266).
    let icon_or_image = props["icon"].is_string() || props["img"].is_string();
    let label = if source.is_empty() && !icon_or_image {
        "A"
    } else {
        source
    };
    let collapsed = case["shape"] == "collapsed";
    Node {
        id: "A".to_owned(),
        dom_id: "probe-flowchart-A-0".to_owned(),
        label: label.to_owned(),
        shape: if collapsed {
            "collapsedGroup".to_owned()
        } else {
            case["shape"].as_str().unwrap_or_default().to_owned()
        },
        look: case["look"].as_str().unwrap_or_default().to_owned(),
        padding: if collapsed { 8.0 } else { 15.0 },
        min_width: (!collapsed).then_some(120.0),
        css_classes: "default".to_owned(),
        icon: props["icon"].as_str().map(str::to_owned),
        pos: props["pos"].as_str().map(str::to_owned),
        img: props["img"].as_str().map(str::to_owned),
        asset_width: props["w"].as_f64(),
        asset_height: props["h"].as_f64(),
        ..Node::default()
    }
}

/// A box from the fixture's `{x, y, width, height}`.
fn box_of(value: &serde_json::Value) -> BBox {
    BBox::new(
        value["x"].as_f64().expect("x"),
        value["y"].as_f64().expect("y"),
        value["width"].as_f64().expect("width"),
        value["height"].as_f64().expect("height"),
    )
}

/// The translation of a `translate(x, y)` or `translate(x,y)` attribute.
fn translate_of(transform: &str) -> Point {
    let inner = transform
        .trim()
        .strip_prefix("translate(")
        .and_then(|rest| rest.strip_suffix(')'))
        .expect("a translate");
    let mut numbers = inner
        .split([',', ' '])
        .filter(|piece| !piece.is_empty())
        .map(|piece| piece.parse::<f64>().expect("a number"));
    Point::new(numbers.next().expect("x"), numbers.next().unwrap_or(0.0))
}

/// The theme of the reference: redux-color, as the fixtures' configuration sets it.
fn theme() -> Value {
    Theme::ReduxColor.variables(None)
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= TOLERANCE
}

fn close_box(a: BBox, b: BBox) -> bool {
    close(a.x, b.x) && close(a.y, b.y) && close(a.width, b.width) && close(a.height, b.height)
}

fn format_box(box_: BBox) -> String {
    format!(
        "({:.4}, {:.4}, {:.4} x {:.4})",
        box_.x, box_.y, box_.width, box_.height
    )
}

// ---------------------------------------------------------------------------------------------
// The geometry of the outline element, measured as `getBBox` measures it: the leaves of the
// first element, in its own coordinates (its own transform is not part of it).

/// A tag of an SVG markup: its name, its attributes, and whether it closes or stands alone.
struct Tag {
    name: String,
    attributes: Vec<(String, String)>,
    closing: bool,
    self_closing: bool,
}

impl Tag {
    fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    fn number(&self, name: &str) -> f64 {
        self.attribute(name)
            .map_or(0.0, |value| value.parse().expect("a number"))
    }
}

/// The tags of a markup, in order. Only the markup that the port writes is read: attributes are
/// quoted with double quotes, and there are no comments.
fn tags(markup: &str) -> Vec<Tag> {
    let mut out = Vec::new();
    let mut rest = markup;
    while let Some(start) = rest.find('<') {
        let after = &rest[start + 1..];
        let end = after.find('>').expect("the end of a tag");
        let body = &after[..end];
        rest = &after[end + 1..];
        let closing = body.starts_with('/');
        let self_closing = body.ends_with('/');
        let body = body.trim_start_matches('/').trim_end_matches('/').trim();
        let name_end = body.find(char::is_whitespace).unwrap_or(body.len());
        let name = body[..name_end].to_owned();
        let mut attributes = Vec::new();
        let mut attribute_text = &body[name_end..];
        while let Some(equals) = attribute_text.find('=') {
            let key = attribute_text[..equals].trim().to_owned();
            let value_start = attribute_text[equals + 1..]
                .find('"')
                .expect("a quoted value")
                + equals
                + 2;
            let value_end = attribute_text[value_start..]
                .find('"')
                .expect("the end of a value")
                + value_start;
            attributes.push((key, attribute_text[value_start..value_end].to_owned()));
            attribute_text = &attribute_text[value_end + 1..];
        }
        out.push(Tag {
            name,
            attributes,
            closing,
            self_closing,
        });
    }
    out
}

/// The points of an SVG path's data, sampled along its curves and arcs. Commands are the ones the
/// port writes: M, L, C, A, Z and their relative forms, and H and V; a number may have an exponent.
fn path_points(d: &str) -> Vec<Point> {
    let mut numbers: Vec<f64> = Vec::new();
    let mut commands: Vec<(char, usize)> = Vec::new();
    let chars = d.chars();
    let mut pieces: Vec<(Option<char>, String)> = Vec::new();
    let mut current = String::new();
    for c in chars {
        // An `e` inside a number is its exponent (`4.286263797015736e-16`), not a command.
        let exponent =
            matches!(c, 'e' | 'E') && !current.is_empty() && !current.contains(['e', 'E']);
        if c.is_ascii_alphabetic() && !exponent {
            pieces.push((None, std::mem::take(&mut current)));
            pieces.push((Some(c), String::new()));
        } else if c == ',' || c.is_whitespace() {
            pieces.push((None, std::mem::take(&mut current)));
        } else if c == '-' && !current.is_empty() && !current.ends_with(['e', 'E']) {
            pieces.push((None, std::mem::take(&mut current)));
            current.push(c);
        } else {
            current.push(c);
        }
    }
    pieces.push((None, current));
    for (command, text) in pieces {
        if let Some(command) = command {
            commands.push((command, numbers.len()));
        } else if !text.is_empty() {
            numbers.push(text.parse().expect("a number in the path"));
        }
    }

    let mut points = Vec::new();
    let mut current = Point::default();
    let mut start = Point::default();
    for (index, (command, at)) in commands.iter().enumerate() {
        let end = commands.get(index + 1).map_or(numbers.len(), |next| next.1);
        let args = &numbers[*at..end];
        let relative = command.is_ascii_lowercase();
        match command.to_ascii_uppercase() {
            'M' => {
                for (pair_index, pair) in args.chunks(2).enumerate() {
                    current = if relative {
                        Point::new(current.x + pair[0], current.y + pair[1])
                    } else {
                        Point::new(pair[0], pair[1])
                    };
                    if pair_index == 0 {
                        start = current;
                    }
                    points.push(current);
                }
            }
            'L' => {
                for pair in args.chunks(2) {
                    current = Point::new(
                        if relative { current.x } else { 0.0 } + pair[0],
                        if relative { current.y } else { 0.0 } + pair[1],
                    );
                    points.push(current);
                }
            }
            'H' => {
                for value in args {
                    current.x = if relative { current.x } else { 0.0 } + value;
                    points.push(current);
                }
            }
            'V' => {
                for value in args {
                    current.y = if relative { current.y } else { 0.0 } + value;
                    points.push(current);
                }
            }
            'C' => {
                for chunk in args.chunks(6) {
                    let base = if relative { current } else { Point::default() };
                    let c1 = Point::new(base.x + chunk[0], base.y + chunk[1]);
                    let c2 = Point::new(base.x + chunk[2], base.y + chunk[3]);
                    let end_point = Point::new(base.x + chunk[4], base.y + chunk[5]);
                    for step in 1..=256 {
                        let t = f64::from(step) / 256.0;
                        let u = 1.0 - t;
                        points.push(Point::new(
                            u * u * u * current.x
                                + 3.0 * u * u * t * c1.x
                                + 3.0 * u * t * t * c2.x
                                + t * t * t * end_point.x,
                            u * u * u * current.y
                                + 3.0 * u * u * t * c1.y
                                + 3.0 * u * t * t * c2.y
                                + t * t * t * end_point.y,
                        ));
                    }
                    current = end_point;
                }
            }
            'A' => {
                for chunk in args.chunks(7) {
                    let base = if relative { current } else { Point::default() };
                    let target = Point::new(base.x + chunk[5], base.y + chunk[6]);
                    points.extend(arc_points(
                        current,
                        target,
                        chunk[0],
                        chunk[1],
                        chunk[2],
                        chunk[3] != 0.0,
                        chunk[4] != 0.0,
                    ));
                    current = target;
                }
            }
            'Z' => {
                current = start;
            }
            other => panic!("the path command {other} is not used by the port"),
        }
    }
    points
}

/// The points of an elliptical arc, rotated by `rotation` degrees, from `from` to `to`, sampled
/// along its length (SVG's endpoint parameterisation, appendix F.6.5).
fn arc_points(
    from: Point,
    to: Point,
    rx: f64,
    ry: f64,
    rotation: f64,
    large: bool,
    sweep: bool,
) -> Vec<Point> {
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    let radians = rotation.to_radians();
    let (sin, cos) = (jsmath::sin(radians), jsmath::cos(radians));
    let dx2 = (from.x - to.x) / 2.0;
    let dy2 = (from.y - to.y) / 2.0;
    let x1 = cos * dx2 + sin * dy2;
    let y1 = -sin * dx2 + cos * dy2;
    let lambda = x1 * x1 / (rx * rx) + y1 * y1 / (ry * ry);
    if lambda > 1.0 {
        rx *= lambda.sqrt();
        ry *= lambda.sqrt();
    }
    let numerator = rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1;
    let denominator = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let sign = if large != sweep { 1.0 } else { -1.0 };
    let factor = sign * (numerator / denominator).max(0.0).sqrt();
    let cxp = factor * rx * y1 / ry;
    let cyp = factor * -ry * x1 / rx;
    let cx = cos * cxp - sin * cyp + (from.x + to.x) / 2.0;
    let cy = sin * cxp + cos * cyp + (from.y + to.y) / 2.0;
    let angle = |ux: f64, uy: f64, vx: f64, vy: f64| {
        let sign = if ux * vy - uy * vx < 0.0 { -1.0 } else { 1.0 };
        let cos = ((ux * vx + uy * vy) / ((ux * ux + uy * uy).sqrt() * (vx * vx + vy * vy).sqrt()))
            .clamp(-1.0, 1.0);
        sign * jsmath::acos(cos)
    };
    let theta1 = angle(1.0, 0.0, (x1 - cxp) / rx, (y1 - cyp) / ry);
    let mut delta = angle(
        (x1 - cxp) / rx,
        (y1 - cyp) / ry,
        (-x1 - cxp) / rx,
        (-y1 - cyp) / ry,
    );
    if !sweep && delta > 0.0 {
        delta -= 2.0 * PI;
    } else if sweep && delta < 0.0 {
        delta += 2.0 * PI;
    }
    (0..=256)
        .map(|step| {
            let theta = theta1 + delta * f64::from(step) / 256.0;
            Point::new(
                cx + rx * cos * jsmath::cos(theta) - ry * sin * jsmath::sin(theta),
                cy + rx * sin * jsmath::cos(theta) + ry * cos * jsmath::sin(theta),
            )
        })
        .collect()
}

/// The box of the first element of a markup, as its `getBBox` in its own coordinates: the union
/// of the geometry of its leaves. Returns the element's name too.
fn outline_box(markup: &str) -> (String, BBox) {
    let mut depth = 0_usize;
    let mut name = None;
    let mut points: Vec<Point> = Vec::new();
    let mut boxes: Vec<BBox> = Vec::new();
    for tag in tags(markup) {
        if tag.closing {
            depth -= 1;
            if depth == 0 {
                break;
            }
            continue;
        }
        if depth == 0 {
            name = Some(tag.name.clone());
        }
        match tag.name.as_str() {
            "rect" => boxes.push(BBox::new(
                tag.number("x"),
                tag.number("y"),
                tag.number("width"),
                tag.number("height"),
            )),
            "circle" => {
                let r = tag.number("r");
                let (cx, cy) = (tag.number("cx"), tag.number("cy"));
                boxes.push(BBox::new(cx - r, cy - r, 2.0 * r, 2.0 * r));
            }
            "ellipse" => {
                let (rx, ry) = (tag.number("rx"), tag.number("ry"));
                let (cx, cy) = (tag.number("cx"), tag.number("cy"));
                boxes.push(BBox::new(cx - rx, cy - ry, 2.0 * rx, 2.0 * ry));
            }
            "line" => points.extend([
                Point::new(tag.number("x1"), tag.number("y1")),
                Point::new(tag.number("x2"), tag.number("y2")),
            ]),
            "polygon" => {
                let values = tag.attribute("points").unwrap_or_default();
                points.extend(values.split_whitespace().map(|pair| {
                    let (x, y) = pair.split_once(',').expect("a point");
                    Point::new(x.parse().expect("x"), y.parse().expect("y"))
                }));
            }
            "path" => points.extend(path_points(tag.attribute("d").unwrap_or_default())),
            _ => {}
        }
        if depth == 0 && tag.self_closing {
            break;
        }
        if !tag.self_closing {
            depth += 1;
        }
    }
    if let Some(bounds) = BBox::from_points(points) {
        boxes.push(bounds);
    }
    let bbox = boxes.into_iter().reduce(BBox::union).unwrap_or_default();
    (name.unwrap_or_default(), bbox)
}

// ---------------------------------------------------------------------------------------------

/// The largest difference of one case, in pixels, over its node box, label offset and outline box.
#[derive(Default)]
struct Deviation {
    worst: f64,
}

impl Deviation {
    fn box_(&mut self, a: BBox, b: BBox) {
        self.worst = self
            .worst
            .max((a.x - b.x).abs())
            .max((a.y - b.y).abs())
            .max((a.width - b.width).abs())
            .max((a.height - b.height).abs());
    }
}

/// The comparison of one case: the failures it has, each with its cause.
fn check(case: &serde_json::Value, shape: &ShapeOutput, deviation: &mut Deviation) -> Vec<String> {
    let mut failures = Vec::new();
    let label = case["shape"].as_str().unwrap_or_default();
    let look = case["look"].as_str().unwrap_or_default();
    let name = format!(
        "{label} {look} {:?}",
        case["labelSource"].as_str().unwrap_or_default()
    );

    let expected_node = box_of(&case["node"]);
    deviation.box_(shape.bbox, expected_node);
    if !close_box(shape.bbox, expected_node) {
        failures.push(format!(
            "{name}: node box {} but the reference {}",
            format_box(shape.bbox),
            format_box(expected_node)
        ));
    }

    let reference_label = &case["label"];
    match (&shape.label_offset, reference_label.is_null()) {
        (Some(offset), false) => {
            let expected = translate_of(reference_label["transform"].as_str().unwrap_or_default());
            deviation.worst = deviation
                .worst
                .max((offset.x - expected.x).abs())
                .max((offset.y - expected.y).abs());
            if !(close(offset.x, expected.x) && close(offset.y, expected.y)) {
                failures.push(format!(
                    "{name}: label offset ({:.4}, {:.4}) but the reference ({:.4}, {:.4})",
                    offset.x, offset.y, expected.x, expected.y
                ));
            }
        }
        (None, true) => {}
        (Some(_), true) => failures.push(format!("{name}: a label where the reference has none")),
        (None, false) => failures.push(format!("{name}: no label where the reference has one")),
    }

    let (tag, outline) = outline_box(&shape.svg);
    let reference_outline = &case["outline"];
    let expected_tag = reference_outline["tag"].as_str().unwrap_or_default();
    if tag != expected_tag {
        failures.push(format!(
            "{name}: outline element <{tag}> but the reference <{expected_tag}>"
        ));
    }
    let expected_outline = box_of(&reference_outline["bbox"]);
    deviation.box_(outline, expected_outline);
    if !close_box(outline, expected_outline) {
        failures.push(format!(
            "{name}: outline box {} but the reference {}",
            format_box(outline),
            format_box(expected_outline)
        ));
    }
    failures
}

#[test]
fn the_shapes_of_stage_a_match_the_reference_probe() {
    check_fixture(FIXTURE, "stage A and the first half of stage C");
}

#[test]
fn the_shapes_of_stage_c_match_the_reference_probe() {
    check_fixture(FIXTURE_T3C, "stage C (T3c)");
}

/// Draws every case of a fixture and compares it with the reference's measurements.
fn check_fixture(fixture_text: &str, stage: &str) {
    let fixture: serde_json::Value =
        serde_json::from_str(fixture_text).expect("the fixture is JSON");
    let cases = fixture["cases"].as_array().expect("the cases");
    assert!(!cases.is_empty(), "the fixture has cases");
    let config = MermaidConfig::default();
    let globals = Globals::of(&config);
    let tokens = ThemeTokens::from_value(&theme());
    let unified = Config {
        section: Section::of(&config),
        globals: &globals,
        theme: &tokens,
    };

    let mut failures = Vec::new();
    let mut deviation = Deviation::default();
    for case in cases {
        let node = node_of(case);
        let label_box = (!case["label"].is_null()).then(|| box_of(&case["label"]["bbox"]));
        // An icon's or an image's label has the background group the reference measures: the
        // box of the group is the label's box for the node.
        let layout = label_box.map(|bbox| {
            let mut layout = LabelLayout::of_box(bbox);
            if !case["labelGroup"].is_null() {
                layout.background = Some(box_of(&case["labelGroup"]));
            }
            layout
        });
        let shape = match draw(&ShapeInput {
            node: &node,
            label: layout.as_ref(),
            config: &unified,
            dir: Some("TB"),
        }) {
            Ok(shape) => shape,
            Err(error) => {
                failures.push(format!("{} {}: {error}", node.shape, node.look));
                continue;
            }
        };
        failures.extend(check(case, &shape, &mut deviation));
    }
    println!(
        "flowchart shapes {stage}: {} cases, largest deviation {:.4} px (tolerance {TOLERANCE} px)",
        cases.len(),
        deviation.worst
    );
    assert!(
        failures.is_empty(),
        "{} of {} cases differ from the reference:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}
