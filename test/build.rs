extern crate capnpc;

fn main() {
    ::capnpc::compile(".", &["test.capnp"]).unwrap();
}
