//! Safe time functions.
use std::ops::Add;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

#[allow(clippy::module_name_repetitions)]
pub struct DateTime {
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
            (self.month - 1) / 12
        } else {
            return;
        };
        //dbg!(delta_years);
        //dbg!(self.year);
        self.year += delta_years;
        //dbg!(self.year);
        //dbg!(self.month);
        self.month -= 12 * delta_years;
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
            self.hour / 24
        } else {
            self.balance_day();
            return;
        };
        //dbg!(delta_days);
        //dbg!(self.day);
        self.day += delta_days;
        //dbg!(self.day);
        //dbg!(self.hour);
        self.hour -= 24 * delta_days;
        //dbg!(self.hour);
        assert!((0..24).contains(&self.hour));
        self.balance_day();
    }

    fn balance_min(&mut self) {
        let delta_hours = if self.min > 59 {
            self.min / 60
        } else {
            self.balance_hour();
            return;
        };
        //dbg!(delta_hours);
        //dbg!(self.hour);
        self.hour += delta_hours;
        //dbg!(self.hour);
        //dbg!(self.min);
        self.min -= 60 * delta_hours;
        //dbg!(self.min);
        assert!((0..60).contains(&self.min));
        self.balance_hour();
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn balance(&mut self) {
        let delta_mins = if self.sec > 59 {
            self.sec / 60
        } else {
            self.balance_min();
            return;
        };
        //dbg!(delta_mins);
        //dbg!(self.min);
        self.min += delta_mins;
        //dbg!(self.min);
        //dbg!(self.sec);
        self.sec -= 60 * delta_mins;
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
        let dt = self.to_datetime();
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            dt.year, dt.month, dt.day, dt.hour, dt.min, dt.sec
        )
    }
}

#[allow(clippy::module_name_repetitions)]
pub trait ToDateTime {
    fn to_datetime(&self) -> DateTime;
}
impl ToDateTime for SystemTime {
    fn to_datetime(&self) -> DateTime {
        let epoch_seconds = i64::try_from(
            self.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        )
        .unwrap();
        DateTime::new(epoch_seconds)
    }
}

pub trait EpochNs {
    /// Convert to nanoseconds.
    ///
    /// # Panics
    /// Panics when the value would overflow u64.  This happens for dates in the year 2554.
    fn epoch_ns(&self) -> u64;
}
impl EpochNs for SystemTime {
    fn epoch_ns(&self) -> u64 {
        u64::try_from(
            self.duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0))
                .as_nanos(),
        )
        .unwrap()
    }
}
