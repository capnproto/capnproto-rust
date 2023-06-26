#![allow(unused_imports)]

// Has to be top level, contains `tests/example.capnp` and `tests/folder-test/example.capnp`
capnp_import::capnp_import!("tests/example.capnp", "tests/folder-test/*.capnp");

#[test]
fn multiple_test() {
    use tests::example_capnp::{date, person};
    use tests::folder_test::example_capnp::foo;
}
