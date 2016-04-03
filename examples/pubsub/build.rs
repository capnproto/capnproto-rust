extern crate capnpc;

fn main() {
    ::capnpc::compile(".", &["pubsub.capnp"]).unwrap();
}
