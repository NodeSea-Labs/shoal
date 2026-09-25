use crate::{DecodeError, Decoder, Value};

fn new_bytes(v: &[u8]) -> Value {
    Value::Bytes(bytes::Bytes::copy_from_slice(v))
}

fn assert_list(input: &[u8], expected: Vec<Value>) {
    let decoded = Decoder::new(input).decode();
    assert_eq!(decoded, Ok(Value::List(expected)), "input: {input:?}");
}

#[test]
fn list_empty() {
    assert_list(b"le", vec![]);
}

#[test]
fn list_decodes_adjacent_values() {
    assert_list(b"li1ei2ee", vec![Value::Integer(1), Value::Integer(2)]);
}

#[test]
fn list_decodes_mixed_values() {
    assert_list(
        b"li1e1:a0:e",
        vec![Value::Integer(1), new_bytes(b"a"), new_bytes(b"")],
    );
}

#[test]
fn list_preserves_bencode_markers_in_byte_string_payload() {
    assert_list(b"l3:eiee", vec![new_bytes(b"eie")]);
}

#[test]
fn list_decodes_nested_lists() {
    assert_list(
        b"lli1eeli-2eee",
        vec![
            Value::List(vec![Value::Integer(1)]),
            Value::List(vec![Value::Integer(-2)]),
        ],
    );
}

#[test]
fn list_rejects_every_incomplete_prefix() {
    let input = b"lli1ee1:ae";
    for end in 0..input.len() {
        assert!(
            Decoder::new(&input[..end]).decode().is_err(),
            "accepted truncated input: {:?}",
            &input[..end]
        );
    }

    assert_list(
        input,
        vec![Value::List(vec![Value::Integer(1)]), new_bytes(b"a")],
    );
}

#[test]
fn list_rejects_missing_terminator() {
    assert_eq!(
        Decoder::new(b"li1e").decode(),
        Err(DecodeError::UnexpectedEndOfInput {
            expected: 1,
            actual: 0,
        })
    );
}

#[test]
fn list_rejects_unsupported_child_type() {
    assert_eq!(
        Decoder::new(b"lx").decode(),
        Err(DecodeError::UnsupportedType { marker: b'x' })
    );
}

#[test]
fn list_rejects_trailing_top_level_data() {
    assert_eq!(
        Decoder::new(b"li1ee0:").decode(),
        Err(DecodeError::TrailingData { remaining: 2 })
    );
}

#[test]
fn list_rejects_excessive_recursive_nesting() {
    const MAX_DEPTH: usize = 100;

    let mut input = vec![b'l'; MAX_DEPTH + 1];
    input.extend_from_slice(b"i0e");
    input.extend(std::iter::repeat_n(b'e', MAX_DEPTH + 1));

    assert_eq!(
        Decoder::new(&input).decode(),
        Err(DecodeError::NestingTooDeep { limit: MAX_DEPTH })
    );
}

#[test]
fn list_accepts_nesting_at_the_limit() {
    const MAX_DEPTH: usize = 100;

    let mut input = vec![b'l'; MAX_DEPTH];
    input.extend_from_slice(b"i0e");
    input.extend(std::iter::repeat_n(b'e', MAX_DEPTH));

    let decoded = Decoder::new(&input).decode().unwrap();
    let mut current = &decoded;
    for _ in 0..MAX_DEPTH {
        let Value::List(values) = current else {
            panic!("expected a list at each nesting level");
        };
        assert_eq!(values.len(), 1);
        current = &values[0];
    }
    assert_eq!(current, &Value::Integer(0));
}
