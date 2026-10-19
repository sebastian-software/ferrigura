//! The colours and sizes a gitGraph reads from one set of theme tokens.
//!
//! Only the redux themes ship (M6-PLAN, themes), so the colours are the ones `styles.js` writes for
//! a colour theme (`redux-color`, `redux-dark-color`): the branch colours come from
//! `borderColorArray`, and colour 0 is the node border. The reference chooses the dark variants by
//! the theme's name (`DARK_THEMES`); ferrigura passes that choice in as `dark`.

use ferrigura_model::theme::ThemeTokens;

use crate::svg::css_text;

/// The colours of one theme set, and the sizes the gitGraph uses from it.
#[derive(Debug, Clone, PartialEq)]
pub struct Palette {
    /// Whether the set is a dark theme.
    pub dark: bool,
    node_border: String,
    main_bkg: String,
    text_color: String,
    line: String,
    tag_label_color: String,
    borders: Vec<String>,
    /// `THEME_COLOR_LIMIT`: the number of branch colours, twelve in the redux themes.
    pub colour_limit: usize,
    /// `strokeWidth`, as the stylesheet writes it.
    pub stroke_width: String,
    /// `noteFontWeight`, as the stylesheet writes it.
    pub note_font_weight: String,
    /// The size of the root text, `fontSize`, in pixels: the branch labels are set in it.
    pub font_size: f64,
    /// `commitLabelFontSize` in pixels.
    pub commit_label_font_size: f64,
    /// `tagLabelFontSize` in pixels.
    pub tag_label_font_size: f64,
    /// `dropShadow`, the filter of the tag labels.
    pub drop_shadow: String,
    /// `filterColor`, the flood colour of the drop shadow.
    pub filter_color: String,
    /// Whether the theme paints its backgrounds with a gradient (`useGradient`).
    pub use_gradient: bool,
    /// `gradientStart`.
    pub gradient_start: String,
    /// `gradientStop`.
    pub gradient_stop: String,
}

impl Palette {
    /// The palette of a set of theme tokens. `dark` is the theme's darkness, as the reference takes
    /// it from the theme's name.
    #[must_use]
    pub fn new(tokens: &ThemeTokens, dark: bool) -> Self {
        Self {
            dark,
            node_border: tokens.node_border.text().unwrap_or_default(),
            main_bkg: tokens.main_bkg.text().unwrap_or_default(),
            text_color: tokens.text_color.text().unwrap_or_default(),
            line: tokens
                .commit_line_color
                .text()
                .or_else(|| tokens.line_color.text())
                .unwrap_or_default(),
            tag_label_color: tokens.tag_label_color.text().unwrap_or_default(),
            borders: tokens.border_color_array.texts(),
            colour_limit: tokens
                .theme_color_limit
                .text()
                .and_then(|limit| limit.parse().ok())
                .unwrap_or(12),
            stroke_width: tokens.stroke_width.text().unwrap_or_default(),
            note_font_weight: tokens.note_font_weight.text().unwrap_or_default(),
            font_size: tokens.font_size.px().unwrap_or(16.0),
            commit_label_font_size: tokens.commit_label_font_size.px().unwrap_or(10.0),
            tag_label_font_size: tokens.tag_label_font_size.px().unwrap_or(10.0),
            drop_shadow: tokens.drop_shadow.text().unwrap_or_default(),
            filter_color: tokens.filter_color.text().unwrap_or_default(),
            use_gradient: tokens.use_gradient.truthy(),
            gradient_start: tokens.gradient_start.text().unwrap_or_default(),
            gradient_stop: tokens.gradient_stop.text().unwrap_or_default(),
        }
    }

    /// The node border, which is also colour 0 of the branches.
    #[must_use]
    pub fn node_border(&self) -> &str {
        &self.node_border
    }

    /// The main background: the fill of merges and of the highlight's inner square.
    #[must_use]
    pub fn main_bkg(&self) -> &str {
        &self.main_bkg
    }

    /// The text colour: the title and the highlight of tags.
    #[must_use]
    pub fn text_color(&self) -> &str {
        &self.text_color
    }

    /// The colour of the branch lines: `commitLineColor`, else `lineColor`.
    #[must_use]
    pub fn line(&self) -> &str {
        &self.line
    }

    /// The text colour of the tags.
    #[must_use]
    pub fn tag_label_color(&self) -> &str {
        &self.tag_label_color
    }

    /// The colour of branch `index`, `borderColorArray[index]` (colour 0 is the node border). The
    /// array wraps around, as `i % borderColorArray.length` does.
    #[must_use]
    pub fn border(&self, index: usize) -> String {
        if index == 0 {
            return self.node_border.clone();
        }
        if self.borders.is_empty() {
            return String::new();
        }
        self.borders[index % self.borders.len()].clone()
    }

    /// The fill of a branch label's background: the main background in a dark theme, and in
    /// colour 0; the branch colour otherwise.
    #[must_use]
    pub fn label_fill(&self, index: usize) -> String {
        if index == 0 || self.dark {
            return self.main_bkg.clone();
        }
        self.border(index)
    }

    /// The colour of a cherry-pick's eyes and of the line through them: white in a light theme,
    /// black in a dark one.
    #[must_use]
    pub fn eye(&self) -> &'static str {
        if self.dark { "#000000" } else { "#fff" }
    }

    /// The custom properties that carry the colours, as `--name: value;` declarations. The rules
    /// of `style.rs` refer to them by name.
    #[must_use]
    pub fn declarations(&self) -> String {
        let mut out = String::new();
        let mut push = |name: &str, value: &str| {
            out.push_str(&format!("--{name}:{};", css_text(value)));
        };
        push("nodeBorder", &self.node_border);
        push("mainBkg", &self.main_bkg);
        push("textColor", &self.text_color);
        push("gitLine", &self.line);
        push("tagLabelColor", &self.tag_label_color);
        push("gitEye", self.eye());
        for index in 1..self.colour_limit {
            push(&format!("gitBorder{index}"), &self.border(index));
            push(&format!("gitLabelFill{index}"), &self.label_fill(index));
        }
        out
    }
}
