fn main() {
    ::capnpc::CompilerCommand::new()
        .file("eval.capnp")
        .file("catrank.capnp")
        .file("carsales.capnp")
        .run()
        .expect("compiling schemas");
}
