//! Bencode value types and parsing support.
//!
//! The decoder supports integers, byte strings, recursively nested lists, and
//! dictionaries with byte-string keys.
mod decoder;
mod errors;
mod value;

pub use decoder::Decoder;
pub use errors::DecodeError;
pub use value::Value;

#[cfg(test)]
mod tests;
