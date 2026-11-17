use capnp::{Word, message, serialize};

#[test]
pub fn serialize_read_message_no_alloc() {
    let mut buffer = [capnp::word(0, 0, 0, 0, 0, 0, 0, 0); 200];
    {
        let allocator =
            message::SingleSegmentAllocator::new(capnp::Word::words_to_bytes_mut(&mut buffer[..]));
        let mut msg = message::Builder::new(allocator);
        msg.set_root("hello world!").unwrap();

        let mut out_buffer = [capnp::word(0, 0, 0, 0, 0, 0, 0, 0); 256];

        serialize::write_message(Word::words_to_bytes_mut(&mut out_buffer), &msg).unwrap();

        let mut read_buffer = [capnp::word(0, 0, 0, 0, 0, 0, 0, 0); 256];

        let reader = serialize::read_message_no_alloc(
            &mut Word::words_to_bytes(&out_buffer),
            Word::words_to_bytes_mut(&mut read_buffer),
            message::ReaderOptions::new(),
        )
        .unwrap();

        let s: capnp::text::Reader = reader.get_root().unwrap();
        assert_eq!("hello world!", s);
    }
}

#[repr(C, align(8))]
struct BufferWrapper<const N: usize> {
    bytes: [u8; N],
}

impl<const N: usize> AsRef<[u8]> for BufferWrapper<N> {
    fn as_ref(&self) -> &[u8] {
        &self.bytes[..]
    }
}

#[test]
pub fn no_alloc_buffer_segments_from_buffer() {
    let buffer = BufferWrapper {
        bytes: [
            0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x42, 0x00,
            0x00, 0x00, 97, 98, 99, 100, 101, 102, 103, 0, // "abcdefg" with null terminator
        ],
    };
    let segs = serialize::NoAllocBufferSegments::from_buffer(buffer, Default::default()).unwrap();
    let message = message::Reader::new(segs, Default::default());
    let t = message.get_root::<capnp::text::Reader>().unwrap();
    assert_eq!(t, "abcdefg");
}
