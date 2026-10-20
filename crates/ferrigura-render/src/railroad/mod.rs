//! The railroad diagram: a port of `packages/mermaid/src/diagrams/railroad/railroadRenderer.ts`
//! (`draw`, lines 638-666) with its layout ([`layout`]) and its styles ([`style`]). The research is in
//! `docs-port/railroad/NOTES.md`, section 6, and the parser in `docs-port/railroad/PLAN.md`.
//!
//! The four syntaxes share this renderer. The root names the detector of the syntax as the reference
//! does (`aria-roledescription`), which the model does not hold: the section carries it.
//!
//! Nothing is laid out by a library: the sizes come from the measured texts and the configured
//! spacing. The browser measures the text when it shows the SVG, with the fonts the SVG embeds.

pub mod layout;
pub mod style;
pub mod text;

use ferrigura_model::config::RailroadDiagramConfig;
use ferrigura_model::railroad::RailroadDiagram;
use ferrigura_text::{EmbeddedFamily, FontFamily, FontStyle, FontWeight, Run, embed_runs, face};

use crate::RenderError;
use crate::a11y;
use crate::common::Element;
use crate::common::style::base_style_sheet;
use crate::common::viewbox::configure_svg_size;
use crate::context::Context;
use crate::svg::number;
use layout::DrawnText;
use style::Options;
use text::{TextMeasure, shown_text};

/// The XML namespace of the root element.
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// The size of an empty diagram: `configureRailroadSvgSize(svg, { height: 100, width: 200 })`
/// (`railroadRenderer.ts` 650-654).
const EMPTY_WIDTH: f64 = 200.0;
const EMPTY_HEIGHT: f64 = 100.0;

/// What the railroad renderer reads besides the model: the `railroad` section of the configuration,
/// and the detector id of the syntax the text was written in (`railroad`, `railroadEbnf`,
/// `railroadAbnf` or `railroadPeg`), which names the SVG.
#[derive(Debug, Clone, Copy)]
pub struct RailroadSection<'a> {
    /// The `railroad` section of the configuration.
    pub config: &'a RailroadDiagramConfig,
    /// The id of the detector of the text's syntax, from `ferrigura_syntax::railroad::detector_id`.
    pub detector: &'static str,
}

/// Renders the diagram to an SVG document.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read or subset.
pub fn render(
    input: &Context<'_, RailroadDiagram, RailroadSection<'_>>,
) -> Result<String, RenderError> {
    let id = input.id;
    let tokens = input.theme.light;
    let options = Options::new(input.section.config, input.theme);

    let drawing = if input.diagram.rules.is_empty() {
        None
    } else {
        let mut measure = TextMeasure::new(options.font_size);
        Some(layout::draw_rules(
            &input.diagram.rules,
            &options.spacing,
            &options.light,
            &mut measure,
        )?)
    };
    let (width, height) = match &drawing {
        Some(drawing) => (drawing.width, drawing.height),
        None => (EMPTY_WIDTH, EMPTY_HEIGHT),
    };
    let text = drawing
        .as_ref()
        .map_or_else(DrawnText::default, |drawing| drawing.text.clone());

    // The fonts: every character the SVG draws, in the face it is set in (the rule names in bold).
    let mut runs = Vec::new();
    if !text.regular.is_empty() {
        runs.push(Run {
            text: &text.regular,
            face: face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal),
        });
    }
    if !text.bold.is_empty() {
        runs.push(Run {
            text: &text.bold,
            face: face(FontFamily::Sans, FontWeight::Bold, FontStyle::Normal),
        });
    }
    let families: Vec<EmbeddedFamily> = embed_runs(&runs)?;
    let family = families.first().map_or_else(
        || {
            tokens
                .font_family
                .text()
                .unwrap_or_else(|| "sans-serif".to_owned())
        },
        |family| family.css_family.clone(),
    );
    let theme_size = tokens.font_size.text().unwrap_or_else(|| "16px".to_owned());
    let style = base_style_sheet(
        id,
        input.theme,
        &family,
        &theme_size,
        &families,
        &style::own_rules(id, &options, &family),
    );

    // `draw` (`railroadRenderer.ts` 638-666): the class, then the size, then the viewBox, on the root
    // that `mermaidAPI` made with its id, width and namespace.
    let mut root = Element::new("svg")
        .with("id", id)
        .with("width", "100%")
        .with("xmlns", SVG_NAMESPACE)
        .with("class", "railroad-diagram");
    configure_svg_size(&mut root, height, width, options.use_max_width);
    root.set(
        "viewBox",
        format!("0 0 {} {}", number(width), number(height)),
    );

    // `addA11yInfo` (`accessibility.ts`): the title, then the description, are the first children;
    // the root refers to them and names its diagram type.
    let title = input.diagram.acc_title.as_str();
    let description = input.diagram.acc_descr.as_str();
    root.set("role", a11y::SVG_ROLE);
    root.set("aria-roledescription", input.section.detector);
    if !description.is_empty() {
        root.set("aria-describedby", a11y::description_id(id));
    }
    if !title.is_empty() {
        root.set("aria-labelledby", a11y::title_id(id));
    }
    if !title.is_empty() {
        root.push_child(
            Element::new("title")
                .with("id", a11y::title_id(id))
                .text(shown_text(title)),
        );
    }
    if !description.is_empty() {
        root.push_child(
            Element::new("desc")
                .with("id", a11y::description_id(id))
                .text(shown_text(description)),
        );
    }
    root.push_child(Element::new("style").raw_text(style));
    // The empty group `mermaidAPI` appends before the renderer draws.
    root.push_child(Element::new("g"));
    if let Some(drawing) = drawing {
        for rule in drawing.rules {
            root.push_child(rule);
        }
    }
    Ok(root.render())
}
