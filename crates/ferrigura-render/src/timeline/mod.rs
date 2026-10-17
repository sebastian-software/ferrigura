//! The timeline diagram: a port of `diagrams/timeline/timelineRenderer.ts` (the horizontal layout),
//! `timelineRendererVertical.ts` (the `TD` direction), `svgDraw.js` (the nodes and their wrapping)
//! and `styles.js`. The research is in `docs-port/timeline/NOTES.md`, section 5.
//!
//! The layouts are in `horizontal.rs` and `vertical.rs`; a node (`drawNode`) and its wrapped text are
//! in `node.rs` and `text.rs`; the style sheet is in `style.rs`. Only the redux themes ship
//! (`docs-port/M6-PLAN.md`), so the classic branch of `drawNode` and `defaultBkg` is not ported: a
//! theme that is not shipped is drawn with a redux one, and the reference's redux branch is the one
//! that is drawn.

mod horizontal;
mod node;
mod style;
mod text;
mod vertical;

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::timeline::TimelineDiagram;
use ferrigura_text::{
    EmbeddedFamily, Face, FontFamily, FontStyle, FontWeight, Run, TextBox, embed_runs, face,
    text_box, x_height,
};

use crate::RenderError;
use crate::a11y;
use crate::common::style::base_style_sheet_with;
use crate::common::text::{collapse_white_space, collapsed_width};
use crate::common::viewbox::{BBox, setup_graph_viewbox};
use crate::common::{Element, decode_entities};
use crate::svg::{escape, number};
use crate::theme::Themes;
use crate::variables::{px, text_of, texts_of, truthy};

/// The XML namespace of the root element.
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// What the timeline renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct TimelineInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a TimelineDiagram,
    /// The effective configuration, in the timeline diagram's scope.
    pub config: &'a MermaidConfig,
    /// The colours and sizes of the theme.
    pub themes: Themes<'a>,
    /// The id of the SVG element. It scopes every selector and prefixes every element id, so it
    /// must be a CSS identifier.
    pub id: &'a str,
}

/// Renders the diagram to an SVG document.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read or subset.
pub fn render(input: &TimelineInput<'_>) -> Result<String, RenderError> {
    let diagram = input.diagram;
    let cx = Context::new(input)?;
    let mut canvas = Canvas {
        head: vec![Element::new("g"), Element::new("g")],
        ..Canvas::default()
    };

    let vertical = diagram.direction == "TD";
    if vertical {
        vertical::draw(&cx, &mut canvas, diagram)?;
    } else {
        horizontal::draw(&cx, &mut canvas, diagram)?;
    }

    // The fonts: the glyphs of every text the diagram draws, in the regular and the bold face.
    let texts = drawn_texts(diagram);
    let runs = [
        Run {
            text: &texts,
            face: cx.regular,
        },
        Run {
            text: &texts,
            face: cx.node_face,
        },
    ];
    let families: Vec<EmbeddedFamily> = embed_runs(&runs)?;
    let font_family = families.first().map_or_else(
        || text_of(input.themes.primary(), "fontFamily").unwrap_or_else(|| "sans-serif".to_owned()),
        |family| format!("\"{}\"", family.name),
    );
    let style = base_style_sheet_with(
        input.id,
        input.themes,
        &font_family,
        &cx.css_font_size,
        &families,
        &style::own_rules(&cx),
    );

    // The marker of the arrows. The vertical renderer calls `initGraphics` without the id, so its
    // marker is `undefined-arrowhead`, and its arrows refer to `arrowhead`: no arrow is drawn
    // (`docs-port/timeline/NOTES.md` 5.12, R1).
    let marker_id = if vertical {
        "undefined-arrowhead".to_owned()
    } else {
        format!("{}-arrowhead", input.id)
    };
    let mut defs = Element::new("defs").child(
        Element::new("marker")
            .with("id", marker_id)
            .with("refX", "5")
            .with("refY", "2")
            .with("markerWidth", "6")
            .with("markerHeight", "4")
            .with("orient", "auto")
            .child(Element::new("path").with("d", "M 0,0 V 4 L6,2 Z")),
    );
    for element in canvas.defs {
        defs = defs.child(element);
    }

    // `setupGraphViewbox`: the box of everything drawn, with the padding around it.
    let bounds = canvas.bounds.unwrap_or_else(zero_box);
    let mut root = Element::new("svg")
        .with("id", input.id)
        .with("width", "100%")
        .with("xmlns", SVG_NAMESPACE);
    setup_graph_viewbox(&mut root, bounds, cx.padding, cx.use_max_width);
    // The accessible title and description are not drawn: `mermaidAPI` finds no getter on the timeline
    // database, and the renderer's own call is commented out (NOTES 5.12, R7). The root only has
    // its role and its diagram type.
    root.set("role", a11y::SVG_ROLE);
    root.set("aria-roledescription", "timeline");

    if let Some(lead) = canvas.lead {
        root = root.child(lead);
    }
    root = root.child(Element::new("style").raw_text(style));
    for group in canvas.head {
        root = root.child(group);
    }
    root = root.child(defs);
    for element in canvas.body {
        root = root.child(element);
    }
    Ok(root.render())
}

/// The state of one render that the layouts share: the configuration, the fonts and the colours.
pub(super) struct Context<'a> {
    /// The id of the SVG element: it prefixes the ids and the selectors.
    pub svg_id: &'a str,
    /// `timeline.leftMargin` (150 by default).
    pub left_margin: f64,
    /// `timeline.padding`, or 50 when it is not set.
    pub padding: f64,
    /// `timeline.useMaxWidth`.
    pub use_max_width: bool,
    /// `timeline.disableMulticolor`.
    pub disable_multicolor: bool,
    /// Whether the look is `neo`: the drop shadow and the `data-look` attribute.
    pub neo: bool,
    /// Whether the dark variant is drawn, as `theme.includes('dark')` reads it.
    pub dark: bool,
    /// Whether the theme gets a gradient: `useGradient` in the neo look, and not in the `neutral` theme.
    pub gradient: bool,
    /// `gradientStart` and `gradientStop` of the primary scheme.
    pub gradient_start: String,
    /// See [`Context::gradient_start`].
    pub gradient_stop: String,
    /// The top-level `fontSize` of the configuration, in pixels: the heights of nodes use it.
    pub node_font_size: f64,
    /// The CSS font size of the text: the theme's `fontSize`, as the stylesheet writes it.
    pub css_font_size: String,
    /// The CSS font size in pixels, the size the text is measured at.
    pub text_size: f64,
    /// The regular face: the text of the virtual nodes, which sit outside the `section-N` groups.
    pub regular: &'static Face,
    /// The bold face, which the title is set in.
    pub bold: &'static Face,
    /// The face of the node text: the theme's `fontWeight` (600 in the redux themes) takes the bold face.
    pub node_face: &'static Face,
    /// The ascent of the text box of a line, rounded (`text_box`).
    pub ascent: f64,
    /// The descent of the text box of a line, rounded.
    pub descent: f64,
    /// Half the x-height: `dominant-baseline: middle` puts the middle of the x-height on the position.
    pub middle: f64,
    /// The size of the title, `4ex` of the regular face.
    pub title_size: f64,
    /// The number of colour slots, `THEME_COLOR_LIMIT` (12 by default).
    pub limit: usize,
    /// `fontWeight` of the theme, as the stylesheet writes it (600 in the redux themes).
    pub font_weight: String,
    /// The colours of the primary scheme.
    pub primary: Palette,
    /// The colours of the dark scheme, when both schemes are written.
    pub secondary: Option<Palette>,
}

/// The colours of one section slot: the fill of its nodes and the stroke of their border.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Colours {
    /// The fill of the nodes.
    pub fill: String,
    /// The stroke of the nodes.
    pub stroke: String,
}

/// The colours of one scheme: the two colours every slot uses, and the colours of the slots, which
/// are computed when they are read.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Palette {
    /// `nodeBorder`.
    pub node_border: String,
    /// `mainBkg`.
    pub main_bkg: String,
    /// Whether the scheme is dark. A dark scheme fills every node with `mainBkg`.
    pub dark: bool,
    /// `borderColorArray`.
    border: Vec<String>,
}

impl Palette {
    /// The palette of a set of theme variables (`styles.js` `genReduxSections`, `colorThemeGate.ts`).
    fn new(variables: &ferrigura_model::value::Value, dark: bool) -> Self {
        Self {
            node_border: text_of(variables, "nodeBorder").unwrap_or_default(),
            main_bkg: text_of(variables, "mainBkg").unwrap_or_default(),
            dark,
            border: texts_of(variables, "borderColorArray"),
        }
    }

    /// The colours of slot `index`: `borderColorArray[index % length]` is the stroke, and the fill is
    /// that colour too, or `mainBkg` in a dark scheme. With no array, the stroke is `nodeBorder` and
    /// the fill `mainBkg`, as the reference takes the undefined colour (`genReduxSections`).
    pub fn colours(&self, index: usize) -> Colours {
        let slot =
            (!self.border.is_empty()).then(|| self.border[index % self.border.len()].clone());
        let fill = match (&slot, self.dark) {
            (Some(colour), false) => colour.clone(),
            _ => self.main_bkg.clone(),
        };
        let stroke = slot.unwrap_or_else(|| self.node_border.clone());
        Colours { fill, stroke }
    }
}

impl<'a> Context<'a> {
    fn new(input: &TimelineInput<'a>) -> Result<Self, RenderError> {
        let config = input.config;
        let timeline = &config.timeline;
        let themes = input.themes;
        let variables = themes.primary();
        let theme = config.theme.as_str();

        let text_size = px(variables, "fontSize").unwrap_or(16.0);
        let weight = text_of(variables, "fontWeight")
            .and_then(|text| text.trim().parse::<f64>().ok())
            .unwrap_or(400.0);
        let regular = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
        let bold = face(FontFamily::Sans, FontWeight::Bold, FontStyle::Normal);
        let node_face = if weight > 500.0 { bold } else { regular };
        let TextBox {
            ascent, descent, ..
        } = text_box(node_face, text_size)?;
        let middle = x_height(node_face, text_size)? / 2.0;
        let title_size = 4.0 * x_height(regular, text_size)?;

        let limit = text_of(variables, "THEME_COLOR_LIMIT")
            .and_then(|text| text.trim().parse::<f64>().ok())
            .filter(|limit| limit.is_finite() && *limit >= 1.0)
            .map_or(12, |limit| limit.floor() as usize);
        // A theme name that contains `dark` is the dark variant (`drawNode`'s `isDark`). With two
        // schemes the primary one is the light set, and the dark set is dark.
        let primary_dark = matches!(themes, Themes::Single(_)) && theme.contains("dark");
        let primary = Palette::new(variables, primary_dark);
        let secondary = match themes {
            Themes::Schemes { dark, .. } => Some(Palette::new(dark, true)),
            Themes::Single(_) => None,
        };

        Ok(Self {
            svg_id: input.id,
            left_margin: timeline.left_margin as f64,
            padding: timeline.padding.unwrap_or(50.0),
            use_max_width: timeline.use_max_width,
            disable_multicolor: timeline.disable_multicolor,
            neo: config.look == "neo",
            dark: primary_dark,
            gradient: truthy(variables, "useGradient")
                && config.look == "neo"
                && theme != "neutral",
            gradient_start: text_of(variables, "gradientStart").unwrap_or_default(),
            gradient_stop: text_of(variables, "gradientStop").unwrap_or_default(),
            node_font_size: config.font_size,
            css_font_size: format!("{}px", number(text_size)),
            text_size,
            regular,
            bold,
            node_face,
            ascent,
            descent,
            middle,
            title_size,
            limit,
            font_weight: text_of(variables, "fontWeight").unwrap_or_else(|| "400".to_owned()),
            primary,
            secondary,
        })
    }
}

/// The boxes of the drawing: the groups and rules that the layouts append in order, and the box
/// `getBBox` gives the root once they are drawn.
#[derive(Debug, Default)]
pub(super) struct Canvas {
    /// The two empty groups before the defs.
    pub head: Vec<Element>,
    /// The elements of `defs` after the arrow marker: the drop shadow and the gradient.
    pub defs: Vec<Element>,
    /// The drawing, in document order.
    pub body: Vec<Element>,
    /// An element that the vertical layout moves to the front of the root (`lower()`).
    pub lead: Option<Element>,
    /// The union of the boxes drawn so far, in the root's user units.
    pub bounds: Option<BBox>,
    /// `nodeCount` of `svgDraw.js`: the number of node paths drawn, for their ids.
    pub node_count: usize,
    /// Whether the drop shadow of the neo look is in `defs` already (`drawNode` adds it once).
    pub filter_added: bool,
}

impl Canvas {
    /// Adds a box to the box of the drawing.
    pub fn include(&mut self, bounds: BBox) {
        self.bounds = Some(match self.bounds {
            None => bounds,
            Some(so_far) => union(so_far, bounds),
        });
    }

    /// Adds a box, if there is one.
    pub fn include_some(&mut self, bounds: Option<BBox>) {
        if let Some(bounds) = bounds {
            self.include(bounds);
        }
    }
}

/// The union of two boxes. A box with no width or height still counts, as the reference's
/// `getBBox` counts a horizontal rule.
pub(super) fn union(a: BBox, b: BBox) -> BBox {
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    let right = (a.x + a.width).max(b.x + b.width);
    let bottom = (a.y + a.height).max(b.y + b.height);
    BBox {
        x,
        y,
        width: right - x,
        height: bottom - y,
    }
}

/// The box moved by a translation.
pub(super) fn translated(bounds: BBox, dx: f64, dy: f64) -> BBox {
    BBox {
        x: bounds.x + dx,
        y: bounds.y + dy,
        ..bounds
    }
}

/// The box of a line from one point to another.
pub(super) fn line_box(x1: f64, y1: f64, x2: f64, y2: f64) -> BBox {
    BBox {
        x: x1.min(x2),
        y: y1.min(y2),
        width: (x1 - x2).abs(),
        height: (y1 - y2).abs(),
    }
}

/// The box of an empty drawing, as `getBBox` gives one: the origin, with no size.
pub(super) fn zero_box() -> BBox {
    BBox {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
    }
}

/// The markup of a text the diagram draws. The reference escapes the text when it serialises the
/// SVG and then decodes the entities over the whole markup (`mermaidAPI.ts` 359), so the escape comes
/// first and the decode after it.
pub(super) fn markup(text: &str) -> String {
    decode_entities(&escape(text))
}

/// The text a reader sees for an encoded text: its markup with the entities read, as the browser
/// shows it. The boxes of `getBBox` are those of the glyphs drawn, so they are measured on this text,
/// while the wrapping measures the encoded text, as the reference does while it lays out (NOTES 5.12,
/// R11). An entity the markup does not name, such as `&foo;`, stays as it is.
pub(super) fn shown(text: &str) -> String {
    let markup = markup(text);
    let mut out = String::with_capacity(markup.len());
    let mut rest = markup.as_str();
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        let Some(end) = rest.find(';').filter(|end| *end <= 12) else {
            break;
        };
        let entity = &rest[1..end];
        let read = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "#39" => Some('\''),
            numeric if numeric.starts_with('#') => {
                numeric[1..].parse::<u32>().ok().and_then(char::from_u32)
            }
            _ => None,
        };
        match read {
            Some(character) => {
                out.push(character);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The title: the text the timeline writes with `x` as given, the baseline at `y = 20`, in `4ex`
/// (the x-height of the regular face at the CSS size, `timelineRenderer.ts` 209-216). It is set in
/// the bold face. Returns the element and the box `getBBox` gives it.
pub(super) fn title(
    cx: &Context<'_>,
    x: f64,
    text: &str,
) -> Result<(Element, Option<BBox>), RenderError> {
    let shown_text = shown(text);
    let width = collapsed_width(cx.bold, &shown_text, cx.title_size)?;
    let TextBox {
        ascent, descent, ..
    } = text_box(cx.bold, cx.title_size)?;
    let element = Element::new("text")
        .with("x", number(x))
        .with("font-size", "4ex")
        .with("font-weight", "bold")
        .with("y", "20")
        .raw_text(markup(text));
    // A title with no glyph, such as one blank, has no box (as a node text with no glyph has none).
    let bounds = (!collapse_white_space(&shown_text).is_empty()).then_some(BBox {
        x,
        y: 20.0 - ascent,
        width,
        height: ascent + descent,
    });
    Ok((element, bounds))
}

/// Every text the diagram shows, as a reader sees it: the glyphs the embedded subsets must hold. A
/// character written as an entity is a character of its own here, not the entity's letters.
fn drawn_texts(diagram: &TimelineDiagram) -> String {
    let mut texts = vec![shown(&diagram.title)];
    texts.extend(diagram.sections.iter().map(|text| shown(text)));
    for period in &diagram.periods {
        texts.push(shown(&period.period));
        texts.extend(period.events.iter().map(|text| shown(text)));
    }
    texts.join(" ")
}

/// `translate(x, y)` as the reference writes it in a `transform` attribute.
pub(super) fn translate(x: f64, y: f64) -> String {
    format!("translate({}, {})", number(x), number(y))
}
