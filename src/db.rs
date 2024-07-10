//! Serialization to/from PostgreSQL

use diesel::deserialize::FromSql;
use diesel::deserialize::Result as DeserializeResult;
use diesel::pg::data_types::PgTimestamp;
use diesel::pg::Pg;
use diesel::pg::PgValue;
use diesel::serialize::Output;
use diesel::serialize::Result as SerializeResult;
use diesel::serialize::ToSql;
use diesel::sql_types;

use crate::interval::TimeInterval;
use crate::DateTime;

impl ToSql<sql_types::Timestamp, Pg> for DateTime {
  fn to_sql<'se>(&'se self, out: &mut Output<'se, '_, Pg>) -> SerializeResult {
    let micros_from_epoch = (*self - PG_EPOCH).as_microseconds();
    ToSql::<sql_types::Timestamp, Pg>::to_sql(&PgTimestamp(micros_from_epoch), &mut out.reborrow())
  }
}

impl ToSql<sql_types::Timestamptz, Pg> for DateTime {
  fn to_sql<'se>(&'se self, out: &mut Output<'se, '_, Pg>) -> SerializeResult {
    ToSql::<sql_types::Timestamp, Pg>::to_sql(self, out)
  }
}

impl FromSql<sql_types::Timestamp, Pg> for DateTime {
  fn from_sql(bytes: PgValue<'_>) -> DeserializeResult<Self> {
    let PgTimestamp(micros) = FromSql::<diesel::sql_types::Timestamp, Pg>::from_sql(bytes)?;
    let seconds = micros.div_euclid(1_000_000);
    let micros = match micros.signum() {
      0 => 0,
      1 => micros % 1_000_000,
      -1 => 1_000_000 - (micros % 1_000_000).abs(),
      _ => unreachable!("signum always returns -1, 0, or 1"),
    };
    let duration = TimeInterval::new(seconds, micros as u32 * 1_000);
    Ok(PG_EPOCH + duration)
  }
}

impl FromSql<sql_types::Timestamptz, Pg> for DateTime {
  fn from_sql(bytes: PgValue<'_>) -> DeserializeResult<Self> {
    FromSql::<sql_types::Timestamp, Pg>::from_sql(bytes)
  }
}

const PG_EPOCH: DateTime = datetime! { 2000-01-01 00:00:00 };
