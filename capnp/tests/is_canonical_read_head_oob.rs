/// Tests that `is_canonical()` does not advance its read head past the end of
/// the segment when it reaches an InlineComposite list.
///
/// The root struct's only pointer field is an InlineComposite list whose tag
/// is the root pointer word. When the canonical walk reaches that list, its
/// read head is already one past the end of the segment. Previously it then
/// advanced the read head by another word (to skip the tag) before checking
/// that the list was where it expected, which is undefined behavior.
///
///     cargo +nightly miri test --test is_canonical_read_head_oob
#[test]
pub fn is_canonical_read_head_oob() {
    let segment: &[capnp::Word] = &[
        // Root: struct pointer, offset 0, 0 data words, 1 pointer.
        capnp::word(0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00),
        // Pointer field: List pointer, offset -2 (so the tag is word 0),
        // element size InlineComposite (7), word count 1.
        //   low u32   = (-2 << 2) | WirePointerKind::List(1) = 0xfffffff9
        //   upper u32 = (1 << 3) | 7 = 0x0000000f
        capnp::word(0xf9, 0xff, 0xff, 0xff, 0x0f, 0x00, 0x00, 0x00),
    ];

    let segments = &[capnp::Word::words_to_bytes(segment)];
    let segment_array = capnp::message::SegmentArray::new(segments);
    let message = capnp::message::Reader::new(segment_array, Default::default());
    assert!(!message.is_canonical().unwrap());
}
