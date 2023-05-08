fn main() {
    ::capnpc::CompilerCommand::new()
        .file("fill.capnp")
        .file("corpora.capnp")
        .file("addressbook.capnp")
        .file("shapes.capnp")
        .run()
        .expect("compiling schema");
}
