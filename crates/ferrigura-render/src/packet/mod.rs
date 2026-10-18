//! The packet diagram: a port of `packages/mermaid/src/diagrams/packet/renderer.ts` (`draw`, lines
//! 8-34, and `drawWord`, lines 36-112), with the styles of `styles.ts` in [`style`]. The research is
//! in `docs-port/packet/NOTES.md`, section 6.
//!
//! Nothing is measured and nothing is laid out. A block is drawn from its bits: its rectangle starts
//! at its column times the bit width, plus one, and its width is its bits times the bit width, less
//! the horizontal padding; its label and its bit numbers are placed from that box. The rows are the
//! parser's, already split at the row boundaries (`ferrigura_syntax::packet`), and the size of the
//! root follows from their number. The browser measures the text when it shows the SVG, with the
//! fonts the SVG embeds.

mod style;

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::packet::{PacketBlock, PacketDiagram};
use ferrigura_syntax::sanitize::decode_references;
use ferrigura_text::{EmbeddedFamily, FontFamily, FontStyle, FontWeight, Run, embed_runs, face};

use crate::RenderError;
use crate::a11y;
use crate::common::Element;
use crate::common::decode_entities;
use crate::common::style::base_style_sheet_with;
use crate::common::viewbox::configure_svg_size;
use crate::svg::number;
use crate::theme::Themes;
use crate::variables::text_of;
use style::Options;

/// The XML namespace of the root element.
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// The room `showBits` adds below the rows, to `paddingY` (`db.ts` 26-28).
const BIT_NUMBER_SPACE: f64 = 10.0;

/// What the packet renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct PacketInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a PacketDiagram,
    /// The effective configuration, in the packet diagram's scope.
    pub config: &'a MermaidConfig,
    /// The colours and sizes of the theme.
    pub themes: Themes<'a>,
    /// The id of the SVG element. It scopes every selector and prefixes every element id, so it
    /// must be a CSS identifier.
    pub id: &'a str,
    /// `bitsPerRow` as the reference's arithmetic reads it, from the effective configuration
    /// (`ferrigura_syntax::packet::bits_per_row`). The typed configuration replaces a string that is
    /// not a number by the default, where the reference computes `NaN` with it.
    pub bits_per_row: f64,
}

/// The numbers a row is drawn with: `getConfig` of the DB (`db.ts` 21-30), with `paddingY` already
/// raised when the bit numbers are shown, and the bit order.
struct Geometry {
    row_height: f64,
    padding_x: f64,
    padding_y: f64,
    bit_width: f64,
    bits_per_row: f64,
    show_bits: bool,
    /// `bitOrder` is `descending`: each row is mirrored (`renderer.ts` 52-58).
    descending: bool,
}

impl Geometry {
    fn of(input: &PacketInput<'_>) -> Self {
        let config = &input.config.packet;
        Self {
            row_height: config.row_height,
            padding_x: config.padding_x,
            padding_y: if config.show_bits {
                config.padding_y + BIT_NUMBER_SPACE
            } else {
                config.padding_y
            },
            bit_width: config.bit_width,
            bits_per_row: input.bits_per_row,
            show_bits: config.show_bits,
            descending: config.bit_order == "descending",
        }
    }
}

/// Renders the diagram to an SVG document.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read or subset.
pub fn render(input: &PacketInput<'_>) -> Result<String, RenderError> {
    let diagram = input.diagram;
    let id = input.id;
    let config = &input.config.packet;
    let variables = input.themes.primary();
    let options = Options::of(variables);
    let geometry = Geometry::of(input);

    // `draw` (`renderer.ts` 8-20): the height is one row for each row and one more, less the row
    // height when there is no title; the width is the bits of a row and two more.
    let total_row_height = geometry.row_height + geometry.padding_y;
    let title_gap = if diagram.title.is_empty() {
        geometry.row_height
    } else {
        0.0
    };
    let svg_height = total_row_height * (diagram.rows.len() as f64 + 1.0) - title_gap;
    let svg_width = geometry.bit_width * geometry.bits_per_row + 2.0;

    // The rows (`renderer.ts` 22-24), then the title (`renderer.ts` 26-33), which comes last.
    let rows: Vec<Element> = diagram
        .rows
        .iter()
        .enumerate()
        .map(|(row_number, row)| row_group(row_number, row, &geometry, &options))
        .collect();
    let title = Element::new("text")
        .with("x", number(svg_width / 2.0))
        .with("y", number(svg_height - total_row_height / 2.0))
        .with("dominant-baseline", "middle")
        .with("text-anchor", "middle")
        .with("class", "packetTitle")
        .with("fill", options.title_color.as_str())
        .text(shown_text(&diagram.title));

    // `addA11yInfo` (`accessibility.ts` 27-65): the description is inserted first, then the title,
    // so the title is the first child, and the root refers to both.
    let mut accessibility = Vec::new();
    if !diagram.acc_title.is_empty() {
        accessibility.push(
            Element::new("title")
                .with("id", a11y::title_id(id))
                .text(shown_text(&diagram.acc_title)),
        );
    }
    if !diagram.acc_descr.is_empty() {
        accessibility.push(
            Element::new("desc")
                .with("id", a11y::description_id(id))
                .text(shown_text(&diagram.acc_descr)),
        );
    }

    // The fonts: every character the SVG draws, in the regular face (the packet sets no other).
    let mut texts = String::new();
    for element in accessibility
        .iter()
        .chain(&rows)
        .chain(std::iter::once(&title))
    {
        element.collect_text(&mut texts);
    }
    let families: Vec<EmbeddedFamily> = embed_runs(&[Run {
        text: &texts,
        face: face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal),
    }])?;
    let font_family = families.first().map_or_else(
        || text_of(variables, "fontFamily").unwrap_or_else(|| "sans-serif".to_owned()),
        |family| family.css_family.clone(),
    );
    let font_size = text_of(variables, "fontSize").unwrap_or_else(|| "16px".to_owned());
    let style = base_style_sheet_with(
        id,
        input.themes,
        &font_family,
        &font_size,
        &families,
        &style::own_rules(id, &options),
    );

    // `draw`'s `viewBox`, then `configureSvgSize` (`renderer.ts` 13-20), on the root that
    // `mermaidAPI` made with its id, width and namespace.
    let mut root = Element::new("svg")
        .with("id", id)
        .with("width", "100%")
        .with("xmlns", SVG_NAMESPACE)
        .with(
            "viewBox",
            format!("0 0 {} {}", number(svg_width), number(svg_height)),
        );
    configure_svg_size(&mut root, svg_height, svg_width, config.use_max_width);
    root.set("role", a11y::SVG_ROLE);
    root.set("aria-roledescription", "packet");
    if !diagram.acc_descr.is_empty() {
        root.set("aria-describedby", a11y::description_id(id));
    }
    if !diagram.acc_title.is_empty() {
        root.set("aria-labelledby", a11y::title_id(id));
    }

    for element in accessibility {
        root.push_child(element);
    }
    root.push_child(Element::new("style").raw_text(style));
    // The empty group `mermaidAPI` appends before the renderer draws.
    root.push_child(Element::new("g"));
    for row in rows {
        root.push_child(row);
    }
    root.push_child(title);
    Ok(root.render())
}

/// The group of one row (`drawWord`, `renderer.ts` 36-112): for each block, in order, its
/// rectangle, its label and, when the bit numbers are shown, its leading and trailing bit numbers.
fn row_group(
    row_number: usize,
    row: &[PacketBlock],
    geometry: &Geometry,
    options: &Options,
) -> Element {
    let mut group = Element::new("g");
    let word_y =
        row_number as f64 * (geometry.row_height + geometry.padding_y) + geometry.padding_y;
    for block in row {
        // `bits` is the block's width in bits (`renderer.ts` 54); its column is its first bit's
        // position in the row (57), mirrored when the bits run from the top down (58).
        let bits = block.end - block.start + 1.0;
        let first_column = block.start % geometry.bits_per_row;
        let column = if geometry.descending {
            geometry.bits_per_row - first_column - bits
        } else {
            first_column
        };
        let block_x = column * geometry.bit_width + 1.0;
        let width = bits * geometry.bit_width - geometry.padding_x;

        group.push_child(
            Element::new("rect")
                .with("x", number(block_x))
                .with("y", number(word_y))
                .with("width", number(width))
                .with("height", number(geometry.row_height))
                .with("class", "packetBlock")
                .with("fill", options.block_fill_color.as_str())
                .with("stroke", options.block_stroke_color.as_str()),
        );
        group.push_child(
            Element::new("text")
                .with("x", number(block_x + width / 2.0))
                .with("y", number(word_y + geometry.row_height / 2.0))
                .with("class", "packetLabel")
                .with("dominant-baseline", "middle")
                .with("text-anchor", "middle")
                .with("fill", options.label_color.as_str())
                .text(shown_text(&block.label)),
        );
        if !geometry.show_bits {
            continue;
        }

        // A mirrored block reads from its highest bit on the left down to its lowest on the right.
        let (leading, trailing) = if geometry.descending {
            (block.end, block.start)
        } else {
            (block.start, block.end)
        };
        let single = bits == 1.0;
        let bit_y = word_y - 2.0;
        group.push_child(
            Element::new("text")
                .with(
                    "x",
                    number(block_x + if single { width / 2.0 } else { 0.0 }),
                )
                .with("y", number(bit_y))
                .with("class", "packetByte start")
                .with("dominant-baseline", "auto")
                .with("text-anchor", if single { "middle" } else { "start" })
                .with("fill", options.start_byte_color.as_str())
                .text(number(leading)),
        );
        // The trailing bit number is written only when the block is wider than one bit.
        if !single {
            group.push_child(
                Element::new("text")
                    .with("x", number(block_x + width))
                    .with("y", number(bit_y))
                    .with("class", "packetByte end")
                    .with("dominant-baseline", "auto")
                    .with("text-anchor", "end")
                    .with("fill", options.end_byte_color.as_str())
                    .text(number(trailing)),
            );
        }
    }
    group
}

/// The text an SVG `text` element shows for a string of the diagram. The encoding wrote a
/// placeholder for each `&` and `#` entity the user typed (`encodeEntities`), and the reference
/// turns the placeholders into the references they stand for when it writes the markup
/// (`decodeEntities`, `utils.ts` 904-906, and `cleanUpSvgCode`); the browser reads those references.
/// Every other character is a character of the text, so it is escaped first, as the serialiser
/// writes a text node.
fn shown_text(text: &str) -> String {
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    decode_references(&decode_entities(&escaped))
}
