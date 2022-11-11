fn main() {
    capnpc::CompilerCommand::new()
        .file("addressbook.capnp")
        .run()
        .expect("compiling schema");
}
