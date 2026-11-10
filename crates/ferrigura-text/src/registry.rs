//! Host-owned fonts, resolved by CSS family name, weight, and style.
//!
//! A resolved face owns its font bytes and fallback faces through [`Arc`]. Borrowing that face for
//! a [`crate::Run`] or [`crate::Word`] does not require a static font or keep the registry alive.

use std::borrow::Cow;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use read_fonts::FontRef;
use read_fonts::types::Tag;

use crate::fonts::FontData;
use crate::{Face, FontFamily, FontStyle, FontWeight, TextError};

/// An explicit set of local fonts and fallback family names. An empty registry has no defaults.
#[derive(Debug, Clone, Default)]
pub struct FontRegistry {
    faces: Vec<Arc<Face>>,
    fallbacks: Vec<String>,
}

impl FontRegistry {
    /// Creates an empty registry. No system fonts are inspected and no fonts are downloaded.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether no font face has been registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }

    /// Registers one face from owned TrueType or OpenType bytes.
    ///
    /// A repeated family, numeric weight, and style replaces that face. Family names are matched
    /// without ASCII case distinctions. Registration never infers a family from the file's name.
    ///
    /// # Errors
    ///
    /// Rejects empty names, weights outside 1..=1000, invalid font data, and OpenType-SVG fonts.
    pub fn register(
        &mut self,
        family: impl Into<String>,
        data: impl Into<Vec<u8>>,
        weight: FontWeight,
        style: FontStyle,
    ) -> Result<(), TextError> {
        let family = family.into();
        let family = self
            .faces
            .iter()
            .find(|face| face.family_name().eq_ignore_ascii_case(&family))
            .map_or(family.clone(), |face| face.family_name().to_owned());
        if family.trim().is_empty() {
            return Err(TextError::Registration(
                "the font family must not be empty".into(),
            ));
        }
        if !(1..=1000).contains(&weight.css_value()) {
            return Err(TextError::Registration(
                "font weight must be between 1 and 1000".into(),
            ));
        }
        let data = data.into();
        let font = FontRef::new(&data).map_err(|_| TextError::InvalidFont)?;
        if font.table_data(Tag::new(b"SVG ")).is_some() {
            return Err(TextError::Registration(
                "OpenType-SVG fonts are not supported".into(),
            ));
        }
        crate::kern::validate(&font)?;
        let face = Face {
            family: FontFamily::Registered,
            family_name: Cow::Owned(family),
            weight,
            style,
            file_name: Cow::Borrowed(""),
            font: OnceLock::new(),
            data: FontData::Registered(data.into()),
            fallbacks: Vec::new(),
            variable: OnceLock::new(),
        };
        // Validate the shaper too, before replacing a usable face.
        face.font()?;
        self.faces.retain(|previous| {
            !(previous
                .family_name()
                .eq_ignore_ascii_case(face.family_name())
                && previous.weight().css_value() == weight.css_value()
                && previous.style() == style)
        });
        self.faces.push(Arc::new(face));
        Ok(())
    }

    /// Registers a face from a local file. The bytes are read immediately and owned by the registry.
    ///
    /// # Errors
    ///
    /// Reports a file read error or an error from [`Self::register`].
    pub fn register_file(
        &mut self,
        family: impl Into<String>,
        path: impl AsRef<Path>,
        weight: FontWeight,
        style: FontStyle,
    ) -> Result<(), TextError> {
        let path = path.as_ref();
        let data = std::fs::read(path).map_err(|error| {
            TextError::Registration(format!("cannot read font {}: {error}", path.display()))
        })?;
        self.register(family, data, weight, style)
    }

    /// Sets the fallback families after the requested family list, in the given order.
    ///
    /// Names are resolved when [`Self::resolve`] is called, so registration order does not matter.
    /// Unregistered fallback names are skipped. Missing characters are reported by
    /// [`Self::missing_glyphs`] and stay in the primary face's missing glyph.
    pub fn set_fallbacks(&mut self, families: impl IntoIterator<Item = impl Into<String>>) {
        self.fallbacks = families.into_iter().map(Into::into).collect();
    }

    /// Resolves the first registered family in a CSS family list, then the nearest weight in the
    /// requested style. If italic is absent, the upright face is used. Equal weight distances favor
    /// the lighter weight. Remaining registered families form the per-character fallback chain,
    /// followed by the registry's explicit fallback families.
    ///
    /// The returned handle owns every face it uses, independently of this registry.
    ///
    /// # Errors
    ///
    /// Reports an unresolved family list, including how to register a font.
    pub fn resolve(
        &self,
        families: &str,
        weight: FontWeight,
        style: FontStyle,
    ) -> Result<Arc<Face>, TextError> {
        let requested = family_names(families);
        let mut candidates = requested
            .iter()
            .chain(&self.fallbacks)
            .filter_map(|name| self.resolve_family(name, weight, style));
        // Explicit fallbacks do not silently satisfy an unregistered primary family list.
        let primary = requested.iter().find_map(|name| self.resolve_family(name, weight, style))
            .ok_or_else(|| TextError::Registration(format!(
                "no registered font matches {families:?}; register a font with FontRegistry::register or register_file"
            )))?;
        let mut resolved = (*primary).clone();
        resolved.variable = OnceLock::new();
        resolved.fallbacks = Vec::new();
        for candidate in &mut candidates {
            if !candidate
                .family_name()
                .eq_ignore_ascii_case(primary.family_name())
                && !resolved.fallbacks.iter().any(|previous| {
                    previous
                        .family_name()
                        .eq_ignore_ascii_case(candidate.family_name())
                })
            {
                resolved.fallbacks.push(candidate);
            }
        }
        Ok(Arc::new(resolved))
    }

    fn resolve_family(
        &self,
        name: &str,
        weight: FontWeight,
        style: FontStyle,
    ) -> Option<Arc<Face>> {
        let faces: Vec<_> = self
            .faces
            .iter()
            .filter(|face| face.family_name().eq_ignore_ascii_case(name))
            .collect();
        let selected_style = if faces.iter().any(|face| face.style() == style) {
            style
        } else if faces.iter().any(|face| face.style() == FontStyle::Normal) {
            FontStyle::Normal
        } else {
            FontStyle::Italic
        };
        faces
            .into_iter()
            .filter(|face| face.style() == selected_style)
            .min_by_key(|face| {
                (
                    face.weight().css_value().abs_diff(weight.css_value()),
                    face.weight().css_value(),
                )
            })
            .cloned()
    }

    /// A deterministic fingerprint of registered names, designs, bytes, and fallback order.
    ///
    /// This is an identity for render caches and SVG ids, not a cryptographic digest.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        fn add(hash: &mut u64, bytes: &[u8]) {
            for byte in bytes {
                *hash = (*hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        let mut hash = 0xcbf2_9ce4_8422_2325;
        let mut faces: Vec<_> = self.faces.iter().collect();
        faces.sort_by_key(|face| {
            (
                face.family_name(),
                face.weight().css_value(),
                face.style().css_keyword(),
            )
        });
        for face in faces {
            add(&mut hash, &(face.family_name().len() as u64).to_le_bytes());
            add(&mut hash, face.family_name().as_bytes());
            add(&mut hash, &face.weight().css_value().to_le_bytes());
            add(&mut hash, face.style().css_keyword().as_bytes());
            add(&mut hash, &(face.data().len() as u64).to_le_bytes());
            add(&mut hash, face.data());
        }
        for family in &self.fallbacks {
            add(&mut hash, &(family.len() as u64).to_le_bytes());
            add(&mut hash, family.as_bytes());
        }
        hash
    }

    /// Reports each uncovered character once, in text order, with the primary family name.
    ///
    /// Measurement and embedding retain those characters in the primary face, so its `.notdef`
    /// glyph is drawn. This explicit diagnostic lets a host surface warnings without process-global
    /// logging or callbacks during layout.
    #[must_use]
    pub fn missing_glyphs(face: &Face, text: &str) -> Vec<MissingGlyph> {
        let mut missing = Vec::new();
        for character in text.chars() {
            if !character.is_control()
                && !face.covers(character)
                && !face
                    .registered_fallbacks()
                    .iter()
                    .any(|fallback| fallback.covers(character))
                && !missing
                    .iter()
                    .any(|entry: &MissingGlyph| entry.character == character)
            {
                missing.push(MissingGlyph {
                    character,
                    family: face.family_name().to_owned(),
                });
            }
        }
        missing
    }
}

/// An uncovered character, measured and drawn with the primary face's missing glyph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingGlyph {
    /// The uncovered Unicode character.
    pub character: char,
    /// The primary registered family.
    pub family: String,
}

impl std::fmt::Display for MissingGlyph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "font {:?} has no glyph for U+{:04X} ({})",
            self.family, self.character as u32, self.character
        )
    }
}

/// Splits a CSS family list, respecting quoted commas and escapes.
fn family_names(list: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut name = String::new();
    let mut quote = None;
    let mut escaped = false;
    for character in list.chars() {
        if escaped {
            name.push(character);
            escaped = false;
            continue;
        }
        if character == '\\' {
            escaped = true;
            continue;
        }
        if let Some(delimiter) = quote {
            if character == delimiter {
                quote = None;
            } else {
                name.push(character);
            }
        } else if character == '\'' || character == '"' {
            quote = Some(character);
        } else if character == ',' {
            if !name.trim().is_empty() {
                names.push(name.trim().to_owned());
            }
            name.clear();
        } else {
            name.push(character);
        }
    }
    if !name.trim().is_empty() {
        names.push(name.trim().to_owned());
    }
    names
}
