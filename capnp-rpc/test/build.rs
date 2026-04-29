fn main() {
    ::capnpc::CompilerCommand::new()
        .import_path(".")
        .file("test.capnp")
        .run()
        .unwrap();
}
