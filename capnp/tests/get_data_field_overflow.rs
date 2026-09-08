#![cfg(feature = "alloc")]

use capnp::message::{Builder, HeapAllocator};
use capnp::schema_capnp;
use capnp::traits::IntoInternalStructReader;

// https://github.com/capnproto/capnproto-rust/issues/682

#[test]
fn get_data_field_overflow() {
    let mut msg = Builder::new(HeapAllocator::new());
    msg.init_root::<schema_capnp::node::Builder>();

    let reader = msg.into_reader();
    let sr = reader
        .get_root::<schema_capnp::node::Reader>()
        .unwrap()
        .into_internal_struct_reader();

    // (usize::MAX + 1) wraps to 0 in release, bypassing bounds check → OOB read
    let val: u64 = sr
        .get_pointer_field(0)
        .get_struct(None)
        .unwrap()
        .get_data_field(usize::MAX);
    std::hint::black_box(val);
}
