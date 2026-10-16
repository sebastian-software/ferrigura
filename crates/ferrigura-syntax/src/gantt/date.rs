//! The date rules of the gantt DB, for the reference's dayjs 1.11.21 run in UTC (the port's zone,
//! and the zone the reference runs in for the differential run, so local time equals UTC).
//!
//! - [`strict_parse`] is `dayjs(str, format, true)`: the customParseFormat plugin, its token table,
//!   the parts a format leaves out (taken from the clock), and the strict check that formats the
//!   parsed time again and compares it with the input.
//! - [`format`] is `dayjs(time).format(format)` with the default format and advancedFormat. The
//!   tokens `w`, `wo`, `ww`, `gggg`, `z` and `zzz` call methods of plugins the reference does not
//!   load, so they throw; [`Thrown`] carries that to the DB, which rejects the diagram.
//! - [`add`] is `add(amount, unit)`: `Math.round` for days and weeks, calendar months and years
//!   clamped to the length of the month, and fixed milliseconds for the smaller units.
//! - [`legacy_parse`] is `new Date(string).getTime()` of V8 for the forms of PLAN section 5.3. The
//!   accepted forms are: digits alone (a year, the two-digit rule below); `YYYY`, `YYYY-MM` and
//!   `YYYY/MM`; `Y-M-D` and `Y/M/D` with any digit counts; `M-D-Y` and `M/D/Y` (the year takes the
//!   two-digit rule); an optional time `HH:mm[:ss[.fff]]` after a space, or after `T` for an ISO
//!   date; an optional zone `Z` or `±HH:mm` / `±HHmm`. Month 1 to 12, day 1 to 31 (days roll over),
//!   hour 0 to 24 (24 only at `00:00`), minute and second 0 to 59. Every other form is NaN. The
//!   forms that V8 accepts and this module does not (`Jan 1 2014`, `2014/1`, `1/6`, `1.5`, signed
//!   years, `Y-M` with a one-digit year) are gaps; the differential run reports them if an input
//!   reaches one.
//!
//! Times are JS Date values: milliseconds since the epoch, integral, NaN for an Invalid Date.

use crate::text::{is_js_space, js_trim};

/// A JS Date value: milliseconds since the epoch in UTC; NaN is an Invalid Date.
pub(crate) type Time = f64;

const DAY_MS: f64 = 86_400_000.0;
const CLIP_MS: f64 = 8.64e15;
const DEFAULT_FORMAT: &str = "YYYY-MM-DDTHH:mm:ssZ";

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const MONTHS_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const WEEKDAYS_SHORT: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const WEEKDAYS_MIN: [&str; 7] = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];

/// The units of `add`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unit {
    Millisecond,
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Year,
}

/// dayjs threw: the format uses a token whose method the reference's plugins do not provide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Thrown(pub(crate) String);

/// `TimeClip`: NaN when the value is outside ±8.64e15 ms or not a number, else the integer part.
pub(crate) fn time_clip(value: f64) -> Time {
    if !value.is_finite() || value.abs() > CLIP_MS {
        f64::NAN
    } else {
        // `+ 0.0` turns -0 into +0, as TimeClip does.
        value.trunc() + 0.0
    }
}

/// Days since the epoch of a proleptic Gregorian date; `month` is 1 to 12.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let year_of_era = y - era * 400;
    let month_index = (month + 9) % 12;
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The (year, month 1 to 12, day) of a day number since the epoch.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// The calendar parts of a finite time value, in UTC.
#[derive(Debug, Clone, Copy)]
struct Parts {
    year: i64,
    /// 0 for January.
    month: i64,
    day: i64,
    /// 0 for Sunday.
    weekday: i64,
    hour: i64,
    minute: i64,
    second: i64,
    ms: i64,
    /// The day number since the epoch.
    days: i64,
}

fn parts(time: Time) -> Parts {
    let ms = time as i64;
    let days = ms.div_euclid(86_400_000);
    let rem = ms.rem_euclid(86_400_000);
    let (year, month, day) = civil_from_days(days);
    Parts {
        year,
        month: month - 1,
        day,
        weekday: (days + 4).rem_euclid(7),
        hour: rem / 3_600_000,
        minute: rem / 60_000 % 60,
        second: rem / 1000 % 60,
        ms: rem % 1000,
        days,
    }
}

/// `MakeDay`: the day number of a year, a month (0 for January, out of range rolls over) and a day
/// (out of range rolls over). NaN for a non-finite argument.
fn make_day(year: f64, month: f64, date: f64) -> f64 {
    if !(year.is_finite() && month.is_finite() && date.is_finite()) {
        return f64::NAN;
    }
    let (year, month, date) = (year.trunc(), month.trunc(), date.trunc());
    if year.abs() > 1.0e9 || month.abs() > 1.0e9 || date.abs() > 1.0e9 {
        return f64::NAN;
    }
    let total_year = year + (month / 12.0).floor();
    let month_in_year = month.rem_euclid(12.0);
    days_from_civil(total_year as i64, month_in_year as i64 + 1, 1) as f64 + date - 1.0
}

/// The JS constructor `new Date(year, month, …)` and `Date.UTC`: a year from 0 to 99 means 1900 to
/// 1999, then `MakeDay` and `MakeTime`.
fn make_date(
    year: f64,
    month: f64,
    date: f64,
    hour: f64,
    minute: f64,
    second: f64,
    ms: f64,
) -> Time {
    let year = if (0.0..=99.0).contains(&year.trunc()) {
        year.trunc() + 1900.0
    } else {
        year
    };
    make_date_raw(year, month, date, hour, minute, second, ms)
}

/// `MakeDate(MakeDay, MakeTime)` without the year rule, as `setMonth` and the legacy parser use it.
fn make_date_raw(
    year: f64,
    month: f64,
    date: f64,
    hour: f64,
    minute: f64,
    second: f64,
    ms: f64,
) -> Time {
    let day = make_day(year, month, date);
    let time = hour.trunc() * 3_600_000.0
        + minute.trunc() * 60_000.0
        + second.trunc() * 1000.0
        + ms.trunc();
    time_clip(day * DAY_MS + time)
}

/// Days in a month (`month` is 1 to 12).
fn days_in_month(year: i64, month: i64) -> i64 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    days_from_civil(next_year, next_month, 1) - days_from_civil(year, month, 1)
}

/// The ISO week number and the ISO week-numbering year of a day number.
fn iso_week_and_year(days: i64) -> (i64, i64) {
    let iso_weekday = (days + 3).rem_euclid(7) + 1;
    let thursday = days + (4 - iso_weekday);
    let (year, _, _) = civil_from_days(thursday);
    let week = (thursday - days_from_civil(year, 1, 1)) / 7 + 1;
    (week, year)
}

/// `dayjs(time).isoWeekday()`: 1 for Monday to 7 for Sunday. NaN is 0 here, and callers never pass
/// NaN.
pub(crate) fn iso_weekday(time: Time) -> u32 {
    let weekday = parts(time).weekday;
    if weekday == 0 { 7 } else { weekday as u32 }
}

/// The year of a time value, as `getFullYear()`; NaN for NaN.
pub(crate) fn full_year(time: Time) -> f64 {
    if time.is_nan() {
        f64::NAN
    } else {
        parts(time).year as f64
    }
}

/// `new Date(time)` at UTC midnight of its day, as `setHours(0, 0, 0, 0)` gives; NaN for NaN.
pub(crate) fn start_of_day(time: Time) -> Time {
    if time.is_nan() {
        return f64::NAN;
    }
    let days = (time as i64).div_euclid(86_400_000);
    days as f64 * DAY_MS
}

/// Sets the year and the month of a time value, keeping the day of month where it exists, as
/// dayjs `$set` does for months and years: the day is clamped to the length of the new month.
fn set_year_month(time: Time, year: f64, month: f64) -> Time {
    if time.is_nan() {
        return f64::NAN;
    }
    let original = parts(time);
    let first = make_day(year, month, 1.0);
    if first.is_nan() {
        return f64::NAN;
    }
    let (new_year, new_month, _) = civil_from_days(first as i64);
    let day = original.day.min(days_in_month(new_year, new_month));
    let days = days_from_civil(new_year, new_month, day);
    let time_of_day = (time as i64).rem_euclid(86_400_000);
    time_clip(days as f64 * DAY_MS + time_of_day as f64)
}

/// `dayjs(time).add(amount, unit).toDate().getTime()`, with dayjs's rounding for days and weeks.
pub(crate) fn add(time: Time, amount: f64, unit: Unit) -> Time {
    if time.is_nan() {
        return f64::NAN;
    }
    let round = |value: f64| (value + 0.5).floor();
    match unit {
        Unit::Day => time_clip(time + round(amount) * DAY_MS),
        Unit::Week => time_clip(time + round(7.0 * amount) * DAY_MS),
        Unit::Month => {
            let original = parts(time);
            set_year_month(time, original.year as f64, original.month as f64 + amount)
        }
        Unit::Year => {
            let original = parts(time);
            set_year_month(time, original.year as f64 + amount, original.month as f64)
        }
        Unit::Millisecond => time_clip(time + amount),
        Unit::Second => time_clip(time + amount * 1000.0),
        Unit::Minute => time_clip(time + amount * 60_000.0),
        Unit::Hour => time_clip(time + amount * 3_600_000.0),
    }
}

/// The reference's `parseDuration`: `/^(\d+(?:\.\d+)?)([Mdhmswy]|ms)$/` on the trimmed text.
pub(crate) fn parse_duration(input: &str) -> Option<(f64, Unit)> {
    let text = js_trim(input);
    let digits = text.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let mut end = digits;
    if text[end..].starts_with('.') {
        let fraction = text[end + 1..]
            .bytes()
            .take_while(u8::is_ascii_digit)
            .count();
        if fraction == 0 {
            return None;
        }
        end += 1 + fraction;
    }
    let unit = match &text[end..] {
        "ms" => Unit::Millisecond,
        "M" => Unit::Month,
        "d" => Unit::Day,
        "h" => Unit::Hour,
        "m" => Unit::Minute,
        "s" => Unit::Second,
        "w" => Unit::Week,
        "y" => Unit::Year,
        _ => return None,
    };
    let value: f64 = text[..end].parse().ok()?;
    Some((value, unit))
}

/// JavaScript's `ToNumber` of a string: blanks are trimmed, the empty string is 0, hexadecimal,
/// octal and binary prefixes are read, and anything else that is not a decimal literal is NaN.
pub(crate) fn js_to_number(text: &str) -> f64 {
    let trimmed = js_trim(text);
    if trimmed.is_empty() {
        return 0.0;
    }
    for (prefix, radix) in [
        ("0x", 16),
        ("0X", 16),
        ("0o", 8),
        ("0O", 8),
        ("0b", 2),
        ("0B", 2),
    ] {
        if let Some(digits) = trimmed.strip_prefix(prefix) {
            return u64::from_str_radix(digits, radix).map_or(f64::NAN, |value| value as f64);
        }
    }
    match trimmed {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    if is_decimal_literal(trimmed) {
        trimmed.parse().unwrap_or(f64::NAN)
    } else {
        f64::NAN
    }
}

/// `[+-]? (digits [. digits?] | . digits) ([eE] [+-]? digits)?`, the whole text.
fn is_decimal_literal(text: &str) -> bool {
    let body = text.strip_prefix(['+', '-']).unwrap_or(text);
    let (mantissa, exponent) = match body.find(['e', 'E']) {
        Some(index) => (&body[..index], Some(&body[index + 1..])),
        None => (body, None),
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let mantissa_ok = match mantissa.split_once('.') {
        Some((whole, fraction)) => {
            (whole.is_empty() || digits(whole))
                && (fraction.is_empty() || digits(fraction))
                && !(whole.is_empty() && fraction.is_empty())
        }
        None => digits(mantissa),
    };
    let exponent_ok = match exponent {
        None => true,
        Some(e) => digits(e.strip_prefix(['+', '-']).unwrap_or(e)),
    };
    mantissa_ok && exponent_ok
}

/// Strict parsing, `dayjs(input, format, true)`, with `format` already trimmed by the caller. `now`
/// is the clock. `Ok(None)` is an invalid date; `Err` is a throw of the reference.
pub(crate) fn strict_parse(input: &str, format: &str, now: Time) -> Result<Option<Time>, Thrown> {
    let time = if format == "x" || format == "X" {
        let factor = if format == "X" { 1000.0 } else { 1.0 };
        time_clip(js_to_number(input) * factor)
    } else {
        parse_fields(input, format, now).unwrap_or(f64::NAN)
    };
    if time.is_nan() {
        return Ok(None);
    }
    let shown = format_time(time, format)?;
    Ok((shown == input).then_some(time))
}

/// Formats a time as `dayjs(time).format(format)` does, in UTC. NaN is `Invalid Date`. The empty
/// format gives dayjs's default string.
pub(crate) fn format(time: Time, format: &str) -> Result<String, Thrown> {
    format_time(time, format)
}

fn format_time(time: Time, format: &str) -> Result<String, Thrown> {
    if time.is_nan() {
        return Ok("Invalid Date".to_owned());
    }
    let pattern = if format.is_empty() {
        DEFAULT_FORMAT
    } else {
        format
    };
    let now = parts(time);
    let advanced = advanced_pass(pattern, time, now)?;
    Ok(base_pass(&advanced, now))
}

/// The tokens of `formatStr.replace(…)` in advancedFormat: `Q`, the week tokens, `Do`, `X`, `x`, `k`
/// and the ISO week tokens; a bracketed literal stays for the base pass.
fn advanced_pass(pattern: &str, time: Time, now: Parts) -> Result<String, Thrown> {
    let chars: Vec<char> = pattern.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '['
            && let Some(end) = closing_bracket(&chars, i)
        {
            out.extend(&chars[i..=end]);
            i = end + 1;
            continue;
        }
        let next = chars.get(i + 1).copied();
        match c {
            'Q' => out.push_str(&((now.month + 3) / 3).to_string()),
            'w' => {
                return Err(Thrown(
                    "the week-of-year token needs the week plugin".to_owned(),
                ));
            }
            'W' => {
                let (week, _) = iso_week_and_year(now.days);
                if next == Some('W') {
                    out.push_str(&pad_start(&week.to_string(), 2));
                    i += 1;
                } else {
                    out.push_str(&week.to_string());
                }
            }
            'z' => {
                return Err(Thrown(
                    "the offset-name token needs the timezone plugin".to_owned(),
                ));
            }
            'g' if chars[i..].starts_with(&['g', 'g', 'g', 'g']) => {
                return Err(Thrown(
                    "the week-year token needs the week plugin".to_owned(),
                ));
            }
            'G' if chars[i..].starts_with(&['G', 'G', 'G', 'G']) => {
                let (_, year) = iso_week_and_year(now.days);
                out.push_str(&year.to_string());
                i += 3;
            }
            'D' if next == Some('o') => {
                out.push('[');
                out.push_str(&ordinal(now.day));
                out.push(']');
                i += 1;
            }
            'X' => out.push_str(&(time as i64).div_euclid(1000).to_string()),
            'x' => out.push_str(&(time as i64).to_string()),
            'k' => {
                let twice = next == Some('k');
                let hour = if now.hour == 0 { 24 } else { now.hour };
                let shown = hour.to_string();
                out.push_str(&if twice { pad_start(&shown, 2) } else { shown });
                if twice {
                    i += 1;
                }
            }
            _ => out.push(c),
        }
        i += 1;
    }
    Ok(out)
}

/// The index of the `]` that closes the bracket at `start`, as `\[([^\]]+)]` matches it.
fn closing_bracket(chars: &[char], start: usize) -> Option<usize> {
    let end = (start + 1..chars.len()).find(|&k| chars[k] == ']')?;
    (end > start + 1).then_some(end)
}

/// The tokens of the base pass (`REGEX_FORMAT`): the years, months, days, weekdays, hours, minutes,
/// seconds, zone and milliseconds. A bracketed literal is written as its text.
fn base_pass(text: &str, now: Parts) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '['
            && let Some(end) = closing_bracket(&chars, i)
        {
            out.extend(&chars[i + 1..end]);
            i = end + 1;
            continue;
        }
        let run = chars[i..].iter().take_while(|&&k| k == c).count();
        let hour12 = if now.hour % 12 == 0 {
            12
        } else {
            now.hour % 12
        };
        let (used, shown) = match c {
            'Y' if run >= 4 => (4, pad_start(&now.year.to_string(), 4)),
            'Y' if run >= 2 => (2, year_two_digits(now.year)),
            'M' => {
                let used = run.min(4);
                let shown = match used {
                    1 => (now.month + 1).to_string(),
                    2 => pad_start(&(now.month + 1).to_string(), 2),
                    3 => MONTHS_SHORT[now.month as usize].to_owned(),
                    _ => MONTHS[now.month as usize].to_owned(),
                };
                (used, shown)
            }
            'D' => {
                let used = run.min(2);
                let shown = if used == 1 {
                    now.day.to_string()
                } else {
                    pad_start(&now.day.to_string(), 2)
                };
                (used, shown)
            }
            'd' => {
                let used = run.min(4);
                let day = now.weekday as usize;
                let shown = match used {
                    1 => day.to_string(),
                    2 => WEEKDAYS_MIN[day].to_owned(),
                    3 => WEEKDAYS_SHORT[day].to_owned(),
                    _ => WEEKDAYS[day].to_owned(),
                };
                (used, shown)
            }
            'H' => {
                let used = run.min(2);
                let shown = if used == 1 {
                    now.hour.to_string()
                } else {
                    pad_start(&now.hour.to_string(), 2)
                };
                (used, shown)
            }
            'h' => {
                let used = run.min(2);
                let shown = if used == 1 {
                    hour12.to_string()
                } else {
                    pad_start(&hour12.to_string(), 2)
                };
                (used, shown)
            }
            'a' => (1, if now.hour < 12 { "am" } else { "pm" }.to_owned()),
            'A' => (1, if now.hour < 12 { "AM" } else { "PM" }.to_owned()),
            'm' => {
                let used = run.min(2);
                let shown = if used == 1 {
                    now.minute.to_string()
                } else {
                    pad_start(&now.minute.to_string(), 2)
                };
                (used, shown)
            }
            's' => {
                let used = run.min(2);
                let shown = if used == 1 {
                    now.second.to_string()
                } else {
                    pad_start(&now.second.to_string(), 2)
                };
                (used, shown)
            }
            'Z' => {
                let used = run.min(2);
                let shown = if used == 1 { "+00:00" } else { "+0000" };
                (used, shown.to_owned())
            }
            'S' if run >= 3 => (3, pad_start(&now.ms.to_string(), 3)),
            _ => (1, c.to_string()),
        };
        out.push_str(&shown);
        i += used;
    }
    out
}

/// `Utils.s(value, length, '0')`: the value as text, padded on the left with zeros.
fn pad_start(text: &str, length: usize) -> String {
    if text.len() >= length {
        return text.to_owned();
    }
    let mut out = "0".repeat(length - text.len());
    out.push_str(text);
    out
}

/// `String(year).slice(-2)`.
fn year_two_digits(year: i64) -> String {
    let text = year.to_string();
    let start = text.len().saturating_sub(2);
    text[start..].to_owned()
}

/// The English ordinal of a day, as `en` gives it (`1st`, `2nd`, `11th`, `21st`).
fn ordinal(day: i64) -> String {
    const SUFFIX: [&str; 4] = ["th", "st", "nd", "rd"];
    let v = day % 100;
    let t = (v - 20) % 10;
    let suffix = if (0..=3).contains(&t) {
        SUFFIX[t as usize]
    } else if (0..=3).contains(&v) {
        SUFFIX[v as usize]
    } else {
        "th"
    };
    format!("{day}{suffix}")
}

/// A token of the format for the strict parse: a literal of some length, or a token that reads a
/// value with a matcher and sets a part.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Piece {
    Literal(usize),
    Parse(Matcher, Setter),
}

/// The regular expressions of customParseFormat, as the search they run: `exec` finds the first
/// match anywhere in the rest of the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Matcher {
    One,
    Two,
    Three,
    Four,
    OneToTwo,
    Signed,
    Word,
    Offset,
}

/// The setters of customParseFormat's token table.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Setter {
    Meridiem { lower: bool },
    Quarter,
    Millis { factor: f64 },
    Seconds,
    Minutes,
    Hours,
    Day,
    DayOrdinal,
    Week,
    Month,
    MonthName { short: bool },
    Year,
    YearTwoDigits,
    Zone,
}

/// The token table of customParseFormat: the token's matcher and setter.
fn expression(token: &str) -> Option<(Matcher, Setter)> {
    use Matcher as M;
    use Setter as S;
    Some(match token {
        "A" => (M::Word, S::Meridiem { lower: false }),
        "a" => (M::Word, S::Meridiem { lower: true }),
        "Q" => (M::One, S::Quarter),
        "S" => (M::One, S::Millis { factor: 100.0 }),
        "SS" => (M::Two, S::Millis { factor: 10.0 }),
        "SSS" => (M::Three, S::Millis { factor: 1.0 }),
        "s" | "ss" => (M::OneToTwo, S::Seconds),
        "m" | "mm" => (M::OneToTwo, S::Minutes),
        "H" | "HH" | "h" | "hh" => (M::OneToTwo, S::Hours),
        "D" => (M::OneToTwo, S::Day),
        "DD" => (M::Two, S::Day),
        "Do" => (M::Word, S::DayOrdinal),
        "w" => (M::OneToTwo, S::Week),
        "ww" => (M::Two, S::Week),
        "M" => (M::OneToTwo, S::Month),
        "MM" => (M::Two, S::Month),
        "MMM" => (M::Word, S::MonthName { short: true }),
        "MMMM" => (M::Word, S::MonthName { short: false }),
        "Y" | "YYYY" => (if token == "Y" { M::Signed } else { M::Four }, S::Year),
        "YY" => (M::Two, S::YearTwoDigits),
        "Z" | "ZZ" => (M::Offset, S::Zone),
        _ => return None,
    })
}

/// customParseFormat's `formattingTokens`: the pieces of a format, as its `match` returns them.
/// Characters that no alternative matches are dropped, as they are by `match`.
fn tokenize_format(format: &str) -> Vec<Piece> {
    let chars: Vec<char> = format.chars().collect();
    let mut pieces = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        // `\[[^[]*\]`: the last `]` before the next `[`.
        if c == '[' {
            let stop = (i + 1..chars.len())
                .find(|&k| chars[k] == '[')
                .unwrap_or(chars.len());
            if let Some(close) = (i + 1..stop).rev().find(|&k| chars[k] == ']') {
                pieces.push(Piece::Literal(close - i - 1));
                i = close + 1;
                continue;
            }
        }
        // `[-_:/.,()\s]+`
        if is_separator(c) {
            let run = chars[i..].iter().take_while(|&&k| is_separator(k)).count();
            pieces.push(Piece::Literal(run));
            i += run;
            continue;
        }
        let same = |limit: usize| {
            chars[i..]
                .iter()
                .take_while(|&&k| k == c)
                .count()
                .min(limit)
        };
        let token_len = match c {
            'A' | 'a' | 'Q' => 1,
            'Y' => {
                if same(4) == 4 {
                    4
                } else {
                    same(2)
                }
            }
            'w' => same(2),
            'M' => same(4),
            'D' => {
                if chars.get(i + 1) == Some(&'o') {
                    2
                } else {
                    same(2)
                }
            }
            'h' | 'H' | 'm' | 's' => same(2),
            'S' => same(3),
            'z' => 1,
            'Z' => same(2),
            _ => 0,
        };
        if token_len == 0 {
            i += 1;
            continue;
        }
        let token: String = chars[i..i + token_len].iter().collect();
        match expression(&token) {
            Some((matcher, setter)) => pieces.push(Piece::Parse(matcher, setter)),
            None => pieces.push(Piece::Literal(token.chars().count())),
        }
        i += token_len;
    }
    pieces
}

fn is_separator(c: char) -> bool {
    matches!(c, '-' | '_' | ':' | '/' | '.' | ',' | '(' | ')') || is_js_space(c)
}

fn is_digit(unit: u16) -> bool {
    (u16::from(b'0')..=u16::from(b'9')).contains(&unit)
}

fn is_sign(unit: u16) -> bool {
    unit == u16::from(b'+') || unit == u16::from(b'-')
}

/// `[^-_:/,()\s\d]`, one UTF-16 unit.
fn is_word_unit(unit: u16) -> bool {
    match char::from_u32(u32::from(unit)) {
        None => true,
        Some(c) => {
            !(c.is_ascii_digit()
                || matches!(c, '-' | '_' | ':' | '/' | ',' | '(' | ')')
                || is_js_space(c))
        }
    }
}

/// The first match of a matcher in `part`, as `(start, length)`, with the search of `RegExp.exec`.
fn exec(matcher: Matcher, part: &[u16]) -> Option<(usize, usize)> {
    let n = part.len();
    let digits_at = |at: usize, count: usize| {
        at + count <= n && part[at..at + count].iter().all(|&u| is_digit(u))
    };
    match matcher {
        Matcher::One => (0..n).find(|&i| is_digit(part[i])).map(|i| (i, 1)),
        Matcher::Two => (0..n).find(|&i| digits_at(i, 2)).map(|i| (i, 2)),
        Matcher::Three => (0..n).find(|&i| digits_at(i, 3)).map(|i| (i, 3)),
        Matcher::Four => (0..n).find(|&i| digits_at(i, 4)).map(|i| (i, 4)),
        Matcher::OneToTwo => (0..n)
            .find(|&i| is_digit(part[i]))
            .map(|i| (i, if digits_at(i + 1, 1) { 2 } else { 1 })),
        Matcher::Signed => (0..n).find_map(|i| {
            if !is_digit(part[i]) && !(is_sign(part[i]) && digits_at(i + 1, 1)) {
                return None;
            }
            let start = i;
            let sign = usize::from(is_sign(part[i]));
            let run = part[i + sign..]
                .iter()
                .take_while(|&&u| is_digit(u))
                .count();
            Some((start, sign + run))
        }),
        Matcher::Word => (0..n).find_map(|i| {
            let digits = part[i..].iter().take_while(|&&u| is_digit(u)).count();
            let at = i + digits;
            if at < n && is_word_unit(part[at]) {
                let run = part[at..].iter().take_while(|&&u| is_word_unit(u)).count();
                Some((i, digits + run))
            } else {
                None
            }
        }),
        Matcher::Offset => (0..n).find_map(|i| {
            if part[i] == u16::from(b'Z') {
                return Some((i, 1));
            }
            if is_sign(part[i]) && digits_at(i + 1, 2) {
                let mut end = i + 3;
                if end < n && part[end] == u16::from(b':') {
                    end += 1;
                }
                if digits_at(end, 2) {
                    end += 2;
                }
                return Some((i, end - i));
            }
            None
        }),
    }
}

/// The part values a strict parse collects (`time` in customParseFormat's `parseFormattedInput`).
#[derive(Debug, Default)]
struct Fields {
    year: Option<f64>,
    month: Option<f64>,
    day: Option<f64>,
    /// Whether `day` came from the text of a `Do` token, which is a truthy string even for `0`.
    day_is_text: bool,
    hours: Option<f64>,
    minutes: Option<f64>,
    seconds: Option<f64>,
    milliseconds: Option<f64>,
    zone: Option<f64>,
    week: Option<f64>,
    afternoon: Option<bool>,
}

/// The number a token's text reads as (`+input`).
fn number_of(text: &[u16]) -> f64 {
    js_to_number(&String::from_utf16_lossy(text))
}

/// `offsetFromString` of customParseFormat: the offset in minutes, with the sign as the zone
/// expression reads it (a `+` offset is negative).
fn offset_minutes(text: &str) -> f64 {
    if text.is_empty() || text == "Z" {
        return 0.0;
    }
    let chars: Vec<char> = text.chars().collect();
    let mut parts: Vec<String> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '+' || chars[i] == '-' {
            parts.push(chars[i].to_string());
            i += 1;
        } else if chars
            .get(i..i + 2)
            .is_some_and(|pair| pair.iter().all(char::is_ascii_digit))
        {
            parts.push(chars[i..i + 2].iter().collect());
            i += 2;
        } else {
            i += 1;
        }
    }
    let sign = parts.first().cloned().unwrap_or_default();
    let hours: f64 = parts.get(1).and_then(|h| h.parse().ok()).unwrap_or(0.0);
    let minutes: f64 = parts.get(2).and_then(|m| m.parse().ok()).unwrap_or(0.0);
    let total = hours * 60.0 + minutes;
    if total == 0.0 {
        0.0
    } else if sign == "+" {
        -total
    } else {
        total
    }
}

impl Fields {
    /// The setter of a token, applied to its matched value. `None` is a throw inside the plugin's
    /// try block, which makes the date invalid.
    fn apply(&mut self, setter: Setter, value: &[u16]) -> Option<()> {
        let text = String::from_utf16_lossy(value);
        match setter {
            Setter::Meridiem { lower } => {
                self.afternoon = Some(text == if lower { "pm" } else { "PM" });
            }
            Setter::Quarter => self.month = Some((number_of(value) - 1.0) * 3.0 + 1.0),
            Setter::Millis { factor } => self.milliseconds = Some(number_of(value) * factor),
            Setter::Seconds => self.seconds = Some(number_of(value)),
            Setter::Minutes => self.minutes = Some(number_of(value)),
            Setter::Hours => self.hours = Some(number_of(value)),
            Setter::Day => {
                self.day = Some(number_of(value));
                self.day_is_text = false;
            }
            Setter::DayOrdinal => {
                // `input.match(/\d+/)[0]`, then the first ordinal of 1 to 31 that equals the text.
                let digits = value.iter().take_while(|&&u| !is_digit(u)).count();
                let run = value[digits..].iter().take_while(|&&u| is_digit(u)).count();
                if run == 0 {
                    return None;
                }
                self.day = Some(number_of(&value[digits..digits + run]));
                self.day_is_text = true;
                if let Some(day) = (1..=31).find(|&day| ordinal(day) == text) {
                    self.day = Some(day as f64);
                    self.day_is_text = false;
                }
            }
            Setter::Week => self.week = Some(number_of(value)),
            Setter::Month => self.month = Some(number_of(value)),
            Setter::MonthName { short } => {
                let names: Vec<String> = if short {
                    MONTHS_SHORT.iter().map(|s| (*s).to_owned()).collect()
                } else {
                    MONTHS.iter().map(|s| (*s).to_owned()).collect()
                };
                let index = names.iter().position(|name| *name == text)?;
                self.month = Some(index as f64 + 1.0);
            }
            Setter::Year => self.year = Some(number_of(value)),
            Setter::YearTwoDigits => {
                let year = number_of(value);
                self.year = Some(year + if year > 68.0 { 1900.0 } else { 2000.0 });
            }
            Setter::Zone => self.zone = Some(offset_minutes(&text)),
        }
        Some(())
    }

    /// `correctHours`: the afternoon flag of `a` and `A` turns the hour into the 24-hour one.
    fn correct_hours(&mut self) {
        let afternoon = self.afternoon.take();
        if let (Some(afternoon), Some(hours)) = (afternoon, self.hours) {
            if afternoon {
                if hours < 12.0 {
                    self.hours = Some(hours + 12.0);
                }
            } else if hours == 12.0 {
                self.hours = Some(0.0);
            }
        }
    }

    /// The date of the fields, with the defaults from the clock, as `parseFormattedInput` builds it.
    fn build(&self, now: Time) -> Option<Time> {
        let now = parts(now);
        let truthy = |value: Option<f64>| value.filter(|v| *v != 0.0 && !v.is_nan());
        let year = truthy(self.year);
        let month = truthy(self.month);
        let day_truthy = match self.day {
            Some(day) => self.day_is_text || (day != 0.0 && !day.is_nan()),
            None => false,
        };
        let day = match self.day {
            Some(day) if day_truthy => day,
            _ if year.is_none() && month.is_none() => now.day as f64,
            _ => 1.0,
        };
        let year_value = year.unwrap_or(now.year as f64);
        let month_value = if year.is_some() && month.is_none() {
            0.0
        } else {
            match self.month {
                Some(m) if m > 0.0 => m - 1.0,
                _ => now.month as f64,
            }
        };
        let or_zero = |value: Option<f64>| truthy(value).unwrap_or(0.0);
        let hours = or_zero(self.hours);
        let minutes = or_zero(self.minutes);
        let seconds = or_zero(self.seconds);
        let milliseconds = or_zero(self.milliseconds);
        if let Some(offset) = self.zone {
            let time = make_date(
                year_value,
                month_value,
                day,
                hours,
                minutes,
                seconds,
                milliseconds + offset * 60.0 * 1000.0,
            );
            return (!time.is_nan()).then_some(time);
        }
        if truthy(self.week).is_some() {
            // `dayjs(newDate).week(week)`: the week plugin is not loaded, so this throws.
            return None;
        }
        let time = make_date(
            year_value,
            month_value,
            day,
            hours,
            minutes,
            seconds,
            milliseconds,
        );
        (!time.is_nan()).then_some(time)
    }
}

/// The parse of customParseFormat for one input: the tokens read the input from left to right,
/// each matched value is removed from the input, and the parts are built into a time.
fn parse_fields(input: &str, format: &str, now: Time) -> Option<Time> {
    // `format.match(formattingTokens)` is null when nothing matches, and the parser then throws.
    let pieces = tokenize_format(format);
    if pieces.is_empty() {
        return None;
    }
    let mut text: Vec<u16> = input.encode_utf16().collect();
    let mut fields = Fields::default();
    let mut start = 0usize;
    for piece in pieces {
        match piece {
            Piece::Literal(length) => start += length,
            Piece::Parse(matcher, setter) => {
                let part = text.get(start..)?;
                let (at, length) = exec(matcher, part)?;
                let value: Vec<u16> = part[at..at + length].to_vec();
                fields.apply(setter, &value)?;
                // `input.replace(value, '')`: the first occurrence of the value, anywhere.
                if let Some(index) = text
                    .windows(value.len())
                    .position(|window| window == value.as_slice())
                {
                    text.drain(index..index + value.len());
                }
            }
        }
    }
    fields.correct_hours();
    fields.build(now)
}

/// The V8 legacy parse of `new Date(string)` for the forms of the module's documentation. The input
/// is already trimmed by the caller. NaN for every other form.
pub(crate) fn legacy_parse(input: &str) -> Time {
    legacy(input).unwrap_or(f64::NAN)
}

fn legacy(input: &str) -> Option<Time> {
    if input.is_empty() {
        return None;
    }
    if input.bytes().all(|b| b.is_ascii_digit()) {
        return lone_number(input);
    }
    let bytes = input.as_bytes();
    let digits_at = |at: usize| {
        bytes[at..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count()
    };

    // The date: one to three numbers, separated by `-` or `/`, the same separator throughout.
    let mut fields: Vec<&str> = Vec::new();
    let mut separator: Option<u8> = None;
    let mut pos = 0;
    loop {
        let run = digits_at(pos);
        if run == 0 || run > 9 {
            return None;
        }
        fields.push(&input[pos..pos + run]);
        pos += run;
        if fields.len() == 3 {
            break;
        }
        match bytes.get(pos) {
            Some(&sep) if (sep == b'-' || sep == b'/') && separator.is_none_or(|s| s == sep) => {
                separator = Some(sep);
                pos += 1;
            }
            _ => break,
        }
    }
    let iso_form = separator == Some(b'-') && fields.len() >= 2 && fields[0].len() == 4;
    let (year, month, day) = match fields.as_slice() {
        [year, month] if year.len() >= 3 => (number(year)?, number(month)?, 1.0),
        [first, second, third] if first.len() >= 3 => {
            (number(first)?, number(second)?, number(third)?)
        }
        [first, second, third] => {
            let year = two_digit_year(third)?;
            (year, number(first)?, number(second)?)
        }
        _ => return None,
    };
    if !(1.0..=12.0).contains(&month) || !(1.0..=31.0).contains(&day) {
        return None;
    }

    // The time, after a space, or after `T` for an ISO date.
    let mut time = 0.0;
    let mut zone = 0.0;
    let rest = &input[pos..];
    if !rest.is_empty() {
        let (has_time, body) = match rest.as_bytes()[0] {
            b' ' => (true, &rest[1..]),
            b'T' if iso_form
                && fields.len() == 3
                && fields[1].len() == 2
                && fields[2].len() == 2 =>
            {
                (true, &rest[1..])
            }
            _ => return None,
        };
        if !has_time {
            return None;
        }
        let (clock, rest_after_clock) = parse_clock(body)?;
        let (hour, minute, second, millis) = clock;
        if hour > 24.0
            || minute > 59.0
            || second > 59.0
            || (hour == 24.0 && (minute + second + millis) > 0.0)
        {
            return None;
        }
        time = hour * 3_600_000.0 + minute * 60_000.0 + second * 1000.0 + millis;
        zone = parse_zone(rest_after_clock)?;
    }

    let value = make_day(year, month - 1.0, day) * DAY_MS + time - zone * 60_000.0;
    let clipped = time_clip(value);
    (!clipped.is_nan()).then_some(clipped)
}

/// A number of a date field, read as its digits; `None` when it is too long for a year.
fn number(text: &str) -> Option<f64> {
    if text.len() > 9 {
        return None;
    }
    text.parse().ok()
}

/// The year of a field that has at most two or three digits: below 50 is 2000 and above, below 100
/// is 1900 and above; other values are the year as written.
fn two_digit_year(text: &str) -> Option<f64> {
    let value = number(text)?;
    if text.len() <= 3 && value < 100.0 {
        Some(if value < 50.0 {
            value + 2000.0
        } else {
            value + 1900.0
        })
    } else {
        Some(value)
    }
}

/// A bare number, as V8 reads it: four or more digits are a year; up to three digits are a month
/// of 2001 when they are 1 to 12, the year 2000 or 1900 plus the number for 32 to 99, and a year as
/// written from 100; 0 is 2000, and 13 to 31 are not a date.
fn lone_number(digits: &str) -> Option<Time> {
    let value = number(digits)?;
    let (year, month) = if digits.len() >= 4 {
        (value, 0.0)
    } else if value == 0.0 {
        (2000.0, 0.0)
    } else if value <= 12.0 {
        (2001.0, value - 1.0)
    } else if value <= 31.0 {
        return None;
    } else if value < 50.0 {
        (2000.0 + value, 0.0)
    } else if value < 100.0 {
        (1900.0 + value, 0.0)
    } else {
        (value, 0.0)
    };
    let time = time_clip(make_day(year, month, 1.0) * DAY_MS);
    (!time.is_nan()).then_some(time)
}

/// The clock of a legacy time: hours, minutes, seconds and milliseconds.
type Clock = (f64, f64, f64, f64);

/// `HH:mm[:ss[.fff…]]` at the start of `text`: the clock parts in ms, and the rest of the text.
fn parse_clock(text: &str) -> Option<(Clock, &str)> {
    let hour_digits = text.bytes().take_while(u8::is_ascii_digit).count();
    if !(1..=2).contains(&hour_digits) {
        return None;
    }
    let hour: f64 = text[..hour_digits].parse().ok()?;
    let rest = text[hour_digits..].strip_prefix(':')?;
    let minute = two_fixed(rest)?;
    let mut rest = &rest[2..];
    let mut second = 0.0;
    let mut millis = 0.0;
    if let Some(after) = rest.strip_prefix(':') {
        second = two_fixed(after)?;
        rest = &after[2..];
        if let Some(fraction) = rest.strip_prefix('.') {
            let count = fraction.bytes().take_while(u8::is_ascii_digit).count();
            if count == 0 {
                return None;
            }
            let padded: String = fraction[..count]
                .chars()
                .chain("000".chars())
                .take(3)
                .collect();
            millis = padded.parse().ok()?;
            rest = &fraction[count..];
        }
    }
    Some(((hour, minute, second, millis), rest))
}

fn two_fixed(text: &str) -> Option<f64> {
    let digits = text.get(..2)?;
    if digits.bytes().all(|b| b.is_ascii_digit()) {
        digits.parse().ok()
    } else {
        None
    }
}

/// The zone of the legacy form: nothing (UTC), `Z`, or `±HH:mm` and `±HHmm`, in minutes east of
/// UTC. `None` is a form the module does not read.
fn parse_zone(text: &str) -> Option<f64> {
    match text {
        "" | "Z" => Some(0.0),
        _ => {
            let sign = match text.as_bytes().first()? {
                b'+' => 1.0,
                b'-' => -1.0,
                _ => return None,
            };
            let body = &text[1..];
            let (hours, minutes) = match body.len() {
                4 if body.bytes().all(|b| b.is_ascii_digit()) => (&body[..2], &body[2..]),
                5 if body.as_bytes()[2] == b':' => (&body[..2], &body[3..]),
                _ => return None,
            };
            let hours: f64 = two_fixed(hours)?;
            let minutes: f64 = two_fixed(minutes)?;
            Some(sign * (hours * 60.0 + minutes))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: f64 = -30_270_326_400_000.0;

    #[test]
    fn civil_days_round_trip() {
        for days in [-719_162_i64, -1, 0, 1, 16_071, 2_932_896] {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days);
        }
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2014, 1, 6), 16_076);
    }

    #[test]
    fn time_clip_truncates_and_limits() {
        assert_eq!(time_clip(1.9), 1.0);
        assert!(time_clip(8.64e15 + 1.0).is_nan());
        assert_eq!(time_clip(-0.0).to_bits(), 0.0_f64.to_bits());
    }

    #[test]
    fn strict_parse_reads_full_dates() {
        assert_eq!(
            strict_parse("2014-01-06", "YYYY-MM-DD", NOW),
            Ok(Some(1_388_966_400_000.0))
        );
        assert_eq!(strict_parse("2014-1-6", "YYYY-MM-DD", NOW), Ok(None));
        assert_eq!(strict_parse("2024-02-30", "YYYY-MM-DD", NOW), Ok(None));
        assert_eq!(strict_parse("2014-01-06", "", NOW), Ok(None));
    }

    #[test]
    fn strict_parse_takes_missing_parts_from_the_clock() {
        // 1010-10-10T00:00Z: a day-only format takes the clock's month and year.
        assert_eq!(strict_parse("1", "D", NOW), Ok(Some(-30_271_104_000_000.0)));
        assert_eq!(
            strict_parse("17:30", "HH:mm", NOW),
            Ok(Some(-30_270_263_400_000.0))
        );
        assert_eq!(strict_parse("2014", "#", NOW), Ok(None));
    }

    #[test]
    fn strict_parse_reads_timestamps() {
        assert_eq!(strict_parse("-5", "x", NOW), Ok(Some(-5.0)));
        assert_eq!(strict_parse("120", "X", NOW), Ok(Some(120_000.0)));
        assert_eq!(strict_parse("1.5", "X", NOW), Ok(None));
    }

    #[test]
    fn format_follows_dayjs_tokens() {
        let t = 1_388_966_400_000.0;
        assert_eq!(format(t, "YYYY-MM-DD").unwrap_or_default(), "2014-01-06");
        assert_eq!(format(t, "dddd").unwrap_or_default(), "Monday");
        assert_eq!(format(t, "Do").unwrap_or_default(), "6th");
        assert_eq!(format(t, "x").unwrap_or_default(), "1388966400000");
        assert_eq!(
            format(t, "").unwrap_or_default(),
            "2014-01-06T00:00:00+00:00"
        );
        assert_eq!(format(t, "Date").unwrap_or_default(), "6amte");
    }

    #[test]
    fn week_tokens_throw_as_the_reference_does() {
        assert!(format(0.0, "w").is_err());
        assert!(format(0.0, "ww").is_err());
        assert!(format(0.0, "zzz").is_err());
    }

    #[test]
    fn add_moves_months_and_years_with_clamping() {
        let jan31 = 1_391_126_400_000.0; // 2014-01-31
        assert_eq!(
            format(add(jan31, 1.0, Unit::Month), "YYYY-MM-DD").unwrap_or_default(),
            "2014-02-28"
        );
        assert_eq!(
            format(add(jan31, 1.0, Unit::Year), "YYYY-MM-DD").unwrap_or_default(),
            "2015-01-31"
        );
        assert_eq!(
            format(add(jan31, 1.5, Unit::Day), "YYYY-MM-DD").unwrap_or_default(),
            "2014-02-02"
        );
        assert_eq!(add(0.0, 0.5, Unit::Hour), 1_800_000.0);
        assert!(add(f64::NAN, 1.0, Unit::Day).is_nan());
    }

    #[test]
    fn durations_follow_the_reference_pattern() {
        assert_eq!(parse_duration("3d"), Some((3.0, Unit::Day)));
        assert_eq!(parse_duration(" 5ms "), Some((5.0, Unit::Millisecond)));
        assert_eq!(parse_duration("1.5h"), Some((1.5, Unit::Hour)));
        assert_eq!(parse_duration("3D"), None);
        assert_eq!(parse_duration("-1d"), None);
        assert_eq!(parse_duration("1e3d"), None);
    }

    #[test]
    fn legacy_forms_follow_v8() {
        // The values are V8's `new Date(string).getTime()` under TZ=UTC (the exporter's oracle).
        let cases: &[(&str, f64)] = &[
            ("2014-01-06", 1_388_966_400_000.0),
            ("2014-01", 1_388_534_400_000.0),
            ("2014", 1_388_534_400_000.0),
            ("2014-02-30", 1_393_718_400_000.0),
            ("2014-13-01", f64::NAN),
            ("2014-01-32", f64::NAN),
            ("2014-1-1", 1_388_534_400_000.0),
            ("2019-09-1", 1_567_296_000_000.0),
            ("202-12-01", -55_763_856_000_000.0),
            ("202", -55_792_713_600_000.0),
            ("202304", 6_321_930_336_000_000.0),
            ("0", 946_684_800_000.0),
            ("1", 978_307_200_000.0),
            ("13", f64::NAN),
            ("7", 993_945_600_000.0),
            ("49", 2_493_072_000_000.0),
            ("50", -631_152_000_000.0),
            ("99", 915_148_800_000.0),
            ("100", -59_011_459_200_000.0),
            ("2014/01/06", 1_388_966_400_000.0),
            ("1/6/2014", 1_388_966_400_000.0),
            ("1/6/99", 915_580_800_000.0),
            ("1/6/49", 2_493_504_000_000.0),
            ("1/6/50", -630_720_000_000.0),
            ("1-6-14", 1_388_966_400_000.0),
            ("01/06/14", 1_388_966_400_000.0),
            ("007", 993_945_600_000.0),
            ("0000", -62_167_219_200_000.0),
            ("2014/13/01", f64::NAN),
            ("2014-01-06T00:00:00Z", 1_388_966_400_000.0),
            ("2014-01-06T10:00", 1_389_002_400_000.0),
            ("2014-01-06T10:00:00.123Z", 1_389_002_400_123.0),
            ("2014-01-06T10:00+02:00", 1_388_995_200_000.0),
            ("2014-01-06T10:00+0200", 1_388_995_200_000.0),
            ("2014-01-06 10:00", 1_389_002_400_000.0),
            ("2014-01-06 10:00:00.5", 1_389_002_400_500.0),
            ("2014-01-06T24:00", 1_389_052_800_000.0),
            ("2014-01-06T10:60", f64::NAN),
            ("2014-01-06T23:59:60", f64::NAN),
            ("2014-1-6T10:00", f64::NAN),
            ("2014-01-06T10:00+02", f64::NAN),
            ("foo", f64::NAN),
            ("", f64::NAN),
        ];
        for (text, expected) in cases {
            let got = legacy_parse(text);
            assert!(
                (got.is_nan() && expected.is_nan()) || got == *expected,
                "legacy_parse({text:?}) = {got}, expected {expected}"
            );
        }
    }
}
