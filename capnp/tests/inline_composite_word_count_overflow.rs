/// Tests that the bounds check on an InlineComposite list cannot be bypassed
/// on 32-bit targets by overflowing the list's size in bytes.
///
/// The list claims a word count of 2^29 - 1, so the bounds check covers 2^29
/// words (including the tag), which is 2^32 bytes. On a 32-bit target that
/// overflows `usize`. Previously, in release builds, it wrapped to 0 and the
/// bounds check passed, allowing out-of-bounds reads of the list elements.
///
///     cargo +nightly miri test --release --target i686-unknown-linux-gnu \
///         --test inline_composite_word_count_overflow
use capnp::primitive_list;

#[test]
pub fn inline_composite_word_count_overflow() {
    let segment: &[capnp::Word] = &[
        // Root: List pointer, offset 0, element size InlineComposite (7),
        // word count 2^29 - 1.
        //   low u32   = (0 << 2) | WirePointerKind::List(1) = 0x00000001
        //   upper u32 = ((2^29 - 1) << 3) | 7 = 0xffffffff
        capnp::word(0x01, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff),
        // Tag: struct with 2 elements, each with 1 data word and 0 pointers.
        //   low u32   = (2 << 2) | WirePointerKind::Struct(0) = 0x00000008
        //   upper u32 = data words 1 | (pointers 0 << 16) = 0x00000001
        capnp::word(0x08, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00),
    ];

    let segments = &[capnp::Word::words_to_bytes(segment)];
    let segment_array = capnp::message::SegmentArray::new(segments);
    let mut options = capnp::message::ReaderOptions::new();
    // With the default traversal limit, the read limiter rejects the list.
    options.traversal_limit_in_words(None);
    let message = capnp::message::Reader::new(segment_array, options);
    let root: capnp::any_pointer::Reader = message.get_root().unwrap();

    let result = root.get_as::<primitive_list::Reader<u64>>();
    assert!(result.is_err());
}
