/// Tests that a pointer with a huge offset is rejected without panicking on
/// 32-bit targets.
///
/// The largest positive pointer offset is 2^29 words, which is 2^32 bytes.
/// Previously, on 32-bit targets, converting the resulting index to `usize`
/// panicked.
///
///     cargo +nightly miri test --target i686-unknown-linux-gnu --test check_offset_overflow
use capnp::any_pointer;

#[test]
pub fn check_offset_overflow() {
    // Root: struct pointer with offset field 2^29 - 1 and 1 data word.
    //   low u32   = ((2^29 - 1) << 2) | WirePointerKind::Struct(0) = 0x7ffffffc
    //   upper u32 = data words 1 | (pointers 0 << 16) = 0x00000001
    let segment: &[capnp::Word] = &[capnp::word(0xfc, 0xff, 0xff, 0x7f, 0x01, 0x00, 0x00, 0x00)];

    let segments = &[capnp::Word::words_to_bytes(segment)];
    let segment_array = capnp::message::SegmentArray::new(segments);
    let message = capnp::message::Reader::new(segment_array, Default::default());
    let root: any_pointer::Reader = message.get_root().unwrap();
    assert!(root.target_size().is_err());
}
