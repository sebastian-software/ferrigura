//! The reasons the layout is not computed.

use std::fmt;

/// Why [`crate::run_swimlane_layout_core`] does not lay a diagram out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The input uses a non-default option, which the port does not implement yet:
    /// `ignoreCrossLaneEdges: false` (the gravity layering) or `automaticLaneOrdering: true`.
    NonDefaultOption(&'static str),
    /// A node or edge id outside ASCII. The collation of the layout is ICU's root order for ASCII
    /// only; ids outside it need an ICU collator, which is not ported.
    NonAsciiId(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NonDefaultOption(option) => write!(
                f,
                "the swimlane option {option} is not ported; only the default layout is"
            ),
            Error::NonAsciiId(id) => write!(
                f,
                "the id {id:?} is not ASCII; the layout collates ASCII ids only"
            ),
        }
    }
}

impl std::error::Error for Error {}
