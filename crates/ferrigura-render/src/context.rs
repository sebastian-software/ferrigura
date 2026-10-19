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
/// classes, states and entity relations) are not globals: they are part of the renderer's section
/// (`unified::Section`).
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
    use std::fs;
    use std::path::{Path, PathBuf};

    /// The files below `src` that name the whole configuration outside their tests: the two
    /// constructors that the facade calls (`Globals::of` here, `unified::Section::of`), and a module
    /// that only the tests compile.
    const WHOLE_CONFIG: [&str; 3] = [
        "context.rs",
        "unified/mod.rs",
        "unified/shapes/test_support.rs",
    ];

    /// No renderer takes the whole configuration: each reads its section, the globals and the theme
    /// of its [`Context`](super::Context) (`docs-port/typed-views.md`). The theme by name cannot be
    /// read at all: the helpers of `variables` are compiled for the tests only.
    #[test]
    fn no_renderer_takes_the_whole_configuration() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        collect(&root, &mut files);
        assert!(files.len() > 100, "the sources were not found");
        for file in files {
            let name = file
                .strip_prefix(&root)
                .unwrap_or(&file)
                .to_string_lossy()
                .replace('\\', "/");
            if WHOLE_CONFIG.contains(&name.as_str()) {
                continue;
            }
            let source = fs::read_to_string(&file).unwrap_or_default();
            for (number, line) in outside_tests(&source) {
                assert!(
                    !names_whole_config(line),
                    "{name}:{number} names the whole configuration"
                );
            }
        }
    }

    /// The `.rs` files below a directory.
    fn collect(dir: &Path, files: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect(&path, files);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }

    /// The lines of a source, numbered from 1, without the items marked `#[cfg(test)]` and without
    /// comments. An item ends where its braces close, or at its `;` when it has none.
    fn outside_tests(source: &str) -> Vec<(usize, &str)> {
        let mut lines = Vec::new();
        let mut skipping = false;
        let mut opened = false;
        let mut depth = 0_i64;
        for (index, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if !skipping && trimmed == "#[cfg(test)]" {
                (skipping, opened, depth) = (true, false, 0);
                continue;
            }
            if skipping {
                for c in line.chars() {
                    match c {
                        '{' => (opened, depth) = (true, depth + 1),
                        '}' => depth -= 1,
                        _ => {}
                    }
                }
                if (opened && depth <= 0) || (!opened && trimmed.ends_with(';')) {
                    skipping = false;
                }
                continue;
            }
            if !trimmed.starts_with("//") {
                lines.push((index + 1, line));
            }
        }
        lines
    }

    /// Whether a line names `MermaidConfig` itself, not a longer name such as `MermaidConfigElk`.
    fn names_whole_config(line: &str) -> bool {
        line.match_indices("MermaidConfig").any(|(at, name)| {
            line[at + name.len()..]
                .chars()
                .next()
                .is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_'))
        })
    }
}
