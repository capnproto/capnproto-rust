// Copyright (c) 2025 Ben Jackson [puremourning@gmail.com] and Cap'n Proto contributors
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

capnp::generated_code!(pub mod test_capnp);
capnp::generated_code!(pub mod json_test_capnp);

#[cfg(test)]
mod tests {
    use crate::json_test_capnp::test_json_annotations;
    use crate::test_capnp::{
        test_json_flatten_union, test_json_types, test_union, test_unnamed_union, TestEnum,
    };
    use capnp::message;
    use capnp_json as json;

    #[test]
    fn test_encode_json_types_default() {
        let mut builder = message::Builder::new_default();
        let root: test_json_types::Builder<'_> = builder.init_root();
        let expected = r#"{"voidField":null,"boolField":false,"int8Field":0,"int16Field":0,"int32Field":0,"int64Field":0,"uInt8Field":0,"uInt16Field":0,"uInt32Field":0,"uInt64Field":0,"float32Field":0,"float64Field":0,"enumField":"foo"}"#;
        assert_eq!(expected, json::to_json(root.reborrow_as_reader()).unwrap());
    }

    #[test]
    fn test_encode_all_json_types() {
        let mut builder = message::Builder::new_default();
        let mut root: test_json_types::Builder<'_> = builder.init_root();
        root.set_int8_field(-8);
        root.set_int16_field(-16);
        root.set_int32_field(-32);
        root.set_int64_field(-64);
        root.set_u_int8_field(8);
        root.set_u_int16_field(16);
        root.set_u_int32_field(32);
        root.set_u_int64_field(64);
        root.set_bool_field(true);
        root.set_void_field(());
        root.set_text_field("hello");
        root.set_float32_field(1.32);
        root.set_float64_field(1.64);
        root.set_data_field(&[0xde, 0xad, 0xbe, 0xef, 0xca, 0xfe, 0xba, 0xbe]);
        root.set_base64_field(&[0xde, 0xad, 0xbe, 0xef, 0xca, 0xfe, 0xba, 0xbe]);
        root.set_hex_field(&[0xde, 0xad, 0xbe, 0xef, 0xca, 0xfe, 0xba, 0xbe]);
        {
            let mut embedded = root.reborrow().init_struct_field();
            let mut text_list = embedded.reborrow().init_text_list(2);
            text_list.set(0, "frist");
            text_list.set(1, "segund");
            embedded.set_text_field("inner");
            let mut hex_list = embedded.reborrow().init_hex_list(2);
            hex_list.set(0, &[0xde, 0xad, 0xbe, 0xef]);
            hex_list.set(1, &[0xba, 0xdf, 0x00, 0xd0]);
            let mut based_list = embedded.reborrow().init_base64_list(2);
            based_list.set(0, &[0xde, 0xad, 0xbe, 0xef]);
            based_list.set(1, &[0xba, 0xdf, 0x00, 0xd0]);
        }
        root.set_enum_field(TestEnum::Quux);
        {
            let mut enum_list = root.reborrow().init_enum_list(3);
            enum_list.set(0, TestEnum::Foo);
            enum_list.set(1, TestEnum::Bar);
            enum_list.set(2, TestEnum::Garply);
        }
        {
            let mut floats = root.reborrow().init_float32_list(3);
            floats.set(0, f32::NAN);
            floats.set(1, f32::INFINITY);
            floats.set(2, f32::NEG_INFINITY);
        }
        {
            let mut floats = root.reborrow().init_float64_list(3);
            floats.set(0, f64::NAN);
            floats.set(1, f64::INFINITY);
            floats.set(2, f64::NEG_INFINITY);
        }

        let expected = concat!(
            "{",
            r#""voidField":null,"#,
            r#""boolField":true,"#,
            r#""int8Field":-8,"#,
            r#""int16Field":-16,"#,
            r#""int32Field":-32,"#,
            r#""int64Field":-64,"#,
            r#""uInt8Field":8,"#,
            r#""uInt16Field":16,"#,
            r#""uInt32Field":32,"#,
            r#""uInt64Field":64,"#,
            r#""float32Field":1.3200000524520874,"#,
            r#""float64Field":1.64,"#,
            r#""textField":"hello","#,
            r#""dataField":[222,173,190,239,202,254,186,190],"#,
            r#""base64Field":"3q2+78r+ur4=","#,
            r#""hexField":"deadbeefcafebabe","#,
            r#""structField":{"#,
            r#""voidField":null,"#,
            r#""boolField":false,"#,
            r#""int8Field":0,"#,
            r#""int16Field":0,"#,
            r#""int32Field":0,"#,
            r#""int64Field":0,"#,
            r#""uInt8Field":0,"#,
            r#""uInt16Field":0,"#,
            r#""uInt32Field":0,"#,
            r#""uInt64Field":0,"#,
            r#""float32Field":0,"#,
            r#""float64Field":0,"#,
            r#""textField":"inner","#,
            r#""enumField":"foo","#,
            r#""textList":["frist","segund"],"#,
            r#""base64List":["3q2+7w==","ut8A0A=="],"#,
            r#""hexList":["deadbeef","badf00d0"]"#,
            "},",
            r#""enumField":"quux","#,
            r#""float32List":["NaN","Infinity","-Infinity"],"#,
            r#""float64List":["NaN","Infinity","-Infinity"],"#,
            r#""enumList":["foo","bar","garply"]"#,
            "}"
        );
        assert_eq!(expected, json::to_json(root.reborrow_as_reader()).unwrap());
    }

    // Union encoding with flattening

    #[test]
    fn test_named_union_non_flattened() {
        let mut builder = message::Builder::new_default();
        let mut root: test_union::Builder<'_> = builder.init_root();
        root.set_bit0(true);
        root.set_bit2(false);
        root.set_bit3(true);
        root.set_bit4(false);
        root.set_bit5(true);
        root.set_bit6(false);
        root.set_bit7(true);
        root.set_byte0(0xAA);
        let mut union0 = root.reborrow().init_union0();
        union0.set_u0f0sp("not this one");
        union0.set_u0f0s16(-12345);

        let expected = concat!(
            "{",
            r#""union0":{"u0f0s16":-12345},"#,
            r#""union1":{"u1f0s0":null},"#,
            r#""union2":{"u2f0s1":false},"#,
            r#""union3":{"u3f0s1":false},"#,
            r#""bit0":true,"#,
            r#""bit2":false,"#,
            r#""bit3":true,"#,
            r#""bit4":false,"#,
            r#""bit5":true,"#,
            r#""bit6":false,"#,
            r#""bit7":true,"#,
            r#""byte0":170"#,
            "}",
        );

        assert_eq!(expected, json::to_json(root.reborrow_as_reader()).unwrap());
    }

    #[test]
    fn test_unnamed_union() {
        let mut builder = message::Builder::new_default();
        let mut root: test_unnamed_union::Builder<'_> = builder.init_root();
        root.set_before("before");
        root.set_middle(1234);
        root.set_after("after");
        root.set_foo(16);
        root.set_bar(32);
        let expected = concat!(
            "{",
            r#""before":"before","#,
            r#""middle":1234,"#,
            r#""after":"after","#,
            r#""bar":32"#,
            "}",
        );
        assert_eq!(expected, json::to_json(root.reborrow_as_reader()).unwrap());
    }

    #[test]
    fn test_named_union_flattened() {
        let mut builder = message::Builder::new_default();
        let mut root: test_json_flatten_union::Builder<'_> = builder.init_root();
        root.set_before("before");
        root.set_middle(1234);
        root.set_after("after");
        let mut maybe = root.reborrow().init_maybe();
        maybe.set_foo(16);
        maybe.set_bar(32);

        let expected = concat!(
            "{",
            r#""before":"before","#,
            r#""maybe_bar":32,"#,
            r#""middle":1234,"#,
            r#""after":"after","#,
            r#""foo":0,"#,
            r#""bar":0,"#,
            r#""nested_baz":0,"#,
            r#""baz":0"#,
            "}",
        );
        assert_eq!(expected, json::to_json(root.reborrow_as_reader()).unwrap());
    }

    #[test]
    fn test_discriminated_union() {
        let mut builder = message::Builder::new_default();
        let mut root: test_json_annotations::Builder<'_> = builder.init_root();

        let mut expected = String::from("{");

        root.set_some_field("Some Field");
        expected.push_str(r#""names-can_contain!anything Really":"Some Field","#);

        {
            let mut a_group = root.reborrow().init_a_group();
            // a_group is flattenned
            a_group.set_flat_foo(0xF00);
            expected.push_str(r#""flatFoo":3840,"#);

            a_group.set_flat_bar("0xBaa");
            expected.push_str(r#""flatBar":"0xBaa","#);

            a_group.reborrow().init_flat_baz().set_hello(true);
            expected.push_str(r#""renamed-flatBaz":{"hello":true},"#);

            a_group.reborrow().init_double_flat().set_flat_qux("Qux");
            expected.push_str(r#""flatQux":"Qux","#);
        }

        {
            let mut prefixed_group = root.reborrow().init_prefixed_group();
            prefixed_group.set_foo("Foo");
            expected.push_str(r#""pfx.foo":"Foo","#);

            prefixed_group.set_bar(0xBAA);
            expected.push_str(r#""pfx.renamed-bar":2986,"#);

            prefixed_group.reborrow().init_baz().set_hello(false);
            expected.push_str(r#""pfx.baz":{"hello":false},"#);

            prefixed_group.reborrow().init_more_prefix().set_qux("Qux");
            expected.push_str(r#""pfx.xfp.qux":"Qux","#);
        }

        {
            let mut a_union_bar = root.reborrow().init_a_union().init_bar();
            expected.push_str(r#""union-type":"renamed-bar","#);
            a_union_bar.set_bar_member(0xAAB);
            expected.push_str(r#""barMember":2731,"#);
            a_union_bar.set_multi_member("Member");
            expected.push_str(r#""multiMember":"Member","#);
        }

        {
            let mut dependency = root.reborrow().init_dependency();
            dependency.set_foo("dep-foo");
            expected.push_str(r#""dependency":{"renamed-foo":"dep-foo"},"#);
        }

        {
            let mut simple_group = root.reborrow().init_simple_group();
            simple_group.set_grault("grault");
            expected.push_str(r#""simpleGroup":{"renamed-grault":"grault"},"#);
        }

        {
            let mut e = root.reborrow().init_enums(4);
            e.set(0, crate::json_test_capnp::TestJsonAnnotatedEnum::Foo);
            e.set(1, crate::json_test_capnp::TestJsonAnnotatedEnum::Bar);
            e.set(2, crate::json_test_capnp::TestJsonAnnotatedEnum::Baz);
            e.set(3, crate::json_test_capnp::TestJsonAnnotatedEnum::Qux);
            expected.push_str(r#""enums":["foo","renamed-bar","renamed-baz","qux"],"#);
        }

        {
            let mut b_union = root.reborrow().init_b_union();
            expected.push_str(r#""bUnion":"renamed-bar","#);
            b_union.set_bar(100);
            expected.push_str(r#""bValue":100,"#);
        }

        {
            let mut external_union = root.reborrow().init_external_union();
            external_union.reborrow().init_bar().set_value("Value");
            expected.push_str(r#""externalUnion":{"type":"bar","value":"Value"},"#);
        }

        {
            let mut union_with_void = root.reborrow().init_union_with_void();
            union_with_void.set_void_value(());
            expected.push_str(r#""unionWithVoid":{"type":"voidValue"},"#);
        }

        expected.pop(); // Remove trailing comma
        expected.push('}');

        assert_eq!(expected, json::to_json(root.reborrow_as_reader()).unwrap());
    }

    #[test]
    fn test_base64_union() {
        let mut builder = message::Builder::new_default();
        let mut root: crate::json_test_capnp::test_base64_union::Builder<'_> = builder.init_root();

        root.set_foo(&[0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(
            r#"{"foo":"3q2+7w=="}"#,
            json::to_json(root.reborrow_as_reader()).unwrap()
        );
    }

    #[test]
    fn test_string_encoding() {
        let mut builder = message::Builder::new_default();
        let mut root: crate::json_test_capnp::test_flattened_struct::Builder<'_> =
            builder.init_root();

        root.set_value("");
        assert_eq!(
            r#"{"value":""}"#,
            json::to_json(root.reborrow_as_reader()).unwrap()
        );

        root.set_value("tab: \t, newline: \n, carriage return: \r, quote: \", backslash: \\");
        assert_eq!(
            r#"{"value":"tab: \t, newline: \n, carriage return: \r, quote: \", backslash: \\"}"#,
            json::to_json(root.reborrow_as_reader()).unwrap()
        );

        root.set_value("unicode: †eśt");
        assert_eq!(
            r#"{"value":"unicode: †eśt"}"#,
            json::to_json(root.reborrow_as_reader()).unwrap()
        );

        root.set_value("backspace: \u{0008}, formfeed: \u{000C}");
        assert_eq!(
            r#"{"value":"backspace: \b, formfeed: \f"}"#,
            json::to_json(root.reborrow_as_reader()).unwrap()
        );

        root.set_value("bell: \u{0007}, SOH: \u{0001}");
        assert_eq!(
            r#"{"value":"bell: \u0007, SOH: \u0001"}"#,
            json::to_json(root.reborrow_as_reader()).unwrap()
        );
    }

    #[test]
    fn test_nested_data_list() -> capnp::Result<()> {
        let mut builder = message::Builder::new_default();
        let mut root = builder.init_root::<crate::json_test_capnp::nested_hex::Builder<'_>>();
        let mut awd = root.reborrow().init_data_all_the_way_down(2);
        let mut first = awd.reborrow().init(0, 2);
        first.set(0, &[0xde, 0xad, 0xbe, 0xef]);
        first.set(1, &[0xef, 0xbe, 0xad, 0xde]);
        let mut second = awd.reborrow().init(1, 1);
        second.set(0, &[0xba, 0xdf, 0x00, 0xd0]);

        assert_eq!(
            r#"{"dataAllTheWayDown":[["deadbeef","efbeadde"],["badf00d0"]]}"#,
            json::to_json(root.reborrow_as_reader())?
        );

        Ok(())
    }

    // Decode

    #[test]
    fn test_decode_simple() -> capnp::Result<()> {
        let mut builder = message::Builder::new_default();
        let mut root: test_json_types::Builder<'_> = builder.init_root();
        json::from_json(
            r#"
            {
              "voidField": null,
              "boolField": true,
              "int8Field": -8,
              "int16Field": -16,
              "int32Field": -32,
              "int64Field": -64,
              "uInt8Field": 8,
              "uInt16Field": 16,
              "uInt32Field": 32,
              "uInt64Field": 64,
              "float32Field": 1.3200000524520874,
              "float64Field": 0.164e2,
              "textField": "hello",
              "dataField": [
                222,
                173

                ,

                190,
                239,
                202,
                254,
                186,
                190
              ],
              "base64Field": "3q2+78r+ur4=",
              "hexField": "deadbeefcafebabe",
              "structField": {
                "voidField": null,
                "boolField": false,
                "int8Field": 0,
                "int16Field": 0,
                "int32Field": 0,
                "int64Field": 0,
                "uInt8Field": 0,
                "uInt16Field"
                : 0,
                "uInt32Field": 0,
                "uInt64Field": 0,
                "float32Field": 0,
                "float64Field": 0,
                "textField": "inner",
                "enumField": "foo",
                "textList": [
                  "frist",
                  "segund"
                ],
                "base64List": [
                  "3q2+7w==",
                  "ut8A0A=="
                ],
                "hexList": [
                  "deadbeef",
                  "badf00d0"
                ]
              },
              "enumField": "quux",
              "float32List": [
                "NaN",
                "Infinity",
                "-Infinity"
              ],
              "float64List": [
                "NaN",
                "Infinity" ,
                "-Infinity"
              ],
              "enumList": [
                "foo",
                "bar",
                "garply"
              ],
              "int64List": [
                1,
                2,
                4,
                8
              ]
            }
          "#,
            root.reborrow(),
        )?;

        let reader = root.into_reader();
        assert_eq!((), reader.get_void_field());
        assert!(reader.get_bool_field());
        assert_eq!(-8, reader.get_int8_field());
        assert_eq!(-16, reader.get_int16_field());
        assert_eq!(-32, reader.get_int32_field());
        assert_eq!(-64, reader.get_int64_field());
        assert_eq!(8, reader.get_u_int8_field());
        assert_eq!(16, reader.get_u_int16_field());
        assert_eq!(32, reader.get_u_int32_field());
        assert_eq!(64, reader.get_u_int64_field());
        assert_eq!(1.32, reader.get_float32_field());
        assert_eq!(16.4, reader.get_float64_field());
        assert_eq!("hello", reader.get_text_field()?.to_str()?);
        assert_eq!(
            [0xde, 0xad, 0xbe, 0xef, 0xca, 0xfe, 0xba, 0xbe],
            reader.get_data_field()?
        );
        assert_eq!(
            [0xde, 0xad, 0xbe, 0xef, 0xca, 0xfe, 0xba, 0xbe],
            reader.get_base64_field()?
        );
        assert_eq!(
            [0xde, 0xad, 0xbe, 0xef, 0xca, 0xfe, 0xba, 0xbe],
            reader.get_hex_field()?
        );

        for i in 0..4 {
            assert_eq!(1 << i, reader.get_int64_list()?.get(i as u32));
        }

        Ok(())
    }

    #[test]
    fn test_encode_with_empty_flattened() -> capnp::Result<()> {
        let mut builder = capnp::message::Builder::new_default();
        let root =
            builder.init_root::<crate::json_test_capnp::test_json_annotations::Builder<'_>>();

        assert_eq!(
            r#"{"flatFoo":0,"renamed-flatBaz":{"hello":false},"pfx.renamed-bar":0,"pfx.baz":{"hello":false},"union-type":"foo","multiMember":0,"simpleGroup":{},"unionWithVoid":{"type":"intValue","intValue":0}}"#,
            json::to_json(root.reborrow_as_reader())?
        );

        Ok(())
    }

    #[test]
    fn test_decode_flattened() -> capnp::Result<()> {
        let j = r#"
        {
          "names-can_contain!anything Really": "Some Field",
          "flatFoo": 1234,
          "flatBar": "0xBaa",
          "renamed-flatBaz": {"hello": true},
          "flatQux": "Qux",
          "pfx.baz": {"hello": true},
          "union-type": "renamed-bar",
          "barMember": 2731,
          "multiMember": "Member",
          "bUnion": "renamed-bar",
          "bValue": 100
        }
      "#;
        let mut builder = capnp::message::Builder::new_default();
        let mut root =
            builder.init_root::<crate::json_test_capnp::test_json_annotations::Builder<'_>>();
        json::from_json(j, root.reborrow())?;

        let reader = root.into_reader();
        assert_eq!("Some Field", reader.get_some_field()?.to_str()?);
        assert_eq!(1234, reader.get_a_group().get_flat_foo());
        assert_eq!("0xBaa", reader.get_a_group().get_flat_bar()?.to_str()?);
        assert!(reader.get_a_group().get_flat_baz().get_hello());
        assert_eq!(
            "Qux",
            reader
                .get_a_group()
                .get_double_flat()
                .get_flat_qux()?
                .to_str()?
        );
        assert!(reader.get_prefixed_group().get_baz().get_hello());
        assert!(matches!(
            reader.get_a_union().which()?,
            crate::json_test_capnp::test_json_annotations::a_union::Bar(_)
        ));
        {
            let bar = match reader.get_a_union().which()? {
                crate::json_test_capnp::test_json_annotations::a_union::Bar(b) => b,
                _ => panic!("Expected Bar"),
            };
            assert_eq!(2731, bar.get_bar_member());
            assert_eq!("Member", bar.get_multi_member()?.to_str()?);
        }
        assert!(matches!(
            reader.get_b_union().which()?,
            crate::json_test_capnp::test_json_annotations::b_union::Bar(_)
        ));
        {
            let bar = match reader.get_b_union().which()? {
                crate::json_test_capnp::test_json_annotations::b_union::Bar(b) => b,
                _ => panic!("Expected Bar"),
            };
            assert_eq!(100, bar);
        }

        Ok(())
    }

    #[test]
    fn test_decode_base64_union() -> capnp::Result<()> {
        {
            let j = r#"
            {
              "foo":"3q2+7w=="
            }
          "#;
            let mut builder = capnp::message::Builder::new_default();
            let mut root =
                builder.init_root::<crate::json_test_capnp::test_base64_union::Builder<'_>>();
            json::from_json(j, root.reborrow())?;

            let reader = root.into_reader();
            assert!(matches!(
                reader.which()?,
                crate::json_test_capnp::test_base64_union::Foo(_)
            ));
            {
                let foo = match reader.which()? {
                    crate::json_test_capnp::test_base64_union::Foo(f) => f,
                    _ => panic!("Expected Foo"),
                }?;
                assert_eq!(&[0xde, 0xad, 0xbe, 0xef], foo);
            }
        }

        {
            let j = r#"
            {
              "bar":"To the bar!"
            }
          "#;
            let mut builder = capnp::message::Builder::new_default();
            let mut root =
                builder.init_root::<crate::json_test_capnp::test_base64_union::Builder<'_>>();
            json::from_json(j, root.reborrow())?;

            let reader = root.into_reader();
            assert!(matches!(
                reader.which()?,
                crate::json_test_capnp::test_base64_union::Bar(_)
            ));
            {
                let bar = match reader.which()? {
                    crate::json_test_capnp::test_base64_union::Bar(b) => b?,
                    _ => panic!("Expected Foo"),
                };
                assert_eq!("To the bar!", bar.to_str()?);
            }
        }

        // When both variants are present, we pick the first one in the spec
        {
            let j = r#"
            {
              "bar":"To the bar!",
              "foo":"3q2+7w=="
            }
          "#;
            let mut builder = capnp::message::Builder::new_default();
            let mut root =
                builder.init_root::<crate::json_test_capnp::test_base64_union::Builder<'_>>();
            json::from_json(j, root.reborrow())?;

            let reader = root.into_reader();
            assert!(matches!(
                reader.which()?,
                crate::json_test_capnp::test_base64_union::Foo(_)
            ));
            {
                let foo = match reader.which()? {
                    crate::json_test_capnp::test_base64_union::Foo(f) => f,
                    _ => panic!("Expected Foo"),
                }?;
                assert_eq!(&[0xde, 0xad, 0xbe, 0xef], foo);
            }
        }

        {
            let j = r#"
            {
              "bar":"To the bar!",
              "foo":"3q2+7w=="
            }
          "#;
            let mut builder = capnp::message::Builder::new_default();
            let mut root =
                builder.init_root::<crate::json_test_capnp::test_renamed_anon_union::Builder<'_>>();
            json::from_json(j, root.reborrow())?;

            let reader = root.into_reader();
            assert!(matches!(
                reader.which()?,
                crate::json_test_capnp::test_renamed_anon_union::Bar(_)
            ));
            {
                let bar = match reader.which()? {
                    crate::json_test_capnp::test_renamed_anon_union::Bar(b) => b?,
                    _ => panic!("Expected Foo"),
                };
                assert_eq!("To the bar!", bar.to_str()?);
            }
        }

        {
            let j = r#"
            {
              "bar":"To the bar!",
              "renamed-foo":"3q2+7w=="
            }
          "#;
            let mut builder = capnp::message::Builder::new_default();
            let mut root =
                builder.init_root::<crate::json_test_capnp::test_renamed_anon_union::Builder<'_>>();
            json::from_json(j, root.reborrow())?;

            let reader = root.into_reader();
            assert!(matches!(
                reader.which()?,
                crate::json_test_capnp::test_renamed_anon_union::Foo(_)
            ));
            {
                let foo = match reader.which()? {
                    crate::json_test_capnp::test_renamed_anon_union::Foo(f) => f,
                    _ => panic!("Expected Foo"),
                }?;
                assert_eq!(&[0xde, 0xad, 0xbe, 0xef], foo);
            }
        }
        Ok(())
    }

    #[test]
    fn test_decode_nested_data_list() -> capnp::Result<()> {
        let json = r#"{"dataAllTheWayDown":[["deadbeef","efbeadde"],["badf00d0"]]}"#;
        let mut builder = message::Builder::new_default();
        let mut root = builder.init_root::<crate::json_test_capnp::nested_hex::Builder<'_>>();
        json::from_json(json, root.reborrow())?;

        let reader = root.into_reader();

        {
            let awd = reader.get_data_all_the_way_down()?;
            let first = awd.get(0)?;
            assert_eq!(2, first.len());
            assert_eq!(&[0xde, 0xad, 0xbe, 0xef], first.get(0)?);
            assert_eq!(&[0xef, 0xbe, 0xad, 0xde], first.get(1)?);
            let second = awd.get(1)?;
            assert_eq!(1, second.len());
            assert_eq!(&[0xba, 0xdf, 0x00, 0xd0], second.get(0)?);
        }

        Ok(())
    }

    #[test]
    fn test_decode_union_with_void() -> capnp::Result<()> {
        let json = r#"
        {
          "unionWithVoid": {
            "type": "voidValue"
          }
        }
      "#;

        let mut builder = capnp::message::Builder::new_default();
        let mut root =
            builder.init_root::<crate::json_test_capnp::test_json_annotations::Builder<'_>>();
        json::from_json(json, root.reborrow())?;

        let reader = root.into_reader();
        assert!(matches!(
            reader.get_union_with_void().which()?,
            crate::json_test_capnp::test_json_annotations::union_with_void::VoidValue(_)
        ));

        Ok(())
    }
}
