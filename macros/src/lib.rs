//! Macro for converting from a domain-specific interval language to nanoseconds.
//!
//! This crate is an implementation detail for `datetime-rs`. You should not depend on it directly,
//! and its contents are subject to change.

use std::sync::LazyLock;

use proc_macro::TokenStream;
use proc_macro2::TokenTree;
use quote::quote;
use regex::Regex;
use syn::Result;
use syn::Token;
use syn::parse::Parse;
use syn::parse::ParseStream;

/// Create an expression of seconds and microseconds from a domain-specific language.
///
/// This macro is private API that powers the `datetime::time_delta!` macro. It should not be used
/// directly.
#[proc_macro]
pub fn nanoseconds(tokens: TokenStream) -> TokenStream {
  let delta = match syn::parse::<Delta>(tokens) {
    Ok(delta) => delta,
    Err(err) => return err.into_compile_error().into(),
  };
  let nanos = delta.nanoseconds;
  quote! { #nanos }.into()
}

struct Delta {
  nanoseconds: i128,
}

#[derive(Debug, Default)]
struct Pieces {
  days: i64,
  hours: i64,
  minutes: i64,
  seconds: i64,
  nanos: u32,
}

impl Pieces {
  fn as_seconds(&self) -> i64 {
    (self.days * 86_400) + (self.hours * 3_600) + (self.minutes * 60) + self.seconds
  }
}

impl From<Pieces> for Delta {
  fn from(p: Pieces) -> Self {
    Self { nanoseconds: p.as_seconds() as i128 * 1_000_000_000 + p.nanos as i128 }
  }
}

impl Parse for Delta {
  fn parse(input: ParseStream) -> Result<Self> {
    // Do we have an operator? Determine our multiplier.
    let signum = match input.peek(Token![+]) || input.peek(Token![-]) {
      true => match input.parse::<syn::BinOp>()? {
        syn::BinOp::Add(_) => 1,
        syn::BinOp::Sub(_) => -1,
        _ => unreachable!("Token must be + or -."),
      },
      false => 1,
    };

    macro_rules! err {
      ($span:expr, $msg:literal $(,)? $($args:expr),*) => {
        syn::Error::new($span, format!($msg, $($args),*))
      }
    }

    // Parse out the strings of the individual deltas.
    let mut pieces = Pieces::default();
    while let Ok(token) = input.parse::<TokenTree>() {
      let delta = token.to_string();
      let captures = (DELTA_STRING.captures(delta.as_str()))
        .ok_or_else(|| err!(token.span(), "Invalid duration string: {delta}"))?;

      // Add the individual captured components to the total seconds and nanos.
      macro_rules! capture_piece {
      ($captures:ident[$index:literal] $trim:literal $p:ident $tokens:ident $unit:ident) => {{
        let secs = $captures.get($index)
          .map(|i| i.as_str().trim_end_matches($trim))
          .map(|s| s.parse::<i64>().map_err(|_| {
            let err_msg = stringify!(invalid $unit);
            err!($tokens.span(), "{err_msg}")
          }))
          .transpose()?
          .unwrap_or_default();
        let current = $p.as_seconds();
        if current != 0 && secs > current.abs() {
          return Err(err!($tokens.span(), "Place only larger units of time before {}.", stringify!($unit)))?;
        }
        if secs != 0 && $p.$unit != 0 {
          return Err(err!($tokens.span(), "Only declare {} once.", stringify!($unit)));
        }
        secs
        }};
      }
      pieces.days += capture_piece!(captures[1] 'd' pieces token days) * signum;
      pieces.hours += capture_piece!(captures[2] 'h' pieces token hours) * signum;
      pieces.minutes += capture_piece!(captures[3] 'm' pieces token minutes) * signum;
      let (secs, nanoseconds) = captures
        .get(4)
        .map(|s| -> syn::Result<(i64, u32)> {
          let split = s.as_str().trim_end_matches('s').split('.').collect::<Vec<&str>>();
          match split.len() == 1 {
            true => Ok((
              split[0].parse::<i64>().map_err(|_| err!(token.span(), "invalid seconds"))? * signum,
              0,
            )),
            false => {
              if split[1].len() > 9 {
                Err(err!(token.span(), "Offset precision greater than nanoseconds"))?;
              }
              let mut s = split[0].parse::<i64>().unwrap() * signum;
              let mut n = split[1].parse::<u32>().unwrap() * 10u32.pow(9 - split[1].len() as u32);
              // The nanos aren't signum-aware, so if this is a negative delta, invert the nanos.
              if s <= 0 && n != 0 {
                s -= 1;
                n = 1_000_000_000 - n;
              }
              Ok((s, n))
            },
          }
        })
        .transpose()?
        .unwrap_or_default();

      // Make sure separate pieces come in the right order.
      if pieces.as_seconds() != 0 && secs > pieces.as_seconds().abs() {
        Err(err!(token.span(), "Place only larger units of time before seconds."))?;
      }
      if pieces.nanos > 0 && nanoseconds > 0 {
        Err(err!(token.span(), "Fractional seconds may only be declared once."))?;
      }

      // Increment total seconds and nanos.
      pieces.seconds += secs;
      pieces.nanos += nanoseconds;
    }

    // Done; return the seconds and nanoseconds.
    Ok(pieces.into())
  }
}

static DELTA_STRING: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"^([\d]+d)?([\d]+h)?([\d]+m)?([\d]+\.?[\d]*s)?$").expect("valid regex")
});
