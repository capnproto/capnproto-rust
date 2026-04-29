fn main() -> Result<(), Box<dyn std::error::Error>> {
    capnpc::CompilerCommand::new()
        .import_path(".")
        .file("bytestream.capnp")
        .run()?;
    Ok(())
}
