//! Macro for converting from a domain-specific interval language to nanoseconds.
//!
//! This crate is an implementation detail for `datetime-rs`. You should not depend on it directly,
//! and its contents are subject to change.

use proc_macro::TokenStream;

/// Create an expression of seconds and microseconds from a domain-specific language.
///
/// This macro is private API that powers the `datetime::time_interval!` macro. It should not be
/// used directly.
#[proc_macro]
pub fn nanoseconds(tokens: TokenStream) -> TokenStream {
  datetime_rs_codegen::nanoseconds(tokens.into()).into()
}
