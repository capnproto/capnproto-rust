fn main() {
    capnpc::CompilerCommand::new()
        .file("external.capnp")
        .run()
        .expect("compiling schema");
}
