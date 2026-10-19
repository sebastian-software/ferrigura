//! The configuration the C4 renderer reads: `getRequiredConfig('c4')` (`c4Renderer.ts:566`), which
//! is the `C4DiagramConfig` of the schema (`config.schema.yaml:507`), with the element palette and
//! the element fonts of `c4ShapeAdapter.ts` and `styles.js`.

use ferrigura_model::config::C4DiagramConfig;
use ferrigura_model::value::Value;

/// The font size of the schema when a font size is not a number (`C4DiagramConfig`'s defaults).
const DEFAULT_FONT_SIZE: f64 = 14.0;

/// A font of the configuration: the family as CSS reads it, the size in pixels and the weight as
/// CSS reads it (`normal`, `bold` or a number).
#[derive(Debug, Clone, PartialEq)]
pub struct Font {
    /// The CSS font family, quotes included.
    pub family: String,
    /// The font size in pixels.
    pub size: f64,
    /// The CSS font weight.
    pub weight: String,
}

impl Font {
    /// The weight as a number, as the measurement takes it: `normal` is 400 and `bold` is 700.
    #[must_use]
    pub fn weight_number(&self) -> f64 {
        match self.weight.as_str() {
            "normal" => 400.0,
            "bold" => 700.0,
            other => other.trim().parse::<f64>().unwrap_or(400.0),
        }
    }
}

/// The configured look of one element type: its label font, and its fill and border (the
/// `<type>_bg_color` and `<type>_border_color` of the schema, `c4ShapeAdapter.ts:142-146`).
#[derive(Debug, Clone, PartialEq)]
pub struct ElementConf {
    /// The font of the element's label (`styles.js`).
    pub font: Font,
    /// The fill of the element, unless `UpdateElementStyle` sets one.
    pub bg: String,
    /// The border of the element, unless `UpdateElementStyle` sets one.
    pub border: String,
}

/// The C4 configuration, with the values the renderer reads, in the reference's units.
#[derive(Debug, Clone)]
pub struct Conf {
    /// The schema section, for the element table.
    section: C4DiagramConfig,
    /// `diagramMarginX`: the margin of the screen and the gap between boundaries.
    pub diagram_margin_x: f64,
    /// `diagramMarginY`.
    pub diagram_margin_y: f64,
    /// `c4ShapeMargin`: the gap around each shape and boundary.
    pub shape_margin: f64,
    /// `c4ShapePadding`: the padding of each shape's label.
    pub shape_padding: f64,
    /// `width`: the width of an element, the minimum box.
    pub width: f64,
    /// `nextLinePaddingX`: added to the x of a wrapped row.
    pub next_line_padding_x: f64,
    /// `wrap` of the C4 section: whether element labels wrap, and whether boundary and relation texts
    /// may wrap together with the diagram's own `wrap` flag.
    pub wrap: bool,
    /// `useMaxWidth`: the root is sized to its `max-width`.
    pub use_max_width: bool,
    /// The boundary font (`boundaryFont*`).
    pub boundary: Font,
    /// The relation font (`messageFont*`).
    pub message: Font,
}

impl Conf {
    /// The renderer's configuration of the `c4` section of the configuration.
    #[must_use]
    pub fn new(section: &C4DiagramConfig) -> Self {
        let section = section.clone();
        Self {
            diagram_margin_x: section.diagram_margin_x as f64,
            diagram_margin_y: section.diagram_margin_y as f64,
            shape_margin: section.c4_shape_margin as f64,
            shape_padding: section.c4_shape_padding as f64,
            width: section.width as f64,
            next_line_padding_x: section.next_line_padding_x,
            wrap: section.wrap,
            use_max_width: section.use_max_width,
            boundary: font(
                &section.boundary_font_family,
                &section.boundary_font_size,
                &section.boundary_font_weight,
            ),
            message: font(
                &section.message_font_family,
                &section.message_font_size,
                &section.message_font_weight,
            ),
            section,
        }
    }

    /// The configuration of an element type (`person`, `external_system_db` and so on), or `None` for
    /// a name the palette does not have (`C4_ELEMENT_TYPES`, `c4ShapeAdapter.ts:117-130`).
    #[must_use]
    pub fn element(&self, type_name: &str) -> Option<ElementConf> {
        let c = &self.section;
        let (family, size, weight, bg, border) = match type_name {
            "person" => (
                &c.person_font_family,
                &c.person_font_size,
                &c.person_font_weight,
                &c.person_bg_color,
                &c.person_border_color,
            ),
            "external_person" => (
                &c.external_person_font_family,
                &c.external_person_font_size,
                &c.external_person_font_weight,
                &c.external_person_bg_color,
                &c.external_person_border_color,
            ),
            "system" => (
                &c.system_font_family,
                &c.system_font_size,
                &c.system_font_weight,
                &c.system_bg_color,
                &c.system_border_color,
            ),
            "external_system" => (
                &c.external_system_font_family,
                &c.external_system_font_size,
                &c.external_system_font_weight,
                &c.external_system_bg_color,
                &c.external_system_border_color,
            ),
            "system_db" => (
                &c.system_db_font_family,
                &c.system_db_font_size,
                &c.system_db_font_weight,
                &c.system_db_bg_color,
                &c.system_db_border_color,
            ),
            "external_system_db" => (
                &c.external_system_db_font_family,
                &c.external_system_db_font_size,
                &c.external_system_db_font_weight,
                &c.external_system_db_bg_color,
                &c.external_system_db_border_color,
            ),
            "system_queue" => (
                &c.system_queue_font_family,
                &c.system_queue_font_size,
                &c.system_queue_font_weight,
                &c.system_queue_bg_color,
                &c.system_queue_border_color,
            ),
            "external_system_queue" => (
                &c.external_system_queue_font_family,
                &c.external_system_queue_font_size,
                &c.external_system_queue_font_weight,
                &c.external_system_queue_bg_color,
                &c.external_system_queue_border_color,
            ),
            "container" => (
                &c.container_font_family,
                &c.container_font_size,
                &c.container_font_weight,
                &c.container_bg_color,
                &c.container_border_color,
            ),
            "external_container" => (
                &c.external_container_font_family,
                &c.external_container_font_size,
                &c.external_container_font_weight,
                &c.external_container_bg_color,
                &c.external_container_border_color,
            ),
            "container_db" => (
                &c.container_db_font_family,
                &c.container_db_font_size,
                &c.container_db_font_weight,
                &c.container_db_bg_color,
                &c.container_db_border_color,
            ),
            "external_container_db" => (
                &c.external_container_db_font_family,
                &c.external_container_db_font_size,
                &c.external_container_db_font_weight,
                &c.external_container_db_bg_color,
                &c.external_container_db_border_color,
            ),
            "container_queue" => (
                &c.container_queue_font_family,
                &c.container_queue_font_size,
                &c.container_queue_font_weight,
                &c.container_queue_bg_color,
                &c.container_queue_border_color,
            ),
            "external_container_queue" => (
                &c.external_container_queue_font_family,
                &c.external_container_queue_font_size,
                &c.external_container_queue_font_weight,
                &c.external_container_queue_bg_color,
                &c.external_container_queue_border_color,
            ),
            "component" => (
                &c.component_font_family,
                &c.component_font_size,
                &c.component_font_weight,
                &c.component_bg_color,
                &c.component_border_color,
            ),
            "external_component" => (
                &c.external_component_font_family,
                &c.external_component_font_size,
                &c.external_component_font_weight,
                &c.external_component_bg_color,
                &c.external_component_border_color,
            ),
            "component_db" => (
                &c.component_db_font_family,
                &c.component_db_font_size,
                &c.component_db_font_weight,
                &c.component_db_bg_color,
                &c.component_db_border_color,
            ),
            "external_component_db" => (
                &c.external_component_db_font_family,
                &c.external_component_db_font_size,
                &c.external_component_db_font_weight,
                &c.external_component_db_bg_color,
                &c.external_component_db_border_color,
            ),
            "component_queue" => (
                &c.component_queue_font_family,
                &c.component_queue_font_size,
                &c.component_queue_font_weight,
                &c.component_queue_bg_color,
                &c.component_queue_border_color,
            ),
            "external_component_queue" => (
                &c.external_component_queue_font_family,
                &c.external_component_queue_font_size,
                &c.external_component_queue_font_weight,
                &c.external_component_queue_bg_color,
                &c.external_component_queue_border_color,
            ),
            _ => return None,
        };
        Some(ElementConf {
            font: font(family, size, weight),
            bg: bg.clone(),
            border: border.clone(),
        })
    }
}

/// A font from its family, size and weight options.
fn font(family: &str, size: &Option<Value>, weight: &Option<Value>) -> Font {
    Font {
        family: family.to_owned(),
        size: size.as_ref().map_or(DEFAULT_FONT_SIZE, size_of),
        weight: weight
            .as_ref()
            .map_or_else(|| "normal".to_owned(), weight_of),
    }
}

/// A font size as the reference reads it: a number as it is, a text by its leading number (`14px`).
fn size_of(value: &Value) -> f64 {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(leading_number))
        .unwrap_or(DEFAULT_FONT_SIZE)
}

/// A font weight as CSS text: the text as it is, or a number as JavaScript writes it.
fn weight_of(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other
            .as_f64()
            .map_or_else(|| "normal".to_owned(), crate::svg::number),
    }
}

/// The leading number of a CSS length, as `parseFloat` reads it.
#[must_use]
pub fn leading_number(text: &str) -> Option<f64> {
    let trimmed = text.trim_start();
    let end = trimmed
        .char_indices()
        .take_while(|(index, c)| {
            c.is_ascii_digit() || *c == '.' || (*index == 0 && (*c == '-' || *c == '+'))
        })
        .map(|(index, c)| index + c.len_utf8())
        .last()?;
    trimmed[..end].parse::<f64>().ok()
}
