use crate::{DecodeError, Decoder, Value};
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
        Err(DecodeError::InvalidByteStringLength)
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
