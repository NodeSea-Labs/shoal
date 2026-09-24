use crate::{Decoder, Value};

fn assert_integer(input: &[u8], expected: i64) {
    match Decoder::new(input).decode() {
        Ok(Value::Integer(actual)) => {
            assert_eq!(actual, expected, "incorrect integer for input: {input:?}");
        }
        _ => panic!("expected a valid integer for input: {input:?}"),
    }
}

fn assert_invalid(input: &[u8]) {
    assert!(
        Decoder::new(input).decode().is_err(),
        "expected an error for invalid input: {input:?}"
    );
}

#[test]
fn integer_valid_values() {
    let cases: &[(&[u8], i64)] = &[
        (b"i0e", 0),
        (b"i1e", 1),
        (b"i9e", 9),
        (b"i10e", 10),
        (b"i42e", 42),
        (b"i123456789e", 123456789),
        (b"i-1e", -1),
        (b"i-9e", -9),
        (b"i-10e", -10),
        (b"i-42e", -42),
        (b"i-123456789e", -123456789),
    ];

    for &(input, expected) in cases {
        assert_integer(input, expected);
    }
}

#[test]
fn integer_valid_boundaries() {
    let cases: &[(&[u8], i64)] = &[
        (b"i2147483647e", i32::MAX as i64),
        (b"i2147483648e", 2147483648),
        (b"i-2147483648e", i32::MIN as i64),
        (b"i-2147483649e", -2147483649),
        (b"i4294967295e", u32::MAX as i64),
        (b"i4294967296e", 4294967296),
        (b"i9223372036854775807e", i64::MAX),
        (b"i-9223372036854775808e", i64::MIN),
    ];

    for &(input, expected) in cases {
        assert_integer(input, expected);
    }
}

#[test]
fn integer_overflow() {
    let cases: &[&[u8]] = &[
        // i64::MAX + 1
        b"i9223372036854775808e",
        // i64::MIN - 1
        b"i-9223372036854775809e",
        // u64::MAX
        b"i18446744073709551615e",
        // u64::MAX + 1
        b"i18446744073709551616e",
        // Larger positive and negative integers
        b"i999999999999999999999999999999e",
        b"i-999999999999999999999999999999e",
    ];

    for &input in cases {
        assert_invalid(input);
    }
}

#[test]
fn integer_leading_zeros() {
    let cases: &[&[u8]] = &[
        b"i00e",
        b"i01e",
        b"i001e",
        b"i000123e",
        b"i0123456789e",
        b"i-00e",
        b"i-01e",
        b"i-001e",
        b"i-000123e",
    ];

    for &input in cases {
        assert_invalid(input);
    }
}

#[test]
fn integer_negative_zero() {
    assert_invalid(b"i-0e");
}

#[test]
fn integer_invalid_sign() {
    let cases: &[&[u8]] = &[
        b"i+0e", b"i+1e", b"i+123e", b"i-e", b"i+e", b"i--1e", b"i++1e", b"i+-1e", b"i-+1e",
        b"i1-2e", b"i1+2e",
    ];

    for &input in cases {
        assert_invalid(input);
    }
}

#[test]
fn integer_missing_components() {
    let cases: &[&[u8]] = &[
        b"", b"i", b"e", b"ie", b"i-", b"i123", b"i-123", b"123e", b"-123e", b"123", b"i123E",
    ];

    for &input in cases {
        assert_invalid(input);
    }
}

#[test]
fn integer_invalid_characters() {
    let cases: &[&[u8]] = &[
        // Letters
        b"ia e",
        b"i12ae",
        b"i1Ee",
        // Decimal point
        b"i1.0e",
        b"i-1.5e",
        // Whitespace
        b"i 1e",
        b"i1 e",
        b"i1 2e",
        b"i\t1e",
        b"i1\ne",
        b"i1\re",
        // NUL and control bytes
        b"i\0e",
        b"i1\0e",
        b"i\x01e",
        // Non-ASCII bytes
        b"i\x80e",
        b"i\xffe",
        b"i\xc2\xb9e",
    ];

    for &input in cases {
        assert_invalid(input);
    }
}

#[test]
fn integer_extra_bytes() {
    let cases: &[&[u8]] = &[
        b"i1ee",
        b"i1eee",
        b"i1e0",
        b"i1e ",
        b"i1e\0",
        b"\0i1e",
        b"xi1e",
        b"i1ei2e",
        b"i1ehello",
    ];

    for &input in cases {
        assert_invalid(input);
    }
}

#[test]
fn integer_reports_trailing_bytes_after_terminator() {
    assert_eq!(
        Decoder::new(b"i1ee").decode(),
        Err(crate::DecodeError::TrailingData { remaining: 1 })
    );
}

#[test]
fn integer_truncated_input() {
    let valid = b"i-9223372036854775808e";

    // Every proper prefix is incomplete.
    for end in 0..valid.len() {
        assert_invalid(&valid[..end]);
    }

    // The complete input must remain valid.
    assert_integer(valid, i64::MIN);
}

#[test]
fn integer_all_single_byte_payloads() {
    for byte in u8::MIN..=u8::MAX {
        let input = [b'i', byte, b'e'];

        if byte.is_ascii_digit() {
            assert_integer(&input, (byte - b'0') as i64);
        } else {
            assert_invalid(&input);
        }
    }
}

#[test]
fn integer_very_long_number() {
    // A long numeric payload must not overflow or panic.
    let input = format!("i{}e", "9".repeat(4096));

    assert_invalid(input.as_bytes());
}

#[test]
fn integer_roundtrip_from_i64() {
    let cases = [
        i64::MIN,
        i64::MIN + 1,
        i32::MIN as i64,
        -1000,
        -1,
        0,
        1,
        1000,
        i32::MAX as i64,
        i64::MAX - 1,
        i64::MAX,
    ];

    for value in cases {
        let encoded = format!("i{value}e");

        assert_integer(encoded.as_bytes(), value);
    }
}
