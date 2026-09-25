use crate::{DecodeError, DecodeOptions, DecodeWarning, Decoder, DictionaryKeyPolicy, Value};
use bytes::Bytes;

fn byte_string(value: &[u8]) -> Bytes {
    Bytes::copy_from_slice(value)
}

fn assert_dictionary(input: &[u8], expected: Vec<(Bytes, Value)>) {
    assert_eq!(
        Decoder::new(input).decode(),
        Ok(Value::Dictionary(expected)),
        "input: {input:?}"
    );
}

fn assert_dictionary_preserving_input(
    input: &[u8],
    expected: Vec<(Bytes, Value)>,
) -> Vec<DecodeWarning> {
    let options = DecodeOptions::builder()
        .dictionary_key_policy(DictionaryKeyPolicy::PreserveInput)
        .build();
    let output = Decoder::with_options(input, options)
        .decode_with_warnings()
        .unwrap_or_else(|error| panic!("unexpected error for {input:?}: {error}"));
    assert_eq!(
        output.value,
        Value::Dictionary(expected),
        "input: {input:?}"
    );
    output.warnings
}

#[test]
fn dictionary_empty() {
    assert_dictionary(b"de", vec![]);
}

#[test]
fn dictionary_decodes_mixed_values() {
    assert_dictionary(
        b"d1:ai12453e1:b3:aaa1:c3:bbbe",
        vec![
            (byte_string(b"a"), Value::Integer(12453)),
            (byte_string(b"b"), Value::Bytes(byte_string(b"aaa"))),
            (byte_string(b"c"), Value::Bytes(byte_string(b"bbb"))),
        ],
    );
}

#[test]
fn dictionary_preserves_libtorrent_unsorted_input_when_configured() {
    // libtorrent parses this input and reports a soft error because `X` sorts
    // before lowercase ASCII keys by raw byte order. PreserveInput keeps the
    // parser permissive and reports a warning, matching libtorrent's behavior.
    let warnings = assert_dictionary_preserving_input(
        b"d1:ai12453e1:b3:aaa1:c3:bbb1:X10:0123456789e",
        vec![
            (byte_string(b"a"), Value::Integer(12453)),
            (byte_string(b"b"), Value::Bytes(byte_string(b"aaa"))),
            (byte_string(b"c"), Value::Bytes(byte_string(b"bbb"))),
            (byte_string(b"X"), Value::Bytes(byte_string(b"0123456789"))),
        ],
    );
    assert_eq!(
        warnings,
        vec![DecodeWarning::UnsortedDictionaryKey {
            key: byte_string(b"X"),
            previous_key: byte_string(b"c"),
        }]
    );
}

#[test]
fn dictionary_preserves_unsorted_input_order_when_configured() {
    // This mirrors libtorrent's dict_at sample: parsing preserves wire order
    // rather than sorting entries into a map.
    let warnings = assert_dictionary_preserving_input(
        b"d3:fooi1e3:bari2ee",
        vec![
            (byte_string(b"foo"), Value::Integer(1)),
            (byte_string(b"bar"), Value::Integer(2)),
        ],
    );
    assert_eq!(
        warnings,
        vec![DecodeWarning::UnsortedDictionaryKey {
            key: byte_string(b"bar"),
            previous_key: byte_string(b"foo"),
        }]
    );
}

#[test]
fn dictionary_reports_libtorrent_unordered_key_warnings() {
    let cases: &[(&[u8], &[u8], &[u8])] = &[
        (b"d2:abi1e2:aai2ee", b"ab", b"aa"),
        (b"d2:bai1e2:aai2ee", b"ba", b"aa"),
        (b"d2:aai1e1:ai2ee", b"aa", b"a"),
    ];

    for &(input, previous_key, key) in cases {
        let output = Decoder::with_options(
            input,
            DecodeOptions::builder()
                .dictionary_key_policy(DictionaryKeyPolicy::PreserveInput)
                .build(),
        )
        .decode_with_warnings()
        .unwrap();

        assert_eq!(
            output.value,
            expected_two_integer_entries(previous_key, key)
        );
        assert_eq!(
            output.warnings,
            vec![DecodeWarning::UnsortedDictionaryKey {
                key: byte_string(key),
                previous_key: byte_string(previous_key),
            }],
            "input: {input:?}"
        );
    }
}

#[test]
fn dictionary_warnings_propagate_from_nested_values() {
    let input = b"ld1:bi1e1:ai2eee";
    let output = Decoder::with_options(
        input,
        DecodeOptions::builder()
            .dictionary_key_policy(DictionaryKeyPolicy::PreserveInput)
            .build(),
    )
    .decode_with_warnings()
    .unwrap();

    assert_eq!(
        output.value,
        Value::List(vec![Value::Dictionary(vec![
            (byte_string(b"b"), Value::Integer(1)),
            (byte_string(b"a"), Value::Integer(2)),
        ])])
    );
    assert_eq!(
        output.warnings,
        vec![DecodeWarning::UnsortedDictionaryKey {
            key: byte_string(b"a"),
            previous_key: byte_string(b"b"),
        }]
    );
}

#[test]
fn dictionary_does_not_warn_for_sorted_libtorrent_key_cases() {
    let cases: &[(&[u8], Value)] = &[
        (
            b"d1:ai1e2:aai2ee",
            Value::Dictionary(vec![
                (byte_string(b"a"), Value::Integer(1)),
                (byte_string(b"aa"), Value::Integer(2)),
            ]),
        ),
        (
            b"d2:aai1e1:bi2ee",
            Value::Dictionary(vec![
                (byte_string(b"aa"), Value::Integer(1)),
                (byte_string(b"b"), Value::Integer(2)),
            ]),
        ),
    ];

    for &(input, ref expected) in cases {
        let output = Decoder::new(input).decode_with_warnings().unwrap();
        assert_eq!(&output.value, expected, "input: {input:?}");
        assert!(output.warnings.is_empty(), "input: {input:?}");
    }
}

fn expected_two_integer_entries(first_key: &[u8], second_key: &[u8]) -> Value {
    Value::Dictionary(vec![
        (byte_string(first_key), Value::Integer(1)),
        (byte_string(second_key), Value::Integer(2)),
    ])
}

#[test]
fn dictionary_rejects_descending_keys_by_default() {
    for input in [
        &b"d2:abi1e2:aai2ee"[..],
        &b"d2:bai1e2:aai2ee"[..],
        &b"d2:aai1e1:ai2ee"[..],
    ] {
        assert_eq!(
            Decoder::new(input).decode(),
            Err(DecodeError::UnsortedDictionaryKey),
            "input: {input:?}"
        );
    }
}

#[test]
fn dictionary_orders_keys_by_raw_bytes() {
    assert_dictionary(
        b"d2:a\xfei1e2:a\xffi2ee",
        vec![
            (byte_string(b"a\xfe"), Value::Integer(1)),
            (byte_string(b"a\xff"), Value::Integer(2)),
        ],
    );

    assert_eq!(
        Decoder::new(b"d2:a\xffi1e2:a\xfei2ee").decode(),
        Err(DecodeError::UnsortedDictionaryKey)
    );
}

#[test]
fn dictionary_accepts_sorted_prefix_keys_by_default() {
    assert_dictionary(
        b"d1:ai1e2:aai2ee",
        vec![
            (byte_string(b"a"), Value::Integer(1)),
            (byte_string(b"aa"), Value::Integer(2)),
        ],
    );
    assert_dictionary(
        b"d2:aai1e1:bi2ee",
        vec![
            (byte_string(b"aa"), Value::Integer(1)),
            (byte_string(b"b"), Value::Integer(2)),
        ],
    );
}

#[test]
fn dictionary_rejects_duplicate_keys_by_default() {
    assert_eq!(
        Decoder::new(b"d2:aai1e2:aai2ee").decode(),
        Err(DecodeError::DuplicateDictionaryKey)
    );
}

#[test]
fn dictionary_preserves_duplicate_keys_when_configured() {
    let warnings = assert_dictionary_preserving_input(
        b"d2:aai1e2:aai2ee",
        vec![
            (byte_string(b"aa"), Value::Integer(1)),
            (byte_string(b"aa"), Value::Integer(2)),
        ],
    );
    assert_eq!(
        warnings,
        vec![DecodeWarning::DuplicateDictionaryKey {
            key: byte_string(b"aa"),
        }]
    );
}

#[test]
fn dictionary_warns_about_non_adjacent_duplicate_keys_when_configured() {
    let warnings = assert_dictionary_preserving_input(
        b"d1:ai1e1:bi2e1:ai3ee",
        vec![
            (byte_string(b"a"), Value::Integer(1)),
            (byte_string(b"b"), Value::Integer(2)),
            (byte_string(b"a"), Value::Integer(3)),
        ],
    );
    assert_eq!(
        warnings,
        vec![
            DecodeWarning::UnsortedDictionaryKey {
                key: byte_string(b"a"),
                previous_key: byte_string(b"b"),
            },
            DecodeWarning::DuplicateDictionaryKey {
                key: byte_string(b"a"),
            },
        ]
    );
}

#[test]
fn dictionary_decodes_nested_values() {
    assert_dictionary(
        b"d4:listli1e1:xe4:nestd1:bi2eee",
        vec![
            (
                byte_string(b"list"),
                Value::List(vec![Value::Integer(1), Value::Bytes(byte_string(b"x"))]),
            ),
            (
                byte_string(b"nest"),
                Value::Dictionary(vec![(byte_string(b"b"), Value::Integer(2))]),
            ),
        ],
    );
}

#[test]
fn dictionary_key_preserves_nul_and_non_utf8_bytes() {
    assert_dictionary(
        b"d4:a\x00b\xffi1ee",
        vec![(byte_string(b"a\x00b\xff"), Value::Integer(1))],
    );
}

#[test]
fn dictionary_decodes_many_entries() {
    let mut input = Vec::from(&b"d"[..]);
    let mut expected = Vec::new();

    for index in 0..1000 {
        let key = format!("{index:04}");
        input.extend_from_slice(b"4:");
        input.extend_from_slice(key.as_bytes());
        input.extend_from_slice(format!("i{index}e").as_bytes());
        expected.push((byte_string(key.as_bytes()), Value::Integer(index)));
    }

    input.push(b'e');
    assert_dictionary(&input, expected);
}

#[test]
fn dictionary_rejects_non_byte_string_key() {
    assert_eq!(
        Decoder::new(b"di5e1:ae").decode(),
        Err(DecodeError::InvalidDictionaryKey)
    );
}

#[test]
fn dictionary_rejects_non_string_key_marker() {
    assert_eq!(
        Decoder::new(b"di2ei0ee").decode(),
        Err(DecodeError::InvalidDictionaryKey)
    );
}

#[test]
fn dictionary_rejects_non_digit_key_prefix() {
    assert_eq!(
        Decoder::new(b"df00:").decode(),
        Err(DecodeError::InvalidDictionaryKey)
    );
}

#[test]
fn dictionary_rejects_key_length_without_separator() {
    assert_eq!(
        Decoder::new(b"d1000").decode(),
        Err(DecodeError::InvalidByteStringLength)
    );
}

#[test]
fn dictionary_rejects_truncated_large_key_payload() {
    assert_eq!(
        Decoder::new(b"d1000:..e").decode(),
        Err(DecodeError::UnexpectedEndOfInput {
            expected: 1000,
            actual: 3,
        })
    );
}

#[test]
fn dictionary_rejects_empty_large_key_payload() {
    assert_eq!(
        Decoder::new(b"d1000:").decode(),
        Err(DecodeError::UnexpectedEndOfInput {
            expected: 1000,
            actual: 0,
        })
    );
}

#[test]
fn dictionary_token_budget_counts_keys_and_values() {
    let options = DecodeOptions::builder().max_tokens(3).build();
    assert_eq!(
        Decoder::with_options(b"d1:ai1ee", options).decode(),
        Ok(Value::Dictionary(vec![(
            byte_string(b"a"),
            Value::Integer(1)
        )]))
    );

    assert_eq!(
        Decoder::with_options(b"d1:ai1e1:bi2ee", options).decode(),
        Err(DecodeError::TokenLimitExceeded { limit: 3 })
    );
}

#[test]
fn dictionary_rejects_key_without_value() {
    assert_eq!(
        Decoder::new(b"d1:ae").decode(),
        Err(DecodeError::MissingDictionaryValue)
    );
}

#[test]
fn dictionary_rejects_missing_terminator() {
    assert_eq!(
        Decoder::new(b"d1:ai1e").decode(),
        Err(DecodeError::UnexpectedEndOfInput {
            expected: 1,
            actual: 0,
        })
    );
}

#[test]
fn dictionary_rejects_truncated_key_payload() {
    assert_eq!(
        Decoder::new(b"d3:ab").decode(),
        Err(DecodeError::UnexpectedEndOfInput {
            expected: 3,
            actual: 2,
        })
    );
}

#[test]
fn dictionary_rejects_every_incomplete_prefix() {
    let input = b"d1:ai1e1:b3:twoe";
    for end in 0..input.len() {
        assert!(
            Decoder::new(&input[..end]).decode().is_err(),
            "accepted truncated input: {:?}",
            &input[..end]
        );
    }

    assert_dictionary(
        input,
        vec![
            (byte_string(b"a"), Value::Integer(1)),
            (byte_string(b"b"), Value::Bytes(byte_string(b"two"))),
        ],
    );
}

#[test]
fn dictionary_rejects_trailing_top_level_data() {
    assert_eq!(
        Decoder::new(b"de0:").decode(),
        Err(DecodeError::TrailingData { remaining: 2 })
    );
}

#[test]
fn dictionary_rejects_excessive_recursive_nesting() {
    const MAX_DEPTH: usize = 100;

    let mut input = Vec::new();
    for _ in 0..=MAX_DEPTH {
        input.extend_from_slice(b"d1:a");
    }
    input.extend_from_slice(b"i0e");
    for _ in 0..=MAX_DEPTH {
        input.extend_from_slice(b"e");
    }

    assert_eq!(
        Decoder::new(&input).decode(),
        Err(DecodeError::NestingTooDeep { limit: MAX_DEPTH })
    );
}

#[test]
fn dictionary_accepts_nesting_at_the_limit() {
    const MAX_DEPTH: usize = 100;

    let mut input = Vec::new();
    for _ in 0..MAX_DEPTH {
        input.extend_from_slice(b"d1:a");
    }
    input.extend_from_slice(b"i0e");
    input.extend(std::iter::repeat_n(b'e', MAX_DEPTH));

    let decoded = Decoder::new(&input).decode().unwrap();
    let mut current = &decoded;
    for _ in 0..MAX_DEPTH {
        let Value::Dictionary(entries) = current else {
            panic!("expected a dictionary at each nesting level");
        };
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, byte_string(b"a"));
        current = &entries[0].1;
    }
    assert_eq!(current, &Value::Integer(0));
}
