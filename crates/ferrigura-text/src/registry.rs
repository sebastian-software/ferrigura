//! Host-owned fonts, resolved by CSS family name, weight, and style.
//!
//! A resolved face owns its font bytes and fallback faces through [`Arc`]. Borrowing that face for
//! a [`crate::Run`] or [`crate::Word`] does not require a static font or keep the registry alive.

use std::borrow::{Borrow, Cow};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::{Arc, OnceLock, PoisonError, RwLock};

use read_fonts::FontRef;
use read_fonts::types::Tag;

use crate::embed::{EmbeddedSubset, Fnv1a};
use crate::fonts::{FaceMeta, FontData, SharedFaceMeta};
use crate::measure::ShapePlans;
use crate::{Face, FontFamily, FontStyle, FontWeight, TextError};

const MAX_RESOLVED_CACHE_ENTRIES: usize = 256;

trait ResolveKeyRef {
    fn parts(&self) -> (&str, u16, FontStyle);
}

impl PartialEq for dyn ResolveKeyRef + '_ {
    fn eq(&self, other: &Self) -> bool {
        self.parts() == other.parts()
    }
}

impl Eq for dyn ResolveKeyRef + '_ {}

impl Hash for dyn ResolveKeyRef + '_ {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let (families, weight, style) = self.parts();
        families.hash(state);
        weight.hash(state);
        style.hash(state);
    }
}

struct ResolveKey {
    families: String,
    weight: u16,
    style: FontStyle,
}

impl ResolveKeyRef for ResolveKey {
    fn parts(&self) -> (&str, u16, FontStyle) {
        (&self.families, self.weight, self.style)
    }
}

impl PartialEq for ResolveKey {
    fn eq(&self, other: &Self) -> bool {
        self.parts() == other.parts()
    }
}

impl Eq for ResolveKey {}

impl Hash for ResolveKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let (families, weight, style) = self.parts();
        families.hash(state);
        weight.hash(state);
        style.hash(state);
    }
}

impl<'a> Borrow<dyn ResolveKeyRef + 'a> for ResolveKey {
    fn borrow(&self) -> &(dyn ResolveKeyRef + 'a) {
        self
    }
}

struct ResolveLookup<'a> {
    families: &'a str,
    weight: u16,
    style: FontStyle,
}

impl ResolveKeyRef for ResolveLookup<'_> {
    fn parts(&self) -> (&str, u16, FontStyle) {
        (self.families, self.weight, self.style)
    }
}

#[derive(Default)]
struct ResolveCache(RwLock<HashMap<ResolveKey, Arc<Face>>>);

impl ResolveCache {
    fn clear(&self) {
        self.0
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }
}

impl Clone for ResolveCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl std::fmt::Debug for ResolveCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolveCache").finish_non_exhaustive()
    }
}

/// An explicit set of local fonts and fallback family names. An empty registry has no defaults.
#[derive(Debug, Clone, Default)]
pub struct FontRegistry {
    faces: Vec<Arc<Face>>,
    fallbacks: Vec<String>,
    resolved_cache: ResolveCache,
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
        let data = Arc::new(data.into());
        let font = FontRef::new(data.as_slice()).map_err(|_| TextError::InvalidFont)?;
        if font.table_data(Tag::new(b"SVG ")).is_some() {
            return Err(TextError::Registration(
                "OpenType-SVG fonts are not supported".into(),
            ));
        }
        crate::kern::validate(&font)?;
        let embed_whole = crate::embed::requires_whole_font(&font);
        let whole_embed = embed_whole.then(|| {
            Arc::new(EmbeddedSubset::new(
                Arc::clone(&data),
                weight.css_value(),
                style,
            ))
        });
        let data_fingerprint = crate::embed::fnv1a(data.as_slice());
        let data_len = data.len();
        let mut face = Face {
            family: FontFamily::Registered,
            family_name: Cow::Owned(family),
            weight,
            style,
            file_name: Cow::Borrowed(""),
            font: OnceLock::new(),
            plans: ShapePlans::new(),
            meta: SharedFaceMeta::new(),
            data: FontData::Registered {
                bytes: Arc::clone(&data),
                embed_whole,
                whole_embed,
            },
            fallbacks: Vec::new(),
            variable: OnceLock::new(),
            data_fingerprint: Some(data_fingerprint),
        };
        // Validate the shaper too, before replacing a usable face, and cache parsed table metadata.
        let meta = {
            let shaped_font = face.font()?;
            FaceMeta::from_font_ref(&font, data_len, Some(shaped_font))
        };
        face.meta = SharedFaceMeta::with_meta(meta);
        self.faces.retain(|previous| {
            !(previous
                .family_name()
                .eq_ignore_ascii_case(face.family_name())
                && previous.weight().css_value() == weight.css_value()
                && previous.style() == style)
        });
        self.faces.push(Arc::new(face));
        self.resolved_cache.clear();
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
    /// [`Self::missing_glyphs`] and measured with the primary face's missing glyph. SVG viewers
    /// may choose a system fallback for these characters.
    pub fn set_fallbacks(&mut self, families: impl IntoIterator<Item = impl Into<String>>) {
        self.fallbacks = families.into_iter().map(Into::into).collect();
        self.resolved_cache.clear();
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
        let weight_value = weight.css_value();
        let lookup = ResolveLookup {
            families,
            weight: weight_value,
            style,
        };
        if let Some(hit) = self
            .resolved_cache
            .0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&lookup as &dyn ResolveKeyRef)
            .cloned()
        {
            return Ok(hit);
        }
        let requested = family_names(families);
        let mut requested_faces = requested
            .iter()
            .filter_map(|name| self.resolve_family(name, weight, style));
        // Explicit fallbacks do not silently satisfy an unregistered primary family list.
        let primary = requested_faces.next().ok_or_else(|| {
            TextError::Registration(format!(
                "no registered font matches {families:?}; register a font with FontRegistry::register or register_file"
            ))
        })?;
        let mut resolved = (*primary).clone();
        resolved.variable = OnceLock::new();
        resolved.fallbacks = Vec::new();
        for candidate in requested_faces.chain(
            self.fallbacks
                .iter()
                .filter_map(|name| self.resolve_family(name, weight, style)),
        ) {
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
        let _ = resolved.css_variable();
        let resolved = Arc::new(resolved);
        let mut cache = self
            .resolved_cache
            .0
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        if cache.len() < MAX_RESOLVED_CACHE_ENTRIES {
            cache
                .entry(ResolveKey {
                    families: families.to_owned(),
                    weight: weight_value,
                    style,
                })
                .or_insert_with(|| Arc::clone(&resolved));
        }
        Ok(resolved)
    }

    fn resolve_family(
        &self,
        name: &str,
        weight: FontWeight,
        style: FontStyle,
    ) -> Option<Arc<Face>> {
        let mut has_any = false;
        let mut has_requested = false;
        let mut has_normal = false;
        for face in &self.faces {
            if face.family_name().eq_ignore_ascii_case(name) {
                has_any = true;
                if face.style() == style {
                    has_requested = true;
                    break;
                }
                if face.style() == FontStyle::Normal {
                    has_normal = true;
                }
            }
        }
        if !has_any {
            return None;
        }
        let selected_style = if has_requested {
            style
        } else if has_normal {
            FontStyle::Normal
        } else {
            FontStyle::Italic
        };
        self.faces
            .iter()
            .filter(|face| {
                face.family_name().eq_ignore_ascii_case(name) && face.style() == selected_style
            })
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
        let mut hash = Fnv1a::new();
        let mut faces: Vec<_> = self.faces.iter().collect();
        faces.sort_by_key(|face| {
            (
                face.family_name(),
                face.weight().css_value(),
                face.style().css_keyword(),
            )
        });
        for face in faces {
            hash.update(&(face.family_name().len() as u64).to_le_bytes());
            hash.update(face.family_name().as_bytes());
            hash.update(&face.weight().css_value().to_le_bytes());
            hash.update(face.style().css_keyword().as_bytes());
            hash.update(&(face.data().len() as u64).to_le_bytes());
            hash.update(face.data());
        }
        for family in &self.fallbacks {
            hash.update(&(family.len() as u64).to_le_bytes());
            hash.update(family.as_bytes());
        }
        hash.finish()
    }

    /// Reports each uncovered character once, in text order, with the primary family name.
    ///
    /// Measurement retains uncovered characters in the primary face and uses its `.notdef`
    /// glyph. A viewer may draw a system fallback instead. This explicit diagnostic lets a host
    /// surface warnings without process-global logging or callbacks during layout.
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

/// An uncovered character, measured with the primary face's missing glyph.
/// SVG viewers may draw a system fallback instead; register coverage for matching layout.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FontFamily, face};

    #[test]
    fn resolve_caches_hits_and_invalidates_on_registry_mutations() {
        let mut registry = FontRegistry::new();
        let sans = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal)
            .data()
            .to_vec();
        let bold = face(FontFamily::Sans, FontWeight::Bold, FontStyle::Normal)
            .data()
            .to_vec();
        let arrows = face(FontFamily::Arrows, FontWeight::Normal, FontStyle::Normal)
            .data()
            .to_vec();
        registry
            .register(
                "AppSans",
                sans.clone(),
                FontWeight::Normal,
                FontStyle::Normal,
            )
            .unwrap();
        registry
            .register("AppArrows", arrows, FontWeight::Normal, FontStyle::Normal)
            .unwrap();

        let first = registry
            .resolve("AppSans", FontWeight::Normal, FontStyle::Normal)
            .unwrap();
        let second = registry
            .resolve("AppSans", FontWeight::Number(400), FontStyle::Normal)
            .unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        assert!(first.css_variable().is_some());

        registry.set_fallbacks(["AppArrows"]);
        let with_fallback = registry
            .resolve("AppSans", FontWeight::Normal, FontStyle::Normal)
            .unwrap();
        assert!(!Arc::ptr_eq(&first, &with_fallback));
        assert_eq!(with_fallback.registered_fallbacks().len(), 1);

        registry
            .register("AppSans", bold, FontWeight::Normal, FontStyle::Normal)
            .unwrap();
        let replaced = registry
            .resolve("AppSans", FontWeight::Normal, FontStyle::Normal)
            .unwrap();
        assert!(!Arc::ptr_eq(&with_fallback, &replaced));
        assert_ne!(first.data_fingerprint(), replaced.data_fingerprint());
    }
}
