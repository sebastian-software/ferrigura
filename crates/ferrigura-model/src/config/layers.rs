//! The layers of configuration: the merges of `assignWithDepth` and `cleanAndMerge`, the sanitizing
//! of directives, and the resolution of the appearance settings. Ported from
//! `packages/mermaid/src/config.ts`, `utils/sanitizeDirective.ts`, `assignWithDepth.ts` and
//! `utils.ts`; the order of the layers is the one `mermaidAPI` applies.

use std::collections::BTreeMap as Map;
use std::sync::{Arc, OnceLock};

use ferroni::Regex;

use super::{CONFIG_KEYS, cached_defaults, is_top_key};
use crate::theme::{Theme, has_overrides};
use crate::value::Value;

/// The settings a diagram section may set for its own diagram type.
const APPEARANCE_KEYS: [&str; 3] = ["theme", "look", "layout"];

/// The deepest configuration value nesting that merge and sanitization walks descend into.
const MAX_DEPTH: usize = 128;

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
/// it; it is sanitized here, as `addDirective` does.
///
/// The result is the reference's `getConfig()`, `themeVariables` included: the theme's variables with
/// the overrides applied, as `updateCurrentConfig` computes them.
#[must_use]
pub fn effective(site: Option<&Value>, diagram_key: Option<&str>, directive: &Value) -> Value {
    SiteConfig::new(site).effective(diagram_key, directive)
}

/// The resolved `themeVariables` of a diagram configuration: either the site's cached variables for
/// a shipped theme (unchanged by the directive), or freshly computed variables.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedThemeVariables {
    /// Identical to [`SiteConfig::site_theme_variables`] for this theme.
    Site(Theme),
    /// Modified by the directive (either directive `themeVariables` or a theme change with
    /// overrides).
    Custom(Value),
}

/// The site configuration: what `initialize` gives, merged onto the defaults, with the values that
/// the layers below read. A renderer resolves it once, so that a render merges only the diagram's own
/// front matter and directives on top of it, with [`SiteConfig::effective`] or
/// [`SiteConfig::effective_for`]. [`effective`] is the same merge in one call.
#[derive(Debug, Clone)]
pub struct SiteConfig {
    /// What the user passed to `initialize`, or an empty object: the reference's `site_delta`.
    settings: Value,
    /// `merged` merged onto an empty object, the first layer of every diagram's configuration. It
    /// does not depend on the diagram, so it is merged once and copied for each one.
    first_layer: Value,
    /// The keys that a diagram may not set, from the merged `secure` list.
    secure: Vec<String>,
    /// The theme name of the merged site configuration (`theme_name(&merged, "default")`).
    site_theme: String,
    /// Cached `themeVariables` for `Theme::ReduxColor` with the site's `themeVariables` overrides.
    light_theme_variables: OnceLock<Arc<Value>>,
    /// Cached `themeVariables` for `Theme::ReduxDarkColor` with the site's `themeVariables` overrides.
    dark_theme_variables: OnceLock<Arc<Value>>,
}

impl PartialEq for SiteConfig {
    fn eq(&self, other: &Self) -> bool {
        self.settings == other.settings
    }
}

impl SiteConfig {
    /// The site configuration of the settings given to `initialize`, or of the defaults when there are
    /// none.
    #[must_use]
    pub fn new(site: Option<&Value>) -> Self {
        let settings = site.cloned().unwrap_or_else(Value::empty_object);
        let mut merged = Value::empty_object();
        assign_with_depth(&mut merged, cached_defaults(), 2);
        assign_with_depth(&mut merged, &settings, 2);
        let secure = secure_keys(&merged);
        let site_theme = theme_name(&merged, "default");
        let mut first_layer = Value::empty_object();
        assign_with_depth(&mut first_layer, &merged, 2);
        Self {
            settings,
            first_layer,
            secure,
            site_theme,
            light_theme_variables: OnceLock::new(),
            dark_theme_variables: OnceLock::new(),
        }
    }

    /// The settings given to `initialize`, as they were given, or an empty object.
    #[must_use]
    pub fn settings(&self) -> &Value {
        &self.settings
    }

    /// The theme name resolved from the site configuration (`initialize` over defaults).
    #[must_use]
    pub fn site_theme(&self) -> &str {
        &self.site_theme
    }

    /// Whether the site configuration supplies any `themeVariables` overrides.
    #[must_use]
    pub fn has_theme_overrides(&self) -> bool {
        has_overrides(self.settings.get("themeVariables"))
    }

    /// The `themeVariables` of `theme` with the site's `themeVariables` overrides applied.
    ///
    /// Reuses the process-wide default variables when the site sets no overrides, and caches the
    /// computed variables on this `SiteConfig` otherwise.
    #[must_use]
    pub fn site_theme_variables(&self, theme: Theme) -> &Value {
        let site_overrides = self.settings.get("themeVariables");
        if !has_overrides(site_overrides) {
            return theme.default_variables();
        }
        let slot = match theme {
            Theme::ReduxColor => &self.light_theme_variables,
            Theme::ReduxDarkColor => &self.dark_theme_variables,
        };
        slot.get_or_init(|| Arc::new(theme.variables(site_overrides)))
    }

    /// The configuration of one diagram, as [`effective`] gives it: the site configuration with the
    /// diagram's directive merged on top.
    #[must_use]
    pub fn effective(&self, diagram_key: Option<&str>, directive: &Value) -> Value {
        let defaults = cached_defaults();
        let (mut config, resolved) = if is_empty_object(directive) {
            let mut config = self.first_layer.clone();
            resolve_appearance(&mut config, &[&self.settings, defaults], diagram_key);
            let theme = Theme::from_name(
                config
                    .get("theme")
                    .and_then(Value::as_str)
                    .unwrap_or(&self.site_theme),
            );
            (config, ResolvedThemeVariables::Site(theme))
        } else {
            let (directive, sum) = self.prepare_directive(directive);
            let mut config = self.first_layer.clone();
            assign_with_depth(&mut config, &sum, 2);
            resolve_appearance(
                &mut config,
                &[&sum, &directive, &self.settings, defaults],
                diagram_key,
            );
            let resolved = self.resolve_theme_variables(&sum, &config);
            (config, resolved)
        };
        let variables = match resolved {
            ResolvedThemeVariables::Site(theme) => self.site_theme_variables(theme).clone(),
            ResolvedThemeVariables::Custom(variables) => variables,
        };
        if let Value::Object(map) = &mut config {
            map.insert("themeVariables".to_owned(), variables);
        }
        config
    }

    /// Like [`Self::effective`], but populates only the top-level configuration keys (excluding
    /// `themeVariables`) and the requested diagram `sections` (plus `diagram_key`), returning the
    /// resolved `themeVariables` separately without cloning unused diagram sections or the
    /// site-level `themeVariables` map.
    #[must_use]
    pub fn effective_for(
        &self,
        diagram_key: Option<&str>,
        sections: &[&str],
        directive: &Value,
    ) -> (Value, ResolvedThemeVariables) {
        let defaults = cached_defaults();
        let include_key = |key: &str| -> bool {
            key != "themeVariables"
                && (Some(key) == diagram_key || sections.contains(&key) || is_top_key(key))
        };
        let mut config = match &self.first_layer {
            Value::Object(map) => Value::Object(
                map.iter()
                    .filter(|(key, _)| include_key(key))
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
            ),
            other => other.clone(),
        };
        if is_empty_object(directive) {
            resolve_appearance(&mut config, &[&self.settings, defaults], diagram_key);
            let theme = Theme::from_name(
                config
                    .get("theme")
                    .and_then(Value::as_str)
                    .unwrap_or(&self.site_theme),
            );
            return (config, ResolvedThemeVariables::Site(theme));
        }
        let (directive, sum) = self.prepare_directive(directive);
        assign_selected_with_depth(&mut config, &sum, 2, include_key);
        resolve_appearance(
            &mut config,
            &[&sum, &directive, &self.settings, defaults],
            diagram_key,
        );
        let resolved = self.resolve_theme_variables(&sum, &config);
        (config, resolved)
    }

    /// The user's theme overrides of a diagram with the directive given, as
    /// [`user_theme_variables`] gives them.
    #[must_use]
    pub fn user_theme_variables(&self, directive: &Value) -> Value {
        theme_overrides(Some(&self.settings), directive)
    }

    fn prepare_directive(&self, directive: &Value) -> (Value, Value) {
        let mut directive = directive.clone();
        sanitize_directive(&mut directive);
        add_font_family_variable(&mut directive);
        sanitize(&mut directive, &self.secure);
        let mut sum = Value::empty_object();
        assign_with_depth(&mut sum, &directive, 2);
        (directive, sum)
    }

    /// Resolves the `themeVariables` of the configuration, as `updateCurrentConfig` computes them.
    /// When the directive leaves `themeVariables` empty or absent, returns
    /// [`ResolvedThemeVariables::Site`] so callers can borrow the cached site-level variables.
    fn resolve_theme_variables(&self, sum: &Value, config: &Value) -> ResolvedThemeVariables {
        let directive_overrides = sum.get("themeVariables");
        let resolved_theme_name = config
            .get("theme")
            .and_then(Value::as_str)
            .unwrap_or(&self.site_theme);
        let resolved_theme = Theme::from_name(resolved_theme_name);
        let Some(directive_overrides) = directive_overrides.filter(|value| match value {
            Value::Object(map) => !map.is_empty(),
            Value::Array(items) => !items.is_empty(),
            _ => true,
        }) else {
            return ResolvedThemeVariables::Site(resolved_theme);
        };
        let theme_set_by_directive = sum
            .get("theme")
            .and_then(Value::as_str)
            .is_some_and(|name| !name.is_empty());
        if theme_set_by_directive || resolved_theme_name != self.site_theme {
            let mut overrides = self
                .settings
                .get("themeVariables")
                .cloned()
                .unwrap_or_else(Value::empty_object);
            assign_with_depth(&mut overrides, directive_overrides, 2);
            ResolvedThemeVariables::Custom(resolved_theme.variables(Some(&overrides)))
        } else {
            let mut variables = self
                .site_theme_variables(Theme::from_name(&self.site_theme))
                .clone();
            assign_with_depth(&mut variables, directive_overrides, 2);
            ResolvedThemeVariables::Custom(variables)
        }
    }
}

fn is_empty_object(value: &Value) -> bool {
    matches!(value, Value::Object(map) if map.is_empty())
}

fn theme_name(config: &Value, fallback: &str) -> String {
    config
        .get("theme")
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_owned()
}

/// The user's theme overrides, as `updateCurrentConfig` merges them before a theme computes its
/// variables: the `initialize` overrides, with the directive's overrides on top and the directive's
/// `fontFamily` included. A theme that is not the configured one takes these, so that a color
/// scheme can be rendered with another shipped theme and the same overrides.
#[must_use]
pub fn user_theme_variables(site: Option<&Value>, directive: &Value) -> Value {
    theme_overrides(site, directive)
}

/// The overrides of [`user_theme_variables`]: the `initialize` overrides with the directive's on top.
fn theme_overrides(site: Option<&Value>, directive: &Value) -> Value {
    let mut overrides = site
        .and_then(|site| site.get("themeVariables"))
        .cloned()
        .unwrap_or_else(Value::empty_object);
    if directive.get("themeVariables").is_none() && directive.get("fontFamily").is_none() {
        return overrides;
    }
    let mut directive = directive.clone();
    sanitize_directive(&mut directive);
    add_font_family_variable(&mut directive);
    if let Some(directive_overrides) = directive.get("themeVariables") {
        assign_with_depth(&mut overrides, directive_overrides, 2);
    }
    overrides
}

/// JavaScript's truthiness of a configuration value.
fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(flag) => *flag,
        Value::Number(number) => *number != 0.0 && !number.is_nan(),
        Value::String(text) => !text.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// `addDirective` gives a directive's `fontFamily` to the theme variables, unless they set their own.
fn add_font_family_variable(directive: &mut Value) {
    let Some(font_family) = directive.get("fontFamily").cloned() else {
        return;
    };
    if !is_truthy(&font_family) {
        return;
    }
    let has_own = directive
        .get("themeVariables")
        .and_then(|variables| variables.get("fontFamily"))
        .is_some_and(is_truthy);
    if has_own {
        return;
    }
    let mut variables = directive
        .get("themeVariables")
        .cloned()
        .unwrap_or_else(Value::empty_object);
    if let Value::Object(map) = &mut variables {
        map.insert("fontFamily".to_owned(), font_family);
    }
    if let Value::Object(map) = directive {
        map.insert("themeVariables".to_owned(), variables);
    }
}

/// Merges `source` into `dst` as `assignWithDepth` does, down to `depth` levels. Objects merge,
/// arrays take the new items that they lack, and a value of another type never replaces an object
/// below the depth limit. See the reference's `assignWithDepth` for the cases.
pub fn assign_with_depth(dst: &mut Value, src: &Value, depth: i32) {
    assign_with_depth_inner(dst, src, depth, 0);
}

fn assign_with_depth_inner(dst: &mut Value, src: &Value, depth: i32, recursion: usize) {
    if recursion >= MAX_DEPTH {
        return;
    }
    if let Value::Array(items) = src {
        if let Value::Array(target) = dst {
            for item in items {
                if !target.contains(item) {
                    target.push(item.clone());
                }
            }
        } else {
            for item in items {
                assign_with_depth_inner(dst, item, depth, recursion + 1);
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
                assign_with_depth_inner(child, src_value, depth - 1, recursion + 1);
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

/// Merges selected top-level keys of object `src` into object `dst` with [`assign_with_depth`].
fn assign_selected_with_depth(
    dst: &mut Value,
    src: &Value,
    depth: i32,
    mut include: impl FnMut(&str) -> bool,
) {
    let (Value::Object(target), Value::Object(source)) = (dst, src) else {
        return;
    };
    for (key, src_value) in source {
        if !include(key) || src_value.is_null() {
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
                assign_with_depth_inner(child, src_value, depth - 1, 1);
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
    deep_merge_inner(target, source, 0);
}

fn deep_merge_inner(target: &mut Value, source: &Value, depth: usize) {
    if depth >= MAX_DEPTH {
        return;
    }
    match (target, source) {
        (Value::Object(target), Value::Object(source)) => {
            for (key, value) in source {
                match target.get_mut(key) {
                    Some(existing)
                        if (existing.is_object() && value.is_object())
                            || (existing.is_array() && value.is_array()) =>
                    {
                        deep_merge_inner(existing, value, depth + 1);
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
                    deep_merge_inner(&mut target[index], value, depth + 1);
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
/// `themeVariables` values with characters that no color or font name uses.
pub fn sanitize_directive(args: &mut Value) {
    sanitize_directive_inner(args, 0);
}

fn sanitize_directive_inner(args: &mut Value, depth: usize) {
    if depth >= MAX_DEPTH {
        match args {
            Value::Array(items) => items.clear(),
            Value::Object(map) => map.clear(),
            _ => {}
        }
        return;
    }
    match args {
        Value::Array(items) => {
            for item in items {
                sanitize_directive_inner(item, depth + 1);
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
                    || CONFIG_KEYS.binary_search(&key.as_str()).is_err()
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
                        None => sanitize_directive_inner(value, depth + 1),
                    }
                    continue;
                }
                if let Value::String(text) = value
                    && (key.contains("themeCSS") || key.contains("fontFamily"))
                {
                    let cleaned = sanitize_css(text);
                    if key.contains("fontFamily")
                        && cleaned == *text
                        && text
                            .bytes()
                            .any(|b| matches!(b, b';' | b'{' | b'}' | b'\\' | b'\n' | b'\r'))
                    {
                        text.clear();
                    } else {
                        *text = cleaned;
                    }
                }
            }
            if let Some(variables) = map.get_mut("themeVariables") {
                sanitize_theme_variables(variables, depth + 1);
            }
        }
        _ => {}
    }
}

/// Recursively sanitizes `themeVariables` values, including arrays of override objects that
/// `assign_with_depth` would otherwise unpack into the theme map.
fn sanitize_theme_variables(value: &mut Value, depth: usize) {
    if depth >= MAX_DEPTH {
        match value {
            Value::Array(items) => items.clear(),
            Value::Object(map) => map.clear(),
            Value::String(text) => text.clear(),
            _ => {}
        }
        return;
    }
    match value {
        Value::Object(variables) => {
            variables.retain(|key, _| !key.contains("proto") && !key.contains("constr"));
            for entry in variables.values_mut() {
                sanitize_theme_variables(entry, depth + 1);
            }
        }
        Value::Array(items) => {
            for item in items {
                sanitize_theme_variables(item, depth + 1);
            }
        }
        Value::String(text) if !theme_value_pattern().is_match(text) => {
            text.clear();
        }
        _ => {}
    }
}

/// Removes what a diagram may not take from the text, as `sanitize` in `config.ts` does: the
/// secure keys, prototype keys, and strings with markup or `url(data:`, at every depth.
fn sanitize(options: &mut Value, secure: &[String]) {
    sanitize_inner(options, secure, 0);
}

fn sanitize_inner(options: &mut Value, secure: &[String], depth: usize) {
    let Value::Object(map) = options else {
        return;
    };
    if depth >= MAX_DEPTH {
        map.clear();
        return;
    }
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
            sanitize_inner(child, secure, depth + 1);
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
/// first usable value across `layers`. The reference's `resolveAppearance` does the same.
fn resolve_appearance(config: &mut Value, layers: &[&Value], diagram_key: Option<&str>) {
    let Some(diagram_key) = diagram_key else {
        return;
    };
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
            Regex::new(
                r"(?i)\A#[0-9a-f]{3,8}\z|\Argb\([0-9\s%,.]+\)\z|\Ahsl\([0-9\s%,.]+\)\z|\A[a-z]+\z",
            )
            .unwrap_or_else(|_| unreachable!("a fixed pattern"))
        })),
        "filenameIcons" | "extensionIcons" => Some(ICONS.get_or_init(|| {
            Regex::new(r"\A[A-Za-z0-9_-]+(?::[A-Za-z0-9_-]+)?\z")
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
        Regex::new(r##"\A[0-9 "#%(),.;A-Za-z]+\z"##)
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
    fn site_configuration_keys_are_not_sanitized() {
        let site = json!({ "unknownKey": 1 });
        let config = effective(Some(&site), None, &json!({}));
        assert_eq!(config["unknownKey"], json!(1));
    }

    #[test]
    fn a_flowchart_defaults_to_the_neo_look_and_the_redux_color_theme() {
        // `config.appearance.spec.ts`: the flowchart section's defaults resolve onto the top level.
        let config = effective(None, Some("flowchart"), &json!({}));
        assert_eq!(config["look"], json!("neo"));
        assert_eq!(config["theme"], json!("redux-color"));
        let classic = effective(None, Some("flowchart"), &json!({ "look": "classic" }));
        assert_eq!(classic["look"], json!("classic"));
    }

    #[test]
    fn a_site_configuration_resolved_once_gives_the_effective_configuration_of_each_call() {
        // A renderer keeps the site configuration and merges each diagram's directive on top of it;
        // the result must be the one `effective` gives in one call, for every diagram section.
        let site = json!({
            "theme": "neutral",
            "themeVariables": { "primaryColor": "#336699" },
            "flowchart": { "curve": "basis" },
            "secure": ["fontSize"],
        });
        let directives = [
            json!({}),
            json!({ "theme": "forest", "pie": { "theme": "dark" } }),
            json!({ "fontSize": 40, "themeVariables": { "lineColor": "#ff00aa" } }),
            json!({ "fontFamily": "Arial", "look": "handDrawn", "layout": "elk" }),
        ];
        let resolved = SiteConfig::new(Some(&site));
        for key in [None, Some("pie"), Some("flowchart"), Some("mindmap")] {
            for directive in &directives {
                assert_eq!(
                    resolved.effective(key, directive),
                    effective(Some(&site), key, directive),
                    "{key:?} with {directive:?}"
                );
                assert_eq!(
                    resolved.user_theme_variables(directive),
                    user_theme_variables(Some(&site), directive),
                );
            }
        }
        assert_eq!(resolved, SiteConfig::new(Some(&site)));
        // Site-level theme variables are cached by pointer once resolved.
        let first_light = resolved.site_theme_variables(Theme::ReduxColor);
        let second_light = resolved.site_theme_variables(Theme::ReduxColor);
        assert!(std::ptr::eq(first_light, second_light));
    }

    #[test]
    fn effective_for_matches_effective_on_every_included_key_and_theme_variables() {
        let sites = [
            json!({}),
            json!({
                "theme": "neutral",
                "themeVariables": { "primaryColor": "#336699", "fontFamily": "Fixture" },
                "flowchart": { "curve": "basis" },
                "secure": ["fontSize"],
            }),
            json!({ "theme": "dark", "themeVariables": { "lineColor": "#abcdef" } }),
        ];
        let directives = [
            json!({}),
            json!({ "theme": "forest", "pie": { "theme": "dark", "textPosition": 0.9 } }),
            json!({ "fontSize": 40, "themeVariables": { "lineColor": "#ff00aa" } }),
            json!({ "fontFamily": "Arial", "look": "handDrawn", "layout": "elk" }),
            json!({ "themeVariables": {} }),
            json!({ "themeVariables": 3, "gantt": { "barHeight": 30 } }),
        ];
        for site in &sites {
            let resolved = SiteConfig::new(Some(site));
            for (diagram_key, sections) in [
                (None, &[][..]),
                (Some("pie"), &[][..]),
                (
                    Some("flowchart"),
                    &["flowchart", "elk", "class", "er", "state"][..],
                ),
                (Some("kanban"), &["mindmap"][..]),
                (Some("treemap"), &[][..]),
            ] {
                for directive in &directives {
                    let full = resolved.effective(diagram_key, directive);
                    let (scoped, theme_vars) =
                        resolved.effective_for(diagram_key, sections, directive);
                    let scoped_map = scoped.as_object().expect("scoped config is an object");
                    assert!(!scoped_map.contains_key("themeVariables"));
                    for (key, value) in scoped_map {
                        assert_eq!(
                            full.get(key),
                            Some(value),
                            "mismatch for key {key} ({diagram_key:?}, {directive:?})"
                        );
                    }
                    if let Some(diagram_key) = diagram_key {
                        assert_eq!(
                            scoped.get(diagram_key),
                            full.get(diagram_key),
                            "diagram section {diagram_key}"
                        );
                    }
                    for &section in sections {
                        assert_eq!(
                            scoped.get(section),
                            full.get(section),
                            "extra section {section}"
                        );
                    }
                    let actual_vars = match &theme_vars {
                        ResolvedThemeVariables::Site(theme) => {
                            resolved.site_theme_variables(*theme)
                        }
                        ResolvedThemeVariables::Custom(vars) => vars,
                    };
                    assert_eq!(
                        Some(actual_vars),
                        full.get("themeVariables"),
                        "themeVariables mismatch ({diagram_key:?}, {directive:?})"
                    );
                }
            }
        }
    }

    #[test]
    fn sanitize_directive_bounds_deeply_nested_theme_variables() {
        let mut nested = json!({ "primaryColor": "red;}</style><script>alert(1)</script>" });
        for _ in 0..512 {
            nested = Value::Array(vec![nested]);
        }
        let mut directive = Value::Object(Map::from([("themeVariables".to_owned(), nested)]));
        sanitize_directive(&mut directive);
        let mut cursor = directive.get("themeVariables");
        let mut depth = 0;
        while let Some(Value::Array(items)) = cursor {
            depth += 1;
            cursor = items.first();
        }
        assert!(depth <= MAX_DEPTH);
        assert!(cursor.is_none());
    }
}
