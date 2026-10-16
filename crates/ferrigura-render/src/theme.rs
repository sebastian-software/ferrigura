//! The themes a diagram is drawn with: one set of theme variables, or a light and a dark set
//! (`docs-port/decisions/0007`). Shared by every renderer.

use ferrigura_model::value::Value;

/// The themes a diagram is drawn with.
#[derive(Debug, Clone, Copy)]
pub enum Themes<'a> {
    /// One set of variables, written as it is.
    Single(&'a Value),
    /// A light and a dark set. The light set is written; the dark set applies under
    /// `prefers-color-scheme: dark`, so one SVG follows the viewer's scheme.
    Schemes {
        /// The variables of the light scheme.
        light: &'a Value,
        /// The variables of the dark scheme.
        dark: &'a Value,
    },
}

impl Themes<'_> {
    /// The set that gives the presentation attributes and the sizes: the single set, or the light one.
    pub(crate) fn primary(&self) -> &Value {
        match self {
            Self::Single(variables) => variables,
            Self::Schemes { light, .. } => light,
        }
    }
}
