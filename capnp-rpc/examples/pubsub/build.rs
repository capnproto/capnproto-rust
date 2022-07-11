fn main() {
    ::capnpc::CompilerCommand::new().file("pubsub.capnp").run().unwrap();
}
