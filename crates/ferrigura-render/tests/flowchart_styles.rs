//! The flowchart style sheet against the reference's compiled CSS.
//!
//! `tests/data/flowchart-styles.json` holds the CSS that `mermaidAPI.ts`'s `createUserStyles` compiles
//! for the flowchart, with the harness configuration, for each case (written by
//! `tools/port/flowchart-styles.ts`, `pnpm port:styles-flowchart`). For each case this test renders the
//! sheet with ferrigura and compares the two rule by rule:
//!
//! - every declaration of the reference is in the sheet, with the same selector, the same importance
//!   and the same value once the custom properties of the sheet are substituted, in the same order;
//! - every declaration of the sheet is in the reference, except the documented extras below;
//! - every reference rule that the sheet leaves out is listed in `OMITTED` or `OMITTED_PARTS`, with
//!   the reason, and every entry of those lists is used by some case.
//!
//! A selector list is compared one selector at a time. Values are compared without their whitespace,
//! which CSS does not give meaning to, and the `!important` flag is kept apart.

use std::collections::BTreeSet;

use ferrigura_model::config::{defaults, typed};
use ferrigura_model::flowchart::FlowClass;
use ferrigura_model::theme::{Theme, ThemeTokens};
use ferrigura_model::value::Value;
use ferrigura_render::context::{Globals, ThemeView};
use ferrigura_render::flowchart::style::style_sheet;

const REFERENCE: &str = include_str!("data/flowchart-styles.json");

/// Reference selectors that ferrigura does not write, in their masked form (see [`mask`]), with the
/// reason. Each is a rule that no element of a flowchart's SVG can match, or that the port does not
/// draw (the exceptions are listed in `docs-port/flowchart/render-notes/styles.md`, section 17).
const OMITTED: &[(&str, &str)] = &[
    (
        "#flowchart .error-icon",
        "error diagrams only (styles.ts:71-77)",
    ),
    (
        "#flowchart .error-text",
        "error diagrams only (styles.ts:71-77)",
    ),
    (
        "#flowchart svg",
        "no nested svg element in a flowchart (styles.ts:107-110)",
    ),
    (
        "#flowchart p",
        "margins of HTML paragraphs; labels are SVG text (decision 0004)",
    ),
    (
        "#flowchart span",
        "HTML labels only; labels are SVG text (decision 0004)",
    ),
    (
        "#flowchart .cluster-label span",
        "HTML labels only (decision 0004)",
    ),
    (
        "#flowchart .cluster-label span p",
        "HTML labels only (decision 0004)",
    ),
    (
        "#flowchart .cluster span",
        "HTML labels only (decision 0004)",
    ),
    (
        "#flowchart .edgeLabel p",
        "HTML labels only (decision 0004)",
    ),
    (
        "#flowchart .labelBkg",
        "the background is a div of HTML labels only (createText.ts:69-71)",
    ),
    (
        "#flowchart .icon-shape p",
        "HTML labels only (decision 0004)",
    ),
    (
        "#flowchart .image-shape p",
        "HTML labels only (decision 0004)",
    ),
    (
        "#flowchart .label-icon",
        "icons in labels are written by the HTML path only (createText.ts:344)",
    ),
    (
        "#flowchart .node .label-icon path",
        "icons in labels are written by the HTML path only (createText.ts:344)",
    ),
    (
        "#flowchart div.mermaidTooltip",
        "the tooltip is appended to body, never into the SVG (svgDrawCommon.ts:140-145)",
    ),
    (
        "#flowchart .node .katex path",
        "math in labels is out of scope (RENDER-PLAN, scope)",
    ),
    (
        "#flowchart .root .anchor path",
        "anchors come from state diagrams (anchor.ts draws a circle)",
    ),
    (
        "#flowchart .arrowheadPath",
        "no producer: arrowheadStyle is never read for rendering (styles.md, section 17)",
    ),
    (
        "#flowchart .edgePaths .path",
        "edge paths carry no path class (styles.md, section 17)",
    ),
    (
        "#flowchart :root",
        "--mermaid-font-family matches nothing once namespaced (mermaidAPI.ts:152-154)",
    ),
    (
        "#flowchart .node .neo-node",
        "no producer of the class neo-node (styles.md, N1)",
    ),
    (
        "#flowchart [data-look=\"*\"].node .neo-line path",
        "neo-line comes from class diagrams only (classBox.ts)",
    ),
    (
        "#flowchart [data-look=\"*\"].node circle .state-start",
        "state diagrams only (stateStart.ts)",
    ),
    (
        "#flowchart [data-look=\"*\"].icon-shape .icon",
        "no producer of the class icon (styles.md, N9)",
    ),
    (
        "#flowchart [data-look=\"*\"].icon-shape .icon-neo path",
        "no producer of the class icon-neo (styles.md, N10)",
    ),
];

/// Reference selectors that ferrigura does not write, matched by a part of their masked form.
const OMITTED_PARTS: &[(&str, &str)] = &[
    (
        "rough-node",
        "the handDrawn look is out of scope (RENDER-PLAN, scope)",
    ),
    (
        ".swimlane.cluster",
        "swimlane layout is not ported: ELK Layered only (decision 0006)",
    ),
    (
        "data-color-id=\"color-*\"].cluster:not(.swimlane) path",
        "a flowchart cluster is a rect; its path form is for handDrawn",
    ),
    (
        "collapsed-group path",
        "a collapsed group is drawn as a rect (collapsedGroup.ts:135)",
    ),
];

/// At-rules that ferrigura does not write, by name, with the reason.
const OMITTED_AT_RULES: &[(&str, &str)] = &[(
    "@keyframes edge-animation-frame",
    "no rule uses this animation (styles.ts:47-54)",
)];

/// Declarations that ferrigura writes and the reference does not, with the reason. Custom property
/// declarations are not listed here: they are the sheet's values, not rules, and the test reads them
/// separately.
const EXTRAS: &[(&str, &str, &str)] = &[
    (
        "#flowchart text",
        "white-space",
        "text keeps the spaces the measurement counted (decision 0004; pie.rs)",
    ),
    (
        "#flowchart-drop-shadow feDropShadow",
        "flood-color",
        "the flood of the drop shadows follows the scheme (decision 0007)",
    ),
    (
        "#flowchart-drop-shadow-small feDropShadow",
        "flood-color",
        "the flood of the drop shadows follows the scheme (decision 0007)",
    ),
];

/// One parsed declaration of a rule, with the selector it belongs to.
#[derive(Debug, Clone, PartialEq)]
struct Entry {
    selector: String,
    property: String,
    value: String,
    important: bool,
}

impl Entry {
    fn describe(&self) -> String {
        format!(
            "{} {{{}:{}{}}}",
            self.selector,
            self.property,
            self.value,
            if self.important { " !important" } else { "" }
        )
    }
}

/// A parsed rule or at-rule.
#[derive(Debug)]
enum Item {
    Rule {
        selectors: Vec<String>,
        declarations: Vec<(String, String, bool)>,
    },
    /// An at-rule with a block of its own, as normalised text (for `@keyframes`).
    AtRule { name: String, text: String },
    /// `@media`, with the rules it holds.
    Media { items: Vec<Item> },
}

/// The index of the brace that closes the one at `open`.
fn closing(text: &str, open: usize) -> usize {
    let mut depth = 0_usize;
    for (index, byte) in text.bytes().enumerate().skip(open) {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return index;
                }
            }
            _ => {}
        }
    }
    panic!("an unclosed block in the CSS: {text}");
}

/// The selectors of a selector list, split at the commas that are not inside brackets, parentheses or
/// quotes.
fn split_selectors(list: &str) -> Vec<String> {
    let mut selectors = Vec::new();
    let mut depth = 0_i32;
    let mut quoted = false;
    let mut current = String::new();
    for c in list.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                current.push(c);
            }
            '[' | '(' if !quoted => {
                depth += 1;
                current.push(c);
            }
            ']' | ')' if !quoted => {
                depth -= 1;
                current.push(c);
            }
            ',' if !quoted && depth == 0 => {
                selectors.push(std::mem::take(&mut current));
            }
            _ => current.push(c),
        }
    }
    selectors.push(current);
    selectors
        .into_iter()
        .map(|selector| selector.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect()
}

/// The declarations of a block: `property: value`, with `!important` split off.
fn declarations(body: &str) -> Vec<(String, String, bool)> {
    body.split(';')
        .filter_map(|declaration| {
            let (property, value) = declaration.split_once(':')?;
            let property = property.trim().to_owned();
            let value = value.trim();
            let (value, important) = match value.strip_suffix("!important") {
                Some(rest) => (rest.trim_end(), true),
                None => (value, false),
            };
            Some((property, value.to_owned(), important))
        })
        .collect()
}

/// Parses a stylesheet into rules and at-rules, in order.
fn parse(css: &str) -> Vec<Item> {
    let mut items = Vec::new();
    let mut rest = css.trim_start();
    while !rest.is_empty() {
        let open = rest.find('{').expect("a block in the CSS");
        let close = closing(rest, open);
        let prelude = rest[..open].trim();
        let body = &rest[open + 1..close];
        if let Some(at_rule) = prelude.strip_prefix('@') {
            if at_rule.starts_with("media") {
                items.push(Item::Media { items: parse(body) });
            } else {
                let name = format!("@{}", at_rule.trim());
                let text = format!("{name}{{{body}}}")
                    .split_whitespace()
                    .collect::<String>();
                items.push(Item::AtRule { name, text });
            }
        } else {
            items.push(Item::Rule {
                selectors: split_selectors(prelude),
                declarations: declarations(body),
            });
        }
        rest = rest[close + 1..].trim_start();
    }
    items
}

/// The selector with every colour slot and every look written as `*`, so that the lists of omitted
/// rules need not name each of the twelve slots.
fn mask(selector: &str) -> String {
    let mut out = String::new();
    let mut rest = selector;
    loop {
        let next = ["data-look=\"", "data-color-id=\"color-"]
            .iter()
            .filter_map(|marker| rest.find(marker).map(|at| (at, *marker)))
            .min_by_key(|(at, _)| *at);
        let Some((at, marker)) = next else {
            break;
        };
        let after_marker = at + marker.len();
        out.push_str(&rest[..after_marker]);
        let after = &rest[after_marker..];
        let close = after.find('"').unwrap_or(after.len());
        out.push('*');
        rest = &after[close..];
    }
    out.push_str(rest);
    out
}

/// Why a reference selector is left out, and the key of its entry in the tables.
fn omission(selector: &str) -> Option<(&'static str, &'static str)> {
    let masked = mask(selector);
    if let Some(&(key, reason)) = OMITTED.iter().find(|(key, _)| *key == masked) {
        return Some((key, reason));
    }
    OMITTED_PARTS
        .iter()
        .find(|(part, _)| masked.contains(*part))
        .map(|&(part, reason)| (part, reason))
}

/// The declarations of the sheet's custom properties, by their name without `--`.
fn custom_properties(items: &[Item], id: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for item in items {
        match item {
            Item::Rule {
                selectors,
                declarations,
            } => {
                if selectors.iter().any(|selector| selector == id) {
                    for (property, value, _) in declarations {
                        if let Some(name) = property.strip_prefix("--") {
                            found.push((name.to_owned(), value.clone()));
                        }
                    }
                }
            }
            Item::Media { items } => found.extend(custom_properties(items, id)),
            Item::AtRule { .. } => {}
        }
    }
    found
}

/// The entries of the rules, in order, with `var(--name)` replaced by the value of the custom property.
fn entries(items: &[Item], custom: &[(String, String)], resolve: bool) -> Vec<Entry> {
    let mut out = Vec::new();
    for item in items {
        match item {
            Item::Rule {
                selectors,
                declarations,
            } => {
                for selector in selectors {
                    for (property, value, important) in declarations {
                        if property.starts_with("--") {
                            continue;
                        }
                        let value = if resolve {
                            resolve_variables(value, custom)
                        } else {
                            value.clone()
                        };
                        out.push(Entry {
                            selector: selector.clone(),
                            property: property.clone(),
                            value: value.split_whitespace().collect(),
                            important: *important,
                        });
                    }
                }
            }
            Item::Media { items } => out.extend(entries(items, custom, resolve)),
            Item::AtRule { .. } => {}
        }
    }
    out
}

/// The value with each `var(--name)` replaced by the custom property's value.
fn resolve_variables(value: &str, custom: &[(String, String)]) -> String {
    let mut out = value.to_owned();
    while let Some(start) = out.find("var(--") {
        let end = out[start..]
            .find(')')
            .map(|offset| start + offset)
            .expect("a closed var() in the sheet");
        let name = &out[start + 6..end];
        let replacement = custom
            .iter()
            .find(|(declared, _)| declared == name)
            .map(|(_, colour)| colour.clone())
            .unwrap_or_else(|| panic!("the custom property --{name} is not declared"));
        out.replace_range(start..=end, &replacement);
    }
    out
}

/// The at-rules of a stylesheet by their normalised text.
fn at_rules(items: &[Item]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for item in items {
        match item {
            Item::AtRule { name, text } => out.push((name.clone(), text.clone())),
            Item::Media { items } => out.extend(at_rules(items)),
            Item::Rule { .. } => {}
        }
    }
    out
}

/// Undoes the XML escaping of the `<style>` text.
fn unescape(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// The configuration of a case: the theme and the look, from the defaults.
fn config(theme: &str, look: &str) -> ferrigura_model::config::MermaidConfig {
    let mut value = defaults();
    if let Some(object) = value.as_object_mut() {
        object.insert("theme".to_owned(), Value::from(theme));
        object.insert("look".to_owned(), Value::from(look));
    }
    typed(&value).expect("the defaults read back into the configuration types")
}

/// The theme variables of a case, with the font family the harness gives and the case's overrides.
fn variables(theme: &str, overrides: &serde_json::Value) -> Value {
    let mut merged = serde_json::json!({ "fontFamily": "\"Lexica Ultralegible\"" });
    if let (Some(target), Some(extra)) = (merged.as_object_mut(), overrides.as_object()) {
        for (key, value) in extra {
            target.insert(key.clone(), value.clone());
        }
    }
    Theme::from_name(theme).variables(Some(&Value::from(merged)))
}

#[test]
fn the_style_sheet_matches_the_reference_rule_by_rule() {
    let expected: serde_json::Value =
        serde_json::from_str(REFERENCE).expect("the reference data is JSON");
    let svg_id = expected["svgId"].as_str().expect("svgId").to_owned();
    let cases = expected["cases"].as_array().expect("cases");
    assert!(!cases.is_empty());

    let mut failures = Vec::new();
    let mut used_omissions: BTreeSet<&'static str> = BTreeSet::new();
    for case in cases {
        let name = case["name"].as_str().expect("name");
        let theme = case["theme"].as_str().expect("theme");
        let look = case["look"].as_str().expect("look");
        let variables = variables(theme, &case["overrides"]);
        let classes: Vec<FlowClass> = case["classDefs"]
            .as_array()
            .expect("classDefs")
            .iter()
            .map(|class| FlowClass {
                id: class["id"].as_str().expect("id").to_owned(),
                styles: class["styles"]
                    .as_array()
                    .expect("styles")
                    .iter()
                    .map(|style| style.as_str().expect("style").to_owned())
                    .collect(),
            })
            .collect();
        let tokens = ThemeTokens::from_value(&variables);
        let sheet = unescape(&style_sheet(
            &svg_id,
            ThemeView {
                light: &tokens,
                dark: None,
            },
            &Globals::of(&config(theme, look)),
            &classes,
        ));
        let actual_items = parse(&sheet);
        let expected_items = parse(case["css"].as_str().expect("css"));
        let id = format!("#{svg_id}");
        let custom = custom_properties(&actual_items, &id);

        // The at-rules: each of the reference's (not omitted) must be in the sheet.
        let actual_at_rules = at_rules(&actual_items);
        for (at_name, text) in at_rules(&expected_items) {
            if let Some((key, _)) = OMITTED_AT_RULES.iter().find(|(key, _)| *key == at_name) {
                used_omissions.insert(key);
                continue;
            }
            if !actual_at_rules.iter().any(|(_, actual)| *actual == text) {
                failures.push(format!("[{name}] missing at-rule: {text}"));
            }
        }

        // The declarations, in order.
        let mut expected_entries = Vec::new();
        for item in &expected_items {
            if let Item::Rule {
                selectors,
                declarations,
            } = item
            {
                for selector in selectors {
                    if let Some((key, _)) = omission(selector) {
                        used_omissions.insert(key);
                        continue;
                    }
                    for (property, value, important) in declarations {
                        expected_entries.push(Entry {
                            selector: selector.clone(),
                            property: property.clone(),
                            value: value.split_whitespace().collect(),
                            important: *important,
                        });
                    }
                }
            }
        }
        let actual_entries = entries(&actual_items, &custom, true);
        let mut matched = vec![false; actual_entries.len()];
        let mut cursor = 0;
        for expected_entry in &expected_entries {
            if let Some(index) = (cursor..actual_entries.len())
                .find(|&index| actual_entries[index] == *expected_entry)
            {
                matched[index] = true;
                cursor = index + 1;
            } else if let Some(index) = (0..actual_entries.len())
                .find(|&index| !matched[index] && actual_entries[index] == *expected_entry)
            {
                matched[index] = true;
                failures.push(format!(
                    "[{name}] out of order: {}",
                    expected_entry.describe()
                ));
            } else {
                failures.push(format!("[{name}] missing: {}", expected_entry.describe()));
            }
        }
        for (index, entry) in actual_entries.iter().enumerate() {
            if matched[index] {
                continue;
            }
            let documented = EXTRAS.iter().any(|(selector, property, _)| {
                entry.selector == *selector && entry.property == *property
            });
            if !documented {
                failures.push(format!("[{name}] extra: {}", entry.describe()));
            }
        }
    }

    for (key, _) in OMITTED.iter().chain(OMITTED_PARTS.iter()) {
        assert!(
            used_omissions.contains(key),
            "the omitted rule {key} matches no reference rule; remove it from the table"
        );
    }
    assert!(
        failures.is_empty(),
        "the style sheet differs from the reference:\n{}",
        failures.join("\n")
    );
}

#[test]
fn the_reference_rules_left_out_are_documented() {
    for (key, reason) in OMITTED.iter().chain(OMITTED_PARTS.iter()) {
        assert!(!reason.is_empty(), "{key} has no reason");
    }
    for (selector, property, reason) in EXTRAS {
        assert!(!reason.is_empty(), "{selector} {property} has no reason");
    }
}

#[test]
fn a_selector_is_masked_by_its_colour_slot_and_look() {
    assert_eq!(
        mask("#flowchart [data-look=\"neo\"][data-color-id=\"color-11\"].node .collapsed-group"),
        "#flowchart [data-look=\"*\"][data-color-id=\"color-*\"].node .collapsed-group"
    );
    assert_eq!(
        split_selectors("#a .b,#a span"),
        vec!["#a .b".to_owned(), "#a span".to_owned()]
    );
    assert_eq!(
        split_selectors("#a [x=\"1,2\"]:not(.y,.z),#a p"),
        vec!["#a [x=\"1,2\"]:not(.y,.z)".to_owned(), "#a p".to_owned()]
    );
}
