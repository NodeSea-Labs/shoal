//! Bencode value types and parsing support.
//!
//! The decoder supports integers, byte strings, and recursively nested lists.
mod decoder;
mod errors;
mod value;

pub use decoder::Decoder;
pub use errors::DecodeError;
pub use value::Value;

#[cfg(test)]
mod tests;
