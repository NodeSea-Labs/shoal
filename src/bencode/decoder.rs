use crate::{DecodeError, bencode::Value};

/// Decodes one supported Bencode value from a borrowed input buffer.
///
/// The decoder borrows the input so byte-string values can refer to their
/// payload without allocating or copying it. A decoder consumes exactly one
/// complete value; trailing bytes are reported as an error.
pub struct Decoder<'a> {
    /// The complete encoded value being decoded.
    input: &'a [u8],
}

/// A decoded value borrowing from the input, or the reason decoding failed.
type ParseResult<'a> = Result<Value<'a>, DecodeError>;

impl<'a> Decoder<'a> {
    /// Creates a decoder that borrows `input` for the lifetime `'a`.
    pub fn new(input: &'a [u8]) -> Self {
        Self { input }
    }

    /// Decodes exactly one integer or byte string from the input.
    ///
    /// The returned byte string borrows its payload from the input buffer.
    /// The first byte selects the value parser; unsupported markers are
    /// reported separately from malformed integer or byte-string encodings.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError::EmptyInput`] for an empty buffer,
    /// [`DecodeError::UnsupportedType`] for an unsupported value marker, or a
    /// more specific error when the selected value is malformed, out of range,
    /// incomplete, or followed by extra bytes.
    pub fn decode(&self) -> ParseResult<'a> {
        let Some(&prefix) = self.input.first() else {
            return Err(DecodeError::EmptyInput);
        };

        match prefix {
            b'i' => Self::parse_integer(self.input),
            b'0'..=b'9' => Self::parse_byte_string(self.input),
            marker => Err(DecodeError::UnsupportedType { marker }),
        }
    }

    /// Parses one complete integer and rejects non-canonical representations.
    ///
    /// Bencode integers use decimal digits between `i` and `e`. Leading zeroes
    /// and negative zero are rejected so a number has only one valid encoding;
    /// values outside `i64` are rejected because [`Value::Integer`] stores an
    /// `i64`.
    fn parse_integer(input: &'a [u8]) -> ParseResult<'a> {
        if !input.starts_with(b"i") || !input.ends_with(b"e") {
            return Err(DecodeError::InvalidInteger);
        }

        let number = &input[1..input.len() - 1];
        let digits = number.strip_prefix(b"-").unwrap_or(number);
        if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
            return Err(DecodeError::InvalidInteger);
        }

        if digits[0] == b'0' && (digits.len() > 1 || number.starts_with(b"-")) {
            return Err(DecodeError::InvalidInteger);
        }

        let number = str::from_utf8(number).map_err(|_| DecodeError::InvalidInteger)?;
        let value = number
            .parse::<i64>()
            .map_err(|_| DecodeError::IntegerOutOfRange)?;

        Ok(Value::Integer(value))
    }

    /// Parses one complete byte string using its decimal length prefix.
    ///
    /// Only the bytes before the first colon form the length. The payload is
    /// kept as arbitrary bytes, and its length must match the prefix exactly;
    /// this prevents payload contents from being interpreted as syntax and
    /// prevents trailing input from being silently accepted.
    fn parse_byte_string(input: &'a [u8]) -> ParseResult<'a> {
        let Some(separator) = input.iter().position(|&byte| byte == b':') else {
            return Err(DecodeError::InvalidByteStringLength);
        };

        let length_bytes = &input[..separator];
        if length_bytes.is_empty()
            || !length_bytes.iter().all(u8::is_ascii_digit)
            || (length_bytes.len() > 1 && length_bytes[0] == b'0')
        {
            return Err(DecodeError::InvalidByteStringLength);
        }

        let length_text =
            str::from_utf8(length_bytes).map_err(|_| DecodeError::InvalidByteStringLength)?;
        let length = length_text
            .parse::<usize>()
            .map_err(|_| DecodeError::ByteStringLengthOutOfRange)?;
        let payload = &input[separator + 1..];

        if payload.len() < length {
            return Err(DecodeError::UnexpectedEndOfInput {
                expected: length,
                actual: payload.len(),
            });
        }
        if payload.len() > length {
            return Err(DecodeError::TrailingData {
                remaining: payload.len() - length,
            });
        }

        Ok(Value::ByteString(payload))
    }
}
