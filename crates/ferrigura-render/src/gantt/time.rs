//! The time intervals and the ticks of d3-time 3.1.0, in UTC.
//!
//! The reference uses the local-time intervals (`timeDay`, `timeMonth`, ...). The port runs in UTC
//! (M6-PLAN, gantt time), where the local calendar is the UTC calendar, so the UTC definitions here
//! are the reference's. The source is `d3-time/src/{interval,ticks,millisecond,second,minute,hour,
//! day,week,month,year,duration}.js` (version 3.1.0, `node_modules/.pnpm`).
//!
//! Times are epoch milliseconds as JavaScript numbers: integral `f64` values, and NaN for an Invalid
//! Date. Calendar fields are read with the Gregorian calendar of JavaScript's `Date`.

/// `durationDay` in milliseconds.
pub(crate) const DAY: f64 = 86_400_000.0;
/// `durationWeek` in milliseconds.
pub(crate) const WEEK: f64 = 7.0 * DAY;
/// `durationYear` in milliseconds: 365 days, as d3-time takes it.
pub(crate) const YEAR: f64 = 365.0 * DAY;

/// The units of d3-time's base intervals. `Week(d)` is the weekday `d` (0 is Sunday).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Base {
    Millisecond,
    Second,
    Minute,
    Hour,
    Day,
    Week(u8),
    Month,
    Year,
}

/// The calendar fields of a time, read as `Date`'s UTC getters read them.
struct Fields {
    year: i64,
    /// The month, 0 to 11.
    month: i64,
    /// The day of the month, 1 to 31.
    day: i64,
    /// The weekday, 0 (Sunday) to 6.
    weekday: i64,
}

fn fields(time: f64) -> Fields {
    let days = (time / DAY).floor() as i64;
    let (year, month, day) = civil_from_days(days);
    Fields {
        year,
        month,
        day,
        weekday: (days + 4).rem_euclid(7),
    }
}

/// The date of a day number, counted from 1970-01-01: the year, the month (0 to 11) and the day.
pub(crate) fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month - 1, day)
}

/// The day number of a date, as `Date.UTC` takes it: the month may lie outside 0 to 11 and the day
/// outside its month, and both roll over.
pub(crate) fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year + month.div_euclid(12);
    let month = month.rem_euclid(12) + 1;
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The year of a time, as `getUTCFullYear` gives it.
pub(crate) fn year_of(time: f64) -> i64 {
    fields(time).year
}

/// The milliseconds of a time since its UTC midnight.
pub(crate) fn ms_of_day(time: f64) -> f64 {
    time.rem_euclid(DAY)
}

/// `floor` to a multiple of `unit` counted from the epoch, for the units that divide a day.
fn floor_to(time: f64, unit: f64) -> f64 {
    time - time.rem_euclid(unit)
}

impl Base {
    /// The start of the unit that contains `time` (the interval's `floor`).
    pub(crate) fn floor(self, time: f64) -> f64 {
        match self {
            Self::Millisecond => time,
            Self::Second => floor_to(time, 1_000.0),
            Self::Minute => floor_to(time, 60_000.0),
            Self::Hour => floor_to(time, 3_600_000.0),
            Self::Day => floor_to(time, DAY),
            Self::Week(weekday) => {
                let midnight = floor_to(time, DAY);
                let today = fields(midnight).weekday;
                midnight - ((today + 7 - i64::from(weekday)) % 7) as f64 * DAY
            }
            Self::Month => {
                let f = fields(time);
                days_from_civil(f.year, f.month, 1) as f64 * DAY
            }
            Self::Year => days_from_civil(year_of(time), 0, 1) as f64 * DAY,
        }
    }

    /// `offset(time, n)` of the unit, without the filters of [`Interval`]: the calendar moves by `n`
    /// units and keeps the day of the month and the time of day, as `setMonth` and `setFullYear`
    /// do.
    pub(crate) fn offset(self, time: f64, n: f64) -> f64 {
        match self {
            Self::Millisecond => time + n,
            Self::Second => time + n * 1_000.0,
            Self::Minute => time + n * 60_000.0,
            Self::Hour => time + n * 3_600_000.0,
            Self::Day => time + n * DAY,
            Self::Week(_) => time + n * WEEK,
            Self::Month => {
                let f = fields(time);
                let months = f.year * 12 + f.month + n as i64;
                let (year, month) = (months.div_euclid(12), months.rem_euclid(12));
                days_from_civil(year, month, f.day) as f64 * DAY + ms_of_day(time)
            }
            Self::Year => {
                let f = fields(time);
                days_from_civil(f.year + n as i64, f.month, f.day) as f64 * DAY + ms_of_day(time)
            }
        }
    }

    /// `count(start, end)`: the whole units from the start of the unit of `start` to that of
    /// `end` (both floored first, and the result floored), as d3-time counts them in UTC.
    pub(crate) fn count(self, start: f64, end: f64) -> f64 {
        let (a, b) = (self.floor(start), self.floor(end));
        let units = match self {
            Self::Millisecond => b - a,
            Self::Second => (b - a) / 1_000.0,
            Self::Minute => (b - a) / 60_000.0,
            Self::Hour => (b - a) / 3_600_000.0,
            Self::Day => (b - a) / DAY,
            Self::Week(_) => (b - a) / WEEK,
            Self::Month => {
                let (fa, fb) = (fields(a), fields(b));
                ((fb.year * 12 + fb.month) - (fa.year * 12 + fa.month)) as f64
            }
            Self::Year => (fields(b).year - fields(a).year) as f64,
        };
        units.floor()
    }

    /// `ceil`: the first start of the unit at or after `time`.
    pub(crate) fn ceil(self, time: f64) -> f64 {
        let floored = self.floor(time - 1.0);
        let next = self.offset(floored, 1.0);
        self.floor(next)
    }

    /// The field that `every` filters on: `getUTCSeconds` and the others, `None` for the units that
    /// count from the epoch instead (milliseconds and weeks).
    fn field(self, time: f64) -> Option<i64> {
        match self {
            Self::Second => Some(((time / 1_000.0).floor() as i64).rem_euclid(60)),
            Self::Minute => Some(((time / 60_000.0).floor() as i64).rem_euclid(60)),
            Self::Hour => Some(((time / 3_600_000.0).floor() as i64).rem_euclid(24)),
            Self::Day => Some(fields(time).day - 1),
            Self::Month => Some(fields(time).month),
            Self::Year => Some(fields(time).year),
            Self::Millisecond | Self::Week(_) => None,
        }
    }
}

/// An interval of d3-time: a base unit, or one of the forms `every` makes from it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Interval {
    /// The unit itself.
    Base(Base),
    /// `millisecond.every(k)` for `k > 1`: the instants that are multiples of `k` milliseconds.
    MillisecondEvery(f64),
    /// `year.every(k)`: the years that are multiples of `k`, from January 1 at midnight.
    YearEvery(f64),
    /// `unit.every(k)` for `k > 1`: the instants where the field (or, for weeks, the count from the
    /// epoch) is a multiple of `k`.
    Filtered(Base, f64),
}

impl Interval {
    /// The interval's `floor`.
    pub(crate) fn floor(self, time: f64) -> f64 {
        match self {
            Self::Base(base) => base.floor(time),
            Self::MillisecondEvery(k) => (time / k).floor() * k,
            Self::YearEvery(k) => {
                let year = (year_of(time) as f64 / k).floor() * k;
                days_from_civil(year as i64, 0, 1) as f64 * DAY
            }
            Self::Filtered(base, _) => {
                let mut date = time;
                loop {
                    date = base.floor(date);
                    if self.test(date) {
                        return date;
                    }
                    date -= 1.0;
                }
            }
        }
    }

    /// The interval's `offseti`: moves by `n` units from `time`, for a filter by `n` matching
    /// instants.
    fn offset_raw(self, time: f64, n: f64) -> f64 {
        match self {
            Self::Base(base) => base.offset(time, n),
            Self::MillisecondEvery(k) => time + n * k,
            Self::YearEvery(k) => Base::Year.offset(time, n * k),
            Self::Filtered(base, _) => {
                let mut date = time;
                if n >= 0.0 {
                    for _ in 0..(n as i64) {
                        loop {
                            date = base.offset(date, 1.0);
                            if self.test(date) {
                                break;
                            }
                        }
                    }
                } else {
                    for _ in 0..((-n) as i64) {
                        loop {
                            date = base.offset(date, -1.0);
                            if self.test(date) {
                                break;
                            }
                        }
                    }
                }
                date
            }
        }
    }

    /// Whether a floored instant belongs to a filtered interval.
    fn test(self, time: f64) -> bool {
        match self {
            Self::Filtered(base, step) => match base.field(time) {
                Some(field) => (field as f64) % step == 0.0,
                None => base.count(0.0, time) % step == 0.0,
            },
            _ => true,
        }
    }

    /// `ceil`: the first instant at or after `time` that starts an interval.
    fn ceil(self, time: f64) -> f64 {
        let floored = self.floor(time - 1.0);
        let offset = self.offset_raw(floored, 1.0);
        self.floor(offset)
    }

    /// `range(start, stop)`: the instants from the first start at or after `start` that are before
    /// `stop`, one per interval (d3-time's `interval.range` with step 1).
    pub(crate) fn range(self, start: f64, stop: f64) -> Vec<f64> {
        let mut out = Vec::new();
        let mut start = self.ceil(start);
        if start.is_nan() || stop.is_nan() || start >= stop {
            return out;
        }
        loop {
            let previous = start;
            out.push(previous);
            start = self.offset_raw(start, 1.0);
            start = self.floor(start);
            if !(previous < start && start < stop) {
                break;
            }
        }
        out
    }

    /// `every(k)` of the unit: `None` when `k` is not a positive number after flooring.
    pub(crate) fn every(base: Base, k: f64) -> Option<Self> {
        let k = k.floor();
        if !k.is_finite() || k <= 0.0 {
            return None;
        }
        Some(match base {
            Base::Millisecond if k > 1.0 => Self::MillisecondEvery(k),
            Base::Millisecond => Self::Base(Base::Millisecond),
            Base::Year => Self::YearEvery(k),
            _ if k > 1.0 => Self::Filtered(base, k),
            _ => Self::Base(base),
        })
    }
}

/// The table of tick intervals of d3-time (`ticks.js:14-33`): the unit, the step and the duration
/// the step covers, in ascending order of duration.
const TICK_INTERVALS: [(Base, f64, f64); 18] = [
    (Base::Second, 1.0, 1_000.0),
    (Base::Second, 5.0, 5_000.0),
    (Base::Second, 15.0, 15_000.0),
    (Base::Second, 30.0, 30_000.0),
    (Base::Minute, 1.0, 60_000.0),
    (Base::Minute, 5.0, 300_000.0),
    (Base::Minute, 15.0, 900_000.0),
    (Base::Minute, 30.0, 1_800_000.0),
    (Base::Hour, 1.0, 3_600_000.0),
    (Base::Hour, 3.0, 10_800_000.0),
    (Base::Hour, 6.0, 21_600_000.0),
    (Base::Hour, 12.0, 43_200_000.0),
    (Base::Day, 1.0, DAY),
    (Base::Day, 2.0, 2.0 * DAY),
    (Base::Week(0), 1.0, WEEK),
    (Base::Month, 1.0, 30.0 * DAY),
    (Base::Month, 3.0, 90.0 * DAY),
    (Base::Year, 1.0, YEAR),
];

/// How a scale's ticks are chosen: a desired count, or an interval.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum TickCount {
    /// About this many ticks (d3's default is 10).
    Count(f64),
    /// The instants of an interval.
    Interval(Interval),
}

/// `ticks(start, stop, count)` of d3-time (`ticks.js:24-31`): the ticks between the two times, in
/// ascending order, or descending when `stop` is before `start`.
pub(crate) fn ticks(start: f64, stop: f64, count: TickCount) -> Vec<f64> {
    let reverse = stop < start;
    let (start, stop) = if reverse {
        (stop, start)
    } else {
        (start, stop)
    };
    let interval = match count {
        TickCount::Interval(interval) => Some(interval),
        TickCount::Count(count) => tick_interval(start, stop, count),
    };
    let mut out = interval.map_or_else(Vec::new, |interval| interval.range(start, stop + 1.0));
    if reverse {
        out.reverse();
    }
    out
}

/// `tickInterval(start, stop, count)` of d3-time (`ticks.js:33-42`): the interval whose duration is
/// nearest to the target spacing, or a millisecond or year interval outside the table.
fn tick_interval(start: f64, stop: f64, count: f64) -> Option<Interval> {
    let target = (stop - start).abs() / count;
    // bisector(step).right: the first entry whose duration is greater than the target.
    let index = TICK_INTERVALS.partition_point(|&(_, _, duration)| duration <= target);
    if index == TICK_INTERVALS.len() {
        return Interval::every(Base::Year, tick_step(start / YEAR, stop / YEAR, count));
    }
    if index == 0 {
        let step = tick_step(start, stop, count);
        let step = if step.is_nan() { step } else { step.max(1.0) };
        return Interval::every(Base::Millisecond, step);
    }
    let (_, _, lower) = TICK_INTERVALS[index - 1];
    let (_, _, upper) = TICK_INTERVALS[index];
    let chosen = if target / lower < upper / target {
        index - 1
    } else {
        index
    };
    let (base, step, _) = TICK_INTERVALS[chosen];
    Interval::every(base, step)
}

/// `tickStep(start, stop, count)` of d3-array: a step of 1, 2 or 5 times a power of ten.
fn tick_step(start: f64, stop: f64, count: f64) -> f64 {
    let step0 = (stop - start).abs()
        / if count.is_nan() {
            f64::NAN
        } else {
            count.max(0.0)
        };
    let mut step1 = 10f64.powf((step0.ln() / std::f64::consts::LN_10).floor());
    let error = step0 / step1;
    if error >= 50f64.sqrt() {
        step1 *= 10.0;
    } else if error >= 10f64.sqrt() {
        step1 *= 5.0;
    } else if error >= 2f64.sqrt() {
        step1 *= 2.0;
    }
    if stop < start { -step1 } else { step1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2014-01-01T00:00Z, a Wednesday.
    const JAN_1_2014: f64 = 1_388_534_400_000.0;

    #[test]
    fn the_calendar_matches_the_js_date() {
        assert_eq!(civil_from_days(0), (1970, 0, 1));
        assert_eq!(fields(JAN_1_2014).weekday, 3);
        assert_eq!(
            days_from_civil(2024, 0, 29),
            days_from_civil(2024, 1, 1) - 3
        );
        // Month and day overflow roll over, as Date.UTC takes them.
        assert_eq!(days_from_civil(2014, 0, 32), days_from_civil(2014, 1, 1));
        assert_eq!(days_from_civil(2014, 12, 1), days_from_civil(2015, 0, 1));
    }

    #[test]
    fn sunday_weeks_start_on_sunday() {
        // 2014-01-01 is a Wednesday; its week starts on Sunday 2013-12-29.
        let start = Base::Week(0).floor(JAN_1_2014);
        assert_eq!(fields(start).weekday, 0);
        assert_eq!(start, JAN_1_2014 - 3.0 * DAY);
    }

    #[test]
    fn the_day_ticks_of_one_day_are_midnights() {
        let ticks = ticks(
            JAN_1_2014,
            JAN_1_2014 + 3.0 * DAY,
            TickCount::Interval(Interval::Base(Base::Day)),
        );
        assert_eq!(
            ticks,
            vec![
                JAN_1_2014,
                JAN_1_2014 + DAY,
                JAN_1_2014 + 2.0 * DAY,
                JAN_1_2014 + 3.0 * DAY
            ]
        );
    }

    #[test]
    fn every_second_day_counts_the_day_of_the_month() {
        let interval = Interval::every(Base::Day, 2.0).expect("an interval");
        let ticks = interval.range(JAN_1_2014, JAN_1_2014 + 4.0 * DAY);
        // The days whose day of the month minus one is even: January 1, 3, and so on.
        assert_eq!(ticks, vec![JAN_1_2014, JAN_1_2014 + 2.0 * DAY]);
    }

    #[test]
    fn default_ticks_pick_a_day_for_a_ten_day_span() {
        let ticks = ticks(JAN_1_2014, JAN_1_2014 + 10.0 * DAY, TickCount::Count(10.0));
        assert_eq!(ticks.len(), 11);
        assert_eq!(ticks[1] - ticks[0], DAY);
    }

    #[test]
    fn reversed_ticks_come_back_descending() {
        let forward = ticks(JAN_1_2014, JAN_1_2014 + 2.0 * DAY, TickCount::Count(10.0));
        let backward = ticks(JAN_1_2014 + 2.0 * DAY, JAN_1_2014, TickCount::Count(10.0));
        let mut reversed = forward.clone();
        reversed.reverse();
        assert_eq!(backward, reversed);
    }

    #[test]
    fn tick_steps_are_one_two_or_five_powers_of_ten() {
        assert_eq!(tick_step(0.0, 100.0, 10.0), 10.0);
        assert_eq!(tick_step(0.0, 35.0, 10.0), 5.0);
        assert_eq!(tick_step(0.0, 17.0, 10.0), 2.0);
        assert_eq!(tick_step(10.0, 0.0, 10.0), -1.0);
    }
}
