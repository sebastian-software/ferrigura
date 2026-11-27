//! One style sheet for both color schemes (`docs/decisions/0011-themes-as-css-variables.md`).
//!
//! With `colorScheme: auto`, a diagram generates its color rules once per scheme. [`both`] merges
//! the two sheets into one: every color that differs becomes `light-dark(light, dark)`, which
//! resolves against the `color-scheme` the SVG inherits. Inline in a page, the diagram follows the
//! page's scheme; as a standalone document, `svg:root` declares `color-scheme: light dark`, so it
//! follows the viewer's. A value that is not a color, or a dark sheet whose rules do not line up
//! with the light one, keeps the dark value under `prefers-color-scheme: dark`.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

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
    let mut dark_counts: HashMap<&str, usize> = HashMap::with_capacity(dark_rules.len());
    let mut dark_by_key: HashMap<Key<'_>, usize> = HashMap::with_capacity(dark_rules.len());
    for (index, rule) in dark_rules.iter().enumerate() {
        let seen = dark_counts.entry(rule.selector).or_insert(0);
        dark_by_key.insert((rule.selector, *seen), index);
        *seen += 1;
    }
    let mut dark_used = vec![false; dark_rules.len()];
    let mut light_counts: HashMap<&str, usize> = HashMap::with_capacity(light_rules.len());
    let mut merged = String::with_capacity(STANDALONE.len() + light.len() + dark.len() / 2);
    merged.push_str(STANDALONE);
    let mut fallback = String::new();
    let mut kept = String::new();
    let mut dark_only = String::new();
    for light_rule in &light_rules {
        let seen = light_counts.entry(light_rule.selector).or_insert(0);
        let key = (light_rule.selector, *seen);
        *seen += 1;
        let dark_rule = dark_by_key.get(&key).copied().map(|index| {
            dark_used[index] = true;
            &dark_rules[index]
        });
        kept.clear();
        dark_only.clear();
        if let Some(dark_rule) = dark_rule {
            let mut dark_values: HashMap<&str, &str> =
                HashMap::with_capacity(dark_rule.declarations.len());
            for &(property, value) in &dark_rule.declarations {
                dark_values.insert(property, value);
            }
            let mut light_props: HashSet<&str> =
                HashSet::with_capacity(light_rule.declarations.len());
            for &(property, light_value) in &light_rule.declarations {
                light_props.insert(property);
                match dark_values.get(property).copied() {
                    None => {
                        let _ = write!(kept, "{property}:{light_value};");
                    }
                    Some(dark_value) if dark_value == light_value => {
                        let _ = write!(kept, "{property}:{dark_value};");
                    }
                    Some(dark_value) => match light_dark(light_value, dark_value) {
                        Some(combined) => {
                            let _ = write!(kept, "{property}:{combined};");
                        }
                        None => {
                            let _ = write!(dark_only, "{property}:{dark_value};");
                            let _ = write!(kept, "{property}:{light_value};");
                        }
                    },
                }
            }
            for &(property, dark_value) in &dark_rule.declarations {
                if !light_props.contains(property) {
                    let _ = write!(dark_only, "{property}:{dark_value};");
                }
            }
        } else {
            for &(property, light_value) in &light_rule.declarations {
                let _ = write!(kept, "{property}:{light_value};");
            }
        }
        let _ = write!(merged, "{}{{{kept}}}", light_rule.selector);
        if !dark_only.is_empty() {
            let _ = write!(fallback, "{}{{{dark_only}}}", light_rule.selector);
        }
    }
    for (index, dark_rule) in dark_rules.iter().enumerate() {
        if !dark_used[index] {
            let _ = write!(fallback, "{}{{", dark_rule.selector);
            for &(property, value) in &dark_rule.declarations {
                let _ = write!(fallback, "{property}:{value};");
            }
            fallback.push('}');
        }
    }
    if !fallback.is_empty() {
        merged.push_str(DARK_MEDIA);
        merged.push_str(&fallback);
        merged.push('}');
    }
    merged
}

/// A rule's selector and how many rules with that selector come before it.
type Key<'a> = (&'a str, usize);

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
    let bytes = value.as_bytes();
    bytes
        .get(..4)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(b"var("))
        || bytes
            .get(..11)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(b"light-dark("))
        || value.eq_ignore_ascii_case("transparent")
        || value.eq_ignore_ascii_case("currentcolor")
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

    #[test]
    fn many_rules_and_properties_merge_preserving_order() {
        let mut light = String::new();
        let mut dark = String::new();
        for i in 0..300 {
            light.push_str(&format!(
                "#d .r{i}{{fill:#fff;stroke:#eee;opacity:1;color:RGB(1, 2, 3);}}"
            ));
            dark.push_str(&format!(
                "#d .r{i}{{stroke:#111;fill:#000;opacity:0.5;color:RGBA(4, 5, 6, 0.5);}}"
            ));
        }
        let merged = both(&light, &dark);
        assert!(merged.contains("#d .r0{fill:light-dark(#fff,#000);stroke:light-dark(#eee,#111);opacity:1;color:light-dark(RGB(1, 2, 3),RGBA(4, 5, 6, 0.5));}"));
        assert!(merged.contains("#d .r299{fill:light-dark(#fff,#000);"));
        assert!(merged.contains("@media (prefers-color-scheme: dark){#d .r0{opacity:0.5;}"));
    }
}
