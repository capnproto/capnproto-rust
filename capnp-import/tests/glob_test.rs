#![allow(unused_imports)]
use capnp_import::capnp_import;

// Has to be top level, contains `tests/example.capnp` and `tests/folder-test/example.capnp`
capnp_import!("capnp-import/tests/**/*.capnp");

#[test]
fn glob_test() {
    use example_capnp::{date, person};
    use foo_capnp::foo;
}
