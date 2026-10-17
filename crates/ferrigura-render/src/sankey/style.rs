//! The colours and rules of a sankey diagram: `styles.js` (lines 1-36). The colours of the outlined
//! labels are custom properties on the diagram's id, and the labels also carry them as presentation
//! attributes (`docs-port/decisions/0007`). Every rule of `styles.js` is kept: each matches an
//! element that the renderer draws.

use ferrigura_model::value::Value;

use crate::theme::Themes;
use crate::variables::text_of;

/// The two colours a sankey takes from the theme.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Palette {
    /// `options.mainBkg || options.background || '#fff'`: the stroke behind the outlined labels.
    pub label_background: String,
    /// `options.textColor`: the colour of the outlined labels' text.
    pub label_text: String,
}

impl Palette {
    /// The palette of one set of theme variables (`styles.js` 10-21).
    pub fn new(variables: &Value) -> Self {
        let label_background = ["mainBkg", "background"]
            .iter()
            .find_map(|name| text_of(variables, name).filter(|text| !text.is_empty()))
            .unwrap_or_else(|| "#fff".to_owned());
        Self {
            label_background,
            label_text: text_of(variables, "textColor").unwrap_or_default(),
        }
    }
}

/// The colours of the schemes: the primary one and, when both are written, the dark one.
pub(super) struct Schemes {
    /// The palette the presentation attributes take.
    pub primary: Palette,
    /// The palette of the dark scheme, under `prefers-color-scheme: dark`.
    pub dark: Option<Palette>,
}

impl Schemes {
    pub fn new(themes: Themes<'_>) -> Self {
        match themes {
            Themes::Single(variables) => Self {
                primary: Palette::new(variables),
                dark: None,
            },
            Themes::Schemes { light, dark } => Self {
                primary: Palette::new(light),
                dark: Some(Palette::new(dark)),
            },
        }
    }
}

/// The custom properties of one scheme, on the diagram's id.
fn tokens(palette: &Palette) -> String {
    format!(
        "--sankeyLabelBackground:{};--sankeyLabelText:{};",
        palette.label_background, palette.label_text
    )
}

/// The rules the sankey adds to the base style sheet, as CSS text: the colour properties of the
/// schemes, then the rules of `styles.js`. The caller escapes the whole sheet once.
pub(super) fn own_rules(id: &str, font_family: &str, schemes: &Schemes) -> String {
    let mut css = match &schemes.dark {
        None => format!("#{id}{{{}}}", tokens(&schemes.primary)),
        Some(dark) => format!(
            "#{id}{{{}}}@media (prefers-color-scheme: dark){{#{id}{{{}}}}}",
            tokens(&schemes.primary),
            tokens(dark)
        ),
    };
    css.push_str(&format!(
        "#{id} .label,#{id} .node-labels{{font-family:{font_family};}}\
#{id} .sankey-label-bg{{stroke:var(--sankeyLabelBackground);stroke-width:4px;stroke-linejoin:round;paint-order:stroke;}}\
#{id} .sankey-label-fg{{fill:var(--sankeyLabelText);}}\
#{id} .node rect{{shape-rendering:crispEdges;}}\
#{id} .link{{fill:none;stroke-opacity:0.5;mix-blend-mode:multiply;}}"
    ));
    css
}
