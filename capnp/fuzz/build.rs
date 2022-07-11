fn main() {
    ::capnpc::CompilerCommand::new()
        .file("fuzzers/test.capnp")
        .src_prefix("fuzzers")
        .run()
        .expect("compiling schema");
}
