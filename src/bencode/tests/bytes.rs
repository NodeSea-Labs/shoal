use crate::{DecodeError, Decoder, Value};

fn assert_bytes(input: &[u8], expected: &[u8]) {
    match Decoder::new(input).decode() {
        Ok(Value::Bytes(actual)) => {
            assert_eq!(actual, expected, "incorrect bytes for input: {input:?}");
        }
        _ => panic!("expected valid bytes for input: {input:?}"),
    }
}

fn assert_invalid(input: &[u8]) {
    assert!(
        Decoder::new(input).decode().is_err(),
        "expected error for invalid input: {input:?}"
    );
}

fn assert_error(input: &[u8], expected: DecodeError) {
    assert_eq!(
        Decoder::new(input).decode(),
        Err(expected),
        "unexpected error for input: {input:?}"
    );
}

#[test]
fn bytes_valid_values() {
    let cases: &[(&[u8], &[u8])] = &[
        (b"0:", b""),
        (b"1:a", b"a"),
        (b"1:0", b"0"),
        (b"1::", b":"),
        (b"1:e", b"e"),
        (b"2:ab", b"ab"),
        (b"5:hello", b"hello"),
        (b"11:hello world", b"hello world"),
        (b"4:i123", b"i123"),
        (b"3:l0:", b"l0:"),
        (b"1: ", b" "),
        (b"1:\n", b"\n"),
        (b"2:\r\n", b"\r\n"),
    ];

    for &(input, expected) in cases {
        assert_bytes(input, expected);
    }
}

#[test]
fn bytes_binary_data() {
    let cases: &[(&[u8], &[u8])] = &[
        // NUL
        (b"1:\x00", b"\x00"),
        // Non-UTF-8 bytes
        (b"1:\xff", b"\xff"),
        (b"1:\x80", b"\x80"),
        // Multiple binary bytes
        (b"3:\x00\xff\x80", b"\x00\xff\x80"),
        // Bencode markers are ordinary data
        (b"4:ilde", b"ilde"),
        // The colon is also ordinary data
        (b"3:a:b", b"a:b"),
        // Embedded NUL
        (b"3:a\x00b", b"a\x00b"),
    ];

    for &(input, expected) in cases {
        assert_bytes(input, expected);
    }
}

#[test]
fn bytes_all_byte_values() {
    let data: Vec<u8> = (u8::MIN..=u8::MAX).collect();

    let mut encoded = b"256:".to_vec();
    encoded.extend_from_slice(&data);

    assert_bytes(&encoded, &data);
}

#[test]
fn bytes_invalid_length_format() {
    let cases: &[(&[u8], DecodeError)] = &[
        (b"1 :a", DecodeError::InvalidByteStringLength),
        (b"1.0:a", DecodeError::InvalidByteStringLength),
        (b"1e:a", DecodeError::InvalidByteStringLength),
        (b"0", DecodeError::InvalidByteStringLength),
        (b"12", DecodeError::InvalidByteStringLength),
    ];

    for &(input, ref expected) in cases {
        assert_eq!(
            Decoder::new(input).decode().as_ref(),
            Err(expected),
            "unexpected error for input: {input:?}"
        );
    }
}

#[test]
fn bytes_rejects_unsupported_root_markers() {
    let cases: &[(&[u8], u8)] = &[
        (b":", b':'),
        (b":hello", b':'),
        (b"-1:a", b'-'),
        (b"+1:a", b'+'),
        (b" 1:a", b' '),
        (b"a:a", b'a'),
        (b"\x001:a", 0),
        (b"\xff1:a", 0xff),
    ];

    for &(input, marker) in cases {
        assert_error(input, DecodeError::UnsupportedType { marker });
    }
}

#[test]
fn bytes_leading_zeros() {
    let cases: &[&[u8]] = &[
        b"00:",
        b"000:",
        b"01:a",
        b"001:a",
        b"0001:a",
        b"05:hello",
        b"005:hello",
        b"010:0123456789",
    ];

    for &input in cases {
        assert_error(input, DecodeError::InvalidByteStringLength);
    }
}

#[test]
fn bytes_length_too_short() {
    let cases: &[(&[u8], usize, usize)] = &[
        (b"1:", 1, 0),
        (b"2:a", 2, 1),
        (b"3:ab", 3, 2),
        (b"5:hell", 5, 4),
        (b"10:hello", 10, 5),
    ];

    for &(input, expected, actual) in cases {
        assert_error(
            input,
            DecodeError::UnexpectedEndOfInput { expected, actual },
        );
    }
}

#[test]
fn bytes_empty_input_reports_empty_input() {
    assert_error(b"", DecodeError::EmptyInput);
}

#[test]
fn bytes_rejects_trailing_data_after_payload() {
    let cases: &[(&[u8], usize)] = &[
        (b"0:a", 1),
        (b"0:0:", 2),
        (b"1:ai1e", 3),
        (b"1:ae", 1),
        (b"1:a ", 1),
        (b"1:a\x00", 1),
        (b"1:ab", 1),
        (b"2:abc", 1),
        (b"3:abcd", 1),
        (b"5:hello!", 1),
        (b"0::", 1),
        (b"1:aehello", 6),
        (b"5:helloi42e", 4),
        (b"i1e:a", 2),
    ];

    for &(input, remaining) in cases {
        assert_error(input, DecodeError::TrailingData { remaining });
    }
}

#[test]
fn bytes_length_overflow() {
    let cases: &[&[u8]] = &[
        b"999999999999999999999999999999:a",
        b"18446744073709551616:a",
    ];

    for &input in cases {
        assert_error(input, DecodeError::ByteStringLengthOutOfRange);
    }
}

#[test]
fn bytes_truncated_input() {
    let valid = b"11:hello world";

    // All proper prefixes must be rejected.
    for end in 0..valid.len() {
        assert_invalid(&valid[..end]);
    }

    assert_bytes(valid, b"hello world");
}

#[test]
fn bytes_large_payload() {
    let data = vec![b'a'; 1_000_000];

    let mut encoded = b"1000000:".to_vec();
    encoded.extend_from_slice(&data);

    assert_bytes(&encoded, &data);
}

#[test]
fn bytes_parses_formatted_length_prefixes() {
    let lengths = [0, 1, 2, 9, 10, 99, 100, 255, 256, 1024, 4096];

    for len in lengths {
        let data = vec![b'x'; len];

        let mut encoded = format!("{len}:").into_bytes();
        encoded.extend_from_slice(&data);

        assert_bytes(&encoded, &data);
    }
}
