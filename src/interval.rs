use std::ops::Add;
use std::ops::AddAssign;
use std::ops::Div;
use std::ops::Sub;
use std::ops::SubAssign;

use crate::DateTime;

/// An interval of time between two timestamps.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, PartialOrd, Ord)]
pub struct TimeInterval {
  seconds: i64,
  nanos: u32,
}

impl TimeInterval {
  /// Create a new time interval from seconds and nanoseconds.
  pub const fn new(seconds: i64, nanos: u32) -> Self {
    Self { seconds, nanos }
  }

  /// The number of seconds this interval represents.
  ///
  /// Note that the nanoseconds value is always positive, even if seconds is negative. For example,
  /// an interval representing -2.5 seconds will be represented as -3 seconds and 500,000,000
  /// nanos.
  pub const fn seconds(&self) -> i64 {
    self.seconds
  }

  /// The number of nanoseconds this interval represents.
  ///
  /// Note that the nanoseconds value is always positive, even if seconds is negative. For example,
  /// an interval representing -2.5 seconds will be represented as -3 seconds and 500,000,000
  /// nanos.
  pub const fn nanoseconds(&self) -> u32 {
    self.nanos
  }

  /// The number of milliseconds this interval represents.
  pub const fn as_milliseconds(&self) -> i64 {
    self.seconds * 1_000 + (self.nanos / 1_000_000) as i64
  }

  /// The number of microseconds this interval represents.
  pub const fn as_microseconds(&self) -> i64 {
    self.seconds * 1_000_000 + (self.nanos / 1_000) as i64
  }

  /// The number of nanoseconds this interval represents.
  pub const fn as_nanoseconds(&self) -> i128 {
    self.seconds as i128 * 1_000_000_000 + self.nanos as i128
  }
}

impl Add<TimeInterval> for DateTime {
  type Output = DateTime;

  fn add(self, rhs: TimeInterval) -> Self::Output {
    let seconds = self.seconds + rhs.seconds;
    let nanos = self.nanos + rhs.nanos;
    Self {
      seconds,
      nanos,
      #[cfg(feature = "tz")]
      tz: self.tz,
    }
  }
}

impl AddAssign<TimeInterval> for DateTime {
  fn add_assign(&mut self, rhs: TimeInterval) {
    self.seconds += rhs.seconds;
    self.nanos += rhs.nanos;
    while self.nanos >= 1_000_000_000 {
      self.seconds += 1;
      self.nanos -= 1_000_000_000;
    }
  }
}

impl Sub<TimeInterval> for DateTime {
  type Output = DateTime;

  #[allow(clippy::suspicious_arithmetic_impl)]
  fn sub(self, rhs: TimeInterval) -> Self::Output {
    let mut seconds = self.seconds - rhs.seconds;
    let nanos = self.nanos.checked_sub(rhs.nanos).unwrap_or_else(|| {
      seconds -= 1;
      self.nanos + 1_000_000_000 - rhs.nanos
    });
    Self {
      seconds,
      nanos,
      #[cfg(feature = "tz")]
      tz: self.tz,
    }
  }
}

impl SubAssign<TimeInterval> for DateTime {
  fn sub_assign(&mut self, rhs: TimeInterval) {
    self.seconds -= rhs.seconds;
    match self.nanos >= rhs.nanos {
      true => self.nanos -= rhs.nanos,
      false => {
        self.nanos += 1_000_000_000;
        self.seconds -= 1;
        self.nanos -= rhs.nanos;
      },
    }
  }
}

impl Sub for DateTime {
  type Output = TimeInterval;

  #[allow(clippy::suspicious_arithmetic_impl)]
  fn sub(self, rhs: Self) -> Self::Output {
    let mut seconds = self.seconds - rhs.seconds;
    let nanos = self.nanos.checked_sub(rhs.nanos).unwrap_or_else(|| {
      seconds -= 1;
      self.nanos + 1_000_000_000 - rhs.nanos
    });
    TimeInterval { seconds, nanos }
  }
}

impl Div for TimeInterval {
  type Output = f64;

  fn div(self, rhs: Self) -> Self::Output {
    self.as_nanoseconds() as f64 / rhs.as_nanoseconds() as f64
  }
}

#[cfg(test)]
mod tests {
  use assert2::check;

  use super::*;
  use crate::DateTime;
  use crate::datetime;

  #[test]
  fn test_add() {
    check!(
      datetime! { 2012-04-21 11:00:00 } + TimeInterval::new(3600, 0)
        == datetime! { 2012-04-21 12:00:00 }
    );
    check!(
      datetime! { 2012-04-21 11:00:00 } + TimeInterval::new(1800, 0)
        == datetime! { 2012-04-21 11:30:00 }
    );
    check!(
      datetime! { 2012-04-21 11:00:00 } + TimeInterval::new(0, 500_000_000)
        == DateTime::ymd(2012, 4, 21).hms(11, 0, 0).nanos(500_000_000).build()
    );
  }

  #[test]
  fn test_add_assign() {
    let mut dt = datetime! { 2012-04-21 11:00:00 };
    dt += TimeInterval::new(3600, 0);
    check!(dt == datetime! { 2012-04-21 12:00:00 });
    dt += TimeInterval::new(0, 750_000_000);
    dt += TimeInterval::new(0, 250_000_000);
    check!(dt == datetime! { 2012-04-21 12:00:01 });
  }

  #[test]
  fn test_sub() {
    check!(
      datetime! { 2012-04-21 11:00:00 } - TimeInterval::new(3600, 0)
        == datetime! { 2012-04-21 10:00:00 }
    );
    check!(
      datetime! { 2012-04-21 11:00:00 } - TimeInterval::new(0, 500_000_000)
        == DateTime::ymd(2012, 4, 21).hms(10, 59, 59).nanos(500_000_000).build()
    );
  }

  #[test]
  fn test_sub_assign() {
    let mut dt = datetime! { 2012-04-21 11:00:00 };
    dt -= TimeInterval::new(3600, 0);
    check!(dt == datetime! { 2012-04-21 10:00:00 });
    dt -= TimeInterval::new(0, 750_000_000);
    dt -= TimeInterval::new(0, 350_000_000);
    dt -= TimeInterval::new(0, 900_000_000);
    check!(dt == datetime! { 2012-04-21 09:59:58 });
  }

  #[test]
  fn test_sub_dt() {
    check!(
      datetime! { 2012-04-21 11:00:00 } - datetime! { 2012-04-21 10:00:00 }
        == TimeInterval::new(3600, 0)
    );
    check!(
      datetime! { 2012-04-21 11:00:00 } - datetime! { 2012-04-21 12:00:00 }
        == TimeInterval::new(-3600, 0)
    );
  }

  #[test]
  fn test_div() {
    check!(TimeInterval::new(3600, 0) / TimeInterval::new(1800, 0) == 2.0);
    check!(TimeInterval::new(-1800, 0) / TimeInterval::new(-3600, 0) == 0.5);
    check!(TimeInterval::new(-1800, 0) / TimeInterval::new(3600, 0) == -0.5);
    check!(TimeInterval::new(0, 3600) / TimeInterval::new(0, 1800) == 2.0);
  }

  #[test]
  fn test_as() {
    let dur = TimeInterval::new(5, 0);
    check!(dur.as_milliseconds() == 5_000);
    check!(dur.as_microseconds() == 5_000_000);
    check!(dur.as_nanoseconds() == 5_000_000_000);
  }
}
