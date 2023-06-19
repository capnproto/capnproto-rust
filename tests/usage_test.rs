#![allow(unused_imports)]
use capnp_import::capnp_import;

//capnp_import!("tests/example.capnp", "tests/folder-test/*.capnp");
capnp_import!("tests/example.capnp");
//capnp_import!("tests/example.capnp");
//capnp_import!("tests/folder-test/*");

#[test]
fn basic_file_test() {
    //use example2_capnp::foo;
    use example_capnp::date;
    use example_capnp::person;
}

#[test]
fn folder_test() {
    //use person;
}
