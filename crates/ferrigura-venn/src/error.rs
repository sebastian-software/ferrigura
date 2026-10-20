//! The reasons the library does not lay a diagram out.

use std::fmt;

/// Why a call does not return a layout. Each variant is a case where the JavaScript library throws,
/// or where the port refuses an input it does not reproduce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// There are no areas, so there is no set to start the layout from (the library throws a
    /// TypeError reading the first of no sets).
    NoAreas,
    /// An area has no set.
    EmptyArea,
    /// An area names a set that has no single-set area of its own, so the library has no circle
    /// for it (it throws a TypeError).
    UnknownSet(String),
    /// A set is named `__proto__`. Assigning that name in a plain object changes the object's
    /// prototype rather than adding a key, so the library's behaviour is not reproduced.
    ReservedName,
    /// A distance cannot be bracketed: the library's `bisect` throws "Initial bisect points must
    /// have opposite signs". This happens for a negative overlap.
    NoBracket,
    /// The greedy layout finds no positioned set to place a set against. The library throws
    /// "ERROR: missing pairwise overlap information". `addMissingAreas` makes this unreachable for
    /// valid input.
    MissingPairwiseOverlap,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NoAreas => write!(f, "there are no areas to lay out"),
            Error::EmptyArea => write!(f, "an area has no set"),
            Error::UnknownSet(set) => write!(f, "the set {set:?} has no single-set area"),
            Error::ReservedName => write!(f, "a set named __proto__ is not supported"),
            Error::NoBracket => write!(
                f,
                "Initial bisect points must have opposite signs (a negative overlap)"
            ),
            Error::MissingPairwiseOverlap => {
                write!(f, "ERROR: missing pairwise overlap information")
            }
        }
    }
}

impl std::error::Error for Error {}
