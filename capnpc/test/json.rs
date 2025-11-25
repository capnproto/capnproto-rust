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

use crate::json_test_capnp::test_json_annotations;
use crate::test_capnp::{
    test_json_flatten_union, test_json_types, test_union, test_unnamed_union, TestEnum,
};
use capnp::dynamic_value;
use capnp::message::{self};

use capnp::json::{self};

// Primitive and Pointer field encoding

#[test]
fn test_encode_json_types_default() {
    let mut builder = message::Builder::new_default();
    let root: test_json_types::Builder<'_> = builder.init_root();
    let root: dynamic_value::Builder<'_> = root.into();

    let msg = root.into_reader();
    let json_str = json::to_json(msg).unwrap();
    let expected = r#"{"voidField":null,"boolField":false,"int8Field":0,"int16Field":0,"int32Field":0,"int64Field":0,"uInt8Field":0,"uInt16Field":0,"uInt32Field":0,"uInt64Field":0,"float32Field":0,"float64Field":0,"enumField":"foo"}"#;
    assert_eq!(expected, json_str);
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

    let root: dynamic_value::Builder<'_> = root.into();

    let msg = root.into_reader();
    let json_str = json::to_json(msg).unwrap();
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
        r#""enumList":["foo","bar","garply"]"#,
        "}"
    );
    assert_eq!(expected, json_str);
}

#[test]
fn test_integer_encoding() {}

#[test]
fn test_float_encoding() {}

#[test]
fn test_hex_encoding() {}

#[test]
fn test_base64_encoding() {}

#[test]
fn test_string_encoding() {}

#[test]
fn test_array_encoding() {}

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

    let root: dynamic_value::Builder<'_> = root.into();
    let msg = root.into_reader();
    let json_str = json::to_json(msg).unwrap();

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

    assert_eq!(expected, json_str);
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
    let root: dynamic_value::Builder<'_> = root.into();
    let msg = root.into_reader();
    let json_str = json::to_json(msg).unwrap();
    let expected = concat!(
        "{",
        r#""before":"before","#,
        r#""middle":1234,"#,
        r#""bar":32,"#,
        r#""after":"after""#,
        "}",
    );
    assert_eq!(expected, json_str);
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

    let root: dynamic_value::Builder<'_> = root.into();
    let msg = root.into_reader();
    let json_str = json::to_json(msg).unwrap();
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
    assert_eq!(expected, json_str);
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
        let mut b_union = root.reborrow().init_b_union();
        expected.push_str(r#""bUnion":"foo","#);
        b_union.set_foo("b-free");
        expected.push_str(r#""bValue":"b-free","#);
    }

    {
        let mut external_union = root.reborrow().init_external_union();
        external_union.reborrow().init_bar().set_value("Value");
        expected.push_str(r#""externalUnion":{"type":"bar","value":"Value"},"#);
    }

    {
        let mut union_with_void = root.reborrow().init_union_with_void();
        union_with_void.set_void_value(());
        expected.push_str(r#""unionWithVoid":{"type":"voidValue","voidValue":null},"#);
    }

    expected.pop(); // Remove trailing comma
    expected.push('}');

    let root: dynamic_value::Builder<'_> = root.into();
    let msg = root.into_reader();
    let json_str = json::to_json(msg).unwrap();
    assert_eq!(expected, json_str);
}

