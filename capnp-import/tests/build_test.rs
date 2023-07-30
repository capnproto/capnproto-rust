include!(concat!(env!("OUT_DIR"), "/extract_bin.rs"));

#[test]
fn binary_decision_test() {
    assert!(commandhandle().unwrap().path().exists());
}
