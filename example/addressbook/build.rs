extern crate capnpc;

fn main() {
    ::capnpc::compile(Path::new("."),
                      vec!(Path::new("addressbook.capnp")).as_slice());
}
