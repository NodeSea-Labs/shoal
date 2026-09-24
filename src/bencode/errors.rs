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
}
