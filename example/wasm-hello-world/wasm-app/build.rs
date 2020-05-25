fn main() {
    ::capnpc::CompilerCommand::new()
        .file("../wasm-hello-world.capnp")
        .src_prefix("../")
        .run()
        .expect("compiling schema");
}
