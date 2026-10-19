//! The functions of khroma 2.1.0 that the redux themes use, ported from the sources in its `src/`
//! (`color/`, `channels/`, `methods/` and `utils/`).
//!
//! khroma keeps one shared `Channels` object that every parse overwrites, and its methods read and
//! write channels lazily: a colour parsed as RGB gets its HSL channels on the first read, and the
//! reverse. This port keeps those lazy reads and writes, because they decide which floating point
//! operations run and in which order. Numbers are rounded as `Lang.round` rounds them, and printed
//! as JavaScript prints them, so the strings are the reference's strings byte for byte.
//!
//! Ported: parsing of hex, `rgb()`/`rgba()`, `hsl()`/`hsla()` and the keywords; `adjust`, `change`,
//! `darken`, `lighten`, `invert`, `mix`, `luminance`, `isLight`, `isDark` and the numeric form of
//! `rgba`. Not ported, because the themes do not reach them: the string form of `rgba`, `hsla`,
//! the `to*` conversions, and the other `methods/` of khroma.

use std::f64::consts::PI;
use std::sync::LazyLock;

use regex::Regex;

use super::ThemeError;

/// A channel of a colour, named as khroma names them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// Red, from 0 to 255.
    R,
    /// Green, from 0 to 255.
    G,
    /// Blue, from 0 to 255.
    B,
    /// Hue in degrees. It is reduced with `%`, so it can be negative.
    H,
    /// Saturation in percent, from 0 to 100.
    S,
    /// Lightness in percent, from 0 to 100.
    L,
    /// Alpha, from 0 to 1.
    A,
}

/// The channels a colour holds: RGB, HSL, or both computed from one another. khroma calls it `Type`
/// and a colour cannot change from one to the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// Neither was set by a setter yet: the colour is as it was parsed.
    All,
    /// RGB channels were set.
    Rgb,
    /// HSL channels were set.
    Hsl,
}

/// The stored channel values. A channel that khroma has not stored is `None`, which is `undefined`
/// in the reference.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Data {
    r: Option<f64>,
    g: Option<f64>,
    b: Option<f64>,
    h: Option<f64>,
    s: Option<f64>,
    l: Option<f64>,
    a: f64,
}

/// A parsed colour: its channels, the text it was parsed from, and whether a setter changed it.
#[derive(Debug, Clone)]
struct Channels {
    color: Option<String>,
    changed: bool,
    data: Data,
    kind: Kind,
}

impl Channels {
    /// A colour as khroma's `ChannelsReusable.set` makes it: unchanged, with the given text.
    fn new(data: Data, color: Option<String>) -> Self {
        Self {
            color,
            changed: false,
            data,
            kind: Kind::All,
        }
    }

    /// Stores the HSL channels that are missing, computed from the RGB ones (`_ensureHSL`).
    fn ensure_hsl(&mut self) {
        if self.data.h.is_none() {
            self.data.h = Some(rgb2hsl(&self.data, Channel::H));
        }
        if self.data.s.is_none() {
            self.data.s = Some(rgb2hsl(&self.data, Channel::S));
        }
        if self.data.l.is_none() {
            self.data.l = Some(rgb2hsl(&self.data, Channel::L));
        }
    }

    /// Stores the RGB channels that are missing, computed from the HSL ones (`_ensureRGB`).
    fn ensure_rgb(&mut self) {
        if self.data.r.is_none() {
            self.data.r = Some(hsl2rgb(&self.data, Channel::R));
        }
        if self.data.g.is_none() {
            self.data.g = Some(hsl2rgb(&self.data, Channel::G));
        }
        if self.data.b.is_none() {
            self.data.b = Some(hsl2rgb(&self.data, Channel::B));
        }
    }

    /// Reads an RGB channel: the stored value unless the colour is HSL-typed (the `get r()` of
    /// khroma and its siblings).
    fn read_rgb(&mut self, channel: Channel) -> f64 {
        let stored = match channel {
            Channel::R => self.data.r,
            Channel::G => self.data.g,
            _ => self.data.b,
        };
        if self.kind != Kind::Hsl
            && let Some(value) = stored
        {
            return value;
        }
        self.ensure_hsl();
        hsl2rgb(&self.data, channel)
    }

    /// Reads an HSL channel: the stored value unless the colour is RGB-typed.
    fn read_hsl(&mut self, channel: Channel) -> f64 {
        let stored = match channel {
            Channel::H => self.data.h,
            Channel::S => self.data.s,
            _ => self.data.l,
        };
        if self.kind != Kind::Rgb
            && let Some(value) = stored
        {
            return value;
        }
        self.ensure_rgb();
        rgb2hsl(&self.data, channel)
    }

    /// Reads a channel by its name.
    fn get(&mut self, channel: Channel) -> f64 {
        match channel {
            Channel::R | Channel::G | Channel::B => self.read_rgb(channel),
            Channel::H | Channel::S | Channel::L => self.read_hsl(channel),
            Channel::A => self.data.a,
        }
    }

    /// Makes the colour RGB or HSL typed, as khroma's `Type.set` does. A colour of the other type
    /// is an error, which the reference throws.
    fn set_kind(&mut self, kind: Kind) -> Result<(), ThemeError> {
        if self.kind != Kind::All && self.kind != kind {
            return Err(ThemeError::new(
                "Cannot change both RGB and HSL channels at the same time",
            ));
        }
        self.kind = kind;
        Ok(())
    }

    /// Writes a channel by its name and marks the colour changed, as khroma's setters do.
    fn set(&mut self, channel: Channel, value: f64) -> Result<(), ThemeError> {
        match channel {
            Channel::R | Channel::G | Channel::B => {
                self.set_kind(Kind::Rgb)?;
                self.changed = true;
                match channel {
                    Channel::R => self.data.r = Some(value),
                    Channel::G => self.data.g = Some(value),
                    _ => self.data.b = Some(value),
                }
            }
            Channel::H | Channel::S | Channel::L => {
                self.set_kind(Kind::Hsl)?;
                self.changed = true;
                match channel {
                    Channel::H => self.data.h = Some(value),
                    Channel::S => self.data.s = Some(value),
                    _ => self.data.l = Some(value),
                }
            }
            Channel::A => {
                self.changed = true;
                self.data.a = value;
            }
        }
        Ok(())
    }
}

/// Clamps a value of the given channel to its range, as khroma's `utils/channel.ts` does. A NaN
/// stays NaN, as it does there.
fn clamp(channel: Channel, value: f64) -> f64 {
    // `f64::clamp` keeps a NaN as NaN, and a value of -0 as -0, as the comparisons of khroma do.
    match channel {
        Channel::R | Channel::G | Channel::B => value.clamp(0.0, 255.0),
        Channel::H => value % 360.0,
        Channel::S | Channel::L => value.clamp(0.0, 100.0),
        Channel::A => value.clamp(0.0, 1.0),
    }
}

/// `Math.max` of three numbers: NaN if any of them is NaN.
fn js_max3(a: f64, b: f64, c: f64) -> f64 {
    if a.is_nan() || b.is_nan() || c.is_nan() {
        f64::NAN
    } else {
        a.max(b).max(c)
    }
}

/// `Math.min` of three numbers: NaN if any of them is NaN.
fn js_min3(a: f64, b: f64, c: f64) -> f64 {
    if a.is_nan() || b.is_nan() || c.is_nan() {
        f64::NAN
    } else {
        a.min(b).min(c)
    }
}

/// Whether a stored number is truthy in JavaScript: not undefined, zero or NaN.
fn truthy(value: Option<f64>) -> bool {
    value.is_some_and(|number| number != 0.0 && !number.is_nan())
}

/// `rgb2hsl` of `utils/channel.ts`: one HSL channel of the RGB channels of the data.
fn rgb2hsl(data: &Data, channel: Channel) -> f64 {
    let r = data.r.unwrap_or(f64::NAN) / 255.0;
    let g = data.g.unwrap_or(f64::NAN) / 255.0;
    let b = data.b.unwrap_or(f64::NAN) / 255.0;

    let max = js_max3(r, g, b);
    let min = js_min3(r, g, b);
    let l = (max + min) / 2.0;

    if channel == Channel::L {
        return l * 100.0;
    }

    if max == min {
        return 0.0;
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    if channel == Channel::S {
        return s * 100.0;
    }

    if max == r {
        ((g - b) / d + if g < b { 6.0 } else { 0.0 }) * 60.0
    } else if max == g {
        ((b - r) / d + 2.0) * 60.0
    } else if max == b {
        ((r - g) / d + 4.0) * 60.0
    } else {
        -1.0
    }
}

/// `hue2rgb` of `utils/channel.ts`.
fn hue2rgb(p: f64, q: f64, t: f64) -> f64 {
    let mut t = t;
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }

    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

/// `hsl2rgb` of `utils/channel.ts`: one RGB channel of the HSL channels of the data.
fn hsl2rgb(data: &Data, channel: Channel) -> f64 {
    let l_value = data.l.unwrap_or(f64::NAN);
    if !truthy(data.s) {
        return l_value * 2.55;
    }

    let h = data.h.unwrap_or(f64::NAN) / 360.0;
    let s = data.s.unwrap_or(f64::NAN) / 100.0;
    let l = l_value / 100.0;

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        (l + s) - (l * s)
    };
    let p = 2.0 * l - q;

    match channel {
        Channel::R => hue2rgb(p, q, h + 1.0 / 3.0) * 255.0,
        Channel::G => hue2rgb(p, q, h) * 255.0,
        _ => hue2rgb(p, q, h - 1.0 / 3.0) * 255.0,
    }
}

/// `Lang.round` of `utils/lang.ts`: `Math.round` to ten decimals.
fn lang_round(number: f64) -> f64 {
    js_round(number * 10_000_000_000.0) / 10_000_000_000.0
}

/// `Math.round`: halves go towards +infinity, unlike Rust's `f64::round`.
fn js_round(number: f64) -> f64 {
    let floor = number.floor();
    if number - floor >= 0.5 {
        floor + 1.0
    } else {
        floor
    }
}

/// `Number.prototype.toString` of JavaScript, which writes the channels of a colour.
pub use crate::value::number_text as js_number;

/// `hex` of `color/hex.ts`: `#rgb`, `#rgba`, `#rrggbb` and `#rrggbbaa`.
fn parse_hex(color: &str) -> Option<Channels> {
    let hex = color.strip_prefix('#')?;
    let length = hex.len();
    if !matches!(length, 3 | 4 | 6 | 8) || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let dec = u32::from_str_radix(hex, 16).ok()?;

    let has_alpha = length % 4 == 0;
    let is_full_length = length > 4;
    let multiplier: f64 = if is_full_length { 1.0 } else { 17.0 };
    let bits: i64 = if is_full_length { 8 } else { 4 };
    let bits_offset: i64 = if has_alpha { 0 } else { -1 };
    let mask: u32 = if is_full_length { 255 } else { 15 };
    let channel = |index: i64| -> f64 {
        let shift = bits * (bits_offset + index);
        let shift = u32::try_from(shift).unwrap_or(0);
        f64::from((dec >> shift) & mask) * multiplier
    };

    let data = Data {
        r: Some(channel(3)),
        g: Some(channel(2)),
        b: Some(channel(1)),
        a: if has_alpha {
            f64::from(dec & mask) * multiplier / 255.0
        } else {
            1.0
        },
        ..Data::default()
    };
    Some(Channels::new(data, Some(color.to_owned())))
}

/// The CSS keywords of khroma's `color/keyword.ts`, with their hex values.
const KEYWORDS: &[(&str, &str)] = &[
    ("aliceblue", "#f0f8ff"),
    ("antiquewhite", "#faebd7"),
    ("aqua", "#00ffff"),
    ("aquamarine", "#7fffd4"),
    ("azure", "#f0ffff"),
    ("beige", "#f5f5dc"),
    ("bisque", "#ffe4c4"),
    ("black", "#000000"),
    ("blanchedalmond", "#ffebcd"),
    ("blue", "#0000ff"),
    ("blueviolet", "#8a2be2"),
    ("brown", "#a52a2a"),
    ("burlywood", "#deb887"),
    ("cadetblue", "#5f9ea0"),
    ("chartreuse", "#7fff00"),
    ("chocolate", "#d2691e"),
    ("coral", "#ff7f50"),
    ("cornflowerblue", "#6495ed"),
    ("cornsilk", "#fff8dc"),
    ("crimson", "#dc143c"),
    ("cyanaqua", "#00ffff"),
    ("darkblue", "#00008b"),
    ("darkcyan", "#008b8b"),
    ("darkgoldenrod", "#b8860b"),
    ("darkgray", "#a9a9a9"),
    ("darkgreen", "#006400"),
    ("darkgrey", "#a9a9a9"),
    ("darkkhaki", "#bdb76b"),
    ("darkmagenta", "#8b008b"),
    ("darkolivegreen", "#556b2f"),
    ("darkorange", "#ff8c00"),
    ("darkorchid", "#9932cc"),
    ("darkred", "#8b0000"),
    ("darksalmon", "#e9967a"),
    ("darkseagreen", "#8fbc8f"),
    ("darkslateblue", "#483d8b"),
    ("darkslategray", "#2f4f4f"),
    ("darkslategrey", "#2f4f4f"),
    ("darkturquoise", "#00ced1"),
    ("darkviolet", "#9400d3"),
    ("deeppink", "#ff1493"),
    ("deepskyblue", "#00bfff"),
    ("dimgray", "#696969"),
    ("dimgrey", "#696969"),
    ("dodgerblue", "#1e90ff"),
    ("firebrick", "#b22222"),
    ("floralwhite", "#fffaf0"),
    ("forestgreen", "#228b22"),
    ("fuchsia", "#ff00ff"),
    ("gainsboro", "#dcdcdc"),
    ("ghostwhite", "#f8f8ff"),
    ("gold", "#ffd700"),
    ("goldenrod", "#daa520"),
    ("gray", "#808080"),
    ("green", "#008000"),
    ("greenyellow", "#adff2f"),
    ("grey", "#808080"),
    ("honeydew", "#f0fff0"),
    ("hotpink", "#ff69b4"),
    ("indianred", "#cd5c5c"),
    ("indigo", "#4b0082"),
    ("ivory", "#fffff0"),
    ("khaki", "#f0e68c"),
    ("lavender", "#e6e6fa"),
    ("lavenderblush", "#fff0f5"),
    ("lawngreen", "#7cfc00"),
    ("lemonchiffon", "#fffacd"),
    ("lightblue", "#add8e6"),
    ("lightcoral", "#f08080"),
    ("lightcyan", "#e0ffff"),
    ("lightgoldenrodyellow", "#fafad2"),
    ("lightgray", "#d3d3d3"),
    ("lightgreen", "#90ee90"),
    ("lightgrey", "#d3d3d3"),
    ("lightpink", "#ffb6c1"),
    ("lightsalmon", "#ffa07a"),
    ("lightseagreen", "#20b2aa"),
    ("lightskyblue", "#87cefa"),
    ("lightslategray", "#778899"),
    ("lightslategrey", "#778899"),
    ("lightsteelblue", "#b0c4de"),
    ("lightyellow", "#ffffe0"),
    ("lime", "#00ff00"),
    ("limegreen", "#32cd32"),
    ("linen", "#faf0e6"),
    ("magenta", "#ff00ff"),
    ("maroon", "#800000"),
    ("mediumaquamarine", "#66cdaa"),
    ("mediumblue", "#0000cd"),
    ("mediumorchid", "#ba55d3"),
    ("mediumpurple", "#9370db"),
    ("mediumseagreen", "#3cb371"),
    ("mediumslateblue", "#7b68ee"),
    ("mediumspringgreen", "#00fa9a"),
    ("mediumturquoise", "#48d1cc"),
    ("mediumvioletred", "#c71585"),
    ("midnightblue", "#191970"),
    ("mintcream", "#f5fffa"),
    ("mistyrose", "#ffe4e1"),
    ("moccasin", "#ffe4b5"),
    ("navajowhite", "#ffdead"),
    ("navy", "#000080"),
    ("oldlace", "#fdf5e6"),
    ("olive", "#808000"),
    ("olivedrab", "#6b8e23"),
    ("orange", "#ffa500"),
    ("orangered", "#ff4500"),
    ("orchid", "#da70d6"),
    ("palegoldenrod", "#eee8aa"),
    ("palegreen", "#98fb98"),
    ("paleturquoise", "#afeeee"),
    ("palevioletred", "#db7093"),
    ("papayawhip", "#ffefd5"),
    ("peachpuff", "#ffdab9"),
    ("peru", "#cd853f"),
    ("pink", "#ffc0cb"),
    ("plum", "#dda0dd"),
    ("powderblue", "#b0e0e6"),
    ("purple", "#800080"),
    ("rebeccapurple", "#663399"),
    ("red", "#ff0000"),
    ("rosybrown", "#bc8f8f"),
    ("royalblue", "#4169e1"),
    ("saddlebrown", "#8b4513"),
    ("salmon", "#fa8072"),
    ("sandybrown", "#f4a460"),
    ("seagreen", "#2e8b57"),
    ("seashell", "#fff5ee"),
    ("sienna", "#a0522d"),
    ("silver", "#c0c0c0"),
    ("skyblue", "#87ceeb"),
    ("slateblue", "#6a5acd"),
    ("slategray", "#708090"),
    ("slategrey", "#708090"),
    ("snow", "#fffafa"),
    ("springgreen", "#00ff7f"),
    ("tan", "#d2b48c"),
    ("teal", "#008080"),
    ("thistle", "#d8bfd8"),
    ("transparent", "#00000000"),
    ("turquoise", "#40e0d0"),
    ("violet", "#ee82ee"),
    ("wheat", "#f5deb3"),
    ("white", "#ffffff"),
    ("whitesmoke", "#f5f5f5"),
    ("yellow", "#ffff00"),
    ("yellowgreen", "#9acd32"),
];

/// `keyword` of `color/keyword.ts`: the lowercased name, read as its hex value.
fn parse_keyword(color: &str) -> Option<Channels> {
    let lower = color.to_lowercase();
    let (_, hex) = KEYWORDS.iter().find(|(name, _)| *name == lower)?;
    parse_hex(hex)
}

/// Builds a pattern that matches `word` in any letter case, so that the patterns do not depend on
/// the case folding of `regex`, which folds more than JavaScript's `/i` does.
fn ci(word: &str) -> String {
    word.chars()
        .map(|character| {
            if character.is_ascii_alphabetic() {
                let lower = character.to_ascii_lowercase();
                let upper = character.to_ascii_uppercase();
                format!("[{lower}{upper}]")
            } else {
                character.to_string()
            }
        })
        .collect()
}

/// The whitespace of JavaScript's `\s`, which `regex`'s `\s` does not match exactly.
const WS: &str = r"[\t\n\x0B\x0C\r \x{A0}\x{1680}\x{2000}-\x{200A}\x{2028}\x{2029}\x{202F}\x{205F}\x{3000}\x{FEFF}]";

/// A decimal number with an optional sign, as the reference's colour expressions read it.
const NUMBER: &str = r"-?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)";

/// The pattern of `color/rgb.ts`, with the same groups: red, its percent sign, green, its percent
/// sign, blue, its percent sign, and optionally alpha with its percent sign.
fn rgb_pattern() -> String {
    let separator = format!("{WS}*?(?:,|{WS}){WS}*?");
    let exponent = format!("{NUMBER}(?:[eE][0-9]+)?");
    let channel = format!("({exponent}(%?))");
    let alpha = format!("(?:{WS}*?(?:,|/){WS}*?\\+?{channel})?");
    let open = format!("^{}[aA]?\\({WS}*?", ci("rgb"));
    format!("{open}{channel}{separator}{channel}{separator}{channel}{alpha}{WS}*?\\)$")
}

/// The pattern of `color/hsl.ts`, with the same groups: hue, saturation, lightness, and optionally
/// alpha with its percent sign.
fn hsl_pattern() -> String {
    let separator = format!("{WS}*?(?:,|{WS}){WS}*?");
    let hue_unit = format!(
        "(?:{}|{}|{}|{})",
        ci("deg"),
        ci("grad"),
        ci("rad"),
        ci("turn")
    );
    let hue = format!("(-?(?:[0-9]+(?:\\.[0-9]+)?|\\.[0-9]+)(?:[eE]-?[0-9]+)?{hue_unit}?)");
    let percent = "(-?(?:[0-9]+(?:\\.[0-9]+)?|\\.[0-9]+)(?:[eE]-?[0-9]+)?%)".to_owned();
    let alpha_number = "(-?(?:[0-9]+(?:\\.[0-9]+)?|\\.[0-9]+)(?:[eE]-?[0-9]+)?(%)?)".to_owned();
    let alpha = format!("(?:{WS}*?(?:,|/){WS}*?\\+?{alpha_number})?");
    let open = format!("^{}[aA]?\\({WS}*?", ci("hsl"));
    format!("{open}{hue}{separator}{percent}{separator}{percent}{alpha}{WS}*?\\)$")
}

/// The RGB pattern, compiled once. The pattern is a constant, so the `None` case is unreachable.
static RGB_RE: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(&rgb_pattern()).ok());

/// The HSL pattern, compiled once. The pattern is a constant, so the `None` case is unreachable.
static HSL_RE: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(&hsl_pattern()).ok());

/// `parseFloat` of JavaScript: the longest decimal number at the start of the text, or NaN.
fn parse_float(text: &str) -> f64 {
    let bytes = text.as_bytes();
    let length = bytes.len();
    let mut index = 0;
    let negative = bytes.first() == Some(&b'-');
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        index += 1;
    }
    if text
        .get(index..)
        .is_some_and(|rest| rest.starts_with("Infinity"))
    {
        return if negative {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }

    let integer_start = index;
    while index < length && bytes[index].is_ascii_digit() {
        index += 1;
    }
    let integer_digits = index - integer_start;

    // A dot counts when a digit precedes it, as in `5.` or `5.e3`, but not in `.` alone.
    let mut fraction_digits = 0;
    if index < length && bytes[index] == b'.' {
        let mut end = index + 1;
        while end < length && bytes[end].is_ascii_digit() {
            end += 1;
        }
        fraction_digits = end - index - 1;
        if integer_digits > 0 || fraction_digits > 0 {
            index = end;
        }
    }
    if integer_digits == 0 && fraction_digits == 0 {
        return f64::NAN;
    }

    if index < length && matches!(bytes[index], b'e' | b'E') {
        let mut end = index + 1;
        if end < length && matches!(bytes[end], b'+' | b'-') {
            end += 1;
        }
        let exponent_start = end;
        while end < length && bytes[end].is_ascii_digit() {
            end += 1;
        }
        if end > exponent_start {
            index = end;
        }
    }

    text.get(..index)
        .and_then(|number| number.parse::<f64>().ok())
        .unwrap_or(f64::NAN)
}

/// Splits a hue into its number and its unit, as `hueRe` of `color/hsl.ts` does: the unit is one of
/// `grad`, `rad`, `turn` or `deg`, matched without regard to case, and the number is what is left.
fn split_hue_unit(hue: &str) -> (&str, Option<&str>) {
    let lower = hue.to_ascii_lowercase();
    for unit in ["grad", "turn", "deg", "rad"] {
        let length = unit.len();
        if hue.len() > length && lower.ends_with(unit) {
            let cut = hue.len() - length;
            if let (Some(number), Some(suffix)) = (hue.get(..cut), hue.get(cut..)) {
                return (number, Some(suffix));
            }
        }
    }
    (hue, None)
}

/// `_hue2deg` of `color/hsl.ts`. The unit is compared as written, so `GRAD` keeps its value.
fn hue2deg(hue: &str) -> f64 {
    let (number, unit) = split_hue_unit(hue);
    match unit {
        Some("grad") => clamp(Channel::H, parse_float(number) * 0.9),
        Some("rad") => clamp(Channel::H, parse_float(number) * 180.0 / PI),
        Some("turn") => clamp(Channel::H, parse_float(number) * 360.0),
        _ => clamp(Channel::H, parse_float(hue)),
    }
}

/// `rgb` of `color/rgb.ts`.
fn parse_rgb(color: &str) -> Option<Channels> {
    if !matches!(color.as_bytes().first(), Some(b'r' | b'R')) {
        return None;
    }
    let captures = RGB_RE.as_ref()?.captures(color)?;
    let part = |index: usize| captures.get(index).map_or("", |found| found.as_str());
    let percent = |index: usize| !part(index).is_empty();

    let channel = |value: usize, sign: usize, to_channel: Channel| {
        let number = parse_float(part(value));
        let scaled = if percent(sign) { number * 2.55 } else { number };
        clamp(to_channel, scaled)
    };
    let data = Data {
        r: Some(channel(1, 2, Channel::R)),
        g: Some(channel(3, 4, Channel::G)),
        b: Some(channel(5, 6, Channel::B)),
        a: if captures.get(7).is_some() {
            let number = parse_float(part(7));
            clamp(Channel::A, if percent(8) { number / 100.0 } else { number })
        } else {
            1.0
        },
        ..Data::default()
    };
    Some(Channels::new(data, Some(color.to_owned())))
}

/// `hsl` of `color/hsl.ts`.
fn parse_hsl(color: &str) -> Option<Channels> {
    if !matches!(color.as_bytes().first(), Some(b'h' | b'H')) {
        return None;
    }
    let captures = HSL_RE.as_ref()?.captures(color)?;
    let part = |index: usize| captures.get(index).map_or("", |found| found.as_str());

    let data = Data {
        h: Some(hue2deg(part(1))),
        s: Some(clamp(Channel::S, parse_float(part(2)))),
        l: Some(clamp(Channel::L, parse_float(part(3)))),
        a: if captures.get(4).is_some() {
            let number = parse_float(part(4));
            clamp(
                Channel::A,
                if captures.get(5).is_some() {
                    number / 100.0
                } else {
                    number
                },
            )
        } else {
            1.0
        },
        ..Data::default()
    };
    Some(Channels::new(data, Some(color.to_owned())))
}

/// `Color.parse` of `color/index.ts`: a string the colour formats read, or an error.
fn parse(color: &str) -> Result<Channels, ThemeError> {
    parse_hex(color)
        .or_else(|| parse_rgb(color))
        .or_else(|| parse_hsl(color))
        .or_else(|| parse_keyword(color))
        .ok_or_else(|| ThemeError::new(format!("Unsupported color format: \"{color}\"")))
}

/// `dec2hex` of `utils/unit.ts` with the `DEC2HEX` table: two hex digits, or `undefined` when the
/// rounded value is not one of the table's keys (NaN, or outside 0 to 255).
fn dec2hex(value: f64) -> String {
    let rounded = js_round(value);
    if (0.0..=255.0).contains(&rounded) {
        format!("{:02x}", rounded as u8)
    } else {
        "undefined".to_owned()
    }
}

/// `Color.stringify` of `color/index.ts`: the text the colour was parsed from when nothing changed,
/// otherwise the hex, `rgb()`/`rgba()` or `hsl()`/`hsla()` form the channels call for.
fn stringify(channels: &mut Channels) -> String {
    if !channels.changed
        && let Some(color) = channels.color.as_deref()
        && !color.is_empty()
    {
        return color.to_owned();
    }

    if channels.kind == Kind::Hsl || channels.data.r.is_none() {
        return hsl_string(channels);
    }

    let needs_rgb = channels.data.a < 1.0
        || !is_integer(channels.get(Channel::R))
        || !is_integer(channels.get(Channel::G))
        || !is_integer(channels.get(Channel::B));
    if needs_rgb {
        rgb_string(channels)
    } else {
        hex_string(channels)
    }
}

/// `Number.isInteger` of JavaScript.
fn is_integer(value: f64) -> bool {
    value.is_finite() && value.trunc() == value
}

/// `Hex.stringify` of `color/hex.ts`.
fn hex_string(channels: &mut Channels) -> String {
    let r = channels.get(Channel::R);
    let g = channels.get(Channel::G);
    let b = channels.get(Channel::B);
    let alpha = channels.get(Channel::A);
    if alpha < 1.0 {
        format!(
            "#{}{}{}{}",
            dec2hex(r),
            dec2hex(g),
            dec2hex(b),
            dec2hex(alpha * 255.0)
        )
    } else {
        format!("#{}{}{}", dec2hex(r), dec2hex(g), dec2hex(b))
    }
}

/// `RGB.stringify` of `color/rgb.ts`.
fn rgb_string(channels: &mut Channels) -> String {
    let r = channels.get(Channel::R);
    let g = channels.get(Channel::G);
    let b = channels.get(Channel::B);
    let alpha = channels.get(Channel::A);
    if alpha < 1.0 {
        format!(
            "rgba({}, {}, {}, {})",
            js_number(lang_round(r)),
            js_number(lang_round(g)),
            js_number(lang_round(b)),
            js_number(lang_round(alpha))
        )
    } else {
        format!(
            "rgb({}, {}, {})",
            js_number(lang_round(r)),
            js_number(lang_round(g)),
            js_number(lang_round(b))
        )
    }
}

/// `HSL.stringify` of `color/hsl.ts`. The alpha of `hsla()` is not rounded, as in the reference.
fn hsl_string(channels: &mut Channels) -> String {
    let h = channels.get(Channel::H);
    let s = channels.get(Channel::S);
    let l = channels.get(Channel::L);
    let alpha = channels.get(Channel::A);
    if alpha < 1.0 {
        format!(
            "hsla({}, {}%, {}%, {})",
            js_number(lang_round(h)),
            js_number(lang_round(s)),
            js_number(lang_round(l)),
            js_number(alpha)
        )
    } else {
        format!(
            "hsl({}, {}%, {}%)",
            js_number(lang_round(h)),
            js_number(lang_round(s)),
            js_number(lang_round(l))
        )
    }
}

/// `adjust` of `methods/adjust.ts`: the colour with each channel moved by its amount. An amount of
/// zero (or NaN) leaves a channel alone; a colour that ends up unchanged keeps its text.
///
/// # Errors
///
/// Returns an error when the colour is not one khroma reads.
pub fn adjust(color: &str, channels: &[(Channel, f64)]) -> Result<String, ThemeError> {
    let mut parsed = parse(color)?;
    let mut changes: Vec<(Channel, f64)> = Vec::new();
    for &(channel, amount) in channels {
        if amount == 0.0 || amount.is_nan() {
            continue;
        }
        let current = parsed.get(channel);
        changes.push((channel, current + amount));
    }
    change(color, &changes)
}

/// `change` of `methods/change.ts`: the colour with each channel set to its value, clamped.
///
/// # Errors
///
/// Returns an error when the colour is not one khroma reads, or the channels mix RGB and HSL.
pub fn change(color: &str, channels: &[(Channel, f64)]) -> Result<String, ThemeError> {
    let mut parsed = parse(color)?;
    for &(channel, value) in channels {
        parsed.set(channel, clamp(channel, value))?;
    }
    Ok(stringify(&mut parsed))
}

/// `adjustChannel` of `methods/adjust_channel.ts`: one channel moved by an amount.
fn adjust_channel(color: &str, channel: Channel, amount: f64) -> Result<String, ThemeError> {
    let mut parsed = parse(color)?;
    let current = parsed.get(channel);
    let next = clamp(channel, current + amount);
    if current != next {
        parsed.set(channel, next)?;
    }
    Ok(stringify(&mut parsed))
}

/// `darken` of `methods/darken.ts`: the lightness lowered by the amount, in percent.
///
/// # Errors
///
/// Returns an error when the colour is not one khroma reads.
pub fn darken(color: &str, amount: f64) -> Result<String, ThemeError> {
    adjust_channel(color, Channel::L, -amount)
}

/// `lighten` of `methods/lighten.ts`: the lightness raised by the amount, in percent.
///
/// # Errors
///
/// Returns an error when the colour is not one khroma reads.
pub fn lighten(color: &str, amount: f64) -> Result<String, ThemeError> {
    adjust_channel(color, Channel::L, amount)
}

/// `invert` of `methods/invert.ts`: each RGB channel inverted, then mixed with the colour itself by
/// `weight` percent. The reference's default weight is 100.
///
/// # Errors
///
/// Returns an error when the colour is not one khroma reads.
pub fn invert(color: &str, weight: f64) -> Result<String, ThemeError> {
    let mut inverse = parse(color)?;
    let r = inverse.get(Channel::R);
    inverse.set(Channel::R, 255.0 - r)?;
    let g = inverse.get(Channel::G);
    inverse.set(Channel::G, 255.0 - g)?;
    let b = inverse.get(Channel::B);
    inverse.set(Channel::B, 255.0 - b)?;
    mix_channels(&mut inverse, color, weight)
}

/// `mix` of `methods/mix.ts`: the two colours mixed, with `weight` percent of the first.
///
/// # Errors
///
/// Returns an error when either colour is not one khroma reads.
pub fn mix(color1: &str, color2: &str, weight: f64) -> Result<String, ThemeError> {
    let mut first = parse(color1)?;
    mix_channels(&mut first, color2, weight)
}

/// The mix of `mix` for a first colour that is already parsed, as `invert` passes one.
fn mix_channels(first: &mut Channels, color2: &str, weight: f64) -> Result<String, ThemeError> {
    let r1 = first.get(Channel::R);
    let g1 = first.get(Channel::G);
    let b1 = first.get(Channel::B);
    let a1 = first.get(Channel::A);

    let mut second = parse(color2)?;
    let r2 = second.get(Channel::R);
    let g2 = second.get(Channel::G);
    let b2 = second.get(Channel::B);
    let a2 = second.get(Channel::A);

    let weight_scale = weight / 100.0;
    let weight_normalized = (weight_scale * 2.0) - 1.0;
    let alpha_delta = a1 - a2;
    let weight1_combined = if weight_normalized * alpha_delta == -1.0 {
        weight_normalized
    } else {
        (weight_normalized + alpha_delta) / (1.0 + weight_normalized * alpha_delta)
    };
    let weight1 = (weight1_combined + 1.0) / 2.0;
    let weight2 = 1.0 - weight1;

    let r = (r1 * weight1) + (r2 * weight2);
    let g = (g1 * weight1) + (g2 * weight2);
    let b = (b1 * weight1) + (b2 * weight2);
    let a = (a1 * weight_scale) + (a2 * (1.0 - weight_scale));
    Ok(rgba(r, g, b, a))
}

/// `rgba(r, g, b, a)` of `methods/rgba.ts` with numbers: the channels clamped, then formatted.
#[must_use]
pub fn rgba(r: f64, g: f64, b: f64, a: f64) -> String {
    let data = Data {
        r: Some(clamp(Channel::R, r)),
        g: Some(clamp(Channel::G, g)),
        b: Some(clamp(Channel::B, b)),
        a: clamp(Channel::A, a),
        ..Data::default()
    };
    let mut channels = Channels::new(data, None);
    stringify(&mut channels)
}

/// `toLinear` of `methods/luminance.ts`: a channel value in linear light.
fn to_linear(channel: f64) -> f64 {
    let normal = channel / 255.0;
    if channel > 0.03928 {
        ((normal + 0.055) / 1.055).powf(2.4)
    } else {
        normal / 12.92
    }
}

/// `luminance` of `methods/luminance.ts`: the relative luminance, rounded to ten decimals.
///
/// # Errors
///
/// Returns an error when the colour is not one khroma reads.
pub fn luminance(color: &str) -> Result<f64, ThemeError> {
    let mut parsed = parse(color)?;
    let r = parsed.get(Channel::R);
    let g = parsed.get(Channel::G);
    let b = parsed.get(Channel::B);
    let value = 0.2126 * to_linear(r) + 0.7152 * to_linear(g) + 0.0722 * to_linear(b);
    Ok(lang_round(value))
}

/// `isLight` of `methods/is_light.ts`: a luminance of at least one half.
///
/// # Errors
///
/// Returns an error when the colour is not one khroma reads.
pub fn is_light(color: &str) -> Result<bool, ThemeError> {
    Ok(luminance(color)? >= 0.5)
}

/// `isDark` of `methods/is_dark.ts`: the negation of [`is_light`].
///
/// # Errors
///
/// Returns an error when the colour is not one khroma reads.
pub fn is_dark(color: &str) -> Result<bool, ThemeError> {
    Ok(!is_light(color)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adjusted(color: &str, channels: &[(Channel, f64)]) -> String {
        match adjust(color, channels) {
            Ok(text) => text,
            Err(error) => format!("error: {error}"),
        }
    }

    #[test]
    fn the_patterns_compile() {
        assert!(RGB_RE.is_some());
        assert!(HSL_RE.is_some());
    }

    #[test]
    fn js_numbers_print_as_javascript_does() {
        assert_eq!(js_number(0.0), "0");
        assert_eq!(js_number(-0.0), "0");
        assert_eq!(js_number(127.5), "127.5");
        assert_eq!(js_number(-120.0), "-120");
        assert_eq!(js_number(0.000_001), "0.000001");
        assert_eq!(js_number(0.000_000_1), "1e-7");
        assert_eq!(js_number(1e20), "100000000000000000000");
        assert_eq!(js_number(1e21), "1e+21");
        assert_eq!(js_number(1.5e21), "1.5e+21");
        assert_eq!(js_number(0.5333333333333333), "0.5333333333333333");
        assert_eq!(js_number(f64::NAN), "NaN");
        assert_eq!(js_number(f64::NEG_INFINITY), "-Infinity");
    }

    #[test]
    fn js_round_rounds_halves_up() {
        assert_eq!(js_round(2.5), 3.0);
        assert_eq!(js_round(-2.5), -2.0);
        assert_eq!(js_round(0.49999999999999994), 0.0);
        assert_eq!(lang_round(0.333_333_333_333_333_3), 0.3333333333);
    }

    #[test]
    fn parse_float_reads_the_longest_number_prefix() {
        assert_eq!(parse_float("120deg"), 120.0);
        assert_eq!(parse_float("1e2deg"), 100.0);
        assert_eq!(parse_float("1e"), 1.0);
        assert_eq!(parse_float("-.5"), -0.5);
        assert_eq!(parse_float("50%"), 50.0);
        assert!(parse_float(".").is_nan());
    }

    #[test]
    fn hue_units_convert_only_in_lower_case() {
        assert_eq!(hue2deg("0.5turn"), 180.0);
        assert_eq!(hue2deg("100grad"), 90.0);
        assert_eq!(hue2deg("120DEG"), 120.0);
        assert_eq!(hue2deg("1GRAD"), 1.0);
        assert_eq!(hue2deg("400"), 40.0);
    }

    #[test]
    fn hex_colours_parse_in_all_four_lengths() {
        let short = parse("#0f08").ok();
        assert_eq!(
            short.map(|mut channels| (
                channels.get(Channel::R),
                channels.get(Channel::G),
                channels.data.a
            )),
            Some((0.0, 255.0, 0.5333333333333333))
        );
        assert!(parse("#12345").is_err());
        assert!(parse("#ggg").is_err());
        assert!(parse("").is_err());
    }

    #[test]
    fn colours_that_do_not_change_keep_their_text() {
        assert_eq!(adjusted("#fff", &[]), "#fff");
        assert_eq!(adjusted("#abc", &[(Channel::L, 0.0)]), "#abc");
        assert_eq!(adjusted("white", &[]), "#ffffff");
        assert_eq!(lighten("#ffffff", 4.0).ok().as_deref(), Some("#ffffff"));
        assert_eq!(darken("#000000", 4.0).ok().as_deref(), Some("#000000"));
        assert_eq!(darken("#fff", 0.0).ok().as_deref(), Some("#fff"));
    }

    #[test]
    fn adjust_moves_hues_and_channels() {
        assert_eq!(
            adjusted("#cccccc", &[(Channel::H, -120.0)]),
            "hsl(-120, 0%, 80%)"
        );
        assert_eq!(
            adjusted("#1f2020", &[(Channel::H, -160.0)]),
            "hsl(20, 1.5873015873%, 12.3529411765%)"
        );
        assert_eq!(
            adjusted("#ECECFE", &[(Channel::H, 180.0), (Channel::L, 5.0)]),
            "hsl(60, 90%, 100%)"
        );
        assert_eq!(
            adjusted("#28253D", &[(Channel::S, -40.0), (Channel::L, -10.0)]),
            "hsl(247.5, 0%, 9.2156862745%)"
        );
        assert_eq!(
            adjusted(
                "#ECECFE",
                &[(Channel::R, 5.0), (Channel::G, 5.0), (Channel::B, 5.0)]
            ),
            "#f1f1ff"
        );
        assert_eq!(
            adjusted(
                "rgb(10, 20, 30)",
                &[
                    (Channel::R, -15.0),
                    (Channel::G, -15.0),
                    (Channel::B, -15.0)
                ]
            ),
            "#00050f"
        );
        assert_eq!(
            adjusted("rgb(10.5, 20.25, 30.75)", &[(Channel::L, 10.0)]),
            "hsl(211.1111111111, 49.0909090909%, 18.0882352941%)"
        );
        assert_eq!(
            adjusted("rgb(300%, -5%, 50%)", &[(Channel::L, 0.0001)]),
            "hsl(330, 100%, 50.0001%)"
        );
    }

    #[test]
    fn adjust_reads_hsl_and_alpha_colours() {
        assert_eq!(
            adjusted("hsl(120, 50%, 50%)", &[(Channel::L, 5.0)]),
            "hsl(120, 50%, 55%)"
        );
        assert_eq!(
            adjusted("hsl(120, 50%, 50%)", &[(Channel::H, -200.0)]),
            "hsl(-80, 50%, 50%)"
        );
        assert_eq!(
            adjusted("hsl(120 50% 50% / 0.5)", &[(Channel::L, 5.0)]),
            "hsla(120, 50%, 55%, 0.5)"
        );
        assert_eq!(
            adjusted("rgba(10 20 30 / 50%)", &[(Channel::L, 10.0)]),
            "hsla(210, 50%, 17.8431372549%, 0.5)"
        );
        assert_eq!(
            adjusted("#0f08", &[(Channel::L, 10.0)]),
            "hsla(120, 100%, 60%, 0.5333333333333333)"
        );
        assert_eq!(
            adjusted("#0f08", &[(Channel::R, 5.0)]),
            "rgba(5, 255, 0, 0.5333333333)"
        );
        assert_eq!(
            adjusted("#0f0f", &[(Channel::L, 10.0)]),
            "hsl(120, 100%, 60%)"
        );
    }

    #[test]
    fn adjust_reads_hue_units() {
        assert_eq!(
            adjusted("hsl(0.5turn 50% 50%)", &[(Channel::H, 10.0)]),
            "hsl(190, 50%, 50%)"
        );
        assert_eq!(
            adjusted("hsl(100grad 50% 50%)", &[(Channel::H, 10.0)]),
            "hsl(100, 50%, 50%)"
        );
        assert_eq!(
            adjusted("hsl(1rad 50% 50%)", &[(Channel::H, 10.0)]),
            "hsl(67.2957795131, 50%, 50%)"
        );
        assert_eq!(
            adjusted("hsl(120DEG 50% 50%)", &[(Channel::H, 10.0)]),
            "hsl(130, 50%, 50%)"
        );
    }

    #[test]
    fn darken_and_lighten_move_the_lightness() {
        assert_eq!(
            darken("#ffffff", 10.0).ok().as_deref(),
            Some("hsl(0, 0%, 90%)")
        );
        assert_eq!(
            darken("#ffa2a2", 75.0).ok().as_deref(),
            Some("hsl(0, 100%, 6.7647058824%)")
        );
        assert_eq!(
            darken("rgba(10, 20, 30, 0.5)", 10.0).ok().as_deref(),
            Some("hsla(210, 50%, 0%, 0.5)")
        );
        assert_eq!(
            darken("hsl(120, 50%, 50%)", 10.0).ok().as_deref(),
            Some("hsl(120, 50%, 40%)")
        );
        assert_eq!(
            darken("white", 10.0).ok().as_deref(),
            Some("hsl(0, 0%, 90%)")
        );
        assert_eq!(
            lighten("#333", 16.0).ok().as_deref(),
            Some("hsl(0, 0%, 36%)")
        );
        assert_eq!(
            lighten("#111113", 16.0).ok().as_deref(),
            Some("hsl(240, 5.5555555556%, 23.0588235294%)")
        );
        assert_eq!(
            lighten("transparent", 10.0).ok().as_deref(),
            Some("hsla(0, 0%, 10%, 0)")
        );
    }

    #[test]
    fn invert_mixes_the_inverse_with_the_colour() {
        assert_eq!(invert("#333", 100.0).ok().as_deref(), Some("#cccccc"));
        assert_eq!(invert("#ffffff", 100.0).ok().as_deref(), Some("#000000"));
        assert_eq!(
            invert("#ffffff", 50.0).ok().as_deref(),
            Some("rgb(127.5, 127.5, 127.5)")
        );
        assert_eq!(invert("#1f2020", 100.0).ok().as_deref(), Some("#e0dfdf"));
        assert_eq!(invert("white", 100.0).ok().as_deref(), Some("#000000"));
        assert_eq!(invert("#ccc", 100.0).ok().as_deref(), Some("#333333"));
        assert_eq!(
            invert("hsl(120, 50%, 50%)", 100.0).ok().as_deref(),
            Some("rgb(191.25, 63.75, 191.25)")
        );
        assert_eq!(
            invert("rgba(10, 20, 30, 0.5)", 100.0).ok().as_deref(),
            Some("rgba(245, 235, 225, 0.5)")
        );
        assert_eq!(
            invert("#0f08", 100.0).ok().as_deref(),
            Some("rgba(255, 0, 255, 0.5333333333)")
        );
        assert_eq!(
            invert("rgb(10.5, 20.25, 30.75)", 100.0).ok().as_deref(),
            Some("rgb(244.5, 234.75, 224.25)")
        );
    }

    #[test]
    fn mix_and_rgba_format_numbers() {
        assert_eq!(
            mix("#ffffff", "#000000", 50.0).ok().as_deref(),
            Some("rgb(127.5, 127.5, 127.5)")
        );
        assert_eq!(rgba(255.0, 255.0, 255.0, 0.25), "rgba(255, 255, 255, 0.25)");
        assert_eq!(rgba(0.0, 0.0, 0.0, 1.0), "#000000");
        assert_eq!(
            rgba(10.5, 20.0, 30.0, 0.333_333_333_333),
            "rgba(10.5, 20, 30, 0.3333333333)"
        );
        assert_eq!(rgba(300.0, -5.0, 128.0, 2.0), "#ff0080");
    }

    #[test]
    fn luminance_and_darkness_follow_the_reference() {
        assert_eq!(luminance("#808080").ok(), Some(0.2158605001));
        assert_eq!(luminance("hsl(120, 50%, 50%)").ok(), Some(0.3881969253));
        assert_eq!(luminance("#333").ok(), Some(0.0331047666));
        assert_eq!(is_dark("#111113").ok(), Some(true));
        assert_eq!(is_dark("#ffffff").ok(), Some(false));
        assert_eq!(is_dark("hsl(120, 50%, 50%)").ok(), Some(true));
        assert_eq!(is_light("rgb(128, 128, 128)").ok(), Some(false));
    }

    #[test]
    fn colours_khroma_does_not_read_are_errors() {
        assert!(darken("unreadable", 1.0).is_err());
        assert!(darken("rgb(1,2)", 1.0).is_err());
        assert!(invert("", 100.0).is_err());
        assert!(is_dark("#12345").is_err());
    }
}
