//! The dayjs 1.11.21 operations of `ganttRenderer.js`, in UTC (the port's zone, where the local
//! calendar is the UTC calendar): the day boundaries, `format('YYYY-MM-DD')`, `add` of days and
//! months, `diff(…, 'year')`, and the length of a duration unit (`dayjs.duration`).

use super::time::{DAY, civil_from_days, days_from_civil, ms_of_day, year_of};

/// `dayjs(time).startOf('day')`.
pub(crate) fn start_of_day(time: f64) -> f64 {
    time - ms_of_day(time)
}

/// `dayjs(time).endOf('day')`: the last millisecond of the day.
pub(crate) fn end_of_day(time: f64) -> f64 {
    start_of_day(time) + DAY - 1.0
}

/// `dayjs(time).add(n, 'd')`.
pub(crate) fn add_days(time: f64, n: f64) -> f64 {
    time + n * DAY
}

/// The calendar month, 0 to 11, and day of the month of a time.
fn month_and_day(time: f64) -> (i64, i64, i64) {
    civil_from_days((time / DAY).floor() as i64)
}

/// `dayjs(time).add(n, 'M')`: moves by `n` months, clamping the day to the length of the month, as
/// dayjs's `$set` does.
pub(crate) fn add_months(time: f64, n: f64) -> f64 {
    let (year, month, day) = month_and_day(time);
    let months = year * 12 + month + n as i64;
    let (year, month) = (months.div_euclid(12), months.rem_euclid(12));
    let length = days_from_civil(year, month + 1, 1) - days_from_civil(year, month, 1);
    let day = day.min(length);
    days_from_civil(year, month, day) as f64 * DAY + ms_of_day(time)
}

/// `monthDiff(a, b)` of dayjs's utilities (moment's algorithm): the months from `b` to `a`, with a
/// fraction for the part of a month.
fn month_diff(a: f64, b: f64) -> f64 {
    let (_, _, day_a) = month_and_day(a);
    let (_, _, day_b) = month_and_day(b);
    if day_a < day_b {
        return -month_diff(b, a);
    }
    let (year_a, month_a) = (year_of(a), month_and_day(a).1);
    let (year_b, month_b) = (year_of(b), month_and_day(b).1);
    let whole = (year_b - year_a) * 12 + (month_b - month_a);
    let whole = whole as f64;
    let anchor = add_months(a, whole);
    let early = b - anchor < 0.0;
    let anchor2 = add_months(a, whole + if early { -1.0 } else { 1.0 });
    let span = if early {
        anchor - anchor2
    } else {
        anchor2 - anchor
    };
    let result = -(whole + (b - anchor) / span);
    if result.is_nan() || result == 0.0 {
        0.0
    } else {
        result
    }
}

/// `dayjs(a).diff(dayjs(b), 'year')`: the whole years from `b` to `a`, truncated towards zero.
pub(crate) fn years_between(a: f64, b: f64) -> f64 {
    let years = month_diff(a, b) / 12.0;
    if years < 0.0 {
        let ceil = years.ceil();
        if ceil == 0.0 { 0.0 } else { ceil }
    } else {
        years.floor()
    }
}

/// `dayjs(time).format('YYYY-MM-DD')`: the year padded to four digits, then the month and the day.
pub(crate) fn format_date(time: f64) -> String {
    let (month, day) = (month_and_day(time).1, month_and_day(time).2);
    format!("{:04}-{:02}-{:02}", year_of(time), month + 1, day)
}

/// The length in milliseconds of one unit of dayjs's `duration`: `unit` is one of the unit names of
/// the tick interval regular expression. A month is a twelfth of 365 days, as dayjs takes it.
pub(crate) fn unit_milliseconds(unit: &str) -> Option<f64> {
    Some(match unit {
        "millisecond" => 1.0,
        "second" => 1_000.0,
        "minute" => 60_000.0,
        "hour" => 3_600_000.0,
        "day" => DAY,
        "week" => 7.0 * DAY,
        "month" => 365.0 * DAY / 12.0,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2014-01-31T10:00Z.
    const JAN_31: f64 = 1_391_162_400_000.0;

    #[test]
    fn a_month_is_clamped_to_its_length() {
        // 2014-01-31 plus one month is 2014-02-28, as dayjs gives.
        assert_eq!(format_date(add_months(JAN_31, 1.0)), "2014-02-28");
        assert_eq!(format_date(add_months(JAN_31, 12.0)), "2015-01-31");
        assert_eq!(
            add_months(JAN_31, 1.0) - start_of_day(add_months(JAN_31, 1.0)),
            10.0 * 3_600_000.0
        );
    }

    #[test]
    fn the_years_between_two_dates_are_truncated() {
        // Six whole years are 72 months; one month short of them is still five years.
        assert_eq!(years_between(add_months(JAN_31, 72.0), JAN_31), 6.0);
        assert_eq!(years_between(add_months(JAN_31, 71.0), JAN_31), 5.0);
        assert_eq!(years_between(JAN_31, add_months(JAN_31, 72.0)), -6.0);
    }

    #[test]
    fn the_date_is_formatted_with_a_padded_year() {
        assert_eq!(format_date(JAN_31), "2014-01-31");
        assert_eq!(format_date(0.0), "1970-01-01");
    }

    #[test]
    fn the_day_boundaries_are_midnights() {
        assert_eq!(start_of_day(JAN_31), 1_391_126_400_000.0);
        assert_eq!(end_of_day(JAN_31), 1_391_212_799_999.0);
    }
}
