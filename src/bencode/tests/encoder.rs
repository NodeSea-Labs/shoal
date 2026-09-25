use crate::{EncodeError, EncodeOptions, Encoder, Value, decode, encode, encode_with_options};
use bytes::Bytes;

#[test]
fn encoder_formats_integers_and_preserves_integer_boundaries() {
    let cases: &[(i64, &[u8])] = &[
        (i64::MIN, b"i-9223372036854775808e"),
        (-1, b"i-1e"),
        (0, b"i0e"),
        (1, b"i1e"),
        (i64::MAX, b"i9223372036854775807e"),
    ];

    for &(integer, expected) in cases {
        assert_eq!(encode(&Value::Integer(integer)).unwrap(), expected);
    }
}

#[test]
fn encoder_writes_raw_byte_strings() {
    let value = Value::Bytes(Bytes::from_static(b"\0:e\xff"));

    assert_eq!(encode(&value).unwrap(), b"4:\0:e\xff");
}

#[test]
fn encoder_sorts_dictionary_keys_by_raw_bytes() {
    let value = Value::Dictionary(vec![
        (Bytes::from_static(b"z"), Value::Integer(1)),
        (Bytes::from_static(b"a\xff"), Value::Integer(2)),
        (Bytes::from_static(b"a\0"), Value::Integer(3)),
    ]);

    let encoded = encode(&value).unwrap();
    assert_eq!(encoded, b"d2:a\0i3e2:a\xffi2e1:zi1ee");
    assert_eq!(
        decode(&encoded),
        Ok(Value::Dictionary(vec![
            (Bytes::from_static(b"a\0"), Value::Integer(3)),
            (Bytes::from_static(b"a\xff"), Value::Integer(2)),
            (Bytes::from_static(b"z"), Value::Integer(1)),
        ]))
    );
}

#[test]
fn encoder_serializes_nested_values() {
    let value = Value::List(vec![
        Value::Integer(-3),
        Value::Bytes(Bytes::from_static(b"spam")),
        Value::Dictionary(vec![(Bytes::from_static(b"key"), Value::List(vec![]))]),
    ]);

    assert_eq!(encode(&value).unwrap(), b"li-3e4:spamd3:keyleee");
    assert_eq!(decode(&encode(&value).unwrap()), Ok(value));
}

#[test]
fn encoder_rejects_duplicate_dictionary_keys() {
    let value = Value::Dictionary(vec![
        (Bytes::from_static(b"key"), Value::Integer(1)),
        (Bytes::from_static(b"key"), Value::Integer(2)),
    ]);

    assert_eq!(
        encode(&value),
        Err(EncodeError::DuplicateDictionaryKey {
            key: Bytes::from_static(b"key"),
        })
    );
}

#[test]
fn encoder_rejects_values_beyond_supported_nesting_depth() {
    const MAX_DEPTH: usize = 100;

    let mut value = Value::Integer(0);
    for _ in 0..=MAX_DEPTH {
        value = Value::List(vec![value]);
    }

    assert_eq!(
        encode(&value),
        Err(EncodeError::NestingTooDeep { limit: MAX_DEPTH })
    );
}

#[test]
fn encoder_uses_caller_configured_nesting_depth() {
    let value = Value::List(vec![Value::List(vec![Value::Integer(1)])]);
    let options = EncodeOptions {
        max_nesting_depth: 1,
    };

    assert_eq!(
        Encoder::with_options(options).encode(&value),
        Err(EncodeError::NestingTooDeep { limit: 1 })
    );
    assert_eq!(
        encode_with_options(
            &value,
            EncodeOptions {
                max_nesting_depth: 2,
            }
        ),
        Ok(b"lli1eee".to_vec())
    );
}
