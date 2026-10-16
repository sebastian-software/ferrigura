//! Text measurement for ferrigura labels, with the fonts bundled in the crate.
//!
//! The crate answers the questions a diagram layout asks about its labels: how wide a run of
//! text is in a given face and size, where the text box of a line lies, how Mermaid wraps a
//! label, and how to embed the glyphs a diagram uses as a font subset. See
//! `docs-port/decisions/0005-text-measurement-and-fonts.md` for the choices behind it.
//!
//! The registry in [`fonts`] maps every (family, weight, style) choice to exactly one bundled
//! face. Lexica Ultralegible covers Latin scripts only; other scripts need fallback fonts that
//! are not part of this crate yet.

#![warn(missing_docs)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod fonts;

pub use fonts::{Face, FontFamily, FontStyle, FontWeight, face};
