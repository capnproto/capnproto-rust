#[test]
pub fn total_size_out_of_bounds() {
    let segment: &[capnp::Word] = &[
        capnp::word(0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00),
        capnp::word(0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00),
    ];

    let segments = &[capnp::Word::words_to_bytes(segment)];
    let segment_array = capnp::message::SegmentArray::new(segments);
    let message = capnp::message::Reader::new(segment_array, Default::default());
    let root: capnp::any_pointer::Reader = message.get_root().unwrap();

    // At one point, this failed in miri with:
    // error: pointer computed at offset 33554448, outside bounds of allocation Runtime(702) which has size 16
    let result = root.target_size();

    assert!(result.is_err()); // pointer out-of-bounds error
}
