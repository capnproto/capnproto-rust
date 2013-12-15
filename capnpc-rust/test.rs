/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

#[feature(globs)];

#[link(name = "test", vers = "alpha", author = "dwrensha")];

#[crate_type = "bin"];

extern mod capnp;

pub mod test_capnp;

#[test]
fn test_prim_list () {
    use capnp::message::*;
    use test_capnp::*;

    // Make the first segment small to force allocation of a second segment.
    MessageBuilder::new(50,
                        SUGGESTED_ALLOCATION_STRATEGY,
                        |message| {

        let testPrimList = message.init_root::<TestPrimList::Builder>();

        let uint8_list = testPrimList.init_uint8_list(100);

        for i in range(0, uint8_list.size()) {
            uint8_list.set(i, i as u8);
        }

        let uint64List = testPrimList.init_uint64_list(20);

        for i in range(0, uint64List.size()) {
            uint64List.set(i, i as u64);
        }

        let boolList = testPrimList.init_bool_list(65);

        boolList.set(0, true);
        boolList.set(1, true);
        boolList.set(2, true);
        boolList.set(3, true);
        boolList.set(5, true);
        boolList.set(8, true);
        boolList.set(13, true);
        boolList.set(64, true);

        assert!(boolList[0]);
        assert!(!boolList[4]);
        assert!(!boolList[63]);
        assert!(boolList[64]);

        let voidList = testPrimList.init_void_list(1025);
        voidList.set(257, ());

        testPrimList.as_reader(|testPrimListReader| {
            let uint8List = testPrimListReader.get_uint8_list();
            for i in range(0, uint8List.size()) {
                assert!(uint8List[i] == i as u8);
            }
            let uint64List = testPrimListReader.get_uint64_list();
            for i in range(0, uint64List.size()) {
                 assert!(uint64List[i] == i as u64);
            }

            let boolList = testPrimListReader.get_bool_list();
            assert!(boolList[0]);
            assert!(boolList[1]);
            assert!(boolList[2]);
            assert!(boolList[3]);
            assert!(!boolList[4]);
            assert!(boolList[5]);
            assert!(!boolList[6]);
            assert!(!boolList[7]);
            assert!(boolList[8]);
            assert!(!boolList[9]);
            assert!(!boolList[10]);
            assert!(!boolList[11]);
            assert!(!boolList[12]);
            assert!(boolList[13]);
            assert!(!boolList[63]);
            assert!(boolList[64]);

            assert!(testPrimListReader.get_void_list().size() == 1025);
        });
    });
}

#[test]
fn test_blob () {
    use capnp::message::*;
    use test_capnp::*;

    MessageBuilder::new_default(
        |message| {

            let test_blob = message.init_root::<TestBlob::Builder>();

            test_blob.set_text_field("abcdefghi");
            test_blob.set_data_field([0u8, 1u8, 2u8, 3u8, 4u8]);

            test_blob.as_reader(|test_blob_reader| {

                    assert!(test_blob_reader.get_text_field() == "abcdefghi");
                    assert!(test_blob_reader.get_data_field() == [0u8, 1u8, 2u8, 3u8, 4u8]);
                });
        });
}


#[test]
fn test_big_struct() {

    use capnp::message::*;
    use test_capnp::*;

    // Make the first segment small to force allocation of a second segment.
    MessageBuilder::new(5,
                        SUGGESTED_ALLOCATION_STRATEGY,
                        |message| {

        let bigStruct = message.init_root::<TestBigStruct::Builder>();

        bigStruct.set_bool_field(false);
        bigStruct.set_int8_field(-128);
        bigStruct.set_int16_field(0);
        bigStruct.set_int32_field(1009);

        let inner = bigStruct.init_struct_field();
        inner.set_float64_field(0.1234567);

        inner.set_bool_field_b(true);

        bigStruct.set_bool_field(true);

        bigStruct.as_reader(|bigStructReader| {
            assert!(bigStructReader.get_int8_field() == -128);
            assert!(bigStructReader.get_int32_field() == 1009);

            let innerReader = bigStructReader.get_struct_field();
            assert!(!innerReader.get_bool_field_a());
            assert!(innerReader.get_bool_field_b());
            assert!(innerReader.get_float64_field() == 0.1234567);
        });
    });
}

#[test]
fn test_complex_list () {
    use capnp::message::*;
    use test_capnp::*;

    MessageBuilder::new_default(|message| {

        let test_complex_list = message.init_root::<TestComplexList::Builder>();

        let enumList = test_complex_list.init_enum_list(100);

        for i in range::<uint>(0, 10) {
            enumList.set(i, AnEnum::Qux);
        }
        for i in range::<uint>(10, 20) {
            enumList.set(i, AnEnum::Bar);
        }

        let text_list = test_complex_list.init_text_list(2);
        text_list.set(0, "garply");
        text_list.set(1, "foo");

        let data_list = test_complex_list.init_data_list(2);
        data_list.set(0, [0u8, 1u8, 2u8]);
        data_list.set(1, [255u8, 254u8, 253u8]);

        let prim_list_list = test_complex_list.init_prim_list_list(2);
        let prim_list = prim_list_list.init(0, 3);
        prim_list.set(0, 5);
        prim_list.set(1, 6);
        prim_list.set(2, 7);
        assert!(prim_list.size() == 3);
        let prim_list = prim_list_list.init(1, 1);
        prim_list.set(0,-1);

        // get_writable_list_pointer is unimplemented
        //prim_list_list[0].set(0, 1);

        let prim_list_list_list = test_complex_list.init_prim_list_list_list(2);
        let prim_list_list = prim_list_list_list.init(0, 2);
        let prim_list = prim_list_list.init(0, 2);
        prim_list.set(0, 0);
        prim_list.set(1, 1);
        let prim_list = prim_list_list.init(1, 1);
        prim_list.set(0, 255);
        let prim_list_list = prim_list_list_list.init(1, 1);
        let prim_list = prim_list_list.init(0, 3);
        prim_list.set(0, 10);
        prim_list.set(1, 9);
        prim_list.set(2, 8);

        let enum_list_list = test_complex_list.init_enum_list_list(2);
        let enum_list = enum_list_list.init(0, 1);
        enum_list.set(0, AnEnum::Bar);
        let enum_list = enum_list_list.init(1, 2);
        enum_list.set(0, AnEnum::Foo);
        enum_list.set(1, AnEnum::Qux);

        let text_list_list = test_complex_list.init_text_list_list(1);
        text_list_list.init(0,1).set(0, "abc");

        let data_list_list = test_complex_list.init_data_list_list(1);
        data_list_list.init(0,1).set(0, [255, 254, 253]);

        let struct_list_list = test_complex_list.init_struct_list_list(1);
        struct_list_list.init(0,1)[0].set_int8_field(-1);

        test_complex_list.as_reader(|complex_list_reader| {
            let enumListReader = complex_list_reader.get_enum_list();
            for i in range::<uint>(0,10) {
                assert!(enumListReader[i] == Some(AnEnum::Qux));
            }
            for i in range::<uint>(10,20) {
                assert!(enumListReader[i] == Some(AnEnum::Bar));
            }

            let text_list = complex_list_reader.get_text_list();
            assert!(text_list.size() == 2);
            assert!(text_list[0] == "garply");
            assert!(text_list[1] == "foo");

            let data_list = complex_list_reader.get_data_list();
            assert!(data_list.size() == 2);
            assert!(data_list[0] == [0u8, 1u8, 2u8]);
            assert!(data_list[1] == [255u8, 254u8, 253u8]);

            let prim_list_list = complex_list_reader.get_prim_list_list();
            assert!(prim_list_list.size() == 2);
            assert!(prim_list_list[0].size() == 3);
            assert!(prim_list_list[0][0] == 5);
            assert!(prim_list_list[0][1] == 6);
            assert!(prim_list_list[0][2] == 7);
            assert!(prim_list_list[1][0] == -1);

            let prim_list_list_list = complex_list_reader.get_prim_list_list_list();
            assert!(prim_list_list_list[0][0][0] == 0);
            assert!(prim_list_list_list[0][0][1] == 1);
            assert!(prim_list_list_list[0][1][0] == 255);
            assert!(prim_list_list_list[1][0][0] == 10);
            assert!(prim_list_list_list[1][0][1] == 9);
            assert!(prim_list_list_list[1][0][2] == 8);

            let enum_list_list = complex_list_reader.get_enum_list_list();
            assert!(enum_list_list[0][0] == Some(AnEnum::Bar));
            assert!(enum_list_list[1][0] == Some(AnEnum::Foo));
            assert!(enum_list_list[1][1] == Some(AnEnum::Qux));

            assert!(complex_list_reader.get_text_list_list()[0][0] == "abc");
            assert!(complex_list_reader.get_data_list_list()[0][0] == [255, 254, 253]);

            assert!(complex_list_reader.get_struct_list_list()[0][0].get_int8_field() == -1);
        });
    });
}

#[test]
fn test_any_pointer() {
    use capnp::message::MessageBuilder;
    use test_capnp::TestAnyPointer;

    MessageBuilder::new_default(
        |message| {

            let test_any_pointer = message.init_root::<TestAnyPointer::Builder>();

            let any_pointer = test_any_pointer.init_any_pointer_field();
            any_pointer.set_as_text("xyzzy");

            test_any_pointer.as_reader(|reader| {
                    assert!(reader.get_any_pointer_field().get_as_text() == "xyzzy");
                });
        });
}

#[test]
fn test_writable_struct_pointer() {
    use capnp::message::MessageBuilder;
    use test_capnp::TestBigStruct;

    MessageBuilder::new_default(
        |message| {
            let big_struct = message.init_root::<TestBigStruct::Builder>();

            let struct_field = big_struct.init_struct_field();
            assert!(struct_field.get_uint64_field() == 0);
            struct_field.set_uint64_field(-7);
            assert!(struct_field.get_uint64_field() == -7);
            assert!(big_struct.get_struct_field().get_uint64_field() == -7);
            let struct_field = big_struct.init_struct_field();
            assert!(struct_field.get_uint64_field() == 0);

            // getting before init is the same as init
            let other_struct_field = big_struct.get_another_struct_field();
            assert!(other_struct_field.get_uint64_field() == 0);
            other_struct_field.set_uint32_field(-31);

            // unimplemented
            // other_struct_field.as_reader(|reader| { big_struct.set_struct_field(reader) });

        });

}
