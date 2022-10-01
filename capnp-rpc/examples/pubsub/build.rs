fn main() -> Result<(), Box<dyn std::error::Error>> {
    capnpc::CompilerCommand::new().file("pubsub.capnp").run()?;
    Ok(())
}
