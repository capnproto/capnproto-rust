extern crate capnpc;

fn main() {
    ::capnpc::compile(".", &["eval.capnp", "catrank.capnp", "carsales.capnp"]).unwrap();
}
