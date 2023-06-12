fn main() {
    capnpc::CompilerCommand::new()
        .file("external.capnp")
        .import_path("../")
        .run()
        .expect("compiling schema");
}
