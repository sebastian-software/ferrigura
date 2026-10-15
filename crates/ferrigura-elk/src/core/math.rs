//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.core/src/org/eclipse/elk/core/math/KVector.java
//! Source: plugins/org.eclipse.elk.core/src/org/eclipse/elk/core/math/Spacing.java
//!
//! Geometry types of ELK core, `org.eclipse.elk.core.math`.
//!
//! Ports `KVector` (the parts layout needs) and the `Spacing` family behind
//! `ElkMargin` and `ElkPadding`. Parsing follows the Java methods exactly,
//! including the errors that `LayoutOptionData.parseValue` turns into
//! "option not set".

use std::fmt;

/// `KVector`: a two-dimensional vector.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct KVector {
    pub x: f64,
    pub y: f64,
}

/// Why a string could not be parsed as an object option value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    message: String,
}

impl ParseError {
    fn new(message: impl Into<String>) -> Self {
        ParseError {
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ParseError {}

/// Characters that `KVector.parse` and `Spacing.parse` strip at both ends.
const OPENING: &str = "([{\"' \t\r\n";
const CLOSING: &str = ")]}\"' \t\r\n";

fn trim_delimiters(text: &str) -> &str {
    let start = text
        .char_indices()
        .find(|(_, c)| !OPENING.contains(*c))
        .map_or(text.len(), |(index, _)| index);
    let rest = &text[start..];
    let end = rest
        .char_indices()
        .rev()
        .find(|(_, c)| !CLOSING.contains(*c))
        .map_or(0, |(index, c)| index + c.len_utf8());
    &rest[..end]
}

/// `Double.parseDouble` on a trimmed token: decimal numbers and the Java
/// spellings `NaN` and `Infinity`.
pub fn parse_java_double(token: &str) -> Option<f64> {
    match token {
        "NaN" => Some(f64::NAN),
        "Infinity" | "+Infinity" => Some(f64::INFINITY),
        "-Infinity" => Some(f64::NEG_INFINITY),
        _ if token.is_empty()
            || token.contains(|c: char| c.is_ascii_alphabetic() && c != 'e' && c != 'E') =>
        {
            None
        }
        _ => token.parse().ok(),
    }
}

impl KVector {
    /// `KVector.parse`: exactly two numbers, separated by `,` `;` or a line break.
    pub fn parse(text: &str) -> Result<KVector, ParseError> {
        let body = trim_delimiters(text);
        if body.is_empty() {
            return Err(ParseError::new(
                "The given string does not contain any numbers.",
            ));
        }
        let tokens: Vec<&str> = split_java(body, &[',', ';', '\r', '\n']);
        if tokens.len() != 2 {
            return Err(ParseError::new(format!(
                "Exactly two numbers are expected, {} were found.",
                tokens.len()
            )));
        }
        let x = parse_java_double(tokens[0].trim());
        let y = parse_java_double(tokens[1].trim());
        match (x, y) {
            (Some(x), Some(y)) => Ok(KVector { x, y }),
            _ => Err(ParseError::new(
                "The given string contains parts that cannot be parsed as numbers.",
            )),
        }
    }
}

/// The four sides of `ElkMargin` or `ElkPadding` (both extend `Spacing`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Spacing {
    pub top: f64,
    pub left: f64,
    pub bottom: f64,
    pub right: f64,
}

impl Spacing {
    /// `Spacing.parse`: a list such as `[top=12,left=12,bottom=12,right=12]`.
    /// Keys not listed keep their zero default; unknown keys are ignored.
    pub fn parse(text: &str) -> Result<Spacing, ParseError> {
        let mut spacing = Spacing::default();
        let body = trim_delimiters(text);
        if body.is_empty() {
            return Ok(spacing);
        }
        for token in split_java(body, &[',', ';']) {
            let parts = split_java(token, &['=']);
            if parts.len() != 2 {
                return Err(ParseError::new("Expecting a list of key-value pairs."));
            }
            let key = parts[0].trim();
            let value = parse_java_double(parts[1].trim()).ok_or_else(|| {
                ParseError::new("The given string contains parts that cannot be parsed as numbers.")
            })?;
            match key {
                "top" => spacing.top = value,
                "left" => spacing.left = value,
                "bottom" => spacing.bottom = value,
                "right" => spacing.right = value,
                _ => {}
            }
        }
        Ok(spacing)
    }
}

/// `String.split(regex)` for a set of single-character separators: empty
/// trailing tokens are dropped, as Java does.
fn split_java<'a>(text: &'a str, separators: &[char]) -> Vec<&'a str> {
    let mut tokens: Vec<&str> = text.split(|c| separators.contains(&c)).collect();
    while tokens.last().is_some_and(|token| token.is_empty()) {
        tokens.pop();
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vectors_parse_like_java() {
        assert_eq!(
            KVector::parse("(50, 0)").unwrap(),
            KVector { x: 50.0, y: 0.0 }
        );
        assert!(KVector::parse("(50)").is_err());
        assert!(KVector::parse("").is_err());
    }

    #[test]
    fn spacings_parse_the_listed_sides_only() {
        let margin = Spacing::parse("[top=12,left=12,bottom=12,right=12]").unwrap();
        assert_eq!(margin.top, 12.0);
        assert_eq!(margin.right, 12.0);
        let partial = Spacing::parse("[top=24]").unwrap();
        assert_eq!(
            partial,
            Spacing {
                top: 24.0,
                ..Spacing::default()
            }
        );
        assert!(Spacing::parse("[top]").is_err());
    }
}
