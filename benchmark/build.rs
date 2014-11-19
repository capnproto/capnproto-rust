extern crate capnpc;

fn main() {
    ::capnpc::compile(Path::new("."),
                      &[Path::new("eval.capnp"),
                       Path::new("catrank.capnp"),
                       Path::new("carsales.capnp")]);
}
