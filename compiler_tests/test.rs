/*
 * Copyright (c) 2013 - 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

#![crate_type = "bin"]

extern crate capnp;

pub mod test_capnp;

mod tests {
    use capnp::message::{MessageBuilder, MallocMessageBuilder, BuilderOptions};

    #[test]
    fn test_prim_list () {

        use test_capnp::test_prim_list;

        // Make the first segment small to force allocation of a second segment.
        let mut message = MallocMessageBuilder::new(*BuilderOptions::new().first_segment_words(50));

        let mut test_prim_list = message.init_root::<test_prim_list::Builder>();

        let mut uint8_list = test_prim_list.init_uint8_list(100);

        for i in range(0, uint8_list.size()) {
            uint8_list.set(i, i as u8);
        }

        let mut uint64_list = test_prim_list.init_uint64_list(20);

        for i in range(0, uint64_list.size()) {
            uint64_list.set(i, i as u64);
        }

        assert_eq!(test_prim_list.has_bool_list(), false);
        let mut bool_list = test_prim_list.init_bool_list(65);
        assert_eq!(test_prim_list.has_bool_list(), true);

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

        assert_eq!(test_prim_list.has_void_list(), false);
        let mut void_list = test_prim_list.init_void_list(1025);
        assert_eq!(test_prim_list.has_void_list(), true);
        void_list.set(257, ());


        let test_prim_list_reader = test_prim_list.as_reader();
        let uint8_list = test_prim_list_reader.get_uint8_list();
        for i in range(0, uint8_list.size()) {
            assert_eq!(uint8_list.get(i), i as u8);
        }
        let uint64_list = test_prim_list_reader.get_uint64_list();
        for i in range(0, uint64_list.size()) {
            assert_eq!(uint64_list.get(i), i as u64);
        }

        assert_eq!(test_prim_list_reader.has_bool_list(), true);
        let bool_list = test_prim_list_reader.get_bool_list();
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

        assert_eq!(test_prim_list_reader.get_void_list().size(), 1025);
    }

    #[test]
    fn test_struct_list () {

        use test_capnp::test_struct_list;

        let mut message = MallocMessageBuilder::new_default();

        let mut test_struct_list = message.init_root::<test_struct_list::Builder>();

        test_struct_list.init_struct_list(4);
        let struct_list = test_struct_list.get_struct_list();
        struct_list.get(0).init_uint8_list(1).set(0, 5u8);

        {
            let reader = test_struct_list.as_reader();
            assert_eq!(reader.get_struct_list().get(0).get_uint8_list().get(0), 5u8);
        }
    }

    #[test]
    fn test_blob () {
        use test_capnp::test_blob;

        let mut message = MallocMessageBuilder::new_default();
        let mut test_blob = message.init_root::<test_blob::Builder>();

        assert_eq!(test_blob.has_text_field(), false);
        test_blob.set_text_field("abcdefghi");
        assert_eq!(test_blob.has_text_field(), true);

        assert_eq!(test_blob.has_data_field(), false);
        test_blob.set_data_field([0u8, 1u8, 2u8, 3u8, 4u8]);
        assert_eq!(test_blob.has_data_field(), true);

        let test_blob_reader = test_blob.as_reader();

        assert_eq!(test_blob_reader.has_text_field(), true);
        assert_eq!(test_blob_reader.has_data_field(), true);

        assert_eq!(test_blob_reader.get_text_field(), "abcdefghi");
        assert!(test_blob_reader.get_data_field() == [0u8, 1u8, 2u8, 3u8, 4u8]);

        let text_builder = test_blob.init_text_field(10);
        assert_eq!(test_blob.as_reader().get_text_field(),
                   "\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00");
        let mut writer = ::std::io::BufWriter::new(text_builder.as_mut_bytes());
        writer.write("aabbccddee".as_bytes()).unwrap();

        let data_builder = test_blob.init_data_field(7);
        assert!(test_blob.as_reader().get_data_field() ==
                [0u8,0u8,0u8,0u8,0u8,0u8,0u8]);
        for c in data_builder.iter_mut() {
            *c = 5;
        }
        data_builder[0] = 4u8;

        assert_eq!(test_blob.as_reader().get_text_field(), "aabbccddee");
        assert!(test_blob.as_reader().get_data_field() == [4u8,5u8,5u8,5u8,5u8,5u8,5u8]);

        let bytes = test_blob.get_text_field().as_mut_bytes();
        bytes[4] = 'z' as u8;
        bytes[5] = 'z' as u8;
        assert_eq!(test_blob.as_reader().get_text_field(), "aabbzzddee");

        test_blob.get_data_field()[2] = 10;
        assert!(test_blob.as_reader().get_data_field() == [4u8,5u8,10u8,5u8,5u8,5u8,5u8]);
    }


    #[test]
    fn test_big_struct() {
        use test_capnp::test_big_struct;

        // Make the first segment small to force allocation of a second segment.
        let mut message = MallocMessageBuilder::new(*BuilderOptions::new().first_segment_words(5));

        let mut big_struct = message.init_root::<test_big_struct::Builder>();

        big_struct.set_bool_field(false);
        big_struct.set_int8_field(-128);
        big_struct.set_int16_field(0);
        big_struct.set_int32_field(1009);

        assert_eq!(big_struct.has_struct_field(), false);
        let mut inner = big_struct.init_struct_field();
        assert_eq!(big_struct.has_struct_field(), true);
        inner.set_float64_field(0.1234567);

        inner.set_bool_field_b(true);

        big_struct.set_bool_field(true);


        let big_struct_reader = big_struct.as_reader();
        assert_eq!(big_struct_reader.has_struct_field(), true);
        assert_eq!(big_struct_reader.get_int8_field(), -128);
        assert_eq!(big_struct_reader.get_int32_field(), 1009);

        let inner_reader = big_struct_reader.get_struct_field();
        assert!(!inner_reader.get_bool_field_a());
        assert!(inner_reader.get_bool_field_b());
        assert_eq!(inner_reader.get_float64_field(), 0.1234567);
    }

    #[test]
    fn test_complex_list () {
        use test_capnp::{test_complex_list, an_enum};

        let mut message = MallocMessageBuilder::new_default();

        let mut test_complex_list = message.init_root::<test_complex_list::Builder>();

        let mut enum_list = test_complex_list.init_enum_list(100);

        for i in range::<u32>(0, 10) {
            enum_list.set(i, an_enum::Qux);
        }
        for i in range::<u32>(10, 20) {
            enum_list.set(i, an_enum::Bar);
        }

        let mut text_list = test_complex_list.init_text_list(2);
        text_list.set(0, "garply");
        text_list.set(1, "foo");

        let mut data_list = test_complex_list.init_data_list(2);
        data_list.set(0, [0u8, 1u8, 2u8]);
        data_list.set(1, [255u8, 254u8, 253u8]);

        let mut prim_list_list = test_complex_list.init_prim_list_list(2);
        let mut prim_list = prim_list_list.init(0, 3);
        prim_list.set(0, 5);
        prim_list.set(1, 6);
        prim_list.set(2, 7);
        assert_eq!(prim_list.size(), 3);
        let mut prim_list = prim_list_list.init(1, 1);
        prim_list.set(0,-1);

        let mut prim_list_list_list = test_complex_list.init_prim_list_list_list(2);
        let mut prim_list_list = prim_list_list_list.init(0, 2);
        let mut prim_list = prim_list_list.init(0, 2);
        prim_list.set(0, 0);
        prim_list.set(1, 1);
        let mut prim_list = prim_list_list.init(1, 1);
        prim_list.set(0, 255);
        let mut prim_list_list = prim_list_list_list.init(1, 1);
        let mut prim_list = prim_list_list.init(0, 3);
        prim_list.set(0, 10);
        prim_list.set(1, 9);
        prim_list.set(2, 8);

        let mut enum_list_list = test_complex_list.init_enum_list_list(2);
        let mut enum_list = enum_list_list.init(0, 1);
        enum_list.set(0, an_enum::Bar);
        let mut enum_list = enum_list_list.init(1, 2);
        enum_list.set(0, an_enum::Foo);
        enum_list.set(1, an_enum::Qux);

        let mut text_list_list = test_complex_list.init_text_list_list(1);
        text_list_list.init(0,1).set(0, "abc");

        let mut data_list_list = test_complex_list.init_data_list_list(1);
        data_list_list.init(0,1).set(0, [255, 254, 253]);

        let mut struct_list_list = test_complex_list.init_struct_list_list(1);
        struct_list_list.init(0,1).get(0).set_int8_field(-1);


        let complex_list_reader = test_complex_list.as_reader();
        let enum_list_reader = complex_list_reader.get_enum_list();
        for i in range::<u32>(0,10) {
            assert!(enum_list_reader.get(i) == Some(an_enum::Qux));
        }
        for i in range::<u32>(10,20) {
            assert!(enum_list_reader.get(i) == Some(an_enum::Bar));
        }

        let text_list = complex_list_reader.get_text_list();
        assert_eq!(text_list.size(), 2);
        assert_eq!(text_list.get(0), "garply");
        assert_eq!(text_list.get(1), "foo");

        let data_list = complex_list_reader.get_data_list();
        assert_eq!(data_list.size(), 2);
        assert!(data_list.get(0) == [0u8, 1u8, 2u8]);
        assert!(data_list.get(1) == [255u8, 254u8, 253u8]);

        let prim_list_list = complex_list_reader.get_prim_list_list();
        assert_eq!(prim_list_list.size(), 2);
        assert_eq!(prim_list_list.get(0).size(), 3);
        assert!(prim_list_list.get(0).get(0) == 5);
        assert!(prim_list_list.get(0).get(1) == 6);
        assert!(prim_list_list.get(0).get(2) == 7);
        assert!(prim_list_list.get(1).get(0) == -1);

        let prim_list_list_list = complex_list_reader.get_prim_list_list_list();
        assert!(prim_list_list_list.get(0).get(0).get(0) == 0);
        assert!(prim_list_list_list.get(0).get(0).get(1) == 1);
        assert!(prim_list_list_list.get(0).get(1).get(0) == 255);
        assert!(prim_list_list_list.get(1).get(0).get(0) == 10);
        assert!(prim_list_list_list.get(1).get(0).get(1) == 9);
        assert!(prim_list_list_list.get(1).get(0).get(2) == 8);

        let enum_list_list = complex_list_reader.get_enum_list_list();
        assert!(enum_list_list.get(0).get(0) == Some(an_enum::Bar));
        assert!(enum_list_list.get(1).get(0) == Some(an_enum::Foo));
        assert!(enum_list_list.get(1).get(1) == Some(an_enum::Qux));

        assert!(complex_list_reader.get_text_list_list().get(0).get(0) == "abc");
        assert!(complex_list_reader.get_data_list_list().get(0).get(0) == [255, 254, 253]);

        assert!(complex_list_reader.get_struct_list_list().get(0).get(0).get_int8_field() == -1);
    }

    #[test]
    fn test_defaults() {
        use test_capnp::test_defaults;

        let mut message = MallocMessageBuilder::new_default();
        let mut test_defaults = message.init_root::<test_defaults::Builder>();

        assert_eq!(test_defaults.get_void_field(), ());
        assert_eq!(test_defaults.get_bool_field(), true);
        assert_eq!(test_defaults.get_int8_field(), -123);
        assert_eq!(test_defaults.get_int16_field(), -12345);
        assert_eq!(test_defaults.get_int32_field(), -12345678);
        assert_eq!(test_defaults.get_int64_field(), -123456789012345);
        assert_eq!(test_defaults.get_uint8_field(), 234u8);
        assert_eq!(test_defaults.get_uint16_field(), 45678u16);
        assert_eq!(test_defaults.get_uint32_field(), 3456789012u32);
        assert_eq!(test_defaults.get_uint64_field(), 12345678901234567890u64);
        assert_eq!(test_defaults.get_float32_field(), 1234.5);
        assert_eq!(test_defaults.get_float64_field(), -123e45);

        test_defaults.set_bool_field(false);
        assert_eq!(test_defaults.get_bool_field(), false);
        test_defaults.set_int8_field(63);
        assert_eq!(test_defaults.get_int8_field(), 63);
    }

    #[test]
    fn test_any_pointer() {
        use test_capnp::{test_any_pointer, test_empty_struct, test_big_struct};

        let mut message = MallocMessageBuilder::new_default();
        let mut test_any_pointer = message.init_root::<test_any_pointer::Builder>();

        let mut any_pointer = test_any_pointer.init_any_pointer_field();
        any_pointer.set_as_text("xyzzy");

        {
            let reader = test_any_pointer.as_reader();
            assert_eq!(reader.get_any_pointer_field().get_as_text(), "xyzzy");
        }

        any_pointer.init_as_struct::<test_empty_struct::Builder>();
        any_pointer.get_as_struct::<test_empty_struct::Builder>();

        {
            let reader = test_any_pointer.as_reader();
            reader.get_any_pointer_field().get_as_struct::<test_empty_struct::Reader>();
        }

        {
            let mut message = MallocMessageBuilder::new_default();
            let mut test_big_struct = message.init_root::<test_big_struct::Builder>();
            test_big_struct.set_int32_field(-12345);
            any_pointer.set_as_struct(&test_big_struct.as_reader());
        }

        fn _test_lifetimes(body : test_big_struct::Reader) {
            let mut message = MallocMessageBuilder::new_default();
            message.set_root(&body);
        }

    }

    #[test]
    fn test_writable_struct_pointer() {
        use test_capnp::test_big_struct;

        let mut message = MallocMessageBuilder::new_default();
        let mut big_struct = message.init_root::<test_big_struct::Builder>();

        let mut struct_field = big_struct.init_struct_field();
        assert_eq!(struct_field.get_uint64_field(), 0);

        struct_field.set_uint64_field(-7);
        assert_eq!(struct_field.get_uint64_field(), -7);
        assert_eq!(big_struct.get_struct_field().get_uint64_field(), -7);
        let struct_field = big_struct.init_struct_field();
        assert_eq!(struct_field.get_uint64_field(), 0);
        assert_eq!(struct_field.get_uint32_field(), 0);

        // getting before init is the same as init
        let mut other_struct_field = big_struct.get_another_struct_field();
        assert_eq!(other_struct_field.get_uint64_field(), 0);
        other_struct_field.set_uint32_field(-31);

        let reader = other_struct_field.as_reader();
        big_struct.set_struct_field(reader);
        assert_eq!(big_struct.get_struct_field().get_uint32_field(), -31);
        assert_eq!(other_struct_field.get_uint32_field(), -31);
        other_struct_field.set_uint32_field(42);
        assert_eq!(big_struct.get_struct_field().get_uint32_field(), -31);
        assert_eq!(other_struct_field.get_uint32_field(), 42);
        assert_eq!(big_struct.get_another_struct_field().get_uint32_field(), 42);
    }

    #[test]
    fn test_union() {
        use test_capnp::test_union;

        let mut message = MallocMessageBuilder::new_default();
        let mut union_struct = message.init_root::<test_union::Builder>();

        union_struct.get_union0().set_u0f0s0(());
        match union_struct.get_union0().which() {
            Some(test_union::union0::U0f0s0(())) => {}
            _ => panic!()
        }
        union_struct.init_union0().set_u0f0s1(true);
        match union_struct.get_union0().which() {
            Some(test_union::union0::U0f0s1(true)) => {}
            _ => panic!()
        }
        union_struct.init_union0().set_u0f0s8(127);
        match union_struct.get_union0().which() {
            Some(test_union::union0::U0f0s8(127)) => {}
            _ => panic!()
        }

        assert_eq!(union_struct.get_union0().has_u0f0sp(), false);
        union_struct.init_union0().set_u0f0sp("abcdef");
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

        let mut message1 = MallocMessageBuilder::new_default();
        let mut message2 = MallocMessageBuilder::new_default();
        let mut struct1 = message1.init_root::<test_big_struct::Builder>();
        struct1.set_uint8_field(3);
        message2.set_root(&struct1.as_reader());
        let struct2 = message2.get_root::<test_big_struct::Builder>();

        assert_eq!(struct2.get_uint8_field(), 3u8);
    }

    #[test]
    fn upgrade_struct() {
        use test_capnp::{test_old_version, test_new_version};

        let mut message = MallocMessageBuilder::new_default();
        {
            let mut old_version = message.init_root::<test_old_version::Builder>();
            old_version.set_old1(123);
        }
        {
            let new_version = message.get_root::<test_new_version::Builder>();
            new_version.get_new2();
            assert_eq!(new_version.get_new3().get_int8_field(), -123);
        }
    }
}
