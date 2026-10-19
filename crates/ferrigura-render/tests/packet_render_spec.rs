//! Port of `packages/mermaid/src/diagrams/packet/renderer.spec.ts`. Test names follow the spec's
//! titles in snake_case. The spec reads the rendered SVG back: the x and width of each block, its
//! label, and its bit numbers with their anchors (`left`, `center`, `right`), row by row. The
//! reader below does the same on the SVG text, and the tests after the eleven check what the spec
//! does not: the root's size and viewBox, the title, the accessible names, the entities of a label,
//! and the styles that `themeVariables.packet` sets.

use ferrigura_model::config::{effective, typed};
use ferrigura_model::theme::{Theme, ThemeTokens};
use ferrigura_model::value::Value;
use ferrigura_render::context::{Context, Globals, ThemeView};
use ferrigura_render::packet::{PacketSection, render};
use regex::Regex;

const ID: &str = "packet-diagram";

/// Renders a packet text with the redux colour theme, its configuration read the way the facade
/// reads it: the front matter and the directives of the text give the `packet` section.
fn draw(text: &str) -> String {
    draw_with_variables(text, &Theme::ReduxColor.variables(None))
}

fn draw_with_variables(text: &str, variables: &Value) -> String {
    draw_configured(text, None, variables)
}

/// Renders a packet text with the site configuration `site` (what `initialize` gets) under the
/// text's own front matter and directives, as the facade does.
fn draw_configured(text: &str, site: Option<&Value>, variables: &Value) -> String {
    let directive = ferrigura_syntax::config::read(text).expect("the directives read");
    let configured = effective(site, Some("packet"), &directive);
    let config = typed(&configured).expect("the configuration is typed");
    let diagram =
        ferrigura_syntax::packet::parse_with(text, &configured).expect("the packet parses");
    let tokens = ThemeTokens::from_value(variables);
    let globals = Globals::of(&config);
    let section = PacketSection {
        config: config.packet.clone(),
        bits_per_row: ferrigura_syntax::packet::bits_per_row(&configured),
    };
    render(&Context {
        diagram: &diagram,
        section: &section,
        globals: &globals,
        theme: ThemeView {
            light: &tokens,
            dark: None,
        },
        id: ID,
    })
    .expect("the packet renders")
}

/// One bit number as the spec reads it back: where it sits on its block, what it says, its x.
#[derive(Debug, PartialEq)]
struct RenderedBit {
    position: &'static str,
    text: String,
    x: f64,
}

/// One block of a row, read back from its rectangle, its label and its bit numbers.
#[derive(Debug, PartialEq)]
struct RenderedBlock {
    label: String,
    x: f64,
    width: f64,
    bits: Vec<RenderedBit>,
}

/// The attribute `name` of an element's attribute text (which starts with a space), if it is set.
fn attribute(attributes: &str, name: &str) -> Option<String> {
    let pattern = Regex::new(&format!(r#" {name}="([^"]*)""#)).expect("a valid pattern");
    pattern
        .captures(attributes)
        .map(|captures| captures[1].to_owned())
}

fn number_attribute(attributes: &str, name: &str) -> f64 {
    attribute(attributes, name)
        .and_then(|value| value.parse().ok())
        .unwrap_or(f64::NAN)
}

/// The rows of a rendered packet, each listing its blocks from left to right. The renderer writes
/// a block's rectangle, then its label, then its bit numbers, so the elements of a row are folded
/// back into blocks in document order. The empty group `mermaidAPI` writes has no children.
fn read_rows(svg: &str) -> Vec<Vec<RenderedBlock>> {
    let row = Regex::new(r"<g>(.*?)</g>").expect("a valid pattern");
    let element =
        Regex::new(r"<(rect|text) ([^>]*?)(?:/>|>([^<]*)</text>)").expect("a valid pattern");
    row.captures_iter(svg)
        .map(|row| {
            let mut blocks: Vec<RenderedBlock> = Vec::new();
            for captures in element.captures_iter(&row[1]) {
                let attributes = format!(" {}", &captures[2]);
                let text = captures.get(3).map_or("", |text| text.as_str());
                let class = attribute(&attributes, "class").unwrap_or_default();
                if class == "packetBlock" {
                    blocks.push(RenderedBlock {
                        label: String::new(),
                        x: number_attribute(&attributes, "x"),
                        width: number_attribute(&attributes, "width"),
                        bits: Vec::new(),
                    });
                } else if class == "packetLabel" {
                    blocks.last_mut().expect("a label follows its block").label = text.to_owned();
                } else if class.starts_with("packetByte") {
                    let position = match attribute(&attributes, "text-anchor").as_deref() {
                        Some("start") => "left",
                        Some("middle") => "center",
                        Some("end") => "right",
                        other => panic!("a bit number has no anchor: {other:?}"),
                    };
                    blocks
                        .last_mut()
                        .expect("a bit number follows its block")
                        .bits
                        .push(RenderedBit {
                            position,
                            text: text.to_owned(),
                            x: number_attribute(&attributes, "x"),
                        });
                }
            }
            blocks.sort_by(|a, b| a.x.total_cmp(&b.x));
            blocks
        })
        .collect()
}

/// Every bit number of a row, in the order a reader meets them going left to right.
fn bit_numbers_left_to_right(row: &[RenderedBlock]) -> Vec<f64> {
    row.iter()
        .flat_map(|block| {
            block
                .bits
                .iter()
                .map(|bit| bit.text.parse::<f64>().expect("a bit number"))
        })
        .collect()
}

/// Each field's drawn width, keyed by label, which `bitOrder` must not change.
fn widths_by_label(rows: &[Vec<RenderedBlock>]) -> Vec<(String, f64)> {
    let mut widths: Vec<(String, f64)> = rows
        .iter()
        .flatten()
        .map(|block| (block.label.clone(), block.width))
        .collect();
    widths.sort_by(|a, b| a.0.cmp(&b.0));
    widths
}

const TWO_FIELDS: &str = "packet
0-7: \"foo\"
8-15: \"bar\"
";

const REGISTER: &str = "packet
0-7: \"DATA\"
8-11: \"TYPE\"
12: \"EN\"
13-15: \"RESERVED\"
";

/// The `bitsPerRow` and `bitOrder` settings of a packet, as the front matter gives them.
fn configured(settings: &str, body: &str) -> String {
    format!("---\nconfig:\n  packet:\n{settings}---\n{body}")
}

fn block(label: &str, x: f64, width: f64, bits: Vec<RenderedBit>) -> RenderedBlock {
    RenderedBlock {
        label: label.to_owned(),
        x,
        width,
        bits,
    }
}

fn bit(position: &'static str, text: &str, x: f64) -> RenderedBit {
    RenderedBit {
        position,
        text: text.to_owned(),
        x,
    }
}

#[test]
fn lays_fields_out_from_bit_0_each_numbered_from_its_start_bit() {
    let rows = read_rows(&draw(TWO_FIELDS));
    assert_eq!(
        rows,
        vec![vec![
            block(
                "foo",
                1.0,
                251.0,
                vec![bit("left", "0", 1.0), bit("right", "7", 252.0)]
            ),
            block(
                "bar",
                257.0,
                251.0,
                vec![bit("left", "8", 257.0), bit("right", "15", 508.0)]
            ),
        ]]
    );
}

#[test]
fn renders_an_explicit_ascending_bit_order_exactly_like_the_default() {
    let by_default = read_rows(&draw(&configured("    bitsPerRow: 16\n", REGISTER)));
    let explicit = read_rows(&draw(&configured(
        "    bitsPerRow: 16\n    bitOrder: ascending\n",
        REGISTER,
    )));
    assert_eq!(explicit, by_default);
}

#[test]
fn mirrors_the_row_so_it_reads_from_the_most_significant_bit_down() {
    let rows = read_rows(&draw(&configured(
        "    bitsPerRow: 16\n    bitOrder: descending\n",
        TWO_FIELDS,
    )));
    assert_eq!(
        rows,
        vec![vec![
            block(
                "bar",
                1.0,
                251.0,
                vec![bit("left", "15", 1.0), bit("right", "8", 252.0)]
            ),
            block(
                "foo",
                257.0,
                251.0,
                vec![bit("left", "7", 257.0), bit("right", "0", 508.0)]
            ),
        ]]
    );
}

#[test]
fn numbers_a_row_of_mixed_width_fields_straight_down_from_its_highest_bit() {
    let ascending = read_rows(&draw(&configured("    bitsPerRow: 16\n", REGISTER)));
    let descending = read_rows(&draw(&configured(
        "    bitsPerRow: 16\n    bitOrder: descending\n",
        REGISTER,
    )));
    let labels: Vec<&str> = descending[0]
        .iter()
        .map(|block| block.label.as_str())
        .collect();
    assert_eq!(labels, ["RESERVED", "EN", "TYPE", "DATA"]);
    assert_eq!(
        bit_numbers_left_to_right(&ascending[0]),
        [0.0, 7.0, 8.0, 11.0, 12.0, 13.0, 15.0]
    );
    assert_eq!(
        bit_numbers_left_to_right(&descending[0]),
        [15.0, 13.0, 12.0, 11.0, 8.0, 7.0, 0.0]
    );
}

#[test]
fn keeps_every_field_the_width_it_has_in_ascending_order() {
    let ascending = read_rows(&draw(&configured("    bitsPerRow: 16\n", REGISTER)));
    let descending = read_rows(&draw(&configured(
        "    bitsPerRow: 16\n    bitOrder: descending\n",
        REGISTER,
    )));
    assert_eq!(widths_by_label(&descending), widths_by_label(&ascending));
}

#[test]
fn centres_the_number_on_a_single_bit_field_as_ascending_order_does() {
    let ascending = read_rows(&draw(&configured("    bitsPerRow: 16\n", REGISTER)));
    let descending = read_rows(&draw(&configured(
        "    bitsPerRow: 16\n    bitOrder: descending\n",
        REGISTER,
    )));
    let enable_bit = bit("center", "12", 0.0);
    for rows in [&ascending, &descending] {
        let en = rows[0]
            .iter()
            .find(|block| block.label == "EN")
            .expect("the EN field");
        assert_eq!(en.bits.len(), 1);
        assert_eq!(en.bits[0].position, enable_bit.position);
        assert_eq!(en.bits[0].text, enable_bit.text);
    }
}

#[test]
fn mirrors_each_row_of_a_multi_row_packet_on_its_own() {
    let rows = read_rows(&draw(&configured(
        "    bitsPerRow: 8\n    bitOrder: descending\n",
        TWO_FIELDS,
    )));
    let placed: Vec<Vec<(String, f64, f64)>> = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|block| (block.label.clone(), block.x, block.width))
                .collect()
        })
        .collect();
    assert_eq!(
        placed,
        vec![
            vec![("foo".to_owned(), 1.0, 251.0)],
            vec![("bar".to_owned(), 1.0, 251.0)],
        ]
    );
    let numbers: Vec<Vec<f64>> = rows
        .iter()
        .map(|row| bit_numbers_left_to_right(row))
        .collect();
    assert_eq!(numbers, vec![vec![7.0, 0.0], vec![15.0, 8.0]]);
}

#[test]
fn mirrors_within_bits_per_row_so_a_part_filled_row_is_flush_right() {
    let row = read_rows(&draw("packet\n0-7: \"foo\"\n"));
    let mirrored = read_rows(&draw(&configured(
        "    bitOrder: descending\n",
        "packet\n0-7: \"foo\"\n",
    )));
    // 8 bits of a 32-bit row: flush left ascending, flush right descending.
    assert_eq!(row[0].len(), 1);
    assert_eq!((row[0][0].x, row[0][0].width), (1.0, 251.0));
    assert_eq!(mirrored[0].len(), 1);
    assert_eq!(
        (mirrored[0][0].x, mirrored[0][0].width),
        (24.0 * 32.0 + 1.0, 251.0)
    );
}

#[test]
fn draws_no_bit_numbers_with_show_bits_false_and_bit_order_ascending() {
    assert_no_bit_numbers("ascending");
}

#[test]
fn draws_no_bit_numbers_with_show_bits_false_and_bit_order_descending() {
    assert_no_bit_numbers("descending");
}

fn assert_no_bit_numbers(bit_order: &str) {
    let svg = draw(&configured(
        &format!("    bitsPerRow: 16\n    showBits: false\n    bitOrder: {bit_order}\n"),
        REGISTER,
    ));
    let rows = read_rows(&svg);
    assert!(rows.iter().flatten().all(|block| block.bits.is_empty()));
    assert!(!svg.contains(r#"class="packetByte"#));
}

#[test]
fn still_mirrors_the_row_with_show_bits_false() {
    let rows = read_rows(&draw(&configured(
        "    bitsPerRow: 16\n    showBits: false\n    bitOrder: descending\n",
        REGISTER,
    )));
    let labels: Vec<&str> = rows[0].iter().map(|block| block.label.as_str()).collect();
    assert_eq!(labels, ["RESERVED", "EN", "TYPE", "DATA"]);
}

// The checks the spec does not make. The numbers are the ones the reference gives for the fixtures
// (`pnpm port:geometry packet`) and the probes of `tools/port/geometry/probes/packet`.

/// The `viewBox` of the root, as written.
fn view_box(svg: &str) -> String {
    attribute(svg, "viewBox").expect("the root has a viewBox")
}

#[test]
fn a_one_row_packet_with_a_title_is_as_high_as_two_rows_less_one_row_height() {
    // `svgHeight = totalRowHeight * (rows + 1) - (title ? 0 : rowHeight)`: 47 * 2 with the title.
    let svg = draw("packet\ntitle Hello world\n0-10: \"hello\"\n");
    assert_eq!(view_box(&svg), "0 0 1026 94");
    // The title sits half a row above the bottom: 94 - 47 / 2.
    assert!(svg.contains(r#"<text x="513" y="70.5" dominant-baseline="middle" text-anchor="middle" class="packetTitle""#));
}

#[test]
fn without_a_title_the_height_drops_one_row_height() {
    let svg = draw("packet\n0-10: \"hello\"\n");
    assert_eq!(view_box(&svg), "0 0 1026 62");
}

#[test]
fn a_packet_without_rows_is_one_row_high() {
    assert_eq!(view_box(&draw("packet\n")), "0 0 1026 15");
}

#[test]
fn show_bits_false_takes_the_space_of_the_bit_numbers_from_the_height() {
    // Seven rows of 32 bits, no title: (32 + 15) * 8 - 32 with the bit numbers, (32 + 5) * 8 - 32
    // without them.
    let body = "packet\n0-223: \"data\"\n";
    assert_eq!(view_box(&draw(body)), "0 0 1026 344");
    assert_eq!(
        view_box(&draw(&configured("    showBits: false\n", body))),
        "0 0 1026 264"
    );
    // The same packet with a title, as the fixture `showbits-false` has: no row is taken out.
    let titled = "packet\ntitle Packet Diagram\n0-223: \"data\"\n";
    assert_eq!(
        view_box(&draw(&configured("    showBits: false\n", titled))),
        "0 0 1026 296"
    );
}

#[test]
fn the_root_is_sized_as_the_reference_sizes_it_with_max_width() {
    let svg = draw("packet\n0-10: \"hello\"\n");
    assert!(svg.starts_with(&format!(
        r#"<svg id="{ID}" width="100%" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1026 62" style="max-width: 1026px;" role="graphics-document document" aria-roledescription="packet">"#
    )));
}

#[test]
fn without_max_width_the_root_takes_a_height_and_a_width() {
    // The root keeps the width `mermaidAPI` gave it, and takes the height after the viewBox, as the
    // reference's attributes do (checked against its SVG for `probes/packet/p12-fixed-size`).
    let svg = draw(&configured(
        "    useMaxWidth: false\n",
        "packet\n0-10: \"hello\"\n",
    ));
    assert!(svg.starts_with(&format!(
        r#"<svg id="{ID}" width="1026" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1026 62" height="62""#
    )));
}

#[test]
fn the_accessible_names_are_the_root_s_title_and_description() {
    let svg = draw("packet\naccTitle: Packet title\naccDescr: Packet description\n0-7: \"a\"\n");
    assert!(svg.contains(&format!(r#"aria-describedby="chart-desc-{ID}""#)));
    assert!(svg.contains(&format!(r#"aria-labelledby="chart-title-{ID}""#)));
    assert!(svg.contains(&format!(
        r#"<title id="chart-title-{ID}">Packet title</title><desc id="chart-desc-{ID}">Packet description</desc>"#
    )));
}

#[test]
fn a_bits_per_row_given_to_initialize_splits_the_rows_as_the_text_does() {
    // The site configuration sets `bitsPerRow` to 16: a 20-bit field is two rows, 16 bits and 4.
    let site = Value::from(serde_json::json!({ "packet": { "bitsPerRow": 16 } }));
    let svg = draw_configured(
        "packet\n0-19: \"wide\"\n",
        Some(&site),
        &Theme::ReduxColor.variables(None),
    );
    assert_eq!(view_box(&svg), "0 0 514 109");
    let rows = read_rows(&svg);
    assert_eq!(
        rows,
        vec![
            vec![block(
                "wide",
                1.0,
                507.0,
                vec![bit("left", "0", 1.0), bit("right", "15", 508.0)]
            )],
            vec![block(
                "wide",
                1.0,
                123.0,
                vec![bit("left", "16", 1.0), bit("right", "19", 124.0)]
            )],
        ]
    );
}

#[test]
fn a_label_shows_its_entities_as_characters_and_its_own_ampersands_as_written() {
    // `#amp;` is the entity the user typed; `&amp;` is text, shown as written.
    let svg = draw("packet\n0-7: \"a #amp; b &amp; <c>\"\n");
    assert!(svg.contains(">a &amp; b &amp;amp; &lt;c&gt;</text>"));
}

#[test]
fn the_style_sheet_takes_the_packet_options_of_the_theme_variables() {
    let mut variables = Theme::ReduxColor.variables(None);
    variables
        .as_object_mut()
        .expect("the theme is an object")
        .insert(
            "packet".to_owned(),
            Value::from(serde_json::json!({ "blockFillColor": "red", "labelFontSize": "9px" })),
        );
    let svg = draw_with_variables("packet\n0-7: \"a\"\n", &variables);
    assert!(svg.contains(&format!("#{ID}{{--packetStartByteColor:black;")));
    assert!(svg.contains("--packetBlockFillColor:red;}"));
    assert!(svg.contains(&format!(
        "#{ID} .packetLabel{{fill:var(--packetLabelColor);font-size:9px;}}"
    )));
    // The rectangle takes the same colour as a presentation attribute, for viewers without
    // custom properties.
    assert!(svg.contains(r#"class="packetBlock" fill="red" stroke="black"/>"#));
}
