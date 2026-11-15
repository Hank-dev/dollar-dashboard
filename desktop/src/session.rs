//! US equity session clock (NYSE regular hours) without a timezone database.
//! Holiday and early-close tables cover 2026 and 2027; later years fall back to weekdays.

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, TimeZone, Utc, Weekday};

const HOLIDAYS: &[(i32, u32, u32)] = &[
    (2026, 1, 1),
    (2026, 1, 19),
    (2026, 2, 16),
    (2026, 4, 3),
    (2026, 5, 25),
    (2026, 6, 19),
    (2026, 7, 3),
    (2026, 9, 7),
    (2026, 11, 26),
    (2026, 12, 25),
    (2027, 1, 1),
    (2027, 1, 18),
    (2027, 2, 15),
    (2027, 3, 26),
    (2027, 5, 31),
    (2027, 6, 18),
    (2027, 7, 5),
    (2027, 9, 6),
    (2027, 11, 25),
    (2027, 12, 24),
];

/// Sessions that close at 13:00 New York time.
const EARLY_CLOSES: &[(i32, u32, u32)] = &[(2026, 11, 27), (2026, 12, 24), (2027, 11, 26)];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Session {
    pub open: bool,
    /// Next open when closed, today's close when open.
    pub next_change: i64,
}

pub fn us_equities(now: i64) -> Session {
    let ny = ny_date_time(now);
    let today = ny.date();
    if trading_day(today) {
        let open = at_ny(today, 9, 30);
        let close = at_ny(today, if early_close(today) { 13 } else { 16 }, 0);
        if now < open {
            return Session {
                open: false,
                next_change: open,
            };
        }
        if now < close {
            return Session {
                open: true,
                next_change: close,
            };
        }
    }
    let mut day = today + Duration::days(1);
    while !trading_day(day) {
        day += Duration::days(1);
    }
    Session {
        open: false,
        next_change: at_ny(day, 9, 30),
    }
}

pub fn trading_day(date: NaiveDate) -> bool {
    !matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
        && !HOLIDAYS.contains(&(date.year(), date.month(), date.day()))
}

fn early_close(date: NaiveDate) -> bool {
    EARLY_CLOSES.contains(&(date.year(), date.month(), date.day()))
}

/// New York is UTC−4 from the second Sunday of March (07:00 UTC) to the
/// first Sunday of November (06:00 UTC), UTC−5 otherwise.
pub fn ny_offset_hours(now: i64) -> i64 {
    let Some(utc) = DateTime::<Utc>::from_timestamp(now, 0) else {
        return -5;
    };
    let year = utc.year();
    let start = nth_sunday(year, 3, 2).map(|d| utc_at(d, 7));
    let end = nth_sunday(year, 11, 1).map(|d| utc_at(d, 6));
    match (start, end) {
        (Some(start), Some(end)) if now >= start && now < end => -4,
        _ => -5,
    }
}

fn ny_date_time(now: i64) -> chrono::NaiveDateTime {
    DateTime::<Utc>::from_timestamp(now + ny_offset_hours(now) * 3600, 0)
        .unwrap_or_default()
        .naive_utc()
}

/// Unix time of a New York wall-clock time on `date`.
fn at_ny(date: NaiveDate, hour: u32, minute: u32) -> i64 {
    let naive = date.and_time(NaiveTime::from_hms_opt(hour, minute, 0).unwrap_or_default());
    let guess = Utc.from_utc_datetime(&naive).timestamp();
    // The offset at the guessed instant is right except within hours of a DST switch.
    guess - ny_offset_hours(guess + 5 * 3600) * 3600
}

fn nth_sunday(year: i32, month: u32, n: u32) -> Option<NaiveDate> {
    let first = NaiveDate::from_ymd_opt(year, month, 1)?;
    let offset = (7 - first.weekday().num_days_from_sunday()) % 7;
    first.checked_add_signed(Duration::days((offset + 7 * (n - 1)) as i64))
}

fn utc_at(date: NaiveDate, hour: u32) -> i64 {
    Utc.from_utc_datetime(&date.and_time(NaiveTime::from_hms_opt(hour, 0, 0).unwrap_or_default()))
        .timestamp()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(y: i32, m: u32, d: u32, h: u32, min: u32) -> i64 {
        Utc.with_ymd_and_hms(y, m, d, h, min, 0).unwrap().timestamp()
    }

    #[test]
    fn holidays_fall_on_weekdays() {
        for (y, m, d) in HOLIDAYS.iter().chain(EARLY_CLOSES) {
            let date = NaiveDate::from_ymd_opt(*y, *m, *d).unwrap();
            assert!(
                !matches!(date.weekday(), Weekday::Sat | Weekday::Sun),
                "{date}"
            );
        }
    }

    #[test]
    fn dst_switches_on_the_us_schedule() {
        assert_eq!(ny_offset_hours(utc(2026, 3, 8, 6, 59)), -5);
        assert_eq!(ny_offset_hours(utc(2026, 3, 8, 7, 0)), -4);
        assert_eq!(ny_offset_hours(utc(2026, 11, 1, 5, 59)), -4);
        assert_eq!(ny_offset_hours(utc(2026, 11, 1, 6, 0)), -5);
    }

    #[test]
    fn regular_session_opens_and_closes_in_new_york_time() {
        // Friday 16 Oct 2026, EDT: 09:30–16:00 New York is 13:30–20:00 UTC.
        let before = us_equities(utc(2026, 10, 16, 13, 0));
        assert!(!before.open);
        assert_eq!(before.next_change, utc(2026, 10, 16, 13, 30));
        let during = us_equities(utc(2026, 10, 16, 15, 0));
        assert!(during.open);
        assert_eq!(during.next_change, utc(2026, 10, 16, 20, 0));
        // Saturday reopens Monday 19 Oct.
        let weekend = us_equities(utc(2026, 10, 17, 9, 0));
        assert!(!weekend.open);
        assert_eq!(weekend.next_change, utc(2026, 10, 19, 13, 30));
    }

    #[test]
    fn holidays_and_early_closes_change_the_schedule() {
        // Thanksgiving is closed; Friday after closes at 13:00 EST (18:00 UTC).
        let thanksgiving = us_equities(utc(2026, 11, 26, 16, 0));
        assert!(!thanksgiving.open);
        assert_eq!(thanksgiving.next_change, utc(2026, 11, 27, 14, 30));
        let black_friday = us_equities(utc(2026, 11, 27, 15, 0));
        assert!(black_friday.open);
        assert_eq!(black_friday.next_change, utc(2026, 11, 27, 18, 0));
    }
}
