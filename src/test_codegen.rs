use codegen::*;
use codegen_types::*;

struct CapnpcTest {
    path: ::tempdir::TempDir,
    buffer: Vec<u8>,
    message: ::capnp::message::Reader<::capnp::serialize::OwnedSegments>,
}

impl CapnpcTest {
    fn new(schema:&str) -> CapnpcTest {
        use std::io::Write;

        let tmp = ::tempdir::TempDir::new("capnpc-unit-test").unwrap();
        let prefix = tmp.path().to_path_buf().clone();
        let mut filename = prefix.clone();
        filename.push("utest");
        ::std::fs::File::create(&filename).unwrap().write_all(schema.as_bytes()).unwrap();

        let mut command = ::std::process::Command::new("capnp");
        command.arg("compile").arg("-o").arg("-")
               .arg(&format!("--src-prefix={}", prefix.to_str().unwrap()));
        command.arg(&filename);
        let parsed = command.output().unwrap();
        let buffer:Vec<u8> = parsed.stdout;

        let mut reader = ::std::io::Cursor::new(buffer.clone());
        let message = ::capnp::serialize::read_message(&mut reader, ::capnp::message::ReaderOptions::new()).unwrap();

        CapnpcTest {
            path: tmp,
            buffer: buffer,
            message: message,
        }
    }

    #[allow(dead_code)]
    fn capnp_decode_dump(&self) {
        use std::io::Write;
        let mut command = ::std::process::Command::new("capnp");
        command.arg("decode").arg("/usr/local/include/capnp/schema.capnp").arg("CodeGeneratorRequest");
        command.stdin(::std::process::Stdio::piped());
        let mut process = command.spawn().unwrap();
        process.stdin.as_mut().unwrap().write_all(&*self.buffer).unwrap();
        let _ = process.wait();
    }

    fn codegen(&self) {
        let reader = ::std::io::Cursor::new(self.buffer.clone());
        ::codegen::main(reader, self.path.path()).unwrap();
    }

    fn compile(&self) -> ::std::process::ExitStatus {
        use std::io::Write;
        let mut libfile = self.path.path().to_path_buf().clone();
        libfile.push("lib.rs");
        let lib = br#"extern crate capnp;
        mod utest_capnp { include!("utest_capnp.rs"); }
        "#;
        ::std::fs::File::create(&libfile).unwrap().write_all(lib).unwrap();
        let mut command = ::std::process::Command::new("rustc");
        command.arg(libfile.to_str().unwrap());
        command.arg("--crate-type").arg("lib").arg("-g");
        command.arg("--out-dir").arg(self.path.path().to_str().unwrap());
        command.arg("-L").arg("test/target/debug");
        command.arg("-L").arg("test/target/debug/deps");
        command.status().unwrap()
    }

    #[allow(dead_code)]
    fn persist(self) -> () {
        panic!("Persisted test files to {}", self.path.into_path().to_str().unwrap());
    }

    fn compile_or_persist(self) {
        if !self.compile().success() {
            self.persist()
        }
    }
}

#[test]
fn test_context_basics() {
    let test = CapnpcTest::new("@0x99d187209d25cee7; struct Foo { foo @0: UInt64; }");
    {
        let gen = ::codegen::GeneratorContext::new(&test.message).unwrap();
        assert_eq!(1, gen.request.get_requested_files().unwrap().iter().count());
        let file = gen.request.get_requested_files().unwrap().get(0);
        assert_eq!(0x99d187209d25cee7u64, file.get_id());
        let file_node = &gen.node_map[&file.get_id()];
        let nodes = file_node.get_nested_nodes().unwrap();
        assert_eq!(1, nodes.len());
        let st = nodes.get(0);
        assert_eq!("Foo", st.get_name().unwrap());
    }
    test.codegen();
    test.compile_or_persist();
}

#[cfg(test)]
fn get_node_by_name<'a>(gen: &'a ::codegen::GeneratorContext, name:&str)
        -> Option<&'a ::schema_capnp::node::Reader<'a>> {
    gen.node_map.values().find(|n| n.get_display_name().unwrap() == name)
}

#[cfg(test)]
fn node_as_struct<'a>(st:&::schema_capnp::node::Reader<'a>)
        -> ::schema_capnp::node::struct_::Reader<'a> {
    match st.which().unwrap() {
        ::schema_capnp::node::Struct(struct_reader) => struct_reader,
        _ => { panic!("expected a struct here") }
    }
}

#[cfg(test)]
fn field_as_slot<'a>(field:&::schema_capnp::field::Reader<'a>)
        -> ::schema_capnp::field::slot::Reader<'a> {
    match field.which().unwrap() {
        ::schema_capnp::field::Slot(slot) => slot,
        _ => panic!("expected a slot"),
    }
}

#[cfg(test)]
fn type_string_for(gen: &::codegen::GeneratorContext, st:&::schema_capnp::node::struct_::Reader, field_name:&str) -> String {
    let field = st.get_fields().unwrap().iter().find(|f| f.get_name().unwrap() == field_name).unwrap();
    field_as_slot(&field).get_type().unwrap().type_string(&gen, Module::Reader, "'a")
}

#[test]
fn test_stringify_basics() {
    let test = CapnpcTest::new("@0x99d187209d25cee7; struct Foo { foo @0: UInt64; }");
    {
        let gen = ::codegen::GeneratorContext::new(&test.message).unwrap();
        let st = node_as_struct(get_node_by_name(&gen, "utest:Foo").unwrap());
        assert_eq!(1, st.get_fields().unwrap().len());
        let field = st.get_fields().unwrap().get(0);
        assert_eq!("foo", field.get_name().unwrap());
        let text = getter_text(&gen, &field, true);
        assert_eq!("u64", text.0);
    }
    test.codegen();
    test.compile_or_persist();
}

#[test]
fn test_non_generic_map() {
    let test = CapnpcTest::new(r#"@0x99d187209d25cee7; struct Map {
        entries @0 :List(Entry);
        struct Entry { key @0 :Text; value @1 :Text; }
    }"#);
    test.codegen();
    test.compile_or_persist();
}

#[test]
fn test_trivial_generic() {
    let test = CapnpcTest::new(r#"@0x99d187209d25cee7; struct Gen(T) { t @0: T; }"#);
    test.codegen();
    test.compile_or_persist();
}

#[test]
fn test_nested_generic() {
    let test = CapnpcTest::new(r#"@0x99d187209d25cee7; struct Gen(T) { struct Inner { t @0: T; } }"#);
    test.codegen();
    test.compile_or_persist();
}

#[test]
#[ignore]
fn test_generic_list() {
    let test = CapnpcTest::new(r#"@0x99d187209d25cee7; struct Gen(T) { l @0 :List(T); }"#);
    test.codegen();
    test.compile_or_persist();
}

#[test]
fn test_map_example_from_doc() {
    let test = CapnpcTest::new(r#"@0x99d187209d25cee7; struct Map(Key, Value) {
        entries @0 :List(Entry);
        struct Entry { key @0 :Key; value @1 :Value; }
    }"#);
    {
        let gen = ::codegen::GeneratorContext::new(&test.message).unwrap();

        // Map structure: generic
        // need 2 parameters named Key and Value, parameter expansion is noop
        let map = get_node_by_name(&gen, "utest:Map").unwrap();
        assert!(map.get_is_generic());
        assert_eq!(2, map.get_parameters().unwrap().len());
        let map_params:Vec<&str> = map.get_parameters().unwrap().iter().map(|p| p.get_name().unwrap()).collect();
        assert_eq!(vec!("Key", "Value"), map_params);
        let map_expanded_params:Vec<String> = map.expand_parameters(&gen);
        assert_eq!(vec!("Key".to_string(), "Value".to_string()), map_expanded_params);

        // Map:Entry structure: generic
        // no parameters in schema but inherits parent Map ones
        let entry = get_node_by_name(&gen, "utest:Map.Entry").unwrap();
        assert!(entry.get_is_generic());
        assert_eq!(0, entry.get_parameters().unwrap().len());
        let entry_expanded_params:Vec<String> = entry.expand_parameters(&gen);
        assert_eq!(vec!("Key".to_string(), "Value".to_string()), entry_expanded_params);

        // Map.entries field is a list of implicitely parameterized entries
        // in rust code, we need that to be explicit
        let map_st = node_as_struct(map);
        assert_eq!(1, map_st.get_fields().unwrap().len());
        assert_eq!("struct_list::Reader<'a,::utest_capnp::map::entry::Owned<KeyReader,ValueReader,KeyBuilder,ValueBuilder>>",
                type_string_for(&gen, &map_st, "entries"));

        let entry_st = node_as_struct(&entry);
        assert_eq!(2, entry_st.get_fields().unwrap().len());
        assert_eq!("key", entry_st.get_fields().unwrap().get(0).get_name().unwrap());
        assert_eq!("value", entry_st.get_fields().unwrap().get(1).get_name().unwrap());
    }

    test.codegen();
    test.compile_or_persist();
}

#[test]
fn test_partial_parameter_list_expansion() {
    let test = CapnpcTest::new(r#"@0x99d187209d25cee7; 
        struct TestGenerics(Foo, Bar) {
          foo @0 :Foo; rev @1 :TestGenerics(Bar, Foo);
          struct Inner { foo @0 :Foo; bar @1 :Bar; }
          struct Inner2(Baz) { bar @0 :Bar; baz @1 :Baz; innerBound @2 :Inner; innerUnbound @3 :TestGenerics.Inner; }
          interface Interface(Qux) { call @0 Inner2(Text) -> (qux :Qux, gen :TestGenerics(Text, Data)); }
    } "#);
    {
        let gen = ::codegen::GeneratorContext::new(&test.message).unwrap();
        // TestGenerics parameters list
        let test_gen = get_node_by_name(&gen, "utest:TestGenerics").unwrap();
        assert!(test_gen.get_is_generic());
        let test_gen_params:Vec<&str> = test_gen.get_parameters().unwrap().iter().map(|p| p.get_name().unwrap()).collect();
        assert_eq!(vec!("Foo", "Bar"), test_gen_params);
        let test_gen_expanded_params:Vec<String> = test_gen.expand_parameters(&gen);
        assert_eq!(vec!("Foo".to_string(), "Bar".to_string()), test_gen_expanded_params);

        // TestGenerics.Inner parameters list
        let inner = get_node_by_name(&gen, "utest:TestGenerics.Inner").unwrap();
        let inner_gen_params:Vec<&str> = inner.get_parameters().unwrap().iter().map(|p| p.get_name().unwrap()).collect();
        assert_eq!(0, inner_gen_params.len());
        let inner_expanded_params:Vec<String> = inner.expand_parameters(&gen);
        assert_eq!(vec!("Foo".to_string(), "Bar".to_string()), inner_expanded_params);

        // TestGenerics.Inner2 parameters list
        let inner2 = get_node_by_name(&gen, "utest:TestGenerics.Inner2").unwrap();
        let inner2_gen_params:Vec<&str> = inner2.get_parameters().unwrap().iter().map(|p| p.get_name().unwrap()).collect();
        assert_eq!(vec!("Baz"), inner2_gen_params);
        let inner2_expanded_params:Vec<String> = inner2.expand_parameters(&gen);
        assert_eq!(vec!("Baz".to_string(), "Bar".to_string(), "Foo".to_string()), inner2_expanded_params);

        // TestGenerics.Interface parameters list
        let interface = get_node_by_name(&gen, "utest:TestGenerics.Interface").unwrap();
        let interface_params:Vec<&str> = interface.get_parameters().unwrap().iter().map(|p| p.get_name().unwrap()).collect();
        assert_eq!(vec!("Qux"), interface_params);
        let interface_expanded_params:Vec<String> = interface.expand_parameters(&gen);
        assert_eq!(vec!("Qux".to_string(), "Foo".to_string(), "Bar".to_string()), interface_expanded_params);

        // TestGenerics.Interface.call_result parameters list
        let call_results = get_node_by_name(&gen, "utest:TestGenerics.Interface.call$Results").unwrap();
        let call_results_params:Vec<&str> = call_results.get_parameters().unwrap().iter().map(|p| p.get_name().unwrap()).collect();
        assert_eq!(0, call_results_params.len());
        let call_result_expanded_params:Vec<String> = call_results.expand_parameters(&gen);
        assert_eq!(vec!("Qux".to_string()), call_result_expanded_params);

        // TestGenerics fields types
        let test_gen_st = node_as_struct(test_gen);
        assert_eq!(2, test_gen_st.get_fields().unwrap().len());
        assert_eq!("FooReader", type_string_for(&gen, &test_gen_st, "foo"));
        assert_eq!("::utest_capnp::test_generics::Reader<'a,BarReader,FooReader,BarBuilder,FooBuilder>", type_string_for(&gen, &test_gen_st, "rev"));

        // TestGenerics.Inner fields types
        let inner_st = node_as_struct(inner);
        assert_eq!(2, inner_st.get_fields().unwrap().len());
        assert_eq!("FooReader", type_string_for(&gen, &inner_st, "foo"));
        assert_eq!("BarReader", type_string_for(&gen, &inner_st, "bar"));

        // TestGenerics.Inner2 fields types
        let inner2_st = node_as_struct(inner2);
        assert_eq!(4, inner2_st.get_fields().unwrap().len());
        assert_eq!("BarReader", type_string_for(&gen, &inner2_st, "bar"));
        assert_eq!("BazReader", type_string_for(&gen, &inner2_st, "baz"));
        assert_eq!("::utest_capnp::test_generics::inner::Reader<'a,FooReader,BarReader,FooBuilder,BarBuilder>", type_string_for(&gen, &inner2_st, "innerBound"));
        assert_eq!("::utest_capnp::test_generics::inner::Reader<'a,::capnp::any_pointer::Reader<'a>,::capnp::any_pointer::Reader<'a>,::capnp::any_pointer::Builder<'a>,::capnp::any_pointer::Builder<'a>>", type_string_for(&gen, &inner2_st, "innerUnbound"));

        // TestGenerics.Interface.call types
        let interface_as_iface = match interface.which().unwrap() {
            ::schema_capnp::node::Interface(it) => it,
            _ => { panic!("expected an interface here") }
        };
        let call_method = interface_as_iface.get_methods().unwrap().iter().next().unwrap();
        let param_type = gen.node_map[&call_method.get_param_struct_type()];
        assert_eq!("::utest_capnp::test_generics::inner2::Reader<text::Reader,FooReader,BarReader,text::Builder,FooBuilder,BarBuilder>",
                    param_type.type_string(&gen, &call_method.get_param_brand().unwrap(), None, Module::Reader, ""));
        let result_type = gen.node_map[&call_method.get_result_struct_type()];
        assert_eq!("::Reader<QuxReader,QuxBuilder>",
                    result_type.type_string(&gen, &call_method.get_result_brand().unwrap(), Some(&vec!()), Module::Reader, ""));
    }
}
