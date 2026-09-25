//! A small Bencode encoder and decoder for integers, byte strings, lists, and dictionaries.
//!
//! Decoded byte strings own their payload in reference-counted `Bytes` storage
//! and can outlive the input. Malformed input and unsupported value types are
//! represented by [`DecodeError`]; encoding failures use [`EncodeError`].
mod bencode;

pub use bencode::{
    DecodeError, DecodeOptions, DecodeOptionsBuilder, DecodeOutput, DecodeWarning, Decoder,
    DictionaryKeyPolicy, EncodeError, EncodeOptions, Encoder, Value, decode, decode_with_options,
    decode_with_warnings, encode, encode_with_options,
};
