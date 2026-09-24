/// An error returned when a Bencode value cannot be decoded.
#[derive(Debug, thiserror::Error)]
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

    /// The declared byte-string payload is shorter than the input provides.
    #[error("incomplete byte string: expected {expected} bytes, got {actual}")]
    UnexpectedEndOfInput { expected: usize, actual: usize },

    /// Bytes remain after the decoded value.
    #[error("{remaining} trailing bytes after Bencode value")]
    TrailingData { remaining: usize },
}
