//! The layers of configuration: the merges of `assignWithDepth` and `cleanAndMerge`, the sanitising
//! of directives, and the resolution of the appearance settings. Ported from
//! `packages/mermaid/src/config.ts`, `utils/sanitizeDirective.ts`, `assignWithDepth.ts` and
//! `utils.ts`; the order of the layers is the one `mermaidAPI` applies.

use std::collections::BTreeMap as Map;

use regex::Regex;

use super::{CONFIG_KEYS, defaults};
use crate::value::Value;

/// The settings a diagram section may set for its own diagram type.
const APPEARANCE_KEYS: [&str; 3] = ["theme", "look", "layout"];

/// The themes of the reference, `themes/index.js`.
const THEMES: [&str; 11] = [
    "base",
    "dark",
    "default",
    "forest",
    "neutral",
    "neo",
    "neo-dark",
    "redux",
    "redux-dark",
    "redux-color",
    "redux-dark-color",
];

/// The looks of the reference, `config.ts` `LOOKS`.
const LOOKS: [&str; 3] = ["classic", "handDrawn", "neo"];

/// The configuration after one directive, as `updateCurrentConfig` computes it.
///
/// `site` is what the user passed to `initialize`, or `None`. `diagram_key` is the configuration
/// section of the diagram type (`pie` for pie), or `None` when the type is unknown. `directive` is
/// the merged front matter and directive configuration of the text, as `preprocessDiagram` returns
/// it; it is sanitised here, as `addDirective` does.
///
/// The result is the reference's `getConfig()` without `themeVariables`, which the theme port
/// computes later.
#[must_use]
pub fn effective(site: Option<&Value>, diagram_key: Option<&str>, directive: &Value) -> Value {
    let defaults = defaults();
    let site_delta = site.cloned().unwrap_or_else(|| Value::Object(Map::new()));

    let mut site_config = Value::Object(Map::new());
    assign_with_depth(&mut site_config, &defaults, 2);
    assign_with_depth(&mut site_config, &site_delta, 2);

    let mut directives = vec![directive.clone()];
    for item in &mut directives {
        sanitize_directive(item);
    }

    let secure = secure_keys(&site_config);
    let mut sum = Value::Object(Map::new());
    for item in &mut directives {
        sanitize(item, &secure);
        assign_with_depth(&mut sum, item, 2);
    }

    let mut config = Value::Object(Map::new());
    assign_with_depth(&mut config, &site_config, 2);
    assign_with_depth(&mut config, &sum, 2);
    resolve_appearance(
        &mut config,
        &sum,
        &directives,
        &site_delta,
        &defaults,
        diagram_key,
    );
    if let Value::Object(map) = &mut config {
        map.remove("themeVariables");
    }
    config
}

/// Merges `source` into `dst` as `assignWithDepth` does, down to `depth` levels. Objects merge,
/// arrays take the new items that they lack, and a value of another type never replaces an object
/// below the depth limit. See the reference's `assignWithDepth` for the cases.
pub fn assign_with_depth(dst: &mut Value, src: &Value, depth: i32) {
    if let Value::Array(items) = src {
        if let Value::Array(target) = dst {
            for item in items {
                if !target.contains(item) {
                    target.push(item.clone());
                }
            }
        } else {
            for item in items {
                assign_with_depth(dst, item, depth);
            }
        }
        return;
    }

    if dst.is_null() || depth <= 0 {
        match (dst.as_object_mut(), src) {
            (Some(target), Value::Object(source)) => {
                for (key, value) in source {
                    target.insert(key.clone(), value.clone());
                }
            }
            (Some(_), Value::Null) => {}
            _ => *dst = src.clone(),
        }
        return;
    }

    let (Value::Object(target), Value::Object(source)) = (dst, src) else {
        return;
    };
    for (key, src_value) in source {
        if src_value.is_null() {
            continue;
        }
        if src_value.is_object() || src_value.is_array() {
            let child = target.entry(key.clone()).or_insert_with(|| {
                if src_value.is_array() {
                    Value::Array(Vec::new())
                } else {
                    Value::Object(Map::new())
                }
            });
            if child.is_object() || child.is_array() || child.is_null() {
                assign_with_depth(child, src_value, depth - 1);
            }
        } else {
            match target.get(key) {
                Some(existing)
                    if existing.is_object() || existing.is_array() || existing.is_null() => {}
                _ => {
                    target.insert(key.clone(), src_value.clone());
                }
            }
        }
    }
}

/// Deep merge of two configuration values, as the `merge` of es-toolkit that `cleanAndMerge`
/// uses. Objects merge key by key, arrays by index, and any other value replaces the target's.
pub fn deep_merge(target: &mut Value, source: &Value) {
    match (target, source) {
        (Value::Object(target), Value::Object(source)) => {
            for (key, value) in source {
                match target.get_mut(key) {
                    Some(existing)
                        if (existing.is_object() && value.is_object())
                            || (existing.is_array() && value.is_array()) =>
                    {
                        deep_merge(existing, value);
                    }
                    _ => {
                        target.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        (Value::Array(target), Value::Array(source)) => {
            for (index, value) in source.iter().enumerate() {
                if index < target.len() {
                    deep_merge(&mut target[index], value);
                } else {
                    target.push(value.clone());
                }
            }
        }
        (target, source) => *target = source.clone(),
    }
}

/// Removes what a directive may not set, as `sanitizeDirective` does: keys that are not configuration
/// keys, prototype keys, `null` values, unbalanced CSS in `themeCSS` and `fontFamily`, and
/// `themeVariables` values with characters that no colour or font name uses.
pub fn sanitize_directive(args: &mut Value) {
    match args {
        Value::Array(items) => {
            for item in items {
                sanitize_directive(item);
            }
        }
        Value::Object(map) => {
            let keys: Vec<String> = map.keys().cloned().collect();
            for key in keys {
                let Some(value) = map.get(&key) else {
                    continue;
                };
                if key.starts_with("__")
                    || key.contains("proto")
                    || key.contains("constr")
                    || !CONFIG_KEYS.contains(&key.as_str())
                    || value.is_null()
                {
                    map.remove(&key);
                    continue;
                }
                let Some(value) = map.get_mut(&key) else {
                    continue;
                };
                if value.is_object() {
                    match dictionary_pattern(&key) {
                        Some(pattern) => sanitize_dictionary(value, pattern),
                        None => sanitize_directive(value),
                    }
                    continue;
                }
                if let Value::String(text) = value
                    && (key.contains("themeCSS") || key.contains("fontFamily"))
                {
                    let cleaned = sanitize_css(text);
                    *text = cleaned;
                }
            }
            if let Some(Value::Object(variables)) = map.get_mut("themeVariables") {
                for value in variables.values_mut() {
                    if let Value::String(text) = value
                        && !theme_value_pattern().is_match(text)
                    {
                        text.clear();
                    }
                }
            }
        }
        _ => {}
    }
}

/// Removes what a diagram may not take from the text, as `sanitize` in `config.ts` does: the
/// secure keys, prototype keys, and strings with markup or `url(data:`, at every depth.
fn sanitize(options: &mut Value, secure: &[String]) {
    let Value::Object(map) = options else {
        return;
    };
    map.remove("secure");
    for key in secure {
        map.remove(key);
    }
    map.retain(|key, _| !key.starts_with("__"));

    let keys: Vec<String> = map.keys().cloned().collect();
    for key in keys {
        let markup = matches!(
            map.get(&key),
            Some(Value::String(text)) if text.contains('<') || text.contains('>') || text.contains("url(data:")
        );
        if markup {
            map.remove(&key);
        } else if let Some(child @ Value::Object(_)) = map.get_mut(&key) {
            sanitize(child, secure);
        }
    }
}

fn secure_keys(config: &Value) -> Vec<String> {
    config
        .get("secure")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// Sets `theme`, `look` and `layout` on the top level and in the diagram's section, with the
/// first usable value of: the directive's section, the directive, the `initialize` section, the
/// `initialize` value, the defaults. The reference's `resolveAppearance` does the same.
fn resolve_appearance(
    config: &mut Value,
    sum: &Value,
    directives: &[Value],
    site_delta: &Value,
    defaults: &Value,
    diagram_key: Option<&str>,
) {
    let Some(diagram_key) = diagram_key else {
        return;
    };
    let layers: Vec<&Value> = std::iter::once(sum)
        .chain(directives.iter().rev())
        .chain([site_delta, defaults])
        .collect();
    for key in APPEARANCE_KEYS {
        let Some(value) = layers
            .iter()
            .find_map(|layer| read_appearance(layer, diagram_key, key))
        else {
            continue;
        };
        let Value::Object(map) = config else {
            return;
        };
        map.insert(key.to_owned(), value.clone());
        if let Some(Value::Object(section)) = map.get_mut(diagram_key)
            && section.contains_key(key)
        {
            section.insert(key.to_owned(), value);
        }
    }
}

fn read_appearance(layer: &Value, diagram_key: &str, key: &str) -> Option<Value> {
    let section = layer.get(diagram_key).and_then(|section| section.get(key));
    [section, layer.get(key)]
        .into_iter()
        .flatten()
        .find(|value| is_usable_appearance(key, value))
        .cloned()
}

fn is_usable_appearance(key: &str, value: &Value) -> bool {
    match (key, value) {
        ("theme", Value::String(name)) => name == "null" || THEMES.contains(&name.as_str()),
        ("look", Value::String(name)) => LOOKS.contains(&name.as_str()),
        ("look", _) => false,
        ("layout", _) => !value.is_null(),
        _ => false,
    }
}

fn dictionary_pattern(key: &str) -> Option<&'static Regex> {
    static NODE_COLORS: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    static ICONS: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    match key {
        "nodeColors" => Some(NODE_COLORS.get_or_init(|| {
            Regex::new(r"(?i)^#[0-9a-f]{3,8}$|^rgb\([0-9\s%,.]+\)$|^hsl\([0-9\s%,.]+\)$|^[a-z]+$")
                .unwrap_or_else(|_| unreachable!("a fixed pattern"))
        })),
        "filenameIcons" | "extensionIcons" => Some(ICONS.get_or_init(|| {
            Regex::new(r"^[A-Za-z0-9_-]+(?::[A-Za-z0-9_-]+)?$")
                .unwrap_or_else(|_| unreachable!("a fixed pattern"))
        })),
        _ => None,
    }
}

fn sanitize_dictionary(value: &mut Value, pattern: &Regex) {
    let Value::Object(map) = value else {
        return;
    };
    map.retain(|key, entry| {
        !(key.starts_with("__")
            || key.contains("proto")
            || key.contains("constr")
            || !matches!(entry, Value::String(text) if pattern.is_match(text)))
    });
}

fn theme_value_pattern() -> &'static Regex {
    static PATTERN: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r##"^[0-9 "#%(),.;A-Za-z]+$"##)
            .unwrap_or_else(|_| unreachable!("a fixed pattern"))
    })
}

/// Returns the CSS unchanged when its braces balance, and an error comment otherwise, as
/// `sanitizeCss` does.
fn sanitize_css(text: &str) -> String {
    const ERROR: &str = "{ /* ERROR: Unbalanced CSS */ }";
    let mut open = 0_usize;
    let mut close = 0_usize;
    for c in text.chars() {
        if open < close {
            return ERROR.to_owned();
        }
        match c {
            '{' => open += 1,
            '}' => close += 1,
            _ => {}
        }
    }
    if open == close {
        text.to_owned()
    } else {
        ERROR.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `serde_json::json!`, as a configuration value.
    macro_rules! json {
        ($($tokens:tt)*) => {
            Value::from(serde_json::json!($($tokens)*))
        };
    }

    #[test]
    fn assign_merges_objects_two_levels_deep() {
        let mut dst = json!({ "a": { "b": { "x": 1 } }, "c": 1 });
        assign_with_depth(&mut dst, &json!({ "a": { "b": { "y": 2 } } }), 2);
        assert_eq!(dst, json!({ "a": { "b": { "x": 1, "y": 2 } }, "c": 1 }));
    }

    #[test]
    fn assign_does_not_clobber_objects_with_primitives() {
        let mut dst = json!({ "foo": { "bar": "bar" }, "bar": "foo" });
        assign_with_depth(&mut dst, &json!({ "foo": "foo", "bar": "bar" }), 2);
        assert_eq!(dst, json!({ "foo": { "bar": "bar" }, "bar": "bar" }));
    }

    #[test]
    fn assign_appends_missing_array_items() {
        let mut dst = json!({ "list": ["a"] });
        assign_with_depth(&mut dst, &json!({ "list": ["a", "b"] }), 2);
        assert_eq!(dst, json!({ "list": ["a", "b"] }));
    }

    #[test]
    fn deep_merge_takes_later_values_and_merges_objects() {
        let mut target = json!({ "gantt": { "a": 1 }, "theme": "dark" });
        deep_merge(
            &mut target,
            &json!({ "gantt": { "b": 2 }, "theme": "forest" }),
        );
        assert_eq!(
            target,
            json!({ "gantt": { "a": 1, "b": 2 }, "theme": "forest" })
        );
    }

    #[test]
    fn directive_keeps_configuration_keys_only() {
        let mut directive = json!({
            "theme": "forest",
            "nonsense": 1,
            "__proto__": { "x": 1 },
            "look": null,
            "secure": ["theme"],
        });
        sanitize_directive(&mut directive);
        assert_eq!(directive, json!({ "theme": "forest" }));
    }

    #[test]
    fn directive_drops_unbalanced_css_and_odd_theme_variables() {
        let mut directive = json!({
            "fontFamily": "a { b",
            "themeVariables": { "primaryColor": "#fff", "lineColor": "<x>" },
        });
        sanitize_directive(&mut directive);
        assert_eq!(
            directive["fontFamily"],
            json!("{ /* ERROR: Unbalanced CSS */ }")
        );
        assert_eq!(directive["themeVariables"]["primaryColor"], json!("#fff"));
        assert_eq!(directive["themeVariables"]["lineColor"], json!(""));
    }

    #[test]
    fn a_directive_cannot_set_a_secure_key() {
        // `sanitize_directive` is the key gate only; the secure keys are removed when the directive
        // joins the configuration, in `effective`.
        let mut directive = json!({ "securityLevel": "loose", "theme": "forest" });
        sanitize_directive(&mut directive);
        assert_eq!(
            directive,
            json!({ "securityLevel": "loose", "theme": "forest" })
        );
        let config = effective(None, None, &json!({ "securityLevel": "loose" }));
        assert_eq!(config["securityLevel"], json!("strict"));
    }

    #[test]
    fn the_diagram_section_wins_over_the_top_level_appearance() {
        let directive = json!({ "theme": "dark", "pie": { "theme": "forest" } });
        let config = effective(None, Some("pie"), &directive);
        assert_eq!(config["theme"], json!("forest"));
        assert_eq!(config["pie"]["theme"], json!("forest"));
    }

    #[test]
    fn an_unknown_theme_name_falls_through_to_the_site_config() {
        let site = json!({ "theme": "neutral" });
        let config = effective(Some(&site), Some("pie"), &json!({ "theme": "nope" }));
        assert_eq!(config["theme"], json!("neutral"));
    }

    #[test]
    fn site_configuration_keys_are_not_sanitised() {
        let site = json!({ "unknownKey": 1 });
        let config = effective(Some(&site), None, &json!({}));
        assert_eq!(config["unknownKey"], json!(1));
    }
}
