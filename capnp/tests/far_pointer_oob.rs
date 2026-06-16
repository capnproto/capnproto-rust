/// At one point, this test triggered out-of-bounds pointer arithmetic
/// (undefined behavior observable in Miri) in `wire_helpers::follow_fars`.
#[test]
pub fn far_pointer_out_of_bounds_position() {
    // word 0: a single far pointer.
    //   offset_and_kind (low u32, LE) = (pos << 3) | (double_far << 2) | Far(2)
    //   with pos = 0x1FFFFFFF (the maximum 29-bit far position), double_far = 0:
    //     (0x1FFFFFFF << 3) | 2 = 0xFFFFFFFA
    //   upper u32 = far_segment_id = 0
    // word 1: padding so the segment is non-empty.
    let segment: &[capnp::Word] = &[
        capnp::word(0xFA, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00),
        capnp::word(0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00),
    ];

    let segments = &[capnp::Word::words_to_bytes(segment)];
    let segment_array = capnp::message::SegmentArray::new(segments);
    let message = capnp::message::Reader::new(segment_array, Default::default());
    let root: capnp::any_pointer::Reader = message.get_root().unwrap();

    // Triggers follow_fars(), which computes seg_start.add(0x1FFFFFFF * 8),
    // an out-of-bounds pointer relative to the 2-word segment allocation.
    let result = root.target_size();

    // Functionally the library rejects it (the bounds check fails), but the
    // out-of-bounds pointer computation has already occurred.
    assert!(result.is_err());
}
