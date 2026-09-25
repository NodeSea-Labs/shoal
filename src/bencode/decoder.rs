use crate::{DecodeError, bencode::Value};

/// Maximum number of nested lists and dictionaries accepted by the decoder.
const MAX_NESTING_DEPTH: usize = 100;

/// Decodes supported Bencode values from an input buffer.
///
/// The decoder borrows the input while parsing, but copies each byte-string
/// payload into owned [`bytes::Bytes`] storage. Decoded values can therefore
/// outlive both the decoder and the input buffer. Each parsing operation
/// returns the next unread byte so values can be decoded sequentially inside
/// containers.
pub struct Decoder<'a> {
    /// The complete encoded input being decoded.
    input: &'a [u8],
}

/// A decoded value and the absolute offset of the next unread input byte.
type ParseResult = Result<(Value, usize), DecodeError>;

impl<'a> Decoder<'a> {
    /// Creates a decoder that borrows `input` for the lifetime `'a`.
    pub fn new(input: &'a [u8]) -> Self {
        Self { input }
    }

    /// Decodes exactly one complete Bencode value.
    ///
    /// Integers, byte strings, lists, and dictionaries are supported. Lists
    /// and dictionary values may contain any supported value recursively;
    /// dictionary keys must be byte strings. Byte-string payloads are copied
    /// into owned [`bytes::Bytes`] values, so the decoded value does not borrow
    /// from the input.
    ///
    /// # Errors
    ///
    /// Returns an error for empty input, unsupported value markers, malformed
    /// values, incomplete input, nesting deeper than 100 containers, or bytes
    /// remaining after the decoded value.
    pub fn decode(&self) -> Result<Value, DecodeError> {
        if self.input.is_empty() {
            return Err(DecodeError::EmptyInput);
        }

        // Parse from the root, then enforce the single-value contract here;
        // nested parsers must leave following bytes available to their parent.
        let (value, next_offset) = Self::parse_value_at(self.input, 0, 0)?;
        if next_offset < self.input.len() {
            return Err(DecodeError::TrailingData {
                remaining: self.input.len() - next_offset,
            });
        }

        Ok(value)
    }

    /// Selects a parser from the value marker at `offset`.
    ///
    /// The returned offset is absolute in `input`, allowing a container parser
    /// to continue at the next value without rescanning already parsed bytes.
    fn parse_value_at(input: &'a [u8], offset: usize, nesting_depth: usize) -> ParseResult {
        let Some(&marker) = input.get(offset) else {
            return Err(DecodeError::UnexpectedEndOfInput {
                expected: 1,
                actual: 0,
            });
        };

        // Bound recursive containers before descending so hostile nesting
        // cannot grow the call stack without limit.
        match marker {
            b'i' => Self::parse_integer_at(input, offset),
            b'0'..=b'9' => Self::parse_byte_string_at(input, offset),
            b'l' | b'd' if nesting_depth >= MAX_NESTING_DEPTH => Err(DecodeError::NestingTooDeep {
                limit: MAX_NESTING_DEPTH,
            }),
            b'l' => Self::parse_list_at(input, offset, nesting_depth + 1),
            b'd' => Self::parse_dictionary_at(input, offset, nesting_depth + 1),
            _ => Err(DecodeError::UnsupportedType { marker }),
        }
    }

    /// Parses an integer beginning at `offset` and returns its next offset.
    ///
    /// The terminator search is relative to the current value, while the
    /// returned offset remains absolute in the original input. Leading zeroes
    /// and negative zero are rejected to enforce canonical integer encoding;
    /// values outside `i64` are rejected because [`Value::Integer`] uses `i64`.
    fn parse_integer_at(input: &'a [u8], offset: usize) -> ParseResult {
        let remaining = input.get(offset..).ok_or(DecodeError::InvalidInteger)?;
        if remaining.first() != Some(&b'i') {
            return Err(DecodeError::InvalidInteger);
        }

        let Some(end_relative) = remaining.iter().position(|&byte| byte == b'e') else {
            return Err(DecodeError::InvalidInteger);
        };

        // Search only after this integer's marker so a parent can pass a slice
        // containing later sibling values without changing this value's end.
        let number = &remaining[1..end_relative];
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
        let next_offset = offset + end_relative + 1;

        Ok((Value::Integer(value), next_offset))
    }

    /// Parses a byte string beginning at `offset` and returns its next offset.
    ///
    /// Only bytes before the first colon form the decimal length. The payload
    /// is sliced from the current offset because it may contain arbitrary
    /// bytes, including colons and Bencode markers. Bytes after the declared
    /// payload belong to the enclosing value and are left for its parser.
    fn parse_byte_string_at(input: &'a [u8], offset: usize) -> ParseResult {
        let remaining = input
            .get(offset..)
            .ok_or(DecodeError::InvalidByteStringLength)?;
        let Some(separator) = remaining.iter().position(|&byte| byte == b':') else {
            return Err(DecodeError::InvalidByteStringLength);
        };

        let length_bytes = &remaining[..separator];
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
        // Treat the declared payload as opaque bytes: a payload may itself
        // contain colons or Bencode markers, so scanning it would mis-parse it.
        let payload = &remaining[separator + 1..];
        if payload.len() < length {
            return Err(DecodeError::UnexpectedEndOfInput {
                expected: length,
                actual: payload.len(),
            });
        }

        let next_offset = offset + separator + 1 + length;
        Ok((
            Value::Bytes(bytes::Bytes::copy_from_slice(&payload[..length])),
            next_offset,
        ))
    }

    /// Parses a list beginning at `offset` and returns the offset after `e`.
    ///
    /// A list may contain any supported value. Each child parser advances the
    /// cursor by returning its next absolute offset; the list terminator is
    /// consumed by the list parser rather than treated as a value marker.
    fn parse_list_at(input: &'a [u8], offset: usize, nesting_depth: usize) -> ParseResult {
        if input.get(offset) != Some(&b'l') {
            return Err(DecodeError::InvalidList);
        }

        let mut values = Vec::new();
        let mut cursor = offset + 1;
        loop {
            match input.get(cursor) {
                // Only this list parser consumes its terminator; child parsers
                // return before it so nested and adjacent lists remain distinct.
                Some(b'e') => return Ok((Value::List(values), cursor + 1)),
                None => {
                    return Err(DecodeError::UnexpectedEndOfInput {
                        expected: 1,
                        actual: 0,
                    });
                }
                Some(_) => {
                    let (value, next_offset) = Self::parse_value_at(input, cursor, nesting_depth)?;
                    values.push(value);
                    // Every successful child must advance the absolute cursor;
                    // this keeps siblings parseable without copying input.
                    cursor = next_offset;
                }
            }
        }
    }

    /// Parses alternating byte-string keys and values until the dictionary
    /// terminator.
    ///
    /// The returned offset points immediately after the dictionary's `e`.
    fn parse_dictionary_at(input: &'a [u8], offset: usize, nesting_depth: usize) -> ParseResult {
        if input.get(offset) != Some(&b'd') {
            return Err(DecodeError::InvalidDictionary);
        }

        let mut entries = Vec::new();
        let mut cursor = offset + 1;
        loop {
            match input.get(cursor) {
                None => {
                    return Err(DecodeError::UnexpectedEndOfInput {
                        expected: 1,
                        actual: 0,
                    });
                }
                Some(b'e') => return Ok((Value::Dictionary(entries), cursor + 1)),
                Some(_) => {
                    let (key, value_offset) = Self::parse_byte_string_at(input, cursor)?;
                    // A dictionary terminator can close the container, but it
                    // cannot stand in for the value paired with this key.
                    if input.get(value_offset) == Some(&b'e') {
                        return Err(DecodeError::MissingDictionaryValue);
                    }
                    let (value, next_entry_offset) =
                        Self::parse_value_at(input, value_offset, nesting_depth)?;
                    let Value::Bytes(key_bytes) = key else {
                        return Err(DecodeError::InvalidDictionary);
                    };
                    entries.push((key_bytes, value));
                    cursor = next_entry_offset;
                }
            }
        }
    }
}
