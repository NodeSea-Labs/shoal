use crate::{DecodeError, bencode::Value};
use std::collections::HashSet;

/// Maximum number of nested lists and dictionaries accepted by the decoder.
const MAX_NESTING_DEPTH: usize = 100;
/// Maximum number of values and dictionary keys decoded by default.
const MAX_TOKENS: usize = 2_000_000;

/// Resource and validation options applied while decoding a Bencode value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeOptions {
    max_nesting_depth: usize,
    max_tokens: usize,
    dictionary_key_policy: DictionaryKeyPolicy,
}

impl Default for DecodeOptions {
    fn default() -> Self {
        Self {
            max_nesting_depth: MAX_NESTING_DEPTH,
            max_tokens: MAX_TOKENS,
            dictionary_key_policy: DictionaryKeyPolicy::RejectNonCanonical,
        }
    }
}

impl DecodeOptions {
    /// Starts building options from their defaults.
    pub fn builder() -> DecodeOptionsBuilder {
        DecodeOptionsBuilder::default()
    }

    /// Returns the maximum number of nested lists and dictionaries.
    pub fn max_nesting_depth(self) -> usize {
        self.max_nesting_depth
    }

    /// Returns the maximum number of decoded values and dictionary keys.
    pub fn max_tokens(self) -> usize {
        self.max_tokens
    }

    /// Returns the policy used for unordered or duplicate dictionary keys.
    pub fn dictionary_key_policy(self) -> DictionaryKeyPolicy {
        self.dictionary_key_policy
    }
}

/// Controls handling of dictionary keys that are unordered or duplicated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DictionaryKeyPolicy {
    /// Reject keys that are not strictly increasing in raw byte order.
    #[default]
    RejectNonCanonical,
    /// Preserve keys as they appear, including unordered and duplicate keys.
    /// Use [`Decoder::decode_with_warnings`] to inspect non-fatal diagnostics.
    PreserveInput,
}

/// A decoded value together with any non-fatal canonicality warnings.
#[derive(Debug, PartialEq, Eq)]
pub struct DecodeOutput {
    /// The parsed Bencode value.
    pub value: Value,
    /// Non-fatal issues found while decoding in a permissive mode.
    pub warnings: Vec<DecodeWarning>,
}

impl DecodeOutput {
    /// Returns the parsed Bencode value.
    pub fn value(&self) -> &Value {
        &self.value
    }

    /// Returns the non-fatal warnings produced while decoding.
    pub fn warnings(&self) -> &[DecodeWarning] {
        &self.warnings
    }

    /// Returns whether decoding produced any non-fatal warnings.
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Returns the decoded value and warnings, consuming this output.
    pub fn into_parts(self) -> (Value, Vec<DecodeWarning>) {
        (self.value, self.warnings)
    }

    /// Returns the decoded value, discarding any warnings.
    pub fn into_value(self) -> Value {
        self.value
    }
}

/// A non-fatal issue reported by permissive decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeWarning {
    /// A dictionary key sorts before the preceding key by raw byte order.
    UnsortedDictionaryKey {
        /// The key that appeared out of order.
        key: bytes::Bytes,
        /// The key immediately preceding it in the encoded input.
        previous_key: bytes::Bytes,
    },
    /// A dictionary key duplicates an earlier key in the same dictionary.
    DuplicateDictionaryKey {
        /// The repeated key.
        key: bytes::Bytes,
    },
}

/// Builder for decoder resource and validation options.
#[derive(Debug, Clone, Copy, Default)]
pub struct DecodeOptionsBuilder {
    options: DecodeOptions,
}

impl DecodeOptionsBuilder {
    /// Sets the maximum number of nested lists and dictionaries.
    pub fn max_nesting_depth(mut self, max_nesting_depth: usize) -> Self {
        self.options.max_nesting_depth = max_nesting_depth;
        self
    }

    /// Sets the maximum number of values and dictionary keys.
    pub fn max_tokens(mut self, max_tokens: usize) -> Self {
        self.options.max_tokens = max_tokens;
        self
    }

    /// Sets how unordered or duplicate dictionary keys are handled.
    pub fn dictionary_key_policy(mut self, policy: DictionaryKeyPolicy) -> Self {
        self.options.dictionary_key_policy = policy;
        self
    }

    /// Builds the decoder options.
    pub fn build(self) -> DecodeOptions {
        self.options
    }
}

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
    options: DecodeOptions,
}

/// Decodes exactly one Bencode value using the default options.
///
/// Use [`decode_with_options`] to customize resource limits or dictionary-key
/// handling, or [`decode_with_warnings`] to retain non-fatal diagnostics.
pub fn decode(input: &[u8]) -> Result<Value, DecodeError> {
    Decoder::new(input).decode()
}

/// Decodes exactly one Bencode value using caller-provided options.
///
/// This returns only the value and discards non-fatal warnings. Use
/// [`decode_with_warnings`] when warnings must be inspected.
pub fn decode_with_options(input: &[u8], options: DecodeOptions) -> Result<Value, DecodeError> {
    Decoder::with_options(input, options).decode()
}

/// Decodes one Bencode value and returns any non-fatal canonicality warnings.
pub fn decode_with_warnings(
    input: &[u8],
    options: DecodeOptions,
) -> Result<DecodeOutput, DecodeError> {
    Decoder::with_options(input, options).decode_with_warnings()
}

/// A decoded value and the absolute offset of the next unread input byte.
type ParseResult = Result<(Value, usize), DecodeError>;

impl<'a> Decoder<'a> {
    /// Creates a decoder that borrows `input` for the lifetime `'a`.
    pub fn new(input: &'a [u8]) -> Self {
        Self::with_options(input, DecodeOptions::default())
    }

    /// Creates a decoder with caller-defined parsing and validation options.
    pub fn with_options(input: &'a [u8], options: DecodeOptions) -> Self {
        Self { input, options }
    }

    /// Decodes exactly one complete Bencode value.
    ///
    /// Integers, byte strings, lists, and dictionaries are supported. Lists
    /// and dictionary values may contain any supported value recursively;
    /// dictionary keys must be byte strings. Byte-string payloads are copied
    /// into owned [`bytes::Bytes`] values, so the decoded value does not borrow
    /// from the input.
    ///
    /// This convenience method discards non-fatal warnings. Use
    /// [`Self::decode_with_warnings`] when permissive parsing is configured and
    /// callers need to inspect canonicality issues.
    ///
    /// # Errors
    ///
    /// Returns an error for empty input, unsupported value markers, malformed
    /// values, incomplete input, configured nesting or token limits, non-canonical
    /// dictionary keys under the default policy, or bytes remaining after the
    /// decoded value.
    pub fn decode(&self) -> Result<Value, DecodeError> {
        self.decode_with_warnings().map(|output| output.value)
    }

    /// Decodes one value and returns non-fatal canonicality warnings.
    ///
    /// With [`DictionaryKeyPolicy::PreserveInput`], unordered and duplicate
    /// dictionary keys are retained in the decoded value and reported in
    /// `warnings`. Under the default strict policy, these conditions remain
    /// fatal errors instead. This method does not log or print diagnostics.
    pub fn decode_with_warnings(&self) -> Result<DecodeOutput, DecodeError> {
        if self.input.is_empty() {
            return Err(DecodeError::EmptyInput);
        }

        // Parse from the root, then enforce the single-value contract here;
        // nested parsers must leave following bytes available to their parent.
        let mut token_count = 0;
        let mut warnings = Vec::new();
        let (value, next_offset) = Self::parse_value_at(
            self.input,
            0,
            0,
            &mut token_count,
            &mut warnings,
            self.options,
        )?;
        if next_offset < self.input.len() {
            return Err(DecodeError::TrailingData {
                remaining: self.input.len() - next_offset,
            });
        }

        Ok(DecodeOutput { value, warnings })
    }

    /// Selects a parser from the value marker at `offset`.
    ///
    /// The returned offset is absolute in `input`, allowing a container parser
    /// to continue at the next value without rescanning already parsed bytes.
    fn parse_value_at(
        input: &'a [u8],
        offset: usize,
        nesting_depth: usize,
        token_count: &mut usize,
        warnings: &mut Vec<DecodeWarning>,
        options: DecodeOptions,
    ) -> ParseResult {
        Self::consume_token(token_count, options.max_tokens)?;
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
            b'l' | b'd' if nesting_depth >= options.max_nesting_depth => {
                Err(DecodeError::NestingTooDeep {
                    limit: options.max_nesting_depth,
                })
            }
            b'l' => Self::parse_list_at(
                input,
                offset,
                nesting_depth + 1,
                token_count,
                warnings,
                options,
            ),
            b'd' => Self::parse_dictionary_at(
                input,
                offset,
                nesting_depth + 1,
                token_count,
                warnings,
                options,
            ),
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
    fn parse_list_at(
        input: &'a [u8],
        offset: usize,
        nesting_depth: usize,
        token_count: &mut usize,
        warnings: &mut Vec<DecodeWarning>,
        options: DecodeOptions,
    ) -> ParseResult {
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
                    let (value, next_offset) = Self::parse_value_at(
                        input,
                        cursor,
                        nesting_depth,
                        token_count,
                        warnings,
                        options,
                    )?;
                    values.push(value);
                    // Every successful child must advance the absolute cursor;
                    // this keeps siblings parseable without copying input.
                    cursor = next_offset;
                }
            }
        }
    }

    /// Parses alternating byte-string keys and values until the dictionary
    /// terminator. Depending on the configured policy, keys must be strictly
    /// increasing in raw byte order or are preserved and reported as warnings.
    ///
    /// The returned offset points immediately after the dictionary's `e`.
    fn parse_dictionary_at(
        input: &'a [u8],
        offset: usize,
        nesting_depth: usize,
        token_count: &mut usize,
        warnings: &mut Vec<DecodeWarning>,
        options: DecodeOptions,
    ) -> ParseResult {
        if input.get(offset) != Some(&b'd') {
            return Err(DecodeError::InvalidDictionary);
        }

        let mut entries = Vec::new();
        // Strict mode only needs the preceding key. Permissive mode must also
        // detect duplicates separated by other (possibly unordered) entries.
        let mut seen_keys = (options.dictionary_key_policy == DictionaryKeyPolicy::PreserveInput)
            .then(HashSet::<bytes::Bytes>::new);
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
                    if !input[cursor].is_ascii_digit() {
                        return Err(DecodeError::InvalidDictionaryKey);
                    }
                    Self::consume_token(token_count, options.max_tokens)?;
                    let (key, value_offset) = Self::parse_byte_string_at(input, cursor)?;
                    // A dictionary terminator can close the container, but it
                    // cannot stand in for the value paired with this key.
                    if input.get(value_offset) == Some(&b'e') {
                        return Err(DecodeError::MissingDictionaryValue);
                    }
                    let (value, next_entry_offset) = Self::parse_value_at(
                        input,
                        value_offset,
                        nesting_depth,
                        token_count,
                        warnings,
                        options,
                    )?;
                    let Value::Bytes(key_bytes) = key else {
                        return Err(DecodeError::InvalidDictionary);
                    };
                    if let Some((previous_key, _)) = entries.last() {
                        match key_bytes.as_ref().cmp(previous_key.as_ref()) {
                            std::cmp::Ordering::Less => match options.dictionary_key_policy {
                                DictionaryKeyPolicy::RejectNonCanonical => {
                                    return Err(DecodeError::UnsortedDictionaryKey);
                                }
                                DictionaryKeyPolicy::PreserveInput => {
                                    warnings.push(DecodeWarning::UnsortedDictionaryKey {
                                        key: key_bytes.clone(),
                                        previous_key: previous_key.clone(),
                                    });
                                }
                            },
                            std::cmp::Ordering::Equal
                                if options.dictionary_key_policy
                                    == DictionaryKeyPolicy::RejectNonCanonical =>
                            {
                                return Err(DecodeError::DuplicateDictionaryKey);
                            }
                            std::cmp::Ordering::Greater => {}
                            std::cmp::Ordering::Equal => {}
                        }
                    }
                    if let Some(seen_keys) = &mut seen_keys
                        && !seen_keys.insert(key_bytes.clone())
                    {
                        warnings.push(DecodeWarning::DuplicateDictionaryKey {
                            key: key_bytes.clone(),
                        });
                    }
                    entries.push((key_bytes, value));
                    cursor = next_entry_offset;
                }
            }
        }
    }

    /// Counts one parsed value or dictionary key and enforces the token budget.
    fn consume_token(token_count: &mut usize, limit: usize) -> Result<(), DecodeError> {
        if *token_count >= limit {
            return Err(DecodeError::TokenLimitExceeded { limit });
        }
        *token_count += 1;
        Ok(())
    }
}
