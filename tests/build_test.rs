include!(concat!(env!("OUT_DIR"), "/extract_bin.rs"));

#[test]
fn binary_decision_test() {
    assert_eq!(commandhandle().unwrap().path().exists(), true);
}
