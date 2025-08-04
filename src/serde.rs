use std::fmt;

use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use serde::de::Visitor;

use crate::DateTime;

#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl Serialize for DateTime {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    if self.nanos == 0 {
      serializer.collect_str(&self.format("%Y-%m-%dT%H:%M:%S%z"))
    } else if self.nanos % 1_000 == 0 {
      serializer.collect_str(&self.format("%Y-%m-%dT%H:%M:%S%.6f%z"))
    } else {
      serializer.collect_str(&self.format("%Y-%m-%dT%H:%M:%S%.9f%z"))
    }
  }
}

struct DateTimeVisitor;

impl Visitor<'_> for DateTimeVisitor {
  type Value = DateTime;

  #[cfg(not(tarpaulin_include))]
  fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
    formatter.write_str("a YYYY-MM-DD HH:MM:SS date string")
  }

  fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Self::Value, E> {
    s.parse().map_err(E::custom)
  }
}

#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl<'de> Deserialize<'de> for DateTime {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    deserializer.deserialize_str(DateTimeVisitor)
  }
}

#[cfg(test)]
mod tests {
  use serde_test::Token;
  use serde_test::assert_tokens;

  use crate::DateTime;
  use crate::datetime;

  #[test]
  fn test_serde() {
    assert_tokens(&datetime! { 2012-04-21 11:00:00 }, &[Token::Str("2012-04-21T11:00:00")]);
    assert_tokens(&DateTime::ymd(2024, 7, 4).hms(15, 30, 45).nanos(123_456_000).build(), &[
      Token::Str("2024-07-04T15:30:45.123456"),
    ]);
    assert_tokens(&DateTime::ymd(2024, 7, 4).hms(15, 30, 45).nanos(123_456_789).build(), &[
      Token::Str("2024-07-04T15:30:45.123456789"),
    ]);
  }

  #[cfg(feature = "tz")]
  #[test]
  fn test_serde_tz() {
    assert_tokens(&datetime! { 2012-04-21 11:00:00 us::EASTERN }, &[Token::Str(
      "2012-04-21T11:00:00-0400",
    )]);
    assert_tokens(&datetime! { 2012-04-21 11:00:00 europe::BERLIN }, &[Token::Str(
      "2012-04-21T11:00:00+0200",
    )]);
  }
}
