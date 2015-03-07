#![feature(path)]

extern crate capnpc;

fn main() {
    ::capnpc::compile(::std::path::Path::new("."),
                      &[::std::path::Path::new("eval.capnp"),
                       ::std::path::Path::new("catrank.capnp"),
                       ::std::path::Path::new("carsales.capnp")]).unwrap();
}
