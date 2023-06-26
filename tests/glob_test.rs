#![allow(unused_imports)]
use capnp_import::capnp_import;

// Has to be top level, contains `tests/example.capnp` and `tests/folder-test/example.capnp`
capnp_import!("tests/**/*.capnp");

#[test]
fn glob_test() {
    use tests::example_capnp::{date, person};
    use tests::folder_test::example_capnp::foo;
}
