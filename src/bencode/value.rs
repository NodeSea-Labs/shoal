/// A decoded Bencode value supported by this crate.
///
/// Byte strings borrow their payload directly from the input buffer.
pub enum Value<'a> {
    /// A signed integer encoded as a Bencode integer.
    Integer(i64),
    /// An arbitrary byte sequence encoded as a Bencode byte string.
    ByteString(&'a [u8]),
}
