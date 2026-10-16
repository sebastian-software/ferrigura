//! The info diagram: a port of `packages/mermaid/src/diagrams/info/infoRenderer.ts`. The research is in
//! `docs-port/timeline/NOTES.md`, section 8.
//!
//! The reference draws the version of Mermaid as `v<version>` at (100, 40), centred, in 32 px, in a
//! root of 100 by 400 that uses the full width up to 400 px. The version is `injected.version`
//! (`infoDb.ts`), a build constant: [`MERMAID_VERSION`] is the version of the reference's
//! `packages/mermaid/package.json`, and the text is the same for every diagram of that build.

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::info::InfoDiagram;
use ferrigura_text::{FontFamily, FontStyle, FontWeight, Run, embed_runs, face};

use crate::RenderError;
use crate::a11y;
use crate::common::Element;
use crate::common::style::base_style_sheet;
use crate::common::viewbox::configure_svg_size;
use crate::theme::Themes;
use crate::variables::text_of;

/// The version the reference writes: `packages/mermaid/package.json` at the commit of this port.
pub const MERMAID_VERSION: &str = "12.2.0";

/// The XML namespace of the root element.
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// What the info renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct InfoInput<'a> {
    /// The parsed diagram. The info diagram has no content of its own: its title and accessibility
    /// texts are not drawn by the reference.
    pub diagram: &'a InfoDiagram,
    /// The effective configuration, in the info diagram's scope.
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
/// Returns an error if the bundled font cannot be read or subset.
pub fn render(input: &InfoInput<'_>) -> Result<String, RenderError> {
    let text = format!("v{MERMAID_VERSION}");
    let sans = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
    let families = embed_runs(&[Run {
        text: &text,
        face: sans,
    }])?;
    let variables = input.themes.primary();
    let font_family = families.first().map_or_else(
        || text_of(variables, "fontFamily").unwrap_or_else(|| "sans-serif".to_owned()),
        |family| format!("\"{}\"", family.name),
    );
    let font_size = text_of(variables, "fontSize").unwrap_or_else(|| "16px".to_owned());
    let style = base_style_sheet(input.id, input.themes, &font_family, &font_size, &families);

    // The attributes in the order the reference sets them: `mermaidAPI` writes the id, the width and
    // the namespace, `infoRenderer` the size (which adds the style), and the accessibility roles.
    let mut root = Element::new("svg")
        .with("id", input.id)
        .with("width", "100%")
        .with("xmlns", SVG_NAMESPACE);
    configure_svg_size(&mut root, 100.0, 400.0, true);
    root.set("role", a11y::SVG_ROLE);
    root.set("aria-roledescription", "info");

    let version = Element::new("text")
        .with("x", "100")
        .with("y", "40")
        .with("class", "version")
        .with("font-size", "32")
        .with("style", "text-anchor: middle;")
        .text(text);
    let root = root
        .child(Element::new("style").raw_text(style))
        // The empty group `mermaidAPI` appends to every SVG before the renderer draws.
        .child(Element::new("g"))
        .child(Element::new("g").child(version));
    Ok(root.render())
}
