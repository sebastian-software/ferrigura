//! Render-level tests of the sequence diagram: the output of the reference's rendering specs, read
//! back from the SVG. The bounds block of `sequenceDiagram.spec.js` (`:1783-1916`) is unit tests in
//! `src/sequence/bounds.rs`, and `drawText` of `svgDraw.js` is unit tests in `src/sequence/text.rs`.
//!
//! The reference reads its `bounds` object after a draw, under a jsdom text mock (text lines are 10
//! px high, widths are mocked). Ferrigura keeps the bounds inside the renderer, so the relations are
//! read from the drawing: [`drawn_edges`] gives the edges from the view box and the margins, the way
//! the renderer sets the view box from the bounds. Each relation is restated with the real text
//! measurement. The constants of the reference (`+10`, `+20`) are kept.

use ferrigura_model::config::{self, MermaidConfig};
use ferrigura_model::theme::{Theme, ThemeTokens};
use ferrigura_model::value::Value;
use ferrigura_render::context::{Context, Globals, ThemeView};
use ferrigura_render::sequence;
use ferrigura_syntax::sequence::{Options, parse_with};

/// The font of the geometry harness (`tools/port/geometry/common.ts`, `FORCED_SETTINGS`).
const FONT_FAMILY: &str = "\"Lexica Ultralegible\"";

/// An element of the SVG, read back with its attributes and children.
#[derive(Debug, Default)]
struct Node {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<Node>,
    text: String,
}

impl Node {
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// A numeric attribute; panics when it is absent, which is a test failure.
    fn num(&self, name: &str) -> f64 {
        self.attr(name)
            .unwrap_or_else(|| panic!("<{}> has no {name}", self.name))
            .parse()
            .unwrap_or_else(|_| panic!("<{}> has a {name} that is not a number", self.name))
    }

    fn has_class(&self, class: &str) -> bool {
        self.attr("class")
            .is_some_and(|value| value.split_whitespace().any(|token| token == class))
    }

    /// The text of the element and of its descendants.
    fn text_content(&self) -> String {
        let mut text = self.text.clone();
        for child in &self.children {
            text.push_str(&child.text_content());
        }
        text
    }
}

/// Every element of the subtree whose predicate holds, in document order.
fn all<'a>(node: &'a Node, keep: &dyn Fn(&Node) -> bool) -> Vec<&'a Node> {
    let mut found = Vec::new();
    collect(node, keep, &mut found);
    found
}

fn collect<'a>(node: &'a Node, keep: &dyn Fn(&Node) -> bool, found: &mut Vec<&'a Node>) {
    for child in &node.children {
        if keep(child) {
            found.push(child);
        }
        collect(child, keep, found);
    }
}

/// Decodes the entities the renderer writes (`&amp;`, `&quot;`, `&lt;`, `&gt;`, `&apos;`, and the
/// numeric references).
fn decode(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        let Some(end) = rest.find(';') else { break };
        let entity = &rest[1..end];
        let decoded = match entity {
            "amp" => Some('&'),
            "quot" => Some('"'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "apos" => Some('\''),
            _ => entity
                .strip_prefix('#')
                .and_then(|digits| match digits.strip_prefix('x') {
                    Some(hex) => u32::from_str_radix(hex, 16).ok(),
                    None => digits.parse::<u32>().ok(),
                })
                .and_then(char::from_u32),
        };
        match decoded {
            Some(character) => out.push(character),
            None => out.push_str(&rest[..=end]),
        }
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}

/// Parses an SVG document into its element tree. The renderer writes well-formed markup with
/// quoted attributes and no comments, so a small scanner is enough.
fn parse_svg(svg: &str) -> Node {
    let mut stack = vec![Node::default()];
    let mut rest = svg;
    while !rest.is_empty() {
        let Some(open) = rest.find('<') else {
            stack.last_mut().unwrap().text.push_str(&decode(rest));
            break;
        };
        if open > 0 {
            stack
                .last_mut()
                .unwrap()
                .text
                .push_str(&decode(&rest[..open]));
        }
        rest = &rest[open..];
        if rest.starts_with("<!") || rest.starts_with("<?") {
            rest = &rest[rest.find('>').map_or(rest.len(), |end| end + 1)..];
        } else if let Some(after) = rest.strip_prefix("</") {
            let end = after.find('>').unwrap_or(after.len());
            let closed = stack.pop().expect("a closing tag has an open element");
            stack.last_mut().unwrap().children.push(closed);
            rest = &after[(end + 1).min(after.len())..];
        } else {
            let mut node = Node::default();
            let body = &rest[1..];
            let name_end = body
                .find(|c: char| c.is_whitespace() || c == '/' || c == '>')
                .unwrap_or(body.len());
            node.name = body[..name_end].to_owned();
            let mut cursor = &body[name_end..];
            let self_closing = loop {
                cursor = cursor.trim_start();
                if let Some(after) = cursor.strip_prefix("/>") {
                    cursor = after;
                    break true;
                }
                if let Some(after) = cursor.strip_prefix('>') {
                    cursor = after;
                    break false;
                }
                let key_end = cursor
                    .find(|c: char| c == '=' || c.is_whitespace() || c == '>' || c == '/')
                    .unwrap_or(cursor.len());
                let key = cursor[..key_end].to_owned();
                cursor = cursor[key_end..].trim_start();
                let Some(after_equals) = cursor.strip_prefix('=') else {
                    // A bare attribute carries no value.
                    node.attrs.push((key, String::new()));
                    continue;
                };
                let after_equals = after_equals.trim_start();
                let quote = after_equals
                    .chars()
                    .next()
                    .expect("an attribute has a value");
                let value_body = &after_equals[quote.len_utf8()..];
                let value_end = value_body
                    .find(quote)
                    .expect("an attribute value is closed");
                node.attrs.push((key, decode(&value_body[..value_end])));
                cursor = &value_body[value_end + quote.len_utf8()..];
            };
            rest = cursor;
            if self_closing {
                stack.last_mut().unwrap().children.push(node);
            } else {
                stack.push(node);
            }
        }
    }
    while stack.len() > 1 {
        let closed = stack.pop().unwrap();
        stack.last_mut().unwrap().children.push(closed);
    }
    stack
        .pop()
        .unwrap()
        .children
        .into_iter()
        .next()
        .expect("an svg element")
}

/// The configuration the geometry harness renders with: the defaults, the classic look, the
/// redux-color theme and the harness font.
fn render_config() -> MermaidConfig {
    let mut config = config::typed(&config::defaults()).expect("the defaults are a configuration");
    config.look = "classic".into();
    config.theme = "redux-color".into();
    config.font_family = FONT_FAMILY.into();
    config
}

/// The sequence section of the APA block (`sequenceDiagram.spec.js:1918-1931`).
fn apa_config() -> MermaidConfig {
    let mut config = render_config();
    let section = &mut config.sequence;
    section.diagram_margin_x = 50;
    section.diagram_margin_y = 10;
    section.actor_margin = 50;
    section.width = 150;
    section.height = 65;
    section.box_margin = 10;
    section.message_margin = 40;
    section.box_text_margin = 15;
    section.note_margin = 25;
    section.wrap = false;
    section.mirror_actors = false;
    config
}

/// The sequence section of the mirror block (`sequenceDiagram.spec.js:2337-2352`).
fn mirror_config() -> MermaidConfig {
    let mut config = apa_config();
    config.sequence.mirror_actors = true;
    config.sequence.bottom_margin_adj = 1;
    config
}

impl Node {
    /// A numeric attribute, zero when it is absent (the reference's `getAttribute(...) ?? 0`).
    fn num_or_zero(&self, name: &str) -> f64 {
        self.attr(name)
            .and_then(|value| value.parse().ok())
            .unwrap_or(0.0)
    }
}

/// The view box of the drawing: `min-x`, `min-y`, `width`, `height`.
fn view_box(svg: &Node) -> [f64; 4] {
    let value = svg.attr("viewBox").expect("the svg has a view box");
    let numbers: Vec<f64> = value
        .split_whitespace()
        .map(|part| part.parse().expect("a view box number"))
        .collect();
    numbers.try_into().expect("a view box has four numbers")
}

/// The edges of the drawing, with the origin the reference's bounds start from. The renderer sets
/// the view box to the bounds plus the diagram margins (and, on a mirrored diagram, minus the
/// margin the footer does not need), so the bounds are read back by undoing those.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Edges {
    startx: f64,
    starty: f64,
    stopx: f64,
    stopy: f64,
}

fn drawn_edges(svg: &Node, config: &MermaidConfig) -> Edges {
    let [min_x, min_y, width, height] = view_box(svg);
    let margin_x = config.sequence.diagram_margin_x as f64;
    let margin_y = config.sequence.diagram_margin_y as f64;
    let mut box_height = height - 2.0 * margin_y;
    if config.sequence.mirror_actors {
        box_height += config.sequence.box_margin as f64 - config.sequence.bottom_margin_adj as f64;
    }
    let startx = min_x + margin_x;
    let starty = min_y + margin_y;
    Edges {
        startx,
        starty,
        stopx: startx + width - 2.0 * margin_x,
        stopy: starty + box_height,
    }
}

/// The lines the messages are drawn on, in the order they were drawn.
fn message_lines(svg: &Node) -> Vec<&Node> {
    all(svg, &|node| {
        node.name == "line" && node.attr("data-et") == Some("message")
    })
}

/// `lastMessage().stopy`: the model's bottom of the last message. The reference bounds a message
/// by bumping the row ten before its text and drawing its line at the end of the row, so the model
/// stops ten above that line.
fn last_message_stopy(svg: &Node) -> f64 {
    let lines = message_lines(svg);
    let last = lines.last().expect("the diagram has a message");
    last.num("y1") - 10.0
}

/// The SVG the renderer writes for a diagram under a configuration and theme variables.
fn render_svg_with(text: &str, config: &MermaidConfig, variables: &Value) -> String {
    let diagram = parse_with(
        text,
        Options {
            wrap: config.sequence.wrap,
        },
    )
    .expect("the diagram parses");
    let tokens = ThemeTokens::from_value(variables);
    sequence::render(&Context {
        diagram: &diagram,
        section: &config.sequence,
        globals: &Globals::of(config),
        theme: ThemeView {
            light: &tokens,
            dark: None,
        },
        id: "t",
    })
    .expect("the diagram renders")
}

/// The SVG the renderer writes for a diagram under a configuration, with the theme's variables.
fn render_svg(text: &str, config: &MermaidConfig) -> String {
    render_svg_with(text, config, &variables_of(&config.theme, None))
}

/// The theme's variables, with the overrides applied (`themeVariables`).
fn variables_of(theme: &str, overrides: Option<&Value>) -> Value {
    Theme::from_name(theme).variables(overrides)
}

/// The element tree of the SVG the renderer writes for a diagram under a configuration.
fn render(text: &str, config: &MermaidConfig) -> Node {
    parse_svg(&render_svg(text, config))
}

/// The element tree of the SVG for a diagram under a configuration and theme variables.
fn render_with(text: &str, config: &MermaidConfig, variables: &Value) -> Node {
    parse_svg(&render_svg_with(text, config, variables))
}

/// The `wrap` flag of each message of a diagram, parsed with the given automatic wrap.
fn message_wraps(text: &str, wrap: bool) -> Vec<bool> {
    parse_with(text, Options { wrap })
        .expect("the diagram parses")
        .messages
        .iter()
        .map(|message| message.wrap)
        .collect()
}

/// The top rectangle of an actor, by its name.
fn actor_rect<'a>(svg: &'a Node, name: &str) -> &'a Node {
    let rects = all(svg, &|node| {
        node.name == "rect" && node.has_class("actor-top") && node.attr("name") == Some(name)
    });
    rects
        .first()
        .copied()
        .unwrap_or_else(|| panic!("the actor {name} is drawn"))
}

/// The right edge of the rightmost actor: the `stopx` that the actors give the bounds.
fn rightmost_actor_edge(svg: &Node) -> f64 {
    all(svg, &|node| {
        node.name == "rect" && node.has_class("actor-top")
    })
    .iter()
    .map(|rect| rect.num("x") + rect.num("width"))
    .fold(f64::NEG_INFINITY, f64::max)
}

/// The rectangles of the notes, in document order.
fn note_rects(svg: &Node) -> Vec<&Node> {
    let notes = all(svg, &|node| {
        node.name == "g" && node.attr("data-et") == Some("note")
    });
    notes
        .into_iter()
        .filter_map(|note| note.children.iter().find(|child| child.name == "rect"))
        .collect()
}

/// The bottom edge of the last note (`lastNote().stopy`): its top and its height.
fn last_note_bottom(svg: &Node) -> f64 {
    let rects = note_rects(svg);
    let rect = rects.last().expect("the diagram has a note");
    rect.num("y") + rect.num("height")
}

/// The bottom actor of a mirrored diagram: its top and its height (`lastActor()`).
fn last_bottom_actor(svg: &Node) -> (f64, f64) {
    let rects = all(svg, &|node| {
        node.name == "rect" && node.has_class("actor-bottom")
    });
    let actor = rects.last().expect("the diagram has a bottom actor");
    (actor.num("y"), actor.num("height"))
}

/// The lowest edge of the loop frames: the bottom of the last loop (`lastLoop().stopy`).
fn loop_bottom(svg: &Node) -> f64 {
    all(svg, &|node| {
        node.name == "line" && node.has_class("loopLine")
    })
    .iter()
    .flat_map(|line| [line.num("y1"), line.num("y2")])
    .fold(f64::NEG_INFINITY, f64::max)
}

/// The rectangle of a `rect` block: the background that the block draws (`class="rect"`).
fn background_rect(svg: &Node) -> &Node {
    let rects = all(svg, &|node| node.name == "rect" && node.has_class("rect"));
    rects
        .last()
        .copied()
        .expect("the diagram draws a background")
}

/// The text of the sequence numbers, in document order.
fn sequence_numbers(svg: &Node) -> Vec<String> {
    all(svg, &|node| {
        node.name == "text" && node.has_class("sequenceNumber")
    })
    .iter()
    .map(|text| text.text_content())
    .collect()
}

/// The participant group with a given id.
fn participant<'a>(svg: &'a Node, id: &str) -> &'a Node {
    let groups = all(svg, &|node| {
        node.name == "g"
            && node.attr("data-et") == Some("participant")
            && node.attr("data-id") == Some(id)
    });
    groups
        .first()
        .copied()
        .unwrap_or_else(|| panic!("the participant {id} is drawn"))
}

/// The lifeline of a participant.
fn lifeline<'a>(svg: &'a Node, id: &str) -> &'a Node {
    let lines = all(svg, &|node| {
        node.name == "line"
            && node.attr("data-et") == Some("life-line")
            && node.attr("data-id") == Some(id)
    });
    lines
        .first()
        .copied()
        .unwrap_or_else(|| panic!("the lifeline of {id} is drawn"))
}

/// The label of a participant: its first text with content.
fn label_of(group: &Node) -> &Node {
    all(group, &|node| {
        node.name == "text" && !node.text_content().is_empty()
    })
    .first()
    .copied()
    .or_else(|| (group.name == "text").then_some(group))
    .expect("the participant has a label")
}

/// The height of one line of the sans font at a size, as the text measurement gives it.
fn line_height(size: f64) -> f64 {
    let face = ferrigura_text::face(
        ferrigura_text::FontFamily::Sans,
        ferrigura_text::FontWeight::Normal,
        ferrigura_text::FontStyle::Normal,
    );
    ferrigura_text::text_box(face, size)
        .expect("the bundled font is read")
        .height
}

/// The colours of the theme's palette, read from its variables.
fn palette(variables: &Value, key: &str) -> Vec<String> {
    variables
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// The stroke and fill of an actor, read from the inline style of its drawing.
fn actor_colours(svg: &Node, id: &str) -> (Option<String>, Option<String>) {
    let group = participant(svg, id);
    let styled = all(group, &|node| {
        node.attr("style")
            .is_some_and(|style| style.contains("stroke:"))
    });
    let style = styled
        .first()
        .and_then(|node| node.attr("style"))
        .unwrap_or_default();
    let declaration = |name: &str| {
        style
            .split(';')
            .filter_map(|part| part.split_once(':'))
            .find(|(key, _)| key.trim() == name)
            .map(|(_, value)| value.trim_start().to_owned())
    };
    (declaration("stroke"), declaration("fill"))
}

/// The declarations of the first style rule whose selector names a class.
fn rule_declarations<'a>(css: &'a str, class: &str) -> &'a str {
    let start = css
        .find(class)
        .unwrap_or_else(|| panic!("the style sheet has a rule for {class}"));
    let open = start + css[start..].find('{').expect("the rule has a block");
    let close = open + css[open..].find('}').expect("the block closes");
    &css[open + 1..close]
}

mod sequence_diagram_apa_rendering {
    //! `sequenceDiagram.spec.js:1917-2335`, the APA rendering block. Its `bounds` reads are the
    //! [`drawn_edges`](super::drawn_edges) of the drawing. Where the reference's expected `stopx`
    //! counts the actor spacing under its mocked text width, the test reads the actors from the
    //! drawing and keeps the reference's value as the least it can be: a real message can only
    //! widen the spacing.

    use super::*;

    // The reference's `it.each` rows set nothing: `textPlacement` is read by `svgDraw.js` from the
    // configuration, and the block never sets it. So the four rows render the same drawing, and each
    // row is one test with the reference's name.

    #[test]
    fn it_should_handle_one_actor_when_text_placement_is_tspan() {
        // Reference: sequenceDiagram.spec.js:1957, the `textPlacement` table, row `tspan`.
        one_actor_bounds();
    }

    #[test]
    fn it_should_handle_one_actor_when_text_placement_is_fo() {
        // Reference: sequenceDiagram.spec.js:1957, row `fo`.
        one_actor_bounds();
    }

    #[test]
    fn it_should_handle_one_actor_when_text_placement_is_old() {
        // Reference: sequenceDiagram.spec.js:1957, row `old`.
        one_actor_bounds();
    }

    #[test]
    fn it_should_handle_one_actor_when_text_placement_is_undefined() {
        // Reference: sequenceDiagram.spec.js:1957, row `undefined`.
        one_actor_bounds();
    }

    /// The bounds of one participant: origin at the top left, as wide as the actor and as high as
    /// its box.
    fn one_actor_bounds() {
        let config = apa_config();
        let svg = render("sequenceDiagram\nparticipant Alice", &config);
        let edges = drawn_edges(&svg, &config);
        assert_eq!(edges.startx, 0.0);
        assert_eq!(edges.starty, 0.0);
        assert_eq!(edges.stopx, config.sequence.width as f64);
        assert_eq!(edges.stopy, config.sequence.height as f64);
    }

    #[test]
    fn should_handle_same_actor_with_different_whitespace_properly() {
        // Reference: sequenceDiagram.spec.js:1974.
        let svg = render(
            "sequenceDiagram\nparticipant Alice\nparticipant Alice\nparticipant Alice\n",
            &apa_config(),
        );
        let participants = all(&svg, &|node| {
            node.name == "g" && node.attr("data-et") == Some("participant")
        });
        assert_eq!(participants.len(), 1, "one actor is drawn");
        assert_eq!(participants[0].attr("data-id"), Some("Alice"));
    }

    #[test]
    fn should_handle_one_actor_and_a_centered_note() {
        // Reference: sequenceDiagram.spec.js:1985.
        let config = apa_config();
        let svg = render(
            "sequenceDiagram\nparticipant Alice\nNote over Alice: Alice thinks\n",
            &config,
        );
        let edges = drawn_edges(&svg, &config);
        assert_eq!(edges.startx, 0.0);
        assert_eq!(edges.starty, 0.0);
        assert_eq!(edges.stopx, config.sequence.width as f64);
        assert_eq!(edges.stopy, last_note_bottom(&svg));
    }

    #[test]
    fn should_handle_one_actor_and_a_note_to_the_left() {
        // Reference: sequenceDiagram.spec.js:2003.
        let config = apa_config();
        let svg = render(
            "sequenceDiagram\nparticipant Alice\nNote left of Alice: Alice thinks",
            &config,
        );
        let conf = &config.sequence;
        let edges = drawn_edges(&svg, &config);
        assert_eq!(
            edges.startx,
            -(conf.width as f64 / 2.0) - conf.actor_margin as f64 / 2.0
        );
        assert_eq!(edges.starty, 0.0);
        assert_eq!(edges.stopx, conf.width as f64);
        assert_eq!(edges.stopy, last_note_bottom(&svg));
    }

    #[test]
    fn should_handle_one_actor_and_a_note_to_the_right() {
        // Reference: sequenceDiagram.spec.js:2019.
        let config = apa_config();
        let svg = render(
            "sequenceDiagram\nparticipant Alice\nNote right of Alice: Alice thinks",
            &config,
        );
        let conf = &config.sequence;
        let edges = drawn_edges(&svg, &config);
        assert_eq!(edges.startx, 0.0);
        assert_eq!(edges.starty, 0.0);
        assert_eq!(
            edges.stopx,
            conf.width as f64 / 2.0 + conf.actor_margin as f64 / 2.0 + conf.width as f64
        );
        assert_eq!(edges.stopy, last_note_bottom(&svg));
    }

    #[test]
    fn should_handle_two_actors() {
        // Reference: sequenceDiagram.spec.js:2035.
        let config = apa_config();
        let svg = render(
            "sequenceDiagram\nAlice->Bob: Hello Bob, how are you?",
            &config,
        );
        let conf = &config.sequence;
        let edges = drawn_edges(&svg, &config);
        assert_eq!(edges.startx, 0.0);
        assert_eq!(edges.starty, 0.0);
        assert_eq!(edges.stopx, rightmost_actor_edge(&svg));
        assert!(edges.stopx >= (conf.width * 2 + conf.actor_margin) as f64);
        assert_eq!(edges.stopy, last_message_stopy(&svg) + 10.0);
    }

    #[test]
    fn should_handle_two_actors_in_a_box() {
        // Reference: sequenceDiagram.spec.js:2048.
        let config = apa_config();
        let svg = render(
            "sequenceDiagram\nbox rgb(34, 56, 0) Group1\nparticipant Alice\nparticipant Bob\nend\nAlice->Bob: Hello Bob, how are you?",
            &config,
        );
        let conf = &config.sequence;
        let edges = drawn_edges(&svg, &config);
        assert_eq!(edges.startx, 0.0);
        assert_eq!(edges.starty, 0.0);
        // The box is as wide as the actors and the box text margin on the right (`box.width`).
        assert_eq!(
            edges.stopx,
            rightmost_actor_edge(&svg) + conf.box_text_margin as f64
        );
        assert!(edges.stopx >= (conf.width * 2 + conf.actor_margin + conf.box_text_margin) as f64);
        assert_eq!(edges.stopy, last_message_stopy(&svg) + 20.0);
    }

    #[test]
    fn should_handle_two_actors_with_init_directive() {
        // Reference: sequenceDiagram.spec.js:2066.
        let config = apa_config();
        let plain = "sequenceDiagram\nAlice->Bob: Hello Bob, how are you?";
        let directive = format!("%%{{init: {{'logLevel': 0}}}}%%\n{plain}");
        // The directive is read into the configuration, which the drawing does not use for the log
        // level: the drawing is the same with and without it.
        assert_eq!(render_svg(&directive, &config), render_svg(plain, &config));
        let svg = render(&directive, &config);
        let edges = drawn_edges(&svg, &config);
        assert_eq!(edges.stopx, rightmost_actor_edge(&svg));
        assert_eq!(edges.stopy, last_message_stopy(&svg) + 10.0);
    }

    #[test]
    fn should_handle_two_actors_with_init_directive_with_multiline_directive() {
        // Reference: sequenceDiagram.spec.js:2083. The `%%{wrap}%%` directive sets the `wrap` the
        // configuration reads, so every message wraps.
        let mut config = apa_config();
        config.sequence.wrap = true;
        let text = "%%{init: { 'logLevel': 0}}%%\nsequenceDiagram\n%%{wrap}%%\nAlice->Bob: Hello Bob, how are you?";
        assert!(message_wraps(text, true).iter().all(|wrap| *wrap));
        let svg = render(text, &config);
        let edges = drawn_edges(&svg, &config);
        assert_eq!(edges.stopx, rightmost_actor_edge(&svg));
        assert_eq!(edges.stopy, last_message_stopy(&svg) + 10.0);
    }

    #[test]
    fn should_handle_two_actors_and_two_centered_shared_notes() {
        // Reference: sequenceDiagram.spec.js:2104.
        let config = apa_config();
        let svg = render(
            "sequenceDiagram\nAlice->Bob: Hello Bob, how are you?\nNote over Alice,Bob: Looks\nNote over Bob,Alice: Looks back\n",
            &config,
        );
        let edges = drawn_edges(&svg, &config);
        assert_eq!(edges.startx, 0.0);
        assert_eq!(edges.starty, 0.0);
        assert_eq!(edges.stopx, rightmost_actor_edge(&svg));
        assert_eq!(edges.stopy, last_note_bottom(&svg));
    }

    #[test]
    fn should_draw_two_actors_and_two_messages() {
        // Reference: sequenceDiagram.spec.js:2121.
        let config = apa_config();
        let svg = render(
            "sequenceDiagram\nAlice->Bob: Hello Bob, how are you?\nBob->Alice: Fine!",
            &config,
        );
        let edges = drawn_edges(&svg, &config);
        assert_eq!(edges.startx, 0.0);
        assert_eq!(edges.starty, 0.0);
        assert_eq!(edges.stopx, rightmost_actor_edge(&svg));
        assert_eq!(edges.stopy, last_message_stopy(&svg) + 10.0);
    }

    #[test]
    fn should_draw_two_actors_notes_to_the_right() {
        // Reference: sequenceDiagram.spec.js:2136. The note sits `noteMargin` to the right of Bob's
        // centre, so its right edge is the reference's `expStopX`, under the mocked spacing.
        let config = apa_config();
        let conf = &config.sequence;
        let svg = render(
            "sequenceDiagram\nAlice->Bob: Hello Bob, how are you?\nNote right of Bob: Bob thinks\nBob->Alice: Fine!",
            &config,
        );
        let edges = drawn_edges(&svg, &config);
        assert_eq!(edges.startx, 0.0);
        assert_eq!(edges.starty, 0.0);
        let bob = actor_rect(&svg, "Bob");
        let note = note_rects(&svg)[0];
        assert_eq!(
            note.num("x"),
            bob.num("x") + bob.num("width") / 2.0 + conf.note_margin as f64
        );
        let note_right = note.num("x") + note.num("width");
        assert_eq!(edges.stopx, note_right);
        let expected_stopx =
            (conf.actor_margin + conf.width + conf.width / 2 + conf.note_margin + conf.width)
                as f64;
        assert!(edges.stopx >= expected_stopx);
        assert_eq!(edges.stopy, last_message_stopy(&svg) + 10.0);
    }

    #[test]
    fn should_draw_two_actors_notes_to_the_left() {
        // Reference: sequenceDiagram.spec.js:2155.
        let config = apa_config();
        let conf = &config.sequence;
        let svg = render(
            "sequenceDiagram\nAlice->Bob: Hello Bob, how are you?\nNote left of Alice: Bob thinks\nBob->Alice: Fine!",
            &config,
        );
        let edges = drawn_edges(&svg, &config);
        assert_eq!(
            edges.startx,
            -(conf.width as f64 / 2.0) - conf.actor_margin as f64 / 2.0
        );
        assert_eq!(edges.starty, 0.0);
        assert_eq!(edges.stopx, rightmost_actor_edge(&svg));
        assert!(edges.stopx >= (conf.width * 2 + conf.actor_margin) as f64);
        assert_eq!(edges.stopy, last_message_stopy(&svg) + 10.0);
    }

    #[test]
    fn should_draw_two_actors_notes_to_the_left_with_text_wrapped_inline() {
        // Reference: sequenceDiagram.spec.js:2172. The `wrap:` prefix makes the message wrap.
        let config = apa_config();
        let conf = &config.sequence;
        let text = "sequenceDiagram\nAlice->>Bob:wrap: Hello Bob, how are you? If you are not available right now, I can leave you a message. Please get back to me as soon as you can!\nNote left of Alice: Bob thinks\nBob->>Alice: Fine!";
        assert!(message_wraps(text, false)[0]);
        let svg = render(text, &config);
        let edges = drawn_edges(&svg, &config);
        assert_eq!(
            edges.startx,
            -(conf.width as f64 / 2.0) - conf.actor_margin as f64 / 2.0
        );
        assert_eq!(edges.starty, 0.0);
        assert_eq!(edges.stopx, rightmost_actor_edge(&svg));
        assert!(edges.stopx >= (conf.width * 2 + conf.actor_margin) as f64);
        assert_eq!(edges.stopy, last_message_stopy(&svg) + 10.0);
    }

    #[test]
    fn should_draw_two_actors_notes_to_the_left_with_text_wrapped_directive() {
        // Reference: sequenceDiagram.spec.js:2191. The `%%{wrap}%%` directive makes every message wrap.
        let mut config = apa_config();
        config.sequence.wrap = true;
        let conf = &config.sequence;
        let text = "sequenceDiagram\n%%{wrap}%%\nAlice->>Bob: Hello Bob, how are you? If you are not available right now, I can leave you a message. Please get back to me as soon as you can!\nNote left of Alice: Bob thinks\nBob->>Alice: Fine!";
        assert!(message_wraps(text, true).iter().all(|wrap| *wrap));
        let svg = render(text, &config);
        let edges = drawn_edges(&svg, &config);
        assert_eq!(
            edges.startx,
            -(conf.width as f64 / 2.0) - conf.actor_margin as f64 / 2.0
        );
        assert_eq!(edges.starty, 0.0);
        assert_eq!(edges.stopx, rightmost_actor_edge(&svg));
        assert_eq!(edges.stopy, last_message_stopy(&svg) + 10.0);
    }

    #[test]
    fn should_draw_two_actors_notes_to_the_left_with_text_wrapped_and_the_init_directive_sets_the_theme_to_dark()
     {
        // Reference: sequenceDiagram.spec.js:2215. The directive sets `theme: 'dark'`, which resolves
        // to the dark palette: the drawing is the one of `redux-dark-color`.
        let mut config = apa_config();
        config.sequence.wrap = true;
        // `theme: 'dark'` resolves to the dark palette the harness ships; the renderer gets that name.
        assert_eq!(Theme::from_name("dark"), Theme::ReduxDarkColor);
        config.theme = "redux-dark-color".into();
        let conf = &config.sequence;
        let text = "sequenceDiagram\n%%{wrap}%%\nAlice->>Bob: Hello Bob, how are you? If you are not available right now, I can leave you a message. Please get back to me as soon as you can!\nNote left of Alice: Bob thinks\nBob->>Alice: Fine!";
        assert_eq!(
            drawn_edges(&render(text, &config), &config).startx,
            -(conf.width as f64 / 2.0) - conf.actor_margin as f64 / 2.0
        );
        let svg = render(text, &config);
        let edges = drawn_edges(&svg, &config);
        assert_eq!(edges.stopx, rightmost_actor_edge(&svg));
        assert_eq!(edges.stopy, last_message_stopy(&svg) + 10.0);
    }

    #[test]
    fn should_draw_two_actors_notes_to_the_left_with_text_wrapped_and_the_init_directive_sets_the_theme_to_dark_and_font_family_to_menlo_font_size_to_18_and_font_weight_to_800()
     {
        // Reference: sequenceDiagram.spec.js:2240. The directive sets the top-level font family and
        // size, and the message font weight to 400 (the reference's assertion; the title says 800).
        let mut config = apa_config();
        config.sequence.wrap = true;
        config.theme = "redux-dark-color".into();
        config.font_family = "Menlo".into();
        config.font_size = 18.0;
        config.sequence.message_font_weight =
            Some(ferrigura_model::value::Value::from(serde_json::json!(400)));
        let conf = &config.sequence;
        let text = "sequenceDiagram\nAlice->>Bob: Hello Bob, how are you? If you are not available right now, I can leave you a message. Please get back to me as soon as you can!\nNote left of Alice: Bob thinks\nBob->>Alice: Fine!";
        assert!(message_wraps(text, true).iter().all(|wrap| *wrap));
        let svg = render(text, &config);
        let edges = drawn_edges(&svg, &config);
        assert_eq!(
            edges.startx,
            -(conf.width as f64 / 2.0) - conf.actor_margin as f64 / 2.0
        );
        assert_eq!(edges.starty, 0.0);
        // The message text is set in the configured family and size.
        let message_texts = all(&svg, &|node| {
            node.name == "text" && node.has_class("messageText")
        });
        assert!(!message_texts.is_empty());
        for text in message_texts {
            let style = text.attr("style").unwrap_or_default();
            assert!(style.contains("font-family: Menlo"), "style: {style}");
            assert!(style.contains("font-size: 18px"), "style: {style}");
        }
        assert_eq!(edges.stopx, rightmost_actor_edge(&svg));
        assert_eq!(edges.stopy, last_message_stopy(&svg) + 10.0);
    }

    #[test]
    fn should_draw_two_loops() {
        // Reference: sequenceDiagram.spec.js:2266.
        let config = apa_config();
        let svg = render(
            "sequenceDiagram\nAlice->Bob: Hello Bob, how are you?\nloop Cheers\nBob->Alice: Fine!\nend",
            &config,
        );
        let edges = drawn_edges(&svg, &config);
        assert_eq!(edges.startx, 0.0);
        assert_eq!(edges.starty, 0.0);
        assert_eq!(edges.stopx, rightmost_actor_edge(&svg));
        assert_eq!(edges.stopy, loop_bottom(&svg));
    }

    #[test]
    fn should_draw_background_rect() {
        // Reference: sequenceDiagram.spec.js:2283.
        let config = apa_config();
        let svg = render(
            "sequenceDiagram\nAlice->Bob: Hello Bob, are you alright?\nrect rgb(0, 0, 0)\nBob->Alice: I feel surrounded by darkness\nend",
            &config,
        );
        let edges = drawn_edges(&svg, &config);
        assert_eq!(edges.startx, 0.0);
        assert_eq!(edges.starty, 0.0);
        assert_eq!(edges.stopx, rightmost_actor_edge(&svg));
        let rect = background_rect(&svg);
        assert_eq!(edges.stopy, rect.num("y") + rect.num("height"));
    }

    #[test]
    fn should_increment_the_sequence_number_with_a_decimal_in_the_hundredths_place() {
        // Reference: sequenceDiagram.spec.js:2301. The visibility of the numbers is set by draw.
        let config = apa_config();
        let svg = render(
            "sequenceDiagram\nautonumber 10.01 .01\nAlice->Bob:Hello Bob, how are you?\nBob-->Alice: I am good thanks!\nAlice-->Bob: Have a good day!",
            &config,
        );
        assert_eq!(sequence_numbers(&svg), ["10.01", "10.02", "10.03"]);
    }

    #[test]
    fn should_increment_the_sequence_number_with_a_decimal_in_the_tenths_place() {
        // Reference: sequenceDiagram.spec.js:2318.
        let config = apa_config();
        let svg = render(
            "sequenceDiagram\nautonumber 10.1 .1\nAlice->Bob:Hello Bob, how are you?\nBob-->Alice: I am good thanks!\nAlice-->Bob: Have a good day!",
            &config,
        );
        assert_eq!(sequence_numbers(&svg), ["10.1", "10.2", "10.3"]);
    }
}

mod sequence_diagram_mirror_rendering {
    //! `sequenceDiagram.spec.js:2336-2375`, the mirror block (its parse-level test is in
    //! `sequence_parse_spec.rs`).

    use super::*;

    // As in the APA block, the four rows render the same drawing: the block does not set the
    // `textPlacement` the rows name.

    #[test]
    fn should_handle_one_actor_when_text_placement_is_tspan() {
        // Reference: sequenceDiagram.spec.js:2370, row `tspan`.
        mirrored_one_actor_bounds();
    }

    #[test]
    fn should_handle_one_actor_when_text_placement_is_fo() {
        // Reference: sequenceDiagram.spec.js:2370, row `fo`.
        mirrored_one_actor_bounds();
    }

    #[test]
    fn should_handle_one_actor_when_text_placement_is_old() {
        // Reference: sequenceDiagram.spec.js:2370, row `old`.
        mirrored_one_actor_bounds();
    }

    #[test]
    fn should_handle_one_actor_when_text_placement_is_undefined() {
        // Reference: sequenceDiagram.spec.js:2370, row `undefined`.
        mirrored_one_actor_bounds();
    }

    /// The bounds of one mirrored participant: the footer's box ends `boxMargin` below its own
    /// bottom edge (`lastActor().stopy + height + boxMargin`).
    fn mirrored_one_actor_bounds() {
        let config = mirror_config();
        let svg = render("sequenceDiagram\nparticipant Alice", &config);
        let edges = drawn_edges(&svg, &config);
        let (top, height) = last_bottom_actor(&svg);
        assert_eq!(edges.startx, 0.0);
        assert_eq!(edges.starty, 0.0);
        assert_eq!(edges.stopx, config.sequence.width as f64);
        assert_eq!(
            edges.stopy,
            top + height + config.sequence.box_margin as f64
        );
    }
}
/// The configuration of a render under a look (`classic` or `neo`).
fn look_config(look: &str) -> MermaidConfig {
    let mut config = render_config();
    config.look = look.into();
    config
}

/// The number of distinct values, with a tolerance far below any drawn distance.
fn distinct(values: &[f64]) -> usize {
    values
        .iter()
        .map(|value| (value * 1000.0).round() as i64)
        .collect::<std::collections::BTreeSet<_>>()
        .len()
}

/// The text of the first message label.
fn message_label(svg: &Node) -> &Node {
    all(svg, &|node| {
        node.name == "text" && node.has_class("messageText")
    })
    .first()
    .copied()
    .expect("the diagram has a message label")
}

/// One participant of a kind, on its own, with the id `A`. `actor` is the stick figure keyword;
/// the other kinds are the `type` of a participant.
fn one_participant(kind: &str) -> String {
    if kind == "actor" {
        "sequenceDiagram\nactor A\n".to_owned()
    } else {
        format!("sequenceDiagram\nparticipant A@{{ \"type\" : \"{kind}\" }} as A\n")
    }
}

/// The drawing of one participant of a kind under a look.
fn alone(kind: &str, look: &str) -> Node {
    render(&one_participant(kind), &look_config(look))
}

/// The height of a glyph: from the top of its circle to the lowest line of the figure.
fn glyph_height(figure: &Node) -> f64 {
    let circle = all(figure, &|node| node.name == "circle")[0];
    let top = circle.num("cy") - circle.num("r");
    let feet = all(figure, &|node| node.name == "line")
        .iter()
        .map(|line| line.num("y2"))
        .fold(f64::NEG_INFINITY, f64::max);
    feet - top
}

/// The bottom edge of the box of a participant: the lowest rectangle of its drawing.
fn box_bottom(group: &Node) -> f64 {
    glyph_bottom(group, translate_y(group)).expect("the participant is drawn")
}

/// The height of the top box of a participant (its first `actor-top` rectangle).
fn box_height(group: &Node) -> f64 {
    all(group, &|node| {
        node.name == "rect" && node.has_class("actor-top")
    })[0]
        .num("height")
}

/// The `svgDraw.spec.js` rectangle, loop and background helpers. The reference's `drawText` cases
/// are unit tests in `src/sequence/text.rs`.
mod svg_draw {
    use super::*;
    use ferrigura_render::common::draw::{Bound, RectData, draw_background_rect, draw_rect};

    #[test]
    fn should_read_config_at_render_time_for_each_rect() {
        // Reference: svgDraw.spec.js:3. The reference's test double answers `classic` for the look.
        // Here the look is part of the configuration of the render: no rectangle of a classic
        // diagram carries `data-look`.
        let svg = render(
            "sequenceDiagram\nAlice->Bob: hi\nNote right of Bob: a\nloop L\nBob->Alice: b\nend",
            &look_config("classic"),
        );
        let rects = all(&svg, &|node| node.name == "rect");
        assert!(!rects.is_empty());
        assert!(rects.iter().all(|rect| rect.attr("data-look").is_none()));
    }

    #[test]
    fn should_read_config_at_render_time_for_each_rect_2() {
        // Reference: svgDraw.spec.js:42. The reference reads the look for each rectangle as it draws
        // it. A render has one look, so every note rectangle of a neo diagram is marked, and none of
        // a classic one.
        let text = "sequenceDiagram\nAlice->Bob: hi\nNote right of Bob: a\nNote left of Alice: b";
        let neo_svg = render(text, &look_config("neo"));
        let marked = note_rects(&neo_svg);
        assert_eq!(marked.len(), 2);
        assert!(
            marked
                .iter()
                .all(|rect| rect.attr("data-look") == Some("neo"))
        );
        let classic_svg = render(text, &look_config("classic"));
        let plain = note_rects(&classic_svg);
        assert_eq!(plain.len(), 2);
        assert!(plain.iter().all(|rect| rect.attr("data-look").is_none()));
    }

    #[test]
    fn should_append_a_rectangle() {
        // Reference: svgDraw.spec.js:71.
        let rect = draw_rect(&RectData {
            x: 10.0,
            y: 10.0,
            fill: Some("#ccc".into()),
            stroke: Some("red".into()),
            width: 20.0,
            height: 20.0,
            rx: Some(10.0),
            ry: Some(10.0),
            class: Some("unitTestRectangleClass".into()),
            ..RectData::default()
        });
        assert!(rect.render().starts_with("<rect "));
        assert_eq!(rect.get("x"), Some("10"));
        assert_eq!(rect.get("y"), Some("10"));
        assert_eq!(rect.get("fill"), Some("#ccc"));
        assert_eq!(rect.get("stroke"), Some("red"));
        assert_eq!(rect.get("width"), Some("20"));
        assert_eq!(rect.get("height"), Some("20"));
        assert_eq!(rect.get("rx"), Some("10"));
        assert_eq!(rect.get("ry"), Some("10"));
        assert_eq!(rect.get("class"), Some("unitTestRectangleClass"));
    }

    #[test]
    fn should_not_add_the_class_attribute_if_a_class_is_not_provided() {
        // Reference: svgDraw.spec.js:97.
        let rect = draw_rect(&RectData {
            x: 10.0,
            y: 10.0,
            fill: Some("#ccc".into()),
            stroke: Some("red".into()),
            width: 20.0,
            height: 20.0,
            rx: Some(10.0),
            ry: Some(10.0),
            ..RectData::default()
        });
        assert_eq!(rect.get("fill"), Some("#ccc"));
        assert_eq!(rect.get("class"), None);
    }

    #[test]
    fn should_use_section_title_class_for_section_titles_so_custom_loop_text_css_does_not_hide_them()
     {
        // Reference: svgDraw.spec.js:294. The `else` section's title is drawn with the section-title
        // class, and no loop text carries it.
        let svg = render(
            "sequenceDiagram\nAlice->Bob: hi\nalt Yes\nBob->Alice: a\nelse Command\nBob->Alice: b\nend",
            &look_config("classic"),
        );
        let sections = all(&svg, &|node| {
            node.name == "text" && node.has_class("sectionTitle")
        });
        assert!(
            sections
                .iter()
                .any(|text| text.text_content().contains("Command"))
        );
        let loop_texts = all(&svg, &|node| {
            node.name == "text" && node.has_class("loopText")
        });
        assert!(
            loop_texts
                .iter()
                .all(|text| !text.text_content().contains("Command"))
        );
    }

    #[test]
    fn should_append_a_rect_before_the_previous_element_within_a_given_bound() {
        // Reference: svgDraw.spec.js:332. The background is the rectangle of its bounds, with the
        // class `rect`, and the sequence renderer lowers it, so it is drawn before the content it sits
        // behind.
        let rect = draw_background_rect(&Bound {
            startx: 50.0,
            stopx: 150.0,
            starty: 200.0,
            stopy: 260.0,
            fill: "#ccc".into(),
            stroke: "#000".into(),
        });
        assert_eq!(rect.get("x"), Some("50"));
        assert_eq!(rect.get("y"), Some("200"));
        assert_eq!(rect.get("width"), Some("100"));
        assert_eq!(rect.get("height"), Some("60"));
        assert_eq!(rect.get("fill"), Some("#ccc"));
        assert_eq!(rect.get("class"), Some("rect"));

        let svg = render(
            "sequenceDiagram\nAlice->Bob: hi\nrect rgb(0, 0, 0)\nBob->Alice: b\nend",
            &look_config("classic"),
        );
        let order = all(&svg, &|_| true);
        let background = order
            .iter()
            .position(|node| node.name == "rect" && node.has_class("rect"))
            .expect("the background is drawn");
        let first_message = order
            .iter()
            .position(|node| node.name == "line" && node.attr("data-et") == Some("message"))
            .expect("a message is drawn");
        assert!(background < first_message, "the background comes first");
        let line = *message_lines(&svg)
            .last()
            .expect("a message is inside the block");
        let rect = background_rect(&svg);
        assert!(rect.num("x") <= line.num("x1").min(line.num("x2")));
        assert!(rect.num("x") + rect.num("width") >= line.num("x1").max(line.num("x2")));
        assert!(rect.num("y") <= line.num("y1"));
        assert!(rect.num("y") + rect.num("height") >= line.num("y1"));
    }
}

/// `sequenceRenderer.spec.js:46-96`.
mod sequence_renderer {
    use super::*;

    #[test]
    fn passes_min_startx_stopx_and_abs_stopx_startx_to_draw_text_when_startx_stopx() {
        // Reference: sequenceRenderer.spec.js:60. For a right-to-left message the reference gives the
        // text the smaller of the two x and the width between them. The label then starts at the left
        // end of the arrow, or sits in its middle when centred, as for a left-to-right message.
        let mut config = render_config();
        config.sequence.message_align = "left".into();
        let ltr = render("sequenceDiagram\nAlice->>Bob: RTL label", &config);
        let rtl = render("sequenceDiagram\nBob->>Alice: RTL label", &config);
        let (ltr_line, rtl_line) = (message_lines(&ltr)[0], message_lines(&rtl)[0]);
        let left = |line: &Node| line.num("x1").min(line.num("x2"));
        let ltr_offset = message_label(&ltr).num("x") - left(ltr_line);
        let rtl_offset = message_label(&rtl).num("x") - left(rtl_line);
        assert_eq!(rtl_offset, ltr_offset);

        config.sequence.message_align = "center".into();
        let centred = render("sequenceDiagram\nBob->>Alice: RTL label", &config);
        let line = message_lines(&centred)[0];
        let middle = (line.num("x1") + line.num("x2")) / 2.0;
        // The reference rounds the anchored x (`Math.round`), so a half is rounded up.
        assert_eq!(message_label(&centred).num("x"), (middle + 0.5).floor());
    }
}

/// `actorBandModel.spec.ts`: the actor bands, pure in the reference, drawn here with the pipeline.
/// The reference emulates `getBBox` with a 19 px text line; here the text is measured, and the
/// bands are read from the drawing.
mod actor_band_model {
    use super::*;
    use std::collections::BTreeMap;

    const MIXED_ROW: &str = "sequenceDiagram\n  actor User\n  participant Plain as Plain\n  participant B@{ \"type\" : \"boundary\" } as Bound\n  participant C@{ \"type\" : \"control\" } as Ctrl\n  participant E@{ \"type\" : \"entity\" } as Ent\n  participant DB@{ \"type\" : \"database\" } as Database\n  participant Q@{ \"type\" : \"queue\" } as Que\n  participant Co@{ \"type\" : \"collections\" } as Coll\n  User ->> DB: query\n";

    /// The labels that hang below a glyph: their baseline is shared.
    const OUTSIDE_LABELS: [&str; 5] = ["User", "Bound", "Ctrl", "Ent", "Database"];

    fn lifelines(svg: &Node) -> Vec<&Node> {
        all(svg, &|node| {
            node.name == "line" && node.attr("data-et") == Some("life-line")
        })
    }

    /// `splitLabels`: the labels above the first datum (header) and below the second (footer), by
    /// their text. The reference keeps the last of two labels with one text.
    fn split_labels(svg: &Node, datum_top: f64) -> (BTreeMap<String, f64>, BTreeMap<String, f64>) {
        let mut header = BTreeMap::new();
        let mut footer = BTreeMap::new();
        for text in all(svg, &|node| node.name == "text" && node.has_class("actor")) {
            let y = text.num("y");
            let side = if y <= datum_top {
                &mut header
            } else {
                &mut footer
            };
            side.insert(text.text_content(), y);
        }
        (header, footer)
    }

    #[test]
    fn starts_and_ends_every_lifeline_on_the_two_datum_lines() {
        // Reference: actorBandModel.spec.ts:139.
        let svg = render(MIXED_ROW, &look_config("neo"));
        let lines = lifelines(&svg);
        assert_eq!(lines.len(), 8);
        let tops: Vec<f64> = lines.iter().map(|line| line.num("y1")).collect();
        let bottoms: Vec<f64> = lines.iter().map(|line| line.num("y2")).collect();
        assert_eq!(distinct(&tops), 1);
        assert_eq!(distinct(&bottoms), 1);
    }

    #[test]
    fn puts_every_outside_label_on_one_baseline_clear_of_the_lifeline_header_and_footer() {
        // Reference: actorBandModel.spec.ts:148. The label's ink ends above the datum: its centre
        // plus half the measured line height.
        let svg = render(MIXED_ROW, &look_config("neo"));
        let datum_top = lifelines(&svg)[0].num("y1");
        let (header, footer) = split_labels(&svg, datum_top);
        let header_ys: Vec<f64> = OUTSIDE_LABELS
            .iter()
            .map(|name| {
                *header
                    .get(*name)
                    .unwrap_or_else(|| panic!("{name} has a header label"))
            })
            .collect();
        assert_eq!(distinct(&header_ys), 1);
        assert!(header_ys[0] + line_height(16.0) / 2.0 < datum_top);
        let footer_ys: Vec<f64> = OUTSIDE_LABELS
            .iter()
            .map(|name| {
                *footer
                    .get(*name)
                    .unwrap_or_else(|| panic!("{name} has a footer label"))
            })
            .collect();
        assert_eq!(distinct(&footer_ys), 1);
    }

    #[test]
    fn keeps_a_multiline_label_on_the_datum_growing_away_from_it() {
        // Reference: actorBandModel.spec.ts:162. A second line grows the row, the datum stays one
        // line, and the single-line neighbours keep their distance to the datum.
        let single_svg = render(MIXED_ROW, &look_config("neo"));
        let multi_text =
            MIXED_ROW.replace("actor User", "actor User as First line<br/>Second line");
        let multi_svg = render(&multi_text, &look_config("neo"));
        let multi_lines = lifelines(&multi_svg);
        let multi_tops: Vec<f64> = multi_lines.iter().map(|line| line.num("y1")).collect();
        assert_eq!(distinct(&multi_tops), 1);
        let single_datum = lifelines(&single_svg)[0].num("y1");
        let multi_datum = multi_lines[0].num("y1");
        assert!(multi_datum > single_datum);
        let single_database = split_labels(&single_svg, single_datum).0["Database"];
        let multi_database = split_labels(&multi_svg, multi_datum).0["Database"];
        assert_eq!(multi_datum - multi_database, single_datum - single_database);
    }

    #[test]
    fn renders_every_glyph_inside_the_bands_transforms_included() {
        // Reference: actorBandModel.spec.ts:187. Nothing pokes below the datum, and the icon family
        // (user, boundary, control, entity) sits feet on one line, with the transforms applied.
        let svg = render(MIXED_ROW, &look_config("neo"));
        let datum_top = lifelines(&svg)[0].num("y1");
        let mut bottoms = BTreeMap::new();
        for group in all(&svg, &|node| {
            node.name == "g" && node.attr("data-et") == Some("participant")
        }) {
            if let Some(bottom) = glyph_bottom(group, translate_y(group)) {
                bottoms.insert(group.attr("data-id").unwrap_or_default().to_owned(), bottom);
            }
        }
        for (id, bottom) in &bottoms {
            assert!(*bottom <= datum_top + 0.5, "{id} pokes below the datum");
        }
        let icons: Vec<f64> = ["User", "B", "C", "E"]
            .iter()
            .map(|id| (bottoms[*id] * 10.0).round() / 10.0)
            .collect();
        assert_eq!(distinct(&icons), 1);
    }

    #[test]
    fn reports_one_uniform_actor_height_back_into_the_pipeline() {
        // Reference: actorBandModel.spec.ts:220. Every lifeline spans the same height.
        let svg = render(MIXED_ROW, &look_config("neo"));
        let spans: Vec<f64> = lifelines(&svg)
            .iter()
            .map(|line| line.num("y2") - line.num("y1"))
            .collect();
        assert_eq!(distinct(&spans), 1);
    }
}

/// The `translate` of a node's own transform, in y.
fn translate_y(node: &Node) -> f64 {
    let Some(transform) = node.attr("transform") else {
        return 0.0;
    };
    let Some(start) = transform.find("translate(") else {
        return 0.0;
    };
    let inner = &transform[start + "translate(".len()..];
    let inner = &inner[..inner.find(')').unwrap_or(inner.len())];
    inner
        .split([',', ' '])
        .filter(|part| !part.is_empty())
        .nth(1)
        .and_then(|value| value.parse().ok())
        .unwrap_or(0.0)
}

/// The lowest y of a path, read the way the reference reads a path: the second number of each
/// `x,y` pair in its data.
fn path_bottom(path: &Node) -> Option<f64> {
    let data = path.attr("d")?;
    let pair = regex::Regex::new(r"([\d.-]+)[\s,]([\d.-]+)").expect("a valid pattern");
    pair.captures_iter(data)
        .filter_map(|captures| captures[2].parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .fold(None, |bottom: Option<f64>, value| {
            Some(bottom.map_or(value, |bottom| bottom.max(value)))
        })
}

/// The lowest point of the shapes of a participant, with each group's translation above a shape
/// applied, as the browser paints it. Only shapes count (`circle`, `line`, `rect`, `path`), and
/// not the lifeline.
fn glyph_bottom(node: &Node, offset: f64) -> Option<f64> {
    let mut bottom: Option<f64> = None;
    let mut keep = |value: f64| {
        bottom = Some(bottom.map_or(value, |bottom| bottom.max(value)));
    };
    for child in &node.children {
        let own = offset + translate_y(child);
        let shape = match child.name.as_str() {
            "circle" => Some(own + child.num_or_zero("cy") + child.num_or_zero("r")),
            "line" if child.attr("data-et") != Some("life-line") => {
                Some(own + child.num_or_zero("y1").max(child.num_or_zero("y2")))
            }
            "rect" => Some(own + child.num_or_zero("y") + child.num_or_zero("height")),
            "path" => path_bottom(child).map(|value| own + value),
            _ => None,
        };
        if let Some(value) = shape {
            keep(value);
        }
        if let Some(inner) = glyph_bottom(child, own) {
            keep(inner);
        }
    }
    bottom
}

/// `actorSizing.spec.ts`: the stick figure under the band model, against `classic` as the invariant.
mod stick_figure_actor_sizing {
    use super::*;

    /// The glyph band of the round icons (`actorBands.ts:41`).
    const GLYPH_BAND_HEIGHT: f64 = 44.0;

    /// The gap between a label and its lifeline (`actorBands.ts:47`).
    const LABEL_LIFELINE_GAP: f64 = 6.0;

    /// The y of the label of a participant with an id.
    fn label_y(svg: &Node) -> f64 {
        label_of(participant(svg, "A")).num("y")
    }

    #[test]
    fn puts_its_label_where_a_database_puts_its_own_on_classic() {
        // Reference: actorSizing.spec.ts:98, row `classic`.
        same_label_as_database("classic");
    }

    #[test]
    fn puts_its_label_where_a_database_puts_its_own_on_neo() {
        // Reference: actorSizing.spec.ts:98, row `neo`.
        same_label_as_database("neo");
    }

    fn same_label_as_database(look: &str) {
        let stick = label_y(&alone("actor", look));
        let database = label_y(&alone("database", look));
        assert_eq!(stick, database);
    }

    #[test]
    fn leaves_the_classic_figure_at_the_size_it_has_always_been() {
        // Reference: actorSizing.spec.ts:108. The classic head has the radius it always had, and the
        // figure fills its box.
        let svg = alone("actor", "classic");
        let figure = participant(&svg, "A");
        let head = all(figure, &|node| node.name == "circle")[0];
        assert_eq!(head.attr("r"), Some("15"));
        assert_eq!(
            glyph_height(figure),
            look_config("classic").sequence.height as f64
        );
    }

    #[test]
    fn draws_the_figure_smaller_on_neo_than_on_classic_at_the_glyph_band_size() {
        // Reference: actorSizing.spec.ts:118. The neo head is smaller than the classic one.
        //
        // The reference's second clause (the same box height on both looks) holds under its
        // jsdom mock only: in the real pipeline a neo participant's box is taller than a classic
        // one (see `reports_a_height_that_is_not_shrunk_by_the_look`), so it is not restated here.
        let classic = alone("actor", "classic");
        let neo = alone("actor", "neo");
        let classic_head =
            all(participant(&classic, "A"), &|node| node.name == "circle")[0].num("r");
        let neo_head = all(participant(&neo, "A"), &|node| node.name == "circle")[0].num("r");
        assert!(neo_head < classic_head);
    }

    #[test]
    fn fills_the_shared_glyph_band_on_neo() {
        // Reference: actorSizing.spec.ts:130. The neo figure spans the glyph band.
        let svg = alone("actor", "neo");
        let figure = participant(&svg, "A");
        assert!((glyph_height(figure) - GLYPH_BAND_HEIGHT).abs() < 1e-9);
    }

    #[test]
    fn anchors_the_label_at_the_datum_on_neo() {
        // Reference: actorSizing.spec.ts:139. The label's centre sits half its line height and the
        // clearance above the lifeline's datum.
        let svg = alone("actor", "neo");
        let datum = lifeline(&svg, "A").num("y1");
        assert_eq!(
            label_y(&svg),
            datum - LABEL_LIFELINE_GAP - line_height(16.0) / 2.0
        );
    }

    #[test]
    fn reports_a_height_that_is_not_shrunk_by_the_look() {
        // Reference: actorSizing.spec.ts:150. The reference's assertion is that the height the look
        // reports is the input height, 65 on both looks, under its mocked measurement. In the real
        // pipeline a neo participant's box is 76 high and a classic one 65: the look grows the box and
        // does not shrink it, which is what is restated here.
        let classic = alone("participant", "classic");
        let neo = alone("participant", "neo");
        assert!(box_height(participant(&neo, "A")) >= box_height(participant(&classic, "A")));
    }
}

/// `lifelineStart.spec.ts`: where a participant's lifeline begins. Each shape is drawn on its own,
/// as the reference draws it.
mod lifeline_start {
    use super::*;

    /// The lifeline's top, the bottom of the participant's box (`lifelineStart`).
    fn lifeline_top(kind: &str, look: &str) -> f64 {
        lifeline(&alone(kind, look), "A").num("y1")
    }

    /// The gap between the label of a participant and its lifeline.
    fn label_to_lifeline_gap(kind: &str, look: &str) -> f64 {
        let svg = alone(kind, look);
        let label = label_of(participant(&svg, "A"));
        let label_bottom = label.num("y") + line_height(16.0) / 2.0;
        lifeline(&svg, "A").num("y1") - label_bottom
    }

    #[test]
    fn starts_every_icon_and_label_below_shape_at_the_same_place_on_neo() {
        // Reference: lifelineStart.spec.ts:86. Shapes that hang their label below a glyph start their
        // lifelines at one height on neo.
        let tops: Vec<f64> = ["boundary", "control", "entity", "database", "actor"]
            .iter()
            .map(|kind| lifeline_top(kind, "neo"))
            .collect();
        assert_eq!(distinct(&tops), 1);
    }

    #[test]
    fn leaves_the_same_gap_between_label_and_lifeline_for_all_of_them() {
        // Reference: lifelineStart.spec.ts:98. Glyphs of different heights leave the same air under
        // their labels.
        let gaps: Vec<f64> = ["boundary", "control", "entity", "database", "actor"]
            .iter()
            .map(|kind| label_to_lifeline_gap(kind, "neo"))
            .collect();
        assert_eq!(distinct(&gaps), 1);
    }

    #[test]
    fn starts_at_the_box_bottom_for_shapes_whose_label_sits_inside_the_box() {
        // Reference: lifelineStart.spec.ts:109. A label centred in the box leaves nothing to clear, so
        // the lifeline starts where the box ends.
        for kind in ["participant", "collections"] {
            let svg = alone(kind, "neo");
            let group = participant(&svg, "A");
            assert_eq!(lifeline(&svg, "A").num("y1"), box_bottom(group), "{kind}");
        }
        // A queue draws no box of its own, so its box is the row's: its lifeline starts where a
        // participant's box ends in the same row.
        let row = render(
            "sequenceDiagram\nparticipant P\nparticipant Q@{ \"type\" : \"queue\" } as Q",
            &look_config("neo"),
        );
        assert_eq!(
            lifeline(&row, "Q").num("y1"),
            box_bottom(participant(&row, "P"))
        );
    }

    #[test]
    fn ends_the_round_icons_at_the_same_height_on_neo() {
        // Reference: lifelineStart.spec.ts:117. The round icons of the family end at one height.
        let bottoms: Vec<f64> = ["boundary", "control", "entity"]
            .iter()
            .map(|kind| {
                let svg = alone(kind, "neo");
                let circle = all(participant(&svg, "A"), &|node| node.name == "circle")[0];
                circle.num("cy") + circle.num("r")
            })
            .collect();
        assert_eq!(distinct(&bottoms), 1);
    }

    #[test]
    fn leaves_the_classic_icon_centres_alone() {
        // Reference: lifelineStart.spec.ts:131. The classic icon centres are where they were: 12, 32 and
        // 25 below the top of the participant.
        let centres: Vec<(&str, f64)> = ["boundary", "control", "entity"]
            .iter()
            .map(|kind| {
                let svg = alone(kind, "classic");
                let circle = all(participant(&svg, "A"), &|node| node.name == "circle")[0];
                (*kind, circle.num("cy"))
            })
            .collect();
        assert_eq!(
            centres,
            [("boundary", 12.0), ("control", 32.0), ("entity", 25.0)]
        );
    }

    #[test]
    fn leaves_every_classic_lifeline_exactly_where_it_was() {
        // Reference: lifelineStart.spec.ts:141.
        let classic: Vec<(&str, f64)> = [
            "participant",
            "queue",
            "collections",
            "boundary",
            "control",
            "entity",
            "database",
            "actor",
        ]
        .iter()
        .map(|kind| (*kind, lifeline_top(kind, "classic")))
        .collect();
        assert_eq!(
            classic,
            [
                ("participant", 65.0),
                ("queue", 65.0),
                ("collections", 65.0),
                ("boundary", 80.0),
                ("control", 75.0),
                ("entity", 75.0),
                ("database", 75.0),
                ("actor", 80.0),
            ]
        );
    }
}

/// `rectSectionFill.spec.ts`: the fill of a `rect` block when the author gives no colour. The
/// reference tests every theme; ferrigura ships two, so each shipped theme is one row.
mod sequence_rect_section_fill {
    use super::*;

    const SHIPPED_THEMES: [&str; 2] = ["redux-color", "redux-dark-color"];

    /// The fill a `rect` block resolves to: `rectBkgColor`, else `actorBkg`.
    fn resolved_rect_fill(variables: &Value) -> String {
        ["rectBkgColor", "actorBkg"]
            .iter()
            .find_map(|key| {
                variables
                    .get(key)
                    .and_then(Value::as_str)
                    .filter(|value| !value.is_empty())
            })
            .unwrap_or("rgba(128, 128, 128, 0.5)")
            .to_owned()
    }

    /// The fill that the drawn `rect` block carries.
    fn drawn_rect_fill(theme: &str, overrides: Option<&Value>) -> String {
        let variables = variables_of(theme, overrides);
        let svg = render_with(
            "sequenceDiagram\nAlice->Bob: hi\nrect\nBob->Alice: b\nend",
            &look_config("classic"),
            &variables,
        );
        background_rect(&svg)
            .attr("fill")
            .unwrap_or_default()
            .to_owned()
    }

    #[test]
    fn is_distinguishable_from_the_background_on_the_theme() {
        // Reference: rectSectionFill.spec.ts:24, one row per theme. The drawn fill is the resolved one,
        // and it is not the background.
        for theme in SHIPPED_THEMES {
            let variables = variables_of(theme, None);
            let fill = drawn_rect_fill(theme, None);
            assert_eq!(fill, resolved_rect_fill(&variables), "{theme}");
            let background = variables
                .get("background")
                .and_then(Value::as_str)
                .unwrap_or_default();
            assert_ne!(fill.to_lowercase(), background.to_lowercase(), "{theme}");
        }
    }

    #[test]
    fn stays_distinguishable_when_the_background_is_overridden_to_black_on_the_theme() {
        // Reference: rectSectionFill.spec.ts:30, one row per theme. The override is black; the fill
        // must not be.
        let overrides = Value::from(serde_json::json!({ "background": "#000000" }));
        for theme in SHIPPED_THEMES {
            let fill = drawn_rect_fill(theme, Some(&overrides));
            assert_ne!(fill.to_lowercase(), "#000000", "{theme}");
        }
    }

    #[test]
    fn is_still_overridable_through_theme_variables() {
        // Reference: rectSectionFill.spec.ts:44. The theme variable `rectBkgColor` is what the block
        // is filled with.
        let overrides = Value::from(serde_json::json!({ "rectBkgColor": "#abcdef" }));
        assert_eq!(drawn_rect_fill("redux-color", Some(&overrides)), "#abcdef");
    }
}

/// `noteFontWeight.spec.ts`: the stylesheet of the note text, and the theme variable that git reads.
mod sequence_note_font_weight {
    use super::*;

    #[test]
    fn does_not_force_a_weight_on_note_text_under_the_theme() {
        // Reference: noteFontWeight.spec.ts:28, one row per theme. The note text's rule sets no weight:
        // the configuration governs it.
        for theme in ["redux-color", "redux-dark-color"] {
            let svg = render_with(
                "sequenceDiagram\nAlice->Bob: hi\nNote right of Bob: a",
                &look_config("classic"),
                &variables_of(theme, None),
            );
            let css = all(&svg, &|node| node.name == "style")[0].text_content();
            let declarations = rule_declarations(&css, ".noteText");
            assert!(
                !declarations.contains("font-weight"),
                "{theme}: {declarations}"
            );
        }
    }

    #[test]
    fn leaves_the_theme_variable_alone_because_git_depends_on_it() {
        // Reference: noteFontWeight.spec.ts:36. The theme keeps `noteFontWeight` at 600, which git's
        // labels read.
        let variables = variables_of("redux-color", None);
        assert_eq!(
            variables.get("noteFontWeight").and_then(Value::as_f64),
            Some(600.0)
        );
    }
}

/// `palettePicking.spec.ts`: the actor colours are cycled through the theme's palettes.
mod palette_picking {
    use super::*;

    /// The actor colours of `count` participants, in order, on a classic diagram.
    fn colours(variables: &Value, count: usize) -> Vec<(Option<String>, Option<String>)> {
        let mut text = String::from("sequenceDiagram\n");
        for index in 0..count {
            text.push_str(&format!("participant P{index}\n"));
        }
        let svg = render_with(&text, &look_config("classic"), variables);
        (0..count)
            .map(|index| actor_colours(&svg, &format!("P{index}")))
            .collect()
    }

    /// The palette overrides of a theme variable pair.
    fn palettes(border: &[&str], fill: &[&str]) -> Value {
        Value::from(serde_json::json!({ "borderColorArray": border, "bkgColorArray": fill }))
    }

    /// Each actor is stroked with the border palette and filled with the background palette, both
    /// cycled by their own length (`paletteColor`).
    fn assert_cycles(variables: &Value, border: &[String], fill: &[String], count: usize) {
        for (index, (stroke, background)) in colours(variables, count).iter().enumerate() {
            assert_eq!(
                stroke.as_deref(),
                Some(border[index % border.len()].as_str()),
                "stroke of P{index}"
            );
            let expected = (!fill.is_empty()).then(|| fill[index % fill.len()].clone());
            assert_eq!(background, &expected, "fill of P{index}");
        }
    }

    #[test]
    fn cycles_within_the_palette_it_was_given() {
        // Reference: palettePicking.spec.ts:26. Six actors over a three-colour palette repeat it.
        let variables = variables_of(
            "redux-color",
            Some(&palettes(&["#a", "#b", "#c"], &["#a", "#b", "#c"])),
        );
        let strokes: Vec<Option<String>> = colours(&variables, 6)
            .into_iter()
            .map(|(stroke, _)| stroke)
            .collect();
        let expected: Vec<Option<String>> = ["#a", "#b", "#c", "#a", "#b", "#c"]
            .iter()
            .map(|colour| Some((*colour).to_owned()))
            .collect();
        assert_eq!(strokes, expected);
    }

    #[test]
    fn returns_undefined_for_an_empty_palette_rather_than_a_substitute_colour() {
        // Reference: palettePicking.spec.ts:37. An empty background palette gives no fill, so the
        // stylesheet decides; the border still colours the actors.
        let variables = variables_of("redux-color", Some(&palettes(&["#a", "#b", "#c"], &[])));
        for (stroke, fill) in colours(&variables, 3) {
            assert!(stroke.is_some());
            assert_eq!(fill, None);
        }
    }

    #[test]
    fn never_runs_off_the_end_of_a_short_palette() {
        // Reference: palettePicking.spec.ts:43. Twelve actors over a three-colour palette all have a
        // colour, and only the palette's colours are used.
        let variables = variables_of(
            "redux-color",
            Some(&palettes(&["#a", "#b", "#c"], &["#a", "#b", "#c"])),
        );
        let all_colours = colours(&variables, 12);
        assert!(
            all_colours
                .iter()
                .all(|(stroke, fill)| stroke.is_some() && fill.is_some())
        );
        let used: std::collections::BTreeSet<String> = all_colours
            .iter()
            .filter_map(|(stroke, _)| stroke.clone())
            .collect();
        assert_eq!(
            used,
            ["#a", "#b", "#c"]
                .iter()
                .map(|colour| (*colour).to_owned())
                .collect()
        );
    }

    #[test]
    fn is_independent_of_any_other_palette_length() {
        // Reference: palettePicking.spec.ts:50. A two-colour background palette does not cut the
        // border palette short: all twelve actors are filled, cycling the two colours.
        let border = palette(&variables_of("redux-color", None), "borderColorArray");
        let variables = variables_of(
            "redux-color",
            Some(&Value::from(
                serde_json::json!({ "bkgColorArray": ["#x", "#y"] }),
            )),
        );
        let fill: Vec<String> = vec!["#x".to_owned(), "#y".to_owned()];
        assert_cycles(&variables, &border, &fill, 12);
    }

    #[test]
    fn redux_color() {
        // Reference: palettePicking.spec.ts:66, row `redux-color`.
        let variables = variables_of("redux-color", None);
        let border = palette(&variables, "borderColorArray");
        let fill = palette(&variables, "bkgColorArray");
        assert_cycles(&variables, &border, &fill, 24);
    }

    #[test]
    fn redux_dark_color() {
        // Reference: palettePicking.spec.ts:66, row `redux-dark-color`.
        let variables = variables_of("redux-dark-color", None);
        let border = palette(&variables, "borderColorArray");
        let fill = palette(&variables, "bkgColorArray");
        assert_cycles(&variables, &border, &fill, 24);
    }

    #[test]
    fn redux_dark_color_still_yields_no_fill_so_the_stylesheet_keeps_deciding() {
        // Reference: palettePicking.spec.ts:82. The dark theme ships no background palette, so no
        // actor has a fill of its own.
        let variables = variables_of("redux-dark-color", None);
        assert!(palette(&variables, "bkgColorArray").is_empty());
        assert!(
            colours(&variables, 3)
                .iter()
                .all(|(_, fill)| fill.is_none())
        );
    }

    #[test]
    #[ignore = "reads the reference's source text; no behaviour to port"]
    fn has_no_raw_palette_indexing_left() {
        // Reference: palettePicking.spec.ts:101. The reference's check reads its own `svgDraw.js`.
        // The Rust analogue reads the sequence sources: a palette is only ever indexed by its own
        // length, `palette[... % palette.len()]`, never by the length of another palette.
        let sources = [
            include_str!("../src/sequence/actors.rs"),
            include_str!("../src/sequence/blocks.rs"),
            include_str!("../src/sequence/messages.rs"),
        ];
        for source in sources {
            for line in source.lines().filter(|line| line.contains("palette[")) {
                assert!(line.contains("% palette.len()]"), "{line}");
            }
        }
    }

    #[test]
    #[ignore = "reads the reference's source text; no behaviour to port"]
    fn routes_every_actor_colour_through_palette_color_each_to_its_own_property() {
        // Reference: palettePicking.spec.ts:108. The reference's check reads its own `svgDraw.js`.
        // The Rust analogue reads `actors.rs`: the stroke takes the border palette and the fill the
        // background palette, and the two are balanced.
        let source = include_str!("../src/sequence/actors.rs");
        let strokes: Vec<&str> = source
            .lines()
            .filter(|line| line.contains("(\"stroke\", palette_color("))
            .collect();
        let fills: Vec<&str> = source
            .lines()
            .filter(|line| line.contains("(\"fill\", palette_color("))
            .collect();
        assert!(
            strokes
                .iter()
                .all(|line| line.contains("palette_color(&border,"))
        );
        assert!(
            fills
                .iter()
                .all(|line| line.contains("palette_color(&bkg,"))
        );
        assert_eq!(strokes.len(), fills.len());
        assert!(!strokes.is_empty());
    }
}

/// The port's own check of the actor menu, which no reference spec reads: `drawPopup` lists the
/// links with `for…in` (`svgDraw.js:113-128`), in the key order of the actor's links object. The
/// array indices come first, ascending, then the other keys in insertion order; a repeated key
/// keeps its place and takes the new value.
mod actor_menu {
    use super::*;

    /// The label, the target and the text's `y` of each item of the only menu, in document order.
    fn menu_items(svg: &Node) -> Vec<(String, String, f64)> {
        let menus = all(svg, &|node| node.has_class("actorPopupMenu"));
        assert_eq!(menus.len(), 1, "one actor has links");
        menus[0]
            .children
            .iter()
            .filter(|child| child.name == "a")
            .map(|item| {
                let text = all(item, &|node| node.name == "text")[0];
                (
                    item.text_content(),
                    item.attr("xlink:href").unwrap_or_default().to_owned(),
                    text.num("y"),
                )
            })
            .collect()
    }

    #[test]
    fn lists_the_links_in_the_key_order_of_a_javascript_object() {
        let svg = render(
            "sequenceDiagram\nparticipant A\n\
             links A: {\"Zulu\": \"https://menu.test/z\", \"10\": \"https://menu.test/10\", \"Alpha\": \"https://menu.test/a\", \"2\": \"https://menu.test/2\"}\n\
             link A: Zulu @ https://menu.test/z2\nlink A: Mike @ https://menu.test/m\nA->>B: x",
            &render_config(),
        );
        let items = menu_items(&svg);
        let entries: Vec<(&str, &str)> = items
            .iter()
            .map(|(label, target, _)| (label.as_str(), target.as_str()))
            .collect();
        assert_eq!(
            entries,
            [
                ("2", "https://menu.test/2"),
                ("10", "https://menu.test/10"),
                ("Zulu", "https://menu.test/z2"),
                ("Alpha", "https://menu.test/a"),
                ("Mike", "https://menu.test/m"),
            ]
        );
        // Each item is drawn 30 px below the one before, so the document order is the menu's order.
        let steps: Vec<f64> = items.windows(2).map(|pair| pair[1].2 - pair[0].2).collect();
        assert_eq!(steps, [30.0; 4]);
    }
}
