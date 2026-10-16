//! The time formats of d3-time-format 4.1.0 in UTC with the default (en-US) locale: the directives
//! that `axisFormat` takes (`d3-time-format/src/locale.js`, `utcFormats`, `newFormat`).
//!
//! The reference formats in local time; in the port's UTC zone the local getters are the UTC
//! getters, so the UTC table gives the same text.

use super::time::{Base, DAY, civil_from_days, ms_of_day, year_of};

const DAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const SHORT_DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
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
const SHORT_MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
/// The en-US `dateTime`, `date` and `time` specifiers.
const DATE_TIME: &str = "%x, %X";
const DATE: &str = "%-m/%-d/%Y";
const TIME: &str = "%-I:%M:%S %p";

/// `timeFormat(specifier)` in UTC: the text of a time in the specifier's directives.
pub(crate) fn utc_format(specifier: &str, time: f64) -> String {
    let mut out = String::new();
    let chars: Vec<char> = specifier.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '%' {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        i += 1;
        let Some(&first) = chars.get(i) else {
            // A lone `%` at the end: the reference writes nothing for it.
            break;
        };
        // The padding modifier: `-` pads with nothing, `_` with spaces, `0` with zeros.
        let (fill, directive) = match first {
            '-' => (Some(""), chars.get(i + 1).copied()),
            '_' => (Some(" "), chars.get(i + 1).copied()),
            '0' => (Some("0"), chars.get(i + 1).copied()),
            other => (None, Some(other)),
        };
        if fill.is_some() {
            i += 1;
        }
        let Some(directive) = directive else {
            break;
        };
        i += 1;
        // `e` pads with spaces by default, every other directive with zeros.
        let fill = fill.unwrap_or(if directive == 'e' { " " } else { "0" });
        match directive_text(directive, time, fill) {
            Some(text) => out.push_str(&text),
            // An unknown directive writes its letter, and the `%` is dropped (`newFormat`).
            None => out.push(directive),
        }
    }
    out
}

/// The text of one directive, or `None` when the directive is not one of the table's.
fn directive_text(directive: char, time: f64, fill: &str) -> Option<String> {
    let f = Fields::of(time);
    let text = match directive {
        'a' => SHORT_DAYS[f.weekday].to_owned(),
        'A' => DAYS[f.weekday].to_owned(),
        'b' => SHORT_MONTHS[f.month].to_owned(),
        'B' => MONTHS[f.month].to_owned(),
        'c' => utc_format(DATE_TIME, time),
        'd' | 'e' => pad(f.day, fill, 2),
        'f' => format!("{}000", pad(f.millisecond, fill, 3)),
        'g' => pad(iso_year(time) % 100, fill, 2),
        'G' => pad(iso_year(time) % 10_000, fill, 4),
        'H' => pad(f.hour, fill, 2),
        'I' => {
            let hour12 = f.hour % 12;
            pad(if hour12 == 0 { 12 } else { hour12 }, fill, 2)
        }
        'j' => pad(
            1 + Base::Day.count(Base::Year.floor(time), time) as i64,
            fill,
            3,
        ),
        'L' => pad(f.millisecond, fill, 3),
        'm' => pad(f.month as i64 + 1, fill, 2),
        'M' => pad(f.minute, fill, 2),
        'p' => if f.hour >= 12 { "PM" } else { "AM" }.to_owned(),
        'q' => (1 + f.month / 3).to_string(),
        'Q' => format!("{}", time),
        's' => format!("{}", (time / 1_000.0).floor()),
        'S' => pad(f.second, fill, 2),
        'u' => (if f.weekday == 0 { 7 } else { f.weekday }).to_string(),
        'U' => pad(
            Base::Week(0).count(Base::Year.floor(time) - 1.0, time) as i64,
            fill,
            2,
        ),
        'V' => pad(iso_week(time), fill, 2),
        'w' => f.weekday.to_string(),
        'W' => pad(
            Base::Week(1).count(Base::Year.floor(time) - 1.0, time) as i64,
            fill,
            2,
        ),
        'x' => utc_format(DATE, time),
        'X' => utc_format(TIME, time),
        'y' => pad(f.year % 100, fill, 2),
        'Y' => pad(f.year % 10_000, fill, 4),
        'Z' => "+0000".to_owned(),
        '%' => "%".to_owned(),
        _ => return None,
    };
    Some(text)
}

/// The fields of a time that the directives read.
struct Fields {
    year: i64,
    month: usize,
    day: i64,
    weekday: usize,
    hour: i64,
    minute: i64,
    second: i64,
    millisecond: i64,
}

impl Fields {
    fn of(time: f64) -> Self {
        let days = (time / DAY).floor() as i64;
        let (year, month, day) = civil_from_days(days);
        let ms = ms_of_day(time) as i64;
        Self {
            year,
            month: month as usize,
            day,
            weekday: (days + 4).rem_euclid(7) as usize,
            hour: ms / 3_600_000,
            minute: ms / 60_000 % 60,
            second: ms / 1_000 % 60,
            millisecond: ms % 1_000,
        }
    }
}

/// `pad(value, fill, width)` of d3-time-format: the number with its sign, filled to `width`
/// characters with `fill` repeated (nothing for the `-` modifier).
fn pad(value: i64, fill: &str, width: usize) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let digits = value.unsigned_abs().to_string();
    let missing = width.saturating_sub(digits.len());
    format!("{sign}{}{digits}", fill.repeat(missing))
}

/// `dISO(d)`: the Thursday of the ISO week of a time (the week's Thursday, or the next Thursday for a
/// week that starts after it).
fn iso_thursday(time: f64) -> f64 {
    let weekday = Fields::of(time).weekday;
    if weekday >= 4 || weekday == 0 {
        Base::Week(4).floor(time)
    } else {
        Base::Week(4).ceil(time)
    }
}

/// `formatUTCYearISO`'s and `formatUTCFullYearISO`'s year: the year of the ISO week's Thursday.
fn iso_year(time: f64) -> i64 {
    year_of(iso_thursday(time))
}

/// `formatUTCWeekNumberISO`: the ISO week number of a time.
fn iso_week(time: f64) -> i64 {
    let thursday = iso_thursday(time);
    let year_start = Base::Year.floor(thursday);
    let starts_on_thursday = i64::from(Fields::of(year_start).weekday == 4);
    Base::Week(4).count(year_start, thursday) as i64 + starts_on_thursday
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2014-01-06T14:05:09.007Z, a Monday: 2014-01-01 plus five days and 14:05:09.007.
    const SAMPLE: f64 = 1_389_017_109_007.0;

    #[test]
    fn the_axis_directives_of_the_fixtures_format_a_time() {
        assert_eq!(utc_format("%Y-%m-%d", SAMPLE), "2014-01-06");
        assert_eq!(utc_format("%d/%m", SAMPLE), "06/01");
        assert_eq!(utc_format("%-d/%-m", SAMPLE), "6/1");
        assert_eq!(utc_format("%H:%M:%S.%L", SAMPLE), "14:05:09.007");
        assert_eq!(utc_format("%I %p", SAMPLE), "02 PM");
        assert_eq!(utc_format("%s", SAMPLE), "1389017109");
        assert_eq!(utc_format("%Hh%M", SAMPLE), "14h05");
    }

    #[test]
    fn a_literal_space_and_an_unknown_directive_are_written_as_they_are() {
        assert_eq!(utc_format(" %d", SAMPLE), " 06");
        assert_eq!(utc_format("%k", SAMPLE), "k");
    }

    #[test]
    fn the_week_and_day_of_year_directives_count_from_the_year_start() {
        // 2014-01-06 is day 6 of 2014 and in ISO week 2 (week 1 starts on Monday 2013-12-30).
        assert_eq!(utc_format("%j", SAMPLE), "006");
        assert_eq!(utc_format("%V", SAMPLE), "02");
        assert_eq!(utc_format("%u %w", SAMPLE), "1 1");
    }

    #[test]
    fn a_year_is_padded_to_four_digits() {
        assert_eq!(utc_format("%Y", 0.0), "1970");
        assert_eq!(utc_format("%y", 0.0), "70");
    }
}
