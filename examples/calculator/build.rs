extern crate capnpc;

fn main() {
    ::capnpc::compile(".", &["calculator.capnp"]).unwrap();
}
