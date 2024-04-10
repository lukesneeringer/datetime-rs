use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Error;
use std::fmt::Formatter;
use std::fmt::Result;
use std::fmt::Write;

use crate::DateTime;

/// A date with a requested format.
pub struct FormattedDateTime<'a> {
  pub(crate) dt: &'a DateTime,
  pub(crate) format: &'a str,
}

impl<'a> FormattedDateTime<'a> {
  fn offset(&self) -> String {
    format!(
      "{}{:2}{:2}",
      match self.dt.tz_seconds().signum() {
        0.. => '+',
        ..=-1 => '-',
      },
      self.dt.tz_seconds() / 60,
      self.dt.tz_seconds() % 60,
    )
  }
}

impl<'a> Debug for FormattedDateTime<'a> {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    Display::fmt(self, f)
  }
}

impl<'a> Display for FormattedDateTime<'a> {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    // Iterate over the format string and consume it.
    let dt = self.dt;
    let mut flag = false;
    let mut padding = Padding::Default;
    let mut prefix = None;
    let mut div = 1;
    for c in self.format.chars() {
      if flag {
        // Apply padding if this is a padding change.
        #[rustfmt::skip]
        match c {
          '0' => { padding = Padding::Zero; continue; },
          '-' => { padding = Padding::Suppress; continue; },
          '_' => { padding = Padding::Space; continue; },
          '.' => { prefix = Some('.'); continue; },
          '3' => { div = 1_000_000; continue; },
          '6' => { div = 1_000; continue; },
          _ => {},
        };

        if c != 'f' && (div != 1 || prefix.is_some()) {
          panic!("Invalid modifier; `.`, `3`, and `6` only allowed on `f` (fractional seconds).");
        }

        // Set up a macro to process padding.
        macro_rules! write_padded {
          ($f:ident, $pad:ident, $level:literal, $e:expr) => {
            match $pad {
              Padding::Default | Padding::Zero => write!($f, concat!("{:0", $level, "}"), $e),
              Padding::Space => write!($f, concat!("{:", $level, "}"), $e),
              Padding::Suppress => write!($f, "{}", $e),
            }
          };
        }

        // Write out the formatted component.
        flag = false;
        match c {
          'Y' => write_padded!(f, padding, 4, dt.year())?,
          'C' => write_padded!(f, padding, 2, dt.year() / 100)?,
          'y' => write_padded!(f, padding, 2, dt.year() % 100)?,
          'm' => write_padded!(f, padding, 2, dt.month())?,
          'b' | 'h' => write!(f, "{}", dt.month_abbv())?,
          'B' => write!(f, "{}", dt.month_name())?,
          'd' => write_padded!(f, padding, 2, dt.day())?,
          'a' => write!(f, "{}", dt.weekday().to_string().chars().take(3).collect::<String>())?,
          'A' => write!(f, "{}", dt.weekday())?,
          'w' => write!(f, "{}", dt.weekday() as u8)?,
          'u' => write!(f, "{}", match dt.weekday() {
            crate::Weekday::Sunday => 7,
            _ => self.dt.weekday() as u8,
          })?,
          // U, W
          'j' => write_padded!(f, padding, 3, dt.day_of_year())?,
          'H' => write_padded!(f, padding, 2, dt.hour())?,
          'I' => write_padded!(f, padding, 2, match dt.hour() {
            0 => 12,
            1..=12 => dt.hour(),
            13.. => dt.hour() - 12,
          })?,
          'M' => write_padded!(f, padding, 2, dt.minute())?,
          'S' => write_padded!(f, padding, 2, dt.second())?,
          'z' => write!(f, "{}", self.offset())?,
          'P' => write!(f, "{}", if dt.hour() > 12 { "PM" } else { "AM" })?,
          'p' => write!(f, "{}", if dt.hour() > 12 { "pm" } else { "am" })?,
          's' => write!(f, "{}", dt.seconds)?,
          'f' => {
            if let Some(pre) = prefix {
              f.write_char(pre)?;
            }
            match div {
              1_000 => write!(f, "{:06}", dt.nanosecond() / div)?,
              1_000_000 => write!(f, "{:03}", dt.nanosecond() / div)?,
              _ => write!(f, "{:09}", dt.nanosecond())?,
            };
            prefix = None;
            div = 1;
          },
          'D' => write!(f, "{:02}/{:02}/{:02}", dt.month(), dt.day(), dt.year())?,
          'F' => write!(f, "{:04}-{:02}-{:02}", dt.year(), dt.month(), dt.day())?,
          'v' => write!(f, "{:2}-{}-{:04}", dt.day(), dt.month_abbv(), dt.year())?,
          'R' => write!(f, "{:2}:{:2}", dt.hour(), dt.minute())?,
          'T' => write!(f, "{:2}:{:2}:{:2}", dt.hour(), dt.minute(), dt.second())?,
          't' => f.write_char('\t')?,
          'n' => f.write_char('\n')?,
          '%' => f.write_char('%')?,
          _ => Err(Error)?,
        }
      } else if c == '%' {
        flag = true;
        padding = Padding::Default;
      } else {
        f.write_char(c)?;
      }
    }
    Ok(())
  }
}

impl<'a> PartialEq<&str> for FormattedDateTime<'a> {
  fn eq(&self, other: &&str) -> bool {
    &self.to_string().as_str() == other
  }
}

macro_rules! month_str {
  ($($num:literal => $short:ident ~ $long:ident)*) => {
    impl DateTime {
      /// The English name of the month.
      const fn month_name(&self) -> &'static str {
        match self.month() {
          $($num => stringify!($long),)*
          #[cfg(not(tarpaulin_include))]
          _ => panic!("Fictitious month"),
        }
      }

      /// The three-letter abbreviation of the month.
      const fn month_abbv(&self) -> &'static str {
        match self.month() {
          $($num => stringify!($short),)*
          #[cfg(not(tarpaulin_include))]
          _ => panic!("Fictitious month"),
        }
      }
    }
  }
}
month_str! {
   1 => Jan ~ January
   2 => Feb ~ February
   3 => Mar ~ March
   4 => Apr ~ April
   5 => May ~ May
   6 => Jun ~ June
   7 => Jul ~ July
   8 => Aug ~ August
   9 => Sep ~ September
  10 => Oct ~ October
  11 => Nov ~ November
  12 => Dec ~ December
}

/// A padding modifier
enum Padding {
  /// Use the default padding (usually either `0` or nothing).
  Default,
  /// Explicitly pad with `0`
  Zero,
  /// Explicitly pad with ` `.
  Space,
  /// Explicitly prevent padding, even if the token has default padding.
  Suppress,
}

#[cfg(test)]
mod tests {
  use assert2::check;

  use crate::datetime;

  #[test]
  fn test_format() {
    let date = datetime! { 2012-04-21 11:00:00 };
    for (fmt_string, date_str) in [
      ("%Y-%m-%d", "2012-04-21"),
      ("%F", "2012-04-21"),
      ("%v", "21-Apr-2012"),
      ("%Y-%m-%d %H:%M:%S", "2012-04-21 11:00:00"),
      ("%Y-%m-%d %I:%M:%S %P", "2012-04-21 11:00:00 AM"),
      ("%H:%M:%S", "11:00:00"),
      ("%B %-d, %Y", "April 21, 2012"),
      ("%B %-d, %C%y", "April 21, 2012"),
      ("%A, %B %-d, %Y", "Saturday, April 21, 2012"),
      ("%d %h %Y", "21 Apr 2012"),
      ("%a %d %b %Y", "Sat 21 Apr 2012"),
      ("%m/%d/%y", "04/21/12"),
      ("year: %Y / day: %j", "year: 2012 / day: 112"),
      ("%%", "%"),
      ("%w %u", "6 6"),
      ("%t %n", "\t \n"),
    ] {
      check!(date.format(fmt_string).to_string() == date_str);
      check!(date.format(fmt_string) == date_str);
      check!(format!("{:?}", date.format(fmt_string)) == date_str);
    }
  }

  #[test]
  fn test_padding() {
    let date = datetime! { 2024-07-04 17:30:00 };
    for (fmt_string, date_str) in
      [("%Y-%m-%d", "2024-07-04"), ("%B %-d, %Y", "July 4, 2024"), ("%-d-%h-%Y", "4-Jul-2024")]
    {
      check!(date.format(fmt_string).to_string() == date_str);
      check!(date.format(fmt_string) == date_str);
    }
  }
}
