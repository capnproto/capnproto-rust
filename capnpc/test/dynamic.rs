use crate::test_capnp::{test_all_types, test_defaults};
use crate::test_util::{self};
use capnp::message::{self};
use capnp::{dynamic_list, dynamic_struct, dynamic_value};

#[test]
fn test_dynamic_reader() {
    let mut builder = message::Builder::new_default();
    let root: test_all_types::Builder<'_> = builder.init_root();
    let mut root: dynamic_value::Builder<'_> = root.into();

    test_util::dynamic_init_test_message(root.reborrow().downcast());

    let struct_reader = root.into_reader().downcast::<dynamic_struct::Reader<'_>>();
    assert!(struct_reader.which().unwrap().is_none());
    test_util::dynamic_check_test_message(struct_reader);
}

#[test]
fn test_dynamic_builder() {
    let mut builder = message::Builder::new_default();
    let root: test_all_types::Builder<'_> = builder.init_root();
    let mut root: dynamic_value::Builder<'_> = root.into();
    test_util::dynamic_init_test_message(root.reborrow().downcast());

    let struct_builder = root.downcast::<dynamic_struct::Builder<'_>>();
    assert!(struct_builder.which().unwrap().is_none());
    test_util::dynamic_check_test_message_builder(struct_builder);
}

#[test]
fn test_defaults() {
    use crate::test_capnp::test_defaults;

    let message = message::Builder::new_default();
    let test_defaults = message
        .get_root_as_reader::<test_defaults::Reader<'_>>()
        .expect("get_root_as_reader()");
    let root: dynamic_value::Reader<'_> = test_defaults.into();
    test_util::dynamic_check_test_message(root.downcast());
}

#[test]
fn test_defaults_builder() {
    use crate::test_capnp::test_defaults;

    let mut message = message::Builder::new_default();
    let test_defaults = message.get_root::<test_defaults::Builder<'_>>().unwrap();
    let root: dynamic_value::Builder<'_> = test_defaults.into();
    test_util::dynamic_check_test_message_builder(root.downcast());
}

#[test]
fn test_unions() {
    use crate::test_capnp::test_union;
    use capnp::{dynamic_struct, dynamic_value};
    let mut message = message::Builder::new_default();
    let mut root: test_union::Builder<'_> = message.init_root();
    root.reborrow().get_union0().set_u0f1s32(1234567);
    root.reborrow().get_union1().set_u1f1sp("foo");
    root.reborrow().get_union2().set_u2f0s1(true);
    root.reborrow()
        .get_union3()
        .set_u3f0s64(1234567890123456789);

    let dynamic: dynamic_value::Reader<'_> = root.reborrow().into_reader().into();
    let dynamic: dynamic_struct::Reader<'_> = dynamic.downcast();
    {
        let u: dynamic_struct::Reader<'_> = dynamic.get_named("union0").unwrap().downcast();
        assert!(u.has_named("u0f1s32").unwrap());
        assert!(!u.has_named("u0f1s16").unwrap());
        assert_eq!(
            "u0f1s32",
            u.which().unwrap().unwrap().get_proto().get_name().unwrap()
        );
        assert_eq!(1234567i32, u.get_named("u0f1s32").unwrap().downcast());
    }
    {
        let u: dynamic_struct::Reader<'_> = dynamic.get_named("union1").unwrap().downcast();
        let w = u.which().unwrap().unwrap();
        assert_eq!("u1f1sp", w.get_proto().get_name().unwrap());
        assert_eq!(
            "foo",
            u.get(w).unwrap().downcast::<capnp::text::Reader<'_>>()
        );
    }
    {
        let u: dynamic_struct::Reader<'_> = dynamic.get_named("union2").unwrap().downcast();
        let w = u.which().unwrap().unwrap();
        assert_eq!("u2f0s1", w.get_proto().get_name().unwrap());
        assert_eq!(true, u.get(w).unwrap().downcast());
    }
    {
        let u: dynamic_struct::Reader<'_> = dynamic.get_named("union3").unwrap().downcast();
        let w = u.which().unwrap().unwrap();
        assert_eq!("u3f0s64", w.get_proto().get_name().unwrap());
        assert_eq!(1234567890123456789i64, u.get(w).unwrap().downcast());
    }

    // Again, as a builder.
    let dynamic: dynamic_value::Builder<'_> = root.into();
    let mut dynamic: dynamic_struct::Builder<'_> = dynamic.downcast();
    {
        let mut u: dynamic_struct::Builder<'_> =
            dynamic.reborrow().get_named("union0").unwrap().downcast();
        assert!(u.has_named("u0f1s32").unwrap());
        assert!(!u.has_named("u0f1s16").unwrap());
        assert_eq!(
            "u0f1s32",
            u.reborrow()
                .which()
                .unwrap()
                .unwrap()
                .get_proto()
                .get_name()
                .unwrap()
        );
        assert_eq!(1234567i32, u.get_named("u0f1s32").unwrap().downcast());
    }
    {
        let mut u: dynamic_struct::Builder<'_> =
            dynamic.reborrow().get_named("union1").unwrap().downcast();
        let w = u.reborrow().which().unwrap().unwrap();
        assert_eq!("u1f1sp", w.get_proto().get_name().unwrap());
        assert_eq!(
            u.get(w)
                .unwrap()
                .into_reader()
                .downcast::<capnp::text::Reader<'_>>(),
            "foo"
        );
    }
    {
        let mut u: dynamic_struct::Builder<'_> =
            dynamic.reborrow().get_named("union2").unwrap().downcast();
        let w = u.reborrow().which().unwrap().unwrap();
        assert_eq!("u2f0s1", w.get_proto().get_name().unwrap());
        assert_eq!(true, u.get(w).unwrap().downcast());
    }
    {
        let mut u: dynamic_struct::Builder<'_> =
            dynamic.reborrow().get_named("union3").unwrap().downcast();
        let w = u.reborrow().which().unwrap().unwrap();
        assert_eq!("u3f0s64", w.get_proto().get_name().unwrap());
        assert_eq!(1234567890123456789i64, u.get(w).unwrap().downcast());
    }
}

#[test]
fn test_generics() {
    use crate::test_capnp::{test_all_types, test_generics};
    use capnp::text;
    let mut message = message::Builder::new_default();
    let root: test_generics::Builder<'_, test_all_types::Owned, text::Owned> = message.init_root();

    let root: dynamic_value::Builder<'_> = root.into();
    let mut root: dynamic_struct::Builder<'_> = root.downcast();

    #[allow(clippy::disallowed_names)]
    let foo = root.reborrow().get_named("foo").unwrap();
    test_util::dynamic_init_test_message(foo.downcast());

    root.reborrow().set_named("bar", "abcde".into()).unwrap();

    test_util::dynamic_check_test_message_builder(
        root.reborrow().get_named("foo").unwrap().downcast(),
    );
    let root = root.into_reader();
    test_util::dynamic_check_test_message(root.get_named("foo").unwrap().downcast());

    assert_eq!(
        "abcde",
        root.get_named("bar")
            .unwrap()
            .downcast::<capnp::text::Reader<'_>>()
    );
}

#[test]
fn test_generic_annotation() -> ::capnp::Result<()> {
    use crate::test_capnp::{test_generics, test_use_generics};
    let mut message = message::Builder::new_default();
    let root: test_use_generics::Builder<'_> = message.init_root();
    let root: dynamic_value::Builder<'_> = root.into();
    let root: dynamic_struct::Builder<'_> = root.downcast();
    let annotations = root.get_schema().get_annotations()?;
    assert_eq!(1, annotations.len());
    let ann = annotations.get(0);
    assert_eq!(ann.get_id(), test_generics::ann::ID);
    assert_eq!(
        "foo",
        ann.get_value()?.downcast::<capnp::text::Reader<'_>>()
    );
    Ok(())
}

#[test]
fn test_complex_list() {
    use crate::test_capnp::test_complex_list;

    let mut message = message::Builder::new_default();
    let root = message.init_root::<test_complex_list::Builder<'_>>();
    let root: dynamic_value::Builder<'_> = root.into();
    let mut root: dynamic_struct::Builder<'_> = root.downcast();

    {
        let mut prim_list_list: dynamic_list::Builder<'_> = root
            .reborrow()
            .initn_named("primListList", 2)
            .unwrap()
            .downcast();
        let mut prim_list: dynamic_list::Builder<'_> =
            prim_list_list.reborrow().init(0, 3).unwrap().downcast();
        prim_list.set(0, 5i32.into()).unwrap();
        prim_list.set(1, 6i32.into()).unwrap();
        prim_list.set(2, 7i32.into()).unwrap();
        assert_eq!(prim_list.len(), 3);

        let mut prim_list: dynamic_list::Builder<'_> =
            prim_list_list.reborrow().init(1, 1).unwrap().downcast();
        prim_list.set(0, (-1i32).into()).unwrap();
    }

    let complex_list_reader = root.into_reader();
    let prim_list_list: dynamic_list::Reader<'_> = complex_list_reader
        .get_named("primListList")
        .unwrap()
        .downcast();
    assert_eq!(prim_list_list.len(), 2);
    let prim_list: dynamic_list::Reader<'_> = prim_list_list.get(0).unwrap().downcast();
    assert_eq!(prim_list.len(), 3);
    assert_eq!(5i32, prim_list.get(0).unwrap().downcast());
    assert_eq!(6i32, prim_list.get(1).unwrap().downcast());
    assert_eq!(7i32, prim_list.get(2).unwrap().downcast());
}

#[test]
fn test_stringify() {
    use crate::test_capnp::{test_all_types, TestEnum};
    let mut message = message::Builder::new_default();
    let mut root: test_all_types::Builder<'_> = message.init_root();
    root.set_int8_field(3);
    root.set_enum_field(TestEnum::Bar);
    root.set_text_field("hello world");
    root.set_data_field(&[1, 2, 3, 4, 5, 127, 255]);
    let mut bool_list = root.reborrow().init_bool_list(2);
    bool_list.set(0, false);
    bool_list.set(1, true);
    let mut inner = root.reborrow().init_struct_field();
    inner.set_u_int32_field(123456);
    let stringified = format!("{:?}", root.into_reader());
    assert_eq!(stringified, "(voidField = (), boolField = false, int8Field = 3, int16Field = 0, int32Field = 0, int64Field = 0, uInt8Field = 0, uInt16Field = 0, uInt32Field = 0, uInt64Field = 0, float32Field = 0, float64Field = 0, textField = \"hello world\", dataField = 0x\"01020304057fff\", structField = (voidField = (), boolField = false, int8Field = 0, int16Field = 0, int32Field = 0, int64Field = 0, uInt8Field = 0, uInt16Field = 0, uInt32Field = 123456, uInt64Field = 0, float32Field = 0, float64Field = 0, enumField = foo), enumField = bar, boolList = [false, true])");
}

#[test]
fn test_stringify_union_list() {
    use crate::test_capnp::test_union;
    use capnp::struct_list;
    let mut message = message::Builder::new_default();
    let mut root: struct_list::Builder<'_, test_union::Owned> = message.initn_root(2);
    {
        let mut union0 = root.reborrow().get(0).get_union0();
        union0.set_u0f0s8(10);
    }
    {
        let mut union0 = root.reborrow().get(1).get_union0();
        union0.set_u0f0s32(111111);
    }

    let stringified = format!("{:#?}", root.into_reader());
    assert_eq!(
        stringified,
        r#"[
  (
    union0 = (
      u0f0s8 = 10
    ),
    union1 = (
      u1f0s0 = ()
    ),
    union2 = (
      u2f0s1 = false
    ),
    union3 = (
      u3f0s1 = false
    ),
    bit0 = false,
    bit2 = false,
    bit3 = false,
    bit4 = false,
    bit5 = false,
    bit6 = false,
    bit7 = false,
    byte0 = 0
  ),
  (
    union0 = (
      u0f0s32 = 111111
    ),
    union1 = (
      u1f0s0 = ()
    ),
    union2 = (
      u2f0s1 = false
    ),
    union3 = (
      u3f0s1 = false
    ),
    bit0 = false,
    bit2 = false,
    bit3 = false,
    bit4 = false,
    bit5 = false,
    bit6 = false,
    bit7 = false,
    byte0 = 0
  )
]"#
    );
}

#[test]
fn test_stringify_prim_list() {
    use capnp::primitive_list;
    let mut message = message::Builder::new_default();
    let mut root: primitive_list::Builder<'_, u16> = message.initn_root(3);
    root.set(0, 5);
    root.set(1, 6);
    root.set(2, 7);

    let stringified = format!("{:?}", root.into_reader());
    assert_eq!(stringified, "[5, 6, 7]");
}

#[test]
fn test_stringify_enum_list() {
    use crate::test_capnp::TestEnum;
    use capnp::enum_list;
    let mut message = message::Builder::new_default();
    let mut root: enum_list::Builder<'_, TestEnum> = message.initn_root(2);
    root.set(0, TestEnum::Bar);
    root.set(1, TestEnum::Garply);

    let stringified = format!("{:?}", root.into_reader());
    assert_eq!(stringified, "[bar, garply]");
}

#[test]
fn test_stringify_text_list() {
    use capnp::text_list;
    let mut message = message::Builder::new_default();
    message.set_root(&["abcd", "efgh", "ijkl", "mnop"]).unwrap();

    let stringified = format!(
        "{:?}",
        message
            .get_root_as_reader::<text_list::Reader<'_>>()
            .unwrap()
    );
    assert_eq!(stringified, "[\"abcd\", \"efgh\", \"ijkl\", \"mnop\"]");
}

#[test]
fn test_stringify_data_list() {
    let mut message = message::Builder::new_default();
    let mut root: capnp::data_list::Builder<'_> = message.initn_root(2);
    root.set(0, &[11, 12]);
    root.set(1, &[22, 23]);

    let stringified = format!("{:?}", root.into_reader());
    assert_eq!(stringified, "[0x\"0b0c\", 0x\"1617\"]");
}

#[test]
fn test_stringify_list_list() {
    use capnp::{list_list, primitive_list};
    let mut message = message::Builder::new_default();
    let mut root: list_list::Builder<'_, primitive_list::Owned<i32>> = message.initn_root(2);
    {
        let mut l0 = root.reborrow().init(0, 3);
        l0.set(0, 1111);
        l0.set(1, 2222);
        l0.set(2, 3333);
    }

    {
        let mut l1 = root.reborrow().init(1, 1);
        l1.set(0, 123456);
    }

    let stringified = format!("{:?}", root.into_reader());
    assert_eq!(stringified, "[[1111, 2222, 3333], [123456]]");
}

#[test]
fn test_get_named_missing() {
    let mut builder = message::Builder::new_default();
    let root: test_all_types::Builder<'_> = builder.init_root();
    let root: dynamic_value::Builder<'_> = root.into();
    let mut root: dynamic_struct::Builder<'_> = root.downcast();
    test_util::dynamic_init_test_message(root.reborrow());
    let root = root.into_reader();
    // try a bunch of fields that don't exist
    assert!(root.get_named("AAAAAAA").is_err());
    assert!(root.has_named("AAAAAAA").is_err());
    assert!(root.get_named("abcdef").is_err());
    assert!(root.has_named("abcdef").is_err());
    assert!(root.get_named("zzzzzzz").is_err());
    assert!(root.has_named("zzzzzzz").is_err());
}

#[test]
fn test_downcasts() {
    let mut builder = message::Builder::new_default();
    let root: test_all_types::Builder<'_> = builder.init_root();
    let mut root: dynamic_value::Builder<'_> = root.into();

    test_util::dynamic_init_test_message(root.reborrow().downcast());

    {
        let root_typed = root.reborrow().downcast_struct::<test_all_types::Owned>();
        assert_eq!(root_typed.get_int16_field(), -12345);

        let root_typed_reader = root
            .reborrow()
            .into_reader()
            .downcast_struct::<test_all_types::Owned>();
        assert_eq!(root_typed_reader.get_int16_field(), -12345);
    }
    let mut root_struct: dynamic_struct::Builder<'_> = root.reborrow().downcast();
    {
        let int8_list: capnp::primitive_list::Builder<'_, i8> = root_struct
            .reborrow()
            .get_named("int8List")
            .unwrap()
            .downcast();
        assert_eq!(int8_list.len(), 2);
    }

    {
        let struct_list: capnp::struct_list::Builder<'_, test_all_types::Owned> = root_struct
            .reborrow()
            .get_named("structList")
            .unwrap()
            .downcast();
        assert_eq!(struct_list.len(), 3);
    }

    {
        let enum_list: capnp::enum_list::Builder<'_, crate::test_capnp::TestEnum> = root_struct
            .reborrow()
            .get_named("enumList")
            .unwrap()
            .downcast();
        assert_eq!(enum_list.len(), 2);
    }

    {
        let text_list: capnp::text_list::Builder<'_> = root_struct
            .reborrow()
            .get_named("textList")
            .unwrap()
            .downcast();
        assert_eq!(text_list.len(), 3);
        assert_eq!(text_list.get(1).unwrap().to_str().unwrap(), "xyzzy");
    }

    {
        let data_list: capnp::data_list::Builder<'_> = root_struct
            .reborrow()
            .get_named("dataList")
            .unwrap()
            .downcast();
        assert_eq!(data_list.len(), 3);
    }

    let root_struct: dynamic_struct::Reader<'_> = root_struct.into_reader();
    {
        let int8_list: capnp::primitive_list::Reader<'_, i8> =
            root_struct.get_named("int8List").unwrap().downcast();
        assert_eq!(int8_list.len(), 2);
    }

    {
        let struct_list: capnp::struct_list::Reader<'_, test_all_types::Owned> =
            root_struct.get_named("structList").unwrap().downcast();
        assert_eq!(struct_list.len(), 3);
    }

    {
        let enum_list: capnp::enum_list::Reader<'_, crate::test_capnp::TestEnum> =
            root_struct.get_named("enumList").unwrap().downcast();
        assert_eq!(enum_list.len(), 2);
    }

    {
        let text_list: capnp::text_list::Reader<'_> =
            root_struct.get_named("textList").unwrap().downcast();
        assert_eq!(text_list.len(), 3);
        assert_eq!(text_list.get(1).unwrap().to_str().unwrap(), "xyzzy");
    }

    {
        let data_list: capnp::data_list::Reader<'_> =
            root_struct.get_named("dataList").unwrap().downcast();
        assert_eq!(data_list.len(), 3);
    }
}

#[test]
fn introspect_loose_equals() {
    use capnp::introspect::Introspect;

    assert!(test_all_types::Owned::introspect().loose_equals(test_all_types::Owned::introspect()));

    assert!(!test_all_types::Owned::introspect().loose_equals(test_defaults::Owned::introspect()))
}
