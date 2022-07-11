fn main() {
    ::capnpc::CompilerCommand::new().file("hello_world.capnp").run().unwrap();
}
