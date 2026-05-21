fn main() -> Result<(), Box<dyn std::error::Error>> {
    capnpc::CompilerCommand::new()
        .file("echo.capnp")
        .run()?;
    capnpc::CompilerCommand::new()
        .file("shared_secret.capnp")
        .run()?;
    Ok(())
}
