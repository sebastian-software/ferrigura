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

use std::borrow::Borrow;
use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use base64::Engine;
use read_fonts::collections::IntSet;
use read_fonts::types::{NameId, Tag};
use read_fonts::{FontRef, TableProvider};
use skera::{DEFAULT_LAYOUT_FEATURES, Plan, SubsetFlags, subset_font};

use crate::error::TextError;
use crate::fonts::{Face, FontFamily, FontStyle};
use crate::measure::{Run, for_each_fallback_run};

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

trait CacheKeyRef {
    fn parts(&self) -> (usize, usize, &[char]);
}

impl PartialEq for dyn CacheKeyRef + '_ {
    fn eq(&self, other: &Self) -> bool {
        self.parts() == other.parts()
    }
}

impl Eq for dyn CacheKeyRef + '_ {}

impl Hash for dyn CacheKeyRef + '_ {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let (addr, len, chars) = self.parts();
        addr.hash(state);
        len.hash(state);
        chars.hash(state);
    }
}

#[derive(Clone)]
struct CacheKey {
    data_address: usize,
    data_len: usize,
    characters: Arc<[char]>,
}

impl CacheKeyRef for CacheKey {
    fn parts(&self) -> (usize, usize, &[char]) {
        (self.data_address, self.data_len, &self.characters)
    }
}

impl PartialEq for CacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.parts() == other.parts()
    }
}

impl Eq for CacheKey {}

impl Hash for CacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let (addr, len, chars) = self.parts();
        addr.hash(state);
        len.hash(state);
        chars.hash(state);
    }
}

impl<'a> Borrow<dyn CacheKeyRef + 'a> for CacheKey {
    fn borrow(&self) -> &(dyn CacheKeyRef + 'a) {
        self
    }
}

struct LookupKey<'a> {
    data_address: usize,
    data_len: usize,
    characters: &'a [char],
}

impl CacheKeyRef for LookupKey<'_> {
    fn parts(&self) -> (usize, usize, &[char]) {
        (self.data_address, self.data_len, self.characters)
    }
}

/// A subset (or complete CFF font) paired with its precomputed base64 payload and single-face
/// FNV-1a hash state so repeated renders skip both re-hashing and re-encoding.
#[derive(Debug)]
pub(crate) struct EmbeddedSubset {
    pub(crate) bytes: Arc<Vec<u8>>,
    pub(crate) base64: String,
    pub(crate) solo_weight: u16,
    pub(crate) solo_style: FontStyle,
    pub(crate) solo_hash: u64,
}

impl EmbeddedSubset {
    pub(crate) fn new(bytes: Arc<Vec<u8>>, weight: u16, style: FontStyle) -> Self {
        let mut hasher = Fnv1a::new();
        hash_face_subset(&mut hasher, weight, style, bytes.as_slice());
        let base64 = base64::engine::general_purpose::STANDARD.encode(bytes.as_slice());
        Self {
            bytes,
            base64,
            solo_weight: weight,
            solo_style: style,
            solo_hash: hasher.finish(),
        }
    }
}

enum SubsetData {
    Uncached(Vec<u8>),
    Cached(Arc<EmbeddedSubset>),
}

impl SubsetData {
    fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Uncached(bytes) => bytes,
            Self::Cached(entry) => entry.bytes.as_slice(),
        }
    }
}

struct CacheEntry {
    // A weak reference keeps the allocation identity unique without retaining the font bytes.
    _data: Option<std::sync::Weak<Vec<u8>>>,
    subset: Arc<EmbeddedSubset>,
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
            let mut characters: Vec<char> = stretches
                .iter()
                .filter(|run| same_face(run.face, face))
                .flat_map(|run| run.text.chars())
                .collect();
            if metric_faces.iter().any(|metric| same_face(metric, face))
                || characters
                    .iter()
                    .any(|character| COLLAPSED_WHITE_SPACE.contains(character))
            {
                characters.push(' ');
            }
            characters.sort_unstable();
            characters.dedup();
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
    let mut stretches = Vec::with_capacity(runs.len());
    for run in runs {
        let _ = for_each_fallback_run::<std::convert::Infallible>(*run, |stretch| {
            stretches.push(stretch);
            Ok(())
        });
    }
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

fn cached_subset(face: &Face, characters: &[char]) -> Result<SubsetData, TextError> {
    if let crate::fonts::FontData::Registered {
        bytes: data,
        embed_whole: true,
        whole_embed,
    } = &face.data
    {
        if let Some(entry) = whole_embed {
            return Ok(SubsetData::Cached(Arc::clone(entry)));
        }
        return Ok(SubsetData::Cached(Arc::new(EmbeddedSubset::new(
            data.clone(),
            face.weight().css_value(),
            face.style(),
        ))));
    }
    let cache = ACTIVE_CACHE.with(|active| active.borrow().clone());
    let Some(cache) = cache else {
        return subset_sorted_characters(face, characters).map(SubsetData::Uncached);
    };
    let (data_address, data_owner) = match &face.data {
        crate::fonts::FontData::Bundled(data) => (data.as_ptr() as usize, None),
        crate::fonts::FontData::Registered { bytes: data, .. } => {
            (Arc::as_ptr(data) as usize, Some(Arc::downgrade(data)))
        }
    };
    let data_len = face.data().len();
    let lookup = LookupKey {
        data_address,
        data_len,
        characters,
    };
    {
        let mut state = cache.0.lock().unwrap_or_else(|e| e.into_inner());
        state.clock = state.clock.wrapping_add(1);
        let now = state.clock;
        if let Some(entry) = state.entries.get_mut(&lookup as &dyn CacheKeyRef) {
            entry.used = now;
            return Ok(SubsetData::Cached(Arc::clone(&entry.subset)));
        }
    }
    let subset_bytes = subset_sorted_characters(face, characters)?;
    let estimated_base64 = base64::encoded_len(subset_bytes.len(), true).unwrap_or(usize::MAX);
    let estimated_charge = ENTRY_OVERHEAD
        .saturating_add(std::mem::size_of_val(characters))
        .saturating_add(subset_bytes.capacity())
        .saturating_add(estimated_base64);
    if estimated_charge > CACHE_BUDGET {
        return Ok(SubsetData::Uncached(subset_bytes));
    }
    let subset = Arc::new(EmbeddedSubset::new(
        Arc::new(subset_bytes),
        face.weight().css_value(),
        face.style(),
    ));
    let charge = ENTRY_OVERHEAD
        .saturating_add(std::mem::size_of_val(characters))
        .saturating_add(subset.bytes.capacity())
        .saturating_add(subset.base64.capacity());
    if charge <= CACHE_BUDGET {
        let key = CacheKey {
            data_address,
            data_len,
            characters: Arc::from(characters),
        };
        let mut state = cache.0.lock().unwrap_or_else(|e| e.into_inner());
        if !state.entries.contains_key(&key) {
            while state.entries.len() >= CACHE_ENTRIES || state.bytes + charge > CACHE_BUDGET {
                let Some(oldest) = state
                    .entries
                    .iter()
                    .min_by_key(|(_, entry)| entry.used)
                    .map(|(key, _)| key.clone())
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
                    subset: Arc::clone(&subset),
                    charge,
                    used,
                },
            );
        }
    }
    Ok(SubsetData::Cached(subset))
}

fn subset_sorted_characters(face: &Face, characters: &[char]) -> Result<Vec<u8>, TextError> {
    let mut unicodes = IntSet::empty();
    for character in characters {
        unicodes.insert(u32::from(*character));
    }
    if characters
        .iter()
        .any(|character| COLLAPSED_WHITE_SPACE.contains(character))
    {
        unicodes.insert(u32::from(' '));
    }
    // The name records 0 to 6 in English, which is what a browser needs for the family.
    let mut name_ids = IntSet::empty();
    name_ids.insert_range(NameId::from(0)..=NameId::from(6));
    subset_data(face.data(), &unicodes, &name_ids)
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
    if requires_whole_font(&font) {
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

pub(crate) fn requires_whole_font(font: &FontRef<'_>) -> bool {
    font.table_data(Tag::new(b"CFF ")).is_some() || font.table_data(Tag::new(b"CFF2")).is_some()
}

/// The family name of a set of subsets: the streaming 64-bit FNV-1a hash of each face's weight,
/// style, subset length and subset bytes. Fixed-width integers are little-endian, so the hash does
/// not depend on the platform.
fn family_name(family: FontFamily, subsets: &[(&Face, SubsetData)]) -> String {
    let hash = family_hash(subsets);
    format!("fg-{hash:016x}-{}", family.css_suffix())
}

fn family_hash(subsets: &[(&Face, SubsetData)]) -> u64 {
    let mut hasher = Fnv1a::new();
    for (index, (face, data)) in subsets.iter().enumerate() {
        let weight = face.weight().css_value();
        let style = face.style();
        if index == 0
            && let SubsetData::Cached(entry) = data
            && entry.solo_weight == weight
            && entry.solo_style == style
        {
            hasher = Fnv1a::from_state(entry.solo_hash);
            continue;
        }
        hash_face_subset(&mut hasher, weight, style, data.as_bytes());
    }
    hasher.finish()
}

fn hash_face_subset(hasher: &mut Fnv1a, weight: u16, style: FontStyle, data: &[u8]) {
    hasher.update(&weight.to_le_bytes());
    hasher.update_byte(match style {
        FontStyle::Normal => 0,
        FontStyle::Italic => 1,
    });
    hasher.update(&(data.len() as u64).to_le_bytes());
    hasher.update(data);
}

/// Streaming 64-bit FNV-1a. It is deterministic on every platform and needs no dependency. It is not
/// a cryptographic hash: a collision would only mix two subsets under one name, and the name is not
/// a security boundary.
pub(crate) struct Fnv1a(u64);

impl Fnv1a {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    pub(crate) const fn new() -> Self {
        Self(Self::OFFSET_BASIS)
    }

    pub(crate) const fn from_state(state: u64) -> Self {
        Self(state)
    }

    pub(crate) fn update(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = (self.0 ^ u64::from(*byte)).wrapping_mul(Self::PRIME);
        }
    }

    pub(crate) fn update_byte(&mut self, byte: u8) {
        self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(Self::PRIME);
    }

    pub(crate) const fn finish(self) -> u64 {
        self.0
    }
}

/// 64-bit FNV-1a over a contiguous slice.
pub(crate) fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hasher = Fnv1a::new();
    hasher.update(bytes);
    hasher.finish()
}

fn font_face_rules(name: &str, subsets: &[(&Face, SubsetData)]) -> Result<String, TextError> {
    let mut capacity = 0usize;
    for (face, data) in subsets {
        let variations = variation_defaults(face)?;
        let encoded_len = match data {
            SubsetData::Cached(entry) => entry.base64.len(),
            SubsetData::Uncached(bytes) => base64::encoded_len(bytes.len(), true).unwrap_or(0),
        };
        capacity = capacity
            .saturating_add(96)
            .saturating_add(name.len())
            .saturating_add(variations.len())
            .saturating_add(encoded_len);
    }
    let mut css = String::with_capacity(capacity);
    for (face, data) in subsets {
        let format = if data.as_bytes().starts_with(b"OTTO") {
            "otf"
        } else {
            "ttf"
        };
        let variations = variation_defaults(face)?;
        let _ = write!(
            css,
            "@font-face{{font-family:\"{name}\";font-weight:{};font-style:{};{variations}src:url(data:font/{format};base64,",
            face.weight().css_value(),
            face.style().css_keyword(),
        );
        match data {
            SubsetData::Cached(entry) => css.push_str(&entry.base64),
            SubsetData::Uncached(bytes) => {
                base64::engine::general_purpose::STANDARD.encode_string(bytes, &mut css);
            }
        }
        css.push_str(")}");
    }
    Ok(css)
}

/// A variable font is measured at its default location. Per-face descriptors freeze that same
/// location in the browser, including wght/opsz axes that CSS otherwise changes automatically.
fn variation_defaults(face: &Face) -> Result<&str, TextError> {
    face.meta()?
        .variation_defaults
        .as_deref()
        .ok_or(TextError::InvalidFont)
}

/// Parses the `fvar` default axis settings once per face; `None` signals an invalid `fvar` table.
pub(crate) fn parse_variation_defaults(font: &FontRef<'_>) -> Option<String> {
    if font.table_data(Tag::new(b"fvar")).is_none() {
        return Some(String::new());
    }
    let fvar = font.fvar().ok()?;
    let axes = fvar.axes().ok()?;
    let mut settings = Vec::new();
    for axis in axes {
        let mut tag = String::new();
        for byte in axis.axis_tag().to_be_bytes() {
            if !(0x20..=0x7e).contains(&byte) {
                return None;
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
        Some(String::new())
    } else {
        Some(format!("font-variation-settings:{};", settings.join(",")))
    }
}

#[cfg(test)]
mod selection_tests {
    use super::select_runs;
    use crate::{FontRegistry, FontStyle, FontWeight, Run};

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
    fn registered_cff_reuses_font_bytes_without_cache_entries() {
        let mut registry = crate::FontRegistry::new();
        let data = include_bytes!("../tests/fixtures/SourceCodePro-Regular.otf");
        registry
            .register("cff", data.to_vec(), FontWeight::Normal, FontStyle::Normal)
            .unwrap();
        let face = registry
            .resolve("cff", FontWeight::Normal, FontStyle::Normal)
            .unwrap();
        let crate::fonts::FontData::Registered {
            bytes: original,
            embed_whole: true,
            whole_embed: Some(precomputed),
        } = &face.data
        else {
            panic!("expected CFF");
        };
        let cache = Arc::new(SubsetCache::default());
        with_subset_cache(&cache, || {
            for text in ["alpha", "beta", "gamma"] {
                let mut characters: Vec<char> = text.chars().collect();
                characters.sort_unstable();
                characters.dedup();
                let SubsetData::Cached(embedded) = cached_subset(&face, &characters).unwrap()
                else {
                    panic!("expected cached CFF payload");
                };
                assert!(Arc::ptr_eq(precomputed, &embedded));
                assert!(Arc::ptr_eq(original, &embedded.bytes));
                assert_eq!(embedded.bytes.as_slice(), subset_face(&face, text).unwrap());
            }
        });
        assert!(cache.0.lock().unwrap().entries.is_empty());
    }

    #[test]
    fn streaming_fnv1a_matches_materialized_canonical_bytes() {
        let regular_bytes = subset_face(regular(), "Hello").unwrap();
        let bold_bytes = subset_face(bold(), "World").unwrap();
        let materialize = |items: &[(&Face, &[u8])]| {
            let mut raw = Vec::new();
            for (face, bytes) in items {
                raw.extend_from_slice(&face.weight().css_value().to_le_bytes());
                raw.push(match face.style() {
                    FontStyle::Normal => 0,
                    FontStyle::Italic => 1,
                });
                raw.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
                raw.extend_from_slice(bytes);
            }
            fnv1a(&raw)
        };
        let expected_single = materialize(&[(regular(), &regular_bytes)]);
        let expected_multi = materialize(&[(regular(), &regular_bytes), (bold(), &bold_bytes)]);

        let uncached_single = [(regular(), SubsetData::Uncached(regular_bytes.clone()))];
        assert_eq!(family_hash(&uncached_single), expected_single);

        let cached_regular = Arc::new(EmbeddedSubset::new(
            Arc::new(regular_bytes.clone()),
            regular().weight().css_value(),
            regular().style(),
        ));
        let cached_bold = Arc::new(EmbeddedSubset::new(
            Arc::new(bold_bytes.clone()),
            bold().weight().css_value(),
            bold().style(),
        ));
        let cached_single = [(regular(), SubsetData::Cached(Arc::clone(&cached_regular)))];
        assert_eq!(family_hash(&cached_single), expected_single);

        let uncached_multi = [
            (regular(), SubsetData::Uncached(regular_bytes)),
            (bold(), SubsetData::Uncached(bold_bytes)),
        ];
        assert_eq!(family_hash(&uncached_multi), expected_multi);

        let cached_multi = [
            (regular(), SubsetData::Cached(cached_regular)),
            (bold(), SubsetData::Cached(cached_bold)),
        ];
        assert_eq!(family_hash(&cached_multi), expected_multi);
    }

    #[test]
    fn subset_cache_hit_reuses_precomputed_payload() {
        let cache = Arc::new(SubsetCache::default());
        let mut chars: Vec<char> = "reuse".chars().collect();
        chars.sort_unstable();
        chars.dedup();
        with_subset_cache(&cache, || {
            let SubsetData::Cached(first) = cached_subset(regular(), &chars).unwrap() else {
                panic!("expected cached payload");
            };
            let SubsetData::Cached(second) = cached_subset(regular(), &chars).unwrap() else {
                panic!("expected cached payload");
            };
            assert!(Arc::ptr_eq(&first, &second));
            assert!(!first.base64.is_empty());
        });
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
                crate::fonts::FontData::Registered { bytes: data, .. } => Arc::downgrade(data),
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
