//! Integration with DuckDB.

use duckdb::Result;
use duckdb::types::FromSql;
use duckdb::types::FromSqlError;
use duckdb::types::FromSqlResult;
use duckdb::types::TimeUnit;
use duckdb::types::ToSql;
use duckdb::types::ToSqlOutput;
use duckdb::types::ValueRef;

use crate::DateTime;
use crate::Precision;

macro_rules! from_sql {
  ($($precision:ident($var:ident) => $e:expr)*) => {
    impl FromSql for DateTime {
      fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
          $(ValueRef::Timestamp(TimeUnit::$precision, $var) => Ok($e),
          ValueRef::Time64(TimeUnit::$precision, $var) => Ok($e),)*
          _ => Err(FromSqlError::InvalidType),
        }
      }
    }
  }
}

from_sql! {
  Second(seconds) => DateTime::from_timestamp(seconds, 0)
  Millisecond(ms) => DateTime::from_timestamp(ms / 1_000, (ms % 1_000) as u32 * 1_000_000)
  Microsecond(us) => DateTime::from_timestamp(us / 1_000_000, (us % 1_000_000) as u32 * 1_000)
  Nanosecond(ns)  => DateTime::from_timestamp(ns / 1_000_000_000, (ns % 1_000_000_000) as u32)
}

impl ToSql for DateTime {
  fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
    match self.precision() {
      Precision::Second =>
        Ok(ToSqlOutput::Borrowed(ValueRef::Timestamp(TimeUnit::Second, self.as_seconds()))),
      Precision::Millisecond => Ok(ToSqlOutput::Borrowed(ValueRef::Timestamp(
        TimeUnit::Millisecond,
        self.as_milliseconds(),
      ))),
      Precision::Microsecond => Ok(ToSqlOutput::Borrowed(ValueRef::Timestamp(
        TimeUnit::Microsecond,
        self.as_microseconds(),
      ))),
      Precision::Nanosecond => Ok(ToSqlOutput::Borrowed(ValueRef::Timestamp(
        TimeUnit::Nanosecond,
        (self.as_nanoseconds().try_into())
          .map_err(|e| duckdb::Error::ToSqlConversionFailure(Box::new(e)))?,
      ))),
    }
  }
}

#[cfg(test)]
mod tests {
  use assert2::check;

  use super::*;
  use crate::datetime;

  #[test]
  fn test_from_sql() -> FromSqlResult<()> {
    use TimeUnit::*;
    for (precision, multiplier) in
      [(Second, 1), (Millisecond, 1_000), (Microsecond, 1_000_000), (Nanosecond, 1_000_000_000)]
    {
      let input = ValueRef::Timestamp(precision, 1335020400 * multiplier);
      let dt = DateTime::column_result(input)?;
      check!(dt == datetime! { 2012-04-21 15:00:00 });
      let input = ValueRef::Time64(precision, 1335020400 * multiplier);
      let dt = DateTime::column_result(input)?;
      check!(dt == datetime! { 2012-04-21 15:00:00 });
    }
    Ok(())
  }

  #[test]
  fn test_to_sql() -> Result<()> {
    let dt = datetime! { 2012-04-21 15:00:00 };
    let output = dt.to_sql()?;
    if let ToSqlOutput::Borrowed(ValueRef::Timestamp(TimeUnit::Second, seconds)) = output {
      check!(seconds == 1335020400);
    } else {
      check!(false, "Incorrect type");
    }
    Ok(())
  }
}
