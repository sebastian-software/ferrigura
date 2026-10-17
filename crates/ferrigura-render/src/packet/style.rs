//! The colours and sizes of a packet diagram: `styles.ts` (`defaultPacketStyleOptions`, lines 5-16,
//! and the rules of `styles`, lines 18-45).
//!
//! The reference's `getStyles` passes `{...config.themeVariables, theme, look, svgId}` to the
//! packet styles (`mermaidAPI.ts` 327-333), and `styles` reads its `packet` key from that object. So
//! the options are `themeVariables.packet`, not `config.packet`, which has no colour; the theme's own
//! colours are not read, and a packet is drawn in the same colours in both schemes. The merge with
//! the defaults is `cleanAndMerge` (`utils.ts` 841), which keeps every option of an object and
//! ignores the rest; a `packet` that is not an object merges nothing.
//!
//! Each colour is a custom property declared on the diagram's id, and each coloured element also
//! carries its colour as a presentation attribute (`docs-port/decisions/0007`). The rules name the
//! properties, and every selector starts with the id, as `compileCSS` writes them.

use ferrigura_model::value::Value;

/// The options of the packet styles after the merge with the defaults: `PacketStyleOptions`
/// (`types.ts`), each as the template of `styles.ts` writes it.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Options {
    /// The font size of the bit numbers (`byteFontSize`).
    pub byte_font_size: String,
    /// The colour of the leading bit number of a block (`startByteColor`).
    pub start_byte_color: String,
    /// The colour of the trailing bit number of a block (`endByteColor`).
    pub end_byte_color: String,
    /// The colour of a block's label (`labelColor`).
    pub label_color: String,
    /// The font size of a block's label (`labelFontSize`).
    pub label_font_size: String,
    /// The colour of the title (`titleColor`).
    pub title_color: String,
    /// The font size of the title (`titleFontSize`).
    pub title_font_size: String,
    /// The stroke of a block (`blockStrokeColor`).
    pub block_stroke_color: String,
    /// The stroke width of a block (`blockStrokeWidth`), written as it is.
    pub block_stroke_width: String,
    /// The fill of a block (`blockFillColor`).
    pub block_fill_color: String,
}

impl Options {
    /// The options of the theme variables: the defaults of `defaultPacketStyleOptions`, with each
    /// option that `themeVariables.packet` sets written in its place.
    pub(super) fn of(variables: &Value) -> Self {
        let packet = variables.get("packet").and_then(Value::as_object);
        let read = |name: &str, default: &str| {
            packet
                .and_then(|options| options.get(name))
                .map_or_else(|| default.to_owned(), template_text)
        };
        Self {
            byte_font_size: read("byteFontSize", "10px"),
            start_byte_color: read("startByteColor", "black"),
            end_byte_color: read("endByteColor", "black"),
            label_color: read("labelColor", "black"),
            label_font_size: read("labelFontSize", "12px"),
            title_color: read("titleColor", "black"),
            title_font_size: read("titleFontSize", "14px"),
            block_stroke_color: read("blockStrokeColor", "black"),
            block_stroke_width: read("blockStrokeWidth", "1"),
            block_fill_color: read("blockFillColor", "#efefef"),
        }
    }
}

/// The value of an option as the template literal of `styles.ts` writes it: `${value}`. A number is
/// written as JavaScript writes it, an array as its items joined with commas (`null` items empty),
/// and an object as `[object Object]`.
fn template_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => crate::svg::number(*number),
        Value::Bool(flag) => flag.to_string(),
        Value::Null => "null".to_owned(),
        Value::Array(items) => items
            .iter()
            .map(|item| match item {
                Value::Null => String::new(),
                other => template_text(other),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

/// The rules of the packet, after the base rules of the diagram and before its look rules, as the
/// reference's `styles` places them. The colours are custom properties on the id, and the rules
/// refer to them with `var(…)`.
pub(super) fn own_rules(id: &str, options: &Options) -> String {
    format!(
        "#{id}{{\
--packetStartByteColor:{start};\
--packetEndByteColor:{end};\
--packetLabelColor:{label};\
--packetTitleColor:{title};\
--packetBlockStrokeColor:{stroke};\
--packetBlockFillColor:{fill};}}\
#{id} .packetByte{{font-size:{byte_size};}}\
#{id} .packetByte.start{{fill:var(--packetStartByteColor);}}\
#{id} .packetByte.end{{fill:var(--packetEndByteColor);}}\
#{id} .packetLabel{{fill:var(--packetLabelColor);font-size:{label_size};}}\
#{id} .packetTitle{{fill:var(--packetTitleColor);font-size:{title_size};}}\
#{id} .packetBlock{{stroke:var(--packetBlockStrokeColor);stroke-width:{stroke_width};fill:var(--packetBlockFillColor);}}",
        start = options.start_byte_color,
        end = options.end_byte_color,
        label = options.label_color,
        title = options.title_color,
        stroke = options.block_stroke_color,
        fill = options.block_fill_color,
        byte_size = options.byte_font_size,
        label_size = options.label_font_size,
        title_size = options.title_font_size,
        stroke_width = options.block_stroke_width,
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn variables_with_packet(packet: Value) -> Value {
        let mut variables = BTreeMap::new();
        variables.insert("packet".to_owned(), packet);
        Value::Object(variables)
    }

    #[test]
    fn without_a_packet_key_the_defaults_are_used() {
        let options = Options::of(&Value::empty_object());
        assert_eq!(options.block_fill_color, "#efefef");
        assert_eq!(options.block_stroke_width, "1");
        assert_eq!(options.title_font_size, "14px");
    }

    #[test]
    fn the_packet_key_of_the_theme_variables_overrides_the_defaults() {
        let mut packet = BTreeMap::new();
        packet.insert("blockFillColor".to_owned(), Value::String("red".to_owned()));
        packet.insert("blockStrokeWidth".to_owned(), Value::Number(2.0));
        let options = Options::of(&variables_with_packet(Value::Object(packet)));
        assert_eq!(options.block_fill_color, "red");
        assert_eq!(options.block_stroke_width, "2");
        assert_eq!(options.label_color, "black");
    }

    #[test]
    fn a_packet_key_that_is_not_an_object_merges_nothing() {
        let options = Options::of(&variables_with_packet(Value::String("red".to_owned())));
        assert_eq!(options.block_fill_color, "#efefef");
    }

    #[test]
    fn own_rules_name_the_custom_properties_and_scope_them_to_the_id() {
        let options = Options::of(&Value::empty_object());
        let css = own_rules("packet-diagram", &options);
        assert!(css.starts_with("#packet-diagram{--packetStartByteColor:black;"));
        assert!(css.contains("#packet-diagram .packetBlock{stroke:var(--packetBlockStrokeColor);stroke-width:1;fill:var(--packetBlockFillColor);}"));
    }
}
