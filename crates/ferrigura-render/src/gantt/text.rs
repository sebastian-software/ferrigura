//! The text of labels, as the reference's SVG holds it: the entity codes of the input, the line
//! breaks of section names, and the width of a run as `getBBox` reports it.

use ferrigura_text::{FontFamily, FontStyle, FontWeight, face, layout_width, measure};

use crate::RenderError;
use crate::svg::escape;

/// A label's text: what the SVG writes (`xml`, with the entity references the reference's decoding
/// leaves in it) and what the DOM reads from it (`plain`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Label {
    /// The text as XML, ready to sit between the tags of an element.
    pub(crate) xml: String,
    /// The text the element holds once parsed.
    pub(crate) plain: String,
}

/// The placeholders that the parser writes for an entity code (`ferrigura-syntax`'s
/// `encode_entities`, which follows `utils.ts:874-896`): `ﬂ°°N¶ß` for a number and `ﬂ°name¶ß` for a
/// name. The reference decodes them in the SVG string (`decodeEntities`, `utils.ts:904-906`), so the
/// entity is read back as the character it names.
const NUMBER_OPEN: &str = "ﬂ°°";
const NAME_OPEN: &str = "ﬂ°";
const CLOSE: &str = "¶ß";

/// The text of a label, from the text the model holds. Entity placeholders become entity
/// references, which the SVG parser reads as the characters they name; an entity name that XML
/// does not define stays as the literal text `&name;`. Every other character is escaped as it is.
pub(crate) fn label(raw: &str) -> Label {
    let mut xml = String::new();
    let mut plain = String::new();
    let mut rest = raw;
    while !rest.is_empty() {
        if let Some((entity, after)) = placeholder(rest) {
            match entity {
                Entity::Number(number) => match number.parse::<u32>().ok().and_then(char::from_u32)
                {
                    Some(character) => {
                        xml.push_str(&format!("&#{number};"));
                        plain.push(character);
                    }
                    None => push_literal(&mut xml, &mut plain, &format!("&#{number};")),
                },
                Entity::Name(name) => match named(name) {
                    Some(character) => {
                        xml.push_str(&format!("&{name};"));
                        plain.push(character);
                    }
                    None => push_literal(&mut xml, &mut plain, &format!("&{name};")),
                },
            }
            rest = after;
            continue;
        }
        let mut characters = rest.chars();
        let character = characters.next().unwrap_or_default();
        push_literal(&mut xml, &mut plain, &character.to_string());
        rest = characters.as_str();
    }
    Label { xml, plain }
}

/// A placeholder of an entity code.
enum Entity<'a> {
    Number(&'a str),
    Name(&'a str),
}

/// The entity placeholder at the start of `text`, and the text after it.
fn placeholder(text: &str) -> Option<(Entity<'_>, &str)> {
    let (body, number) = if let Some(body) = text.strip_prefix(NUMBER_OPEN) {
        (body, true)
    } else {
        (text.strip_prefix(NAME_OPEN)?, false)
    };
    let end = body.find(CLOSE)?;
    let inner = &body[..end];
    let after = &body[end + CLOSE.len()..];
    if inner.is_empty() {
        return None;
    }
    if number {
        return inner
            .bytes()
            .all(|byte| byte.is_ascii_digit())
            .then_some((Entity::Number(inner), after));
    }
    inner
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        .then_some((Entity::Name(inner), after))
}

/// The character of a name that XML defines.
fn named(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => None,
    }
}

/// Appends literal text to both the XML and the plain text.
fn push_literal(xml: &mut String, plain: &mut String, text: &str) {
    xml.push_str(&escape(text));
    plain.push_str(text);
}

/// The pieces of a section name between its line breaks: `<br>`, `</br>`, `<br/>` and `<br />` in
/// any letter case (`common.ts:8`, `/<\/?br\s*\/?>/gi`).
pub(crate) fn line_breaks(raw: &str) -> Vec<&str> {
    let bytes = raw.as_bytes();
    let mut rows = Vec::new();
    let mut row_start = 0;
    let mut index = 0;
    while index < bytes.len() {
        if let Some(end) = line_break_at(raw, index) {
            rows.push(&raw[row_start..index]);
            index = end;
            row_start = end;
        } else {
            index += 1;
        }
    }
    rows.push(&raw[row_start..]);
    rows
}

/// The end of a line break that starts at `start`, if there is one.
fn line_break_at(raw: &str, start: usize) -> Option<usize> {
    let rest = &raw[start..];
    if !rest.starts_with('<') {
        return None;
    }
    let mut index = 1;
    let bytes = rest.as_bytes();
    if bytes.get(index) == Some(&b'/') {
        index += 1;
    }
    if !rest.get(index..index + 2)?.eq_ignore_ascii_case("br") {
        return None;
    }
    index += 2;
    index += bytes[index..]
        .iter()
        .take_while(|b| b.is_ascii_whitespace())
        .count();
    if bytes.get(index) == Some(&b'/') {
        index += 1;
    }
    if bytes.get(index) == Some(&b'>') {
        Some(start + index + 1)
    } else {
        None
    }
}

/// The width of a run of text in the sans face at `size` pixels, as `getBBox().width` reports it:
/// the advance in layout units of 1/64 px (`ferrigura_text::layout_width`), of the text as the
/// browser lays it out (see [`laid_out`]).
pub(crate) fn width(plain: &str, size: f64) -> Result<f64, RenderError> {
    let sans = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
    Ok(layout_width(measure(sans, &laid_out(plain), size)?))
}

/// The text as SVG lays it out with the default `white-space`: each run of spaces, tabs and line
/// breaks is one space, and the spaces at either end are dropped. A label's trailing blanks are
/// therefore not in its `getBBox`, as the reference's measurements show (`width-76.03125` for
/// `Completed task` followed by twelve blanks).
fn laid_out(plain: &str) -> String {
    let mut out = String::with_capacity(plain.len());
    for character in plain.chars() {
        let character = if character.is_whitespace() {
            ' '
        } else {
            character
        };
        if character == ' ' && out.ends_with(' ') {
            continue;
        }
        out.push(character);
    }
    out.trim_matches(' ').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(text: &str) -> String {
        format!("{NAME_OPEN}{text}{CLOSE}")
    }

    fn number(text: &str) -> String {
        format!("{NUMBER_OPEN}{text}{CLOSE}")
    }

    #[test]
    fn a_placeholder_is_read_as_the_character_it_names() {
        let label = label(&format!("a {} b {}", name("quot"), number("35")));
        assert_eq!(label.plain, "a \" b #");
        assert_eq!(label.xml, "a &quot; b &#35;");
    }

    #[test]
    fn a_name_xml_does_not_define_stays_as_text() {
        let label = label(&name("nbsp"));
        assert_eq!(label.plain, "&nbsp;");
        assert_eq!(label.xml, "&amp;nbsp;");
    }

    #[test]
    fn text_without_placeholders_is_escaped() {
        let label = label("a <b> & #c; \"d\"");
        assert_eq!(label.plain, "a <b> & #c; \"d\"");
        assert_eq!(label.xml, "a &lt;b&gt; &amp; #c; &quot;d&quot;");
    }

    #[test]
    fn blanks_at_the_ends_and_runs_of_blanks_are_not_laid_out() {
        assert_eq!(laid_out("Completed task            "), "Completed task");
        assert_eq!(laid_out("  a \t b  "), "a b");
        assert_eq!(laid_out("   "), "");
    }

    #[test]
    fn the_breaks_of_a_section_name_split_it() {
        assert_eq!(
            line_breaks("A<br>B<BR/>C<br />D</br>E"),
            vec!["A", "B", "C", "D", "E"]
        );
        assert_eq!(line_breaks("no breaks"), vec!["no breaks"]);
        assert_eq!(line_breaks("x<bra>y"), vec!["x<bra>y"]);
    }
}
