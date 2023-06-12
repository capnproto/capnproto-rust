fn main() {
    capnpc::CompilerCommand::new()
        .link_override(0xe6f94f52f7be8fe2, "external_crate")
        .file("../test/test.capnp")
        .src_prefix("../test/")
        .run()
        .expect("compiling schema");
}
