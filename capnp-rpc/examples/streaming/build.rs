fn main() -> Result<(), Box<dyn std::error::Error>> {
    capnpc::CompilerCommand::new()
        .file("streaming.capnp")
        .run()?;
    Ok(())
}
