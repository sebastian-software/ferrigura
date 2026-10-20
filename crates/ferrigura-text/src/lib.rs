//! Text measurement for ferrigura labels, with the fonts bundled in the crate.
//!
//! The crate answers the questions a diagram layout asks about its labels: how wide a run of
//! text is in a given face and size, where the text box of a line lies, how Mermaid wraps a
//! label, and how to embed the glyphs a diagram uses as a font subset. See
//! `docs-port/decisions/0005-text-measurement-and-fonts.md` for the choices behind it.
//!
//! The registry in [`fonts`] maps every (family, weight, style) choice to exactly one bundled
//! face. Lexica Ultralegible covers Latin scripts only, so the arrows (U+2190-21FF) are set in
//! the bundled symbol face, Noto Sans Math, character by character, as a browser does with a
//! font-family list ([`fallback_runs`]). Other scripts need fallback fonts that are not part of
//! this crate yet.

#![warn(missing_docs)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod bbox;
pub mod embed;
mod error;
pub mod fonts;
pub mod measure;
pub mod metrics;
pub mod wrap;

pub use bbox::{Extent, measure_box, measure_line_box};
pub use embed::{EmbeddedFamily, cut_font, embed_runs, subset_face};
pub use error::TextError;
pub use fonts::{Face, FontFamily, FontStyle, FontWeight, face};
pub use measure::{Run, fallback_runs, layout_width, measure, measure_data, measure_line};
pub use metrics::{FontMetrics, TextBox, font_metrics, text_box, x_height};
pub use wrap::{
    Word, fit_line, measure_words, split_line_to_fit_width, split_line_to_words,
    split_text_to_chars, split_word_to_fit_width, wrap_line,
};
