# Date & Time

[![ci](https://github.com/lukesneeringer/datetime-rs/actions/workflows/ci.yaml/badge.svg)](https://github.com/lukesneeringer/datetime-rs/actions/workflows/ci.yaml)
[![codecov](https://codecov.io/gh/lukesneeringer/datetime-rs/branch/main/graph/badge.svg?token=YbiBQd8Vn6)](https://codecov.io/gh/lukesneeringer/datetime-rs)
[![release](https://img.shields.io/crates/v/datetime-rs.svg)](https://crates.io/crates/datetime-rs)
[![docs](https://img.shields.io/badge/docs-release-blue)](https://docs.rs/datetime-rs/latest/datetime/)

The `datetime` crate provides a simple, easy-to-use `DateTime` struct (and corresponding macro).
DateTime provides storage for a date and time, and optionally a time zone (if the `tz` feature is
enabled).

The underlying storage is a Unix timestamp, so `DateTime` objects are comparable (even when in
different time zones). Additonally, if you don't need the concept of time zones (e.g. because you
can assume one), you can leave the `tz` feature off and not take the baggage.

A `DateTime` with no time zone specified behaves identically to UTC.

## Examples

Making a `DateTime`:

```rs
use datetime::DateTime;

let dt = DateTime::ymd(2012, 4, 21).hms(11, 0, 0).build();
```

You can also use the `datetime!` macro to get a syntax resembling a date literal:

```rs
use datetime::datetime;

let dt = datetime! { 2012-04-21 11:00:00 };
```

## Features

`datetime-rs` ships with the following features:

- **`diesel-pg`**: Enables interop with PostgreSQL `TIMESTAMP` columns using Diesel.
- **`log`**: Adds a `log::kv::ToValue` implementation.
- **`serde`**: Enables serialization and desearialization with `serde`. _(Enabled by default.)_
- **`tz`**: Enables support for time-zone-aware date construction.
