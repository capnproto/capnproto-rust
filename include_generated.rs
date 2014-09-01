/*
 * This is a hack to allow us to do capnpc-rust code generation and still use Cargo.
 */


#![crate_name="include_generated"]
#![crate_type = "lib"]

extern crate capnp;

pub mod rpc_capnp;
pub mod rpc_twoparty_capnp;



