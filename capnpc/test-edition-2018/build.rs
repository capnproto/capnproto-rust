fn main() {
     capnpc::CompilerCommand::new()
        .file("../test/test.capnp")
        .src_prefix("../test/")
        .run()
        .expect("compiling schema");
}
