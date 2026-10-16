//! Scanner helpers shared by the lexers that follow Jison's rules: the start-condition stack and
//! the matching of literals and runs of characters.
//!
//! Jison tries the rules of a lexer in the order they are written and takes the first one that
//! matches, not the longest. A literal keyword gets an implicit word boundary when it ends in a
//! word character. The helpers return byte lengths, so a rule table reads like the grammar.

use crate::text::{is_js_space, is_word};

/// The start conditions of a lexer: a base state and the states pushed on top of it. Popping the
/// base state is a no-op, as in Jison.
#[derive(Debug, Clone)]
pub(crate) struct StateStack<S: Copy> {
    base: S,
    pushed: Vec<S>,
}

impl<S: Copy> StateStack<S> {
    /// A stack holding only `base`.
    pub(crate) fn new(base: S) -> Self {
        Self {
            base,
            pushed: Vec::new(),
        }
    }

    /// The state on top of the stack.
    pub(crate) fn top(&self) -> S {
        self.pushed.last().copied().unwrap_or(self.base)
    }

    /// Enters `state` on top of the current one.
    pub(crate) fn push(&mut self, state: S) {
        self.pushed.push(state);
    }

    /// Leaves the current state; the base state stays.
    pub(crate) fn pop(&mut self) {
        self.pushed.pop();
    }
}

/// Byte length of `literal` if `rest` starts with it.
pub(crate) fn literal_len(rest: &str, literal: &str) -> Option<usize> {
    rest.starts_with(literal).then_some(literal.len())
}

/// Byte length of `word` if `rest` starts with it and no word character follows, as Jison's
/// implicit `\b` after a keyword literal.
pub(crate) fn word_len(rest: &str, word: &str) -> Option<usize> {
    let after = rest.get(word.len()..)?;
    (rest.starts_with(word) && !after.chars().next().is_some_and(is_word)).then_some(word.len())
}

/// Byte length of the run of whitespace at the start of `rest` (JavaScript `\s`).
pub(crate) fn space_len(rest: &str) -> usize {
    rest.char_indices()
        .find(|&(_, c)| !is_js_space(c))
        .map_or(rest.len(), |(index, _)| index)
}

/// Byte length of `word`, optional whitespace, the `delimiter` character, and optional whitespace:
/// `word\s*"delimiter"\s*`.
pub(crate) fn delimited_form_len(rest: &str, word: &str, delimiter: char) -> Option<usize> {
    let after_word = rest.strip_prefix(word)?;
    let spaces = space_len(after_word);
    let after_spaces = &after_word[spaces..];
    let after_delimiter = after_spaces.strip_prefix(delimiter)?;
    let trailing = space_len(after_delimiter);
    Some(word.len() + spaces + delimiter.len_utf8() + trailing)
}

/// Byte length up to, not including, the next line feed.
pub(crate) fn line_len(rest: &str) -> usize {
    rest.find('\n').unwrap_or(rest.len())
}
