extern crate capnpc;

fn main() {
     ::capnpc::CompileCommand::new()
        .file("test.capnp")
        .run()
        .expect("compiling schema");
}
