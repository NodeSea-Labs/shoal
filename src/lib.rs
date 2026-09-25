//! A small parser for Bencode integers, byte strings, lists, and dictionaries.
//!
//! Decoded byte strings own their payload in reference-counted `Bytes` storage
//! and can outlive the input. Malformed input and unsupported value types are
//! represented by [`DecodeError`].
mod bencode;

pub use bencode::{
    DecodeError, DecodeOptions, DecodeOptionsBuilder, DecodeOutput, DecodeWarning, Decoder,
    DictionaryKeyPolicy, Value, decode, decode_with_options, decode_with_warnings,
};
