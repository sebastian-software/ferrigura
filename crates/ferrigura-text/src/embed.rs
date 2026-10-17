//! Font subsets for embedding in an SVG, so a viewer draws every label in the face it was measured
//! in.
//!
//! Each face is cut down with `skera` (Google's port of `hb-subset`) to the characters the diagram
//! draws with it. The subsets keep the default layout features, so kerning and ligatures survive,
//! keep the legacy `kern` table, and drop hinting. Each face becomes an `@font-face` rule with the
//! subset as a base64 TTF data URL.
//!
//! `@font-face` rules are global to the HTML page, so a family name has to be unique to its subset:
//! two diagrams with different subsets under one name would break each other's labels. The name
//! is `fg-` and a 64-bit FNV-1a hash of the subsets, then the family suffix, such as
//! `fg-0123456789abcdef-sans`. Equal subsets get equal names, and the hash is computed over the
//! subset bytes and their weight and style, so a different subset gets a different name.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use base64::Engine;
use read_fonts::FontRef;
use read_fonts::collections::IntSet;
use read_fonts::types::{NameId, Tag};
use skera::{DEFAULT_LAYOUT_FEATURES, Plan, SubsetFlags, subset_font};

use crate::error::TextError;
use crate::fonts::{FACES, Face, FontFamily, FontStyle};
use crate::measure::Run;

/// The embedded subsets of one font family in one diagram.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedFamily {
    /// The family whose faces are embedded.
    pub family: FontFamily,
    /// The family name the labels refer to, such as `fg-0123456789abcdef-sans`.
    pub name: String,
    /// One `@font-face` rule for each face of the family the diagram uses. Each rule names
    /// [`EmbeddedFamily::name`] and holds the subset as a base64 TTF data URL.
    pub css: String,
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
/// Returns an error if a bundled face cannot be parsed or subsetting fails.
pub fn embed_runs(runs: &[Run<'_>]) -> Result<Vec<EmbeddedFamily>, TextError> {
    let mut embedded = Vec::new();
    for family in [FontFamily::Sans, FontFamily::Mono] {
        let mut subsets: Vec<(&'static Face, Vec<u8>)> = Vec::new();
        for face in FACES.iter().copied().filter(|f| f.family() == family) {
            let used = runs.iter().any(|run| same_face(run.face, face));
            if !used {
                continue;
            }
            let characters: BTreeSet<char> = runs
                .iter()
                .filter(|run| same_face(run.face, face))
                .flat_map(|run| run.text.chars())
                .collect();
            subsets.push((face, subset_characters(face, &characters)?));
        }
        if subsets.is_empty() {
            continue;
        }
        let name = family_name(family, &subsets);
        let css = font_face_rules(&name, &subsets);
        embedded.push(EmbeddedFamily { family, name, css });
    }
    Ok(embedded)
}

/// The subset of `face` to the characters of `text`, with the default layout features, the legacy
/// `kern` table and no hinting.
///
/// # Errors
///
/// Returns an error if the bundled face cannot be parsed or subsetting fails.
pub fn subset_face(face: &Face, text: &str) -> Result<Vec<u8>, TextError> {
    let characters: BTreeSet<char> = text.chars().collect();
    subset_characters(face, &characters)
}

fn same_face(a: &Face, b: &Face) -> bool {
    a.family() == b.family() && a.weight() == b.weight() && a.style() == b.style()
}

/// The white space that SVG text collapses: the browser draws each run of it as one space, so a
/// text with a line break or a tab needs the space glyph even when it has no space of its own.
const COLLAPSED_WHITE_SPACE: [char; 3] = ['\n', '\r', '\t'];

fn subset_characters(face: &Face, characters: &BTreeSet<char>) -> Result<Vec<u8>, TextError> {
    let font = FontRef::new(face.data()).map_err(|_| TextError::InvalidFont)?;
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
    // Every script and the default layout features, so kerning and ligatures are kept.
    let mut scripts = IntSet::empty();
    scripts.invert();
    let mut features = IntSet::empty();
    features.extend(DEFAULT_LAYOUT_FEATURES.iter().copied());
    // The name records 0 to 6 in English, which is what a browser needs for the family.
    let mut name_ids = IntSet::empty();
    name_ids.insert_range(NameId::from(0)..=NameId::from(6));
    let mut languages = IntSet::empty();
    languages.insert(0x0409);
    // No table is dropped: the legacy `kern` table stays, as it does in the spike. Hinting goes.
    let no_tables: IntSet<Tag> = IntSet::empty();
    let plan = Plan::new(
        &IntSet::empty(),
        &unicodes,
        &font,
        SubsetFlags::SUBSET_FLAGS_NO_HINTING,
        &no_tables,
        &scripts,
        &features,
        &name_ids,
        &languages,
    );
    subset_font(&font, &plan).map_err(|error| TextError::Subset(error.to_string()))
}

/// The family name of a set of subsets: the hash of each face's weight, style and bytes.
fn family_name(family: FontFamily, subsets: &[(&'static Face, Vec<u8>)]) -> String {
    let hash = fnv1a(&canonical_bytes(subsets));
    format!("fg-{hash:016x}-{}", family.css_suffix())
}

/// The bytes the family hash covers, face by face: the weight, the style, the length of the subset
/// and the subset itself. Fixed-width integers are little-endian, so the bytes do not depend on the
/// platform.
fn canonical_bytes(subsets: &[(&'static Face, Vec<u8>)]) -> Vec<u8> {
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
fn fnv1a(bytes: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    bytes.iter().fold(OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(PRIME)
    })
}

fn font_face_rules(name: &str, subsets: &[(&'static Face, Vec<u8>)]) -> String {
    let mut css = String::new();
    for (face, data) in subsets {
        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        let _ = write!(
            css,
            "@font-face{{font-family:\"{name}\";font-weight:{};font-style:{};src:url(data:font/ttf;base64,{encoded})}}",
            face.weight().css_value(),
            face.style().css_keyword(),
        );
    }
    css
}
