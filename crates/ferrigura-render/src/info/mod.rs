//! The info diagram: a port of `packages/mermaid/src/diagrams/info/infoRenderer.ts`. The research is in
//! `docs-port/timeline/NOTES.md`, section 8.
//!
//! The reference draws the version of Mermaid as `v<version>` at (100, 40), centred, in 32 px, in a
//! root of 100 by 400 that uses the full width up to 400 px. The version is `injected.version`
//! (`infoDb.ts`), a build constant: [`MERMAID_VERSION`] is the version of the reference's
//! `packages/mermaid/package.json`, and the text is the same for every diagram of that build.

use ferrigura_model::info::InfoDiagram;
use ferrigura_text::{FontFamily, FontStyle, FontWeight, Run, embed_runs, face};

use crate::RenderError;
use crate::a11y;
use crate::common::Element;
use crate::common::style::base_style_sheet;
use crate::common::viewbox::configure_svg_size;
use crate::context::Context;

/// The version the reference writes: `packages/mermaid/package.json` at the commit of this port.
pub const MERMAID_VERSION: &str = "12.2.0";

/// The XML namespace of the root element.
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// Renders the diagram to an SVG document. The info diagram has no configuration section of its own,
/// so its context has the unit section; its title and accessibility texts are not drawn by the
/// reference, so the diagram is not read beyond its type.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read or subset.
pub fn render(input: &Context<'_, InfoDiagram, ()>) -> Result<String, RenderError> {
    let text = format!("v{MERMAID_VERSION}");
    let sans = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
    let families = embed_runs(&[Run {
        text: &text,
        face: sans,
    }])?;
    let tokens = input.theme.light;
    let font_family = families.first().map_or_else(
        || {
            tokens
                .font_family
                .text()
                .unwrap_or_else(|| "sans-serif".to_owned())
        },
        |family| family.css_family.clone(),
    );
    let font_size = tokens.font_size.text().unwrap_or_else(|| "16px".to_owned());
    let style = base_style_sheet(
        input.id,
        input.theme,
        &font_family,
        &font_size,
        &families,
        "",
    );

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
