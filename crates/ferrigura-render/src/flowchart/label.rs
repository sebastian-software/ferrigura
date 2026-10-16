//! Labels as SVG text: a port of `createText` in `packages/mermaid/src/rendering-util/createText.ts`
//! on its `htmlLabels: false` path, with `handle-markdown-text.ts` and `splitText.ts`.
//!
//! A label is split into lines of words, each word in its own face (bold or italic from Markdown or
//! the label's style), wrapped at the given width as the reference wraps, and measured with
//! `ferrigura-text`. The SVG keeps the reference's structure (one `tspan.text-outer-tspan` per line,
//! one `tspan.text-inner-tspan` per word) but places every run at an explicit `x` with
//! `text-anchor` start (`docs-port/decisions/0004` and `0005`). See
//! `docs-port/flowchart/render-notes/labels.md`.

use ferrigura_model::flowchart::LabelType;
use ferrigura_text::{FontFamily, FontStyle, FontWeight, Run, face};

use super::geom::BBox;
use crate::RenderError;

/// What to lay out: the arguments of `createText`.
#[derive(Debug, Clone)]
pub struct LabelSpec<'a> {
    /// The label text, as the model holds it.
    pub text: &'a str,
    /// How the text is written: plain, a string, or Markdown.
    pub label_type: LabelType,
    /// The width at which lines wrap; `f64::INFINITY` never wraps.
    pub width: f64,
    /// `createText`'s `isNode`: node labels are centred by the stylesheet in the reference, the
    /// others by `text-anchor="middle"`. Both are centred on `x = 0`.
    pub is_node: bool,
    /// Whether a background rectangle is drawn behind the text (edge labels).
    pub add_background: bool,
    /// The label's style declarations (`key:value`), from which the font size, weight and style
    /// are read.
    pub styles: &'a [String],
    /// The font size in pixels when the style does not set one.
    pub font_size: f64,
}

/// One word or styled run of a line, placed.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelRun {
    /// The text, with the leading space the reference gives every word after the first.
    pub text: String,
    /// The weight.
    pub weight: FontWeight,
    /// The style.
    pub style: FontStyle,
    /// The x of the run's start, in the label's coordinates.
    pub x: f64,
    /// The measured width.
    pub width: f64,
}

/// One line of a label.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelLine {
    /// The runs, left to right.
    pub runs: Vec<LabelRun>,
    /// The width of the line.
    pub width: f64,
    /// The baseline's y, in the label's coordinates.
    pub baseline: f64,
}

/// A laid-out label.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelLayout {
    /// The lines, top to bottom.
    pub lines: Vec<LabelLine>,
    /// The box `getBBox` gives for the label's `<text>`, in the label's coordinates.
    pub bbox: BBox,
    /// The background rectangle, when the spec asked for one: `bbox` padded by 2 on every side.
    pub background: Option<BBox>,
    /// The font size of the text.
    pub font_size: f64,
}

impl LabelLayout {
    /// A label without text that measures `bbox`, for tests of the code that consumes labels.
    #[must_use]
    pub fn of_box(bbox: BBox) -> Self {
        Self {
            lines: Vec::new(),
            bbox,
            background: None,
            font_size: 16.0,
        }
    }

    /// The runs of the label, to subset the fonts for.
    #[must_use]
    pub fn text_runs(&self) -> Vec<Run<'_>> {
        self.lines
            .iter()
            .flat_map(|line| &line.runs)
            .map(|run| Run {
                text: &run.text,
                face: face(FontFamily::Sans, run.weight, run.style),
            })
            .collect()
    }

    /// The label's SVG: the `<g>` that `createFormattedText` returns, with the background
    /// rectangle and the text. `text_style` is the inline style of the `<text>` element.
    #[must_use]
    pub fn svg(&self, text_style: &str) -> String {
        let _ = text_style;
        todo!("write the label SVG (task T2)")
    }
}

/// Lays out a label as `createText` does.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read.
pub fn layout(spec: &LabelSpec<'_>) -> Result<LabelLayout, RenderError> {
    let _ = spec;
    todo!("port createText (task T2)")
}
