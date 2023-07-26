#![allow(unused_imports)]

// Has to be top level
capnp_import::capnp_import!("capnp-import/tests/example.capnp");

#[test]
fn simple_test() {
    use example_capnp::{date, person};
}
