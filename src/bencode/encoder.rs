use crate::{EncodeError, Value};

/// Resource limits applied while encoding a Bencode value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncodeOptions {
    /// Maximum number of lists and dictionaries allowed along a value path.
    ///
    /// A value of `0` allows scalar values but rejects every list or dictionary.
    pub max_nesting_depth: usize,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self {
            max_nesting_depth: 100,
        }
    }
}

/// Encodes Bencode values using the configured resource limits.
///
/// Dictionaries are emitted in raw-byte key order, regardless of the order of
/// entries in [`Value::Dictionary`]. Duplicate keys and values deeper than the
/// configured limit are rejected.
#[derive(Debug, Clone, Copy, Default)]
pub struct Encoder {
    options: EncodeOptions,
}

impl Encoder {
    /// Creates an encoder with the default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an encoder with caller-provided options.
    pub fn with_options(options: EncodeOptions) -> Self {
        Self { options }
    }

    /// Encodes a value as canonical Bencode bytes.
    ///
    /// Dictionary entries are sorted by their raw key bytes. The input value is
    /// borrowed and is not reordered or otherwise modified.
    pub fn encode(&self, value: &Value) -> Result<Vec<u8>, EncodeError> {
        let mut output = Vec::new();
        encode_value(value, &mut output, 0, self.options)?;
        Ok(output)
    }
}

/// Encodes a value as canonical Bencode bytes.
///
/// Dictionary entries are serialized in ascending raw-byte key order. Duplicate
/// keys are rejected because they cannot be represented as a canonical
/// dictionary. Byte-string payloads are written unchanged and may contain any
/// byte values.
pub fn encode(value: &Value) -> Result<Vec<u8>, EncodeError> {
    Encoder::new().encode(value)
}

/// Encodes a value using caller-provided resource limits.
pub fn encode_with_options(value: &Value, options: EncodeOptions) -> Result<Vec<u8>, EncodeError> {
    Encoder::with_options(options).encode(value)
}

/// Recursively appends one value's Bencode representation to `output`.
///
/// `nesting_depth` is the number of enclosing containers, and `options`
/// applies the same depth limit throughout the traversal. Dictionary keys are
/// sorted and validated before their entries are emitted.
fn encode_value(
    value: &Value,
    output: &mut Vec<u8>,
    nesting_depth: usize,
    options: EncodeOptions,
) -> Result<(), EncodeError> {
    // `nesting_depth` counts only containers that enclose this value. Scalars
    // at the configured limit remain valid; another container does not.
    match value {
        Value::Integer(integer) => {
            output.extend_from_slice(format!("i{integer}e").as_bytes());
        }
        Value::Bytes(bytes) => {
            output.extend_from_slice(format!("{}:", bytes.len()).as_bytes());
            output.extend_from_slice(bytes);
        }
        Value::List(values) => {
            check_nesting_depth(nesting_depth, options)?;
            output.push(b'l');
            for value in values {
                encode_value(value, output, nesting_depth + 1, options)?;
            }
            output.push(b'e');
        }
        Value::Dictionary(entries) => {
            check_nesting_depth(nesting_depth, options)?;
            // Sort references so canonical output does not mutate the caller's
            // Vec-backed dictionary or copy its values.
            let mut sorted_entries: Vec<_> = entries.iter().collect();
            sorted_entries.sort_unstable_by(|(left, _), (right, _)| left.cmp(right));

            // Sorting makes every duplicate adjacent, so a single pass can
            // reject duplicates before writing this dictionary's marker.
            for pair in sorted_entries.windows(2) {
                if pair[0].0 == pair[1].0 {
                    return Err(EncodeError::DuplicateDictionaryKey {
                        key: pair[0].0.clone(),
                    });
                }
            }

            output.push(b'd');
            for (key, value) in sorted_entries {
                output.extend_from_slice(format!("{}:", key.len()).as_bytes());
                output.extend_from_slice(key);
                encode_value(value, output, nesting_depth + 1, options)?;
            }
            output.push(b'e');
        }
    }

    Ok(())
}

/// Rejects a container when its zero-based depth would exceed the configured limit.
fn check_nesting_depth(nesting_depth: usize, options: EncodeOptions) -> Result<(), EncodeError> {
    if nesting_depth >= options.max_nesting_depth {
        return Err(EncodeError::NestingTooDeep {
            limit: options.max_nesting_depth,
        });
    }
    Ok(())
}
