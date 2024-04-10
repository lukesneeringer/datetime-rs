use std::fmt;

use serde::de::Visitor;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;

use crate::DateTime;

impl Serialize for DateTime {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    if self.nanos == 0 {
      serializer.collect_str(&self.format("%Y-%m-%dT%H:%M:%S"))
    } else if self.nanos % 1_000 == 0 {
      serializer.collect_str(&self.format("%Y-%m-%dT%H:%M:%S%.6f"))
    } else {
      serializer.collect_str(&self.format("%Y-%m-%dT%H:%M:%S%.9f"))
    }
  }
}

struct DateTimeVisitor;

impl<'de> Visitor<'de> for DateTimeVisitor {
  type Value = DateTime;

  #[cfg(not(tarpaulin_include))]
  fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
    formatter.write_str("a YYYY-MM-DD HH:MM:SS date string")
  }

  fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Self::Value, E> {
    s.parse().map_err(E::custom)
  }
}

impl<'de> Deserialize<'de> for DateTime {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    deserializer.deserialize_str(DateTimeVisitor)
  }
}

#[cfg(test)]
mod tests {
  use serde_test::assert_tokens;
  use serde_test::Token;

  use crate::datetime;
  use crate::DateTime;

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
}
