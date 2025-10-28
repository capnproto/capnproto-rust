fn main() {
    capnpc::CompilerCommand::new()
        .crate_provides("external_crate", [0xe6f94f52f7be8fe2])
        .file("../test/test.capnp")
        .src_prefix("../test/")
        .run()
        .expect("compiling schema");
}
