use crate::{
    test_capnp::{
        json_data, json_rename, recognized_enum, simple_named_union, simple_nested_struct,
        simple_unnamed_union, test_all_types, test_struct_list, unrecognized_enum, SupersetEnum,
    },
    test_util,
};
use assert_json_diff::assert_json_eq;
use capnp::{dynamic_value, json::serialize::OnEnumerantNotInSchema, message, Result};
#[allow(unused)]
use pretty_assertions::{assert_eq, assert_ne};
use serde_json::json;

macro_rules! check_ser {
    ($builder:expr, $opts:expr, $expected:expr) => {
        match (
            capnp::json::serialize(($builder).reborrow_as_reader().into(), $opts).unwrap(),
            $expected,
        ) {
            (actual, expected) => {
                if actual != expected {
                    eprintln!("actual: {}", serde_json::to_string_pretty(&actual).unwrap());
                    assert_json_eq!(expected, actual);
                }
            }
        }
    };
}

macro_rules! check_de {
    ($ty:ident, $json:expr, $expected:expr) => {{
        let json = $json;
        let expected = $expected;
        let expected_norm = expected.replace(" ", "").replace("\n", "");

        let mut message = message::Builder::new_default();
        let mut root = message.init_root::<$ty::Builder<'_>>();
        capnp::json::deserialize_into(root.reborrow().into(), &json).unwrap();

        let actual = format!("{:?}", root);
        let actual_norm = actual.replace(" ", "").replace("\n", "");

        if expected_norm != actual_norm {
            assert_eq!(expected, actual);
        }
    }};
}

#[test]
fn deserialize_struct_list() {
    check_de!(
        test_struct_list,
        json!({
            "structList": [
                {
                    "uint8List": [1, 2, 3],
                }
            ]
        }),
        "(structList = [(uint8List = [1, 2, 3])])"
    );
}

#[test]
fn deserialize_unnamed_union_with_variant() {
    check_de!(
        simple_unnamed_union,
        json!({
            "variant": 42
        }),
        "(variant = 42)"
    )
}

#[test]
fn deserialize_named_union_with_variant() {
    check_de!(
        simple_named_union,
        json!({
            "value": {
                "variant": 42,
            },
        }),
        "(value = (variant = 42))"
    )
}

#[test]
fn deserialize_unnamed_union_no_fields() {
    check_de!(simple_unnamed_union, json!({}), "(unset = ())")
}

#[test]
fn deserialize_named_union_no_fields() {
    check_de!(
        simple_named_union,
        json!({
            "value": {},
        }),
        "(value = (unset = ())"
    )
}

#[test]
fn deserialize_unnamed_union_multiple_variants() {
    // We should pick the lowest matching variant in this case
    check_de!(
        simple_unnamed_union,
        json!({
            "variant": 42,
            "otherVariant": 43,
        }),
        "(variant = 42)"
    );
}

#[test]
fn deserialize_named_union_multiple_variants() {
    // We should pick the lowest matching variant in this case
    check_de!(
        simple_named_union,
        json!({
            "value": {
                "variant": 42,
                "otherVariant": 43,
            }
        }),
        "(value = (variant = 42))"
    );
}

#[test]
fn serialize_unnamed_union() -> Result<()> {
    let mut m = message::Builder::new_default();
    let mut subject = m.init_root::<simple_unnamed_union::Builder<'_>>();

    subject.reborrow().set_variant(42);

    check_ser!(
        subject,
        Default::default(),
        json!({
            "variant": 42
        })
    );
    Ok(())
}

#[test]
fn round_trip_all_types() -> Result<()> {
    let json = {
        let mut m = message::Builder::new_default();
        let mut subject = m.init_root::<test_all_types::Builder<'_>>();

        test_util::dynamic_init_test_message(
            dynamic_value::Builder::from(subject.reborrow()).downcast(),
        );
        eprintln!("{:#?}", subject.reborrow().into_reader());

        capnp::json::serialize(subject.reborrow_as_reader().into(), Default::default())?
    };
    eprintln!("{}", serde_json::to_string_pretty(&json).unwrap());

    let mut m = message::Builder::new_default();
    let mut subject: dynamic_value::Builder<'_> =
        m.init_root::<test_all_types::Builder<'_>>().into();

    capnp::json::deserialize_into(subject.reborrow(), &json)?;

    eprintln!("{:#?}", subject.reborrow().into_reader());
    test_util::dynamic_check_test_message(subject.into_reader().downcast());

    Ok(())
}

#[test]
fn serialize_simple_nested_struct() -> Result<()> {
    let mut m = message::Builder::new_default();

    let mut subject = m.init_root::<simple_nested_struct::Builder<'_>>();
    check_ser!(subject, Default::default(), json!({ "field": null }));

    subject.reborrow().init_field();
    check_ser!(
        subject,
        Default::default(),
        json!({ "field": { "nested": false } })
    );

    Ok(())
}

#[test]
fn serialize_data_format_annotations() -> Result<()> {
    let mut m = message::Builder::new_default();
    let mut subject = m.init_root::<json_data::Builder<'_>>();

    subject.set_hex(b"hex value");
    subject.set_base64(b"base64 val");
    {
        let mut b = subject.reborrow().init_hex_list(2);
        b.set(0, b"value 0");
        b.set(1, b"value 1");
    }

    check_ser!(
        subject,
        Default::default(),
        json!({
            "hex": "6865782076616c7565",
            "base64": "YmFzZTY0IHZhbA==",
            "hexList": ["76616c75652030", "76616c75652031"]
        })
    );
    Ok(())
}

#[test]
fn deserialize_data_format_annotations() {
    check_de!(
        json_data,
        json!({
            "hex": "6865782076616c7565",
            "base64": "YmFzZTY0IHZhbA==",
            "hexList": ["76616c75652030", "76616c75652031"],
        }),
        r#"(
            hex = 0x"6865782076616c7565",
            base64 = 0x"6261736536342076616c",
            hexList = [0x"76616c75652030", 0x"76616c75652031"]
        )"#
    );
}

#[test]
fn serialize_name_annotation() -> Result<()> {
    let mut m = message::Builder::new_default();
    let mut subject = m.init_root::<json_rename::Builder<'_>>();

    {
        let mut b = subject.reborrow().init_group();
        b.set_field(json_rename::Enum::Set);
    }
    {
        let mut b = subject.reborrow().init_a_union();
        b.set_set(42);
    }

    check_ser!(
        subject,
        Default::default(),
        json!({
            "renamed-group": {
                "renamed-field": "renamed-enumerant",
            },
            "renamed-union": {
                "set": 42,
            },
        })
    );

    Ok(())
}

#[test]
fn deserialize_name_annotation() {
    check_de!(
        json_rename,
        json!({
            "renamed-group": {
                "renamed-field": "renamed-enumerant",
            },
            "renamed-union": {
                "set": 42,
            },
        }),
        "(group = (field = set), aUnion = (set = 42))"
    );
}

#[test]
fn serialize_unrecognized_enum_field() -> Result<()> {
    let mut m = message::Builder::new_default();
    {
        let mut b = m.init_root::<recognized_enum::Builder<'_>>();
        b.set_field(SupersetEnum::Unique);
    }

    let mut bytes = Vec::new();
    capnp::serialize::write_message(&mut bytes, &m).unwrap();

    let m2 = capnp::serialize::read_message(bytes.as_slice(), Default::default()).unwrap();
    let subject = m2.get_root::<unrecognized_enum::Reader<'_>>().unwrap();

    assert_eq!(subject.get_field(), Err(capnp::NotInSchema(1)));

    let actual_use_number = capnp::json::serialize(
        subject.into(),
        capnp::json::serialize::Opts {
            on_enumerant_not_in_schema: OnEnumerantNotInSchema::UseNumber,
            ..Default::default()
        },
    )
    .unwrap();
    assert_json_eq!(actual_use_number, json!({"field": 1}));

    let actual_error = capnp::json::serialize(
        subject.into(),
        capnp::json::serialize::Opts {
            on_enumerant_not_in_schema: OnEnumerantNotInSchema::Error,
            ..Default::default()
        },
    );
    assert!(actual_error.is_err());

    Ok(())
}

#[test]
fn serialize_empty_all_types() -> Result<()> {
    let mut m = message::Builder::new_default();
    let subject = m.init_root::<test_all_types::Builder<'_>>();

    check_ser!(
        subject,
        Default::default(),
        json!({
            "boolField": false,
            "boolList": null,
            "dataField": null,
            "dataList": null,
            "enumField": "foo",
            "enumList": null,
            "float32Field": 0.0,
            "float32List": null,
            "float64Field": 0.0,
            "float64List": null,
            "int16Field": 0,
            "int16List": null,
            "int32Field": 0,
            "int32List": null,
            "int64Field": 0,
            "int64List": null,
            "int8Field": 0,
            "int8List": null,
            "interfaceField": null,
            "interfaceList": null,
            "structField": null,
            "structList": null,
            "textField": null,
            "textList": null,
            "uInt16Field": 0,
            "uInt16List": null,
            "uInt32Field": 0,
            "uInt32List": null,
            "uInt64Field": 0,
            "uInt64List": null,
            "uInt8Field": 0,
            "uInt8List": null,
            "voidField": null,
            "voidList": null
        })
    );
    Ok(())
}

#[test]
fn serialize_populated_all_types() {
    let mut m = message::Builder::new_default();
    let mut subject = m.init_root::<test_all_types::Builder<'_>>();

    test_util::dynamic_init_test_message(
        dynamic_value::Builder::from(subject.reborrow()).downcast(),
    );

    check_ser!(
        subject,
        Default::default(),
        serde_json::from_str::<serde_json::Value>(
            r#"
    {
        "boolField": true,
        "boolList": [
          true,
          false,
          false,
          true
        ],
        "dataField": "YmFy",
        "dataList": null,
        "enumField": "corge",
        "enumList": null,
        "float32Field": 1234.5,
        "float32List": null,
        "float64Field": -1.23e47,
        "float64List": null,
        "int16Field": -12345,
        "int16List": null,
        "int32Field": -12345678,
        "int32List": null,
        "int64Field": -123456789012345,
        "int64List": null,
        "int8Field": -123,
        "int8List": [
          111,
          -111
        ],
        "interfaceField": null,
        "interfaceList": null,
        "structField": {
          "boolField": true,
          "boolList": null,
          "dataField": null,
          "dataList": null,
          "enumField": "foo",
          "enumList": null,
          "float32Field": 0.0,
          "float32List": null,
          "float64Field": 0.0,
          "float64List": null,
          "int16Field": 3456,
          "int16List": null,
          "int32Field": 0,
          "int32List": null,
          "int64Field": 0,
          "int64List": null,
          "int8Field": -12,
          "int8List": null,
          "interfaceField": null,
          "interfaceList": null,
          "structField": null,
          "structList": null,
          "textField": null,
          "textList": null,
          "uInt16Field": 0,
          "uInt16List": null,
          "uInt32Field": 0,
          "uInt32List": null,
          "uInt64Field": 0,
          "uInt64List": null,
          "uInt8Field": 0,
          "uInt8List": null,
          "voidField": null,
          "voidList": null
        },
        "structList": [
          {
            "boolField": false,
            "boolList": null,
            "dataField": null,
            "dataList": null,
            "enumField": "foo",
            "enumList": null,
            "float32Field": 0.0,
            "float32List": null,
            "float64Field": 0.0,
            "float64List": null,
            "int16Field": 0,
            "int16List": null,
            "int32Field": 0,
            "int32List": null,
            "int64Field": 0,
            "int64List": null,
            "int8Field": 0,
            "int8List": null,
            "interfaceField": null,
            "interfaceList": null,
            "structField": null,
            "structList": null,
            "textField": "structlist 1",
            "textList": null,
            "uInt16Field": 0,
            "uInt16List": null,
            "uInt32Field": 0,
            "uInt32List": null,
            "uInt64Field": 0,
            "uInt64List": null,
            "uInt8Field": 0,
            "uInt8List": null,
            "voidField": null,
            "voidList": null
          },
          {
            "boolField": false,
            "boolList": null,
            "dataField": null,
            "dataList": null,
            "enumField": "foo",
            "enumList": null,
            "float32Field": 0.0,
            "float32List": null,
            "float64Field": 0.0,
            "float64List": null,
            "int16Field": 0,
            "int16List": null,
            "int32Field": 0,
            "int32List": null,
            "int64Field": 0,
            "int64List": null,
            "int8Field": 0,
            "int8List": null,
            "interfaceField": null,
            "interfaceList": null,
            "structField": null,
            "structList": null,
            "textField": "structlist 2",
            "textList": null,
            "uInt16Field": 0,
            "uInt16List": null,
            "uInt32Field": 0,
            "uInt32List": null,
            "uInt64Field": 0,
            "uInt64List": null,
            "uInt8Field": 0,
            "uInt8List": null,
            "voidField": null,
            "voidList": null
          },
          {
            "boolField": false,
            "boolList": null,
            "dataField": null,
            "dataList": null,
            "enumField": "foo",
            "enumList": null,
            "float32Field": 0.0,
            "float32List": null,
            "float64Field": 0.0,
            "float64List": null,
            "int16Field": 0,
            "int16List": null,
            "int32Field": 0,
            "int32List": null,
            "int64Field": 0,
            "int64List": null,
            "int8Field": 0,
            "int8List": null,
            "interfaceField": null,
            "interfaceList": null,
            "structField": null,
            "structList": null,
            "textField": "structlist 3",
            "textList": null,
            "uInt16Field": 0,
            "uInt16List": null,
            "uInt32Field": 0,
            "uInt32List": null,
            "uInt64Field": 0,
            "uInt64List": null,
            "uInt8Field": 0,
            "uInt8List": null,
            "voidField": null,
            "voidList": null
          }
        ],
        "textField": "foo",
        "textList": [
          "plugh",
          "xyzzy",
          "thud"
        ],
        "uInt16Field": 45678,
        "uInt16List": null,
        "uInt32Field": 3456789012,
        "uInt32List": null,
        "uInt64Field": 12345678901234567890,
        "uInt64List": null,
        "uInt8Field": 234,
        "uInt8List": null,
        "voidField": null,
        "voidList": [
          null,
          null,
          null
        ]
      }
"#
        )
        .unwrap()
    );
}
