extern crate capnp;
extern crate core;
extern crate external_crate;

pub mod test_capnp {
    include!(concat!(env!("OUT_DIR"), "/test_capnp.rs"));
}
