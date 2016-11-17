extern crate capnpc;

fn main() {
    ::capnpc::CompileCommand::new()
        .file("addressbook.capnp")
        .run()
        .expect("compiling schema");
}}
