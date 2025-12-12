#[cfg(test)]
mod tests {
    use crate::json_test_capnp::test_json_annotations;
    use std::io::Write;

    use capnp::message;

    fn cpp_binary_to_json(proto: &str, kind: &str, data: &[u8]) -> capnp::Result<String> {
        let output = std::process::Command::new("capnp")
            .args(["convert", "binary:json", proto, kind])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .and_then(|child| {
                child.stdin.as_ref().map(|mut stdin| stdin.write_all(data));
                child.wait_with_output()
            })?;
        String::from_utf8(output.stdout).map_err(|e| e.into())
    }

    fn cpp_json_to_binary(proto: &str, kind: &str, data: &[u8]) -> capnp::Result<Vec<u8>> {
        let output = std::process::Command::new("capnp")
            .args(["convert", "json:binary", proto, kind])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .and_then(|child| {
                child.stdin.as_ref().map(|mut stdin| stdin.write_all(data));
                child.wait_with_output()
            })?;
        Ok(output.stdout)
    }

    fn make_test_message<A: capnp::message::Allocator>(
        builder: &mut message::Builder<A>,
    ) -> capnp::Result<test_json_annotations::Builder<'_>> {
        let mut root: test_json_annotations::Builder<'_> = builder.init_root();

        root.set_some_field("Some Field");
        {
            let mut a_group = root.reborrow().init_a_group();
            // a_group is flattenned
            a_group.set_flat_foo(0xF00);
            a_group.set_flat_bar("0xBaa");
            a_group.reborrow().init_flat_baz().set_hello(true);
            a_group.reborrow().init_double_flat().set_flat_qux("Qux");
        }

        {
            let mut prefixed_group = root.reborrow().init_prefixed_group();
            prefixed_group.set_foo("Foo");
            prefixed_group.set_bar(0xBAA);
            prefixed_group.reborrow().init_baz().set_hello(false);
            prefixed_group.reborrow().init_more_prefix().set_qux("Qux");
        }

        {
            let mut a_union_bar = root.reborrow().init_a_union().init_bar();
            a_union_bar.set_bar_member(0xAAB);
            a_union_bar.set_multi_member("Member");
        }

        {
            let mut dependency = root.reborrow().init_dependency();
            dependency.set_foo("dep-foo");
        }

        {
            let mut simple_group = root.reborrow().init_simple_group();
            simple_group.set_grault("grault");
        }

        {
            let mut e = root.reborrow().init_enums(4);
            e.set(0, crate::json_test_capnp::TestJsonAnnotatedEnum::Foo);
            e.set(1, crate::json_test_capnp::TestJsonAnnotatedEnum::Bar);
            e.set(2, crate::json_test_capnp::TestJsonAnnotatedEnum::Baz);
            e.set(3, crate::json_test_capnp::TestJsonAnnotatedEnum::Qux);
        }

        {
            let mut b_union = root.reborrow().init_b_union();
            b_union.set_bar(100);
        }

        {
            let mut external_union = root.reborrow().init_external_union();
            external_union.reborrow().init_bar().set_value("Value");
        }

        {
            let mut union_with_void = root.reborrow().init_union_with_void();
            union_with_void.set_void_value(());
        }

        Ok(root)
    }

    fn check_test_message(reader: test_json_annotations::Reader<'_>) -> capnp::Result<()> {
        assert_eq!(reader.get_some_field()?, "Some Field");

        {
            let a_group = reader.get_a_group();
            assert_eq!(a_group.get_flat_foo(), 0xF00);
            assert_eq!(a_group.get_flat_bar()?, "0xBaa");
            assert!(a_group.get_flat_baz().get_hello());
            assert_eq!(a_group.get_double_flat().get_flat_qux()?, "Qux");
        }

        {
            let prefixed_group = reader.get_prefixed_group();
            assert_eq!(prefixed_group.get_foo()?, "Foo");
            assert_eq!(prefixed_group.get_bar(), 0xBAA);
            assert!(!prefixed_group.get_baz().get_hello());
            assert_eq!(prefixed_group.get_more_prefix().get_qux()?, "Qux");
        }

        {
            let a_union = reader.get_a_union();
            match a_union.which()? {
                crate::json_test_capnp::test_json_annotations::a_union::Bar(bar) => {
                    assert_eq!(bar.get_bar_member(), 0xAAB);
                    assert_eq!(bar.get_multi_member()?, "Member");
                }
                _ => panic!("Expected Bar variant"),
            }
        }

        {
            let dependency = reader.get_dependency()?;
            assert_eq!(dependency.get_foo()?, "dep-foo");
        }

        {
            let simple_group = reader.get_simple_group();
            assert_eq!(simple_group.get_grault()?, "grault");
        }

        {
            let enums = reader.get_enums()?;
            assert_eq!(enums.len(), 4);
            assert_eq!(
                enums.get(0)?,
                crate::json_test_capnp::TestJsonAnnotatedEnum::Foo
            );
            assert_eq!(
                enums.get(1)?,
                crate::json_test_capnp::TestJsonAnnotatedEnum::Bar
            );
            assert_eq!(
                enums.get(2)?,
                crate::json_test_capnp::TestJsonAnnotatedEnum::Baz
            );
            assert_eq!(
                enums.get(3)?,
                crate::json_test_capnp::TestJsonAnnotatedEnum::Qux
            );
        }

        {
            let b_union = reader.get_b_union();
            match b_union.which()? {
                crate::json_test_capnp::test_json_annotations::b_union::Bar(value) => {
                    assert_eq!(value, 100);
                }
                _ => panic!("Expected Bar variant"),
            }
        }

        {
            let external_union = reader.get_external_union()?;
            match external_union.which()? {
                crate::json_test_capnp::test_json_annotations3::Bar(bar) => {
                    assert_eq!(bar?.get_value()?, "Value");
                }
                _ => panic!("Expected Bar variant"),
            }
        }

        {
            let union_with_void = reader.get_union_with_void();
            match union_with_void.which()? {
                crate::json_test_capnp::test_json_annotations::union_with_void::VoidValue(()) => {
                    // ok
                }
                _ => panic!("Expected VoidValue variant"),
            }
        }
        Ok(())
    }

    #[test]
    fn read_json_from_cpp_encoder() -> capnp::Result<()> {
        let mut builder = message::Builder::new_default();
        make_test_message(&mut builder)?;
        let mut buf = vec![];
        capnp::serialize::write_message(&mut buf, &builder)?;
        let cpp_json = cpp_binary_to_json("./json-test.capnp", "TestJsonAnnotations", &buf)?;

        let mut buidler = message::Builder::new_default();
        let mut root = buidler.init_root::<test_json_annotations::Builder<'_>>();
        eprintln!("CPP generated JSON: {}", cpp_json);
        capnp_json::from_json(&cpp_json, root.reborrow())?;

        check_test_message(root.into_reader())
    }

    #[test]
    fn write_json_to_cpp() -> capnp::Result<()> {
        let mut builder = message::Builder::new_default();
        let root = make_test_message(&mut builder)?;
        let json = capnp_json::to_json(root.into_reader())?;
        eprintln!("Generated JSON: {}", json);
        let cpp_binary =
            cpp_json_to_binary("./json-test.capnp", "TestJsonAnnotations", json.as_bytes())?;
        let mut cpp_binary = cpp_binary.as_slice();

        let msg = capnp::serialize::read_message_from_flat_slice(
            &mut cpp_binary,
            capnp::message::ReaderOptions::default(),
        )?;

        check_test_message(msg.get_root::<test_json_annotations::Reader<'_>>()?)
    }

    #[test]
    fn roundtrip_unnamed_discriminator() -> capnp::Result<()> {
        let mut builder = message::Builder::new_default();
        let mut root =
            builder.init_root::<crate::json_test_capnp::unnamed_discriminator::Builder>();
        root.reborrow().init_baz().set_bar(100);
        root.reborrow().init_sbaz().set_sfoo("Hello");

        let rust_json = capnp_json::to_json(root.reborrow_as_reader())?;
        eprintln!("Generated JSON: {}", rust_json);

        let mut buf = vec![];
        capnp::serialize::write_message(&mut buf, &builder)?;
        let cpp_json = cpp_binary_to_json("./json-test.capnp", "UnnamedDiscriminator", &buf)?;
        eprintln!("CPP generated JSON: {}", cpp_json);

        let mut read_json_builder = message::Builder::new_default();
        let mut read_json_root =
            read_json_builder.init_root::<crate::json_test_capnp::unnamed_discriminator::Builder>();
        capnp_json::from_json(&cpp_json, read_json_root.reborrow())?;
        let read_json_root = read_json_root.into_reader();

        assert_eq!(
            100,
            match read_json_root.get_baz().which()? {
                crate::json_test_capnp::unnamed_discriminator::baz::Bar(bar) => bar,
                _ => panic!("Expected Bar variant"),
            },
        );
        assert_eq!(
            "Hello",
            match read_json_root.get_sbaz().which()? {
                crate::json_test_capnp::unnamed_discriminator::sbaz::Sfoo(sfoo) =>
                    sfoo?.to_str()?,
                _ => panic!("Expected SFoo variant"),
            },
        );

        let cpp_binary = cpp_json_to_binary(
            "./json-test.capnp",
            "UnnamedDiscriminator",
            rust_json.as_bytes(),
        )?;
        let mut cpp_binary = cpp_binary.as_slice();

        let read_binary = capnp::serialize::read_message_from_flat_slice(
            &mut cpp_binary,
            capnp::message::ReaderOptions::default(),
        )?;
        let read_binary_root =
            read_binary.get_root::<crate::json_test_capnp::unnamed_discriminator::Reader<'_>>()?;

        assert_eq!(
            100,
            match read_binary_root.get_baz().which()? {
                crate::json_test_capnp::unnamed_discriminator::baz::Bar(bar) => bar,
                _ => panic!("Expected Bar variant"),
            },
        );
        assert_eq!(
            "Hello",
            match read_binary_root.get_sbaz().which()? {
                crate::json_test_capnp::unnamed_discriminator::sbaz::Sfoo(sfoo) =>
                    sfoo?.to_str()?,
                _ => panic!("Expected SFoo variant"),
            },
        );

        Ok(())
    }

    #[test]
    fn roundtrip_named_discriminator() -> capnp::Result<()> {
        let mut builder = message::Builder::new_default();
        let mut root = builder.init_root::<crate::json_test_capnp::named_discriminator::Builder>();
        root.reborrow().init_baz().set_bar(100);
        root.reborrow().init_sbaz().set_sfoo("Hello");

        let rust_json = capnp_json::to_json(root.reborrow_as_reader())?;
        eprintln!("Generated JSON: {}", rust_json);

        let mut buf = vec![];
        capnp::serialize::write_message(&mut buf, &builder)?;
        let cpp_json = cpp_binary_to_json("./json-test.capnp", "UnnamedDiscriminator", &buf)?;
        eprintln!("CPP generated JSON: {}", cpp_json);

        let mut read_json_builder = message::Builder::new_default();
        let mut read_json_root =
            read_json_builder.init_root::<crate::json_test_capnp::named_discriminator::Builder>();
        capnp_json::from_json(&cpp_json, read_json_root.reborrow())?;
        let read_json_root = read_json_root.into_reader();

        assert_eq!(
            100,
            match read_json_root.get_baz().which()? {
                crate::json_test_capnp::named_discriminator::baz::Bar(bar) => bar,
                _ => panic!("Expected Bar variant"),
            },
        );
        assert_eq!(
            "Hello",
            match read_json_root.get_sbaz().which()? {
                crate::json_test_capnp::named_discriminator::sbaz::Sfoo(sfoo) => sfoo?.to_str()?,
                _ => panic!("Expected SFoo variant"),
            },
        );

        let cpp_binary = cpp_json_to_binary(
            "./json-test.capnp",
            "UnnamedDiscriminator",
            rust_json.as_bytes(),
        )?;
        let mut cpp_binary = cpp_binary.as_slice();

        let read_binary = capnp::serialize::read_message_from_flat_slice(
            &mut cpp_binary,
            capnp::message::ReaderOptions::default(),
        )?;
        let read_binary_root =
            read_binary.get_root::<crate::json_test_capnp::named_discriminator::Reader<'_>>()?;

        assert_eq!(
            100,
            match read_binary_root.get_baz().which()? {
                crate::json_test_capnp::named_discriminator::baz::Bar(bar) => bar,
                _ => panic!("Expected Bar variant"),
            },
        );
        assert_eq!(
            "Hello",
            match read_binary_root.get_sbaz().which()? {
                crate::json_test_capnp::named_discriminator::sbaz::Sfoo(sfoo) => sfoo?.to_str()?,
                _ => panic!("Expected SFoo variant"),
            },
        );

        Ok(())
    }
}
