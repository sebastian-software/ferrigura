//! Per-render font resolution and owned handles. No process or thread-local font selection exists.

use std::sync::Arc;

use ferrigura_text::{Face, FontFamily, FontRegistry, FontStyle, FontWeight, TextError};

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
        }
    }

    /// Creates a context from explicitly provided fonts and a configured CSS family list.
    #[must_use]
    pub fn new(registry: Option<&Arc<FontRegistry>>, family: &str) -> Self {
        Self {
            registry: registry.cloned(),
            family: family.to_owned(),
        }
    }

    /// Resolves the configured family at the requested numeric weight and style.
    ///
    /// # Errors
    /// Reports an unregistered family with registration guidance.
    pub fn face(&self, weight: FontWeight, style: FontStyle) -> Result<FontFace, TextError> {
        match &self.registry {
            Some(registry) => registry.resolve(&self.family, weight, style).map(|face| {
                let _ = face.css_variable();
                FontFace::Registered(face)
            }),
            None => Ok(FontFace::Bundled(ferrigura_text::face(
                FontFamily::Sans,
                weight,
                style,
            ))),
        }
    }

    /// Resolves an explicit code family, or the bundled mono face in compatibility mode.
    ///
    /// # Errors
    /// Reports an unregistered code family. Register `monospace` when using that default name.
    pub fn mono(&self, weight: FontWeight, style: FontStyle) -> Result<FontFace, TextError> {
        match &self.registry {
            Some(registry) => registry.resolve("monospace", weight, style).map(|face| {
                let _ = face.css_variable();
                FontFace::Registered(face)
            }),
            None => Ok(FontFace::Bundled(ferrigura_text::face(
                FontFamily::Mono,
                weight,
                style,
            ))),
        }
    }

    /// Uses a per-type configured CSS family list with the same registered files.
    #[must_use]
    pub fn with_family(&self, family: &str) -> Self {
        Self {
            registry: self.registry.clone(),
            family: family.to_owned(),
        }
    }
}
