//! Advance widths of text, shaped with the font's default features.
//!
//! The width of a run is the sum of its shaped advances, so kerning and ligatures are included,
//! as they are in the browser. Shaping a run never crosses the run's own boundaries.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, OnceLock, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

use harfrust::font::Font;
use harfrust::{Buffer, Direction, ShapeOptions, ShapePlan, ShapePlanKey, ShaperFont, shape};
use unicode_segmentation::UnicodeSegmentation;

use crate::error::TextError;
use crate::fonts::{Face, FontData, FontFamily, FontStyle, FontWeight, face};

/// One run of text set in one face, for [`measure_line`].
#[derive(Debug, Clone, Copy)]
pub struct Run<'a, 'f> {
    /// The text of the run.
    pub text: &'a str,
    /// The face the run is set in.
    pub face: &'f Face,
}

/// The faces a text's font-family list falls back to, in order, after the text's own face: the
/// arrows, the text symbols and the emoji.
///
/// The three cover disjoint code points (the emoji face holds no text-default symbol, and the text
/// symbols hold no emoji-presentation code point), so the order decides nothing among them.
#[must_use]
pub fn fallback_faces() -> [&'static Face; 3] {
    [
        face(FontFamily::Arrows, FontWeight::Normal, FontStyle::Normal),
        face(
            FontFamily::TextSymbols,
            FontWeight::Normal,
            FontStyle::Normal,
        ),
        face(FontFamily::Emoji, FontWeight::Normal, FontStyle::Normal),
    ]
}

/// Resolves the same fallback runs as measurement and checks their shaped glyphs.
///
/// This internal preflight must receive visible text, not measurement-only placeholders. Its
/// offsets are relative to the run. It does not validate bidi layout or viewer font selection.
///
/// # Errors
/// Returns a missing-glyph error for a visible grapheme that shapes to glyph zero.
#[doc(hidden)]
pub fn checked_fallback_runs<'a, 'f>(
    run: Run<'a, 'f>,
    requested_family: &str,
) -> Result<Vec<Run<'a, 'f>>, TextError> {
    let resolved = fallback_runs(run);
    let mut start = 0;
    for stretch in &resolved {
        let shaped = shape_face(stretch.face, stretch.text)?;
        for glyph in &shaped.glyphs {
            if glyph.glyph != 0 {
                continue;
            }
            let offset = start + glyph.cluster as usize;
            let (byte_offset, sequence) = run
                .text
                .grapheme_indices(true)
                .find(|(index, sequence)| *index <= offset && offset < *index + sequence.len())
                .ok_or_else(|| TextError::Registration("invalid shaped cluster offset".into()))?;
            if !sequence.chars().all(char::is_control) {
                return Err(TextError::MissingGlyph {
                    sequence: sequence.to_owned(),
                    family: requested_family.to_owned(),
                    byte_offset,
                });
            }
        }
        start += stretch.text.len();
    }
    Ok(resolved)
}

/// The stretches of a run as a browser sets them with the run's font-family list, which is the
/// run's own face and then the fallback faces of [`fallback_faces`].
///
/// A character the run's face lacks is set in the first fallback face that covers it. Every other
/// character stays in the run's face, where a character that no face covers is drawn with the
/// face's own missing glyph. Consecutive characters in one face form one stretch, so the stretches
/// follow each other in the order of the text and cover all of it. A run without any such character
/// comes back as itself, and so does an empty run, which keeps its place in the embedded subsets.
/// A run in a fallback face is not split.
///
/// This is the per-character fallback that Chromium makes for a font-family list: a run of text is
/// split where the first family lacks a character, and each part is shaped in the family that has
/// it. Every measurement and every embedded subset goes through it, so a viewer draws the characters
/// in the face they were measured in.
#[must_use]
pub fn fallback_runs<'a, 'f>(run: Run<'a, 'f>) -> Vec<Run<'a, 'f>> {
    let mut stretches = Vec::with_capacity(1);
    let _ = for_each_fallback_run::<std::convert::Infallible>(run, |stretch| {
        stretches.push(stretch);
        Ok(())
    });
    stretches
}

/// Visits each fallback stretch of `run` in order without allocating when all characters stay in
/// `run.face` (or when falling back to registered/bundled faces).
pub(crate) fn for_each_fallback_run<'a, 'f, E>(
    run: Run<'a, 'f>,
    mut visit: impl FnMut(Run<'a, 'f>) -> Result<(), E>,
) -> Result<(), E> {
    if run.text.is_empty()
        || !matches!(
            run.face.family(),
            FontFamily::Sans | FontFamily::Mono | FontFamily::Registered
        )
        || (run.face.family() == FontFamily::Registered
            && run.face.registered_fallbacks().is_empty())
    {
        return visit(run);
    }
    let primary_charmap = run.face.font().ok().map(|font| font.charmap());
    let mut start = 0;
    let mut current: &Face = run.face;
    for (index, character) in run.text.char_indices() {
        let covered = primary_charmap
            .as_ref()
            .is_some_and(|charmap| charmap.map_unicode(character).is_some());
        let chosen = if covered {
            run.face
        } else {
            find_fallback(run.face, character)
        };
        if !std::ptr::eq(chosen, current) {
            if index > start {
                visit(Run {
                    text: &run.text[start..index],
                    face: current,
                })?;
            }
            start = index;
            current = chosen;
        }
    }
    visit(Run {
        text: &run.text[start..],
        face: current,
    })
}

fn find_fallback(primary: &Face, character: char) -> &Face {
    if primary.family() == FontFamily::Registered {
        primary
            .registered_fallbacks()
            .iter()
            .map(AsRef::as_ref)
            .find(|fallback| fallback.covers(character))
            .unwrap_or(primary)
    } else {
        fallback_faces()
            .into_iter()
            .find(|fallback| fallback.covers(character))
            .unwrap_or(primary)
    }
}

/// The width of a text run as Chromium lays it out, in pixels: its advance, rounded up to a
/// multiple of 1/64 px.
///
/// Blink keeps positions in layout units of 1/64 px and converts each run's advance with a round up
/// (`LayoutUnit::FromFloatCeil`), so the run after it starts on the grid, and a line's width, the
/// `getComputedTextLength` that Mermaid's wrapping compares, is the sum of the snapped runs. A width
/// within float noise of a grid point is taken to be on it. Verified against Chromium in
/// `tests/labels_chromium.rs` (`docs/diagrams/flowchart.md#labels`).
#[must_use]
pub fn layout_width(advance: f64) -> f64 {
    const UNITS_PER_PIXEL: f64 = 64.0;
    ((advance * UNITS_PER_PIXEL) - 1e-3).ceil() / UNITS_PER_PIXEL
}

/// The advance width, in pixels, of `text` set as one run in `face` at `size` pixels. The characters
/// that `face` lacks and a fallback face has are set in that face ([`fallback_runs`]), and the
/// advances of the stretches are summed without any snapping: a caller snaps the whole run.
///
/// # Errors
///
/// Returns an error if the font cannot be parsed or the shaper rejects the text.
pub fn measure(face: &Face, text: &str, size: f64) -> Result<f64, TextError> {
    let mut total = 0.0;
    for_each_fallback_run(Run { text, face }, |stretch| {
        let font = stretch.face.font()?;
        let shaped = shape_face(stretch.face, stretch.text)?;
        total += shaped.advance as f64 * size / f64::from(font.units_per_em());
        Ok(())
    })?;
    Ok(total)
}

/// As [`measure`], for font data that is not one of the bundled faces, such as a subset made by
/// [`crate::embed`].
///
/// # Errors
///
/// Returns an error if `data` does not parse as a font or the shaper rejects the text.
pub fn measure_data(data: &[u8], text: &str, size: f64) -> Result<f64, TextError> {
    let font = Font::new(data.to_vec(), 0).ok_or(TextError::InvalidFont)?;
    advance(&font, text, size)
}

/// The advance width, in pixels, of a line made of runs at `size` pixels.
///
/// Each run is measured in its own face, with the fallback of [`measure`], and the widths are summed.
/// No kerning or ligature is applied across two runs, which matches the reference: Mermaid puts each
/// styled word in its own `tspan`, and a change of face starts a new shaping run in the browser too.
///
/// Mermaid draws every word after the first with a leading space in its own `tspan`, and that
/// space takes the style of the word. To lay a line out the way Mermaid does, start each run with
/// the space that precedes it, as in `" bold"` in bold after `"Mixed"` in regular.
///
/// # Errors
///
/// Returns the first error [`measure`] reports for a run.
pub fn measure_line(runs: &[Run<'_, '_>], size: f64) -> Result<f64, TextError> {
    let mut total = 0.0;
    for run in runs {
        total += measure(run.face, run.text, size)?;
    }
    Ok(total)
}

fn advance(font: &Font, text: &str, size: f64) -> Result<f64, TextError> {
    let shaped = shape_run(font, None, text)?;
    Ok(shaped.advance as f64 * size / f64::from(font.units_per_em()))
}

/// One glyph of a shaped run, with the pen position its outline is drawn from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlacedGlyph {
    /// The glyph id in the font the run was shaped in.
    pub(crate) glyph: u32,
    /// UTF-8 byte offset of the source shaping cluster within the run.
    pub(crate) cluster: u32,
    /// The x of the glyph's origin from the start of the run, in font units: the pen position plus
    /// the glyph's own offset.
    pub(crate) x: i64,
}

/// A shaped run: its glyphs in order and its advance, both in font units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShapedRun {
    pub(crate) glyphs: Vec<PlacedGlyph>,
    /// The sum of the glyph advances, in font units.
    pub(crate) advance: i64,
}

const MAX_ENTRIES: usize = 1024;
const MAX_BYTES: usize = 1024 * 1024;

#[derive(Default)]
struct ShapeCache {
    faces: HashMap<(usize, usize), FaceCache>,
    entries: usize,
    bytes: usize,
}

struct FaceCache {
    // Keep registered font bytes alive while their address is used as identity.
    _data: FontData,
    runs: HashMap<String, Rc<ShapedRun>>,
}

thread_local! {
    static CACHE: RefCell<Option<ShapeCache>> = const { RefCell::new(None) };
}

/// Runs one render with a bounded cache of shaped runs. Nested scopes are independent.
pub fn render_scope<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(Option<ShapeCache>);
    impl Drop for Restore {
        fn drop(&mut self) {
            CACHE.with(|cache| *cache.borrow_mut() = self.0.take());
        }
    }
    let previous = CACHE.with(|cache| cache.replace(Some(ShapeCache::default())));
    let _restore = Restore(previous);
    f()
}

pub(crate) enum Shaped {
    Owned(ShapedRun),
    Shared(Rc<ShapedRun>),
}

impl std::ops::Deref for Shaped {
    type Target = ShapedRun;
    fn deref(&self) -> &ShapedRun {
        match self {
            Self::Owned(run) => run,
            Self::Shared(run) => run,
        }
    }
}

pub(crate) fn shape_face(face: &Face, text: &str) -> Result<Shaped, TextError> {
    let data = face.data();
    let key = (data.as_ptr() as usize, data.len());
    if let Some(hit) = CACHE.with(|cache| {
        cache
            .borrow()
            .as_ref()?
            .faces
            .get(&key)?
            .runs
            .get(text)
            .cloned()
    }) {
        return Ok(Shaped::Shared(hit));
    }
    let shaped = shape_run(face.font()?, Some(&face.plans), text)?;
    Ok(CACHE.with(|cache| {
        if let Some(cache) = cache.borrow_mut().as_mut() {
            let bytes = 256usize.saturating_add(text.len()).saturating_add(
                shaped
                    .glyphs
                    .capacity()
                    .saturating_mul(std::mem::size_of::<PlacedGlyph>()),
            );
            if cache.entries < MAX_ENTRIES && bytes <= MAX_BYTES.saturating_sub(cache.bytes) {
                let shaped = Rc::new(shaped);
                cache
                    .faces
                    .entry(key)
                    .or_insert_with(|| FaceCache {
                        _data: face.data.clone(),
                        runs: HashMap::new(),
                    })
                    .runs
                    .insert(text.to_owned(), Rc::clone(&shaped));
                cache.entries += 1;
                cache.bytes += bytes;
                return Shaped::Shared(shaped);
            }
        }
        Shaped::Owned(shaped)
    }))
}

/// The most shape plans one face keeps. Labels need one plan per script they are written in, so a
/// few cover every diagram; text in further scripts is shaped with a plan built for the run.
const MAX_PLANS: usize = 8;

/// The shape plans of one face, kept with the face for as long as it lives.
///
/// A plan holds the features and lookups HarfRust selected for the font and a buffer's segment
/// properties, and compiling it is a large part of shaping a short label. It borrows nothing, so it
/// is reused across renders and threads. A plan is picked by HarfRust's own [`ShapePlanKey`] for the
/// buffer's script, direction and language and no user features, so a run is shaped with the plan
/// that [`shape`] would have built for it; the font's variation position is part of that key too.
///
/// The clones of a face share its plans: a registered face is cloned whenever it is resolved, and
/// every clone keeps the font of the face it was cloned from.
pub(crate) struct ShapePlans(OnceLock<Arc<RwLock<Vec<Arc<ShapePlan>>>>>);

impl ShapePlans {
    pub(crate) const fn new() -> Self {
        Self(OnceLock::new())
    }

    fn shared(&self) -> &Arc<RwLock<Vec<Arc<ShapePlan>>>> {
        self.0.get_or_init(Arc::default)
    }

    // A panic cannot leave the list half written (a plan is pushed whole), so a poisoned lock
    // still holds valid plans.
    fn read(&self) -> RwLockReadGuard<'_, Vec<Arc<ShapePlan>>> {
        self.shared().read().unwrap_or_else(PoisonError::into_inner)
    }

    fn write(&self) -> RwLockWriteGuard<'_, Vec<Arc<ShapePlan>>> {
        self.shared()
            .write()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// The plan for shaping `buffer` in `font`, which must be the font of the face these plans
    /// belong to. `None` leaves building the plan to [`shape`].
    fn plan(&self, font: &Font, buffer: &Buffer) -> Option<Arc<ShapePlan>> {
        let (direction, script, language) =
            (buffer.direction(), buffer.script(), buffer.language());
        if direction == Direction::Invalid {
            return None;
        }
        let key = ShapePlanKey::new(font, script, direction).language(language);
        if let Some(plan) = self.read().iter().find(|plan| key.matches(plan)) {
            return Some(Arc::clone(plan));
        }
        let plan = Arc::new(ShapePlan::new(font, direction, script, language, &[]));
        let mut plans = self.write();
        if let Some(plan) = plans.iter().find(|plan| key.matches(plan)) {
            return Some(Arc::clone(plan));
        }
        if plans.len() < MAX_PLANS {
            plans.push(Arc::clone(&plan));
        }
        Some(plan)
    }
}

impl Clone for ShapePlans {
    fn clone(&self) -> Self {
        Self(OnceLock::from(Arc::clone(self.shared())))
    }
}

const MAX_REUSED_BUFFER_BYTES: usize = 4096;

thread_local! {
    static SHAPE_BUFFER: RefCell<Option<Buffer>> = const { RefCell::new(None) };
}

/// Shapes `text` with the font's default features, and reports where each glyph is drawn. `plans`
/// are the shape plans of the face `font` belongs to; without them the plan is built for this run.
pub(crate) fn shape_run(
    font: &Font,
    plans: Option<&ShapePlans>,
    text: &str,
) -> Result<ShapedRun, TextError> {
    let shaper = ShaperFont::new(font);
    let mut buffer = SHAPE_BUFFER
        .with(|slot| slot.borrow_mut().take())
        .unwrap_or_default();
    buffer.clear();
    buffer.push_str(text);
    buffer.guess_segment_properties();
    let plan = plans.and_then(|plans| plans.plan(font, &buffer));
    shape(
        &shaper,
        &mut buffer,
        ShapeOptions::new().plan(plan.as_deref()),
    )
    .map_err(TextError::Shape)?;
    let mut pen: i64 = 0;
    let mut glyphs = Vec::with_capacity(buffer.len());
    for (info, position) in buffer.glyph_infos().iter().zip(buffer.glyph_positions()) {
        glyphs.push(PlacedGlyph {
            glyph: info.glyph_id,
            cluster: info.cluster,
            x: pen + i64::from(position.x_offset),
        });
        pen += i64::from(position.x_advance);
    }
    if text.len() <= MAX_REUSED_BUFFER_BYTES {
        buffer.clear();
        SHAPE_BUFFER.with(|slot| *slot.borrow_mut() = Some(buffer));
    }
    Ok(ShapedRun {
        glyphs,
        advance: pen,
    })
}

#[cfg(test)]
mod tests {
    use super::layout_width;

    #[test]
    fn scoped_shapes_reuse_and_restore_after_nested_panic() {
        use super::{CACHE, render_scope, shape_face};
        use crate::fonts::{FontFamily, FontStyle, FontWeight, face};
        let face = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
        render_scope(|| {
            let first = shape_face(face, "office →").unwrap();
            let hit = shape_face(face, "office →").unwrap();
            assert!(
                matches!((&first, &hit), (super::Shaped::Shared(a), super::Shaped::Shared(b)) if std::rc::Rc::ptr_eq(a, b))
            );
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                render_scope(|| {
                    let inner = shape_face(face, "office →").unwrap();
                    assert!(
                        matches!((&first, &inner), (super::Shaped::Shared(a), super::Shaped::Shared(b)) if !std::rc::Rc::ptr_eq(a, b))
                    );
                    panic!("test unwind");
                })
            }));
            let restored = shape_face(face, "office →").unwrap();
            assert!(
                matches!((&first, &restored), (super::Shaped::Shared(a), super::Shaped::Shared(b)) if std::rc::Rc::ptr_eq(a, b))
            );
        });
        CACHE.with(|cache| assert!(cache.borrow().is_none()));
    }

    #[test]
    fn scoped_results_match_direct_shapes_across_faces_and_threads() {
        use super::{Run, ShapePlans, fallback_runs, render_scope, shape_face, shape_run};
        use crate::fonts::{FontFamily, FontStyle, FontWeight, face};
        // A copy of the regular face with no plans yet, so the threads build its plans at the
        // same time; the bundled faces share the plans every other test builds.
        let mut cold = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal).clone();
        cold.plans = ShapePlans::new();
        let cold = &cold;
        std::thread::scope(|scope| {
            for thread in 0..4 {
                scope.spawn(move || {
                    let check = || {
                        let faces = [
                            face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal),
                            face(FontFamily::Sans, FontWeight::Normal, FontStyle::Italic),
                            cold,
                        ];
                        for face in faces {
                            for text in ["office → ☀", "שלום → 😀", "αβγ 12"] {
                                for stretch in fallback_runs(Run { face, text }) {
                                    let expected =
                                        shape_run(stretch.face.font().unwrap(), None, stretch.text)
                                            .unwrap();
                                    for _ in 0..2 {
                                        assert_eq!(
                                            *shape_face(stretch.face, stretch.text).unwrap(),
                                            expected
                                        );
                                    }
                                }
                            }
                        }
                    };
                    // Half the threads shape inside a render, half outside one, where only the
                    // plans are reused.
                    if thread % 2 == 0 {
                        render_scope(check);
                    } else {
                        check();
                    }
                });
            }
        });
    }

    /// Glyph ids, clusters, advances and offsets of `text` shaped with HarfRust's `shape`, with
    /// the plan taken from `plans`, or without them as plain `shape`, which builds its plan for the
    /// call.
    fn raw_shape(
        font: &harfrust::font::Font,
        plans: Option<&super::ShapePlans>,
        text: &str,
    ) -> Vec<(u32, u32, i32, i32, i32, i32)> {
        use harfrust::{Buffer, ShapeOptions, ShaperFont, shape};
        let shaper = ShaperFont::new(font);
        let mut buffer = Buffer::new();
        buffer.push_str(text);
        buffer.guess_segment_properties();
        let plan = plans.map(|plans| {
            plans
                .plan(font, &buffer)
                .expect("a guessed buffer has a plan")
        });
        shape(
            &shaper,
            &mut buffer,
            ShapeOptions::new().plan(plan.as_deref()),
        )
        .unwrap();
        buffer
            .glyph_infos()
            .iter()
            .zip(buffer.glyph_positions())
            .map(|(info, position)| {
                (
                    info.glyph_id,
                    info.cluster,
                    position.x_advance,
                    position.y_advance,
                    position.x_offset,
                    position.y_offset,
                )
            })
            .collect()
    }

    const VARIED_TEXTS: [&str; 22] = [
        "",
        " ",
        "  \t ",
        "Hello, World!",
        "0123456789 +-*/=().,;:!?\"'%&#@",
        "AV To Wa ffi fl Ty",
        "office → ☀",
        "←↑→↓⇒⇔",
        "♥ ☂ ✓ ★",
        "😀👍🏽 🇩🇪 ❤️",
        "a→b😀c♥d",
        "Ünïcödé façade naïve",
        "αβγ δέλτα",
        "кириллица",
        "مرحبا بالعالم",
        "שלום עולם",
        "漢字とかな",
        "ไทย",
        "देवनागरी",
        "\u{FB02}°name¶ß",
        "x\u{301}y\u{200D}z",
        "line\nbreak",
    ];

    #[test]
    fn planned_shapes_match_plain_shapes_in_every_bundled_face() {
        use super::{Run, fallback_runs, render_scope, shape_face, shape_run};
        use crate::fonts::FACES;
        for face in FACES {
            let font = face.font().unwrap();
            for text in VARIED_TEXTS {
                // Twice, so the second shape reuses the plan the first one built.
                for _ in 0..2 {
                    assert_eq!(
                        raw_shape(font, Some(&face.plans), text),
                        raw_shape(font, None, text),
                        "{} {text:?}",
                        face.file_name()
                    );
                }
                let expected = shape_run(font, None, text).unwrap();
                assert_eq!(shape_run(font, Some(&face.plans), text).unwrap(), expected);
                assert_eq!(*shape_face(face, text).unwrap(), expected);
                render_scope(|| assert_eq!(*shape_face(face, text).unwrap(), expected));
                // The stretches a measurement shapes, in the fallback faces as well.
                for stretch in fallback_runs(Run { text, face }) {
                    let font = stretch.face.font().unwrap();
                    assert_eq!(
                        raw_shape(font, Some(&stretch.face.plans), stretch.text),
                        raw_shape(font, None, stretch.text),
                        "{} {:?}",
                        stretch.face.file_name(),
                        stretch.text
                    );
                }
            }
        }
    }

    #[test]
    fn a_face_keeps_a_bounded_number_of_plans() {
        use super::{MAX_PLANS, ShapePlans};
        use crate::fonts::{FontFamily, FontStyle, FontWeight, face};
        let mut fresh = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal).clone();
        fresh.plans = ShapePlans::new();
        let font = fresh.font().unwrap();
        // More scripts than the face keeps plans for; the ones past the bound still shape alike.
        for text in VARIED_TEXTS
            .iter()
            .chain(&["ᚠᚢᚦ", "ქართული", "Հայերեն", "ᎠᎡᎢ", "ㄅㄆㄇ"])
        {
            assert_eq!(
                raw_shape(font, Some(&fresh.plans), text),
                raw_shape(font, None, text)
            );
        }
        assert_eq!(fresh.plans.read().len(), MAX_PLANS);
    }

    #[test]
    fn clones_of_a_face_share_its_plans() {
        use super::{ShapePlans, shape_face};
        use crate::fonts::{FontFamily, FontStyle, FontWeight, face};
        let mut original = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal).clone();
        original.plans = ShapePlans::new();
        let copy = original.clone();
        shape_face(&copy, "shared").unwrap();
        assert_eq!(original.plans.read().len(), 1);
        assert!(std::sync::Arc::ptr_eq(
            original.plans.shared(),
            copy.plans.shared()
        ));
    }

    #[test]
    fn oversized_runs_are_not_cached() {
        use super::{CACHE, render_scope, shape_face};
        use crate::fonts::{FontFamily, FontStyle, FontWeight, face};
        let face = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
        render_scope(|| {
            let text = "a".repeat(70_000);
            shape_face(face, &text).unwrap();
            CACHE.with(|cache| assert_eq!(cache.borrow().as_ref().unwrap().entries, 0));
        });
    }

    #[test]
    fn a_width_is_rounded_up_to_the_layout_unit() {
        // 38.1 px is not on the grid of 1/64 px; the next grid point above it is 38.109375.
        assert_eq!(layout_width(38.1), 38.109375);
        // 26 px is on the grid and stays where it is, even with float noise.
        assert_eq!(layout_width(26.0), 26.0);
        assert_eq!(layout_width(26.000_000_01), 26.0);
        // Float noise just above a grid point is still on it; a clear excess is rounded up.
        assert_eq!(layout_width(10.015_625_5), 10.015625);
        assert_eq!(layout_width(10.02), 10.03125);
    }

    #[test]
    fn fallback_fast_path_and_mixed_stretches_match() {
        use super::{Run, fallback_runs, for_each_fallback_run};
        use crate::fonts::{FontFamily, FontStyle, FontWeight, face};
        use crate::registry::FontRegistry;

        let sans = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
        for text in ["", "Hello, World!", "AV To Wa", "a→b", "→", "a→b😀c♥d"] {
            let mut visited = Vec::new();
            let _ = for_each_fallback_run::<std::convert::Infallible>(
                Run { text, face: sans },
                |stretch| {
                    visited.push((stretch.text, std::ptr::from_ref(stretch.face)));
                    Ok(())
                },
            );
            let expected: Vec<_> = fallback_runs(Run { text, face: sans })
                .into_iter()
                .map(|stretch| (stretch.text, std::ptr::from_ref(stretch.face)))
                .collect();
            assert_eq!(visited, expected, "text={text:?}");
        }

        // Registered face without fallbacks stays a single stretch even for uncovered characters.
        let mut registry = FontRegistry::new();
        registry
            .register(
                "only",
                sans.data().to_vec(),
                FontWeight::Normal,
                FontStyle::Normal,
            )
            .unwrap();
        let registered = registry
            .resolve("only", FontWeight::Normal, FontStyle::Normal)
            .unwrap();
        let stretches = fallback_runs(Run {
            text: "a→b😀",
            face: &registered,
        });
        assert_eq!(stretches.len(), 1);
        assert_eq!(stretches[0].text, "a→b😀");
        assert!(std::ptr::eq(stretches[0].face, &*registered));
    }

    #[test]
    fn reused_shape_buffer_resets_segment_properties_deterministically() {
        use super::shape_run;
        use crate::fonts::{FontFamily, FontStyle, FontWeight, face};

        let sans = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
        let font = sans.font().unwrap();
        // Verify Buffer::clear() restores default segment properties identical to Buffer::new().
        {
            let fresh = super::Buffer::new();
            let mut reused = super::Buffer::new();
            reused.push_str("שלום עולם");
            reused.guess_segment_properties();
            assert_ne!(reused.direction(), fresh.direction());
            reused.clear();
            assert_eq!(reused.direction(), fresh.direction());
            assert_eq!(reused.script(), fresh.script());
            assert_eq!(reused.language(), fresh.language());
        }
        // Alternate RTL, LTR, empty, emoji, combining marks, and kerning pairs to prove the
        // thread-local buffer resets direction/script/language cleanly between runs.
        let sequence = [
            "שלום עולם",
            "Hello, World!",
            "مرحبا بالعالم",
            "",
            "AV To Wa ffi fl Ty",
            "漢字とかな",
            "😀👍🏽 🇩🇪 ❤️",
            "x\u{301}y\u{200D}z",
            "שלום",
            "Hello, World!",
        ];
        for text in sequence {
            let actual = shape_run(font, Some(&sans.plans), text).unwrap();
            let raw = raw_shape(font, None, text);
            let expected_glyphs: Vec<_> = {
                let mut pen = 0_i64;
                raw.iter()
                    .map(|&(glyph, cluster, x_adv, _, x_off, _)| {
                        let placed = super::PlacedGlyph {
                            glyph,
                            cluster,
                            x: pen + i64::from(x_off),
                        };
                        pen += i64::from(x_adv);
                        placed
                    })
                    .collect()
            };
            let expected_advance: i64 = raw.iter().map(|r| i64::from(r.2)).sum();
            assert_eq!(actual.glyphs, expected_glyphs, "text={text:?}");
            assert_eq!(actual.advance, expected_advance, "text={text:?}");
        }
    }
}
