capnp::generated_code!(pub mod app_config_capnp);

use std::path::Path;

use crate::app_config_capnp::app_config;

fn serialize_json_round_trip() {
    let mut message = capnp::message::Builder::new_default();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| format!("{}/sample-config.json", env!("CARGO_MANIFEST_DIR")));
    let json = std::fs::read_to_string(Path::new(&path)).unwrap();
    let mut deserializer = serde_json::Deserializer::from_str(&json);
    let mut root = message.init_root::<app_config::Builder<'_>>();
    root.deserialize_serde(&mut deserializer).unwrap();

    let config = message
        .get_root_as_reader::<app_config::Reader<'_>>()
        .unwrap();
    assert_eq!(
        "hive-worker",
        config.get_app_name().unwrap().to_str().unwrap()
    );
    assert_eq!(7000, config.get_port());
    assert!(config.get_verbose());

    let json = serde_json::to_string_pretty(&config).unwrap();
    println!("{json}");
}

fn main() {
    serialize_json_round_trip();
}

#[test]
fn test_serialize_json_round_trip() {
    serialize_json_round_trip();
}
