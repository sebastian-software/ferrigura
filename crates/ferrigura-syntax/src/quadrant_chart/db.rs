//! The quadrant chart DB, `quadrantDb.ts`, with the header setters of `commonDb.ts`, and the
//! conversion of its state to the model. The setters follow `docs-port/quadrant/PLAN.md`,
//! sections 3 and 4: the texts are sanitised when they are set, a point is put in front of the
//! others, a class replaces its styles in place, and the style parser throws on what it does not
//! accept.

use ferrigura_model::quadrant_chart::{QuadrantChartDiagram, QuadrantPoint, QuadrantStyle};

use crate::sanitize::sanitize_text;
use crate::text::{
    is_js_space, js_number, js_trim, newline_then_space_to_newline, strip_leading_space,
};

/// The state the quadrant grammar builds. Every text is sanitised as it is set.
#[derive(Debug)]
pub(crate) struct QuadrantDb {
    title: String,
    acc_title: String,
    acc_descr: String,
    quadrant_texts: [String; 4],
    x_axis_left_text: String,
    x_axis_right_text: String,
    y_axis_bottom_text: String,
    y_axis_top_text: String,
    /// The points in the order the DB holds them: each new point goes in front (`addPoints`).
    points: Vec<QuadrantPoint>,
    /// The classes in the order of their first definition (a `Map`, where `set` keeps the place).
    classes: Vec<(String, QuadrantStyle)>,
}

impl QuadrantDb {
    /// The DB after `clear()`: every text empty, no points and no classes.
    pub(crate) fn new() -> Self {
        Self {
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            quadrant_texts: [String::new(), String::new(), String::new(), String::new()],
            x_axis_left_text: String::new(),
            x_axis_right_text: String::new(),
            y_axis_bottom_text: String::new(),
            y_axis_top_text: String::new(),
            points: Vec::new(),
            classes: Vec::new(),
        }
    }

    /// `commonDb.ts` `setDiagramTitle`: sanitised, not trimmed (the grammar trims).
    pub(crate) fn set_diagram_title(&mut self, text: &str) {
        self.title = sanitize_text(text);
    }

    /// `commonDb.ts` `setAccTitle`: sanitised, then without its leading white space.
    pub(crate) fn set_acc_title(&mut self, text: &str) {
        self.acc_title = strip_leading_space(&sanitize_text(text)).to_owned();
    }

    /// `commonDb.ts` `setAccDescription`: sanitised, then each line break and the white space after
    /// it reduced to one line break.
    pub(crate) fn set_acc_description(&mut self, text: &str) {
        self.acc_descr = newline_then_space_to_newline(&sanitize_text(text));
    }

    /// `setQuadrant1Text` … `setQuadrant4Text`; `quadrant` is 1 to 4.
    pub(crate) fn set_quadrant_text(&mut self, quadrant: usize, text: &str) {
        self.quadrant_texts[quadrant - 1] = Self::text_sanitizer(text);
    }

    pub(crate) fn set_x_axis_left_text(&mut self, text: &str) {
        self.x_axis_left_text = Self::text_sanitizer(text);
    }

    pub(crate) fn set_x_axis_right_text(&mut self, text: &str) {
        self.x_axis_right_text = Self::text_sanitizer(text);
    }

    pub(crate) fn set_y_axis_bottom_text(&mut self, text: &str) {
        self.y_axis_bottom_text = Self::text_sanitizer(text);
    }

    pub(crate) fn set_y_axis_top_text(&mut self, text: &str) {
        self.y_axis_top_text = Self::text_sanitizer(text);
    }

    /// `addPoint(obj, className, x, y, styles)`: the styles are parsed first, then the point goes
    /// in front of the others. `x` and `y` are the text the grammar read, which the DB converts
    /// with `+`, so the model holds `Number(text)`.
    pub(crate) fn add_point(
        &mut self,
        text: &str,
        class_name: &str,
        x: &str,
        y: &str,
        styles: &[String],
    ) -> Result<(), String> {
        let style = parse_styles(styles)?;
        self.points.insert(
            0,
            QuadrantPoint {
                text: Self::text_sanitizer(text),
                class_name: class_name.to_owned(),
                x: js_number(x),
                y: js_number(y),
                style,
            },
        );
        Ok(())
    }

    /// `addClass(name, styles)`: a class of the same name replaces the styles in its place.
    pub(crate) fn add_class(&mut self, name: &str, styles: &[String]) -> Result<(), String> {
        let style = parse_styles(styles)?;
        match self
            .classes
            .iter_mut()
            .find(|(existing, _)| existing == name)
        {
            Some(entry) => entry.1 = style,
            None => self.classes.push((name.to_owned(), style)),
        }
        Ok(())
    }

    /// `textSanitizer`: `sanitizeText(text.trim(), config)`.
    fn text_sanitizer(text: &str) -> String {
        sanitize_text(js_trim(text))
    }

    /// The model of the state the parse left.
    pub(crate) fn into_model(self) -> QuadrantChartDiagram {
        let [
            quadrant1_text,
            quadrant2_text,
            quadrant3_text,
            quadrant4_text,
        ] = self.quadrant_texts;
        QuadrantChartDiagram {
            title: self.title,
            acc_title: self.acc_title,
            acc_descr: self.acc_descr,
            quadrant1_text,
            quadrant2_text,
            quadrant3_text,
            quadrant4_text,
            x_axis_left_text: self.x_axis_left_text,
            x_axis_right_text: self.x_axis_right_text,
            y_axis_bottom_text: self.y_axis_bottom_text,
            y_axis_top_text: self.y_axis_top_text,
            points: self.points,
            classes: self.classes,
            ..QuadrantChartDiagram::new()
        }
    }
}

/// `parseStyles`: each `name: value` text, split at its colon (`style.trim().split(/\s*:\s*/)`).
/// Only the first two pieces count, as the destructuring of the reference takes them. `radius`
/// is a digit string, `color` and `stroke-color` a hex code with or without `#`, and `stroke-width`
/// a number of pixels. Any other name, and a missing or invalid value, is an error.
pub(crate) fn parse_styles(styles: &[String]) -> Result<QuadrantStyle, String> {
    let mut style = QuadrantStyle::default();
    for text in styles {
        let pieces = split_at_colon(js_trim(text));
        let key = pieces.first().copied().unwrap_or("");
        let value = pieces.get(1).copied();
        match key {
            "radius" => {
                let value = valid(value, is_digits, key, "number")?;
                style.radius = Some(js_number(value));
            }
            "color" => {
                style.color = Some(valid(value, is_hex_code, key, "hex code")?.to_owned());
            }
            "stroke-color" => {
                style.stroke_color = Some(valid(value, is_hex_code, key, "hex code")?.to_owned());
            }
            "stroke-width" => {
                let value = valid(value, is_pixels, key, "number of pixels (eg. 10px)")?;
                style.stroke_width = Some(value.to_owned());
            }
            other => return Err(format!("style named {other} is not supported.")),
        }
    }
    Ok(style)
}

/// The value of a style when it passes `check`. A missing value fails the check, as `undefined`
/// does in the reference.
fn valid<'a>(
    value: Option<&'a str>,
    check: fn(&str) -> bool,
    key: &str,
    kind: &str,
) -> Result<&'a str, String> {
    match value {
        Some(value) if check(value) => Ok(value),
        _ => Err(format!(
            "value for {key} {} is invalid, please use a valid {kind}",
            value.unwrap_or("undefined")
        )),
    }
}

/// `\d+`, the whole text.
fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// `#?([\dA-Fa-f]{6}|[\dA-Fa-f]{3})`, the whole text.
fn is_hex_code(text: &str) -> bool {
    let digits = text.strip_prefix('#').unwrap_or(text);
    (digits.len() == 3 || digits.len() == 6) && digits.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// `\d+px`, the whole text.
fn is_pixels(text: &str) -> bool {
    text.strip_suffix("px").is_some_and(is_digits)
}

/// `text.split(/\s*:\s*/)`: the pieces between the colons, with the white space around each colon
/// taken out of them.
fn split_at_colon(text: &str) -> Vec<&str> {
    let mut pieces = Vec::new();
    let mut piece_start = 0;
    loop {
        let Some(colon) = text[piece_start..]
            .find(':')
            .map(|found| piece_start + found)
        else {
            pieces.push(&text[piece_start..]);
            return pieces;
        };
        let mut separator_start = colon;
        for (index, c) in text[piece_start..colon].char_indices().rev() {
            if !is_js_space(c) {
                break;
            }
            separator_start = piece_start + index;
        }
        pieces.push(&text[piece_start..separator_start]);
        let after_colon = colon + 1;
        let spaces = text[after_colon..]
            .chars()
            .take_while(|&c| is_js_space(c))
            .map(char::len_utf8)
            .sum::<usize>();
        piece_start = after_colon + spaces;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_takes_the_white_space_around_each_colon() {
        assert_eq!(split_at_colon("color : #fff"), vec!["color", "#fff"]);
        assert_eq!(split_at_colon("color:#fff:x"), vec!["color", "#fff", "x"]);
        assert_eq!(split_at_colon("color:"), vec!["color", ""]);
        assert_eq!(split_at_colon(":x"), vec!["", "x"]);
        assert_eq!(split_at_colon("plain"), vec!["plain"]);
    }

    #[test]
    fn hex_codes_are_three_or_six_digits_with_an_optional_hash() {
        assert!(is_hex_code("#ff0000"));
        assert!(is_hex_code("abc"));
        assert!(!is_hex_code("ffaa"));
        assert!(!is_hex_code("#f677779"));
        assert!(!is_hex_code("##abc"));
    }
}
