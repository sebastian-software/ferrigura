//! Themes of ferrigura: `redux-color` (light) and `redux-dark-color` (dark), Mermaid's current
//! defaults, with their derived colours computed here, as `docs-port/decisions/0007` decides.
//!
//! A theme gives the `themeVariables` of the configuration: the colour tokens, the font settings and
//! the per-diagram values. The reference computes them in `packages/mermaid/src/themes/` with the
//! colour functions of `khroma`; the port of those functions is in [`khroma`].

use std::fmt;

use crate::value::Value;

pub mod khroma;

/// A theme that ferrigura ships.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    /// `redux-color`, the light theme.
    ReduxColor,
    /// `redux-dark-color`, the dark theme.
    ReduxDarkColor,
}

/// Why the variables of a theme could not be computed. The reference throws in the same cases: a
/// colour that the colour functions cannot read, such as `unreadable`, or a value that is not a
/// string where a colour is needed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeError {
    message: String,
}

impl ThemeError {
    /// An error with the given message.
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// What went wrong, as the reference words it.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ThemeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ThemeError {}

impl Theme {
    /// The theme for a configured `theme` name. A name that ferrigura does not ship falls back to a
    /// theme by its darkness: names ending in `dark` take the dark theme, everything else the light one.
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        if name.ends_with("dark") {
            Self::ReduxDarkColor
        } else {
            Self::ReduxColor
        }
    }

    /// The `themeVariables` of the theme, with the user's overrides applied as the reference's
    /// `applyOverride` does. Placeholder until the theme port lands.
    #[must_use]
    pub fn variables(self, overrides: Option<&Value>) -> Value {
        let _ = (self, overrides);
        Value::empty_object()
    }
}
