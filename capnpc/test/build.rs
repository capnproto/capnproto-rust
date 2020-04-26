extern crate capnpc;

fn main() {
     capnpc::CompilerCommand::new()
        .file("test.capnp")
        .file("in-submodule.capnp")
        .file("in-other-submodule.capnp")
        .file("schema/test-in-dir.capnp")
        .file("schema-with-src-prefix/test-in-src-prefix-dir.capnp")
        .src_prefix("schema-with-src-prefix")
        .run()
        .expect("compiling schema");
}
