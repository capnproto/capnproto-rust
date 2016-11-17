extern crate capnpc;

fn main() {
    let mut command = ::capnpc::CompileCommand::new();
    command.file("test.capnp");
    command.run().expect("compiling schema");
}
