#[macro_use]
extern crate capnp;

fn try_go() -> ::capnp::Result<()> {
    let segment: &[::capnp::Word] = &[
        capnp_word!(0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00),
        capnp_word!(0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00),
    ];

    let segments = &[segment];
    let segment_array = capnp::message::SegmentArray::new(segments);
    let message = capnp::message::Reader::new(segment_array, Default::default());
    let root: capnp::any_pointer::Reader = try!(message.get_root());

    // At one point, this failed with:
    // error: pointer computed at offset 33554448, outside bounds of allocation Runtime(702) which has size 16
    let result = root.target_size();

    assert!(result.is_err()); // pointer out-of-bounds error

    Ok(())
}

pub fn main() {
    let _ = try_go();
}
