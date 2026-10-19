//! The colours of the venn diagram, one set per colour scheme: the palette of the circles
//! (`venn1` to `venn8`, `vennRenderer.ts` 41-50), the colours derived from the palette, and the
//! text colours. A `colorScheme: auto` render has two sets; the light set gives the presentation
//! attributes and the dark set the `prefers-color-scheme: dark` rules (`docs-port/decisions/0007`).

use ferrigura_model::theme::ThemeTokens;
use ferrigura_model::theme::khroma::{darken, is_dark, lighten};

use crate::RenderError;
use crate::context::ThemeView;

/// The colours of one scheme that the venn diagram reads from its theme.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Scheme {
    /// `themeColors`: `venn1` to `venn8` with the falsy ones removed, in order (`filter(Boolean)`).
    palette: Vec<String>,
    /// `primaryColor`: the circle colour when there is no palette and no style.
    primary: Option<String>,
    /// `primaryTextColor ?? textColor`: `defaultTextColor`.
    default_text: Option<String>,
    /// `vennTitleTextColor || titleColor`: the title's fill.
    title: Option<String>,
    /// `vennSetTextColor`: the text of the text nodes.
    set_text: Option<String>,
    /// `isDark(background || '#f4f4f4')`: the circles' labels are lightened on a dark background.
    dark: bool,
}

/// The `background` the reference assumes when the theme gives none (`vennRenderer.ts` 120).
const DEFAULT_BACKGROUND: &str = "#f4f4f4";

/// The amount the circle labels are lightened or darkened by (`vennRenderer.ts` 164).
const LABEL_SHADE: f64 = 30.0;

impl Scheme {
    /// The colours of one set of theme tokens.
    ///
    /// # Errors
    ///
    /// A background the colour functions cannot read is a render error, as the reference throws
    /// on it (`isDark`, `vennRenderer.ts` 120).
    pub fn of(tokens: &ThemeTokens) -> Result<Self, RenderError> {
        let palette = [
            &tokens.venn.get(1),
            &tokens.venn.get(2),
            &tokens.venn.get(3),
            &tokens.venn.get(4),
            &tokens.venn.get(5),
            &tokens.venn.get(6),
            &tokens.venn.get(7),
            &tokens.venn.get(8),
        ]
        .into_iter()
        .filter(|token| token.truthy())
        .filter_map(|token| token.text())
        .collect();
        let background =
            non_empty(tokens.background.text()).unwrap_or_else(|| DEFAULT_BACKGROUND.to_owned());
        Ok(Self {
            palette,
            primary: tokens.primary_color.text(),
            default_text: tokens
                .primary_text_color
                .text()
                .or_else(|| tokens.text_color.text()),
            title: non_empty(tokens.venn_title_text_color.text())
                .or_else(|| non_empty(tokens.title_color.text())),
            set_text: tokens.venn_set_text_color.text(),
            dark: is_dark(&background).map_err(|error| RenderError::Config(error.to_string()))?,
        })
    }

    /// `vennTitleTextColor || titleColor`, when either is set.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// `vennSetTextColor`, for the text nodes (`styles.ts`, `.venn-text-node`).
    pub fn set_text(&self) -> Option<&str> {
        self.set_text.as_deref()
    }

    /// `vennSetTextColor ?? defaultTextColor`: the intersections' labels (`vennRenderer.ts` 205).
    pub fn intersection_text(&self) -> Option<String> {
        self.set_text.clone().or_else(|| self.default_text.clone())
    }

    /// `baseColor` of the circle with the index `index` among the circles (`vennRenderer.ts`
    /// 128-129): its style's fill when it has one, else the palette colour of the index, else the
    /// primary colour. `None` when none of them is set, as the reference's `undefined`.
    pub fn base(&self, index: usize, custom_fill: Option<&str>) -> Option<String> {
        custom_fill
            .map(str::to_owned)
            .or_else(|| {
                (!self.palette.is_empty()).then(|| self.palette[index % self.palette.len()].clone())
            })
            .or_else(|| self.primary.clone())
    }

    /// The label colour of a circle (`vennRenderer.ts` 163-164): the style's colour when it has
    /// one, else the base colour lightened on a dark background and darkened on a light one.
    ///
    /// # Errors
    ///
    /// The colour functions refuse a base colour they cannot read, and so does the render.
    pub fn label(&self, base: Option<&str>) -> Result<String, RenderError> {
        let base = base.unwrap_or("");
        let shaded = if self.dark {
            lighten(base, LABEL_SHADE)
        } else {
            darken(base, LABEL_SHADE)
        };
        shaded.map_err(|error| RenderError::Config(error.to_string()))
    }
}

/// The two schemes of a render: the light (or only) one, and the dark one of `colorScheme: auto`.
pub(super) struct Schemes {
    pub light: Scheme,
    pub dark: Option<Scheme>,
}

impl Schemes {
    /// The schemes of a theme view.
    ///
    /// # Errors
    ///
    /// See [`Scheme::of`].
    pub fn new(theme: ThemeView<'_>) -> Result<Self, RenderError> {
        Ok(Self {
            light: Scheme::of(theme.light)?,
            dark: theme.dark.map(Scheme::of).transpose()?,
        })
    }
}

/// A text that is set and not empty, as the reference's `||` reads it.
pub(super) fn non_empty(text: Option<String>) -> Option<String> {
    text.filter(|text| !text.is_empty())
}
