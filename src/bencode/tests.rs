mod bytes;
mod dictionary;
mod encoder;
mod integer;
mod list;

#[test]
fn public_decode_helpers_use_expected_options() {
    use crate::{
        DecodeOptions, DecodeWarning, DictionaryKeyPolicy, Value, decode, decode_with_options,
        decode_with_warnings,
    };

    assert_eq!(decode(b"i42e"), Ok(Value::Integer(42)));

    let options = DecodeOptions::builder()
        .dictionary_key_policy(DictionaryKeyPolicy::PreserveInput)
        .build();
    assert_eq!(
        options.dictionary_key_policy(),
        DictionaryKeyPolicy::PreserveInput
    );
    assert!(options.max_nesting_depth() > 0);
    assert!(options.max_tokens() > 0);
    assert_eq!(
        decode_with_options(b"d1:bi1e1:ai2ee", options),
        Ok(Value::Dictionary(vec![
            (::bytes::Bytes::from_static(b"b"), Value::Integer(1)),
            (::bytes::Bytes::from_static(b"a"), Value::Integer(2)),
        ]))
    );

    let output = decode_with_warnings(b"d1:bi1e1:ai2ee", options).unwrap();
    assert!(output.has_warnings());
    assert_eq!(output.warnings().len(), 1);
    assert_eq!(
        output.value(),
        &Value::Dictionary(vec![
            (::bytes::Bytes::from_static(b"b"), Value::Integer(1)),
            (::bytes::Bytes::from_static(b"a"), Value::Integer(2)),
        ])
    );
    assert_eq!(
        output.warnings(),
        vec![DecodeWarning::UnsortedDictionaryKey {
            key: ::bytes::Bytes::from_static(b"a"),
            previous_key: ::bytes::Bytes::from_static(b"b"),
        }]
    );
    let (value, warnings) = output.into_parts();
    assert!(matches!(value, Value::Dictionary(_)));
    assert_eq!(warnings.len(), 1);
}
