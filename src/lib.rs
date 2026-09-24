//! A small parser for Bencode integers, byte strings, and lists.
//!
//! Decoded byte strings own their payload in reference-counted `Bytes` storage
//! and can outlive the input. Malformed input and unsupported value types are
//! represented by [`DecodeError`].
mod bencode;

pub use bencode::{DecodeError, Decoder, Value};
