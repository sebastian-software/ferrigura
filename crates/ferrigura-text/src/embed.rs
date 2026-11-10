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

use std::collections::BTreeSet;
use std::fmt::Write as _;

use base64::Engine;
use read_fonts::FontRef;
use read_fonts::collections::IntSet;
use read_fonts::types::{NameId, Tag};
use skera::{DEFAULT_LAYOUT_FEATURES, Plan, SubsetFlags, subset_font};

use crate::error::TextError;
use crate::fonts::{Face, FontFamily, FontStyle};
use crate::measure::{Run, fallback_runs};

/// The embedded subsets of one font family in one diagram.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedFamily {
    /// The family whose faces are embedded.
    pub family: FontFamily,
    /// The host-registered family name, or the bundled face identifier.
    pub source_name: String,
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
/// Returns an error if a bundled face cannot be parsed or subsetting fails.
pub fn embed_runs(runs: &[Run<'_, '_>]) -> Result<Vec<EmbeddedFamily>, TextError> {
    // The stretches each run is set in, after the fallback to the symbol face: a subset holds the
    // characters drawn in its face, and no other.
    let stretches: Vec<Run<'_, '_>> = runs.iter().flat_map(|run| fallback_runs(*run)).collect();
    let mut faces: Vec<&Face> = stretches.iter().map(|run| run.face).collect();
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
    let mut embedded = Vec::new();
    let mut index = 0;
    while index < faces.len() {
        let primary = faces[index];
        let mut subsets = Vec::new();
        while index < faces.len() && same_family(primary, faces[index]) {
            let face = faces[index];
            let characters: BTreeSet<char> = stretches
                .iter()
                .filter(|run| same_face(run.face, face))
                .flat_map(|run| run.text.chars())
                .collect();
            subsets.push((face, subset_characters(face, &characters)?));
            index += 1;
        }
        let name = family_name(primary.family(), &subsets);
        let css = font_face_rules(&name, &subsets);
        embedded.push(EmbeddedFamily {
            family: primary.family(),
            source_name: primary.family_name().to_owned(),
            name,
            css_family: String::new(),
            css,
            variables: Vec::new(),
        });
    }
    for index in 0..embedded.len() {
        let primary = faces.iter().find(|face| {
            face.family() == embedded[index].family
                && (face.family() != FontFamily::Registered
                    || face.family_name() == embedded[index].source_name)
        });
        let fallback_names: Vec<String> = if embedded[index].family == FontFamily::Registered {
            primary
                .into_iter()
                .flat_map(|face| face.registered_fallbacks())
                .filter_map(|fallback| {
                    embedded
                        .iter()
                        .find(|family| {
                            family.family == FontFamily::Registered
                                && family
                                    .source_name
                                    .eq_ignore_ascii_case(fallback.family_name())
                        })
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
                    .find(|family| {
                        family.family == face.family()
                            && family.source_name.eq_ignore_ascii_case(face.family_name())
                    })
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

/// The subset of `face` to the characters of `text`, with the default layout features, the legacy
/// `kern` table and no hinting. CFF and CFF2 fonts are returned complete because the subsetter
/// cannot serialize their outlines; embedding preserves their appearance and advances.
///
/// # Errors
///
/// Returns an error if the bundled face cannot be parsed or subsetting fails.
pub fn subset_face(face: &Face, text: &str) -> Result<Vec<u8>, TextError> {
    let characters: BTreeSet<char> = text.chars().collect();
    subset_characters(face, &characters)
}

fn same_face(a: &Face, b: &Face) -> bool {
    same_family(a, b)
        && a.weight().css_value() == b.weight().css_value()
        && a.style() == b.style()
        && a.data() == b.data()
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
fn family_name(family: FontFamily, subsets: &[(&Face, Vec<u8>)]) -> String {
    let hash = fnv1a(&canonical_bytes(subsets));
    format!("fg-{hash:016x}-{}", family.css_suffix())
}

/// The bytes the family hash covers, face by face: the weight, the style, the length of the subset
/// and the subset itself. Fixed-width integers are little-endian, so the bytes do not depend on the
/// platform.
fn canonical_bytes(subsets: &[(&Face, Vec<u8>)]) -> Vec<u8> {
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

fn font_face_rules(name: &str, subsets: &[(&Face, Vec<u8>)]) -> String {
    let mut css = String::new();
    for (face, data) in subsets {
        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        let format = if data.starts_with(b"OTTO") {
            "otf"
        } else {
            "ttf"
        };
        let _ = write!(
            css,
            "@font-face{{font-family:\"{name}\";font-weight:{};font-style:{};src:url(data:font/{format};base64,{encoded})}}",
            face.weight().css_value(),
            face.style().css_keyword(),
        );
    }
    css
}
