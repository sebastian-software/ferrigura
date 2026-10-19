//! The colours and rules of a treemap: `styles.ts` for the redux themes, the class declarations of
//! `handDrawnShapeStyles.ts` (`styles2Map`, `isLabelStyle`, `styles2String`), and the inline
//! `style` attribute as d3's `style` calls edit it.
//!
//! The rules of `styles.ts` are the three that match an element: `.treemapLabel`, `.treemapValue`
//! and `.treemapTitle`. The rules for `.treemapNode.section` and `.treemapNode.leaf` match no
//! element (a section has the class `treemapSection`, a leaf the class `leaf<N>x`), so they are not
//! written. Every colour is a custom property declared on the diagram's id, `--textColor` from the
//! base sheet and `--titleColor` here (`docs-port/decisions/0007`).

use ferrigura_model::theme::ThemeTokens;

use crate::common::is_js_whitespace;
use crate::context::ThemeView;
use crate::svg::css_text;

/// `isLabelStyle` (`handDrawnShapeStyles.ts` 53-74): the properties that style a label.
const LABEL_STYLES: [&str; 18] = [
    "color",
    "font-size",
    "font-family",
    "font-weight",
    "font-style",
    "text-decoration",
    "text-align",
    "text-transform",
    "line-height",
    "letter-spacing",
    "word-spacing",
    "text-shadow",
    "text-overflow",
    "white-space",
    "word-wrap",
    "word-break",
    "overflow-wrap",
    "hyphens",
];

/// `isLabelStyle(key)`.
#[must_use]
pub fn is_label_style(key: &str) -> bool {
    LABEL_STYLES.contains(&key)
}

/// The declarations of a node's classes as `styles2String` writes them (`handDrawnShapeStyles.ts`
/// 75-104): each one `key:value !important`, the label ones apart, and the ones whose key holds
/// `stroke` on their own list.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Styles {
    /// The label declarations, joined with `;` (`labelStyles`).
    pub label: String,
    /// The other declarations, joined with `;` (`nodeStyles`).
    pub node: String,
    /// The declarations whose property holds `stroke` (`borderStyles`).
    pub border: Vec<String>,
}

/// `styles2String({ cssCompiledStyles })` for a node with these compiled styles. `styles2Map`
/// splits each style at `:` (the first two pieces), trims both sides, and keeps the first place of a
/// key that repeats; a style without a value has the value `undefined`, which joins as nothing.
#[must_use]
pub fn styles_to_string(compiled: &[String]) -> Styles {
    let mut styles = Styles::default();
    let mut label = Vec::new();
    let mut node = Vec::new();
    for (key, value) in styles2_map(compiled) {
        let declaration = format!("{key}:{} !important", value.unwrap_or_default());
        if is_label_style(&key) {
            label.push(declaration);
        } else {
            if key.contains("stroke") {
                styles.border.push(declaration.clone());
            }
            node.push(declaration);
        }
    }
    styles.label = label.join(";");
    styles.node = node.join(";");
    styles
}

/// `styles2Map(styles)` (`handDrawnShapeStyles.ts` 44-51): an ordered map of the trimmed keys to
/// their trimmed values. A key given again keeps its place and takes the new value.
fn styles2_map(compiled: &[String]) -> Vec<(String, Option<String>)> {
    let mut map: Vec<(String, Option<String>)> = Vec::new();
    for style in compiled {
        let mut pieces = style.split(':');
        let key = trim(pieces.next().unwrap_or_default()).to_owned();
        let value = pieces.next().map(|value| trim(value).to_owned());
        match map.iter_mut().find(|(existing, _)| *existing == key) {
            Some(entry) => entry.1 = value,
            None => map.push((key, value)),
        }
    }
    map
}

/// `String.prototype.trim`.
fn trim(text: &str) -> &str {
    text.trim_matches(is_js_whitespace)
}

/// `label.replace('color:', 'fill:')`: the first `color:` of a label style string becomes `fill:`.
#[must_use]
pub fn colour_as_fill(label: &str) -> String {
    label.replacen("color:", "fill:", 1)
}

/// The declarations of an inline `style` attribute, in order, as d3's `style(name, value)` edits
/// them. A declaration with `!important` keeps that flag.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Inline {
    declarations: Vec<Declaration>,
}

/// One declaration of an inline style.
#[derive(Debug, Clone, PartialEq)]
struct Declaration {
    name: String,
    value: String,
    important: bool,
}

impl Inline {
    /// The declarations of an attribute text, as the style parser reads them: `name: value` pieces
    /// split at `;`, and the flag of a trailing `!important`. A property that is given again moves to
    /// the place of the later declaration (the block keeps one declaration per property), unless the
    /// earlier one is `!important` and the later is not.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut declarations: Vec<Declaration> = Vec::new();
        for piece in text.split(';') {
            let Some((name, value)) = piece.split_once(':') else {
                continue;
            };
            let value = value.trim();
            let (value, important) = match value.strip_suffix("!important") {
                Some(rest) => (rest.trim_end(), true),
                None => (value, false),
            };
            let declaration = Declaration {
                name: name.trim().to_owned(),
                value: value.to_owned(),
                important,
            };
            match declarations
                .iter()
                .position(|existing| existing.name == declaration.name)
            {
                Some(index) => {
                    if declarations[index].important && !declaration.important {
                        continue;
                    }
                    declarations.remove(index);
                    declarations.push(declaration);
                }
                None => declarations.push(declaration),
            }
        }
        Self { declarations }
    }

    /// `style(name, value)`: the first declaration of the name takes the value, without the flag,
    /// or the declaration is added at the end.
    pub fn set(&mut self, name: &str, value: &str) {
        match self
            .declarations
            .iter_mut()
            .find(|declaration| declaration.name == name)
        {
            Some(declaration) => {
                declaration.value = value.to_owned();
                declaration.important = false;
            }
            None => self.declarations.push(Declaration {
                name: name.to_owned(),
                value: value.to_owned(),
                important: false,
            }),
        }
    }

    /// `style(name, null)`: every declaration of the name is removed.
    pub fn remove(&mut self, name: &str) {
        self.declarations
            .retain(|declaration| declaration.name != name);
    }

    /// `style(name)` (`getPropertyValue`): the value of the property in the block, which keeps one
    /// declaration per property.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<String> {
        self.declarations
            .iter()
            .find(|declaration| declaration.name == name)
            .map(|declaration| declaration.value.clone())
    }

    /// The attribute text: `name: value`, with ` !important` where it has the flag, joined by `; `.
    #[must_use]
    pub fn to_attribute(&self) -> String {
        self.declarations
            .iter()
            .map(|declaration| {
                let flag = if declaration.important {
                    " !important"
                } else {
                    ""
                };
                format!("{}: {}{flag}", declaration.name, declaration.value)
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// The `<style>` text of the treemap's own rules, with the title's colour as a custom property on
/// the diagram's id, in both schemes when there are two.
#[must_use]
pub fn own_rules(id: &str, theme: ThemeView<'_>) -> String {
    let tokens = |tokens: &ThemeTokens| {
        format!(
            "--titleColor:{};",
            css_text(&tokens.title_color.text().unwrap_or_default())
        )
    };
    let colours = match theme.dark {
        None => format!("#{id}{{{}}}", tokens(theme.light)),
        Some(dark) => format!(
            "#{id}{{{}}}@media (prefers-color-scheme: dark){{#{id}{{{}}}}}",
            tokens(theme.light),
            tokens(dark)
        ),
    };
    format!(
        "{colours}#{id} .treemapLabel{{fill:var(--textColor);font-size:12px;}}\
#{id} .treemapValue{{fill:var(--textColor);font-size:10px;}}\
#{id} .treemapTitle{{fill:var(--titleColor);font-size:14px;}}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_label_key_goes_to_the_label_declarations() {
        let styles = styles_to_string(&[
            "fill:#ff0000".to_owned(),
            "color:#ffffff".to_owned(),
            "stroke-width:2px".to_owned(),
        ]);
        assert_eq!(
            styles.node,
            "fill:#ff0000 !important;stroke-width:2px !important"
        );
        assert_eq!(styles.label, "color:#ffffff !important");
        assert_eq!(styles.border, ["stroke-width:2px !important"]);
        assert_eq!(colour_as_fill(&styles.label), "fill:#ffffff !important");
    }

    #[test]
    fn a_bare_style_is_a_declaration_without_a_value() {
        let styles = styles_to_string(&[String::new()]);
        assert_eq!(styles.node, ": !important");
    }

    #[test]
    fn a_repeated_key_keeps_its_place_and_takes_the_new_value() {
        let styles = styles_to_string(&[
            "fill:red".to_owned(),
            "stroke:black".to_owned(),
            "fill:blue".to_owned(),
        ]);
        assert_eq!(styles.node, "fill:blue !important;stroke:black !important");
    }

    #[test]
    fn an_inline_style_keeps_the_place_of_a_property_it_sets_again() {
        let mut inline = Inline::parse("font-size: 12px; fill:#000 !important");
        inline.set("font-size", "9px");
        inline.remove("display");
        assert_eq!(
            inline.to_attribute(),
            "font-size: 9px; fill: #000 !important"
        );
        inline.set("display", "none");
        assert_eq!(inline.get("display").as_deref(), Some("none"));
    }

    #[test]
    fn a_repeated_property_keeps_its_important_value_at_the_later_place() {
        let inline = Inline::parse("font-size: 38px;fill:#28253D;font-size:20px !important");
        assert_eq!(inline.get("font-size").as_deref(), Some("20px"));
        assert_eq!(
            inline.to_attribute(),
            "fill: #28253D; font-size: 20px !important"
        );
        let mut set = inline;
        set.set("font-size", "12px");
        assert_eq!(set.to_attribute(), "fill: #28253D; font-size: 12px");
    }
}
