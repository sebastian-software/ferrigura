//! Per-render font resolution and owned handles. No process or thread-local font selection exists.

use std::collections::HashMap;
use std::sync::{Arc, PoisonError, RwLock};

use ferrigura_text::{Face, FontFamily, FontRegistry, FontStyle, FontWeight, TextError};

const MAX_CONTEXT_CACHE_ENTRIES: usize = 64;

type FaceCacheKey = (bool, u16, FontStyle);
type FaceCache = Arc<RwLock<HashMap<FaceCacheKey, FontFace>>>;

/// A face owned by the render, or a compatibility reference to an existing bundled face.
#[derive(Debug, Clone)]
pub enum FontFace {
    /// A bundled compatibility face. This path preserves the default output and allocations.
    Bundled(&'static Face),
    /// A host-supplied face with owned font bytes and fallback faces.
    Registered(Arc<Face>),
}

impl std::ops::Deref for FontFace {
    type Target = Face;
    fn deref(&self) -> &Face {
        match self {
            Self::Bundled(face) => face,
            Self::Registered(face) => face,
        }
    }
}

impl PartialEq for FontFace {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(&**self, &**other)
            || (self.family() == other.family()
                && self.family_name() == other.family_name()
                && self.weight().css_value() == other.weight().css_value()
                && self.style() == other.style()
                && self.data() == other.data()
                && self.css_variable() == other.css_variable())
    }
}

impl FontFace {
    /// Retains a borrowed face in a label or text collector.
    #[must_use]
    pub fn retain(face: &Face) -> Self {
        if face.family() == FontFamily::Registered {
            Self::Registered(Arc::new(face.clone()))
        } else {
            Self::Bundled(ferrigura_text::face(
                face.family(),
                face.weight(),
                face.style(),
            ))
        }
    }

    /// The CSS family reference for this face; custom inputs bind it after subsetting.
    #[must_use]
    pub fn css_reference(&self) -> Option<String> {
        self.css_variable().map(|name| format!("var({name})"))
    }
}

/// The configured family and immutable font inputs of one render or text role.
#[derive(Debug, Clone)]
pub struct FontContext {
    registry: Option<Arc<FontRegistry>>,
    family: String,
    cache: Option<FaceCache>,
}

impl PartialEq for FontContext {
    fn eq(&self, other: &Self) -> bool {
        self.family == other.family
            && match (&self.registry, &other.registry) {
                (None, None) => true,
                (Some(a), Some(b)) => Arc::ptr_eq(a, b) || a.fingerprint() == b.fingerprint(),
                _ => false,
            }
    }
}

impl FontContext {
    /// A font context with the legacy bundled defaults, used by direct renderer callers.
    #[must_use]
    pub fn bundled() -> Self {
        Self {
            registry: None,
            family: String::new(),
            cache: None,
        }
    }

    /// Creates a context from explicitly provided fonts and a configured CSS family list.
    #[must_use]
    pub fn new(registry: Option<&Arc<FontRegistry>>, family: &str) -> Self {
        Self {
            registry: registry.cloned(),
            family: family.to_owned(),
            cache: registry.map(|_| Arc::new(RwLock::new(HashMap::new()))),
        }
    }

    /// Resolves the configured family at the requested numeric weight and style.
    ///
    /// # Errors
    /// Reports an unregistered family with registration guidance.
    pub fn face(&self, weight: FontWeight, style: FontStyle) -> Result<FontFace, TextError> {
        self.resolve_cached(false, &self.family, FontFamily::Sans, weight, style)
    }

    /// Resolves an explicit code family, or the bundled mono face without supplied fonts.
    ///
    /// # Errors
    /// Reports an unregistered code family. Register `monospace` when using that default name.
    pub fn mono(&self, weight: FontWeight, style: FontStyle) -> Result<FontFace, TextError> {
        self.resolve_cached(true, "monospace", FontFamily::Mono, weight, style)
    }

    fn resolve_cached(
        &self,
        is_mono: bool,
        family: &str,
        bundled: FontFamily,
        weight: FontWeight,
        style: FontStyle,
    ) -> Result<FontFace, TextError> {
        let Some(registry) = &self.registry else {
            return Ok(FontFace::Bundled(ferrigura_text::face(
                bundled, weight, style,
            )));
        };
        let key = (is_mono, weight.css_value(), style);
        if let Some(cache) = &self.cache
            && let Some(hit) = cache
                .read()
                .unwrap_or_else(PoisonError::into_inner)
                .get(&key)
                .cloned()
        {
            return Ok(hit);
        }
        let face = registry.resolve(family, weight, style)?;
        let _ = face.css_variable();
        let resolved = FontFace::Registered(face);
        if let Some(cache) = &self.cache {
            let mut entries = cache.write().unwrap_or_else(PoisonError::into_inner);
            if entries.len() < MAX_CONTEXT_CACHE_ENTRIES {
                entries.entry(key).or_insert_with(|| resolved.clone());
            }
        }
        Ok(resolved)
    }

    /// Uses a per-type configured CSS family list with the same registered files.
    #[must_use]
    pub fn with_family(&self, family: &str) -> Self {
        if family == self.family {
            return self.clone();
        }
        Self::new(self.registry.as_ref(), family)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_caches_resolved_registered_faces_and_shares_across_clones() {
        let mut registry = FontRegistry::new();
        let data = ferrigura_text::face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal)
            .data()
            .to_vec();
        registry
            .register("CustomSans", data, FontWeight::Normal, FontStyle::Normal)
            .unwrap();
        let registry = Arc::new(registry);
        let ctx = FontContext::new(Some(&registry), "CustomSans");
        let cloned = ctx.clone();

        let first = ctx.face(FontWeight::Normal, FontStyle::Normal).unwrap();
        let second = ctx
            .face(FontWeight::Number(400), FontStyle::Normal)
            .unwrap();
        let via_clone = cloned.face(FontWeight::Normal, FontStyle::Normal).unwrap();
        let same_family = ctx
            .with_family("CustomSans")
            .face(FontWeight::Normal, FontStyle::Normal)
            .unwrap();

        assert!(std::ptr::eq(&*first, &*second));
        assert!(std::ptr::eq(&*first, &*via_clone));
        assert!(std::ptr::eq(&*first, &*same_family));
        assert_eq!(first, second);
    }
}
