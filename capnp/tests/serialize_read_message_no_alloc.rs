use capnp::{message, serialize, Word};

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
