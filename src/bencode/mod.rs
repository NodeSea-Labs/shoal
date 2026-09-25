//! Bencode value types, encoding, and parsing support.
//!
//! The decoder and encoder support integers, byte strings, recursively nested
//! lists, and dictionaries with byte-string keys.
mod decoder;
mod encoder;
mod errors;
mod value;

pub use decoder::{
    DecodeOptions, DecodeOptionsBuilder, DecodeOutput, DecodeWarning, Decoder, DictionaryKeyPolicy,
    decode, decode_with_options, decode_with_warnings,
};
pub use encoder::{EncodeOptions, Encoder, encode, encode_with_options};
pub use errors::{DecodeError, EncodeError};
pub use value::Value;

#[cfg(test)]
mod tests;
