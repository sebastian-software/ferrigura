//! Font subsets for embedding in an SVG, so a viewer draws every label in the face it was measured
//! in.
//!
//! Each face is cut down with `skera` (Google's port of `hb-subset`) to the characters the diagram
//! draws with it. The subsets keep the default layout features, so kerning and ligatures survive,
//! keep the legacy `kern` table, and drop hinting. Each face becomes an `@font-face` rule with the
//! subset as a base64 TTF data URL.
//!
//! A run whose characters the sans or mono face lacks and a fallback face has (the arrows, the text
//! symbols, the emoji) is cut the way [`crate::fallback_runs`] cuts it, so each fallback face gets a
//! subset of its own. The text families name those subsets after their own in their `css_family`,
//! which is the font-family list a browser walks for each character.
//!
//! `@font-face` rules are global to the HTML page, so a family name has to be unique to its subset:
//! two diagrams with different subsets under one name would break each other's labels. The name
//! is `fg-` and a 64-bit FNV-1a hash of the subsets, then the family suffix, such as
//! `fg-0123456789abcdef-sans`. Equal subsets get equal names, and the hash is computed over the
//! subset bytes and their weight and style, so a different subset gets a different name.

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;
use std::sync::{Arc, Mutex};

use base64::Engine;
use read_fonts::collections::IntSet;
use read_fonts::types::{NameId, Tag};
use read_fonts::{FontRef, TableProvider};
use skera::{DEFAULT_LAYOUT_FEATURES, Plan, SubsetFlags, subset_font};

use crate::error::TextError;
use crate::fonts::{Face, FontFamily, FontStyle};
use crate::measure::{Run, fallback_runs};

/// Subsets retained by one reusable renderer. The budget includes keys and values; font data is
/// shared with the renderer's immutable registry, not copied into the cache.
#[derive(Default)]
#[doc(hidden)]
pub struct SubsetCache(Mutex<CacheState>);

#[derive(Default)]
struct CacheState {
    entries: HashMap<CacheKey, CacheEntry>,
    bytes: usize,
    clock: u64,
}

#[derive(Hash, PartialEq, Eq)]
struct CacheKey {
    data_address: usize,
    data_len: usize,
    characters: Box<[char]>,
}

struct CacheEntry {
    // A weak reference keeps the allocation identity unique without retaining the font bytes.
    _data: Option<std::sync::Weak<Vec<u8>>>,
    subset: Arc<Vec<u8>>,
    charge: usize,
    used: u64,
}

impl std::fmt::Debug for SubsetCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubsetCache").finish_non_exhaustive()
    }
}

// The shorthand keeps several renderers. Bound each one and also cap the number of hash entries.
// Reserve space for the maximum map allocation even after eviction, since HashMap retains capacity.
const CACHE_BUDGET: usize = 1024 * 1024 - 256 * 256;
const CACHE_ENTRIES: usize = 256;
// Accounts conservatively for the map bucket, Arc header and per-entry bookkeeping.
const ENTRY_OVERHEAD: usize = 256;

thread_local! {
    static ACTIVE_CACHE: std::cell::RefCell<Option<Arc<SubsetCache>>> = const { std::cell::RefCell::new(None) };
}

/// Runs a render with its subset cache, restoring any outer scope on return or unwind.
#[doc(hidden)]
pub fn with_subset_cache<T>(cache: &Arc<SubsetCache>, render: impl FnOnce() -> T) -> T {
    struct Restore(Option<Arc<SubsetCache>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            ACTIVE_CACHE.with(|active| *active.borrow_mut() = self.0.take());
        }
    }
    let previous = ACTIVE_CACHE.with(|active| active.borrow_mut().replace(cache.clone()));
    let _restore = Restore(previous);
    render()
}

/// The embedded subsets of one font family in one diagram.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedFamily {
    /// The family whose faces are embedded.
    pub family: FontFamily,
    /// The host-registered family name, or the bundled face identifier.
    pub source_name: String,
    /// The exact supplied design's weight. Bundled family groups use `None`.
    pub weight: Option<u16>,
    /// The exact supplied design's style. Bundled family groups use `None`.
    pub style: Option<FontStyle>,
    /// Identity of the supplied font file, distinguishing retained registry versions.
    pub data_fingerprint: Option<u64>,
    /// The family name the labels refer to, such as `fg-0123456789abcdef-sans`.
    pub name: String,
    /// The `font-family` value of the text set in this family, quoted for CSS: the family name, then
    /// the names of the fallback families the diagram embeds, in the order of
    /// [`crate::fallback_faces`], such as `"fg-0123456789abcdef-sans", "fg-fedcba9876543210-symbols"`.
    /// A browser then sets each character the first family lacks in the first fallback that has it,
    /// as [`crate::fallback_runs`] measured it.
    pub css_family: String,
    /// One `@font-face` rule for each face of the family the diagram uses. Each rule names
    /// [`EmbeddedFamily::name`] and holds the subset as a base64 TTF data URL.
    pub css: String,
    /// Stable supplied-face variables and their subset family lists, scoped by the SVG writer.
    pub variables: Vec<(String, String)>,
}

/// Subsets the faces that `runs` use, to the characters each run draws with them, and returns one
/// embedded family for each font family in use. Families come out in the order of
/// [`FontFamily`]: sans first, then mono.
///
/// The output depends only on the subsets, so calling this twice on runs with the same subsets
/// gives the same names and the same CSS.
///
/// # Errors
///
/// Returns an error if a face cannot be parsed or subsetting fails.
pub fn embed_runs(runs: &[Run<'_, '_>]) -> Result<Vec<EmbeddedFamily>, TextError> {
    let SelectedRuns {
        stretches,
        faces,
        metric_faces,
    } = select_runs(runs)?;
    let mut embedded = Vec::new();
    let mut index = 0;
    while index < faces.len() {
        let primary = faces[index];
        let mut subsets = Vec::new();
        while index < faces.len()
            && same_family(primary, faces[index])
            && (primary.family() != FontFamily::Registered
                || (primary.weight().css_value() == faces[index].weight().css_value()
                    && primary.style() == faces[index].style()))
        {
            let face = faces[index];
            let mut characters: BTreeSet<char> = stretches
                .iter()
                .filter(|run| same_face(run.face, face))
                .flat_map(|run| run.text.chars())
                .collect();
            if metric_faces.iter().any(|metric| same_face(metric, face)) {
                characters.insert(' ');
            }
            subsets.push((face, cached_subset(face, &characters)?));
            index += 1;
        }
        let name = family_name(primary.family(), &subsets);
        let css = font_face_rules(&name, &subsets)?;
        embedded.push(EmbeddedFamily {
            family: primary.family(),
            source_name: primary.family_name().to_owned(),
            data_fingerprint: primary.data_fingerprint(),
            weight: (primary.family() == FontFamily::Registered)
                .then(|| primary.weight().css_value()),
            style: (primary.family() == FontFamily::Registered).then(|| primary.style()),
            name,
            css_family: String::new(),
            css,
            variables: Vec::new(),
        });
    }
    for index in 0..embedded.len() {
        let primary = faces
            .iter()
            .find(|face| matches_embedded(face, &embedded[index]));
        let fallback_names: Vec<String> = if embedded[index].family == FontFamily::Registered {
            primary
                .into_iter()
                .flat_map(|face| face.registered_fallbacks())
                .filter_map(|fallback| {
                    embedded
                        .iter()
                        .find(|family| matches_embedded(fallback, family))
                        .map(|family| family.name.clone())
                })
                .collect()
        } else if matches!(embedded[index].family, FontFamily::Sans | FontFamily::Mono) {
            [
                FontFamily::Arrows,
                FontFamily::TextSymbols,
                FontFamily::Emoji,
            ]
            .iter()
            .filter_map(|fallback| {
                embedded
                    .iter()
                    .find(|family| family.family == *fallback)
                    .map(|family| family.name.clone())
            })
            .collect()
        } else {
            Vec::new()
        };
        embedded[index].css_family = std::iter::once(embedded[index].name.clone())
            .chain(fallback_names)
            .map(|name| format!("\"{name}\""))
            .collect::<Vec<_>>()
            .join(", ");
    }
    // Bind original requested faces, not only the stretches: all glyphs may have fallen back.
    let mut variables = std::collections::BTreeMap::new();
    for run in runs {
        let Some(variable) = run.face.css_variable() else {
            continue;
        };
        let names: Vec<_> = std::iter::once(run.face)
            .chain(run.face.registered_fallbacks().iter().map(AsRef::as_ref))
            .filter_map(|face| {
                embedded
                    .iter()
                    .find(|family| matches_embedded(face, family))
                    .map(|family| format!("\"{}\"", family.name))
            })
            .collect();
        variables.insert(variable.to_owned(), names.join(", "));
    }
    if let Some(first) = embedded.first_mut() {
        first.variables = variables.into_iter().collect();
    }
    Ok(embedded)
}

/// The actual fallback stretches and faces selected for a set of measured runs. Keeping this
/// separate from serialization lets font output share the same selection without subsetting.
struct SelectedRuns<'a, 'f> {
    stretches: Vec<Run<'a, 'f>>,
    faces: Vec<&'f Face>,
    metric_faces: Vec<&'f Face>,
}

fn select_runs<'a, 'f>(runs: &[Run<'a, 'f>]) -> Result<SelectedRuns<'a, 'f>, TextError> {
    // The stretches each run is set in, after the fallback to the symbol face: a subset holds the
    // characters drawn in its face, and no other.
    let stretches: Vec<Run<'_, '_>> = runs.iter().flat_map(|run| fallback_runs(*run)).collect();
    let mut faces: Vec<&Face> = stretches.iter().map(|run| run.face).collect();
    // SVG line boxes use the first family's metrics even when every character falls back.
    // Keep supplied primary faces in the cascade; a space gives the metrics-only subset a cmap.
    let metric_faces: Vec<&Face> = runs
        .iter()
        .map(|run| run.face)
        .filter(|face| face.family() == FontFamily::Registered)
        .filter(|face| !faces.iter().any(|drawn| same_face(face, drawn)))
        .collect();
    faces.extend(metric_faces.iter().copied());
    faces.sort_by_key(|face| {
        (
            family_order(face.family()),
            if face.family() == FontFamily::Registered {
                face.family_name().to_ascii_lowercase()
            } else {
                String::new()
            },
            face.weight().css_value(),
            match face.style() {
                FontStyle::Normal => 0,
                FontStyle::Italic => 1,
            },
        )
    });
    faces.dedup_by(|a, b| same_face(a, b));
    for pair in faces.windows(2) {
        if same_family(pair[0], pair[1])
            && pair[0].weight().css_value() == pair[1].weight().css_value()
            && pair[0].style() == pair[1].style()
        {
            return Err(TextError::Registration(format!(
                "cannot embed different font data for family {:?}, weight {}, style {} in one SVG",
                pair[0].family_name(),
                pair[0].weight().css_value(),
                pair[0].style().css_keyword()
            )));
        }
    }
    Ok(SelectedRuns {
        stretches,
        faces,
        metric_faces,
    })
}

/// The subset of `face` to the characters of `text`, with the default layout features, the legacy
/// `kern` table and no hinting. CFF and CFF2 fonts are returned complete because the subsetter
/// cannot serialize their outlines; embedding preserves their appearance and advances.
///
/// # Errors
///
/// Returns an error if the face cannot be parsed or subsetting fails.
pub fn subset_face(face: &Face, text: &str) -> Result<Vec<u8>, TextError> {
    let characters: BTreeSet<char> = text.chars().collect();
    subset_characters(face, &characters)
}

fn matches_embedded(face: &Face, family: &EmbeddedFamily) -> bool {
    family.family == face.family()
        && (face.family() != FontFamily::Registered
            || (family.source_name.eq_ignore_ascii_case(face.family_name())
                && family.data_fingerprint == face.data_fingerprint()
                && family.weight == Some(face.weight().css_value())
                && family.style == Some(face.style())))
}

fn same_face(a: &Face, b: &Face) -> bool {
    same_family(a, b)
        && a.weight().css_value() == b.weight().css_value()
        && a.style() == b.style()
        && (std::ptr::eq(a.data(), b.data()) || a.data() == b.data())
}

fn same_family(a: &Face, b: &Face) -> bool {
    a.family() == b.family()
        && (a.family() != FontFamily::Registered
            || a.family_name().eq_ignore_ascii_case(b.family_name()))
}

fn family_order(family: FontFamily) -> u8 {
    match family {
        FontFamily::Sans => 0,
        FontFamily::Mono => 1,
        FontFamily::Arrows => 2,
        FontFamily::TextSymbols => 3,
        FontFamily::Emoji => 4,
        FontFamily::Registered => 5,
    }
}

/// The white space that SVG text collapses: the browser draws each run of it as one space, so a
/// text with a line break or a tab needs the space glyph even when it has no space of its own.
const COLLAPSED_WHITE_SPACE: [char; 3] = ['\n', '\r', '\t'];

fn cached_subset(face: &Face, characters: &BTreeSet<char>) -> Result<Arc<Vec<u8>>, TextError> {
    let cache = ACTIVE_CACHE.with(|active| active.borrow().clone());
    let Some(cache) = cache else {
        return subset_characters(face, characters).map(Arc::new);
    };
    let mut normalized = characters.clone();
    if COLLAPSED_WHITE_SPACE.iter().any(|c| normalized.contains(c)) {
        normalized.insert(' ');
    }
    let (data_address, data_owner) = match &face.data {
        crate::fonts::FontData::Bundled(data) => (data.as_ptr() as usize, None),
        crate::fonts::FontData::Registered(data) => {
            (Arc::as_ptr(data) as usize, Some(Arc::downgrade(data)))
        }
    };
    let key = CacheKey {
        data_address,
        data_len: face.data().len(),
        characters: normalized.into_iter().collect(),
    };
    {
        let mut state = cache.0.lock().unwrap_or_else(|e| e.into_inner());
        state.clock = state.clock.wrapping_add(1);
        let now = state.clock;
        if let Some(entry) = state.entries.get_mut(&key) {
            entry.used = now;
            return Ok(entry.subset.clone());
        }
    }
    let subset = Arc::new(subset_characters(face, characters)?);
    let charge = ENTRY_OVERHEAD
        .saturating_add(key.characters.len() * std::mem::size_of::<char>())
        .saturating_add(subset.capacity());
    if charge <= CACHE_BUDGET {
        let mut state = cache.0.lock().unwrap_or_else(|e| e.into_inner());
        if !state.entries.contains_key(&key) {
            while state.entries.len() >= CACHE_ENTRIES || state.bytes + charge > CACHE_BUDGET {
                let Some(oldest) = state
                    .entries
                    .iter()
                    .min_by_key(|(_, entry)| entry.used)
                    .map(|(key, _)| CacheKey {
                        data_address: key.data_address,
                        data_len: key.data_len,
                        characters: key.characters.clone(),
                    })
                else {
                    break;
                };
                if let Some(entry) = state.entries.remove(&oldest) {
                    state.bytes -= entry.charge;
                }
            }
            state.clock = state.clock.wrapping_add(1);
            let used = state.clock;
            state.bytes += charge;
            state.entries.insert(
                key,
                CacheEntry {
                    _data: data_owner,
                    subset: subset.clone(),
                    charge,
                    used,
                },
            );
        }
    }
    Ok(subset)
}

fn subset_characters(face: &Face, characters: &BTreeSet<char>) -> Result<Vec<u8>, TextError> {
    let mut unicodes = IntSet::empty();
    for character in characters {
        unicodes.insert(u32::from(*character));
    }
    if COLLAPSED_WHITE_SPACE
        .iter()
        .any(|character| characters.contains(character))
    {
        unicodes.insert(u32::from(' '));
    }
    // The name records 0 to 6 in English, which is what a browser needs for the family.
    let mut name_ids = IntSet::empty();
    name_ids.insert_range(NameId::from(0)..=NameId::from(6));
    subset_data(face.data(), &unicodes, &name_ids)
}

/// Cuts a font file to the characters given, for the bundled faces (the cut `cut_font` names).
///
/// The cut keeps the name records 0 to 6, 13 and 14 in English, the license records included, and
/// otherwise the plan of [`subset_face`]: every script, the default layout features, the legacy
/// `kern` table, the color tables (COLR, CPAL) and no hinting.
///
/// # Errors
///
/// Returns an error if `data` does not parse as a font or subsetting fails.
pub fn cut_font(
    data: &[u8],
    characters: impl IntoIterator<Item = char>,
) -> Result<Vec<u8>, TextError> {
    let mut unicodes = IntSet::empty();
    for character in characters {
        unicodes.insert(u32::from(character));
    }
    let mut name_ids = IntSet::empty();
    name_ids.insert_range(NameId::from(0)..=NameId::from(6));
    name_ids.insert(NameId::from(13));
    name_ids.insert(NameId::from(14));
    subset_data(data, &unicodes, &name_ids)
}

/// The subset of font `data` to `unicodes`, with the plan the embedded subsets and the bundled cuts
/// share: every script, the default layout features, the legacy `kern` table and no hinting.
fn subset_data(
    data: &[u8],
    unicodes: &IntSet<u32>,
    name_ids: &IntSet<NameId>,
) -> Result<Vec<u8>, TextError> {
    let font = FontRef::new(data).map_err(|_| TextError::InvalidFont)?;
    if font.table_data(Tag::new(b"SVG ")).is_some() {
        return Err(TextError::Registration(
            "OpenType-SVG fonts are not supported".into(),
        ));
    }
    // skera does not serialize CFF outlines. Keep the complete font instead of returning an
    // invalid subset without outlines. This also preserves CFF subroutines and CFF2 variations.
    if font.table_data(Tag::new(b"CFF ")).is_some() || font.table_data(Tag::new(b"CFF2")).is_some()
    {
        return Ok(data.to_vec());
    }
    // Every script and the default layout features, so kerning and ligatures are kept.
    let mut scripts = IntSet::empty();
    scripts.invert();
    let mut features = IntSet::empty();
    features.extend(DEFAULT_LAYOUT_FEATURES.iter().copied());
    let mut languages = IntSet::empty();
    languages.insert(0x0409);
    // No table is dropped: the legacy `kern` table stays, as it does in the spike. Hinting goes.
    let no_tables: IntSet<Tag> = IntSet::empty();
    let plan = Plan::new(
        &IntSet::empty(),
        unicodes,
        &font,
        SubsetFlags::SUBSET_FLAGS_NO_HINTING,
        &no_tables,
        &scripts,
        &features,
        name_ids,
        &languages,
    );
    let subset = subset_font(&font, &plan).map_err(|error| TextError::Subset(error.to_string()))?;
    crate::kern::restore(&font, subset, &plan)
}

/// The family name of a set of subsets: the hash of each face's weight, style and bytes.
fn family_name(family: FontFamily, subsets: &[(&Face, Arc<Vec<u8>>)]) -> String {
    let hash = fnv1a(&canonical_bytes(subsets));
    format!("fg-{hash:016x}-{}", family.css_suffix())
}

/// The bytes the family hash covers, face by face: the weight, the style, the length of the subset
/// and the subset itself. Fixed-width integers are little-endian, so the bytes do not depend on the
/// platform.
fn canonical_bytes(subsets: &[(&Face, Arc<Vec<u8>>)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (face, data) in subsets {
        bytes.extend_from_slice(&face.weight().css_value().to_le_bytes());
        bytes.push(match face.style() {
            FontStyle::Normal => 0,
            FontStyle::Italic => 1,
        });
        bytes.extend_from_slice(&(data.len() as u64).to_le_bytes());
        bytes.extend_from_slice(data);
    }
    bytes
}

/// 64-bit FNV-1a. It is deterministic on every platform and needs no dependency. It is not a
/// cryptographic hash: a collision would only mix two subsets under one name, and the name is not
/// a security boundary.
pub(crate) fn fnv1a(bytes: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    bytes.iter().fold(OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(PRIME)
    })
}

fn font_face_rules(name: &str, subsets: &[(&Face, Arc<Vec<u8>>)]) -> Result<String, TextError> {
    let mut css = String::new();
    for (face, data) in subsets {
        let encoded = base64::engine::general_purpose::STANDARD.encode(data.as_slice());
        let format = if data.starts_with(b"OTTO") {
            "otf"
        } else {
            "ttf"
        };
        let variations = variation_defaults(face)?;
        let _ = write!(
            css,
            "@font-face{{font-family:\"{name}\";font-weight:{};font-style:{};{variations}src:url(data:font/{format};base64,{encoded})}}",
            face.weight().css_value(),
            face.style().css_keyword(),
        );
    }
    Ok(css)
}

/// A variable font is measured at its default location. Per-face descriptors freeze that same
/// location in the browser, including wght/opsz axes that CSS otherwise changes automatically.
fn variation_defaults(face: &Face) -> Result<String, TextError> {
    let font = FontRef::new(face.data()).map_err(|_| TextError::InvalidFont)?;
    if font.table_data(Tag::new(b"fvar")).is_none() {
        return Ok(String::new());
    }
    let fvar = font.fvar().map_err(|_| TextError::InvalidFont)?;
    let axes = fvar.axes().map_err(|_| TextError::InvalidFont)?;
    let mut settings = Vec::new();
    for axis in axes {
        let mut tag = String::new();
        for byte in axis.axis_tag().to_be_bytes() {
            if !(0x20..=0x7e).contains(&byte) {
                return Err(TextError::InvalidFont);
            }
            if byte == b'"' || byte == b'\\' {
                let _ = write!(tag, "\\{byte:02x} ");
            } else {
                tag.push(char::from(byte));
            }
        }
        settings.push(format!("\"{tag}\" {}", axis.default_value().to_f64()));
    }
    if settings.is_empty() {
        Ok(String::new())
    } else {
        Ok(format!("font-variation-settings:{};", settings.join(",")))
    }
}

#[cfg(test)]
mod selection_tests {
    use super::{embed_runs, select_runs};
    use crate::{FontRegistry, FontStyle, FontWeight, Run};

    #[test]
    fn grouping_preserves_data_and_metadata_equality() {
        let bytes = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fonts/LexicaUltralegible-Regular.ttf"
        ))
        .unwrap();
        let make_face = |family: &str, data: Vec<u8>, weight| {
            let mut registry = FontRegistry::new();
            registry
                .register(family, data, weight, FontStyle::Normal)
                .unwrap();
            registry.resolve(family, weight, FontStyle::Normal).unwrap()
        };
        let original = make_face("Example", bytes.clone(), FontWeight::Normal);
        let equal = make_face("example", bytes.clone(), FontWeight::Normal);
        let alias = make_face("Alias", bytes.clone(), FontWeight::Normal);
        let bold = make_face("Example", bytes.clone(), FontWeight::Bold);
        assert!(!std::ptr::eq(original.data(), equal.data()));
        let runs = [
            Run {
                text: "A",
                face: &original,
            },
            Run {
                text: "B",
                face: &original,
            },
            Run {
                text: "C",
                face: &equal,
            },
        ];
        assert_eq!(select_runs(&runs).unwrap().faces.len(), 1);
        assert_eq!(
            embed_runs(&runs).unwrap()[0].css,
            embed_runs(&[Run {
                text: "ABC",
                face: &original
            }])
            .unwrap()[0]
                .css
        );
        assert_eq!(
            select_runs(&[
                Run {
                    text: "A",
                    face: &original
                },
                Run {
                    text: "B",
                    face: &alias
                },
                Run {
                    text: "C",
                    face: &bold
                }
            ])
            .unwrap()
            .faces
            .len(),
            3
        );
        let mut changed = bytes;
        changed.push(0);
        let different = make_face("Example", changed, FontWeight::Normal);
        assert!(
            select_runs(&[
                Run {
                    text: "A",
                    face: &original
                },
                Run {
                    text: "B",
                    face: &different
                }
            ])
            .is_err()
        );
    }

    #[test]
    fn selection_preserves_fallback_stretches_and_primary_line_metrics() {
        let mut registry = FontRegistry::new();
        let fonts = concat!(env!("CARGO_MANIFEST_DIR"), "/fonts/");
        for (name, file) in [
            ("primary", "LexicaUltralegible-Regular.ttf"),
            ("arrows", "NotoSansMath-Regular-Arrows.ttf"),
        ] {
            registry
                .register_file(
                    name,
                    format!("{fonts}{file}"),
                    FontWeight::Normal,
                    FontStyle::Normal,
                )
                .unwrap();
        }
        registry.set_fallbacks(["arrows"]);
        let primary = registry
            .resolve("primary", FontWeight::Normal, FontStyle::Normal)
            .unwrap();
        let runs = [Run {
            text: "→",
            face: &primary,
        }];
        let selected = select_runs(&runs).unwrap();
        assert_eq!(selected.stretches.len(), 1);
        assert_eq!(selected.stretches[0].text, "→");
        assert_eq!(selected.stretches[0].face.family_name(), "arrows");
        assert_eq!(selected.metric_faces.len(), 1);
        assert_eq!(selected.metric_faces[0].family_name(), "primary");
        assert_eq!(selected.faces.len(), 2);
        assert!(
            selected
                .faces
                .iter()
                .any(|face| face.family_name() == "primary")
        );
        assert!(
            selected
                .faces
                .iter()
                .any(|face| face.family_name() == "arrows")
        );
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;
    use crate::{FontFamily, FontStyle, FontWeight, Run, face};

    fn regular() -> &'static Face {
        face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal)
    }
    fn bold() -> &'static Face {
        face(FontFamily::Sans, FontWeight::Bold, FontStyle::Normal)
    }

    #[test]
    fn cache_preserves_output_and_normalizes_effective_characters() {
        let cache = Arc::new(SubsetCache::default());
        for text in ["", "a\nb", "a\n b", "→", "Hello"] {
            let runs = [Run {
                text,
                face: regular(),
            }];
            let expected = embed_runs(&runs).unwrap();
            let actual = with_subset_cache(&cache, || embed_runs(&runs).unwrap());
            assert_eq!(expected, actual);
        }
        let state = cache.0.lock().unwrap();
        // The two newline inputs have the same effective Unicode set.
        assert_eq!(state.entries.len(), 4);
        assert!(state.bytes <= CACHE_BUDGET);
    }

    #[test]
    fn cache_distinguishes_font_data_and_stays_bounded() {
        let cache = Arc::new(SubsetCache::default());
        with_subset_cache(&cache, || {
            for n in 0..300 {
                let text = format!("A{n}{}", char::from_u32(0x1000 + n).unwrap());
                for face in [regular(), bold()] {
                    let runs = [Run { text: &text, face }];
                    assert_eq!(embed_runs(&runs).unwrap(), embed_without_cache(&runs));
                }
            }
        });
        let state = cache.0.lock().unwrap();
        assert!(state.entries.len() <= CACHE_ENTRIES);
        assert!(state.bytes <= CACHE_BUDGET);
    }

    fn embed_without_cache(runs: &[Run<'_, '_>]) -> Vec<EmbeddedFamily> {
        let saved = ACTIVE_CACHE.with(|active| active.borrow_mut().take());
        let result = embed_runs(runs).unwrap();
        ACTIVE_CACHE.with(|active| *active.borrow_mut() = saved);
        result
    }

    #[test]
    fn registered_faces_keep_aliases_and_do_not_pin_font_data() {
        use crate::FontRegistry;
        let cache = Arc::new(SubsetCache::default());
        let weak = {
            let mut registry = FontRegistry::new();
            registry
                .register(
                    "primary",
                    regular().data().to_vec(),
                    FontWeight::Normal,
                    FontStyle::Normal,
                )
                .unwrap();
            registry
                .register(
                    "arrows",
                    face(FontFamily::Arrows, FontWeight::Normal, FontStyle::Normal)
                        .data()
                        .to_vec(),
                    FontWeight::Normal,
                    FontStyle::Normal,
                )
                .unwrap();
            registry.set_fallbacks(["arrows"]);
            let primary = registry
                .resolve("primary", FontWeight::Normal, FontStyle::Normal)
                .unwrap();
            let weak = match &primary.data {
                crate::fonts::FontData::Registered(data) => Arc::downgrade(data),
                _ => unreachable!(),
            };
            for text in ["", "→", "Hello →"] {
                let runs = [Run {
                    text,
                    face: &primary,
                }];
                let expected = embed_runs(&runs).unwrap();
                assert_eq!(
                    with_subset_cache(&cache, || embed_runs(&runs).unwrap()),
                    expected
                );
            }
            weak
        };
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn scopes_restore_on_unwind_and_concurrent_renders_share_safely() {
        let cache = Arc::new(SubsetCache::default());
        let inner = Arc::new(SubsetCache::default());
        with_subset_cache(&cache, || {
            let _ =
                std::panic::catch_unwind(|| with_subset_cache(&inner, || panic!("test unwind")));
            ACTIVE_CACHE
                .with(|active| assert!(Arc::ptr_eq(active.borrow().as_ref().unwrap(), &cache)));
        });
        ACTIVE_CACHE.with(|active| assert!(active.borrow().is_none()));
        std::thread::scope(|scope| {
            for _ in 0..8 {
                let cache = &cache;
                scope.spawn(move || {
                    with_subset_cache(cache, || {
                        let runs = [Run {
                            text: "concurrent",
                            face: regular(),
                        }];
                        for _ in 0..10 {
                            assert_eq!(embed_runs(&runs).unwrap(), embed_without_cache(&runs));
                        }
                    })
                });
            }
        });
        assert!(cache.0.lock().unwrap().bytes <= CACHE_BUDGET);
    }
}
