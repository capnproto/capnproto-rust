#![deny(warnings)]

use std::io;

use capnp::{self, serialize_packed};
use offst_capnp_conv::{capnp_conv, CapnpConvError, ReadCapnp, WriteCapnp};

#[allow(unused)]
mod test_capnp {
    include!(concat!(env!("OUT_DIR"), "/capnp/test_capnp.rs"));
}

#[allow(unused)]
#[capnp_conv(test_capnp::test_struct_inner)]
#[derive(Debug, Clone, PartialEq)]
struct TestStructInner {
    inner_u8: u8,
}

#[allow(unused)]
#[capnp_conv(test_capnp::test_struct)]
#[derive(Debug, Clone, PartialEq)]
struct TestStruct {
    my_bool: bool,
    my_int8: i8,
    my_int16: i16,
    my_int32: i32,
    my_int64: i64,
    my_uint8: u8,
    my_uint16: u16,
    my_uint32: u32,
    my_uint64: u64,
    // my_float32: f32,
    // my_float64: f64,
    my_text: String,
    my_data: Vec<u8>,
    struct_inner: TestStructInner,
    my_list: Vec<TestStructInner>,
}

#[test]
fn capnp_serialize_basic_struct() {
    // Serialize:
    let test_struct = TestStruct {
        my_bool: true,
        my_int8: -1i8,
        my_int16: 1i16,
        my_int32: -1i32,
        my_int64: 1i64,
        my_uint8: 1u8,
        my_uint16: 2u16,
        my_uint32: 3u32,
        my_uint64: 4u64,
        // my_float32: -0.5f32,
        // my_float64: 0.5f64,
        my_text: "my_text".to_owned(),
        my_data: vec![1, 2, 3, 4, 5u8],
        struct_inner: TestStructInner { inner_u8: 1u8 },
        my_list: vec![
            TestStructInner { inner_u8: 2u8 },
            TestStructInner { inner_u8: 3u8 },
            TestStructInner { inner_u8: 4u8 },
        ],
    };

    let mut builder = capnp::message::Builder::new_default();
    let mut test_struct_builder = builder.init_root::<test_capnp::test_struct::Builder>();

    test_struct.write_capnp(&mut test_struct_builder);

    let mut data = Vec::new();
    serialize_packed::write_message(&mut data, &builder).unwrap();

    // Deserialize:
    let mut cursor = io::Cursor::new(&data);
    let reader =
        serialize_packed::read_message(&mut cursor, capnp::message::ReaderOptions::new()).unwrap();
    let test_struct_reader = reader
        .get_root::<test_capnp::test_struct::Reader>()
        .unwrap();

    let test_struct2 = TestStruct::read_capnp(&test_struct_reader).unwrap();

    assert_eq!(test_struct, test_struct2);
}
