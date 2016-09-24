extern crate capnpc;

fn main() {
    ::capnpc::compile(".", &["addressbook.capnp"]).unwrap();
}
