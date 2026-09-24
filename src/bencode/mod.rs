//! Bencode value types and parsing support.
//!
//! This module currently handles integer and byte-string values.
mod decoder;
mod errors;
mod value;

pub use decoder::Decoder;
pub use errors::DecodeError;
pub use value::Value;

#[cfg(test)]
mod tests;
