//! Safe time functions.
use std::ops::Add;
use std::time::{Duration, SystemTime};

fn is_leap_year(year: i64) -> bool {
    if year % 400 == 0 {
        true
    } else if year % 100 == 0 {
        false
    } else {
        year % 4 == 0
    }
}

fn year_len_days(year: i64) -> i64 {
    if is_leap_year(year) {
        366
    } else {
        365
    }
}

#[allow(clippy::missing_panics_doc)]
#[allow(clippy::match_same_arms)]
#[must_use]
pub fn month_len_days(year: i64, month: i64) -> i64 {
    match month {
        1 => 31,
        2 if (year % 400) == 0 => 29,
        2 if (year % 100) == 0 => 28,
        2 if (year % 4) == 0 => 29,
        2 => 28,
        3 => 31,
        4 => 30,
        5 => 31,
        6 => 30,
        7 => 31,
        8 => 31,
        9 => 30,
        10 => 31,
        11 => 30,
        12 => 31,
        _ => unimplemented!(),
    }
}

struct DateTime {
    pub year: i64,
    pub month: i64,
    pub day: i64,
    pub hour: i64,
    pub min: i64,
    pub sec: i64,
}
impl DateTime {
    // Epoch time assumes that every day is the same length, 24 * 60 * 60 seconds.
    // It ignores leap seconds.
    #[must_use]
    pub fn new(epoch_seconds: i64) -> Self {
        let mut dt = Self {
            year: 1970,
            month: 1,
            day: 1,
            hour: 0,
            min: 0,
            sec: epoch_seconds,
        };
        dt.balance();
        dt
    }

    fn balance_month(&mut self) {
        let delta_years = if self.month > 12 {
            0 - (self.month - 1) / 12
        } else {
            return;
        };
        //dbg!(delta_years);
        //dbg!(self.year);
        self.year -= delta_years;
        //dbg!(self.year);
        //dbg!(self.month);
        self.month += 12 * delta_years;
        //dbg!(self.month);
        assert!((1..=12).contains(&self.month));
    }

    fn balance_day(&mut self) {
        self.balance_month();
        while self.day > 366 {
            //dbg!(self.day);
            self.day -= year_len_days(self.year);
            //dbg!(self.day);
            //dbg!(self.year);
            self.year += 1;
            //dbg!(self.year);
        }
        while self.day > month_len_days(self.year, self.month) {
            //dbg!(self.day);
            self.day -= month_len_days(self.year, self.month);
            //dbg!(self.day);
            //dbg!(self.month);
            self.month += 1;
            //dbg!(self.month);
            self.balance_month();
        }
    }

    fn balance_hour(&mut self) {
        let delta_days = if self.hour > 23 {
            0 - self.hour / 24
        } else {
            self.balance_day();
            return;
        };
        //dbg!(delta_days);
        //dbg!(self.day);
        self.day -= delta_days;
        //dbg!(self.day);
        //dbg!(self.hour);
        self.hour += 24 * delta_days;
        //dbg!(self.hour);
        assert!((0..24).contains(&self.hour));
        self.balance_day();
    }

    fn balance_min(&mut self) {
        let delta_hours = if self.min > 59 {
            0 - self.min / 60
        } else {
            self.balance_hour();
            return;
        };
        //dbg!(delta_hours);
        //dbg!(self.hour);
        self.hour -= delta_hours;
        //dbg!(self.hour);
        //dbg!(self.min);
        self.min += 60 * delta_hours;
        //dbg!(self.min);
        assert!((0..60).contains(&self.min));
        self.balance_hour();
    }

    pub fn balance(&mut self) {
        let delta_mins = if self.sec > 59 {
            0 - self.sec / 60
        } else {
            self.balance_min();
            return;
        };
        //dbg!(delta_mins);
        //dbg!(self.min);
        self.min -= delta_mins;
        //dbg!(self.min);
        //dbg!(self.sec);
        self.sec += 60 * delta_mins;
        //dbg!(self.sec);
        assert!((0..60).contains(&self.sec));
        self.balance_min();
    }
}
impl Add<Duration> for DateTime {
    type Output = DateTime;

    fn add(mut self, rhs: Duration) -> Self::Output {
        self.sec += i64::try_from(rhs.as_secs()).unwrap();
        self.balance();
        self
    }
}

#[allow(clippy::module_name_repetitions)]
pub trait FormatTime {
    fn iso8601_utc(&self) -> String;
}

impl FormatTime for SystemTime {
    fn iso8601_utc(&self) -> String {
        let epoch_seconds = i64::try_from(
            self.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        )
        .unwrap();
        let dt = DateTime::new(epoch_seconds);
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            dt.year, dt.month, dt.day, dt.hour, dt.min, dt.sec
        )
    }
}

#[allow(clippy::unreadable_literal)]
#[cfg(test)]
mod tests {
    use super::{DateTime, FormatTime};
    use std::time::{Duration, SystemTime};
    const MIN: u64 = 60;
    const HOUR: u64 = 60 * MIN;
    const DAY: u64 = 24 * HOUR;

    #[test]
    fn date_time_new() {
        for (expected, epoch_seconds) in [
            ((1970, 1, 1, 0, 0, 0), 0),
            ((1970, 1, 1, 0, 0, 1), 1),
            ((1970, 1, 1, 0, 0, 59), 59),
            ((1970, 1, 1, 0, 1, 0), 60),
            ((1970, 1, 1, 0, 1, 1), 61),
            ((1970, 1, 1, 0, 59, 0), 59 * 60),
            ((1970, 1, 1, 1, 0, 0), 60 * 60),
            ((1970, 1, 1, 23, 0, 0), 23 * 60 * 60),
            ((1970, 1, 1, 23, 59, 59), 86400 - 1),
            ((1970, 1, 2, 0, 0, 0), 86400),
            ((1970, 1, 2, 0, 0, 1), 86400 + 1),
            ((1970, 1, 31, 23, 59, 59), 31 * 86400 - 1),
            ((1970, 2, 1, 0, 0, 0), 31 * 86400),
            ((1970, 3, 1, 0, 0, 0), 59 * 86400),
            ((1970, 12, 31, 23, 59, 59), 31535999),
            ((1971, 1, 1, 0, 0, 0), 31536000),
            ((1972, 6, 30, 23, 59, 59), 78796799),
            ((1972, 7, 1, 0, 0, 0), 78796800),
            ((2022, 3, 30, 7, 29, 33), 1648625373),
            ((2022, 3, 30, 7, 29, 33), 1648625373),
            ((2100, 2, 28, 23, 59, 59), 4107542399),
            ((2100, 3, 1, 0, 0, 0), 4107542400),
        ] {
            let dt = DateTime::new(epoch_seconds);
            assert_eq!(
                expected,
                (dt.year, dt.month, dt.day, dt.hour, dt.min, dt.sec)
            );
        }
    }

    #[test]
    fn date_time_add() {
        for (initial, seconds_to_add, expected) in [
            ((1970, 1, 1, 0, 0, 0), 0, (1970, 1, 1, 0, 0, 0)),
            ((1970, 1, 1, 0, 0, 0), 1, (1970, 1, 1, 0, 0, 1)),
            ((2004, 2, 28, 23, 59, 59), 1, (2004, 2, 29, 0, 0, 0)),
            ((2100, 2, 28, 23, 59, 59), 1, (2100, 3, 1, 0, 0, 0)),
            ((2000, 2, 28, 23, 59, 59), 1, (2000, 2, 29, 0, 0, 0)),
            ((2004, 2, 28, 0, 0, 0), 365 * DAY, (2005, 2, 27, 0, 0, 0)),
            ((2100, 2, 28, 0, 0, 0), 365 * DAY, (2101, 2, 28, 0, 0, 0)),
            ((2000, 2, 28, 0, 0, 0), 365 * DAY, (2001, 2, 27, 0, 0, 0)),
            ((2004, 2, 28, 0, 0, 0), 366 * DAY, (2005, 2, 28, 0, 0, 0)),
            ((2100, 2, 28, 0, 0, 0), 366 * DAY, (2101, 3, 1, 0, 0, 0)),
            ((2000, 2, 28, 0, 0, 0), 366 * DAY, (2001, 2, 28, 0, 0, 0)),
            ((2004, 2, 28, 0, 0, 0), 367 * DAY, (2005, 3, 1, 0, 0, 0)),
            ((2100, 2, 28, 0, 0, 0), 367 * DAY, (2101, 3, 2, 0, 0, 0)),
            ((2000, 2, 28, 0, 0, 0), 367 * DAY, (2001, 3, 1, 0, 0, 0)),
            ((2004, 2, 28, 0, 0, 0), 1462 * DAY, (2008, 2, 29, 0, 0, 0)),
            ((2100, 2, 28, 0, 0, 0), 1462 * DAY, (2104, 3, 1, 0, 0, 0)),
            ((2000, 2, 28, 0, 0, 0), 1462 * DAY, (2004, 2, 29, 0, 0, 0)),
        ] {
            let dt = DateTime {
                year: initial.0,
                month: initial.1,
                day: initial.2,
                hour: initial.3,
                min: initial.4,
                sec: initial.5,
            } + Duration::from_secs(seconds_to_add);
            assert_eq!(
                expected,
                (dt.year, dt.month, dt.day, dt.hour, dt.min, dt.sec),
                "{:?} + {}",
                initial,
                seconds_to_add,
            );
        }
    }

    #[test]
    fn test_iso8601_utc() {
        for (expected, epoch_seconds) in [
            ("1970-01-01T00:00:00Z", 0),
            ("2022-03-30T07:29:33Z", 1648625373),
            ("2100-02-28T23:59:59Z", 4107542399),
        ] {
            assert_eq!(
                expected,
                (SystemTime::UNIX_EPOCH + Duration::from_secs(epoch_seconds)).iso8601_utc()
            );
        }
    }
}
