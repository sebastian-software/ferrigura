//! Configuration carried by the text: the front matter `config` block and the `%%{init}%%`,
//! `%%{initialize}%%` and `%%{wrap}%%` directives.
//!
//! Ported from `packages/mermaid/src/preprocess.ts` (`processFrontmatter`, `processDirectives`,
//! `preprocessDiagram`), `utils.ts` (`detectDirective`, `detectInit`) and
//! `diagram-api/frontmatter.ts`. The result is `config` of `preprocessDiagram`: the front matter
//! configuration merged with the directives, not yet sanitised. The sanitising and the layers of
//! the configuration are in `ferrigura_model::config::effective`.

use ferrigura_model::config::{assign_with_depth, deep_merge, sanitize_directive};
use serde_json::{Map, Value};

use crate::text::{is_js_space, is_line_term, is_word};
use crate::{DiagramType, ParseError, detect, preprocess};

/// Reads the configuration that the text carries; see the module documentation.
pub fn read(text: &str) -> Result<Value, ParseError> {
    let prepared = preprocess::prepare(text);
    let front_matter = front_matter_config(prepared.front_matter.as_deref())?;

    let diagram = detect(text).map(DiagramType::config_key);
    let mut directive =
        detect_init(&prepared.text, diagram).unwrap_or_else(|| Value::Object(Map::new()));
    if let (Some(wrap), Value::Object(map)) = (detect_wrap(&prepared.text), &mut directive) {
        map.insert("wrap".to_owned(), Value::Bool(wrap));
    }

    let mut merged = Value::Object(Map::new());
    deep_merge(&mut merged, &front_matter);
    deep_merge(&mut merged, &directive);
    Ok(merged)
}

/// The `config` block of the front matter, with the legacy `displayMode` placed in `gantt`, as
/// `processFrontmatter` does. Only object-valued blocks are read.
fn front_matter_config(body: Option<&str>) -> Result<Value, ParseError> {
    let Some(body) = body else {
        return Ok(Value::Object(Map::new()));
    };
    let parsed: Value = serde_yaml_ng::from_str(body).map_err(|error| ParseError {
        line: 1,
        column: 1,
        message: format!("invalid front matter: {error}"),
    })?;
    let Value::Object(fields) = parsed else {
        return Ok(Value::Object(Map::new()));
    };

    let mut config = match fields.get("config") {
        Some(Value::Object(config)) => config.clone(),
        _ => Map::new(),
    };
    if let Some(mode) = fields.get("displayMode").filter(|value| is_truthy(value)) {
        let mode = match mode {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        };
        let gantt = config
            .entry("gantt")
            .or_insert_with(|| Value::Object(Map::new()));
        if let Value::Object(gantt) = gantt {
            gantt.insert("displayMode".to_owned(), Value::String(mode));
        }
    }
    Ok(Value::Object(config))
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(flag) => *flag,
        Value::Number(number) => number.as_f64().is_some_and(|float| float != 0.0),
        Value::String(text) => !text.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// `detectInit`: the arguments of the `init` and `initialize` directives, merged in order, with
/// the `config` key moved under the diagram's section. `None` when there is none, or when the
/// arguments are not an object.
fn detect_init(text: &str, diagram: Option<&str>) -> Option<Value> {
    let found = directives(text).ok()?;
    let inits: Vec<&Directive> = found
        .iter()
        .filter(|directive| is_init_type(&directive.kind))
        .collect();
    let mut results = match inits.as_slice() {
        [] => return None,
        [only] => only.args.clone(),
        many => {
            let mut args = Value::Array(
                many.iter()
                    .map(|directive| directive.args.clone())
                    .collect(),
            );
            sanitize_directive(&mut args);
            let mut merged = Value::Object(Map::new());
            assign_with_depth(&mut merged, &args, 2);
            merged
        }
    };
    let Value::Object(map) = &mut results else {
        return None;
    };
    if let Some(config) = map.remove("config")
        && let Some(key) = diagram
    {
        map.insert(key.to_owned(), config);
    }
    Some(results)
}

/// `wrap` as `processDirectives` sets it: `Some(true)` for a `%%{wrap}%%` directive, `Some(false)`
/// when several directives named like `wrap` appear and none is exactly `wrap`, `None` otherwise.
fn detect_wrap(text: &str) -> Option<bool> {
    let found = directives(text).ok()?;
    let wraps: Vec<&Directive> = found
        .iter()
        .filter(|directive| directive.kind.contains("wrap"))
        .collect();
    match wraps.as_slice() {
        [] => None,
        [only] => (only.kind == "wrap").then_some(true),
        many => Some(many.iter().any(|directive| directive.kind == "wrap")),
    }
}

/// `init\b` or `initialize\b` anywhere in the type name, as the reference matches it.
fn is_init_type(kind: &str) -> bool {
    ["init", "initialize"].iter().any(|word| {
        kind.match_indices(word).any(|(at, _)| {
            kind[at + word.len()..]
                .chars()
                .next()
                .is_none_or(|next| !is_word(next))
        })
    })
}

/// A directive as `detectDirective` returns it.
struct Directive {
    kind: String,
    args: Value,
}

/// `detectDirective(text)`: every `%%{ … }%%` directive in the text, in order. `Err` when the
/// arguments of any directive are not JSON; the reference then reports no directive at all.
fn directives(text: &str) -> Result<Vec<Directive>, ()> {
    let text = remove_comment_lines(text.trim()).replace('\'', "\"");
    let chars: Vec<char> = text.chars().collect();
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(next) = next_directive(&chars, from) {
        from = next.end;
        let args = match next.args {
            Args::None => Value::Null,
            Args::Word(word) => Value::String(word),
            Args::Json(json) => serde_json::from_str(json.trim()).map_err(|_| ())?,
        };
        found.push(Directive {
            kind: next.kind,
            args,
        });
    }
    Ok(found)
}

/// The arguments of one directive, before they are parsed.
enum Args {
    None,
    Word(String),
    Json(String),
}

struct Next {
    kind: String,
    args: Args,
    end: usize,
}

/// The first directive at or after `from`, matched as `directiveRegex` matches it.
fn next_directive(chars: &[char], from: usize) -> Option<Next> {
    (from..chars.len().saturating_sub(2))
        .filter(|&start| chars[start..].starts_with(&['%', '%', '{']))
        .find_map(|start| match_directive(chars, start))
}

/// The directive that starts at `start`: `%%{`, the type (`name:` or a bare word), a word or the
/// text up to the closing `}%%`, and the closing `}%%` when it is there.
fn match_directive(chars: &[char], start: usize) -> Option<Next> {
    let word_at = |from: usize| -> usize {
        let mut end = from;
        while end < chars.len() && is_word(chars[end]) {
            end += 1;
        }
        end
    };
    let space_at = |from: usize| -> usize {
        let mut end = from;
        while end < chars.len() && is_js_space(chars[end]) {
            end += 1;
        }
        end
    };

    let type_start = space_at(start + 3);
    let type_end = word_at(type_start);
    if type_end == type_start {
        return None;
    }
    let kind: String = chars[type_start..type_end].iter().collect();
    let after_type = space_at(type_end);
    let after_name = if chars.get(after_type) == Some(&':') {
        after_type + 1
    } else {
        type_end
    };

    let args_start = space_at(after_name);
    let word_end = word_at(args_start);
    let (args, content_end) = if word_end > args_start {
        let word: String = chars[args_start..word_end].iter().collect();
        (Args::Word(word), word_end)
    } else {
        let json_end = json_content_end(chars, args_start);
        if json_end > args_start {
            let json: String = chars[args_start..json_end].iter().collect();
            (Args::Json(json), json_end)
        } else {
            (Args::None, args_start)
        }
    };

    let mut end = space_at(content_end);
    if starts_with(chars, end, &['}', '%', '%']) {
        end += 3;
    }
    Some(Next { kind, args, end })
}

/// Where the free text of a directive stops: before the first `}%%`, or at a line break that is
/// not `\r\n` or `\n`, which the reference's `.` cannot cross.
fn json_content_end(chars: &[char], from: usize) -> usize {
    let mut i = from;
    while i < chars.len() {
        if starts_with(chars, i, &['}', '%', '%']) {
            break;
        }
        match chars[i] {
            '\n' => i += 1,
            '\r' if chars.get(i + 1) == Some(&'\n') => i += 2,
            c if is_line_term(c) => break,
            _ => i += 1,
        }
    }
    i
}

/// `commentWithoutDirectives` of `detectDirective`: a `%%}%%` line is removed with its line break.
fn remove_comment_lines(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if starts_with(&chars, i, &['%', '%', '}', '%', '%']) {
            let line_end = (i + 2..chars.len()).find(|&k| is_line_term(chars[k]));
            if let Some(end) = line_end.filter(|&k| chars[k] == '\n') {
                i = end + 1;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn starts_with(chars: &[char], at: usize, pattern: &[char]) -> bool {
    chars.get(at..at + pattern.len()) == Some(pattern)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn reads_an_init_directive() {
        let config = read("%%{init: {\"theme\": \"forest\"}}%%\npie\n\"a\": 1").expect("reads");
        assert_eq!(config, json!({ "theme": "forest" }));
    }

    #[test]
    fn reads_initialize_with_single_quotes() {
        let config = read("%%{initialize: {'theme': 'neutral'}}%%\npie").expect("reads");
        assert_eq!(config, json!({ "theme": "neutral" }));
    }

    #[test]
    fn a_directive_overrides_the_front_matter() {
        let text = "---\nconfig:\n  theme: dark\n  look: neo\n---\n%%{init: {\"theme\": \"forest\"}}%%\npie";
        let config = read(text).expect("reads");
        assert_eq!(config, json!({ "theme": "forest", "look": "neo" }));
    }

    #[test]
    fn the_config_key_moves_under_the_diagram_section() {
        let config = read("%%{init: {\"config\": {\"donutHole\": 0.3}}}%%\npie").expect("reads");
        assert_eq!(config, json!({ "pie": { "donutHole": 0.3 } }));
    }

    #[test]
    fn several_init_directives_merge_in_order() {
        let text = "%%{init: {\"theme\": \"dark\"}}%%\n%%{init: {\"theme\": \"forest\", \"look\": \"neo\"}}%%\npie";
        let config = read(text).expect("reads");
        assert_eq!(config, json!({ "theme": "forest", "look": "neo" }));
    }

    #[test]
    fn invalid_json_drops_every_directive() {
        let text = "%%{init: {\"theme\": \"forest\"}}%%\n%%{init: {theme: dark}}%%\npie";
        assert_eq!(read(text).expect("reads"), json!({}));
    }

    #[test]
    fn wrap_directives() {
        assert_eq!(
            read("%%{wrap}%%\npie").expect("reads"),
            json!({ "wrap": true })
        );
        assert_eq!(
            read("%%{wrap}%%\n%%{wrap}%%\npie").expect("reads"),
            json!({ "wrap": true })
        );
        assert_eq!(read("%%{wrapper}%%\npie").expect("reads"), json!({}));
    }

    #[test]
    fn a_directive_without_arguments_adds_nothing() {
        assert_eq!(read("%%{init}%%\npie").expect("reads"), json!({}));
    }

    #[test]
    fn directives_inside_the_diagram_count() {
        let text = "pie\n%%{init: {\"theme\": \"dark\"}}%%\n\"a\": 1";
        assert_eq!(read(text).expect("reads"), json!({ "theme": "dark" }));
    }

    #[test]
    fn display_mode_goes_to_gantt() {
        let config = read("---\ndisplayMode: compact\n---\npie").expect("reads");
        assert_eq!(config, json!({ "gantt": { "displayMode": "compact" } }));
    }

    #[test]
    fn invalid_front_matter_yaml_is_an_error() {
        assert!(read("---\nconfig: [\n---\npie").is_err());
    }

    #[test]
    fn comment_lines_with_directive_closers_are_removed() {
        assert_eq!(remove_comment_lines("%%}%%x\npie"), "pie");
    }
}
