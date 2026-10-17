//! The actors of a journey and their legend: a circle and the name of each actor, wrapped to
//! `maxLabelWidth` (`journeyRenderer.ts` 19-116, the wrap 38-92).
//!
//! The reference keeps the actors in a JavaScript object, so their order in the legend is the order
//! of `Object.keys`: the names that are array indices first, in ascending order, then the others in
//! the order they were added. The name `__proto__` cannot be an own key of that object, so it is
//! never in the legend, while its colour and position still reach its circles on the tasks
//! (`journeyRenderer.ts` 25, 153, 330).

use std::collections::HashMap;

use ferrigura_model::config::JourneyDiagramConfig;

use super::text::svg_width;
use crate::RenderError;
use crate::common::Element;
use crate::common::draw::{TextData, draw_text};
use crate::common::text::split_line_breaks;
use crate::svg::number;

/// The name the reference cannot keep as an own key of its actors object.
const PROTO_NAME: &str = "__proto__";

/// One actor: its colour, and its position in the order of `getActors`, which numbers the
/// `actor-N` class of its circles.
#[derive(Debug, Clone, PartialEq)]
pub struct Actor {
    /// The name, as the task gives it.
    pub name: String,
    /// The colour, which cycles through `actorColours`.
    pub colour: String,
    /// The position in the order of `getActors`.
    pub position: usize,
}

/// The actors of a journey, in the order of `getActors`, with the order of the legend.
#[derive(Debug, Clone, PartialEq)]
pub struct Actors {
    actors: Vec<Actor>,
    by_name: HashMap<String, usize>,
    /// The indices of the legend's entries, in the order of `Object.keys`.
    legend: Vec<usize>,
}

impl Actors {
    /// The actors with their names in the given order, coloured from `colours` by position.
    #[must_use]
    pub fn new(names: &[String], colours: &[String]) -> Self {
        let actors: Vec<Actor> = names
            .iter()
            .enumerate()
            .map(|(position, name)| Actor {
                name: name.clone(),
                colour: if colours.is_empty() {
                    String::new()
                } else {
                    colours[position % colours.len()].clone()
                },
                position,
            })
            .collect();
        let by_name = actors
            .iter()
            .enumerate()
            .map(|(index, actor)| (actor.name.clone(), index))
            .collect();
        let mut indices: Vec<(u32, usize)> = Vec::new();
        let mut others = Vec::new();
        for (index, actor) in actors.iter().enumerate() {
            if actor.name == PROTO_NAME {
                continue;
            }
            match array_index(&actor.name) {
                Some(number) => indices.push((number, index)),
                None => others.push(index),
            }
        }
        indices.sort_unstable();
        let legend = indices
            .into_iter()
            .map(|(_, index)| index)
            .chain(others)
            .collect();
        Self {
            actors,
            by_name,
            legend,
        }
    }

    /// The actor of a name, if the diagram has one.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Actor> {
        self.by_name.get(name).map(|&index| &self.actors[index])
    }

    /// The entries of the legend, in the order they are drawn.
    pub fn legend(&self) -> impl Iterator<Item = &Actor> + '_ {
        self.legend.iter().map(|&index| &self.actors[index])
    }

    /// The number of entries in the legend. The reference's bounds take 50 px for each of them.
    #[must_use]
    pub fn legend_len(&self) -> usize {
        self.legend.len()
    }
}

/// The value of a canonical array index (`Object.keys` lists these first): a decimal number
/// without a sign or a leading zero, below 2^32 - 1.
fn array_index(name: &str) -> Option<u32> {
    if name.is_empty()
        || !name.bytes().all(|byte| byte.is_ascii_digit())
        || (name.len() > 1 && name.starts_with('0'))
    {
        return None;
    }
    let value: u64 = name.parse().ok()?;
    (value < u64::from(u32::MAX)).then_some(value as u32)
}

/// The legend and the width it adds to the left margin.
#[derive(Debug, Clone, PartialEq)]
pub struct Legend {
    /// The circles and texts of the legend, in order.
    pub elements: Vec<Element>,
    /// The widest legend line that is wider than half the minimum margin, as `maxWidth` of the
    /// reference (`journeyRenderer.ts` 109-111).
    pub max_width: f64,
}

/// Draws the legend: each actor's circle at x 20, then its name in lines at x 40, the next entry
/// below the taller of the two (`yPos`, at least 20 px per entry).
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub fn draw(
    actors: &Actors,
    config: &JourneyDiagramConfig,
    size: f64,
    texts: &mut Vec<String>,
) -> Result<Legend, RenderError> {
    let max_label_width = config.max_label_width as f64;
    let left_margin = config.left_margin as f64;
    let box_text_margin = config.box_text_margin as f64;
    let mut elements = Vec::new();
    let mut max_width: f64 = 0.0;
    let mut y_position = 60.0;
    for actor in actors.legend() {
        elements.push(actor_circle(
            20.0,
            y_position,
            7.0,
            &actor.colour,
            actor.position,
            None,
        ));
        let lines = wrap(&actor.name, max_label_width, size)?;
        for (index, line) in lines.iter().enumerate() {
            let shown = split_line_breaks(line).join(" ");
            let line_width = svg_width(&shown, size)?;
            texts.push(shown);
            elements.push(draw_text(&TextData {
                x: 40.0,
                y: y_position + 7.0 + index as f64 * 20.0,
                text: line.clone(),
                text_margin: box_text_margin,
                ..TextData::default()
            }));
            // `conf.leftMargin` is the spacing baseline; the width grows only past it.
            if line_width > max_width && line_width > left_margin - line_width {
                max_width = line_width;
            }
        }
        y_position += f64::max(20.0, lines.len() as f64 * 20.0);
    }
    Ok(Legend {
        elements,
        max_width,
    })
}

/// The circle of an actor, as `drawCircle` writes it: its position, its colour and the stroke.
/// A circle on a task carries the actor's name as its `title`.
#[must_use]
pub fn actor_circle(
    cx: f64,
    cy: f64,
    radius: f64,
    colour: &str,
    position: usize,
    title: Option<&str>,
) -> Element {
    let mut circle = Element::new("circle")
        .with("cx", number(cx))
        .with("cy", number(cy))
        .with("class", format!("actor-{position}"))
        .with("fill", colour)
        .with("stroke", "#000")
        .with("r", number(radius));
    if let Some(title) = title {
        circle = circle.child(Element::new("title").text(title));
    }
    circle
}

/// `wrapLegend` (`journeyRenderer.ts` 45-92): the name on one line when it is no wider than
/// `max_width`; otherwise its words are packed into lines, and a word that is too wide on its own
/// is broken into pieces with a hyphen. The reference's own quirk is kept: a character that is
/// too wide with its hyphen gives a line of the hyphen alone.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub fn wrap(name: &str, max_width: f64, size: f64) -> Result<Vec<String>, RenderError> {
    if svg_width(name, size)? <= max_width {
        return Ok(vec![name.to_owned()]);
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in name.split(' ') {
        let test = if current.is_empty() {
            word.to_owned()
        } else {
            format!("{current} {word}")
        };
        if svg_width(&test, size)? <= max_width {
            current = test;
            continue;
        }
        if !current.is_empty() {
            lines.push(std::mem::take(&mut current));
        }
        current = word.to_owned();
        if svg_width(word, size)? > max_width {
            let mut broken = String::new();
            for character in word.chars() {
                broken.push(character);
                if svg_width(&format!("{broken}-"), size)? > max_width {
                    // `brokenWord.slice(0, -1) + '-'`: the piece without the character that
                    // overflowed, then the hyphen.
                    broken.pop();
                    lines.push(format!("{broken}-"));
                    broken = character.to_string();
                }
            }
            current = broken;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn the_legend_lists_array_indices_first_and_leaves_out_the_prototype_name() {
        let actors = Actors::new(
            &names(&["Zed", "10", "__proto__", "2", "Ann"]),
            &names(&["#1", "#2"]),
        );
        let order: Vec<&str> = actors.legend().map(|actor| actor.name.as_str()).collect();
        assert_eq!(order, ["2", "10", "Zed", "Ann"]);
        // The prototype name keeps its place in the order of `getActors`, so its colour follows.
        assert_eq!(actors.get("__proto__").map(|a| a.position), Some(2));
        assert_eq!(
            actors.get("__proto__").map(|a| a.colour.as_str()),
            Some("#1")
        );
        assert_eq!(actors.legend_len(), 4);
    }

    #[test]
    fn a_name_that_is_not_canonical_is_not_an_index() {
        assert_eq!(array_index("0"), Some(0));
        assert_eq!(array_index("007"), None);
        assert_eq!(array_index("-1"), None);
        assert_eq!(array_index("4294967295"), None);
        assert_eq!(array_index("4294967294"), Some(4_294_967_294));
    }

    /// The reference's vector: a long label wraps at the spaces, in the font of the harness
    /// (Lexica 14 px) and a maximum of 320 px (`NOTES.md` 7.3.1).
    #[test]
    fn a_long_name_wraps_at_the_spaces() {
        let lines = wrap(
            "This is a long label that will be split into multiple lines to test the wrapping functionality",
            320.0,
            14.0,
        )
        .expect("measures");
        assert_eq!(
            lines,
            [
                "This is a long label that will be split into multiple",
                "lines to test the wrapping functionality",
            ]
        );
    }

    /// The reference's vector: a word too wide for its line is broken at the characters, with a
    /// hyphen, and the stray hyphen of a first character too wide with its hyphen (`NOTES.md` 7.3.1).
    #[test]
    fn a_word_too_wide_is_broken_with_hyphens() {
        let lines = wrap("Supercalifragilisticexpialidocious", 100.0, 14.0).expect("measures");
        assert_eq!(lines, ["Supercalifragili-", "sticexpialidoci-", "ous",]);
        assert_eq!(wrap("AB", 5.0, 14.0).expect("measures"), ["-", "A-", "B"]);
    }
}
