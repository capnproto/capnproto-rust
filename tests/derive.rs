#![deny(warnings)]

use std::convert::TryFrom;

use offst_capnp_conv::{
    capnp_conv, CapnpConvError, FromCapnpBytes, ReadCapnp, ToCapnpBytes, WriteCapnp,
};

#[allow(unused)]
mod test_capnp {
    include!(concat!(env!("OUT_DIR"), "/capnp/test_capnp.rs"));
}

#[capnp_conv(test_capnp::test_struct_inner)]
#[derive(Debug, Clone, PartialEq)]
struct TestStructInner {
    inner_u8: u8,
}

#[capnp_conv(test_capnp::test_struct::inline_union)]
#[derive(Debug, Clone, PartialEq)]
enum InlineUnion {
    FirstVariant(u64),
    SecondVariant(TestStructInner),
    ThirdVariant,
}

#[capnp_conv(test_capnp::test_union)]
#[derive(Debug, Clone, PartialEq)]
enum TestUnion {
    VariantOne(u64),
    VariantTwo(TestStructInner),
    VariantThree,
}

#[capnp_conv(test_capnp::list_union)]
#[derive(Debug, Clone, PartialEq)]
enum ListUnion {
    Empty,
    WithList(Vec<TestStructInner>),
    TestUnion(TestUnion),
}

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
    my_text: String,
    my_data: Vec<u8>,
    struct_inner: TestStructInner,
    my_primitive_list: Vec<u16>,
    my_list: Vec<TestStructInner>,
    inline_union: InlineUnion,
    external_union: TestUnion,
    list_union: ListUnion,
}

#[test]
fn capnp_serialize_basic_struct() {
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
        my_text: "my_text".to_owned(),
        my_data: vec![1, 2, 3, 4, 5u8],
        struct_inner: TestStructInner { inner_u8: 1u8 },
        my_primitive_list: vec![10, 11, 12, 13, 14u16],
        my_list: vec![
            TestStructInner { inner_u8: 2u8 },
            TestStructInner { inner_u8: 3u8 },
            TestStructInner { inner_u8: 4u8 },
        ],
        inline_union: InlineUnion::SecondVariant(TestStructInner { inner_u8: 5u8 }),
        external_union: TestUnion::VariantOne(6u64),
        list_union: ListUnion::WithList(vec![
            TestStructInner { inner_u8: 10u8 },
            TestStructInner { inner_u8: 11u8 },
        ]),
    };

    let data = test_struct.to_capnp_bytes().unwrap();
    let test_struct2 = TestStruct::from_capnp_bytes(&data).unwrap();

    assert_eq!(test_struct, test_struct2);
}

#[capnp_conv(test_capnp::float_struct)]
#[derive(Debug, Clone)]
struct FloatStruct {
    my_float32: f32,
    my_float64: f64,
}

/// We test floats separately, because in Rust floats to not implement PartialEq
#[test]
fn capnp_serialize_floats() {
    let float_struct = FloatStruct {
        my_float32: -0.5f32,
        my_float64: 0.5f64,
    };

    let data = float_struct.to_capnp_bytes().unwrap();
    let float_struct2 = FloatStruct::from_capnp_bytes(&data).unwrap();

    // Sloppily check that the floats are close enough (We can't compare them directly, as they
    // don't implement PartialEq)
    assert_eq!(
        (float_struct.my_float32 * 10000.0).trunc(),
        (float_struct2.my_float32 * 10000.0).trunc()
    );

    assert_eq!(
        (float_struct.my_float64 * 10000.0).trunc(),
        (float_struct2.my_float64 * 10000.0).trunc()
    );
}
