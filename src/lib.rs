//! A small parser for the integer and byte-string values used by Bencode.
//!
//! The parser currently recognizes integer and byte-string values. Malformed
//! input and unsupported value types are represented by [`DecodeError`].
mod bencode;

pub use bencode::{DecodeError, Decoder, Value};
