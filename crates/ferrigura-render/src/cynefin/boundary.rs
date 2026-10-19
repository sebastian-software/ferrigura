//! The boundary waves of the Cynefin diagram: `cynefinBoundaries.ts` of the reference. The seeded
//! generator, the hash of the id, the seed that a render uses, and the paths of the fold, the
//! horizontal boundary, the cliff and the confusion ellipse.
//!
//! The arithmetic is JavaScript's: `|0` is [`to_int32`], `Math.imul` is `wrapping_mul`, and the
//! numbers are written with [`js_number`], so a path has the digits of the reference. The reference
//! truncates a fractional seed (`| 0`), so seeds 1 and 1.5 draw the same waves (probed).

use crate::common::d3::js_number;

/// `0x6d2b79f5`, the constant the generator adds to its seed (`cynefinBoundaries.ts`, line 6).
const MULBERRY_CONSTANT: f64 = 1_831_565_813.0;

/// 2^32, the modulus of `ToInt32`.
const TWO_POW_32: f64 = 4_294_967_296.0;

/// The segments of the fold and of the horizontal boundary (`segments = 7`, lines 60 and 102).
const SEGMENTS: usize = 7;

/// `ToInt32`: the number truncated toward zero and taken modulo 2^32, as a signed 32-bit integer.
/// `NaN` and the infinities give 0. This is `|0` of the reference.
#[must_use]
pub fn to_int32(value: f64) -> i32 {
    if !value.is_finite() {
        return 0;
    }
    // `rem_euclid` gives a whole number in [0, 2^32), which `u32` holds exactly.
    let modulo = value.trunc().rem_euclid(TWO_POW_32);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let bits = modulo as u32;
    bits.cast_signed()
}

/// `t >>> n` as the reference's int32 operand: the unsigned shift, taken back as an int32 (the
/// results of the shifts used here are below 2^31, so the cast keeps the value).
fn shift_right(t: i32, n: u32) -> i32 {
    (t.cast_unsigned() >> n).cast_signed()
}

/// `seededRandom(seed)` (`cynefinBoundaries.ts`, lines 5-10): a value in [0, 1) from mulberry32, the
/// generator of the reference.
#[must_use]
pub fn seeded_random(seed: f64) -> f64 {
    let mut t = to_int32(seed + MULBERRY_CONSTANT);
    t = (t ^ shift_right(t, 15)).wrapping_mul(t | 1);
    t ^= t.wrapping_add((t ^ shift_right(t, 7)).wrapping_mul(t | 61));
    f64::from((t ^ shift_right(t, 14)).cast_unsigned()) / TWO_POW_32
}

/// `hashString(str)` (lines 15-23): the hash of the UTF-16 units of the text, as a 32-bit integer.
#[must_use]
pub fn hash_string(text: &str) -> i32 {
    text.encode_utf16().fold(0_i32, |hash, unit| {
        hash.wrapping_shl(5)
            .wrapping_sub(hash)
            .wrapping_add(i32::from(unit))
    })
}

/// `resolveSeed(configured, id)` (lines 32-41): a finite, non-zero configured seed is used as it is,
/// and anything else gives the hash of the SVG id. The configured value is not truncated here, so a
/// negative seed works.
#[must_use]
pub fn resolve_seed(configured: f64, id: &str) -> f64 {
    if configured.is_finite() && configured != 0.0 {
        configured
    } else {
        f64::from(hash_string(id))
    }
}

/// `generateFoldPath(width, height, seed, amplitude)` (lines 52-84): the vertical wave through the
/// centre. Eight points down the height, each jittered by the generator, joined by seven cubic
/// segments whose control points are offset horizontally.
#[must_use]
pub fn fold_path(width: f64, height: f64, seed: f64, amplitude: f64) -> String {
    let cx = width / 2.0;
    let segment_height = height / SEGMENTS as f64;
    let points: Vec<(f64, f64)> = (0..=SEGMENTS)
        .map(|index| {
            let i = index as f64;
            let jitter = seeded_random(seed + i * 17.0) * amplitude * 2.0 - amplitude;
            (cx + jitter, i * segment_height)
        })
        .collect();

    let mut d = format!("M{},{}", js_number(points[0].0), js_number(points[0].1));
    for (i, pair) in points.windows(2).enumerate() {
        let (p0, p1) = (pair[0], pair[1]);
        let mid_y = (p0.1 + p1.1) / 2.0;
        let direction = if i % 2 == 0 { 1.0 } else { -1.0 };
        let offset = amplitude * 1.5 * direction * seeded_random(seed + i as f64 * 31.0 + 7.0);
        let cp1x = p0.0 + offset;
        let cp2x = p1.0 - offset;
        d.push_str(&format!(
            " C{},{} {},{} {},{}",
            js_number(cp1x),
            js_number(mid_y),
            js_number(cp2x),
            js_number(mid_y),
            js_number(p1.0),
            js_number(p1.1)
        ));
    }
    d
}

/// `generateHorizontalBoundary(width, height, seed, amplitude)` (lines 94-126): the wave through the
/// middle of the width, the same arithmetic as the fold with the axes exchanged. The renderer passes
/// `seed + 100` (renderer line 146).
#[must_use]
pub fn horizontal_path(width: f64, height: f64, seed: f64, amplitude: f64) -> String {
    let cy = height / 2.0;
    let segment_width = width / SEGMENTS as f64;
    let points: Vec<(f64, f64)> = (0..=SEGMENTS)
        .map(|index| {
            let i = index as f64;
            let jitter = seeded_random(seed + i * 23.0) * amplitude * 2.0 - amplitude;
            (i * segment_width, cy + jitter)
        })
        .collect();

    let mut d = format!("M{},{}", js_number(points[0].0), js_number(points[0].1));
    for (i, pair) in points.windows(2).enumerate() {
        let (p0, p1) = (pair[0], pair[1]);
        let mid_x = (p0.0 + p1.0) / 2.0;
        let direction = if i % 2 == 0 { 1.0 } else { -1.0 };
        let offset = amplitude * 1.5 * direction * seeded_random(seed + i as f64 * 37.0 + 11.0);
        let cp1y = p0.1 + offset;
        let cp2y = p1.1 - offset;
        d.push_str(&format!(
            " C{},{} {},{} {},{}",
            js_number(mid_x),
            js_number(cp1y),
            js_number(mid_x),
            js_number(cp2y),
            js_number(p1.0),
            js_number(p1.1)
        ));
    }
    d
}

/// `generateCliffPath(width, height)` (lines 135-151): the steep S-curve from the centre to the
/// bottom edge, with a fixed amplitude of 3% of the width and no seed.
#[must_use]
pub fn cliff_path(width: f64, height: f64) -> String {
    let cx = width / 2.0;
    let top_y = height * 0.5;
    let bottom_y = height;
    let amplitude = width * 0.03;
    let span = bottom_y - top_y;
    let parts = [
        format!("M{},{}", js_number(cx), js_number(top_y)),
        format!(
            "C{},{}",
            js_number(cx + amplitude),
            js_number(top_y + span * 0.2)
        ),
        format!(
            "{},{}",
            js_number(cx - amplitude * 1.5),
            js_number(top_y + span * 0.55)
        ),
        format!(
            "{},{}",
            js_number(cx + amplitude * 0.5),
            js_number(top_y + span * 0.75)
        ),
        format!(
            "C{},{}",
            js_number(cx - amplitude),
            js_number(top_y + span * 0.85)
        ),
        format!(
            "{},{}",
            js_number(cx + amplitude * 0.3),
            js_number(top_y + span * 0.95)
        ),
        format!("{},{}", js_number(cx), js_number(bottom_y)),
    ];
    parts.join(" ")
}

/// `generateConfusionPath(cx, cy, rx, ry)` (lines 161-169): the ellipse of the confusion domain, as
/// two arcs from its left point back to it.
#[must_use]
pub fn confusion_path(cx: f64, cy: f64, rx: f64, ry: f64) -> String {
    let radius_x = js_number(rx);
    let radius_y = js_number(ry);
    let centre_y = js_number(cy);
    let left = js_number(cx - rx);
    let right = js_number(cx + rx);
    format!(
        "M{left},{centre_y} A{radius_x},{radius_y} 0 1,1 {right},{centre_y} A{radius_x},{radius_y} 0 1,1 {left},{centre_y} Z"
    )
}
