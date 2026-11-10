//! Text measurement for ferrigura labels, with local registered or bundled fonts.
//!
//! The crate answers the questions a diagram layout asks about its labels: how wide a run of
//! text is in a given face and size, where the text box of a line lies, how Mermaid wraps a
//! label, and how to embed the glyphs a diagram uses as a font subset. See
//! `docs/decisions/0010-text-measurement-and-fonts.md` for the choices behind it.
//!
//! [`FontRegistry`] owns host-supplied TTF or OTF files and resolves CSS family lists, weights,
//! styles, and explicit per-character fallback. Resolved faces are shared owned handles; runs
//! borrow them without requiring static storage. Missing glyphs can be reported through
//! [`FontRegistry::missing_glyphs`]. Registration never discovers system fonts or downloads files.
//!
//! The compatibility selector [`face`] still supplies the diagram renderer's bundled Lexica
//! Ultralegible and Atkinson Hyperlegible Mono faces, with Noto symbols and Twemoji fallback.
//! Their licenses and sources are in `NOTICE.md`.

#![warn(missing_docs)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod bbox;
pub mod embed;
mod error;
pub mod fonts;
mod kern;
pub mod measure;
pub mod metrics;
pub mod registry;
pub mod wrap;

pub use bbox::{Extent, measure_box, measure_line_box};
pub use embed::{EmbeddedFamily, cut_font, embed_runs, subset_face};
pub use error::TextError;
pub use fonts::{Face, FontFamily, FontStyle, FontWeight, face};
pub use measure::{
    Run, fallback_faces, fallback_runs, layout_width, measure, measure_data, measure_line,
};
pub use metrics::{FontMetrics, TextBox, font_metrics, text_box, x_height};
pub use registry::{FontRegistry, MissingGlyph};
pub use wrap::{
    Word, fit_line, measure_words, split_line_to_fit_width, split_line_to_words,
    split_text_to_chars, split_word_to_fit_width, wrap_line,
};
