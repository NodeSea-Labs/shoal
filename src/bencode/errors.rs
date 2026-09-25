/// An error returned when a Bencode value cannot be decoded.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    /// The input buffer is empty.
    #[error("input is empty")]
    EmptyInput,

    /// The input begins with an unsupported Bencode value marker.
    #[error("unsupported Bencode value marker: {marker:#04x}")]
    UnsupportedType { marker: u8 },

    /// An integer does not follow Bencode's integer encoding rules.
    #[error("invalid Bencode integer")]
    InvalidInteger,

    /// A syntactically valid integer is outside the supported `i64` range.
    #[error("Bencode integer is outside the i64 range")]
    IntegerOutOfRange,

    /// The byte-string length prefix is malformed.
    #[error("invalid Bencode byte-string length")]
    InvalidByteStringLength,

    /// The byte-string length cannot be represented by this platform.
    #[error("Bencode byte-string length is outside the usize range")]
    ByteStringLengthOutOfRange,

    /// The input ends before the required encoded bytes are available.
    #[error("unexpected end of input: expected {expected} bytes, got {actual}")]
    UnexpectedEndOfInput { expected: usize, actual: usize },

    /// Bytes remain after the decoded value.
    #[error("{remaining} trailing bytes after Bencode value")]
    TrailingData { remaining: usize },

    /// A list marker was expected but not found.
    #[error("invalid Bencode list")]
    InvalidList,

    /// A dictionary marker was expected but not found.
    #[error("invalid Bencode dictionary")]
    InvalidDictionary,

    /// A dictionary key does not use the byte-string encoding.
    #[error("Bencode dictionary key is not a byte string")]
    InvalidDictionaryKey,

    /// A dictionary key is not followed by a value.
    #[error("Bencode dictionary key is missing its value")]
    MissingDictionaryValue,

    /// The input exceeds the maximum supported nesting depth.
    #[error("Bencode nesting depth exceeds the limit of {limit}")]
    NestingTooDeep { limit: usize },

    /// The input contains more parsed values and dictionary keys than allowed.
    #[error("Bencode token count exceeds the limit of {limit}")]
    TokenLimitExceeded { limit: usize },
}
