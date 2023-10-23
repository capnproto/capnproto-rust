#![cfg(feature = "alloc")]

use capnp::message;

#[test]
pub fn single_segment_allocator() {
    let mut buffer = capnp::Word::allocate_zeroed_vec(200);
    let allocator =
        message::SingleSegmentAllocator::new(capnp::Word::words_to_bytes_mut(&mut buffer[..]));
    let mut msg = message::Builder::new(allocator);
    msg.set_root("hello world!").unwrap();

    let s: capnp::text::Reader = msg.get_root_as_reader().unwrap();
    assert_eq!("hello world!", s);
}
