//! The element label of a C4 shape: `c4LabelHelper.ts`. A stack of sections, the name, the
//! stereotype and the description, each laid out as `createText` lays a text out, centred, with a
//! gap of 3 px between the sections (`SECTION_GAP`, line 9). The sections are drawn with the classes
//! that `styles.js` styles, so their sizes are the element's: the name is bold, the stereotype and the
//! description take `0.75em` and `0.82em` of it.

use ferrigura_model::flowchart::LabelType;
use ferrigura_syntax::sanitize::sanitize_text;
use ferrigura_text::FontWeight;

use crate::RenderError;
use crate::common::decode_entities;
use crate::svg::{escape, number};
use crate::unified::label::{self, Anchor, LabelLayout, LabelSpec};

/// The gap between two sections, in pixels (`c4LabelHelper.ts:9`).
const SECTION_GAP: f64 = 3.0;

/// The font of one section: its size in pixels and its weight, which the section's text inherits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SectionFont {
    /// The font size in pixels.
    pub size: f64,
    /// The weight of the text.
    pub weight: FontWeight,
}

/// One section of the label, laid out and placed in the label's coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    /// The CSS class of the section: `c4-name`, `c4-type` or `c4-descr`.
    pub class: &'static str,
    /// The text as laid out.
    pub layout: LabelLayout,
    /// The translation of the section's group.
    pub translate: (f64, f64),
}

/// The laid-out label: its sections and the box of the whole label (`totalWidth` and `totalHeight`).
#[derive(Debug, Clone, PartialEq)]
pub struct Stacked {
    /// The widest section.
    pub width: f64,
    /// The sections' heights and the gaps between them.
    pub height: f64,
    /// The sections, top to bottom.
    pub sections: Vec<Section>,
}

/// Lays out the sections given as `(class, text, font)`, wrapping each at `wrap_width` (`f64::INFINITY`
/// does not wrap). The texts are sanitised after their entities are decoded, as `c4LabelHelper` does
/// (line 61).
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read.
pub fn layout(
    sections: &[(&'static str, String, SectionFont)],
    wrap_width: f64,
) -> Result<Stacked, RenderError> {
    let mut laid = Vec::with_capacity(sections.len());
    for (class, text, font) in sections {
        let shown = sanitize_text(&decode_entities(text));
        let spec = LabelSpec {
            text: &shown,
            label_type: LabelType::String,
            width: wrap_width,
            is_node: true,
            anchor: Anchor::Middle,
            wrap_in_style: true,
            add_background: false,
            styles: &[],
            font_size: font.size,
            title: false,
        };
        let layout = label::layout_inheriting(&spec, Some(font.weight))?;
        laid.push((*class, layout));
    }

    // `totalWidth` is the widest section's box, and each section is moved so that its box is centred
    // on it (line 86); the sections then stack with the gap (lines 84-89).
    let total_width = laid
        .iter()
        .map(|(_, layout)| layout.bbox.width)
        .fold(0.0_f64, f64::max);
    let mut y = 0.0_f64;
    let mut placed = Vec::with_capacity(laid.len());
    for (class, layout) in laid {
        let bbox = layout.bbox;
        let translate = (total_width / 2.0 - bbox.x - bbox.width / 2.0, y - bbox.y);
        y += bbox.height + SECTION_GAP;
        placed.push(Section {
            class,
            layout,
            translate,
        });
    }
    let total_height = if placed.is_empty() {
        0.0
    } else {
        y - SECTION_GAP
    };
    Ok(Stacked {
        width: total_width,
        height: total_height,
        sections: placed,
    })
}

/// The `<g class="label">` of a shape: its style, its translation, the empty rectangle that the
/// reference inserts first (`c4LabelHelper.ts:91`), and the sections. `fill` is the colour of the
/// text: the element's font colour, `#FFFFFF` by default.
#[must_use]
pub fn group(stacked: &Stacked, translate: (f64, f64), fill: &str) -> String {
    let mut out = format!(
        "<g class=\"label\" style=\"color:{fill} !important\" transform=\"translate({}, {})\"><rect></rect>",
        number(translate.0),
        number(translate.1),
    );
    for section in &stacked.sections {
        out.push_str(&section_group(section, fill));
    }
    out.push_str("</g>");
    out
}

/// One section: its `g` with the class and its translation, and the `createText` markup with every
/// line's outer tspan centred and every word's tspan without weight or style (`c4LabelHelper.ts`
/// 72-78), so the CSS of the section decides them.
fn section_group(section: &Section, fill: &str) -> String {
    let mut out = format!(
        "<g class=\"{}\" transform=\"translate({}, {})\"><g><rect class=\"background\" style=\"stroke: none\"></rect><text y=\"-10.1\" style=\"fill:{fill} !important\">",
        section.class,
        number(section.translate.0),
        number(section.translate.1),
    );
    for (index, line) in section.layout.lines.iter().enumerate() {
        let y = index as f64 * 1.1 - 0.1;
        out.push_str(&format!(
            "<tspan class=\"text-outer-tspan row\" x=\"0\" y=\"{}em\" dy=\"1.1em\" text-anchor=\"middle\">",
            number(y),
        ));
        for run in &line.runs {
            out.push_str(&format!(
                "<tspan class=\"text-inner-tspan\">{}</tspan>",
                escape(&run.text),
            ));
        }
        out.push_str("</tspan>");
    }
    out.push_str("</text></g></g>");
    out
}
