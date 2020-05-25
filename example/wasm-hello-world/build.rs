fn main() {
    ::capnpc::CompilerCommand::new()
        .file("wasm-hello-world.capnp")
        .run()
        .expect("compiling schema");
}
