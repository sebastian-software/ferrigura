//! Themes of ferrigura: `redux-color` (light) and `redux-dark-color` (dark), Mermaid's current
//! defaults, with their derived colours computed here, as `docs-port/decisions/0007` decides.
//!
//! A theme gives the `themeVariables` of the configuration: the colour tokens, the font settings and
//! the per-diagram values. The reference computes them in `packages/mermaid/src/themes/` with the
//! colour functions of `khroma`; the port of those functions is in [`khroma`].

use std::borrow::Cow;
use std::fmt;
use std::sync::LazyLock;

use crate::value::Value;

pub use tokens::{Family, ThemeTokens, Token};

mod helpers;
pub mod khroma;
mod redux_color;
mod redux_dark_color;
pub mod tokens;

/// The result of computing the variables of a theme.
pub(crate) type Res<T> = Result<T, ThemeError>;

/// The variables of `redux-color` without overrides, computed once per process.
static REDUX_COLOR: LazyLock<Res<Value>> = LazyLock::new(|| redux_color::variables(None));

/// The variables of `redux-dark-color` without overrides, computed once per process.
static REDUX_DARK_COLOR: LazyLock<Res<Value>> = LazyLock::new(|| redux_dark_color::variables(None));

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
    /// The theme for a configured `theme` name: `redux-color` and `redux-dark-color` are themselves.
    /// A name that ferrigura does not ship falls back to a theme by its darkness: names ending in
    /// `dark` take the dark theme, everything else the light one.
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        match name {
            "redux-color" => Self::ReduxColor,
            "redux-dark-color" => Self::ReduxDarkColor,
            _ if name.ends_with("dark") => Self::ReduxDarkColor,
            _ => Self::ReduxColor,
        }
    }

    /// The `themeVariables` of the theme, with the user's overrides applied as the reference's
    /// `applyOverride` does. The overrides are an object; `None` means there are none. When a colour
    /// cannot be read this gives an empty object, and [`Theme::try_variables`] gives the error.
    #[must_use]
    pub fn variables(self, overrides: Option<&Value>) -> Value {
        self.variables_shared(overrides).into_owned()
    }

    /// The `themeVariables` of the theme, as [`Theme::variables`] gives them, borrowed from the copy
    /// that is computed once per process when there are no overrides to apply. An empty object gives
    /// the same variables as none: the theme's own.
    #[must_use]
    pub fn variables_shared(self, overrides: Option<&Value>) -> Cow<'static, Value> {
        self.shared(overrides)
            .unwrap_or_else(|_| Cow::Owned(Value::empty_object()))
    }

    /// The `themeVariables` of the theme, as [`Theme::variables`] gives them, or the error when a
    /// colour cannot be read.
    ///
    /// # Errors
    ///
    /// Returns a [`ThemeError`] when a colour of the theme or of the overrides is not one that the
    /// colour functions read.
    pub fn try_variables(self, overrides: Option<&Value>) -> Result<Value, ThemeError> {
        self.shared(overrides).map(Cow::into_owned)
    }

    /// The variables of the theme: the copy computed once per process without overrides, and a new
    /// computation with them.
    fn shared(self, overrides: Option<&Value>) -> Result<Cow<'static, Value>, ThemeError> {
        if has_overrides(overrides) {
            return self.compute(overrides).map(Cow::Owned);
        }
        match self.base() {
            Ok(variables) => Ok(Cow::Borrowed(variables)),
            Err(error) => Err(error.clone()),
        }
    }

    /// The variables of the theme without overrides, computed once per process.
    fn base(self) -> &'static Res<Value> {
        match self {
            Self::ReduxColor => &REDUX_COLOR,
            Self::ReduxDarkColor => &REDUX_DARK_COLOR,
        }
    }

    /// The variables of the theme, computed with the overrides.
    fn compute(self, overrides: Option<&Value>) -> Res<Value> {
        match self {
            Self::ReduxColor => redux_color::variables(overrides),
            Self::ReduxDarkColor => redux_dark_color::variables(overrides),
        }
    }
}

/// Whether the overrides change a variable. Without any, the theme's variables are its own:
/// `calculate` gives the same for `None`, an empty object and an empty array.
fn has_overrides(overrides: Option<&Value>) -> bool {
    match overrides {
        Some(Value::Object(map)) => !map.is_empty(),
        Some(Value::Array(items)) => !items.is_empty(),
        _ => false,
    }
}
