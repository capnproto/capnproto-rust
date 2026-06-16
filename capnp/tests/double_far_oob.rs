/// Out-of-bounds pointer arithmetic reachable via a double-far pointer.
///
/// `follow_fars` returns the *double-far* target pointer WITHOUT bounds-checking
/// it (only the 2-word landing pad is bounds-checked). `read_list_pointer`'s
/// InlineComposite branch then does
///
///     ptr = ptr.add(BYTES_PER_WORD);
///
/// on that pointer *before* its own bounds_check. If the double-far target
/// position is out of bounds, this `.add()` is undefined behavior (the result
/// escapes the allocated object).
///
/// Run under Miri to observe:
///     cargo +nightly miri test --test double_far_oob
use capnp::primitive_list;

#[test]
pub fn double_far_inline_composite_oob() {
    // Single segment (far_segment_id = 0 throughout).
    //
    // word 0 (root): double-far pointer -> landing pad at word index 1.
    //   offset_and_kind = (pos << 3) | (double_far << 2) | Far(2)
    //   pos = 1 (landing pad word index), double_far = 1:
    //     (1 << 3) | (1 << 2) | 2 = 0x0E
    //   upper u32 = far_segment_id = 0
    //
    // word 1 (landing pad, far pointer #2): single far with an OOB position.
    //   pos = 0x1FFFFFFF (max 29-bit), double_far = 0:
    //     (0x1FFFFFFF << 3) | 2 = 0xFFFFFFFA
    //   upper u32 = double_far_segment_id = 0
    //
    // word 2 (landing pad tag): a List pointer, element size InlineComposite.
    //   low u32  = WirePointerKind::List (1)
    //   upper u32 = (word_count << 3) | InlineComposite(7) = (1 << 3) | 7 = 0x0F
    let segment: &[capnp::Word] = &[
        capnp::word(0x0E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00),
        capnp::word(0xFA, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00),
        capnp::word(0x01, 0x00, 0x00, 0x00, 0x0F, 0x00, 0x00, 0x00),
    ];

    let segments = &[capnp::Word::words_to_bytes(segment)];
    let segment_array = capnp::message::SegmentArray::new(segments);
    let message = capnp::message::Reader::new(segment_array, Default::default());
    let root: capnp::any_pointer::Reader = message.get_root().unwrap();

    // Reading the root as a list reaches read_list_pointer's InlineComposite
    // branch, which performs `ptr.add(BYTES_PER_WORD)` on the (out-of-bounds)
    // double-far target pointer before bounds-checking it.
    let result = root.get_as::<primitive_list::Reader<u8>>();
    assert!(result.is_err());
}
