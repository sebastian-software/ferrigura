//! One style sheet for both color schemes (`docs/decisions/0011-themes-as-css-variables.md`).
//!
//! With `colorScheme: auto`, a diagram generates its color rules once per scheme. [`both`] merges
//! the two sheets into one: every color that differs becomes `light-dark(light, dark)`, which
//! resolves against the `color-scheme` the SVG inherits. Inline in a page, the diagram follows the
//! page's scheme; as a standalone document, `svg:root` declares `color-scheme: light dark`, so it
//! follows the viewer's. A value that is not a color, or a dark sheet whose rules do not line up
//! with the light one, keeps the dark value under `prefers-color-scheme: dark`.

use ferrigura_model::theme::khroma;

/// Declares both schemes on a standalone SVG. Inline, the SVG is never the root, so it inherits
/// the page's `color-scheme` instead.
const STANDALONE: &str = "svg:root{color-scheme:light dark;}";
const DARK_MEDIA: &str = "@media (prefers-color-scheme: dark){";

/// The rules of `light` with the colors of `dark` merged in as `light-dark()` values. Both sheets
/// are flat rule lists (`selector{property:value;…}`) that the same code wrote for each scheme.
///
/// The result keeps the cascade of the light sheet followed by the dark one under the media query:
/// a rule is matched by its selector (and its position among rules of that selector), a declaration
/// by its property. A property that only the light rule declares keeps its value in both schemes,
/// as before. What `light-dark()` cannot express (a value that is not a color, or a property or rule
/// that only the dark sheet declares) stays under `prefers-color-scheme: dark`.
#[must_use]
pub fn both(light: &str, dark: &str) -> String {
    let (Some(light_rules), Some(dark_rules)) = (rules(light), rules(dark)) else {
        return separate(light, dark);
    };
    let mut dark_by_key: Vec<(Key<'_>, &Rule<'_>, bool)> = keyed(&dark_rules)
        .into_iter()
        .map(|(key, rule)| (key, rule, false))
        .collect();
    let mut merged = String::from(STANDALONE);
    let mut fallback = String::new();
    for (key, light_rule) in keyed(&light_rules) {
        let dark_rule = dark_by_key
            .iter_mut()
            .find(|(dark_key, _, used)| !*used && *dark_key == key)
            .map(|entry| {
                entry.2 = true;
                entry.1
            });
        let mut kept = String::new();
        let mut dark_only = String::new();
        for (property, light_value) in &light_rule.declarations {
            let dark_value = dark_rule.and_then(|rule| value_of(rule, property));
            let value = match dark_value {
                None => (*light_value).to_owned(),
                Some(dark_value) if dark_value == *light_value => dark_value.to_owned(),
                Some(dark_value) => light_dark(light_value, dark_value).unwrap_or_else(|| {
                    dark_only.push_str(&format!("{property}:{dark_value};"));
                    (*light_value).to_owned()
                }),
            };
            kept.push_str(&format!("{property}:{value};"));
        }
        if let Some(dark_rule) = dark_rule {
            for (property, dark_value) in &dark_rule.declarations {
                if value_of(light_rule, property).is_none() {
                    dark_only.push_str(&format!("{property}:{dark_value};"));
                }
            }
        }
        merged.push_str(&format!("{}{{{kept}}}", light_rule.selector));
        if !dark_only.is_empty() {
            fallback.push_str(&format!("{}{{{dark_only}}}", light_rule.selector));
        }
    }
    for (_, dark_rule, used) in &dark_by_key {
        if !used {
            fallback.push_str(&format!("{}{{{}}}", dark_rule.selector, body(dark_rule)));
        }
    }
    if !fallback.is_empty() {
        merged.push_str(&format!("{DARK_MEDIA}{fallback}}}"));
    }
    merged
}

/// A rule's selector and how many rules with that selector come before it.
type Key<'a> = (&'a str, usize);

fn keyed<'a, 'b>(rules: &'b [Rule<'a>]) -> Vec<(Key<'a>, &'b Rule<'a>)> {
    rules
        .iter()
        .enumerate()
        .map(|(index, rule)| {
            let seen = rules[..index]
                .iter()
                .filter(|earlier| earlier.selector == rule.selector)
                .count();
            ((rule.selector, seen), rule)
        })
        .collect()
}

/// The last value a rule declares for a property, as the cascade applies it.
fn value_of<'a>(rule: &Rule<'a>, property: &str) -> Option<&'a str> {
    rule.declarations
        .iter()
        .rev()
        .find(|(name, _)| *name == property)
        .map(|(_, value)| *value)
}

fn body(rule: &Rule<'_>) -> String {
    rule.declarations
        .iter()
        .map(|(property, value)| format!("{property}:{value};"))
        .collect()
}

/// The sheets as they are, with the dark one under the media query.
fn separate(light: &str, dark: &str) -> String {
    format!("{STANDALONE}{light}{DARK_MEDIA}{dark}}}")
}

/// `light-dark(light, dark)` for two colors, keeping a shared `!important`.
fn light_dark(light: &str, dark: &str) -> Option<String> {
    let (light, light_important) = split_important(light);
    let (dark, dark_important) = split_important(dark);
    if light_important != dark_important || !is_color(light) || !is_color(dark) {
        return None;
    }
    let important = if light_important { "!important" } else { "" };
    Some(format!("light-dark({light},{dark}){important}"))
}

fn split_important(value: &str) -> (&str, bool) {
    match value.strip_suffix("!important") {
        Some(rest) => (rest.trim_end(), true),
        None => (value, false),
    }
}

/// A value `light-dark()` accepts: a color the theme functions parse, a variable (the sheets refer
/// only to color variables), or a color keyword.
fn is_color(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("var(")
        || lower.starts_with("light-dark(")
        || lower == "transparent"
        || lower == "currentcolor"
        || khroma::luminance(value).is_ok()
}

struct Rule<'a> {
    selector: &'a str,
    declarations: Vec<(&'a str, &'a str)>,
}

/// The rules of a flat sheet, or `None` for nested blocks or text that is not a rule list.
fn rules(css: &str) -> Option<Vec<Rule<'_>>> {
    let mut rules = Vec::new();
    let mut rest = css.trim();
    while !rest.is_empty() {
        let open = rest.find('{')?;
        let selector = rest[..open].trim();
        if selector.is_empty() || selector.starts_with('@') {
            return None;
        }
        let body_start = open + 1;
        let close = closing_brace(rest, body_start)?;
        let body = &rest[body_start..close];
        rules.push(Rule {
            selector,
            declarations: declarations(body)?,
        });
        rest = rest[close + 1..].trim_start();
    }
    Some(rules)
}

/// The index of the `}` that ends a block starting at `from`, or `None` when the block nests one.
fn closing_brace(css: &str, from: usize) -> Option<usize> {
    let mut quote = None;
    for (index, character) in css[from..].char_indices() {
        match (quote, character) {
            (Some(open), _) if character == open => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(character),
            (None, '{') => return None,
            (None, '}') => return Some(from + index),
            _ => {}
        }
    }
    None
}

/// The `property:value` pairs of a block body, split at the semicolons outside parentheses and
/// strings.
fn declarations(body: &str) -> Option<Vec<(&str, &str)>> {
    let mut declarations = Vec::new();
    let mut depth = 0_usize;
    let mut quote = None;
    let mut start = 0;
    for (index, character) in body.char_indices() {
        match (quote, character) {
            (Some(open), _) if character == open => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(character),
            (None, '(') => depth += 1,
            (None, ')') => depth = depth.checked_sub(1)?,
            (None, ';') if depth == 0 => {
                push_declaration(&mut declarations, &body[start..index])?;
                start = index + 1;
            }
            _ => {}
        }
    }
    push_declaration(&mut declarations, &body[start..])?;
    Some(declarations)
}

fn push_declaration<'a>(declarations: &mut Vec<(&'a str, &'a str)>, text: &'a str) -> Option<()> {
    let text = text.trim();
    if text.is_empty() {
        return Some(());
    }
    let (property, value) = text.split_once(':')?;
    declarations.push((property.trim(), value.trim()));
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn differing_colors_become_light_dark_values() {
        assert_eq!(
            both(
                "#d{--a:#ffffff;--b:1px;--c:#000000;}",
                "#d{--a:#111111;--b:1px;--c:#000000;}"
            ),
            "svg:root{color-scheme:light dark;}#d{--a:light-dark(#ffffff,#111111);--b:1px;--c:#000000;}"
        );
    }

    #[test]
    fn functional_colors_variables_and_important_values_merge() {
        assert_eq!(
            both(
                "#d .x{fill:hsl(-120, 0%, 80%);stroke:var(--a, #fff)!important;}",
                "#d .x{fill:rgba(1, 2, 3, 0.5);stroke:var(--a, #000)!important;}"
            ),
            "svg:root{color-scheme:light dark;}#d .x{fill:light-dark(hsl(-120, 0%, 80%),rgba(1, 2, 3, 0.5));\
stroke:light-dark(var(--a, #fff),var(--a, #000))!important;}"
        );
    }

    #[test]
    fn a_value_that_is_not_a_color_keeps_the_media_query() {
        assert_eq!(
            both("#d{fill:#fff;opacity:1;}", "#d{fill:#000;opacity:0.5;}"),
            "svg:root{color-scheme:light dark;}#d{fill:light-dark(#fff,#000);opacity:1;}\
@media (prefers-color-scheme: dark){#d{opacity:0.5;}}"
        );
    }

    #[test]
    fn declarations_and_rules_only_the_dark_sheet_has_keep_the_media_query() {
        assert_eq!(
            both(
                "#d{fill:#fff;--only-light:#eee;}#d .a{fill:#fff;}",
                "#d{stroke:#111;fill:#000;}#d .b{fill:#222;}"
            ),
            "svg:root{color-scheme:light dark;}#d{fill:light-dark(#fff,#000);--only-light:#eee;}#d .a{fill:#fff;}\
@media (prefers-color-scheme: dark){#d{stroke:#111;}#d .b{fill:#222;}}"
        );
    }

    #[test]
    fn repeated_selectors_pair_up_in_order() {
        assert_eq!(
            both(
                "#d{fill:#fff;}#d{fill:#eee;}",
                "#d{fill:#000;}#d{fill:#111;}"
            ),
            "svg:root{color-scheme:light dark;}#d{fill:light-dark(#fff,#000);}#d{fill:light-dark(#eee,#111);}"
        );
    }

    #[test]
    fn nested_blocks_stay_separate() {
        let light = "@keyframes a{from{fill:#fff;}}";
        let dark = "@keyframes a{from{fill:#000;}}";
        assert!(both(light, dark).contains(DARK_MEDIA));
    }

    #[test]
    fn semicolons_inside_strings_and_parentheses_do_not_split() {
        let rules = rules("#d{content:\"a;b\";background:url(data:x;y);}").expect("rules");
        assert_eq!(
            rules[0].declarations,
            vec![("content", "\"a;b\""), ("background", "url(data:x;y)")]
        );
    }
}
