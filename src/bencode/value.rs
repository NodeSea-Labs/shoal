use bytes::Bytes;

/// A decoded Bencode value supported by this crate.
///
/// Byte strings own their payload in reference-counted [`Bytes`] storage, so a
/// decoded value does not borrow the input buffer.
#[derive(Debug, PartialEq, Eq)]
pub enum Value {
    /// A signed integer encoded as a Bencode integer.
    Integer(i64),
    /// An arbitrary byte sequence encoded as a Bencode byte string.
    Bytes(Bytes),
    /// A list of Bencode values.
    List(Vec<Value>),
}
