//! `ClassMember` of `classTypes.ts` (the text of an attribute or a method) and `parseGenericTypes`
//! of `common.ts` (`~T~` written as `<T>`).
//!
//! The method pattern of the reference, `([#+~-])?(.+)\((.*)\)([\s$*])?(.*)([$*])?`, is matched
//! here by hand. Its greedy groups fix the match: it starts at the first character, and group 2
//! ends at the last `(` that has a `)` after it; group 3 ends at the last `)`; group 5 takes the
//! rest, so the trailing classifier of group 6 is never set. Line terminators other than `\n`
//! (U+2028, U+2029, `\r`) are not excluded from `.`: they cannot reach a member (line breaks are
//! normalised, and the grammar's MEMBER rule stops at `\n`).

use crate::text::{is_js_space, js_trim};

/// The visibility characters the reference accepts; an empty visibility is also one of them.
const VISIBILITY: &[char] = &['#', '+', '~', '-'];

/// The parsed fields of a member, before the DB stores it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Member {
    pub is_method: bool,
    pub visibility: String,
    pub classifier: String,
    pub id: String,
    pub parameters: Option<String>,
    pub return_type: Option<String>,
    pub text: String,
}

/// `ClassMember`'s constructor and `parseMember` on the sanitised text of a member. Returns `None`
/// where the reference throws: a method whose text has a `)` but no `(` before it, so that the
/// method pattern does not match and the name is left undefined (`this.id.startsWith` throws).
pub(crate) fn parse_member(input: &str, is_method: bool) -> Option<Member> {
    if is_method {
        parse_method(input)
    } else {
        Some(parse_attribute(input))
    }
}

/// The method branch: the pattern, the trimming of its groups, and the classifier that the return
/// type may end with.
fn parse_method(input: &str) -> Option<Member> {
    let parts = method_parts(input)?;
    let visibility = parts.visibility.map(String::from).unwrap_or_default();
    let mut classifier = parts
        .classifier
        .map(|c| c.to_string().trim().to_owned())
        .unwrap_or_default();
    let parameters = js_trim(parts.parameters).to_owned();
    let mut return_type = js_trim(parts.return_type).to_owned();
    if classifier.is_empty()
        && let Some(last) = return_type.chars().last()
        && (last == '$' || last == '*')
    {
        classifier = last.to_string();
        return_type.pop();
    }
    let id = trim_keeping_one_leading_space(parts.id);
    let text = member_text(
        &visibility,
        &id,
        Some(&parameters),
        (!return_type.is_empty()).then_some(return_type.as_str()),
        true,
    );
    Some(Member {
        is_method: true,
        visibility,
        classifier,
        id,
        parameters: Some(parameters),
        return_type: Some(return_type),
        text,
    })
}

/// The attribute branch: a visibility from the first character and a classifier from the last,
/// and the name between them.
fn parse_attribute(input: &str) -> Member {
    let first = input.chars().next();
    let last = input.chars().last();
    let visibility = match first {
        Some(c) if VISIBILITY.contains(&c) => c.to_string(),
        _ => String::new(),
    };
    let classifier = match last {
        Some(c) if c == '$' || c == '*' => c.to_string(),
        _ => String::new(),
    };
    let start = if visibility.is_empty() { 0 } else { 1 };
    let end = if classifier.is_empty() {
        input.len()
    } else {
        input.len() - 1
    };
    let raw_id = input.get(start..end).unwrap_or("");
    let id = trim_keeping_one_leading_space(raw_id);
    let text = member_text(&visibility, &id, None, None, false);
    Member {
        is_method: false,
        visibility,
        classifier,
        id,
        parameters: None,
        return_type: None,
        text,
    }
}

/// `this.id = this.id.startsWith(' ') ? ' ' + this.id.trim() : this.id.trim()`.
fn trim_keeping_one_leading_space(id: &str) -> String {
    if id.starts_with(' ') {
        format!(" {}", js_trim(id))
    } else {
        js_trim(id).to_owned()
    }
}

/// The display text of a member: the visibility escaped with a backslash, the name with its
/// generic types, and for a method `(parameters) : type`, with `<` and `>` written as `&lt;` and
/// `&gt;`. A text that starts with an escaped `&lt;` becomes `~`.
fn member_text(
    visibility: &str,
    id: &str,
    parameters: Option<&str>,
    return_type: Option<&str>,
    is_method: bool,
) -> String {
    let mut combined = String::new();
    if !visibility.is_empty() {
        combined.push('\\');
        combined.push_str(visibility);
    }
    combined.push_str(&parse_generic_types(id));
    if is_method {
        combined.push('(');
        combined.push_str(&parse_generic_types(parameters.unwrap_or("")));
        combined.push(')');
        if let Some(return_type) = return_type {
            combined.push_str(" : ");
            combined.push_str(&parse_generic_types(return_type));
        }
    }
    let text = combined.replace('<', "&lt;").replace('>', "&gt;");
    match text.strip_prefix("\\&lt;") {
        Some(rest) => format!("~{rest}"),
        None => text,
    }
}

/// The groups of the method pattern.
struct MethodParts<'a> {
    visibility: Option<char>,
    id: &'a str,
    parameters: &'a str,
    classifier: Option<char>,
    return_type: &'a str,
}

/// Matches `([#+~-])?(.+)\((.*)\)([\s$*])?(.*)([$*])?` on `input` as the reference does: the
/// match starts at 0 when it matches at all, and each group is as greedy as the pattern allows.
fn method_parts(input: &str) -> Option<MethodParts<'_>> {
    let bytes = input.as_bytes();
    let opens: Vec<usize> = bytes
        .iter()
        .enumerate()
        .filter(|&(_, &b)| b == b'(')
        .map(|(index, _)| index)
        .collect();
    let has_close_after = |open: usize| bytes[open + 1..].contains(&b')');
    // The visibility group takes the first character if it can, and the name then starts at 1.
    let with_visibility = bytes
        .first()
        .filter(|&&b| matches!(b, b'#' | b'+' | b'~' | b'-'))
        .and_then(|_| {
            opens
                .iter()
                .rev()
                .find(|&&open| open >= 2 && has_close_after(open))
        });
    let (visibility, start, open) = match with_visibility {
        Some(&open) => (Some(bytes[0] as char), 1, open),
        None => {
            let open = *opens
                .iter()
                .rev()
                .find(|&&open| open >= 1 && has_close_after(open))?;
            (None, 0, open)
        }
    };
    let close = bytes.iter().rposition(|&b| b == b')')?;
    if close <= open {
        return None;
    }
    let id = input.get(start..open)?;
    let parameters = input.get(open + 1..close)?;
    let mut rest_start = close + 1;
    let classifier = match input.get(rest_start..).and_then(|rest| rest.chars().next()) {
        Some(c) if is_js_space(c) || c == '$' || c == '*' => {
            rest_start += c.len_utf8();
            Some(c)
        }
        _ => None,
    };
    let return_type = input.get(rest_start..)?;
    Some(MethodParts {
        visibility,
        id,
        parameters,
        classifier,
        return_type,
    })
}

/// `parseGenericTypes` (`common.ts`): the input is split at commas, and two neighbouring pieces
/// that each have one `~` are joined again; each piece is then processed by `processSet`.
pub(crate) fn parse_generic_types(input: &str) -> String {
    let mut items: Vec<String> = Vec::new();
    for (index, part) in input.split(',').enumerate() {
        if index > 0 {
            items.push(",".to_owned());
        }
        items.push(part.to_owned());
    }
    let mut output: Vec<String> = Vec::new();
    let mut index = 0;
    while index < items.len() {
        let mut this = items[index].clone();
        if this == "," && index > 0 && index + 1 < items.len() {
            let previous = &items[index - 1];
            let next = &items[index + 1];
            if tilde_count(previous) == 1 && tilde_count(next) == 1 {
                this = format!("{previous},{next}");
                index += 1;
                output.pop();
            }
        }
        output.push(process_set(&this));
        index += 1;
    }
    output.concat()
}

/// `countOccurrence(string, '~')`.
fn tilde_count(text: &str) -> usize {
    text.matches('~').count()
}

/// `processSet` (`common.ts`): the pairs of `~` become `<` and `>`, a lone leading `~` is kept.
fn process_set(input: &str) -> String {
    let tildes = tilde_count(input);
    if tildes <= 1 {
        return input.to_owned();
    }
    let mut text = input;
    let mut has_starting_tilde = false;
    if tildes % 2 != 0 && text.starts_with('~') {
        text = &text[1..];
        has_starting_tilde = true;
    }
    let mut chars: Vec<char> = text.chars().collect();
    let mut first = chars.iter().position(|&c| c == '~');
    let mut last = chars.iter().rposition(|&c| c == '~');
    while let (Some(low), Some(high)) = (first, last) {
        if low == high {
            break;
        }
        chars[low] = '<';
        chars[high] = '>';
        first = chars.iter().position(|&c| c == '~');
        last = chars.iter().rposition(|&c| c == '~');
    }
    let mut out = String::with_capacity(input.len());
    if has_starting_tilde {
        out.push('~');
    }
    out.extend(chars);
    out
}
