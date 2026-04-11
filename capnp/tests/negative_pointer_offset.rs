/// Regression test for a bug where a struct pointer with a large negative
/// signed offset would cause `ReaderArenaImpl::check_offset` to panic with
/// a `TryFromIntError` (from `usize::try_from` on a negative `i64`) instead
/// of returning `MessageContainsOutOfBoundsPointer`.
///
/// Originally discovered by `cargo fuzz run test_all_types`.
#[test]
pub fn negative_root_pointer_offset() {
    // Root struct pointer whose signed 30-bit offset field decodes to a
    // large negative value, pointing well before the start of the segment.
    let segment: &[capnp::Word] = &[
        capnp::word(0x00, 0x00, 0x6d, 0x97, 0x01, 0x00, 0x00, 0x00),
        capnp::word(0x00, 0x00, 0x6d, 0x6d, 0x6d, 0x6d, 0xff, 0x00),
    ];

    let segments = &[capnp::Word::words_to_bytes(segment)];
    let segment_array = capnp::message::SegmentArray::new(segments);
    let message = capnp::message::Reader::new(segment_array, Default::default());
    let root: capnp::any_pointer::Reader = message.get_root().unwrap();

    // Before the fix, this panicked in `check_offset` instead of returning
    // an out-of-bounds error.
    let result = root.target_size();

    assert!(result.is_err());
}
