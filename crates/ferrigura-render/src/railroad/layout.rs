//! The layout and the elements of a railroad diagram: the renderer class of
//! `packages/mermaid/src/diagrams/railroad/railroadRenderer.ts` (`RailroadRenderer`, lines 61-623).
//!
//! Every number is a measured text (the [`Measure`] of the text), a configured value, or one of the
//! constants of the reference (the rounded corner 10, the gap of 10 after a rule's end, the 20 of the
//! rule name). An expression is laid out from its parts: [`Dimensions`] gives its size and the parts
//! above and below its centre line, and each form places its children from them. The elements are
//! built in the order the reference appends them to its groups, so the markup has the reference's
//! order (`railroadRenderer.ts`, section 6.3 of the research).
//!
//! The text the layout measures is the database's; the text it draws is [`super::text::shown_text`].
//! The layout does not depend on the fonts that are drawn, so a test can give it the reference's
//! stub measurement (`railroadRenderer.spec.ts`, which sets `getBBox` to the text length times 8).

use std::fmt::Write as _;

use ferrigura_model::railroad::{RailroadExpr, RailroadRule};
use ferrigura_text::TextError;

use super::style::Colours;
use super::text::shown_text;
use crate::common::Element;
use crate::svg::number;

/// The width and height of a text, as `getBBox` gives them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextSize {
    /// The width of the text's box.
    pub width: f64,
    /// The height of the text's box.
    pub height: f64,
}

/// The measurement of a text (`measureText`, `railroadRenderer.ts` 74-93): the box of the text in
/// the configured font. Errors are the bundled font's.
pub trait Measure {
    /// The box of `text`.
    fn text_size(&mut self, text: &str) -> Result<TextSize, TextError>;
}

/// The numbers the layout reads from the configuration (`railroadRenderer.ts` reads them as
/// `this.config`, after the sanitising of `buildRailroadStyleOptions`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spacing {
    /// The space around each text and each rule (`padding`).
    pub padding: f64,
    /// The gap between alternatives and between rules (`verticalSeparation`).
    pub vertical_separation: f64,
    /// The gap between the items of a sequence (`horizontalSeparation`).
    pub horizontal_separation: f64,
    /// The radius of the arcs of choices, optionals and repetitions (`arcRadius`).
    pub arc_radius: f64,
    /// The radius of the start and the end markers (`markerRadius`).
    pub marker_radius: f64,
}

/// The size of an expression and the parts above and below its centre line
/// (`RenderResult.dimensions`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dimensions {
    /// The width.
    pub width: f64,
    /// The height.
    pub height: f64,
    /// The distance from the top of the expression to its centre line.
    pub up: f64,
    /// The distance from the centre line to the bottom of the expression.
    pub down: f64,
}

/// The characters a drawing shows, by weight, for the fonts that embed them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DrawnText {
    /// The text of the regular weight: the terminals, the non-terminals and the specials.
    pub regular: String,
    /// The text of the bold weight: the rule names.
    pub bold: String,
}

/// The drawing of a diagram: its rule groups, in order, the size of the diagram and the characters
/// it shows.
#[derive(Debug, Clone, PartialEq)]
pub struct Drawing {
    /// One `g.railroad-rule` for each rule, in the order of the rules.
    pub rules: Vec<Element>,
    /// The width of the diagram (`renderDiagram`, `railroadRenderer.ts` 620).
    pub width: f64,
    /// The height of the diagram (`railroadRenderer.ts` 621).
    pub height: f64,
    /// The characters the diagram shows.
    pub text: DrawnText,
}

/// Lays out the rules of a diagram (`renderDiagram`, `railroadRenderer.ts` 609-623). The diagram must
/// have at least one rule: an empty diagram has its own size, which the caller gives.
///
/// # Errors
///
/// Returns the error of the measurement.
pub fn draw_rules<M: Measure>(
    rules: &[RailroadRule],
    spacing: &Spacing,
    colours: &Colours,
    measure: &mut M,
) -> Result<Drawing, TextError> {
    let mut layout = Layout {
        measure,
        spacing,
        colours,
        text: DrawnText::default(),
    };
    let mut elements = Vec::with_capacity(rules.len());
    let mut y = spacing.padding;
    let mut max_width = 0.0_f64;
    for rule in rules {
        let (element, size) = layout.rule(rule, y)?;
        elements.push(element);
        y += size.height + spacing.vertical_separation;
        max_width = max_width.max(size.width);
    }
    Ok(Drawing {
        rules: elements,
        width: max_width + spacing.padding * 2.0,
        height: y + spacing.padding,
        text: layout.text,
    })
}

/// The state of one layout: the measurement, the spacing, the colours the elements carry, and the
/// characters drawn so far.
struct Layout<'a, M> {
    measure: &'a mut M,
    spacing: &'a Spacing,
    colours: &'a Colours,
    text: DrawnText,
}

/// The shape of a box element: the rounded corners of a terminal (`rx`, `ry`, `railroadRenderer.ts`
/// 112-113) or none, and the colours of its rectangle and of its text.
#[derive(Clone, Copy)]
struct BoxStyle<'c> {
    class: &'static str,
    rounded: bool,
    fill: &'c str,
    stroke: &'c str,
    text_fill: &'c str,
}

impl<M: Measure> Layout<'_, M> {
    fn expression(&mut self, node: &RailroadExpr) -> Result<(Element, Dimensions), TextError> {
        match node {
            RailroadExpr::Terminal { value } => self.terminal(value),
            RailroadExpr::Nonterminal { name } => self.nonterminal(name),
            RailroadExpr::Special { text } => self.special(text),
            RailroadExpr::Sequence { elements } => self.sequence(elements),
            RailroadExpr::Choice { alternatives } => self.choice(alternatives),
            RailroadExpr::Optional { element } => self.optional(element),
            RailroadExpr::Repetition { element, min, .. } => self.repetition(element, *min),
        }
    }

    /// `renderTerminal` (`railroadRenderer.ts` 98-131).
    fn terminal(&mut self, value: &str) -> Result<(Element, Dimensions), TextError> {
        let colours = self.colours;
        let style = BoxStyle {
            class: "railroad-terminal",
            rounded: true,
            fill: &colours.terminal_fill,
            stroke: &colours.terminal_stroke,
            text_fill: &colours.terminal_text,
        };
        self.labelled_box(style, value, value)
    }

    /// `renderNonTerminal` (`railroadRenderer.ts` 136-162).
    fn nonterminal(&mut self, name: &str) -> Result<(Element, Dimensions), TextError> {
        let colours = self.colours;
        let style = BoxStyle {
            class: "railroad-nonterminal",
            rounded: false,
            fill: &colours.non_terminal_fill,
            stroke: &colours.non_terminal_stroke,
            text_fill: &colours.non_terminal_text,
        };
        self.labelled_box(style, name, name)
    }

    /// `renderSpecial` (`railroadRenderer.ts` 473-499): the text is `? text ?`, measured and drawn.
    fn special(&mut self, text: &str) -> Result<(Element, Dimensions), TextError> {
        let colours = self.colours;
        let style = BoxStyle {
            class: "railroad-special",
            rounded: false,
            fill: &colours.special_fill,
            stroke: &colours.special_stroke,
            text_fill: &colours.non_terminal_text,
        };
        let shown = format!("? {text} ?");
        self.labelled_box(style, &shown, &shown)
    }

    /// The group of a box with its text: the box is the text plus `padding` on each side, and the
    /// text sits at its centre. Terminals, non-terminals and specials share the arithmetic.
    fn labelled_box(
        &mut self,
        style: BoxStyle<'_>,
        measured: &str,
        drawn: &str,
    ) -> Result<(Element, Dimensions), TextError> {
        let size = self.measure.text_size(measured)?;
        let padding = self.spacing.padding;
        let width = size.width + padding * 2.0;
        let height = size.height + padding * 2.0;

        let mut rect = Element::new("rect")
            .with("x", "0")
            .with("y", "0")
            .with("width", number(width))
            .with("height", number(height));
        if style.rounded {
            rect.set("rx", "10");
            rect.set("ry", "10");
        }
        rect.set("fill", style.fill);
        rect.set("stroke", style.stroke);

        let shown = shown_text(drawn);
        self.text.regular.push_str(&shown);
        let text = Element::new("text")
            .with("x", number(width / 2.0))
            .with("y", number(height / 2.0))
            .with("fill", style.text_fill)
            .text(shown);

        let element = Element::new("g")
            .with("class", style.class)
            .child(rect)
            .child(text);
        Ok((
            element,
            Dimensions {
                width,
                height,
                up: height / 2.0,
                down: height / 2.0,
            },
        ))
    }

    /// `renderSequence` (`railroadRenderer.ts` 167-217): the items side by side, on their common
    /// centre line, with a line of `horizontalSeparation` after each but the last.
    fn sequence(&mut self, elements: &[RailroadExpr]) -> Result<(Element, Dimensions), TextError> {
        let mut rendered = Vec::with_capacity(elements.len());
        for element in elements {
            rendered.push(self.expression(element)?);
        }
        let spacing = self.spacing.horizontal_separation;
        let mut total_width = 0.0_f64;
        let mut max_up = 0.0_f64;
        let mut max_down = 0.0_f64;
        for (_, size) in &rendered {
            total_width += size.width;
            max_up = max_up.max(size.up);
            max_down = max_down.max(size.down);
        }
        // `(rendered.length - 1) * horizontalSeparation`, which is negative for no item (`-1`).
        total_width += (rendered.len() as f64 - 1.0) * spacing;

        let count = rendered.len();
        let mut group = Element::new("g").with("class", "railroad-sequence");
        let mut x = 0.0_f64;
        for (index, (mut element, size)) in rendered.into_iter().enumerate() {
            set_translate(&mut element, x, max_up - size.up);
            group.push_child(element);
            if index + 1 < count {
                let line_x1 = x + size.width;
                let line_x2 = line_x1 + spacing;
                group.push_child(
                    self.line(
                        Path::default()
                            .move_to(line_x1, max_up)
                            .line_to(line_x2, max_up),
                    ),
                );
            }
            x += size.width + spacing;
        }
        Ok((
            group,
            Dimensions {
                width: total_width,
                height: max_up + max_down,
                up: max_up,
                down: max_down,
            },
        ))
    }

    /// `renderChoice` (`railroadRenderer.ts` 222-313): the alternatives one under the other, each
    /// joined to the centre line by an arc on the left and one on the right. An alternative on the
    /// centre line is joined by a straight line.
    fn choice(
        &mut self,
        alternatives: &[RailroadExpr],
    ) -> Result<(Element, Dimensions), TextError> {
        let mut rendered = Vec::with_capacity(alternatives.len());
        for alternative in alternatives {
            rendered.push(self.expression(alternative)?);
        }
        let vertical = self.spacing.vertical_separation;
        let radius = self.spacing.arc_radius;
        let mut max_width = 0.0_f64;
        let mut total_height = 0.0_f64;
        for (_, size) in &rendered {
            max_width = max_width.max(size.width);
            total_height += size.height;
        }
        total_height += (rendered.len() as f64 - 1.0) * vertical;

        let total_width = max_width + radius * 4.0;
        let centre_y = total_height / 2.0;
        let mut group = Element::new("g").with("class", "railroad-choice");
        let mut y = 0.0_f64;
        for (mut element, size) in rendered {
            let elem_y = y;
            let elem_centre_y = elem_y + size.up;
            let elem_x = radius * 2.0 + (max_width - size.width) / 2.0;
            set_translate(&mut element, elem_x, elem_y);
            group.push_child(element);
            let below = elem_centre_y > centre_y;

            // The arc from the centre line into the alternative (`railroadRenderer.ts` 245-266).
            let left = if elem_centre_y == centre_y {
                Path::default()
                    .move_to(0.0, centre_y)
                    .line_to(elem_x, elem_centre_y)
            } else {
                let step = if below { radius } else { -radius };
                Path::default()
                    .move_to(0.0, centre_y)
                    .arc_to(radius, below, radius, centre_y + step)
                    .line_to(radius, elem_centre_y - step)
                    .arc_to(radius, !below, radius * 2.0, elem_centre_y)
                    .line_to(elem_x, elem_centre_y)
            };
            group.push_child(self.line(left));

            // The arc from the alternative back to the centre line, on the right lane
            // (`railroadRenderer.ts` 268-299).
            let right_start = elem_x + size.width;
            let right_lane_x = total_width - radius * 2.0;
            let right = if elem_centre_y == centre_y {
                Path::default()
                    .move_to(right_start, elem_centre_y)
                    .line_to(total_width, centre_y)
            } else {
                let step = if below { -radius } else { radius };
                let lane_step = if below { radius } else { -radius };
                Path::default()
                    .move_to(right_start, elem_centre_y)
                    .line_to(right_lane_x, elem_centre_y)
                    .arc_to(radius, !below, total_width - radius, elem_centre_y + step)
                    .line_to(total_width - radius, centre_y + lane_step)
                    .arc_to(radius, below, total_width, centre_y)
            };
            group.push_child(self.line(right));

            y += size.height + vertical;
        }
        Ok((
            group,
            Dimensions {
                width: total_width,
                height: total_height,
                up: centre_y,
                down: total_height - centre_y,
            },
        ))
    }

    /// `renderOptional` (`railroadRenderer.ts` 318-366): the item on the centre line, and a bypass
    /// arc above it.
    fn optional(&mut self, element: &RailroadExpr) -> Result<(Element, Dimensions), TextError> {
        let (mut inner, inner_size) = self.expression(element)?;
        let radius = self.spacing.arc_radius;
        let arc_height = radius * 2.0;
        let total_width = inner_size.width + radius * 4.0;
        let total_height = inner_size.height + arc_height;

        let mut group = Element::new("g").with("class", "railroad-optional");
        let elem_x = radius * 2.0;
        let elem_y = arc_height;
        set_translate(&mut inner, elem_x, elem_y);
        group.push_child(inner);
        let centre_y = elem_y + inner_size.up;

        group.push_child(
            self.line(
                Path::default()
                    .move_to(0.0, centre_y)
                    .line_to(radius * 2.0, centre_y),
            ),
        );
        group.push_child(
            self.line(
                Path::default()
                    .move_to(elem_x + inner_size.width, centre_y)
                    .line_to(total_width, centre_y),
            ),
        );
        group.push_child(self.line(bypass(radius, centre_y, total_width)));

        Ok((
            group,
            Dimensions {
                width: total_width,
                height: total_height,
                up: centre_y,
                down: total_height - centre_y,
            },
        ))
    }

    /// `renderRepetition` (`railroadRenderer.ts` 371-468). Only the minimum is drawn: a minimum of 0
    /// adds the bypass above, and any other minimum does not, whatever the maximum is (the reference
    /// reads `node.min` alone, `railroadRenderer.ts` 521-522 and 378).
    fn repetition(
        &mut self,
        element: &RailroadExpr,
        min: f64,
    ) -> Result<(Element, Dimensions), TextError> {
        let (mut inner, inner_size) = self.expression(element)?;
        let radius = self.spacing.arc_radius;
        let arc_height = radius * 2.0;
        let total_width = inner_size.width + radius * 4.0;
        let has_bypass = min == 0.0;
        let total_height =
            inner_size.height + arc_height + if has_bypass { arc_height } else { 0.0 };

        let mut group = Element::new("g").with("class", "railroad-repetition");
        let elem_x = radius * 2.0;
        let elem_y = if has_bypass { arc_height } else { 0.0 };
        set_translate(&mut inner, elem_x, elem_y);
        group.push_child(inner);
        let centre_y = elem_y + inner_size.up;
        let inner_end = elem_x + inner_size.width;

        group.push_child(
            self.line(
                Path::default()
                    .move_to(0.0, centre_y)
                    .line_to(radius * 2.0, centre_y),
            ),
        );
        group.push_child(
            self.line(
                Path::default()
                    .move_to(inner_end, centre_y)
                    .line_to(total_width, centre_y),
            ),
        );

        // The loop below the item, back to its start (`railroadRenderer.ts` 415-442).
        let loop_y = elem_y + inner_size.height + radius;
        group.push_child(
            self.line(
                Path::default()
                    .move_to(inner_end, centre_y)
                    .arc_to(radius, true, inner_end + radius, centre_y + radius)
                    .line_to(inner_end + radius, loop_y)
                    .arc_to(radius, true, inner_end, loop_y + radius)
                    .line_to(radius * 2.0, loop_y + radius)
                    .arc_to(radius, true, radius, loop_y)
                    .line_to(radius, centre_y + radius)
                    .arc_to(radius, true, radius * 2.0, centre_y),
            ),
        );

        if has_bypass {
            group.push_child(self.line(bypass(radius, centre_y, total_width)));
        }

        Ok((
            group,
            Dimensions {
                width: total_width,
                height: total_height,
                up: centre_y,
                down: total_height - centre_y,
            },
        ))
    }

    /// `renderRule` (`railroadRenderer.ts` 535-604): the rule's name, its definition, the start and
    /// end markers and the two lines that join them to the definition. Returns the rule's group and
    /// the rule's size (`height` and `width`).
    fn rule(&mut self, rule: &RailroadRule, y: f64) -> Result<(Element, Size), TextError> {
        let padding = self.spacing.padding;
        let marker = self.spacing.marker_radius;
        let rule_name = format!("{} =", rule.name);
        let name_width = self.measure.text_size(&rule_name)?.width + 20.0;
        let definition_x = name_width + 20.0;

        let (definition, result) = self.expression(&rule.definition)?;
        let baseline_y = result.up.max(20.0);
        let definition_y = baseline_y - result.up;
        let mut definition_group = Element::new("g");
        set_translate(&mut definition_group, definition_x, definition_y);
        definition_group.push_child(definition);

        let shown = shown_text(&rule_name);
        self.text.bold.push_str(&shown);
        let name_group = Element::new("g")
            .with("class", "railroad-rule-name-group")
            .child(
                Element::new("text")
                    .with("class", "railroad-rule-name")
                    .with("x", "0")
                    .with("y", number(baseline_y))
                    .with("fill", self.colours.rule_name.as_str())
                    .text(shown),
            );

        let start = Element::new("g")
            .with("class", "railroad-start")
            .child(self.marker(name_width, baseline_y));
        let end_x = definition_x + result.width + 10.0;
        let end = Element::new("g")
            .with("class", "railroad-end")
            .child(self.marker(end_x, baseline_y));

        let start_line = self.line(
            Path::default()
                .move_to(name_width + marker, baseline_y)
                .line_to(definition_x, baseline_y),
        );
        let end_line = self.line(
            Path::default()
                .move_to(definition_x + result.width, baseline_y)
                .line_to(definition_x + result.width + 10.0 - marker, baseline_y),
        );

        let mut group = Element::new("g").with("class", "railroad-rule");
        set_translate(&mut group, 0.0, y);
        group.push_child(definition_group);
        group.push_child(name_group);
        group.push_child(start);
        group.push_child(end);
        group.push_child(start_line);
        group.push_child(end_line);

        Ok((
            group,
            Size {
                height: (definition_y + result.height + padding * 2.0).max(40.0),
                width: definition_x + result.width + 10.0 + marker,
            },
        ))
    }

    /// A marker: a circle of the marker radius at `(x, y)`, with the marker's fill.
    fn marker(&self, x: f64, y: f64) -> Element {
        Element::new("circle")
            .with("cx", number(x))
            .with("cy", number(y))
            .with("r", number(self.spacing.marker_radius))
            .with("fill", self.colours.marker_fill.as_str())
    }

    /// A line of the diagram: a path of class `railroad-line` with the line colour.
    fn line(&self, path: Path) -> Element {
        Element::new("path")
            .with("class", "railroad-line")
            .with("d", path.build())
            .with("stroke", self.colours.line.as_str())
    }
}

/// The size of a rule (`renderRule`'s `{ height, width }`).
struct Size {
    height: f64,
    width: f64,
}

/// The bypass arc above an element: from the centre line on the left, over the top, and back down
/// on the right (`railroadRenderer.ts` 345-355 and 445-457, the same path in both).
fn bypass(radius: f64, centre_y: f64, total_width: f64) -> Path {
    Path::default()
        .move_to(0.0, centre_y)
        .arc_to(radius, false, radius, centre_y - radius)
        .line_to(radius, radius)
        .arc_to(radius, true, radius * 2.0, 0.0)
        .line_to(total_width - radius * 2.0, 0.0)
        .arc_to(radius, true, total_width - radius, radius)
        .line_to(total_width - radius, centre_y - radius)
        .arc_to(radius, false, total_width, centre_y)
}

/// Sets the `transform` of an element that is placed at `(x, y)`, after its class, as the
/// reference's `setAttribute('transform', 'translate(x, y)')` does.
fn set_translate(element: &mut Element, x: f64, y: f64) {
    element.set(
        "transform",
        format!("translate({}, {})", number(x), number(y)),
    );
}

/// The `d` of a path, written as the reference's `PathBuilder` writes it (`railroadRenderer.ts`
/// 17-56): each command and its numbers, separated by spaces, with the trailing space trimmed.
#[derive(Default)]
struct Path(String);

impl Path {
    fn move_to(mut self, x: f64, y: f64) -> Self {
        let _ = write!(self.0, "M {} {} ", number(x), number(y));
        self
    }

    fn line_to(mut self, x: f64, y: f64) -> Self {
        let _ = write!(self.0, "L {} {} ", number(x), number(y));
        self
    }

    /// An arc of `radius` to `(x, y)`: a quarter circle of the renderer, with no rotation and the
    /// small arc, as every arc of `railroadRenderer.ts` is drawn.
    fn arc_to(mut self, radius: f64, sweep: bool, x: f64, y: f64) -> Self {
        let _ = write!(
            self.0,
            "A {} {} 0 0 {} {} {} ",
            number(radius),
            number(radius),
            u8::from(sweep),
            number(x),
            number(y)
        );
        self
    }

    fn build(self) -> String {
        self.0.trim().to_owned()
    }
}
