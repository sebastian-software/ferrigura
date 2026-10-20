//! The styles of a tree view: `styles.ts` (`defaultTreeViewDiagramStyles`, lines 5-14, and the rules
//! of `styles`, lines 16-56).
//!
//! The reference's `getStyles` passes `{...config.themeVariables, theme, look, svgId}` to the tree
//! view styles, and `styles` reads its `treeView` key from that object (`mermaidAPI.ts` 327-333).
//! So the options are `themeVariables.treeView`, merged over the defaults the way `cleanAndMerge`
//! merges them (lodash `merge`: an option that is set, `null` included, replaces the default).
//!
//! Each colour is a custom property declared on the diagram's id, and each coloured element also
//! carries its colour as a presentation attribute (`docs-port/decisions/0007`), as the packet
//! renderer does. The sizes and the weights stay in the rules, which name the classes the elements
//! carry; a class that a user gives a node (`:::name`) takes these rules too, as in the reference.

use ferrigura_model::theme::Token;
use ferrigura_model::theme::tokens::TreeViewTokens;
use ferrigura_model::value::Value;

use crate::RenderError;

/// The options of the tree view styles after the merge with the defaults, each as the template of
/// `styles.ts` writes it.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Options {
    /// The font size of the labels and of the descriptions (`labelFontSize`).
    pub label_font_size: String,
    /// The colour of a label (`labelColor`).
    pub label_color: String,
    /// The colour of a connector and a trunk line (`lineColor`).
    pub line_color: String,
    /// The colour of an icon, which the icons take as `currentColor` (`iconColor`).
    pub icon_color: String,
    /// The colour of a description (`descriptionColor`).
    pub description_color: String,
    /// The fill of a highlight rectangle (`highlightBg`).
    pub highlight_bg: String,
    /// The stroke of a highlight rectangle (`highlightStroke`).
    pub highlight_stroke: String,
}

impl Options {
    /// The options of the theme variables: the defaults, with each option that
    /// `themeVariables.treeView` sets written in its place.
    pub(super) fn of(group: Option<&TreeViewTokens>) -> Self {
        let read = |token: fn(&TreeViewTokens) -> &Token, default: &str| {
            group
                .and_then(|options| token(options).value())
                .map_or_else(|| default.to_owned(), value_text)
        };
        Self {
            label_font_size: read(|options| &options.label_font_size, "16px"),
            label_color: read(|options| &options.label_color, "black"),
            line_color: read(|options| &options.line_color, "black"),
            icon_color: read(|options| &options.icon_color, "#546e7a"),
            description_color: read(|options| &options.description_color, "#6a9955"),
            highlight_bg: read(|options| &options.highlight_bg, "rgba(255, 193, 7, 0.15)"),
            highlight_stroke: read(|options| &options.highlight_stroke, "#ffc107"),
        }
    }

    /// The font size of the labels in pixels, as the browser reads the `font-size` of the sheet. A
    /// size in pixels is the only unit the port measures.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::Unsupported`] for a size in another unit or without one, which the
    /// port does not measure yet.
    pub(super) fn label_px(&self) -> Result<f64, RenderError> {
        css_px(&self.label_font_size).ok_or_else(|| {
            RenderError::Unsupported(format!(
                "the tree view's labelFontSize {:?} is not a size in pixels",
                self.label_font_size
            ))
        })
    }
}

/// The value of an option as the template literal of `styles.ts` writes it: `${value}`. A number is
/// written as JavaScript writes it, an array as its items joined with commas (`null` items empty),
/// and an object as `[object Object]`.
fn value_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => crate::svg::number(*number),
        Value::Bool(flag) => flag.to_string(),
        Value::Null => "null".to_owned(),
        Value::Array(items) => items
            .iter()
            .map(|item| match item {
                Value::Null => String::new(),
                other => value_text(other),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

/// The pixel count of a CSS `font-size` written as a number of pixels (`20px`, ` 14.5PX `), or
/// `None` for any other text.
fn css_px(text: &str) -> Option<f64> {
    let number = text
        .trim()
        .strip_suffix("px")
        .or_else(|| text.trim().strip_suffix("PX"))?;
    let valid = !number.is_empty()
        && number
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | '+' | '-' | 'e' | 'E'));
    let size: f64 = if valid {
        number.parse().ok()?
    } else {
        return None;
    };
    size.is_finite().then_some(size)
}

/// The rules of the tree view, after the base rules of the diagram: the colours as custom properties
/// on the id, and the rules that name the classes of the elements. The order is the order of
/// `styles.ts` (lines 29-55).
pub(super) fn own_rules(id: &str, options: &Options) -> String {
    format!(
        "#{id}{{\
--treeViewLabelColor:{label};\
--treeViewLineColor:{line};\
--treeViewIconColor:{icon};\
--treeViewDescriptionColor:{description};\
--treeViewHighlightBg:{bg};\
--treeViewHighlightStroke:{stroke};}}\
#{id} .treeView-node-label{{font-size:{size};fill:var(--treeViewLabelColor);white-space:pre;}}\
#{id} .treeView-node-dir{{font-weight:bold;}}\
#{id} .treeView-node-line{{stroke:var(--treeViewLineColor);}}\
#{id} .treeView-node-icon{{color:var(--treeViewIconColor);}}\
#{id} .treeView-node-description{{font-size:{size};fill:var(--treeViewDescriptionColor);font-style:italic;white-space:pre;}}\
#{id} .treeView-highlight-bg{{fill:var(--treeViewHighlightBg);stroke:var(--treeViewHighlightStroke);stroke-width:1;}}",
        label = options.label_color,
        line = options.line_color,
        icon = options.icon_color,
        description = options.description_color,
        bg = options.highlight_bg,
        stroke = options.highlight_stroke,
        size = options.label_font_size,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_size_in_pixels_is_read_and_others_are_not() {
        assert_eq!(css_px("16px"), Some(16.0));
        assert_eq!(css_px(" 20.5PX "), Some(20.5));
        assert_eq!(css_px("20"), None);
        assert_eq!(css_px("2em"), None);
        assert_eq!(css_px("inf"), None);
        assert_eq!(css_px("px"), None);
    }
}
