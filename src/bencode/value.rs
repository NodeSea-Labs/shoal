/// A decoded Bencode value supported by this crate.
///
/// Byte strings borrow their payload directly from the input buffer.
#[derive(Debug, PartialEq, Eq)]
pub enum Value<'a> {
    /// A signed integer encoded as a Bencode integer.
    Integer(i64),
    /// An arbitrary byte sequence encoded as a Bencode byte string.
    ByteString(&'a [u8]),
    /// A list of Bencode values.
    List(Vec<Value<'a>>),
}
