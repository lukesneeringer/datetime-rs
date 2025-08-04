//! `datetime-rs` provides a representation of a date and time.
//!
//! Internal storage is a Unix timestamp and, if the `tz` feature is enabled (which it is not by
//! default), optionally a `TimeZone`.

#![doc(html_root_url = "https://docs.rs/datetime-rs/latest")]
#![cfg_attr(docsrs, feature(doc_cfg))]

use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;
use std::time::SystemTime;

use format::FormattedDateTime;
use strptime::ParseError;
use strptime::ParseResult;
use strptime::Parser;
use strptime::RawDateTime;

/// Construct a date and time from a `YYYY-MM-DD HH:MM:SS` literal.
#[macro_export]
macro_rules! datetime {
  ($y:literal-$m:literal-$d:literal $h:literal : $mi:literal : $s:literal) => {{
    #[allow(clippy::zero_prefixed_literal)]
    {
      $crate::DateTime::ymd($y, $m, $d).hms($h, $mi, $s).build()
    }
  }};
  ($y:literal-$m:literal-$d:literal $h:literal : $mi:literal : $s:literal $($tz:ident)::+) => {{
    #[cfg(feature = "tz")]
    #[allow(clippy::zero_prefixed_literal)]
    {
      match $crate::DateTime::ymd($y, $m, $d).hms($h, $mi, $s).tz($crate::tz::$($tz)::+) {
        Ok(dt) => dt.build(),
        Err(_) => panic!("invalid date/time and time zone combination"),
      }
    }
    #[cfg(not(feature = "tz"))]
    {
      compile_error!("The `tz` feature must be enabled to specify a time zone.");
    }
  }};
}

#[cfg(feature = "diesel-pg")]
mod diesel_pg;
#[cfg(feature = "duckdb")]
mod duckdb;
mod format;
pub mod interval;
#[cfg(feature = "serde")]
mod serde;

pub use date::Date;
pub use date::Weekday;
pub use date::date;

/// Time zone compnents.
///
/// These are re-exported from the `date-rs` crate.
#[cfg(feature = "tz")]
#[cfg_attr(docsrs, doc(cfg(feature = "tz")))]
pub mod tz {
  pub use date::tz::*;

  #[derive(Clone, Copy, Debug, Eq, PartialEq)]
  pub(crate) enum TimeZone {
    Unspecified,
    Tz(crate::tz::TimeZoneRef<'static>),
    FixedOffset(i32),
  }

  impl TimeZone {
    pub(crate) const fn ut_offset(&self, timestamp: i64) -> TzResult<i32> {
      match self {
        Self::Unspecified => Ok(0),
        Self::FixedOffset(offset) => Ok(*offset),
        Self::Tz(tz) => match tz.find_local_time_type(timestamp) {
          Ok(t) => Ok(t.ut_offset()),
          Err(e) => Err(e),
        },
      }
    }
  }
}

/// A representation of a date and time.
#[derive(Clone, Copy, Eq)]
#[cfg_attr(feature = "diesel-pg", derive(diesel::AsExpression, diesel::FromSqlRow))]
#[cfg_attr(feature = "diesel-pg", diesel(
    sql_type = diesel::sql_types::Timestamp,
    sql_type = diesel::sql_types::Timestamptz))]
pub struct DateTime {
  seconds: i64,
  nanos: u32,
  #[cfg(feature = "tz")]
  tz: tz::TimeZone,
}

impl DateTime {
  /// Create a new date and time object.
  pub const fn ymd(year: i16, month: u8, day: u8) -> DateTimeBuilder {
    DateTimeBuilder {
      date: Date::new(year, month, day),
      seconds: 0,
      nanos: 0,
      #[cfg(feature = "tz")]
      tz: tz::TimeZone::Unspecified,
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
      tz: tz::TimeZone::Unspecified,
    }
  }

  /// Create a new date and time object from the given Unix timestamp in milliseconds.
  pub const fn from_timestamp_millis(millis: i64) -> Self {
    Self::from_timestamp(millis.div_euclid(1_000), millis.rem_euclid(1_000) as u32)
  }

  /// Create a new date and time object from the given Unix timestamp in microseconds.
  pub const fn from_timestamp_micros(micros: i64) -> Self {
    Self::from_timestamp(micros.div_euclid(1_000_000), micros.rem_euclid(1_000_000) as u32)
  }

  /// Create a new date and time object from the given Unix timestamp in nanoseconds.
  pub const fn from_timestamp_nanos(nanos: i128) -> Self {
    Self::from_timestamp(
      nanos.div_euclid(1_000_000_000) as i64,
      nanos.rem_euclid(1_000_000_000) as u32,
    )
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

#[cfg(feature = "tz")]
impl DateTime {
  /// Set the time zone to the provided time zone, without adjusting the underlying absolute
  /// timestamp.
  ///
  /// This method modifies the wall clock time while maintaining the underlying absolute timestamp.
  /// To modify the timestamp instead, use `in_tz`.
  #[inline]
  pub const fn with_tz(mut self, tz: tz::TimeZoneRef<'static>) -> Self {
    self.tz = tz::TimeZone::Tz(tz);
    self
  }

  /// Set the timestamp to the same wall clock time in the provided time zone.
  ///
  /// This method modifies the underlying timestamp while maintaining the wall clock time.
  /// To maintain the timestamp instead, use `with_tz`.
  #[inline]
  pub const fn in_tz(mut self, tz: tz::TimeZoneRef<'static>) -> Self {
    let existing_ut_offset = match self.tz.ut_offset(self.seconds) {
      Ok(offset) => offset as i64,
      Err(_) => panic!("Invalid time zone."),
    };
    let desired_ut_offset = match tz.find_local_time_type(self.seconds) {
      Ok(t) => t.ut_offset() as i64,
      Err(_) => panic!("Invalid time zone for this timestamp."),
    };
    self.seconds += existing_ut_offset - desired_ut_offset;
    self.tz = tz::TimeZone::Tz(tz);
    self
  }
}

/// Accessors
impl DateTime {
  /// The year for this date.
  #[inline]
  pub const fn year(&self) -> i16 {
    Date::from_timestamp(self.tz_adjusted_seconds()).year()
  }

  /// The month for this date.
  #[inline]
  pub const fn month(&self) -> u8 {
    Date::from_timestamp(self.tz_adjusted_seconds()).month()
  }

  /// The day of the month for this date.
  #[inline]
  pub const fn day(&self) -> u8 {
    Date::from_timestamp(self.tz_adjusted_seconds()).day()
  }

  /// The day of the week for this date.
  #[inline]
  pub const fn weekday(&self) -> Weekday {
    Date::from_timestamp(self.tz_adjusted_seconds()).weekday()
  }

  /// The hour of the day for this date and time. Range: `[0, 24)`
  #[inline]
  pub const fn hour(&self) -> u8 {
    (self.tz_adjusted_seconds() % 86_400 / 3_600) as u8
  }

  /// The minute of the hour for this date and time. Range: `[0, 60)`
  #[inline]
  pub const fn minute(&self) -> u8 {
    ((self.tz_adjusted_seconds() % 3600) / 60) as u8
  }

  /// The second of the minute for this date and time. Range: `[0, 60)`
  #[inline]
  pub const fn second(&self) -> u8 {
    (self.tz_adjusted_seconds() % 60) as u8
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
    Date::from_timestamp(self.tz_adjusted_seconds())
  }

  /// The number of seconds since the Unix epoch for this date and time.
  #[inline]
  pub const fn as_seconds(&self) -> i64 {
    self.seconds
  }

  /// The number of milliseconds since the Unix epoch for this date and time.
  #[inline]
  pub const fn as_milliseconds(&self) -> i64 {
    self.seconds * 1_000 + (self.nanos / 1_000_000) as i64
  }

  /// The number of microseconds since the Unix epoch for this date and time.
  #[inline]
  pub const fn as_microseconds(&self) -> i64 {
    self.seconds * 1_000_000 + (self.nanos / 1_000) as i64
  }

  /// The number of nanoseconds since the Unix epoch for this date and time.
  #[inline]
  pub const fn as_nanoseconds(&self) -> i128 {
    self.seconds as i128 * 1_000_000_000 + self.nanos as i128
  }

  /// The precision required to represent this timestamp with no fidelity loss.
  #[inline]
  pub const fn precision(&self) -> Precision {
    if self.nanos == 0 {
      Precision::Second
    } else if self.nanos % 1_000_000 == 0 {
      Precision::Millisecond
    } else if self.nanos % 1_000 == 0 {
      Precision::Microsecond
    } else {
      Precision::Nanosecond
    }
  }

  /// Provide the number of seconds since the epoch in the time zone with the same offset as this
  /// datetime's time zone.
  #[inline(always)]
  const fn tz_adjusted_seconds(&self) -> i64 {
    self.seconds + self.tz_offset()
  }

  /// Provide the offset, in seconds
  const fn tz_offset(&self) -> i64 {
    #[cfg(feature = "tz")]
    {
      match self.tz.ut_offset(self.seconds) {
        Ok(offset) => offset as i64,
        Err(_) => panic!("Invalid time zone"),
      }
    }
    #[cfg(not(feature = "tz"))]
    0
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
    if let Ok(dt) = Parser::new("%Y-%m-%dT%H:%M:%S").parse(s) { return dt.try_into(); }
    if let Ok(dt) = Parser::new("%Y-%m-%dT%H:%M:%S%z").parse(s) { return dt.try_into(); }
    if let Ok(dt) = Parser::new("%Y-%m-%d %H:%M:%S").parse(s) { return dt.try_into(); }
    if let Ok(dt) = Parser::new("%Y-%m-%d %H:%M:%S%z").parse(s) { return dt.try_into(); }
    if let Ok(dt) = Parser::new("%Y-%m-%dT%H:%M:%S%.6f").parse(s) { return dt.try_into(); }
    if let Ok(dt) = Parser::new("%Y-%m-%dT%H:%M:%S%.6f%z").parse(s) { return dt.try_into(); }
    if let Ok(dt) = Parser::new("%Y-%m-%d %H:%M:%S%.6f").parse(s) { return dt.try_into(); }
    if let Ok(dt) = Parser::new("%Y-%m-%d %H:%M:%S%.6f%z").parse(s) { return dt.try_into(); }
    if let Ok(dt) = Parser::new("%Y-%m-%dT%H:%M:%S%.9f").parse(s) { return dt.try_into(); }
    if let Ok(dt) = Parser::new("%Y-%m-%dT%H:%M:%S%.9f%z").parse(s) { return dt.try_into(); }
    if let Ok(dt) = Parser::new("%Y-%m-%d %H:%M:%S%.9f").parse(s) { return dt.try_into(); }
    if let Ok(dt) = Parser::new("%Y-%m-%d %H:%M:%S%.9f%z").parse(s) { return dt.try_into(); }
    if let Ok(dt) = Parser::new("%Y-%m-%d %H:%M:%SZ").parse(s) { return dt.try_into(); }
    Parser::new("%Y-%m-%dT%H:%M:%SZ").parse(s)?.try_into()
  }
}

impl TryFrom<RawDateTime> for DateTime {
  type Error = ParseError;

  fn try_from(value: RawDateTime) -> ParseResult<Self> {
    let date = value.date()?;
    let time = value.time().unwrap_or_default();
    Ok(match time.utc_offset() {
      #[cfg(feature = "tz")]
      Some(utc_offset) => Self::ymd(date.year(), date.month(), date.day())
        .hms(time.hour(), time.minute(), time.second())
        .nanos(time.nanosecond() as u32)
        .utc_offset(utc_offset)
        .build(),
      #[cfg(not(feature = "tz"))]
      Some(_) => panic!("Enable the `tz` feature to parse datetimes with UTC offsets."),
      None => Self::ymd(date.year(), date.month(), date.day())
        .hms(time.hour(), time.minute(), time.second())
        .nanos(time.nanosecond() as u32)
        .build(),
    })
  }
}

impl fmt::Debug for DateTime {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if self.nanos == 0 {
      write!(f, "{}", self.format("%Y-%m-%d %H:%M:%S"))
    } else if self.nanos % 1_000_000 == 0 {
      write!(f, "{}", self.format("%Y-%m-%d %H:%M:%S%.3f"))
    } else if self.nanos % 1_000 == 0 {
      write!(f, "{}", self.format("%Y-%m-%d %H:%M:%S%.6f"))
    } else {
      write!(f, "{}", self.format("%Y-%m-%d %H:%M:%S%.9f"))
    }
  }
}

#[cfg(feature = "log")]
impl log::kv::ToValue for DateTime {
  fn to_value(&self) -> log::kv::Value<'_> {
    log::kv::Value::from_debug(self)
  }
}

/// An intermediate builder for [`DateTime`].
#[must_use]
pub struct DateTimeBuilder {
  date: Date,
  seconds: i64,
  nanos: u32,
  #[cfg(feature = "tz")]
  tz: tz::TimeZone,
  offset: i64,
}

impl DateTimeBuilder {
  /// Attach an hour, minute, and second to the datetime.
  pub const fn hms(mut self, hour: u8, minute: u8, second: u8) -> Self {
    assert!(hour < 24, "Hour out of bounds");
    assert!(minute < 60, "Minute out of bounds");
    assert!(second < 60, "Second out of bounds");
    self.seconds = (hour as i64 * 3600) + (minute as i64 * 60) + second as i64;
    self
  }

  /// Attach fractional to the datetime.
  pub const fn nanos(mut self, nanos: u32) -> Self {
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
  pub const fn tz(mut self, tz: tz::TimeZoneRef<'static>) -> tz::TzResult<Self> {
    self.offset = match tz.find_local_time_type(self.date.timestamp() + self.seconds) {
      Ok(t) => t.ut_offset() as i64,
      Err(e) => return Err(e),
    };
    self.tz = tz::TimeZone::Tz(tz);
    Ok(self)
  }

  /// Attach a UTC offset to the datetime.
  ///
  /// This method assumes that the offset _modifies_ the underlying timestamp; in other words, the
  /// YMD/HMS specified to the date and time builder should be preserved, and the offset applied to
  /// the underlying timestamp to preserve the date and time on the wall clock.
  #[cfg(feature = "tz")]
  pub(crate) const fn utc_offset(mut self, offset: i32) -> Self {
    self.offset = offset as i64;
    self.tz = tz::TimeZone::FixedOffset(offset);
    self
  }

  /// Build the final [`DateTime`] object.
  pub const fn build(self) -> DateTime {
    DateTime {
      seconds: self.date.timestamp() + self.seconds - self.offset,
      nanos: self.nanos,
      #[cfg(feature = "tz")]
      tz: self.tz,
    }
  }
}

trait Sealed {}
impl Sealed for date::Date {}

/// Convert from a date into a datetime, by way of a builder.
#[allow(private_bounds)]
pub trait FromDate: Sealed {
  /// Create a `DateTimeBuilder` for this Date.
  fn hms(self, hour: u8, minute: u8, second: u8) -> DateTimeBuilder;
}

impl FromDate for date::Date {
  fn hms(self, hour: u8, minute: u8, second: u8) -> DateTimeBuilder {
    DateTimeBuilder {
      date: self,
      seconds: 0,
      nanos: 0,
      #[cfg(feature = "tz")]
      tz: tz::TimeZone::Unspecified,
      offset: 0,
    }
    .hms(hour, minute, second)
  }
}

/// The precision that this timestamp requires in order to represent with no fidelity loss.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Precision {
  Second,
  Millisecond,
  Microsecond,
  Nanosecond,
}

#[cfg(test)]
mod tests {
  use assert2::check;
  use strptime::ParseResult;

  use crate::DateTime;
  use crate::FromDate;
  use crate::Precision;
  use crate::interval::TimeInterval;
  #[cfg(feature = "tz")]
  use crate::tz;

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

  #[test]
  #[cfg(feature = "tz")]
  fn test_parse_str_tz() -> ParseResult<()> {
    for s in
      ["2012-04-21 11:00:00-0400", "2012-04-21T11:00:00-0400", "2012-04-21 11:00:00.000000-0400"]
    {
      let dt = s.parse::<DateTime>()?;
      check!(dt.year() == 2012);
      check!(dt.month() == 4);
      check!(dt.day() == 21);
      check!(dt.hour() == 11);
    }
    Ok(())
  }

  #[test]
  #[allow(clippy::inconsistent_digit_grouping)]
  fn test_as_precision() {
    let dt = DateTime::ymd(2012, 4, 21).hms(15, 0, 0).build();
    check!(dt.as_seconds() == 1335020400);
    check!(dt.as_milliseconds() == 1335020400_000);
    check!(dt.as_microseconds() == 1335020400_000_000);
    check!(dt.as_nanoseconds() == 1335020400_000_000_000);
  }

  #[test]
  fn test_precision() {
    let mut dt = DateTime::ymd(2012, 4, 21).hms(15, 0, 0).build();
    check!(dt.precision() == Precision::Second);
    dt += TimeInterval::new(0, 1_000_000);
    check!(dt.precision() == Precision::Millisecond);
    dt += TimeInterval::new(0, 1_000);
    check!(dt.precision() == Precision::Microsecond);
    dt += TimeInterval::new(0, 1);
    check!(dt.precision() == Precision::Nanosecond);
  }

  #[cfg(feature = "tz")]
  #[test]
  fn test_tz() -> tz::TzResult<()> {
    let dt = DateTime::ymd(2012, 4, 21).hms(11, 0, 0).tz(tz::us::EASTERN)?.build();
    check!(dt.as_seconds() == 1335020400);
    check!(dt.year() == 2012);
    check!(dt.month() == 4);
    check!(dt.day() == 21);
    check!(dt.hour() == 11);
    let dt = DateTime::ymd(1970, 1, 1).tz(tz::us::PACIFIC)?.build();
    check!(dt.as_seconds() == 3600 * 8);
    Ok(())
  }

  #[cfg(feature = "tz")]
  #[test]
  fn test_unix_tz() {
    #[allow(clippy::inconsistent_digit_grouping)]
    for dt in [
      DateTime::from_timestamp(1335020400, 0),
      DateTime::from_timestamp_millis(1335020400_000),
      DateTime::from_timestamp_micros(1335020400_000_000),
      DateTime::from_timestamp_nanos(1335020400_000_000_000),
    ] {
      let dt = dt.with_tz(tz::us::EASTERN);
      check!(dt.as_seconds() == 1335020400);
      check!(dt.year() == 2012);
      check!(dt.month() == 4);
      check!(dt.day() == 21);
      check!(dt.hour() == 11);
    }
  }

  #[cfg(feature = "tz")]
  #[test]
  fn test_in_tz() {
    let dt = DateTime::from_timestamp(1335020400, 0).with_tz(tz::us::EASTERN);
    check!(dt.hour() == 11);
    check!(dt.in_tz(tz::us::CENTRAL).hour() == 11);
    check!(dt.as_seconds() - dt.in_tz(tz::us::CENTRAL).as_seconds() == -3600);
    check!(dt.in_tz(tz::europe::LONDON).hour() == 11);
    check!(dt.as_seconds() - dt.in_tz(tz::europe::LONDON).as_seconds() == 3600 * 5);
  }

  #[test]
  fn test_from_date_trait() {
    let dt = date::date! { 2012-04-21 }.hms(11, 0, 0).build();
    check!(dt.year() == 2012);
    check!(dt.month() == 4);
    check!(dt.day() == 21);
    check!(dt.hour() == 11);
  }

  #[test]
  fn test_debug() {
    let dt = date::date! { 2012-04-21 }.hms(15, 0, 0).build();
    check!(format!("{:?}", dt) == "2012-04-21 15:00:00");
    let dt = date::date! { 2012-04-21 }.hms(15, 0, 0).nanos(500_000_000).build();
    check!(format!("{:?}", dt) == "2012-04-21 15:00:00.500");
    let dt = date::date! { 2012-04-21 }.hms(15, 0, 0).nanos(123_450_000).build();
    check!(format!("{:?}", dt) == "2012-04-21 15:00:00.123450");
    let dt = date::date! { 2012-04-21 }.hms(15, 0, 0).nanos(123_456_789).build();
    check!(format!("{:?}", dt) == "2012-04-21 15:00:00.123456789");
  }
}
