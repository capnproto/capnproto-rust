use capnp_import;

include!(concat!(env!("OUT_DIR"), "/binary_decision.rs"));

#[test]
fn binary_decision_test() {
    assert_eq!(CAPNP_BIN_PATH.is_empty(), false);
}
