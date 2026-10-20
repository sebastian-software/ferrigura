//! The bundled font registry.
//!
//! Each (family, weight, style) choice names one face. The font files are compiled into the
//! crate, so measurement and embedding never depend on fonts installed on the viewer's system.

use std::fmt;
use std::sync::OnceLock;

use harfrust::font::Font;

use crate::error::TextError;

/// A bundled font family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontFamily {
    /// Lexica Ultralegible, the default text font, based on Atkinson Hyperlegible.
    Sans,
    /// Atkinson Hyperlegible Mono, for code.
    Mono,
    /// The arrows block (U+2190-21FF) of Noto Sans Math. It is never chosen for text directly: it
    /// is a fallback of every text, which takes the characters the text's own family lacks (see
    /// [`crate::fallback_runs`]).
    Arrows,
    /// Noto Sans Symbols 2, cut to the text-default symbols of U+2600-27BF. A fallback of every text,
    /// after the arrows; it never holds an emoji-presentation code point.
    TextSymbols,
    /// Twemoji Mozilla (COLRv0), cut to the code points with emoji presentation that it covers. A
    /// fallback of every text, after the text symbols.
    Emoji,
}

impl FontFamily {
    /// The suffix of the family names that embedded subsets get, such as `fg-…-sans`. The arrows
    /// keep the suffix `symbols` they have had since they were added, so the names of the diagrams
    /// that draw arrows do not change.
    #[must_use]
    pub fn css_suffix(self) -> &'static str {
        match self {
            Self::Sans => "sans",
            Self::Mono => "mono",
            Self::Arrows => "symbols",
            Self::TextSymbols => "text-symbols",
            Self::Emoji => "emoji",
        }
    }
}

/// The weight of a face, as CSS `font-weight` names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontWeight {
    /// Weight 400, the `normal` of CSS.
    Normal,
    /// Weight 700, the `bold` of CSS.
    Bold,
}

impl FontWeight {
    /// The numeric CSS `font-weight` of this weight.
    #[must_use]
    pub fn css_value(self) -> u16 {
        match self {
            Self::Normal => 400,
            Self::Bold => 700,
        }
    }
}

/// The slant of a face, as CSS `font-style` names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontStyle {
    /// The upright face.
    Normal,
    /// The italic face, which is a design of its own and not a slanted upright.
    Italic,
}

impl FontStyle {
    /// The CSS `font-style` keyword of this style.
    #[must_use]
    pub fn css_keyword(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Italic => "italic",
        }
    }
}

/// One bundled font file and the choice it answers to.
pub struct Face {
    family: FontFamily,
    weight: FontWeight,
    style: FontStyle,
    file_name: &'static str,
    /// Parsed on first use, then shared by every measurement of this face.
    font: OnceLock<Option<Font>>,
    data: &'static [u8],
}

impl fmt::Debug for Face {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Face")
            .field("family", &self.family)
            .field("weight", &self.weight)
            .field("style", &self.style)
            .field("file_name", &self.file_name)
            .finish_non_exhaustive()
    }
}

impl Face {
    /// The parsed font, read from the bundled data on first use.
    pub(crate) fn font(&self) -> Result<&Font, TextError> {
        self.font
            .get_or_init(|| Font::new(self.data, 0))
            .as_ref()
            .ok_or(TextError::InvalidFont)
    }

    /// Whether the face has a glyph for `character`, from its Unicode `cmap`. A character mapped to
    /// glyph 0 counts as unmapped, as the `cmap` lookup reports it.
    #[must_use]
    pub fn covers(&self, character: char) -> bool {
        self.font()
            .is_ok_and(|font| font.charmap().map_unicode(character).is_some())
    }

    /// The family this face belongs to.
    #[must_use]
    pub fn family(&self) -> FontFamily {
        self.family
    }

    /// The weight of this face.
    #[must_use]
    pub fn weight(&self) -> FontWeight {
        self.weight
    }

    /// The style of this face.
    #[must_use]
    pub fn style(&self) -> FontStyle {
        self.style
    }

    /// The file name the font was bundled from, such as `LexicaUltralegible-Bold.ttf`.
    #[must_use]
    pub fn file_name(&self) -> &'static str {
        self.file_name
    }

    /// The complete TrueType file.
    #[must_use]
    pub fn data(&self) -> &'static [u8] {
        self.data
    }
}

static LEXICA_REGULAR: Face = Face {
    family: FontFamily::Sans,
    weight: FontWeight::Normal,
    style: FontStyle::Normal,
    file_name: "LexicaUltralegible-Regular.ttf",
    font: OnceLock::new(),
    data: include_bytes!("../fonts/LexicaUltralegible-Regular.ttf"),
};

static LEXICA_ITALIC: Face = Face {
    family: FontFamily::Sans,
    weight: FontWeight::Normal,
    style: FontStyle::Italic,
    file_name: "LexicaUltralegible-Italic.ttf",
    font: OnceLock::new(),
    data: include_bytes!("../fonts/LexicaUltralegible-Italic.ttf"),
};

static LEXICA_BOLD: Face = Face {
    family: FontFamily::Sans,
    weight: FontWeight::Bold,
    style: FontStyle::Normal,
    file_name: "LexicaUltralegible-Bold.ttf",
    font: OnceLock::new(),
    data: include_bytes!("../fonts/LexicaUltralegible-Bold.ttf"),
};

static LEXICA_BOLD_ITALIC: Face = Face {
    family: FontFamily::Sans,
    weight: FontWeight::Bold,
    style: FontStyle::Italic,
    file_name: "LexicaUltralegible-BoldItalic.ttf",
    font: OnceLock::new(),
    data: include_bytes!("../fonts/LexicaUltralegible-BoldItalic.ttf"),
};

static ATKINSON_MONO_REGULAR: Face = Face {
    family: FontFamily::Mono,
    weight: FontWeight::Normal,
    style: FontStyle::Normal,
    file_name: "AtkinsonHyperlegibleMono-Regular.ttf",
    font: OnceLock::new(),
    data: include_bytes!("../fonts/AtkinsonHyperlegibleMono-Regular.ttf"),
};

static ATKINSON_MONO_ITALIC: Face = Face {
    family: FontFamily::Mono,
    weight: FontWeight::Normal,
    style: FontStyle::Italic,
    file_name: "AtkinsonHyperlegibleMono-Italic.ttf",
    font: OnceLock::new(),
    data: include_bytes!("../fonts/AtkinsonHyperlegibleMono-Italic.ttf"),
};

static ATKINSON_MONO_BOLD: Face = Face {
    family: FontFamily::Mono,
    weight: FontWeight::Bold,
    style: FontStyle::Normal,
    file_name: "AtkinsonHyperlegibleMono-Bold.ttf",
    font: OnceLock::new(),
    data: include_bytes!("../fonts/AtkinsonHyperlegibleMono-Bold.ttf"),
};

static ATKINSON_MONO_BOLD_ITALIC: Face = Face {
    family: FontFamily::Mono,
    weight: FontWeight::Bold,
    style: FontStyle::Italic,
    file_name: "AtkinsonHyperlegibleMono-BoldItalic.ttf",
    font: OnceLock::new(),
    data: include_bytes!("../fonts/AtkinsonHyperlegibleMono-BoldItalic.ttf"),
};

/// The arrows block of Noto Sans Math 3.000 (U+2190-21FF). It has one style, so every weight and
/// style of the arrows family is this face; a browser synthesises the bold or the slant of it where
/// the text asks for one, and the advances are the same (`tests/fallback.rs`).
static NOTO_MATH_ARROWS: Face = Face {
    family: FontFamily::Arrows,
    weight: FontWeight::Normal,
    style: FontStyle::Normal,
    file_name: "NotoSansMath-Regular-Arrows.ttf",
    font: OnceLock::new(),
    data: include_bytes!("../fonts/NotoSansMath-Regular-Arrows.ttf"),
};

/// Noto Sans Symbols 2 cut to the text-default code points of U+2600-27BF. Like the arrows, one
/// style for every choice of its family (`tests/fallback.rs`).
static NOTO_SANS_SYMBOLS2_TEXT: Face = Face {
    family: FontFamily::TextSymbols,
    weight: FontWeight::Normal,
    style: FontStyle::Normal,
    file_name: "NotoSansSymbols2-Regular-TextSymbols.ttf",
    font: OnceLock::new(),
    data: include_bytes!("../fonts/NotoSansSymbols2-Regular-TextSymbols.ttf"),
};

/// Twemoji Mozilla 0.7.0 (COLRv0) cut to the emoji-presentation code points it covers. Its colour
/// layers are drawn by the browser from the COLR and CPAL tables, which a subset keeps.
static TWEMOJI_EMOJI: Face = Face {
    family: FontFamily::Emoji,
    weight: FontWeight::Normal,
    style: FontStyle::Normal,
    file_name: "TwemojiMozilla-COLRv0-Emoji.ttf",
    font: OnceLock::new(),
    data: include_bytes!("../fonts/TwemojiMozilla-COLRv0-Emoji.ttf"),
};

/// Every bundled face: the four Lexica Ultralegible styles, the four Atkinson Hyperlegible Mono
/// styles, then the arrows, the text symbols and the emoji.
pub static FACES: [&Face; 11] = [
    &LEXICA_REGULAR,
    &LEXICA_ITALIC,
    &LEXICA_BOLD,
    &LEXICA_BOLD_ITALIC,
    &ATKINSON_MONO_REGULAR,
    &ATKINSON_MONO_ITALIC,
    &ATKINSON_MONO_BOLD,
    &ATKINSON_MONO_BOLD_ITALIC,
    &NOTO_MATH_ARROWS,
    &NOTO_SANS_SYMBOLS2_TEXT,
    &TWEMOJI_EMOJI,
];

/// The bundled face for one (family, weight, style) choice.
///
/// Every combination has a face. The fallback families have one face for all their choices, as
/// described on each face; the text families have a face for each of their styles.
#[must_use]
pub fn face(family: FontFamily, weight: FontWeight, style: FontStyle) -> &'static Face {
    match (family, weight, style) {
        (FontFamily::Sans, FontWeight::Normal, FontStyle::Normal) => &LEXICA_REGULAR,
        (FontFamily::Sans, FontWeight::Normal, FontStyle::Italic) => &LEXICA_ITALIC,
        (FontFamily::Sans, FontWeight::Bold, FontStyle::Normal) => &LEXICA_BOLD,
        (FontFamily::Sans, FontWeight::Bold, FontStyle::Italic) => &LEXICA_BOLD_ITALIC,
        (FontFamily::Mono, FontWeight::Normal, FontStyle::Normal) => &ATKINSON_MONO_REGULAR,
        (FontFamily::Mono, FontWeight::Normal, FontStyle::Italic) => &ATKINSON_MONO_ITALIC,
        (FontFamily::Mono, FontWeight::Bold, FontStyle::Normal) => &ATKINSON_MONO_BOLD,
        (FontFamily::Mono, FontWeight::Bold, FontStyle::Italic) => &ATKINSON_MONO_BOLD_ITALIC,
        (FontFamily::Arrows, _, _) => &NOTO_MATH_ARROWS,
        (FontFamily::TextSymbols, _, _) => &NOTO_SANS_SYMBOLS2_TEXT,
        (FontFamily::Emoji, _, _) => &TWEMOJI_EMOJI,
    }
}
