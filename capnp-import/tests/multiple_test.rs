#![allow(unused_imports)]

// Has to be top level, contains `tests/example.capnp` and `tests/folder-test/example.capnp`
capnp_import::capnp_import!(
    "capnp-import/tests/example.capnp",
    "capnp-import/tests/folder-test/*.capnp"
);

#[test]
fn multiple_test() {
    use example_capnp::{date, person};
    use foo_capnp::foo;
}
