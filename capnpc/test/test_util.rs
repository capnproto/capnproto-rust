// Copyright (c) 2013-2014 Sandstorm Development Group, Inc. and contributors
// Licensed under the MIT License:
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use crate::test_capnp::{test_all_types, test_defaults, TestEnum};

pub fn init_test_message(mut builder: test_all_types::Builder<'_>) {
    builder.set_void_field(());
    builder.set_bool_field(true);
    builder.set_int8_field(-123);
    builder.set_int16_field(-12345);
    builder.set_int32_field(-12345678);
    builder.set_int64_field(-123456789012345);
    builder.set_u_int8_field(234);
    builder.set_u_int16_field(45678);
    builder.set_u_int32_field(3456789012);
    builder.set_u_int64_field(12345678901234567890);
    builder.set_float32_field(1234.5);
    builder.set_float64_field(-123e45);
    builder.set_text_field("foo");
    builder.set_data_field(b"bar");
    {
        let mut sub_builder = builder.reborrow().init_struct_field();
        sub_builder.set_void_field(());
        sub_builder.set_bool_field(true);
        sub_builder.set_int8_field(-12);
        sub_builder.set_int16_field(3456);
        sub_builder.set_int32_field(-78901234);
        sub_builder.set_int64_field(56789012345678);
        sub_builder.set_u_int8_field(90);
        sub_builder.set_u_int16_field(1234);
        sub_builder.set_u_int32_field(56789012);
        sub_builder.set_u_int64_field(345678901234567890);
        sub_builder.set_float32_field(-1.25e-10);
        sub_builder.set_float64_field(345f64);
        sub_builder.set_text_field("baz");
        sub_builder.set_data_field(b"qux");
        {
            let mut sub_sub_builder = sub_builder.reborrow().init_struct_field();
            sub_sub_builder.set_text_field("nested");
            sub_sub_builder
                .init_struct_field()
                .set_text_field("really nested");
        }
        sub_builder.set_enum_field(TestEnum::Baz);

        sub_builder.reborrow().init_void_list(3);
        sub_builder
            .set_bool_list(&[false, true, false, true, true])
            .unwrap();

        sub_builder.set_int8_list(&[12, -34, -0x80, 0x7f]).unwrap();
        sub_builder
            .set_int16_list(&[1234, -5678, -0x8000, 0x7fff])
            .unwrap();
        sub_builder
            .set_int32_list(&[12345678, -90123456, -0x80000000, 0x7fffffff])
            .unwrap();
        sub_builder
            .set_int64_list(&[
                123456789012345,
                -678901234567890,
                -0x8000000000000000,
                0x7fffffffffffffff,
            ])
            .unwrap();

        sub_builder.set_u_int8_list(&[12, 34, 0, 0xff]).unwrap();
        sub_builder
            .set_u_int16_list(&[1234, 5678, 0, 0xffff])
            .unwrap();

        sub_builder
            .set_u_int32_list(&[12345678, 90123456, 0, 0xffffffff])
            .unwrap();
        sub_builder
            .set_u_int64_list(&[123456789012345, 678901234567890, 0, 0xffffffffffffffff])
            .unwrap();

        sub_builder
            .set_float32_list(&[0f32, 1234567f32, 1e37, -1e37, 1e-37, -1e-37])
            .unwrap();

        sub_builder
            .set_float64_list(&[0f64, 123456789012345f64, 1e306, -1e306, 1e-306, -1e-306])
            .unwrap();

        // ...
        {
            let mut struct_list = sub_builder.reborrow().init_struct_list(3);
            struct_list
                .reborrow()
                .get(0)
                .set_text_field("x structlist 1");
            struct_list
                .reborrow()
                .get(1)
                .set_text_field("x structlist 2");
            struct_list
                .reborrow()
                .get(2)
                .set_text_field("x structlist 3");
        }

        sub_builder
            .set_enum_list(&[TestEnum::Qux, TestEnum::Bar, TestEnum::Grault])
            .unwrap();
    }
    builder.set_enum_field(TestEnum::Corge);

    builder.reborrow().init_void_list(6);

    builder.set_bool_list(&[true, false, false, true]).unwrap();

    // ...

    builder.set_text_list(&["plugh", "xyzzy", "thud"]).unwrap();

    {
        let mut data_list = builder.reborrow().init_data_list(3);
        data_list.set(0, b"oops");
        data_list.set(1, b"exhausted");
        data_list.set(2, b"rfc3092");
    }

    {
        let mut struct_list = builder.reborrow().init_struct_list(3);
        struct_list.reborrow().get(0).set_text_field("structlist 1");
        struct_list.reborrow().get(1).set_text_field("structlist 2");
        struct_list.reborrow().get(2).set_text_field("structlist 3");
    }

    // ...
}

pub trait CheckTestMessage {
    fn check_test_message(_: Self);
}

macro_rules!
check_test_message_impl(($mod:ident::$typ:ident) => (
    impl <'a> CheckTestMessage for $mod::$typ<'a> {
        fn check_test_message(mut reader : $mod::$typ<'a>) {
            #![allow(unused_mut)]
            reader.reborrow().get_void_field();
            assert_eq!(true, reader.reborrow().get_bool_field());
            assert_eq!(-123, reader.reborrow().get_int8_field());
            assert_eq!(-12345, reader.reborrow().get_int16_field());
            assert_eq!(-12345678, reader.reborrow().get_int32_field());
            assert_eq!(-123456789012345, reader.reborrow().get_int64_field());
            assert_eq!(234, reader.reborrow().get_u_int8_field());
            assert_eq!(45678, reader.reborrow().get_u_int16_field());
            assert_eq!(3456789012, reader.reborrow().get_u_int32_field());
            assert_eq!(12345678901234567890, reader.reborrow().get_u_int64_field());
            assert_eq!(1234.5, reader.reborrow().get_float32_field());
            assert_eq!(-123e45, reader.reborrow().get_float64_field());
            assert_eq!("foo", reader.reborrow().get_text_field().unwrap());
            assert_eq!(b"bar", &*reader.reborrow().get_data_field().unwrap());
            {
                let mut sub_reader = reader.reborrow().get_struct_field().unwrap();
                assert_eq!((), sub_reader.reborrow().get_void_field());
                assert_eq!(true, sub_reader.reborrow().get_bool_field());
                assert_eq!(-12, sub_reader.reborrow().get_int8_field());
                assert_eq!(3456, sub_reader.reborrow().get_int16_field());
                assert_eq!(-78901234, sub_reader.reborrow().get_int32_field());
                assert_eq!(56789012345678, sub_reader.reborrow().get_int64_field());
                assert_eq!(90, sub_reader.reborrow().get_u_int8_field());
                assert_eq!(1234, sub_reader.reborrow().get_u_int16_field());
                assert_eq!(56789012, sub_reader.reborrow().get_u_int32_field());
                assert_eq!(345678901234567890, sub_reader.reborrow().get_u_int64_field());
                assert_eq!(-1.25e-10, sub_reader.reborrow().get_float32_field());
                assert_eq!(345f64, sub_reader.reborrow().get_float64_field());
                assert_eq!("baz", sub_reader.reborrow().get_text_field().unwrap());
                assert_eq!(b"qux", &*sub_reader.reborrow().get_data_field().unwrap());
                {
                    let mut sub_sub_reader = sub_reader.reborrow().get_struct_field().unwrap();
                    assert_eq!("nested", sub_sub_reader.reborrow().get_text_field().unwrap());
                    assert_eq!("really nested",
                               sub_sub_reader.get_struct_field().unwrap()
                               .get_text_field().unwrap());
                }
                assert!(Ok(TestEnum::Baz) == sub_reader.reborrow().get_enum_field());
                assert_eq!(false, sub_reader.reborrow().has_interface_field());
                assert_eq!(3, sub_reader.reborrow().get_void_list().unwrap().len());

                {
                    let bool_list = sub_reader.reborrow().get_bool_list().unwrap();
                    assert_eq!(5, bool_list.len());
                    assert_eq!(false, bool_list.get(0));
                    assert_eq!(true, bool_list.get(1));
                    assert_eq!(false, bool_list.get(2));
                    assert_eq!(true, bool_list.get(3));
                    assert_eq!(true, bool_list.get(4));
                }

                {
                    let int8_list = sub_reader.reborrow().get_int8_list().unwrap();
                    assert_eq!(4, int8_list.len());
                    assert_eq!(12, int8_list.get(0));
                    assert_eq!(-34, int8_list.get(1));
                    assert_eq!(-0x80, int8_list.get(2));
                    assert_eq!(0x7f, int8_list.get(3));
                }

                {
                    let int16_list = sub_reader.reborrow().get_int16_list().unwrap();
                    assert_eq!(4, int16_list.len());
                    assert_eq!(1234, int16_list.get(0));
                    assert_eq!(-5678, int16_list.get(1));
                    assert_eq!(-0x8000, int16_list.get(2));
                    assert_eq!(0x7fff, int16_list.get(3));
                }

                {
                    let int32_list = sub_reader.reborrow().get_int32_list().unwrap();
                    assert_eq!(4, int32_list.len());
                    assert_eq!(12345678, int32_list.get(0));
                    assert_eq!(-90123456, int32_list.get(1));
                    assert_eq!(-0x80000000, int32_list.get(2));
                    assert_eq!(0x7fffffff, int32_list.get(3));
                }

                {
                    let int64_list = sub_reader.reborrow().get_int64_list().unwrap();
                    assert_eq!(4, int64_list.len());
                    assert_eq!(123456789012345, int64_list.get(0));
                    assert_eq!(-678901234567890, int64_list.get(1));
                    assert_eq!(-0x8000000000000000, int64_list.get(2));
                    assert_eq!(0x7fffffffffffffff, int64_list.get(3));
                }

                {
                    let uint8_list = sub_reader.reborrow().get_u_int8_list().unwrap();
                    assert_eq!(4, uint8_list.len());
                    assert_eq!(12, uint8_list.get(0));
                    assert_eq!(34, uint8_list.get(1));
                    assert_eq!(0, uint8_list.get(2));
                    assert_eq!(0xff, uint8_list.get(3));
                }

                {
                    let uint16_list = sub_reader.reborrow().get_u_int16_list().unwrap();
                    assert_eq!(4, uint16_list.len());
                    assert_eq!(1234, uint16_list.get(0));
                    assert_eq!(5678, uint16_list.get(1));
                    assert_eq!(0, uint16_list.get(2));
                    assert_eq!(0xffff, uint16_list.get(3));
                }

                {
                    let uint32_list = sub_reader.reborrow().get_u_int32_list().unwrap();
                    assert_eq!(4, uint32_list.len());
                    assert_eq!(12345678, uint32_list.get(0));
                    assert_eq!(90123456, uint32_list.get(1));
                    assert_eq!(0, uint32_list.get(2));
                    assert_eq!(0xffffffff, uint32_list.get(3));
                }

                {
                    let uint64_list = sub_reader.reborrow().get_u_int64_list().unwrap();
                    assert_eq!(4, uint64_list.len());
                    assert_eq!(123456789012345, uint64_list.get(0));
                    assert_eq!(678901234567890, uint64_list.get(1));
                    assert_eq!(0, uint64_list.get(2));
                    assert_eq!(0xffffffffffffffff, uint64_list.get(3));
                }

                {
                    let float32_list = sub_reader.reborrow().get_float32_list().unwrap();
                    assert_eq!(6, float32_list.len());
                    assert_eq!(0f32, float32_list.get(0));
                    assert_eq!(1234567f32, float32_list.get(1));
                    assert_eq!(1e37, float32_list.get(2));
                    assert_eq!(-1e37, float32_list.get(3));
                    assert_eq!(1e-37, float32_list.get(4));
                    assert_eq!(-1e-37, float32_list.get(5));
                }

                {
                    let float64_list = sub_reader.reborrow().get_float64_list().unwrap();
                    assert_eq!(6, float64_list.len());
                    assert_eq!(0f64, float64_list.get(0));
                    assert_eq!(123456789012345f64, float64_list.get(1));
                    assert_eq!(1e306, float64_list.get(2));
                    assert_eq!(-1e306, float64_list.get(3));
                    assert_eq!(1e-306, float64_list.get(4));
                    assert_eq!(-1e-306, float64_list.get(5));
                }

                // ...

                {
                    let mut struct_list = sub_reader.reborrow().get_struct_list().unwrap();
                    assert_eq!(3, struct_list.len());
                    assert_eq!("x structlist 1", struct_list.reborrow().get(0).get_text_field().unwrap());
                    assert_eq!("x structlist 2", struct_list.reborrow().get(1).get_text_field().unwrap());
                    assert_eq!("x structlist 3", struct_list.reborrow().get(2).get_text_field().unwrap());
                }

                {
                    let enum_list = sub_reader.get_enum_list().unwrap();
                    assert_eq!(3, enum_list.len());
                    assert!(Ok(TestEnum::Qux) == enum_list.get(0));
                    assert!(Ok(TestEnum::Bar) == enum_list.get(1));
                    assert!(Ok(TestEnum::Grault) == enum_list.get(2));
                }
            }

            assert!(Ok(TestEnum::Corge) == reader.reborrow().get_enum_field());
            assert_eq!(6, reader.reborrow().get_void_list().unwrap().len());

            {
                let bool_list = reader.reborrow().get_bool_list().unwrap();
                assert_eq!(4, bool_list.len());
                assert_eq!(true, bool_list.get(0));
                assert_eq!(false, bool_list.get(1));
                assert_eq!(false, bool_list.get(2));
                assert_eq!(true, bool_list.get(3));
            }

            // ...

            {
                let mut text_list = reader.reborrow().get_text_list().unwrap();
                assert_eq!(3, text_list.len());
                assert_eq!("plugh", text_list.reborrow().get(0).unwrap());
                assert_eq!("xyzzy", text_list.reborrow().get(1).unwrap());
                assert_eq!("thud", text_list.reborrow().get(2).unwrap());
            }

            {
                let mut data_list = reader.reborrow().get_data_list().unwrap();
                assert_eq!(3, data_list.len());
                assert_eq!(b"oops", data_list.reborrow().get(0).unwrap());
                assert_eq!(b"exhausted", data_list.reborrow().get(1).unwrap());
                assert_eq!(b"rfc3092", data_list.reborrow().get(2).unwrap());
            }

            {
                let mut struct_list = reader.reborrow().get_struct_list().unwrap();
                assert_eq!(3, struct_list.len());
                assert_eq!("structlist 1", struct_list.reborrow().get(0).get_text_field().unwrap());
                assert_eq!("structlist 2", struct_list.reborrow().get(1).get_text_field().unwrap());
                assert_eq!("structlist 3", struct_list.reborrow().get(2).get_text_field().unwrap());

            }

            // ...
        }
    }
));

check_test_message_impl!(test_all_types::Reader);
check_test_message_impl!(test_all_types::Builder);
check_test_message_impl!(test_defaults::Reader);
check_test_message_impl!(test_defaults::Builder);

pub fn dynamic_init_test_message(mut builder: ::capnp::dynamic_struct::Builder<'_>) {
    builder.set_named("voidField", ().into()).unwrap();
    builder.set_named("boolField", true.into()).unwrap();
    builder.set_named("int8Field", (-123i8).into()).unwrap();
    builder.set_named("int16Field", (-12345i16).into()).unwrap();
    builder
        .set_named("int32Field", (-12345678i32).into())
        .unwrap();
    builder
        .set_named("int64Field", (-123456789012345i64).into())
        .unwrap();
    builder.set_named("uInt8Field", (234u8).into()).unwrap();
    builder.set_named("uInt16Field", (45678u16).into()).unwrap();
    builder
        .set_named("uInt32Field", (3456789012u32).into())
        .unwrap();
    builder
        .set_named("uInt64Field", (12345678901234567890u64).into())
        .unwrap();
    builder
        .set_named("float32Field", (1234.5f32).into())
        .unwrap();
    builder
        .set_named("float64Field", (-123e45f64).into())
        .unwrap();
    builder.set_named("textField", "foo".into()).unwrap();
    builder.set_named("dataField", b"bar"[..].into()).unwrap();
    {
        let mut substruct = builder
            .reborrow()
            .init_named("structField")
            .unwrap()
            .downcast::<::capnp::dynamic_struct::Builder<'_>>();
        substruct.set_named("voidField", ().into()).unwrap();
        substruct.set_named("boolField", true.into()).unwrap();
        substruct.set_named("int8Field", (-12i8).into()).unwrap();
        substruct.set_named("int16Field", (3456i16).into()).unwrap();
        substruct
            .set_named("int32Field", (-78901234i32).into())
            .unwrap();
        substruct
            .set_named("int64Field", (56789012345678i64).into())
            .unwrap();
        substruct.set_named("uInt8Field", (90u8).into()).unwrap();
        substruct
            .set_named("uInt16Field", (1234u16).into())
            .unwrap();
        substruct
            .set_named("uInt32Field", (56789012u32).into())
            .unwrap();
        substruct
            .set_named("uInt64Field", (345678901234567890u64).into())
            .unwrap();
        substruct
            .set_named("float32Field", (-1.25e-10f32).into())
            .unwrap();
        substruct
            .set_named("float64Field", (345f64).into())
            .unwrap();
        substruct.set_named("textField", "baz".into()).unwrap();
        substruct.set_named("dataField", b"qux"[..].into()).unwrap();
        {
            let mut subsubstruct = substruct
                .reborrow()
                .init_named("structField")
                .unwrap()
                .downcast::<::capnp::dynamic_struct::Builder<'_>>();
            subsubstruct
                .set_named("textField", "nested".into())
                .unwrap();
            subsubstruct
                .init_named("structField")
                .unwrap()
                .downcast::<::capnp::dynamic_struct::Builder<'_>>()
                .set_named("textField", "really nested".into())
                .unwrap();
        }
        substruct
            .set_named("enumField", TestEnum::Baz.into())
            .unwrap();

        substruct.reborrow().initn_named("voidList", 3).unwrap();

        {
            let mut bool_list = substruct
                .reborrow()
                .initn_named("boolList", 5)
                .unwrap()
                .downcast::<::capnp::dynamic_list::Builder<'_>>();
            bool_list.set(0, false.into()).unwrap();
            bool_list.set(1, true.into()).unwrap();
            bool_list.set(2, false.into()).unwrap();
            bool_list.set(3, true.into()).unwrap();
            bool_list.set(4, true.into()).unwrap();
        }

        {
            let mut int8_list = substruct
                .reborrow()
                .initn_named("int8List", 4)
                .unwrap()
                .downcast::<::capnp::dynamic_list::Builder<'_>>();
            int8_list.set(0, 12i8.into()).unwrap();
            int8_list.set(1, (-34i8).into()).unwrap();
            int8_list.set(2, (-0x80i8).into()).unwrap();
            int8_list.set(3, (0x7fi8).into()).unwrap();
        }
    }
    builder
        .set_named("enumField", TestEnum::Corge.into())
        .unwrap();
    {
        let mut void_list = builder
            .reborrow()
            .initn_named("voidList", 6)
            .unwrap()
            .downcast::<::capnp::dynamic_list::Builder<'_>>();
        for ii in 0..6 {
            void_list.set(ii, ().into()).unwrap();
        }
    }
    {
        let mut bool_list = builder
            .reborrow()
            .initn_named("boolList", 4)
            .unwrap()
            .downcast::<::capnp::dynamic_list::Builder<'_>>();
        bool_list.set(0, true.into()).unwrap();
        bool_list.set(1, false.into()).unwrap();
        bool_list.set(2, false.into()).unwrap();
        bool_list.set(3, true.into()).unwrap();
    }
    {
        let mut int8_list = builder
            .reborrow()
            .initn_named("int8List", 2)
            .unwrap()
            .downcast::<::capnp::dynamic_list::Builder<'_>>();
        int8_list.set(0, 111i8.into()).unwrap();
        int8_list.set(1, (-111i8).into()).unwrap();
    }
    {
        let mut int16_list = builder
            .reborrow()
            .initn_named("int16List", 2)
            .unwrap()
            .downcast::<::capnp::dynamic_list::Builder<'_>>();
        int16_list.set(0, 11111i16.into()).unwrap();
        int16_list.set(1, (-11111i16).into()).unwrap();
    }
    {
        let mut int32_list = builder
            .reborrow()
            .initn_named("int32List", 2)
            .unwrap()
            .downcast::<::capnp::dynamic_list::Builder<'_>>();
        int32_list.set(0, 111111111i32.into()).unwrap();
        int32_list.set(1, (-111111111i32).into()).unwrap();
    }
    {
        let mut int64_list = builder
            .reborrow()
            .initn_named("int64List", 2)
            .unwrap()
            .downcast::<::capnp::dynamic_list::Builder<'_>>();
        int64_list.set(0, 1111111111111111111i64.into()).unwrap();
        int64_list.set(1, (-1111111111111111111i64).into()).unwrap();
    }
    {
        let mut uint8_list = builder
            .reborrow()
            .initn_named("uInt8List", 2)
            .unwrap()
            .downcast::<::capnp::dynamic_list::Builder<'_>>();
        uint8_list.set(0, 111u8.into()).unwrap();
        uint8_list.set(1, 222u8.into()).unwrap();
    }
    {
        let mut uint16_list = builder
            .reborrow()
            .initn_named("uInt16List", 2)
            .unwrap()
            .downcast::<::capnp::dynamic_list::Builder<'_>>();
        uint16_list.set(0, 33333u16.into()).unwrap();
        uint16_list.set(1, 44444u16.into()).unwrap();
    }
    {
        let mut uint32_list = builder
            .reborrow()
            .initn_named("uInt32List", 1)
            .unwrap()
            .downcast::<::capnp::dynamic_list::Builder<'_>>();
        uint32_list.set(0, 3333333333u32.into()).unwrap();
    }
    {
        let mut uint64_list = builder
            .reborrow()
            .initn_named("uInt64List", 1)
            .unwrap()
            .downcast::<::capnp::dynamic_list::Builder<'_>>();
        uint64_list.set(0, 11111111111111111111u64.into()).unwrap();
    }
    {
        let mut float32_list = builder
            .reborrow()
            .initn_named("float32List", 4)
            .unwrap()
            .downcast::<::capnp::dynamic_list::Builder<'_>>();
        float32_list.set(0, 5555.5f32.into()).unwrap();
        float32_list.set(1, f32::INFINITY.into()).unwrap();
        float32_list.set(2, (-f32::INFINITY).into()).unwrap();
        float32_list.set(3, f32::NAN.into()).unwrap();
    }
    {
        let mut float64_list = builder
            .reborrow()
            .initn_named("float64List", 4)
            .unwrap()
            .downcast::<::capnp::dynamic_list::Builder<'_>>();
        float64_list.set(0, 7777.75f64.into()).unwrap();
        float64_list.set(1, f64::INFINITY.into()).unwrap();
        float64_list.set(2, (-f64::INFINITY).into()).unwrap();
        float64_list.set(3, f64::NAN.into()).unwrap();
    }
    {
        let mut text_list: capnp::dynamic_list::Builder<'_> = builder
            .reborrow()
            .initn_named("textList", 3)
            .unwrap()
            .downcast();
        text_list.set(0, "plugh".into()).unwrap();
        text_list.set(1, "xyzzy".into()).unwrap();
        text_list.set(2, "thud".into()).unwrap();
    }
    {
        let mut data_list: capnp::dynamic_list::Builder<'_> = builder
            .reborrow()
            .initn_named("dataList", 3)
            .unwrap()
            .downcast();
        data_list.set(0, b"oops"[..].into()).unwrap();
        data_list.set(1, b"exhausted"[..].into()).unwrap();
        data_list.set(2, b"rfc3092"[..].into()).unwrap();
    }

    {
        let mut struct_list: capnp::dynamic_list::Builder<'_> = builder
            .reborrow()
            .initn_named("structList", 3)
            .unwrap()
            .downcast();
        struct_list
            .reborrow()
            .get(0)
            .unwrap()
            .downcast::<capnp::dynamic_struct::Builder<'_>>()
            .set_named("textField", "structlist 1".into())
            .unwrap();
        struct_list
            .reborrow()
            .get(1)
            .unwrap()
            .downcast::<capnp::dynamic_struct::Builder<'_>>()
            .set_named("textField", "structlist 2".into())
            .unwrap();
        struct_list
            .get(2)
            .unwrap()
            .downcast::<capnp::dynamic_struct::Builder<'_>>()
            .set_named("textField", "structlist 3".into())
            .unwrap();
    }
    {
        let mut enum_list: capnp::dynamic_list::Builder<'_> = builder
            .reborrow()
            .initn_named("enumList", 2)
            .unwrap()
            .downcast();
        enum_list.set(0, TestEnum::Foo.into()).unwrap();
        enum_list.set(1, TestEnum::Garply.into()).unwrap();
    }
}

pub fn dynamic_check_test_message(reader: capnp::dynamic_struct::Reader<'_>) {
    reader.get_named("voidField").unwrap().downcast::<()>();
    assert_eq!(
        true,
        reader.get_named("boolField").unwrap().downcast::<bool>()
    );
    assert_eq!(-123i8, reader.get_named("int8Field").unwrap().downcast());
    assert_eq!(
        -12345i16,
        reader.get_named("int16Field").unwrap().downcast()
    );
    assert_eq!(
        -12345678i32,
        reader.get_named("int32Field").unwrap().downcast()
    );
    assert_eq!(
        -123456789012345i64,
        reader.get_named("int64Field").unwrap().downcast()
    );
    assert_eq!(234u8, reader.get_named("uInt8Field").unwrap().downcast());
    assert_eq!(
        45678u16,
        reader.get_named("uInt16Field").unwrap().downcast()
    );
    assert_eq!(
        3456789012u32,
        reader.get_named("uInt32Field").unwrap().downcast()
    );
    assert_eq!(
        12345678901234567890u64,
        reader.get_named("uInt64Field").unwrap().downcast()
    );
    assert_eq!(
        1234.5f32,
        reader.get_named("float32Field").unwrap().downcast()
    );
    assert_eq!(
        -123e45f64,
        reader.get_named("float64Field").unwrap().downcast()
    );
    assert_eq!(
        "foo",
        reader
            .get_named("textField")
            .unwrap()
            .downcast::<capnp::text::Reader<'_>>()
    );
    assert_eq!(
        &b"bar"[..],
        reader
            .get_named("dataField")
            .unwrap()
            .downcast::<capnp::data::Reader<'_>>()
    );
    {
        let substruct: capnp::dynamic_struct::Reader<'_> =
            reader.get_named("structField").unwrap().downcast();
        substruct.get_named("voidField").unwrap();
        assert_eq!(
            true,
            substruct.get_named("boolField").unwrap().downcast::<bool>()
        );
        assert_eq!(-12i8, substruct.get_named("int8Field").unwrap().downcast());
        assert_eq!(
            3456i16,
            substruct.get_named("int16Field").unwrap().downcast()
        );
    }
    assert_eq!(
        "corge",
        reader
            .get_named("enumField")
            .unwrap()
            .downcast::<capnp::dynamic_value::Enum>()
            .get_enumerant()
            .unwrap()
            .unwrap()
            .get_proto()
            .get_name()
            .unwrap()
    );
    {
        let void_list: capnp::dynamic_list::Reader<'_> =
            reader.get_named("voidList").unwrap().downcast();
        assert_eq!(6, void_list.len());
        for ii in 0..6 {
            let () = void_list.get(ii).unwrap().downcast();
        }
    }
    {
        let bool_list: capnp::dynamic_list::Reader<'_> =
            reader.get_named("boolList").unwrap().downcast();
        assert_eq!(4, bool_list.len());
        assert_eq!(true, bool_list.get(0).unwrap().downcast());
        assert_eq!(false, bool_list.get(1).unwrap().downcast());
        assert_eq!(false, bool_list.get(2).unwrap().downcast());
        assert_eq!(true, bool_list.get(3).unwrap().downcast());
    }
    {
        let int8_list: capnp::dynamic_list::Reader<'_> =
            reader.get_named("int8List").unwrap().downcast();
        assert_eq!(2, int8_list.len());
        assert_eq!(111i8, int8_list.get(0).unwrap().downcast());
        assert_eq!(-111i8, int8_list.get(1).unwrap().downcast());
    }
    {
        let int16_list: capnp::dynamic_list::Reader<'_> =
            reader.get_named("int16List").unwrap().downcast();
        assert_eq!(2, int16_list.len());
        assert_eq!(11111i16, int16_list.get(0).unwrap().downcast());
        assert_eq!(-11111i16, int16_list.get(1).unwrap().downcast());
    }
    {
        let int32_list: capnp::dynamic_list::Reader<'_> =
            reader.get_named("int32List").unwrap().downcast();
        assert_eq!(2, int32_list.len());
        assert_eq!(111111111i32, int32_list.get(0).unwrap().downcast());
        assert_eq!(-111111111i32, int32_list.get(1).unwrap().downcast());
    }
    {
        let int64_list: capnp::dynamic_list::Reader<'_> =
            reader.get_named("int64List").unwrap().downcast();
        assert_eq!(2, int64_list.len());
        assert_eq!(
            1111111111111111111i64,
            int64_list.get(0).unwrap().downcast()
        );
        assert_eq!(
            -1111111111111111111i64,
            int64_list.get(1).unwrap().downcast()
        );
    }
    {
        let uint8_list: capnp::dynamic_list::Reader<'_> =
            reader.get_named("uInt8List").unwrap().downcast();
        assert_eq!(2, uint8_list.len());
        assert_eq!(111u8, uint8_list.get(0).unwrap().downcast());
        assert_eq!(222u8, uint8_list.get(1).unwrap().downcast());
    }
    {
        let uint16_list: capnp::dynamic_list::Reader<'_> =
            reader.get_named("uInt16List").unwrap().downcast();
        assert_eq!(2, uint16_list.len());
        assert_eq!(33333u16, uint16_list.get(0).unwrap().downcast());
        assert_eq!(44444u16, uint16_list.get(1).unwrap().downcast());
    }
    {
        let uint32_list: capnp::dynamic_list::Reader<'_> =
            reader.get_named("uInt32List").unwrap().downcast();
        assert_eq!(1, uint32_list.len());
        assert_eq!(3333333333u32, uint32_list.get(0).unwrap().downcast());
    }
    {
        let uint64_list: capnp::dynamic_list::Reader<'_> =
            reader.get_named("uInt64List").unwrap().downcast();
        assert_eq!(1, uint64_list.len());
        assert_eq!(
            11111111111111111111u64,
            uint64_list.get(0).unwrap().downcast()
        );
    }
    {
        let float32_list: capnp::dynamic_list::Reader<'_> =
            reader.get_named("float32List").unwrap().downcast();
        assert_eq!(4, float32_list.len());
        assert_eq!(5555.5f32, float32_list.get(0).unwrap().downcast());
        assert_eq!(f32::INFINITY, float32_list.get(1).unwrap().downcast());
        assert_eq!(-f32::INFINITY, float32_list.get(2).unwrap().downcast());
        assert!(float32_list.get(3).unwrap().downcast::<f32>().is_nan());
    }
    {
        let float64_list: capnp::dynamic_list::Reader<'_> =
            reader.get_named("float64List").unwrap().downcast();
        assert_eq!(4, float64_list.len());
        assert_eq!(7777.75f64, float64_list.get(0).unwrap().downcast());
        assert_eq!(f64::INFINITY, float64_list.get(1).unwrap().downcast());
        assert_eq!(-f64::INFINITY, float64_list.get(2).unwrap().downcast());
        assert!(float64_list.get(3).unwrap().downcast::<f64>().is_nan());
    }
    {
        let text_list: capnp::dynamic_list::Reader<'_> =
            reader.get_named("textList").unwrap().downcast();
        assert_eq!(3, text_list.len());
        assert_eq!(
            "plugh",
            text_list
                .get(0)
                .unwrap()
                .downcast::<capnp::text::Reader<'_>>()
        );
        assert_eq!(
            "xyzzy",
            text_list
                .get(1)
                .unwrap()
                .downcast::<capnp::text::Reader<'_>>()
        );
        assert_eq!(
            "thud",
            text_list
                .get(2)
                .unwrap()
                .downcast::<capnp::text::Reader<'_>>()
        );
    }

    {
        let data_list: capnp::dynamic_list::Reader<'_> =
            reader.get_named("dataList").unwrap().downcast();
        assert_eq!(3, data_list.len());
        assert_eq!(
            b"oops",
            data_list
                .get(0)
                .unwrap()
                .downcast::<capnp::data::Reader<'_>>()
        );
        assert_eq!(
            b"exhausted",
            data_list
                .get(1)
                .unwrap()
                .downcast::<capnp::data::Reader<'_>>()
        );
        assert_eq!(
            b"rfc3092",
            data_list
                .get(2)
                .unwrap()
                .downcast::<capnp::data::Reader<'_>>()
        );
    }

    {
        let struct_list: capnp::dynamic_list::Reader<'_> =
            reader.get_named("structList").unwrap().downcast();
        assert_eq!(3, struct_list.len());
        assert_eq!(
            "structlist 1",
            struct_list
                .get(0)
                .unwrap()
                .downcast::<capnp::dynamic_struct::Reader<'_>>()
                .get_named("textField")
                .unwrap()
                .downcast::<capnp::text::Reader<'_>>()
        );
        assert_eq!(
            "structlist 2",
            struct_list
                .get(1)
                .unwrap()
                .downcast::<capnp::dynamic_struct::Reader<'_>>()
                .get_named("textField")
                .unwrap()
                .downcast::<capnp::text::Reader<'_>>()
        );
        assert_eq!(
            "structlist 3",
            struct_list
                .get(2)
                .unwrap()
                .downcast::<capnp::dynamic_struct::Reader<'_>>()
                .get_named("textField")
                .unwrap()
                .downcast::<capnp::text::Reader<'_>>()
        );
    }
    {
        let enum_list: capnp::dynamic_list::Reader<'_> =
            reader.get_named("enumList").unwrap().downcast();
        assert_eq!(2, enum_list.len());
        assert_eq!(
            "foo",
            enum_list
                .get(0)
                .unwrap()
                .downcast::<capnp::dynamic_value::Enum>()
                .get_enumerant()
                .unwrap()
                .unwrap()
                .get_proto()
                .get_name()
                .unwrap()
        );
        assert_eq!(
            "garply",
            enum_list
                .get(1)
                .unwrap()
                .downcast::<capnp::dynamic_value::Enum>()
                .get_enumerant()
                .unwrap()
                .unwrap()
                .get_proto()
                .get_name()
                .unwrap()
        );
    }
}

pub fn dynamic_check_test_message_builder(mut builder: capnp::dynamic_struct::Builder<'_>) {
    builder
        .reborrow()
        .get_named("voidField")
        .unwrap()
        .downcast::<()>();
    assert_eq!(
        true,
        builder
            .reborrow()
            .get_named("boolField")
            .unwrap()
            .downcast::<bool>()
    );
    assert_eq!(
        -123i8,
        builder
            .reborrow()
            .get_named("int8Field")
            .unwrap()
            .downcast()
    );
    assert_eq!(
        -12345i16,
        builder
            .reborrow()
            .get_named("int16Field")
            .unwrap()
            .downcast()
    );
    assert_eq!(
        -12345678i32,
        builder
            .reborrow()
            .get_named("int32Field")
            .unwrap()
            .downcast()
    );
    assert_eq!(
        -123456789012345i64,
        builder
            .reborrow()
            .get_named("int64Field")
            .unwrap()
            .downcast()
    );
    assert_eq!(
        234u8,
        builder
            .reborrow()
            .get_named("uInt8Field")
            .unwrap()
            .downcast()
    );
    assert_eq!(
        45678u16,
        builder
            .reborrow()
            .get_named("uInt16Field")
            .unwrap()
            .downcast()
    );
    assert_eq!(
        3456789012u32,
        builder
            .reborrow()
            .get_named("uInt32Field")
            .unwrap()
            .downcast()
    );
    assert_eq!(
        12345678901234567890u64,
        builder
            .reborrow()
            .get_named("uInt64Field")
            .unwrap()
            .downcast()
    );
    assert_eq!(
        1234.5f32,
        builder
            .reborrow()
            .get_named("float32Field")
            .unwrap()
            .downcast()
    );
    assert_eq!(
        -123e45f64,
        builder
            .reborrow()
            .get_named("float64Field")
            .unwrap()
            .downcast()
    );

    assert_eq!(
        "foo",
        builder
            .reborrow()
            .get_named("textField")
            .unwrap()
            .into_reader()
            .downcast::<capnp::text::Reader<'_>>()
    );
    assert_eq!(
        &b"bar"[..],
        builder
            .reborrow()
            .get_named("dataField")
            .unwrap()
            .into_reader()
            .downcast::<capnp::data::Reader<'_>>()
    );

    {
        let mut substruct: capnp::dynamic_struct::Builder<'_> = builder
            .reborrow()
            .get_named("structField")
            .unwrap()
            .downcast();
        substruct.reborrow().get_named("voidField").unwrap();
        assert_eq!(
            true,
            substruct
                .reborrow()
                .get_named("boolField")
                .unwrap()
                .downcast::<bool>()
        );
        assert_eq!(
            -12i8,
            substruct
                .reborrow()
                .get_named("int8Field")
                .unwrap()
                .downcast()
        );
        assert_eq!(
            3456i16,
            substruct
                .reborrow()
                .get_named("int16Field")
                .unwrap()
                .downcast()
        );
    }
    assert_eq!(
        "corge",
        builder
            .reborrow()
            .get_named("enumField")
            .unwrap()
            .downcast::<capnp::dynamic_value::Enum>()
            .get_enumerant()
            .unwrap()
            .unwrap()
            .get_proto()
            .get_name()
            .unwrap()
    );
    {
        let mut void_list: capnp::dynamic_list::Builder<'_> =
            builder.reborrow().get_named("voidList").unwrap().downcast();
        assert_eq!(6, void_list.len());
        for ii in 0..6 {
            let () = void_list.reborrow().get(ii).unwrap().downcast();
        }
    }
    {
        let mut bool_list: capnp::dynamic_list::Builder<'_> =
            builder.reborrow().get_named("boolList").unwrap().downcast();
        assert_eq!(4, bool_list.len());
        assert_eq!(true, bool_list.reborrow().get(0).unwrap().downcast());
        assert_eq!(false, bool_list.reborrow().get(1).unwrap().downcast());
        assert_eq!(false, bool_list.reborrow().get(2).unwrap().downcast());
        assert_eq!(true, bool_list.reborrow().get(3).unwrap().downcast());
    }
    {
        let mut int8_list: capnp::dynamic_list::Builder<'_> =
            builder.reborrow().get_named("int8List").unwrap().downcast();
        assert_eq!(2, int8_list.len());
        assert_eq!(111i8, int8_list.reborrow().get(0).unwrap().downcast());
        assert_eq!(-111i8, int8_list.reborrow().get(1).unwrap().downcast());
    }
    {
        let mut int16_list: capnp::dynamic_list::Builder<'_> = builder
            .reborrow()
            .get_named("int16List")
            .unwrap()
            .downcast();
        assert_eq!(2, int16_list.len());
        assert_eq!(11111i16, int16_list.reborrow().get(0).unwrap().downcast());
        assert_eq!(-11111i16, int16_list.reborrow().get(1).unwrap().downcast());
    }
    {
        let mut int32_list: capnp::dynamic_list::Builder<'_> = builder
            .reborrow()
            .get_named("int32List")
            .unwrap()
            .downcast();
        assert_eq!(2, int32_list.len());
        assert_eq!(
            111111111i32,
            int32_list.reborrow().get(0).unwrap().downcast()
        );
        assert_eq!(
            -111111111i32,
            int32_list.reborrow().get(1).unwrap().downcast()
        );
    }
    {
        let mut int64_list: capnp::dynamic_list::Builder<'_> = builder
            .reborrow()
            .get_named("int64List")
            .unwrap()
            .downcast();
        assert_eq!(2, int64_list.len());
        assert_eq!(
            1111111111111111111i64,
            int64_list.reborrow().get(0).unwrap().downcast()
        );
        assert_eq!(
            -1111111111111111111i64,
            int64_list.reborrow().get(1).unwrap().downcast()
        );
    }
    {
        let mut uint8_list: capnp::dynamic_list::Builder<'_> = builder
            .reborrow()
            .get_named("uInt8List")
            .unwrap()
            .downcast();
        assert_eq!(2, uint8_list.len());
        assert_eq!(111u8, uint8_list.reborrow().get(0).unwrap().downcast());
        assert_eq!(222u8, uint8_list.reborrow().get(1).unwrap().downcast());
    }
    {
        let mut uint16_list: capnp::dynamic_list::Builder<'_> = builder
            .reborrow()
            .get_named("uInt16List")
            .unwrap()
            .downcast();
        assert_eq!(2, uint16_list.len());
        assert_eq!(33333u16, uint16_list.reborrow().get(0).unwrap().downcast());
        assert_eq!(44444u16, uint16_list.reborrow().get(1).unwrap().downcast());
    }
    {
        let uint32_list: capnp::dynamic_list::Builder<'_> = builder
            .reborrow()
            .get_named("uInt32List")
            .unwrap()
            .downcast();
        assert_eq!(1, uint32_list.len());
        assert_eq!(3333333333u32, uint32_list.get(0).unwrap().downcast());
    }
    {
        let uint64_list: capnp::dynamic_list::Builder<'_> = builder
            .reborrow()
            .get_named("uInt64List")
            .unwrap()
            .downcast();
        assert_eq!(1, uint64_list.len());
        assert_eq!(
            11111111111111111111u64,
            uint64_list.get(0).unwrap().downcast()
        );
    }
    {
        let mut float32_list: capnp::dynamic_list::Builder<'_> = builder
            .reborrow()
            .get_named("float32List")
            .unwrap()
            .downcast();
        assert_eq!(4, float32_list.len());
        assert_eq!(
            5555.5f32,
            float32_list.reborrow().get(0).unwrap().downcast()
        );
        assert_eq!(
            f32::INFINITY,
            float32_list.reborrow().get(1).unwrap().downcast()
        );
        assert_eq!(
            -f32::INFINITY,
            float32_list.reborrow().get(2).unwrap().downcast()
        );
        assert!(float32_list.get(3).unwrap().downcast::<f32>().is_nan());
    }
    {
        let mut float64_list: capnp::dynamic_list::Builder<'_> = builder
            .reborrow()
            .get_named("float64List")
            .unwrap()
            .downcast();
        assert_eq!(4, float64_list.len());
        assert_eq!(
            7777.75f64,
            float64_list.reborrow().get(0).unwrap().downcast()
        );
        assert_eq!(
            f64::INFINITY,
            float64_list.reborrow().get(1).unwrap().downcast()
        );
        assert_eq!(
            -f64::INFINITY,
            float64_list.reborrow().get(2).unwrap().downcast()
        );
        assert!(float64_list.get(3).unwrap().downcast::<f64>().is_nan());
    }
    {
        let mut text_list: capnp::dynamic_list::Builder<'_> =
            builder.reborrow().get_named("textList").unwrap().downcast();
        assert_eq!(3, text_list.len());
        assert_eq!(
            "plugh",
            text_list
                .reborrow()
                .get(0)
                .unwrap()
                .into_reader()
                .downcast::<capnp::text::Reader<'_>>()
        );
        assert_eq!(
            "xyzzy",
            text_list
                .reborrow()
                .get(1)
                .unwrap()
                .into_reader()
                .downcast::<capnp::text::Reader<'_>>()
        );
        assert_eq!(
            "thud",
            text_list
                .reborrow()
                .get(2)
                .unwrap()
                .into_reader()
                .downcast::<capnp::text::Reader<'_>>()
        );
    }
    {
        let mut data_list: capnp::dynamic_list::Builder<'_> =
            builder.reborrow().get_named("dataList").unwrap().downcast();
        assert_eq!(3, data_list.len());
        assert_eq!(
            b"oops",
            data_list
                .reborrow()
                .get(0)
                .unwrap()
                .into_reader()
                .downcast::<capnp::data::Reader<'_>>()
        );
        assert_eq!(
            b"exhausted",
            data_list
                .reborrow()
                .get(1)
                .unwrap()
                .into_reader()
                .downcast::<capnp::data::Reader<'_>>()
        );
        assert_eq!(
            b"rfc3092",
            data_list
                .reborrow()
                .get(2)
                .unwrap()
                .into_reader()
                .downcast::<capnp::data::Reader<'_>>()
        );
    }
    {
        let mut struct_list: capnp::dynamic_list::Builder<'_> = builder
            .reborrow()
            .get_named("structList")
            .unwrap()
            .downcast();
        assert_eq!(3, struct_list.len());
        assert_eq!(
            "structlist 1",
            struct_list
                .reborrow()
                .get(0)
                .unwrap()
                .downcast::<capnp::dynamic_struct::Builder<'_>>()
                .get_named("textField")
                .unwrap()
                .into_reader()
                .downcast::<capnp::text::Reader<'_>>()
        );
        assert_eq!(
            "structlist 2",
            struct_list
                .reborrow()
                .get(1)
                .unwrap()
                .downcast::<capnp::dynamic_struct::Builder<'_>>()
                .get_named("textField")
                .unwrap()
                .into_reader()
                .downcast::<capnp::text::Reader<'_>>()
        );
        assert_eq!(
            "structlist 3",
            struct_list
                .reborrow()
                .get(2)
                .unwrap()
                .downcast::<capnp::dynamic_struct::Builder<'_>>()
                .get_named("textField")
                .unwrap()
                .into_reader()
                .downcast::<capnp::text::Reader<'_>>()
        );
    }
    {
        let mut enum_list: capnp::dynamic_list::Builder<'_> =
            builder.reborrow().get_named("enumList").unwrap().downcast();
        assert_eq!(2, enum_list.len());
        assert_eq!(
            "foo",
            enum_list
                .reborrow()
                .get(0)
                .unwrap()
                .downcast::<capnp::dynamic_value::Enum>()
                .get_enumerant()
                .unwrap()
                .unwrap()
                .get_proto()
                .get_name()
                .unwrap()
        );
        assert_eq!(
            "garply",
            enum_list
                .get(1)
                .unwrap()
                .downcast::<capnp::dynamic_value::Enum>()
                .get_enumerant()
                .unwrap()
                .unwrap()
                .get_proto()
                .get_name()
                .unwrap()
        );
    }
}
