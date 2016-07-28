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

#[macro_use]
extern crate capnp;

pub mod test_capnp {
  include!(concat!(env!("OUT_DIR"), "/test_capnp.rs"));
}

#[cfg(test)]
mod test_util;

#[cfg(test)]
mod tests {
    use capnp::message;
    use capnp::message::{ReaderOptions};

    #[test]
    fn test_prim_list () {

        use test_capnp::test_prim_list;

        // Make the first segment small to force allocation of a second segment.
        let mut message = message::Builder::new(message::HeapAllocator::new().first_segment_words(50));

        let mut test_prim_list = message.init_root::<test_prim_list::Builder>();
        assert_eq!(test_prim_list.has_bool_list(), false);
        assert_eq!(test_prim_list.has_void_list(), false);
        {
            {
                let mut uint8_list = test_prim_list.borrow().init_uint8_list(100);
                for i in 0..uint8_list.len() {
                    uint8_list.set(i, i as u8);
                }
            }

            {
                let mut uint64_list = test_prim_list.borrow().init_uint64_list(20);
                for i in 0..uint64_list.len() {
                    uint64_list.set(i, i as u64);
                }
            }

            {
                let mut bool_list = test_prim_list.borrow().init_bool_list(65);

                bool_list.set(0, true);
                bool_list.set(1, true);
                bool_list.set(2, true);
                bool_list.set(3, true);
                bool_list.set(5, true);
                bool_list.set(8, true);
                bool_list.set(13, true);
                bool_list.set(64, true);

                assert!(bool_list.get(0));
                assert!(!bool_list.get(4));
                assert!(!bool_list.get(63));
                assert!(bool_list.get(64));
            }

            let mut void_list = test_prim_list.borrow().init_void_list(1025);
            void_list.set(257, ());
        }
        assert_eq!(test_prim_list.has_bool_list(), true);
        assert_eq!(test_prim_list.has_void_list(), true);

        let test_prim_list_reader = test_prim_list.as_reader();
        let uint8_list = test_prim_list_reader.get_uint8_list().unwrap();
        for i in 0..uint8_list.len() {
            assert_eq!(uint8_list.get(i), i as u8);
        }
        let uint64_list = test_prim_list_reader.get_uint64_list().unwrap();
        for i in 0..uint64_list.len() {
            assert_eq!(uint64_list.get(i), i as u64);
        }

        assert_eq!(test_prim_list_reader.has_bool_list(), true);
        let bool_list = test_prim_list_reader.get_bool_list().unwrap();
        assert!(bool_list.get(0));
        assert!(bool_list.get(1));
        assert!(bool_list.get(2));
        assert!(bool_list.get(3));
        assert!(!bool_list.get(4));
        assert!(bool_list.get(5));
        assert!(!bool_list.get(6));
        assert!(!bool_list.get(7));
        assert!(bool_list.get(8));
        assert!(!bool_list.get(9));
        assert!(!bool_list.get(10));
        assert!(!bool_list.get(11));
        assert!(!bool_list.get(12));
        assert!(bool_list.get(13));
        assert!(!bool_list.get(63));
        assert!(bool_list.get(64));

        assert_eq!(test_prim_list_reader.get_void_list().unwrap().len(), 1025);
    }

    #[test]
    fn test_struct_list () {

        use test_capnp::test_struct_list;

        let mut message = message::Builder::new(message::HeapAllocator::new());

        let mut test_struct_list = message.init_root::<test_struct_list::Builder>();

        test_struct_list.borrow().init_struct_list(4);
        {
            let struct_list = test_struct_list.borrow().get_struct_list().unwrap();
            struct_list.get(0).init_uint8_list(1).set(0, 5u8);
        }

        {
            let reader = test_struct_list.as_reader();
            assert_eq!(reader.get_struct_list().unwrap().get(0).get_uint8_list().unwrap().get(0), 5u8);
        }
    }

    #[test]
    fn test_blob () {
        use test_capnp::test_blob;

        let mut message = message::Builder::new(message::HeapAllocator::new());
        let mut test_blob = message.init_root::<test_blob::Builder>();

        assert_eq!(test_blob.has_text_field(), false);
        test_blob.set_text_field("abcdefghi");
        assert_eq!(test_blob.has_text_field(), true);

        assert_eq!(test_blob.has_data_field(), false);
        test_blob.set_data_field(&[0u8, 1u8, 2u8, 3u8, 4u8]);
        assert_eq!(test_blob.has_data_field(), true);

        {
            let test_blob_reader = test_blob.borrow_as_reader();

            assert_eq!(test_blob_reader.has_text_field(), true);
            assert_eq!(test_blob_reader.has_data_field(), true);

            assert_eq!(test_blob_reader.get_text_field().unwrap(), "abcdefghi");
            assert!(test_blob_reader.get_data_field().unwrap() == [0u8, 1u8, 2u8, 3u8, 4u8]);
        }

        {
            let mut text = test_blob.borrow().init_text_field(10);
            assert_eq!(&*text,"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00");
            text.push_str("aabbccddee");
        }

        test_blob.borrow().init_data_field(7);
        assert!(test_blob.borrow().as_reader().get_data_field().unwrap() ==
                [0u8,0u8,0u8,0u8,0u8,0u8,0u8]);
        {
            let data_builder = test_blob.borrow().get_data_field().unwrap();
            for c in data_builder.iter_mut() {
                *c = 5;
            }
            data_builder[0] = 4u8;
        }

        assert_eq!(test_blob.borrow().as_reader().get_text_field().unwrap(), "aabbccddee");
        assert!(test_blob.borrow().as_reader().get_data_field().unwrap() == [4u8,5u8,5u8,5u8,5u8,5u8,5u8]);

        {
            test_blob.borrow().get_data_field().unwrap()[2] = 10;
        }
        assert!(test_blob.as_reader().get_data_field().unwrap() == [4u8,5u8,10u8,5u8,5u8,5u8,5u8]);
    }


    #[test]
    fn test_big_struct() {
        use test_capnp::test_big_struct;

        // Make the first segment small to force allocation of a second segment.
        let mut message = message::Builder::new(message::HeapAllocator::new().first_segment_words(5));

        let mut big_struct = message.init_root::<test_big_struct::Builder>();

        big_struct.set_bool_field(false);
        big_struct.set_int8_field(-128);
        big_struct.set_int16_field(0);
        big_struct.set_int32_field(1009);

        assert_eq!(big_struct.has_struct_field(), false);
        big_struct.borrow().init_struct_field();
        assert_eq!(big_struct.has_struct_field(), true);
        {
            let mut inner = big_struct.borrow().get_struct_field().unwrap();
            inner.set_float64_field(0.1234567);
            inner.set_bool_field_b(true);
        }

        big_struct.set_bool_field(true);


        let big_struct_reader = big_struct.as_reader();
        assert_eq!(big_struct_reader.has_struct_field(), true);
        assert_eq!(big_struct_reader.get_int8_field(), -128);
        assert_eq!(big_struct_reader.get_int32_field(), 1009);

        let inner_reader = big_struct_reader.get_struct_field().unwrap();
        assert!(!inner_reader.get_bool_field_a());
        assert!(inner_reader.get_bool_field_b());
        assert_eq!(inner_reader.get_float64_field(), 0.1234567);
    }

    #[test]
    fn test_complex_list () {
        use test_capnp::{test_complex_list, AnEnum};

        let mut message = message::Builder::new_default();

        let mut test_complex_list = message.init_root::<test_complex_list::Builder>();

        {
            {
                let mut enum_list = test_complex_list.borrow().init_enum_list(100);
                for i in 0..10 {
                    enum_list.set(i, AnEnum::Qux);
                }
                for i in 10..20 {
                    enum_list.set(i, AnEnum::Bar);
                }
            }

            {
                let mut text_list = test_complex_list.borrow().init_text_list(2);
                text_list.set(0, "garply");
                text_list.set(1, "foo");
            }

            {
                let mut data_list = test_complex_list.borrow().init_data_list(2);
                data_list.set(0, &[0u8, 1u8, 2u8]);
                data_list.set(1, &[255u8, 254u8, 253u8]);
            }

            {
                let mut prim_list_list = test_complex_list.borrow().init_prim_list_list(2);
                {
                    let mut prim_list = prim_list_list.borrow().init(0, 3);
                    prim_list.set(0, 5);
                    prim_list.set(1, 6);
                    prim_list.set(2, 7);
                    assert_eq!(prim_list.len(), 3);
                }
                let mut prim_list = prim_list_list.init(1, 1);
                prim_list.set(0,-1);
            }

            {
                let mut prim_list_list_list = test_complex_list.borrow().init_prim_list_list_list(2);
                {
                    let mut prim_list_list = prim_list_list_list.borrow().init(0, 2);
                    {
                        let mut prim_list = prim_list_list.borrow().init(0, 2);
                        prim_list.set(0, 0);
                        prim_list.set(1, 1);
                    }
                    let mut prim_list = prim_list_list.init(1, 1);
                    prim_list.set(0, 255);
                }
                let prim_list_list = prim_list_list_list.init(1, 1);
                let mut prim_list = prim_list_list.init(0, 3);
                prim_list.set(0, 10);
                prim_list.set(1, 9);
                prim_list.set(2, 8);
            }

            {
                let mut enum_list_list = test_complex_list.borrow().init_enum_list_list(2);
                {
                    let mut enum_list = enum_list_list.borrow().init(0, 1);
                    enum_list.set(0, AnEnum::Bar);
                }
                let mut enum_list = enum_list_list.init(1, 2);
                enum_list.set(0, AnEnum::Foo);
                enum_list.set(1, AnEnum::Qux);
            }

            {
                let text_list_list = test_complex_list.borrow().init_text_list_list(1);
                text_list_list.init(0,1).set(0, "abc");
            }

            {
                let data_list_list = test_complex_list.borrow().init_data_list_list(1);
                data_list_list.init(0,1).set(0, &[255, 254, 253]);
            }

            {
                let struct_list_list = test_complex_list.borrow().init_struct_list_list(1);
                struct_list_list.init(0,1).get(0).set_int8_field(-1);
            }
        }

        let complex_list_reader = test_complex_list.as_reader();
        let enum_list_reader = complex_list_reader.get_enum_list().unwrap();
        for i in 0..10 {
            assert!(enum_list_reader.get(i) == Ok(AnEnum::Qux));
        }
        for i in 10..20 {
            assert!(enum_list_reader.get(i) == Ok(AnEnum::Bar));
        }

        let text_list = complex_list_reader.get_text_list().unwrap();
        assert_eq!(text_list.len(), 2);
        assert_eq!(text_list.get(0).unwrap(), "garply");
        assert_eq!(text_list.get(1).unwrap(), "foo");

        let data_list = complex_list_reader.get_data_list().unwrap();
        assert_eq!(data_list.len(), 2);
        assert!(data_list.get(0).unwrap() == [0u8, 1u8, 2u8]);
        assert!(data_list.get(1).unwrap() == [255u8, 254u8, 253u8]);

        let prim_list_list = complex_list_reader.get_prim_list_list().unwrap();
        assert_eq!(prim_list_list.len(), 2);
        assert_eq!(prim_list_list.get(0).unwrap().len(), 3);
        assert!(prim_list_list.get(0).unwrap().get(0) == 5);
        assert!(prim_list_list.get(0).unwrap().get(1) == 6);
        assert!(prim_list_list.get(0).unwrap().get(2) == 7);
        assert!(prim_list_list.get(1).unwrap().get(0) == -1);

        let prim_list_list_list = complex_list_reader.get_prim_list_list_list().unwrap();
        assert!(prim_list_list_list.get(0).unwrap().get(0).unwrap().get(0) == 0);
        assert!(prim_list_list_list.get(0).unwrap().get(0).unwrap().get(1) == 1);
        assert!(prim_list_list_list.get(0).unwrap().get(1).unwrap().get(0) == 255);
        assert!(prim_list_list_list.get(1).unwrap().get(0).unwrap().get(0) == 10);
        assert!(prim_list_list_list.get(1).unwrap().get(0).unwrap().get(1) == 9);
        assert!(prim_list_list_list.get(1).unwrap().get(0).unwrap().get(2) == 8);

        let enum_list_list = complex_list_reader.get_enum_list_list().unwrap();
        assert!(enum_list_list.get(0).unwrap().get(0) == Ok(AnEnum::Bar));
        assert!(enum_list_list.get(1).unwrap().get(0) == Ok(AnEnum::Foo));
        assert!(enum_list_list.get(1).unwrap().get(1) == Ok(AnEnum::Qux));

        assert!(complex_list_reader.get_text_list_list().unwrap().get(0).unwrap().get(0).unwrap() == "abc");
        assert!(complex_list_reader.get_data_list_list().unwrap().get(0).unwrap().get(0).unwrap() == [255, 254, 253]);

        assert!(complex_list_reader.get_struct_list_list().unwrap().get(0).unwrap().get(0).get_int8_field() == -1);
    }

    #[test]
    fn test_defaults() {
        use test_capnp::test_defaults;

        let mut message = message::Builder::new_default();
        let mut test_defaults = message.init_root::<test_defaults::Builder>();

        assert_eq!(test_defaults.borrow().get_void_field(), ());
        assert_eq!(test_defaults.borrow().get_bool_field(), true);
        assert_eq!(test_defaults.borrow().get_int8_field(), -123);
        assert_eq!(test_defaults.borrow().get_int16_field(), -12345);
        assert_eq!(test_defaults.borrow().get_int32_field(), -12345678);
        assert_eq!(test_defaults.borrow().get_int64_field(), -123456789012345);
        assert_eq!(test_defaults.borrow().get_uint8_field(), 234u8);
        assert_eq!(test_defaults.borrow().get_uint16_field(), 45678u16);
        assert_eq!(test_defaults.borrow().get_uint32_field(), 3456789012u32);
        assert_eq!(test_defaults.borrow().get_uint64_field(), 12345678901234567890u64);
        assert_eq!(test_defaults.borrow().get_float32_field(), 1234.5);
        assert_eq!(test_defaults.borrow().get_float64_field(), -123e45);

        test_defaults.set_bool_field(false);
        test_defaults.set_int8_field(63);
        test_defaults.set_int16_field(-1123);
        test_defaults.set_int32_field(445678);
        test_defaults.set_int64_field(-990123456789);
        test_defaults.set_uint8_field(234);
        test_defaults.set_uint16_field(56789);
        test_defaults.set_uint32_field(123456789);
        test_defaults.set_uint64_field(123456789012345);
        test_defaults.set_float32_field(7890.123);
        test_defaults.set_float64_field(5e55);

        assert_eq!(test_defaults.borrow().get_bool_field(), false);
        assert_eq!(test_defaults.borrow().get_int8_field(), 63);
        assert_eq!(test_defaults.borrow().get_int16_field(), -1123);
        assert_eq!(test_defaults.borrow().get_int32_field(),  445678);
        assert_eq!(test_defaults.borrow().get_int64_field(), -990123456789);
        assert_eq!(test_defaults.borrow().get_uint8_field(), 234);
        assert_eq!(test_defaults.borrow().get_uint16_field(), 56789);
        assert_eq!(test_defaults.borrow().get_uint32_field(),  123456789);
        assert_eq!(test_defaults.borrow().get_uint64_field(),  123456789012345);
        assert_eq!(test_defaults.borrow().get_float32_field(), 7890.123);
        assert_eq!(test_defaults.borrow().get_float64_field(), 5e55);
    }

    #[test]
    fn test_any_pointer() {
        use test_capnp::{test_any_pointer, test_empty_struct, test_big_struct};

        let mut message = message::Builder::new_default();
        let mut test_any_pointer = message.init_root::<test_any_pointer::Builder>();

        test_any_pointer.borrow().init_any_pointer_field().set_as("xyzzy").unwrap();

        {
            let reader = test_any_pointer.borrow().as_reader();
            assert_eq!(reader.get_any_pointer_field().get_as::<::capnp::text::Reader>().unwrap(), "xyzzy");
        }

        test_any_pointer.borrow().get_any_pointer_field().init_as::<test_empty_struct::Builder>();
        test_any_pointer.borrow().get_any_pointer_field().get_as::<test_empty_struct::Builder>().unwrap();

        {
            let reader = test_any_pointer.borrow().as_reader();
            reader.get_any_pointer_field().get_as::<test_empty_struct::Reader>().unwrap();
        }

        {
            let mut message = message::Builder::new_default();
            let mut test_big_struct = message.init_root::<test_big_struct::Builder>();
            test_big_struct.set_int32_field(-12345);
            test_any_pointer.get_any_pointer_field().set_as(test_big_struct.borrow().as_reader()).unwrap();
        }

        fn _test_lifetimes(body : test_big_struct::Reader) {
            let mut message = message::Builder::new_default();
            message.set_root(body).unwrap();
        }

    }

    #[test]
    fn test_writable_struct_pointer() {
        use test_capnp::test_big_struct;

        let mut message = message::Builder::new_default();
        let mut big_struct = message.init_root::<test_big_struct::Builder>();


        let neg_seven : u64 = (-7i64) as u64;
        {
            let mut struct_field = big_struct.borrow().init_struct_field();
            assert_eq!(struct_field.borrow().get_uint64_field(), 0);

            struct_field.set_uint64_field(neg_seven);
            assert_eq!(struct_field.get_uint64_field(), neg_seven);
        }
        assert_eq!(big_struct.borrow().get_struct_field().unwrap().get_uint64_field(), neg_seven);
        {
            let mut struct_field = big_struct.borrow().init_struct_field();
            assert_eq!(struct_field.borrow().get_uint64_field(), 0);
            assert_eq!(struct_field.get_uint32_field(), 0);
        }

        {
            // getting before init is the same as init
            assert_eq!(big_struct.borrow().get_another_struct_field().unwrap().get_uint64_field(), 0);
            big_struct.borrow().get_another_struct_field().unwrap().set_uint32_field(4294967265);

            // Alas, we need to make a copy to appease the borrow checker.
            let mut other_message = message::Builder::new_default();
            other_message.set_root(big_struct.borrow().get_another_struct_field().unwrap().as_reader()).unwrap();
            big_struct.set_struct_field(
                other_message.get_root::<test_big_struct::inner::Builder>().unwrap().as_reader()).unwrap();
        }

        assert_eq!(big_struct.borrow().get_struct_field().unwrap().get_uint32_field(), 4294967265);
        {
            let mut other_struct_field = big_struct.borrow().get_another_struct_field().unwrap();
            assert_eq!(other_struct_field.borrow().get_uint32_field(), 4294967265);
            other_struct_field.set_uint32_field(42);
            assert_eq!(other_struct_field.get_uint32_field(), 42);
        }
        assert_eq!(big_struct.borrow().get_struct_field().unwrap().get_uint32_field(), 4294967265);
        assert_eq!(big_struct.get_another_struct_field().unwrap().get_uint32_field(), 42);
    }

    #[test]
    fn test_generic_one_parameter() {
        use test_capnp::brand_once;

        let mut message_for_brand = message::Builder::new_default();
        let mut branded = message_for_brand.init_root::<brand_once::Builder>();
        {
            let branded_field = branded.borrow().init_branded_field();
            let mut foo = branded_field.init_generic_field();
            foo.set_text_field("blah");
        }

        let reader = branded.as_reader();
        assert_eq!("blah", reader.get_branded_field().unwrap().get_generic_field().unwrap().get_text_field().unwrap());
    }

    #[test]
    fn test_generic_two_parameter() {
        use test_capnp::brand_twice;

        let mut message_for_brand = message::Builder::new_default();
        let mut branded = message_for_brand.init_root::<brand_twice::Builder>();
        {
            let mut baz = branded.borrow().init_baz_field();
            baz.set_foo_field("blah").unwrap();
            let mut bar = baz.init_bar_field();
            bar.set_text_field("some text");
            bar.set_data_field("some data".as_bytes());
        }

        let reader = branded.as_reader();
        assert_eq!("blah", reader.get_baz_field().unwrap().get_foo_field().unwrap());
        assert_eq!("some text", reader.get_baz_field().unwrap().get_bar_field().unwrap().get_text_field().unwrap());
        assert_eq!("some data".as_bytes(), reader.get_baz_field().unwrap().get_bar_field().unwrap().get_data_field().unwrap());
    }

    #[test]
    fn test_generics() {
        use capnp::text;
        use test_capnp::{test_generics, test_all_types};
        let mut message = message::Builder::new_default();
        let mut root: test_generics::Builder<test_all_types::Owned, text::Owned> = message.init_root();
        ::test_util::init_test_message(root.borrow().get_foo().unwrap());
        root.borrow().get_dub().unwrap().set_foo("Hello").unwrap();
        {
            let mut bar: ::capnp::primitive_list::Builder<u8> = root.borrow().get_dub().unwrap().initn_bar(1);
            bar.set(0, 11);
        }
        {
            let mut rev_bar = root.borrow().get_rev().unwrap().get_bar().unwrap();
            rev_bar.set_int8_field(111);
            let mut bool_list = rev_bar.init_bool_list(2);
            bool_list.set(0, false);
            bool_list.set(1, true);
        }

        ::test_util::CheckTestMessage::check_test_message(root.borrow().get_foo().unwrap());
        let root_reader = root.as_reader();
        ::test_util::CheckTestMessage::check_test_message(root_reader.borrow().get_foo().unwrap());
        let dub_reader = root_reader.get_dub().unwrap();
        assert_eq!("Hello", dub_reader.get_foo().unwrap());
        let bar_reader = dub_reader.get_bar().unwrap();
        assert_eq!(bar_reader.len(), 1);
        assert_eq!(bar_reader.get(0), 11);
    }

    #[test]
    fn test_union() {
        use test_capnp::test_union;

        let mut message = message::Builder::new_default();
        let mut union_struct = message.init_root::<test_union::Builder>();

        union_struct.borrow().get_union0().set_u0f0s0(());
        match union_struct.borrow().get_union0().which() {
            Ok(test_union::union0::U0f0s0(())) => {}
            _ => panic!()
        }
        union_struct.borrow().init_union0().set_u0f0s1(true);
        match union_struct.borrow().get_union0().which() {
            Ok(test_union::union0::U0f0s1(true)) => {}
            _ => panic!()
        }
        union_struct.borrow().init_union0().set_u0f0s8(127);
        match union_struct.borrow().get_union0().which() {
            Ok(test_union::union0::U0f0s8(127)) => {}
            _ => panic!()
        }

        assert_eq!(union_struct.borrow().get_union0().has_u0f0sp(), false);
        union_struct.borrow().init_union0().set_u0f0sp("abcdef");
        assert_eq!(union_struct.get_union0().has_u0f0sp(), true);
    }

    #[test]
    fn test_constants() {
        use test_capnp::test_constants;
        assert_eq!(test_constants::VOID_CONST, ());
        assert_eq!(test_constants::BOOL_CONST, true);
        assert_eq!(test_constants::INT8_CONST, -123);
        assert_eq!(test_constants::INT16_CONST, -12345);
        assert_eq!(test_constants::INT32_CONST, -12345678);
        assert_eq!(test_constants::INT64_CONST, -123456789012345);
        assert_eq!(test_constants::UINT8_CONST, 234);
        assert_eq!(test_constants::UINT16_CONST, 45678);
        assert_eq!(test_constants::UINT32_CONST, 3456789012);
        assert_eq!(test_constants::UINT64_CONST, 12345678901234567890);
        assert_eq!(test_constants::FLOAT32_CONST, 1234.5);
        assert_eq!(test_constants::FLOAT64_CONST, -123e45);
    }

    #[test]
    fn test_set_root() {
        use test_capnp::test_big_struct;

        let mut message1 = message::Builder::new_default();
        let mut message2 = message::Builder::new_default();
        let mut struct1 = message1.init_root::<test_big_struct::Builder>();
        struct1.set_uint8_field(3);
        message2.set_root(struct1.as_reader()).unwrap();
        let struct2 = message2.get_root::<test_big_struct::Builder>().unwrap();

        assert_eq!(struct2.get_uint8_field(), 3u8);
    }

    #[test]
    fn upgrade_struct() {
        use test_capnp::{test_old_version, test_new_version};

        let mut message = message::Builder::new_default();
        {
            let mut old_version = message.init_root::<test_old_version::Builder>();
            old_version.set_old1(123);
        }
        {
            let mut new_version = message.get_root::<test_new_version::Builder>().unwrap();
            new_version.borrow().get_new2().unwrap();
            assert_eq!(new_version.get_new3().unwrap().get_int8_field(), -123);
        }
    }

    /*
    This test is for a problem in the schema compiler that was fixed in this commit:
    https://github.com/sandstorm-io/capnproto/commit/db7ca960a677de7d5088ceb2fef355f39bb84365
    TODO: Uncomment this test once the fix makes it into a released version of capnproto-c++.
          Not yet, as of release 0.5.3.
    #[test]
    fn upgrade_union() {
        use test_capnp::{test_old_union_version, test_new_union_version};
        // This tests for a specific case that was broken originally.
        let mut message = message::Builder::new_default();
        {
            let mut old_version = message.init_root::<test_old_union_version::Builder>();
            old_version.set_b(123);
        }

        {
            let new_version = message.get_root::<test_new_union_version::Builder>().unwrap();
            match new_version.which().unwrap() {
                test_new_union_version::B(n) => assert_eq!(n, 123),
                _ => panic!("expected B"),
            }
        }
    }
    */

    #[test]
    fn upgrade_list() {
        use test_capnp::{test_any_pointer, test_lists};

        {
            let mut builder = message::Builder::new_default();
            let mut root = builder.init_root::<test_any_pointer::Builder>();
            {
                let mut list = root.borrow()
                    .get_any_pointer_field().initn_as::<::capnp::primitive_list::Builder<u8>>(3);
                list.set(0, 12);
                list.set(1, 34);
                list.set(2, 56);
            }
            {
                let mut l = root.get_any_pointer_field()
                    .get_as::<::capnp::struct_list::Builder<test_lists::struct8::Owned>>().unwrap();
                assert_eq!(3, l.len());
                assert_eq!(12, l.borrow().get(0).get_f());
                assert_eq!(34, l.borrow().get(1).get_f());
                assert_eq!(56, l.borrow().get(2).get_f());
            }
        }

        {
            let mut builder = message::Builder::new_default();
            let mut root = builder.init_root::<test_any_pointer::Builder>();
            {
                let mut list = root.borrow()
                    .get_any_pointer_field().initn_as::<::capnp::text_list::Builder>(3);
                list.set(0, "foo");
                list.set(1, "bar");
                list.set(2, "baz");
            }
            {
                let mut l = root.get_any_pointer_field()
                    .get_as::<::capnp::struct_list::Builder<test_lists::struct_p::Owned>>().unwrap();
                assert_eq!(3, l.len());
                assert_eq!("foo", &*l.borrow().get(0).get_f().unwrap());
                assert_eq!("bar", &*l.borrow().get(1).get_f().unwrap());
//                assert_eq!("baz", l.borrow().get(2).get_f());
            }
        }

    }

    #[test]
    fn all_types() {
        use test_capnp::{test_all_types};

        let mut message = message::Builder::new_default();
        ::test_util::init_test_message(message.init_root());
        ::test_util::CheckTestMessage::check_test_message(message.get_root::<test_all_types::Builder>().unwrap());
        ::test_util::CheckTestMessage::check_test_message(
            message.get_root::<test_all_types::Builder>().unwrap().as_reader());

    }

    #[test]
    fn all_types_multi_segment() {
        use test_capnp::{test_all_types};

        let builder_options = message::HeapAllocator::new()
            .first_segment_words(1).allocation_strategy(::capnp::message::AllocationStrategy::FixedSize);
        let mut message = message::Builder::new(builder_options);
        ::test_util::init_test_message(message.init_root());
        ::test_util::CheckTestMessage::check_test_message(message.get_root::<test_all_types::Builder>().unwrap());
        ::test_util::CheckTestMessage::check_test_message(
            message.get_root::<test_all_types::Builder>().unwrap().as_reader());
    }

    #[test]
    fn setters() {
        use test_capnp::{test_all_types};

        {
            let mut message = message::Builder::new_default();

            ::test_util::init_test_message(message.init_root::<test_all_types::Builder>());

            let mut message2 = message::Builder::new_default();
            let mut all_types2 = message2.init_root::<test_all_types::Builder>();

            all_types2.set_struct_field(message.get_root::<test_all_types::Builder>().unwrap().as_reader()).unwrap();
            ::test_util::CheckTestMessage::check_test_message(all_types2.borrow().get_struct_field().unwrap());

            let reader = all_types2.as_reader().get_struct_field().unwrap();
            ::test_util::CheckTestMessage::check_test_message(reader);
        }

        {
            let builder_options = message::HeapAllocator::new()
                .first_segment_words(1)
                .allocation_strategy(::capnp::message::AllocationStrategy::FixedSize);
            let mut message = message::Builder::new(builder_options);

            ::test_util::init_test_message(message.init_root::<test_all_types::Builder>());

            let builder_options = message::HeapAllocator::new()
                .first_segment_words(1)
                .allocation_strategy(::capnp::message::AllocationStrategy::FixedSize);
            let mut message2 = message::Builder::new(builder_options);
            let mut all_types2 = message2.init_root::<test_all_types::Builder>();

            all_types2.set_struct_field(message.get_root::<test_all_types::Builder>().unwrap().as_reader()).unwrap();
            ::test_util::CheckTestMessage::check_test_message(all_types2.borrow().get_struct_field().unwrap());

            let reader = all_types2.as_reader().get_struct_field().unwrap();
            ::test_util::CheckTestMessage::check_test_message(reader);
        }
    }

    #[test]
    fn double_far_pointers() {
        let _words: &[::capnp::Word] = &[
            capnp_word!(2,0,0,0,1,0,0,0),
            capnp_word!(2,0,0,0,1,0,0,0),
            capnp_word!(6,0,0,0,1,0,0,0),
            capnp_word!(2,0,0,0,2,0,0,0),
            capnp_word!(0,0,0,0,1,0,0,0),
            capnp_word!(1,7,255,127,0,0,0,0)];
        // ...
    }

    #[test]
    fn text_builder_int_underflow() {
        use test_capnp::{test_any_pointer};

        let mut message = message::Builder::new_default();
        {
            let mut root = message.init_root::<test_any_pointer::Builder>();
            let _ : ::capnp::data::Builder = root.borrow().get_any_pointer_field().initn_as(0);

            // No NUL terminator!
            let result = root.get_any_pointer_field().get_as::<::capnp::text::Builder>();
            assert!(result.is_err());
        }
    }

    #[test]
    fn inline_composite_list_int_overflow() {
        let words: &[::capnp::Word] = &[
            capnp_word!(0,0,0,0,0,0,1,0),
            capnp_word!(1,0,0,0,0x17,0,0,0),
            capnp_word!(0,0,0,128,16,0,0,0),
            capnp_word!(0,0,0,0,0,0,0,0),
            capnp_word!(0,0,0,0,0,0,0,0)];
        let segment_array = &[words];

        let message =
            message::Reader::new(message::SegmentArray::new(segment_array), ReaderOptions::new());

        let root : ::test_capnp::test_any_pointer::Reader = message.get_root().unwrap();
        match root.total_size() {
            Err(e) =>
                assert_eq!("InlineComposite list's elements overrun its word count.", e.description),
            _ => panic!("did not get expected error")
        }

        {
            let result = root.get_any_pointer_field()
                .get_as::<::capnp::struct_list::Reader<::test_capnp::test_all_types::Owned>>();

            assert!(result.is_err());
        }

        let mut message_builder = message::Builder::new_default();
        let builder_root = message_builder.init_root::<::test_capnp::test_any_pointer::Builder>();
        match builder_root.get_any_pointer_field().set_as(root) {
            Err(e) =>
                assert_eq!("InlineComposite list's elements overrun its word count.", e.description),
            _ => panic!("did ont get expected error"),
        }
    }

    #[test]
    fn void_list_amplification() {
        use test_capnp::{test_any_pointer, test_all_types};

        let mut message = message::Builder::new_default();
        {
            let root = message.init_root::<test_any_pointer::Builder>();
            let _ : ::capnp::primitive_list::Builder<()> =
                root.get_any_pointer_field().initn_as((1 << 29) - 1);
        }
        let segments = message.get_segments_for_output();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].len(), 2);

        let reader = message::Reader::new(message::SegmentArray::new(&segments),
                                          ReaderOptions::new());
        let root = reader.get_root::<test_any_pointer::Reader>().unwrap();
        let result = root.get_any_pointer_field().get_as::<::capnp::struct_list::Reader<test_all_types::Owned>>();
        assert!(result.is_err());
    }

    #[test]
    fn empty_struct_list_amplification() {
        use test_capnp::{test_any_pointer, test_empty_struct, test_all_types};

        let mut message = message::Builder::new_default();
        {
            let root = message.init_root::<test_any_pointer::Builder>();
            let _ : ::capnp::struct_list::Builder<test_empty_struct::Owned> =
                root.get_any_pointer_field().initn_as((1 << 29) - 1);
        }
        let segments = message.get_segments_for_output();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].len(), 3);

        let reader =
            message::Reader::new(message::SegmentArray::new(&segments),
                                 ReaderOptions::new());
        let root = reader.get_root::<test_any_pointer::Reader>().unwrap();
        let result = root.get_any_pointer_field().get_as::<::capnp::struct_list::Reader<test_all_types::Owned>>();
        assert!(result.is_err());
    }

    #[test]
    fn null_struct_fields() {
        use test_capnp::{test_all_types};
        let mut message = message::Builder::new_default();
        {
	    let mut test = message.init_root::<test_all_types::Builder>();
	    test.set_text_field("Hello");
        }
        let reader = message.get_root::<test_all_types::Builder>().unwrap().as_reader();
        assert_eq!(reader.get_text_field().unwrap(), "Hello");
        assert_eq!(reader.has_struct_field(), false);
        let nested = reader.get_struct_field().unwrap();
        assert_eq!(nested.get_int8_field(), 0);
        assert_eq!(nested.get_u_int64_field(), 0);
        assert_eq!(nested.get_void_list().unwrap().len(), 0);
        assert_eq!(nested.get_float64_list().unwrap().len(), 0);
        assert_eq!(nested.get_struct_list().unwrap().len(), 0);
        assert_eq!(nested.get_text_field().unwrap(), "");
        assert_eq!(nested.get_data_field().unwrap(), &[]);
    }

    // At one point this failed to typecheck, giving the error:
    // "no method named `get_any_pointer_field` found for type `test_capnp::test_any_pointer::Pipeline`"
    #[allow(unused)]
    fn pipeline_any_pointer(foo : ::test_capnp::test_any_pointer::Pipeline) {
        let _ = foo.get_any_pointer_field();
    }

    #[test]
    fn threads() {
        use test_capnp::{test_all_types};
        {
            let mut message = message::Builder::new_default();
            let mut root = message.init_root::<test_all_types::Builder>();
            ::test_util::init_test_message(root.borrow());
            {
                let _root_reader = root.as_reader();
                // TODO
                //::std::thread::scoped(|| {
                //    let root_reader1 = root_reader;
                //});
            }
        }
    }
}
