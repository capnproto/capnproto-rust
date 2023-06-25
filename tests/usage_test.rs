#![allow(unused_imports)]
use capnp_import::capnp_import;

// Has to be top level
capnp_import!("tests/**/*.capnp");
//capnp_import!("tests/example.capnp");
//capnp_import!("tests/folder-test/*");

// Can be a list of patterns instead
//capnp_import!("tests/example.capnp", "tests/folder-test/*.capnp");

#[test]
fn usage_test() {
    //use example2_capnp::foo;
    use tests::example_capnp::{date, person};
    use tests::folder_test::example_capnp::foo;
}
