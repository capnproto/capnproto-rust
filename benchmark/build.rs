extern crate capnpc;

fn main() {
    ::capnpc::CompileCommand::new()
        .file("eval.capnp")
        .file("catrank.capnp")
        .file("carsales.capnp")
        .run()
        .expect("compiling schemas");
}
