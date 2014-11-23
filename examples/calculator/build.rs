extern crate capnpc;

fn main() {
    ::capnpc::compile(Path::new("."),
                      &[Path::new("calculator.capnp")]).unwrap();
}
