capnp::generated_code!(pub mod app_config_capnp);

use std::path::Path;

use crate::app_config_capnp::app_config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut message = capnp::message::Builder::new_default();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| format!("{}/sample-config.json", env!("CARGO_MANIFEST_DIR")));
    let json = std::fs::read_to_string(Path::new(&path))?;
    let mut deserializer = serde_json::Deserializer::from_str(&json);
    let mut root = message.init_root::<app_config::Builder<'_>>();
    root.deserialize_serde(&mut deserializer)?;

    let config = message.get_root_as_reader::<app_config::Reader<'_>>()?;
    println!("app_name: {}", config.get_app_name()?.to_str()?);
    println!("port: {}", config.get_port());
    println!("verbose: {}", config.get_verbose());

    Ok(())
}
