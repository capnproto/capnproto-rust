/// Tests whether read_list_pointer's `ptr = ptr.add(BYTES_PER_WORD)` (the tag
/// word skip, layout.rs:2529) can be UB for an ordinary NON-far list pointer
/// whose target lands one-past-the-end of the segment.
///
/// `check_offset` permits a target at exactly `this_size` (one-past-the-end is a
/// valid pointer). read_list_pointer's InlineComposite branch then advances by a
/// word *before* bounds-checking, producing a pointer two words past the end.
///
///     cargo +nightly miri test --test inline_composite_tag_oob
use capnp::primitive_list;

#[test]
pub fn inline_composite_one_past_end() {
    // Single 1-word segment containing only the root pointer.
    //
    // Root: List pointer, element size InlineComposite, offset field = 0.
    //   low u32  = (offset << 2) | WirePointerKind::List(1) = 1
    //   upper u32 = (word_count << 3) | InlineComposite(7), word_count = 0 -> 7
    //
    // offset_in_words() = 1 + (0 >> 2) = 1, so the target is word index 1,
    // i.e. exactly one-past-the-end of this 1-word segment.
    let segment: &[capnp::Word] =
        &[capnp::word(0x01, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00)];

    let segments = &[capnp::Word::words_to_bytes(segment)];
    let segment_array = capnp::message::SegmentArray::new(segments);
    let message = capnp::message::Reader::new(segment_array, Default::default());
    let root: capnp::any_pointer::Reader = message.get_root().unwrap();

    let result = root.get_as::<primitive_list::Reader<u8>>();
    assert!(result.is_err());
}
