//! `datetime-rs` provides a representation of a date and time.

use std::cmp::Ordering;
use std::str::FromStr;
use std::time::SystemTime;

use date::Date;
use date::Weekday;
use format::FormattedDateTime;
use strptime::ParseError;
use strptime::ParseResult;
use strptime::Parser;
use strptime::RawDateTime;

mod format;
pub mod interval;
#[cfg(feature = "serde")]
mod serde;

#[macro_export]
macro_rules! datetime {
  ($y:literal-$m:literal-$d:literal $h:literal : $mi:literal : $s:literal) => {{
    #[allow(clippy::zero_prefixed_literal)]
    {
      $crate::DateTime::ymd($y, $m, $d).hms($h, $mi, $s).build()
    }
  }};
}

/// A representation of a date and time.
#[derive(Clone, Copy, Debug, Eq)]
pub struct DateTime {
  seconds: i64,
  nanos: u32,
  #[cfg(feature = "tz")]
  tz: Option<tz::timezone::TimeZoneRef<'static>>,
}

impl DateTime {
  /// Create a new date and time object.
  pub const fn ymd(year: i16, month: u8, day: u8) -> DateTimeBuilder {
    DateTimeBuilder {
      date: Date::new(year, month, day),
      seconds: 0,
      nanos: 0,
      #[cfg(feature = "tz")]
      tz: None,
      offset: 0,
    }
  }

  /// Create a new date and time object from the given Unix timestamp.
  pub const fn from_timestamp(timestamp: i64, nanos: u32) -> Self {
    let mut timestamp = timestamp;
    let mut nanos = nanos;
    while nanos >= 1_000_000_000 {
      nanos -= 1_000_000_000;
      timestamp += 1;
    }
    Self {
      seconds: timestamp,
      nanos,
      #[cfg(feature = "tz")]
      tz: None,
    }
  }

  /// Return the current timestamp.
  ///
  /// ## Panic
  ///
  /// Panics if the system clock is set prior to January 1, 1970.
  pub fn now() -> Self {
    let dur = SystemTime::now()
      .duration_since(SystemTime::UNIX_EPOCH)
      .expect("System clock set prior to January 1, 1970");
    Self::from_timestamp(dur.as_secs() as i64, dur.subsec_nanos())
  }
}

/// Accessors
impl DateTime {
  /// The year for this date.
  #[inline]
  pub fn year(&self) -> i16 {
    Date::from_timestamp(self.tz_seconds()).year()
  }

  /// The month for this date.
  #[inline]
  pub const fn month(&self) -> u8 {
    Date::from_timestamp(self.tz_seconds()).month()
  }

  /// The day of the month for this date.
  #[inline]
  pub const fn day(&self) -> u8 {
    Date::from_timestamp(self.tz_seconds()).day()
  }

  /// The day of the week for this date.
  #[inline]
  pub const fn weekday(&self) -> Weekday {
    Date::from_timestamp(self.tz_seconds()).weekday()
  }

  /// The hour of the day for this date and time. Range: `[0, 24)`
  #[inline]
  pub const fn hour(&self) -> u8 {
    (self.tz_seconds() % 86_400 / 3_600) as u8
  }

  /// The minute of the hour for this date and time. Range: `[0, 60)`
  #[inline]
  pub const fn minute(&self) -> u8 {
    ((self.tz_seconds() % 3600) / 60) as u8
  }

  /// The second of the minute for this date and time. Range: `[0, 60)`
  #[inline]
  pub const fn second(&self) -> u8 {
    (self.tz_seconds() % 60) as u8
  }

  /// The nanosecond of the second for this date and time. Range: `[0, 1_000_000_000)`
  #[inline]
  pub const fn nanosecond(&self) -> u32 {
    self.nanos
  }

  /// The ordinal day of the year.
  #[inline]
  pub const fn day_of_year(&self) -> u16 {
    self.date().day_of_year()
  }

  /// The date corresponding to this datetime.
  #[inline]
  pub const fn date(&self) -> Date {
    Date::from_timestamp(self.tz_seconds())
  }

  /// The Unix timestamp for this date and time.
  #[inline]
  pub const fn timestamp(&self) -> i64 {
    self.seconds
  }

  /// Provide the timestamp adjustment for the time zone.
  #[inline(always)]
  #[cfg(feature = "tz")]
  const fn tz_seconds(&self) -> i64 {
    let Some(tz) = self.tz else { return self.seconds };
    let Ok(tz) = tz.find_local_time_type(self.seconds) else { panic!("Invalid time zone") };
    self.seconds + tz.ut_offset() as i64
  }

  /// A stub method to adjust for time zones, for compatibility with the `tz` feature elsewhere.
  #[inline(always)]
  #[cfg(not(feature = "tz"))]
  const fn tz_seconds(&self) -> i64 {
    self.seconds
  }
}

impl DateTime {
  /// Format the given date and time according to the provided `strftime`-like string.
  pub fn format(&self, format: &'static str) -> FormattedDateTime {
    FormattedDateTime { dt: self, format }
  }
}

impl DateTime {
  /// Parse a date from a string, according to the provided format string.
  pub fn parse(datetime_str: impl AsRef<str>, fmt: &'static str) -> ParseResult<Self> {
    let parser = Parser::new(fmt);
    parser.parse(datetime_str)?.try_into()
  }
}

impl PartialEq for DateTime {
  fn eq(&self, other: &Self) -> bool {
    self.seconds == other.seconds && self.nanos == other.nanos
  }
}

impl PartialOrd for DateTime {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for DateTime {
  fn cmp(&self, other: &Self) -> Ordering {
    let seconds_cmp = self.seconds.cmp(&other.seconds);
    match seconds_cmp {
      Ordering::Equal => self.nanos.cmp(&other.nanos),
      _ => seconds_cmp,
    }
  }
}

impl FromStr for DateTime {
  type Err = ParseError;

  #[rustfmt::skip]
  fn from_str(s: &str) -> ParseResult<Self> {
    // Attempt several common formats.
    if let Ok(dt) = Parser::new("%Y-%m-%d %H:%M:%S").parse(s) { return dt.try_into(); }
    if let Ok(dt) = Parser::new("%Y-%m-%d %H:%M:%S%.6f").parse(s) { return dt.try_into(); }
    if let Ok(dt) = Parser::new("%Y-%m-%d %H:%M:%S%.9f").parse(s) { return dt.try_into(); }
    if let Ok(dt) = Parser::new("%Y-%m-%dT%H:%M:%S").parse(s) { return dt.try_into(); }
    if let Ok(dt) = Parser::new("%Y-%m-%dT%H:%M:%S%.6f").parse(s) { return dt.try_into(); }
    if let Ok(dt) = Parser::new("%Y-%m-%dT%H:%M:%S%.9f").parse(s) { return dt.try_into(); }
    if let Ok(dt) = Parser::new("%Y-%m-%d %H:%M:%SZ").parse(s) { return dt.try_into(); }
    Parser::new("%Y-%m-%dT%H:%M:%SZ").parse(s)?.try_into()
  }
}

impl TryFrom<RawDateTime> for DateTime {
  type Error = ParseError;

  fn try_from(value: RawDateTime) -> ParseResult<Self> {
    let date = value.date()?;
    let time = value.time().unwrap_or_default();
    Ok(
      Self::ymd(date.year(), date.month(), date.day())
        .hms(time.hour(), time.minute(), time.second())
        .nanos(time.nanosecond() as u32)
        .build(),
    )
  }
}

/// An intermediate builder for [`DateTime`].
#[must_use]
pub struct DateTimeBuilder {
  date: Date,
  seconds: i64,
  nanos: u32,
  #[cfg(feature = "tz")]
  tz: Option<tz::timezone::TimeZoneRef<'static>>,
  offset: i64,
}

impl DateTimeBuilder {
  /// Attach an hour, minute, and second to the datetime.
  pub fn hms(mut self, hour: u8, minute: u8, second: u8) -> Self {
    assert!(hour < 24, "Hour out of bounds");
    assert!(minute < 60, "Minute out of bounds");
    assert!(second < 60, "Second out of bounds");
    self.seconds = (hour as i64 * 3600) + (minute as i64 * 60) + second as i64;
    self
  }

  /// Attach fractional to the datetime.
  pub fn nanos(mut self, nanos: u32) -> Self {
    assert!(nanos < 1_000_000_000, "Nanos out of bounds.");
    self.nanos = nanos;
    self
  }

  /// Attach a timezone to the datetime.
  ///
  /// This method assumes that the timezone _modifies_ the underlying timestamp; in other words,
  /// the YMD/HMS specified to the date and time builder should be preserved, and the time zone's
  /// offset applied to the underlying timestamp to preserve the date and time on the wall clock.
  #[cfg(feature = "tz")]
  pub fn tz(mut self, tz: &'static str) -> anyhow::Result<Self> {
    let tz = tzdb::tz_by_name(tz).ok_or(anyhow::format_err!("Time zone not found: {}", tz))?;
    self.offset =
      tz.find_local_time_type(self.date.timestamp() + self.seconds)?.ut_offset() as i64;
    self.tz = Some(tz);
    Ok(self)
  }

  /// Build the final [`DateTime`] object.
  pub fn build(self) -> DateTime {
    DateTime {
      seconds: self.date.timestamp() + self.seconds - self.offset,
      nanos: self.nanos,
      #[cfg(feature = "tz")]
      tz: self.tz,
    }
  }
}

#[cfg(test)]
mod tests {
  use assert2::check;
  use strptime::ParseResult;

  use crate::DateTime;

  #[test]
  fn test_zero() {
    let dt = datetime! { 1970-01-01 00:00:00 };
    check!(dt.seconds == 0);
  }

  #[test]
  fn test_accessors() {
    let dt = datetime! { 2012-04-21 11:00:00 };
    check!(dt.year() == 2012);
    check!(dt.month() == 4);
    check!(dt.day() == 21);
    check!(dt.hour() == 11);
    check!(dt.minute() == 0);
    check!(dt.second() == 0);
  }

  #[test]
  fn test_more_accessors() {
    let dt = datetime! { 2024-02-29 13:15:45 };
    check!(dt.year() == 2024);
    check!(dt.month() == 2);
    check!(dt.day() == 29);
    check!(dt.hour() == 13);
    check!(dt.minute() == 15);
    check!(dt.second() == 45);
  }

  #[test]
  fn test_parse_str() -> ParseResult<()> {
    for s in [
      "2012-04-21 11:00:00",
      "2012-04-21T11:00:00",
      "2012-04-21 11:00:00.000000",
      "2012-04-21 11:00:00Z",
      "2012-04-21T11:00:00.000000",
      "2012-04-21T11:00:00Z",
    ] {
      let dt = s.parse::<DateTime>()?;
      check!(dt.year() == 2012);
      check!(dt.month() == 4);
      check!(dt.day() == 21);
      check!(dt.hour() == 11);
    }

    Ok(())
  }

  #[cfg(feature = "tz")]
  #[test]
  fn test_tz() -> anyhow::Result<()> {
    let dt = DateTime::ymd(2012, 4, 21).hms(11, 0, 0).tz("America/New_York")?.build();
    check!(dt.timestamp() == 1335020400);
    check!(dt.year() == 2012);
    check!(dt.month() == 4);
    check!(dt.day() == 21);
    check!(dt.hour() == 11);
    let dt = DateTime::ymd(1970, 1, 1).tz("America/Los_Angeles")?.build();
    check!(dt.timestamp() == 3600 * 8);
    Ok(())
  }
}
