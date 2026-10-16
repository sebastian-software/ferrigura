//! Labels as SVG text: a port of `createText` in `packages/mermaid/src/rendering-util/createText.ts`
//! on its `htmlLabels: false` path, with `handle-markdown-text.ts` and `splitText.ts`.
//!
//! A label is split into lines of words, each word in its own face (bold or italic from Markdown or
//! the label's style), wrapped at the given width as the reference wraps, and measured with
//! `ferrigura-text`. The SVG keeps the reference's structure (one `tspan.text-outer-tspan` per line,
//! one `tspan.text-inner-tspan` per word) but places every run at an explicit `x` with
//! `text-anchor` start (`docs-port/decisions/0004` and `0005`). See
//! `docs-port/flowchart/render-notes/labels.md`.

use std::fmt::Write as _;

use ferrigura_model::flowchart::LabelType;
use ferrigura_text::{
    Face, FontFamily, FontStyle, FontWeight, Run, TextError, Word, face, fit_line, measure,
    measure_words, text_box,
};

use super::geom::BBox;
use super::markdown::{MarkdownWordType, br_tag_length, markdown_to_lines, non_markdown_to_lines};
use crate::RenderError;
use crate::svg::{escape, number};

/// The line height of labels, in ems: `lh` in `createFormattedText`.
const LINE_HEIGHT: f64 = 1.1;

/// The padding of the background rectangle on each side, in pixels: `padding` in
/// `createFormattedText`.
const BACKGROUND_PADDING: f64 = 2.0;

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
        let mut out = String::from("<g>");
        if let Some(background) = self.background {
            out.push_str(&format!(
                r#"<rect class="background" style="stroke: none" x="{}" y="{}" width="{}" height="{}"></rect>"#,
                number(background.x),
                number(background.y),
                number(background.width),
                number(background.height),
            ));
        }
        out.push_str(&format!(
            r#"<text y="-10.1" text-anchor="start" style="{}">"#,
            escape(text_style)
        ));
        for line in &self.lines {
            // Every line starts at its own x, with `text-anchor` start, so the browser never
            // aligns it by its own measure (decision 0005).
            out.push_str(&format!(
                r#"<tspan class="text-outer-tspan row" x="{}" y="{}" text-anchor="start">"#,
                number(-line.width / 2.0),
                number(line.baseline),
            ));
            for run in &line.runs {
                let weight = match run.weight {
                    FontWeight::Normal => "normal",
                    FontWeight::Bold => "bold",
                };
                let _ = write!(
                    out,
                    r#"<tspan font-style="{}" class="text-inner-tspan" font-weight="{weight}" x="{}">{}</tspan>"#,
                    run.style.css_keyword(),
                    number(run.x),
                    escape(&run.text),
                );
            }
            out.push_str("</tspan>");
        }
        out.push_str("</text></g>");
        out
    }
}

/// Lays out a label as `createText` does.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read.
pub fn layout(spec: &LabelSpec<'_>) -> Result<LabelLayout, RenderError> {
    // The style's font size sets the text. A node label is wrapped inside the group that carries
    // its style, so the wrap sees the size; an edge label is wrapped in its unstyled container.
    let font_size = declared_font_size(spec.styles).unwrap_or(spec.font_size);
    let wrap_size = if spec.is_node {
        font_size
    } else {
        spec.font_size
    };

    // `createText`: `<br>` tags are normalised, then the entity placeholders are decoded.
    let text = decode_entities(&normalize_breaks(spec.text));
    let lines = match spec.label_type {
        LabelType::Markdown => markdown_to_lines(&text),
        LabelType::Text | LabelType::String => non_markdown_to_lines(&text),
    };

    let width = spec.width;
    let mut wrapped: Vec<Vec<Word>> = Vec::new();
    for line in &lines {
        let words: Vec<Word> = line
            .iter()
            .map(|word| Word {
                text: word.content.clone(),
                face: word_face(word.kind),
            })
            .collect();
        let fits = |line: &[Word]| -> Result<bool, TextError> {
            Ok(measure_words(line, wrap_size, decode_html_entities)? <= width)
        };
        wrapped.extend(fit_line(&words, fits)?);
    }

    let pieces: Vec<Vec<(String, &'static Face)>> = wrapped
        .iter()
        .map(|line| {
            line.iter()
                .enumerate()
                .map(|(index, word)| {
                    let shown = decode_html_entities(&word.text);
                    // Every word after the first is drawn with its leading space.
                    let text = if index == 0 {
                        shown
                    } else {
                        format!(" {shown}")
                    };
                    (text, word.face)
                })
                .collect()
        })
        .collect();

    let (lines, bbox) = place(&pieces, font_size)?;
    let background = if spec.add_background && !spec.text.is_empty() {
        // The reference measures the background before the label style is applied to the text,
        // so the rectangle is the box of the unstyled text.
        let (_, unstyled) = place(&pieces, spec.font_size)?;
        Some(BBox::new(
            unstyled.x - BACKGROUND_PADDING,
            unstyled.y - BACKGROUND_PADDING,
            unstyled.width + 2.0 * BACKGROUND_PADDING,
            unstyled.height + 2.0 * BACKGROUND_PADDING,
        ))
    } else {
        None
    };
    Ok(LabelLayout {
        lines,
        bbox,
        background,
        font_size,
    })
}

/// Places the lines at `size` pixels, centred on `x = 0`, and returns them with the box of their
/// text. Each line is centred on its own width, and each run starts where the runs before it end.
/// The box is the union of the runs' boxes, each from the run's ascent to its descent, as
/// `getBBox` takes it; a line without words has no box, so an empty line adds no height.
fn place(
    pieces: &[Vec<(String, &'static Face)>],
    size: f64,
) -> Result<(Vec<LabelLine>, BBox), TextError> {
    let mut lines = Vec::with_capacity(pieces.len());
    let mut bbox: Option<BBox> = None;
    for (index, line) in pieces.iter().enumerate() {
        let baseline = size * (1.0 + LINE_HEIGHT * index as f64);
        let mut widths = Vec::with_capacity(line.len());
        for (text, face) in line {
            widths.push(measure(face, text, size)?);
        }
        let width: f64 = widths.iter().sum();
        let mut x = -width / 2.0;
        let mut runs = Vec::with_capacity(line.len());
        for ((text, face), run_width) in line.iter().zip(widths) {
            let metrics = text_box(face, size)?;
            let run_box = BBox::new(x, baseline - metrics.ascent, run_width, metrics.height);
            bbox = Some(bbox.map_or(run_box, |so_far| so_far.union(run_box)));
            runs.push(LabelRun {
                text: text.clone(),
                weight: face.weight(),
                style: face.style(),
                x,
                width: run_width,
            });
            x += run_width;
        }
        lines.push(LabelLine {
            runs,
            width,
            baseline,
        });
    }
    Ok((lines, bbox.unwrap_or_default()))
}

/// The face of a word: strong words are bold and emphasis is italic. The label's own weight and
/// style are not used, because the reference sets `font-weight` and `font-style` on every word's
/// `tspan` (`createText.ts`), which overrides them; only the font size of the style reaches the
/// text.
fn word_face(kind: MarkdownWordType) -> &'static Face {
    let weight = if kind == MarkdownWordType::Strong {
        FontWeight::Bold
    } else {
        FontWeight::Normal
    };
    let style = if kind == MarkdownWordType::Em {
        FontStyle::Italic
    } else {
        FontStyle::Normal
    };
    face(FontFamily::Sans, weight, style)
}

/// The last `font-size` declaration of a style, in pixels. Only `px` is read; other units leave the
/// default size, as the bundled text has no relative size to resolve them against.
fn declared_font_size(styles: &[String]) -> Option<f64> {
    styles.iter().rev().find_map(|declaration| {
        let (name, value) = declaration.split_once(':')?;
        if !name.trim().eq_ignore_ascii_case("font-size") {
            return None;
        }
        value.trim().strip_suffix("px")?.trim().parse::<f64>().ok()
    })
}

/// `text.replace(/<br\s*\/?>/g, '<br/>')`: every `<br>` tag becomes `<br/>`, in lower case only.
fn normalize_breaks(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while !rest.is_empty() {
        if let Some(length) = br_tag_length(rest, false) {
            out.push_str("<br/>");
            rest = &rest[length..];
        } else if let Some(character) = rest.chars().next() {
            out.push(character);
            rest = &rest[character.len_utf8()..];
        }
    }
    out
}

/// `decodeEntities` in `utils.ts`: the placeholders that the diagram source encodes its entities
/// with become `&#`, `&` and `;` again.
fn decode_entities(text: &str) -> String {
    text.replace("ﬂ°°", "&#")
        .replace("ﬂ°", "&")
        .replace("¶ß", ";")
}

/// `decodeHTMLEntities` in `createText.ts`: `&amp;`, `&lt;` and `&gt;` become their characters,
/// in one pass, and nothing else is decoded.
fn decode_html_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(position) = rest.find('&') {
        out.push_str(&rest[..position]);
        let tail = &rest[position..];
        let decoded = [("&amp;", '&'), ("&lt;", '<'), ("&gt;", '>')]
            .into_iter()
            .find(|(entity, _)| tail.starts_with(entity));
        match decoded {
            Some((entity, character)) => {
                out.push(character);
                rest = &tail[entity.len()..];
            }
            None => {
                out.push('&');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec<'a>(text: &'a str, label_type: LabelType, styles: &'a [String]) -> LabelSpec<'a> {
        LabelSpec {
            text,
            label_type,
            width: 200.0,
            is_node: true,
            add_background: false,
            styles,
            font_size: 16.0,
        }
    }

    /// The text of a label as the SVG shows it: the words of all lines, one after the other, as
    /// `textContent` reads them.
    fn text_content(layout: &LabelLayout) -> String {
        layout
            .lines
            .iter()
            .flat_map(|line| line.runs.iter())
            .map(|run| run.text.as_str())
            .collect()
    }

    /// `createText` in `createText.spec.ts`: decodes HTML entities in the text. The `useHtmlLabels`
    /// variants of the spec are not ported, since HTML labels are out of scope (decision 0004).
    #[test]
    fn decodes_html_entities_in_text_when_use_html_labels_is_false_and_markdown_is_true() {
        let styles = [];
        let layout = layout(&spec(
            "5 &gt; 3 &amp;&amp; 2 &lt; 4",
            LabelType::Markdown,
            &styles,
        ))
        .expect("the label lays out");
        assert_eq!(text_content(&layout), "5 > 3 && 2 < 4");
    }

    #[test]
    fn decodes_html_entities_in_text_when_use_html_labels_is_false_and_markdown_is_false() {
        let styles = [];
        let layout = layout(&spec(
            "5 &gt; 3 &amp;&amp; 2 &lt; 4",
            LabelType::Text,
            &styles,
        ))
        .expect("the label lays out");
        assert_eq!(text_content(&layout), "5 > 3 && 2 < 4");
    }

    #[test]
    fn a_label_is_centred_and_each_run_starts_where_the_last_ends() {
        let styles = [];
        let layout = layout(&spec("**a** b", LabelType::Markdown, &styles)).expect("lays out");
        let line = &layout.lines[0];
        assert_eq!(line.runs.len(), 2);
        assert!((line.runs[0].x + line.width / 2.0).abs() < 1e-9);
        assert!((line.runs[1].x - (line.runs[0].x + line.runs[0].width)).abs() < 1e-9);
        assert_eq!(line.runs[1].text, " b");
    }

    #[test]
    fn the_svg_writes_each_line_and_word_at_an_explicit_x() {
        let styles = [];
        let layout = layout(&spec("a<br/>**b**", LabelType::Markdown, &styles)).expect("lays out");
        let svg = layout.svg("fill: red;");
        assert!(
            svg.starts_with("<g><text y=\"-10.1\" text-anchor=\"start\" style=\"fill: red;\">")
        );
        assert_eq!(
            svg.matches("<tspan class=\"text-outer-tspan row\"").count(),
            2
        );
        assert_eq!(svg.matches("class=\"text-inner-tspan\"").count(), 2);
        assert!(svg.contains("font-weight=\"bold\" x=\""), "{svg}");
        assert!(svg.ends_with("</text></g>"), "{svg}");
    }

    #[test]
    fn the_svg_escapes_the_text_of_a_word() {
        let styles = [];
        let layout = layout(&spec("a &lt; b", LabelType::Text, &styles)).expect("lays out");
        // The second word is drawn with its leading space, and its `<` is written as `&lt;`.
        assert!(
            layout.svg("").contains(" &lt;</tspan>"),
            "{}",
            layout.svg("")
        );
    }

    #[test]
    fn a_font_size_style_sets_the_text_and_leaves_the_background_at_the_default_size() {
        let styles = ["font-size:20px".to_owned()];
        let mut label = spec("Edge label", LabelType::Text, &styles);
        label.is_node = false;
        label.add_background = true;
        let layout = layout(&label).expect("lays out");
        assert_eq!(layout.font_size, 20.0);
        let background = layout.background.expect("the edge label has a background");
        let plain = layout_with_default(&label);
        assert_eq!(
            background,
            plain.background.expect("the default has one too")
        );
        assert_ne!(background.height, layout.bbox.height);
    }

    fn layout_with_default(label: &LabelSpec<'_>) -> LabelLayout {
        let mut plain = label.clone();
        plain.styles = &[];
        layout(&plain).expect("lays out")
    }
}
