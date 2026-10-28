//! The path data as Blink parses it into its byte stream (`SVGPathStringSource::ParseSegment`,
//! `core/svg/svg_path_string_source.cc`, and `SVGPathByteStreamBuilder`, which stores the values
//! verbatim): each segment with its command, its relative flag, and the numbers as floats. The
//! parse stops at the first error and keeps the segments before it (`svg_path_parser::ParsePath`).

use crate::number::{parse_arc_flag, parse_number, skip_spaces};

/// A point as `gfx::PointF` holds it: two floats.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Point {
    pub(crate) x: f32,
    pub(crate) y: f32,
}

/// The kinds of segment (`SVGPathSegType`), without the relative flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Close,
    Move,
    Line,
    Horizontal,
    Vertical,
    Cubic,
    SmoothCubic,
    Quad,
    SmoothQuad,
    Arc,
}

/// One segment of the path data, as its numbers are stored: `p1` and `p2` are the control points,
/// `target` the end. A horizontal or vertical segment keeps its number in `target.x` or `target.y`.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Segment {
    pub(crate) kind: Option<Kind>,
    pub(crate) rel: bool,
    pub(crate) p1: Point,
    pub(crate) p2: Point,
    pub(crate) target: Point,
    pub(crate) rx: f32,
    pub(crate) ry: f32,
    pub(crate) angle: f32,
    pub(crate) large: bool,
    pub(crate) sweep: bool,
}

/// `MapLetterToSegmentType`: the kind and the relative flag of a letter, or `None` for any other
/// character.
fn letter(c: u8) -> Option<(Kind, bool)> {
    Some(match c {
        b'M' => (Kind::Move, false),
        b'm' => (Kind::Move, true),
        b'L' => (Kind::Line, false),
        b'l' => (Kind::Line, true),
        b'H' => (Kind::Horizontal, false),
        b'h' => (Kind::Horizontal, true),
        b'V' => (Kind::Vertical, false),
        b'v' => (Kind::Vertical, true),
        b'C' => (Kind::Cubic, false),
        b'c' => (Kind::Cubic, true),
        b'S' => (Kind::SmoothCubic, false),
        b's' => (Kind::SmoothCubic, true),
        b'Q' => (Kind::Quad, false),
        b'q' => (Kind::Quad, true),
        b'T' => (Kind::SmoothQuad, false),
        b't' => (Kind::SmoothQuad, true),
        b'A' => (Kind::Arc, false),
        b'a' => (Kind::Arc, true),
        b'Z' | b'z' => (Kind::Close, false),
        _ => return None,
    })
}

/// `IsNumberStart`: the characters that can start a number, which repeat the previous command.
fn is_number_start(c: u8) -> bool {
    c.is_ascii_digit() || c == b'+' || c == b'-' || c == b'.'
}

/// The numbers of a segment of `kind`, in the order `ParseSegment` reads them. `None` where it fails.
fn read_numbers(kind: Kind, s: &[u8], pos: &mut usize) -> Option<Segment> {
    let mut segment = Segment::default();
    let point = |pos: &mut usize| -> Option<Point> {
        Some(Point {
            x: parse_number(s, pos)?,
            y: parse_number(s, pos)?,
        })
    };
    match kind {
        // `EatWhitespace` after a close: spaces only.
        Kind::Close => {
            skip_spaces(s, pos);
        }
        Kind::Move | Kind::Line => segment.target = point(pos)?,
        Kind::Horizontal => segment.target.x = parse_number(s, pos)?,
        Kind::Vertical => segment.target.y = parse_number(s, pos)?,
        Kind::Cubic => {
            segment.p1 = point(pos)?;
            segment.p2 = point(pos)?;
            segment.target = point(pos)?;
        }
        Kind::SmoothCubic => {
            segment.p2 = point(pos)?;
            segment.target = point(pos)?;
        }
        Kind::Quad => {
            segment.p1 = point(pos)?;
            segment.target = point(pos)?;
        }
        Kind::SmoothQuad => segment.target = point(pos)?,
        Kind::Arc => {
            segment.rx = parse_number(s, pos)?;
            segment.ry = parse_number(s, pos)?;
            segment.angle = parse_number(s, pos)?;
            segment.large = parse_arc_flag(s, pos)?;
            segment.sweep = parse_arc_flag(s, pos)?;
            segment.target = point(pos)?;
        }
    }
    Some(segment)
}

/// The segments of the path data `d`, as Blink's byte stream holds them.
pub(crate) fn parse(d: &str) -> Vec<Segment> {
    let s = d.as_bytes();
    let mut pos = 0;
    skip_spaces(s, &mut pos);
    let mut out = Vec::new();
    // The kind of the previous command, which a number repeats (`previous_command_`).
    let mut previous: Option<(Kind, bool)> = None;
    while pos < s.len() {
        let lookahead = s[pos];
        let command = match letter(lookahead) {
            Some((kind, rel)) => {
                // The first command is a move (`kExpectedMoveToCommand`).
                if previous.is_none() && kind != Kind::Move {
                    break;
                }
                pos += 1;
                (kind, rel)
            }
            None => {
                // A number repeats the previous command, and a move repeats as a line
                // (`MaybeImplicitCommand`); nothing repeats a close.
                let Some((kind, rel)) = previous else { break };
                if !is_number_start(lookahead) || kind == Kind::Close {
                    break;
                }
                match kind {
                    Kind::Move => (Kind::Line, rel),
                    _ => (kind, rel),
                }
            }
        };
        previous = Some(command);
        let Some(mut segment) = read_numbers(command.0, s, &mut pos) else {
            break;
        };
        segment.kind = Some(command.0);
        segment.rel = command.1;
        out.push(segment);
    }
    out
}
