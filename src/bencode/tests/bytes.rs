use crate::{Decoder, Value};

fn assert_bytes(input: &[u8], expected: &[u8]) {
    match Decoder::new(input).decode() {
        Ok(Value::ByteString(actual)) => {
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
    let cases: &[&[u8]] = &[
        b":", b":hello", b"-1:a", b"+1:a", b" 1:a", b"1 :a", b"1.0:a", b"1e:a", b"i1e:a", b"a:a",
        b"\x001:a", b"\xff1:a",
    ];

    for &input in cases {
        assert_invalid(input);
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
        assert_invalid(input);
    }
}

#[test]
fn bytes_length_too_short() {
    let cases: &[&[u8]] = &[
        b"1:",
        b"2:a",
        b"3:ab",
        b"5:hell",
        b"10:hello",
        b"1",
        b"12",
        b"123",
    ];

    for &input in cases {
        assert_invalid(input);
    }
}

#[test]
fn bytes_length_too_long() {
    let cases: &[&[u8]] = &[b"0:a", b"1:ab", b"2:abc", b"3:abcd", b"5:hello!", b"0::"];

    for &input in cases {
        assert_invalid(input);
    }
}

#[test]
fn bytes_missing_components() {
    let cases: &[&[u8]] = &[b"", b"0", b"1", b"123", b"hello", b"abc:", b"::"];

    for &input in cases {
        assert_invalid(input);
    }
}

#[test]
fn bytes_extra_data() {
    let cases: &[&[u8]] = &[
        b"0:0:",
        b"1:ai1e",
        b"1:ae",
        b"1:a ",
        b"1:a\x00",
        b"1:ab",
        b"1:aehello",
        b"5:helloi42e",
    ];

    for &input in cases {
        assert_invalid(input);
    }
}

#[test]
fn bytes_length_overflow() {
    let cases: &[&[u8]] = &[
        b"999999999999999999999999999999:a",
        b"18446744073709551616:a",
    ];

    for &input in cases {
        assert_invalid(input);
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
    let data = vec![b'a'; 4096];

    let mut encoded = b"4096:".to_vec();
    encoded.extend_from_slice(&data);

    assert_bytes(&encoded, &data);
}

#[test]
fn bytes_roundtrip_lengths() {
    let lengths = [0, 1, 2, 9, 10, 99, 100, 255, 256, 1024, 4096];

    for len in lengths {
        let data = vec![b'x'; len];

        let mut encoded = format!("{len}:").into_bytes();
        encoded.extend_from_slice(&data);

        assert_bytes(&encoded, &data);
    }
}
