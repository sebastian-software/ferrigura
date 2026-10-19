//! What a renderer receives (`docs-port/typed-views.md`): its diagram, its configuration section,
//! the global options that it reads, the theme as typed tokens, and the id of the SVG.
//!
//! The facade resolves the globals and the tokens once per resolved view, so a render gets them by
//! reference and nothing below the facade writes to them.

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::theme::ThemeTokens;

/// What a renderer gets: its diagram, the section of its configuration, the globals, the theme and
/// the id. `Section` is the configuration section of the diagram type (`()` for a diagram without
/// one).
pub struct Context<'a, Model, Section> {
    /// The parsed diagram.
    pub diagram: &'a Model,
    /// The section of the configuration that the diagram type reads.
    pub section: &'a Section,
    /// The global options the renderers read.
    pub globals: &'a Globals,
    /// The theme the render draws with.
    pub theme: ThemeView<'a>,
    /// The id of the SVG element. It scopes every selector and prefixes the element ids, so it must
    /// be a CSS identifier.
    pub id: &'a str,
}

/// The global options that renderers read, taken from the top level of the configuration and
/// resolved once per view. The list is closed: a renderer that needs another global value adds it
/// here. Sections of other diagram types (the `flowchart` section that the unified renderer reads for
/// classes, states and entity relations) are not globals; their renderers take them when they move.
#[derive(Debug, Clone, PartialEq)]
pub struct Globals {
    /// `look`: `classic`, `neo` or `handDrawn`, as the configuration gives it.
    pub look: String,
    /// `theme`: the name of the configured theme.
    pub theme: String,
    /// `fontFamily`.
    pub font_family: String,
    /// `fontSize`, in pixels.
    pub font_size: f64,
    /// `securityLevel`.
    pub security_level: String,
    /// `layout`, as the view resolves it: a mindmap without a user's layout uses `cose-bilkent`.
    pub layout: String,
    /// `htmlLabels`, when the configuration sets it.
    pub html_labels: Option<bool>,
}

impl Globals {
    /// The globals of a configuration.
    #[must_use]
    pub fn of(config: &MermaidConfig) -> Self {
        Self {
            look: config.look.clone(),
            theme: config.theme.clone(),
            font_family: config.font_family.clone(),
            font_size: config.font_size,
            security_level: config.security_level.clone(),
            layout: config.layout.clone(),
            html_labels: config.html_labels,
        }
    }
}

/// The theme of a render. `light` gives the presentation attributes and the sizes, and it is the only
/// set of a render with one colour scheme. With both schemes (`colorScheme: auto`), `dark` is set, and
/// it applies under `prefers-color-scheme: dark` (`docs-port/decisions/0007`).
#[derive(Debug, Clone, Copy)]
pub struct ThemeView<'a> {
    /// The tokens of the light scheme, or of the one scheme that the render draws.
    pub light: &'a ThemeTokens,
    /// The tokens of the dark scheme, when the SVG carries both.
    pub dark: Option<&'a ThemeTokens>,
}

#[cfg(test)]
mod tests {
    /// The renderers that take a [`Context`](super::Context). None of them reads a theme variable by
    /// name: they read the tokens, and the helpers of `variables` are for the renderers not yet moved.
    /// A renderer joins this list when it moves.
    const MIGRATED: [(&str, &str); 5] = [
        ("pie", include_str!("pie.rs")),
        ("info", include_str!("info/mod.rs")),
        ("packet", include_str!("packet/mod.rs")),
        ("treemap", include_str!("treemap/mod.rs")),
        ("treemap/style", include_str!("treemap/style.rs")),
    ];

    /// The helpers of `variables`, as a call or a path to them.
    const HELPERS: [&str; 5] = ["variables::", "text_of(", "texts_of(", "truthy(", "px("];

    #[test]
    fn a_migrated_renderer_reads_the_theme_by_token() {
        for (renderer, source) in MIGRATED {
            for (number, line) in source.lines().enumerate() {
                if line.trim_start().starts_with("//") {
                    continue;
                }
                for helper in HELPERS {
                    for (at, _) in line.match_indices(helper) {
                        // A call of the helper stands alone; a longer name or a method of a token does not.
                        let standalone = line[..at]
                            .chars()
                            .next_back()
                            .is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_' || c == '.'));
                        assert!(
                            !standalone,
                            "{renderer}.rs:{} reads the theme by name with {helper}",
                            number + 1
                        );
                    }
                }
            }
        }
    }
}
